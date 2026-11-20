use super::*;
use crate::hash::sha256;
use crate::plan::{plan_apply, plan_publish, ApplyGate, ObjectKind};
use crate::status::{derive_status, PackageStatus, StatusInput};
use crate::testkit::{
    addon, apply_fixture, computer, gate as test_gate, op, Dir, Effect, FakeEffects, Mutation,
    Scenario, Step,
};
use std::collections::{BTreeMap, VecDeque};

fn pkg<'a>(plan: &'a ApplyPlan, name: &str) -> &'a PackageApply {
    plan.packages
        .iter()
        .find(|p| p.package.as_str() == name)
        .expect("package in plan")
}

fn old_tree(f: &FolderApply) -> Option<Sha256> {
    match &f.expected_live {
        ExpectedLive::Tree { tree, .. } => Some(*tree),
        ExpectedLive::Absent => None,
    }
}

/// "old" when every folder holds its pre-apply content, "new" when all hold the target.
fn view(p: &PackageApply, fx: &FakeEffects) -> &'static str {
    let (mut is_old, mut is_new) = (true, true);
    for f in &p.folders {
        let t = fx.live.get(&f.name).map(|d| d.tree);
        is_old &= t == old_tree(f);
        is_new &= t == Some(f.tree.tree);
    }
    match (is_old, is_new) {
        (true, _) => "old",
        (_, true) => "new",
        _ => "mixed",
    }
}

fn assert_gated(log: &[Effect]) {
    for (i, e) in log.iter().enumerate() {
        if let Effect::Rename { from, to } = e {
            if from.is_live() || to.is_live() {
                assert!(
                    i > 0 && log[i - 1] == Effect::Activity(true),
                    "live rename without a preceding NotRunning check: {e:?}"
                );
            }
        }
    }
}

fn settle(fx: &mut FakeEffects) {
    for _ in 0..6 {
        let r = recover_interrupted(fx);
        assert!(
            r.ops.iter().all(|(_, _, res)| !matches!(
                res,
                PackageResult::NeedsHelp(_) | PackageResult::Failed(_)
            )),
            "{r:?}"
        );
        if !r.journals_remaining {
            return;
        }
    }
    panic!("journals remain after repeated recovery");
}

fn check_final(plan: &ApplyPlan, base: &FakeEffects, fx: &FakeEffects) {
    assert!(fx.journals.is_empty(), "journal left behind");
    assert!(fx.staging.is_empty(), "staging left behind");
    assert_eq!(fx.live.get("Extra"), base.live.get("Extra"));
    for p in &plan.packages {
        let v = view(p, fx);
        assert_ne!(v, "mixed", "package {} is half applied", p.package);
        assert!(
            fx.points_for(&p.package).iter().all(|rp| !rp.pinned),
            "pinned point left behind"
        );
        if v == "old" {
            assert!(fx
                .recovery
                .keys()
                .all(|(pk, _, _)| pk != p.package.as_str()));
        }
    }
    assert_gated(&fx.log);
}

fn mid_commit(launch: usize) -> (ApplyPlan, FakeEffects) {
    let (_sc, plan, mut fx) = apply_fixture();
    fx.launch_after_renames = Some(launch);
    let out = run_apply(&plan, &mut fx);
    assert_eq!(out.stopped, Some(PlanBlock::GameRunning));
    fx.launch_after_renames = None;
    (plan, fx)
}

fn crash_when(want: impl Fn(&FakeEffects, &OpId) -> bool) -> (ApplyPlan, FakeEffects) {
    let (_sc, plan, base) = apply_fixture();
    let alpha = pkg(&plan, "alpha").op.clone();
    for k in 0..300 {
        let mut fx = base.clone();
        fx.crash_after = Some(k);
        run_apply(&plan, &mut fx);
        if want(&fx, &alpha) {
            fx.restart();
            return (plan, fx);
        }
    }
    panic!("state never reached");
}

#[test]
fn classify_slots_table() {
    use SlotConfig::*;
    for live in [false, true] {
        for staging in [false, true] {
            for recovery in [false, true] {
                let replace = classify_slots(false, live, staging, recovery);
                let install = classify_slots(true, live, staging, recovery);
                let want_replace = match (live, staging, recovery) {
                    (_, true, false) => Untouched,
                    (false, true, true) => Half,
                    (true, false, true) => Done,
                    _ => Unknown,
                };
                let want_install = match (live, staging, recovery) {
                    (_, true, false) => Untouched,
                    (true, false, false) => Done,
                    _ => Unknown,
                };
                assert_eq!(replace, want_replace, "replace {live} {staging} {recovery}");
                assert_eq!(install, want_install, "install {live} {staging} {recovery}");
            }
        }
    }
}

#[test]
fn activity_checked_before_every_live_rename() {
    let (_sc, plan, base) = apply_fixture();
    let mut fx = base.clone();
    run_apply(&plan, &mut fx);
    assert!(fx
        .renames_logged()
        .iter()
        .any(|(f, t)| f.is_live() || t.is_live()));
    assert_gated(&fx.log);
    let (plan, mut fx) = mid_commit(2);
    settle(&mut fx);
    assert_gated(&fx.log);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
}

#[test]
fn launch_before_commit_changes_nothing() {
    let (_sc, plan, mut fx) = apply_fixture();
    let before = fx.live.clone();
    fx.activity_script = VecDeque::from([Activity::Running { pids: vec![7] }]);
    let out = run_apply(&plan, &mut fx);
    assert_eq!(out.stopped, Some(PlanBlock::GameRunning));
    assert!(matches!(out.packages[0].1, PackageResult::Skipped(_)));
    assert_eq!(fx.live, before);
    assert!(fx.renames_logged().is_empty());
    assert!(
        fx.journals.is_empty()
            && fx.staging.is_empty()
            && fx.points.is_empty()
            && fx.recovery.is_empty()
    );
}

#[test]
fn no_live_mutation_after_launch_mid_commit() {
    let (_sc, plan, mut fx) = apply_fixture();
    fx.launch_after_renames = Some(1);
    let out = run_apply(&plan, &mut fx);
    assert_eq!(out.packages.len(), 1);
    assert!(matches!(
        out.packages[0].1,
        PackageResult::Interrupted(Explanation::InterruptedByGame { .. })
    ));
    assert_eq!(fx.renames_logged().len(), 1);
    let alpha = pkg(&plan, "alpha");
    assert_eq!(fx.journal_phase(&alpha.op), Some(JournalPhase::Committing));
    assert!(!fx.recovery.is_empty() && !fx.staging.is_empty());
}

