//! Removal tests (removal.md §12). Records are built synthetically and learned into a ledger.

use crate::authority::AuthorityRecord;
use crate::decision::{Candidate, CleanupClass, CycleCx};
use crate::hash::sha256;
use crate::hooks::RemovalHooks;
use crate::ids::{GroupId, Identity, SettingsUnit, Sha256};
use crate::inventory::{FolderStatus, Inventory};
use crate::kinds::{
    Ack, AckItem, AckResult, AckSettings, AuthorityKind, EnrollmentRef, Issuer, Known,
    ManifestFolder, Readd, ReaddEvidence, Removal, RemovalEvidence, RemovalSettings, Seen,
};
use crate::ledger::{derive, Derived, Ledger, Membership};
use crate::package::{package_version, PackageVersion};
use crate::paths::{validate_folder_name, FolderKey};
use crate::plan::ExpectedLive;
use crate::records::{decode, encode, Location, MissingReason, OpKind, RootState, SettingsRoot};
use crate::removal::*;
use crate::settings::OwnedFile;
use crate::testkit::{addon, addon_full, client, computer, enrollment, group_id, inventory};
use crate::toc::IdKind;
use std::collections::BTreeSet;

fn eref(n: u8) -> EnrollmentRef {
    EnrollmentRef {
        computer_id: computer(n),
        enrollment_id: enrollment(n),
    }
}

fn issuer(n: u8, seq: u64) -> Issuer {
    Issuer {
        computer_id: computer(n),
        enrollment_id: enrollment(n),
        issued_at_seq: seq,
    }
}

fn key(n: &str) -> FolderKey {
    validate_folder_name(n).unwrap()
}

fn manifest(names: &[&str]) -> Vec<ManifestFolder> {
    let mut m: Vec<ManifestFolder> = names
        .iter()
        .map(|n| ManifestFolder {
            key: key(n),
            name: (*n).to_string(),
            tree: sha256(n.as_bytes()),
        })
        .collect();
    m.sort_by(|a, b| a.key.cmp(&b.key));
    m
}

fn rem(by: u8, seq: u64, names: &[&str], audience: &[u8]) -> Removal {
    let mut aud: Vec<EnrollmentRef> = audience.iter().map(|n| eref(*n)).collect();
    aud.sort();
    Removal {
        group_id: group_id(),
        issuer: issuer(by, seq),
        evidence: RemovalEvidence::UserAction,
        rejection: None,
        title: "Suite".into(),
        folders: manifest(names),
        removed_hash: sha256(names.join(",").as_bytes()),
        audience: aud,
        settings: RemovalSettings::None,
        written_unix_ms: 1,
    }
}

fn readd(by: u8, seq: u64, names: &[&str], seen: &[(u8, u64)]) -> Readd {
    let mut seen: Vec<Seen> = seen
        .iter()
        .map(|(n, s)| Seen {
            enrollment_id: enrollment(*n),
            seq: *s,
        })
        .collect();
    seen.sort();
    Readd {
        group_id: group_id(),
        issuer: issuer(by, seq),
        evidence: ReaddEvidence::UserAddBack,
        rejection: None,
        title: "Suite".into(),
        folders: manifest(names),
        present_hash: sha256(b"present"),
        known: Known {
            seen,
            extra: vec![],
            parts: vec![],
        },
        written_unix_ms: 2,
    }
}

fn ack_of(by: u8, seq: u64, removal: Sha256, names: &[&str]) -> Ack {
    let mut folders: Vec<FolderKey> = names.iter().map(|n| key(n)).collect();
    folders.sort();
    Ack {
        group_id: group_id(),
        issuer: issuer(by, seq),
        items: vec![AckItem { removal, folders }],
        result: AckResult::Absent,
        removed: vec![],
        settings: AckSettings::NotApplicable,
        written_unix_ms: 3,
    }
}

struct World {
    ledger: Ledger,
    members: Membership,
}

impl World {
    fn new(ns: &[u8]) -> Self {
        let mut members = Membership::default();
        for n in ns {
            members.add(&computer(*n), &enrollment(*n));
        }
        Self {
            ledger: Ledger::new(),
            members,
        }
    }

    fn learn_bytes(&mut self, bytes: Vec<u8>, kind: AuthorityKind, names: &[&str]) -> Sha256 {
        let h = sha256(&bytes);
        let folders = names.iter().map(|n| key(n)).collect();
        self.ledger
            .learn(h, kind, folders, Some(&computer(1)), Some(&bytes));
        h
    }

    fn removal(&mut self, r: &Removal) -> Sha256 {
        let names: Vec<&str> = r.folders.iter().map(|f| f.name.as_str()).collect();
        self.learn_bytes(encode(r), AuthorityKind::Removal, &names)
    }

