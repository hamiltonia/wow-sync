//! Decision freshness and decision-item tests (contracts.md §5.5, ops.md §1).

use crate::authority::AuthorityRecord;
use crate::choice::{Copy, CopyEntry};
use crate::choice_tests::{eref, inv2, key, pkg, pv, Env};
use crate::decide::{
    build_item, encode_authority, new_version_item, DecideBehaviour, PlannedOption, Standing,
};
use crate::decision::{
    DecideError, DecideOutcome, DecisionEffect, DecisionId, DecisionKind, DecisionOption,
    DecisionReach, DecisionScope, FreshEvidence, LocalResolution, OptionId, OptionLabel,
};
use crate::hash::sha256;
use crate::hooks::DecisionHooks;
use crate::inventory::Inventory;
use crate::kinds::{ChoiceAction, CodeChoice};
use crate::package::PackageVersion;
use crate::target::Source;
use crate::testkit::group_id;

fn copy(v: &PackageVersion, i: &Inventory, holder: u8) -> Copy {
    Copy {
        version: v.clone(),
        source: Source::from_inventory(i),
        holders: vec![eref(holder)],
    }
}

fn written(e: &DecisionEffect) -> &CodeChoice {
    match e {
        DecisionEffect::Write(r) => match r.as_ref() {
            AuthorityRecord::CodeChoice(c) => c,
            other => panic!("not a code choice: {other:?}"),
        },
        other => panic!("not a write: {other:?}"),
    }
}

struct World {
    env: Env,
    kept_hash: crate::ids::Sha256,
    invs: Vec<Inventory>,
    vs: Vec<PackageVersion>,
}

/// A keep of 1.0 that considered 1.0 and 2.0, with a 3.0 copy now present.
fn world() -> World {
    let invs: Vec<Inventory> = ["1.0", "2.0", "3.0", "4.0"]
        .iter()
        .map(|v| inv2(v, v))
        .collect();
    let p = pkg(&["alpha", "beta"]);
    let vs: Vec<PackageVersion> = invs.iter().map(|i| pv(&p, i)).collect();
    let mut env = Env::new();
    let g = group_id();
    let cx = env.cx(&g, 1);
    let shown = [
        CopyEntry::new(entry_folders(&vs[0]), vec![]),
        CopyEntry::new(entry_folders(&vs[1]), vec![]),
    ];
    let c = env
        .view(|v| {
            crate::choice::build_keep(
                v,
                &cx,
                crate::kinds::ChoiceReason::Ambiguous,
                &[key("alpha"), key("beta")],
                &shown[0],
                &shown,
                &Source::from_inventory(&invs[0]),
            )
        })
        .unwrap();
    let kept_hash = env.add(&c);
    World {
        env,
        kept_hash,
        invs,
        vs,
    }
}

fn entry_folders(v: &PackageVersion) -> Vec<crate::kinds::ManifestFolder> {
    v.folders
        .iter()
        .map(|f| crate::kinds::ManifestFolder {
            key: f.key.clone(),
            name: f.name.clone(),
            tree: f.tree.unwrap(),
        })
        .collect()
}

fn copies(w: &World, idx: &[usize]) -> Vec<Copy> {
    idx.iter()
        .map(|i| copy(&w.vs[*i], &w.invs[*i], 1))
        .collect()
}

fn item(w: &mut World, idx: &[usize]) -> (crate::decision::DecisionItem, crate::decide::Plans) {
    let cs = copies(w, idx);
    let g = group_id();
    let cx = w.env.cx(&g, 2);
    w.env.view(|v| {
        let p = pkg(&["alpha", "beta"]);
        let nv = v.new_versions(&p, &cs);
        assert_eq!(nv.len(), 1, "exactly one keep raises the decision");
        new_version_item(v, &cx, &cs, &nv[0]).expect("item builds")
    })
}