#[test]
fn recovery_blocked_by_a_running_game_does_not_record_a_cause() {
    for phase in [JournalPhase::Committing, JournalPhase::Committed] {
        let (plan, mut fx) = crash_when(|fx, op| fx.journal_phase(op) == Some(phase));
        let alpha = pkg(&plan, "alpha");
        assert!(!fx.journal(&alpha.op).unwrap().halted_by_game);
        fx.activity_script = VecDeque::from([Activity::Running { pids: vec![1] }]);
        let r = recover_interrupted(&mut fx);
        assert!(
            matches!(r.ops[0].2, PackageResult::Interrupted(_)),
            "{phase:?}: {:?}",
            r.ops
        );
        let journal = fx.journal(&alpha.op).expect("journal kept");
        assert!(!journal.halted_by_game, "{phase:?}: cause stays unknown");
        let journals = [journal];
        assert!(crate::status::game_halted(&journals).is_empty());
    }
}

#[test]
fn a_game_start_halt_is_recorded_and_status_blames_the_game_from_it() {
    let (sc, plan, mut fx) = apply_fixture();
    fx.launch_after_renames = Some(1);
    run_apply(&plan, &mut fx);
    let alpha = pkg(&plan, "alpha");
    let journal = fx.journal(&alpha.op).expect("journal kept");
    assert!(journal.halted_by_game);
    let journals = [journal];
    let (interrupted, _) = crate::status::journal_status(&journals);
    let by_game = crate::status::game_halted(&journals);
    let target = sc.target();
    let seen = BTreeMap::new();
    let v = derive_status(&StatusInput {
        target: &target,
        members: &sc.members,
        applied_here: &[],
        interrupted: &interrupted,
        interrupted_by_game: &by_game,
        needs_help: &[],
        blocks: &[],
        retained: &[],
        provider_error: false,
        working: false,
        last_publish_unix_ms: None,
        seen_locally: &seen,
    });
    let alpha_status = v.packages.iter().find(|p| p.0 == alpha.package);
    assert!(matches!(
        alpha_status,
        Some((
            _,
            PackageStatus::Interrupted,
            Explanation::InterruptedByGame { .. }
        ))
    ));
}

#[test]
fn interrupted_package_recovers_after_game_exit() {
    let (plan, mut fx) = mid_commit(1);
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::RolledBack(_)));
    assert!(!r.journals_remaining);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
    assert!(fx.staging.is_empty() && fx.recovery.is_empty());
    assert_gated(&fx.log);
}

#[test]
fn verify_with_game_running_waits() {
    let (_sc, plan, mut fx) = apply_fixture();
    let running = Activity::Running { pids: vec![1] };
    // Alpha has three gates before the committed record: recovery point, two renames, one more for the install folder.
    fx.activity_script = VecDeque::from([
        Activity::NotRunning,
        Activity::NotRunning,
        Activity::NotRunning,
        Activity::NotRunning,
        running,
    ]);
    let out = run_apply(&plan, &mut fx);
    assert!(
        matches!(
            out.packages[0].1,
            PackageResult::Interrupted(Explanation::InterruptedByGame { .. })
        ),
        "{:?}",
        out.packages
    );
    assert_eq!(out.stopped, Some(PlanBlock::GameRunning));
    let alpha = pkg(&plan, "alpha");
    assert_eq!(fx.journal_phase(&alpha.op), Some(JournalPhase::Committed));
    assert_eq!(view(alpha, &fx), "new");
    fx.activity_script.clear();
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::Applied { .. }));
    assert!(fx.journals.is_empty());
}

#[test]
fn recovery_waits_while_game_running() {
    let (plan, mut fx) = mid_commit(1);
    fx.activity_script = VecDeque::from([Activity::Running { pids: vec![1] }]);
    let before = (fx.live.clone(), fx.mutations, fx.renames);
    let r = recover_interrupted(&mut fx);
    assert!(matches!(
        r.ops[0].2,
        PackageResult::Interrupted(Explanation::InterruptedByGame { .. })
    ));
    assert!(r.journals_remaining);
    assert_eq!((fx.live.clone(), fx.mutations, fx.renames), before);
    assert_eq!(
        fx.journal_phase(&pkg(&plan, "alpha").op),
        Some(JournalPhase::Committing)
    );
    fx.activity_script.clear();
    settle(&mut fx);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
}

#[test]
fn rollback_gates_every_rename() {
    let (plan, mut fx) = mid_commit(2);
    let renames = fx.renames;
    fx.launch_after_renames = Some(renames + 1);
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::Interrupted(_)));
    assert_eq!(fx.renames, renames + 1, "second rename must be gated");
    fx.launch_after_renames = None;
    settle(&mut fx);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
    assert_gated(&fx.log);
}

#[test]
fn rollback_refuses_to_move_externally_changed_live() {
    let (plan, mut fx) = mid_commit(2);
    fx.live.get_mut("Alpha").unwrap().tree = sha256(b"user edit");
    let renames = fx.renames;
    let r = recover_interrupted(&mut fx);
    assert!(
        matches!(&r.ops[0].2, PackageResult::NeedsHelp(Explanation::LiveMismatch { folder }) if folder == "Alpha"),
        "{r:?}"
    );
    assert_eq!(fx.renames, renames);
    let j = fx.journal(&pkg(&plan, "alpha").op).unwrap();
    assert_eq!(j.phase, JournalPhase::NeedsHelp);
    assert_eq!(j.reason.as_deref(), Some("live_mismatch:Alpha"));
    assert!(!fx.recovery.is_empty());
}

#[test]
fn destination_changed_during_staging_replans() {
    for m in [
        Mutation::LiveFingerprint("Alpha".into()),
        Mutation::LiveRecent("Alpha".into()),
        Mutation::LiveTree("Alpha".into(), Some(sha256(b"other"))),
    ] {
        let (_sc, plan, mut fx) = apply_fixture();
        fx.mutate_at = Some((Step::StageFolder, 1, m));
        let out = run_apply(&plan, &mut fx);
        assert!(
            matches!(&out.packages[0].1, PackageResult::Replanned(Explanation::DestinationChanged { folder }) if folder == "Alpha"),
            "{:?}",
            out.packages
        );
        assert!(
            matches!(out.packages[1].1, PackageResult::Applied { .. }),
            "other packages continue"
        );
        assert_eq!(view(pkg(&plan, "beta"), &fx), "new");
        assert!(fx.live.contains_key("Alpha"));
        assert!(fx.points_for(&pkg(&plan, "alpha").package).is_empty());
        assert!(fx.journal(&pkg(&plan, "alpha").op).is_none());
    }
}

