//! The settings cycle bodies that are independent of the service skeleton (settings.md §10.2):
//! observing the scope on disk, capturing a unit into the local store, and composing a settings
//! apply for the existing executor. Bytes are only hashed and copied, never parsed or executed.

#![allow(dead_code)] // called by the cycle in a later step; the unit tests exercise it now

use std::collections::BTreeSet;
use std::path::Path;

use crate::decisions::{write_own_record, Plane};
use wow_sync_adapters::authority_store;
use wow_sync_adapters::fs_safe::WtfScope;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_adapters::settings_store::SettingsStore;
use wow_sync_adapters::sysinfo::{now_unix_ms, random_id_hex};
use wow_sync_adapters::wtf::scan_wtf;
use wow_sync_adapters::FsError;
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{CheckpointRef, GroupId, OpId, SettingsUnit, Sha256};
use wow_sync_core::kinds::{EnrollmentRef, Issuer};
use wow_sync_core::layout;
use wow_sync_core::ledger::derive;
use wow_sync_core::plan::{ExpectedLive, PackageApply};
use wow_sync_core::records::{
    decode, encode, CheckpointFile, LocalUnitState, Location, OpKind, RecoverySlot, RootState,
    SettingsCheckpointManifest, SettingsFileOp, SettingsLocal, SettingsRoot,
};
use wow_sync_core::settings::{
    settings_scope, DeclaringFolder, OwnedFile, SettingsScope, WtfEntry,
};

#[derive(Debug)]
pub enum CaptureError {
    Fs(FsError),
    /// The file changed between the scan and the copy: the capture is suspect, nothing is stored.
    Changed,
    Manifest(String),
}

impl From<FsError> for CaptureError {
    fn from(e: FsError) -> Self {
        Self::Fs(e)
    }
}

/// Step 3s: the settings scope of one variant. Nothing outside the two shapes is read.
pub fn observe_scope(
    variant_dir: &Path,
    decls: &[DeclaringFolder],
    included: &BTreeSet<String>,
    size_cap: Option<u64>,
) -> Result<SettingsScope, FsError> {
    let files = scan_wtf(variant_dir, included, size_cap)?;
    let entries: Vec<WtfEntry> = files
        .into_iter()
        .map(|f| WtfEntry {
            rel: f.rel,
            size: f.size,
            sha256: Sha256(f.sha256),
            unsafe_path: f.unsafe_path,
        })
        .collect();
    let nfc_included: BTreeSet<String> = included
        .iter()
        .map(|s| wow_sync_core::ids::nfc(s))
        .collect();
    Ok(settings_scope(decls, &entries, &nfc_included, size_cap))
}

/// Stores the unit's file as a blob and a one-file manifest; the manifest is written only after
/// the blob, so a failure never leaves a manifest without its bytes. The checkpoint reference is
/// the manifest's hash, so identical captures are not stored twice.
pub fn capture_unit(
    variant_dir: &Path,
    store: &SettingsStore,
    owned: &OwnedFile,
    size_cap: u64,
) -> Result<CheckpointRef, CaptureError> {
    let scope = WtfScope::open(variant_dir, false)?
        .ok_or_else(|| CaptureError::Manifest("WTF is missing".into()))?;
    let root = scope.root();
    let mut file = root.open_read(owned.rel.as_str(), size_cap)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut bytes)
        .map_err(|e| FsError::from_io(&root.path().join(owned.rel.as_str()), &e))?;
    if sha256(&bytes) != owned.sha256 || bytes.len() as u64 != owned.size {
        return Err(CaptureError::Changed);
    }
    let blob = store.put_blob(&bytes)?;
    let name = owned
        .rel
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    let manifest = SettingsCheckpointManifest {
        unit: owned.unit.clone(),
        files: vec![CheckpointFile {
            name,
            size: owned.size,
            blob: Sha256(blob),
        }],
    };
    let encoded = encode(&manifest);
    let h = store.put_manifest(&encoded, &[blob])?;
    Ok(Sha256(h))
}

/// A settings-only apply for the existing executor: one file, `expected_live` from the scan, the
/// blob to stage (`None` removes the live file) and one recovery slot. The checkpoint is the root.
pub fn plan_settings_apply(
    op: OpId,
    owned_now: Option<&OwnedFile>,
    unit: SettingsUnit,
    rel: wow_sync_core::paths::SettingsPath,
    staged_blob: Option<Sha256>,
    checkpoint: CheckpointRef,
    explanation: wow_sync_core::explain::Explanation,
) -> PackageApply {
    let mut p =
        PackageApply::code_apply(op, unit.folder.clone(), checkpoint, Vec::new(), explanation);
    p.kind = OpKind::Settings;
    p.settings_ops = vec![SettingsFileOp {
        unit: unit.clone(),
        live_rel: rel,
        expected_live: match owned_now {
            Some(o) => ExpectedLive::Tree {
                tree: o.sha256,
                fingerprint: o.sha256,
            },
            None => ExpectedLive::Absent,
        },
        staged: staged_blob,
        recovery_slot: RecoverySlot::new("settings/0").expect("fixed slot name"),
    }];
    p.settings_roots = vec![SettingsRoot {
        unit,
        state: RootState::Captured {
            checkpoint,
            location: Location::Local,
        },
    }];
    p
}

