//! Code-choice tests: keep, resume, exclusions and new versions (ops.md §1.5, contracts.md §5.2).

use crate::choice::{
    build_keep, build_resume, valid_hashes, ChoiceBehaviour, ChoiceView, Copy, CopyEntry,
    NewVersion, WriteCx, WriteError,
};
use crate::decision::ConstraintKind;
use crate::hooks::ConstraintHooks;
use crate::ids::{GroupId, Sha256};
use crate::inventory::Inventory;
use crate::kinds::{
    validate_choice, AuthorityKind, ChoiceAction, ChoiceReason, CodeChoice, EnrollmentRef, Issuer,
};
use crate::ledger::{derive, Ledger, Membership};
use crate::package::{package_version, Package, PackageVersion};
use crate::paths::FolderKey;
use crate::records::encode;
use crate::target::{ScopeMatch, Source};
use crate::testkit::{
    addon, client, computer, enrollment, group_id, inventory, member_state, BUILD,
};
use std::collections::BTreeSet;

pub(crate) fn key(name: &str) -> FolderKey {
    FolderKey::lossy(name)
}

/// Two folders, Alpha and Beta, whose content follows the version label.
pub(crate) fn inv2(a: &str, b: &str) -> Inventory {
    inventory(vec![
        addon("Alpha", Some(a), &format!("alpha-{a}")),
        addon("Beta", Some(b), &format!("beta-{b}")),
    ])
}

pub(crate) fn pkg(keys: &[&str]) -> Package {
    Package {
        key: key(keys[0]),
        display: key(keys[0]),
        members: keys.iter().map(|k| key(k)).collect(),
        edges: vec![],
    }
}

pub(crate) fn pv(p: &Package, inv: &Inventory) -> PackageVersion {
    package_version(p, inv, &client(BUILD)).expect("package present")
}

pub(crate) fn entry(v: &PackageVersion, holders: &[u8]) -> CopyEntry {
    let folders = v
        .folders
        .iter()
        .map(|f| crate::kinds::ManifestFolder {
            key: f.key.clone(),
            name: f.name.clone(),
            tree: f.tree.expect("tree"),
        })
        .collect();
    CopyEntry::new(folders, holders.iter().map(|n| enrollment(*n)).collect())
}

pub(crate) fn members(ns: &[u8]) -> Membership {
    let mut m = Membership::default();
    for n in ns {
        m.add(&computer(*n), &enrollment(*n));
    }
    m
}

pub(crate) fn eref(n: u8) -> EnrollmentRef {
    EnrollmentRef {
        computer_id: computer(n),
        enrollment_id: enrollment(n),
    }
}

pub(crate) struct Env {
    pub ledger: Ledger,
    seq: u64,
}

impl Env {
    pub fn new() -> Self {
        Self {
            ledger: Ledger::new(),
            seq: 0,
        }
    }

    pub fn cx<'a>(&mut self, g: &'a GroupId, by: u8) -> WriteCx<'a> {
        self.seq += 1;
        WriteCx {
            group: g,
            issuer: Issuer {
                computer_id: computer(by),
                enrollment_id: enrollment(by),
                issued_at_seq: self.seq,
            },
            now_unix_ms: 1_000 + self.seq,
        }
    }

    pub fn add(&mut self, c: &CodeChoice) -> Sha256 {
        let bytes = encode(c);
        let h = crate::hash::sha256(&bytes);
        self.ledger.learn(
            h,
            AuthorityKind::CodeChoice,
            vec![],
            Some(&computer(1)),
            Some(&bytes),
        );
        h
    }

    pub fn valid(&self) -> BTreeSet<Sha256> {
        valid_hashes(&derive(&self.ledger, &group_id(), &members(&[1, 2, 3])))
    }

    pub fn view<R>(&self, f: impl FnOnce(&ChoiceView) -> R) -> R {
        let v = self.valid();
        f(&ChoiceView::new(&self.ledger, &v))
    }
}

fn src(inv: &Inventory) -> Source {
    Source::from_inventory(inv)
}