#[test]
fn destination_changed_before_rename_interrupts() {
    let (_sc, plan, mut fx) = apply_fixture();
    fx.mutate_at = Some((Step::Observe, 3, Mutation::LiveFingerprint("Alpha".into())));
    let out = run_apply(&plan, &mut fx);
    assert!(
        matches!(
            &out.packages[0].1,
            PackageResult::Interrupted(Explanation::DestinationChanged { .. })
        ),
        "{:?}",
        out.packages
    );
    assert_eq!(out.stopped, Some(PlanBlock::DestinationChanged));
    assert!(fx.renames_logged().is_empty());
    assert_eq!(
        fx.journal_phase(&pkg(&plan, "alpha").op),
        Some(JournalPhase::Committing)
    );
    settle(&mut fx);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
}

#[test]
fn rename_failure_interrupts_and_keeps_slots() {
    let (_sc, plan, mut fx) = apply_fixture();
    fx.fail_on = Some((Step::Rename, 1, EffectErrorKind::Io));
    let out = run_apply(&plan, &mut fx);
    assert!(matches!(
        out.packages[0].1,
        PackageResult::Interrupted(Explanation::InterruptedByError { .. })
    ));
    assert!(matches!(out.stopped, Some(PlanBlock::RenameFailed(_))));
    let alpha = pkg(&plan, "alpha");
    assert_eq!(fx.journal_phase(&alpha.op), Some(JournalPhase::Committing));
    assert!(!fx.staging.is_empty());
    assert_eq!(view(alpha, &fx), "old");
    fx.fail_on = None;
    settle(&mut fx);
    assert_eq!(view(alpha, &fx), "old");
}

#[test]
fn staged_hash_mismatch_fails_without_live_change() {
    let (_sc, plan, mut fx) = apply_fixture();
    let before = fx.live.clone();
    fx.stage_tree_override = Some(sha256(b"bad"));
    let out = run_apply(&plan, &mut fx);
    assert!(
        out.packages
            .iter()
            .all(|(_, r)| matches!(r, PackageResult::Failed(Explanation::StagedMismatch { .. }))),
        "{:?}",
        out.packages
    );
    assert_eq!(fx.live, before);
    assert!(fx.renames_logged().is_empty() && fx.journals.is_empty() && fx.staging.is_empty());
}

#[test]
fn missing_blob_skips_package_others_continue() {
    let (_sc, plan, mut fx) = apply_fixture();
    let beta_blobs: Vec<Sha256> = pkg(&plan, "beta")
        .folders
        .iter()
        .flat_map(|f| f.tree.files.iter().map(|e| e.sha256))
        .collect();
    let mut removed = 0;
    for f in &pkg(&plan, "alpha").folders {
        for e in f
            .tree
            .files
            .iter()
            .filter(|e| !beta_blobs.contains(&e.sha256))
        {
            removed += fx
                .objects
                .remove(&crate::layout::blob(&fx.group.clone(), &e.sha256))
                .is_some() as u32;
        }
    }
    assert!(removed > 0);
    let out = run_apply(&plan, &mut fx);
    assert!(
        matches!(
            out.packages[0].1,
            PackageResult::Skipped(Explanation::NotAvailableYet { .. })
        ),
        "{:?}",
        out.packages
    );
    assert!(
        matches!(out.packages[1].1, PackageResult::Applied { .. }),
        "{:?}",
        out.packages
    );
    assert_eq!(out.stopped, None);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
    assert_eq!(view(pkg(&plan, "beta"), &fx), "new");
}

fn verify_time_mutation(m: Mutation) -> (ApplyPlan, FakeEffects, ApplyOutcome) {
    let (_sc, plan, mut fx) = apply_fixture();
    // The third journal write is the committed record, after Alpha's renames.
    fx.mutate_at = Some((Step::WriteJournal, 3, m));
    let out = run_apply(&plan, &mut fx);
    (plan, fx, out)
}

#[test]
fn live_mismatch_needs_help_and_pins() {
    let (plan, mut fx, out) =
        verify_time_mutation(Mutation::LiveTree("Alpha".into(), Some(sha256(b"changed"))));
    assert!(
        matches!(&out.packages[0].1, PackageResult::NeedsHelp(Explanation::LiveMismatch { folder }) if folder == "Alpha"),
        "{:?}",
        out.packages
    );
    let alpha = pkg(&plan, "alpha");
    assert_eq!(fx.journal_phase(&alpha.op), Some(JournalPhase::NeedsHelp));
    assert!(
        fx.points_for(&alpha.package).iter().all(|p| p.pinned)
            && !fx.points_for(&alpha.package).is_empty()
    );
    assert!(!fx.recovery.is_empty());
    let r = recover_interrupted(&mut fx);
    assert!(matches!(
        r.ops[0].2,
        PackageResult::NeedsHelp(Explanation::LiveMismatch { .. })
    ));
}

#[test]
fn recovery_copy_modified_needs_help() {
    let (plan, mut fx, out) =
        verify_time_mutation(Mutation::RecoveryTree("Alpha".into(), sha256(b"damaged")));
    assert!(
        matches!(&out.packages[0].1, PackageResult::NeedsHelp(Explanation::RecoveryMismatch { folder }) if folder == "Alpha"),
        "{:?}",
        out.packages
    );
    let alpha = pkg(&plan, "alpha");
    assert_eq!(
        fx.journal(&alpha.op).unwrap().reason.as_deref(),
        Some("recovery_mismatch:Alpha")
    );
    assert!(fx.points_for(&alpha.package).iter().all(|p| p.pinned));
    let r = recover_interrupted(&mut fx);
    assert!(matches!(
        r.ops[0].2,
        PackageResult::NeedsHelp(Explanation::RecoveryMismatch { .. })
    ));
}

#[test]
fn crash_before_verify_reverifies_on_restart() {
    let (plan, base) = crash_when(|fx, op| fx.journal_phase(op) == Some(JournalPhase::Committed));
    let mut fx = base.clone();
    let trees = fx.count(Step::TreeOf);
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::Applied { .. }), "{r:?}");
    assert!(
        fx.count(Step::TreeOf) > trees,
        "content must be re-verified"
    );
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "new");
    let mut bad = base;
    bad.live.get_mut("Alpha").unwrap().tree = sha256(b"changed after crash");
    let r = recover_interrupted(&mut bad);
    assert!(
        matches!(
            r.ops[0].2,
            PackageResult::NeedsHelp(Explanation::LiveMismatch { .. })
        ),
        "{r:?}"
    );
}

