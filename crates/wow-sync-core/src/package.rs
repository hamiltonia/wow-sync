//! Folder metadata, package grouping and package comparison (domain.md §5).

use crate::hash::{package_hash, sha256};
use crate::ids::Sha256;
use crate::inventory::{FolderState, FolderStatus, Inventory, TocKind};
use crate::paths::FolderKey;
use crate::toc::{ExternalId, IdKind};
use crate::version::{classify, compare, Version, VersionOrder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    SharedId,
    Dependency,
    NamePrefix,
    Family,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub key: FolderKey,
    pub display: FolderKey,
    pub members: Vec<FolderKey>,
    pub edges: Vec<(FolderKey, FolderKey, EdgeKind)>,
}

/// Compatibility evidence of the computer holding a package version (not identity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub build_version: Option<String>,
    pub toc_mapping: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersion {
    pub package: FolderKey,
    pub hash: Sha256,
    pub folders: Vec<FolderState>,
    pub client: ClientInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetRelation {
    Equal,
    AHasMore,
    BHasMore,
    Different,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmbiguityReason {
    FolderNotUsable,
    /// A differing folder has no recognised `.toc` on one side: only content lineage can order
    /// it (R-05, D-29); with no shared copy to point to, the add-on needs one decision.
    UnrecognisedDiffers,
    VersionAmbiguous(String),
    MembersDisagree,
    SameLabelsDifferentContent,
    NoComparableVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEvidence {
    pub folder: String,
    pub a_label: Option<String>,
    pub b_label: Option<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageOrder {
    Same,
    Greater {
        set: SetRelation,
    },
    Less {
        set: SetRelation,
    },
    Ambiguous {
        reason: AmbiguityReason,
        set: SetRelation,
        evidence: Vec<FolderEvidence>,
    },
}

#[derive(Default)]
struct KeyInfo {
    ids: BTreeSet<ExternalId>,
    deps: BTreeSet<String>,
}

fn stem(key: &str) -> &str {
    key.find(['-', '_']).map_or(key, |i| &key[..i])
}

fn ids_differ(a: &BTreeSet<ExternalId>, b: &BTreeSet<ExternalId>) -> bool {
    [IdKind::Curse, IdKind::Wago, IdKind::WoWI].iter().any(|k| {
        let av: BTreeSet<&str> = a
            .iter()
            .filter(|i| i.kind == *k)
            .map(|i| i.value.as_str())
            .collect();
        let bv: BTreeSet<&str> = b
            .iter()
            .filter(|i| i.kind == *k)
            .map(|i| i.value.as_str())
            .collect();
        !av.is_empty() && !bv.is_empty() && av.is_disjoint(&bv)
    })
}

struct Dsu(Vec<usize>);

impl Dsu {
    fn find(&mut self, x: usize) -> usize {
        if self.0[x] != x {
            let r = self.find(self.0[x]);
            self.0[x] = r;
        }
        self.0[x]
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra.max(rb)] = ra.min(rb);
        }
    }
}

/// Union-find over folder keys using every observation, so all computers derive the same
/// packages from the same inputs.
pub fn group_packages(observations: &[&Inventory]) -> Vec<Package> {
    let mut info: BTreeMap<String, KeyInfo> = BTreeMap::new();
    for inv in observations {
        for f in &inv.folders {
            let e = info.entry(f.key.as_str().to_string()).or_default();
            if f.toc.kind == TocKind::Selected {
                e.ids.extend(f.toc.ids.iter().cloned());
                e.deps
                    .extend(f.toc.hard_deps.iter().map(|d| d.to_lowercase()));
            }
        }
    }
    let keys: Vec<&String> = info.keys().collect();
    let index: BTreeMap<&str, usize> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    let mut edges: BTreeSet<(usize, usize, EdgeKind)> = BTreeSet::new();

    let mut by_id: BTreeMap<&ExternalId, Vec<usize>> = BTreeMap::new();
    for (i, k) in keys.iter().enumerate() {
        for id in &info[*k].ids {
            by_id.entry(id).or_default().push(i);
        }
    }
    for members in by_id.values() {
        for w in members.windows(2) {
            edges.insert((w[0], w[1], EdgeKind::SharedId));
        }
    }
    for (fi, f) in keys.iter().enumerate() {
        for d in &info[*f].deps {
            if let Some(&gi) = index.get(d.as_str()) {
                if gi != fi && f.starts_with(stem(d)) {
                    edges.insert((fi, gi, EdgeKind::Dependency));
                }
            }
        }
        for (gi, g) in keys.iter().enumerate() {
            if gi == fi || f.len() <= g.len() || !f.starts_with(g.as_str()) {
                continue;
            }
            let next = f.as_bytes()[g.len()];
            if next == b'-' || next == b'_' {
                if f.len() > g.len() + 1 {
                    edges.insert((fi, gi, EdgeKind::NamePrefix));
                }
            } else if g.len() >= 5 && !ids_differ(&info[*f].ids, &info[*g].ids) {
                edges.insert((fi, gi, EdgeKind::Family));
            }
        }
    }

    let mut dsu = Dsu((0..keys.len()).collect());
    for (a, b, _) in &edges {
        dsu.union(*a, *b);
    }
    let mut comps: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..keys.len() {
        let r = dsu.find(i);
        comps.entry(r).or_default().push(i);
    }
    let fk = |i: usize| FolderKey::lossy(keys[i]);
    let mut out: Vec<Package> = comps
        .values()
        .map(|members| {
            let member_set: BTreeSet<usize> = members.iter().copied().collect();
            let pkg_edges: Vec<(FolderKey, FolderKey, EdgeKind)> = edges
                .iter()
                .filter(|(a, b, _)| member_set.contains(a) && member_set.contains(b))
                .map(|(a, b, k)| (fk(*a), fk(*b), *k))
                .collect();
            let dep_count = |i: usize| {
                info[keys[i]]
                    .deps
                    .iter()
                    .filter(|d| {
                        index
                            .get(d.as_str())
                            .is_some_and(|g| member_set.contains(g))
                    })
                    .count()
            };
            let with_id: Vec<usize> = members
                .iter()
                .copied()
                .filter(|i| !info[keys[*i]].ids.is_empty())
                .collect();
            let pool = if with_id.is_empty() {
                members.clone()
            } else {
                with_id
            };
            let display = pool
                .iter()
                .copied()
                .min_by_key(|i| (dep_count(*i), keys[*i].clone()))
                .unwrap_or(members[0]);
            Package {
                key: fk(members[0]),
                display: fk(display),
                members: members.iter().map(|i| fk(*i)).collect(),
                edges: pkg_edges,
            }
        })
        .collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

fn usable(f: &FolderState) -> bool {
    f.status == FolderStatus::Ok && f.tree.is_some()
}

/// One computer's folders of the package; `None` when it has none of them.
pub fn package_version(
    pkg: &Package,
    inv: &Inventory,
    client: &ClientInfo,
) -> Option<PackageVersion> {
    let folders: Vec<FolderState> = pkg
        .members
        .iter()
        .filter_map(|k| inv.get(k))
        .cloned()
        .collect();
    if folders.is_empty() {
        return None;
    }
    let hash = if folders.iter().all(usable) {
        let pairs: Vec<(FolderKey, Sha256)> = folders
            .iter()
            .filter_map(|f| f.tree.map(|t| (f.key.clone(), t)))
            .collect();
        package_hash(&pairs)
    } else {
        let mut text = String::from("wowsync-package-unusable-v1\n");
        for f in &folders {
            text.push_str(&format!("{}\t{:?}\t{:?}\n", f.key, f.status, f.tree));
        }
        sha256(text.as_bytes())
    };
    Some(PackageVersion {
        package: pkg.key.clone(),
        hash,
        folders,
        client: client.clone(),
    })
}

fn set_relation(a: &PackageVersion, b: &PackageVersion) -> SetRelation {
    let ka: BTreeSet<&FolderKey> = a.folders.iter().map(|f| &f.key).collect();
    let kb: BTreeSet<&FolderKey> = b.folders.iter().map(|f| &f.key).collect();
    if ka == kb {
        SetRelation::Equal
    } else if kb.is_subset(&ka) {
        SetRelation::AHasMore
    } else if ka.is_subset(&kb) {
        SetRelation::BHasMore
    } else {
        SetRelation::Different
    }
}

fn evidence(
    key: &FolderKey,
    a: Option<&FolderState>,
    b: Option<&FolderState>,
    outcome: &str,
) -> FolderEvidence {
    FolderEvidence {
        folder: key.as_str().to_string(),
        a_label: a.and_then(|f| f.toc.version.clone()),
        b_label: b.and_then(|f| f.toc.version.clone()),
        outcome: outcome.to_string(),
    }
}

pub fn compare_package(a: &PackageVersion, b: &PackageVersion) -> PackageOrder {
    let set = set_relation(a, b);
    // A folder that is not usable is checked before the hash shortcut so that equal
    // placeholder hashes of unusable folders never count as the same content.
    let unusable: Vec<FolderEvidence> = a
        .folders
        .iter()
        .chain(b.folders.iter())
        .filter(|f| !usable(f))
        .map(|f| {
            evidence(
                &f.key,
                Some(f),
                None,
                &format!("folder not usable ({:?})", f.status),
            )
        })
        .collect();
    if !unusable.is_empty() {
        return PackageOrder::Ambiguous {
            reason: AmbiguityReason::FolderNotUsable,
            set,
            evidence: unusable,
        };
    }
    if a.hash == b.hash {
        return PackageOrder::Same;
    }
    let mut ev = Vec::new();
    let mut less = false;
    let mut greater = false;
    let mut equal_label = false;
    let mut ambiguous: Option<AmbiguityReason> = None;
    let note = |r: AmbiguityReason, ambiguous: &mut Option<AmbiguityReason>| {
        if ambiguous.is_none() {
            *ambiguous = Some(r);
        }
    };
    for fa in &a.folders {
        let Some(fb) = b.folders.iter().find(|f| f.key == fa.key) else {
            continue;
        };
        if fa.tree == fb.tree {
            continue;
        }
        if fa.toc.kind != TocKind::Selected || fb.toc.kind != TocKind::Selected {
            ev.push(evidence(
                &fa.key,
                Some(fa),
                Some(fb),
                "unrecognised add-on: no version to compare, the copies differ",
            ));
            note(AmbiguityReason::UnrecognisedDiffers, &mut ambiguous);
            continue;
        }
        let (la, lb) = (
            classify(fa.toc.version.as_deref()),
            classify(fb.toc.version.as_deref()),
        );
        if la == Version::Missing && lb == Version::Missing {
            ev.push(evidence(&fa.key, Some(fa), Some(fb), "unversioned"));
            continue;
        }
        match compare(&la, &lb) {
            VersionOrder::Less => {
                less = true;
                ev.push(evidence(&fa.key, Some(fa), Some(fb), "older"));
            }
            VersionOrder::Greater => {
                greater = true;
                ev.push(evidence(&fa.key, Some(fa), Some(fb), "newer"));
            }
            VersionOrder::Equal => {
                equal_label = true;
                ev.push(evidence(
                    &fa.key,
                    Some(fa),
                    Some(fb),
                    "same label, different content",
                ));
            }
            VersionOrder::Ambiguous(why) => {
                ev.push(evidence(
                    &fa.key,
                    Some(fa),
                    Some(fb),
                    &format!("ambiguous: {why}"),
                ));
                note(AmbiguityReason::VersionAmbiguous(why), &mut ambiguous);
            }
        }
    }
    if let Some(reason) = ambiguous {
        return PackageOrder::Ambiguous {
            reason,
            set,
            evidence: ev,
        };
    }
    match (less, greater) {
        (true, true) => PackageOrder::Ambiguous {
            reason: AmbiguityReason::MembersDisagree,
            set,
            evidence: ev,
        },
        (false, true) => PackageOrder::Greater { set },
        (true, false) => PackageOrder::Less { set },
        (false, false) => {
            let reason = if equal_label {
                AmbiguityReason::SameLabelsDifferentContent
            } else {
                AmbiguityReason::NoComparableVersion
            };
            PackageOrder::Ambiguous {
                reason,
                set,
                evidence: ev,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{addon, addon_full, client, inventory};

    fn group(scans: Vec<crate::inventory::FolderScan>) -> Vec<Package> {
        let inv = inventory(scans);
        group_packages(&[&inv])
    }

    fn members(p: &[Package]) -> Vec<Vec<&str>> {
        p.iter()
            .map(|p| p.members.iter().map(|k| k.as_str()).collect())
            .collect()
    }

    #[test]
    fn shared_curse_id_groups_dbm() {
        let ids = [(IdKind::Curse, "3358")];
        let p = group(vec![
            addon_full("DBM-Core", Some("12.1.23"), &[], &ids, "a"),
            addon_full("DBM-Raids-Midnight", Some("12.1.23"), &[], &ids, "b"),
            addon("Other", Some("1.0"), "c"),
        ]);
        assert_eq!(
            members(&p),
            vec![vec!["dbm-core", "dbm-raids-midnight"], vec!["other"]]
        );
        assert!(p[0].edges.iter().any(|e| e.2 == EdgeKind::SharedId));
        assert_eq!(p[0].display.as_str(), "dbm-core");
    }

    #[test]
    fn dependency_edge_requires_stem_prefix() {
        let p = group(vec![
            addon("DBM-Core", Some("1.0"), "a"),
            addon_full("DBM-Party-BC", Some("1.0"), &["DBM-Core"], &[], "b"),
            addon("AtlasLootClassic", Some("1.0"), "c"),
            addon_full(
                "AtlasLootClassic_Data",
                Some("1.0"),
                &["AtlasLootClassic"],
                &[],
                "d",
            ),
            addon_full(
                "AtlasBIStooltips",
                Some("1.0"),
                &["AtlasLootClassic"],
                &[],
                "e",
            ),
        ]);
        assert_eq!(
            members(&p),
            vec![
                vec!["atlasbistooltips"],
                vec!["atlaslootclassic", "atlaslootclassic_data"],
                vec!["dbm-core", "dbm-party-bc"]
            ]
        );
        let dep = p
            .iter()
            .find(|p| p.members.len() == 2 && p.key.as_str() == "dbm-core")
            .unwrap();
        assert!(dep.edges.iter().any(|e| e.2 == EdgeKind::Dependency));
    }

    #[test]
    fn name_prefix_groups_languages_folder() {
        let p = group(vec![
            addon("GearQuestForever", Some("1.0"), "a"),
            addon("GearQuestForever_Languages", Some("1.0"), "b"),
        ]);
        assert_eq!(
            members(&p),
            vec![vec!["gearquestforever", "gearquestforever_languages"]]
        );
        assert_eq!(p[0].edges[0].2, EdgeKind::NamePrefix);
    }

    #[test]
    fn family_edge_groups_questie() {
        let p = group(vec![
            addon_full("Questie", Some("1.0"), &[], &[(IdKind::Curse, "100")], "a"),
            addon("QuestieDB", Some("1.0"), "b"),
            addon_full(
                "QuestieOther",
                Some("1.0"),
                &[],
                &[(IdKind::Curse, "200")],
                "c",
            ),
        ]);
        assert_eq!(
            members(&p),
            vec![vec!["questie", "questiedb"], vec!["questieother"]]
        );
        assert!(p[0].edges.iter().any(|e| e.2 == EdgeKind::Family));
    }

    #[test]
    fn family_edge_needs_five_char_stem() {
        let p = group(vec![
            addon("Atlas", Some("1.0"), "a"),
            addon("AtlasX", Some("1.0"), "b"),
            addon("Bags", Some("1.0"), "c"),
            addon("BagsPlus", Some("1.0"), "d"),
        ]);
        assert_eq!(
            members(&p),
            vec![vec!["atlas", "atlasx"], vec!["bags"], vec!["bagsplus"]]
        );
    }

    #[test]
    fn grouping_is_deterministic_under_input_order() {
        let a = inventory(vec![
            addon("Questie", Some("1.0"), "a"),
            addon("QuestieDB", Some("1.0"), "b"),
        ]);
        let b = inventory(vec![
            addon_full("Foo", Some("1"), &["Questie"], &[], "c"),
            addon("Questie", Some("1.0"), "a"),
        ]);
        let one = group_packages(&[&a, &b]);
        let two = group_packages(&[&b, &a]);
        assert_eq!(one, two);
        let rev = inventory(vec![
            addon("QuestieDB", Some("1.0"), "b"),
            addon("Questie", Some("1.0"), "a"),
        ]);
        assert_eq!(group_packages(&[&a]), group_packages(&[&rev]));
    }

    #[test]
    fn membership_change_changes_hash() {
        let before = inventory(vec![addon("Suite", Some("1.0"), "a")]);
        let after = inventory(vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_Data", Some("1.0"), "b"),
        ]);
        let c = client("1.0");
        let pb = &group_packages(&[&before])[0];
        let pa = &group_packages(&[&after])[0];
        let h1 = package_version(pb, &before, &c).unwrap().hash;
        let h2 = package_version(pa, &after, &c).unwrap().hash;
        assert_ne!(h1, h2);
        assert!(
            package_version(pa, &inventory(vec![addon("Elsewhere", Some("1"), "x")]), &c).is_none()
        );
    }

    fn pair(
        a: Vec<crate::inventory::FolderScan>,
        b: Vec<crate::inventory::FolderScan>,
    ) -> PackageOrder {
        let (ia, ib) = (inventory(a), inventory(b));
        let pkgs = group_packages(&[&ia, &ib]);
        assert_eq!(pkgs.len(), 1, "{pkgs:?}");
        let c = client("1.0");
        compare_package(
            &package_version(&pkgs[0], &ia, &c).unwrap(),
            &package_version(&pkgs[0], &ib, &c).unwrap(),
        )
    }

    #[test]
    fn identical_trees_compare_same() {
        let s = || {
            vec![
                addon("Suite", Some("1.0"), "a"),
                addon("Suite_Data", Some("1.0"), "b"),
            ]
        };
        assert_eq!(pair(s(), s()), PackageOrder::Same);
    }

    #[test]
    fn newer_member_makes_package_greater() {
        let old = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        let new = vec![
            addon("Suite", Some("1.1"), "a2"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        assert_eq!(
            pair(new.clone(), old.clone()),
            PackageOrder::Greater {
                set: SetRelation::Equal
            }
        );
        assert_eq!(
            pair(old, new),
            PackageOrder::Less {
                set: SetRelation::Equal
            }
        );
    }

    #[test]
    fn mixed_member_orders_are_ambiguous() {
        let a = vec![
            addon("Suite", Some("1.1"), "a2"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        let b = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_Data", Some("1.1"), "b2"),
        ];
        match pair(a, b) {
            PackageOrder::Ambiguous {
                reason, evidence, ..
            } => {
                assert_eq!(reason, AmbiguityReason::MembersDisagree);
                assert_eq!(evidence.len(), 2);
            }
            other => panic!("{other:?}"),
        }
        let c = vec![
            addon("Suite", Some("1.1"), "a2"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        let d = vec![
            addon("Suite", Some("r5"), "a"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        assert!(matches!(
            pair(c, d),
            PackageOrder::Ambiguous {
                reason: AmbiguityReason::VersionAmbiguous(_),
                ..
            }
        ));
    }

    #[test]
    fn same_labels_different_content_is_ambiguous() {
        let a = vec![addon("Suite", Some("1.0"), "a")];
        let b = vec![addon("Suite", Some("1.0"), "different")];
        assert!(matches!(
            pair(a, b),
            PackageOrder::Ambiguous {
                reason: AmbiguityReason::SameLabelsDifferentContent,
                ..
            }
        ));
        let c = vec![addon("Suite", None, "a")];
        let d = vec![addon("Suite", None, "b")];
        assert!(matches!(
            pair(c, d),
            PackageOrder::Ambiguous {
                reason: AmbiguityReason::NoComparableVersion,
                ..
            }
        ));
    }

    #[test]
    fn modified_content_same_label_ambiguous() {
        // A hand-edited copy keeps its label: it must not win or lose, it is ambiguous.
        let released = vec![
            addon("Suite", Some("2.0"), "released"),
            addon("Suite_Data", Some("2.0"), "d"),
        ];
        let edited = vec![
            addon("Suite", Some("2.0"), "released + local edit"),
            addon("Suite_Data", Some("2.0"), "d"),
        ];
        match pair(released, edited) {
            PackageOrder::Ambiguous {
                reason, evidence, ..
            } => {
                assert_eq!(reason, AmbiguityReason::SameLabelsDifferentContent);
                assert!(evidence.iter().any(|e| e.outcome.contains("same label")));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unusable_member_is_ambiguous() {
        let mut bad = addon("Suite", Some("2.0"), "x");
        bad.status = crate::inventory::ScanStatus::Unreadable("locked".into());
        let good = addon("Suite", Some("1.0"), "x");
        assert!(matches!(
            pair(vec![bad.clone()], vec![good]),
            PackageOrder::Ambiguous {
                reason: AmbiguityReason::FolderNotUsable,
                ..
            }
        ));
        assert!(matches!(
            pair(vec![bad.clone()], vec![bad]),
            PackageOrder::Ambiguous {
                reason: AmbiguityReason::FolderNotUsable,
                ..
            }
        ));
    }

    #[test]
    fn unversioned_member_follows_versioned_result() {
        let ids = [(IdKind::Curse, "1")];
        let mk = |core: &str, core_c: &str, bra_c: &str| {
            vec![
                addon_full("DBM-Core", Some(core), &[], &ids, core_c),
                addon_full("DBM-Brawlers", None, &[], &ids, bra_c),
            ]
        };
        assert_eq!(
            pair(mk("2.0", "a2", "b2"), mk("1.0", "a", "b")),
            PackageOrder::Greater {
                set: SetRelation::Equal
            }
        );
    }

    #[test]
    fn set_relation_reported() {
        let full = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_Data", Some("1.0"), "b"),
        ];
        let part = vec![addon("Suite", Some("1.1"), "a2")];
        assert_eq!(
            pair(full.clone(), part.clone()),
            PackageOrder::Less {
                set: SetRelation::AHasMore
            }
        );
        assert_eq!(
            pair(part, full),
            PackageOrder::Greater {
                set: SetRelation::BHasMore
            }
        );
        let a = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_A", Some("1.0"), "x"),
        ];
        let b = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Suite_B", Some("1.0"), "y"),
        ];
        match pair(a, b) {
            PackageOrder::Ambiguous { set, .. }
            | PackageOrder::Greater { set }
            | PackageOrder::Less { set } => {
                assert_eq!(set, SetRelation::Different)
            }
            PackageOrder::Same => panic!(),
        }
    }
}
