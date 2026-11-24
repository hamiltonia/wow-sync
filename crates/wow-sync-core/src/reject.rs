//! Rejection and lift behaviour (rollback.md): active rejections, the level-2 `rejected`
//! check, the per-folder recovered-target constraints and their chain, the rollback plan,
//! the lift and the Q-84 gate. Pure: ledger and sources in, decisions out.

use crate::authority::{covered, AuthorityRecord};
use crate::choice::{project, Copy};
use crate::decision::{Constraint, ConstraintKind};
use crate::explain::Explanation;
use crate::hash::{package_hash, sha256};
use crate::hooks::{ConstraintHooks, Neutral};
use crate::ids::{ComputerId, GroupId, OpId, Sha256};
use crate::kinds::{
    Ack, AckItem, AckResult, AckSettings, Considered, EnrollmentRef, Issuer, Known, Lift,
    ManifestFolder, Readd, ReaddEvidence, RecoveredEntry, RejectedEntry, Rejection,
    RejectionSettings, Removal, RemovalEvidence, RemovalSettings, TreeEntry,
};
use crate::ledger::{resolve_known, Derived, Ledger};
use crate::package::{compare_package, Package, PackageOrder};
use crate::paths::FolderKey;
use crate::plan::{ExpectedLive, FolderAction, FolderApply, PackageApply, RemoveApply};
use crate::records::{encode, FolderTreeRecord, OpKind, PendingRecord, Record};
use crate::target::{same_release, same_release_entries, Release, ScopeMatch, Source};
use std::collections::BTreeSet;

/// The rejection half of `ConstraintHooks`. It carries the hashes of the records that are
/// `valid` (contracts.md §1.11), because the frozen hook signatures take only the ledger.
/// The default knows no valid record and so has no effect.
#[derive(Debug, Clone, Default)]
pub struct RejectBehaviour {
    valid: BTreeSet<Sha256>,
}

fn trees(e: &[TreeEntry]) -> Vec<(FolderKey, Sha256)> {
    e.iter().map(|t| (t.key.clone(), t.tree)).collect()
}

fn rejected_release(r: &Rejection) -> Release {
    Release::new(&r.folders, &trees(&r.rejected.folders))
}

fn recovered_release(r: &Rejection) -> Release {
    Release::new(&r.folders, &trees(&r.recovered.folders))
}

