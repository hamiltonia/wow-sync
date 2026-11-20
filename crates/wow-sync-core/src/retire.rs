//! Retirement command flow (ops.md §2). The barrier itself is in `authority.rs`.
//!
//! A computer that is retired, or whose own retirement is held unresolved, never publishes a new
//! record (Q-60, R4D-31); the plan only pauses publishing. Retiring never reassigns the settings
//! master (T-45-4).

use crate::authority::AuthorityRecord;
use crate::authority::Retired;
use crate::decision::{CycleCx, RetirePlan};
use crate::hash::sha256;
use crate::hooks::RetireHooks;
use crate::ids::{EnrollmentId, Sha256};
use crate::kinds::{EnrollmentRef, Issuer, Retirement};
use crate::records::encode;
use std::collections::BTreeMap;

/// What the barrier says about this computer's own enrollment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnStanding {
    Active,
    /// A retirement naming this computer exists but is unresolved: held, nothing new publishes.
    Held,
    Retired,
}

pub fn own_standing(cx: &CycleCx) -> OwnStanding {
    match cx.derived.barrier.retired(&cx.own.enrollment_id) {
        Retired::No => OwnStanding::Active,
        Retired::Unresolved => OwnStanding::Held,
        Retired::Yes => OwnStanding::Retired,
    }
}

/// Facts the hook cannot read from `CycleCx`: this computer's next sequence number, the highest
/// sequence admitted per other enrollment (the cutoff of a retirement issued for it) and the
/// current settings master.
#[derive(Debug, Clone, Default)]
pub struct RetireBehaviour {
    pub own_next_seq: u64,
    pub admitted_seq: BTreeMap<EnrollmentId, u64>,
    pub master: Option<EnrollmentRef>,
}

impl RetireHooks for RetireBehaviour {
    fn plan_retirement(&self, cx: &CycleCx, subject: &EnrollmentRef) -> RetirePlan {
        if own_standing(cx) != OwnStanding::Active {
            return RetirePlan {
                pause_own_publish: true,
                ..RetirePlan::default()
            };
        }
        let is_self = subject.enrollment_id == cx.own.enrollment_id;
        let member = cx
            .members
            .contains(&subject.computer_id, &subject.enrollment_id);
        if !member || cx.derived.barrier.retired(&subject.enrollment_id) == Retired::Yes {
            return RetirePlan::default();
        }
        let cutoff_seq = if is_self {
            self.own_next_seq.saturating_sub(1)
        } else {
            self.admitted_seq
                .get(&subject.enrollment_id)
                .copied()
                .unwrap_or(0)
        };
        let mut known: Vec<Sha256> = cx
            .ledger
            .entries()
            .filter_map(|e| match cx.ledger.record(&e.hash) {
                Some(AuthorityRecord::Retirement(_)) => Some(e.hash),
                _ => None,
            })
            .collect();
        known.sort();
        known.truncate(64);
        let retirement = Retirement {
            group_id: cx.group_id.clone(),
            issuer: Issuer {
                computer_id: cx.own.computer_id.clone(),
                enrollment_id: cx.own.enrollment_id.clone(),
                issued_at_seq: self.own_next_seq,
            },
            retired: subject.clone(),
            cutoff_seq,
            was_settings_master: self.master.as_ref() == Some(subject),
            retirements_known: known,
            written_unix_ms: cx.now_unix_ms,
        };
        RetirePlan {
            retirement: Some(retirement),
            reassign_master: None,
            pause_own_publish: is_self,
        }
    }
}

/// The content hash of a retirement record as it would be written.
pub fn retirement_hash(r: &Retirement) -> Sha256 {
    sha256(&encode(r))
}
