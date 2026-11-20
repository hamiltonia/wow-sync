//! Feature data and gating for the run-004 screens (retirement, decisions, rollback, removal,
//! settings, About, diagnostics). Pure: no GPUI types, so it runs on every platform.
//!
//! The app never builds a group record itself. It renders what core returns and hands the choice
//! back as a [`GroupAction`]; `backend` is the one place that sends it to the sync service as a
//! runtime `Command`. Controls whose runtime command does not exist yet (updates, diagnostics,
//! notification choices) are shown disabled with an honest reason.
//! Some fields are filled only once the snapshot fields of R4-integrate merge, hence the allow.
#![allow(dead_code)]

use crate::world::World;
use wow_sync_core::decision::{DecisionId, DecisionItem, DecisionScope, EvidenceToken, OptionId};
use wow_sync_core::ids::{SettingsUnit, Sha256};
use wow_sync_core::kinds::{ChoiceReason, EnrollmentRef, PolicyRule};
use wow_sync_core::paths::FolderKey;
use wow_sync_core::settings::ResumeOption;
use wow_sync_core::status::{AllHoldCause, HoldScope};
use wow_sync_runtime::{Command, CommandError, CommandOutcome, RollbackRefusal};

/// Every action a feature screen can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Decide,
    Keep,
    Resume,
    Retire,
    ReEnroll,
    AllowAgain,
    Rollback,
    RestoreRemoved,
    RestoreRemovedSettings,
    SetSettingsPolicy,
    ShareSettings,
    EnableSettings,
    PauseSettings,
    ResumeSettings,
    UseSharedSettings,
    ChooseSettings,
    RestoreSettings,
    CheckUpdates,
    ExportDiagnostics,
    SaveNoticePrefs,
}

impl ActionKind {
    /// Whether the runtime has a command for it on the current base (evidence control map).
    pub fn has_runtime_command(self) -> bool {
        !matches!(
            self,
            ActionKind::CheckUpdates | ActionKind::ExportDiagnostics | ActionKind::SaveNoticePrefs
        )
    }

    /// Writes a group record, so a retired computer cannot do it (ops.md §2.4).
    pub fn publishes(self) -> bool {
        matches!(
            self,
            ActionKind::Decide
                | ActionKind::Keep
                | ActionKind::Resume
                | ActionKind::Retire
                | ActionKind::AllowAgain
                | ActionKind::Rollback
                | ActionKind::RestoreRemoved
                | ActionKind::SetSettingsPolicy
                | ActionKind::ShareSettings
                | ActionKind::ChooseSettings
        )
    }
}

/// The actions the sync thread can dispatch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    wired: Vec<ActionKind>,
}

impl Capabilities {
    pub fn with(mut self, k: ActionKind) -> Self {
        if !self.wired.contains(&k) {
            self.wired.push(k);
        }
        self
    }

    pub fn has(&self, k: ActionKind) -> bool {
        self.wired.contains(&k)
    }

    /// Everything `backend` can send to the service today.
    pub fn runtime() -> Self {
        use ActionKind::*;
        [
            Decide,
            Keep,
            Resume,
            Retire,
            ReEnroll,
            AllowAgain,
            Rollback,
            RestoreRemoved,
            RestoreRemovedSettings,
            SetSettingsPolicy,
            ShareSettings,
            EnableSettings,
            PauseSettings,
            ResumeSettings,
            UseSharedSettings,
            ChooseSettings,
            RestoreSettings,
        ]
        .into_iter()
        .fold(Self::default(), Self::with)
    }
}