/// What the cycle remembers about one unit beyond `SettingsLocal`: whether the authority epoch is
/// clean, the sequence of the last capture, and why that capture was taken (settings.md §5).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnitEpoch {
    pub unit: SettingsUnit,
    pub epoch_clean: bool,
    pub capture_seq: u64,
    pub capture_kind: Option<wow_sync_core::settings::CaptureKind>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct EpochFile {
    units: Vec<UnitEpoch>,
}

fn epoch_name(group: &wow_sync_core::ids::GroupId) -> String {
    format!("settings-epoch-{group}.json")
}

/// Persisted in local state so a restart keeps an experiment held rather than publishing it.
pub struct EpochStore<'a> {
    state: &'a wow_sync_adapters::localstate::LocalState,
    group: wow_sync_core::ids::GroupId,
}

impl<'a> EpochStore<'a> {
    pub fn new(
        state: &'a wow_sync_adapters::localstate::LocalState,
        group: wow_sync_core::ids::GroupId,
    ) -> Self {
        Self { state, group }
    }

    /// Missing or damaged state means no unit is known clean: nothing publishes until a fresh
    /// activation capture proves it.
    pub fn load(&self) -> Vec<UnitEpoch> {
        let file: EpochFile = self
            .state
            .read(&epoch_name(&self.group), 4 * 1024 * 1024)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut seen = BTreeSet::new();
        file.units
            .into_iter()
            .filter(|u| seen.insert(u.unit.clone()))
            .collect()
    }

    pub fn save(&self, units: &[UnitEpoch]) -> Result<(), FsError> {
        let bytes = serde_json::to_vec(&EpochFile {
            units: units.to_vec(),
        })
        .expect("plain data serializes");
        self.state.write_atomic(&epoch_name(&self.group), &bytes)
    }
}

/// What a peer's captured checkpoint of one settings file is made of (single-file units).
pub(crate) fn manifest_blob(
    provider: &ProviderRoot,
    checkpoint: &CheckpointRef,
    unit: &SettingsUnit,
) -> Option<Sha256> {
    let bytes = provider
        .read(&layout::settings_manifest(checkpoint), 64 * 1024)
        .ok()?;
    if sha256(&bytes) != *checkpoint {
        return None;
    }
    let m: SettingsCheckpointManifest = decode(&bytes, 64 * 1024).ok()?;
    if &m.unit != unit {
        return None;
    }
    let blob = m.files.first()?.blob;
    provider
        .open_read(&layout::settings_blob(&blob), BLOB_LIMIT)
        .ok()?;
    Some(blob)
}

/// Copies the checkpoint (blob first, then manifest) to the shared folder, write-if-absent.
pub(crate) fn upload_checkpoint(
    provider: &ProviderRoot,
    store: &SettingsStore,
    checkpoint: &CheckpointRef,
    me: &str,
) -> Result<(), String> {
    let manifest = store
        .get_manifest(&checkpoint.0)
        .map_err(|e| e.to_string())?
        .ok_or("the checkpoint is not in the local store")?;
    let m: SettingsCheckpointManifest = decode(&manifest, 64 * 1024).map_err(|e| e.to_string())?;
    for f in &m.files {
        let blob = store
            .get_blob(&f.blob.0, BLOB_LIMIT)
            .map_err(|e| e.to_string())?
            .ok_or("the blob is not in the local store")?;
        authority_store::put_record(
            provider,
            &layout::settings_blob(&f.blob),
            &blob,
            BLOB_LIMIT,
            me,
        )
        .map_err(|e| e.to_string())?;
    }
    authority_store::put_record(
        provider,
        &layout::settings_manifest(checkpoint),
        &manifest,
        64 * 1024,
        me,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

const BLOB_LIMIT: u64 = 256 * 1024 * 1024;
const LOCAL_LIMIT: usize = 16 * 1024 * 1024;

fn local_name(group: &GroupId) -> String {
    format!("settings-local-{group}.json")
}

/// This computer's settings sync consent, included accounts and per-unit bookkeeping. Damaged
/// or missing state means no consent: nothing is read or applied until the owner confirms again.
pub fn load_local(state: &LocalState, group: &GroupId) -> SettingsLocal {
    state
        .read(&local_name(group), LOCAL_LIMIT as u64)
        .ok()
        .flatten()
        .and_then(|b| decode::<SettingsLocal>(&b, LOCAL_LIMIT).ok())
        .unwrap_or(SettingsLocal {
            units: Vec::new(),
            paused_folders: Vec::new(),
            included_accounts: Vec::new(),
            consent: false,
            first_adoption: None,
        })
}

pub fn save_local(state: &LocalState, group: &GroupId, l: &SettingsLocal) -> Result<(), FsError> {
    state.write_atomic(&local_name(group), &encode(l))
}

/// One paused folder with what a resume would meet (settings.md resume sheet inputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedFolder {
    pub folder: wow_sync_core::paths::FolderKey,
    /// This computer's saved settings differ from the last shared state it applied.
    pub here_changed: bool,
    /// The shared settings moved on to a revision this computer has not applied.
    pub shared_newer: bool,
}

/// The settings facts the GUI needs: consent, accounts, who is master and what is paused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub enabled: bool,
    pub accounts: Vec<String>,
    /// The single policy's master; `None` when there is no policy, no master or a conflict.
    pub master: Option<EnrollmentRef>,
    pub master_name: Option<String>,
    pub exceptions: Vec<wow_sync_core::kinds::PolicyRule>,
    /// Several competing policies exist: nothing is chosen for the owner.
    pub policy_conflict: bool,
    pub paused: Vec<PausedFolder>,
    /// `WTF\Account` folders this computer has that consent could include (V-05). Link or
    /// reparse-point entries are not offered; nothing under them is read.
    pub available_accounts: Vec<String>,
}