    fn readd(&mut self, r: &Readd) -> Sha256 {
        let names: Vec<&str> = r.folders.iter().map(|f| f.name.as_str()).collect();
        self.learn_bytes(encode(r), AuthorityKind::Readd, &names)
    }

    fn ack(&mut self, a: &Ack) -> Sha256 {
        self.learn_bytes(encode(a), AuthorityKind::Ack, &[])
    }

    fn derived(&self) -> Derived {
        derive(&self.ledger, &group_id(), &self.members)
    }

    fn live(&self, f: &str) -> Vec<Sha256> {
        live_removals(&self.ledger, &self.derived(), &key(f))
    }

    fn behaviour(&self) -> RemovalBehaviour {
        RemovalBehaviour::new(&self.ledger, &group_id(), &self.members)
    }
}

fn candidate(holders: &[u8]) -> Candidate {
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let pv: PackageVersion =
        package_version(&group_packages_one(&inv), &inv, &client("1.0")).expect("version");
    Candidate {
        version: pv,
        holders: holders.iter().map(|n| computer(*n)).collect(),
    }
}

fn group_packages_one(inv: &Inventory) -> crate::package::Package {
    crate::package::group_packages(&[inv]).remove(0)
}

fn cx<'a>(
    g: &'a GroupId,
    own: &'a EnrollmentRef,
    w: &'a World,
    d: &'a Derived,
    held: &'a BTreeSet<FolderKey>,
    roots: &'a [SettingsRoot],
) -> CycleCx<'a> {
    CycleCx {
        group_id: g,
        own,
        ledger: &w.ledger,
        derived: d,
        members: &w.members,
        now_unix_ms: 1000,
        held,
        roots,
    }
}

fn root(folder: &str, captured: bool) -> SettingsRoot {
    SettingsRoot {
        unit: SettingsUnit {
            folder: key(folder),
            identity: Identity::account("ACC").unwrap(),
        },
        state: if captured {
            RootState::Captured {
                checkpoint: sha256(b"cp"),
                location: Location::Local,
            }
        } else {
            RootState::Missing {
                reason: MissingReason::Paused,
            }
        },
    }
}

// ---------------------------------------------------------------- merge

#[test]
fn a_removal_alone_is_live_and_a_covering_readd_lifts_it() {
    // T-R4D1-removal-live, T-R4D1-explicit-add-back
    let mut w = World::new(&[1, 2, 3]);
    let r = w.removal(&rem(2, 5, &["Alpha"], &[1, 2, 3]));
    assert_eq!(w.live("Alpha"), vec![r]);
    // Names a different folder: not a re-add of Alpha.
    w.readd(&readd(3, 9, &["Beta"], &[(2, 5)]));
    assert_eq!(w.live("Alpha"), vec![r]);
    w.readd(&readd(3, 9, &["Alpha"], &[(2, 5)]));
    assert!(w.live("Alpha").is_empty());
}

#[test]
fn removal_wins_over_a_concurrent_or_later_removal_relative_to_the_readd() {
    // T-R4D1-removal-wins: the re-add has not seen the removal (seq 7 > known 6), so it stays live.
    let mut w = World::new(&[1, 2, 3]);
    let r = w.removal(&rem(2, 7, &["Alpha"], &[1, 2, 3]));
    w.readd(&readd(3, 10, &["Alpha"], &[(2, 6)]));
    assert_eq!(w.live("Alpha"), vec![r]);
    // A re-add that has seen another enrollment's records does not cover this issuer.
    w.readd(&readd(3, 11, &["Alpha"], &[(1, 99)]));
    assert_eq!(w.live("Alpha"), vec![r]);
}

#[test]
fn conflicting_removals_merge_into_one_unit_and_both_stay_live() {
    // T-R4D1-conflicting-removals-merge
    let mut w = World::new(&[1, 2, 3]);
    let a = w.removal(&rem(2, 5, &["Alpha", "Beta"], &[1, 2, 3]));
    let b = w.removal(&rem(3, 5, &["Beta", "Gamma"], &[1, 2, 3]));
    let c = w.removal(&rem(3, 6, &["Delta"], &[1, 2, 3]));
    let mut both = vec![a, b];
    both.sort();
    assert_eq!(w.live("Beta"), both);
    let d = w.derived();
    let units = removal_units(&w.ledger, &d);
    assert_eq!(units.len(), 2);
    let big = units.iter().find(|u| u.folders.len() == 3).unwrap();
    assert_eq!(big.removals, both);
    let small = units.iter().find(|u| u.folders.len() == 1).unwrap();
    assert_eq!(small.removals, vec![c]);
    // A re-add covering only one of the two leaves the other live.
    w.readd(&readd(1, 20, &["Beta"], &[(2, 5)]));
    assert_eq!(w.live("Beta"), vec![b]);
}

