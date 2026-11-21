//! Settings tests (settings.md §11). Pure inputs; the graph, authority and epoch rules are
//! exercised over abstract nodes because the ledger layer is step 1's.

use std::collections::BTreeSet;

use crate::hash::sha256;
use crate::ids::{Identity, SettingsUnit, Sha256};
use crate::kinds::{EnrollmentRef, PolicyRule, PolicyRuleKind};
use crate::paths::{validate_folder_name, FolderKey};
use crate::settings::*;
use crate::testkit::{computer, enrollment};

fn h(n: u8) -> Sha256 {
    sha256(&[n])
}

fn fk(s: &str) -> FolderKey {
    validate_folder_name(s).expect("folder")
}

fn enr(n: u8) -> EnrollmentRef {
    EnrollmentRef {
        computer_id: computer(n),
        enrollment_id: enrollment(n),
    }
}

fn policy(hash: u8, master: Option<u8>, rules: Vec<PolicyRule>) -> PolicyView {
    PolicyView {
        hash: h(hash),
        master: master.map(enr),
        rules,
    }
}

fn master_rule(folder: &str, n: u8) -> PolicyRule {
    PolicyRule {
        folder: fk(folder),
        rule: PolicyRuleKind::Master,
        computer_id: Some(computer(n)),
        enrollment_id: Some(enrollment(n)),
    }
}

fn not_shared(folder: &str) -> PolicyRule {
    PolicyRule {
        folder: fk(folder),
        rule: PolicyRuleKind::NotShared,
        computer_id: None,
        enrollment_id: None,
    }
}

fn node(id: u8, parents: &[u8], cp: u8, state: NodeState) -> RevNode {
    RevNode {
        hash: h(id),
        parents: parents.iter().map(|p| h(*p)).collect(),
        checkpoint: h(cp),
        state,
    }
}

fn valid(id: u8, parents: &[u8], cp: u8) -> RevNode {
    node(id, parents, cp, NodeState::Valid)
}

fn wtf(rel: &str, size: u64) -> WtfEntry {
    WtfEntry {
        rel: rel.into(),
        size,
        sha256: sha256(rel.as_bytes()),
        unsafe_path: false,
    }
}

fn decl(f: &str, acc: &[&str], chr: &[&str]) -> DeclaringFolder {
    DeclaringFolder {
        folder: fk(f),
        saved_variables: acc.iter().map(|s| s.to_string()).collect(),
        saved_variables_per_character: chr.iter().map(|s| s.to_string()).collect(),
        toc_ambiguous: false,
    }
}

