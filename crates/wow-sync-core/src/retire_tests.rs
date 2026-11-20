//! Retirement flow tests (ops.md §2, T-45-*, T-R4D31-held-self).

use crate::authority::{RetVerdict, Retired};
use crate::choice_tests::{eref, members};
use crate::decision::{CycleCx, RetirePlan};
use crate::hash::sha256;
use crate::hooks::RetireHooks;
use crate::ids::Sha256;
use crate::kinds::{AuthorityKind, EnrollmentRef, Retirement};
use crate::ledger::{derive, Derived, Ledger, Membership};
use crate::records::{encode, Record};
use crate::retire::{own_standing, OwnStanding, RetireBehaviour};
use crate::testkit::{computer, enrollment, group_id};
use std::collections::{BTreeMap, BTreeSet};

fn learn(l: &mut Ledger, r: &Retirement) -> Sha256 {
    let bytes = encode(r);
    let h = sha256(&bytes);
    l.learn(
        h,
        AuthorityKind::Retirement,
        vec![],
        Some(&computer(1)),
        Some(&bytes),
    );
    h
}

fn plan_for(
    l: &Ledger,
    m: &Membership,
    own: u8,
    subject: &EnrollmentRef,
    b: &RetireBehaviour,
) -> RetirePlan {
    let d: Derived = derive(l, &group_id(), m);
    let g = group_id();
    let me = eref(own);
    let held = BTreeSet::new();
    let cx = CycleCx {
        group_id: &g,
        own: &me,
        ledger: l,
        derived: &d,
        members: m,
        now_unix_ms: 5_000,
        held: &held,
        roots: &[],
    };
    b.plan_retirement(&cx, subject)
}

fn behaviour(seq: u64, admitted: &[(u8, u64)]) -> RetireBehaviour {
    RetireBehaviour {
        own_next_seq: seq,
        admitted_seq: admitted
            .iter()
            .map(|(n, s)| (enrollment(*n), *s))
            .collect::<BTreeMap<_, _>>(),
        master: None,
    }
}

fn barrier(l: &Ledger, m: &Membership) -> crate::authority::Barrier {
    derive(l, &group_id(), m).barrier
}

#[test]
fn retiring_another_computer_cuts_its_later_records_off() {
    // T-45-3: cutoff is the highest admitted sequence; 12 is invalid, 9 stays valid.
    let m = members(&[1, 2, 3]);
    let l = Ledger::new();
    let p = plan_for(&l, &m, 1, &eref(2), &behaviour(7, &[(2, 10)]));
    let r = p.retirement.expect("retirement planned");
    assert_eq!(r.cutoff_seq, 10);
    assert_eq!(r.issuer.issued_at_seq, 7);
    assert_eq!(r.retired, eref(2));
    assert!(
        !p.pause_own_publish,
        "retiring another computer keeps this one publishing"
    );
    assert!(r.validate().is_ok());
    let mut l = Ledger::new();
    learn(&mut l, &r);
    let b = barrier(&l, &m);
    assert_eq!(b.retired(&enrollment(2)), Retired::Yes);
    assert_eq!(b.verdict(&enrollment(2), 12), RetVerdict::AfterRetirement);
    assert_eq!(b.verdict(&enrollment(2), 9), RetVerdict::Valid);
    assert_eq!(b.verdict(&enrollment(3), 99), RetVerdict::Valid);
}

#[test]
fn retiring_the_settings_master_never_picks_a_replacement() {
    // T-45-4
    let m = members(&[1, 2]);
    let mut b = behaviour(3, &[(2, 4)]);
    b.master = Some(eref(2));
    let p = plan_for(&Ledger::new(), &m, 1, &eref(2), &b);
    let r = p.retirement.expect("planned");
    assert!(r.was_settings_master);
    assert_eq!(p.reassign_master, None);
    // Retiring a non-master does not claim it was the master.
    b.master = Some(eref(1));
    let p = plan_for(&Ledger::new(), &m, 1, &eref(2), &b);
    assert!(!p.retirement.unwrap().was_settings_master);
}

#[test]
fn mutual_unknowing_retirement_retires_both_in_every_read_order() {
    // T-45-5: neither side knew the other's retirement when it wrote its own.
    let m = members(&[1, 2, 3]);
    let a = plan_for(&Ledger::new(), &m, 1, &eref(2), &behaviour(2, &[(2, 3)]))
        .retirement
        .unwrap();
    let b = plan_for(&Ledger::new(), &m, 2, &eref(1), &behaviour(2, &[(1, 3)]))
        .retirement
        .unwrap();
    assert!(a.retirements_known.is_empty() && b.retirements_known.is_empty());
    for order in [[&a, &b], [&b, &a]] {
        let mut l = Ledger::new();
        for r in order {
            learn(&mut l, r);
        }
        let bar = barrier(&l, &m);
        assert_eq!(bar.retired(&enrollment(1)), Retired::Yes);
        assert_eq!(bar.retired(&enrollment(2)), Retired::Yes);
        assert_eq!(bar.retired(&enrollment(3)), Retired::No);
    }
}

