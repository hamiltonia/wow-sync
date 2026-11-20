//! Rejection, lift and rollback-plan tests (rollback.md §10).

use crate::authority::{decode_authority, AuthorityRecord};
use crate::hash::{package_hash, sha256};
use crate::hooks::ConstraintHooks;
use crate::ids::Sha256;
use crate::inventory::ScanStatus;
use crate::kinds::{
    AckSettings, AuthorityKind, Considered, EnrollmentRef, Issuer, Known, Lift, ManifestFolder,
    Readd, ReaddEvidence, RecoveredEntry, RejectedEntry, Rejection, RejectionSettings,
    RemovalEvidence, TreeEntry,
};
use crate::ledger::{derive, Ledger, Membership};
use crate::package::Package;
use crate::paths::FolderKey;
use crate::plan::{ExpectedLive, FolderAction, PackageApply};
use crate::records::{encode, FolderTreeRecord, OpKind, Record};
use crate::reject::*;
use crate::target::{Content, ScopeMatch, Source};
use std::collections::BTreeSet;

fn trees(e: &[TreeEntry]) -> Vec<(FolderKey, Sha256)> {
    e.iter().map(|t| (t.key.clone(), t.tree)).collect()
}
use crate::testkit::{addon, computer, enrollment, group_id, inventory, op};

fn fk(n: &str) -> FolderKey {
    FolderKey::lossy(n)
}

fn src(items: &[(&str, &str)]) -> Source {
    Source::from_inventory(&inventory(
        items
            .iter()
            .map(|(n, t)| addon(n, Some("1.0"), t))
            .collect(),
    ))
}

/// A source in which the folders named in `unknown` could not be read.
fn src_unknown(items: &[(&str, &str)], unknown: &[&str]) -> Source {
    let scans = items
        .iter()
        .map(|(n, t)| {
            let mut s = addon(n, Some("1.0"), t);
            if unknown.contains(n) {
                s.status = ScanStatus::Unreadable("denied".into());
            }
            s
        })
        .collect();
    Source::from_inventory(&inventory(scans))
}

fn tree(n: &str, t: &str) -> Sha256 {
    match src(&[(n, t)]).content(&fk(n)) {
        Content::Tree(h) => h,
        other => panic!("no tree: {other:?}"),
    }
}

fn entries(items: &[(&str, &str)]) -> Vec<TreeEntry> {
    let mut v: Vec<TreeEntry> = items
        .iter()
        .map(|(n, t)| TreeEntry {
            key: fk(n),
            tree: tree(n, t),
        })
        .collect();
    v.sort_by(|a, b| a.key.cmp(&b.key));
    v
}

fn issuer(n: u8, seq: u64) -> Issuer {
    Issuer {
        computer_id: computer(n),
        enrollment_id: enrollment(n),
        issued_at_seq: seq,
    }
}

fn considered(items: &[(&str, &str)]) -> Considered {
    let e = entries(items);
    Considered {
        package_hash: package_hash(&trees(&e)),
        folders: e,
        holders: vec![],
    }
}

fn rej(
    by: u8,
    seq: u64,
    rejected: &[(&str, &str)],
    recovered: &[(&str, &str)],
    label: &str,
) -> Rejection {
    let r1 = entries(rejected);
    let r0 = entries(recovered);
    let mut union: Vec<FolderKey> = r1.iter().chain(&r0).map(|e| e.key.clone()).collect();
    union.sort();
    union.dedup();
    let mut c = vec![considered(rejected), considered(recovered)];
    c.sort();
    Rejection {
        group_id: group_id(),
        issuer: issuer(by, seq),
        folders: union,
        rejected: RejectedEntry {
            package_hash: package_hash(&trees(&r1)),
            version_label: label.into(),
            folders: r1,
        },
        recovered: RecoveredEntry {
            package_hash: package_hash(&trees(&r0)),
            folders: r0,
        },
        considered: c,
        settings: RejectionSettings::NotApplicable,
        known: Known::default(),
        considered_parts: vec![],
        written_unix_ms: 9,
    }
}

struct World {
    l: Ledger,
    m: Membership,
}

impl World {
    fn new() -> Self {
        let mut m = Membership::default();
        for n in 1..=3 {
            m.add(&computer(n), &enrollment(n));
        }
        Self {
            l: Ledger::new(),
            m,
        }
    }

