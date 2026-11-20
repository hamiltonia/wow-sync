//! The local authority ledger: entries, derived validity and segment files
//! (contracts.md §1.7, §1.11). Validity is never stored; it is a pure function of the
//! entries, their bodies and historical membership, so arrival order cannot change it.

use crate::authority::{
    decode_authority, retirement_statuses, AuthorityError, AuthorityRecord, Barrier, RetNode,
    RetStatus, RetVerdict,
};
use crate::ids::{ComputerId, EnrollmentId, GroupId, Sha256};
use crate::kinds::{AuthorityKind, EnrollmentRef, Issuer, Part, PartItem, PartOf};
use crate::paths::FolderKey;
use crate::records::{decode, Record, RecordError, MIB};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SEGMENT_BYTES: usize = 4 * MIB;
/// Above this many entries the app shows a diagnostic; entries are never dropped (§1.7).
pub const LONG_HISTORY_ENTRIES: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingReason {
    Unreadable,
    UnknownSchema,
    MissingReference,
    RetirementUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvalidReason {
    OtherGroup,
    NotAMember,
    WrongReference,
    VoidRetirement,
    AfterRetirement,
    /// A kind rule failed; the text is the rule's reason (for example `policy_body`).
    Kind(String),
}

impl InvalidReason {
    pub fn name(&self) -> &str {
        match self {
            Self::OtherGroup => "other_group",
            Self::NotAMember => "not_a_member",
            Self::WrongReference => "wrong_reference",
            Self::VoidRetirement => "void_retirement",
            Self::AfterRetirement => "after_retirement",
            Self::Kind(k) => k,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Valid,
    Pending(PendingReason),
    Invalid(InvalidReason),
}

impl Status {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub hash: Sha256,
    pub kind: AuthorityKind,
    #[serde(default)]
    pub folders: Vec<FolderKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Issuer>,
    #[serde(default)]
    pub learned_from: Vec<ComputerId>,
    pub mirrored_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Record(Box<AuthorityRecord>),
    /// No good copy was readable.
    Unreadable,
    UnknownSchema,
    /// A good copy that fails the kind's own rules; the text is the reason.
    Rejected(String),
}

/// Grow-only set of every (computer, enrollment) pair the group has ever had (§1.10).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pairs: BTreeSet<EnrollmentRef>,
}

impl Membership {
    pub fn pairs(&self) -> impl Iterator<Item = &EnrollmentRef> {
        self.pairs.iter()
    }
    pub fn len(&self) -> usize {
        self.pairs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
    pub fn add(&mut self, c: &ComputerId, e: &EnrollmentId) {
        self.pairs.insert(EnrollmentRef {
            computer_id: c.clone(),
            enrollment_id: e.clone(),
        });
    }
    pub fn contains(&self, c: &ComputerId, e: &EnrollmentId) -> bool {
        self.pairs.contains(&EnrollmentRef {
            computer_id: c.clone(),
            enrollment_id: e.clone(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ledger {
    entries: BTreeMap<Sha256, Entry>,
    bodies: BTreeMap<Sha256, Body>,
    dismissed: BTreeSet<Sha256>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, h: &Sha256) -> Option<&Entry> {
        self.entries.get(h)
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn body(&self, h: &Sha256) -> Option<&Body> {
        self.bodies.get(h)
    }

    pub fn record(&self, h: &Sha256) -> Option<&AuthorityRecord> {
        match self.bodies.get(h)? {
            Body::Record(r) => Some(r),
            _ => None,
        }
    }

    /// Learns a hash from a ref. `bytes` is a good-copy candidate; none means unreadable for now.
    /// The entry is only ever added to or completed, never removed (§1.7).
    pub fn learn(
        &mut self,
        hash: Sha256,
        kind_hint: AuthorityKind,
        folders: Vec<FolderKey>,
        from: Option<&ComputerId>,
        bytes: Option<&[u8]>,
    ) {
        let entry = self.entries.entry(hash).or_insert_with(|| Entry {
            hash,
            kind: kind_hint,
            folders,
            issuer: None,
            learned_from: Vec::new(),
            mirrored_seq: None,
        });
        if let Some(c) = from {
            if !entry.learned_from.contains(c) {
                entry.learned_from.push(c.clone());
                entry.learned_from.sort();
            }
        }
        let have_record = matches!(self.bodies.get(&hash), Some(Body::Record(_)));
        if have_record {
            return;
        }
        let body = match bytes {
            None => self.bodies.get(&hash).cloned().unwrap_or(Body::Unreadable),
            Some(b) => match decode_authority(b, &hash) {
                Ok(r) => {
                    entry.kind = r.kind();
                    entry.issuer = r.issuer().cloned();
                    Body::Record(Box::new(r))
                }
                Err(AuthorityError::TooLarge | AuthorityError::BadCopy) => Body::Unreadable,
                Err(AuthorityError::UnknownSchema(_)) => Body::UnknownSchema,
                Err(AuthorityError::Invalid(e)) => Body::Rejected(reason_of(&e)),
            },
        };
        self.bodies.insert(hash, body);
    }

    pub fn mark_mirrored(&mut self, h: &Sha256, own_seq: u64) {
        if let Some(e) = self.entries.get_mut(h) {
            e.mirrored_seq.get_or_insert(own_seq);
        }
    }

    pub fn dismiss(&mut self, h: Sha256) {
        self.dismissed.insert(h);
    }

    pub fn dismissed(&self) -> &BTreeSet<Sha256> {
        &self.dismissed
    }
}

/// Kind rules report a short reason token as the first word of their message.
fn reason_of(msg: &str) -> String {
    let w = msg.split_whitespace().next().unwrap_or("invalid");
    if w.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        w.to_string()
    } else {
        "invalid_record".to_string()
    }
}

/// The hashes a record references and the kind each must have (V-REF). `None` accepts any kind.
fn references(r: &AuthorityRecord) -> Vec<(Sha256, Option<AuthorityKind>)> {
    use AuthorityKind as K;
    let mut v: Vec<(Sha256, Option<K>)> = Vec::new();
    let known = |v: &mut Vec<(Sha256, Option<K>)>, k: &crate::kinds::Known| {
        v.extend(k.parts.iter().map(|h| (*h, Some(K::Part))));
        v.extend(k.extra.iter().map(|h| (*h, None)));
    };
    match r {
        AuthorityRecord::Removal(x) => v.extend(x.rejection.map(|h| (h, Some(K::Rejection)))),
        AuthorityRecord::Readd(x) => {
            v.extend(x.rejection.map(|h| (h, Some(K::Rejection))));
            known(&mut v, &x.known);
        }
        AuthorityRecord::Ack(x) => v.extend(x.items.iter().map(|i| (i.removal, Some(K::Removal)))),
        AuthorityRecord::CodeChoice(x) => {
            known(&mut v, &x.known);
            for p in x
                .considered_parts
                .iter()
                .chain(&x.exclude_parts)
                .chain(&x.allow_again_parts)
            {
                v.push((*p, Some(K::Part)));
            }
        }
        AuthorityRecord::Rejection(x) => {
            known(&mut v, &x.known);
            v.extend(x.considered_parts.iter().map(|h| (*h, Some(K::Part))));
        }
        AuthorityRecord::Lift(x) => {
            known(&mut v, &x.known);
            v.extend(x.lifts.iter().map(|h| (*h, Some(K::Rejection))));
        }
        AuthorityRecord::SettingsPolicy(x) => {
            known(&mut v, &x.known);
            v.extend(x.folders_parts.iter().map(|h| (*h, Some(K::Part))));
        }
        AuthorityRecord::SettingsRevision(x) => {
            known(&mut v, &x.known);
            v.extend(x.parents.iter().map(|h| (*h, Some(K::SettingsRevision))));
            v.extend(x.parents_parts.iter().map(|h| (*h, Some(K::Part))));
            v.extend(x.policy.iter().map(|h| (*h, Some(K::SettingsPolicy))));
        }
        AuthorityRecord::Retirement(x) => v.extend(
            x.retirements_known
                .iter()
                .map(|h| (*h, Some(K::Retirement))),
        ),
        AuthorityRecord::Part(x) => v.extend(x.more.iter().map(|h| (*h, Some(K::Part)))),
    }
    v
}

#[derive(Debug, Clone)]
pub struct Derived {
    pub status: BTreeMap<Sha256, Status>,
    pub barrier: Barrier,
    pub retirement_status: BTreeMap<Sha256, RetStatus>,
}

impl Derived {
    pub fn status_of(&self, h: &Sha256) -> Option<&Status> {
        self.status.get(h)
    }
}

/// Derives every entry's status. Pure and order-independent (A-20).
pub fn derive(ledger: &Ledger, group: &GroupId, members: &Membership) -> Derived {
    // Retirement nodes: settled by group, membership of issuer and retired pair only; the DAG
    // then decides Effective, Void or Unresolved.
    let mut nodes: BTreeMap<Sha256, RetNode> = BTreeMap::new();
    let mut valid_ret = BTreeMap::new();
    for e in ledger
        .entries()
        .filter(|e| e.kind == AuthorityKind::Retirement)
    {
        let node = match ledger.body(&e.hash) {
            Some(Body::Record(r)) => match r.as_ref() {
                AuthorityRecord::Retirement(x) => {
                    if &x.group_id != group {
                        RetNode::Invalid
                    } else if !members.contains(&x.issuer.computer_id, &x.issuer.enrollment_id)
                        || !members.contains(&x.retired.computer_id, &x.retired.enrollment_id)
                    {
                        RetNode::Unresolved
                    } else {
                        valid_ret.insert(e.hash, x.clone());
                        RetNode::Valid(x.clone())
                    }
                }
                _ => RetNode::Invalid,
            },
            Some(Body::Rejected(_)) => RetNode::Invalid,
            _ => RetNode::Unresolved,
        };
        nodes.insert(e.hash, node);
    }
    let retirement_status = retirement_statuses(&nodes);
    let barrier = Barrier::new(&valid_ret, &retirement_status);

    let mut status = BTreeMap::new();
    for e in ledger.entries() {
        let s = one(
            ledger,
            &e.hash,
            group,
            members,
            &barrier,
            &retirement_status,
        );
        status.insert(e.hash, s);
    }
    Derived {
        status,
        barrier,
        retirement_status,
    }
}

fn one(
    ledger: &Ledger,
    h: &Sha256,
    group: &GroupId,
    members: &Membership,
    barrier: &Barrier,
    ret: &BTreeMap<Sha256, RetStatus>,
) -> Status {
    use InvalidReason as I;
    let r = match ledger.body(h) {
        Some(Body::Record(r)) => r.as_ref(),
        Some(Body::UnknownSchema) => return Status::Pending(PendingReason::UnknownSchema),
        Some(Body::Rejected(why)) => return Status::Invalid(I::Kind(why.clone())),
        _ => return Status::Pending(PendingReason::Unreadable),
    };
    if r.group_id() != group {
        return Status::Invalid(I::OtherGroup);
    }
    if let Some(i) = r.issuer() {
        let retired_ok = match r {
            AuthorityRecord::Retirement(x) => {
                members.contains(&x.retired.computer_id, &x.retired.enrollment_id)
            }
            _ => true,
        };
        if !members.contains(&i.computer_id, &i.enrollment_id) || !retired_ok {
            return Status::Invalid(I::NotAMember);
        }
    }
    let mut missing = false;
    for (rh, want) in references(r) {
        match ledger.entry(&rh) {
            None => missing = true,
            Some(e) => {
                if want.is_some_and(|k| k != e.kind) {
                    return Status::Invalid(I::WrongReference);
                }
            }
        }
    }
    if missing {
        return Status::Pending(PendingReason::MissingReference);
    }
    for (of, heads) in part_chains(r) {
        match read_parts(ledger, heads, of) {
            Ok(_) => {}
            Err("missing_part") => return Status::Pending(PendingReason::MissingReference),
            Err("wrong_reference") => return Status::Invalid(I::WrongReference),
            Err(why) => return Status::Invalid(I::Kind(why.into())),
        }
    }
    match r {
        AuthorityRecord::Retirement(_) => match ret.get(h) {
            Some(RetStatus::Void) => return Status::Invalid(I::VoidRetirement),
            Some(RetStatus::Unresolved) | None => {
                return Status::Pending(PendingReason::RetirementUnresolved)
            }
            Some(RetStatus::Effective) => {}
        },
        _ => {
            if let Some(i) = r.issuer() {
                match barrier.verdict(&i.enrollment_id, i.issued_at_seq) {
                    RetVerdict::Valid => {}
                    RetVerdict::AfterRetirement => return Status::Invalid(I::AfterRetirement),
                    RetVerdict::RetirementUnresolved => {
                        return Status::Pending(PendingReason::RetirementUnresolved)
                    }
                }
            }
        }
    }
    match kind_rules(ledger, members, r) {
        Ok(()) => Status::Valid,
        Err(why) => Status::Invalid(I::Kind(why)),
    }
}

/// The V-KIND rules that need the ledger (the shape rules ran at decode).
fn kind_rules(ledger: &Ledger, members: &Membership, r: &AuthorityRecord) -> Result<(), String> {
    match r {
        AuthorityRecord::Removal(x) => {
            if x.evidence == crate::kinds::RemovalEvidence::Rollback {
                let ok = x.rejection.is_some_and(|h| match ledger.record(&h) {
                    Some(AuthorityRecord::Rejection(j)) => j.issuer == x.issuer,
                    _ => false,
                });
                if !ok {
                    return Err("removal_evidence".into());
                }
            }
        }
        AuthorityRecord::Ack(a) => {
            for it in &a.items {
                let Some(AuthorityRecord::Removal(rm)) = ledger.record(&it.removal) else {
                    return Err("malformed_ack".into());
                };
                if !it
                    .folders
                    .iter()
                    .all(|f| rm.folders.iter().any(|m| &m.key == f))
                {
                    return Err("malformed_ack".into());
                }
            }
        }
        AuthorityRecord::Lift(l) => {
            let mut union = BTreeSet::new();
            for h in &l.lifts {
                if let Some(AuthorityRecord::Rejection(j)) = ledger.record(h) {
                    union.extend(j.folders.iter().cloned());
                }
            }
            let named: BTreeSet<FolderKey> = l.folders.iter().cloned().collect();
            if union != named {
                return Err("lift_shape".into());
            }
        }
        AuthorityRecord::SettingsRevision(v) => {
            for h in &v.parents {
                match ledger.record(h) {
                    Some(AuthorityRecord::SettingsRevision(p)) if p.unit == v.unit => {}
                    _ => return Err("revision_parent".into()),
                }
            }
        }
        AuthorityRecord::SettingsPolicy(p) => {
            if let Err(why) = policy_folders(ledger, p) {
                return Err(why.into());
            }
            if let Some(m) = &p.master {
                if !members.contains(&m.computer_id, &m.enrollment_id) {
                    return Err("policy_body".into());
                }
            }
            for rule in &p.folders {
                if let (Some(c), Some(e)) = (&rule.computer_id, &rule.enrollment_id) {
                    if !members.contains(c, e) {
                        return Err("policy_body".into());
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Reads a part chain: all items of the given `of`, following `more`. A part reachable twice
/// or deeper than 8 makes the read fail (`bad_part`), a part of another `of` fails with
/// `wrong_reference`, and a part that is not (yet) readable fails with `missing_part`.
pub fn read_parts(
    ledger: &Ledger,
    heads: &[Sha256],
    of: PartOf,
) -> Result<Vec<PartItem>, &'static str> {
    let mut out: Vec<PartItem> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack: Vec<(Sha256, usize)> = heads.iter().map(|h| (*h, 1)).collect();
    while let Some((h, depth)) = stack.pop() {
        if !seen.insert(h) {
            return Err("bad_part");
        }
        if depth > crate::kinds::MAX_PART_DEPTH {
            return Err("bad_part");
        }
        let Some(AuthorityRecord::Part(p)) = ledger.record(&h) else {
            return Err("missing_part");
        };
        let p: &Part = p;
        if p.of != of {
            return Err("wrong_reference");
        }
        out.extend(p.items.iter().cloned());
        stack.extend(p.more.iter().map(|m| (*m, depth + 1)));
    }
    out.sort();
    Ok(out)
}

/// The full value of a causal summary: inline entries plus every reachable part.
pub fn resolve_known(
    ledger: &Ledger,
    k: &crate::kinds::Known,
) -> Result<crate::kinds::Known, &'static str> {
    let mut out = crate::kinds::Known {
        seen: k.seen.clone(),
        extra: k.extra.clone(),
        parts: vec![],
    };
    for i in read_parts(ledger, &k.parts, PartOf::Known)? {
        match i {
            PartItem::Seen(s) => out.seen.push(s),
            PartItem::Hash(h) => out.extra.push(h),
            _ => return Err("bad_part"),
        }
    }
    out.seen.sort();
    out.seen.dedup();
    out.extra.sort();
    out.extra.dedup();
    Ok(out)
}

/// The full rule list of a policy. The same folder twice, inline or in any part, is
/// `policy_body` (settings.md §3.2).
pub fn policy_folders(
    ledger: &Ledger,
    p: &crate::kinds::SettingsPolicy,
) -> Result<Vec<crate::kinds::PolicyRule>, &'static str> {
    let mut rules = p.folders.clone();
    for i in read_parts(ledger, &p.folders_parts, PartOf::PolicyFolders)? {
        match i {
            PartItem::Rule(r) => rules.push(r),
            _ => return Err("bad_part"),
        }
    }
    rules.sort_by(|a, b| a.folder.cmp(&b.folder));
    if rules.windows(2).any(|w| w[0].folder == w[1].folder) {
        return Err("policy_body");
    }
    Ok(rules)
}

/// Every part chain a record names, with the `of` it must have.
fn part_chains(r: &AuthorityRecord) -> Vec<(PartOf, &[Sha256])> {
    match r {
        AuthorityRecord::Readd(x) => vec![(PartOf::Known, &x.known.parts)],
        AuthorityRecord::Lift(x) => vec![(PartOf::Known, &x.known.parts)],
        AuthorityRecord::CodeChoice(x) => vec![
            (PartOf::Known, &x.known.parts),
            (PartOf::Considered, &x.considered_parts),
            (PartOf::Exclude, &x.exclude_parts),
            (PartOf::AllowAgain, &x.allow_again_parts),
        ],
        AuthorityRecord::Rejection(x) => vec![
            (PartOf::Known, &x.known.parts),
            (PartOf::Considered, &x.considered_parts),
        ],
        AuthorityRecord::SettingsPolicy(x) => vec![
            (PartOf::Known, &x.known.parts),
            (PartOf::PolicyFolders, &x.folders_parts),
        ],
        AuthorityRecord::SettingsRevision(x) => vec![
            (PartOf::Known, &x.known.parts),
            (PartOf::RevisionParents, &x.parents_parts),
        ],
        _ => vec![],
    }
}

// ---------------------------------------------------------------- journal vs ledger (V-JRN)

/// Ledger-level journal checks (contracts.md §2.2 V-JRN): every `remove` folder is named by a
/// listed removal, and each listed removal is valid in the ledger or has its acknowledgement
/// among the journal's pending records.
pub fn validate_journal_against_ledger(
    j: &crate::records::ApplyJournalV2,
    ledger: &Ledger,
    derived: &Derived,
) -> Result<(), String> {
    use crate::plan::FolderAction;
    let acked: BTreeMap<Sha256, Vec<FolderKey>> = j
        .pending_records
        .iter()
        .filter_map(|p| match decode_authority(p.bytes.as_bytes(), &p.hash) {
            Ok(AuthorityRecord::Ack(a)) => Some(a),
            _ => None,
        })
        .flat_map(|a| a.items.into_iter().map(|i| (i.removal, i.folders)))
        .collect();
    let mut named: BTreeSet<FolderKey> = BTreeSet::new();
    for h in &j.removals {
        let pending_ack = acked.get(h);
        match (derived.status_of(h), ledger.record(h)) {
            (Some(Status::Valid), Some(AuthorityRecord::Removal(r))) => {
                named.extend(r.folders.iter().map(|f| f.key.clone()));
            }
            _ if pending_ack.is_some() => {
                named.extend(pending_ack.into_iter().flatten().cloned());
            }
            _ => return Err("removal_not_valid".into()),
        }
    }
    for f in j
        .folders
        .iter()
        .filter(|f| f.action == FolderAction::Remove)
    {
        if !named.contains(&f.key) {
            return Err("remove_folder_unnamed".into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- authority index chain (§1.6)

/// A member's published `authority_index` pointer in its code state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexHead {
    pub head: Sha256,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    Missing(Sha256),
    Bad(Sha256),
    Foreign(Sha256),
    Loop,
    TotalMismatch { found: u64, total: u64 },
}

/// What one walk of an index chain produced. `refs` are the refs of the newly read chunks and
/// are valid to learn even when `error` is set: a broken chain never makes learned refs go away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRead {
    pub refs: Vec<crate::kinds::AuthorityRef>,
    /// Chunk hash to the number of refs from the oldest chunk through it, for chunks read whole.
    pub processed: BTreeMap<Sha256, u64>,
    pub error: Option<IndexError>,
}

/// Walks from the head along `prev` until a chunk in `known_complete` (hash to cumulative ref
/// count) is reached. The sum over the chain must equal `total`.
pub fn read_index_chain(
    head: &IndexHead,
    group: &GroupId,
    enrollment: &EnrollmentId,
    known_complete: &BTreeMap<Sha256, u64>,
    fetch: &dyn Fn(&Sha256) -> Option<Vec<u8>>,
) -> IndexRead {
    let mut chain: Vec<(Sha256, crate::kinds::AuthorityIndexChunk)> = Vec::new();
    let mut base = 0u64;
    let mut seen = BTreeSet::new();
    let mut cur = Some(head.head);
    let mut error = None;
    while let Some(h) = cur {
        if let Some(n) = known_complete.get(&h) {
            base = *n;
            break;
        }
        if !seen.insert(h) {
            error = Some(IndexError::Loop);
            break;
        }
        let Some(bytes) = fetch(&h) else {
            error = Some(IndexError::Missing(h));
            break;
        };
        let decoded = if sha256_matches(&bytes, &h) {
            decode::<crate::kinds::AuthorityIndexChunk>(&bytes, crate::kinds::MAX_INDEX_BYTES).ok()
        } else {
            None
        };
        let Some(chunk) = decoded else {
            error = Some(IndexError::Bad(h));
            break;
        };
        if &chunk.group_id != group || &chunk.enrollment_id != enrollment {
            error = Some(IndexError::Foreign(h));
            break;
        }
        cur = chunk.prev;
        chain.push((h, chunk));
    }
    let mut refs = Vec::new();
    let mut processed = BTreeMap::new();
    let mut count = base;
    for (h, c) in chain.into_iter().rev() {
        // A broken chain keeps the refs of the chunks read; only whole chains count as processed.
        count += c.refs.len() as u64;
        refs.extend(c.refs);
        if error.is_none() {
            processed.insert(h, count);
        }
    }
    if error.is_none() && count != head.total {
        error = Some(IndexError::TotalMismatch {
            found: count,
            total: head.total,
        });
    }
    IndexRead {
        refs,
        processed,
        error,
    }
}

fn sha256_matches(bytes: &[u8], h: &Sha256) -> bool {
    &crate::hash::sha256(bytes) == h
}

/// Adds refs to a chain. When the head chunk is not full it is replaced by a chunk with the same
/// `prev` holding its refs plus the new ones; the rest follow in new chunks. Returns the chunk
/// files to write (write-if-absent) and the new pointer.
pub fn append_index_refs(
    group: &GroupId,
    enrollment: &EnrollmentId,
    head: Option<(&Sha256, &crate::kinds::AuthorityIndexChunk)>,
    total: u64,
    new_refs: Vec<crate::kinds::AuthorityRef>,
) -> (Vec<(Sha256, Vec<u8>)>, IndexHead) {
    let max = crate::kinds::MAX_INDEX_REFS;
    let (mut prev, mut pending) = match head {
        Some((_, c)) if c.refs.len() < max => (c.prev, c.refs.clone()),
        Some((h, _)) => (Some(*h), Vec::new()),
        None => (None, Vec::new()),
    };
    pending.extend(new_refs.iter().cloned());
    let added = new_refs.len() as u64;
    let mut files = Vec::new();
    let mut last = None;
    for batch in pending.chunks(max) {
        let chunk = crate::kinds::AuthorityIndexChunk {
            group_id: group.clone(),
            enrollment_id: enrollment.clone(),
            prev,
            refs: batch.to_vec(),
        };
        let bytes = crate::records::encode(&chunk);
        let h = crate::hash::sha256(&bytes);
        files.push((h, bytes));
        prev = Some(h);
        last = Some(h);
    }
    let head_hash = last.or(head.map(|(h, _)| *h)).expect("a chain has a head");
    (
        files,
        IndexHead {
            head: head_hash,
            total: total + added,
        },
    )
}

// ---------------------------------------------------------------- segment files

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerSegment {
    pub entries: Vec<Entry>,
}

impl Record for LedgerSegment {
    const SCHEMA: &'static str = "wowsync.authority-ledger-segment";
    const MAJOR: u32 = 1;
    const LIMIT: usize = SEGMENT_BYTES;
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Splits entries into segment files of at most 4 MiB, oldest entries first.
pub fn encode_segments(entries: &[Entry]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur: Vec<Entry> = Vec::new();
    let mut cur_bytes = 128usize;
    for e in entries {
        let n = serde_json::to_string_pretty(e).map_or(512, |s| {
            // The entry is indented four more spaces on every line inside the segment.
            s.len() + 4 + 4 * s.bytes().filter(|b| *b == b'\n').count() + 4
        });
        if !cur.is_empty() && cur_bytes + n > SEGMENT_BYTES {
            out.push(crate::records::encode(&LedgerSegment {
                entries: std::mem::take(&mut cur),
            }));
            cur_bytes = 128;
        }
        cur_bytes += n;
        cur.push(e.clone());
    }
    if !cur.is_empty() {
        out.push(crate::records::encode(&LedgerSegment { entries: cur }));
    }
    out
}

pub fn decode_segment(bytes: &[u8]) -> Result<LedgerSegment, RecordError> {
    decode::<LedgerSegment>(bytes, SEGMENT_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256;
    use crate::kinds::{
        Ack, AckItem, AckResult, AckSettings, ManifestFolder, Removal, RemovalEvidence,
        RemovalSettings, Retirement,
    };
    use crate::records::encode;
    use crate::testkit::{computer, enrollment, golden, group_id};

    fn issuer(n: u8, seq: u64) -> Issuer {
        Issuer {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
            issued_at_seq: seq,
        }
    }

    fn eref(n: u8) -> EnrollmentRef {
        EnrollmentRef {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
        }
    }

    fn retirement(by: u8, retired: u8, cutoff: u64) -> Retirement {
        Retirement {
            group_id: group_id(),
            issuer: issuer(by, 1),
            retired: eref(retired),
            cutoff_seq: cutoff,
            was_settings_master: false,
            retirements_known: vec![],
            written_unix_ms: 7,
        }
    }

    fn removal(by: u8, seq: u64) -> Removal {
        Removal {
            group_id: group_id(),
            issuer: issuer(by, seq),
            evidence: RemovalEvidence::UserAction,
            rejection: None,
            title: "Suite".into(),
            folders: vec![ManifestFolder {
                key: FolderKey::lossy("suite"),
                name: "Suite".into(),
                tree: sha256(b"tree"),
            }],
            removed_hash: sha256(b"pkg"),
            audience: vec![eref(1), eref(2)],
            settings: RemovalSettings::None,
            written_unix_ms: 7,
        }
    }

    fn members(ns: &[u8]) -> Membership {
        let mut m = Membership::default();
        for n in ns {
            m.add(&computer(*n), &enrollment(*n));
        }
        m
    }

    fn learn<R: Record>(l: &mut Ledger, r: &R, kind: AuthorityKind) -> Sha256 {
        let bytes = encode(r);
        let h = sha256(&bytes);
        l.learn(h, kind, vec![], Some(&computer(1)), Some(&bytes));
        h
    }

    #[test]
    fn golden_bytes_and_name_is_the_hash() {
        // T-C01-golden-bytes (removal, retirement, ack)
        let rm = encode(&removal(2, 3));
        golden("removal-1.json", &rm);
        golden("retirement-1.json", &encode(&retirement(1, 2, 10)));
        let mut l = Ledger::new();
        let h = learn(&mut l, &removal(2, 3), AuthorityKind::Removal);
        assert_eq!(h, sha256(&rm));
        assert!(matches!(l.body(&h), Some(Body::Record(_))));
        let ack = Ack {
            group_id: group_id(),
            issuer: issuer(1, 4),
            items: vec![AckItem {
                removal: h,
                folders: vec![FolderKey::lossy("suite")],
            }],
            result: AckResult::Removed,
            removed: vec![],
            settings: AckSettings::NotApplicable,
            written_unix_ms: 8,
        };
        golden("removal-ack-1.json", &encode(&ack));
    }

    #[test]
    fn journal_removals_must_be_valid_and_name_every_remove_folder() {
        // V-JRN ledger-level checks
        use crate::plan::{ExpectedLive, FolderAction};
        use crate::records::{ApplyJournalV2, JournalFolderV2, OpKind, RecoverySlot};
        let mut l = Ledger::new();
        let rm = learn(&mut l, &removal(2, 3), AuthorityKind::Removal);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        let f = |name: &str, key: &str| JournalFolderV2 {
            name: name.into(),
            key: FolderKey::lossy(key),
            action: FolderAction::Remove,
            expected_live: ExpectedLive::Absent,
            staged: None,
            recovery_slot: RecoverySlot::new("r/0").unwrap(),
        };
        let j = |folders: Vec<JournalFolderV2>, removals: Vec<Sha256>| ApplyJournalV2 {
            op: crate::testkit::op(1),
            group_id: group_id(),
            started_unix_ms: 1,
            kind: OpKind::Remove,
            package: FolderKey::lossy("suite"),
            target_hash: None,
            phase: crate::records::JournalPhase::Staging,
            from_phase: None,
            reason: None,
            halted_by_game: false,
            folders,
            removals,
            pending_records: vec![],
            settings_ops: vec![],
        };
        assert_eq!(
            validate_journal_against_ledger(&j(vec![f("Suite", "suite")], vec![rm]), &l, &d),
            Ok(())
        );
        assert_eq!(
            validate_journal_against_ledger(&j(vec![f("Other", "other")], vec![rm]), &l, &d),
            Err("remove_folder_unnamed".into())
        );
        assert_eq!(
            validate_journal_against_ledger(
                &j(vec![f("Suite", "suite")], vec![sha256(b"unknown")]),
                &l,
                &d
            ),
            Err("removal_not_valid".into())
        );
    }

    #[test]
    fn bad_copy_is_skipped_and_a_good_copy_is_used() {
        // T-C03-bad-copy
        let bytes = encode(&removal(2, 3));
        let h = sha256(&bytes);
        let mut l = Ledger::new();
        l.learn(h, AuthorityKind::Removal, vec![], None, Some(b"damaged"));
        assert!(matches!(l.body(&h), Some(Body::Unreadable)));
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(
            d.status_of(&h),
            Some(&Status::Pending(PendingReason::Unreadable))
        );
        l.learn(h, AuthorityKind::Removal, vec![], None, Some(&bytes));
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(d.status_of(&h), Some(&Status::Valid));
    }

    #[test]
    fn a_reference_of_the_wrong_kind_is_invalid() {
        // T-C02-retired-forms-rejected (wrong_reference)
        let mut l = Ledger::new();
        let ret = learn(&mut l, &retirement(1, 2, 10), AuthorityKind::Retirement);
        let ack = Ack {
            group_id: group_id(),
            issuer: issuer(1, 4),
            items: vec![AckItem {
                removal: ret,
                folders: vec![FolderKey::lossy("suite")],
            }],
            result: AckResult::Absent,
            removed: vec![],
            settings: AckSettings::NotApplicable,
            written_unix_ms: 8,
        };
        let h = learn(&mut l, &ack, AuthorityKind::Ack);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(
            d.status_of(&h),
            Some(&Status::Invalid(InvalidReason::WrongReference))
        );
    }

    #[test]
    fn historical_membership_and_non_member_retirement() {
        // T-C13-historical-membership
        let mut l = Ledger::new();
        let rm = learn(&mut l, &removal(2, 3), AuthorityKind::Removal);
        let non_member = learn(&mut l, &retirement(1, 7, 3), AuthorityKind::Retirement);
        let m = members(&[1, 2]);
        let d = derive(&l, &group_id(), &m);
        assert_eq!(d.status_of(&rm), Some(&Status::Valid));
        assert_eq!(
            d.status_of(&non_member),
            Some(&Status::Invalid(InvalidReason::NotAMember))
        );
        // Enrollment 2 left member.json but stays in the grow-only membership set.
        let d = derive(&l, &group_id(), &members(&[1, 2, 7]));
        assert_eq!(d.status_of(&rm), Some(&Status::Valid));
    }

    #[test]
    fn records_above_a_retirement_cutoff_are_invalid_in_any_order() {
        // T-C10 at the ledger: B's seq-11 removal after A retires B at 10.
        let mut l = Ledger::new();
        let rm = learn(&mut l, &removal(2, 11), AuthorityKind::Removal);
        let before = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(before.status_of(&rm), Some(&Status::Valid));
        let r = learn(&mut l, &retirement(1, 2, 10), AuthorityKind::Retirement);
        let after = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(after.status_of(&r), Some(&Status::Valid));
        assert_eq!(
            after.status_of(&rm),
            Some(&Status::Invalid(InvalidReason::AfterRetirement))
        );
    }

    #[test]
    fn the_ledger_barrier_gates_member_states_end_to_end() {
        // A-38 admission: a record above the cutoff is never admitted; an unresolved retirement
        // holds as pending(retirement_unresolved) and waits instead of being judged.
        use crate::admit::{
            admit_member_states, apply_barrier, barrier_gate, BarrierGate, MemberLatest, MemberObs,
            StateFileObs,
        };
        use crate::layout::state_file_name;
        use crate::records::AdmissionWatermarks;
        use crate::testkit;

        let member_obs = |n: u8, seq: u64| {
            let m = testkit::member_record(&computer(n));
            let inv = testkit::inventory(vec![testkit::addon("Suite", Some("1.0"), "a")]);
            let bytes = encode(&testkit::member_state(&computer(n), seq, &inv));
            MemberObs {
                computer: computer(n),
                files: vec![StateFileObs {
                    name: state_file_name(&m.enrollment_id, seq),
                    size: bytes.len() as u64,
                    cloud_only: false,
                }],
                member: Ok(m),
                newest_bytes: Some(Ok(bytes)),
            }
        };
        let admit = |barrier: &crate::authority::Barrier, n: u8, seq: u64| {
            let (views, _) = admit_member_states(
                &testkit::group_record(),
                &[member_obs(n, seq)],
                &AdmissionWatermarks::empty(group_id()),
            );
            apply_barrier(views, barrier)
        };

        // Effective retirement of computer 2 at cutoff 2.
        let mut l = Ledger::new();
        let after = learn(&mut l, &removal(2, 3), AuthorityKind::Removal);
        learn(&mut l, &retirement(1, 2, 2), AuthorityKind::Retirement);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(
            d.status_of(&after),
            Some(&Status::Invalid(InvalidReason::AfterRetirement))
        );
        let e2 = enrollment(2);
        assert_eq!(barrier_gate(&d.barrier, &e2, 2), BarrierGate::Admitted);
        assert_eq!(
            barrier_gate(&d.barrier, &e2, 3),
            BarrierGate::AfterRetirement
        );
        let (kept, retired) = admit(&d.barrier, 2, 3);
        assert!(kept.is_empty(), "a retired member contributes no state");
        assert_eq!(retired, vec![e2.clone()]);
        let (kept, _) = admit(&d.barrier, 1, 3);
        assert!(matches!(kept[0].latest, MemberLatest::Valid { seq: 3, .. }));

        // A retirement that depends on a record nobody has: unresolved, not void.
        let mut l = Ledger::new();
        let mut unresolved = retirement(1, 2, 2);
        unresolved.retirements_known = vec![sha256(b"not here")];
        learn(&mut l, &unresolved, AuthorityKind::Retirement);
        let held = learn(&mut l, &removal(2, 3), AuthorityKind::Removal);
        let before_cut = learn(&mut l, &removal(2, 2), AuthorityKind::Removal);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert_eq!(
            d.status_of(&held),
            Some(&Status::Pending(PendingReason::RetirementUnresolved))
        );
        assert_eq!(d.status_of(&before_cut), Some(&Status::Valid));
        assert_eq!(barrier_gate(&d.barrier, &e2, 3), BarrierGate::Held);
        assert_eq!(barrier_gate(&d.barrier, &e2, 2), BarrierGate::Admitted);
        let (kept, retired) = admit(&d.barrier, 2, 3);
        assert!(retired.is_empty());
        assert!(
            matches!(&kept[0].latest, MemberLatest::Unusable { seq: 3, reason } if reason.contains("retirement")),
            "{:?}",
            kept[0].latest
        );
        let (kept, _) = admit(&d.barrier, 2, 2);
        assert!(matches!(kept[0].latest, MemberLatest::Valid { seq: 2, .. }));
    }

    #[test]
    fn segments_reload_past_sixteen_mib() {
        // T-C09-ledger-segments
        let names: Vec<FolderKey> = (0..2000)
            .map(|i| FolderKey::lossy(&format!("{i:0>50}")))
            .collect();
        let entries: Vec<Entry> = (0..170u32)
            .map(|i| Entry {
                hash: sha256(&i.to_le_bytes()),
                kind: AuthorityKind::Removal,
                folders: names.clone(),
                issuer: None,
                learned_from: vec![computer(1)],
                mirrored_seq: Some(1),
            })
            .collect();
        let segs = encode_segments(&entries);
        assert!(segs.iter().map(Vec::len).sum::<usize>() > 16 * MIB);
        assert!(segs.len() >= 4 && segs.iter().all(|s| s.len() <= SEGMENT_BYTES));
        let mut back = Vec::new();
        for s in &segs {
            back.extend(decode_segment(s).unwrap().entries);
        }
        assert_eq!(back, entries);
    }

    fn eid(n: u32) -> EnrollmentId {
        EnrollmentId::parse(&format!("{:032x}", n + 0x1000)).unwrap()
    }

    fn readd_with(known: crate::kinds::Known) -> crate::kinds::Readd {
        crate::kinds::Readd {
            group_id: group_id(),
            issuer: issuer(1, 5000),
            evidence: crate::kinds::ReaddEvidence::UserAddBack,
            rejection: None,
            title: "Suite".into(),
            folders: vec![ManifestFolder {
                key: FolderKey::lossy("suite"),
                name: "Suite".into(),
                tree: sha256(b"tree"),
            }],
            present_hash: sha256(b"present"),
            known,
            written_unix_ms: 9,
        }
    }

    fn learn_part(l: &mut Ledger, h: Sha256, bytes: &[u8]) {
        l.learn(h, AuthorityKind::Part, vec![], None, Some(bytes));
    }

    #[test]
    fn readd_after_1024_removals_fits_and_supersedes_them_but_not_a_concurrent_one() {
        // T-C08-known-bound
        use crate::kinds::{Known, Seen};
        let known = Known {
            seen: vec![Seen {
                enrollment_id: enrollment(2),
                seq: 1024,
            }],
            extra: vec![],
            parts: vec![],
        };
        let bytes = encode(&readd_with(known.clone()));
        assert!(bytes.len() < 64 * 1024, "{}", bytes.len());
        let none = BTreeSet::new();
        for s in 1..=1024u64 {
            let h = sha256(&s.to_le_bytes());
            assert!(crate::authority::covered(&known, &none, &issuer(2, s), &h));
        }
        let h = sha256(&1025u64.to_le_bytes());
        assert!(!crate::authority::covered(
            &known,
            &none,
            &issuer(2, 1025),
            &h
        ));
        // A removal of another enrollment is not covered either.
        assert!(!crate::authority::covered(&known, &none, &issuer(3, 1), &h));
    }

    #[test]
    fn readd_after_1024_issuers_moves_seen_into_parts_and_waits_for_every_part() {
        // T-R4D4-1024-issuers-readd
        use crate::kinds::{pack_known, Known, Seen};
        let seen: Vec<Seen> = (0..1024)
            .map(|n| Seen {
                enrollment_id: eid(n),
                seq: 1,
            })
            .collect();
        let base = encode(&readd_with(Known::default())).len();
        let (known, parts) = pack_known(&group_id(), seen, vec![], base);
        assert!(!known.parts.is_empty() && !parts.is_empty());
        assert!(known.seen.len() < 1024);
        let rd = readd_with(known.clone());
        let bytes = encode(&rd);
        assert!(bytes.len() <= 64 * 1024);
        assert!(parts.iter().all(|(_, b)| b.len() <= 64 * 1024));
        let mut l = Ledger::new();
        let h = learn(&mut l, &rd, AuthorityKind::Readd);
        let m = members(&[1]);
        let status = |l: &Ledger| derive(l, &group_id(), &m).status_of(&h).cloned();
        assert_eq!(
            status(&l),
            Some(Status::Pending(PendingReason::MissingReference))
        );
        for (ph, pb) in &parts[..parts.len() - 1] {
            learn_part(&mut l, *ph, pb);
        }
        if parts.len() > 1 {
            assert_eq!(
                status(&l),
                Some(Status::Pending(PendingReason::MissingReference))
            );
        }
        let (ph, pb) = &parts[parts.len() - 1];
        learn_part(&mut l, *ph, pb);
        assert_eq!(status(&l), Some(Status::Valid));
        let full = resolve_known(&l, &known).unwrap();
        assert_eq!(full.seen.len(), 1024);
        let none = BTreeSet::new();
        for n in 0..1024 {
            let by = Issuer {
                computer_id: computer(1),
                enrollment_id: eid(n),
                issued_at_seq: 1,
            };
            assert!(crate::authority::covered(&full, &none, &by, &sha256(b"x")));
        }
    }

    fn eref_hash(pairs: &[(&str, &[u8])]) -> (Vec<crate::kinds::TreeEntry>, Sha256) {
        let entries: Vec<crate::kinds::TreeEntry> = pairs
            .iter()
            .map(|(k, t)| crate::kinds::TreeEntry {
                key: FolderKey::lossy(k),
                tree: sha256(t),
            })
            .collect();
        let v: Vec<(FolderKey, Sha256)> = entries.iter().map(|e| (e.key.clone(), e.tree)).collect();
        (entries, crate::hash::package_hash(&v))
    }

    #[test]
    fn golden_bytes_of_every_remaining_kind_part_and_index_chunk() {
        // T-C01-golden-bytes: readd, code-choice, rejection, lift, policy, revision, part, index
        use crate::kinds::*;
        let suite = || vec![FolderKey::lossy("suite")];
        let mut all: Vec<(&str, Vec<u8>, AuthorityKind)> = Vec::new();

        all.push((
            "readd-1.json",
            encode(&readd_with(Known {
                seen: vec![Seen {
                    enrollment_id: enrollment(2),
                    seq: 3,
                }],
                extra: vec![sha256(b"mirrored")],
                parts: vec![],
            })),
            AuthorityKind::Readd,
        ));

        let (tree_a, hash_a) = eref_hash(&[("suite", b"a")]);
        let (tree_b, hash_b) = eref_hash(&[("suite", b"b")]);
        let choice = CodeChoice {
            group_id: group_id(),
            issuer: issuer(1, 6),
            reason: ChoiceReason::Ambiguous,
            action: ChoiceAction::Keep,
            folders: suite(),
            keep: Some(KeepEntry {
                package_hash: hash_a,
                folders: vec![ManifestFolder {
                    key: FolderKey::lossy("suite"),
                    name: "Suite".into(),
                    tree: sha256(b"a"),
                }],
            }),
            considered: {
                let mut v = vec![
                    Considered {
                        package_hash: hash_a,
                        folders: tree_a.clone(),
                        holders: vec![enrollment(1)],
                    },
                    Considered {
                        package_hash: hash_b,
                        folders: tree_b.clone(),
                        holders: vec![enrollment(2)],
                    },
                ];
                v.sort();
                v
            },
            exclude: vec![],
            allow_again: None,
            known: Default::default(),
            considered_parts: vec![],
            exclude_parts: vec![],
            allow_again_parts: vec![],
            written_unix_ms: 9,
        };
        all.push((
            "code-choice-1.json",
            encode(&choice),
            AuthorityKind::CodeChoice,
        ));

        let rejection = Rejection {
            group_id: group_id(),
            issuer: issuer(1, 7),
            folders: suite(),
            rejected: RejectedEntry {
                package_hash: hash_b,
                version_label: "2.0".into(),
                folders: tree_b.clone(),
            },
            recovered: RecoveredEntry {
                package_hash: hash_a,
                folders: tree_a.clone(),
            },
            considered: {
                let mut v = vec![
                    Considered {
                        package_hash: hash_a,
                        folders: tree_a,
                        holders: vec![enrollment(1)],
                    },
                    Considered {
                        package_hash: hash_b,
                        folders: tree_b,
                        holders: vec![enrollment(2)],
                    },
                ];
                v.sort();
                v
            },
            settings: RejectionSettings::NotApplicable,
            known: Default::default(),
            considered_parts: vec![],
            written_unix_ms: 9,
        };
        let rej_bytes = encode(&rejection);
        let rej_hash = sha256(&rej_bytes);
        all.push(("rejection-1.json", rej_bytes, AuthorityKind::Rejection));

        let lift = Lift {
            group_id: group_id(),
            issuer: issuer(1, 8),
            folders: suite(),
            lifts: vec![rej_hash],
            known: Default::default(),
            written_unix_ms: 9,
        };
        all.push(("rejection-lift-1.json", encode(&lift), AuthorityKind::Lift));

        let policy = policy_with(vec![rule(1), rule(2)], vec![]);
        let pol_bytes = encode(&policy);
        let pol_hash = sha256(&pol_bytes);
        all.push((
            "settings-policy-1.json",
            pol_bytes,
            AuthorityKind::SettingsPolicy,
        ));

        let revision = SettingsRevision {
            group_id: group_id(),
            issuer: issuer(1, 9),
            unit: crate::ids::SettingsUnit {
                folder: FolderKey::lossy("suite"),
                identity: crate::ids::Identity::account("ACC").unwrap(),
            },
            kind: RevisionKind::Push,
            checkpoint: sha256(b"checkpoint"),
            parents: vec![],
            parents_parts: vec![],
            policy: vec![pol_hash],
            code_package_hash: None,
            known: Default::default(),
            written_unix_ms: 9,
        };
        all.push((
            "settings-revision-1.json",
            encode(&revision),
            AuthorityKind::SettingsRevision,
        ));

        let part = Part {
            group_id: group_id(),
            of: PartOf::Known,
            items: vec![PartItem::Seen(Seen {
                enrollment_id: enrollment(2),
                seq: 4,
            })],
            more: vec![],
        };
        all.push(("authority-part-1.json", encode(&part), AuthorityKind::Part));

        for (name, bytes, kind) in &all {
            golden(name, bytes);
            let h = sha256(bytes);
            let r = decode_authority(bytes, &h).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(r.kind(), *kind, "{name}");
            let mut l = Ledger::new();
            l.learn(h, *kind, vec![], None, Some(bytes));
            assert!(matches!(l.body(&h), Some(Body::Record(_))), "{name}");
        }

        let chunk = AuthorityIndexChunk {
            group_id: group_id(),
            enrollment_id: enrollment(2),
            prev: None,
            refs: refs(0..2),
        };
        let bytes = encode(&chunk);
        golden("authority-index-1.json", &bytes);
        assert_eq!(
            decode::<AuthorityIndexChunk>(&bytes, MAX_INDEX_BYTES).unwrap(),
            chunk
        );
    }

    #[test]
    fn readd_with_unreadable_issuer_indexes_lists_every_record_in_extra_parts() {
        // T-R4D4-mirror-only-readd
        use crate::kinds::{pack_known, Known};
        let hashes: Vec<Sha256> = (0..1024u32).map(|n| sha256(&n.to_le_bytes())).collect();
        let base = encode(&readd_with(Known::default())).len();
        let (known, parts) = pack_known(&group_id(), vec![], hashes.clone(), base);
        assert!(known.seen.is_empty() && !known.parts.is_empty());
        let rd = readd_with(known.clone());
        assert!(encode(&rd).len() <= 64 * 1024);
        let mut l = Ledger::new();
        for x in &known.extra {
            l.learn(*x, AuthorityKind::Removal, vec![], None, None);
        }
        let h = learn(&mut l, &rd, AuthorityKind::Readd);
        for (ph, pb) in &parts {
            learn_part(&mut l, *ph, pb);
        }
        let d = derive(&l, &group_id(), &members(&[1]));
        assert_eq!(d.status_of(&h), Some(&Status::Valid));
        let full = resolve_known(&l, &known).unwrap();
        assert_eq!(full.extra.len(), 1024);
        let none = BTreeSet::new();
        // The issuers are unknown here; only the hash covers them.
        for x in &hashes {
            assert!(crate::authority::covered(&full, &none, &issuer(9, 99), x));
        }
        assert!(!crate::authority::covered(
            &full,
            &none,
            &issuer(9, 99),
            &sha256(b"concurrent")
        ));
    }

    fn rule(n: u32) -> crate::kinds::PolicyRule {
        crate::kinds::PolicyRule {
            folder: FolderKey::lossy(&format!("addon{n:05}")),
            rule: crate::kinds::PolicyRuleKind::NotShared,
            computer_id: None,
            enrollment_id: None,
        }
    }

    fn policy_with(
        folders: Vec<crate::kinds::PolicyRule>,
        parts: Vec<Sha256>,
    ) -> crate::kinds::SettingsPolicy {
        crate::kinds::SettingsPolicy {
            group_id: group_id(),
            issuer: issuer(1, 3),
            master: None,
            folders,
            known: Default::default(),
            folders_parts: parts,
            written_unix_ms: 9,
        }
    }

    #[test]
    fn a_policy_of_5000_folders_round_trips_in_parts_and_a_duplicate_is_policy_body() {
        // T-R4D4-full-policy
        use crate::kinds::{pack_items, PartItem};
        let rules: Vec<_> = (0..5000).map(rule).collect();
        let base = encode(&policy_with(vec![], vec![])).len();
        let packed = pack_items(
            &group_id(),
            PartOf::PolicyFolders,
            rules.iter().cloned().map(PartItem::Rule).collect(),
            base,
        );
        let inline: Vec<_> = packed
            .inline
            .iter()
            .map(|i| match i {
                PartItem::Rule(r) => r.clone(),
                _ => unreachable!(),
            })
            .collect();
        assert!(packed.parts.len() > 2 && packed.heads.len() <= 16);
        let pol = policy_with(inline, packed.heads.clone());
        assert!(encode(&pol).len() <= 64 * 1024);
        assert!(packed.parts.iter().all(|(_, b)| b.len() <= 64 * 1024));
        let mut l = Ledger::new();
        let h = learn(&mut l, &pol, AuthorityKind::SettingsPolicy);
        for (ph, pb) in &packed.parts {
            learn_part(&mut l, *ph, pb);
        }
        let m = members(&[1]);
        assert_eq!(
            derive(&l, &group_id(), &m).status_of(&h),
            Some(&Status::Valid)
        );
        assert_eq!(policy_folders(&l, &pol).unwrap(), rules);

        // The same folder in two parts.
        let part = |items: Vec<u32>| Part {
            group_id: group_id(),
            of: PartOf::PolicyFolders,
            items: items.into_iter().map(|n| PartItem::Rule(rule(n))).collect(),
            more: vec![],
        };
        let mut l = Ledger::new();
        let mut heads = Vec::new();
        for p in [part(vec![1, 7]), part(vec![7, 9])] {
            let b = encode(&p);
            let ph = sha256(&b);
            learn_part(&mut l, ph, &b);
            heads.push(ph);
        }
        heads.sort();
        let dup = policy_with(vec![], heads);
        let h = learn(&mut l, &dup, AuthorityKind::SettingsPolicy);
        assert_eq!(
            derive(&l, &group_id(), &m).status_of(&h),
            Some(&Status::Invalid(InvalidReason::Kind("policy_body".into())))
        );
    }

    fn refs(range: std::ops::Range<u32>) -> Vec<crate::kinds::AuthorityRef> {
        range
            .map(|n| crate::kinds::AuthorityRef {
                hash: sha256(&n.to_le_bytes()),
                kind: AuthorityKind::Removal,
                folders: vec![FolderKey::lossy("suite")],
            })
            .collect()
    }

    #[test]
    fn index_chain_replaces_a_partial_head_and_reports_a_broken_chain_without_forgetting() {
        // T-C12-index-chain
        let (g, e) = (group_id(), enrollment(2));
        let mut store: BTreeMap<Sha256, Vec<u8>> = BTreeMap::new();
        let put = |files: Vec<(Sha256, Vec<u8>)>, s: &mut BTreeMap<Sha256, Vec<u8>>| {
            for (h, b) in files {
                s.insert(h, b);
            }
        };

        // A partial head chunk is replaced, not extended.
        let (files, h1) = append_index_refs(&g, &e, None, 0, refs(0..3));
        put(files, &mut store);
        let old = decode::<crate::kinds::AuthorityIndexChunk>(&store[&h1.head], MIB).unwrap();
        let (files, h2) = append_index_refs(&g, &e, Some((&h1.head, &old)), 3, refs(3..5));
        assert_eq!(files.len(), 1);
        put(files, &mut store);
        assert_ne!(h2.head, h1.head);
        assert!(
            store.contains_key(&h1.head),
            "the replaced chunk stays on disk"
        );
        let new = decode::<crate::kinds::AuthorityIndexChunk>(&store[&h2.head], MIB).unwrap();
        assert_eq!((new.prev, new.refs.len(), h2.total), (old.prev, 5, 5));

        // A long chain, read whole, then incrementally.
        let (files, big) = append_index_refs(&g, &e, None, 0, refs(0..2500));
        assert_eq!(files.len(), 3);
        let mut store: BTreeMap<Sha256, Vec<u8>> = BTreeMap::new();
        put(files, &mut store);
        let fetch = |s: &BTreeMap<Sha256, Vec<u8>>, h: &Sha256| s.get(h).cloned();
        let first = read_index_chain(&big, &g, &e, &BTreeMap::new(), &|h| fetch(&store, h));
        assert_eq!((first.refs.len(), first.error.clone()), (2500, None));
        let mut l = Ledger::new();
        for r in &first.refs {
            l.learn(r.hash, r.kind, r.folders.clone(), Some(&computer(2)), None);
        }
        assert_eq!(l.len(), 2500);

        let head_chunk =
            decode::<crate::kinds::AuthorityIndexChunk>(&store[&big.head], MIB).unwrap();
        let (files, grown) = append_index_refs(
            &g,
            &e,
            Some((&big.head, &head_chunk)),
            2500,
            refs(2500..2501),
        );
        put(files, &mut store);
        let more = read_index_chain(&grown, &g, &e, &first.processed, &|h| fetch(&store, h));
        assert_eq!((more.refs.len(), more.error.clone()), (453, None));

        // A missing middle chunk: unreadable, and what was learned stays.
        let mid = head_chunk.prev.expect("a middle chunk");
        let mut broken = store.clone();
        broken.remove(&mid);
        let r = read_index_chain(&big, &g, &e, &BTreeMap::new(), &|h| fetch(&broken, h));
        assert_eq!(r.error, Some(IndexError::Missing(mid)));
        assert!(r.processed.is_empty());
        for x in &r.refs {
            l.learn(x.hash, x.kind, x.folders.clone(), Some(&computer(2)), None);
        }
        assert_eq!(l.len(), 2500);

        // A bad copy, a total that does not add up, and a foreign chunk.
        let mut bad = store.clone();
        bad.insert(mid, b"damaged".to_vec());
        let r = read_index_chain(&big, &g, &e, &BTreeMap::new(), &|h| fetch(&bad, h));
        assert_eq!(r.error, Some(IndexError::Bad(mid)));
        let off = IndexHead { total: 2499, ..big };
        let r = read_index_chain(&off, &g, &e, &BTreeMap::new(), &|h| fetch(&store, h));
        assert_eq!(
            r.error,
            Some(IndexError::TotalMismatch {
                found: 2500,
                total: 2499
            })
        );
        let r = read_index_chain(&big, &g, &enrollment(3), &BTreeMap::new(), &|h| {
            fetch(&store, h)
        });
        assert_eq!(r.error, Some(IndexError::Foreign(big.head)));
    }

    fn unit(folder: &str) -> crate::ids::SettingsUnit {
        crate::ids::SettingsUnit {
            folder: FolderKey::lossy(folder),
            identity: crate::ids::Identity::account("ACC").unwrap(),
        }
    }

    fn revision(
        by: u8,
        seq: u64,
        u: &crate::ids::SettingsUnit,
        kind: crate::kinds::RevisionKind,
        parents: Vec<Sha256>,
        checkpoint: &[u8],
    ) -> crate::kinds::SettingsRevision {
        crate::kinds::SettingsRevision {
            group_id: group_id(),
            issuer: issuer(by, seq),
            unit: u.clone(),
            kind,
            checkpoint: sha256(checkpoint),
            parents,
            parents_parts: vec![],
            policy: vec![sha256(&encode(&policy_with(vec![], vec![])))],
            code_package_hash: None,
            known: Default::default(),
            written_unix_ms: 9,
        }
    }

    fn with_policy(l: &mut Ledger) {
        learn(
            l,
            &policy_with(vec![], vec![]),
            AuthorityKind::SettingsPolicy,
        );
    }

    #[test]
    fn a_settings_revision_parent_needs_the_same_unit_and_any_kind() {
        // settings.md §3.3: only the unit must match; a push may be followed by an auto
        use crate::kinds::RevisionKind;
        use crate::settings::graph_from_ledger;
        let u = unit("suite");
        let mut l = Ledger::new();
        with_policy(&mut l);
        let push = learn(
            &mut l,
            &revision(1, 5, &u, RevisionKind::Push, vec![], b"c1"),
            AuthorityKind::SettingsRevision,
        );
        let auto = learn(
            &mut l,
            &revision(1, 6, &u, RevisionKind::Auto, vec![push], b"c2"),
            AuthorityKind::SettingsRevision,
        );
        let other = learn(
            &mut l,
            &revision(1, 7, &unit("other"), RevisionKind::Auto, vec![push], b"c3"),
            AuthorityKind::SettingsRevision,
        );
        let d = derive(&l, &group_id(), &members(&[1]));
        assert_eq!(d.status_of(&push), Some(&Status::Valid));
        assert_eq!(d.status_of(&auto), Some(&Status::Valid));
        assert_eq!(
            d.status_of(&other),
            Some(&Status::Invalid(InvalidReason::Kind(
                "revision_parent".into()
            )))
        );
        let g = graph_from_ledger(&l, &d, &u);
        assert_eq!(g.heads, vec![auto]);
        assert!(!g.pending);
        let go = graph_from_ledger(&l, &d, &unit("other"));
        assert!(go.heads.is_empty() && go.nodes.is_empty());
    }

    #[test]
    fn graph_from_ledger_keeps_equal_heads_and_marks_an_unknown_parent_pending() {
        // R4D-7 / R4D-8 at ledger level
        use crate::kinds::RevisionKind;
        use crate::settings::graph_from_ledger;
        let u = unit("suite");
        let mut l = Ledger::new();
        with_policy(&mut l);
        let a = learn(
            &mut l,
            &revision(1, 5, &u, RevisionKind::Push, vec![], b"same"),
            AuthorityKind::SettingsRevision,
        );
        let b = learn(
            &mut l,
            &revision(2, 5, &u, RevisionKind::Push, vec![], b"same"),
            AuthorityKind::SettingsRevision,
        );
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        let g = graph_from_ledger(&l, &d, &u);
        assert_eq!(g.heads.len(), 2);
        assert_eq!(g.head_states.len(), 1);
        assert!(g.nodes.contains_key(&a) && g.nodes.contains_key(&b));
        let missing = revision(
            1,
            6,
            &u,
            RevisionKind::Auto,
            vec![sha256(b"nowhere")],
            b"c9",
        );
        learn(&mut l, &missing, AuthorityKind::SettingsRevision);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        assert!(graph_from_ledger(&l, &d, &u).pending);
    }

    #[test]
    fn policy_heads_drop_a_policy_that_a_later_one_knows() {
        use crate::kinds::{Known, Seen};
        use crate::settings::policy_heads;
        let mut l = Ledger::new();
        let mut first = policy_with(vec![rule(1)], vec![]);
        first.issuer = issuer(1, 3);
        let a = learn(&mut l, &first, AuthorityKind::SettingsPolicy);
        let mut second = policy_with(vec![rule(2)], vec![]);
        second.issuer = issuer(2, 4);
        second.known = Known {
            seen: vec![Seen {
                enrollment_id: enrollment(1),
                seq: 3,
            }],
            extra: vec![],
            parts: vec![],
        };
        let b = learn(&mut l, &second, AuthorityKind::SettingsPolicy);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        let heads: Vec<Sha256> = policy_heads(&l, &d).iter().map(|p| p.hash).collect();
        assert_eq!(heads, vec![b]);
        let mut rival = policy_with(vec![rule(3)], vec![]);
        rival.issuer = issuer(1, 9);
        let c = learn(&mut l, &rival, AuthorityKind::SettingsPolicy);
        let d = derive(&l, &group_id(), &members(&[1, 2]));
        let mut heads: Vec<Sha256> = policy_heads(&l, &d).iter().map(|p| p.hash).collect();
        heads.sort();
        let mut want = vec![b, c];
        want.sort();
        assert_eq!(heads, want);
        assert!(!heads.contains(&a));
    }
}
