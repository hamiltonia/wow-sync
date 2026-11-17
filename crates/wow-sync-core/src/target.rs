//! Group target computation (domain.md §6, R-05, R-06).

use crate::admit::{MemberLatest, MemberView};
use crate::explain::{AmbiguousCandidate, Explanation};
use crate::hash::package_hash;
use crate::ids::{ComputerId, GroupId, Sha256};
use crate::inventory::{FileEntry, FolderState, FolderStatus, Inventory, TocKind};
use crate::package::{
    compare_package, group_packages, package_version, ClientInfo, Package, PackageOrder,
    PackageVersion,
};
use crate::paths::FolderKey;
use crate::records::{CodeState, FolderTreeRecord, GroupRecord};
use crate::variant::ConfirmedVariant;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeAvail {
    Ok(FolderTreeRecord),
    Missing,
    CloudOnly,
    Corrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionReason {
    Ambiguous,
    NoCompatibleToc,
    DifferentTocMapping,
    /// Not produced by core since D-21 (no build gating); kept only for the prototype crate.
    DifferentClientBuild,
    UnvalidatedTocMapping,
    PartialCandidate,
}

pub struct TargetInput<'a> {
    pub group: &'a GroupRecord,
    pub variant: &'a ConfirmedVariant,
    pub local: &'a Inventory,
    pub local_client: &'a ClientInfo,
    pub local_id: &'a ComputerId,
    pub members: &'a [MemberView],
    pub trees: &'a BTreeMap<Sha256, TreeAvail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageTarget {
    InSync,
    Chosen {
        version: PackageVersion,
        sources: Vec<ComputerId>,
        why: Explanation,
    },
    NeedsDecision {
        reason: DecisionReason,
        candidates: Vec<(PackageVersion, Vec<ComputerId>)>,
        why: Explanation,
    },
    Unavailable {
        why: Explanation,
    },
    /// A folder of this package is blocked for this client: never published or replaced.
    NotSynced {
        why: Explanation,
    },
}

/// The package content this computer should end up with (for `InSync` and `Chosen`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTarget {
    pub hash: Sha256,
    pub folders: Vec<(FolderKey, Sha256)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTarget {
    pub group_id: GroupId,
    pub packages: BTreeMap<FolderKey, PackageTarget>,
    pub effective: BTreeMap<FolderKey, EffectiveTarget>,
    pub local_hash: BTreeMap<FolderKey, Sha256>,
}

struct Cand {
    pv: PackageVersion,
    holders: Vec<(ComputerId, ClientInfo)>,
}

fn state_inventory(state: &CodeState) -> Inventory {
    let mut folders = state.folders.clone();
    folders.sort_by(|a, b| a.key.cmp(&b.key));
    Inventory {
        folders,
        listing_stable: true,
    }
}

fn label(pv: &PackageVersion, pkg: &Package) -> String {
    let pick = pv
        .folders
        .iter()
        .find(|f| f.key == pkg.display)
        .filter(|f| f.toc.version.is_some())
        .or_else(|| pv.folders.iter().find(|f| f.toc.version.is_some()));
    pick.and_then(|f| f.toc.version.clone())
        .unwrap_or_else(|| "unversioned".to_string())
}

fn display_name(pv: &PackageVersion, pkg: &Package) -> String {
    pv.folders
        .iter()
        .find(|f| f.key == pkg.display)
        .map_or_else(|| pkg.display.as_str().to_string(), |f| f.name.clone())
}

type Ineligible = (DecisionReason, Explanation);

fn folder_problem(f: &FolderState) -> Option<Ineligible> {
    if f.status != FolderStatus::Ok {
        return Some((
            DecisionReason::NoCompatibleToc,
            Explanation::NoCompatibleToc {
                folder: f.name.clone(),
            },
        ));
    }
    match f.toc.kind {
        TocKind::Selected => None,
        TocKind::Unvalidated => Some((
            DecisionReason::UnvalidatedTocMapping,
            Explanation::UnvalidatedTocMapping {
                folder: f.name.clone(),
            },
        )),
        _ => Some((
            DecisionReason::NoCompatibleToc,
            Explanation::NoCompatibleToc {
                folder: f.name.clone(),
            },
        )),
    }
}