    fn add<R: Record>(&mut self, r: &R, kind: AuthorityKind) -> Sha256 {
        let bytes = encode(r);
        let h = sha256(&bytes);
        self.l
            .learn(h, kind, vec![], Some(&computer(1)), Some(&bytes));
        h
    }

    fn rej(&mut self, r: &Rejection) -> Sha256 {
        self.add(r, AuthorityKind::Rejection)
    }

    fn b(&self) -> RejectBehaviour {
        RejectBehaviour::new(&derive(&self.l, &group_id(), &self.m))
    }
}

fn package(members: &[&str]) -> Package {
    Package {
        key: fk(members[0]),
        display: fk(members[0]),
        members: members.iter().map(|m| fk(m)).collect(),
        edges: vec![],
    }
}

fn constraint_trees(b: &RejectBehaviour, l: &Ledger, f: &str) -> Vec<Sha256> {
    let mut v: Vec<Sha256> = b
        .live_constraints(l, &fk(f))
        .iter()
        .map(|c| c.tree.expect("tree"))
        .collect();
    v.sort();
    v
}

#[test]
fn a_rejection_blocks_exact_content_over_its_own_folders_only() {
    // T-R04, T-R4D23-added-folder-rollback, T-R4D28-exact-source-unrelated-addon
    let mut w = World::new();
    let x = rej(
        1,
        5,
        &[("details", "u"), ("details_extra", "bad")],
        &[("details", "u")],
        "2.1",
    );
    w.rej(&x);
    let b = w.b();
    let p = package(&["details", "details_extra"]);
    let r1 = src(&[("details", "u"), ("details_extra", "bad")]);
    let r0 = src(&[("details", "u")]);
    let r1_and_unrelated = src(&[("details", "u"), ("details_extra", "bad"), ("other", "z")]);
    assert_eq!(b.rejected(&w.l, &p, &r1), ScopeMatch::Same);
    assert_eq!(b.rejected(&w.l, &p, &r1_and_unrelated), ScopeMatch::Same);
    assert_eq!(
        b.rejected(&w.l, &p, &r0),
        ScopeMatch::Different,
        "R0 shares a tree with R1 but lacks details_extra, so it is not the rejected release"
    );
    let changed = src(&[("details", "u"), ("details_extra", "other-bytes")]);
    assert_eq!(b.rejected(&w.l, &p, &changed), ScopeMatch::Different);
    let unreadable = src_unknown(
        &[("details", "u"), ("details_extra", "bad")],
        &["details_extra"],
    );
    assert_eq!(b.rejected(&w.l, &p, &unreadable), ScopeMatch::Unknown);
    assert_eq!(
        b.rejected(&w.l, &package(&["other"]), &r1),
        ScopeMatch::Different,
        "a package outside the rejection's folders is never compared"
    );
    assert_eq!(
        constraint_trees(&b, &w.l, "details"),
        vec![tree("details", "u")]
    );
    assert!(constraint_trees(&b, &w.l, "details_extra").is_empty());
}

#[test]
fn only_valid_records_count_and_the_default_has_no_effect() {
    let mut w = World::new();
    let x = rej(9, 5, &[("details", "a")], &[("details", "b")], "2.1");
    w.rej(&x);
    let p = package(&["details"]);
    let b = w.b();
    assert_eq!(
        b.rejected(&w.l, &p, &src(&[("details", "a")])),
        ScopeMatch::Different,
        "a rejection from a computer that is not a member is invalid"
    );
    assert!(b.live_constraints(&w.l, &fk("details")).is_empty());
    w.rej(&rej(1, 5, &[("details", "a")], &[("details", "b")], "2.1"));
    let none = RejectBehaviour::default();
    assert_eq!(
        none.rejected(&w.l, &p, &src(&[("details", "a")])),
        ScopeMatch::Different
    );
    assert!(none.live_constraints(&w.l, &fk("details")).is_empty());
    assert_eq!(
        w.b().rejected(&w.l, &p, &src(&[("details", "a")])),
        ScopeMatch::Same
    );
}

