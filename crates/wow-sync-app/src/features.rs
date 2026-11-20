//! Feature data and gating for the run-004 screens (retirement, decisions, rollback, removal,
//! settings, About, diagnostics). Pure: no GPUI types, so it runs on every platform.
//!
//! The app never builds a group record itself. It renders what core returns and hands the choice
//! back as a [`GroupAction`]; `backend` is the one place that sends it to the sync service as a
//! runtime `Command`. Controls whose runtime command does not exist yet (updates, diagnostics,
//! notification choices) are shown disabled with an honest reason.

use crate::world::World;
use std::path::PathBuf;
use wow_sync_core::decision::{DecisionId, DecisionItem, DecisionScope, EvidenceToken, OptionId};
use wow_sync_core::diagnostics::{ExportOptions, REFUSED_TEXT, SHARED_FOLDER_WARNING};
use wow_sync_core::ids::{SettingsUnit, Sha256};
use wow_sync_core::kinds::{ChoiceReason, EnrollmentRef, PolicyRule};
use wow_sync_core::paths::FolderKey;
use wow_sync_core::settings::ResumeOption;
use wow_sync_core::status::{AllHoldCause, HoldScope};
use wow_sync_runtime::{
    Command, CommandError, CommandOutcome, ExportError, RollbackRefusal, SaveError,
};

/// Every action a feature screen can ask for.
// Some variants wait for items 7-8 or have no screen yet (Keep, Resume, NotWired).
#[allow(dead_code)]
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
    SaveDiagnostics,
}

impl ActionKind {
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
            CheckUpdates,
            ExportDiagnostics,
            SaveDiagnostics,
        ]
        .into_iter()
        .fold(Self::default(), Self::with)
    }
}

/// A request the GUI hands to the sync thread; one-to-one with the runtime's `Command`.
#[allow(dead_code)]
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
    CheckUpdates,
    /// Builds the preview text; nothing is written.
    ExportDiagnostics {
        options: ExportOptions,
    },
    SaveDiagnostics {
        options: ExportOptions,
        dest: PathBuf,
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
            GroupAction::CheckUpdates => ActionKind::CheckUpdates,
            GroupAction::ExportDiagnostics { .. } => ActionKind::ExportDiagnostics,
            GroupAction::SaveDiagnostics { .. } => ActionKind::SaveDiagnostics,
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
            GroupAction::CheckUpdates => Command::CheckUpdates,
            GroupAction::ExportDiagnostics { options } => Command::ExportDiagnostics { options },
            GroupAction::SaveDiagnostics { options, dest } => {
                Command::SaveDiagnostics { options, dest }
            }
        }
    }
}

/// What came back from the sync thread, in the app's own terms.
#[allow(dead_code)]
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
    /// An update check was started or answered from the last result.
    UpdateStarted,
    /// The diagnostics text, with the "Replaced: …" line; saved or only previewed by kind.
    Exported {
        text: String,
        summary: String,
    },
    /// The export or save was refused; the text says why in plain words.
    ExportRefused(String),
    Failed(String),
}

