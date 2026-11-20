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
#[derive(Debug, Clone, Default)]
pub struct Membership {
    pairs: BTreeSet<EnrollmentRef>,
}

impl Membership {
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
                    Some(AuthorityRecord::SettingsRevision(p))
                        if p.unit == v.unit && p.kind == v.kind => {}
                    _ => return Err("revision_parent".into()),
                }
            }
        }
        AuthorityRecord::SettingsPolicy(p) => {
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

/// Reads a part chain: all items of the given `of`, following `more`. A part whose group or `of`
/// differs, or a duplicate item across parts, makes the whole read fail (`bad_part`).
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
            continue;
        }
        if depth > crate::kinds::MAX_PART_DEPTH {
            return Err("bad_part");
        }
        let Some(AuthorityRecord::Part(p)) = ledger.record(&h) else {
            return Err("missing_part");
        };
        let p: &Part = p;
        if p.of != of {
            return Err("bad_part");
        }
        out.extend(p.items.iter().cloned());
        stack.extend(p.more.iter().map(|m| (*m, depth + 1)));
    }
    out.sort();
    Ok(out)
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
        let n = serde_json::to_string_pretty(e).map_or(512, |s| s.len() + 4);
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