#[test]
fn invalid_and_foreign_removals_never_count() {
    // T-R4D1-valid-only: a removal by a non-member is invalid and never merges.
    let mut w = World::new(&[1, 2]);
    w.removal(&rem(9, 5, &["Alpha"], &[1, 2]));
    assert!(w.live("Alpha").is_empty());
}

// ---------------------------------------------------------------- stale copies

#[test]
fn an_offline_peer_that_still_owes_cannot_resurrect_a_removed_package() {
    // T-R4D3-no-resurrection
    let mut w = World::new(&[1, 2, 3]);
    let r = w.removal(&rem(2, 5, &["Alpha"], &[1, 2, 3]));
    let b = w.behaviour();
    let c = candidate(&[3]);
    assert!(b.stale_copy(&w.ledger, &key("alpha"), &c));
    assert!(b.stale_copy(&w.ledger, &key("alpha"), &candidate(&[3, 2])));
    // One holder that already discharged makes the copy a legitimate candidate.
    w.ack(&ack_of(3, 8, r, &["Alpha"]));
    let b = w.behaviour();
    assert!(!b.stale_copy(&w.ledger, &key("alpha"), &c));
    // A holder outside the audience (enrolled later) is never stale because of this removal.
    let mut w2 = World::new(&[1, 2, 3, 4]);
    w2.removal(&rem(2, 5, &["Alpha"], &[1, 2, 3]));
    assert!(!w2
        .behaviour()
        .stale_copy(&w2.ledger, &key("alpha"), &candidate(&[4])));
    // Without validated state the neutral answer is given.
    assert!(!RemovalBehaviour::default().stale_copy(&w.ledger, &key("alpha"), &c));
}

#[test]
fn states_written_before_discharge_are_not_candidates() {
    // T-R4D3-eligible-from
    let mut w = World::new(&[1, 2, 3]);
    let r = w.removal(&rem(2, 5, &["Alpha"], &[1, 2, 3]));
    let d = w.derived();
    let e3 = eref(3);
    assert!(state_blocked(&w.ledger, &d, &e3, &r, &key("alpha"), 100));
    w.ack(&ack_of(3, 8, r, &["Alpha"]));
    let d = w.derived();
    assert_eq!(
        eligible_from(
            &w.ledger,
            &d,
            &e3,
            (
                r,
                match w.ledger.record(&r) {
                    Some(AuthorityRecord::Removal(x)) => x,
                    _ => unreachable!(),
                }
            ),
            &key("alpha")
        ),
        Some(8)
    );
    assert!(state_blocked(&w.ledger, &d, &e3, &r, &key("alpha"), 7));
    assert!(!state_blocked(&w.ledger, &d, &e3, &r, &key("alpha"), 8));
    // Not in the audience: nothing blocks it.
    assert!(!state_blocked(
        &w.ledger,
        &d,
        &eref(4),
        &r,
        &key("alpha"),
        1
    ));
}

#[test]
fn a_readd_discharges_the_owing_enrollment_too() {
    // T-R4D3-readd-discharges
    let mut w = World::new(&[1, 2, 3]);
    let r = w.removal(&rem(2, 5, &["Alpha"], &[1, 2, 3]));
    assert!(owes(&w.ledger, &w.derived(), &eref(3), &r, &key("alpha")));
    w.readd(&readd(3, 9, &["Alpha"], &[(2, 5)]));
    assert!(!owes(&w.ledger, &w.derived(), &eref(3), &r, &key("alpha")));
    assert!(owes(&w.ledger, &w.derived(), &eref(1), &r, &key("alpha")));
}

// ---------------------------------------------------------------- pending

#[test]
fn an_unreadable_removal_holds_installs_until_dismissed() {
    // T-R4D2-pending-hint
    let mut w = World::new(&[1, 2]);
    let h = sha256(b"junk");
    w.ledger.learn(
        h,
        AuthorityKind::Removal,
        vec![key("alpha")],
        Some(&computer(2)),
        Some(b"{ not json"),
    );
    let b = w.behaviour();
    assert!(b.merged_removals(&w.ledger, &key("alpha")).pending);
    assert!(!b.merged_removals(&w.ledger, &key("beta")).pending);
    w.ledger.dismiss(h);
    let b = w.behaviour();
    assert!(!b.merged_removals(&w.ledger, &key("alpha")).pending);
}

// ---------------------------------------------------------------- reinstall