/// The account folder names under `<variant>\WTF\Account`, sorted. A missing `WTF` or `Account`
/// folder, or an unreadable one, gives an empty list.
pub fn available_accounts(variant_dir: &std::path::Path) -> Vec<String> {
    use wow_sync_adapters::fs_safe::{ReparseKind, WtfScope};
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return Vec::new();
    };
    let Ok(entries) = scope.root().list("Account") else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .into_iter()
        .filter(|e| e.is_dir && e.reparse == ReparseKind::None)
        .map(|e| e.name)
        .collect();
    names.sort();
    names
}

/// Account, realm and character folder names under `<variant>\WTF\Account`, each sorted and
/// deduplicated, for the diagnostics second-defence scan (ops.md §5.3). `SavedVariables` is
/// not a realm. Unreadable or linked folders are skipped.
pub fn wtf_names(variant_dir: &std::path::Path) -> (Vec<String>, Vec<String>, Vec<String>) {
    use wow_sync_adapters::fs_safe::{ReparseKind, WtfScope};
    let accounts = available_accounts(variant_dir);
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return (accounts, Vec::new(), Vec::new());
    };
    let dirs = |rel: &str| -> Vec<String> {
        scope
            .root()
            .list(rel)
            .map(|v| {
                v.into_iter()
                    .filter(|e| e.is_dir && e.reparse == ReparseKind::None)
                    .map(|e| e.name)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (mut realms, mut chars) = (Vec::new(), Vec::new());
    for a in &accounts {
        for r in dirs(&format!("Account/{a}"))
            .into_iter()
            .filter(|r| !r.eq_ignore_ascii_case("SavedVariables"))
        {
            chars.extend(dirs(&format!("Account/{a}/{r}")));
            realms.push(r);
        }
    }
    for v in [&mut realms, &mut chars] {
        v.sort();
        v.dedup();
    }
    (accounts, realms, chars)
}

pub fn settings_snapshot(
    local: &SettingsLocal,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &std::collections::BTreeMap<wow_sync_core::ids::EnrollmentId, String>,
) -> SettingsSnapshot {
    let heads = wow_sync_core::settings::policy_heads(ledger, derived);
    let (master, exceptions, policy_conflict) = match heads.as_slice() {
        [one] => (one.master.clone(), one.rules.clone(), false),
        [] => (None, Vec::new(), false),
        _ => (None, Vec::new(), true),
    };
    let mut paused: Vec<PausedFolder> = Vec::new();
    for folder in &local.paused_folders {
        let (mut here_changed, mut shared_newer) = (false, false);
        for u in local.units.iter().filter(|u| &u.unit.folder == folder) {
            let graph = wow_sync_core::settings::graph_from_ledger(ledger, derived, &u.unit);
            let latest = u.captures.first();
            here_changed |= latest.is_some()
                && latest != u.last_applied.as_ref()
                && (u.last_applied.is_some() || !graph.heads.is_empty());
            shared_newer |= !graph.pending
                && graph.head_states.len() == 1
                && graph.heads.iter().any(|h| !u.basis.contains(h));
        }
        paused.push(PausedFolder {
            folder: folder.clone(),
            here_changed,
            shared_newer,
        });
    }
    SettingsSnapshot {
        enabled: local.consent,
        accounts: local.included_accounts.clone(),
        master_name: master
            .as_ref()
            .and_then(|m| names.get(&m.enrollment_id).cloned()),
        master,
        exceptions,
        policy_conflict,
        paused,
        available_accounts: Vec::new(),
    }
}

fn use_shared_name(group: &GroupId) -> String {
    format!("settings-use-shared-{group}.json")
}

/// The units the owner asked to "Use shared settings" for, until a cycle acts on them.
pub fn load_use_shared(state: &LocalState, group: &GroupId) -> Vec<SettingsUnit> {
    state
        .read(&use_shared_name(group), LOCAL_LIMIT as u64)
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_use_shared(
    state: &LocalState,
    group: &GroupId,
    units: &[SettingsUnit],
) -> Result<(), FsError> {
    if units.is_empty() {
        return state.remove(&use_shared_name(group));
    }
    state.write_atomic(
        &use_shared_name(group),
        &serde_json::to_vec(units).expect("plain data serializes"),
    )
}

/// The folders whose selected `.toc` declares saved variables (metadata only, never Lua).
pub fn declaring_folders(scans: &[wow_sync_core::inventory::FolderScan]) -> Vec<DeclaringFolder> {
    use wow_sync_core::toc::{select_toc, TocSelection};
    use wow_sync_core::variant::TocMapping;
    let mut out = Vec::new();
    for s in scans {
        if s.status != wow_sync_core::inventory::ScanStatus::Ok {
            continue;
        }
        let Ok(folder) = wow_sync_core::paths::validate_folder_name(&s.name) else {
            continue;
        };
        if let TocSelection::Selected { info, .. } =
            select_toc(&s.name, &s.tocs, &TocMapping::ValidatedPlain)
        {
            out.push(DeclaringFolder {
                folder,
                saved_variables: info.saved_variables,
                saved_variables_per_character: info.saved_variables_per_character,
                toc_ambiguous: false,
            });
        }
    }
    out
}

/// What one settings cycle needs from the service. `cap` is the approved saved-file size; with no
/// approved value (`None`) nothing is read, captured, published or applied (Q-77).
pub(crate) struct StepArgs<'a> {
    pub cap: Option<u64>,
    pub state: &'a LocalState,
    pub provider: &'a ProviderRoot,
    pub variant_dir: &'a Path,
    pub group: &'a GroupId,
    pub own: &'a EnrollmentRef,
    pub inventory: &'a wow_sync_core::inventory::Inventory,
    pub decls: &'a [DeclaringFolder],
    pub game_blocks: bool,
}

#[derive(Debug, Default)]
pub(crate) struct StepOutcome {
    pub applies: Vec<PackageApply>,
    pub notes: Vec<String>,
    /// Revisions this cycle wrote, in order.
    pub published: Vec<Sha256>,
}

fn unit_rel(unit: &SettingsUnit, name: &str) -> Option<wow_sync_core::paths::SettingsPath> {
    use wow_sync_core::paths::{account_settings_path, character_settings_path};
    match unit.identity.realm_character() {
        None => account_settings_path(unit.identity.account_name(), name).ok(),
        Some((realm, ch)) => {
            character_settings_path(unit.identity.account_name(), realm, ch, name).ok()
        }
    }
}

/// Steps 3s, 5s and 7s for every unit this computer may touch (settings.md §10.3): capture the
/// owned files, publish the authority's changes inside its epoch, and plan the settings applies
/// the target asks for. Returns the operations; every ledger write is already persisted.
pub(crate) fn settings_cycle(plane: &mut Plane, a: &StepArgs, next_seq: &mut u64) -> StepOutcome {
    use wow_sync_core::settings::{
        authority_for, build_auto, graph_from_ledger, policy_heads, publish_decision, target_case,
        Authority, CaptureKind, Case, Publish, PublishInput, TargetInput,
    };
    let mut out = StepOutcome::default();
    let Some(cap) = a.cap else {
        return out;
    };
    if a.game_blocks {
        return out;
    }
    let mut local = load_local(a.state, a.group);
    if !local.consent {
        return out;
    }
    let included: BTreeSet<String> = local.included_accounts.iter().cloned().collect();
    let derived = derive(&plane.ledger, a.group, &plane.members);
    let heads = policy_heads(&plane.ledger, &derived);
    if heads.is_empty() {
        return out;
    }
    let Ok(scope) = observe_scope(a.variant_dir, a.decls, &included, Some(cap)) else {
        out.notes.push("settings could not be read".into());
        return out;
    };
    let Ok(store) = SettingsStore::open(a.variant_dir) else {
        out.notes
            .push("the settings store could not be opened".into());
        return out;
    };
    let epochs = EpochStore::new(a.state, a.group.clone());
    let mut eps = epochs.load();
    let mut units: BTreeSet<SettingsUnit> = scope.owned.iter().map(|o| o.unit.clone()).collect();
    for e in plane.ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = plane.ledger.record(&e.hash) {
            if included.contains(r.unit.identity.account_name()) {
                units.insert(r.unit.clone());
            }
        }
    }
    let mut written = false;
    let mut dirty = false;
    let mut use_shared = load_use_shared(a.state, a.group);
    let mut requests_changed = false;
    for unit in units {
        if scope.blocked.contains(&unit) {
            continue;
        }
        let Some(name) = a
            .inventory
            .folders
            .iter()
            .find(|f| f.key == unit.folder)
            .map(|f| f.name.clone())
        else {
            continue;
        };
        let Some(rel) = unit_rel(&unit, &name) else {
            continue;
        };
        let owned = scope.owned.iter().find(|o| o.unit == unit);
        let live = match owned {
            Some(o) => match capture_unit(a.variant_dir, &store, o, cap) {
                Ok(c) => Some(c),
                Err(_) => {
                    out.notes
                        .push(format!("{name}: settings changed while being read"));
                    continue;
                }
            },
            None => None,
        };
        let li = match local.units.iter().position(|u| u.unit == unit) {
            Some(i) => i,
            None => {
                local.units.push(LocalUnitState {
                    unit: unit.clone(),
                    captures: Vec::new(),
                    basis: Vec::new(),
                    last_applied: None,
                    pinned: false,
                    authority_since: None,
                });
                local.units.len() - 1
            }
        };
        let ei = match eps.iter().position(|u| u.unit == unit) {
            Some(i) => i,
            None => {
                eps.push(UnitEpoch {
                    unit: unit.clone(),
                    epoch_clean: false,
                    capture_seq: 0,
                    capture_kind: None,
                });
                eps.len() - 1
            }
        };
        let graph = graph_from_ledger(&plane.ledger, &derived, &unit);
        let authority = authority_for(&heads, &unit.folder);
        let is_authority = matches!(&authority, Authority::Holder(h) if h == a.own);
        let paused = local.paused_folders.contains(&unit.folder);
        dirty = true;

        // Becoming the authority starts a new epoch: the activation capture never publishes.
        if is_authority && local.units[li].authority_since.is_none() {
            let since = eps[ei].capture_seq;
            local.units[li].authority_since = Some(since);
            eps[ei].capture_seq = since + 1;
            eps[ei].capture_kind = Some(CaptureKind::Activation);
            eps[ei].epoch_clean = wow_sync_core::settings::epoch_clean(
                live,
                local.units[li].last_applied,
                !graph.nodes.is_empty(),
            );
            if let Some(l) = live {
                local.units[li].captures.insert(0, l);
            }
            continue;
        }
        if !is_authority && local.units[li].authority_since.is_some() {
            local.units[li].authority_since = None;
            eps[ei].epoch_clean = false;
        }

        let previous_seq = eps[ei].capture_seq;
        let changed = live.is_some() && live != local.units[li].captures.first().copied();
        if changed {
            eps[ei].capture_seq += 1;
            eps[ei].capture_kind = Some(CaptureKind::SessionEnd);
            if let Some(l) = live {
                local.units[li].captures.insert(0, l);
                local.units[li].captures.truncate(8);
            }
        }
        let case = target_case(&TargetInput {
            graph: &graph,
            authority: &authority,
            is_authority,
            paused,
            basis: &local.units[li].basis,
            basis_checkpoint: local.units[li].last_applied,
            live,
            epoch_clean: eps[ei].epoch_clean,
        });
        // An explicit "Use shared settings" turns a held local difference into a yield; every
        // other case ignores it, and the request is spent once the unit was evaluated.
        let case = match (&case, use_shared.contains(&unit), graph.head_states.len()) {
            (Case::LocalDifferent(_), true, 1) => Case::Yield {
                to: *graph.head_states.keys().next().expect("one head state"),
                capture_first: true,
            },
            _ => case,
        };
        if !paused && use_shared.contains(&unit) {
            use_shared.retain(|u| u != &unit);
            requests_changed = true;
        }
        match case {
            Case::InSync { basis } => {
                local.units[li].basis = basis;
                local.units[li].last_applied = live;
            }
            Case::NoSharedCopy | Case::LocalDifferent(_) if is_authority && changed => {
                let decision = publish_decision(&PublishInput {
                    authority: authority.clone(),
                    own: a.own.clone(),
                    paused,
                    complete: true,
                    kind: CaptureKind::SessionEnd,
                    changed_in_session: true,
                    epoch_clean: eps[ei].epoch_clean,
                    authority_since: local.units[li].authority_since,
                    compared_with_local_seq: previous_seq,
                });
                let Some(cp) = live.filter(|_| decision == Publish::Auto) else {
                    continue;
                };
                let rec = match build_auto(
                    &plane.ledger,
                    &derived,
                    a.group,
                    Issuer {
                        computer_id: a.own.computer_id.clone(),
                        enrollment_id: a.own.enrollment_id.clone(),
                        issued_at_seq: *next_seq,
                    },
                    unit.clone(),
                    cp,
                    &local.units[li].basis,
                    now_unix_ms() as u64,
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        out.notes.push(format!("{name}: not shared ({e:?})"));
                        continue;
                    }
                };
                if let Err(e) =
                    upload_checkpoint(a.provider, &store, &cp, a.own.computer_id.as_str())
                {
                    out.notes.push(format!("{name}: not shared ({e})"));
                    continue;
                }
                match write_own_record(
                    plane,
                    a.provider,
                    a.group,
                    &a.own.computer_id,
                    &AuthorityRecord::SettingsRevision(rec),
                ) {
                    Ok(h) => {
                        *next_seq += 1;
                        written = true;
                        out.published.push(h);
                        local.units[li].basis = vec![h];
                        local.units[li].last_applied = Some(cp);
                    }
                    Err(e) => out.notes.push(format!("{name}: not shared ({e})")),
                }
            }
            Case::FirstAdoption(to) if live == Some(to) => {
                local.units[li].basis = graph.heads.clone();
                local.units[li].last_applied = live;
            }
            Case::FirstAdoption(to) | Case::Yield { to, .. } => {
                let Some(blob) = manifest_blob(a.provider, &to, &unit) else {
                    out.notes
                        .push(format!("{name}: shared settings are not available yet"));
                    continue;
                };
                let op = OpId::parse(&random_id_hex()).expect("random id");
                out.applies.push(plan_settings_apply(
                    op,
                    owned,
                    unit.clone(),
                    rel,
                    Some(blob),
                    to,
                    wow_sync_core::explain::Explanation::Applied {
                        package: format!("{name} settings"),
                    },
                ));
            }
            _ => {}
        }
    }
    if requests_changed {
        let _ = save_use_shared(a.state, a.group, &use_shared);
    }
    if dirty {
        let _ = save_local(a.state, a.group, &local);
        let _ = epochs.save(&eps);
    }
    if written {
        let _ = plane.save(a.state);
    }
    out
}