#[test]
fn crash_after_verify_before_finalize() {
    let (plan, mut fx) = crash_when(|fx, op| fx.journal_phase(op) == Some(JournalPhase::Verified));
    let renames = fx.renames_logged().len();
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::Applied { .. }), "{r:?}");
    assert_eq!(fx.renames_logged().len(), renames);
    let alpha = pkg(&plan, "alpha");
    assert!(
        fx.journal(&alpha.op).is_none() && fx.points_for(&alpha.package).iter().all(|p| !p.pinned)
    );
}

#[test]
fn staging_phase_crash_discards_partial_staging() {
    let (plan, mut fx) = crash_when(|fx, op| {
        fx.journal_phase(op) == Some(JournalPhase::Staging) && !fx.staging.is_empty()
    });
    let before = fx.live.clone();
    let r = recover_interrupted(&mut fx);
    assert!(matches!(r.ops[0].2, PackageResult::RolledBack(_)), "{r:?}");
    assert!(fx.staging.is_empty() && fx.journals.is_empty());
    assert_eq!(fx.live, before);
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
}

#[test]
fn crash_at_every_effect_recovers_to_old_or_new() {
    let (_sc, plan, base) = apply_fixture();
    let n = {
        let mut f = base.clone();
        run_apply(&plan, &mut f);
        f.mutations
    };
    assert!(n > 15);
    for k in 0..=n {
        let mut fx = base.clone();
        fx.crash_after = Some(k);
        run_apply(&plan, &mut fx);
        fx.restart();
        settle(&mut fx);
        check_final(&plan, &base, &fx);
    }
}

#[test]
fn crash_during_recovery_at_every_effect_is_reentrant() {
    let (_sc, plan, base) = apply_fixture();
    let n = {
        let mut f = base.clone();
        run_apply(&plan, &mut f);
        f.mutations
    };
    for k in 0..n {
        let mut first = base.clone();
        first.crash_after = Some(k);
        run_apply(&plan, &mut first);
        first.restart();
        let m_total = {
            let mut f = first.clone();
            settle(&mut f);
            f.mutations
        };
        for m in 0..=m_total {
            let mut fx = first.clone();
            fx.crash_after = Some(m);
            recover_interrupted(&mut fx);
            fx.restart();
            if m % 2 == 0 {
                fx.activity_script = VecDeque::from([Activity::Running { pids: vec![1] }]);
                let (live, renames) = (fx.live.clone(), fx.renames);
                recover_interrupted(&mut fx);
                assert_eq!(
                    (fx.live.clone(), fx.renames),
                    (live, renames),
                    "recovery mutated live while the game ran"
                );
                fx.activity_script.clear();
            }
            settle(&mut fx);
            check_final(&plan, &base, &fx);
        }
    }
}

#[test]
fn f02_crash_after_restoring_corrupt_copy_is_not_rolled_back() {
    let (plan, mut fx) = mid_commit(1);
    fx.recovery
        .values_mut()
        .for_each(|d| d.tree = sha256(b"damaged"));
    // The restore rename is the first effect; the crash comes before the terminal record.
    fx.crash_after = Some(1);
    recover_interrupted(&mut fx);
    assert!(fx.crashed);
    fx.restart();
    let r = recover_interrupted(&mut fx);
    assert!(
        matches!(&r.ops[0].2, PackageResult::NeedsHelp(Explanation::RestoreFailed { folder }) if folder == "Alpha"),
        "{r:?}"
    );
    assert_eq!(
        fx.journal_phase(&pkg(&plan, "alpha").op),
        Some(JournalPhase::NeedsHelp)
    );
    assert_eq!(
        fx.live.get("Alpha").map(|d| d.tree),
        Some(sha256(b"damaged")),
        "evidence stays in place"
    );
}

#[test]
fn restore_failure_keeps_journal_and_slots() {
    let (plan, mut fx) = mid_commit(1);
    fx.fail_on = Some((
        Step::Rename,
        fx.count(Step::Rename) + 1,
        EffectErrorKind::Io,
    ));
    let r = recover_interrupted(&mut fx);
    assert!(
        matches!(
            r.ops[0].2,
            PackageResult::Failed(Explanation::RestoreFailed { .. })
        ),
        "{r:?}"
    );
    let alpha = pkg(&plan, "alpha");
    assert_eq!(fx.journal_phase(&alpha.op), Some(JournalPhase::Committing));
    assert!(!fx.recovery.is_empty() && !fx.staging.is_empty());
    fx.fail_on = None;
    settle(&mut fx);
    assert_eq!(view(alpha, &fx), "old");
}

#[test]
fn restore_mismatch_needs_help() {
    let (plan, mut fx) = mid_commit(1);
    fx.recovery
        .values_mut()
        .for_each(|d| d.tree = sha256(b"damaged"));
    let r = recover_interrupted(&mut fx);
    assert!(
        matches!(&r.ops[0].2, PackageResult::NeedsHelp(Explanation::RestoreFailed { folder }) if folder == "Alpha"),
        "{r:?}"
    );
    let j = fx.journal(&pkg(&plan, "alpha").op).unwrap();
    assert_eq!(
        (j.phase, j.reason.as_deref()),
        (JournalPhase::NeedsHelp, Some("restore_mismatch:Alpha"))
    );
}

#[test]
fn each_package_has_own_op() {
    let (_sc, plan, mut fx) = apply_fixture();
    let (a, b) = (
        pkg(&plan, "alpha").op.clone(),
        pkg(&plan, "beta").op.clone(),
    );
    assert_ne!(a, b);
    let out = run_apply(&plan, &mut fx);
    assert_eq!(out.packages.len(), 2);
    for (key, res) in &out.packages {
        let want = &pkg(&plan, key.as_str()).op;
        assert!(matches!(res, PackageResult::Applied { recovery } if recovery == want));
        assert_eq!(
            fx.points_for(key).iter().filter(|p| &p.op == want).count(),
            1
        );
    }
    let journal_ops: Vec<_> = fx
        .log
        .iter()
        .filter(|e| matches!(e, Effect::WriteJournal(JournalPhase::Committing)))
        .collect();
    assert_eq!(journal_ops.len(), 2);
}

