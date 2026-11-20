//! Code-choice behaviour (ops.md §1, contracts.md §5.3). Step-1 stub, handed to R4-core-ops.

use crate::decision::Constraint;
use crate::hooks::{ConstraintHooks, Neutral};
use crate::ledger::Ledger;
use crate::package::Package;
use crate::paths::FolderKey;
use crate::target::{ScopeMatch, Source};

/// The keep and resume half of `ConstraintHooks`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChoiceBehaviour;

impl ConstraintHooks for ChoiceBehaviour {
    fn live_constraints(&self, l: &Ledger, f: &FolderKey) -> Vec<Constraint> {
        Neutral.live_constraints(l, f)
    }
    fn excluded(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        Neutral.excluded(l, p, s)
    }
    fn rejected(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch {
        Neutral.rejected(l, p, s)
    }
}
