//! Code-choice behaviour: keep, resume and exclusions (ops.md §1.5, contracts.md §4.4, §5.2, §5.3).
//!
//! Everything here is a pure function of the ledger and its derived validity. Whole-source
//! checks use `same_release` (contracts.md §4.2): a source against each recorded entry over that
//! entry's own scope, never per folder and never through a local package hash.

use crate::authority::{covered, AuthorityRecord};
use crate::decision::{Constraint, ConstraintKind};
use crate::hash::package_hash;
use crate::hooks::{ConstraintHooks, Neutral};
use crate::ids::{ComputerId, EnrollmentId, GroupId, Sha256};
use crate::inventory::{FolderState, FolderStatus, TocKind, TocSummary};
use crate::kinds::{
    validate_choice, ChoiceAction, ChoiceReason, CodeChoice, Considered, EnrollmentRef, Exclude,
    Issuer, KeepEntry, Known, ManifestFolder, PartItem, PartOf, TreeEntry, MAX_RECORD_BYTES,
};
use crate::ledger::{read_parts, resolve_known, Derived, Ledger, Status};
use crate::package::{compare_package, ClientInfo, Package, PackageOrder, PackageVersion};
use crate::paths::FolderKey;
use crate::records::encode;
use crate::target::{same_release, Release, ScopeMatch, Source};
use std::collections::{BTreeMap, BTreeSet};

/// The hashes of every entry that is currently `Valid`.
pub fn valid_hashes(d: &Derived) -> BTreeSet<Sha256> {
    d.status
        .iter()
        .filter(|(_, s)| matches!(s, Status::Valid))
        .map(|(h, _)| *h)
        .collect()
}

/// The keep and resume half of `ConstraintHooks`. The hooks receive only the ledger, so the
/// behaviour carries the valid set derived for that ledger. Without it (`Default`) every hook is
/// neutral, which keeps the slice behaviour.
#[derive(Debug, Clone, Default)]
pub struct ChoiceBehaviour {
    valid: Option<BTreeSet<Sha256>>,
}

impl ChoiceBehaviour {
    pub fn for_derived(d: &Derived) -> Self {
        Self {
            valid: Some(valid_hashes(d)),
        }
    }
}

impl ConstraintHooks for ChoiceBehaviour {
    fn live_constraints(&self, l: &Ledger, f: &FolderKey) -> Vec<Constraint> {
        match &self.valid {
            Some(v) => ChoiceView::new(l, v).constraints(f),
            None => Neutral.live_constraints(l, f),
        }
    }
    fn excluded(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        match &self.valid {
            Some(v) => ChoiceView::new(l, v).excluded(p, s),
            None => Neutral.excluded(l, p, s),
        }
    }
    fn rejected(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        Neutral.rejected(l, p, s)
    }
    fn choices_conflict(&self, l: &Ledger, p: &Package) -> bool {
        self.valid
            .as_ref()
            .is_some_and(|v| ChoiceView::new(l, v).resume_excludes_a_live_keep(p))
    }
}

struct Node<'a> {
    hash: Sha256,
    issuer: &'a Issuer,
    folders: Vec<FolderKey>,
    choice: Option<&'a CodeChoice>,
}

/// A read-only view of the valid records that can end a code choice for the folders they name:
/// code choices, rejections and re-adds (contracts.md §4.4).
pub struct ChoiceView<'a> {
    ledger: &'a Ledger,
    nodes: Vec<Node<'a>>,
    known: BTreeMap<Sha256, Known>,
}

