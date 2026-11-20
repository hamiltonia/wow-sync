//! Decision freshness (contracts.md §5.5, ops.md §1). Step-1 stub, handed to R4-core-ops.

use crate::decision::{DecideOutcome, DecisionId, EvidenceToken, FreshEvidence, OptionId};
use crate::hooks::{DecisionHooks, Neutral};

#[derive(Debug, Clone, Copy, Default)]
pub struct DecideBehaviour;

impl DecisionHooks for DecideBehaviour {
    fn decide(
        &self,
        item: &DecisionId,
        option: &OptionId,
        shown: &EvidenceToken,
        now: &FreshEvidence,
    ) -> DecideOutcome {
        Neutral.decide(item, option, shown, now)
    }
}
