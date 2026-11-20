//! Removal behaviour (removal.md). Step-1 stub: the neutral hooks, handed to R4-core-removal.

use crate::decision::{AckItem, Candidate, CleanupClass, CycleCx, RemovalView};
use crate::hooks::{Neutral, RemovalHooks};
use crate::ledger::Ledger;
use crate::paths::FolderKey;
use crate::plan::PackageApply;

#[derive(Debug, Clone, Copy, Default)]
pub struct RemovalBehaviour;

impl RemovalHooks for RemovalBehaviour {
    fn merged_removals(&self, l: &Ledger, f: &FolderKey) -> RemovalView {
        Neutral.merged_removals(l, f)
    }
    fn stale_copy(&self, l: &Ledger, f: &FolderKey, c: &Candidate) -> bool {
        Neutral.stale_copy(l, f, c)
    }
    fn classify_due(&self, cx: &CycleCx) -> Vec<(AckItem, CleanupClass)> {
        Neutral.classify_due(cx)
    }
    fn plan_removals(&self, cx: &CycleCx) -> Vec<PackageApply> {
        Neutral.plan_removals(cx)
    }
}