/// One step of the recovered chain: what X's constraint stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainTarget {
    /// The rejection whose recovered release this is.
    pub rejection: Sha256,
    pub folders: Vec<TreeEntry>,
    /// Indices into the sources handed to the chain that are proven usable copies; `None`
    /// when the chain was decided from the ledger alone.
    pub copies: Option<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chain {
    /// The constraint stands for these recovered releases.
    Targets(Vec<ChainTarget>),
    /// Every release on the chain is proven rejected: no constraint.
    AllRejected,
    /// Branch 1 (R4D-29): the constrained release has no copy anywhere.
    Unavailable,
    /// Branch 3: a copy is undecided and none is proven usable.
    Waiting,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyState {
    Rejected,
    NotRejected,
    Unknown,
}

impl RejectBehaviour {
    pub fn new(derived: &Derived) -> Self {
        Self {
            valid: derived
                .status
                .iter()
                .filter(|(_, s)| s.is_valid())
                .map(|(h, _)| *h)
                .collect(),
        }
    }

    /// Valid rejections that no valid lift names (rollback.md §2.3).
    pub fn active<'a>(&self, l: &'a Ledger) -> Vec<(Sha256, &'a Rejection)> {
        let mut lifted = BTreeSet::new();
        let mut out = Vec::new();
        for e in l.entries() {
            if !self.valid.contains(&e.hash) {
                continue;
            }
            match l.record(&e.hash) {
                Some(AuthorityRecord::Lift(x)) => lifted.extend(x.lifts.iter().copied()),
                Some(AuthorityRecord::Rejection(r)) => out.push((e.hash, r)),
                _ => {}
            }
        }
        out.retain(|(h, _)| !lifted.contains(h));
        out
    }

    /// Whether a valid code choice, rejection or re-add other than `x` names `f` and covers
    /// `x` through its `known` (R4D-21).
    fn covered_for(&self, l: &Ledger, x: &Sha256, f: &FolderKey) -> bool {
        l.entries().any(|e| {
            if e.hash == *x || !self.valid.contains(&e.hash) {
                return false;
            }
            let (names, known, issuer) = match l.record(&e.hash) {
                Some(AuthorityRecord::CodeChoice(c)) => {
                    (c.folders.contains(f), &c.known, &c.issuer)
                }
                Some(AuthorityRecord::Rejection(r)) => (r.folders.contains(f), &r.known, &r.issuer),
                Some(AuthorityRecord::Readd(r)) => {
                    (r.folders.iter().any(|m| &m.key == f), &r.known, &r.issuer)
                }
                _ => return false,
            };
            if !names {
                return false;
            }
            match resolve_known(l, known) {
                Ok(k) => covered(&k, &BTreeSet::new(), issuer, x),
                Err(_) => false,
            }
        })
    }

    /// The recovered chain of rejection `x` (rollback.md §3.1). `sources` are the admitted
    /// sources that decide when two entries cannot be compared; without them an
    /// undecided entry comparison leaves X's constraint standing.
    pub fn recovered_chain(&self, l: &Ledger, x: &Sha256, sources: Option<&[Source]>) -> Chain {
        let active = self.active(l);
        self.walk(&active, x, sources, &mut BTreeSet::new())
    }

    fn walk(
        &self,
        active: &[(Sha256, &Rejection)],
        x: &Sha256,
        sources: Option<&[Source]>,
        visited: &mut BTreeSet<Sha256>,
    ) -> Chain {
        let Some((_, xr)) = active.iter().find(|(h, _)| h == x) else {
            return Chain::AllRejected;
        };
        if !visited.insert(*x) {
            return Chain::AllRejected;
        }
        let rel = recovered_release(xr);
        let mut same = Vec::new();
        let mut unknown = false;
        for (h, y) in active {
            match same_release_entries(&rel, &rejected_release(y)) {
                ScopeMatch::Same => same.push(*h),
                ScopeMatch::Unknown => unknown = true,
                ScopeMatch::Different => {}
            }
        }
        let stands = |copies: Option<Vec<usize>>| {
            Chain::Targets(vec![ChainTarget {
                rejection: *x,
                folders: xr.recovered.folders.clone(),
                copies,
            }])
        };
        if !same.is_empty() {
            return self.advance(active, &same, sources, visited);
        }
        if !unknown {
            return stands(None);
        }
        let Some(src) = sources else {
            return stands(None);
        };
        let copies: Vec<usize> = (0..src.len())
            .filter(|i| same_release(&src[*i], &rel) == ScopeMatch::Same)
            .collect();
        if copies.is_empty() {
            return Chain::Unavailable;
        }
        let mut hit = BTreeSet::new();
        let mut states = Vec::new();
        for i in &copies {
            let mut rejected = false;
            let mut unk = false;
            for (h, y) in active {
                match same_release(&src[*i], &rejected_release(y)) {
                    ScopeMatch::Same => {
                        rejected = true;
                        hit.insert(*h);
                    }
                    ScopeMatch::Unknown => unk = true,
                    ScopeMatch::Different => {}
                }
            }
            states.push(if rejected {
                CopyState::Rejected
            } else if unk {
                CopyState::Unknown
            } else {
                CopyState::NotRejected
            });
        }
        if states.iter().all(|s| *s == CopyState::Rejected) {
            let hit: Vec<Sha256> = hit.into_iter().collect();
            return self.advance(active, &hit, sources, visited);
        }
        let usable: Vec<usize> = copies
            .iter()
            .zip(&states)
            .filter(|(_, s)| **s == CopyState::NotRejected)
            .map(|(i, _)| *i)
            .collect();
        if usable.is_empty() {
            return Chain::Waiting;
        }
        stands(Some(usable))
    }

    fn advance(
        &self,
        active: &[(Sha256, &Rejection)],
        next: &[Sha256],
        sources: Option<&[Source]>,
        visited: &mut BTreeSet<Sha256>,
    ) -> Chain {
        let parts: Vec<Chain> = next
            .iter()
            .map(|y| self.walk(active, y, sources, visited))
            .collect();
        if parts.contains(&Chain::Waiting) {
            return Chain::Waiting;
        }
        if parts.contains(&Chain::Unavailable) {
            return Chain::Unavailable;
        }
        let mut targets: Vec<ChainTarget> = parts
            .into_iter()
            .flat_map(|p| match p {
                Chain::Targets(t) => t,
                _ => Vec::new(),
            })
            .collect();
        targets.sort_by_key(|t| t.rejection);
        targets.dedup_by_key(|t| t.rejection);
        if targets.is_empty() {
            Chain::AllRejected
        } else {
            Chain::Targets(targets)
        }
    }

    /// Q-78: a candidate with a blocked version's label over the same folders but different
    /// content. Returns the rejection it looks like.
    pub fn lookalike(
        &self,
        l: &Ledger,
        label: &str,
        folders: &[FolderKey],
        source: &Source,
    ) -> Option<Sha256> {
        let mut want = folders.to_vec();
        want.sort();
        self.active(l).into_iter().find_map(|(h, r)| {
            (r.rejected.version_label == label
                && r.folders == want
                && same_release(source, &rejected_release(r)) == ScopeMatch::Different)
                .then_some(h)
        })
    }

    /// Q-81: a source newer than the recovered target and not among what rejection `x` was
    /// issued over raises a notice; it never installs. `newer` is the domain ordering result.
    pub fn newer_after_rollback(
        &self,
        l: &Ledger,
        x: &Sha256,
        source: &Source,
        newer: bool,
    ) -> bool {
        let Some((_, r)) = self.active(l).into_iter().find(|(h, _)| h == x) else {
            return false;
        };
        newer
            && !r.considered.iter().any(|c| {
                same_release(source, &Release::new(&r.folders, &trees(&c.folders)))
                    == ScopeMatch::Same
            })
    }

    /// Q-81 over a package's non-rejected copies: for each live rollback whose recovered release
    /// is held in `copies`, the other copies that are demonstrably newer and not among what the
    /// rejection considered. They raise `NewerAfterRollback` and never install.
    pub fn newer_after_rollbacks(
        &self,
        l: &Ledger,
        p: &Package,
        copies: &[Copy],
    ) -> Vec<NewerAfterRollback> {
        let mut live = BTreeSet::new();
        for f in &p.members {
            for c in self.live_constraints(l, f) {
                if c.kind == ConstraintKind::Rejection {
                    live.insert(c.record);
                }
            }
        }
        let active = self.active(l);
        let mut out = Vec::new();
        for h in live {
            let Some((_, r)) = active.iter().find(|(x, _)| *x == h) else {
                continue;
            };
            let rel = recovered_release(r);
            let Some(recovered) = copies
                .iter()
                .position(|c| same_release(&c.source, &rel) == ScopeMatch::Same)
            else {
                continue;
            };
            let base = project(&copies[recovered].version, &r.folders);
            let raised: Vec<usize> = copies
                .iter()
                .enumerate()
                .filter(|(i, c)| {
                    let newer = matches!(
                        compare_package(&project(&c.version, &r.folders), &base),
                        PackageOrder::Greater { .. }
                    );
                    *i != recovered && self.newer_after_rollback(l, &h, &c.source, newer)
                })
                .map(|(i, _)| i)
                .collect();
            if !raised.is_empty() {
                let mut scope = r.folders.clone();
                scope.sort();
                out.push(NewerAfterRollback {
                    rejection: h,
                    scope,
                    recovered,
                    copies: raised,
                });
            }
        }
        out
    }
}

