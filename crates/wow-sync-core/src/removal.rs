//! Removal behaviour (removal.md): the merged removal, who owes what, observation of local
//! disappearances, acknowledgements and remove operations. Pure: no effects, no clocks.

use crate::authority::{covered, AuthorityRecord};
use crate::decision::{Candidate, CleanupClass, CycleCx, RemovalView};
use crate::explain::Explanation;
use crate::hash::{package_hash, sha256};
use crate::hooks::{Neutral, RemovalHooks};
use crate::ids::{GroupId, OpId, Sha256};
use crate::inventory::{FolderState, FolderStatus, Inventory};
use crate::kinds::AuthorityKind as K;
use crate::kinds::{
    Ack, AckItem, AckResult, AckSettings, EnrollmentRef, Issuer, Known, ManifestFolder, Readd,
    ReaddEvidence, Removal, RemovalEvidence, RemovalSettings, Seen,
};
use crate::ledger::{resolve_known, Body, Derived, Ledger, Membership, Status};
use crate::package::{
    compare_package, group_packages, package_version, ClientInfo, Package, PackageOrder,
};
use crate::paths::{validate_folder_name, FolderKey};
use crate::plan::{ExpectedLive, PackageApply, RemoveApply};
use crate::records::{
    encode, MissingReason, OpKind, PendingRecord, RecoverySlot, RootState, SettingsFileOp,
    SettingsRoot,
};
use crate::settings::OwnedFile;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Most items one acknowledgement carries (contracts.md §1.6).
pub const ACK_BATCH: usize = crate::kinds::MAX_ACK_ITEMS;
/// Two scans this far apart must agree before a disappearance is a removal (E4, D-21).
pub const OBSERVE_GAP_MS: u64 = 30_000;

// ---------------------------------------------------------------- ledger readers

fn is_valid(d: &Derived, h: &Sha256) -> bool {
    d.status_of(h).is_some_and(Status::is_valid)
}

/// Every valid removal, in hash order.
fn removals<'a>(l: &'a Ledger, d: &Derived) -> Vec<(Sha256, &'a Removal)> {
    l.entries()
        .filter(|e| e.kind == K::Removal && is_valid(d, &e.hash))
        .filter_map(|e| match l.record(&e.hash) {
            Some(AuthorityRecord::Removal(r)) => Some((e.hash, r)),
            _ => None,
        })
        .collect()
}

fn readds<'a>(l: &'a Ledger, d: &Derived) -> Vec<&'a Readd> {
    l.entries()
        .filter(|e| e.kind == K::Readd && is_valid(d, &e.hash))
        .filter_map(|e| match l.record(&e.hash) {
            Some(AuthorityRecord::Readd(r)) => Some(r),
            _ => None,
        })
        .collect()
}

fn acks<'a>(l: &'a Ledger, d: &Derived) -> Vec<&'a Ack> {
    l.entries()
        .filter(|e| e.kind == K::Ack && is_valid(d, &e.hash))
        .filter_map(|e| match l.record(&e.hash) {
            Some(AuthorityRecord::Ack(r)) => Some(r),
            _ => None,
        })
        .collect()
}

fn names(folders: &[ManifestFolder], x: &FolderKey) -> bool {
    folders.iter().any(|f| &f.key == x)
}

/// A re-add covers a removal when its resolved `known` includes it (contracts.md §1.8). A `known`
/// whose parts cannot be read covers nothing: the removal stays live.
fn re_covers(l: &Ledger, a: &Readd, rh: &Sha256, r: &Removal) -> bool {
    resolve_known(l, &a.known)
        .ok()
        .is_some_and(|k| covered(&k, &BTreeSet::new(), &r.issuer, rh))
}

/// The removals naming `x` that no valid re-add covers: `Live(x)`.
pub fn live_removals(l: &Ledger, d: &Derived, x: &FolderKey) -> Vec<Sha256> {
    let re: Vec<&Readd> = readds(l, d)
        .into_iter()
        .filter(|a| names(&a.folders, x))
        .collect();
    removals(l, d)
        .into_iter()
        .filter(|(_, r)| names(&r.folders, x))
        .filter(|(h, r)| !re.iter().any(|a| re_covers(l, a, h, r)))
        .map(|(h, _)| h)
        .collect()
}

/// Removals that are live for at least one folder, linked into units by shared live folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalUnit {
    pub folders: Vec<FolderKey>,
    pub removals: Vec<Sha256>,
}