#[test]
fn new_version_item_offers_keep_take_or_follow_and_each_writes_one_choice() {
    // T-26-4, T-26-5, T-26-8, T-26-9, T-R4D10-new-version-after-keep (Q-49)
    let mut w = world();
    let (it, plans) = item(&mut w, &[0, 1, 2]);
    assert_eq!(it.reason, DecisionKind::NewVersionAfterDecision);
    assert_eq!(it.reach, DecisionReach::Group);
    assert_eq!(
        it.scope,
        DecisionScope::Folders(vec![key("alpha"), key("beta")])
    );
    let labels: Vec<&OptionLabel> = it.options.iter().map(|o| &o.label).collect();
    assert_eq!(
        labels,
        vec![
            &OptionLabel::KeepCurrent,
            &OptionLabel::Release {
                version_label: "3.0".into()
            },
            &OptionLabel::ChangeDecision,
        ]
    );
    let id = DecisionId(it.scope.clone());
    let b = DecideBehaviour::new(plans, Standing::Active);
    let fresh = FreshEvidence {
        items: vec![it.clone()],
    };
    let run = |n: u32| b.decide(&id, &OptionId(n), &it.evidence, &fresh);
    let DecideOutcome::Effect(keep_current) = run(0) else {
        panic!("keep current")
    };
    let DecideOutcome::Effect(take_new) = run(1) else {
        panic!("take new")
    };
    let DecideOutcome::Effect(follow) = run(2) else {
        panic!("follow")
    };

    let kc = written(&keep_current);
    assert_eq!(kc.action, ChoiceAction::Keep);
    assert_eq!(kc.keep.as_ref().unwrap().package_hash, w.vs[0].hash);
    assert!(
        kc.considered.iter().any(|c| c.package_hash == w.vs[2].hash),
        "3.0 now considered"
    );
    assert!(kc.exclude.iter().any(|e| e.package_hash == w.vs[2].hash));

    let tn = written(&take_new);
    assert_eq!(tn.keep.as_ref().unwrap().package_hash, w.vs[2].hash);
    let ex: Vec<_> = tn.exclude.iter().map(|e| e.package_hash).collect();
    assert!(
        ex.contains(&w.vs[0].hash) && ex.contains(&w.vs[1].hash) && !ex.contains(&w.vs[2].hash)
    );

    let f = written(&follow);
    assert_eq!(f.action, ChoiceAction::Resume);
    assert!(
        f.exclude.iter().any(|e| e.package_hash == w.vs[1].hash),
        "old exclusions carried"
    );

    // Keeping the current copy ends the prompt for what was shown; a later copy asks again.
    let kc_hash = w.env.add(&kc.clone());
    let cs3 = copies(&w, &[0, 1, 2]);
    w.env.view(|v| {
        assert!(v.new_versions(&pkg(&["alpha", "beta"]), &cs3).is_empty());
        assert_eq!(v.live_choices(&key("alpha"))[0].0, kc_hash);
        let cs4 = copies(&w, &[0, 1, 2, 3]);
        assert_eq!(
            v.new_versions(&pkg(&["alpha", "beta"]), &cs4)[0].copies,
            vec![3]
        );
    });
}

#[test]
fn a_changed_item_refuses_and_writes_nothing() {
    // T-C15-decide-fresh, T-26-3: a newer copy appeared after the owner opened the item.
    let mut w = world();
    let (shown, plans) = item(&mut w, &[0, 1, 2]);
    let (now, _) = item(&mut w, &[0, 1, 2, 3]);
    assert_ne!(
        shown.evidence, now.evidence,
        "new evidence changes the token"
    );
    let id = DecisionId(shown.scope.clone());
    let b = DecideBehaviour::new(plans, Standing::Active);
    let out = b.decide(
        &id,
        &OptionId(1),
        &shown.evidence,
        &FreshEvidence {
            items: vec![now.clone()],
        },
    );
    assert_eq!(out, DecideOutcome::Refused(DecideError::Changed));
    // The same evidence is accepted, so the refusal is about freshness only.
    let ok = b.decide(
        &id,
        &OptionId(1),
        &shown.evidence,
        &FreshEvidence {
            items: vec![shown.clone()],
        },
    );
    assert!(matches!(
        ok,
        DecideOutcome::Effect(DecisionEffect::Write(_))
    ));
    // An item that no longer exists is stale too.
    let gone = b.decide(
        &id,
        &OptionId(1),
        &shown.evidence,
        &FreshEvidence::default(),
    );
    assert_eq!(gone, DecideOutcome::Refused(DecideError::Changed));
    let _ = w.kept_hash;
}

