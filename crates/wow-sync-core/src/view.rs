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
    /// An interrupted update that the next sync finishes or restores; the game is not running.
    FinishOnNextSync,
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

pub fn next_action(status: &PackageStatus, _why: &Explanation, game_running: bool) -> NextAction {
    match status {
        PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_) => {
            NextAction::Nothing
        }
        PackageStatus::Available => NextAction::ApplyOnNextSync,
        PackageStatus::Blocked(PlanBlock::GameRunning) => NextAction::CloseGame,
        PackageStatus::Interrupted if game_running => NextAction::CloseGame,
        PackageStatus::Interrupted => NextAction::FinishOnNextSync,
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
            NextAction::FinishOnNextSync => {
                "Nothing to do. It will be finished or restored on the next sync."
            }
            NextAction::WaitForDropbox => {
                "Wait for Dropbox to finish downloading, or make the folder available offline."
            }
            NextAction::WaitForFiles => "Wait for the add-on files to stop changing.",
            NextAction::ChooseCopy => "Choose which copy to keep.",
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
        assert_eq!(
            next_action(&PackageStatus::InSync, &e, false),
            NextAction::Nothing
        );
        assert_eq!(
            next_action(&PackageStatus::Available, &e, false),
            NextAction::ApplyOnNextSync
        );
        assert_eq!(
            next_action(&PackageStatus::Blocked(PlanBlock::GameRunning), &e, true),
            NextAction::CloseGame
        );
        assert_eq!(
            next_action(&PackageStatus::Unavailable, &e, false),
            NextAction::WaitForDropbox
        );
        assert_eq!(
            next_action(
                &PackageStatus::NeedsDecision(DecisionReason::Ambiguous),
                &e,
                false
            ),
            NextAction::ChooseCopy
        );
        assert_eq!(
            next_action(&PackageStatus::NeedsHelp("x".into()), &e, false),
            NextAction::ResolveHelp
        );
    }

    #[test]
    fn an_interruption_asks_to_close_wow_only_while_the_game_runs() {
        let e = Explanation::Interrupted {
            package: "alpha".into(),
        };
        let running = next_action(&PackageStatus::Interrupted, &e, true);
        assert_eq!(running, NextAction::CloseGame);
        let idle = next_action(&PackageStatus::Interrupted, &e, false);
        assert!(!idle.sentence().contains("WoW"), "{}", idle.sentence());
        assert!(idle.sentence().contains("next sync"));
    }

    #[test]
    fn a_decision_is_never_described_as_nothing_to_do() {
        let e = Explanation::InSync;
        let a = next_action(
            &PackageStatus::NeedsDecision(DecisionReason::Ambiguous),
            &e,
            false,
        );
        assert!(!a.sentence().starts_with("Nothing"));
    }
}