pub fn removal_units(l: &Ledger, d: &Derived) -> Vec<RemovalUnit> {
    let mut groups: Vec<(BTreeSet<Sha256>, BTreeSet<FolderKey>)> = Vec::new();
    for (h, r) in removals(l, d) {
        let live: BTreeSet<FolderKey> = r
            .folders
            .iter()
            .filter(|f| live_removals(l, d, &f.key).contains(&h))
            .map(|f| f.key.clone())
            .collect();
        if live.is_empty() {
            continue;
        }
        let mut hs = BTreeSet::from([h]);
        let mut fs = live;
        let mut i = 0;
        while i < groups.len() {
            if groups[i].1.is_disjoint(&fs) {
                i += 1;
            } else {
                let (g_h, g_f) = groups.remove(i);
                hs.extend(g_h);
                fs.extend(g_f);
            }
        }
        groups.push((hs, fs));
    }
    let mut out: Vec<RemovalUnit> = groups
        .into_iter()
        .map(|(h, f)| RemovalUnit {
            folders: f.into_iter().collect(),
            removals: h.into_iter().collect(),
        })
        .collect();
    out.sort_by(|a, b| a.folders.cmp(&b.folders));
    out
}

// ---------------------------------------------------------------- obligations

fn in_audience(r: &Removal, e: &EnrollmentRef) -> bool {
    r.audience.contains(e)
}

/// The first own state generation from which `e`'s copy of `x` is a candidate again: the
/// earliest valid ack item or covering re-add of that enrollment (removal.md §3.4, R4D-3).
pub fn eligible_from(
    l: &Ledger,
    d: &Derived,
    e: &EnrollmentRef,
    r: (Sha256, &Removal),
    x: &FolderKey,
) -> Option<u64> {
    let (rh, rec) = r;
    let by_ack = acks(l, d)
        .into_iter()
        .filter(|a| a.issuer.enrollment_id == e.enrollment_id)
        .filter(|a| {
            a.items
                .iter()
                .any(|i| i.removal == rh && i.folders.contains(x))
        })
        .map(|a| a.issuer.issued_at_seq);
    let by_readd = readds(l, d)
        .into_iter()
        .filter(|a| a.issuer.enrollment_id == e.enrollment_id && names(&a.folders, x))
        .filter(|a| re_covers(l, a, &rh, rec))
        .map(|a| a.issuer.issued_at_seq);
    by_ack.chain(by_readd).min()
}

fn removal_of<'a>(l: &'a Ledger, d: &Derived, rh: &Sha256) -> Option<&'a Removal> {
    match l.record(rh) {
        Some(AuthorityRecord::Removal(r)) if is_valid(d, rh) => Some(r),
        _ => None,
    }
}

/// `e` owes removal `rh` for `x`: in the audience, names the folder, and discharged nothing yet.
pub fn owes(l: &Ledger, d: &Derived, e: &EnrollmentRef, rh: &Sha256, x: &FolderKey) -> bool {
    let Some(r) = removal_of(l, d, rh) else {
        return false;
    };
    in_audience(r, e) && names(&r.folders, x) && eligible_from(l, d, e, (*rh, r), x).is_none()
}

/// A state of `e` written at `state_seq` must not be a candidate source for `x` because of `rh`.
pub fn state_blocked(
    l: &Ledger,
    d: &Derived,
    e: &EnrollmentRef,
    rh: &Sha256,
    x: &FolderKey,
    state_seq: u64,
) -> bool {
    let Some(r) = removal_of(l, d, rh) else {
        return false;
    };
    if !in_audience(r, e) || !names(&r.folders, x) {
        return false;
    }
    match eligible_from(l, d, e, (*rh, r), x) {
        None => true,
        Some(from) => state_seq < from,
    }
}

/// Every holder of the copy still owes some removal of `f`, so the copy is a stale one.
fn candidate_is_stale(
    l: &Ledger,
    d: &Derived,
    f: &FolderKey,
    holders: &[crate::ids::ComputerId],
) -> bool {
    if holders.is_empty() {
        return false;
    }
    let rs: Vec<(Sha256, &Removal)> = removals(l, d)
        .into_iter()
        .filter(|(_, r)| names(&r.folders, f))
        .collect();
    holders.iter().all(|c| {
        rs.iter().any(|(h, r)| {
            r.audience
                .iter()
                .filter(|e| &e.computer_id == c)
                .any(|e| owes(l, d, e, h, f))
        })
    })
}

/// Removals naming `f` whose content is not readable yet and that the user has not dismissed.
fn pending_for(l: &Ledger, d: &Derived, f: &FolderKey) -> bool {
    l.entries().any(|e| {
        e.kind == K::Removal
            && e.folders.contains(f)
            && !l.dismissed().contains(&e.hash)
            && (matches!(d.status_of(&e.hash), Some(Status::Pending(_)))
                || matches!(
                    l.body(&e.hash),
                    None | Some(Body::Unreadable | Body::UnknownSchema)
                ))
    })
}

fn acked_here(l: &Ledger, d: &Derived, own: &EnrollmentRef, rh: &Sha256, x: &FolderKey) -> bool {
    acks(l, d).into_iter().any(|a| {
        a.issuer.enrollment_id == own.enrollment_id
            && a.items
                .iter()
                .any(|i| &i.removal == rh && i.folders.contains(x))
    })
}