/// Eligible holders of the candidate, or the first reason it cannot be applied automatically.
fn eligible_holders(
    c: &Cand,
    pkg: &Package,
    input: &TargetInput,
) -> Result<Vec<(ComputerId, ClientInfo)>, Ineligible> {
    if let Some(p) = c.pv.folders.iter().find_map(folder_problem) {
        return Err(p);
    }
    if let Some(f) = input
        .local
        .folders
        .iter()
        .find(|f| pkg.members.contains(&f.key) && f.toc.kind == TocKind::Unvalidated)
    {
        return Err((
            DecisionReason::UnvalidatedTocMapping,
            Explanation::UnvalidatedTocMapping {
                folder: f.name.clone(),
            },
        ));
    }
    let mut first_fail: Option<Ineligible> = None;
    let mut ok = Vec::new();
    for (id, client) in &c.holders {
        if client.toc_mapping != input.local_client.toc_mapping {
            first_fail.get_or_insert((
                DecisionReason::DifferentTocMapping,
                Explanation::DifferentTocMapping {
                    here: input.local_client.toc_mapping.clone(),
                    there: client.toc_mapping.clone(),
                },
            ));
        } else {
            ok.push((id.clone(), client.clone()));
        }
    }
    if ok.is_empty() {
        return Err(first_fail.expect("candidates always have a holder"));
    }
    let have: BTreeSet<&FolderKey> = c.pv.folders.iter().map(|f| &f.key).collect();
    let missing: Vec<String> = pkg
        .members
        .iter()
        .filter(|k| !have.contains(k))
        .map(|k| k.as_str().to_string())
        .collect();
    if !missing.is_empty() {
        return Err((
            DecisionReason::PartialCandidate,
            Explanation::PartialCandidate { missing },
        ));
    }
    Ok(ok)
}

fn effective(pv: &PackageVersion) -> EffectiveTarget {
    EffectiveTarget {
        hash: pv.hash,
        folders: pv
            .folders
            .iter()
            .filter_map(|f| f.tree.map(|t| (f.key.clone(), t)))
            .collect(),
    }
}

pub fn compute_target(input: &TargetInput) -> GroupTarget {
    let mut peers: Vec<(&ComputerId, u64, &CodeState)> = Vec::new();
    for m in input.members {
        if m.computer == *input.local_id {
            continue;
        }
        if let MemberLatest::Valid { seq, state } = &m.latest {
            if state.group_id == input.group.group_id
                && state.product == input.group.product
                && state.computer_id == m.computer
            {
                peers.push((&m.computer, *seq, state));
            }
        }
    }
    peers.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let peer_invs: Vec<Inventory> = peers.iter().map(|p| state_inventory(p.2)).collect();
    let mut observations: Vec<&Inventory> = vec![input.local];
    observations.extend(peer_invs.iter());

    let mut out = GroupTarget {
        group_id: input.group.group_id.clone(),
        packages: BTreeMap::new(),
        effective: BTreeMap::new(),
        local_hash: BTreeMap::new(),
    };
    for pkg in group_packages(&observations) {
        let mut cands: BTreeMap<Sha256, Cand> = BTreeMap::new();
        let local_pv = package_version(&pkg, input.local, input.local_client);
        if let Some(pv) = &local_pv {
            out.local_hash.insert(pkg.key.clone(), pv.hash);
            cands.insert(
                pv.hash,
                Cand {
                    pv: pv.clone(),
                    holders: vec![(input.local_id.clone(), input.local_client.clone())],
                },
            );
        }
        for (i, (id, _, state)) in peers.iter().enumerate() {
            if let Some(pv) = package_version(&pkg, &peer_invs[i], &state.client) {
                let holder = ((*id).clone(), state.client.clone());
                let entry = cands.entry(pv.hash).or_insert_with(|| Cand {
                    pv,
                    holders: Vec::new(),
                });
                if !entry.holders.contains(&holder) {
                    entry.holders.push(holder);
                }
            }
        }
        for c in cands.values_mut() {
            c.holders.sort_by(|a, b| a.0.cmp(&b.0));
        }
        let list: Vec<&Cand> = cands.values().collect();
        let holds_local = |c: &Cand| c.holders.iter().any(|(id, _)| id == input.local_id);

        let dominator = list.iter().find(|c| {
            list.iter().all(|x| {
                x.pv.hash == c.pv.hash
                    || matches!(
                        compare_package(&c.pv, &x.pv),
                        PackageOrder::Same | PackageOrder::Greater { .. }
                    )
            })
        });
        let target = match dominator {
            Some(c) if holds_local(c) => {
                match c
                    .pv
                    .folders
                    .iter()
                    .filter(|f| f.status == FolderStatus::Ok)
                    .find_map(folder_problem)
                {
                    // A blocked local folder is never published or replaced: say so (M2).
                    Some((_, why)) => PackageTarget::NotSynced { why },
                    None => {
                        out.effective.insert(pkg.key.clone(), effective(&c.pv));
                        PackageTarget::InSync
                    }
                }
            }
            Some(c) => match eligible_holders(c, &pkg, input) {
                Err((reason, why)) => PackageTarget::NeedsDecision {
                    reason,
                    candidates: decision_candidates(&list),
                    why,
                },
                Ok(holders) => choose(&mut out, &pkg, c, holders, local_pv.as_ref(), input),
            },
            None => {
                let mut evidence = Vec::new();
                'outer: for a in &list {
                    for b in &list {
                        if let PackageOrder::Ambiguous { evidence: e, .. } =
                            compare_package(&a.pv, &b.pv)
                        {
                            evidence = e;
                            break 'outer;
                        }
                    }
                }
                PackageTarget::NeedsDecision {
                    reason: DecisionReason::Ambiguous,
                    candidates: decision_candidates(&list),
                    why: Explanation::AmbiguousVersions {
                        evidence,
                        candidates: list
                            .iter()
                            .map(|c| AmbiguousCandidate {
                                label: label(&c.pv, &pkg),
                                sources: c
                                    .holders
                                    .iter()
                                    .filter(|(id, _)| id != input.local_id)
                                    .map(|(id, _)| id.clone())
                                    .collect(),
                                here: holds_local(c),
                            })
                            .collect(),
                    },
                }
            }
        };
        out.packages.insert(pkg.key.clone(), target);
    }
    out
}