impl<'a> ChoiceView<'a> {
    pub fn new(ledger: &'a Ledger, valid: &BTreeSet<Sha256>) -> Self {
        let mut nodes = Vec::new();
        let mut known = BTreeMap::new();
        for e in ledger.entries().filter(|e| valid.contains(&e.hash)) {
            let Some(rec) = ledger.record(&e.hash) else {
                continue;
            };
            let (issuer, folders, choice, k) = match rec {
                AuthorityRecord::CodeChoice(c) => (&c.issuer, c.folders.clone(), Some(c), &c.known),
                AuthorityRecord::Rejection(r) => (&r.issuer, r.folders.clone(), None, &r.known),
                AuthorityRecord::Readd(r) => (
                    &r.issuer,
                    r.folders.iter().map(|f| f.key.clone()).collect(),
                    None,
                    &r.known,
                ),
                _ => continue,
            };
            let full = resolve_known(ledger, k).unwrap_or_else(|_| k.clone());
            known.insert(e.hash, full);
            nodes.push(Node {
                hash: e.hash,
                issuer,
                folders,
                choice,
            });
        }
        Self {
            ledger,
            nodes,
            known,
        }
    }

    fn covers(&self, x: &Node, y: &Node) -> bool {
        let none = BTreeSet::new();
        x.hash != y.hash
            && self
                .known
                .get(&x.hash)
                .is_some_and(|k| covered(k, &none, y.issuer, &y.hash))
    }

