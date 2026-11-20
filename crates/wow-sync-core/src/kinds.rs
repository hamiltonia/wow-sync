//! DTOs and well-formedness (V-KIND shape) rules of every authority-family kind
//! (contracts.md §1, removal.md §2, rollback.md §2, settings.md §3, ops.md §2).
//!
//! Validation here needs only the record itself. Rules that read the ledger (references, kind of
//! a referenced record, same issuer and seq) live in [`crate::ledger`].

use crate::hash::package_hash;
use crate::ids::{CheckpointRef, ComputerId, EnrollmentId, GroupId, SettingsUnit, Sha256};
use crate::paths::{validate_folder_name, FolderKey};
use crate::records::{Record, SettingsRoot, KIB};
use serde::{Deserialize, Serialize};

pub const MAX_RECORD_BYTES: usize = 64 * KIB;
pub const MAX_INDEX_REFS: usize = 1024;
pub const MAX_INDEX_BYTES: usize = 256 * KIB;
pub const MAX_FOLDERS: usize = 256;
pub const MAX_ACK_ITEMS: usize = 256;
pub const MAX_LIFTS: usize = 64;
pub const MAX_RETIREMENTS_KNOWN: usize = 64;
pub const MAX_PART_FANOUT: usize = 16;
pub const MAX_PART_DEPTH: usize = 8;
pub const INLINE_FILL_BYTES: usize = 48 * KIB;
pub const MAX_PART_BYTES: usize = 60 * KIB;
pub const MAX_LABEL_BYTES: usize = 128;
pub const MAX_POLICY_WITNESSES: usize = 8;

/// `true` when the slice is strictly ascending (sorted and unique).
pub fn strictly_sorted<T: Ord>(v: &[T]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}