#[test]
fn reinstall_is_held_only_after_this_computer_acknowledged() {
    // T-R4D4-reinstall-hold
    let mut w = World::new(&[1, 2]);
    let r = w.removal(&rem(2, 5, &["Alpha"], &[1, 2]));
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let own = eref(1);
    assert!(reinstall_held(&w.ledger, &w.derived(), &own, &inv).is_empty());
    w.ack(&ack_of(1, 6, r, &["Alpha"]));
    let held = reinstall_held(&w.ledger, &w.derived(), &own, &inv);
    assert_eq!(held, BTreeSet::from([key("alpha")]));
}

#[test]
fn restore_readd_covers_exactly_what_was_seen_and_a_later_removal_still_wins() {
    // T-R4D1-explicit-add-back, T-R4D21-readd-over-rollback (user readd lifting a removal)
    let mut w = World::new(&[1, 2]);
    let r1 = rem(2, 5, &["Alpha"], &[1, 2]);
    let h1 = w.removal(&r1);
    let re = readd_for_restore(
        &w.ledger,
        &group_id(),
        &eref(1),
        &r1,
        ReaddEvidence::UserRestore,
        6,
        10,
    );
    assert_eq!(re.folders, r1.folders);
    w.readd(&re);
    assert!(w.live("Alpha").is_empty());
    let h2 = w.removal(&rem(2, 7, &["Alpha"], &[1, 2]));
    assert_ne!(h1, h2);
    assert_eq!(w.live("Alpha"), vec![h2]);
}

// ---------------------------------------------------------------- restore operation (§7.4)

fn restore_folders(r: &Removal) -> Vec<RestoreFolder> {
    r.folders
        .iter()
        .map(|m| RestoreFolder {
            key: m.key.clone(),
            name: m.name.clone(),
            record: crate::records::FolderTreeRecord {
                tree: m.tree,
                files: vec![],
            },
            live: None,
        })
        .collect()
}

fn restorable(names: &[&str]) -> Removal {
    let mut r = rem(2, 5, names, &[1, 2]);
    r.removed_hash = crate::hash::package_hash(
        &r.folders
            .iter()
            .map(|f| (f.key.clone(), f.tree))
            .collect::<Vec<_>>(),
    );
    r
}

#[test]
fn a_restore_installs_exactly_the_removed_folders_and_reserves_the_readd() {
    let w = World::new(&[1, 2]);
    let r = restorable(&["Alpha", "Beta"]);
    let op = crate::ids::OpId::parse(&"ab".repeat(16)).unwrap();
    let plan = plan_restore(&w.ledger, &eref(1), &r, restore_folders(&r), op, 6, 10).unwrap();
    assert_eq!(plan.kind, OpKind::Restore);
    assert_eq!(plan.target_hash, r.removed_hash);
    assert_eq!(plan.folders.len(), 2);
    assert!(plan.removes.is_empty() && plan.removals.is_empty());
    assert_eq!(plan.pending_records.len(), 1, "only the user_restore readd");
}

#[test]
fn a_restore_whose_package_hash_is_not_the_removed_hash_is_refused() {
    let w = World::new(&[1, 2]);
    let mut r = restorable(&["Alpha"]);
    r.removed_hash = sha256(b"some other release");
    let op = crate::ids::OpId::parse(&"cd".repeat(16)).unwrap();
    let err = plan_restore(
        &w.ledger,
        &eref(1),
        &r,
        restore_folders(&r),
        op.clone(),
        6,
        10,
    )
    .unwrap_err();
    assert_eq!(err, RestoreError::HashMismatch);
    let good = restorable(&["Alpha"]);
    let mut swapped = restore_folders(&good);
    swapped[0].record.tree = sha256(b"substituted tree");
    let err = plan_restore(&w.ledger, &eref(1), &good, swapped, op, 6, 10).unwrap_err();
    assert_eq!(err, RestoreError::ManifestMismatch);
}

// ---------------------------------------------------------------- acknowledgement and planning