    /// The valid code choices live for `f`: none is covered by another valid choice, rejection
    /// or re-add that also names `f`.
    pub fn live_choices(&self, f: &FolderKey) -> Vec<(Sha256, &'a CodeChoice)> {
        let naming: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| n.folders.contains(f))
            .collect();
        let mut out: Vec<(Sha256, &CodeChoice)> = naming
            .iter()
            .filter_map(|y| y.choice.map(|c| (*y, c)))
            .filter(|(y, _)| !naming.iter().any(|x| self.covers(x, y)))
            .map(|(y, c)| (y.hash, c))
            .collect();
        out.sort_by_key(|(h, _)| *h);
        out
    }

    pub fn considered_of(&self, c: &CodeChoice) -> Vec<Considered> {
        let mut v = c.considered.clone();
        if let Ok(items) = read_parts(self.ledger, &c.considered_parts, PartOf::Considered) {
            v.extend(items.into_iter().filter_map(|i| match i {
                PartItem::Considered(x) => Some(x),
                _ => None,
            }));
        }
        v.sort();
        v.dedup();
        v
    }

    pub fn exclude_of(&self, c: &CodeChoice) -> Vec<Exclude> {
        let mut v = c.exclude.clone();
        if let Ok(items) = read_parts(self.ledger, &c.exclude_parts, PartOf::Exclude) {
            v.extend(items.into_iter().filter_map(|i| match i {
                PartItem::Exclude(x) => Some(x),
                _ => None,
            }));
        }
        v.sort();
        v.dedup();
        v
    }

    /// Every valid record naming a folder of `scope`: what a new choice over `scope` covers.
    pub fn known_for(&self, scope: &[FolderKey]) -> Known {
        let mut extra: Vec<Sha256> = self
            .nodes
            .iter()
            .filter(|n| n.folders.iter().any(|k| scope.contains(k)))
            .map(|n| n.hash)
            .collect();
        extra.sort();
        extra.dedup();
        Known {
            seen: vec![],
            extra,
            parts: vec![],
        }
    }

    /// Exclude entries carried by the choices live over `scope` whose own scope touches it.
    pub fn inherited(&self, scope: &[FolderKey]) -> Vec<Exclude> {
        let mut live: BTreeMap<Sha256, &CodeChoice> = BTreeMap::new();
        for k in scope {
            live.extend(self.live_choices(k));
        }
        let mut out: Vec<Exclude> = live
            .values()
            .flat_map(|c| self.exclude_of(c))
            .filter(|e| e.scope.iter().any(|k| scope.contains(k)))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The exclusion verdict for a whole source of package `p` (contracts.md §5.3): `Same` as an
    /// applicable entry over that entry's scope means never selected; `Unknown` waits.
    pub fn excluded(&self, p: &Package, s: &Source) -> ScopeMatch {
        let mut verdict = ScopeMatch::Different;
        for node in &self.nodes {
            let Some(c) = node.choice else { continue };
            let live_here = p.members.iter().any(|f| {
                c.folders.contains(f) && self.live_choices(f).iter().any(|(h, _)| *h == node.hash)
            });
            if !live_here {
                continue;
            }
            for e in self.exclude_of(c) {
                if !e
                    .scope
                    .iter()
                    .any(|k| p.members.contains(k) && c.folders.contains(k))
                {
                    continue;
                }
                let trees: Vec<(FolderKey, Sha256)> =
                    e.folders.iter().map(|t| (t.key.clone(), t.tree)).collect();
                let entry = Release::new(&e.scope, &trees);
                match same_release(s, &entry) {
                    ScopeMatch::Same if self.kept_by_rival(p, node.hash, &entry) => {}
                    ScopeMatch::Same => return ScopeMatch::Same,
                    ScopeMatch::Unknown => verdict = ScopeMatch::Unknown,
                    ScopeMatch::Different => {}
                }
            }
        }
        verdict
    }

    /// A live resume whose exclusion names a release that a live keep over p pins: competing
    /// intentional choices, which are never merged, so the owner must decide.
    pub fn resume_excludes_a_live_keep(&self, p: &Package) -> bool {
        self.nodes.iter().any(|n| {
            let Some(c) = n.choice.filter(|c| c.action != ChoiceAction::Keep) else {
                return false;
            };
            let live_here = p.members.iter().any(|f| {
                c.folders.contains(f) && self.live_choices(f).iter().any(|(h, _)| *h == n.hash)
            });
            live_here
                && self.exclude_of(c).iter().any(|e| {
                    let trees: Vec<(FolderKey, Sha256)> =
                        e.folders.iter().map(|t| (t.key.clone(), t.tree)).collect();
                    e.scope
                        .iter()
                        .any(|k| p.members.contains(k) && c.folders.contains(k))
                        && self.kept_by_rival(p, n.hash, &Release::new(&e.scope, &trees))
                })
        })
    }

    /// Whether another live keep over `p` pins the release `entry` excludes. Two competing live
    /// keeps each exclude the other's copy; neither exclusion may hide the copy the other keeps,
    /// or the conflict would have nothing left to choose between.
    fn kept_by_rival(&self, p: &Package, own: Sha256, entry: &Release) -> bool {
        self.nodes.iter().any(|n| {
            let Some(c) = n.choice else { return false };
            let Some(keep) = c.keep.as_ref().filter(|_| c.action == ChoiceAction::Keep) else {
                return false;
            };
            if n.hash == own {
                return false;
            }
            let live_here = p.members.iter().any(|f| {
                c.folders.contains(f) && self.live_choices(f).iter().any(|(h, _)| *h == n.hash)
            });
            let trees: Vec<(FolderKey, Sha256)> = keep
                .folders
                .iter()
                .map(|m| (m.key.clone(), m.tree))
                .collect();
            live_here
                && same_release(
                    &Source::from_release(&Release::new(&c.folders, &trees)),
                    entry,
                ) == ScopeMatch::Same
        })
    }
    /// The live keeps' constraints for `f`. A resume imposes none.
    pub fn constraints(&self, f: &FolderKey) -> Vec<Constraint> {
        self.live_choices(f)
            .into_iter()
            .filter(|(_, c)| c.action == ChoiceAction::Keep)
            .map(|(h, c)| Constraint {
                record: h,
                kind: ConstraintKind::Keep,
                tree: c
                    .keep
                    .as_ref()
                    .and_then(|k| k.folders.iter().find(|m| &m.key == f))
                    .map(|m| m.tree),
            })
            .collect()
    }

    /// Q-49: copies a live keep did not consider. A copy that is `Different` from every
    /// `considered` entry and demonstrably newer than, or unordered against, the kept copy
    /// raises `NewVersionAfterDecision`; the keep stays. `Unknown` against any entry waits.
    pub fn new_versions(&self, p: &Package, copies: &[Copy]) -> Vec<NewVersion> {
        let mut out = Vec::new();
        let mut keeps: BTreeMap<Sha256, &CodeChoice> = BTreeMap::new();
        for f in &p.members {
            keeps.extend(
                self.live_choices(f)
                    .into_iter()
                    .filter(|(_, c)| c.action == ChoiceAction::Keep),
            );
        }
        for (hash, c) in keeps {
            let Some(keep) = &c.keep else { continue };
            let keep_trees: Vec<(FolderKey, Sha256)> = keep
                .folders
                .iter()
                .map(|m| (m.key.clone(), m.tree))
                .collect();
            let keep_rel = Release::new(&c.folders, &keep_trees);
            let considered: Vec<Release> = self
                .considered_of(c)
                .iter()
                .map(|e| {
                    let t: Vec<(FolderKey, Sha256)> =
                        e.folders.iter().map(|x| (x.key.clone(), x.tree)).collect();
                    Release::new(&c.folders, &t)
                })
                .collect();
            let (kept_index, absent_kept) = match copies
                .iter()
                .position(|x| same_release(&x.source, &keep_rel) == ScopeMatch::Same)
            {
                Some(i) => (i, None),
                None => (copies.len(), Some(absent_copy(p, keep, &keep_rel))),
            };
            let kept_pv = match &absent_kept {
                Some(c) => c.version.clone(),
                None => project(&copies[kept_index].version, &c.folders),
            };
            let mut raised = Vec::new();
            for (i, copy) in copies.iter().enumerate() {
                let all_different = considered
                    .iter()
                    .all(|e| same_release(&copy.source, e) == ScopeMatch::Different);
                if !all_different {
                    continue;
                }
                let order = compare_package(&project(&copy.version, &c.folders), &kept_pv);
                if matches!(
                    order,
                    PackageOrder::Greater { .. } | PackageOrder::Ambiguous { .. }
                ) {
                    raised.push(i);
                }
            }
            if !raised.is_empty() {
                out.push(NewVersion {
                    keep: hash,
                    scope: c.folders.clone(),
                    kept: kept_index,
                    absent_kept,
                    copies: raised,
                });
            }
        }
        out
    }
}