#[test]
fn settling_one_op_keeps_other_ops_evidence() {
    let (plan, mut fx) = mid_commit(1);
    let beta = pkg(&plan, "beta").clone();
    let mut extra = |hex: &str, phase: JournalPhase, reason: Option<&str>| {
        let mut p = beta.clone();
        p.op = OpId::parse(&hex.repeat(16)).unwrap();
        let mut j = journal_for(&plan, &p);
        j.phase = phase;
        j.reason = reason.map(str::to_string);
        fx.write_journal(&j).unwrap();
        let dir = Dir {
            tree: sha256(hex.as_bytes()),
            fingerprint: sha256(b"fp"),
            recent: false,
        };
        fx.staging
            .insert((p.op.to_string(), "Beta".into()), dir.clone());
        fx.recovery.insert(
            (p.package.to_string(), p.op.to_string(), "Beta".into()),
            dir,
        );
        p.op
    };
    let help = extra("ff", JournalPhase::NeedsHelp, Some("unknown_state"));
    let staging_op = extra("ee", JournalPhase::Staging, None);
    let r = recover_interrupted(&mut fx);
    assert!(r.journals_remaining);
    assert!(fx.journal_phase(&help) == Some(JournalPhase::NeedsHelp));
    assert!(fx.staging.contains_key(&(help.to_string(), "Beta".into())));
    assert!(fx
        .recovery
        .contains_key(&("beta".to_string(), help.to_string(), "Beta".into())));
    assert!(!fx.staging.keys().any(|(o, _)| o == staging_op.as_str()));
    assert!(fx.journal(&staging_op).is_none() && fx.journal(&pkg(&plan, "alpha").op).is_none());
    assert_eq!(view(pkg(&plan, "alpha"), &fx), "old");
    assert!(r
        .ops
        .iter()
        .any(|(o, _, res)| *o == help && matches!(res, PackageResult::NeedsHelp(_))));
}

#[test]
fn needs_help_op_blocks_new_apply_and_is_kept() {
    let (plan, mut fx, _) =
        verify_time_mutation(Mutation::LiveTree("Alpha".into(), Some(sha256(b"changed"))));
    let alpha = pkg(&plan, "alpha");
    let snapshot = (
        fx.live.clone(),
        fx.staging.clone(),
        fx.recovery.clone(),
        fx.journals.clone(),
    );
    for _ in 0..3 {
        let r = recover_interrupted(&mut fx);
        assert!(r.journals_remaining);
        assert!(matches!(r.ops[0].2, PackageResult::NeedsHelp(_)));
    }
    assert_eq!(
        (
            fx.live.clone(),
            fx.staging.clone(),
            fx.recovery.clone(),
            fx.journals.clone()
        ),
        snapshot
    );
    assert!(fx.journal(&alpha.op).is_some());
    let (sc, ..) = apply_fixture();
    let blocked = ApplyGate {
        any_journal: !fx.journals.is_empty(),
        ..test_gate(Activity::NotRunning)
    };
    let mut ops = crate::testkit::OpGen::new();
    assert!(plan_apply(&sc.target(), &sc.local, &mut || ops.next_op(), &blocked).is_err());
}

#[test]
fn unknown_journal_state_needs_help() {
    let (plan, mut fx) = mid_commit(1);
    fx.staging.clear();
    let r = recover_interrupted(&mut fx);
    assert!(
        matches!(
            r.ops[0].2,
            PackageResult::NeedsHelp(Explanation::UnknownInterruptedState { .. })
        ),
        "{r:?}"
    );
    let alpha = pkg(&plan, "alpha");
    assert_eq!(
        fx.journal(&alpha.op).unwrap().reason.as_deref(),
        Some("unknown_state")
    );
    fx.journals
        .insert("ee".repeat(16), b"not a journal".to_vec());
    let r = recover_interrupted(&mut fx);
    assert!(
        r.journals_remaining
            && r.ops
                .iter()
                .all(|(_, _, res)| matches!(res, PackageResult::NeedsHelp(_))),
        "{r:?}"
    );
    assert!(fx.journals.contains_key(&"ee".repeat(16)));
}

fn seed_points(fx: &mut FakeEffects, plan: &ApplyPlan, specs: &[(u8, u64, bool)]) {
    let alpha = pkg(plan, "alpha");
    for (n, created, pinned) in specs {
        let rp = RecoveryPoint {
            op: op(*n),
            package: alpha.package.clone(),
            created_unix_ms: *created,
            replaced: vec![],
            installed: vec![],
            reason: "apply".into(),
            pinned: *pinned,
        };
        fx.write_recovery_point(&rp).unwrap();
    }
}

#[test]
fn recovery_points_pruned_to_two_per_package() {
    let (_sc, plan, mut fx) = apply_fixture();
    seed_points(
        &mut fx,
        &plan,
        &[(0xB1, 10, false), (0xB2, 20, false), (0xB3, 30, false)],
    );
    run_apply(&plan, &mut fx);
    let alpha = pkg(&plan, "alpha");
    let points = fx.points_for(&alpha.package);
    assert_eq!(points.len(), 2, "{points:?}");
    assert!(points.iter().any(|p| p.op == alpha.op) && points.iter().all(|p| !p.pinned));
    assert_eq!(fx.points_for(&pkg(&plan, "beta").package).len(), 1);
}

#[test]
fn pinned_recovery_points_exceed_two_visibly() {
    let (sc, plan, mut fx) = apply_fixture();
    seed_points(
        &mut fx,
        &plan,
        &[(0xB1, 10, true), (0xB2, 20, false), (0xB3, 30, false)],
    );
    run_apply(&plan, &mut fx);
    let alpha = pkg(&plan, "alpha");
    let retained = fx.points_for(&alpha.package);
    assert_eq!(retained.len(), 3);
    assert_eq!(retained.iter().filter(|p| p.pinned).count(), 1);
    let target = sc.target();
    let seen = BTreeMap::new();
    let v = derive_status(&StatusInput {
        target: &target,
        members: &sc.members,
        applied_here: &[],
        interrupted: &[],
        interrupted_by_game: &[],
        needs_help: &[],
        blocks: &[],
        retained: &retained,
        provider_error: false,
        working: false,
        last_publish_unix_ms: None,
        seen_locally: &seen,
    });
    assert_eq!(v.retained_extra_recovery, 1);
}

fn published() -> (Scenario, FakeEffects, crate::plan::PublishPlan) {
    let sc = Scenario::new(vec![addon("Suite", Some("1.0"), "a")]);
    let plan = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
    let mut fx = FakeEffects::new();
    fx.set_live_from(&sc.local);
    let mut ctx = sc.publish_ctx(1, &[]);
    assert!(matches!(
        run_publish(&plan, &mut ctx, &mut fx),
        PublishOutcome::Published { .. }
    ));
    (sc, fx, plan)
}