#[test]
fn a_lift_ends_only_the_rejections_it_names() {
    // T-R07, T-R08, Q-79
    let mut w = World::new();
    let x1 = rej(1, 5, &[("details", "a")], &[("details", "b")], "2.1");
    let x2 = rej(2, 6, &[("details", "a")], &[("details", "b")], "2.1");
    let h1 = w.rej(&x1);
    let h2 = w.rej(&x2);
    assert_ne!(h1, h2);
    let b = w.b();
    let lift = build_lift(
        &b,
        &w.l,
        &group_id(),
        issuer(3, 7),
        &[h1, h1],
        Known::default(),
        10,
    )
    .expect("active rejection");
    assert_eq!(lift.lifts, vec![h1]);
    assert_eq!(lift.folders, vec![fk("details")]);
    w.add(&lift, AuthorityKind::Lift);
    let b = w.b();
    let active: Vec<Sha256> = b.active(&w.l).iter().map(|(h, _)| *h).collect();
    assert_eq!(
        active,
        vec![h2],
        "a concurrent rejection of the same content stays"
    );
    assert_eq!(
        b.rejected(&w.l, &package(&["details"]), &src(&[("details", "a")])),
        ScopeMatch::Same
    );
    assert_eq!(
        build_lift(
            &b,
            &w.l,
            &group_id(),
            issuer(3, 8),
            &[h1],
            Known::default(),
            11
        ),
        Err(LiftError::NotActive(h1))
    );
    assert_eq!(
        build_lift(
            &b,
            &w.l,
            &group_id(),
            issuer(3, 8),
            &[],
            Known::default(),
            11
        ),
        Err(LiftError::Empty)
    );
    let both = Lift {
        lifts: {
            let mut v = vec![h1, h2];
            v.sort();
            v
        },
        issuer: issuer(2, 9),
        ..lift
    };
    w.add(&both, AuthorityKind::Lift);
    assert!(w.b().active(&w.l).is_empty());
    assert!(
        constraint_trees(&w.b(), &w.l, "details").is_empty(),
        "a lift also ends the recovered-target constraint"
    );
}

#[test]
fn a_record_covering_a_rejection_ends_it_for_the_folders_it_names_only() {
    // T-R4D21 (partial coverage)
    let mut w = World::new();
    let x = rej(
        1,
        5,
        &[("alpha", "1"), ("beta", "1")],
        &[("alpha", "0"), ("beta", "0")],
        "2.1",
    );
    let hx = w.rej(&x);
    assert_eq!(constraint_trees(&w.b(), &w.l, "alpha").len(), 1);
    let readd = Readd {
        group_id: group_id(),
        issuer: issuer(2, 6),
        evidence: ReaddEvidence::UserRestore,
        rejection: None,
        title: "Alpha".into(),
        folders: vec![ManifestFolder {
            key: fk("alpha"),
            name: "alpha".into(),
            tree: tree("alpha", "9"),
        }],
        present_hash: sha256(b"present"),
        known: Known {
            seen: vec![],
            extra: vec![hx],
            parts: vec![],
        },
        written_unix_ms: 9,
    };
    w.add(&readd, AuthorityKind::Readd);
    let b = w.b();
    assert!(constraint_trees(&b, &w.l, "alpha").is_empty());
    assert_eq!(constraint_trees(&b, &w.l, "beta"), vec![tree("beta", "0")]);
    assert_eq!(
        b.rejected(
            &w.l,
            &package(&["alpha", "beta"]),
            &src(&[("alpha", "1"), ("beta", "1")])
        ),
        ScopeMatch::Same,
        "the rejection itself stays active"
    );
}

#[test]
fn the_chain_follows_rejected_recoveries_and_a_cycle_ends() {
    // T-R10, T-R11 (R4D-9 table)
    let mut w = World::new();
    let hx = w.rej(&rej(1, 5, &[("alpha", "2")], &[("alpha", "1")], "2.2"));
    let hy = w.rej(&rej(2, 5, &[("alpha", "1")], &[("alpha", "0")], "2.1"));
    let b = w.b();
    let cs = b.live_constraints(&w.l, &fk("alpha"));
    assert_eq!(cs.len(), 1, "R1 is rejected, so R0 is the one target");
    assert_eq!(cs[0].tree, Some(tree("alpha", "0")));
    assert_eq!(cs[0].record, hy);
    assert_ne!(cs[0].record, hx);

    let mut w = World::new();
    w.rej(&rej(1, 5, &[("alpha", "1")], &[("alpha", "0")], "2.1"));
    w.rej(&rej(2, 5, &[("alpha", "0")], &[("alpha", "1")], "2.0"));
    let b = w.b();
    assert!(b.live_constraints(&w.l, &fk("alpha")).is_empty());
    let any = b.active(&w.l)[0].0;
    assert_eq!(b.recovered_chain(&w.l, &any, None), Chain::AllRejected);
}