/// One complete copy of a package: its version, its source form and who holds it.
#[derive(Debug, Clone)]
pub struct Copy {
    pub version: PackageVersion,
    pub source: Source,
    pub holders: Vec<EnrollmentRef>,
}

/// A live keep and the indexes (into the `copies` passed in) that raise `NewVersionAfterDecision`.
#[derive(Debug, Clone)]
pub struct NewVersion {
    pub keep: Sha256,
    pub scope: Vec<FolderKey>,
    /// The copy the keep pins: an index into the copies, unused when `absent_kept` is set.
    pub kept: usize,
    /// The kept release as the keep record lists it, when no source holds it now (Q-88: the
    /// owner's computer replaced it with a newer local copy). Its version cannot be read, so
    /// any unconsidered copy is unordered against it.
    pub absent_kept: Option<Copy>,
    pub copies: Vec<usize>,
}

impl PartialEq for NewVersion {
    fn eq(&self, o: &Self) -> bool {
        self.keep == o.keep
            && self.scope == o.scope
            && self.kept == o.kept
            && self.absent_kept.as_ref().map(|c| c.version.hash)
                == o.absent_kept.as_ref().map(|c| c.version.hash)
            && self.copies == o.copies
    }
}

impl Eq for NewVersion {}

impl NewVersion {
    /// The raised copies that only `own` holds: they exist on this computer alone and were
    /// never shared (Q-88, D-26). Such a copy stays installed and unpublished until answered.
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

    /// The kept copy, from the copies passed to `new_versions` or the stand-in; `None` when the
    /// indexes do not belong to `copies`.
    pub fn kept_copy<'a>(&'a self, copies: &'a [Copy]) -> Option<&'a Copy> {
        match &self.absent_kept {
            Some(c) => Some(c),
            None => copies.get(self.kept),
        }
    }
}