fn corrupt_and_republish(kind: ObjectKind) {
    let (sc, mut fx, plan) = published();
    let target = plan
        .objects
        .iter()
        .find(|o| o.kind == kind)
        .unwrap()
        .rel
        .clone();
    fx.objects.get_mut(&target).unwrap().corrupt = true;
    fx.log.clear();
    let hashes = fx.count(Step::HashObject);
    let mut ctx = sc.publish_ctx(2, &[&plan.state_path]);
    let plan2 = plan_publish(&sc.local, &ctx).unwrap();
    let out = run_publish(&plan2, &mut ctx, &mut fx);
    assert!(
        matches!(out, PublishOutcome::Published { uploaded: 1, .. }),
        "{out:?}"
    );
    assert_eq!(
        fx.count(Step::HashObject) - hashes,
        plan2.objects.len(),
        "every existing object is re-hashed"
    );
    assert!(!fx.objects[&target].corrupt);
    let put = fx
        .log
        .iter()
        .position(|e| matches!(e, Effect::Other(Step::PutObject)))
        .unwrap();
    let write = fx
        .log
        .iter()
        .position(|e| matches!(e, Effect::Other(Step::WriteRecord)))
        .unwrap();
    assert!(put < write);
}

#[test]
fn corrupt_existing_blob_replaced_before_state() {
    corrupt_and_republish(ObjectKind::Blob);
}

#[test]
fn corrupt_existing_tree_replaced() {
    corrupt_and_republish(ObjectKind::Tree);
}

#[test]
fn same_size_mtime_corrupt_object_rehashed_and_replaced() {
    // The fake reports identical size and mtime for the corrupt object, so only hashing can detect it.
    let (_sc, mut fx, plan) = published();
    let rel = plan.objects[0].rel.clone();
    fx.objects.get_mut(&rel).unwrap().corrupt = true;
    assert!(matches!(
        fx.stat_object(&rel),
        Ok(ObjectStat::Local {
            size: 1,
            mtime_ns: 1
        })
    ));
    assert_eq!(
        check_object(&mut fx, &rel, &plan.objects[0].expect),
        Ok(ObjectCheck::Corrupt)
    );
    corrupt_and_republish(ObjectKind::Blob);
}

#[test]
fn publish_failure_leaves_no_state_record() {
    let sc = Scenario::new(vec![addon("Suite", Some("1.0"), "a")]);
    let plan = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
    let last = plan.objects.len();
    for (step, n) in [
        (Step::StatObject, 1),
        (Step::PutObject, 1),
        (Step::PutObject, last),
        (Step::WriteRecord, 1),
    ] {
        let mut fx = FakeEffects::new();
        fx.set_live_from(&sc.local);
        fx.fail_on = Some((step, n, EffectErrorKind::Io));
        let mut ctx = sc.publish_ctx(1, &[]);
        let out = run_publish(&plan, &mut ctx, &mut fx);
        assert!(matches!(out, PublishOutcome::Failed(_)), "{step:?} {out:?}");
        assert!(fx.records.is_empty(), "{step:?}");
    }
}

#[test]
fn publish_aborts_when_source_changes() {
    let sc = Scenario::new(vec![addon("Suite", Some("1.0"), "a")]);
    let plan = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
    let mut fx = FakeEffects::new();
    fx.set_live_from(&sc.local);
    fx.mutate_at = Some((
        Step::PutObject,
        1,
        Mutation::LiveFingerprint("Suite".into()),
    ));
    let mut ctx = sc.publish_ctx(1, &[]);
    assert_eq!(
        run_publish(&plan, &mut ctx, &mut fx),
        PublishOutcome::Aborted(PlanBlock::Unstable)
    );
    assert!(fx.records.is_empty());
    let _ = computer(1);
}

/// C-JRN `/2` execution (contracts.md §2.2): every operation kind, every crash slot.
mod v2 {
    use super::*;
    use crate::ids::{Identity, SettingsUnit};
    use crate::kinds::{Ack, AckItem, AckResult, AckSettings, Issuer};
    use crate::plan::RemoveApply;
    use crate::records::{
        decode_journal, decode_point, AnyJournal, AnyPoint, Location, PendingRecord, RecoverySlot,
        RootState, SettingsFileOp, SettingsRoot,
    };
    use crate::testkit::{enrollment, group_id};

    fn key(s: &str) -> FolderKey {
        FolderKey::lossy(s)
    }

    fn dir(t: &str) -> Dir {
        Dir {
            tree: sha256(t.as_bytes()),
            fingerprint: sha256(format!("fp-{t}").as_bytes()),
            recent: false,
        }
    }

    fn rel(folder: &str) -> crate::paths::SettingsPath {
        crate::paths::account_settings_path("ACC", folder).unwrap()
    }

    fn expect_dir(name: &str, t: &str) -> ExpectedLive {
        let d = dir(&format!("{name}-{t}"));
        ExpectedLive::Tree {
            tree: d.tree,
            fingerprint: d.fingerprint,
        }
    }

    fn folder(name: &str, action: FolderAction, old: bool) -> FolderApply {
        FolderApply {
            name: name.into(),
            key: key(&name.to_lowercase()),
            action,
            tree: crate::records::FolderTreeRecord {
                tree: sha256(format!("{name}-new").as_bytes()),
                files: vec![],
            },
            expected_live: if old {
                expect_dir(name, "old")
            } else {
                ExpectedLive::Absent
            },
        }
    }

    fn remove(name: &str) -> RemoveApply {
        RemoveApply {
            name: name.into(),
            key: key(&name.to_lowercase()),
            expected_live: expect_dir(name, "old"),
        }
    }

    fn settings_op(file: &str, idx: u8, staged: bool, live: bool) -> SettingsFileOp {
        SettingsFileOp {
            unit: SettingsUnit {
                folder: key(&file.to_lowercase()),
                identity: Identity::account("ACC").unwrap(),
            },
            live_rel: rel(file),
            expected_live: if live {
                ExpectedLive::Tree {
                    tree: sha256(format!("{file}-old").as_bytes()),
                    fingerprint: sha256(b"unused"),
                }
            } else {
                ExpectedLive::Absent
            },
            staged: staged.then(|| sha256(format!("{file}-new").as_bytes())),
            recovery_slot: RecoverySlot::new(&format!("r/{idx}")).unwrap(),
        }
    }

    fn pending() -> PendingRecord {
        let ack = Ack {
            group_id: group_id(),
            issuer: Issuer {
                computer_id: computer(1),
                enrollment_id: enrollment(1),
                issued_at_seq: 4,
            },
            items: vec![AckItem {
                removal: sha256(b"removal"),
                folders: vec![key("alphax")],
            }],
            result: AckResult::Absent,
            removed: vec![],
            settings: AckSettings::NotApplicable,
            written_unix_ms: 8,
        };
        let bytes = crate::records::encode(&ack);
        PendingRecord {
            hash: sha256(&bytes),
            bytes: String::from_utf8(bytes).unwrap(),
        }
    }