#[test]
fn due_items_include_the_origin_and_split_by_cleanup_class() {
    // T-R4D5-origin-acks, T-R4D5-absent-ack-waits-cleanup
    let mut w = World::new(&[1, 2]);
    let r = w.removal(&rem(1, 5, &["Alpha", "Beta"], &[1, 2]));
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let roots = [root("alpha", true), root("beta", false)];
    let gone = inventory(vec![]);
    let items = due_items(&cx(&g, &own, &w, &d, &held, &roots), Some(&gone));
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|(i, c)| i.removal == r
        && i.folders == vec![key("alpha")]
        && *c == CleanupClass::WaitForCleanup));
    assert!(items
        .iter()
        .any(|(i, c)| i.folders == vec![key("beta")] && *c == CleanupClass::AbsentAcknowledge));
    let acks = absent_acks(&cx(&g, &own, &w, &d, &held, &roots), &items, 9);
    assert_eq!(acks.len(), 1);
    assert_eq!(acks[0].items[0].folders, vec![key("beta")]);
    assert_eq!(acks[0].issuer.issued_at_seq, 9);
    assert_eq!(acks[0].result, AckResult::Absent);
    // A present folder is not yet due; an acknowledged one is not due again.
    let there = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let items = due_items(&cx(&g, &own, &w, &d, &held, &roots), Some(&there));
    assert!(items
        .iter()
        .all(|(i, _)| !i.folders.contains(&key("alpha"))));
    w.ack(&ack_of(1, 6, r, &["Beta"]));
    let d = w.derived();
    let items = due_items(&cx(&g, &own, &w, &d, &held, &roots), Some(&gone));
    assert!(items.iter().all(|(i, _)| !i.folders.contains(&key("beta"))));
    assert!(due_items(&cx(&g, &own, &w, &d, &held, &roots), None).is_empty());
    // Not in the audience: nothing is due.
    let other = eref(3);
    assert!(due_items(&cx(&g, &other, &w, &d, &held, &roots), Some(&gone)).is_empty());
}

#[test]
fn more_than_256_absent_items_split_into_batches() {
    // T-R4D5-ack-batches
    let mut w = World::new(&[1, 2]);
    for n in 0..300u64 {
        w.removal(&rem(2, 5 + n, &[&format!("Addon{n:03}")], &[1, 2]));
    }
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let gone = inventory(vec![]);
    let c = cx(&g, &own, &w, &d, &held, &[]);
    let items = due_items(&c, Some(&gone));
    assert_eq!(items.len(), 300);
    let acks = absent_acks(&c, &items, 1);
    assert_eq!(acks.len(), 2);
    assert_eq!(acks[0].items.len(), ACK_BATCH);
    assert_eq!(acks[1].items.len(), 300 - ACK_BATCH);
    assert_eq!(acks[1].issuer.issued_at_seq, 2);
    for a in &acks {
        assert!(decode::<Ack>(&encode(a), usize::MAX).is_ok());
    }
}

#[test]
fn a_remove_op_carries_the_fixed_ack_and_skips_held_or_unreadable_folders() {
    // T-R4D1-ack-bytes-fixed
    let mut w = World::new(&[1, 2]);
    let r = w.removal(&rem(2, 5, &["Alpha", "Beta"], &[1, 2]));
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let inv = inventory(vec![
        addon("Alpha", Some("1"), "a"),
        addon("Beta", Some("1"), "b"),
        addon("Other", Some("1"), "o"),
    ]);
    let ops = plan_remove_ops(&cx(&g, &own, &w, &d, &held, &[]), &inv, 7);
    assert_eq!(ops.len(), 1);
    let op = &ops[0];
    assert_eq!(op.kind, OpKind::Remove);
    assert_eq!(op.removals, vec![r]);
    let names: Vec<&str> = op.removes.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "Beta"]);
    assert_eq!(op.pending_records.len(), 1);
    let p = &op.pending_records[0];
    assert_eq!(sha256(p.bytes.as_bytes()), p.hash);
    let ack: Ack = decode(p.bytes.as_bytes(), usize::MAX).unwrap();
    assert_eq!(ack.result, AckResult::Removed);
    assert_eq!(ack.issuer.issued_at_seq, 7);
    assert_eq!(ack.items[0].folders, vec![key("alpha"), key("beta")]);
    assert_eq!(ack.removed.len(), 2);
    // Planning again gives the same bytes and the same operation id.
    let again = plan_remove_ops(&cx(&g, &own, &w, &d, &held, &[]), &inv, 7);
    assert_eq!(again[0].op, op.op);
    assert_eq!(again[0].pending_records, op.pending_records);
    // A held folder is never removed (the user reinstalled after this computer acknowledged).
    let held = BTreeSet::from([key("alpha"), key("beta")]);
    assert!(plan_remove_ops(&cx(&g, &own, &w, &d, &held, &[]), &inv, 7).is_empty());
    // An unstable listing plans nothing.
    let mut unstable = inv.clone();
    unstable.listing_stable = false;
    let held = BTreeSet::new();
    assert!(plan_remove_ops(&cx(&g, &own, &w, &d, &held, &[]), &unstable, 7).is_empty());
    // An unreadable folder blocks the whole unit.
    let mut bad = inv.clone();
    bad.folders[1].status = FolderStatus::Unreadable;
    assert!(plan_remove_ops(&cx(&g, &own, &w, &d, &held, &[]), &bad, 7).is_empty());
    // Not in the audience: this computer does not remove.
    let other = eref(3);
    assert!(plan_remove_ops(&cx(&g, &other, &w, &d, &held, &[]), &inv, 7).is_empty());
}

