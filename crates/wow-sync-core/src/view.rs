//! Typed view helpers for the GUI (UG-12): what the user can do next about one add-on. The GUI
//! shows this; it never decides it (A-25).

use crate::explain::Explanation;
use crate::plan::PlanBlock;
use crate::status::PackageStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAction {
    /// Nothing to do.
    Nothing,
    /// The update is ready and the next sync applies it.
    ApplyOnNextSync,
    /// Close the game so the update can finish.
    CloseGame,
    /// Wait for Dropbox to finish delivering the files.
    WaitForDropbox,
    /// Wait for add-on files to stop changing.
    WaitForFiles,
    /// The user must choose which copy to keep.
    ChooseCopy,
    /// An update stopped unexpectedly and needs the user.
    ResolveHelp,
    /// Not synced: nothing can be done from here.
    NotSynced,
}

pub fn next_action(status: &PackageStatus, _why: &Explanation) -> NextAction {
    match status {
        PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_) => {
            NextAction::Nothing
        }
        PackageStatus::Available => NextAction::ApplyOnNextSync,
        PackageStatus::Blocked(PlanBlock::GameRunning) | PackageStatus::Interrupted => {
            NextAction::CloseGame
        }
        PackageStatus::Blocked(PlanBlock::Unstable) => NextAction::WaitForFiles,
        PackageStatus::Blocked(_) => NextAction::Nothing,
        PackageStatus::Unavailable => NextAction::WaitForDropbox,
        PackageStatus::NeedsDecision(_) => NextAction::ChooseCopy,
        PackageStatus::NeedsHelp(_) => NextAction::ResolveHelp,
        PackageStatus::NotSynced => NextAction::NotSynced,
    }
}

impl NextAction {
    /// One plain sentence; empty when there is nothing to say.
    pub fn sentence(self) -> &'static str {
        match self {
            NextAction::Nothing => "Nothing to do.",
            NextAction::ApplyOnNextSync => "Nothing to do. It will be applied on the next sync.",
            NextAction::CloseGame => "Close WoW to let the update finish.",
            NextAction::WaitForDropbox => {
                "Wait for Dropbox to finish downloading, or make the folder available offline."
            }
            NextAction::WaitForFiles => "Wait for the add-on files to stop changing.",
            NextAction::ChooseCopy => {
                "Choose which copy to keep. Only this add-on is affected; other add-ons keep syncing."
            }
            NextAction::ResolveHelp => "See what happened and choose how to continue.",
            NextAction::NotSynced => "Nothing to do. This add-on is not synced.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::DecisionReason;

    #[test]
    fn each_status_has_a_distinct_next_action() {
        let e = Explanation::InSync;
        assert_eq!(next_action(&PackageStatus::InSync, &e), NextAction::Nothing);
        assert_eq!(
            next_action(&PackageStatus::Available, &e),
            NextAction::ApplyOnNextSync
        );
        assert_eq!(
            next_action(&PackageStatus::Blocked(PlanBlock::GameRunning), &e),
            NextAction::CloseGame
        );
        assert_eq!(
            next_action(&PackageStatus::Unavailable, &e),
            NextAction::WaitForDropbox
        );
        assert_eq!(
            next_action(&PackageStatus::NeedsDecision(DecisionReason::Ambiguous), &e),
            NextAction::ChooseCopy
        );
        assert_eq!(
            next_action(&PackageStatus::NeedsHelp("x".into()), &e),
            NextAction::ResolveHelp
        );
    }

    #[test]
    fn a_decision_is_never_described_as_nothing_to_do() {
        let e = Explanation::InSync;
        let a = next_action(&PackageStatus::NeedsDecision(DecisionReason::Ambiguous), &e);
        assert!(!a.sentence().starts_with("Nothing"));
    }
}
