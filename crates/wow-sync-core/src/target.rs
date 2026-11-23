//! Group target computation (domain.md §6, R-05, R-06).

use crate::admit::{MemberLatest, MemberView};
use crate::explain::{AmbiguousCandidate, Explanation};
use crate::hash::package_hash;
use crate::ids::{ComputerId, GroupId, Sha256};
use crate::inventory::{FileEntry, FolderState, FolderStatus, Inventory};
use crate::package::{
    compare_package, group_packages, package_version, AmbiguityReason, ClientInfo, Package,
    PackageOrder, PackageVersion,
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
    UnvalidatedTocMapping,
    PartialCandidate,
    /// Concurrent keeps or choices name different trees for a folder (contracts.md §4.4).
    ChoicesDiffer,
    /// Concurrent rollbacks name different trees for a folder.
    RollbackTargetsDiffer,
}

pub struct TargetInput<'a> {
    pub group: &'a GroupRecord,
    pub variant: &'a ConfirmedVariant,
    pub local: &'a Inventory,
    pub local_client: &'a ClientInfo,
    pub local_id: &'a ComputerId,
    pub members: &'a [MemberView],
    pub trees: &'a BTreeMap<Sha256, TreeAvail>,
    /// The last release of each package this computer held identically with a peer (R-05
    /// content lineage). Empty when unknown: lineage then never orders anything.
    pub shared: &'a BTreeMap<FolderKey, Sha256>,
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
    /// The group removed this package: it is not installed or kept as a target here.
    Remove {
        why: Explanation,
    },
    /// Removal history for this package is still arriving or a local hold applies: nothing is
    /// installed or removed yet.
    Held {
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

/// A folder that is not readable, safe and stable is never a candidate. A folder without a
/// recognised `.toc` is not a problem: it syncs by content (R-05, D-29).
fn folder_problem(f: &FolderState) -> Option<Ineligible> {
    (f.status != FolderStatus::Ok).then(|| {
        (
            DecisionReason::NoCompatibleToc,
            Explanation::NoCompatibleToc {
                folder: f.name.clone(),
            },
        )
    })
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
    compute_inner(input, None).0
}

/// The shared-release memory after this observation (R-05 content lineage): every package whose
/// local copy is complete and identical to the copy of an admitted peer is recorded at that
/// release; other packages keep what was known. Entries of packages no longer seen are dropped.
pub fn shared_releases(input: &TargetInput) -> BTreeMap<FolderKey, Sha256> {
    let peers = valid_peers(input);
    let peer_invs: Vec<Inventory> = peers.iter().map(|p| state_inventory(p.2)).collect();
    let mut observations: Vec<&Inventory> = vec![input.local];
    observations.extend(peer_invs.iter());
    let whole = |pv: &PackageVersion| {
        pv.folders
            .iter()
            .all(|f| f.status == FolderStatus::Ok && f.tree.is_some())
    };
    let mut out = BTreeMap::new();
    for pkg in group_packages(&observations) {
        if let Some(h) = input.shared.get(&pkg.key) {
            out.insert(pkg.key.clone(), *h);
        }
        let Some(local) = package_version(&pkg, input.local, input.local_client) else {
            continue;
        };
        if !whole(&local) {
            continue;
        }
        let held_by_peer = peer_invs.iter().zip(&peers).any(|(inv, (_, _, st))| {
            package_version(&pkg, inv, &st.client)
                .is_some_and(|pv| whole(&pv) && pv.hash == local.hash)
        });
        if held_by_peer {
            out.insert(pkg.key.clone(), local.hash);
        }
    }
    out
}

/// Whether a whole source may be a candidate for a package (contracts.md §4.1 level 2, §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screened {
    Eligible,
    /// Rejected or excluded: never a candidate.
    Out,
    /// Undecided (`Unknown`): not a candidate, and nothing falls back to a lesser one.
    Wait,
}

type Screen<'a> = &'a dyn Fn(&Package, &Source) -> Screened;

fn valid_peers<'a>(input: &'a TargetInput) -> Vec<(&'a ComputerId, u64, &'a CodeState)> {
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
    peers
}

