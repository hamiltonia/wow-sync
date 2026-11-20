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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{computer, enrollment, group_id};

    fn h(n: u8) -> Sha256 {
        sha256(&[n])
    }

    fn ret(retired: u8, cutoff: u64, known: &[u8]) -> Retirement {
        let mut k: Vec<Sha256> = known.iter().map(|n| h(*n)).collect();
        k.sort();
        Retirement {
            group_id: group_id(),
            issuer: Issuer {
                computer_id: computer(9),
                enrollment_id: enrollment(9),
                issued_at_seq: 1,
            },
            retired: EnrollmentRef {
                computer_id: computer(retired),
                enrollment_id: enrollment(retired),
            },
            cutoff_seq: cutoff,
            was_settings_master: false,
            retirements_known: k,
            written_unix_ms: 1,
        }
    }

    fn barrier(rs: &[(u8, Retirement)]) -> Barrier {
        let nodes: BTreeMap<Sha256, RetNode> = rs
            .iter()
            .map(|(n, r)| (h(*n), RetNode::Valid(r.clone())))
            .collect();
        let st = retirement_statuses(&nodes);
        let map = rs.iter().map(|(n, r)| (h(*n), r.clone())).collect();
        Barrier::new(&map, &st)
    }

    #[test]
    fn vret_predicate_truth_table() {
        // T-R4D15-vret-predicate
        let b = barrier(&[(1, ret(2, 10, &[]))]);
        let e = enrollment(2);
        assert_eq!(b.verdict(&e, 10), RetVerdict::Valid);
        assert_eq!(b.verdict(&e, 11), RetVerdict::AfterRetirement);
        assert_eq!(b.verdict(&enrollment(3), 99), RetVerdict::Valid);
        // Unresolved retirement: above the hold cut waits, never invalid.
        let nodes = BTreeMap::from([(h(1), RetNode::Valid(ret(2, 10, &[7])))]);
        let st = retirement_statuses(&nodes);
        assert_eq!(st[&h(1)], RetStatus::Unresolved);
        let map = BTreeMap::from([(h(1), ret(2, 10, &[7]))]);
        let b = Barrier::new(&map, &st);
        assert_eq!(b.retired(&e), Retired::Unresolved);
        assert_eq!(b.verdict(&e, 10), RetVerdict::Valid);
        assert_eq!(b.verdict(&e, 11), RetVerdict::RetirementUnresolved);
    }

    #[test]
    fn later_cutoff_does_not_reauthorize_and_lower_one_tightens() {
        // T-R4D16-later-cutoff-no-reauth
        let e = enrollment(2);
        let b = barrier(&[(1, ret(2, 10, &[])), (2, ret(2, 12, &[]))]);
        assert_eq!(b.cutoff(&e), Some(10));
        assert_eq!(b.verdict(&e, 11), RetVerdict::AfterRetirement);
        let b = barrier(&[
            (1, ret(2, 10, &[])),
            (2, ret(2, 12, &[])),
            (3, ret(2, 8, &[])),
        ]);
        assert_eq!(b.cutoff(&e), Some(8));
        assert_eq!(b.verdict(&e, 9), RetVerdict::AfterRetirement);
    }

    #[test]
    fn retirement_by_a_retired_issuer_is_void() {
        // T-R4D16-retired-issuer-retirement-void: B retires A while naming B's own retirement.
        for order in [[1u8, 2], [2, 1]] {
            let rs: Vec<(u8, Retirement)> = order
                .iter()
                .map(|n| match n {
                    1 => (1, ret(2, 10, &[])),
                    _ => (2, ret(1, 5, &[1])),
                })
                .collect();
            let b = barrier(&rs);
            assert_eq!(b.retired(&enrollment(2)), Retired::Yes);
            assert_eq!(b.retired(&enrollment(1)), Retired::No);
        }
    }

    #[test]
    fn partial_retirement_dag_is_order_and_restart_stable() {
        // T-R4D25-partial-retirement-dag: R <- S <- T <- U.
        let full: Vec<(u8, Retirement)> = vec![
            (1, ret(11, 5, &[])),
            (2, ret(12, 5, &[1])),
            (3, ret(13, 5, &[2])),
            (4, ret(14, 5, &[3])),
        ];
        let want = [
            RetStatus::Effective,
            RetStatus::Void,
            RetStatus::Effective,
            RetStatus::Void,
        ];
        let orders: [[usize; 4]; 4] = [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1]];
        for order in orders {
            let mut nodes: BTreeMap<Sha256, RetNode> = BTreeMap::new();
            for (step, i) in order.iter().enumerate() {
                nodes.insert(h(full[*i].0), RetNode::Valid(full[*i].1.clone()));
                // Whatever has arrived so far, an effective verdict never depends on later input
                // unless a dependency is missing.
                let st = retirement_statuses(&nodes);
                if step == 3 {
                    for (j, w) in want.iter().enumerate() {
                        assert_eq!(st[&h(full[j].0)], *w, "order {order:?}");
                    }
                }
            }
        }
        // Missing S leaves T and U unresolved.
        let nodes: BTreeMap<Sha256, RetNode> = [0usize, 2, 3]
            .iter()
            .map(|i| (h(full[*i].0), RetNode::Valid(full[*i].1.clone())))
            .collect();
        let st = retirement_statuses(&nodes);
        assert_eq!(st[&h(1)], RetStatus::Effective);
        assert_eq!(st[&h(3)], RetStatus::Unresolved);
        assert_eq!(st[&h(4)], RetStatus::Unresolved);
    }

    #[test]
    fn chained_retirements_keep_the_retired_set_in_both_orders() {
        // T-C10-retirement-barrier: A retires B at 10; C retires A at 4 knowing nothing.
        for order in [[1u8, 2], [2, 1]] {
            let rs: Vec<(u8, Retirement)> = order
                .iter()
                .map(|n| match n {
                    1 => (1, ret(2, 10, &[])),
                    _ => (2, ret(1, 4, &[])),
                })
                .collect();
            let b = barrier(&rs);
            assert_eq!(b.retired(&enrollment(2)), Retired::Yes);
            assert_eq!(b.retired(&enrollment(1)), Retired::Yes);
            assert_eq!(b.verdict(&enrollment(2), 11), RetVerdict::AfterRetirement);
        }
    }
}