#[test]
fn concurrent_rollbacks_with_different_targets_leave_two_constraints() {
    // T-R4D9-concurrent-rollbacks (the compose step turns two trees into RollbackTargetsDiffer)
    let mut w = World::new();
    let ha = w.rej(&rej(1, 5, &[("alpha", "1")], &[("alpha", "0")], "2.1"));
    let hb = w.rej(&rej(2, 5, &[("alpha", "1")], &[("alpha", "0b")], "2.1"));
    let cs = w.b().live_constraints(&w.l, &fk("alpha"));
    assert_eq!(cs.len(), 2);
    assert_eq!(constraint_trees(&w.b(), &w.l, "alpha"), {
        let mut v = vec![tree("alpha", "0"), tree("alpha", "0b")];
        v.sort();
        v
    });
    let recs: BTreeSet<Sha256> = cs.iter().map(|c| c.record).collect();
    assert_eq!(recs, BTreeSet::from([ha, hb]));

    let mut w = World::new();
    w.rej(&rej(1, 5, &[("alpha", "1")], &[("alpha", "0")], "2.1"));
    w.rej(&rej(2, 6, &[("alpha", "1")], &[("alpha", "0")], "2.1"));
    let distinct: BTreeSet<Sha256> = constraint_trees(&w.b(), &w.l, "alpha")
        .into_iter()
        .collect();
    assert_eq!(
        distinct,
        BTreeSet::from([tree("alpha", "0")]),
        "two rejections with the same recovered content give one constrained content"
    );
}

/// X: R1 → R0 over {details}; Y rejects R0 over {details, extra}, so comparing X's recovered
/// entry with Y's rejected entry is undecided and the sources decide.
fn four_branch_world() -> (World, Sha256, Sha256) {
    let mut w = World::new();
    let x = w.rej(&rej(
        1,
        5,
        &[("details", "t1")],
        &[("details", "t0")],
        "2.1",
    ));
    let y = w.rej(&rej(
        2,
        5,
        &[("details", "t0"), ("extra", "e")],
        &[("details", "t9")],
        "2.0",
    ));
    (w, x, y)
}

#[test]
fn the_recovered_chain_decides_from_sources_in_four_branches() {
    // T-R4D29-empty-recovered-copies, R4D-32
    let (w, x, y) = four_branch_world();
    let b = w.b();
    let t9 = entries(&[("details", "t9")]);

    assert_eq!(
        b.recovered_chain(&w.l, &x, None),
        Chain::Targets(vec![ChainTarget {
            rejection: x,
            folders: entries(&[("details", "t0")]),
            copies: None
        }]),
        "without sources an undecided comparison leaves X's constraint standing"
    );

    // 1: no copy at all (an empty set is never "every copy rejected")
    assert_eq!(b.recovered_chain(&w.l, &x, Some(&[])), Chain::Unavailable);
    assert_eq!(
        b.recovered_chain(&w.l, &x, Some(&[src(&[("details", "t1")])])),
        Chain::Unavailable
    );

    // 2: every copy is rejected: advance to Y
    let rejected_copy = src(&[("details", "t0"), ("extra", "e")]);
    assert_eq!(
        b.recovered_chain(&w.l, &x, Some(std::slice::from_ref(&rejected_copy))),
        Chain::Targets(vec![ChainTarget {
            rejection: y,
            folders: t9,
            copies: None
        }])
    );

    // 3: a copy is undecided and none is proven usable: wait
    let undecided = src_unknown(&[("details", "t0"), ("extra", "e")], &["extra"]);
    assert_eq!(
        b.recovered_chain(&w.l, &x, Some(std::slice::from_ref(&undecided))),
        Chain::Waiting
    );
    assert_eq!(
        b.recovered_chain(&w.l, &x, Some(&[undecided.clone(), rejected_copy.clone()])),
        Chain::Waiting
    );

    // 4: a proven usable copy stands, only that copy is a source, an undecided one never holds it back
    let usable = src(&[("details", "t0")]);
    assert_eq!(
        b.recovered_chain(&w.l, &x, Some(&[undecided, rejected_copy, usable])),
        Chain::Targets(vec![ChainTarget {
            rejection: x,
            folders: entries(&[("details", "t0")]),
            copies: Some(vec![2])
        }])
    );
}

