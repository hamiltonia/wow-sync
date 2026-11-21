//! Settings behaviour (settings.md): scope and diagnoses, capture classification, the policy
//! authority, the revision graph and its ordered target cases, the authority epoch, publication,
//! paired selection, retention and GC roots, and the resume preview. Pure: no file is read, no
//! byte is parsed or executed; callers supply sizes, hashes and recorded facts.

use std::collections::{BTreeMap, BTreeSet};

use crate::authority::{covered, AuthorityRecord};
use crate::decision::{CycleCx, DecisionKind, SettingsTarget};
use crate::hooks::SettingsHooks;
use crate::ids::{nfc, CheckpointRef, Identity, SettingsUnit, Sha256};
use crate::kinds::{EnrollmentRef, PolicyRule, PolicyRuleKind, RevisionKind, SettingsPolicy};
use crate::ledger::{resolve_known, Derived, Ledger, Status};
use crate::paths::{
    account_settings_path, character_settings_path, classify_wtf_rel, validate_folder_name,
    FolderKey, SettingsPath, WtfClass,
};
use crate::plan::{PackageApply, KEEP_OWN_STATES};
use crate::records::{LocalUnitState, MissingReason, RootState, SettingsRoot};

/// Current plus two previous states per unit, independent of code (settings.md §7).
pub const KEEP_SETTINGS_STATES: usize = KEEP_OWN_STATES;

// ------------------------------------------------------------------------- scope (§1)

/// One file found under `WTF` by the adapter: path relative to `WTF` with `/`, size and hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WtfEntry {
    pub rel: String,
    pub size: u64,
    pub sha256: Sha256,
    /// A component was a link, junction or reparse point.
    pub unsafe_path: bool,
}