/// A live rollback and the copies (indexes into the copies passed in) that raise the notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewerAfterRollback {
    pub rejection: Sha256,
    pub scope: Vec<FolderKey>,
    pub recovered: usize,
    pub copies: Vec<usize>,
}

impl NewerAfterRollback {
    /// The raised copies that only `own` holds: they stay installed and unpublished until the
    /// owner answers (Q-88, D-26).
    pub fn local_only(&self, copies: &[Copy], own: &ComputerId) -> Vec<usize> {
        self.copies
            .iter()
            .copied()
            .filter(|i| {
                copies.get(*i).is_some_and(|c| {
                    !c.holders.is_empty() && c.holders.iter().all(|e| &e.computer_id == own)
                })
            })
            .collect()
    }

    /// What "Stay on the recovered version" dismisses: these exact raised copies.
    pub fn dismissal_key(&self, copies: &[Copy]) -> Sha256 {
        let mut hashes: Vec<Sha256> = self
            .copies
            .iter()
            .filter_map(|i| copies.get(*i).map(|c| c.version.hash))
            .collect();
        hashes.sort();
        let mut bytes = b"newer-after-rollback".to_vec();
        bytes.extend(self.rejection.0);
        for h in hashes {
            bytes.extend(h.0);
        }
        sha256(&bytes)
    }
}

