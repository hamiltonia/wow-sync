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