/// One revision of a unit's settings history (the `SettingsHistory` query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsHistoryEntry {
    pub hash: wow_sync_core::ids::Sha256,
    pub kind: wow_sync_core::kinds::RevisionKind,
    pub checkpoint: wow_sync_core::ids::CheckpointRef,
    pub issuer: wow_sync_core::kinds::Issuer,
    pub written_unix_ms: u64,
    /// Currently a head of the unit's graph: nothing newer builds on it.
    pub is_head: bool,
}

/// The valid revisions of `unit`, oldest first. Invalid and unreadable ones are not history.
pub fn settings_history(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    unit: &SettingsUnit,
) -> Vec<SettingsHistoryEntry> {
    use wow_sync_core::authority::AuthorityRecord;
    let graph = wow_sync_core::settings::graph_from_ledger(ledger, derived, unit);
    let mut out: Vec<SettingsHistoryEntry> = graph
        .nodes
        .keys()
        .filter_map(|h| match ledger.record(h) {
            Some(AuthorityRecord::SettingsRevision(r)) => Some(SettingsHistoryEntry {
                hash: *h,
                kind: r.kind,
                checkpoint: r.checkpoint,
                issuer: r.issuer.clone(),
                written_unix_ms: r.written_unix_ms,
                is_head: graph.heads.contains(h),
            }),
            _ => None,
        })
        .collect();
    out.sort_by_key(|e| (e.written_unix_ms, e.hash));
    out
}