/// A request the GUI hands to the sync thread; one-to-one with the runtime's `Command`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupAction {
    Decide {
        item: DecisionId,
        option: OptionId,
        shown: EvidenceToken,
    },
    Retire {
        subject: EnrollmentRef,
    },
    ReEnroll,
    AllowAgain {
        rejections: Vec<Sha256>,
    },
    RestoreRemoved {
        removal: Sha256,
    },
    RestoreRemovedSettings {
        removal: Sha256,
        unit: SettingsUnit,
    },
    Keep {
        scope: Vec<FolderKey>,
        copy: Sha256,
        why: ChoiceReason,
    },
    Resume {
        scope: Vec<FolderKey>,
        allow_again: Vec<Sha256>,
        why: ChoiceReason,
    },
    Rollback {
        package: FolderKey,
        recovered: Sha256,
    },
    SetSettingsPolicy {
        master: Option<EnrollmentRef>,
        exceptions: Vec<PolicyRule>,
    },
    ShareSettings {
        unit: SettingsUnit,
        checkpoint: Sha256,
    },
    EnableSettings {
        included_accounts: Vec<String>,
    },
    PauseSettings {
        folder: FolderKey,
    },
    ResumeSettings {
        folder: FolderKey,
        choice: ResumeOption,
    },
    UseSharedSettings {
        unit: SettingsUnit,
    },
    ChooseSettings {
        unit: SettingsUnit,
        checkpoint: Sha256,
        shown_heads: Vec<Sha256>,
    },
    RestoreSettings {
        unit: SettingsUnit,
        checkpoint: Sha256,
    },
}

impl GroupAction {
    pub fn kind(&self) -> ActionKind {
        match self {
            GroupAction::Decide { .. } => ActionKind::Decide,
            GroupAction::Retire { .. } => ActionKind::Retire,
            GroupAction::ReEnroll => ActionKind::ReEnroll,
            GroupAction::AllowAgain { .. } => ActionKind::AllowAgain,
            GroupAction::RestoreRemoved { .. } => ActionKind::RestoreRemoved,
            GroupAction::RestoreRemovedSettings { .. } => ActionKind::RestoreRemovedSettings,
            GroupAction::Keep { .. } => ActionKind::Keep,
            GroupAction::Resume { .. } => ActionKind::Resume,
            GroupAction::Rollback { .. } => ActionKind::Rollback,
            GroupAction::SetSettingsPolicy { .. } => ActionKind::SetSettingsPolicy,
            GroupAction::ShareSettings { .. } => ActionKind::ShareSettings,
            GroupAction::EnableSettings { .. } => ActionKind::EnableSettings,
            GroupAction::PauseSettings { .. } => ActionKind::PauseSettings,
            GroupAction::ResumeSettings { .. } => ActionKind::ResumeSettings,
            GroupAction::UseSharedSettings { .. } => ActionKind::UseSharedSettings,
            GroupAction::ChooseSettings { .. } => ActionKind::ChooseSettings,
            GroupAction::RestoreSettings { .. } => ActionKind::RestoreSettings,
        }
    }

    /// The runtime command carrying exactly the typed fields the screen chose.
    pub fn to_command(&self) -> Command {
        match self.clone() {
            GroupAction::Decide {
                item,
                option,
                shown,
            } => Command::Decide {
                item,
                option,
                shown,
            },
            GroupAction::Retire { subject } => Command::Retire { subject },
            GroupAction::ReEnroll => Command::ReEnroll,
            GroupAction::AllowAgain { rejections } => Command::AllowAgain { rejections },
            GroupAction::RestoreRemoved { removal } => Command::RestoreRemoved { removal },
            GroupAction::RestoreRemovedSettings { removal, unit } => {
                Command::RestoreRemovedSettings { removal, unit }
            }
            GroupAction::Keep { scope, copy, why } => Command::Keep { scope, copy, why },
            GroupAction::Resume {
                scope,
                allow_again,
                why,
            } => Command::Resume {
                scope,
                allow_again,
                why,
            },
            GroupAction::Rollback { package, recovered } => {
                Command::Rollback { package, recovered }
            }
            GroupAction::SetSettingsPolicy { master, exceptions } => {
                Command::SetSettingsPolicy { master, exceptions }
            }
            GroupAction::ShareSettings { unit, checkpoint } => {
                Command::ShareSettings { unit, checkpoint }
            }
            GroupAction::EnableSettings { included_accounts } => {
                Command::EnableSettings { included_accounts }
            }
            GroupAction::PauseSettings { folder } => Command::PauseSettings { folder },
            GroupAction::ResumeSettings { folder, choice } => {
                Command::ResumeSettings { folder, choice }
            }
            GroupAction::UseSharedSettings { unit } => Command::UseSharedSettings { unit },
            GroupAction::ChooseSettings {
                unit,
                checkpoint,
                shown_heads,
            } => Command::ChooseSettings {
                unit,
                checkpoint,
                shown_heads,
            },
            GroupAction::RestoreSettings { unit, checkpoint } => {
                Command::RestoreSettings { unit, checkpoint }
            }
        }
    }
}