impl ConstraintHooks for RejectBehaviour {
    fn live_constraints(&self, l: &Ledger, f: &FolderKey) -> Vec<Constraint> {
        let active = self.active(l);
        let mut out: Vec<Constraint> = Vec::new();
        for (h, x) in &active {
            if !x.recovered.folders.iter().any(|t| &t.key == f) || self.covered_for(l, h, f) {
                continue;
            }
            if let Chain::Targets(ts) = self.walk(&active, h, None, &mut BTreeSet::new()) {
                for t in ts {
                    if self.covered_for(l, &t.rejection, f) {
                        continue;
                    }
                    if let Some(e) = t.folders.iter().find(|e| &e.key == f) {
                        let c = Constraint {
                            record: t.rejection,
                            kind: ConstraintKind::Rejection,
                            tree: Some(e.tree),
                        };
                        if !out.contains(&c) {
                            out.push(c);
                        }
                    }
                }
            }
        }
        out
    }

    fn excluded(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        Neutral.excluded(l, p, s)
    }

    /// Level 2: the source against `rejected` of each active rejection over that rejection's
    /// folders only. Rejections that share no folder with the package are never compared.
    fn rejected(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        let mut unknown = false;
        for (_, r) in self.active(l) {
            if !r.folders.iter().any(|f| p.members.contains(f)) {
                continue;
            }
            match same_release(s, &rejected_release(r)) {
                ScopeMatch::Same => return ScopeMatch::Same,
                ScopeMatch::Unknown => unknown = true,
                ScopeMatch::Different => {}
            }
        }
        if unknown {
            ScopeMatch::Unknown
        } else {
            ScopeMatch::Different
        }
    }
}

// ---------------------------------------------------------------- lift

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiftError {
    NotActive(Sha256),
    Empty,
}

/// "Allow again…": a lift naming exactly the given active rejections (Q-79).
pub fn build_lift(
    b: &RejectBehaviour,
    l: &Ledger,
    group_id: &GroupId,
    issuer: Issuer,
    rejections: &[Sha256],
    known: Known,
    now_unix_ms: u64,
) -> Result<Lift, LiftError> {
    if rejections.is_empty() {
        return Err(LiftError::Empty);
    }
    let active = b.active(l);
    let mut lifts: Vec<Sha256> = rejections.to_vec();
    lifts.sort();
    lifts.dedup();
    let mut folders = BTreeSet::new();
    for h in &lifts {
        let Some((_, r)) = active.iter().find(|(a, _)| a == h) else {
            return Err(LiftError::NotActive(*h));
        };
        folders.extend(r.folders.iter().cloned());
    }
    Ok(Lift {
        group_id: group_id.clone(),
        issuer,
        folders: folders.into_iter().collect(),
        lifts,
        known,
        written_unix_ms: now_unix_ms,
    })
}

// ---------------------------------------------------------------- Q-84 gate

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFeatures {
    pub member: EnrollmentRef,
    pub retired: bool,
    pub has_rejection: bool,
    pub has_removal: bool,
}

/// Q-84 default: members that cannot apply blocked versions (or, for a folder-changing
/// rollback, rollback removals) stop a group rollback.
pub fn rollback_blockers(members: &[MemberFeatures], folder_changing: bool) -> Vec<EnrollmentRef> {
    members
        .iter()
        .filter(|m| !m.retired && (!m.has_rejection || (folder_changing && !m.has_removal)))
        .map(|m| m.member.clone())
        .collect()
}