/// Keep `kept` over `scope` having shown `shown`, then learn it.
fn keep(
    env: &mut Env,
    scope: &[FolderKey],
    kept: (&PackageVersion, &Inventory),
    shown: &[&PackageVersion],
) -> (CodeChoice, Sha256) {
    let g = group_id();
    let cx = env.cx(&g, 1);
    let shown: Vec<CopyEntry> = shown.iter().map(|v| entry(v, &[2])).collect();
    let c = env
        .view(|v| {
            build_keep(
                v,
                &cx,
                ChoiceReason::Ambiguous,
                scope,
                &entry(kept.0, &[1]),
                &shown,
                &src(kept.1),
            )
        })
        .expect("keep builds");
    let h = env.add(&c);
    (c, h)
}

fn both() -> Vec<FolderKey> {
    vec![key("alpha"), key("beta")]
}

#[test]
fn keep_pins_the_copy_and_excludes_every_shown_copy_it_is_not() {
    // T-26-2, T-R4D10-vchoice-per-action: a keep pins its tree and never selects the other copy.
    let (i1, i2) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v1, v2) = (pv(&p, &i1), pv(&p, &i2));
    let mut env = Env::new();
    let (c, h) = keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2]);
    assert_eq!(c.action, ChoiceAction::Keep);
    assert_eq!(c.exclude.len(), 1);
    assert_eq!(c.exclude[0].package_hash, v2.hash);
    assert_eq!(c.exclude[0].scope, both());
    assert!(validate_choice(&c).is_ok());

    let alpha_tree = i1.get(&key("alpha")).unwrap().tree;
    env.view(|v| {
        let cs = v.constraints(&key("alpha"));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].record, h);
        assert_eq!(cs[0].kind, ConstraintKind::Keep);
        assert_eq!(cs[0].tree, alpha_tree);
        assert_eq!(v.excluded(&p, &src(&i2)), ScopeMatch::Same);
        assert_eq!(v.excluded(&p, &src(&i1)), ScopeMatch::Different);
    });

    // The hook adapter sees the same ledger; the default adapter stays neutral.
    let d = derive(&env.ledger, &group_id(), &members(&[1, 2, 3]));
    let hook = ChoiceBehaviour::for_derived(&d);
    assert_eq!(hook.excluded(&env.ledger, &p, &src(&i2)), ScopeMatch::Same);
    assert_eq!(hook.live_constraints(&env.ledger, &key("beta")).len(), 1);
    let neutral = ChoiceBehaviour::default();
    assert_eq!(
        neutral.excluded(&env.ledger, &p, &src(&i2)),
        ScopeMatch::Different
    );
    assert!(neutral
        .live_constraints(&env.ledger, &key("alpha"))
        .is_empty());

    // Changing the mind to the excluded copy drops its entry and excludes the old pin instead.
    let (c2, _) = keep(&mut env, &both(), (&v2, &i2), &[&v1, &v2]);
    assert_eq!(c2.exclude.len(), 1);
    assert_eq!(c2.exclude[0].package_hash, v1.hash);
    env.view(|v| {
        assert_eq!(v.excluded(&p, &src(&i2)), ScopeMatch::Different);
        assert_eq!(v.excluded(&p, &src(&i1)), ScopeMatch::Same);
        assert_eq!(v.constraints(&key("alpha")).len(), 1);
    });
}

#[test]
fn writer_never_writes_a_record_that_fails_v_choice() {
    // T-R4D10-vchoice-per-action
    let (i1, i2) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v1, v2) = (pv(&p, &i1), pv(&p, &i2));
    let mut env = Env::new();
    let g = group_id();
    let cx = env.cx(&g, 1);
    env.view(|v| {
        let empty = build_keep(
            v,
            &cx,
            ChoiceReason::Ambiguous,
            &[],
            &entry(&v1, &[1]),
            &[],
            &src(&i1),
        );
        assert_eq!(empty, Err(WriteError::EmptyScope));
        let outside = build_keep(
            v,
            &cx,
            ChoiceReason::Ambiguous,
            &[key("alpha")],
            &entry(&v1, &[1]),
            &[],
            &src(&i1),
        );
        assert_eq!(outside, Err(WriteError::KeyOutsideScope(key("beta"))));
        let none = build_resume(v, &cx, ChoiceReason::Ambiguous, &both(), &[v2.hash]);
        assert_eq!(none, Err(WriteError::AllowAgainNotInherited(v2.hash)));
    });
    let (mut k, _) = keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2]);
    k.allow_again = Some(vec![v2.hash]);
    assert!(
        validate_choice(&k).is_err(),
        "a keep carries no allow_again"
    );
    let cx2 = env.cx(&g, 1);
    let mut r = env
        .view(|v| build_resume(v, &cx2, ChoiceReason::Ambiguous, &both(), &[]))
        .unwrap();
    assert!(validate_choice(&r).is_ok());
    r.keep = k.keep.clone();
    assert!(validate_choice(&r).is_err(), "a resume carries no keep");
}

