//! Authority records as a closed set: decoding, the retirement barrier and known-coverage
//! (contracts.md §1). Pure: bytes in, decisions out.

use crate::hash::sha256;
use crate::ids::{EnrollmentId, Sha256};
use crate::kinds::*;
use crate::records::{decode, Record, RecordError};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityRecord {
    Removal(Removal),
    Readd(Readd),
    Ack(Ack),
    CodeChoice(CodeChoice),
    Rejection(Rejection),
    Lift(Lift),
    SettingsPolicy(SettingsPolicy),
    SettingsRevision(SettingsRevision),
    Retirement(Retirement),
    Part(Part),
}

impl AuthorityRecord {
    pub fn kind(&self) -> AuthorityKind {
        match self {
            Self::Removal(_) => AuthorityKind::Removal,
            Self::Readd(_) => AuthorityKind::Readd,
            Self::Ack(_) => AuthorityKind::Ack,
            Self::CodeChoice(_) => AuthorityKind::CodeChoice,
            Self::Rejection(_) => AuthorityKind::Rejection,
            Self::Lift(_) => AuthorityKind::Lift,
            Self::SettingsPolicy(_) => AuthorityKind::SettingsPolicy,
            Self::SettingsRevision(_) => AuthorityKind::SettingsRevision,
            Self::Retirement(_) => AuthorityKind::Retirement,
            Self::Part(_) => AuthorityKind::Part,
        }
    }

    pub fn issuer(&self) -> Option<&Issuer> {
        Some(match self {
            Self::Removal(r) => &r.issuer,
            Self::Readd(r) => &r.issuer,
            Self::Ack(r) => &r.issuer,
            Self::CodeChoice(r) => &r.issuer,
            Self::Rejection(r) => &r.issuer,
            Self::Lift(r) => &r.issuer,
            Self::SettingsPolicy(r) => &r.issuer,
            Self::SettingsRevision(r) => &r.issuer,
            Self::Retirement(r) => &r.issuer,
            Self::Part(_) => return None,
        })
    }

    pub fn group_id(&self) -> &crate::ids::GroupId {
        match self {
            Self::Removal(r) => &r.group_id,
            Self::Readd(r) => &r.group_id,
            Self::Ack(r) => &r.group_id,
            Self::CodeChoice(r) => &r.group_id,
            Self::Rejection(r) => &r.group_id,
            Self::Lift(r) => &r.group_id,
            Self::SettingsPolicy(r) => &r.group_id,
            Self::SettingsRevision(r) => &r.group_id,
            Self::Retirement(r) => &r.group_id,
            Self::Part(r) => &r.group_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    TooLarge,
    /// The bytes do not hash to the name they were stored under.
    BadCopy,
    UnknownSchema(String),
    Invalid(String),
}

fn schema_of(bytes: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let s = v.get("schema")?.as_str()?;
    Some(s.rsplit_once('/').map_or(s, |(n, _)| n).to_string())
}

fn map_err(e: RecordError) -> AuthorityError {
    match e {
        RecordError::TooLarge => AuthorityError::TooLarge,
        RecordError::UnknownSchema(s) => AuthorityError::UnknownSchema(s),
        RecordError::UnsupportedMajor(m) => AuthorityError::UnknownSchema(format!("major {m}")),
        RecordError::SchemaMissing => AuthorityError::UnknownSchema(String::new()),
        RecordError::Json(e) | RecordError::Invalid(e) => AuthorityError::Invalid(e),
    }
}

/// Decodes stored authority bytes. The record identity is the hash of the exact bytes, so
/// a copy that does not match its name is a bad copy, never a record (contracts.md §1.1).
pub fn decode_authority(
    bytes: &[u8],
    name_hash: &Sha256,
) -> Result<AuthorityRecord, AuthorityError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(AuthorityError::TooLarge);
    }
    if &sha256(bytes) != name_hash {
        return Err(AuthorityError::BadCopy);
    }
    let name = schema_of(bytes).ok_or_else(|| AuthorityError::UnknownSchema(String::new()))?;
    macro_rules! go {
        ($t:ident) => {
            decode::<$t>(bytes, <$t as Record>::LIMIT)
                .map(AuthorityRecord::$t)
                .map_err(map_err)
        };
    }
    match name.as_str() {
        "wowsync.removal" => go!(Removal),
        "wowsync.readd" => go!(Readd),
        "wowsync.removal-ack" => go!(Ack),
        "wowsync.code-choice" => go!(CodeChoice),
        "wowsync.rejection" => go!(Rejection),
        "wowsync.rejection-lift" => go!(Lift),
        "wowsync.settings-policy" => go!(SettingsPolicy),
        "wowsync.settings-revision" => go!(SettingsRevision),
        "wowsync.retirement" => go!(Retirement),
        "wowsync.authority-part" => go!(Part),
        other => Err(AuthorityError::UnknownSchema(other.to_string())),
    }
}

