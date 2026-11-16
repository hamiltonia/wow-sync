//! Durable apply reports (publication.md §2 `wowsync.pending-report/1`, §4). An `applied`
//! result is recorded locally before its journal is removed, so a crash between apply and
//! publish cannot lose it; it is published once and then dropped.

use crate::ids::{EnrollmentId, GroupId};
use crate::records::{
    ApplyJournal, ApplyReport, CodeState, JournalPhase, PendingReport, ReportKind, ReportResult,
};

/// The `applied` result a journal stands for when finalize removes it. Only a `verified`
/// journal qualifies: a staging, committing, committed, rolled-back or needs-help op never
/// claims `applied`. The journal does not record the plan basis, so it is empty here.
pub fn finalized_result(j: &ApplyJournal) -> Option<ApplyReport> {
    (j.phase == JournalPhase::Verified).then(|| ApplyReport {
        op: j.op.clone(),
        basis: Vec::new(),
        results: vec![ReportResult {
            package: j.package.clone(),
            result: ReportKind::Applied,
            package_hash: Some(j.target_hash),
        }],
    })
}

/// Newer results replace older ones for the same package (a later failed op withdraws an
/// earlier `applied` claim); basis entries are merged per computer, newer first.
pub fn merge_report(older: Option<ApplyReport>, newer: ApplyReport) -> ApplyReport {
    let Some(mut out) = older else {
        return newer;
    };
    for r in newer.results {
        match out.results.iter_mut().find(|x| x.package == r.package) {
            Some(x) => *x = r,
            None => out.results.push(r),
        }
    }
    out.results.sort_by(|a, b| a.package.cmp(&b.package));
    for b in newer.basis {
        match out
            .basis
            .iter_mut()
            .find(|x| x.computer_id == b.computer_id)
        {
            Some(x) => *x = b,
            None => out.basis.push(b),
        }
    }
    out.basis
        .sort_by(|a, b| a.computer_id.as_str().cmp(b.computer_id.as_str()));
    out
}

/// The store after recording `newer`; a record of another group or enrollment is discarded.
pub fn record_pending(
    store: Option<PendingReport>,
    group: &GroupId,
    enrollment: &EnrollmentId,
    newer: ApplyReport,
) -> PendingReport {
    let older = store
        .filter(|s| &s.group_id == group && &s.enrollment_id == enrollment)
        .and_then(|s| s.report);
    PendingReport {
        group_id: group.clone(),
        enrollment_id: enrollment.clone(),
        report: Some(merge_report(older, newer)),
    }
}

/// The report still to be published. Results that this enrollment's newest published state
/// already carries are dropped (a crash between publishing and clearing the store must not
/// publish them again); a record of another group or enrollment is ignored.
pub fn outstanding(
    store: Option<&PendingReport>,
    group: &GroupId,
    enrollment: &EnrollmentId,
    latest_own: Option<&CodeState>,
) -> Option<ApplyReport> {
    let s = store.filter(|s| &s.group_id == group && &s.enrollment_id == enrollment)?;
    let mut rep = s.report.clone()?;
    let carried = latest_own
        .filter(|st| &st.group_id == group && &st.enrollment_id == enrollment)
        .and_then(|st| st.apply_report.as_ref());
    if let Some(c) = carried {
        rep.results.retain(|r| !c.results.contains(r));
    }
    (!rep.results.is_empty()).then_some(rep)
}