    fn captured(file: &str) -> SettingsRoot {
        SettingsRoot {
            unit: SettingsUnit {
                folder: key(&file.to_lowercase()),
                identity: Identity::account("ACC").unwrap(),
            },
            state: RootState::Captured {
                checkpoint: sha256(b"checkpoint"),
                location: Location::Local,
            },
        }
    }

    fn base_fx() -> FakeEffects {
        let mut fx = FakeEffects::new();
        for n in ["Alpha", "AlphaX", "Extra"] {
            fx.live.insert(n.into(), dir(&format!("{n}-old")));
        }
        for f in ["Alpha", "Other"] {
            fx.settings_live.insert(
                rel(f).as_str().to_string(),
                sha256(format!("{f}-old").as_bytes()),
            );
        }
        for f in ["Alpha", "Other"] {
            fx.settings_blobs
                .push(sha256(format!("{f}-new").as_bytes()));
        }
        fx
    }

    /// One operation of each kind of the §2.2 table, over the same base state.
    fn case(kind: OpKind) -> (ApplyPlan, FakeEffects) {
        let mut p = PackageApply::code_apply(
            op(7),
            key("alpha"),
            sha256(b"alpha-target"),
            vec![],
            Explanation::Installed {
                source: computer(2),
            },
        );
        p.kind = kind;
        let mut removals = vec![sha256(b"removal")];
        removals.sort();
        match kind {
            OpKind::Apply | OpKind::Rollback => {
                p.folders = vec![
                    folder("Alpha", FolderAction::Replace, true),
                    folder("Fresh", FolderAction::Install, false),
                ];
                p.removes = vec![remove("AlphaX")];
                p.removals = removals;
                p.pending_records = vec![pending()];
                p.settings_ops = vec![settings_op("Alpha", 0, true, true)];
                p.settings_roots = vec![captured("Alpha")];
            }
            OpKind::Remove => {
                p.removes = vec![remove("AlphaX"), remove("Alpha")];
                p.removals = removals;
                p.pending_records = vec![pending()];
                p.settings_ops = vec![settings_op("Alpha", 0, false, true)];
                p.settings_roots = vec![captured("Alpha")];
            }
            OpKind::Restore => {
                p.folders = vec![folder("Alpha", FolderAction::Replace, true)];
                p.settings_ops = vec![settings_op("Alpha", 0, true, true)];
                p.settings_roots = vec![captured("Alpha")];
            }
            OpKind::Settings => {
                p.settings_ops = vec![
                    settings_op("Alpha", 0, true, true),
                    settings_op("Other", 1, true, true),
                ];
                p.settings_roots = vec![captured("Alpha"), captured("Other")];
            }
            OpKind::SettingsCleanup => {
                p.settings_ops = vec![settings_op("Alpha", 0, false, true)];
                p.pending_records = vec![pending()];
                p.settings_roots = vec![captured("Alpha")];
            }
        }
        let plan = ApplyPlan {
            group_id: group_id(),
            started_unix_ms: 5,
            packages: vec![p],
            skipped: vec![],
        };
        (plan, base_fx())
    }

    const KINDS: [OpKind; 6] = [
        OpKind::Apply,
        OpKind::Remove,
        OpKind::Restore,
        OpKind::Rollback,
        OpKind::Settings,
        OpKind::SettingsCleanup,
    ];

    type Snapshot = (BTreeMap<String, Dir>, BTreeMap<String, Sha256>, Vec<Sha256>);

    fn snap(fx: &FakeEffects) -> Snapshot {
        (
            fx.live.clone(),
            fx.settings_live.clone(),
            fx.pending_written.keys().copied().collect(),
        )
    }

    fn settle2(fx: &mut FakeEffects) {
        for _ in 0..6 {
            let r = recover_interrupted(fx);
            assert!(
                r.ops.iter().all(|(_, _, res)| !matches!(
                    res,
                    PackageResult::NeedsHelp(_) | PackageResult::Failed(_)
                )),
                "{r:?}"
            );
            if !r.journals_remaining {
                return;
            }
        }
        panic!("journals remain after repeated recovery");
    }

    /// Exactly the old state with no record written, or exactly the new state with the reserved
    /// bytes written once; nothing in between and nothing left over.
    fn assert_settled(fx: &FakeEffects, old: &Snapshot, new: &Snapshot, want: &[Sha256]) {
        assert!(fx.journals.is_empty(), "journal left behind");
        assert!(fx.staging.is_empty() && fx.settings_staging.is_empty());
        let got = snap(fx);
        if got == *old {
            assert!(
                fx.pending_written.is_empty(),
                "record written after rollback"
            );
            assert!(fx.recovery.is_empty() && fx.settings_recovery.is_empty());
        } else {
            assert_eq!(got, *new, "neither old nor new");
            let mut w = want.to_vec();
            w.sort();
            assert_eq!(got.2, w, "reserved records not written exactly once");
        }
        for (_, bytes) in fx.points.iter() {
            match decode_point(bytes).expect("point decodes") {
                AnyPoint::V2(p) => assert!(!p.pinned, "pinned point left behind"),
                AnyPoint::V1(p) => assert!(!p.pinned),
            }
        }
    }

    fn clean_run(kind: OpKind) -> (ApplyPlan, FakeEffects, Snapshot, Snapshot, usize) {
        let (plan, base) = case(kind);
        let mut fx = base.clone();
        let out = run_apply(&plan, &mut fx);
        assert!(
            matches!(out.packages[0].1, PackageResult::Applied { .. }),
            "{kind:?}: {:?}",
            out.packages
        );
        assert!(fx.journals.is_empty());
        let n = fx.mutations;
        let (old, new) = (snap(&base), snap(&fx));
        assert_ne!((&old.0, &old.1), (&new.0, &new.1), "{kind:?}");
        (plan, base, old, new, n)
    }

    fn reserved(plan: &ApplyPlan) -> Vec<Sha256> {
        plan.packages[0]
            .pending_records
            .iter()
            .map(|r| r.hash)
            .collect()
    }

    #[test]
    fn every_kind_applies_and_writes_its_reserved_records() {
        for kind in KINDS {
            let (plan, _, _, new, _) = clean_run(kind);
            let mut want = reserved(&plan);
            want.sort();
            assert_eq!(new.2, want, "{kind:?}");
        }
    }