fn included(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

const CAP: Option<u64> = Some(1024);

#[test]
fn scope_owns_only_the_two_shapes_and_never_the_excluded_files() {
    let files = vec![
        wtf("Account/ERIC/SavedVariables/Details.lua", 10),
        wtf(
            "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua",
            10,
        ),
        wtf("Account/ERIC/SavedVariables/Details.lua.bak", 10),
        wtf("Account/ERIC/SavedVariables/Blizzard_Foo.lua", 10),
        wtf("Account/ERIC/config-cache.wtf", 10),
        wtf("Account/ERIC/macros-cache.txt", 10),
        wtf("Config.wtf", 10),
        wtf(
            "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua.old",
            10,
        ),
    ];
    let s = settings_scope(
        &[decl("Details", &["DetailsDB"], &["DetailsChar"])],
        &files,
        &included(&["ERIC"]),
        CAP,
    );
    let rels: Vec<&str> = s.owned.iter().map(|o| o.rel.as_str()).collect();
    assert_eq!(
        rels,
        vec![
            "Account/ERIC/SavedVariables/Details.lua",
            "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua"
        ]
    );
    assert!(s.diagnoses.is_empty());
    assert!(s.sync_enabled);
    // the account file's unit matches only the account identity
    assert_eq!(s.owned[0].unit.identity, Identity::account("ERIC").unwrap());
    assert_eq!(
        s.owned[1].unit.identity,
        Identity::character("ERIC", "Argent Dawn", "Thrall").unwrap()
    );
}

#[test]
fn scope_shape_needs_the_matching_declaration_and_an_included_account() {
    let files = vec![
        wtf("Account/ERIC/SavedVariables/Details.lua", 10),
        wtf("Account/OTHER/SavedVariables/Details.lua", 10),
        wtf(
            "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua",
            10,
        ),
        wtf("Account/ERIC/SavedVariables/Gone.lua", 10),
    ];
    // only the account shape is declared: the character file is unowned
    let s = settings_scope(
        &[decl("Details", &["DetailsDB"], &[])],
        &files,
        &included(&["ERIC"]),
        CAP,
    );
    assert_eq!(s.owned.len(), 1);
    assert_eq!(s.owned[0].unit.identity, Identity::account("ERIC").unwrap());
    assert!(s.diagnoses.contains(&ScopeDiagnosis::Unowned {
        file: "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua".into()
    }));
    assert!(s.diagnoses.contains(&ScopeDiagnosis::Unowned {
        file: "Account/ERIC/SavedVariables/Gone.lua".into()
    }));
    // a not-included account yields neither an owned file nor a diagnosis
    assert!(!s.owned.iter().any(|o| o.rel.as_str().contains("OTHER")));
    assert!(!s
        .diagnoses
        .iter()
        .any(|d| format!("{d:?}").contains("OTHER")));
}

#[test]
fn scope_blocks_only_the_affected_units() {
    let files = vec![
        wtf("Account/ERIC/SavedVariables/A.lua", 10),
        wtf("Account/ERIC/SavedVariables/B.lua", 10),
        wtf("Account/ERIC/SavedVariables/C.lua", 10),
        wtf("Account/ERIC/SavedVariables/D.lua", 5000),
        wtf("Account/ERIC/SavedVariables/E.lua", 10),
    ];
    let mut amb = decl("E", &["EDB"], &[]);
    amb.toc_ambiguous = true;
    let mut linked = files.clone();
    linked[2].unsafe_path = true;
    let s = settings_scope(
        &[
            decl("A", &["Shared"], &[]),
            decl("B", &["Shared"], &[]),
            decl("C", &["CDB"], &[]),
            decl("D", &["DDB"], &[]),
            amb,
        ],
        &linked,
        &included(&["ERIC"]),
        CAP,
    );
    assert!(s.owned.is_empty(), "{:?}", s.owned);
    assert_eq!(s.blocked.len(), 5);
    let d = &s.diagnoses;
    assert!(d.iter().any(|x| matches!(x, ScopeDiagnosis::SharedVariable { name, folders } if name == "Shared" && folders.len() == 2)));
    assert!(d
        .iter()
        .any(|x| matches!(x, ScopeDiagnosis::UnsafePath { .. })));
    assert!(d
        .iter()
        .any(|x| matches!(x, ScopeDiagnosis::TooLarge { size: 5000, .. })));
    assert!(d
        .iter()
        .any(|x| matches!(x, ScopeDiagnosis::AmbiguousToc { .. })));
    // another addon next to the blocked ones still syncs
    let ok = settings_scope(
        &[
            decl("A", &["Shared"], &[]),
            decl("B", &["Shared"], &[]),
            decl("F", &["FDB"], &[]),
        ],
        &[
            wtf("Account/ERIC/SavedVariables/A.lua", 1),
            wtf("Account/ERIC/SavedVariables/F.lua", 1),
        ],
        &included(&["ERIC"]),
        CAP,
    );
    assert_eq!(ok.owned.len(), 1);
    assert_eq!(ok.owned[0].unit.folder, fk("F"));
}

#[test]
fn the_approved_cap_is_32_mib_and_a_file_at_the_cap_syncs_but_one_byte_more_does_not() {
    // Q-77, D-26
    const CAP32: u64 = 32 * 1024 * 1024;
    assert_eq!(crate::records::MAX_SETTINGS_FILE_BYTES, Some(CAP32));
    let scope = |size: u64| {
        settings_scope(
            &[decl("A", &["ADB"], &[])],
            &[wtf("Account/ERIC/SavedVariables/A.lua", size)],
            &included(&["ERIC"]),
            crate::records::MAX_SETTINGS_FILE_BYTES,
        )
    };
    let at = scope(CAP32);
    assert!(at.sync_enabled);
    assert_eq!(at.owned.len(), 1);
    assert!(at.diagnoses.is_empty());
    let over = scope(CAP32 + 1);
    assert!(over.owned.is_empty() && over.blocked.len() == 1);
    assert_eq!(
        over.diagnoses,
        vec![ScopeDiagnosis::TooLarge {
            file: "Account/ERIC/SavedVariables/A.lua".into(),
            size: CAP32 + 1
        }]
    );
}

#[test]
fn an_uninstall_takes_only_unshared_unpaused_owned_files_of_the_removed_folders() {
    // T-R5-remove-targets
    let files = vec![
        wtf("Account/ERIC/SavedVariables/Gone.lua", 10),
        wtf("Account/ERIC/SavedVariables/Held.lua", 10),
        wtf("Account/ERIC/SavedVariables/Mine.lua", 10),
        wtf("Account/ERIC/SavedVariables/Big.lua", 5000),
        wtf("Account/ERIC/SavedVariables/Keep.lua", 10),
    ];
    let scope = settings_scope(
        &[
            decl("Gone", &["GoneDB"], &[]),
            decl("Held", &["HeldDB"], &[]),
            // an installed addon declares the same variable as the removed "Mine"
            decl("Mine", &["Common"], &[]),
            decl("Installed", &["Common"], &[]),
            decl("Big", &["BigDB"], &[]),
            decl("Keep", &["KeepDB"], &[]),
        ],
        &files,
        &included(&["ERIC"]),
        CAP,
    );
    let removed: BTreeSet<_> = ["Gone", "Held", "Mine", "Big"]
        .iter()
        .map(|f| fk(f))
        .collect();
    let paused: BTreeSet<_> = [fk("Held")].into();
    let t = removal_targets(&scope, &removed, &paused);
    let taken: Vec<_> = t.files.iter().map(|f| f.unit.folder.clone()).collect();
    assert_eq!(
        taken,
        vec![fk("Gone")],
        "Keep is not removed, so it is never touched"
    );
    let why = |f: &str| {
        t.missing
            .iter()
            .find(|(u, _)| u.folder == fk(f))
            .map(|(_, r)| *r)
    };
    use crate::records::MissingReason as M;
    assert_eq!(why("Held"), Some(M::Paused));
    assert_eq!(why("Mine"), Some(M::SharedFile));
    assert_eq!(why("Big"), Some(M::TooLarge));
    assert_eq!(why("Keep"), None);
}

#[test]
fn without_a_cap_value_sync_stays_disabled() {
    let s = settings_scope(
        &[decl("A", &["ADB"], &[])],
        &[wtf("Account/ERIC/SavedVariables/A.lua", 1)],
        &included(&["ERIC"]),
        None,
    );
    assert!(!s.sync_enabled);
}

#[test]
fn nfc_colliding_accounts_are_diagnosed_and_neither_syncs() {
    let composed = "Caf\u{e9}";
    let decomposed = "Cafe\u{301}";
    let files = vec![
        wtf(&format!("Account/{composed}/SavedVariables/A.lua"), 1),
        wtf(&format!("Account/{decomposed}/SavedVariables/A.lua"), 1),
    ];
    let s = settings_scope(
        &[decl("A", &["ADB"], &[])],
        &files,
        &included(&[&crate::ids::nfc(composed)]),
        CAP,
    );
    assert!(s.owned.is_empty());
    assert!(s
        .diagnoses
        .iter()
        .any(|d| matches!(d, ScopeDiagnosis::IdentityNormalizationCollision { paths } if paths.len() == 2)));
}

fn scans(first: FileFact, second: FileFact, hashed: FileFact) -> SettleScans {
    SettleScans {
        first,
        second,
        hashed,
        wow_running_at_start: false,
        wow_running_at_end: false,
        bak_present: false,
    }
}

#[test]
fn capture_classification_follows_the_incomplete_write_rules() {
    let a = Some((10, h(1)));
    let b = Some((12, h(2)));
    assert_eq!(classify_capture(a, &scans(b, b, b)), CaptureClass::Complete);
    assert_eq!(
        classify_capture(a, &scans(a, a, a)),
        CaptureClass::Unchanged
    );
    // still changing between the settle scans
    assert_eq!(classify_capture(a, &scans(a, b, b)), CaptureClass::Suspect);
    // changed while hashing
    assert_eq!(classify_capture(a, &scans(b, b, a)), CaptureClass::Suspect);
    // shrank to zero
    let z = Some((0, h(3)));
    assert_eq!(classify_capture(a, &scans(z, z, z)), CaptureClass::Suspect);
    // vanished next to a .bak
    let mut s = scans(None, None, None);
    s.bak_present = true;
    assert_eq!(classify_capture(a, &s), CaptureClass::Suspect);
    // vanished with no .bak is a real removal of the file
    assert_eq!(
        classify_capture(a, &scans(None, None, None)),
        CaptureClass::Complete
    );
    // WoW running at either end
    let mut r = scans(b, b, b);
    r.wow_running_at_end = true;
    assert_eq!(classify_capture(a, &r), CaptureClass::Suspect);
    r.wow_running_at_end = false;
    r.wow_running_at_start = true;
    assert_eq!(classify_capture(a, &r), CaptureClass::Suspect);
}

#[test]
fn authority_is_per_folder_with_exceptions_and_never_elected() {
    let p = vec![policy(
        1,
        Some(1),
        vec![master_rule("b", 2), not_shared("c")],
    )];
    assert_eq!(authority_for(&p, &fk("a")), Authority::Holder(enr(1)));
    assert_eq!(authority_for(&p, &fk("b")), Authority::Holder(enr(2)));
    assert_eq!(authority_for(&p, &fk("c")), Authority::NotShared);
    assert_eq!(authority_for(&[], &fk("a")), Authority::NoPolicy);
    let none = vec![policy(1, None, vec![])];
    assert_eq!(authority_for(&none, &fk("a")), Authority::NoMaster);
    // master retired: reported absent, never replaced
    let live: BTreeSet<EnrollmentRef> = [enr(2)].into();
    assert_eq!(master_state(&p, &live), MasterState::Absent { was: enr(1) });
    let live: BTreeSet<EnrollmentRef> = [enr(1)].into();
    assert_eq!(master_state(&p, &live), MasterState::Present(enr(1)));
    assert_eq!(master_state(&none, &live), MasterState::NotChosen);
}

#[test]
fn policy_is_keyed_by_folder_so_regrouping_changes_nothing() {
    // suite {a, b}: a not shared, b default master; the rules name folders, not packages
    let p = vec![policy(1, Some(1), vec![not_shared("a")])];
    assert_eq!(authority_for(&p, &fk("a")), Authority::NotShared);
    assert_eq!(authority_for(&p, &fk("b")), Authority::Holder(enr(1)));
    // folder `a` alone or inside a different suite: same answers because only the folder is used
    assert_eq!(authority_for(&p, &fk("A")), Authority::NotShared);
}

#[test]
fn nine_policy_heads_witness_set_and_a_tenth_head_conflict() {
    let nine: Vec<PolicyView> = (1..=9u8)
        .map(|i| policy(i, Some(9), vec![master_rule("a", 7)]))
        .collect();
    let witnesses = witness_set(&nine);
    assert_eq!(witnesses.len(), 8);
    let mut all: Vec<Sha256> = nine.iter().map(|p| p.hash).collect();
    all.sort();
    assert_eq!(witnesses, all[..8].to_vec());
    let listed: Vec<PolicyView> = nine
        .iter()
        .filter(|p| witnesses.contains(&p.hash))
        .cloned()
        .collect();
    assert!(auto_revision_valid(&enr(7), &listed, &fk("a")));
    // a reader that knows only the nine applies it
    assert_eq!(authority_for(&nine, &fk("a")), Authority::Holder(enr(7)));
    // a reader that also knows a tenth head naming the den: conflict for `a` only, never invalid
    let mut ten = nine.clone();
    ten.push(policy(10, Some(9), vec![master_rule("a", 8)]));
    assert_eq!(authority_for(&ten, &fk("a")), Authority::Conflict);
    assert_eq!(authority_for(&ten, &fk("b")), Authority::Holder(enr(9)));
    assert!(
        auto_revision_valid(&enr(7), &listed, &fk("a")),
        "validity ignores later heads"
    );
    // validating only one listed witness would accept a revision the others reject
    let mixed = vec![
        nine[0].clone(),
        policy(11, Some(9), vec![master_rule("a", 8)]),
    ];
    assert!(!auto_revision_valid(&enr(7), &mixed, &fk("a")));
    assert!(!auto_revision_valid(&enr(7), &[], &fk("a")));
    let too_many: Vec<PolicyView> = (1..=9u8)
        .map(|i| policy(i, Some(1), vec![master_rule("a", 7)]))
        .collect();
    assert!(!auto_revision_valid(&enr(7), &too_many, &fk("a")));
}

#[test]
fn a_reassigned_author_stays_valid_and_never_prompts() {
    // r was valid under P0 (A master). P1 makes B master. The witness list stays P0.
    let p0 = policy(1, Some(1), vec![]);
    assert!(auto_revision_valid(&enr(1), &[p0], &fk("a")));
    let p1 = vec![policy(2, Some(2), vec![])];
    assert_eq!(authority_for(&p1, &fk("a")), Authority::Holder(enr(2)));
    // no publication by A any more
    let d = publish_decision(&PublishInput {
        authority: authority_for(&p1, &fk("a")),
        own: enr(1),
        paused: false,
        complete: true,
        kind: CaptureKind::SessionEnd,
        changed_in_session: true,
        epoch_clean: true,
        authority_since: None,
        compared_with_local_seq: 9,
    });
    assert_eq!(d, Publish::NotAuthority);
}

#[test]
fn equal_heads_are_one_state_and_all_stay_parents() {
    let g = revision_graph(&[valid(1, &[], 5), valid(2, &[], 5)]);
    assert_eq!(g.heads.len(), 2);
    assert_eq!(g.head_states.len(), 1);
    assert_eq!(choice_parents(&g).len(), 2);
    // the master's next change names both
    let c = {
        let mut v = vec![valid(1, &[], 5), valid(2, &[], 5), valid(3, &[1, 2], 6)];
        v.push(valid(4, &[3], 6));
        revision_graph(&v)
    };
    assert_eq!(c.heads, vec![h(4)]);
    let target = |g: &RevisionGraph| {
        target_case(&TargetInput {
            graph: g,
            authority: &Authority::Holder(enr(1)),
            is_authority: false,
            paused: false,
            basis: &[],
            basis_checkpoint: None,
            live: None,
            epoch_clean: true,
        })
    };
    assert_eq!(target(&g), Case::FirstAdoption(h(5)));
    // different checkpoints are a genuine competing change
    let g2 = revision_graph(&[valid(1, &[], 5), valid(2, &[], 6)]);
    assert_eq!(target(&g2), Case::NeedsChoice);
}

#[test]
fn a_child_before_its_parent_waits_and_is_never_a_root() {
    let child_only = revision_graph(&[valid(2, &[1], 6)]);
    assert!(child_only.pending);
    assert!(child_only.nodes.is_empty());
    assert!(child_only.heads.is_empty());
    let with_parent = revision_graph(&[valid(2, &[1], 6), valid(1, &[], 5)]);
    assert!(!with_parent.pending);
    assert_eq!(with_parent.heads, vec![h(2)]);
    let auth = Authority::Holder(enr(1));
    let case = |g: &RevisionGraph, basis: &[Sha256]| {
        target_case(&TargetInput {
            graph: g,
            authority: &auth,
            is_authority: false,
            paused: false,
            basis,
            basis_checkpoint: Some(h(5)),
            live: Some(h(5)),
            epoch_clean: true,
        })
    };
    assert_eq!(case(&child_only, &[h(1)]), Case::Waiting);
    assert_eq!(
        case(&with_parent, &[h(1)]),
        Case::Yield {
            to: h(6),
            capture_first: false
        }
    );
}

#[test]
fn an_invalid_parent_is_absent_not_a_root_and_an_invalid_basis_is_not_shared() {
    // r1 later invalid (issuer retired below it); a push descends from it
    let g = revision_graph(&[node(1, &[], 5, NodeState::Invalid), valid(2, &[1], 6)]);
    assert!(!g.pending);
    assert_eq!(g.heads, vec![h(2)]);
    let auth = Authority::Holder(enr(1));
    let t = |live: u8| {
        target_case(&TargetInput {
            graph: &g,
            authority: &auth,
            is_authority: false,
            paused: false,
            basis: &[h(1)],
            basis_checkpoint: Some(h(5)),
            live: Some(h(live)),
            epoch_clean: true,
        })
    };
    // never a silent overwrite and never first adoption
    assert_eq!(t(5), Case::LocalDifferent(LocalReason::BasisNotShared));
    assert_eq!(t(9), Case::LocalDifferent(LocalReason::BasisNotShared));
    // already equal to the head: in sync
    assert!(matches!(t(6), Case::InSync { .. }));
}

#[test]
fn zero_nodes_with_a_policy_is_no_shared_copy_and_every_revision_invalid_too() {
    let auth = Authority::Holder(enr(1));
    for g in [
        revision_graph(&[]),
        revision_graph(&[node(1, &[], 5, NodeState::Invalid)]),
    ] {
        let c = target_case(&TargetInput {
            graph: &g,
            authority: &auth,
            is_authority: true,
            paused: false,
            basis: &[],
            basis_checkpoint: None,
            live: Some(h(9)),
            epoch_clean: true,
        });
        assert_eq!(c, Case::NoSharedCopy);
    }
    let g = revision_graph(&[]);
    let off = target_case(&TargetInput {
        graph: &g,
        authority: &Authority::NoPolicy,
        is_authority: false,
        paused: false,
        basis: &[],
        basis_checkpoint: None,
        live: None,
        epoch_clean: true,
    });
    assert_eq!(off, Case::Off);
}

fn case_with(
    g: &RevisionGraph,
    is_authority: bool,
    basis: &[u8],
    cb: Option<u8>,
    live: Option<u8>,
    clean: bool,
) -> Case {
    target_case(&TargetInput {
        graph: g,
        authority: &Authority::Holder(enr(1)),
        is_authority,
        paused: false,
        basis: &basis.iter().map(|b| h(*b)).collect::<Vec<_>>(),
        basis_checkpoint: cb.map(h),
        live: live.map(h),
        epoch_clean: clean,
    })
}

#[test]
fn non_authority_yields_with_recovery_and_an_unchanged_head_never_yields() {
    // before h2: head h1, laptop changed its file -> LocalDifferent, nothing applied
    let g1 = revision_graph(&[valid(1, &[], 5)]);
    assert_eq!(
        case_with(&g1, false, &[1], Some(5), Some(9), true),
        Case::LocalDifferent(LocalReason::LocalChange)
    );
    // unchanged file equal to the head: in sync, no yield
    assert!(matches!(
        case_with(&g1, false, &[1], Some(5), Some(5), true),
        Case::InSync { .. }
    ));
    // after h2: the laptop's changed file is kept in recovery before the yield
    let g2 = revision_graph(&[valid(1, &[], 5), valid(2, &[1], 6)]);
    assert_eq!(
        case_with(&g2, false, &[1], Some(5), Some(9), true),
        Case::Yield {
            to: h(6),
            capture_first: true
        }
    );
    // unchanged since the basis: a plain yield
    assert_eq!(
        case_with(&g2, false, &[1], Some(5), Some(5), true),
        Case::Yield {
            to: h(6),
            capture_first: false
        }
    );
    // the authority with a local change: concurrent push, nothing applied
    assert_eq!(
        case_with(&g2, true, &[1], Some(5), Some(9), true),
        Case::LocalDifferent(LocalReason::ConcurrentPush)
    );
}

#[test]
fn a_dirty_epoch_holds_with_its_own_reason() {
    let g = revision_graph(&[valid(1, &[], 5)]);
    assert_eq!(
        case_with(&g, true, &[1], Some(5), Some(9), false),
        Case::LocalDifferent(LocalReason::EpochNotClean)
    );
    assert_eq!(
        case_with(&g, true, &[1], Some(5), Some(9), true),
        Case::LocalDifferent(LocalReason::LocalChange)
    );
}

#[test]
fn paused_and_pending_never_apply() {
    let g = revision_graph(&[valid(1, &[], 5), valid(2, &[1], 6)]);
    let mut i = TargetInput {
        graph: &g,
        authority: &Authority::Holder(enr(1)),
        is_authority: false,
        paused: true,
        basis: &[h(1)],
        basis_checkpoint: Some(h(5)),
        live: Some(h(5)),
        epoch_clean: true,
    };
    assert_eq!(target_case(&i), Case::Paused);
    i.paused = false;
    i.authority = &Authority::Conflict;
    assert_eq!(target_case(&i), Case::HeldForPolicy);
    i.authority = &Authority::NotShared;
    assert_eq!(target_case(&i), Case::Off);
}

#[test]
fn undo_then_next_session_keeps_one_head_state() {
    let g = revision_graph(&[
        valid(1, &[], 5),
        valid(2, &[1], 6),
        valid(3, &[2], 5), // restore revision parents the basis
        valid(4, &[3], 7), // next auto session parents the restore
    ]);
    assert_eq!(g.heads, vec![h(4)]);
    assert_eq!(g.head_states.len(), 1);
}

fn pub_input(since: Option<u64>, clean: bool, compared: u64) -> PublishInput {
    PublishInput {
        authority: Authority::Holder(enr(1)),
        own: enr(1),
        paused: false,
        complete: true,
        kind: CaptureKind::SessionEnd,
        changed_in_session: true,
        epoch_clean: clean,
        authority_since: since,
        compared_with_local_seq: compared,
    }
}

#[test]
fn only_the_authority_publishes_and_only_what_changed_completely() {
    assert_eq!(publish_decision(&pub_input(None, true, 1)), Publish::Auto);
    let mut i = pub_input(None, true, 1);
    i.authority = Authority::Holder(enr(2));
    assert_eq!(publish_decision(&i), Publish::NotAuthority);
    let mut i = pub_input(None, true, 1);
    i.complete = false;
    assert_eq!(publish_decision(&i), Publish::Suspect);
    let mut i = pub_input(None, true, 1);
    i.changed_in_session = false;
    assert_eq!(publish_decision(&i), Publish::Unchanged);
    let mut i = pub_input(None, true, 1);
    i.paused = true;
    assert_eq!(publish_decision(&i), Publish::Paused);
    for k in [CaptureKind::Activation, CaptureKind::BeforeChange] {
        let mut i = pub_input(None, true, 1);
        i.kind = k;
        assert_eq!(publish_decision(&i), Publish::NeverPublishes);
    }
    let mut i = pub_input(None, true, 1);
    i.authority = Authority::NoMaster;
    assert_eq!(publish_decision(&i), Publish::NotAuthority);
}

#[test]
fn the_authority_epoch_holds_an_experiment_and_needs_a_strictly_later_capture() {
    // activation capture differs from the basis: epoch not clean, nothing publishes
    assert!(!epoch_clean(Some(h(9)), Some(h(5)), true));
    assert_eq!(
        publish_decision(&pub_input(Some(5), false, 6)),
        Publish::DirtyEpoch
    );
    // after the user's choice the epoch is clean again and a later session publishes
    assert!(epoch_clean(Some(h(5)), Some(h(5)), true));
    assert_eq!(
        publish_decision(&pub_input(Some(5), true, 6)),
        Publish::Auto
    );
    // isolated: clean epoch, comparison equal to authority_since must not publish
    assert_eq!(
        publish_decision(&pub_input(Some(5), true, 5)),
        Publish::BeforeEpoch
    );
    assert_eq!(
        publish_decision(&pub_input(Some(5), true, 4)),
        Publish::BeforeEpoch
    );
    // no basis: clean only if no shared head exists
    assert!(epoch_clean(Some(h(1)), None, false));
    assert!(!epoch_clean(Some(h(1)), None, true));
}

#[test]
fn paired_selection_order_and_no_cross_computer_ordering() {
    let x = h(1);
    let local = vec![
        LocalHistoryState {
            local_seq: 3,
            code_package_hash: Some(x),
            checkpoint: h(30),
        },
        LocalHistoryState {
            local_seq: 7,
            code_package_hash: Some(x),
            checkpoint: h(70),
        },
        LocalHistoryState {
            local_seq: 9,
            code_package_hash: Some(h(2)),
            checkpoint: h(90),
        },
    ];
    let remote = |cp: u8, anc: bool, code: u8| RemoteCandidate {
        checkpoint: h(cp),
        code_package_hash: Some(h(code)),
        ancestor_of_current: anc,
    };
    assert_eq!(
        paired_selection(Some(h(99)), &local, &[], &x),
        PairedChoice::Exact(h(99))
    );
    assert_eq!(
        paired_selection(None, &local, &[], &x),
        PairedChoice::LocalHistory(h(70))
    );
    assert_eq!(
        paired_selection(None, &[], &[remote(40, false, 1)], &x),
        PairedChoice::Remote(h(40))
    );
    match paired_selection(None, &[], &[remote(40, false, 1), remote(41, false, 1)], &x) {
        PairedChoice::CodeOnlyOffer(v) => assert_eq!(v.len(), 2),
        other => panic!("{other:?}"),
    }
    // ancestors and other code do not count
    assert_eq!(
        paired_selection(None, &[], &[remote(40, true, 1), remote(41, false, 2)], &x),
        PairedChoice::CodeOnlyWarning
    );
    assert_eq!(
        paired_selection(None, &[], &[], &x),
        PairedChoice::CodeOnlyWarning
    );
}

fn st(cp: u8, seq: u64, pins: Vec<Pin>) -> StateEntry {
    StateEntry {
        checkpoint: h(cp),
        local_seq: seq,
        pins,
    }
}

#[test]
fn three_states_survive_and_pins_extend_then_prune_back() {
    let mut states: Vec<StateEntry> = (1..=6).map(|i| st(i, i as u64, vec![])).collect();
    let p = prune_states(&states, &h(6), &BTreeSet::new());
    assert_eq!(p.keep, vec![h(6), h(5), h(4)]);
    assert_eq!(p.delete.len(), 3);
    assert!(p.exceptions.is_empty());
    // a pinned and a recovery-named checkpoint stay, visibly
    states[0].pins = vec![Pin::UnresolvedChoice];
    let named: BTreeSet<Sha256> = [h(2)].into();
    let p = prune_states(&states, &h(6), &named);
    assert!(p.keep.contains(&h(1)) && p.keep.contains(&h(2)));
    assert_eq!(p.delete, vec![h(3)]);
    assert_eq!(p.exceptions, vec![(h(1), Pin::UnresolvedChoice)]);
    // once the reasons end, back to three, oldest first, never the current one
    states[0].pins.clear();
    let p = prune_states(&states, &h(1), &BTreeSet::new());
    assert!(p.keep.contains(&h(1)), "the current state is never pruned");
    assert!(!p.delete.contains(&h(1)));
}

#[test]
fn every_root_kind_keeps_its_checkpoint() {
    let g = GcInputs {
        kept_states: vec![h(1)],
        pinned: vec![h(2)],
        recovery_point_roots: vec![h(3)],
        removal_roots: vec![h(4)],
        graph_heads: vec![h(5)],
        paired_candidates: vec![h(6)],
    };
    let r = gc_roots(&g);
    assert_eq!(r.len(), 6);
    assert!(!r.contains(&h(7)));
    assert!(gc_roots(&GcInputs::default()).is_empty());
}

#[test]
fn resume_explains_each_row_and_never_publishes() {
    use ResumeOption::*;
    let p = resume_preview(false, false);
    assert_eq!(
        (p.kind, p.options.clone()),
        (ResumeKind::NothingChanged, vec![Resume])
    );
    let p = resume_preview(true, false);
    assert_eq!(p.options, vec![KeepMineHere, ShareMine, GoBackToShared]);
    let p = resume_preview(false, true);
    assert_eq!(
        (p.kind, p.options.clone()),
        (ResumeKind::SharedNewer, vec![UseShared, StayPaused])
    );
    let p = resume_preview(true, true);
    assert_eq!(
        p.options,
        vec![StayPaused, UseSharedKeepMineAsRecovery, ShareMineInstead]
    );
    for (a, b) in [(false, false), (true, false), (false, true), (true, true)] {
        assert!(!resume_preview(a, b).publishes);
    }
}

#[test]
fn account_unit_never_matches_another_identity() {
    let a = SettingsUnit {
        folder: fk("a"),
        identity: Identity::account("ERIC").unwrap(),
    };
    let b = SettingsUnit {
        folder: fk("a"),
        identity: Identity::account("eric").unwrap(),
    };
    let c = SettingsUnit {
        folder: fk("a"),
        identity: Identity::character("ERIC", "Argent Dawn", "Thrall").unwrap(),
    };
    assert_ne!(a, b);
    assert_ne!(a, c);
    let s = settings_scope(
        &[decl("a", &["DB"], &[])],
        &[wtf("Account/ERIC/SavedVariables/a.lua", 1)],
        &included(&["ERIC"]),
        CAP,
    );
    assert_eq!(s.owned[0].unit, a);
    assert_ne!(s.owned[0].unit, b);
}