fn owned(folder: &str, bytes: &[u8]) -> OwnedFile {
    OwnedFile {
        unit: SettingsUnit {
            folder: key(folder),
            identity: Identity::account("ACC").unwrap(),
        },
        rel: crate::paths::account_settings_path("ACC", folder).unwrap(),
        size: bytes.len() as u64,
        sha256: sha256(bytes),
    }
}

fn root_missing(folder: &str, reason: MissingReason) -> SettingsRoot {
    SettingsRoot {
        state: RootState::Missing { reason },
        ..root(folder, false)
    }
}

fn op_ack(op: &crate::plan::PackageApply) -> Ack {
    assert_eq!(op.pending_records.len(), 1);
    decode(op.pending_records[0].bytes.as_bytes(), usize::MAX).unwrap()
}

#[test]
fn a_remove_op_moves_captured_settings_and_reserves_the_matching_ack() {
    // T-R5-remove-settings-op
    let mut w = World::new(&[1, 2]);
    w.removal(&rem(2, 5, &["Alpha"], &[1, 2]));
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let files = [owned("alpha", b"AlphaDB = {}")];
    let roots = [root("alpha", true)];
    let ops = plan_remove_ops_with(&cx(&g, &own, &w, &d, &held, &roots), &inv, 7, &files);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].settings_ops.len(), 1);
    let so = &ops[0].settings_ops[0];
    assert_eq!(
        so.staged, None,
        "uninstall moves the file out, nothing is staged"
    );
    assert_eq!(
        so.expected_live,
        ExpectedLive::Tree {
            tree: files[0].sha256,
            fingerprint: files[0].sha256
        }
    );
    assert_eq!(op_ack(&ops[0]).settings, AckSettings::Paired);
    // A paused unit stays in place and the ack says so.
    let paused = [root_missing("alpha", MissingReason::Paused)];
    let ops = plan_remove_ops_with(&cx(&g, &own, &w, &d, &held, &paused), &inv, 7, &files);
    assert!(ops[0].settings_ops.is_empty());
    assert_eq!(op_ack(&ops[0]).settings, AckSettings::KeptPaused);
    // An uncaptured unit (unknown owner, shared, too large) is never moved: code only.
    for reason in [
        MissingReason::UnknownOwner,
        MissingReason::SharedFile,
        MissingReason::TooLarge,
    ] {
        let missing = [root_missing("alpha", reason)];
        let ops = plan_remove_ops_with(&cx(&g, &own, &w, &d, &held, &missing), &inv, 7, &files);
        assert!(ops[0].settings_ops.is_empty(), "{reason:?}");
        assert_eq!(op_ack(&ops[0]).settings, AckSettings::CodeOnly);
    }
    // No roots at all: not applicable, nothing moved even if a file was passed.
    let ops = plan_remove_ops_with(&cx(&g, &own, &w, &d, &held, &[]), &inv, 7, &files);
    assert!(ops[0].settings_ops.is_empty());
    assert_eq!(op_ack(&ops[0]).settings, AckSettings::NotApplicable);
}

#[test]
fn the_ack_says_partial_when_only_some_units_were_captured() {
    let roots = [
        root("alpha", true),
        root_missing("beta", MissingReason::TooLarge),
    ];
    assert_eq!(ack_settings_for(&roots), AckSettings::Partial);
    assert_eq!(ack_settings_for(&[]), AckSettings::NotApplicable);
}

#[test]
fn a_folder_already_gone_gets_a_settings_cleanup_op_with_an_absent_ack() {
    // T-R5-remove-cleanup-op
    let mut w = World::new(&[1, 2]);
    let r = w.removal(&rem(1, 5, &["Alpha"], &[1, 2]));
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let inv = inventory(vec![addon("Other", Some("1"), "o")]);
    let files = [owned("alpha", b"AlphaDB = {}")];
    let roots = [root("alpha", true)];
    let ops = plan_cleanup_ops(&cx(&g, &own, &w, &d, &held, &roots), &inv, 7, &files);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, OpKind::SettingsCleanup);
    assert!(ops[0].removes.is_empty() && ops[0].removals.is_empty());
    assert_eq!(ops[0].settings_ops.len(), 1);
    let ack = op_ack(&ops[0]);
    assert_eq!(ack.result, AckResult::Absent);
    assert_eq!(ack.items[0].removal, r);
    assert_eq!(ack.settings, AckSettings::Paired);
    // Nothing captured: the plain absent acknowledgement applies and no cleanup is planned.
    let none = [root_missing("alpha", MissingReason::Paused)];
    assert!(plan_cleanup_ops(&cx(&g, &own, &w, &d, &held, &none), &inv, 7, &files).is_empty());
    // A folder still on disk is the remove operation's business.
    let present = inventory(vec![addon("Alpha", Some("1"), "a")]);
    assert!(plan_cleanup_ops(&cx(&g, &own, &w, &d, &held, &roots), &present, 7, &files).is_empty());
}

