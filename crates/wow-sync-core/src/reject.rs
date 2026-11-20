//! Rejection and lift behaviour (rollback.md). Step-1 stub, handed to R4-core-settings.

use crate::decision::Constraint;
use crate::hooks::{ConstraintHooks, Neutral};
use crate::ledger::Ledger;
use crate::package::Package;
use crate::paths::FolderKey;
use crate::target::{ScopeMatch, Source};

/// The rejection half of `ConstraintHooks`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RejectBehaviour;

impl ConstraintHooks for RejectBehaviour {
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
