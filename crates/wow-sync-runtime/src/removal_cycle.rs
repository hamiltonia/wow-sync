//! The removal cycle's local state and steps (removal.md §1.6, §5.6, §7): the persisted presence
//! baseline and observation, the gated steps 3a/3b, and the explicit user commands.
//!
//! Nothing here runs by itself yet: `service.rs` has no step skeleton to call these from, so the
//! functions are exercised by their tests and wait for that wiring.
#![allow(dead_code)]

use std::collections::BTreeSet;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::FsError;
use wow_sync_core::decision::{AckItem, CleanupClass, CycleCx};
use wow_sync_core::ids::{GroupId, Sha256};
use wow_sync_core::inventory::{FolderState, Inventory};
use wow_sync_core::kinds::{Ack, EnrollmentRef, Readd, ReaddEvidence, Removal, RemovalEvidence};
use wow_sync_core::ledger::Ledger;
use wow_sync_core::paths::FolderKey;
use wow_sync_core::plan::PackageApply;
use wow_sync_core::removal::{
    absent_acks, observe, plan_remove_ops, readd_for_restore, removal_record, Baseline,
    Observation, ObserveInput, Observed, ObservedRemoval,
};

const LIMIT: u64 = 8 * 1024 * 1024;

fn baseline_name(group: &GroupId) -> String {
    format!("removal-baseline-{group}.json")
}

fn observation_name(group: &GroupId) -> String {
    format!("removal-observation-{group}.json")
}

/// The baseline and observation of one group, kept in local state so a restart neither forgets a
/// removal nor shortens the 30-second confirmation (E4).
pub struct RemovalStore<'a> {
    state: &'a LocalState,
    group: GroupId,
}

impl<'a> RemovalStore<'a> {
    pub fn new(state: &'a LocalState, group: GroupId) -> Self {
        Self { state, group }
    }

    /// A missing, unreadable or foreign-looking file is a fresh start: no baseline can only mean
    /// that nothing is judged missing, never that something is.
    pub fn load(&self) -> (Baseline, Observation) {
        let base = self
            .state
            .read(&baseline_name(&self.group), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| Baseline::from_bytes(&b))
            .unwrap_or_default();
        let obs = self
            .state
            .read(&observation_name(&self.group), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| Observation::from_bytes(&b))
            .unwrap_or_default();
        (base, obs)
    }

    pub fn save(&self, base: &Baseline, obs: &Observation) -> Result<(), FsError> {
        self.state
            .write_atomic(&observation_name(&self.group), &obs.to_bytes())?;
        self.state
            .write_atomic(&baseline_name(&self.group), &base.to_bytes())
    }

    /// A package the group removed and this computer applied is not a local disappearance:
    /// forgetting it keeps the next observation from turning the applied removal into a new one.
    pub fn forget_group_removed(
        &self,
        inventory: &Inventory,
        removed_by_group: impl Fn(&FolderKey) -> bool,
    ) -> Result<(), FsError> {
        let (mut base, mut obs) = self.load();
        let gone: Vec<String> = base
            .packages
            .iter()
            .filter(|(_, fs)| {
                fs.iter()
                    .all(|f| inventory.get(&f.key).is_none() && removed_by_group(&f.key))
            })
            .map(|(n, _)| n.clone())
            .collect();
        if gone.is_empty() {
            return Ok(());
        }
        for n in gone {
            base.packages.remove(&n);
            obs.gone.remove(&n);
        }
        self.save(&base, &obs)
    }

    /// Packages this computer just removed on the group's behalf, whatever the removal's state.
    pub fn forget_keys(&self, keys: &BTreeSet<FolderKey>) -> Result<(), FsError> {
        let (mut base, mut obs) = self.load();
        let gone: Vec<String> = base
            .packages
            .iter()
            .filter(|(_, fs)| fs.iter().all(|f| keys.contains(&f.key)))
            .map(|(n, _)| n.clone())
            .collect();
        if gone.is_empty() {
            return Ok(());
        }
        for n in gone {
            base.packages.remove(&n);
            obs.gone.remove(&n);
        }
        self.save(&base, &obs)
    }