/// What came back from the sync thread, in the app's own terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionResult {
    /// A group record was written.
    Written,
    /// Valid and reserved: written when publishing resumes (paused or held).
    Kept,
    /// Only this computer changed.
    Local,
    /// The evidence changed since it was shown; nothing was written.
    Changed,
    /// Nothing was queued: the rollback cannot be planned from what is live.
    NotAvailable,
    /// This computer is retired or held and cannot publish.
    NotAllowed,
    /// The command had no effect to make.
    NothingToDo,
    /// This build cannot send the action (no runtime command or not wired).
    NotWired,
    Failed(String),
}

impl ActionResult {
    pub fn from_outcome(r: Result<CommandOutcome, CommandError>) -> Self {
        match r {
            Ok(CommandOutcome::Written(_) | CommandOutcome::ReEnrolled) => Self::Written,
            Ok(CommandOutcome::Kept) => Self::Kept,
            Ok(CommandOutcome::Local(_) | CommandOutcome::Recorded) => Self::Local,
            Ok(CommandOutcome::NothingToDo) => Self::NothingToDo,
            Err(CommandError::Decide(wow_sync_core::decision::DecideError::Changed)) => {
                Self::Changed
            }
            Err(CommandError::Settings(wow_sync_core::settings::SettingsWriteError::Changed)) => {
                Self::Changed
            }
            Err(CommandError::NotAllowed) => Self::NotAllowed,
            Err(CommandError::Rollback(RollbackRefusal::NotAvailable)) => Self::NotAvailable,
            Err(CommandError::Rollback(RollbackRefusal::SameRelease)) => {
                Self::Failed("that release is already installed".into())
            }
            Err(CommandError::Rollback(RollbackRefusal::NotInstalled)) => {
                Self::Failed("nothing of that add-on is installed here".into())
            }
            Err(CommandError::Rollback(RollbackRefusal::UnknownRelease(_))) => {
                Self::Failed("that release is no longer held".into())
            }
            Err(CommandError::Decide(_)) => Self::Failed("that choice is no longer offered".into()),
            Err(CommandError::UnknownRejection(_)) => {
                Self::Failed("that exclusion is not active any more".into())
            }
            Err(CommandError::EmptySelection) => Self::Failed("nothing was selected".into()),
            Err(CommandError::UnknownCopy(_)) => {
                Self::Failed("that copy is no longer held by the group".into())
            }
            Err(CommandError::UnknownRemoval(_)) => {
                Self::Failed("that removal is not in the group history".into())
            }
            Err(CommandError::UnknownCheckpoint(_)) => {
                Self::Failed("that settings copy is not available".into())
            }
            Err(CommandError::Settings(_)) => {
                Self::Failed("the settings choice is not valid now".into())
            }
            Err(CommandError::Write(_)) => Self::Failed("the choice could not be written".into()),
            Err(CommandError::UnknownFolder(_)) => {
                Self::Failed("that add-on is not known here".into())
            }
            Err(CommandError::NoSharedSettings(_)) => {
                Self::Failed("there is no single shared copy of those settings to use".into())
            }
            Err(CommandError::UseShareSettings) => {
                Self::Failed("resuming never shares your settings; use Share my settings".into())
            }
            Err(CommandError::Service(_)) => {
                Self::Failed("the sync service could not do that".into())
            }
        }
    }
}