#[test]
fn the_hooks_plan_through_behaviour_state() {
    let mut w = World::new(&[1, 2]);
    w.removal(&rem(2, 5, &["Alpha"], &[1, 2]));
    let d = w.derived();
    let g = group_id();
    let own = eref(1);
    let held = BTreeSet::new();
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let c = cx(&g, &own, &w, &d, &held, &[]);
    assert!(w.behaviour().plan_removals(&c).is_empty());
    let b = w.behaviour().with_inventory(inv, 3);
    assert_eq!(b.plan_removals(&c).len(), 1);
    assert!(b.classify_due(&c).is_empty());
}

#[test]
fn dependents_are_warned_about_and_stay() {
    // T-R4D6-dependency-removed
    let inv = inventory(vec![
        addon_full("Child", Some("1"), &["Alpha"], &[], "c"),
        addon("Alpha", Some("1"), "a"),
    ]);
    let w = dependency_removed(&inv, &[key("alpha")]);
    assert_eq!(w, vec![("Child".to_string(), key("alpha"))]);
    assert!(dependency_removed(&inv, &[key("child")]).is_empty());
}

// ---------------------------------------------------------------- observation

fn input<'a>(
    inv: &'a Inventory,
    now: u64,
    journaled: &'a BTreeSet<FolderKey>,
    dir: &'a str,
) -> ObserveInput<'a> {
    ObserveInput {
        inventory: inv,
        dir_identity: dir,
        variant: "classic_era",
        now_unix_ms: now,
        journaled,
    }
}

fn started(inv: &Inventory) -> (Baseline, Observation) {
    let none = BTreeSet::new();
    let o = observe(
        &Baseline {
            variant: "classic_era".into(),
            dir_identity: "d1".into(),
            packages: Default::default(),
        },
        &Observation::default(),
        &input(inv, 0, &none, "d1"),
    );
    (o.baseline, o.observation)
}

#[test]
fn a_disappearance_needs_two_stable_scans_thirty_seconds_apart() {
    // T-R4D2-observe-two-scans
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let (base, obs) = started(&inv);
    assert!(base.packages.contains_key("alpha"));
    let gone = inventory(vec![]);
    let none = BTreeSet::new();
    let first = observe(&base, &obs, &input(&gone, 1_000, &none, "d1"));
    assert!(first.removed.is_empty());
    assert_eq!(
        first.held,
        vec![("alpha".into(), HoldReason::RemovalObserved)]
    );
    // Too early.
    let early = observe(
        &first.baseline,
        &first.observation,
        &input(&gone, 1_000 + OBSERVE_GAP_MS - 1, &none, "d1"),
    );
    assert!(early.removed.is_empty());
    // Persisted observation survives a restart: the original first-seen time is kept.
    assert_eq!(early.observation.gone["alpha"].0, 1_000);
    let done = observe(
        &early.baseline,
        &early.observation,
        &input(&gone, 1_000 + OBSERVE_GAP_MS, &none, "d1"),
    );
    assert_eq!(done.removed.len(), 1);
    assert_eq!(done.removed[0].evidence, RemovalEvidence::Observed);
    assert!(!done.baseline.packages.contains_key("alpha"));
}

#[test]
fn reappearance_an_unstable_scan_or_a_journal_resets_the_observation() {
    // T-R4D2-observe-reset
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let (base, obs) = started(&inv);
    let gone = inventory(vec![]);
    let none = BTreeSet::new();
    let first = observe(&base, &obs, &input(&gone, 1_000, &none, "d1"));
    // Back again: the observation is gone.
    let back = observe(
        &first.baseline,
        &first.observation,
        &input(&inv, 20_000, &none, "d1"),
    );
    assert!(back.observation.gone.is_empty());
    let again = observe(
        &back.baseline,
        &back.observation,
        &input(&gone, 60_000, &none, "d1"),
    );
    assert!(again.removed.is_empty());
    // An unstable listing changes nothing.
    let mut unstable = gone.clone();
    unstable.listing_stable = false;
    let u = observe(
        &first.baseline,
        &first.observation,
        &input(&unstable, 90_000, &none, "d1"),
    );
    assert_eq!(u.observation, first.observation);
    assert!(u.removed.is_empty());
    // A package that a journal names is never judged.
    let journaled = BTreeSet::from([key("alpha")]);
    let j = observe(
        &first.baseline,
        &first.observation,
        &input(&gone, 90_000, &journaled, "d1"),
    );
    assert!(j.removed.is_empty());
    assert!(j.baseline.packages.contains_key("alpha"));
}

