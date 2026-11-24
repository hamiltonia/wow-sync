//! Extension points (contracts.md §9.4). Signatures are frozen at the step-1 merge.
//! The neutral implementations have no feature effect, so slice behaviour is preserved.

use crate::decision::{
    AckItem, Candidate, CleanupClass, Constraint, CycleCx, DecideError, DecideOutcome, DecisionId,
    EvidenceToken, FreshEvidence, OptionId, RemovalView, RetirePlan, SettingsTarget,
};
use crate::ids::SettingsUnit;
use crate::kinds::EnrollmentRef;
use crate::ledger::Ledger;
use crate::package::Package;
use crate::paths::FolderKey;
use crate::plan::PackageApply;
use crate::records::SettingsRoot;
use crate::target::{ScopeMatch, Source};

/// Filled in `removal.rs`.
pub trait RemovalHooks {
    fn merged_removals(&self, l: &Ledger, f: &FolderKey) -> RemovalView;
    fn stale_copy(&self, l: &Ledger, f: &FolderKey, c: &Candidate) -> bool;
    fn classify_due(&self, cx: &CycleCx) -> Vec<(AckItem, CleanupClass)>;
    fn plan_removals(&self, cx: &CycleCx) -> Vec<PackageApply>;
}

/// Filled in `choice.rs` (keep and resume) and `reject.rs` (rejection). Whole-source checks use
/// `same_release` (contracts.md §4.2): the source against each recorded entry over that entry's
/// scope only, never per folder.
pub trait ConstraintHooks {
    fn live_constraints(&self, l: &Ledger, f: &FolderKey) -> Vec<Constraint>;
    fn excluded(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch;
    fn rejected(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch;
    /// Live choices of different kinds that cannot both stand (a resume excluding what a keep
    /// pins): a decision, never a silent winner.
    fn choices_conflict(&self, _l: &Ledger, _p: &Package) -> bool {
        false
    }
}

/// Filled in `settings.rs`.
pub trait SettingsHooks {
    fn settings_target(&self, cx: &CycleCx, u: &SettingsUnit) -> SettingsTarget;
    fn plan_settings(&self, cx: &CycleCx) -> Vec<PackageApply>;
    fn roots_for(&self, cx: &CycleCx, folders: &[FolderKey]) -> Vec<SettingsRoot>;
}

/// Filled in `decide.rs`.
pub trait DecisionHooks {
    fn decide(
        &self,
        item: &DecisionId,
        option: &OptionId,
        shown: &EvidenceToken,
        now: &FreshEvidence,
    ) -> DecideOutcome;
}

/// Filled in `retire.rs` (command flow only; the barrier is in `authority.rs`).
pub trait RetireHooks {
    fn plan_retirement(&self, cx: &CycleCx, subject: &EnrollmentRef) -> RetirePlan;
}

/// The neutral implementation of every hook trait.
#[derive(Debug, Clone, Copy, Default)]
pub struct Neutral;

impl RemovalHooks for Neutral {
    fn merged_removals(&self, _: &Ledger, _: &FolderKey) -> RemovalView {
        RemovalView::default()
    }
    fn stale_copy(&self, _: &Ledger, _: &FolderKey, _: &Candidate) -> bool {
        false
    }
    fn classify_due(&self, _: &CycleCx) -> Vec<(AckItem, CleanupClass)> {
        Vec::new()
    }
    fn plan_removals(&self, _: &CycleCx) -> Vec<PackageApply> {
        Vec::new()
    }
}

impl ConstraintHooks for Neutral {
    fn live_constraints(&self, _: &Ledger, _: &FolderKey) -> Vec<Constraint> {
        Vec::new()
    }
    fn excluded(&self, _: &Ledger, _: &Package, _: &Source) -> ScopeMatch {
        ScopeMatch::Different
    }
    fn rejected(&self, _: &Ledger, _: &Package, _: &Source) -> ScopeMatch {
        ScopeMatch::Different
    }
}

impl SettingsHooks for Neutral {
    fn settings_target(&self, _: &CycleCx, _: &SettingsUnit) -> SettingsTarget {
        SettingsTarget::NoChange
    }
    fn plan_settings(&self, _: &CycleCx) -> Vec<PackageApply> {
        Vec::new()
    }
    fn roots_for(&self, _: &CycleCx, _: &[FolderKey]) -> Vec<SettingsRoot> {
        Vec::new()
    }
}

impl DecisionHooks for Neutral {
    fn decide(
        &self,
        _: &DecisionId,
        _: &OptionId,
        _: &EvidenceToken,
        _: &FreshEvidence,
    ) -> DecideOutcome {
        DecideOutcome::Refused(DecideError::UnknownItem)
    }
}

impl RetireHooks for Neutral {
    fn plan_retirement(&self, _: &CycleCx, _: &EnrollmentRef) -> RetirePlan {
        RetirePlan::default()
    }
}