/// A holder-less copy of the kept release, rebuilt from the keep record.
fn absent_copy(p: &Package, keep: &KeepEntry, rel: &Release) -> Copy {
    let folders = keep
        .folders
        .iter()
        .map(|m| FolderState {
            name: m.name.clone(),
            key: m.key.clone(),
            status: FolderStatus::Ok,
            tree: Some(m.tree),
            fingerprint: None,
            files: 0,
            bytes: 0,
            toc: TocSummary::empty(TocKind::None),
            file_entries: Vec::new(),
        })
        .collect();
    Copy {
        version: PackageVersion {
            package: p.key.clone(),
            hash: keep.package_hash,
            folders,
            client: ClientInfo {
                build_version: None,
                toc_mapping: String::new(),
            },
        },
        source: Source::from_release(rel),
        holders: Vec::new(),
    }
}

pub(crate) fn project(pv: &PackageVersion, scope: &[FolderKey]) -> PackageVersion {
    let mut p = pv.clone();
    p.folders.retain(|f| scope.contains(&f.key));
    p
}

// ---------------------------------------------------------------- writer (contracts.md §5.2)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    EmptyScope,
    KeyOutsideScope(FolderKey),
    NoTree(FolderKey),
    AllowAgainNotInherited(Sha256),
    Invalid(String),
    TooLarge,
}

pub struct WriteCx<'a> {
    pub group: &'a GroupId,
    pub issuer: Issuer,
    pub now_unix_ms: u64,
}

/// One complete copy as a record lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyEntry {
    pub folders: Vec<ManifestFolder>,
    pub holders: Vec<EnrollmentId>,
}

impl CopyEntry {
    pub fn new(mut folders: Vec<ManifestFolder>, mut holders: Vec<EnrollmentId>) -> Self {
        folders.sort();
        holders.sort();
        holders.dedup();
        Self { folders, holders }
    }

    /// A copy restricted to `scope`; a folder of the scope it lacks is simply not listed.
    pub fn from_copy(copy: &Copy, scope: &[FolderKey]) -> Result<Self, WriteError> {
        let mut folders = Vec::new();
        for f in copy
            .version
            .folders
            .iter()
            .filter(|f| scope.contains(&f.key))
        {
            let tree = f.tree.ok_or_else(|| WriteError::NoTree(f.key.clone()))?;
            folders.push(ManifestFolder {
                key: f.key.clone(),
                name: f.name.clone(),
                tree,
            });
        }
        let holders = copy
            .holders
            .iter()
            .map(|h| h.enrollment_id.clone())
            .collect();
        Ok(Self::new(folders, holders))
    }

    fn trees(&self) -> Vec<TreeEntry> {
        self.folders
            .iter()
            .map(|f| TreeEntry {
                key: f.key.clone(),
                tree: f.tree,
            })
            .collect()
    }

    pub fn hash(&self) -> Sha256 {
        let pairs: Vec<(FolderKey, Sha256)> = self
            .folders
            .iter()
            .map(|f| (f.key.clone(), f.tree))
            .collect();
        package_hash(&pairs)
    }
}

fn normalized_scope(scope: &[FolderKey]) -> Result<Vec<FolderKey>, WriteError> {
    let mut s = scope.to_vec();
    s.sort();
    s.dedup();
    if s.is_empty() {
        Err(WriteError::EmptyScope)
    } else {
        Ok(s)
    }
}

fn finish(c: CodeChoice) -> Result<CodeChoice, WriteError> {
    validate_choice(&c).map_err(|e| WriteError::Invalid(format!("{e:?}")))?;
    if encode(&c).len() > MAX_RECORD_BYTES {
        return Err(WriteError::TooLarge);
    }
    Ok(c)
}