fn sorted_by_key<T, K: Ord>(v: &[T], f: impl Fn(&T) -> K) -> bool {
    v.windows(2).all(|w| f(&w[0]) < f(&w[1]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    Removal,
    Readd,
    Ack,
    CodeChoice,
    Rejection,
    Lift,
    SettingsPolicy,
    SettingsRevision,
    Retirement,
    Part,
}

impl AuthorityKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Removal => "removal",
            Self::Readd => "readd",
            Self::Ack => "ack",
            Self::CodeChoice => "code_choice",
            Self::Rejection => "rejection",
            Self::Lift => "lift",
            Self::SettingsPolicy => "settings_policy",
            Self::SettingsRevision => "settings_revision",
            Self::Retirement => "retirement",
            Self::Part => "part",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issuer {
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
    pub issued_at_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnrollmentRef {
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seen {
    pub enrollment_id: EnrollmentId,
    pub seq: u64,
}

/// Causal summary (contracts.md §1.8).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Known {
    #[serde(default)]
    pub seen: Vec<Seen>,
    #[serde(default)]
    pub extra: Vec<Sha256>,
    #[serde(default)]
    pub parts: Vec<Sha256>,
}

impl Known {
    pub fn well_formed(&self) -> bool {
        strictly_sorted(&self.seen)
            && strictly_sorted(&self.extra)
            && strictly_sorted(&self.parts)
            && self.parts.len() <= MAX_PART_FANOUT * MAX_PART_FANOUT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManifestFolder {
    pub key: FolderKey,
    pub name: String,
    pub tree: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TreeEntry {
    pub key: FolderKey,
    pub tree: Sha256,
}

fn manifest_ok(folders: &[ManifestFolder]) -> bool {
    !folders.is_empty()
        && folders.len() <= MAX_FOLDERS
        && sorted_by_key(folders, |f| f.key.clone())
        && folders
            .iter()
            .all(|f| validate_folder_name(&f.name).is_ok_and(|k| k == f.key))
}

fn tree_entries_hash(folders: &[TreeEntry]) -> Sha256 {
    let v: Vec<(FolderKey, Sha256)> = folders.iter().map(|f| (f.key.clone(), f.tree)).collect();
    package_hash(&v)
}

fn title_ok(t: &str) -> bool {
    t.len() <= MAX_LABEL_BYTES
}

// ---------------------------------------------------------------- removal family

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemovalEvidence {
    Observed,
    UserAction,
    UserConfirmed,
    FolderDrop,
    Rollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaddEvidence {
    UserRestore,
    UserAddBack,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemovalSettings {
    None,
    Checkpoints { roots: Vec<SettingsRoot> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Removal {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub evidence: RemovalEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<Sha256>,
    pub title: String,
    pub folders: Vec<ManifestFolder>,
    pub removed_hash: Sha256,
    pub audience: Vec<EnrollmentRef>,
    pub settings: RemovalSettings,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readd {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub evidence: ReaddEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<Sha256>,
    pub title: String,
    pub folders: Vec<ManifestFolder>,
    pub present_hash: Sha256,
    pub known: Known,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AckItem {
    pub removal: Sha256,
    pub folders: Vec<FolderKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckResult {
    Removed,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckSettings {
    Paired,
    Partial,
    CodeOnly,
    KeptPaused,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub items: Vec<AckItem>,
    pub result: AckResult,
    #[serde(default)]
    pub removed: Vec<ManifestFolder>,
    pub settings: AckSettings,
    pub written_unix_ms: u64,
}

// ---------------------------------------------------------------- code choice and rejection

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceReason {
    Ambiguous,
    PartialCandidate,
    ChoicesDiffer,
    RollbackTargetsDiffer,
    NewerAfterRollback,
    NewVersionAfterDecision,
    RecoveredUnavailable,
    LookalikeOfRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceAction {
    Keep,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeepEntry {
    pub package_hash: Sha256,
    pub folders: Vec<ManifestFolder>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Considered {
    pub package_hash: Sha256,
    pub folders: Vec<TreeEntry>,
    pub holders: Vec<EnrollmentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Exclude {
    pub package_hash: Sha256,
    pub scope: Vec<FolderKey>,
    pub folders: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeChoice {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub reason: ChoiceReason,
    pub action: ChoiceAction,
    pub folders: Vec<FolderKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep: Option<KeepEntry>,
    #[serde(default)]
    pub considered: Vec<Considered>,
    #[serde(default)]
    pub exclude: Vec<Exclude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_again: Option<Vec<Sha256>>,
    pub known: Known,
    #[serde(default)]
    pub considered_parts: Vec<Sha256>,
    #[serde(default)]
    pub exclude_parts: Vec<Sha256>,
    #[serde(default)]
    pub allow_again_parts: Vec<Sha256>,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedEntry {
    pub package_hash: Sha256,
    pub version_label: String,
    pub folders: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredEntry {
    pub package_hash: Sha256,
    pub folders: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionSettings {
    Paired,
    Partial,
    CodeOnly,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub folders: Vec<FolderKey>,
    pub rejected: RejectedEntry,
    pub recovered: RecoveredEntry,
    #[serde(default)]
    pub considered: Vec<Considered>,
    pub settings: RejectionSettings,
    pub known: Known,
    #[serde(default)]
    pub considered_parts: Vec<Sha256>,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lift {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub folders: Vec<FolderKey>,
    pub lifts: Vec<Sha256>,
    pub known: Known,
    pub written_unix_ms: u64,
}

// ---------------------------------------------------------------- settings family

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRuleKind {
    Master,
    NotShared,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PolicyRule {
    pub folder: FolderKey,
    pub rule: PolicyRuleKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_id: Option<ComputerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment_id: Option<EnrollmentId>,
}

impl PolicyRule {
    pub fn well_formed(&self) -> bool {
        match self.rule {
            PolicyRuleKind::Master => self.computer_id.is_some() && self.enrollment_id.is_some(),
            PolicyRuleKind::NotShared => self.computer_id.is_none() && self.enrollment_id.is_none(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsPolicy {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub master: Option<EnrollmentRef>,
    #[serde(default)]
    pub folders: Vec<PolicyRule>,
    pub known: Known,
    #[serde(default)]
    pub folders_parts: Vec<Sha256>,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionKind {
    Auto,
    Push,
    Choice,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRevision {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub unit: SettingsUnit,
    pub kind: RevisionKind,
    pub checkpoint: CheckpointRef,
    #[serde(default)]
    pub parents: Vec<Sha256>,
    #[serde(default)]
    pub parents_parts: Vec<Sha256>,
    pub policy: Vec<Sha256>,
    pub code_package_hash: Option<Sha256>,
    pub known: Known,
    pub written_unix_ms: u64,
}

// ---------------------------------------------------------------- retirement and parts

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retirement {
    pub group_id: GroupId,
    pub issuer: Issuer,
    pub retired: EnrollmentRef,
    pub cutoff_seq: u64,
    pub was_settings_master: bool,
    #[serde(default)]
    pub retirements_known: Vec<Sha256>,
    pub written_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartOf {
    Known,
    Considered,
    Exclude,
    AllowAgain,
    PolicyFolders,
    RevisionParents,
}

/// One item of a part. The variant must match the part's `of` (V-PART).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PartItem {
    Exclude(Exclude),
    Considered(Considered),
    Rule(PolicyRule),
    Seen(Seen),
    Hash(Sha256),
}

impl PartItem {
    pub fn fits(&self, of: PartOf) -> bool {
        matches!(
            (of, self),
            (PartOf::Known, PartItem::Seen(_) | PartItem::Hash(_))
                | (PartOf::Considered, PartItem::Considered(_))
                | (PartOf::Exclude, PartItem::Exclude(_))
                | (PartOf::AllowAgain, PartItem::Hash(_))
                | (PartOf::PolicyFolders, PartItem::Rule(_))
                | (PartOf::RevisionParents, PartItem::Hash(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub group_id: GroupId,
    pub of: PartOf,
    pub items: Vec<PartItem>,
    #[serde(default)]
    pub more: Vec<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityRef {
    pub hash: Sha256,
    pub kind: AuthorityKind,
    #[serde(default)]
    pub folders: Vec<FolderKey>,
}

/// `wowsync.authority-index/1` chunk (contracts.md §1.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityIndexChunk {
    pub group_id: GroupId,
    pub enrollment_id: EnrollmentId,
    pub prev: Option<Sha256>,
    pub refs: Vec<AuthorityRef>,
}

// ---------------------------------------------------------------- validation

fn folders_list_ok(f: &[FolderKey], max: usize) -> bool {
    !f.is_empty() && f.len() <= max && strictly_sorted(f)
}

fn entries_ok(e: &[TreeEntry], scope: &[FolderKey]) -> bool {
    sorted_by_key(e, |x| x.key.clone()) && e.iter().all(|x| scope.contains(&x.key))
}

pub fn validate_removal(r: &Removal) -> Result<(), String> {
    if !manifest_ok(&r.folders) || !strictly_sorted(&r.audience) || !title_ok(&r.title) {
        return Err("malformed_listing".into());
    }
    if r.rejection.is_some() != (r.evidence == RemovalEvidence::Rollback) {
        return Err("removal_evidence".into());
    }
    Ok(())
}

pub fn validate_readd(r: &Readd) -> Result<(), String> {
    if !manifest_ok(&r.folders) || !title_ok(&r.title) {
        return Err("malformed_listing".into());
    }
    if r.rejection.is_some() != (r.evidence == ReaddEvidence::Rollback) || !r.known.well_formed() {
        return Err("readd_evidence".into());
    }
    Ok(())
}

pub fn validate_ack(a: &Ack) -> Result<(), String> {
    let items_ok = !a.items.is_empty()
        && a.items.len() <= MAX_ACK_ITEMS
        && sorted_by_key(&a.items, |i| i.removal)
        && a.items
            .iter()
            .all(|i| folders_list_ok(&i.folders, usize::MAX));
    let shape_ok = match a.result {
        AckResult::Absent => a.removed.is_empty(),
        AckResult::Removed => {
            !a.removed.is_empty()
                && a.removed
                    .iter()
                    .all(|m| a.items.iter().any(|i| i.folders.contains(&m.key)))
                && sorted_by_key(&a.removed, |m| m.key.clone())
        }
    };
    if items_ok && shape_ok {
        Ok(())
    } else {
        Err("malformed_ack".into())
    }
}

pub fn validate_choice(c: &CodeChoice) -> Result<(), String> {
    let bad = || Err("choice_shape".to_string());
    if !folders_list_ok(&c.folders, MAX_FOLDERS) {
        return bad();
    }
    if !strictly_sorted(&c.considered)
        || !c.considered.iter().all(|e| {
            entries_ok(&e.folders, &c.folders) && tree_entries_hash(&e.folders) == e.package_hash
        })
    {
        return bad();
    }
    if !strictly_sorted(&c.exclude)
        || !c.exclude.iter().all(|e| {
            !e.scope.is_empty()
                && e.scope.len() <= MAX_FOLDERS
                && strictly_sorted(&e.scope)
                && entries_ok(&e.folders, &e.scope)
                && e.scope.iter().any(|k| c.folders.contains(k))
        })
    {
        return bad();
    }
    if !c.known.well_formed() {
        return bad();
    }
    match c.action {
        ChoiceAction::Keep => {
            let Some(k) = &c.keep else { return bad() };
            if !manifest_ok(&k.folders) || !k.folders.iter().all(|f| c.folders.contains(&f.key)) {
                return bad();
            }
            let tree: Vec<TreeEntry> = k
                .folders
                .iter()
                .map(|f| TreeEntry {
                    key: f.key.clone(),
                    tree: f.tree,
                })
                .collect();
            if tree_entries_hash(&tree) != k.package_hash {
                return bad();
            }
            if !c
                .considered
                .iter()
                .any(|e| e.package_hash == k.package_hash)
            {
                return bad();
            }
            if c.allow_again.is_some()
                || c.exclude
                    .iter()
                    .any(|e| e.scope == c.folders && e.package_hash == k.package_hash)
            {
                return bad();
            }
        }
        ChoiceAction::Resume => {
            if c.keep.is_some() {
                return bad();
            }
            if let Some(a) = &c.allow_again {
                if !strictly_sorted(a)
                    || a.iter()
                        .any(|h| c.exclude.iter().any(|e| e.package_hash == *h))
                {
                    return bad();
                }
            }
        }
    }
    Ok(())
}

pub fn validate_rejection(r: &Rejection) -> Result<(), String> {
    let bad = |m: &str| Err(m.to_string());
    if !folders_list_ok(&r.folders, MAX_FOLDERS) {
        return bad("folders");
    }
    let list_ok = |e: &[TreeEntry]| !e.is_empty() && sorted_by_key(e, |x| x.key.clone());
    if !list_ok(&r.rejected.folders) || !list_ok(&r.recovered.folders) {
        return bad("entry folders");
    }
    let mut union: Vec<FolderKey> = r
        .rejected
        .folders
        .iter()
        .chain(&r.recovered.folders)
        .map(|f| f.key.clone())
        .collect();
    union.sort();
    union.dedup();
    if union != r.folders {
        return bad("folder union");
    }
    if tree_entries_hash(&r.rejected.folders) != r.rejected.package_hash
        || tree_entries_hash(&r.recovered.folders) != r.recovered.package_hash
        || r.rejected.package_hash == r.recovered.package_hash
    {
        return bad("package hash");
    }
    if !strictly_sorted(&r.considered)
        || !r.considered.iter().all(|e| {
            entries_ok(&e.folders, &r.folders) && tree_entries_hash(&e.folders) == e.package_hash
        })
    {
        return bad("considered");
    }
    if r.considered_parts.is_empty() {
        let has = |h: Sha256| r.considered.iter().any(|e| e.package_hash == h);
        if !has(r.rejected.package_hash) || !has(r.recovered.package_hash) {
            return bad("considered must contain rejected and recovered");
        }
    }
    if r.rejected.version_label.len() > MAX_LABEL_BYTES || !r.known.well_formed() {
        return bad("label or known");
    }
    Ok(())
}

pub fn validate_lift(l: &Lift) -> Result<(), String> {
    if l.lifts.is_empty()
        || l.lifts.len() > MAX_LIFTS
        || !strictly_sorted(&l.lifts)
        || !strictly_sorted(&l.folders)
        || !l.known.well_formed()
    {
        return Err("lift shape".into());
    }
    Ok(())
}

pub fn validate_policy(p: &SettingsPolicy) -> Result<(), String> {
    let bad = || Err("policy_body".to_string());
    if !p.known.well_formed() || !strictly_sorted(&p.folders_parts) {
        return bad();
    }
    if !sorted_by_key(&p.folders, |r| r.folder.clone())
        || !p.folders.iter().all(PolicyRule::well_formed)
    {
        return bad();
    }
    Ok(())
}

pub fn validate_revision(r: &SettingsRevision) -> Result<(), String> {
    if !strictly_sorted(&r.parents)
        || !strictly_sorted(&r.policy)
        || !strictly_sorted(&r.parents_parts)
        || r.policy.is_empty()
        || r.policy.len() > MAX_POLICY_WITNESSES
        || !r.known.well_formed()
    {
        return Err("revision shape".into());
    }
    Ok(())
}

pub fn validate_retirement(r: &Retirement) -> Result<(), String> {
    if !strictly_sorted(&r.retirements_known) || r.retirements_known.len() > MAX_RETIREMENTS_KNOWN {
        return Err("retirements_known".into());
    }
    Ok(())
}

pub fn validate_part(p: &Part) -> Result<(), String> {
    if !strictly_sorted(&p.items)
        || !p.items.iter().all(|i| i.fits(p.of))
        || !strictly_sorted(&p.more)
        || p.more.len() > MAX_PART_FANOUT
    {
        return Err("bad_part".into());
    }
    Ok(())
}

pub fn validate_index_chunk(c: &AuthorityIndexChunk) -> Result<(), String> {
    if c.refs.len() > MAX_INDEX_REFS {
        return Err("too many refs".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    if !c.refs.iter().all(|r| seen.insert(r.hash)) {
        return Err("a ref is listed once per enrollment".into());
    }
    Ok(())
}

// ---------------------------------------------------------------- writer: parts (§1.9)

/// The result of packing one partable list: what stays inline, the heads to name in the
/// parent's `…_parts` field, and every part file (`hash`, exact bytes) to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    pub inline: Vec<PartItem>,
    pub heads: Vec<Sha256>,
    pub parts: Vec<(Sha256, Vec<u8>)>,
}

fn item_bytes<T: Serialize>(item: &T) -> usize {
    serde_json::to_string_pretty(item).map_or(512, |s| {
        s.len() + 8 + 4 * s.bytes().filter(|b| *b == b'\n').count()
    })
}

/// Fills inline items until the parent would exceed 48 KiB (`base_bytes` is the parent without
/// the list), then puts the rest into parts of at most 60 KiB, fan-out at most 16 per level.
pub fn pack_items(
    group: &GroupId,
    of: PartOf,
    mut items: Vec<PartItem>,
    base_bytes: usize,
) -> Packed {
    items.sort();
    items.dedup();
    let mut used = base_bytes;
    let split = items
        .iter()
        .position(|i| {
            used += item_bytes(i);
            used > INLINE_FILL_BYTES
        })
        .unwrap_or(items.len());
    let rest = items.split_off(split);
    let mut parts: Vec<(Sha256, Vec<u8>)> = Vec::new();
    let mut emit = |p: Part| {
        let bytes = crate::records::encode(&p);
        let h = crate::hash::sha256(&bytes);
        parts.push((h, bytes));
        h
    };
    let mut level: Vec<Sha256> = Vec::new();
    let mut cur: Vec<PartItem> = Vec::new();
    let mut size = 512usize;
    for i in rest {
        let n = item_bytes(&i);
        if !cur.is_empty() && size + n > MAX_PART_BYTES - KIB {
            level.push(emit(Part {
                group_id: group.clone(),
                of,
                items: std::mem::take(&mut cur),
                more: vec![],
            }));
            size = 512;
        }
        size += n;
        cur.push(i);
    }
    if !cur.is_empty() {
        level.push(emit(Part {
            group_id: group.clone(),
            of,
            items: cur,
            more: vec![],
        }));
    }
    while level.len() > MAX_PART_FANOUT {
        level = level
            .chunks(MAX_PART_FANOUT)
            .map(|c| {
                let mut more = c.to_vec();
                more.sort();
                emit(Part {
                    group_id: group.clone(),
                    of,
                    items: vec![],
                    more,
                })
            })
            .collect();
    }
    level.sort();
    Packed {
        inline: items,
        heads: level,
        parts,
    }
}

/// Packs a causal summary: `seen` entries and `extra` hashes (contracts.md §1.8).
pub fn pack_known(
    group: &GroupId,
    seen: Vec<Seen>,
    extra: Vec<Sha256>,
    base_bytes: usize,
) -> (Known, Vec<(Sha256, Vec<u8>)>) {
    let items = seen
        .into_iter()
        .map(PartItem::Seen)
        .chain(extra.into_iter().map(PartItem::Hash))
        .collect();
    let p = pack_items(group, PartOf::Known, items, base_bytes);
    let mut k = Known::default();
    for i in p.inline {
        match i {
            PartItem::Seen(s) => k.seen.push(s),
            PartItem::Hash(h) => k.extra.push(h),
            _ => {}
        }
    }
    k.seen.sort();
    k.extra.sort();
    k.parts = p.heads;
    (k, p.parts)
}

macro_rules! authority_record {
    ($t:ty, $schema:literal, $v:path) => {
        impl Record for $t {
            const SCHEMA: &'static str = $schema;
            const MAJOR: u32 = 1;
            const LIMIT: usize = MAX_RECORD_BYTES;
            fn validate(&self) -> Result<(), String> {
                $v(self)
            }
        }
    };
}

authority_record!(Removal, "wowsync.removal", validate_removal);
authority_record!(Readd, "wowsync.readd", validate_readd);
authority_record!(Ack, "wowsync.removal-ack", validate_ack);
authority_record!(CodeChoice, "wowsync.code-choice", validate_choice);
authority_record!(Rejection, "wowsync.rejection", validate_rejection);
authority_record!(Lift, "wowsync.rejection-lift", validate_lift);
authority_record!(SettingsPolicy, "wowsync.settings-policy", validate_policy);
authority_record!(
    SettingsRevision,
    "wowsync.settings-revision",
    validate_revision
);
authority_record!(Retirement, "wowsync.retirement", validate_retirement);
authority_record!(Part, "wowsync.authority-part", validate_part);

impl Record for AuthorityIndexChunk {
    const SCHEMA: &'static str = "wowsync.authority-index";
    const MAJOR: u32 = 1;
    const LIMIT: usize = MAX_INDEX_BYTES;
    fn validate(&self) -> Result<(), String> {
        validate_index_chunk(self)
    }
}