// ---------------------------------------------------------------- known coverage

/// Whether a record is covered by a `Known` summary (contracts.md §1.8).
pub fn covered(known: &Known, parts: &BTreeSet<Sha256>, issuer: &Issuer, hash: &Sha256) -> bool {
    known
        .seen
        .iter()
        .any(|s| s.enrollment_id == issuer.enrollment_id && s.seq >= issuer.issued_at_seq)
        || known.extra.contains(hash)
        || parts.contains(hash)
}

// ---------------------------------------------------------------- retirement barrier

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetStatus {
    Void,
    Effective,
    Unresolved,
}

/// What the ledger knows about a retirement another retirement depends on.
#[derive(Debug, Clone)]
pub enum RetNode {
    Valid(Retirement),
    /// Settled and not effective: invalid for any reason other than membership.
    Invalid,
    /// Missing, pending, or not (yet) a member: unresolved.
    Unresolved,
}

/// The status of each retirement over its `retirements_known` DAG (contracts.md §1.10).
pub fn retirement_statuses(nodes: &BTreeMap<Sha256, RetNode>) -> BTreeMap<Sha256, RetStatus> {
    fn go(
        h: &Sha256,
        nodes: &BTreeMap<Sha256, RetNode>,
        memo: &mut BTreeMap<Sha256, RetStatus>,
        visiting: &mut BTreeSet<Sha256>,
    ) -> RetStatus {
        if let Some(s) = memo.get(h) {
            return *s;
        }
        let status = match nodes.get(h) {
            None | Some(RetNode::Unresolved) => RetStatus::Unresolved,
            Some(RetNode::Invalid) => RetStatus::Void,
            Some(RetNode::Valid(r)) => {
                if !visiting.insert(*h) {
                    return RetStatus::Unresolved;
                }
                let deps: Vec<RetStatus> = r
                    .retirements_known
                    .iter()
                    .map(|d| go(d, nodes, memo, visiting))
                    .collect();
                visiting.remove(h);
                if deps.contains(&RetStatus::Effective) {
                    RetStatus::Void
                } else if deps.iter().all(|d| *d == RetStatus::Void) {
                    RetStatus::Effective
                } else {
                    RetStatus::Unresolved
                }
            }
        };
        memo.insert(*h, status);
        status
    }
    let mut memo = BTreeMap::new();
    for h in nodes.keys() {
        go(h, nodes, &mut memo, &mut BTreeSet::new());
    }
    memo
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retired {
    Yes,
    Unresolved,
    No,
}

/// V-RET verdict for one ordinary record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetVerdict {
    Valid,
    AfterRetirement,
    RetirementUnresolved,
}

#[derive(Debug, Clone, Default)]
pub struct Barrier {
    by_enrollment: BTreeMap<EnrollmentId, Vec<(RetStatus, u64)>>,
}

impl Barrier {
    pub fn new(
        retirements: &BTreeMap<Sha256, Retirement>,
        statuses: &BTreeMap<Sha256, RetStatus>,
    ) -> Self {
        let mut by_enrollment: BTreeMap<EnrollmentId, Vec<(RetStatus, u64)>> = BTreeMap::new();
        for (h, r) in retirements {
            let s = statuses.get(h).copied().unwrap_or(RetStatus::Unresolved);
            if s != RetStatus::Void {
                by_enrollment
                    .entry(r.retired.enrollment_id.clone())
                    .or_default()
                    .push((s, r.cutoff_seq));
            }
        }
        Self { by_enrollment }
    }

    pub fn retired(&self, e: &EnrollmentId) -> Retired {
        let v = self.by_enrollment.get(e).map(Vec::as_slice).unwrap_or(&[]);
        if v.iter().any(|(s, _)| *s == RetStatus::Effective) {
            Retired::Yes
        } else if v.is_empty() {
            Retired::No
        } else {
            Retired::Unresolved
        }
    }

    pub fn cutoff(&self, e: &EnrollmentId) -> Option<u64> {
        self.by_enrollment
            .get(e)?
            .iter()
            .filter(|(s, _)| *s == RetStatus::Effective)
            .map(|(_, c)| *c)
            .min()
    }

    pub fn hold_cut(&self, e: &EnrollmentId) -> Option<u64> {
        self.by_enrollment.get(e)?.iter().map(|(_, c)| *c).min()
    }

    pub fn verdict(&self, e: &EnrollmentId, seq: u64) -> RetVerdict {
        match self.retired(e) {
            Retired::No => RetVerdict::Valid,
            Retired::Yes => {
                if self.cutoff(e).is_some_and(|c| seq <= c) {
                    RetVerdict::Valid
                } else {
                    RetVerdict::AfterRetirement
                }
            }
            Retired::Unresolved => {
                if self.hold_cut(e).is_some_and(|c| seq <= c) {
                    RetVerdict::Valid
                } else {
                    RetVerdict::RetirementUnresolved
                }
            }
        }
    }
}