fn compute_inner(input: &TargetInput, screen: Option<Screen>) -> (GroupTarget, Vec<Package>) {
    let peers = valid_peers(input);
    let peer_invs: Vec<Inventory> = peers.iter().map(|p| state_inventory(p.2)).collect();
    let mut observations: Vec<&Inventory> = vec![input.local];
    observations.extend(peer_invs.iter());
    let mut sources: BTreeMap<&ComputerId, Source> = BTreeMap::new();
    if screen.is_some() {
        sources.insert(input.local_id, Source::from_inventory(input.local));
        for (id, _, state) in &peers {
            sources.insert(id, Source::from_state(state, true));
        }
    }

    let mut out = GroupTarget {
        group_id: input.group.group_id.clone(),
        packages: BTreeMap::new(),
        effective: BTreeMap::new(),
        local_hash: BTreeMap::new(),
    };
    let mut seen_packages = Vec::new();
    for pkg in group_packages(&observations) {
        seen_packages.push(pkg.clone());
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
        if let Some(screen) = screen {
            let (mut waiting, mut out_any) = (false, false);
            for c in cands.values_mut() {
                c.holders.retain(|(id, _)| match sources.get(id) {
                    Some(s) => match screen(&pkg, s) {
                        Screened::Eligible => true,
                        Screened::Out => {
                            out_any = true;
                            false
                        }
                        Screened::Wait => {
                            waiting = true;
                            false
                        }
                    },
                    None => true,
                });
            }
            cands.retain(|_, c| !c.holders.is_empty());
            if waiting {
                out.packages.insert(
                    pkg.key.clone(),
                    PackageTarget::Held {
                        why: Explanation::CheckingAgainstBlocked,
                    },
                );
                continue;
            }
            if cands.is_empty() && out_any {
                out.packages.insert(
                    pkg.key.clone(),
                    PackageTarget::Held {
                        why: Explanation::OnlyBlockedVersions,
                    },
                );
                continue;
            }
        }
        let list: Vec<&Cand> = cands.values().collect();
        let holds_local = |c: &Cand| c.holders.iter().any(|(id, _)| id == input.local_id);

        let dominator = list.iter().find(|c| {
            list.iter().all(|x| {
                x.pv.hash == c.pv.hash
                    || match compare_package(&c.pv, &x.pv) {
                        PackageOrder::Same | PackageOrder::Greater { .. } => true,
                        // Unrecognised content has no version: the copy that changed from the
                        // last shared release, which the other still holds, is the newer one.
                        PackageOrder::Ambiguous {
                            reason: AmbiguityReason::UnrecognisedDiffers,
                            ..
                        } => input.shared.get(&pkg.key) == Some(&x.pv.hash),
                        _ => false,
                    }
            })
        });
        let target = match dominator {
            Some(c) if holds_local(c) => {
                out.effective.insert(pkg.key.clone(), effective(&c.pv));
                PackageTarget::InSync
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
    (out, seen_packages)
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

/// Fills each folder's file list from the published tree records, or says why it cannot.
#[allow(clippy::result_large_err)]
fn attach_trees(version: &mut PackageVersion, input: &TargetInput) -> Result<(), PackageTarget> {
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
        return Err(PackageTarget::Unavailable {
            why: Explanation::TreeCorrupt { tree },
        });
    }
    if !missing.is_empty() {
        return Err(PackageTarget::Unavailable {
            why: Explanation::NotAvailableYet { missing },
        });
    }
    Ok(())
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
    if let Err(t) = attach_trees(&mut version, input) {
        return t;
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

// ---------------------------------------------------------------- release identity (contracts §4.2)

/// What one side holds for one folder key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    Tree(Sha256),
    /// Known not to have the folder.
    Absent,
    /// Not `Ok`, not readable, not admitted, or outside a recorded scope.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeMatch {
    Same,
    Different,
    Unknown,
}

/// One computer's complete copy: an admitted state or the local inventory. It has no scope of its
/// own; it answers any key (R4D-28).
#[derive(Debug, Clone)]
pub struct Source {
    admitted: bool,
    /// Whether a key this copy does not list is known absent. An incomplete copy still answers
    /// for the folders it lists with a known tree, but absence is never evidence.
    complete: bool,
    folders: BTreeMap<FolderKey, Content>,
}

impl Source {
    pub fn from_inventory(inv: &Inventory) -> Self {
        Self::build(&inv.folders, inv.listing_stable, inv.listing_stable)
    }

    pub fn from_state(state: &CodeState, admitted: bool) -> Self {
        Self::build(&state.folders, admitted, state.inventory_complete)
    }

    fn build(folders: &[FolderState], admitted: bool, complete: bool) -> Self {
        let folders = folders
            .iter()
            .map(|f| {
                let c = match (f.status, f.tree) {
                    (FolderStatus::Ok, Some(t)) => Content::Tree(t),
                    _ => Content::Unknown,
                };
                (f.key.clone(), c)
            })
            .collect();
        Self {
            admitted,
            complete,
            folders,
        }
    }

    pub fn content(&self, k: &FolderKey) -> Content {
        if !self.admitted {
            return Content::Unknown;
        }
        match self.folders.get(k) {
            Some(c) => *c,
            None if self.complete => Content::Absent,
            None => Content::Unknown,
        }
    }

    /// A source standing in for a recorded release (a keep whose copy nobody holds now). Like
    /// any source it is compared over a recorded entry's scope only.
    pub fn from_release(r: &Release) -> Self {
        let folders = r.scope.iter().map(|k| (k.clone(), r.content(k))).collect();
        Self {
            admitted: true,
            complete: true,
            folders,
        }
    }
}

/// A recorded release entry: its scope and the trees it lists. A key of the scope it does not
/// list was absent from that copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    scope: Vec<FolderKey>,
    trees: BTreeMap<FolderKey, Sha256>,
}

impl Release {
    pub fn new(scope: &[FolderKey], trees: &[(FolderKey, Sha256)]) -> Self {
        let mut scope = scope.to_vec();
        scope.sort();
        scope.dedup();
        Self {
            scope,
            trees: trees.iter().cloned().collect(),
        }
    }

    pub fn scope(&self) -> &[FolderKey] {
        &self.scope
    }

    pub fn content(&self, k: &FolderKey) -> Content {
        if !self.scope.contains(k) {
            return Content::Unknown;
        }
        self.trees
            .get(k)
            .map_or(Content::Absent, |t| Content::Tree(*t))
    }
}

fn compare(a: Content, b: Content) -> ScopeMatch {
    match (a, b) {
        (Content::Unknown, _) | (_, Content::Unknown) => ScopeMatch::Unknown,
        (x, y) if x == y => ScopeMatch::Same,
        _ => ScopeMatch::Different,
    }
}

fn fold_matches(parts: impl Iterator<Item = ScopeMatch>) -> ScopeMatch {
    let mut out = ScopeMatch::Same;
    for p in parts {
        match p {
            ScopeMatch::Different => return ScopeMatch::Different,
            ScopeMatch::Unknown => out = ScopeMatch::Unknown,
            ScopeMatch::Same => {}
        }
    }
    out
}

/// Source against a recorded entry: K is the entry's scope only.
pub fn same_release(m: &Source, e: &Release) -> ScopeMatch {
    fold_matches(e.scope.iter().map(|k| compare(m.content(k), e.content(k))))
}

/// Two recorded entries: K is the union of their scopes.
pub fn same_release_entries(x: &Release, y: &Release) -> ScopeMatch {
    let keys: BTreeSet<&FolderKey> = x.scope.iter().chain(y.scope.iter()).collect();
    fold_matches(
        keys.into_iter()
            .map(|k| compare(x.content(k), y.content(k))),
    )
}

// ---------------------------------------------------------------- precedence (contracts §4.1)

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    UnresolvedJournal = 1,
    Rejection,
    LocalHold,
    MergedRemoval,
    StaleCopy,
    Constraint,
    PendingRemoval,
    Ordinary,
}

/// Which levels apply to one folder on this computer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderFacts {
    pub journal_unresolved: bool,
    pub source_rejected: bool,
    pub local_hold: bool,
    pub merged_removal: bool,
    pub stale_copy: bool,
    /// `Some(tree)` when a live constraint names the folder; the tree may be absent.
    pub constraint: Option<Option<Sha256>>,
    pub pending_removal: bool,
    /// The projection of the whole ordinary result.
    pub ordinary: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDecision {
    pub level: Level,
    /// The tree to install for the folder, when the level supplies one.
    pub tree: Option<Sha256>,
}

/// The first applicable level decides (§4.1). `None` means no level applies.
pub fn compose_folder(f: &FolderFacts) -> Option<FolderDecision> {
    let at = |level, tree| Some(FolderDecision { level, tree });
    if f.journal_unresolved {
        at(Level::UnresolvedJournal, None)
    } else if f.source_rejected {
        at(Level::Rejection, None)
    } else if f.local_hold {
        at(Level::LocalHold, None)
    } else if f.merged_removal {
        at(Level::MergedRemoval, None)
    } else if f.stale_copy {
        at(Level::StaleCopy, None)
    } else if let Some(t) = f.constraint {
        at(Level::Constraint, t)
    } else if f.pending_removal {
        at(Level::PendingRemoval, None)
    } else if f.ordinary.is_some() {
        at(Level::Ordinary, f.ordinary)
    } else {
        None
    }
}

/// Every complete copy of each package that is not rejected or excluded, for the decision items
/// (contracts.md §5.1). A copy is one release; its holders are the enrollments that have it.
pub struct PackageCopies {
    pub package: Package,
    pub copies: Vec<crate::choice::Copy>,
}

pub fn package_copies(
    input: &TargetInput,
    local_enrollment: &crate::ids::EnrollmentId,
    ledger: &crate::ledger::Ledger,
    constraints: &dyn crate::hooks::ConstraintHooks,
) -> Vec<PackageCopies> {
    let peers = valid_peers(input);
    let peer_invs: Vec<Inventory> = peers.iter().map(|p| state_inventory(p.2)).collect();
    let mut observations: Vec<&Inventory> = vec![input.local];
    observations.extend(peer_invs.iter());
    let enrollment_of = |id: &ComputerId| {
        input
            .members
            .iter()
            .find(|m| &m.computer == id)
            .map(|m| m.enrollment.clone())
    };
    let mut out = Vec::new();
    for pkg in group_packages(&observations) {
        type Holder = (crate::kinds::EnrollmentRef, Source);
        let mut by_hash: BTreeMap<Sha256, (PackageVersion, Vec<Holder>)> = BTreeMap::new();
        if let Some(pv) = package_version(&pkg, input.local, input.local_client) {
            by_hash
                .entry(pv.hash)
                .or_insert_with(|| (pv, Vec::new()))
                .1
                .push((
                    crate::kinds::EnrollmentRef {
                        computer_id: input.local_id.clone(),
                        enrollment_id: local_enrollment.clone(),
                    },
                    Source::from_inventory(input.local),
                ));
        }
        for (i, (id, _, state)) in peers.iter().enumerate() {
            let Some(enrollment) = enrollment_of(id) else {
                continue;
            };
            if let Some(pv) = package_version(&pkg, &peer_invs[i], &state.client) {
                by_hash
                    .entry(pv.hash)
                    .or_insert_with(|| (pv, Vec::new()))
                    .1
                    .push((
                        crate::kinds::EnrollmentRef {
                            computer_id: (*id).clone(),
                            enrollment_id: enrollment,
                        },
                        Source::from_state(state, true),
                    ));
            }
        }
        let mut copies = Vec::new();
        for (_, (pv, holders)) in by_hash {
            let holders: Vec<Holder> = holders
                .into_iter()
                .filter(|(_, s)| {
                    constraints.rejected(ledger, &pkg, s) == ScopeMatch::Different
                        && constraints.excluded(ledger, &pkg, s) == ScopeMatch::Different
                })
                .collect();
            let Some((_, source)) = holders.first().cloned() else {
                continue;
            };
            copies.push(crate::choice::Copy {
                version: pv,
                source,
                holders: holders.into_iter().map(|(r, _)| r).collect(),
            });
        }
        out.push(PackageCopies {
            package: pkg,
            copies,
        });
    }
    out
}

/// The group target after the composition levels this foundation can evaluate. Levels 2, 5, 6
/// and the code-choice parts of 8 come from hooks; with neutral hooks the result is exactly
/// `compute_target` (slice behaviour).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composed {
    pub target: GroupTarget,
    /// Folders a merged removal moves into recovery.
    pub removed: BTreeSet<FolderKey>,
    /// Folders whose installs wait for pending removal information.
    pub deferred: BTreeSet<FolderKey>,
    pub held: BTreeSet<FolderKey>,
}

pub fn compose(
    input: &TargetInput,
    ledger: &crate::ledger::Ledger,
    removal: &dyn crate::hooks::RemovalHooks,
    held: &BTreeSet<FolderKey>,
    constraints: &dyn crate::hooks::ConstraintHooks,
) -> Composed {
    let screen = |p: &Package, s: &Source| {
        let r = constraints.rejected(ledger, p, s);
        let x = constraints.excluded(ledger, p, s);
        if r == ScopeMatch::Same || x == ScopeMatch::Same {
            Screened::Out
        } else if r == ScopeMatch::Unknown || x == ScopeMatch::Unknown {
            Screened::Wait
        } else {
            Screened::Eligible
        }
    };
    let (mut target, packages) = compute_inner(input, Some(&screen));
    for pkg in &packages {
        apply_constraints(pkg, input, ledger, constraints, &screen, &mut target);
    }
    let mut removed = BTreeSet::new();
    let mut deferred = BTreeSet::new();
    for pkg in &packages {
        let (mut any_removed, mut any_held, mut any_pending) = (false, false, false);
        for k in &pkg.members {
            let view = removal.merged_removals(ledger, k);
            let facts = FolderFacts {
                local_hold: held.contains(k),
                merged_removal: !view.live.is_empty(),
                pending_removal: view.pending,
                ..FolderFacts::default()
            };
            match compose_folder(&facts).map(|d| d.level) {
                Some(Level::LocalHold) => any_held = true,
                Some(Level::MergedRemoval) => {
                    removed.insert(k.clone());
                    any_removed = true;
                }
                Some(Level::PendingRemoval) => {
                    deferred.insert(k.clone());
                    any_pending = true;
                }
                _ => {}
            }
        }
        let installing = matches!(
            target.packages.get(&pkg.key),
            Some(PackageTarget::Chosen { .. })
        );
        let overlay = if any_held {
            Some(PackageTarget::Held {
                why: Explanation::WaitingForGroupHistory,
            })
        } else if any_removed {
            Some(PackageTarget::Remove {
                why: Explanation::RemovedByGroup,
            })
        } else if any_pending && installing {
            Some(PackageTarget::Held {
                why: Explanation::WaitingForGroupHistory,
            })
        } else {
            None
        };
        if let Some(o) = overlay {
            target.packages.insert(pkg.key.clone(), o);
            target.effective.remove(&pkg.key);
        }
    }
    Composed {
        target,
        removed,
        deferred,
        held: held.clone(),
    }
}

/// Level 6 (contracts.md §4.4) for one package: each folder with a live constraint takes exactly
/// the constrained tree; several different trees are a decision; no constraint changes nothing.
fn apply_constraints(
    pkg: &Package,
    input: &TargetInput,
    ledger: &crate::ledger::Ledger,
    constraints: &dyn crate::hooks::ConstraintHooks,
    screen: Screen,
    target: &mut GroupTarget,
) {
    use crate::decision::ConstraintKind;
    let mut wanted: BTreeMap<FolderKey, Sha256> = BTreeMap::new();
    let mut differ: Option<DecisionReason> = None;
    for f in &pkg.members {
        let live = constraints.live_constraints(ledger, f);
        let trees: BTreeSet<Sha256> = live.iter().filter_map(|c| c.tree).collect();
        match trees.len() {
            0 => {}
            1 => {
                wanted.insert(f.clone(), *trees.iter().next().expect("one tree"));
            }
            _ => {
                let all_rollbacks = live.iter().all(|c| c.kind == ConstraintKind::Rejection);
                differ.get_or_insert(if all_rollbacks {
                    DecisionReason::RollbackTargetsDiffer
                } else {
                    DecisionReason::ChoicesDiffer
                });
            }
        }
    }
    if let Some(reason) = differ {
        target.packages.insert(
            pkg.key.clone(),
            PackageTarget::NeedsDecision {
                reason,
                candidates: Vec::new(),
                why: Explanation::ConcurrentDecisions,
            },
        );
        target.effective.remove(&pkg.key);
        return;
    }
    if wanted.is_empty() {
        return;
    }
    let local_pv = package_version(pkg, input.local, input.local_client);
    let (mut version, _) = match target.packages.get(&pkg.key) {
        Some(PackageTarget::Chosen {
            version, sources, ..
        }) => (version.clone(), sources.clone()),
        Some(PackageTarget::Held { .. }) | None => return,
        Some(_) => match &local_pv {
            Some(pv) => (pv.clone(), Vec::new()),
            None => return,
        },
    };
    let peers = valid_peers(input);
    let local_source = Source::from_inventory(input.local);
    let peer_sources: Vec<Source> = peers
        .iter()
        .map(|(_, _, s)| Source::from_state(s, true))
        .collect();
    for (f, tree) in &wanted {
        if version
            .folders
            .iter()
            .any(|x| &x.key == f && x.tree == Some(*tree) && x.status == FolderStatus::Ok)
        {
            continue;
        }
        let find = |folders: &[FolderState]| {
            folders
                .iter()
                .find(|x| &x.key == f && x.tree == Some(*tree) && x.status == FolderStatus::Ok)
                .cloned()
        };
        let mut found: Option<FolderState> = None;
        if screen(pkg, &local_source) == Screened::Eligible {
            found = find(&input.local.folders);
        }
        if found.is_none() {
            for (i, (_, _, state)) in peers.iter().enumerate() {
                if screen(pkg, &peer_sources[i]) != Screened::Eligible {
                    continue;
                }
                if let Some(x) = find(&state.folders) {
                    found = Some(x);
                    break;
                }
            }
        }
        let Some(fs) = found else {
            target.packages.insert(
                pkg.key.clone(),
                PackageTarget::Unavailable {
                    why: Explanation::NotAvailableYet {
                        missing: vec![*tree],
                    },
                },
            );
            target.effective.remove(&pkg.key);
            return;
        };
        version.folders.retain(|x| &x.key != f);
        version.folders.push(fs);
    }
    version.folders.sort_by(|a, b| a.key.cmp(&b.key));
    version.hash = package_hash(
        &version
            .folders
            .iter()
            .filter_map(|f| f.tree.map(|t| (f.key.clone(), t)))
            .collect::<Vec<_>>(),
    );
    if target.local_hash.get(&pkg.key) == Some(&version.hash) {
        target
            .effective
            .insert(pkg.key.clone(), effective(&version));
        target
            .packages
            .insert(pkg.key.clone(), PackageTarget::InSync);
        return;
    }
    let sources: Vec<ComputerId> = peers
        .iter()
        .enumerate()
        .filter(|(i, _)| screen(pkg, &peer_sources[*i]) == Screened::Eligible)
        .filter(|(_, (_, _, state))| {
            version.folders.iter().all(|vf| {
                state
                    .folders
                    .iter()
                    .any(|x| x.key == vf.key && x.tree == vf.tree && x.status == FolderStatus::Ok)
            })
        })
        .map(|(_, (id, _, _))| (*id).clone())
        .collect();
    if let Err(t) = attach_trees(&mut version, input) {
        target.packages.insert(pkg.key.clone(), t);
        target.effective.remove(&pkg.key);
        return;
    }
    target
        .effective
        .insert(pkg.key.clone(), effective(&version));
    target.packages.insert(
        pkg.key.clone(),
        PackageTarget::Chosen {
            version,
            sources,
            why: Explanation::ChosenByGroupRecord,
        },
    );
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::decision::{AckItem, Candidate, CleanupClass, Constraint, ConstraintKind, CycleCx};
    use crate::hooks::Neutral;
    use crate::ledger::Ledger;
    use crate::plan::PackageApply;
    use crate::testkit::{addon, inventory, Scenario};

    fn k(s: &str) -> FolderKey {
        FolderKey::lossy(s)
    }

    fn t(n: u8) -> Sha256 {
        crate::hash::sha256(&[n])
    }

    fn src(entries: &[(&str, Option<&str>)]) -> Source {
        let scans = entries
            .iter()
            .map(|(n, tag)| addon(n, Some("1.0"), tag.unwrap_or("x")))
            .collect();
        Source::from_inventory(&inventory(scans))
    }

    #[test]
    fn same_release_exact_source_unrelated_addon_is_ignored() {
        // T-R4D28-exact-source-unrelated-addon: the source's extra folder is outside K.
        let s = src(&[("A", Some("a")), ("Unrelated", Some("u"))]);
        let tree = match s.content(&k("a")) {
            Content::Tree(t) => t,
            c => panic!("{c:?}"),
        };
        let e = Release::new(&[k("a")], &[(k("a"), tree)]);
        assert_eq!(same_release(&s, &e), ScopeMatch::Same);
    }

    #[test]
    fn same_release_table() {
        // T-R4D23-same-release-table
        let s = src(&[("A", Some("a"))]);
        let tree = match s.content(&k("a")) {
            Content::Tree(t) => t,
            c => panic!("{c:?}"),
        };
        let same = Release::new(&[k("a")], &[(k("a"), tree)]);
        let other = Release::new(&[k("a")], &[(k("a"), t(9))]);
        let absent_in_entry = Release::new(&[k("a")], &[]);
        let missing_in_source = Release::new(&[k("a"), k("b")], &[(k("a"), tree), (k("b"), t(2))]);
        let both_absent = Release::new(&[k("a"), k("b")], &[(k("a"), tree)]);
        assert_eq!(same_release(&s, &same), ScopeMatch::Same);
        assert_eq!(same_release(&s, &other), ScopeMatch::Different);
        assert_eq!(same_release(&s, &absent_in_entry), ScopeMatch::Different);
        assert_eq!(same_release(&s, &missing_in_source), ScopeMatch::Different);
        assert_eq!(same_release(&s, &both_absent), ScopeMatch::Same);
    }

    #[test]
    fn same_release_unknown_waits() {
        // T-R4D23-unknown-waits: Unknown never equals Same and never becomes Different.
        let s = Source::from_state(
            &crate::testkit::member_state(
                &crate::testkit::computer(2),
                1,
                &inventory(vec![addon("A", Some("1.0"), "a")]),
            ),
            false,
        );
        let e = Release::new(&[k("a")], &[(k("a"), t(1))]);
        assert_eq!(same_release(&s, &e), ScopeMatch::Unknown);
        let x = Release::new(&[k("a")], &[(k("a"), t(1))]);
        let y = Release::new(&[k("a"), k("b")], &[(k("a"), t(1)), (k("b"), t(2))]);
        assert_eq!(same_release_entries(&x, &y), ScopeMatch::Unknown);
        let z = Release::new(&[k("a"), k("b")], &[(k("a"), t(3)), (k("b"), t(2))]);
        assert_eq!(same_release_entries(&x, &z), ScopeMatch::Different);
    }

    #[test]
    fn incomplete_state_keeps_known_folders_but_absence_is_unknown() {
        let mut st = crate::testkit::member_state(
            &crate::testkit::computer(2),
            1,
            &inventory(vec![addon("A", Some("1.0"), "a")]),
        );
        st.inventory_complete = false;
        let s = Source::from_state(&st, true);
        let tree = match s.content(&k("a")) {
            Content::Tree(t) => t,
            c => panic!("{c:?}"),
        };
        assert_eq!(s.content(&k("zzz")), Content::Unknown);
        let known = Release::new(&[k("a")], &[(k("a"), tree)]);
        assert_eq!(same_release(&s, &known), ScopeMatch::Same);
        let absent = Release::new(&[k("zzz")], &[]);
        assert_eq!(same_release(&s, &absent), ScopeMatch::Unknown);
    }

    #[test]
    fn same_release_regrouped_exclusion() {
        // T-R4D28-regrouped-exclusion: a folder excluded in one entry but listed in another is
        // not evidence of sameness.
        let x = Release::new(&[k("a")], &[(k("a"), t(1))]);
        let y = Release::new(&[k("a"), k("b")], &[(k("a"), t(1))]);
        assert_ne!(same_release_entries(&x, &y), ScopeMatch::Same);
        assert_ne!(same_release_entries(&y, &x), ScopeMatch::Same);
    }

    #[test]
    fn no_hybrid_of_two_releases_in_one_package() {
        // T-R4D24-no-hybrid: a newer core on one peer and newer data on another is never mixed.
        let sc = Scenario::new(vec![
            addon("Suite", Some("1.0"), "c1"),
            addon("Suite_Data", Some("1.0"), "d1"),
        ])
        .peer(
            2,
            vec![
                addon("Suite", Some("2.0"), "c2"),
                addon("Suite_Data", Some("1.0"), "d1"),
            ],
        )
        .peer(
            3,
            vec![
                addon("Suite", Some("1.0"), "c1"),
                addon("Suite_Data", Some("2.0"), "d2"),
            ],
        );
        let t = sc.target();
        let folders: Vec<&(FolderKey, Sha256)> = t
            .effective
            .values()
            .flat_map(|e| e.folders.iter())
            .collect();
        let has = |key: &str, tree: Sha256| folders.iter().any(|(f, h)| *f == k(key) && *h == tree);
        let hybrid = has("suite", sc.peer_tree(2, "suite"))
            && has("suite_data", sc.peer_tree(3, "suite_data"));
        assert!(!hybrid, "{:?}", t.effective);
    }

    #[test]
    fn compose_precedence_adjacent_levels() {
        // T-C14-compose-precedence: each level beats the next one down.
        let order = [
            FolderFacts {
                journal_unresolved: true,
                source_rejected: true,
                ..Default::default()
            },
            FolderFacts {
                source_rejected: true,
                local_hold: true,
                ..Default::default()
            },
            FolderFacts {
                local_hold: true,
                merged_removal: true,
                ..Default::default()
            },
            FolderFacts {
                merged_removal: true,
                stale_copy: true,
                ..Default::default()
            },
            FolderFacts {
                stale_copy: true,
                constraint: Some(None),
                ..Default::default()
            },
            FolderFacts {
                constraint: Some(None),
                pending_removal: true,
                ..Default::default()
            },
            FolderFacts {
                pending_removal: true,
                ordinary: Some(t(1)),
                ..Default::default()
            },
        ];
        let want = [
            Level::UnresolvedJournal,
            Level::Rejection,
            Level::LocalHold,
            Level::MergedRemoval,
            Level::StaleCopy,
            Level::Constraint,
            Level::PendingRemoval,
        ];
        for (f, w) in order.iter().zip(want) {
            assert_eq!(compose_folder(f).map(|d| d.level), Some(w));
        }
        assert_eq!(compose_folder(&FolderFacts::default()), None);
        let ord = FolderFacts {
            ordinary: Some(t(1)),
            ..Default::default()
        };
        assert_eq!(compose_folder(&ord).map(|d| d.level), Some(Level::Ordinary));
    }

    #[test]
    fn neutral_hooks_leave_the_slice_target_unchanged() {
        let sc = Scenario::new(vec![addon("A", Some("1.0"), "a")])
            .peer(2, vec![addon("A", Some("2.0"), "b")]);
        let (group, variant, local_id) = (
            crate::testkit::group_record(),
            crate::testkit::confirmed_forever(),
            crate::testkit::computer(1),
        );
        let input = TargetInput {
            group: &group,
            variant: &variant,
            local: &sc.local,
            local_client: &sc.local_client,
            local_id: &local_id,
            members: &sc.members,
            trees: &sc.trees,
            shared: &sc.shared,
        };
        let c = compose(&input, &Ledger::new(), &Neutral, &BTreeSet::new(), &Neutral);
        assert_eq!(c.target, compute_target(&input));
        assert!(c.removed.is_empty() && c.deferred.is_empty());
    }

    struct Fake {
        constraints: Vec<Constraint>,
        rejected_trees: Vec<Sha256>,
        removed: Vec<FolderKey>,
    }

    impl Fake {
        fn new() -> Self {
            Fake {
                constraints: Vec::new(),
                rejected_trees: Vec::new(),
                removed: Vec::new(),
            }
        }
    }

    impl crate::hooks::RemovalHooks for Fake {
        fn merged_removals(&self, l: &Ledger, f: &FolderKey) -> crate::decision::RemovalView {
            let mut v = Neutral.merged_removals(l, f);
            if self.removed.contains(f) {
                v.live.push(t(9));
            }
            v
        }
        fn stale_copy(&self, l: &Ledger, f: &FolderKey, c: &Candidate) -> bool {
            Neutral.stale_copy(l, f, c)
        }
        fn classify_due(&self, cx: &CycleCx) -> Vec<(AckItem, CleanupClass)> {
            Neutral.classify_due(cx)
        }
        fn plan_removals(&self, cx: &CycleCx) -> Vec<PackageApply> {
            Neutral.plan_removals(cx)
        }
    }

    impl crate::hooks::ConstraintHooks for Fake {
        fn live_constraints(&self, _: &Ledger, _: &FolderKey) -> Vec<Constraint> {
            self.constraints.clone()
        }
        fn excluded(&self, _: &Ledger, _: &Package, _: &Source) -> ScopeMatch {
            ScopeMatch::Different
        }
        fn rejected(&self, _: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
            let hit = p.members.iter().any(
                |m| matches!(s.content(m), Content::Tree(t) if self.rejected_trees.contains(&t)),
            );
            if hit {
                ScopeMatch::Same
            } else {
                ScopeMatch::Different
            }
        }
    }

    fn composed(sc: &Scenario, fake: &Fake, held: &BTreeSet<FolderKey>) -> Composed {
        let (group, variant, local_id) = (
            crate::testkit::group_record(),
            crate::testkit::confirmed_forever(),
            crate::testkit::computer(1),
        );
        let input = TargetInput {
            group: &group,
            variant: &variant,
            local: &sc.local,
            local_client: &sc.local_client,
            local_id: &local_id,
            members: &sc.members,
            trees: &sc.trees,
            shared: &sc.shared,
        };
        compose(&input, &Ledger::new(), fake, held, fake)
    }

    fn keep(tree: Sha256, n: u8) -> Constraint {
        Constraint {
            record: t(n),
            kind: ConstraintKind::Keep,
            tree: Some(tree),
        }
    }

    fn two_versions() -> Scenario {
        Scenario::new(vec![addon("A", Some("1.0"), "a")])
            .peer(2, vec![addon("A", Some("2.0"), "b")])
    }

    #[test]
    fn a_live_keep_pins_the_folder_against_a_newer_candidate() {
        let sc = two_versions();
        let mut fake = Fake::new();
        let local_tree = sc.local.folders[0].tree.unwrap();
        fake.constraints.push(keep(local_tree, 1));
        let c = composed(&sc, &fake, &BTreeSet::new());
        assert_eq!(c.target.packages[&k("a")], PackageTarget::InSync);
        // Without the keep the newer release is chosen.
        let c = composed(&sc, &Fake::new(), &BTreeSet::new());
        assert!(matches!(
            c.target.packages[&k("a")],
            PackageTarget::Chosen { .. }
        ));
    }

    #[test]
    fn a_keep_names_the_tree_that_installs_even_when_older() {
        let sc = Scenario::new(vec![addon("A", Some("1.0"), "a")])
            .peer(2, vec![addon("A", Some("2.0"), "b")])
            .peer(3, vec![addon("A", Some("3.0"), "c")]);
        let mut fake = Fake::new();
        fake.constraints.push(keep(sc.peer_tree(2, "a"), 1));
        let c = composed(&sc, &fake, &BTreeSet::new());
        match &c.target.packages[&k("a")] {
            PackageTarget::Chosen {
                version, sources, ..
            } => {
                assert_eq!(version.folders[0].toc.version.as_deref(), Some("2.0"));
                assert_eq!(sources, &vec![crate::testkit::computer(2)]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn constraints_naming_different_trees_need_a_decision() {
        let sc = two_versions();
        for (kinds, expected) in [
            (
                [ConstraintKind::Keep, ConstraintKind::Keep],
                DecisionReason::ChoicesDiffer,
            ),
            (
                [ConstraintKind::Rejection, ConstraintKind::Rejection],
                DecisionReason::RollbackTargetsDiffer,
            ),
        ] {
            let mut fake = Fake::new();
            fake.constraints = vec![
                Constraint {
                    record: t(1),
                    kind: kinds[0],
                    tree: sc.local.folders[0].tree,
                },
                Constraint {
                    record: t(2),
                    kind: kinds[1],
                    tree: Some(sc.peer_tree(2, "a")),
                },
            ];
            let c = composed(&sc, &fake, &BTreeSet::new());
            match &c.target.packages[&k("a")] {
                PackageTarget::NeedsDecision { reason, .. } => assert_eq!(*reason, expected),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_rejected_release_is_never_a_candidate() {
        let sc = two_versions();
        let mut fake = Fake::new();
        fake.rejected_trees.push(sc.peer_tree(2, "a"));
        let c = composed(&sc, &fake, &BTreeSet::new());
        assert_eq!(c.target.packages[&k("a")], PackageTarget::InSync);
        // The only holder is rejected and the local copy differs: nothing is installable.
        let sc = Scenario::new(vec![]).peer(2, vec![addon("A", Some("2.0"), "b")]);
        let mut fake = Fake::new();
        fake.rejected_trees.push(sc.peer_tree(2, "a"));
        let c = composed(&sc, &fake, &BTreeSet::new());
        assert!(!matches!(
            c.target.packages.get(&k("a")),
            Some(PackageTarget::Chosen { .. })
        ));
    }

    #[test]
    fn removal_and_local_hold_overlay_the_target() {
        let sc = two_versions();
        let mut fake = Fake::new();
        fake.removed.push(k("a"));
        let c = composed(&sc, &fake, &BTreeSet::new());
        assert!(matches!(
            c.target.packages[&k("a")],
            PackageTarget::Remove { .. }
        ));
        assert!(c.removed.contains(&k("a")));
        assert!(!c.target.effective.contains_key(&k("a")));
        let held: BTreeSet<FolderKey> = [k("a")].into();
        let c = composed(&sc, &Fake::new(), &held);
        assert!(matches!(
            c.target.packages[&k("a")],
            PackageTarget::Held { .. }
        ));
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

    fn release(sc: &Scenario, key: &str) -> Sha256 {
        let pkgs = group_packages(&[&sc.local]);
        let p = pkgs.iter().find(|p| p.key.as_str() == key).unwrap();
        package_version(p, &sc.local, &sc.local_client)
            .unwrap()
            .hash
    }

    fn unrec(tag: &str) -> Vec<crate::inventory::FolderScan> {
        vec![crate::testkit::addon_unrecognised("Suite", tag)]
    }

    #[test]
    fn unrecognised_add_on_only_on_a_peer_is_installed() {
        let t = Scenario::new(vec![]).peer(2, unrec("b")).target();
        assert!(matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
    }

    #[test]
    fn identical_unrecognised_copies_are_in_sync() {
        let t = Scenario::new(unrec("a")).peer(2, unrec("a")).target();
        assert_eq!(pkg(&t, "suite"), &PackageTarget::InSync);
    }

    #[test]
    fn differing_unrecognised_copies_without_lineage_need_a_decision() {
        let t = Scenario::new(unrec("a")).peer(2, unrec("b")).target();
        assert_eq!(reason(pkg(&t, "suite")), DecisionReason::Ambiguous);
    }

    #[test]
    fn unrecognised_copy_changed_here_since_the_shared_release_wins() {
        let old = release(&Scenario::new(unrec("b")), "suite");
        let t = Scenario::new(unrec("a"))
            .peer(2, unrec("b"))
            .shared_at("suite", old)
            .target();
        assert_eq!(pkg(&t, "suite"), &PackageTarget::InSync);
    }

    #[test]
    fn unrecognised_copy_changed_on_the_peer_since_the_shared_release_is_installed() {
        let old = release(&Scenario::new(unrec("a")), "suite");
        let t = Scenario::new(unrec("a"))
            .peer(2, unrec("b"))
            .shared_at("suite", old)
            .target();
        match pkg(&t, "suite") {
            PackageTarget::Chosen { sources, .. } => assert_eq!(sources, &vec![computer(2)]),
            other => panic!("expected the peer's copy, got {other:?}"),
        }
    }

    #[test]
    fn unrecognised_copies_both_changed_since_the_shared_release_need_a_decision() {
        let old = release(&Scenario::new(unrec("old")), "suite");
        let t = Scenario::new(unrec("a"))
            .peer(2, unrec("b"))
            .shared_at("suite", old)
            .target();
        assert_eq!(reason(pkg(&t, "suite")), DecisionReason::Ambiguous);
    }

    #[test]
    fn foreign_suffix_only_copy_is_unrecognised_not_blocked() {
        let t = Scenario::new(vec![])
            .peer(2, vec![addon_unvalidated("Suite", "2.0", "b")])
            .target();
        assert!(matches!(pkg(&t, "suite"), PackageTarget::Chosen { .. }));
        let t = Scenario::new(suite("1.0", "a"))
            .peer(2, vec![addon_unvalidated("Suite", "2.0", "b")])
            .target();
        assert_eq!(reason(pkg(&t, "suite")), DecisionReason::Ambiguous);
    }

    #[test]
    fn shared_release_is_recorded_only_when_a_peer_holds_the_same_copy() {
        let sc = Scenario::new(unrec("a")).peer(2, unrec("a"));
        let shared = sc.shared_after();
        assert_eq!(
            shared.get(&FolderKey::lossy("suite")),
            Some(&release(&sc, "suite"))
        );
        let differing = Scenario::new(unrec("a")).peer(2, unrec("b"));
        assert!(differing.shared_after().is_empty());
        let kept = differing.shared_at("suite", release(&sc, "suite"));
        assert_eq!(kept.shared_after().len(), 1);
        let gone = Scenario::new(vec![]).shared_at("suite", release(&sc, "suite"));
        assert!(gone.shared_after().is_empty());
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