/// Present folders that a live removal names after this computer already acknowledged every such
/// removal: the user reinstalled on purpose, so nothing removes it again unasked (§1.7).
pub fn reinstall_held(
    l: &Ledger,
    d: &Derived,
    own: &EnrollmentRef,
    inv: &Inventory,
) -> BTreeSet<FolderKey> {
    inv.folders
        .iter()
        .filter(|f| {
            let live = live_removals(l, d, &f.key);
            !live.is_empty() && live.iter().all(|h| acked_here(l, d, own, h, &f.key))
        })
        .map(|f| f.key.clone())
        .collect()
}

// ---------------------------------------------------------------- hooks

/// The removal hooks. The frozen hook signatures give no derived state or inventory, so the
/// behaviour carries them; without derived state it answers like `Neutral`.
#[derive(Debug, Clone, Default)]
pub struct RemovalBehaviour {
    derived: Option<Derived>,
    inventory: Option<Inventory>,
    next_seq: u64,
}

impl RemovalBehaviour {
    pub fn new(ledger: &Ledger, group: &GroupId, members: &Membership) -> Self {
        Self {
            derived: Some(crate::ledger::derive(ledger, group, members)),
            inventory: None,
            next_seq: 0,
        }
    }

    /// The scan that `classify_due` and `plan_removals` judge, and the own sequence number the
    /// records they plan are issued at.
    pub fn with_inventory(mut self, inventory: Inventory, next_seq: u64) -> Self {
        self.inventory = Some(inventory);
        self.next_seq = next_seq;
        self
    }
}

impl RemovalHooks for RemovalBehaviour {
    fn merged_removals(&self, l: &Ledger, f: &FolderKey) -> RemovalView {
        match &self.derived {
            None => Neutral.merged_removals(l, f),
            Some(d) => RemovalView {
                live: live_removals(l, d, f),
                pending: pending_for(l, d, f),
            },
        }
    }

    fn stale_copy(&self, l: &Ledger, f: &FolderKey, c: &Candidate) -> bool {
        match &self.derived {
            None => Neutral.stale_copy(l, f, c),
            Some(d) => candidate_is_stale(l, d, f, &c.holders),
        }
    }

    fn classify_due(&self, cx: &CycleCx) -> Vec<(AckItem, CleanupClass)> {
        due_items(cx, self.inventory.as_ref())
    }

    fn plan_removals(&self, cx: &CycleCx) -> Vec<PackageApply> {
        match &self.inventory {
            None => Vec::new(),
            Some(inv) => plan_remove_ops(cx, inv, self.next_seq),
        }
    }
}

// ---------------------------------------------------------------- acknowledgement and planning

/// Owed `(removal, folder)` pairs of this computer whose folder is not on disk, split by whether
/// owned settings must be cleaned before the acknowledgement (§5.6).
pub fn due_items(cx: &CycleCx, inv: Option<&Inventory>) -> Vec<(AckItem, CleanupClass)> {
    let Some(inv) = inv else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (h, r) in removals(cx.ledger, cx.derived) {
        if !in_audience(r, cx.own) {
            continue;
        }
        let mut wait = Vec::new();
        let mut absent = Vec::new();
        for f in &r.folders {
            if inv.get(&f.key).is_some() || !owes(cx.ledger, cx.derived, cx.own, &h, &f.key) {
                continue;
            }
            if cx
                .roots
                .iter()
                .any(|root| root.unit.folder == f.key && root.captured())
            {
                wait.push(f.key.clone());
            } else {
                absent.push(f.key.clone());
            }
        }
        for (folders, class) in [
            (wait, CleanupClass::WaitForCleanup),
            (absent, CleanupClass::AbsentAcknowledge),
        ] {
            if !folders.is_empty() {
                out.push((
                    AckItem {
                        removal: h,
                        folders,
                    },
                    class,
                ));
            }
        }
    }
    out
}

fn own_issuer(cx: &CycleCx, seq: u64) -> Issuer {
    Issuer {
        computer_id: cx.own.computer_id.clone(),
        enrollment_id: cx.own.enrollment_id.clone(),
        issued_at_seq: seq,
    }
}

/// `absent` acknowledgements for the items that need no cleanup, at most `ACK_BATCH` items each,
/// issued at `next_seq`, `next_seq + 1`, ….
pub fn absent_acks(cx: &CycleCx, items: &[(AckItem, CleanupClass)], next_seq: u64) -> Vec<Ack> {
    let mut mine: Vec<AckItem> = items
        .iter()
        .filter(|(_, c)| *c == CleanupClass::AbsentAcknowledge)
        .map(|(i, _)| i.clone())
        .collect();
    mine.sort_by_key(|i| i.removal);
    mine.chunks(ACK_BATCH)
        .enumerate()
        .map(|(n, chunk)| Ack {
            group_id: cx.group_id.clone(),
            issuer: own_issuer(cx, next_seq + n as u64),
            items: chunk.to_vec(),
            result: AckResult::Absent,
            removed: Vec::new(),
            settings: AckSettings::NotApplicable,
            written_unix_ms: cx.now_unix_ms,
        })
        .collect()
}