/// Whether the store still holds a result that this enrollment's newest published state
/// carries. Deduplication looks only at the newest own state, so such a store must be durably
/// reduced to `outstanding` before any newer own state is published; otherwise the stale
/// result would be published again once the carrying state is no longer the newest.
pub fn store_needs_reconcile(
    store: Option<&PendingReport>,
    group: &GroupId,
    enrollment: &EnrollmentId,
    latest_own: Option<&CodeState>,
) -> bool {
    let Some(rep) = store
        .filter(|s| &s.group_id == group && &s.enrollment_id == enrollment)
        .and_then(|s| s.report.as_ref())
    else {
        return false;
    };
    let Some(c) = latest_own
        .filter(|st| &st.group_id == group && &st.enrollment_id == enrollment)
        .and_then(|st| st.apply_report.as_ref())
    else {
        return false;
    };
    rep.results.iter().any(|r| c.results.contains(r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ComputerId, OpId, Sha256};
    use crate::paths::FolderKey;
    use crate::plan::{ExpectedLive, FolderAction};
    use crate::records::{BasisEntry, JournalFolder};
    use crate::testkit;

    fn gid() -> GroupId {
        GroupId::parse(&"1".repeat(32)).unwrap()
    }
    fn eid() -> EnrollmentId {
        EnrollmentId::parse(&"2".repeat(32)).unwrap()
    }
    fn op(n: char) -> OpId {
        OpId::parse(&n.to_string().repeat(32)).unwrap()
    }

    fn journal(phase: JournalPhase) -> ApplyJournal {
        ApplyJournal {
            op: op('a'),
            group_id: gid(),
            started_unix_ms: 0,
            package: FolderKey::lossy("alpha"),
            target_hash: Sha256([7; 32]),
            phase,
            from_phase: (phase == JournalPhase::RolledBack).then_some(JournalPhase::Committing),
            reason: None,
            folders: vec![JournalFolder {
                name: "Alpha".into(),
                key: FolderKey::lossy("alpha"),
                action: FolderAction::Install,
                target_tree: Sha256([8; 32]),
                expected_live: ExpectedLive::Absent,
            }],
        }
    }

    fn result(pkg: &str, kind: ReportKind, h: u8) -> ReportResult {
        ReportResult {
            package: FolderKey::lossy(pkg),
            result: kind,
            package_hash: Some(Sha256([h; 32])),
        }
    }

    fn report(o: char, results: Vec<ReportResult>) -> ApplyReport {
        ApplyReport {
            op: op(o),
            basis: vec![],
            results,
        }
    }

    #[test]
    fn only_a_verified_journal_claims_applied() {
        let r = finalized_result(&journal(JournalPhase::Verified)).unwrap();
        assert_eq!(r.results, vec![result("alpha", ReportKind::Applied, 7)]);
        for p in [
            JournalPhase::Staging,
            JournalPhase::Committing,
            JournalPhase::Committed,
            JournalPhase::NeedsHelp,
            JournalPhase::RolledBack,
        ] {
            assert_eq!(finalized_result(&journal(p)), None, "{p:?}");
        }
    }

    #[test]
    fn newer_result_replaces_older_for_same_package() {
        let older = report(
            'a',
            vec![
                result("alpha", ReportKind::Applied, 1),
                result("beta", ReportKind::Applied, 2),
            ],
        );
        let mut newer = report('b', vec![result("alpha", ReportKind::Interrupted, 3)]);
        newer.basis = vec![BasisEntry {
            computer_id: ComputerId::parse(&"c".repeat(32)).unwrap(),
            seq: 4,
        }];
        let m = merge_report(Some(older), newer);
        assert_eq!(
            m.results,
            vec![
                result("alpha", ReportKind::Interrupted, 3),
                result("beta", ReportKind::Applied, 2)
            ]
        );
        assert_eq!(m.basis.len(), 1);
        assert_eq!(m.op, op('a'));
    }

    #[test]
    fn record_discards_other_enrollment() {
        let other = PendingReport {
            group_id: gid(),
            enrollment_id: EnrollmentId::parse(&"9".repeat(32)).unwrap(),
            report: Some(report('a', vec![result("beta", ReportKind::Applied, 2)])),
        };
        let s = record_pending(
            Some(other),
            &gid(),
            &eid(),
            report('b', vec![result("alpha", ReportKind::Applied, 1)]),
        );
        assert_eq!(s.enrollment_id, eid());
        assert_eq!(
            s.report.unwrap().results,
            vec![result("alpha", ReportKind::Applied, 1)]
        );
    }

    #[test]
    fn outstanding_drops_what_the_newest_state_already_carries() {
        let store = PendingReport {
            group_id: gid(),
            enrollment_id: eid(),
            report: Some(report(
                'a',
                vec![
                    result("alpha", ReportKind::Applied, 1),
                    result("beta", ReportKind::Applied, 2),
                ],
            )),
        };
        assert_eq!(
            outstanding(Some(&store), &gid(), &eid(), None),
            store.report.clone()
        );
        let mut st = testkit::member_state(&testkit::computer(1), 1, &testkit::inventory(vec![]));
        st.group_id = gid();
        st.enrollment_id = eid();
        st.apply_report = Some(report('a', vec![result("alpha", ReportKind::Applied, 1)]));
        let left = outstanding(Some(&store), &gid(), &eid(), Some(&st)).unwrap();
        assert_eq!(left.results, vec![result("beta", ReportKind::Applied, 2)]);
        st.apply_report = store.report.clone();
        assert_eq!(outstanding(Some(&store), &gid(), &eid(), Some(&st)), None);
        let other = EnrollmentId::parse(&"9".repeat(32)).unwrap();
        assert_eq!(outstanding(Some(&store), &gid(), &other, None), None);
    }

    #[test]
    fn store_holding_a_carried_result_must_be_reconciled_first() {
        let store = PendingReport {
            group_id: gid(),
            enrollment_id: eid(),
            report: Some(report('a', vec![result("alpha", ReportKind::Applied, 1)])),
        };
        assert!(!store_needs_reconcile(Some(&store), &gid(), &eid(), None));
        let mut st = testkit::member_state(&testkit::computer(1), 1, &testkit::inventory(vec![]));
        st.group_id = gid();
        st.enrollment_id = eid();
        assert!(!store_needs_reconcile(
            Some(&store),
            &gid(),
            &eid(),
            Some(&st)
        ));
        st.apply_report = Some(report('b', vec![result("alpha", ReportKind::Applied, 2)]));
        assert!(!store_needs_reconcile(
            Some(&store),
            &gid(),
            &eid(),
            Some(&st)
        ));
        st.apply_report = store.report.clone();
        assert!(store_needs_reconcile(
            Some(&store),
            &gid(),
            &eid(),
            Some(&st)
        ));
        let cleared = PendingReport {
            report: None,
            ..store.clone()
        };
        assert!(!store_needs_reconcile(
            Some(&cleared),
            &gid(),
            &eid(),
            Some(&st)
        ));
        let other = EnrollmentId::parse(&"9".repeat(32)).unwrap();
        assert!(!store_needs_reconcile(
            Some(&store),
            &gid(),
            &other,
            Some(&st)
        ));
    }
}