#[test]
fn re_enrolled_computer_is_a_new_member_and_the_old_enrollment_stays_retired() {
    // T-45-6
    let mut m = members(&[1, 2, 3]);
    let mut l = Ledger::new();
    let r = plan_for(&l, &m, 1, &eref(2), &behaviour(2, &[(2, 5)]))
        .retirement
        .unwrap();
    learn(&mut l, &r);
    m.add(&computer(2), &enrollment(9));
    let bar = barrier(&l, &m);
    assert_eq!(bar.retired(&enrollment(2)), Retired::Yes);
    assert_eq!(bar.retired(&enrollment(9)), Retired::No);
    assert_eq!(
        bar.verdict(&enrollment(2), 5),
        RetVerdict::Valid,
        "history stays valid"
    );
    // Retiring the old enrollment again plans nothing; an unknown enrollment is not retirable.
    let again = plan_for(&l, &m, 3, &eref(2), &behaviour(2, &[(2, 5)]));
    assert_eq!(again, RetirePlan::default());
    let stranger = EnrollmentRef {
        computer_id: computer(7),
        enrollment_id: enrollment(7),
    };
    assert_eq!(
        plan_for(&l, &m, 3, &stranger, &behaviour(2, &[])),
        RetirePlan::default()
    );
}

#[test]
fn a_retired_computer_plans_no_retirement_and_pauses_publishing() {
    // T-45-7, Q-60: no record is written for a retired self.
    let m = members(&[1, 2, 3]);
    let mut l = Ledger::new();
    let r = plan_for(&Ledger::new(), &m, 2, &eref(1), &behaviour(2, &[(1, 3)]))
        .retirement
        .unwrap();
    learn(&mut l, &r);
    let d = derive(&l, &group_id(), &m);
    let me = eref(1);
    let g = group_id();
    let held = BTreeSet::new();
    let cx = CycleCx {
        group_id: &g,
        own: &me,
        ledger: &l,
        derived: &d,
        members: &m,
        now_unix_ms: 1,
        held: &held,
        roots: &[],
    };
    assert_eq!(own_standing(&cx), OwnStanding::Retired);
    let p = plan_for(&l, &m, 1, &eref(3), &behaviour(9, &[(3, 4)]));
    assert_eq!(p.retirement, None, "a retired computer writes nothing");
    assert!(p.pause_own_publish);
    assert_eq!(p.reassign_master, None);
}

#[test]
fn self_retirement_cuts_off_at_the_last_written_sequence() {
    // T-45-3 (self form), R4D-31
    let m = members(&[1, 2]);
    let p = plan_for(&Ledger::new(), &m, 1, &eref(1), &behaviour(6, &[]));
    let r = p.retirement.expect("self retirement");
    assert_eq!(r.cutoff_seq, 5);
    assert_eq!(r.issuer.issued_at_seq, 6);
    assert!(
        p.pause_own_publish,
        "the retiring computer stops publishing"
    );
    assert!(r.validate().is_ok());
}

#[test]
fn two_effective_retirements_use_the_minimum_cutoff_in_either_order() {
    // T-45-8
    let m = members(&[1, 2, 3, 4]);
    let a = plan_for(&Ledger::new(), &m, 1, &eref(2), &behaviour(2, &[(2, 8)]))
        .retirement
        .unwrap();
    let c = plan_for(&Ledger::new(), &m, 3, &eref(2), &behaviour(2, &[(2, 12)]))
        .retirement
        .unwrap();
    for order in [[&a, &c], [&c, &a]] {
        let mut l = Ledger::new();
        for r in order {
            learn(&mut l, r);
        }
        let bar = barrier(&l, &m);
        assert_eq!(bar.cutoff(&enrollment(2)), Some(8));
        assert_eq!(bar.verdict(&enrollment(2), 11), RetVerdict::AfterRetirement);
        assert_eq!(bar.verdict(&enrollment(2), 8), RetVerdict::Valid);
    }
}

#[test]
fn held_self_is_neither_active_nor_retired_and_publishes_nothing() {
    // T-R4D31-held-self (core part): a retirement of this computer that depends on a record not
    // yet read leaves it held.
    let m = members(&[1, 2]);
    let mut r = plan_for(&Ledger::new(), &m, 2, &eref(1), &behaviour(2, &[(1, 3)]))
        .retirement
        .unwrap();
    r.retirements_known = vec![sha256(b"not yet read")];
    let mut l = Ledger::new();
    learn(&mut l, &r);
    let d = derive(&l, &group_id(), &m);
    assert_eq!(d.barrier.retired(&enrollment(1)), Retired::Unresolved);
    let me = eref(1);
    let g = group_id();
    let held = BTreeSet::new();
    let cx = CycleCx {
        group_id: &g,
        own: &me,
        ledger: &l,
        derived: &d,
        members: &m,
        now_unix_ms: 1,
        held: &held,
        roots: &[],
    };
    assert_eq!(own_standing(&cx), OwnStanding::Held);
    let p = plan_for(&l, &m, 1, &eref(2), &behaviour(9, &[(2, 4)]));
    assert_eq!(p.retirement, None);
    assert!(p.pause_own_publish);
}
