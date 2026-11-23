//! Published content lineage of unrecognised add-ons (R-05, D-29, domain.md §6).
//!
//! An add-on without a selected `.toc` has no version label, so the only evidence that one copy
//! is newer than another is a chain of published records: every computer publishes, for each such
//! package, the copies its current copy descends from. A copy `x` is newer than `y` only when
//! `y` is reachable from `x` through published `ancestors` and `x` is not reachable from `y`.
//! Nothing local-only takes part: every computer reads the same records.

use crate::ids::Sha256;
use crate::inventory::{FolderStatus, Inventory, TocKind};
use crate::package::{group_packages, package_version, ClientInfo, PackageVersion};
use crate::paths::FolderKey;
use crate::records::{CodeState, PackageLineage, MAX_ANCESTORS};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A state's folders as an inventory (no file lists; those come from tree records).
pub fn state_inventory(state: &CodeState) -> Inventory {
    let mut folders = state.folders.clone();
    folders.sort_by(|a, b| a.key.cmp(&b.key));
    Inventory {
        folders,
        listing_stable: true,
    }
}

/// Whether the package has a folder without a selected `.toc` (it syncs by content).
pub fn is_unrecognised(pv: &PackageVersion) -> bool {
    pv.folders.iter().any(|f| f.toc.kind != TocKind::Selected)
}

fn whole(pv: &PackageVersion) -> bool {
    pv.folders
        .iter()
        .all(|f| f.status == FolderStatus::Ok && f.tree.is_some())
}

#[derive(Debug, Clone, Default)]
pub struct LineageGraph {
    parents: BTreeMap<(FolderKey, Sha256), Vec<Sha256>>,
}

impl LineageGraph {
    /// Only an entry that names the package hash its own publisher holds is accepted: a member
    /// cannot vouch for a copy it does not have (RV7-12).
    pub fn from_states<'a>(states: impl IntoIterator<Item = &'a CodeState>) -> Self {
        let states: Vec<&CodeState> = states.into_iter().collect();
        let invs: Vec<Inventory> = states.iter().map(|s| state_inventory(s)).collect();
        let observations: Vec<&Inventory> = invs.iter().collect();
        let packages = group_packages(&observations);
        let mut g = Self::default();
        for (s, inv) in states.iter().zip(&invs) {
            for l in &s.lineage {
                let held = packages
                    .iter()
                    .find(|p| p.key == l.package)
                    .and_then(|p| package_version(p, inv, &s.client))
                    .filter(whole)
                    .map(|pv| pv.hash);
                if held == Some(l.hash) {
                    g.add(l);
                }
            }
        }
        g
    }

    pub fn add(&mut self, l: &PackageLineage) {
        let list = self.parents.entry((l.package.clone(), l.hash)).or_default();
        for a in &l.ancestors {
            if !list.contains(a) {
                list.push(*a);
            }
        }
    }

    /// Every copy reachable from `hash` through published ancestors, nearest first.
    pub fn ancestors(&self, key: &FolderKey, hash: Sha256) -> Vec<Sha256> {
        let mut seen: BTreeSet<Sha256> = BTreeSet::new();
        let mut out = Vec::new();
        let mut queue: VecDeque<Sha256> = VecDeque::from([hash]);
        while let Some(h) = queue.pop_front() {
            for a in self.parents.get(&(key.clone(), h)).into_iter().flatten() {
                if *a != hash && seen.insert(*a) {
                    out.push(*a);
                    queue.push_back(*a);
                }
            }
        }
        out
    }

    /// `x` descends from `y` and `y` does not descend from `x`.
    pub fn newer(&self, key: &FolderKey, x: Sha256, y: Sha256) -> bool {
        x != y && self.ancestors(key, x).contains(&y) && !self.ancestors(key, y).contains(&x)
    }
}

/// The lineage this computer publishes with its next state: for each unrecognised, usable package
/// it holds, the copy it replaced (read from its own previous published state) followed by
/// everything the published records already know about the ancestors of both copies.
pub fn next_lineage(
    local: &Inventory,
    client: &ClientInfo,
    prev: Option<&CodeState>,
    peers: &[&CodeState],
) -> Vec<PackageLineage> {
    let peer_invs: Vec<Inventory> = peers.iter().map(|s| state_inventory(s)).collect();
    let mut observations: Vec<&Inventory> = vec![local];
    observations.extend(peer_invs.iter());
    let graph = LineageGraph::from_states(peers.iter().copied().chain(prev));
    let prev_inv = prev.map(state_inventory);
    let mut out = Vec::new();
    for pkg in group_packages(&observations) {
        let Some(now) = package_version(&pkg, local, client) else {
            continue;
        };
        if !is_unrecognised(&now) || !whole(&now) {
            continue;
        }
        let mut ancestors: Vec<Sha256> = Vec::new();
        let replaced = prev
            .zip(prev_inv.as_ref())
            .and_then(|(p, inv)| package_version(&pkg, inv, &p.client))
            .filter(|pv| whole(pv) && pv.hash != now.hash)
            .map(|pv| pv.hash);
        if let Some(h) = replaced {
            ancestors.push(h);
            ancestors.extend(graph.ancestors(&pkg.key, h));
        }
        ancestors.extend(graph.ancestors(&pkg.key, now.hash));
        let mut seen = BTreeSet::from([now.hash]);
        ancestors.retain(|h| seen.insert(*h));
        // Copies that an admitted peer still holds are kept ahead of the cap, so a peer that
        // was offline through many changes is still recognised as an ancestor (RV7-11).
        let held: BTreeSet<Sha256> = peers
            .iter()
            .zip(&peer_invs)
            .filter_map(|(s, inv)| package_version(&pkg, inv, &s.client))
            .filter(whole)
            .map(|pv| pv.hash)
            .collect();
        let anchors = ancestors.iter().filter(|h| held.contains(h)).count();
        let mut room = MAX_ANCESTORS.saturating_sub(anchors);
        ancestors.retain(|h| {
            held.contains(h) || {
                let keep = room > 0;
                room = room.saturating_sub(1);
                keep
            }
        });
        ancestors.truncate(MAX_ANCESTORS);
        out.push(PackageLineage {
            package: pkg.key.clone(),
            hash: now.hash,
            ancestors,
        });
    }
    out
}