    /// One observation pass; the new baseline and observation are persisted before the result is
    /// returned, so a crash cannot lose a first sighting. A removal this pass matured is also
    /// kept as pending *before* the observation is consumed and returned again by every later
    /// pass until `clear_pending`, so a crash (or a failed write) before its record is written
    /// does not lose it. A pending removal whose folder is installed again is dropped.
    pub fn observe(&self, input: &ObserveInput) -> Result<Observed, FsError> {
        let (base, obs) = self.load();
        let mut out = observe(&base, &obs, input);
        let before = self.load_pending();
        let mut pending: Vec<PendingRemoval> = before
            .iter()
            .filter(|p| {
                p.folders
                    .iter()
                    .all(|f| input.inventory.get(&f.key).is_none())
            })
            .cloned()
            .collect();
        for r in &out.removed {
            if !pending.iter().any(|p| p.package == r.package) {
                pending.push(PendingRemoval::from(r));
            }
        }
        if pending != before {
            self.save_pending(&pending)?;
        }
        self.save(&out.baseline, &out.observation)?;
        out.removed = pending.into_iter().map(ObservedRemoval::from).collect();
        Ok(out)
    }

    fn pending_name(&self) -> String {
        format!("removal-pending-{}.json", self.group)
    }

    fn load_pending(&self) -> Vec<PendingRemoval> {
        self.state
            .read(&self.pending_name(), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save_pending(&self, p: &[PendingRemoval]) -> Result<(), FsError> {
        if p.is_empty() {
            return self.state.remove(&self.pending_name());
        }
        self.state.write_atomic(
            &self.pending_name(),
            &serde_json::to_vec(p).expect("plain data serializes"),
        )
    }

    /// The removal record for `package` is written; it is no longer pending.
    pub fn clear_pending(&self, package: &str) {
        let mut p = self.load_pending();
        p.retain(|r| r.package != package);
        let _ = self.save_pending(&p);
    }
}

/// An observed removal kept across a restart until its record is written.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PendingRemoval {
    package: String,
    folders: Vec<FolderState>,
    evidence: RemovalEvidence,
}

impl From<&ObservedRemoval> for PendingRemoval {
    fn from(r: &ObservedRemoval) -> Self {
        Self {
            package: r.package.clone(),
            folders: r.folders.clone(),
            evidence: r.evidence,
        }
    }
}

impl From<PendingRemoval> for ObservedRemoval {
    fn from(r: PendingRemoval) -> Self {
        Self {
            package: r.package,
            folders: r.folders,
            evidence: r.evidence,
        }
    }
}

// ---------------------------------------------------------------- steps

/// Step 3b: absent acknowledgements for items with nothing to clean. Items that wait for a
/// settings cleanup get none (T-R4D5-absent-ack-waits-cleanup). Ledger writes never touch game
/// files, so a running game does not block them.
pub fn step_3b(cx: &CycleCx, items: &[(AckItem, CleanupClass)], next_seq: u64) -> Vec<Ack> {
    absent_acks(cx, items, next_seq)
}

/// Step 3a: remove operations. Game files change here, so a running game, an unstable listing
/// or an unknown activity plans nothing.
pub fn step_3a(
    cx: &CycleCx,
    inv: &Inventory,
    next_seq: u64,
    game_blocks_changes: bool,
) -> Vec<PackageApply> {
    if game_blocks_changes {
        return Vec::new();
    }
    plan_remove_ops(cx, inv, next_seq)
}

// ---------------------------------------------------------------- commands

fn issuer_audience(own: &EnrollmentRef, mut audience: Vec<EnrollmentRef>) -> Vec<EnrollmentRef> {
    if !audience.contains(own) {
        audience.push(own.clone());
    }
    audience
}

fn as_observed(package: &str, folders: &[FolderState], e: RemovalEvidence) -> ObservedRemoval {
    ObservedRemoval {
        package: package.to_string(),
        folders: folders.to_vec(),
        evidence: e,
    }
}

/// "Remove from group" (and "Remove again" after a reinstall): an explicit removal of what is
/// installed here.
pub fn remove_from_group(
    group: &GroupId,
    own: &EnrollmentRef,
    audience: Vec<EnrollmentRef>,
    title: &str,
    folders: &[FolderState],
    own_seq: u64,
    now_unix_ms: u64,
) -> Removal {
    let o = as_observed(title, folders, RemovalEvidence::UserAction);
    removal_record(
        group,
        own,
        issuer_audience(own, audience),
        &o,
        own_seq,
        now_unix_ms,
    )
}

/// The user confirms that an unclear disappearance was a removal.
pub fn confirm_unclear(
    group: &GroupId,
    own: &EnrollmentRef,
    audience: Vec<EnrollmentRef>,
    removed: &ObservedRemoval,
    own_seq: u64,
    now_unix_ms: u64,
) -> Removal {
    let o = as_observed(
        &removed.package,
        &removed.folders,
        RemovalEvidence::UserConfirmed,
    );
    removal_record(
        group,
        own,
        issuer_audience(own, audience),
        &o,
        own_seq,
        now_unix_ms,
    )
}

/// The user says an unclear disappearance is not a removal: no record is written and the package
/// leaves the baseline, so it is not asked about again.
pub fn keep_unclear(base: &mut Baseline, obs: &mut Observation, package: &str) {
    base.packages.remove(package);
    obs.gone.remove(package);
}

/// "Restore": re-add a removed package for the group (explicit only, Q-55).
pub fn restore(
    ledger: &Ledger,
    group: &GroupId,
    own: &EnrollmentRef,
    removal: &Removal,
    own_seq: u64,
    now_unix_ms: u64,
) -> Readd {
    readd_for_restore(
        ledger,
        group,
        own,
        removal,
        ReaddEvidence::UserRestore,
        own_seq,
        now_unix_ms,
    )
}

/// "Add back": the user reinstalled it on purpose and wants it kept.
pub fn add_back(
    ledger: &Ledger,
    group: &GroupId,
    own: &EnrollmentRef,
    removal: &Removal,
    own_seq: u64,
    now_unix_ms: u64,
) -> Readd {
    readd_for_restore(
        ledger,
        group,
        own,
        removal,
        ReaddEvidence::UserAddBack,
        own_seq,
        now_unix_ms,
    )
}

/// "Continue without it": an unreadable removal stops holding installs, on this computer only.
pub fn continue_without(ledger: &mut Ledger, removal: Sha256) {
    ledger.dismiss(removal);
}

/// Folder keys that a game-file operation must not touch while a journal names them (E5).
pub fn journaled_keys(names: impl IntoIterator<Item = FolderKey>) -> BTreeSet<FolderKey> {
    names.into_iter().collect()
}

/// Synthetic data without the core testkit (the runtime does not enable that feature).
#[cfg(test)]
pub(crate) mod fixtures {
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::{ComputerId, EnrollmentId, GroupId};
    use wow_sync_core::inventory::{FolderState, FolderStatus, Inventory, TocKind, TocSummary};
    use wow_sync_core::paths::validate_folder_name;