/// The sentence shown after an action (decision flows: ops.md §1.3, rollback.md §1).
pub fn result_text(kind: ActionKind, r: &ActionResult) -> String {
    match r {
        ActionResult::Written => match kind {
            ActionKind::Retire => "Retired. It is shared with the group.".into(),
            ActionKind::ReEnroll => "This computer joined the group again.".into(),
            ActionKind::Rollback => {
                "Going back is queued. It is applied when WoW is closed.".into()
            }
            ActionKind::RestoreRemoved => "Restored for every computer in the group.".into(),
            ActionKind::AllowAgain => "Allowed again for every computer in the group.".into(),
            _ => "Saved and shared with the group.".into(),
        },
        ActionResult::Kept => "Saved. It is shared when you resume syncing.".into(),
        ActionResult::Local => "Saved on this computer.".into(),
        ActionResult::Changed => {
            "Something changed while you were choosing. Nothing was saved. Look at the new choices and choose again."
                .into()
        }
        ActionResult::NotAvailable => {
            "That release can't be installed again because its files are no longer available. Nothing was changed."
                .into()
        }
        ActionResult::NotAllowed => RETIRED_REASON.into(),
        ActionResult::NothingToDo => "Nothing needed to change.".into(),
        ActionResult::NotWired => NOT_WIRED.into(),
        ActionResult::Failed(e) => format!("That didn't work: {e}. Nothing was changed."),
    }
}

pub const RETIRED_REASON: &str =
    "This computer was retired. Join the group again to make choices for it.";
pub const NOT_WIRED: &str = "This version of WoW Sync can't send that to the sync service yet.";
pub const NEEDS_UPDATE: &str = "This needs a newer WoW Sync. It is not built into this version.";

/// Whether this computer is still a member that may publish (ops.md §2.4).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Standing {
    #[default]
    Active,
    /// `on` is the local-clock date text, `by` the retiring computer's name.
    Retired { on: String, by: String },
    /// Its own retirement is unresolved: nothing is proven, so no date or "by" line.
    Held,
}

/// A computer that was retired, for the Computers "Retired" group (ops.md §2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetiredComputer {
    pub name: String,
    pub on: String,
    pub by: String,
}

/// Everything the feature screens need that the status snapshot does not carry yet.
#[derive(Clone, Debug)]
pub struct FeatureData {
    pub caps: Capabilities,
    pub standing: Standing,
    pub hold: HoldScope,
    pub decisions: Vec<DecisionItem>,
    /// Peer computer id (hex) to the enrollment a retirement would name.
    pub enrollments: Vec<(String, EnrollmentRef)>,
    pub retired: Vec<RetiredComputer>,
    /// Rejected releases the owner may allow again. Pending snapshot field (R4-integrate).
    pub allow_again: Vec<crate::computers_ui::AllowAgainRow>,
    /// Held releases per add-on for the history list. Pending snapshot field (R4-integrate).
    pub history: Vec<crate::rollback_ui::ReleaseRow>,
    /// Name of a live member whose WoW Sync can't block versions (Q-84), which blocks go-back.
    pub older_peer: Option<String>,
    /// Name of the settings master, when one is chosen.
    pub master: Option<String>,
    /// Settings units, policy and accounts. Pending snapshot field (R4-integrate).
    pub settings: crate::settings_ui::SettingsData,
}

impl Default for FeatureData {
    fn default() -> Self {
        FeatureData {
            caps: Capabilities::default(),
            standing: Standing::Active,
            hold: HoldScope::None,
            decisions: Vec::new(),
            enrollments: Vec::new(),
            retired: Vec::new(),
            allow_again: Vec::new(),
            history: Vec::new(),
            older_peer: None,
            master: None,
            settings: Default::default(),
        }
    }
}