/// Why a unit's settings are or are not syncing here, most blocking reason first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsUnitStatus {
    /// This computer is retired: it records no settings intent.
    Retired,
    /// No consent on this computer (or no approved size cap, Q-77): nothing is read or applied.
    Disabled,
    /// A policy exception keeps this folder's settings on each computer.
    NotShared,
    /// Competing policies disagree about this folder; nothing is chosen for the owner.
    PolicyConflict,
    Paused,
    /// Competing shared revisions exist; the owner picks one (`Command::ChooseSettings`).
    NeedsChoice,
    Active,
}

/// One of the competing shared states Choose offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadChoice {
    /// The value `Command::ChooseSettings { checkpoint }` takes.
    pub checkpoint: wow_sync_core::ids::CheckpointRef,
    /// The revisions currently at this state (equal heads are one state).
    pub revisions: Vec<wow_sync_core::ids::Sha256>,
}

/// Everything the Settings screen needs for one unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitRow {
    pub unit: SettingsUnit,
    pub status: SettingsUnitStatus,
    /// Settings of this unit are being synced here right now.
    pub live: bool,
    /// Empty unless the unit needs a choice and this computer may make it.
    pub competing: Vec<HeadChoice>,
    /// The value `Command::ChooseSettings { shown_heads }` takes: every current head revision.
    pub shown_heads: Vec<wow_sync_core::ids::Sha256>,
    /// Oldest first; the `checkpoint` of an entry is what `Command::RestoreSettings` takes.
    pub history: Vec<SettingsHistoryEntry>,
    pub restore_possible: bool,
    /// The value `Command::ShareSettings { checkpoint }` takes: this computer's latest capture.
    pub latest_capture: Option<wow_sync_core::ids::CheckpointRef>,
    pub can_share: bool,
}