#[test]
fn a_look_alike_of_a_blocked_version_is_found_by_label_and_different_content() {
    // T-R13
    let mut w = World::new();
    w.rej(&rej(1, 5, &[("details", "a")], &[("details", "b")], "2.1"));
    let b = w.b();
    let folders = [fk("details")];
    assert!(b
        .lookalike(&w.l, "2.1", &folders, &src(&[("details", "changed")]))
        .is_some());
    assert!(
        b.lookalike(&w.l, "2.1", &folders, &src(&[("details", "a")]))
            .is_none(),
        "identical content is the blocked version itself, not a look-alike"
    );
    assert!(b
        .lookalike(&w.l, "2.2", &folders, &src(&[("details", "changed")]))
        .is_none());
    assert!(b
        .lookalike(&w.l, "2.1", &[fk("other")], &src(&[("details", "changed")]))
        .is_none());
}

#[test]
fn a_newer_candidate_after_a_rollback_is_a_notice_unless_it_was_considered() {
    // T-R12 (Q-81)
    let mut w = World::new();
    let x = rej(1, 5, &[("details", "2.1")], &[("details", "2.0")], "2.1");
    let hx = w.rej(&x);
    let b = w.b();
    let newer = src(&[("details", "2.2")]);
    assert!(b.newer_after_rollback(&w.l, &hx, &newer, true));
    assert!(!b.newer_after_rollback(&w.l, &hx, &newer, false));
    assert!(
        !b.newer_after_rollback(&w.l, &hx, &src(&[("details", "2.0")]), true),
        "a copy the rejection was issued over never raises it"
    );
    assert!(!b.newer_after_rollback(&w.l, &sha256(b"nope"), &newer, true));
}

fn live(name: &str, tag: &str) -> LiveFolder {
    LiveFolder {
        key: fk(name),
        name: name.into(),
        tree: tree(name, tag),
        expected_live: ExpectedLive::Tree {
            tree: tree(name, tag),
            fingerprint: sha256(name.as_bytes()),
        },
    }
}

fn target(name: &str, tag: &str) -> TargetFolder {
    TargetFolder {
        key: fk(name),
        name: name.into(),
        record: FolderTreeRecord {
            tree: tree(name, tag),
            files: vec![],
        },
    }
}

fn input(r1: Vec<LiveFolder>, r0: Vec<TargetFolder>, covers: Vec<Sha256>) -> RollbackInput {
    RollbackInput {
        op: op(7),
        package: fk("details"),
        group_id: group_id(),
        issuer: issuer(1, 12),
        title: "Details".into(),
        rejected_label: "2.1".into(),
        r1,
        r0,
        considered: vec![],
        settings: RejectionSettings::Paired,
        known: Known::default(),
        audience: vec![
            EnrollmentRef {
                computer_id: computer(2),
                enrollment_id: enrollment(2),
            },
            EnrollmentRef {
                computer_id: computer(1),
                enrollment_id: enrollment(1),
            },
        ],
        readd_covers: covers,
        now_unix_ms: 100,
    }
}

fn decoded(p: &PackageApply) -> Vec<AuthorityRecord> {
    p.pending_records
        .iter()
        .map(|r| {
            assert_eq!(sha256(r.bytes.as_bytes()), r.hash);
            decode_authority(r.bytes.as_bytes(), &r.hash).expect("a valid authority record")
        })
        .collect()
}