/// A keep of `kept` over `scope`. `shown` lists every copy the owner saw; the kept copy is added
/// if missing. `exclude` is inherited plus every copy shown and not kept, minus every entry the
/// kept source is `Same` as. `known` covers every competing record.
pub fn build_keep(
    view: &ChoiceView,
    cx: &WriteCx,
    reason: ChoiceReason,
    scope: &[FolderKey],
    kept: &CopyEntry,
    shown: &[CopyEntry],
    kept_source: &Source,
) -> Result<CodeChoice, WriteError> {
    let scope = normalized_scope(scope)?;
    let mut by_hash: BTreeMap<Sha256, CopyEntry> = BTreeMap::new();
    for e in shown.iter().chain(std::iter::once(kept)) {
        for f in &e.folders {
            if !scope.contains(&f.key) {
                return Err(WriteError::KeyOutsideScope(f.key.clone()));
            }
        }
        let slot = by_hash.entry(e.hash()).or_insert_with(|| CopyEntry {
            folders: e.folders.clone(),
            holders: vec![],
        });
        slot.holders.extend(e.holders.iter().cloned());
        slot.holders.sort();
        slot.holders.dedup();
    }
    let keep_hash = kept.hash();
    let considered: Vec<Considered> = by_hash
        .iter()
        .map(|(h, e)| Considered {
            package_hash: *h,
            folders: e.trees(),
            holders: e.holders.clone(),
        })
        .collect();
    let mut exclude = view.inherited(&scope);
    exclude.extend(
        by_hash
            .iter()
            .filter(|(h, _)| **h != keep_hash)
            .map(|(h, e)| Exclude {
                package_hash: *h,
                scope: scope.clone(),
                folders: e.trees(),
            }),
    );
    exclude.retain(|e| {
        let trees: Vec<(FolderKey, Sha256)> =
            e.folders.iter().map(|t| (t.key.clone(), t.tree)).collect();
        same_release(kept_source, &Release::new(&e.scope, &trees)) != ScopeMatch::Same
            && !(e.scope == scope && e.package_hash == keep_hash)
    });
    exclude.sort();
    exclude.dedup();
    finish(CodeChoice {
        group_id: cx.group.clone(),
        issuer: cx.issuer.clone(),
        reason,
        action: ChoiceAction::Keep,
        keep: Some(KeepEntry {
            package_hash: keep_hash,
            folders: kept.folders.clone(),
        }),
        considered,
        exclude,
        allow_again: None,
        known: view.known_for(&scope),
        considered_parts: vec![],
        exclude_parts: vec![],
        allow_again_parts: vec![],
        folders: scope,
        written_unix_ms: cx.now_unix_ms,
    })
}

/// A resume over `scope`: ends the keeps it covers, imposes no pin and carries every inherited
/// exclusion forward except the hashes in `allow_again` (Q-50). An `allow_again` hash that no
/// inherited exclusion carries is refused.
pub fn build_resume(
    view: &ChoiceView,
    cx: &WriteCx,
    reason: ChoiceReason,
    scope: &[FolderKey],
    allow_again: &[Sha256],
) -> Result<CodeChoice, WriteError> {
    let scope = normalized_scope(scope)?;
    let inherited = view.inherited(&scope);
    let mut allow: Vec<Sha256> = allow_again.to_vec();
    allow.sort();
    allow.dedup();
    if let Some(h) = allow
        .iter()
        .find(|h| !inherited.iter().any(|e| e.package_hash == **h))
    {
        return Err(WriteError::AllowAgainNotInherited(*h));
    }
    let exclude: Vec<Exclude> = inherited
        .into_iter()
        .filter(|e| !allow.contains(&e.package_hash))
        .collect();
    finish(CodeChoice {
        group_id: cx.group.clone(),
        issuer: cx.issuer.clone(),
        reason,
        action: ChoiceAction::Resume,
        keep: None,
        considered: vec![],
        exclude,
        allow_again: if allow.is_empty() { None } else { Some(allow) },
        known: view.known_for(&scope),
        considered_parts: vec![],
        exclude_parts: vec![],
        allow_again_parts: vec![],
        folders: scope,
        written_unix_ms: cx.now_unix_ms,
    })
}