#[test]
fn a_changed_directory_or_variant_discards_the_baseline() {
    // T-R4D2-observe-identity
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let (base, obs) = started(&inv);
    let gone = inventory(vec![]);
    let none = BTreeSet::new();
    let moved = observe(&base, &obs, &input(&gone, 99_000, &none, "d2"));
    assert!(moved.removed.is_empty());
    assert!(moved.baseline.packages.is_empty());
    assert_eq!(moved.baseline.dir_identity, "d2");
    let mut other_variant = input(&gone, 99_000, &none, "d1");
    other_variant.variant = "retail";
    let v = observe(&base, &obs, &other_variant);
    assert!(v.removed.is_empty());
    assert!(v.baseline.packages.is_empty());
}

#[test]
fn a_lookalike_or_new_unusable_entry_makes_the_removal_unclear() {
    // T-R4D2-observe-unclear
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let (base, obs) = started(&inv);
    let none = BTreeSet::new();
    let mut odd = inventory(vec![addon("Alpha_", Some("1"), "x")]);
    odd.folders[0].status = FolderStatus::Unreadable;
    let o = observe(&base, &obs, &input(&odd, 1_000, &none, "d1"));
    assert_eq!(o.held, vec![("alpha".into(), HoldReason::RemovalUnclear)]);
    assert!(o.removed.is_empty());
    assert!(o.baseline.packages.contains_key("alpha"));
    // A look-alike that is itself readable is unclear too.
    let look = inventory(vec![addon("A-lpha", Some("1"), "x")]);
    let o = observe(&base, &obs, &input(&look, 1_000, &none, "d1"));
    assert_eq!(o.held, vec![("alpha".into(), HoldReason::RemovalUnclear)]);
    // An unrelated readable folder does not make it unclear.
    let other = inventory(vec![addon("Beta", Some("1"), "b")]);
    let o = observe(&base, &obs, &input(&other, 1_000, &none, "d1"));
    assert_eq!(o.held, vec![("alpha".into(), HoldReason::RemovalObserved)]);
}

#[test]
fn a_dropped_folder_of_an_upgraded_package_is_a_folder_drop() {
    // T-R4D2-folder-drop
    let ids = [(IdKind::Curse, "3358")];
    let before = inventory(vec![
        addon_full("Suite", Some("1.0"), &[], &ids, "a"),
        addon_full("Suite_Options", Some("1.0"), &[], &ids, "o"),
    ]);
    assert_eq!(group_packages_one(&before).members.len(), 2);
    let (base, obs) = started(&before);
    let none = BTreeSet::new();
    let after = inventory(vec![addon_full("Suite", Some("2.0"), &[], &ids, "a2")]);
    let o = observe(&base, &obs, &input(&after, 1_000, &none, "d1"));
    assert_eq!(o.removed.len(), 1);
    assert_eq!(o.removed[0].evidence, RemovalEvidence::FolderDrop);
    let dropped: Vec<&str> = o.removed[0]
        .folders
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(dropped, vec!["Suite_Options"]);
    // Without an upgrade of the remaining folder a drop is not a removal.
    let same = inventory(vec![addon_full("Suite", Some("1.0"), &[], &ids, "a")]);
    let o = observe(&base, &obs, &input(&same, 1_000, &none, "d1"));
    assert!(o.removed.is_empty());
}

#[test]
fn an_observed_removal_becomes_a_record_that_validates() {
    // T-R4D2-observed-record
    let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
    let (base, obs) = started(&inv);
    let gone = inventory(vec![]);
    let none = BTreeSet::new();
    let first = observe(&base, &obs, &input(&gone, 1, &none, "d1"));
    let done = observe(
        &first.baseline,
        &first.observation,
        &input(&gone, 1 + OBSERVE_GAP_MS, &none, "d1"),
    );
    let r = removal_record(
        &group_id(),
        &eref(1),
        vec![eref(2), eref(1)],
        &done.removed[0],
        4,
        9,
    );
    assert_eq!(r.audience, vec![eref(1), eref(2)]);
    assert_eq!(r.folders[0].key, key("alpha"));
    let mut w = World::new(&[1, 2]);
    let h = w.removal(&r);
    assert_eq!(w.live("Alpha"), vec![h]);
}