#[test]
fn a_rollback_replaces_installs_and_removes_by_the_folder_difference() {
    // T-R01, T-R4D20-rollback-added-folder
    let p = plan_rollback(&input(
        vec![live("details", "1"), live("details_extra", "bad")],
        vec![target("details", "0")],
        vec![],
    ))
    .expect("plan");
    assert_eq!(p.kind, OpKind::Rollback);
    assert_eq!(p.folders.len(), 1);
    assert_eq!(p.folders[0].action, FolderAction::Replace);
    assert_eq!(p.removes.len(), 1);
    assert_eq!(p.removes[0].key, fk("details_extra"));
    assert_eq!(p.removals.len(), 1);
    let recs = decoded(&p);
    assert_eq!(recs.len(), 3, "rejection, removal and this computer's ack");
    let AuthorityRecord::Rejection(r) = &recs[0] else {
        panic!("first record is the rejection")
    };
    assert_eq!(r.folders, vec![fk("details"), fk("details_extra")]);
    assert_eq!(r.recovered.folders, entries(&[("details", "0")]));
    assert_eq!(r.settings, RejectionSettings::Paired);
    let AuthorityRecord::Removal(rm) = &recs[1] else {
        panic!("second is the removal")
    };
    assert_eq!(rm.evidence, RemovalEvidence::Rollback);
    assert_eq!(rm.rejection, Some(p.pending_records[0].hash));
    assert_eq!(rm.folders.len(), 1);
    assert_eq!(p.removals, vec![p.pending_records[1].hash]);
    let AuthorityRecord::Ack(a) = &recs[2] else {
        panic!("third is the ack")
    };
    assert_eq!(a.items[0].removal, p.pending_records[1].hash);
    assert_eq!(a.settings, AckSettings::Paired);

    // F0 \ F1 is installed, and a re-add is written only with covered removals to name
    let covers = vec![sha256(b"old removal")];
    let p = plan_rollback(&input(
        vec![live("details", "1")],
        vec![target("details", "0"), target("details_old", "0")],
        covers.clone(),
    ))
    .expect("plan");
    assert_eq!(
        p.folders.iter().map(|f| f.action).collect::<Vec<_>>(),
        vec![FolderAction::Replace, FolderAction::Install]
    );
    assert_eq!(p.folders[1].expected_live, ExpectedLive::Absent);
    assert!(p.removes.is_empty() && p.removals.is_empty());
    let recs = decoded(&p);
    assert_eq!(recs.len(), 2);
    let AuthorityRecord::Readd(ra) = &recs[1] else {
        panic!("second is the re-add")
    };
    assert_eq!(ra.evidence, ReaddEvidence::Rollback);
    assert_eq!(ra.known.extra, covers);
    assert_eq!(
        decoded(
            &plan_rollback(&input(
                vec![live("details", "1")],
                vec![target("details", "0"), target("details_old", "0")],
                vec![],
            ))
            .expect("plan")
        )
        .len(),
        1
    );
}

#[test]
fn a_rollback_of_the_same_release_or_of_nothing_is_refused() {
    assert_eq!(
        plan_rollback(&input(
            vec![live("details", "1")],
            vec![target("details", "1")],
            vec![]
        )),
        Err(RollbackError::SameRelease)
    );
    assert_eq!(
        plan_rollback(&input(vec![], vec![target("details", "1")], vec![])),
        Err(RollbackError::EmptyRelease)
    );
}

#[test]
fn a_planned_rollback_blocks_its_own_release_once_published() {
    // T-R02, T-R05: the plan's rejection, learned into a ledger, constrains selection
    let p = plan_rollback(&input(
        vec![live("details", "1")],
        vec![target("details", "0")],
        vec![],
    ))
    .expect("plan");
    let mut w = World::new();
    for r in &p.pending_records {
        w.l.learn(
            r.hash,
            AuthorityKind::Rejection,
            vec![],
            Some(&computer(1)),
            Some(r.bytes.as_bytes()),
        );
    }
    let b = w.b();
    assert_eq!(
        b.rejected(&w.l, &package(&["details"]), &src(&[("details", "1")])),
        ScopeMatch::Same
    );
    assert_eq!(
        constraint_trees(&b, &w.l, "details"),
        vec![tree("details", "0")]
    );
    assert_eq!(
        b.rejected(&w.l, &package(&["details"]), &src(&[("details", "0")])),
        ScopeMatch::Different
    );
}

#[test]
fn old_clients_stop_a_group_rollback() {
    // T-R15 (Q-84)
    let m = |n: u8, retired, rej, rem| MemberFeatures {
        member: EnrollmentRef {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
        },
        retired,
        has_rejection: rej,
        has_removal: rem,
    };
    let members = [
        m(1, false, true, true),
        m(2, false, true, false),
        m(3, true, false, false),
    ];
    assert!(rollback_blockers(&members, false).is_empty());
    assert_eq!(rollback_blockers(&members, true).len(), 1);
    let mut old = members.to_vec();
    old[0].has_rejection = false;
    assert_eq!(rollback_blockers(&old, false).len(), 1);
}