#[test]
fn resume_ends_the_pin_but_keeps_inherited_exclusions() {
    // T-R4D10-resume-keeps-exclusions (Q-50)
    let (i1, i2) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v1, v2) = (pv(&p, &i1), pv(&p, &i2));
    let mut env = Env::new();
    let (k, kh) = keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2]);
    let g = group_id();
    let cx = env.cx(&g, 1);
    let r = env
        .view(|v| build_resume(v, &cx, ChoiceReason::NewVersionAfterDecision, &both(), &[]))
        .unwrap();
    assert_eq!(r.action, ChoiceAction::Resume);
    assert!(r.keep.is_none());
    assert_eq!(r.exclude, k.exclude, "exclusions are inherited verbatim");
    assert!(r.known.extra.contains(&kh), "the resume covers the keep");
    let rh = env.add(&r);
    env.view(|v| {
        assert!(
            v.constraints(&key("alpha")).is_empty(),
            "no pin after resume"
        );
        let live: Vec<Sha256> = v
            .live_choices(&key("alpha"))
            .iter()
            .map(|(h, _)| *h)
            .collect();
        assert_eq!(live, vec![rh]);
        assert_eq!(v.excluded(&p, &src(&i2)), ScopeMatch::Same);
        assert_eq!(v.excluded(&p, &src(&i1)), ScopeMatch::Different);
    });
}

#[test]
fn allow_again_releases_only_the_named_hash() {
    // T-R4D10-resume-allow-again
    let (i1, i2, i3) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"), inv2("3.0", "3.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v1, v2, v3) = (pv(&p, &i1), pv(&p, &i2), pv(&p, &i3));
    let mut env = Env::new();
    keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2, &v3]);
    let g = group_id();
    let cx = env.cx(&g, 1);
    let r = env
        .view(|v| {
            build_resume(
                v,
                &cx,
                ChoiceReason::NewVersionAfterDecision,
                &both(),
                &[v2.hash],
            )
        })
        .unwrap();
    assert_eq!(r.allow_again, Some(vec![v2.hash]));
    assert_eq!(r.exclude.len(), 1);
    assert_eq!(r.exclude[0].package_hash, v3.hash);
    env.add(&r);
    env.view(|v| {
        assert_eq!(
            v.excluded(&p, &src(&i2)),
            ScopeMatch::Different,
            "allowed again"
        );
        assert_eq!(
            v.excluded(&p, &src(&i3)),
            ScopeMatch::Same,
            "still excluded"
        );
    });
}

#[test]
fn unconsidered_newer_copy_raises_a_decision_and_never_replaces_the_keep() {
    // T-R4D10-new-version-after-keep (Q-49)
    let i0 = inv2("0.5", "0.5");
    let (i1, i2, i3) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"), inv2("3.0", "3.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v0, v1, v2, v3) = (pv(&p, &i0), pv(&p, &i1), pv(&p, &i2), pv(&p, &i3));
    let mut env = Env::new();
    let (_, kh) = keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2]);
    let copy = |v: &PackageVersion, i: &Inventory| Copy {
        version: v.clone(),
        source: src(i),
        holders: vec![eref(1)],
    };
    let all = vec![
        copy(&v1, &i1),
        copy(&v2, &i2),
        copy(&v3, &i3),
        copy(&v0, &i0),
    ];
    env.view(|v| {
        // v2 was considered, v0 is older: only the unconsidered newer v3 raises it.
        assert_eq!(
            v.new_versions(&p, &all),
            vec![NewVersion {
                keep: kh,
                scope: both(),
                kept: 0,
                absent_kept: None,
                copies: vec![2]
            }]
        );
        // The keep still pins.
        assert_eq!(v.constraints(&key("alpha")).len(), 1);
        // Q-88: with the kept copy held by nobody its version cannot be read, so an
        // unconsidered copy is unordered against it and raises the decision; a considered one
        // (v2) never does.
        let absent = v.new_versions(&p, &all[1..]);
        assert_eq!(absent.len(), 1);
        assert!(absent[0].absent_kept.is_some());
        assert_eq!(absent[0].copies, vec![1, 2], "v3 and the older v0");
        assert!(v.new_versions(&p, &all[1..2]).is_empty());
        // An unadmitted copy is unknown, not different: it waits.
        let st = member_state(&computer(3), 1, &i3);
        let mut waiting = all.clone();
        waiting[2].source = Source::from_state(&st, false);
        assert!(v.new_versions(&p, &waiting).is_empty());
    });
}