impl ActionResult {
    pub fn from_outcome(r: Result<CommandOutcome, CommandError>) -> Self {
        match r {
            Ok(CommandOutcome::Written(_) | CommandOutcome::ReEnrolled) => Self::Written,
            Ok(CommandOutcome::Kept) => Self::Kept,
            Ok(CommandOutcome::Local(_) | CommandOutcome::Recorded) => Self::Local,
            Ok(CommandOutcome::UpdateCheck(_)) => Self::UpdateStarted,
            Ok(CommandOutcome::Exported(e)) => Self::Exported {
                summary: e.replaced.summary(),
                text: e.text,
            },
            Err(CommandError::UpdatesUnavailable) => {
                Self::Failed("update checking is not available in this build".into())
            }
            Err(CommandError::Export(e)) => Self::ExportRefused(export_refusal(&e)),
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

/// The sentence for a refused export or save (diagnostics.md; nothing is ever saved on refusal).
pub fn export_refusal(e: &ExportError) -> String {
    match e {
        ExportError::Refused(_) => REFUSED_TEXT.into(),
        ExportError::Save(SaveError::ForbiddenLocation) => {
            format!("That place is not allowed. {SHARED_FOLDER_WARNING} Nothing was saved.")
        }
        ExportError::Save(SaveError::BadDestination) => {
            "That place can't be saved to. Choose an existing folder. Nothing was saved.".into()
        }
        ExportError::Save(SaveError::Io(_)) => {
            "The file could not be written. Nothing was saved.".into()
        }
    }
}

/// The sentence shown after an action (decision flows: ops.md §1.3, rollback.md §1).
pub fn result_text(kind: ActionKind, r: &ActionResult) -> String {
    match r {
        ActionResult::UpdateStarted => "Checking for updates…".into(),
        ActionResult::Exported { summary, .. } => match kind {
            ActionKind::SaveDiagnostics => format!("Saved. {summary}."),
            _ => format!("Preview ready. {summary}."),
        },
        ActionResult::ExportRefused(t) => t.clone(),
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
    /// This computer's own enrollment, when active: it may be the settings master.
    pub this_enrollment: Option<EnrollmentRef>,
    pub retired: Vec<RetiredComputer>,
    /// Rejected releases the owner may allow again. Pending snapshot field (R4-integrate).
    pub allow_again: Vec<crate::computers_ui::AllowAgainRow>,
    /// Held releases per add-on for the history list. Pending snapshot field (R4-integrate).
    pub history: Vec<crate::rollback_ui::ReleaseRow>,
    /// Removals the group has recorded, for the Removed section.
    pub removed: Vec<wow_sync_runtime::RemovedRow>,
    /// The removal record of each `removed` row (same index), which Restore names.
    pub removal_hashes: Vec<Sha256>,
    /// Name of a live member whose WoW Sync can't block versions (Q-84), which blocks go-back.
    pub older_peer: Option<String>,
    /// Name of the settings master, when one is chosen.
    pub master: Option<String>,
    /// A guided reconfirmation is pending and nothing worse applies (ops.md §1.7).
    pub review_needed: bool,
    /// The last update check result, if any.
    pub update: Option<wow_sync_core::update::UpdateResult>,
    /// The About toggles (update check, notifications).
    pub toggles: wow_sync_runtime::AppToggles,
    /// Settings units, policy and accounts. Not in the snapshot yet (handover).
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
            this_enrollment: None,
            retired: Vec::new(),
            allow_again: Vec::new(),
            history: Vec::new(),
            removed: Vec::new(),
            removal_hashes: Vec::new(),
            older_peer: None,
            master: None,
            review_needed: false,
            update: None,
            toggles: Default::default(),
            settings: Default::default(),
        }
    }
}

/// Fill what the status snapshot carries: standing, hold, retired computers and held releases.
/// Not in the snapshot yet (handover): enrollments of active peers, settings units, the master,
/// rejected releases to allow again and an older-peer flag; those stay empty and their controls
/// say why they are unavailable.
pub fn apply_snapshot(f: &mut FeatureData, s: &wow_sync_runtime::StatusSnapshot) {
    f.hold = s.hold.clone();
    f.review_needed = s.review_needed;
    f.update = s.update.clone();
    f.removed = s.removed.clone();
    f.removal_hashes = s.removal_hashes.clone();
    apply_peers(f, s);
    let retired_on = |enrollment: &wow_sync_core::ids::EnrollmentId| {
        s.peers
            .iter()
            .find(|p| &p.enrollment.enrollment_id == enrollment)
            .and_then(|p| p.retired_at_ms)
            .map(crate::world::day_text)
            .unwrap_or_default()
    };
    f.standing = match (&s.this_retirement, &s.hold) {
        (Some(info), _) => Standing::Retired {
            on: retired_on(&info.enrollment),
            by: s
                .retirements
                .iter()
                .find(|r| r.is_this)
                .map(|r| r.retired_by.clone())
                .unwrap_or_else(|| "another computer".into()),
        },
        (None, HoldScope::AllApplies(AllHoldCause::RetirementUnresolved)) => Standing::Held,
        _ => Standing::Active,
    };
    f.retired = s
        .retirements
        .iter()
        .filter(|r| !r.is_this)
        .map(|r| RetiredComputer {
            name: r.name.clone(),
            on: retired_on(&r.enrollment),
            by: r.retired_by.clone(),
        })
        .collect();
    f.history = s
        .held_releases
        .iter()
        .map(|h| crate::rollback_ui::ReleaseRow {
            package: h.package.clone(),
            title: h.title.clone(),
            version: if h.label.is_empty() {
                "from earlier".into()
            } else {
                h.label.clone()
            },
            package_hash: h.release,
            current: false,
            rejections: s
                .rejected_releases
                .iter()
                .filter(|r| r.release == h.release)
                .flat_map(|r| r.rejections.iter().copied())
                .collect(),
            held_by: h.holders.join(", "),
        })
        .collect();
    f.older_peer = s.older_peer.clone();
    f.allow_again = s
        .rejected_releases
        .iter()
        .map(|r| crate::computers_ui::AllowAgainRow {
            title: r.folders.join(", "),
            version: if r.label.is_empty() {
                "from earlier".into()
            } else {
                r.label.clone()
            },
            rejections: r.rejections.clone(),
            on: crate::world::day_text(r.rejected_at_ms),
        })
        .collect();
}

/// The enrollments a retire or a master choice may name: active computers only, never a
/// retired or unresolved one. This computer is kept apart because it can be master, not retired.
fn apply_peers(f: &mut FeatureData, s: &wow_sync_runtime::StatusSnapshot) {
    use wow_sync_runtime::PeerStanding;
    let active = s
        .peers
        .iter()
        .filter(|p| p.standing == PeerStanding::Active);
    f.enrollments = active
        .clone()
        .filter(|p| !p.is_this)
        .map(|p| {
            (
                p.enrollment.computer_id.as_str().to_string(),
                p.enrollment.clone(),
            )
        })
        .collect();
    f.this_enrollment = active
        .filter(|p| p.is_this)
        .map(|p| p.enrollment.clone())
        .next();
    f.master = s.settings.master_name.clone();
    apply_settings(f, &s.settings);
}

fn apply_settings(f: &mut FeatureData, s: &wow_sync_runtime::SettingsSnapshot) {
    let d = &mut f.settings;
    d.enabled = s.enabled;
    d.accounts = s.accounts.clone();
    d.master = s.master.clone();
    d.exceptions = s.exceptions.clone();
    d.policy_conflict = s.policy_conflict;
    d.paused = s.paused.clone();
}

/// A disabled control carries its reason; an enabled one is `Ok`.
pub type Gate = Result<(), String>;

/// Whether `kind` can be used now. Order: standing (a retired computer never publishes), then
/// whether this build is wired to the runtime command.
pub fn gate(w: &World, kind: ActionKind) -> Gate {
    if kind.publishes() && matches!(w.features.standing, Standing::Retired { .. }) {
        return Err(RETIRED_REASON.into());
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
        assert_eq!(e, NOT_WIRED);
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
        assert!(Capabilities::runtime().has(ActionKind::CheckUpdates));
    }

    fn retirement(name: &str, by: &str, is_this: bool) -> wow_sync_runtime::RetirementRow {
        use wow_sync_core::ids::{ComputerId, EnrollmentId};
        wow_sync_runtime::RetirementRow {
            computer: ComputerId::parse("22222222222222222222222222222222").unwrap(),
            enrollment: EnrollmentId::parse("33333333333333333333333333333333").unwrap(),
            name: name.into(),
            retired_by: by.into(),
            retired_by_id: ComputerId::parse("11111111111111111111111111111111").unwrap(),
            cutoff_seq: 4,
            is_this,
        }
    }

    #[test]
    fn the_snapshot_fills_standing_retired_rows_and_held_releases() {
        use wow_sync_core::ids::{ComputerId, EnrollmentId};
        let mut s = wow_sync_runtime::StatusSnapshot::default();
        let mut f = FeatureData::default();
        apply_snapshot(&mut f, &s);
        assert_eq!(f.standing, Standing::Active);
        assert!(f.retired.is_empty() && f.history.is_empty());

        s.this_retirement = Some(wow_sync_core::status::RetiredInfo {
            enrollment: EnrollmentId::parse("33333333333333333333333333333333").unwrap(),
            retired_by: ComputerId::parse("11111111111111111111111111111111").unwrap(),
            cutoff_seq: 4,
        });
        s.retirements = vec![
            retirement("this computer", "Den PC", true),
            retirement("Laptop", "Den PC", false),
        ];
        s.held_releases = vec![wow_sync_runtime::HeldRelease {
            package: validate_folder_name("Details").unwrap(),
            title: "Details".into(),
            release: Sha256([7; 32]),
            label: "2.0".into(),
            holders: vec!["Den PC".into(), "Laptop".into()],
        }];
        apply_snapshot(&mut f, &s);
        assert_eq!(
            f.standing,
            Standing::Retired {
                on: String::new(),
                by: "Den PC".into()
            }
        );
        assert_eq!(
            f.retired.len(),
            1,
            "this computer is not in the Retired group"
        );
        assert_eq!(f.retired[0].name, "Laptop");
        assert_eq!(f.history[0].version, "2.0");
        assert_eq!(f.history[0].held_by, "Den PC, Laptop");
        assert_eq!(f.history[0].package_hash, Sha256([7; 32]));

        s.this_retirement = None;
        s.hold = HoldScope::AllApplies(AllHoldCause::RetirementUnresolved);
        apply_snapshot(&mut f, &s);
        assert_eq!(
            f.standing,
            Standing::Held,
            "unresolved is not shown as retired"
        );
    }

    fn peer(
        id: &str,
        name: &str,
        standing: wow_sync_runtime::PeerStanding,
        is_this: bool,
        at: Option<u64>,
    ) -> wow_sync_runtime::PeerRow {
        use wow_sync_core::ids::{ComputerId, EnrollmentId};
        wow_sync_runtime::PeerRow {
            enrollment: EnrollmentRef {
                computer_id: ComputerId::parse(id).unwrap(),
                enrollment_id: EnrollmentId::parse(id).unwrap(),
            },
            name: name.into(),
            standing,
            retired_at_ms: at,
            is_this,
        }
    }

    #[test]
    fn only_active_other_computers_can_be_retired_and_the_retirement_date_is_shown() {
        use wow_sync_runtime::PeerStanding::*;
        let den = "22222222222222222222222222222222";
        let mut s = wow_sync_runtime::StatusSnapshot::default();
        s.peers = vec![
            peer(
                "11111111111111111111111111111111",
                "Desk",
                Active,
                true,
                None,
            ),
            peer(den, "Den PC", Active, false, None),
            peer(
                "44444444444444444444444444444444",
                "Old",
                Retired,
                false,
                Some(1_768_219_200_000),
            ),
            peer(
                "55555555555555555555555555555555",
                "Maybe",
                RetirementUnresolved,
                false,
                None,
            ),
        ];
        let mut f = FeatureData::default();
        apply_snapshot(&mut f, &s);
        assert_eq!(
            f.enrollments.len(),
            1,
            "retired and unresolved are not offered"
        );
        assert_eq!(f.enrollments[0].0, den);
        assert!(f.this_enrollment.is_some(), "this computer is kept apart");
        assert!(
            f.enrollments
                .iter()
                .all(|(_, e)| Some(e) != f.this_enrollment.as_ref()),
            "this computer is never offered for retirement"
        );
        s.retirements = vec![retirement("Old", "Den PC", false)];
        s.retirements[0].enrollment =
            wow_sync_core::ids::EnrollmentId::parse("44444444444444444444444444444444").unwrap();
        apply_snapshot(&mut f, &s);
        assert_eq!(f.retired[0].on, "12 January");
    }

    #[test]
    fn rejected_releases_become_allow_again_rows_and_mark_the_held_release_blocked() {
        let mut s = wow_sync_runtime::StatusSnapshot::default();
        s.held_releases = vec![wow_sync_runtime::HeldRelease {
            package: validate_folder_name("Details").unwrap(),
            title: "Details".into(),
            release: Sha256([7; 32]),
            label: "2.0".into(),
            holders: vec!["Den PC".into()],
        }];
        s.rejected_releases = vec![
            wow_sync_runtime::RejectedRelease {
                folders: vec!["Details".into(), "Details_Options".into()],
                label: "2.0".into(),
                release: Sha256([7; 32]),
                rejections: vec![Sha256([1; 32]), Sha256([2; 32])],
                rejected_at_ms: 1_768_219_200_000,
            },
            wow_sync_runtime::RejectedRelease {
                folders: vec!["Bagnon".into()],
                label: String::new(),
                release: Sha256([8; 32]),
                rejections: vec![Sha256([3; 32])],
                rejected_at_ms: 1_768_219_200_000,
            },
        ];
        s.older_peer = Some("Laptop".into());
        let mut f = FeatureData::default();
        apply_snapshot(&mut f, &s);
        assert_eq!(f.allow_again.len(), 2);
        assert_eq!(f.allow_again[0].title, "Details, Details_Options");
        assert_eq!(
            f.allow_again[0].rejections,
            vec![Sha256([1; 32]), Sha256([2; 32])],
            "every record that rejects it, and only those"
        );
        assert_eq!(f.allow_again[0].on, "12 January");
        assert_eq!(f.allow_again[1].version, "from earlier");
        assert_eq!(
            f.history[0].rejections,
            vec![Sha256([1; 32]), Sha256([2; 32])],
            "the held release is blocked by its own rejections, not another's"
        );
        assert_eq!(f.older_peer.as_deref(), Some("Laptop"));
        s.rejected_releases.clear();
        s.older_peer = None;
        apply_snapshot(&mut f, &s);
        assert!(f.allow_again.is_empty() && f.history[0].rejections.is_empty());
        assert_eq!(f.older_peer, None);
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