pub fn settings_units(
    local: &SettingsLocal,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    retired_self: bool,
    cap_enabled: bool,
) -> Vec<SettingsUnitRow> {
    use wow_sync_core::authority::AuthorityRecord;
    use wow_sync_core::settings::{authority_for, graph_from_ledger, policy_heads, Authority};
    let mut units: std::collections::BTreeSet<SettingsUnit> =
        local.units.iter().map(|u| u.unit.clone()).collect();
    for e in ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = ledger.record(&e.hash) {
            units.insert(r.unit.clone());
        }
    }
    let heads = policy_heads(ledger, derived);
    units
        .into_iter()
        .map(|unit| {
            let authority = authority_for(&heads, &unit.folder);
            let graph = graph_from_ledger(ledger, derived, &unit);
            let history = settings_history(ledger, derived, &unit);
            let latest_capture = local
                .units
                .iter()
                .find(|u| u.unit == unit)
                .and_then(|u| u.captures.first().copied());
            let paused = local.paused_folders.contains(&unit.folder);
            let blocked = matches!(authority, Authority::NotShared | Authority::Conflict);
            let actionable =
                !retired_self && local.consent && cap_enabled && !heads.is_empty() && !blocked;
            let competing: Vec<HeadChoice> = if actionable && graph.head_states.len() > 1 {
                graph
                    .head_states
                    .iter()
                    .map(|(c, r)| HeadChoice {
                        checkpoint: *c,
                        revisions: r.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let status = if retired_self {
                SettingsUnitStatus::Retired
            } else if !local.consent || !cap_enabled {
                SettingsUnitStatus::Disabled
            } else if authority == Authority::NotShared {
                SettingsUnitStatus::NotShared
            } else if authority == Authority::Conflict {
                SettingsUnitStatus::PolicyConflict
            } else if paused {
                SettingsUnitStatus::Paused
            } else if !competing.is_empty() {
                SettingsUnitStatus::NeedsChoice
            } else {
                SettingsUnitStatus::Active
            };
            SettingsUnitRow {
                shown_heads: if competing.is_empty() {
                    Vec::new()
                } else {
                    graph.heads.clone()
                },
                live: status == SettingsUnitStatus::Active,
                restore_possible: actionable && !history.is_empty(),
                can_share: actionable && latest_capture.is_some(),
                latest_capture,
                competing,
                history,
                status,
                unit,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::explain::Explanation;
    use wow_sync_core::ids::{ComputerId, Identity};
    use wow_sync_core::layout;
    use wow_sync_core::paths::validate_folder_name;

    fn hex(c: char) -> String {
        std::iter::repeat_n(c, 32).collect()
    }

    fn put(base: &Path, rel: &str, bytes: &[u8]) {
        let p = rel.split('/').fold(base.to_path_buf(), |a, s| a.join(s));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn decls() -> Vec<DeclaringFolder> {
        vec![DeclaringFolder {
            folder: validate_folder_name("Details").unwrap(),
            saved_variables: vec!["DetailsDB".into()],
            saved_variables_per_character: vec![],
            toc_ambiguous: false,
        }]
    }

    fn included() -> BTreeSet<String> {
        ["ERIC".to_string()].into()
    }

    #[test]
    fn the_epoch_state_survives_a_restart_and_unreadable_state_is_not_clean() {
        use wow_sync_core::settings::CaptureKind;
        let t = TempDir::new();
        let state = wow_sync_adapters::localstate::LocalState::open_at(t.path()).unwrap();
        let g = wow_sync_core::ids::GroupId::parse(&hex('9')).unwrap();
        let unit = SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Details").unwrap(),
            identity: Identity::account("ERIC").unwrap(),
        };
        let units = vec![UnitEpoch {
            unit: unit.clone(),
            epoch_clean: false,
            capture_seq: 41,
            capture_kind: Some(CaptureKind::BeforeChange),
        }];
        let store = EpochStore::new(&state, g.clone());
        assert!(store.load().is_empty());
        store.save(&units).unwrap();
        assert_eq!(EpochStore::new(&state, g.clone()).load(), units);
        let other = wow_sync_core::ids::GroupId::parse(&hex('8')).unwrap();
        assert!(EpochStore::new(&state, other).load().is_empty());
        state
            .write_atomic(&epoch_name(&g), b"{\"units\": [ truncated")
            .unwrap();
        assert!(
            store.load().is_empty(),
            "damaged state claims nothing clean"
        );
    }

    #[test]
    fn the_scope_reads_only_owned_files_and_a_capture_stores_blob_then_manifest() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(
            &wtf,
            "Account/ERIC/SavedVariables/Details.lua",
            b"DetailsDB = {}",
        );
        put(&wtf, "Account/ERIC/config-cache.wtf", b"cfg");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(1024)).unwrap();
        assert_eq!(scope.owned.len(), 1);
        assert!(scope.sync_enabled);
        let store = SettingsStore::open(t.path()).unwrap();
        let cp = capture_unit(t.path(), &store, &scope.owned[0], 1024).unwrap();
        let manifest = t
            .path()
            .join(".wowsync")
            .join("settings")
            .join("manifests")
            .join(format!("{cp}.json"));
        assert!(manifest.exists());
        assert_eq!(
            layout::settings_manifest(&cp),
            format!("settings/manifests/{cp}.json")
        );
        let blob = sha256(b"DetailsDB = {}");
        let blob_path = t
            .path()
            .join(".wowsync")
            .join("settings")
            .join("blobs")
            .join(blob.to_string());
        assert_eq!(std::fs::read(&blob_path).unwrap(), b"DetailsDB = {}");
        // identical bytes give the same checkpoint and store nothing new
        let again = capture_unit(t.path(), &store, &scope.owned[0], 1024).unwrap();
        assert_eq!(again, cp);
        // the live file was never changed by capturing
        assert_eq!(
            std::fs::read(
                wtf.join("Account")
                    .join("ERIC")
                    .join("SavedVariables")
                    .join("Details.lua")
            )
            .unwrap(),
            b"DetailsDB = {}"
        );
    }

    #[test]
    fn without_a_cap_nothing_syncs_and_a_file_changed_after_the_scan_is_not_stored() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let off = observe_scope(
            t.path(),
            &decls(),
            &included(),
            wow_sync_core::records::MAX_SETTINGS_FILE_BYTES,
        )
        .unwrap();
        assert!(!off.sync_enabled);
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        put(
            &wtf,
            "Account/ERIC/SavedVariables/Details.lua",
            b"v2 changed",
        );
        let store = SettingsStore::open(t.path()).unwrap();
        let err = capture_unit(t.path(), &store, &scope.owned[0], 64).unwrap_err();
        assert!(matches!(err, CaptureError::Changed), "{err:?}");
        let blobs = t.path().join(".wowsync").join("settings").join("blobs");
        assert!(!blobs.exists() || std::fs::read_dir(blobs).unwrap().next().is_none());
    }

    #[test]
    fn a_failing_store_records_no_state() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        // a plain file where the blob directory must be makes the store write fail
        put(&t.path().join(".wowsync"), "settings/blobs", b"in the way");
        let store = SettingsStore::open(t.path()).unwrap();
        assert!(capture_unit(t.path(), &store, &scope.owned[0], 64).is_err());
        let manifests = t.path().join(".wowsync").join("settings").join("manifests");
        assert!(!manifests.exists());
    }

    #[test]
    fn the_apply_plan_is_settings_only_with_the_scanned_expectation() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        let o = &scope.owned[0];
        let target = sha256(b"v2");
        let p = plan_settings_apply(
            OpId::parse(&hex('7')).unwrap(),
            Some(o),
            o.unit.clone(),
            o.rel.clone(),
            Some(target),
            sha256(b"checkpoint"),
            Explanation::Installed {
                source: ComputerId::parse(&hex('3')).unwrap(),
            },
        );
        assert_eq!(p.kind, OpKind::Settings);
        assert!(p.folders.is_empty() && p.removes.is_empty());
        assert_eq!(p.settings_ops.len(), 1);
        assert_eq!(p.settings_ops[0].staged, Some(target));
        assert_eq!(
            p.settings_ops[0].expected_live,
            ExpectedLive::Tree {
                tree: o.sha256,
                fingerprint: o.sha256
            }
        );
        assert_eq!(
            p.settings_ops[0].unit.identity,
            Identity::account("ERIC").unwrap()
        );
        assert!(p.settings_roots[0].captured());
        let absent = plan_settings_apply(
            OpId::parse(&hex('8')).unwrap(),
            None,
            o.unit.clone(),
            o.rel.clone(),
            Some(target),
            sha256(b"checkpoint"),
            Explanation::Installed {
                source: ComputerId::parse(&hex('3')).unwrap(),
            },
        );
        assert_eq!(absent.settings_ops[0].expected_live, ExpectedLive::Absent);
    }
}