#[test]
fn partial_choice_over_choice_is_decided_per_folder() {
    // T-R4D21-partial-choice-over-choice
    let (i1, i2) = (inv2("1.0", "1.0"), inv2("2.0", "2.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v1, v2) = (pv(&p, &i1), pv(&p, &i2));

    // Grouped: a later keep over Alpha alone covers the older keep for Alpha only.
    let mut env = Env::new();
    let (_, old) = keep(&mut env, &both(), (&v1, &i1), &[&v1, &v2]);
    let pa = pkg(&["alpha"]);
    let (a2, ai2) = (pv(&pa, &i2), &i2);
    let (_, new) = keep(&mut env, &[key("alpha")], (&a2, ai2), &[&a2]);
    env.view(|v| {
        let live =
            |f: &str| -> Vec<Sha256> { v.live_choices(&key(f)).iter().map(|(h, _)| *h).collect() };
        assert_eq!(live("alpha"), vec![new]);
        assert_eq!(live("beta"), vec![old]);
        assert_eq!(
            v.constraints(&key("alpha"))[0].tree,
            i2.get(&key("alpha")).unwrap().tree
        );
        assert_eq!(
            v.constraints(&key("beta"))[0].tree,
            i1.get(&key("beta")).unwrap().tree
        );
    });

    // Ungrouped: choices over disjoint folders never touch each other.
    let mut env = Env::new();
    let pb = pkg(&["beta"]);
    let (a1, b2) = (pv(&pa, &i1), pv(&pb, &i2));
    let (_, ha) = keep(&mut env, &[key("alpha")], (&a1, &i1), &[&a1]);
    let (_, hb) = keep(&mut env, &[key("beta")], (&b2, &i2), &[&b2]);
    env.view(|v| {
        assert_eq!(v.live_choices(&key("alpha"))[0].0, ha);
        assert_eq!(v.live_choices(&key("beta"))[0].0, hb);
    });
}

#[test]
fn carried_exclusion_keeps_its_own_scope_when_the_package_regroups() {
    // T-R4D23-keep-resume-shared-component
    let (i2, i3) = (inv2("2.0", "2.0"), inv2("3.0", "3.0"));
    let p = pkg(&["alpha", "beta"]);
    let (v2, v3) = (pv(&p, &i2), pv(&p, &i3));
    let mut env = Env::new();
    keep(&mut env, &both(), (&v3, &i3), &[&v2, &v3]);

    // A resume over Alpha alone carries the entry with its original two-folder scope.
    let g = group_id();
    let cx = env.cx(&g, 1);
    let r = env
        .view(|v| {
            build_resume(
                v,
                &cx,
                ChoiceReason::NewVersionAfterDecision,
                &[key("alpha")],
                &[],
            )
        })
        .unwrap();
    assert_eq!(r.exclude.len(), 1);
    assert_eq!(r.exclude[0].scope, both());
    env.add(&r);

    env.view(|v| {
        // Alpha grouped alone, Beta grouped alone, and both together all see the V2 copy as
        // excluded, because the match is over the entry's scope, not per folder.
        for members in [vec!["alpha"], vec!["beta"], vec!["alpha", "beta"]] {
            let pk = pkg(&members);
            assert_eq!(v.excluded(&pk, &src(&i2)), ScopeMatch::Same, "{members:?}");
            assert_eq!(
                v.excluded(&pk, &src(&i3)),
                ScopeMatch::Different,
                "{members:?}"
            );
        }
        // A copy that matches V2 on Alpha but not on Beta is a different release.
        let mixed = inv2("2.0", "3.0");
        assert_eq!(v.excluded(&p, &src(&mixed)), ScopeMatch::Different);
        // Beta is still pinned by the original keep; Alpha is not.
        assert!(v.constraints(&key("alpha")).is_empty());
        assert_eq!(v.constraints(&key("beta")).len(), 1);
    });
}