fn hex_prefix(h: &Sha256) -> String {
    h.0.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// One remove operation per connected set of owed removals whose folders are on disk. A folder
/// that is held, unreadable, or without a stable listing is never touched. The acknowledgement is
/// fixed here (journal-time bytes) unless owned settings must be cleaned first.
pub fn plan_remove_ops(cx: &CycleCx, inv: &Inventory, next_seq: u64) -> Vec<PackageApply> {
    plan_remove_ops_with(cx, inv, next_seq, &[])
}

/// What an acknowledgement says about settings, from the roots of its folders (removal.md §5.6).
pub fn ack_settings_for(roots: &[SettingsRoot]) -> AckSettings {
    let captured = roots.iter().filter(|r| r.captured()).count();
    if roots.is_empty() {
        AckSettings::NotApplicable
    } else if captured == roots.len() {
        AckSettings::Paired
    } else if captured > 0 {
        AckSettings::Partial
    } else if roots.iter().all(|r| {
        matches!(
            r.state,
            RootState::Missing {
                reason: MissingReason::Paused
            }
        )
    }) {
        AckSettings::KeptPaused
    } else {
        AckSettings::CodeOnly
    }
}

/// The settings files of `files` whose folder is in `keys`, as move-to-recovery operations
/// (nothing staged), each expecting exactly the scanned bytes.
fn cleanup_ops(
    files: &[OwnedFile],
    roots: &[SettingsRoot],
    keys: &BTreeSet<FolderKey>,
) -> Vec<SettingsFileOp> {
    let captured: BTreeSet<&crate::ids::SettingsUnit> = roots
        .iter()
        .filter(|r| r.captured())
        .map(|r| &r.unit)
        .collect();
    files
        .iter()
        .filter(|f| keys.contains(&f.unit.folder) && captured.contains(&f.unit))
        .enumerate()
        .map(|(idx, f)| SettingsFileOp {
            unit: f.unit.clone(),
            live_rel: f.rel.clone(),
            expected_live: ExpectedLive::Tree {
                tree: f.sha256,
                fingerprint: f.sha256,
            },
            staged: None,
            recovery_slot: RecoverySlot::new(&format!("settings/{idx}")).expect("fixed slot name"),
        })
        .collect()
}

fn pending_ack(ack: &Ack) -> PendingRecord {
    let bytes = encode(ack);
    PendingRecord {
        hash: sha256(&bytes),
        bytes: String::from_utf8(bytes).expect("records are UTF-8"),
    }
}

/// `plan_remove_ops` that also moves the captured settings files of the removed folders into
/// recovery in the same operation (removal.md §1.6 step 6). `files` are the scanned owned files;
/// only a unit whose root in `cx.roots` is captured is moved. The acknowledgement is reserved in
/// the journal and says what happened to settings.
pub fn plan_remove_ops_with(
    cx: &CycleCx,
    inv: &Inventory,
    next_seq: u64,
    files: &[OwnedFile],
) -> Vec<PackageApply> {
    if !inv.listing_stable {
        return Vec::new();
    }
    let mut groups: Vec<(BTreeSet<Sha256>, BTreeSet<FolderKey>)> = Vec::new();
    for (h, r) in removals(cx.ledger, cx.derived) {
        if !in_audience(r, cx.own) {
            continue;
        }
        let keys: BTreeSet<FolderKey> = r
            .folders
            .iter()
            .map(|f| f.key.clone())
            .filter(|k| {
                inv.get(k).is_some()
                    && !cx.held.contains(k)
                    && owes(cx.ledger, cx.derived, cx.own, &h, k)
            })
            .collect();
        if keys.is_empty() {
            continue;
        }
        let mut hs = BTreeSet::from([h]);
        let mut fs = keys;
        let mut i = 0;
        while i < groups.len() {
            if groups[i].1.is_disjoint(&fs) {
                i += 1;
            } else {
                let (g_h, g_f) = groups.remove(i);
                hs.extend(g_h);
                fs.extend(g_f);
            }
        }
        groups.push((hs, fs));
    }
    groups
        .into_iter()
        .filter_map(|(hs, keys)| remove_op(cx, inv, &hs, &keys, next_seq, files))
        .collect()
}

/// One `SettingsCleanup` operation per owed removal whose folders are already gone from disk
/// but whose captured settings are still in place (this computer was the one that uninstalled).
pub fn plan_cleanup_ops(
    cx: &CycleCx,
    inv: &Inventory,
    next_seq: u64,
    files: &[OwnedFile],
) -> Vec<PackageApply> {
    if !inv.listing_stable {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (item, class) in due_items(cx, Some(inv)) {
        if class != CleanupClass::WaitForCleanup {
            continue;
        }
        let keys: BTreeSet<FolderKey> = item.folders.iter().cloned().collect();
        let roots: Vec<SettingsRoot> = cx
            .roots
            .iter()
            .filter(|r| keys.contains(&r.unit.folder))
            .cloned()
            .collect();
        let settings_ops = cleanup_ops(files, &roots, &keys);
        if settings_ops.is_empty() {
            continue;
        }
        let ack = Ack {
            group_id: cx.group_id.clone(),
            issuer: own_issuer(cx, next_seq + out.len() as u64),
            items: vec![item.clone()],
            result: AckResult::Absent,
            removed: Vec::new(),
            settings: ack_settings_for(&roots),
            written_unix_ms: cx.now_unix_ms,
        };
        let seed = format!(
            "wowsync-cleanup-op-v1\n{}\n{}",
            hex_prefix(&item.removal),
            keys.iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        let op = OpId::parse(&sha256(seed.as_bytes()).to_string()[..32]).expect("hex");
        let Some(package) = keys.iter().next().cloned() else {
            continue;
        };
        let mut p = PackageApply::code_apply(
            op,
            package,
            sha256(b""),
            Vec::new(),
            Explanation::Installed {
                source: cx.own.computer_id.clone(),
            },
        );
        p.kind = OpKind::SettingsCleanup;
        p.settings_ops = settings_ops;
        p.pending_records = vec![pending_ack(&ack)];
        p.settings_roots = roots;
        out.push(p);
    }
    out
}

fn remove_op(
    cx: &CycleCx,
    inv: &Inventory,
    hs: &BTreeSet<Sha256>,
    keys: &BTreeSet<FolderKey>,
    next_seq: u64,
    files: &[OwnedFile],
) -> Option<PackageApply> {
    let mut states: Vec<&FolderState> = Vec::new();
    for k in keys {
        let s = inv.get(k)?;
        if s.status != FolderStatus::Ok || s.tree.is_none() || s.fingerprint.is_none() {
            return None;
        }
        states.push(s);
    }
    let removes: Vec<RemoveApply> = states
        .iter()
        .map(|s| RemoveApply {
            name: s.name.clone(),
            key: s.key.clone(),
            expected_live: ExpectedLive::Tree {
                tree: s.tree.expect("checked"),
                fingerprint: s.fingerprint.expect("checked"),
            },
        })
        .collect();
    let roots: Vec<SettingsRoot> = cx
        .roots
        .iter()
        .filter(|r| keys.contains(&r.unit.folder))
        .cloned()
        .collect();
    let items: Vec<AckItem> = hs
        .iter()
        .filter_map(|h| {
            let r = removal_of(cx.ledger, cx.derived, h)?;
            let folders: Vec<FolderKey> = r
                .folders
                .iter()
                .map(|f| f.key.clone())
                .filter(|k| keys.contains(k))
                .collect();
            (!folders.is_empty()).then_some(AckItem {
                removal: *h,
                folders,
            })
        })
        .collect();
    let pairs: Vec<(FolderKey, Sha256)> = states
        .iter()
        .map(|s| (s.key.clone(), s.tree.expect("checked")))
        .collect();
    let pending_records = {
        let ack = Ack {
            group_id: cx.group_id.clone(),
            issuer: own_issuer(cx, next_seq),
            items,
            result: AckResult::Removed,
            removed: states
                .iter()
                .map(|s| ManifestFolder {
                    key: s.key.clone(),
                    name: s.name.clone(),
                    tree: s.tree.expect("checked"),
                })
                .collect(),
            settings: ack_settings_for(&roots),
            written_unix_ms: cx.now_unix_ms,
        };
        vec![pending_ack(&ack)]
    };
    let mut seed = String::from("wowsync-remove-op-v1\n");
    for h in hs {
        seed.push_str(&hex_prefix(h));
        seed.push('\n');
    }
    for k in keys {
        seed.push_str(k.as_str());
        seed.push('\n');
    }
    let id = sha256(seed.as_bytes());
    let op = OpId::parse(&id.to_string()[..32]).expect("hex");
    Some(PackageApply {
        op,
        package: keys.iter().next()?.clone(),
        target_hash: package_hash(&pairs),
        folders: Vec::new(),
        // The explanation set has no removal variant yet; the app renders removals from `kind`.
        explanation: Explanation::Installed {
            source: cx.own.computer_id.clone(),
        },
        kind: OpKind::Remove,
        removes,
        removals: hs.iter().copied().collect(),
        settings_ops: cleanup_ops(files, &roots, keys),
        pending_records,
        settings_roots: roots,
    })
}

// ---------------------------------------------------------------- explicit actions

/// The causal summary of what this ledger has learned: per enrollment, the highest record
/// sequence known (contracts.md §1.8).
pub fn known_summary(l: &Ledger, group: &GroupId) -> Known {
    let mut best: BTreeMap<crate::ids::EnrollmentId, u64> = BTreeMap::new();
    for e in l.entries() {
        let Some(AuthorityRecord::Removal(_) | AuthorityRecord::Readd(_) | AuthorityRecord::Ack(_)) =
            l.record(&e.hash)
        else {
            continue;
        };
        if let (Some(i), Some(rec)) = (&e.issuer, l.record(&e.hash)) {
            if rec.group_id() == group {
                let s = best.entry(i.enrollment_id.clone()).or_insert(0);
                *s = (*s).max(i.issued_at_seq);
            }
        }
    }
    Known {
        seen: best
            .into_iter()
            .map(|(enrollment_id, seq)| Seen { enrollment_id, seq })
            .collect(),
        extra: Vec::new(),
        parts: Vec::new(),
    }
}

/// A re-add of exactly the folders of `removal`, covering everything this ledger has learned.
/// A removal written later, or one this ledger has not learned, is not covered and wins.
pub fn readd_for_restore(
    l: &Ledger,
    group: &GroupId,
    own: &EnrollmentRef,
    removal: &Removal,
    evidence: ReaddEvidence,
    own_seq: u64,
    now_unix_ms: u64,
) -> Readd {
    let pairs: Vec<(FolderKey, Sha256)> = removal
        .folders
        .iter()
        .map(|f| (f.key.clone(), f.tree))
        .collect();
    Readd {
        group_id: group.clone(),
        issuer: Issuer {
            computer_id: own.computer_id.clone(),
            enrollment_id: own.enrollment_id.clone(),
            issued_at_seq: own_seq,
        },
        evidence,
        rejection: None,
        title: removal.title.clone(),
        folders: removal.folders.clone(),
        present_hash: package_hash(&pairs),
        known: known_summary(l, group),
        written_unix_ms: now_unix_ms,
    }
}

/// One folder of a restore: its manifest entry's tree content, and what is live now.
#[derive(Debug, Clone)]
pub struct RestoreFolder {
    pub key: FolderKey,
    pub name: String,
    pub record: crate::records::FolderTreeRecord,
    /// `None` when the folder is absent (the normal case), else the live tree to replace.
    pub live: Option<ExpectedLive>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreError {
    /// No folders to restore.
    Empty,
    /// The staged trees do not match the removal's manifest entry for a folder.
    ManifestMismatch,
    /// The staged package hash is not the record's `removed_hash` (§7.4 step 1).
    HashMismatch,
}

/// The restore operation (removal.md §7.4 steps 1, 3, 5): install or replace exactly the removed
/// folders and reserve the `user_restore` re-add, written only when the operation is verified
/// and finalized. `folders` are the trees the source delivered; they are checked against the
/// removal record before anything is planned.
pub fn plan_restore(
    l: &Ledger,
    own: &EnrollmentRef,
    removal: &Removal,
    folders: Vec<RestoreFolder>,
    op: OpId,
    own_seq: u64,
    now_unix_ms: u64,
) -> Result<PackageApply, RestoreError> {
    if folders.is_empty() {
        return Err(RestoreError::Empty);
    }
    for f in &folders {
        let ok = removal
            .folders
            .iter()
            .any(|m| m.key == f.key && m.name == f.name && m.tree == f.record.tree);
        if !ok {
            return Err(RestoreError::ManifestMismatch);
        }
    }
    let mut pairs: Vec<(FolderKey, Sha256)> = folders
        .iter()
        .map(|f| (f.key.clone(), f.record.tree))
        .collect();
    pairs.sort();
    if package_hash(&pairs) != removal.removed_hash {
        return Err(RestoreError::HashMismatch);
    }
    let readd = readd_for_restore(
        l,
        &removal.group_id,
        own,
        removal,
        ReaddEvidence::UserRestore,
        own_seq,
        now_unix_ms,
    );
    let bytes = encode(&readd);
    let record = PendingRecord {
        hash: sha256(&bytes),
        bytes: String::from_utf8(bytes).expect("records are UTF-8"),
    };
    let mut sorted = folders;
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    let package = sorted[0].key.clone();
    let applies = sorted
        .into_iter()
        .map(|f| crate::plan::FolderApply {
            name: f.name,
            key: f.key,
            action: if f.live.is_some() {
                crate::plan::FolderAction::Replace
            } else {
                crate::plan::FolderAction::Install
            },
            tree: f.record,
            expected_live: f.live.unwrap_or(ExpectedLive::Absent),
        })
        .collect();
    let mut plan = PackageApply::code_apply(
        op,
        package,
        removal.removed_hash,
        applies,
        Explanation::AppliedHere,
    );
    plan.kind = OpKind::Restore;
    plan.pending_records = vec![record];
    Ok(plan)
}

/// Installed addons that list one of the removed folders as a hard dependency: they stay, and
/// the user is told (§5.4).
pub fn dependency_removed(inv: &Inventory, removed: &[FolderKey]) -> Vec<(String, FolderKey)> {
    let mut out = Vec::new();
    for f in &inv.folders {
        if removed.contains(&f.key) {
            continue;
        }
        for dep in &f.toc.hard_deps {
            if let Ok(k) = validate_folder_name(dep) {
                if removed.contains(&k) {
                    out.push((f.name.clone(), k));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- observation

/// The presence baseline (§1.6): per package, the folders seen whole, ok and transfer-eligible.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub variant: String,
    pub dir_identity: String,
    pub packages: BTreeMap<String, Vec<FolderState>>,
}

/// What is currently being judged: package -> (first sighting, latest sighting) of its absence,
/// plus unusable entries seen while nothing was missing (the E6 comparison).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub gone: BTreeMap<String, (u64, u64)>,
    #[serde(default)]
    pub known_unusable: BTreeSet<String>,
}

impl Baseline {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("baseline serializes")
    }
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}

impl Observation {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("observation serializes")
    }
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    RemovalObserved,
    RemovalUnclear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRemoval {
    pub package: String,
    pub folders: Vec<FolderState>,
    pub evidence: RemovalEvidence,
}

#[derive(Debug, Clone)]
pub struct Observed {
    pub baseline: Baseline,
    pub observation: Observation,
    pub removed: Vec<ObservedRemoval>,
    pub held: Vec<(String, HoldReason)>,
}

pub struct ObserveInput<'a> {
    pub inventory: &'a Inventory,
    pub dir_identity: &'a str,
    pub variant: &'a str,
    pub now_unix_ms: u64,
    /// Folders that any journal names (E5).
    pub journaled: &'a BTreeSet<FolderKey>,
}

fn fold(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn eligible(f: &FolderState) -> bool {
    f.transfer_eligible() && f.tree.is_some()
}

/// E6: a new unusable entry, or a new entry that looks like one of `folders`, makes the
/// disappearance unclear.
fn unclear(
    folders: &[FolderState],
    inv: &Inventory,
    baseline_keys: &BTreeSet<FolderKey>,
    known_unusable: &BTreeSet<String>,
) -> bool {
    let looks: BTreeSet<String> = folders.iter().map(|f| fold(&f.name)).collect();
    inv.folders
        .iter()
        .filter(|f| !baseline_keys.contains(&f.key))
        .any(|f| {
            (f.status != FolderStatus::Ok && !known_unusable.contains(f.key.as_str()))
                || looks.contains(&fold(&f.name))
        })
}

/// One pass over a stable scan (E1–E6; E8 is the baseline's transfer-eligibility). Pure.
pub fn observe(base: &Baseline, obs: &Observation, input: &ObserveInput) -> Observed {
    let inv = input.inventory;
    let same_place = base.variant == input.variant && base.dir_identity == input.dir_identity;
    let mut baseline = if same_place {
        base.clone()
    } else {
        Baseline {
            variant: input.variant.to_string(),
            dir_identity: input.dir_identity.to_string(),
            packages: BTreeMap::new(),
        }
    };
    let mut observation = if same_place {
        obs.clone()
    } else {
        Observation::default()
    };
    let mut out = Observed {
        baseline: baseline.clone(),
        observation: observation.clone(),
        removed: Vec::new(),
        held: Vec::new(),
    };
    if !inv.listing_stable {
        out.held = obs
            .gone
            .keys()
            .map(|k| (k.clone(), HoldReason::RemovalObserved))
            .collect();
        out.baseline = base.clone();
        out.observation = obs.clone();
        return out;
    }
    let baseline_keys: BTreeSet<FolderKey> = baseline
        .packages
        .values()
        .flatten()
        .map(|f| f.key.clone())
        .collect();
    let mut any_missing = false;
    let keys: Vec<String> = baseline.packages.keys().cloned().collect();
    for name in keys {
        let folders = baseline.packages[&name].clone();
        if folders.iter().any(|f| input.journaled.contains(&f.key)) {
            observation.gone.remove(&name);
            continue;
        }
        let absent: Vec<&FolderState> = folders
            .iter()
            .filter(|f| inv.get(&f.key).is_none())
            .collect();
        if absent.len() == folders.len() {
            any_missing = true;
            if unclear(&folders, inv, &baseline_keys, &observation.known_unusable) {
                observation.gone.remove(&name);
                out.held.push((name, HoldReason::RemovalUnclear));
                continue;
            }
            let first = observation
                .gone
                .get(&name)
                .map_or(input.now_unix_ms, |(first, _)| *first);
            observation
                .gone
                .insert(name.clone(), (first, input.now_unix_ms));
            if input.now_unix_ms.saturating_sub(first) >= OBSERVE_GAP_MS {
                observation.gone.remove(&name);
                baseline.packages.remove(&name);
                out.removed.push(ObservedRemoval {
                    package: name,
                    folders,
                    evidence: RemovalEvidence::Observed,
                });
            } else {
                out.held.push((name, HoldReason::RemovalObserved));
            }
            continue;
        }
        observation.gone.remove(&name);
        let now: Vec<FolderState> = folders
            .iter()
            .filter_map(|f| inv.get(&f.key).cloned())
            .collect();
        if absent.is_empty() {
            if now.iter().all(eligible) {
                baseline.packages.insert(name, now);
            }
            continue;
        }
        // Some folders left: a removal only when the rest were updated to a newer release.
        let dropped: Vec<FolderState> = absent.into_iter().cloned().collect();
        let Some(pkg_key) = validate_folder_name(&name).ok() else {
            continue;
        };
        let kept: Vec<FolderState> = folders
            .iter()
            .filter(|f| inv.get(&f.key).is_some())
            .cloned()
            .collect();
        if !now.iter().all(eligible) || !upgraded(&pkg_key, &kept, &now) {
            continue;
        }
        any_missing = true;
        if unclear(&dropped, inv, &baseline_keys, &observation.known_unusable) {
            out.held.push((name, HoldReason::RemovalUnclear));
            continue;
        }
        baseline.packages.insert(name.clone(), now);
        out.removed.push(ObservedRemoval {
            package: name,
            folders: dropped,
            evidence: RemovalEvidence::FolderDrop,
        });
    }
    if !any_missing {
        observation.known_unusable.extend(
            inv.folders
                .iter()
                .filter(|f| f.status != FolderStatus::Ok)
                .map(|f| f.key.as_str().to_string()),
        );
    }
    let covered_keys: BTreeSet<FolderKey> = baseline
        .packages
        .values()
        .flatten()
        .map(|f| f.key.clone())
        .chain(baseline_keys)
        .collect();
    for pkg in group_packages(&[inv]) {
        if pkg.members.iter().any(|k| covered_keys.contains(k)) {
            continue;
        }
        let states: Vec<FolderState> = pkg
            .members
            .iter()
            .filter_map(|k| inv.get(k).cloned())
            .collect();
        if states.len() == pkg.members.len()
            && states.iter().all(eligible)
            && !pkg.members.iter().any(|k| input.journaled.contains(k))
        {
            baseline
                .packages
                .insert(pkg.key.as_str().to_string(), states);
        }
    }
    out.baseline = baseline;
    out.observation = observation;
    out
}

/// The remaining folders changed and the new version is greater than the old one over them.
fn upgraded(pkg: &FolderKey, before: &[FolderState], after: &[FolderState]) -> bool {
    if before
        .iter()
        .all(|b| after.iter().any(|a| a.key == b.key && a.tree == b.tree))
    {
        return false;
    }
    let package = Package {
        key: pkg.clone(),
        display: pkg.clone(),
        members: before.iter().map(|f| f.key.clone()).collect(),
        edges: Vec::new(),
    };
    let client = ClientInfo {
        build_version: None,
        toc_mapping: String::new(),
    };
    let old = Inventory {
        folders: before.to_vec(),
        listing_stable: true,
    };
    let new = Inventory {
        folders: after.to_vec(),
        listing_stable: true,
    };
    match (
        package_version(&package, &new, &client),
        package_version(&package, &old, &client),
    ) {
        (Some(n), Some(o)) => matches!(compare_package(&n, &o), PackageOrder::Greater { .. }),
        _ => false,
    }
}

/// The removal record for an observed or explicit removal. The audience is sorted and unique.
pub fn removal_record(
    group: &GroupId,
    own: &EnrollmentRef,
    audience: Vec<EnrollmentRef>,
    removed: &ObservedRemoval,
    own_seq: u64,
    now_unix_ms: u64,
) -> Removal {
    removal_record_with_settings(
        group,
        own,
        audience,
        removed,
        own_seq,
        now_unix_ms,
        Vec::new(),
    )
}

/// `removal_record` carrying the settings roots captured for the removed folders. Roots are
/// sorted by unit; none means `RemovalSettings::None`.
pub fn removal_record_with_settings(
    group: &GroupId,
    own: &EnrollmentRef,
    mut audience: Vec<EnrollmentRef>,
    removed: &ObservedRemoval,
    own_seq: u64,
    now_unix_ms: u64,
    mut roots: Vec<SettingsRoot>,
) -> Removal {
    roots.sort_by(|a, b| a.unit.cmp(&b.unit));
    roots.dedup_by(|a, b| a.unit == b.unit);
    audience.sort();
    audience.dedup();
    let mut folders: Vec<ManifestFolder> = removed
        .folders
        .iter()
        .map(|f| ManifestFolder {
            key: f.key.clone(),
            name: f.name.clone(),
            tree: f.tree.unwrap_or_else(|| sha256(b"")),
        })
        .collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key));
    folders.dedup_by(|a, b| a.key == b.key);
    let pairs: Vec<(FolderKey, Sha256)> = folders.iter().map(|f| (f.key.clone(), f.tree)).collect();
    Removal {
        group_id: group.clone(),
        issuer: Issuer {
            computer_id: own.computer_id.clone(),
            enrollment_id: own.enrollment_id.clone(),
            issued_at_seq: own_seq,
        },
        evidence: removed.evidence,
        rejection: None,
        title: removed.package.clone(),
        folders,
        removed_hash: package_hash(&pairs),
        audience,
        settings: if roots.is_empty() {
            RemovalSettings::None
        } else {
            RemovalSettings::Checkpoints { roots }
        },
        written_unix_ms: now_unix_ms,
    }
}