#[test]
fn unknown_options_and_items_without_options_never_write() {
    // T-R4D10-unusable-metadata-no-override, T-26-1: identity is the scope; options are typed.
    let scope = DecisionScope::Folders(vec![key("alpha")]);
    let (unusable, plans) = build_item(
        scope.clone(),
        DecisionKind::UnusableMetadata(crate::decision::UnusableKind::Unreadable),
        DecisionReach::ThisComputer,
        vec![],
        &[sha256(b"basis")],
    );
    let id = DecisionId(scope);
    let b = DecideBehaviour::new(plans, Standing::Active);
    let fresh = FreshEvidence {
        items: vec![unusable.clone()],
    };
    assert_eq!(
        b.decide(&id, &OptionId(0), &unusable.evidence, &fresh),
        DecideOutcome::Refused(DecideError::UnknownOption)
    );

    // An option the item lists but no plan backs is refused, never invented.
    let option = DecisionOption {
        id: OptionId(0),
        label: OptionLabel::KeepMySettings,
        holders: vec![],
        consequence: Default::default(),
        warning: None,
    };
    let planned = vec![PlannedOption {
        option,
        effect: DecisionEffect::Local(LocalResolution::Dismiss(sha256(b"x"))),
    }];
    let (item, _) = build_item(
        DecisionScope::Folders(vec![key("beta")]),
        DecisionKind::Ambiguous,
        DecisionReach::ThisComputer,
        planned,
        &[],
    );
    let id = DecisionId(item.scope.clone());
    let empty = DecideBehaviour::new(vec![], Standing::Active);
    assert_eq!(
        empty.decide(
            &id,
            &OptionId(0),
            &item.evidence,
            &FreshEvidence {
                items: vec![item.clone()]
            }
        ),
        DecideOutcome::Refused(DecideError::NotAllowed)
    );
}

#[test]
fn a_retired_computer_cannot_publish_a_decision_but_can_decide_locally() {
    // T-45-7 (decision form), R4D-31
    let scope = DecisionScope::Folders(vec![key("alpha")]);
    let mk = |id: u32, effect| PlannedOption {
        option: DecisionOption {
            id: OptionId(id),
            label: OptionLabel::KeepOnThisComputer,
            holders: vec![],
            consequence: Default::default(),
            warning: None,
        },
        effect,
    };
    let mut w = world();
    let (it, plans) = item(&mut w, &[0, 1, 2]);
    let write = plans[0].1.clone();
    let local = DecisionEffect::Local(LocalResolution::KeepOnThisComputer(scope.clone()));
    let (item2, plans2) = build_item(
        scope.clone(),
        DecisionKind::Ambiguous,
        DecisionReach::ThisComputer,
        vec![mk(0, write), mk(1, local.clone())],
        &[],
    );
    let id = DecisionId(scope);
    let fresh = FreshEvidence {
        items: vec![item2.clone()],
    };
    let blocked = DecideBehaviour::new(plans2.clone(), Standing::Blocked);
    assert_eq!(
        blocked.decide(&id, &OptionId(0), &item2.evidence, &fresh),
        DecideOutcome::Refused(DecideError::NotAllowed)
    );
    assert_eq!(
        blocked.decide(&id, &OptionId(1), &item2.evidence, &fresh),
        DecideOutcome::Effect(local)
    );
    let active = DecideBehaviour::new(plans2, Standing::Active);
    assert!(matches!(
        active.decide(&id, &OptionId(0), &item2.evidence, &fresh),
        DecideOutcome::Effect(DecisionEffect::Write(_))
    ));
    let _ = it;
}

#[test]
fn encode_authority_is_the_record_wire_form() {
    let mut w = world();
    let (_, plans) = item(&mut w, &[0, 1, 2]);
    let DecisionEffect::Write(rec) = &plans[0].1 else {
        panic!()
    };
    let bytes = encode_authority(rec);
    assert_eq!(bytes, crate::records::encode(written(&plans[0].1)));
    assert!(!bytes.is_empty());
}

/// Two computers each keep a different copy of the same scope without knowing the other's keep.
fn conflicting_world() -> World {
    let invs: Vec<Inventory> = ["1.0", "2.0"].iter().map(|v| inv2(v, v)).collect();
    let p = pkg(&["alpha", "beta"]);
    let vs: Vec<PackageVersion> = invs.iter().map(|i| pv(&p, i)).collect();
    let mut env = Env::new();
    let g = group_id();
    let shown = [
        CopyEntry::new(entry_folders(&vs[0]), vec![]),
        CopyEntry::new(entry_folders(&vs[1]), vec![]),
    ];
    let mut records = Vec::new();
    for i in 0..2 {
        let cx = env.cx(&g, 1);
        records.push(
            env.view(|v| {
                crate::choice::build_keep(
                    v,
                    &cx,
                    crate::kinds::ChoiceReason::Ambiguous,
                    &[key("alpha"), key("beta")],
                    &shown[i],
                    &shown,
                    &Source::from_inventory(&invs[i]),
                )
            })
            .unwrap(),
        );
    }
    for r in &records {
        env.add(r);
    }
    World {
        env,
        kept_hash: vs[0].hash,
        invs,
        vs,
    }
}