fn decision_candidates(list: &[&Cand]) -> Vec<(PackageVersion, Vec<ComputerId>)> {
    list.iter()
        .map(|c| {
            (
                c.pv.clone(),
                c.holders.iter().map(|h| h.0.clone()).collect(),
            )
        })
        .collect()
}

fn choose(
    out: &mut GroupTarget,
    pkg: &Package,
    c: &Cand,
    holders: Vec<(ComputerId, ClientInfo)>,
    local_pv: Option<&PackageVersion>,
    input: &TargetInput,
) -> PackageTarget {
    let mut version = c.pv.clone();
    version.client = holders[0].1.clone();
    let mut missing = Vec::new();
    let mut corrupt = None;
    for f in &mut version.folders {
        let Some(tree) = f.tree else { continue };
        match input.trees.get(&tree) {
            Some(TreeAvail::Ok(rec)) => {
                f.file_entries = rec
                    .files
                    .iter()
                    .map(|e| FileEntry {
                        path: e.path.clone(),
                        size: e.size,
                        sha256: e.sha256,
                    })
                    .collect();
            }
            Some(TreeAvail::Corrupt) => corrupt = corrupt.or(Some(tree)),
            Some(TreeAvail::Missing | TreeAvail::CloudOnly) | None => missing.push(tree),
        }
    }
    if let Some(tree) = corrupt {
        return PackageTarget::Unavailable {
            why: Explanation::TreeCorrupt { tree },
        };
    }
    if !missing.is_empty() {
        return PackageTarget::Unavailable {
            why: Explanation::NotAvailableYet { missing },
        };
    }
    let sources: Vec<ComputerId> = holders.into_iter().map(|h| h.0).collect();
    let why = match local_pv {
        Some(l) => Explanation::NewerVersion {
            folder: display_name(&c.pv, pkg),
            from: label(l, pkg),
            to: label(&c.pv, pkg),
            source: sources[0].clone(),
        },
        None => Explanation::Installed {
            source: sources[0].clone(),
        },
    };
    debug_assert_eq!(
        version.hash,
        package_hash(
            &version
                .folders
                .iter()
                .filter_map(|f| f.tree.map(|t| (f.key.clone(), t)))
                .collect::<Vec<_>>()
        )
    );
    out.effective.insert(pkg.key.clone(), effective(&version));
    PackageTarget::Chosen {
        version,
        sources,
        why,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{addon, addon_interface, addon_unvalidated, client, computer, Scenario};

    fn suite(v: &str, tag: &str) -> Vec<crate::inventory::FolderScan> {
        vec![addon("Suite", Some(v), tag)]
    }

    fn pkg<'a>(t: &'a GroupTarget, key: &str) -> &'a PackageTarget {
        &t.packages[&FolderKey::lossy(key)]
    }

    fn reason(t: &PackageTarget) -> DecisionReason {
        match t {
            PackageTarget::NeedsDecision { reason, .. } => *reason,
            other => panic!("expected NeedsDecision, got {other:?}"),
        }
    }

    #[test]
    fn newest_demonstrable_version_chosen() {
        let t = Scenario::new(suite("1.0", "a"))
            .peer(2, suite("1.1", "b"))
            .peer(3, suite("1.0", "a"))
            .target();
        match pkg(&t, "suite") {
            PackageTarget::Chosen {
                version,
                sources,
                why,
            } => {
                assert_eq!(sources, &vec![computer(2)]);
                assert_eq!(version.folders[0].toc.version.as_deref(), Some("1.1"));
                assert!(
                    matches!(why, Explanation::NewerVersion { from, to, .. } if from == "1.0" && to == "1.1")
                );
                assert!(!version.folders[0].file_entries.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn independent_packages_converge() {
        let a = vec![
            addon("Alpha", Some("2.0"), "a2"),
            addon("Beta", Some("1.0"), "b1"),
        ];
        let b = vec![
            addon("Alpha", Some("1.0"), "a1"),
            addon("Beta", Some("2.0"), "b2"),
        ];
        let on_a = Scenario::new(a.clone()).peer(2, b.clone()).target();
        let on_b = Scenario::new(b).peer(2, a).target();
        assert_eq!(pkg(&on_a, "alpha"), &PackageTarget::InSync);
        assert!(matches!(pkg(&on_a, "beta"), PackageTarget::Chosen { .. }));
        assert!(matches!(pkg(&on_b, "alpha"), PackageTarget::Chosen { .. }));
        assert_eq!(pkg(&on_b, "beta"), &PackageTarget::InSync);
        assert_eq!(
            on_a.effective[&FolderKey::lossy("alpha")],
            on_b.effective[&FolderKey::lossy("alpha")]
        );
        assert_eq!(
            on_a.effective[&FolderKey::lossy("beta")],
            on_b.effective[&FolderKey::lossy("beta")]
        );
    }

    #[test]
    fn ambiguity_blocks_only_that_package() {
        let local = vec![
            addon("Alpha", Some("1.0"), "mine"),
            addon("Beta", Some("1.0"), "b1"),
        ];
        let peer = vec![
            addon("Alpha", Some("1.0"), "theirs"),
            addon("Beta", Some("2.0"), "b2"),
        ];
        let t = Scenario::new(local).peer(2, peer).target();
        assert_eq!(reason(pkg(&t, "alpha")), DecisionReason::Ambiguous);
        assert!(matches!(pkg(&t, "beta"), PackageTarget::Chosen { .. }));
    }

    #[test]
    fn dominating_candidate_chosen_despite_other_ambiguity() {
        let t = Scenario::new(suite("1.0", "a"))
            .peer(2, suite("2.0", "z"))
            .peer(3, suite("1.0", "different"))
            .target();
        match pkg(&t, "suite") {
            PackageTarget::Chosen { sources, .. } => assert_eq!(sources, &vec![computer(2)]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ineligible_dominating_candidate_no_fallback() {
        let t = Scenario::new(suite("1.0", "a"))
            .peer_client(
                2,
                suite("2.0", "z"),
                mapped("1.15.8.12345", "profile:wowf/1"),
            )
            .peer(3, suite("1.5", "m"))
            .target();
        assert_eq!(
            reason(pkg(&t, "suite")),
            DecisionReason::DifferentTocMapping
        );
        assert!(!t.effective.contains_key(&FolderKey::lossy("suite")));
    }

    #[test]
    fn result_independent_of_input_order_and_duplicates() {
        let base = Scenario::new(suite("1.0", "a"))
            .peer(2, suite("1.2", "b"))
            .peer(3, suite("1.1", "c"))
            .peer(4, suite("1.2", "b"));
        let expected = base.target();
        let mut reversed = base.clone();
        reversed.members.reverse();
        assert_eq!(reversed.target(), expected);
        let mut dup = base.clone();
        let again = dup.members.clone();
        dup.members.extend(again);
        assert_eq!(dup.target(), expected);
        match pkg(&expected, "suite") {
            PackageTarget::Chosen { sources, .. } => {
                assert_eq!(sources, &vec![computer(2), computer(4)])
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn record_timestamps_cannot_influence_target() {
        let base = Scenario::new(suite("1.0", "mine"))
            .peer(2, suite("1.0", "x"))
            .peer(3, suite("1.0", "y"));
        let first = base.target();
        assert_eq!(reason(pkg(&first, "suite")), DecisionReason::Ambiguous);
        let skewed = base.clone().written(2, 1).written(3, u64::MAX / 4);
        assert_eq!(skewed.target(), first);
        let flipped = base.written(2, u64::MAX / 4).written(3, 1);
        assert_eq!(flipped.target(), first);
    }

    #[test]
    fn other_variant_state_ignored() {
        let sc = Scenario::new(suite("1.0", "a"))
            .peer(2, suite("2.0", "b"))
            .peer(3, suite("3.0", "c"))
            .map_state(2, |s| {
                s.group_id = crate::ids::GroupId::parse(&"cd".repeat(16)).unwrap()
            })
            .map_state(3, |s| s.product = "wow_test".into());
        assert_eq!(pkg(&sc.target(), "suite"), &PackageTarget::InSync);
    }

    fn suite_iface(v: &str, iface: &str, tag: &str) -> Vec<crate::inventory::FolderScan> {
        vec![addon_interface("Suite", v, Some(iface), tag)]
    }

    fn mapped(build: &str, mapping: &str) -> ClientInfo {
        let mut c = client(build);
        c.toc_mapping = mapping.into();
        c
    }

    #[test]
    fn different_toc_mapping_needs_decision() {
        let t = Scenario::new(suite("1.0", "a"))
            .peer_client(
                2,
                suite("2.0", "b"),
                mapped("1.15.8.12345", "profile:wowf/1"),
            )
            .target();
        assert_eq!(
            reason(pkg(&t, "suite")),
            DecisionReason::DifferentTocMapping
        );
    }

    #[test]
    fn different_build_syncs_whatever_the_interface_says() {
        // D-21: a game update or an `## Interface:` declaration never withholds an addon.
        let cases: [(Option<&str>, &str); 5] = [
            (Some("11508"), "same interface"),
            (Some("99999"), "different interface"),
            (Some("11508,garbage"), "malformed token"),
            (Some(""), "empty header"),
            (None, "no header"),
        ];
        for (iface, what) in cases {
            let theirs = vec![addon_interface("Suite", "2.0", iface, "b")];
            let t = Scenario::new(suite("1.0", "a"))
                .peer_client(2, theirs, client("2.0.0.1"))
                .target();
            assert!(
                matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }),
                "{what}"
            );
        }
    }

    #[test]
    fn unknown_build_and_new_addon_sync() {
        let unknown = ClientInfo {
            build_version: None,
            toc_mapping: client("x").toc_mapping,
        };
        let t = Scenario::new(suite("1.0", "a"))
            .peer_client(2, suite_iface("2.0", "99999", "b"), unknown)
            .target();
        assert!(matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
        let t = Scenario::new(vec![])
            .peer_client(2, suite_iface("1.0", "99999", "b"), client("2.0.0.1"))
            .target();
        assert!(matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
    }

    #[test]
    fn multi_folder_package_syncs_across_builds_and_interfaces() {
        let peer = vec![
            addon_interface("Suite", "2.0", Some("11508"), "c2"),
            addon_interface("Suite_Data", "2.0", Some("99999"), "d2"),
        ];
        let t = Scenario::new(two_part("1.0", "c1", "1.0"))
            .peer_client(2, peer, client("2.0.0.1"))
            .target();
        assert!(matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
    }

    #[test]
    fn foreign_suffix_only_candidate_not_installed() {
        let t = Scenario::new(suite("1.0", "a"))
            .peer(2, vec![addon_unvalidated("Suite", "2.0", "b")])
            .target();
        assert!(!matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
        assert_eq!(reason(pkg(&t, "suite")), DecisionReason::Ambiguous);
        let t = Scenario::new(vec![])
            .peer(2, vec![addon_unvalidated("Suite", "2.0", "b")])
            .target();
        assert_eq!(
            reason(pkg(&t, "suite")),
            DecisionReason::UnvalidatedTocMapping
        );
    }

    #[test]
    fn unvalidated_mapping_candidate_not_installed() {
        let t = Scenario::new(vec![addon_unvalidated("Suite", "1.0", "a")])
            .peer(2, suite("2.0", "b"))
            .target();
        assert_eq!(reason(pkg(&t, "suite")), DecisionReason::Ambiguous);
    }

    fn two_part(core: &str, core_tag: &str, data: &str) -> Vec<crate::inventory::FolderScan> {
        vec![
            addon("Suite", Some(core), core_tag),
            addon("Suite_Data", Some(data), "data"),
        ]
    }

    #[test]
    fn partial_suite_candidate_needs_decision() {
        let on_a = Scenario::new(two_part("1.0", "c1", "1.0"))
            .peer(2, vec![addon("Suite", Some("2.0"), "c2")])
            .target();
        assert_eq!(
            reason(pkg(&on_a, "suite")),
            DecisionReason::PartialCandidate
        );
        let on_b = Scenario::new(vec![addon("Suite", Some("2.0"), "c2")])
            .peer(2, two_part("1.0", "c1", "1.0"))
            .target();
        assert_eq!(pkg(&on_b, "suite"), &PackageTarget::InSync);
    }

    #[test]
    fn partial_candidate_not_installed_on_empty_peer() {
        let t = Scenario::new(vec![])
            .peer(2, two_part("1.0", "c1", "1.0"))
            .peer(3, vec![addon("Suite", Some("2.0"), "c2")])
            .target();
        match pkg(&t, "suite") {
            PackageTarget::NeedsDecision { reason, why, .. } => {
                assert_eq!(*reason, DecisionReason::PartialCandidate);
                assert!(
                    matches!(why, Explanation::PartialCandidate { missing } if missing == &vec!["suite_data".to_string()])
                );
            }
            other => panic!("{other:?}"),
        }
        assert!(t.effective.is_empty());
    }

    #[test]
    fn complete_candidate_installed_on_empty_peer() {
        let t = Scenario::new(vec![])
            .peer(2, two_part("1.0", "c1", "1.0"))
            .target();
        match pkg(&t, "suite") {
            PackageTarget::Chosen {
                version,
                sources,
                why,
            } => {
                assert_eq!(version.folders.len(), 2);
                assert_eq!(sources, &vec![computer(2)]);
                assert!(matches!(why, Explanation::Installed { .. }));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn corrupt_tree_makes_package_unavailable() {
        let mut sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        let tree = sc.peer_tree(2, "suite");
        sc.trees.insert(tree, TreeAvail::Corrupt);
        assert!(matches!(
            pkg(&sc.target(), "suite"),
            PackageTarget::Unavailable {
                why: Explanation::TreeCorrupt { .. }
            }
        ));
        sc.trees.insert(tree, TreeAvail::CloudOnly);
        assert!(matches!(
            pkg(&sc.target(), "suite"),
            PackageTarget::Unavailable {
                why: Explanation::NotAvailableYet { .. }
            }
        ));
        sc.trees.remove(&tree);
        assert!(matches!(
            pkg(&sc.target(), "suite"),
            PackageTarget::Unavailable {
                why: Explanation::NotAvailableYet { .. }
            }
        ));
    }

    #[test]
    fn absent_package_is_not_removal() {
        let t = Scenario::new(vec![addon("Alpha", Some("1.0"), "a")])
            .peer(2, vec![addon("Beta", Some("1.0"), "b")])
            .target();
        assert_eq!(pkg(&t, "alpha"), &PackageTarget::InSync);
        assert!(matches!(
            pkg(&t, "beta"),
            PackageTarget::Chosen {
                why: Explanation::Installed { .. },
                ..
            }
        ));
        assert_eq!(t.packages.len(), 2);
    }

    #[test]
    fn unavailable_member_is_not_absence() {
        let with = Scenario::new(suite("1.0", "a")).peer(2, suite("1.0", "a"));
        let mut gone = with.clone();
        gone.make_unusable(2);
        let t = gone.target();
        assert_eq!(pkg(&t, "suite"), &PackageTarget::InSync);
        assert_eq!(t.effective, with.target().effective);
        let mut regressed = with;
        regressed.members[0].latest = MemberLatest::Regressed { max_seq_seen: 4 };
        assert_eq!(pkg(&regressed.target(), "suite"), &PackageTarget::InSync);
    }
}