/// A disabled control carries its reason; an enabled one is `Ok`.
pub type Gate = Result<(), String>;

/// Whether `kind` can be used now. Order: standing (a retired computer never publishes), then
/// whether the runtime has the command, then whether this build is wired to it.
pub fn gate(w: &World, kind: ActionKind) -> Gate {
    if kind.publishes() && matches!(w.features.standing, Standing::Retired { .. }) {
        return Err(RETIRED_REASON.into());
    }
    if !kind.has_runtime_command() {
        return Err(NEEDS_UPDATE.into());
    }
    if !w.features.caps.has(kind) {
        return Err(NOT_WIRED.into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Hold scope sentences (UQ-2, ops.md §1.6)

fn scope_title(w: &World, s: &DecisionScope) -> String {
    match s {
        DecisionScope::Folders(keys) => keys
            .iter()
            .find_map(|k| w.addon(k.as_str()).map(|a| a.title.clone()))
            .or_else(|| keys.first().map(|k| k.as_str().to_string()))
            .unwrap_or_else(|| "An add-on".into()),
        DecisionScope::Settings(u) => format!("{} settings", u.folder.as_str()),
    }
}

/// The sentence for how much is held; `None` when nothing is held (no scope sentence then).
pub fn hold_sentence(w: &World) -> Option<String> {
    match &w.features.hold {
        HoldScope::None => None,
        HoldScope::Addons(a) if a.len() == 1 => Some(format!(
            "Only {} is waiting. Everything else keeps syncing.",
            scope_title(w, &a[0])
        )),
        HoldScope::Addons(a) => Some(format!(
            "Only these {} add-ons are waiting. Everything else keeps syncing.",
            a.len()
        )),
        HoldScope::AllApplies(AllHoldCause::UnresolvedJournal { .. }) => {
            Some("Nothing is updated on this computer until this is resolved.".into())
        }
        HoldScope::AllApplies(AllHoldCause::GameRunning) => {
            Some("Nothing is updated on this computer while WoW is running.".into())
        }
        HoldScope::AllApplies(AllHoldCause::ActivityUnknown) => Some(
            "Nothing is updated on this computer until WoW Sync can tell whether WoW is running."
                .into(),
        ),
        HoldScope::AllApplies(AllHoldCause::Paused) => {
            Some("Nothing is updated on this computer while syncing is paused.".into())
        }
        HoldScope::AllApplies(AllHoldCause::Retired) => {
            Some("Nothing is updated on this computer until it joins the group again.".into())
        }
        HoldScope::AllApplies(AllHoldCause::RetirementUnresolved) => {
            Some("Nothing is updated on this computer until the group history is complete.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_core::paths::validate_folder_name;

    fn w() -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w
    }

    fn key(n: &str) -> FolderKey {
        validate_folder_name(n).unwrap()
    }

    #[test]
    fn nothing_is_wired_on_this_base_and_each_control_says_why() {
        let w = w();
        let e = gate(&w, ActionKind::Decide).unwrap_err();
        assert_eq!(e, NOT_WIRED);
        let e = gate(&w, ActionKind::CheckUpdates).unwrap_err();
        assert_eq!(e, NEEDS_UPDATE);
    }

    #[test]
    fn a_wired_action_is_enabled_and_a_retired_computer_cannot_publish() {
        let mut w = w();
        w.features.caps = Capabilities::default()
            .with(ActionKind::Decide)
            .with(ActionKind::ReEnroll);
        assert!(gate(&w, ActionKind::Decide).is_ok());
        w.features.standing = Standing::Retired {
            on: "12 Jan".into(),
            by: "Laptop".into(),
        };
        assert_eq!(gate(&w, ActionKind::Decide).unwrap_err(), RETIRED_REASON);
        assert_eq!(gate(&w, ActionKind::Retire).unwrap_err(), RETIRED_REASON);
        assert!(
            gate(&w, ActionKind::ReEnroll).is_ok(),
            "Join again is the one group action a retired computer has"
        );
    }

    #[test]
    fn a_held_computer_may_still_choose_and_the_choice_is_kept() {
        let mut w = w();
        w.features.caps = Capabilities::default().with(ActionKind::Decide);
        w.features.standing = Standing::Held;
        assert!(gate(&w, ActionKind::Decide).is_ok());
        assert_eq!(
            result_text(ActionKind::Decide, &ActionResult::Kept),
            "Saved. It is shared when you resume syncing."
        );
    }

    #[test]
    fn hold_sentences_follow_the_typed_scope() {
        let mut w = w();
        assert_eq!(
            hold_sentence(&w),
            None,
            "no scope sentence when nothing is held"
        );
        w.features.hold = HoldScope::Addons(vec![DecisionScope::Folders(vec![key("weakauras")])]);
        assert_eq!(
            hold_sentence(&w).unwrap(),
            "Only weakauras is waiting. Everything else keeps syncing."
        );
        w.features.hold = HoldScope::Addons(vec![
            DecisionScope::Folders(vec![key("a")]),
            DecisionScope::Folders(vec![key("b")]),
            DecisionScope::Folders(vec![key("c")]),
        ]);
        assert!(hold_sentence(&w)
            .unwrap()
            .starts_with("Only these 3 add-ons"));
        w.features.hold = HoldScope::AllApplies(AllHoldCause::UnresolvedJournal {
            folders: vec![key("a")],
        });
        assert_eq!(
            hold_sentence(&w).unwrap(),
            "Nothing is updated on this computer until this is resolved."
        );
    }

    #[test]
    fn runtime_outcomes_and_refusals_map_to_distinct_results() {
        use wow_sync_core::decision::DecideError;
        let r = |e| ActionResult::from_outcome(Err(e));
        assert_eq!(
            r(CommandError::Decide(DecideError::Changed)),
            ActionResult::Changed
        );
        assert_eq!(r(CommandError::NotAllowed), ActionResult::NotAllowed);
        assert_eq!(
            r(CommandError::Rollback(RollbackRefusal::NotAvailable)),
            ActionResult::NotAvailable
        );
        assert!(matches!(
            r(CommandError::Rollback(RollbackRefusal::SameRelease)),
            ActionResult::Failed(_)
        ));
        assert_eq!(
            ActionResult::from_outcome(Ok(CommandOutcome::Kept)),
            ActionResult::Kept
        );
        assert_eq!(
            ActionResult::from_outcome(Ok(CommandOutcome::NothingToDo)),
            ActionResult::NothingToDo
        );
    }

    #[test]
    fn an_action_becomes_the_command_with_the_same_typed_fields() {
        let a = GroupAction::Rollback {
            package: wow_sync_core::paths::validate_folder_name("Bagnon").unwrap(),
            recovered: Sha256([9; 32]),
        };
        assert_eq!(
            a.to_command(),
            Command::Rollback {
                package: wow_sync_core::paths::validate_folder_name("Bagnon").unwrap(),
                recovered: Sha256([9; 32]),
            }
        );
        assert_eq!(a.kind(), ActionKind::Rollback);
        assert!(Capabilities::runtime().has(ActionKind::Rollback));
        assert!(!Capabilities::runtime().has(ActionKind::CheckUpdates));
    }

    #[test]
    fn every_result_has_its_own_plain_sentence() {
        let all = [
            ActionResult::Written,
            ActionResult::Kept,
            ActionResult::Local,
            ActionResult::Changed,
            ActionResult::NotAvailable,
            ActionResult::NotAllowed,
            ActionResult::NothingToDo,
            ActionResult::NotWired,
            ActionResult::Failed("disk".into()),
        ];
        let texts: Vec<String> = all
            .iter()
            .map(|r| result_text(ActionKind::Decide, r))
            .collect();
        for (i, a) in texts.iter().enumerate() {
            assert!(!a.is_empty());
            assert!(!a.contains("Changed") && !a.contains("NotAllowed"));
            for b in &texts[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