// ---------------------------------------------------------------- the rollback plan

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFolder {
    pub key: FolderKey,
    pub name: String,
    pub tree: Sha256,
    pub expected_live: ExpectedLive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetFolder {
    pub key: FolderKey,
    pub name: String,
    pub record: FolderTreeRecord,
}

pub struct RollbackInput {
    pub op: OpId,
    pub package: FolderKey,
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub title: String,
    pub rejected_label: String,
    /// R1: what is installed now, with the live expectation.
    pub r1: Vec<LiveFolder>,
    /// R0: the recovered release, with trees to install.
    pub r0: Vec<TargetFolder>,
    pub considered: Vec<Considered>,
    /// The actual settings restore result (R4D-27), not the recovery point's expectation.
    pub settings: RejectionSettings,
    pub known: Known,
    /// The members that must acknowledge a rollback removal.
    pub audience: Vec<EnrollmentRef>,
    /// Earlier removals of F0 \ F1 that a re-add has to cover.
    pub readd_covers: Vec<Sha256>,
    pub now_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackError {
    SameRelease,
    EmptyRelease,
}

fn manifest(f: &[(&FolderKey, &str, Sha256)]) -> Vec<ManifestFolder> {
    let mut v: Vec<ManifestFolder> = f
        .iter()
        .map(|(k, n, t)| ManifestFolder {
            key: (*k).clone(),
            name: (*n).to_string(),
            tree: *t,
        })
        .collect();
    v.sort_by(|a, b| a.key.cmp(&b.key));
    v
}

fn manifest_hash(m: &[ManifestFolder]) -> Sha256 {
    package_hash(
        &m.iter()
            .map(|f| (f.key.clone(), f.tree))
            .collect::<Vec<_>>(),
    )
}

fn pending<R: Record>(r: &R) -> PendingRecord {
    let bytes = encode(r);
    PendingRecord {
        hash: sha256(&bytes),
        bytes: String::from_utf8(bytes).expect("records are UTF-8"),
    }
}

/// Rollback R1 → R0 (rollback.md §1): `replace` for F0 ∩ F1, `install` for F0 \ F1, `remove`
/// for F1 \ F0. The pending records are the rejection, an optional rollback removal with this
/// computer's ack, and an optional re-add for F0 \ F1, in that order.
pub fn plan_rollback(i: &RollbackInput) -> Result<PackageApply, RollbackError> {
    if i.r1.is_empty() || i.r0.is_empty() {
        return Err(RollbackError::EmptyRelease);
    }
    let mut r1_pairs: Vec<(FolderKey, Sha256)> =
        i.r1.iter().map(|f| (f.key.clone(), f.tree)).collect();
    r1_pairs.sort();
    let mut r0_pairs: Vec<(FolderKey, Sha256)> =
        i.r0.iter()
            .map(|f| (f.key.clone(), f.record.tree))
            .collect();
    r0_pairs.sort();
    if r1_pairs == r0_pairs {
        return Err(RollbackError::SameRelease);
    }
    let entry = |p: &[(FolderKey, Sha256)]| -> Vec<TreeEntry> {
        p.iter()
            .map(|(k, t)| TreeEntry {
                key: k.clone(),
                tree: *t,
            })
            .collect()
    };
    let mut union: Vec<FolderKey> = r1_pairs
        .iter()
        .chain(&r0_pairs)
        .map(|(k, _)| k.clone())
        .collect();
    union.sort();
    union.dedup();
    let mut considered = i.considered.clone();
    for e in [&r1_pairs, &r0_pairs] {
        let h = package_hash(e);
        if !considered.iter().any(|x| x.package_hash == h) {
            considered.push(Considered {
                package_hash: h,
                folders: entry(e),
                holders: vec![i.issuer.enrollment_id.clone()],
            });
        }
    }
    considered.sort();
    let rejection = Rejection {
        group_id: i.group_id.clone(),
        issuer: i.issuer.clone(),
        folders: union,
        rejected: RejectedEntry {
            package_hash: package_hash(&r1_pairs),
            version_label: i.rejected_label.clone(),
            folders: entry(&r1_pairs),
        },
        recovered: RecoveredEntry {
            package_hash: package_hash(&r0_pairs),
            folders: entry(&r0_pairs),
        },
        considered,
        settings: i.settings,
        known: i.known.clone(),
        considered_parts: vec![],
        written_unix_ms: i.now_unix_ms,
    };
    let rej = pending(&rejection);
    let mut records = vec![rej.clone()];

    let mut folders = Vec::new();
    let mut removes = Vec::new();
    let mut dropped: Vec<(&FolderKey, &str, Sha256)> = Vec::new();
    for t in &i.r0 {
        let live = i.r1.iter().find(|f| f.key == t.key);
        folders.push(FolderApply {
            name: t.name.clone(),
            key: t.key.clone(),
            action: if live.is_some() {
                FolderAction::Replace
            } else {
                FolderAction::Install
            },
            tree: t.record.clone(),
            expected_live: live.map_or(ExpectedLive::Absent, |f| f.expected_live.clone()),
        });
    }
    for f in &i.r1 {
        if !i.r0.iter().any(|t| t.key == f.key) {
            removes.push(RemoveApply {
                name: f.name.clone(),
                key: f.key.clone(),
                expected_live: f.expected_live.clone(),
            });
            dropped.push((&f.key, f.name.as_str(), f.tree));
        }
    }
    let mut removals = Vec::new();
    if !dropped.is_empty() {
        let listing = manifest(&dropped);
        let mut audience = i.audience.clone();
        audience.sort();
        audience.dedup();
        let removal = Removal {
            group_id: i.group_id.clone(),
            issuer: i.issuer.clone(),
            evidence: RemovalEvidence::Rollback,
            rejection: Some(rej.hash),
            title: i.title.clone(),
            removed_hash: manifest_hash(&listing),
            folders: listing.clone(),
            audience,
            settings: RemovalSettings::None,
            written_unix_ms: i.now_unix_ms,
        };
        let rm = pending(&removal);
        let ack = Ack {
            group_id: i.group_id.clone(),
            issuer: i.issuer.clone(),
            items: vec![AckItem {
                removal: rm.hash,
                folders: listing.iter().map(|m| m.key.clone()).collect(),
            }],
            result: AckResult::Removed,
            removed: listing,
            settings: match i.settings {
                RejectionSettings::Paired => AckSettings::Paired,
                RejectionSettings::Partial => AckSettings::Partial,
                RejectionSettings::CodeOnly => AckSettings::CodeOnly,
                RejectionSettings::NotApplicable => AckSettings::NotApplicable,
            },
            written_unix_ms: i.now_unix_ms,
        };
        removals.push(rm.hash);
        records.push(rm);
        records.push(pending(&ack));
    }
    let added: Vec<(&FolderKey, &str, Sha256)> =
        i.r0.iter()
            .filter(|t| !i.r1.iter().any(|f| f.key == t.key))
            .map(|t| (&t.key, t.name.as_str(), t.record.tree))
            .collect();
    if !added.is_empty() && !i.readd_covers.is_empty() {
        let listing = manifest(&added);
        let mut extra = i.readd_covers.clone();
        extra.sort();
        extra.dedup();
        let readd = Readd {
            group_id: i.group_id.clone(),
            issuer: i.issuer.clone(),
            evidence: ReaddEvidence::Rollback,
            rejection: Some(rej.hash),
            title: i.title.clone(),
            present_hash: manifest_hash(&listing),
            folders: listing,
            known: Known {
                seen: vec![],
                extra,
                parts: vec![],
            },
            written_unix_ms: i.now_unix_ms,
        };
        records.push(pending(&readd));
    }
    let mut plan = PackageApply::code_apply(
        i.op.clone(),
        i.package.clone(),
        package_hash(&r0_pairs),
        folders,
        Explanation::AppliedHere,
    );
    plan.kind = OpKind::Rollback;
    plan.removes = removes;
    plan.removals = removals;
    plan.pending_records = records;
    Ok(plan)
}