    fn hex(n: u8) -> String {
        format!("{n:02x}").repeat(16)
    }

    pub fn computer(n: u8) -> ComputerId {
        ComputerId::parse(&hex(n)).unwrap()
    }

    pub fn enrollment(n: u8) -> EnrollmentId {
        EnrollmentId::parse(&hex(n)).unwrap()
    }

    pub fn group_id() -> GroupId {
        GroupId::parse(&hex(0x11)).unwrap()
    }

    pub fn addon(name: &str, _version: Option<&str>, tag: &str) -> FolderState {
        FolderState {
            name: name.into(),
            key: validate_folder_name(name).unwrap(),
            status: FolderStatus::Ok,
            tree: Some(sha256(format!("{name}:{tag}").as_bytes())),
            fingerprint: Some(sha256(format!("fp:{name}:{tag}").as_bytes())),
            files: 1,
            bytes: 1,
            toc: TocSummary::empty(TocKind::Selected),
            file_entries: vec![],
        }
    }

    pub fn inventory(folders: Vec<FolderState>) -> Inventory {
        Inventory {
            folders,
            listing_stable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{addon, computer, enrollment, group_id, inventory};
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::hash::sha256;
    use wow_sync_core::kinds::{AckResult, AuthorityKind};
    use wow_sync_core::ledger::{derive, Membership};
    use wow_sync_core::records::{decode, encode};
    use wow_sync_core::removal::{HoldReason, OBSERVE_GAP_MS};

    fn eref(n: u8) -> EnrollmentRef {
        EnrollmentRef {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
        }
    }

    fn input<'a>(
        inv: &'a Inventory,
        now: u64,
        none: &'a BTreeSet<FolderKey>,
        dir: &'a str,
    ) -> ObserveInput<'a> {
        ObserveInput {
            inventory: inv,
            dir_identity: dir,
            variant: "classic_era",
            now_unix_ms: now,
            journaled: none,
        }
    }