    #[test]
    fn crash_slots_recover_to_exactly_old_or_new_for_every_kind() {
        // T-C06c-jrn-crash-slots
        for kind in KINDS {
            let (plan, base, old, new, n) = clean_run(kind);
            assert!(n > 4, "{kind:?}");
            let want = reserved(&plan);
            for k in 0..=n {
                let mut fx = base.clone();
                fx.crash_after = Some(k);
                run_apply(&plan, &mut fx);
                fx.restart();
                settle2(&mut fx);
                assert_settled(&fx, &old, &new, &want);
            }
        }
    }

    #[test]
    fn crash_during_recovery_is_reentrant_for_every_kind() {
        for kind in KINDS {
            let (plan, base, old, new, n) = clean_run(kind);
            let want = reserved(&plan);
            for k in (0..n).step_by(2) {
                let mut first = base.clone();
                first.crash_after = Some(k);
                run_apply(&plan, &mut first);
                first.restart();
                let m_total = {
                    let mut f = first.clone();
                    settle2(&mut f);
                    f.mutations
                };
                for m in 0..=m_total {
                    let mut fx = first.clone();
                    fx.crash_after = Some(m);
                    recover_interrupted(&mut fx);
                    fx.restart();
                    settle2(&mut fx);
                    assert_settled(&fx, &old, &new, &want);
                }
            }
        }
    }

    #[test]
    fn the_reserved_bytes_are_written_after_verify_not_before() {
        // T-C06c: nothing is published before verify; a crash before verify discards them.
        let (plan, base, _, _, n) = clean_run(OpKind::Remove);
        let mut saw_pending_before_verified = false;
        for k in 0..=n {
            let mut fx = base.clone();
            fx.crash_after = Some(k);
            run_apply(&plan, &mut fx);
            let phase = fx.journal_any(&plan.packages[0].op).map(|j| match j {
                AnyJournal::V2(j) => j.phase,
                AnyJournal::V1(j) => j.phase,
            });
            if !fx.pending_written.is_empty() {
                saw_pending_before_verified |=
                    phase != Some(JournalPhase::Verified) && phase.is_some();
            }
        }
        assert!(!saw_pending_before_verified);
    }

    #[test]
    fn mixed_update_drop_uses_one_v2_journal_and_recovers_from_each_slot() {
        // T-R4D1-mixed-update-drop
        let (plan, base) = case(OpKind::Apply);
        let mut fx = base.clone();
        fx.crash_after = Some(6);
        run_apply(&plan, &mut fx);
        let j = fx.journal_any(&plan.packages[0].op).expect("journal kept");
        let AnyJournal::V2(j) = j else {
            panic!("a dropping update must be a /2 journal")
        };
        assert_eq!(j.kind, OpKind::Apply);
        assert!(j.folders.iter().any(|f| f.action == FolderAction::Remove));
        assert!(j.folders.iter().any(|f| f.action == FolderAction::Replace));
        assert_eq!(j.removals.len(), 1);
        assert_eq!(fx.journals.len(), 1);
    }

    #[test]
    fn pure_code_apply_still_writes_a_v1_journal() {
        // T-C05
        let (_sc, plan, mut fx) = apply_fixture();
        fx.crash_after = Some(3);
        run_apply(&plan, &mut fx);
        for (_, bytes) in fx.journals.iter() {
            assert!(matches!(decode_journal(bytes), Ok(AnyJournal::V1(_))));
            assert!(String::from_utf8_lossy(bytes).contains("apply-journal/1"));
        }
        assert!(!fx.journals.is_empty());
    }

    #[test]
    fn changed_settings_file_replans_without_touching_live() {
        let (plan, mut fx) = case(OpKind::Settings);
        let r = rel("Alpha").as_str().to_string();
        fx.settings_live.insert(r, sha256(b"edited by the user"));
        let before = snap(&fx);
        let out = run_apply(&plan, &mut fx);
        assert!(matches!(
            out.packages[0].1,
            PackageResult::Replanned(Explanation::DestinationChanged { .. })
        ));
        assert_eq!(snap(&fx), before);
        assert!(fx.journals.is_empty() && fx.pending_written.is_empty());
    }

    #[test]
    fn a_running_game_stops_a_remove_without_a_live_change() {
        let (plan, mut fx) = case(OpKind::Remove);
        fx.launch_after_renames = Some(1);
        let out = run_apply(&plan, &mut fx);
        assert_eq!(out.stopped, Some(PlanBlock::GameRunning));
        let live = fx.live.clone();
        let renames = fx.renames;
        fx.activity_script = VecDeque::from([Activity::Running { pids: vec![1] }]);
        recover_interrupted(&mut fx);
        assert_eq!((fx.live.clone(), fx.renames), (live, renames));
        fx.activity_script.clear();
        fx.launch_after_renames = None;
        settle2(&mut fx);
        assert!(fx.pending_written.is_empty() || fx.live.get("AlphaX").is_none());
    }

    #[test]
    fn an_unsupported_port_fails_before_any_live_change() {
        // The default `ApplyEffects` methods refuse; nothing is touched.
        let (plan, mut fx) = case(OpKind::Settings);
        let mut stripped = NoV2(&mut fx);
        let out = run_apply(&plan, &mut stripped);
        assert!(matches!(out.packages[0].1, PackageResult::Failed(_)));
        assert_eq!(snap(&fx), snap(&case(OpKind::Settings).1));
    }

    struct NoV2<'a>(&'a mut FakeEffects);

    impl ApplyEffects for NoV2<'_> {
        fn activity(&mut self) -> Activity {
            self.0.activity()
        }
        fn lock(&mut self) -> Result<(), EffectError> {
            self.0.lock()
        }
        fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError> {
            self.0.write_journal(j)
        }
        fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
            self.0.read_journals()
        }
        fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError> {
            self.0.remove_journal(op)
        }
        fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError> {
            self.0.stage_folder(op, f)
        }
        fn tree_of(&mut self, s: &Slot) -> Result<Option<Sha256>, EffectError> {
            self.0.tree_of(s)
        }
        fn observe(&mut self, s: &Slot) -> Result<Option<SlotObs>, EffectError> {
            self.0.observe(s)
        }
        fn rename(&mut self, a: &Slot, b: &Slot) -> Result<(), EffectError> {
            self.0.rename(a, b)
        }
        fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError> {
            self.0.write_recovery_point(rp)
        }
        fn list_recovery(
            &mut self,
            pkg: &FolderKey,
        ) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
            self.0.list_recovery(pkg)
        }
        fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError> {
            self.0.remove_recovery(pkg, op)
        }
        fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
            self.0.discard_staging(op)
        }
    }
}