fn conflict_target() -> crate::target::GroupTarget {
    use crate::explain::Explanation;
    use crate::target::{DecisionReason, PackageTarget};
    let mut t = crate::target::GroupTarget {
        group_id: group_id(),
        packages: Default::default(),
        effective: Default::default(),
        local_hash: Default::default(),
    };
    t.packages.insert(
        key("alpha"),
        PackageTarget::NeedsDecision {
            reason: DecisionReason::ChoicesDiffer,
            candidates: vec![],
            why: Explanation::ConcurrentDecisions,
        },
    );
    t
}

#[test]
fn conflicting_keeps_do_not_hide_each_others_copies_and_a_choice_supersedes_both() {
    // Run-009 R9-decide: each keep excludes the other's copy, which emptied the copy list.
    use crate::target::{PackageCopies, ScopeMatch};
    let mut w = conflicting_world();
    let p = pkg(&["alpha", "beta"]);
    for inv in &w.invs {
        let verdict = w.env.view(|v| v.excluded(&p, &Source::from_inventory(inv)));
        assert_ne!(verdict, ScopeMatch::Same, "a kept copy is never hidden");
    }
    let cs = copies(&w, &[0, 1]);
    let pcs = [PackageCopies {
        package: p.clone(),
        copies: cs,
    }];
    let g = group_id();
    let cx = w.env.cx(&g, 2);
    let (items, chosen) = w.env.view(|v| {
        let items = crate::decide::decision_items(v, &cx, &conflict_target(), &pcs);
        assert_eq!(items.len(), 1, "the conflict raises one item");
        let (it, plans) = items[0].clone();
        assert_eq!(it.reason, DecisionKind::ChoicesDiffer);
        let labels: Vec<&OptionLabel> = it.options.iter().map(|o| &o.label).collect();
        assert_eq!(labels.len(), 3, "each copy plus keep on this computer");
        let id = DecisionId(it.scope.clone());
        let b = DecideBehaviour::new(plans, Standing::Active);
        let fresh = FreshEvidence {
            items: vec![it.clone()],
        };
        let DecideOutcome::Effect(e) = b.decide(&id, &OptionId(1), &it.evidence, &fresh) else {
            panic!("choose")
        };
        (items.len(), written(&e).clone())
    });
    assert_eq!(items, 1);
    assert_eq!(chosen.keep.as_ref().unwrap().package_hash, w.vs[1].hash);
    assert!(!chosen
        .exclude
        .iter()
        .any(|e| e.package_hash == w.vs[1].hash));
    let h = w.env.add(&chosen);
    w.env.view(|v| {
        let live = v.live_choices(&key("alpha"));
        assert_eq!(live.len(), 1, "the choice covers both conflicting records");
        assert_eq!(live[0].0, h);
    });
    let _ = w.kept_hash;
}

#[test]
fn a_keep_whose_release_no_copy_holds_still_builds_its_item_without_panicking() {
    // Run-009 round 2: `unwrap_or(&copies[kept])` indexed past the list when the kept release
    // was absent, panicking every sync pass.
    let mut w = world();
    let (it, _) = item(&mut w, &[2]);
    assert_eq!(it.reason, DecisionKind::NewVersionAfterDecision);
    let cs = copies(&w, &[2]);
    let g = group_id();
    let cx = w.env.cx(&g, 2);
    w.env.view(|v| {
        let p = pkg(&["alpha", "beta"]);
        let nv = v.new_versions(&p, &cs);
        let mut bad = nv[0].clone();
        bad.absent_kept = None;
        bad.kept = 7;
        bad.copies = vec![9];
        assert!(bad.kept_copy(&cs).is_none());
        assert!(bad.local_only(&cs, &crate::testkit::computer(1)).is_empty());
        assert!(new_version_item(v, &cx, &cs, &bad).is_err());
    });
}