    #[test]
    fn persisted_observation_keeps_the_first_sighting_across_a_restart() {
        // T-R4D2-observe-persisted
        let dir = TempDir::new();
        let state = LocalState::open_at(&dir.join("state")).unwrap();
        let none = BTreeSet::new();
        let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
        let gone = inventory(vec![]);
        let seed = Baseline {
            variant: "classic_era".into(),
            dir_identity: "d1".into(),
            packages: Default::default(),
        };
        RemovalStore::new(&state, group_id())
            .save(&seed, &Observation::default())
            .unwrap();
        RemovalStore::new(&state, group_id())
            .observe(&input(&inv, 0, &none, "d1"))
            .unwrap();
        let first = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000, &none, "d1"))
            .unwrap();
        assert_eq!(first.held[0].1, HoldReason::RemovalObserved);
        // A new store (a restart) must still know the first sighting.
        let second = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000 + OBSERVE_GAP_MS - 1, &none, "d1"))
            .unwrap();
        assert!(second.removed.is_empty());
        let third = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000 + OBSERVE_GAP_MS, &none, "d1"))
            .unwrap();
        assert_eq!(third.removed.len(), 1);
        // Corrupt files are a fresh start that judges nothing.
        state
            .write_atomic(&baseline_name(&group_id()), b"{ not json")
            .unwrap();
        let (b, o) = RemovalStore::new(&state, group_id()).load();
        assert!(b.packages.is_empty() && o.gone.is_empty());
    }

    #[test]
    fn game_running_blocks_remove_ops_but_not_absent_acks_and_waiting_items_get_none() {
        // T-R4D5-absent-ack-waits-cleanup
        let mut ledger = Ledger::new();
        let mut members = Membership::default();
        members.add(&computer(1), &enrollment(1));
        members.add(&computer(2), &enrollment(2));
        let r = wow_sync_core::removal::removal_record(
            &group_id(),
            &eref(2),
            vec![eref(1), eref(2)],
            &ObservedRemoval {
                package: "Alpha".into(),
                folders: inventory(vec![addon("Alpha", Some("1"), "a")]).folders,
                evidence: RemovalEvidence::UserAction,
            },
            5,
            1,
        );
        let bytes = encode(&r);
        ledger.learn(
            sha256(&bytes),
            AuthorityKind::Removal,
            vec![r.folders[0].key.clone()],
            Some(&computer(2)),
            Some(&bytes),
        );
        let d = derive(&ledger, &group_id(), &members);
        let g = group_id();
        let own = eref(1);
        let held = BTreeSet::new();
        let cx = CycleCx {
            group_id: &g,
            own: &own,
            ledger: &ledger,
            derived: &d,
            members: &members,
            now_unix_ms: 9,
            held: &held,
            roots: &[],
        };
        let present = inventory(vec![addon("Alpha", Some("1"), "a")]);
        assert!(step_3a(&cx, &present, 3, true).is_empty());
        assert_eq!(step_3a(&cx, &present, 3, false).len(), 1);
        let gone = inventory(vec![]);
        let items = wow_sync_core::removal::due_items(&cx, Some(&gone));
        let acks = step_3b(&cx, &items, 3);
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].result, AckResult::Absent);
        let waiting: Vec<(AckItem, CleanupClass)> = items
            .into_iter()
            .map(|(i, _)| (i, CleanupClass::WaitForCleanup))
            .collect();
        assert!(step_3b(&cx, &waiting, 3).is_empty());
    }

    #[test]
    fn commands_write_the_documented_records_and_keep_or_dismiss_locally() {
        // T-R4D4-commands
        let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
        let own = eref(1);
        let r = remove_from_group(
            &group_id(),
            &own,
            vec![eref(2)],
            "Alpha",
            &inv.folders,
            4,
            7,
        );
        assert_eq!(r.evidence, RemovalEvidence::UserAction);
        assert_eq!(r.audience, vec![eref(1), eref(2)]);
        let again = confirm_unclear(
            &group_id(),
            &own,
            vec![],
            &as_observed("Alpha", &inv.folders, RemovalEvidence::Observed),
            5,
            8,
        );
        assert_eq!(again.evidence, RemovalEvidence::UserConfirmed);
        assert_eq!(again.audience, vec![own.clone()]);
        let ledger = Ledger::new();
        let re = restore(&ledger, &group_id(), &own, &r, 6, 9);
        assert_eq!(re.evidence, ReaddEvidence::UserRestore);
        let ab = add_back(&ledger, &group_id(), &own, &r, 6, 9);
        assert_eq!(ab.evidence, ReaddEvidence::UserAddBack);
        assert!(decode::<Readd>(&encode(&ab), usize::MAX).is_ok());
        let mut base = Baseline {
            variant: "v".into(),
            dir_identity: "d".into(),
            packages: [("alpha".to_string(), inv.folders.clone())].into(),
        };
        let mut obs = Observation::default();
        obs.gone.insert("alpha".into(), (1, 2));
        keep_unclear(&mut base, &mut obs, "alpha");
        assert!(base.packages.is_empty() && obs.gone.is_empty());
        let mut l = Ledger::new();
        let h = sha256(b"x");
        continue_without(&mut l, h);
        assert!(l.dismissed().contains(&h));
    }
}