/// What the `.toc` of an installed folder declares (metadata only, never Lua).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaringFolder {
    pub folder: FolderKey,
    pub saved_variables: Vec<String>,
    pub saved_variables_per_character: Vec<String>,
    pub toc_ambiguous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedFile {
    pub unit: SettingsUnit,
    pub rel: SettingsPath,
    pub size: u64,
    pub sha256: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeDiagnosis {
    SharedVariable {
        name: String,
        folders: Vec<FolderKey>,
    },
    Unowned {
        file: String,
    },
    AmbiguousToc {
        folder: FolderKey,
    },
    UnsafePath {
        path: String,
    },
    TooLarge {
        file: String,
        size: u64,
    },
    UnsafeIdentity {
        name: String,
    },
    IdentityNormalizationCollision {
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsScope {
    pub owned: Vec<OwnedFile>,
    pub diagnoses: Vec<ScopeDiagnosis>,
    /// Units that must not sync in either direction because of a diagnosis.
    pub blocked: BTreeSet<SettingsUnit>,
    /// Real-data sync needs an approved size cap (Q-77 PENDING OWNER).
    pub sync_enabled: bool,
}

/// `settings_scope` (§1.1-§1.2). `size_cap` is `None` unless a value is approved; tests pass
/// their own. `included` holds the NFC account names the user included on this computer.
pub fn settings_scope(
    decls: &[DeclaringFolder],
    files: &[WtfEntry],
    included: &BTreeSet<String>,
    size_cap: Option<u64>,
) -> SettingsScope {
    let mut out = SettingsScope {
        sync_enabled: size_cap.is_some(),
        ..Default::default()
    };
    let mut diag: BTreeSet<ScopeDiagnosis> = BTreeSet::new();

    let mut shared_acc: BTreeMap<&str, BTreeSet<FolderKey>> = BTreeMap::new();
    let mut shared_chr: BTreeMap<&str, BTreeSet<FolderKey>> = BTreeMap::new();
    for d in decls {
        for v in &d.saved_variables {
            shared_acc.entry(v).or_default().insert(d.folder.clone());
        }
        for v in &d.saved_variables_per_character {
            shared_chr.entry(v).or_default().insert(d.folder.clone());
        }
    }
    let mut shared_folders: BTreeSet<FolderKey> = BTreeSet::new();
    for (name, set) in shared_acc.iter().chain(shared_chr.iter()) {
        if set.len() > 1 {
            shared_folders.extend(set.iter().cloned());
            diag.insert(ScopeDiagnosis::SharedVariable {
                name: (*name).to_string(),
                folders: set.iter().cloned().collect(),
            });
        }
    }
    for d in decls {
        if d.toc_ambiguous
            && (!d.saved_variables.is_empty() || !d.saved_variables_per_character.is_empty())
        {
            diag.insert(ScopeDiagnosis::AmbiguousToc {
                folder: d.folder.clone(),
            });
        }
    }

    let mut accounts: BTreeSet<String> = BTreeSet::new();
    for f in files {
        if let Some(a) = f
            .rel
            .strip_prefix("Account/")
            .and_then(|r| r.split('/').next())
        {
            accounts.insert(a.to_string());
        }
    }
    let names: Vec<String> = accounts.iter().cloned().collect();
    let colliding: BTreeSet<String> = {
        let mut by_nfc: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for n in &names {
            by_nfc.entry(nfc(n)).or_default().push(n);
        }
        by_nfc
            .into_values()
            .filter(|v| v.len() > 1)
            .flat_map(|v| {
                diag.insert(ScopeDiagnosis::IdentityNormalizationCollision {
                    paths: v.iter().map(|s| (*s).clone()).collect(),
                });
                v.into_iter().cloned()
            })
            .collect()
    };

    for f in files {
        let class = classify_wtf_rel(&f.rel);
        let (account, folder, identity) = match &class {
            WtfClass::Account { account, folder } => (account, folder, Identity::account(account)),
            WtfClass::Character {
                account,
                realm,
                character,
                folder,
            } => (
                account,
                folder,
                Identity::character(account, realm, character),
            ),
            WtfClass::Excluded => continue,
        };
        if !included.contains(&nfc(account)) {
            continue;
        }
        let Ok(identity) = identity else {
            diag.insert(ScopeDiagnosis::UnsafeIdentity {
                name: account.clone(),
            });
            continue;
        };
        let Ok(key) = validate_folder_name(folder) else {
            continue;
        };
        let unit = SettingsUnit {
            folder: key.clone(),
            identity,
        };
        let is_account = matches!(class, WtfClass::Account { .. });
        let declares = decls.iter().find(|d| d.folder == key).is_some_and(|d| {
            if is_account {
                !d.saved_variables.is_empty()
            } else {
                !d.saved_variables_per_character.is_empty()
            }
        });
        if !declares {
            diag.insert(ScopeDiagnosis::Unowned {
                file: f.rel.clone(),
            });
            continue;
        }
        if shared_folders.contains(&key)
            || colliding.contains(account)
            || diag.contains(&ScopeDiagnosis::AmbiguousToc {
                folder: key.clone(),
            })
        {
            out.blocked.insert(unit);
            continue;
        }
        if f.unsafe_path {
            diag.insert(ScopeDiagnosis::UnsafePath {
                path: f.rel.clone(),
            });
            out.blocked.insert(unit);
            continue;
        }
        if size_cap.is_some_and(|cap| f.size > cap) {
            diag.insert(ScopeDiagnosis::TooLarge {
                file: f.rel.clone(),
                size: f.size,
            });
            out.blocked.insert(unit);
            continue;
        }
        let rel = match &class {
            WtfClass::Account { account, folder } => account_settings_path(account, folder),
            WtfClass::Character {
                account,
                realm,
                character,
                folder,
            } => character_settings_path(account, realm, character, folder),
            WtfClass::Excluded => continue,
        };
        if let Ok(rel) = rel {
            out.owned.push(OwnedFile {
                unit,
                rel,
                size: f.size,
                sha256: f.sha256,
            });
        }
    }
    for d in &diag {
        if let ScopeDiagnosis::SharedVariable { folders, .. } = d {
            for f in folders {
                for o in &out.owned {
                    if &o.unit.folder == f {
                        out.blocked.insert(o.unit.clone());
                    }
                }
            }
        }
    }
    out.owned.retain(|o| !out.blocked.contains(&o.unit));
    out.diagnoses = diag.into_iter().collect();
    out
}

/// What an uninstall may take from `WTF` for the removed folders (removal.md §1.6 step 6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemovalTargets {
    /// Files to capture and then move into recovery.
    pub files: Vec<OwnedFile>,
    /// Units seen but left in place, with the reason.
    pub missing: Vec<(SettingsUnit, MissingReason)>,
}

/// Select the removed folders' units from a scope computed over the installed declarations
/// plus the removed folders' own. A shared or ambiguous owner, an oversized or unsafe file and
/// a paused folder stay where they are; the planner then records them as not captured.
pub fn removal_targets(
    scope: &SettingsScope,
    folders: &BTreeSet<FolderKey>,
    paused: &BTreeSet<FolderKey>,
) -> RemovalTargets {
    let mut out = RemovalTargets::default();
    for o in &scope.owned {
        if !folders.contains(&o.unit.folder) {
            continue;
        }
        if paused.contains(&o.unit.folder) {
            out.missing.push((o.unit.clone(), MissingReason::Paused));
        } else {
            out.files.push(o.clone());
        }
    }
    for u in scope.blocked.iter().filter(|u| folders.contains(&u.folder)) {
        let stem = format!("/{}.lua", u.folder.as_str()).to_lowercase();
        let reason = scope
            .diagnoses
            .iter()
            .find_map(|d| match d {
                ScopeDiagnosis::SharedVariable { folders: f, .. } if f.contains(&u.folder) => {
                    Some(MissingReason::SharedFile)
                }
                ScopeDiagnosis::AmbiguousToc { folder } if folder == &u.folder => {
                    Some(MissingReason::UnknownOwner)
                }
                ScopeDiagnosis::TooLarge { file, .. } if file.to_lowercase().ends_with(&stem) => {
                    Some(MissingReason::TooLarge)
                }
                ScopeDiagnosis::UnsafePath { path } if path.to_lowercase().ends_with(&stem) => {
                    Some(MissingReason::Unreadable)
                }
                _ => None,
            })
            .unwrap_or(MissingReason::IdentityCollision);
        out.missing.push((u.clone(), reason));
    }
    out
}

// ------------------------------------------------------------------------- capture (§2)

/// A unit's file as one scan saw it: `None` when absent.
pub type FileFact = Option<(u64, Sha256)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleScans {
    pub first: FileFact,
    pub second: FileFact,
    pub hashed: FileFact,
    pub wow_running_at_start: bool,
    pub wow_running_at_end: bool,
    pub bak_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureClass {
    Complete,
    Suspect,
    Unchanged,
}

/// §2.2: suspect unless WoW was stopped throughout, the file was identical across both settle
/// scans and the hashing, nothing shrank to zero and nothing vanished next to a `.bak`.
/// `prev` is the previous capture's or basis checkpoint's file.
pub fn classify_capture(prev: FileFact, s: &SettleScans) -> CaptureClass {
    if s.wow_running_at_start || s.wow_running_at_end {
        return CaptureClass::Suspect;
    }
    if s.first != s.second || s.second != s.hashed {
        return CaptureClass::Suspect;
    }
    if matches!(s.hashed, Some((0, _))) {
        return CaptureClass::Suspect;
    }
    if prev.is_some() && s.hashed.is_none() && s.bak_present {
        return CaptureClass::Suspect;
    }
    if s.hashed == prev {
        CaptureClass::Unchanged
    } else {
        CaptureClass::Complete
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    SessionEnd,
    FoundAtStart,
    BeforeChange,
    Activation,
}

// ------------------------------------------------------------------------- authority (§5.1)

/// A policy head as the authority rules see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyView {
    pub hash: Sha256,
    pub master: Option<EnrollmentRef>,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authority {
    NoPolicy,
    Holder(EnrollmentRef),
    NoMaster,
    NotShared,
    /// Policy heads disagree for this folder only.
    Conflict,
}

fn one_authority(p: &PolicyView, folder: &FolderKey) -> Authority {
    // Keyed by folder key, never by package key (R4D-6).
    match p.rules.iter().find(|r| &r.folder == folder) {
        Some(r) => match (&r.rule, r.computer_id.clone(), r.enrollment_id.clone()) {
            (PolicyRuleKind::NotShared, _, _) => Authority::NotShared,
            (PolicyRuleKind::Master, Some(c), Some(e)) => Authority::Holder(EnrollmentRef {
                computer_id: c,
                enrollment_id: e,
            }),
            _ => Authority::NoMaster,
        },
        None => match &p.master {
            Some(m) => Authority::Holder(m.clone()),
            None => Authority::NoMaster,
        },
    }
}

pub fn authority_for(heads: &[PolicyView], folder: &FolderKey) -> Authority {
    let mut it = heads.iter().map(|p| one_authority(p, folder));
    let Some(first) = it.next() else {
        return Authority::NoPolicy;
    };
    if it.all(|a| a == first) {
        first
    } else {
        Authority::Conflict
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasterState {
    Present(EnrollmentRef),
    Absent { was: EnrollmentRef },
    NotChosen,
}

/// The default master of the heads, never elected here: a retired or missing master is `Absent`.
pub fn master_state(heads: &[PolicyView], live: &BTreeSet<EnrollmentRef>) -> MasterState {
    let masters: BTreeSet<Option<&EnrollmentRef>> =
        heads.iter().map(|p| p.master.as_ref()).collect();
    match masters.into_iter().collect::<Vec<_>>().as_slice() {
        [Some(m)] if live.contains(m) => MasterState::Present((*m).clone()),
        [Some(m)] => MasterState::Absent { was: (*m).clone() },
        _ => MasterState::NotChosen,
    }
}

/// An `auto` revision is valid iff its issuer is the authority under every listed witness; a
/// witness is never re-checked against later policies (R4D-30). 1..=8 witnesses.
pub fn auto_revision_valid(
    issuer: &EnrollmentRef,
    witnesses: &[PolicyView],
    folder: &FolderKey,
) -> bool {
    !witnesses.is_empty()
        && witnesses.len() <= 8
        && witnesses
            .iter()
            .all(|w| one_authority(w, folder) == Authority::Holder(issuer.clone()))
}

/// The witnesses an `auto` revision lists: the 8 smallest policy-head hashes.
pub fn witness_set(heads: &[PolicyView]) -> Vec<Sha256> {
    let mut h: Vec<Sha256> = heads.iter().map(|p| p.hash).collect();
    h.sort();
    h.truncate(8);
    h
}

/// Valid policies no other valid policy's `known` covers.
pub fn policy_heads(ledger: &Ledger, derived: &Derived) -> Vec<PolicyView> {
    let valid: Vec<(Sha256, &SettingsPolicy)> = ledger
        .entries()
        .filter(|e| derived.status.get(&e.hash) == Some(&Status::Valid))
        .filter_map(|e| match ledger.record(&e.hash) {
            Some(AuthorityRecord::SettingsPolicy(p)) => Some((e.hash, p)),
            _ => None,
        })
        .collect();
    valid
        .iter()
        .filter(|(h, p)| {
            !valid.iter().any(|(oh, o)| {
                oh != h
                    && resolve_known(ledger, &o.known)
                        .map(|k| covered(&k, &BTreeSet::new(), &p.issuer, h))
                        .unwrap_or(false)
            })
        })
        .map(|(h, p)| PolicyView {
            hash: *h,
            master: p.master.clone(),
            rules: crate::ledger::policy_folders(ledger, p).unwrap_or_default(),
        })
        .collect()
}

// ------------------------------------------------------------------------- revision graph (§4.2)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Valid,
    Pending,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevNode {
    pub hash: Sha256,
    pub parents: Vec<Sha256>,
    pub checkpoint: CheckpointRef,
    pub state: NodeState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionGraph {
    pub nodes: BTreeMap<Sha256, RevNode>,
    pub pending: bool,
    pub heads: Vec<Sha256>,
    /// Heads grouped by checkpoint: equal heads are one state and all stay parents (R4D-7).
    pub head_states: BTreeMap<CheckpointRef, Vec<Sha256>>,
}

/// Valid nodes only; a valid node whose parent is unknown is pending, never a root; an invalid
/// parent is absent for ancestry (R4D-8).
pub fn revision_graph(all: &[RevNode]) -> RevisionGraph {
    let known: BTreeSet<Sha256> = all.iter().map(|n| n.hash).collect();
    let mut g = RevisionGraph::default();
    for n in all {
        match n.state {
            NodeState::Invalid => {}
            NodeState::Pending => g.pending = true,
            NodeState::Valid if n.parents.iter().any(|p| !known.contains(p)) => g.pending = true,
            NodeState::Valid => {
                g.nodes.insert(n.hash, n.clone());
            }
        }
    }
    let named: BTreeSet<Sha256> = g
        .nodes
        .values()
        .flat_map(|n| n.parents.iter().copied())
        .collect();
    g.heads = g
        .nodes
        .keys()
        .filter(|h| !named.contains(h))
        .copied()
        .collect();
    for h in &g.heads {
        g.head_states
            .entry(g.nodes[h].checkpoint)
            .or_default()
            .push(*h);
    }
    g
}

impl RevisionGraph {
    fn ancestors_or_self(&self, h: &Sha256) -> BTreeSet<Sha256> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![*h];
        while let Some(x) = todo.pop() {
            if !self.nodes.contains_key(&x) || !seen.insert(x) {
                continue;
            }
            todo.extend(self.nodes[&x].parents.iter().copied());
        }
        seen
    }

    /// Every element is an ancestor-or-equal of some head.
    pub fn below_heads(&self, set: &BTreeSet<Sha256>) -> bool {
        let reach: BTreeSet<Sha256> = self
            .heads
            .iter()
            .flat_map(|h| self.ancestors_or_self(h))
            .collect();
        set.iter().all(|s| reach.contains(s))
    }
}

/// The ledger's revisions of one unit, with the derived status of each.
pub fn graph_from_ledger(ledger: &Ledger, derived: &Derived, unit: &SettingsUnit) -> RevisionGraph {
    let mut all = Vec::new();
    for e in ledger.entries() {
        let Some(AuthorityRecord::SettingsRevision(r)) = ledger.record(&e.hash) else {
            continue;
        };
        if &r.unit != unit {
            continue;
        }
        let state = match derived.status.get(&e.hash) {
            Some(Status::Valid) => NodeState::Valid,
            Some(Status::Pending(_)) | None => NodeState::Pending,
            Some(Status::Invalid(_)) => NodeState::Invalid,
        };
        all.push(RevNode {
            hash: e.hash,
            parents: r.parents.clone(),
            checkpoint: r.checkpoint,
            state,
        });
    }
    revision_graph(&all)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalReason {
    BasisNotShared,
    ConcurrentPush,
    EpochNotClean,
    /// Unshared local change (case 6).
    LocalChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Case {
    Paused,
    Waiting,
    Off,
    HeldForPolicy,
    NoSharedCopy,
    NeedsChoice,
    FirstAdoption(CheckpointRef),
    InSync {
        basis: Vec<Sha256>,
    },
    LocalDifferent(LocalReason),
    Yield {
        to: CheckpointRef,
        capture_first: bool,
    },
}

#[derive(Debug, Clone)]
pub struct TargetInput<'a> {
    pub graph: &'a RevisionGraph,
    pub authority: &'a Authority,
    pub is_authority: bool,
    pub paused: bool,
    pub basis: &'a [Sha256],
    /// The checkpoint of the basis head state (`Cb`).
    pub basis_checkpoint: Option<CheckpointRef>,
    /// The unit's live checkpoint (`L`); `None` when there is no file.
    pub live: Option<CheckpointRef>,
    pub epoch_clean: bool,
}

/// The ordered cases of §4.2.
pub fn target_case(i: &TargetInput) -> Case {
    let g = i.graph;
    if i.paused {
        return Case::Paused;
    }
    if g.pending {
        return Case::Waiting;
    }
    match i.authority {
        Authority::NoPolicy | Authority::NotShared => return Case::Off,
        Authority::Conflict => return Case::HeldForPolicy,
        _ => {}
    }
    if g.nodes.is_empty() {
        return Case::NoSharedCopy;
    }
    if g.head_states.len() >= 2 {
        return Case::NeedsChoice;
    }
    let c_h = *g.head_states.keys().next().expect("one head state");
    let heads: BTreeSet<Sha256> = g.heads.iter().copied().collect();
    let in_sync = i.live == Some(c_h);
    if i.basis.is_empty() {
        return Case::FirstAdoption(c_h);
    }
    let bv: BTreeSet<Sha256> = i
        .basis
        .iter()
        .filter(|b| g.nodes.contains_key(b))
        .copied()
        .collect();
    if bv.is_empty() || !g.below_heads(&bv) {
        return if in_sync {
            Case::InSync {
                basis: g.heads.clone(),
            }
        } else {
            Case::LocalDifferent(LocalReason::BasisNotShared)
        };
    }
    if bv == heads {
        return if in_sync {
            Case::InSync {
                basis: g.heads.clone(),
            }
        } else if i.is_authority && !i.epoch_clean {
            Case::LocalDifferent(LocalReason::EpochNotClean)
        } else {
            Case::LocalDifferent(LocalReason::LocalChange)
        };
    }
    if in_sync {
        return Case::InSync {
            basis: g.heads.clone(),
        };
    }
    if i.live.is_some() && i.live == i.basis_checkpoint {
        return Case::Yield {
            to: c_h,
            capture_first: false,
        };
    }
    if i.is_authority {
        Case::LocalDifferent(LocalReason::ConcurrentPush)
    } else {
        Case::Yield {
            to: c_h,
            capture_first: true,
        }
    }
}

/// A `choice` names every head of every head state, never merging bytes (§4.4).
pub fn choice_parents(g: &RevisionGraph) -> Vec<Sha256> {
    g.heads.clone()
}

// ------------------------------------------------------------------------- epoch and publication (§4.1)

/// The activation capture decides `epoch_clean`: it must equal the basis checkpoint. With no
/// basis it is clean only when no shared head exists to diverge from.
pub fn epoch_clean(
    activation: Option<CheckpointRef>,
    basis_checkpoint: Option<CheckpointRef>,
    shared_head_exists: bool,
) -> bool {
    match basis_checkpoint {
        Some(b) => activation == Some(b),
        None => !shared_head_exists,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishInput {
    pub authority: Authority,
    pub own: EnrollmentRef,
    pub paused: bool,
    pub complete: bool,
    pub kind: CaptureKind,
    pub changed_in_session: bool,
    pub epoch_clean: bool,
    pub authority_since: Option<u64>,
    pub compared_with_local_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Publish {
    Auto,
    NotAuthority,
    Paused,
    Suspect,
    Unchanged,
    NeverPublishes,
    DirtyEpoch,
    BeforeEpoch,
}

/// §4.1 publication table. Only the authority publishes automatically, only a complete,
/// changed capture inside a clean epoch whose comparison is strictly after activation.
pub fn publish_decision(i: &PublishInput) -> Publish {
    if matches!(i.kind, CaptureKind::Activation | CaptureKind::BeforeChange) {
        return Publish::NeverPublishes;
    }
    if i.authority != Authority::Holder(i.own.clone()) {
        return Publish::NotAuthority;
    }
    if i.paused {
        return Publish::Paused;
    }
    if !i.complete {
        return Publish::Suspect;
    }
    if !i.changed_in_session {
        return Publish::Unchanged;
    }
    if let Some(since) = i.authority_since {
        if !i.epoch_clean {
            return Publish::DirtyEpoch;
        }
        if i.compared_with_local_seq <= since {
            return Publish::BeforeEpoch;
        }
    }
    Publish::Auto
}

// ------------------------------------------------------------------------- paired selection (§4.6)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairedChoice {
    Exact(CheckpointRef),
    LocalHistory(CheckpointRef),
    Remote(CheckpointRef),
    /// Several unordered remote candidates: offered explicitly, none applied by default.
    CodeOnlyOffer(Vec<CheckpointRef>),
    CodeOnlyWarning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHistoryState {
    pub local_seq: u64,
    pub code_package_hash: Option<Sha256>,
    pub checkpoint: CheckpointRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCandidate {
    pub checkpoint: CheckpointRef,
    pub code_package_hash: Option<Sha256>,
    pub ancestor_of_current: bool,
}

/// Restoring code `x`. Never compares peers' `local_seq` or clocks.
pub fn paired_selection(
    exact: Option<CheckpointRef>,
    local: &[LocalHistoryState],
    remote: &[RemoteCandidate],
    x: &Sha256,
) -> PairedChoice {
    if let Some(c) = exact {
        return PairedChoice::Exact(c);
    }
    if let Some(l) = local
        .iter()
        .filter(|l| l.code_package_hash.as_ref() == Some(x))
        .max_by_key(|l| l.local_seq)
    {
        return PairedChoice::LocalHistory(l.checkpoint);
    }
    let cands: BTreeSet<CheckpointRef> = remote
        .iter()
        .filter(|r| !r.ancestor_of_current && r.code_package_hash.as_ref() == Some(x))
        .map(|r| r.checkpoint)
        .collect();
    match cands.len() {
        0 => PairedChoice::CodeOnlyWarning,
        1 => PairedChoice::Remote(cands.into_iter().next().expect("one")),
        _ => PairedChoice::CodeOnlyOffer(cands.into_iter().collect()),
    }
}

// ------------------------------------------------------------------------- retention (§7)

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pin {
    UnresolvedChoice,
    PairedRecovery,
    RemovalPoint,
    SuspectUntilComplete,
    NeedsHelp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEntry {
    pub checkpoint: CheckpointRef,
    pub local_seq: u64,
    pub pins: Vec<Pin>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prune {
    pub keep: Vec<CheckpointRef>,
    pub delete: Vec<CheckpointRef>,
    /// States kept beyond three, with their reasons (a visible temporary exception).
    pub exceptions: Vec<(CheckpointRef, Pin)>,
}

/// Newest three by `local_seq` stay; a pinned state or one a retained recovery point names
/// stays too; the rest go, oldest first. Never the current one.
pub fn prune_states(
    states: &[StateEntry],
    current: &CheckpointRef,
    named_by_points: &BTreeSet<CheckpointRef>,
) -> Prune {
    let mut sorted: Vec<&StateEntry> = states.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.local_seq));
    let mut p = Prune::default();
    let mut recent: BTreeSet<CheckpointRef> = BTreeSet::new();
    for s in sorted.iter().take(KEEP_SETTINGS_STATES) {
        recent.insert(s.checkpoint);
    }
    for s in sorted {
        let named = named_by_points.contains(&s.checkpoint);
        let is_recent = recent.contains(&s.checkpoint);
        if is_recent || named || s.checkpoint == *current || !s.pins.is_empty() {
            p.keep.push(s.checkpoint);
            if !is_recent {
                if let Some(pin) = s.pins.first() {
                    p.exceptions.push((s.checkpoint, *pin));
                }
            }
        } else {
            p.delete.push(s.checkpoint);
        }
    }
    p.delete.reverse();
    p
}

#[derive(Debug, Clone, Default)]
pub struct GcInputs {
    pub kept_states: Vec<CheckpointRef>,
    pub pinned: Vec<CheckpointRef>,
    pub recovery_point_roots: Vec<CheckpointRef>,
    pub removal_roots: Vec<CheckpointRef>,
    pub graph_heads: Vec<CheckpointRef>,
    pub paired_candidates: Vec<CheckpointRef>,
}

/// Every checkpoint a root names (Q-76). Unrooted ones are provider collector candidates only.
pub fn gc_roots(i: &GcInputs) -> BTreeSet<CheckpointRef> {
    [
        &i.kept_states,
        &i.pinned,
        &i.recovery_point_roots,
        &i.removal_roots,
        &i.graph_heads,
        &i.paired_candidates,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect()
}

// ------------------------------------------------------------------------- resume (§6)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeKind {
    NothingChanged,
    HereChangedNotShared,
    SharedNewer,
    SharedNewerReplacesMine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeOption {
    Resume,
    KeepMineHere,
    ShareMine,
    GoBackToShared,
    UseShared,
    StayPaused,
    UseSharedKeepMineAsRecovery,
    ShareMineInstead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePreview {
    pub kind: ResumeKind,
    /// Default first. Resume itself never publishes.
    pub options: Vec<ResumeOption>,
    pub publishes: bool,
}

pub fn resume_preview(here_changed: bool, shared_newer: bool) -> ResumePreview {
    use ResumeOption::*;
    let (kind, options) = match (here_changed, shared_newer) {
        (false, false) => (ResumeKind::NothingChanged, vec![Resume]),
        (true, false) => (
            ResumeKind::HereChangedNotShared,
            vec![KeepMineHere, ShareMine, GoBackToShared],
        ),
        (false, true) => (ResumeKind::SharedNewer, vec![UseShared, StayPaused]),
        (true, true) => (
            ResumeKind::SharedNewerReplacesMine,
            vec![StayPaused, UseSharedKeepMineAsRecovery, ShareMineInstead],
        ),
    };
    ResumePreview {
        kind,
        options,
        publishes: false,
    }
}

// ------------------------------------------------------------------------- hooks

/// What this computer recorded about one unit (the caller loads it from local state).
#[derive(Debug, Clone, Default)]
pub struct UnitFacts {
    pub basis: Vec<Sha256>,
    pub basis_checkpoint: Option<CheckpointRef>,
    pub live: Option<CheckpointRef>,
    pub paused: bool,
    pub epoch_clean: bool,
    pub local: Option<LocalUnitState>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsBehaviour {
    pub units: BTreeMap<SettingsUnit, UnitFacts>,
    pub missing: BTreeMap<SettingsUnit, MissingReason>,
}

impl SettingsBehaviour {
    pub fn target_for(&self, cx: &CycleCx, u: &SettingsUnit) -> Case {
        let Some(f) = self.units.get(u) else {
            return Case::Off;
        };
        let graph = graph_from_ledger(cx.ledger, cx.derived, u);
        let heads = policy_heads(cx.ledger, cx.derived);
        let authority = authority_for(&heads, &u.folder);
        let is_authority = authority == Authority::Holder(cx.own.clone());
        target_case(&TargetInput {
            graph: &graph,
            authority: &authority,
            is_authority,
            paused: f.paused,
            basis: &f.basis,
            basis_checkpoint: f.basis_checkpoint,
            live: f.live,
            epoch_clean: f.epoch_clean,
        })
    }
}

impl SettingsHooks for SettingsBehaviour {
    fn settings_target(&self, cx: &CycleCx, u: &SettingsUnit) -> SettingsTarget {
        match self.target_for(cx, u) {
            Case::Paused | Case::Waiting | Case::HeldForPolicy => SettingsTarget::Waiting,
            Case::Off | Case::NoSharedCopy | Case::InSync { .. } => SettingsTarget::NoChange,
            Case::NeedsChoice => SettingsTarget::NeedsDecision(DecisionKind::ChoicesDiffer),
            Case::FirstAdoption(c) | Case::Yield { to: c, .. } => SettingsTarget::Adopt(c),
            Case::LocalDifferent(_) => SettingsTarget::KeepLocal,
        }
    }

    // The apply bodies need staged blobs and journals; the runtime composes them (settings_cycle).
    fn plan_settings(&self, _: &CycleCx) -> Vec<PackageApply> {
        Vec::new()
    }

    fn roots_for(&self, _: &CycleCx, folders: &[FolderKey]) -> Vec<SettingsRoot> {
        let wanted: BTreeSet<&FolderKey> = folders.iter().collect();
        let mut units: BTreeSet<&SettingsUnit> = self.units.keys().collect();
        units.extend(self.missing.keys());
        units
            .into_iter()
            .filter(|u| wanted.contains(&u.folder))
            .map(|u| {
                let state = match self.missing.get(u) {
                    Some(reason) => RootState::Missing { reason: *reason },
                    None => match self
                        .units
                        .get(u)
                        .and_then(|f| f.local.as_ref())
                        .and_then(|l| l.captures.first())
                    {
                        Some(c) => RootState::Captured {
                            checkpoint: *c,
                            location: crate::records::Location::Local,
                        },
                        None => RootState::Missing {
                            reason: MissingReason::Unreadable,
                        },
                    },
                };
                SettingsRoot {
                    unit: u.clone(),
                    state,
                }
            })
            .collect()
    }
}

/// Whether a kind of revision needs witnesses to be valid.
pub fn needs_witness(k: RevisionKind) -> bool {
    matches!(k, RevisionKind::Auto)
}

// ------------------------------------------------------------------------- user-written records

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsWriteError {
    /// A rule is malformed, or two rules name the same folder.
    BadRule,
    /// The named computer is not a live member of the group.
    NotLive,
    /// No policy exists, so a revision has no witness.
    NoPolicy,
    /// The folder's settings are local only on every computer.
    NotShared,
    /// A revision of the unit is not readable yet: a push now could orphan it.
    HistoryIncomplete,
    /// An automatic revision needs the issuer to be the unit's authority.
    NotAuthority,
    /// The unit does not have competing head states, so there is nothing to choose between.
    NotCompeting,
    /// The heads changed since the owner was shown them: show the choice again.
    Changed,
    /// The chosen checkpoint is not one of the competing head states.
    NotAHead,
    /// The removal did not capture this unit's settings, so there is nothing to restore to it.
    NotRemovedUnit,
}

/// The causal summary of the settings policies this ledger has learned.
pub fn settings_known(l: &Ledger, group: &crate::ids::GroupId) -> crate::kinds::Known {
    let mut best: BTreeMap<crate::ids::EnrollmentId, u64> = BTreeMap::new();
    for e in l.entries() {
        let (Some(AuthorityRecord::SettingsPolicy(p)), Some(i)) = (l.record(&e.hash), &e.issuer)
        else {
            continue;
        };
        if &p.group_id == group {
            let s = best.entry(i.enrollment_id.clone()).or_insert(0);
            *s = (*s).max(i.issued_at_seq);
        }
    }
    crate::kinds::Known {
        seen: best
            .into_iter()
            .map(|(enrollment_id, seq)| crate::kinds::Seen { enrollment_id, seq })
            .collect(),
        extra: Vec::new(),
        parts: Vec::new(),
    }
}

fn live_member(
    members: &crate::ledger::Membership,
    d: &Derived,
    who: &crate::kinds::EnrollmentRef,
) -> bool {
    members.contains(&who.computer_id, &who.enrollment_id)
        && d.barrier.retired(&who.enrollment_id) == crate::authority::Retired::No
}

/// A policy naming `master` and the per-folder exceptions `rules`, covering every policy this
/// ledger has learned. It is an explicit user choice: nothing elects a master (settings.md §5.2).
/// A long rule list never refuses: the rules that do not fit inline go into parts (contracts.md
/// §1.9), returned as the exact `(hash, bytes)` files to write before the policy.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn build_policy(
    l: &Ledger,
    d: &Derived,
    members: &crate::ledger::Membership,
    group: &crate::ids::GroupId,
    issuer: crate::kinds::Issuer,
    master: Option<EnrollmentRef>,
    mut rules: Vec<PolicyRule>,
    now_unix_ms: u64,
) -> Result<(SettingsPolicy, Vec<(Sha256, Vec<u8>)>), SettingsWriteError> {
    if rules.iter().any(|r| !r.well_formed()) {
        return Err(SettingsWriteError::BadRule);
    }
    rules.sort_by(|a, b| a.folder.cmp(&b.folder));
    if rules.windows(2).any(|w| w[0].folder == w[1].folder) {
        return Err(SettingsWriteError::BadRule);
    }
    let named = master.iter().cloned().chain(rules.iter().filter_map(|r| {
        Some(EnrollmentRef {
            computer_id: r.computer_id.clone()?,
            enrollment_id: r.enrollment_id.clone()?,
        })
    }));
    if named.into_iter().any(|w| !live_member(members, d, &w)) {
        return Err(SettingsWriteError::NotLive);
    }
    let mut policy = SettingsPolicy {
        group_id: group.clone(),
        issuer,
        master,
        folders: Vec::new(),
        known: settings_known(l, group),
        folders_parts: Vec::new(),
        written_unix_ms: now_unix_ms,
    };
    let base = crate::records::encode(&policy).len();
    let packed = crate::kinds::pack_items(
        group,
        crate::kinds::PartOf::PolicyFolders,
        rules
            .into_iter()
            .map(crate::kinds::PartItem::Rule)
            .collect(),
        base,
    );
    policy.folders = packed
        .inline
        .into_iter()
        .filter_map(|i| match i {
            crate::kinds::PartItem::Rule(r) => Some(r),
            _ => None,
        })
        .collect();
    policy.folders_parts = packed.heads;
    Ok((policy, packed.parts))
}

/// The automatic revision of the unit's authority after a completed, changed capture inside its
/// epoch (settings.md §4.1). Parents are the issuer's basis, limited to nodes that exist.
#[allow(clippy::too_many_arguments)]
pub fn build_auto(
    l: &Ledger,
    d: &Derived,
    group: &crate::ids::GroupId,
    issuer: crate::kinds::Issuer,
    unit: SettingsUnit,
    checkpoint: CheckpointRef,
    basis: &[Sha256],
    now_unix_ms: u64,
) -> Result<crate::kinds::SettingsRevision, SettingsWriteError> {
    let heads = policy_heads(l, d);
    match authority_for(&heads, &unit.folder) {
        Authority::NoPolicy => return Err(SettingsWriteError::NoPolicy),
        Authority::NotShared => return Err(SettingsWriteError::NotShared),
        Authority::Holder(h) if h.enrollment_id == issuer.enrollment_id => {}
        _ => return Err(SettingsWriteError::NotAuthority),
    }
    let g = graph_from_ledger(l, d, &unit);
    if g.pending {
        return Err(SettingsWriteError::HistoryIncomplete);
    }
    let mut parents: Vec<Sha256> = basis
        .iter()
        .filter(|b| g.nodes.contains_key(b))
        .copied()
        .collect();
    parents.sort();
    parents.dedup();
    Ok(crate::kinds::SettingsRevision {
        group_id: group.clone(),
        issuer,
        unit,
        kind: RevisionKind::Auto,
        checkpoint,
        parents,
        parents_parts: Vec::new(),
        policy: witness_set(&heads),
        code_package_hash: None,
        known: settings_known(l, group),
        written_unix_ms: now_unix_ms,
    })
}

/// The explicit resolution of competing head states (settings.md §4.4): a `choice` revision of
/// `checkpoint` whose parents are every head of every head state. Refused when the unit is not
/// competing, when `shown_heads` is not exactly the heads now (a head arrived meanwhile), or
/// when `checkpoint` is not one of the competing states.
#[allow(clippy::too_many_arguments)]
pub fn build_choice(
    l: &Ledger,
    d: &Derived,
    group: &crate::ids::GroupId,
    issuer: crate::kinds::Issuer,
    unit: SettingsUnit,
    checkpoint: CheckpointRef,
    shown_heads: &[Sha256],
    now_unix_ms: u64,
) -> Result<crate::kinds::SettingsRevision, SettingsWriteError> {
    let heads = policy_heads(l, d);
    if heads.is_empty() {
        return Err(SettingsWriteError::NoPolicy);
    }
    let g = graph_from_ledger(l, d, &unit);
    if g.pending {
        return Err(SettingsWriteError::HistoryIncomplete);
    }
    if g.head_states.len() < 2 {
        return Err(SettingsWriteError::NotCompeting);
    }
    let mut shown = shown_heads.to_vec();
    shown.sort();
    shown.dedup();
    let mut now_heads = choice_parents(&g);
    now_heads.sort();
    if shown != now_heads {
        return Err(SettingsWriteError::Changed);
    }
    if !g.head_states.contains_key(&checkpoint) {
        return Err(SettingsWriteError::NotAHead);
    }
    Ok(crate::kinds::SettingsRevision {
        group_id: group.clone(),
        issuer,
        unit,
        kind: RevisionKind::Choice,
        checkpoint,
        parents: now_heads,
        parents_parts: Vec::new(),
        policy: witness_set(&heads),
        code_package_hash: None,
        known: settings_known(l, group),
        written_unix_ms: now_unix_ms,
    })
}

/// "Restore" an earlier copy of `unit` (settings.md §3.3): a `restore` revision of `checkpoint`
/// whose parents are the issuer's `basis`, limited to nodes that exist. Explicit, so it needs no
/// authority witness beyond the policy heads it was written under.
#[allow(clippy::too_many_arguments)]
pub fn build_restore(
    l: &Ledger,
    d: &Derived,
    group: &crate::ids::GroupId,
    issuer: crate::kinds::Issuer,
    unit: SettingsUnit,
    checkpoint: CheckpointRef,
    basis: &[Sha256],
    now_unix_ms: u64,
) -> Result<crate::kinds::SettingsRevision, SettingsWriteError> {
    let heads = policy_heads(l, d);
    if heads.is_empty() {
        return Err(SettingsWriteError::NoPolicy);
    }
    if authority_for(&heads, &unit.folder) == Authority::NotShared {
        return Err(SettingsWriteError::NotShared);
    }
    let g = graph_from_ledger(l, d, &unit);
    if g.pending {
        return Err(SettingsWriteError::HistoryIncomplete);
    }
    let mut parents: Vec<Sha256> = basis
        .iter()
        .filter(|b| g.nodes.contains_key(b))
        .copied()
        .collect();
    parents.sort();
    parents.dedup();
    Ok(crate::kinds::SettingsRevision {
        group_id: group.clone(),
        issuer,
        unit,
        kind: RevisionKind::Restore,
        checkpoint,
        parents,
        parents_parts: Vec::new(),
        policy: witness_set(&heads),
        code_package_hash: None,
        known: settings_known(l, group),
        written_unix_ms: now_unix_ms,
    })
}

/// The checkpoint a removal captured for `unit`: settings return only to the same unit, and only
/// when the removal holds a captured state of it (settings.md §8).
pub fn removed_unit_checkpoint(
    removal: &crate::kinds::Removal,
    unit: &SettingsUnit,
) -> Result<CheckpointRef, SettingsWriteError> {
    match &removal.settings {
        crate::kinds::RemovalSettings::Checkpoints { roots } => roots
            .iter()
            .find_map(|r| match (&r.unit == unit, &r.state) {
                (true, RootState::Captured { checkpoint, .. }) => Some(*checkpoint),
                _ => None,
            })
            .ok_or(SettingsWriteError::NotRemovedUnit),
        crate::kinds::RemovalSettings::None => Err(SettingsWriteError::NotRemovedUnit),
    }
}

/// An explicit "Share my settings" revision for `unit` (settings.md §4.1): any computer may
/// push, whether or not it is the authority. Its parents are every head of the unit's graph.
pub fn build_push(
    l: &Ledger,
    d: &Derived,
    group: &crate::ids::GroupId,
    issuer: crate::kinds::Issuer,
    unit: SettingsUnit,
    checkpoint: CheckpointRef,
    now_unix_ms: u64,
) -> Result<crate::kinds::SettingsRevision, SettingsWriteError> {
    let heads = policy_heads(l, d);
    if heads.is_empty() {
        return Err(SettingsWriteError::NoPolicy);
    }
    if authority_for(&heads, &unit.folder) == Authority::NotShared {
        return Err(SettingsWriteError::NotShared);
    }
    let g = graph_from_ledger(l, d, &unit);
    if g.pending {
        return Err(SettingsWriteError::HistoryIncomplete);
    }
    Ok(crate::kinds::SettingsRevision {
        group_id: group.clone(),
        issuer,
        unit,
        kind: RevisionKind::Push,
        checkpoint,
        parents: g.heads.clone(),
        parents_parts: Vec::new(),
        policy: witness_set(&heads),
        code_package_hash: None,
        known: settings_known(l, group),
        written_unix_ms: now_unix_ms,
    })
}
