//! View-model for settings sync: unit status, master and exceptions, share, pause and resume,
//! use shared, choose, restore (settings.md §4-§6, §9). Pure: every control is either a typed
//! `GroupAction` or the reason it is unavailable; nothing here writes.

use crate::features::{gate, ActionKind, GroupAction};
use crate::world::World;
use wow_sync_core::ids::{SettingsUnit, Sha256};
use wow_sync_core::kinds::{EnrollmentRef, PolicyRule, PolicyRuleKind};
use wow_sync_core::paths::FolderKey;
use wow_sync_core::settings::{resume_preview, LocalReason, ResumeOption};

/// What the unit is doing here, one per row of settings.md §9.1.
// The service reports only some of these; the rest are built by tests.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitStatus {
    Off,
    /// This computer is retired and records no settings intent.
    Retired,
    /// Syncing here right now.
    Active,
    InSync {
        /// None for this computer's own copy.
        from: Option<String>,
    },
    Waiting {
        from: String,
    },
    LocalDifferent(LocalReason),
    NoSharedCopy,
    Paused,
    NeedsChoice,
    PolicyConflict,
    MasterAbsent {
        name: String,
    },
    Diagnosed,
    Suspect,
    CaptureFailed,
    NotShared,
    /// The add-on's settings are shared but this character has not been on this computer yet.
    WaitingForIdentity {
        who: Option<String>,
    },
    /// The settings file is too big to share automatically.
    TooLarge {
        size: Option<u64>,
    },
    /// Other add-ons declare the same saved variable, so ownership is not clear.
    OwnedBySeveral {
        with: Vec<String>,
    },
    /// New settings are ready but WoW is running.
    WaitingForWow,
    /// The shared copy has not arrived in the shared folder yet.
    WaitingForSharedCopy,
}

/// One saved copy of a unit, shown by when and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copy {
    pub checkpoint: Sha256,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRow {
    pub unit: SettingsUnit,
    /// The add-on's name, for sentences.
    pub addon: String,
    pub status: UnitStatus,
    /// Settings of this unit are being synced here right now.
    pub live: bool,
    /// This computer's latest capture, the thing "Share my settings" publishes.
    pub latest_capture: Option<Sha256>,
    pub can_share: bool,
    /// The competing heads of a `NeedsChoice` unit.
    pub heads: Vec<Copy>,
    /// Every current head revision, which a choice must name.
    pub shown_heads: Vec<Sha256>,
    /// Older saved copies that "Restore" can bring back.
    pub history: Vec<Copy>,
    /// Copies of this computer's own settings that a shared copy replaced (recovery copies).
    pub local_copies: Vec<Copy>,
    pub restore_possible: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsData {
    pub units: Vec<UnitRow>,
    pub exceptions: Vec<PolicyRule>,
    pub master: Option<EnrollmentRef>,
    pub accounts: Vec<String>,
    /// Account folders found on this computer that settings sync could include.
    pub available_accounts: Vec<String>,
    pub enabled: bool,
    /// Competing master choices exist; nothing is chosen for the owner.
    pub policy_conflict: bool,
    /// Paused folders with what a resume would meet.
    pub paused: Vec<wow_sync_runtime::PausedFolder>,
    /// "Pause all settings" is on for this computer.
    pub paused_all: bool,
    /// What "Share these now" would push (runtime `to_share`).
    pub to_share: Vec<ShareItem>,
    /// Some computer already shared settings in this group (not the first adoption).
    pub group_has_shared_settings: bool,
}

/// One add-on's settings the runtime says could be pushed now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareItem {
    pub unit: SettingsUnit,
    /// Nothing was ever shared for it; otherwise it would replace the shared copy.
    pub not_shared_yet: bool,
}

/// Shown when no add-on has settings to list yet.
pub const NO_UNITS: &str = "No add-on settings are known yet. They are listed here once WoW Sync has saved or received some.";

/// A button: its label and either the action it sends or why it is unavailable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    pub label: String,
    pub action: Result<GroupAction, String>,
}

fn control(w: &World, kind: ActionKind, label: impl Into<String>, a: GroupAction) -> Control {
    Control {
        label: label.into(),
        action: gate(w, kind).map(|()| a),
    }
}

fn off(label: impl Into<String>, reason: &str) -> Control {
    Control {
        label: label.into(),
        action: Err(reason.into()),
    }
}

pub fn identity_words(u: &SettingsUnit) -> String {
    match u.identity.realm_character() {
        Some((realm, ch)) => format!("{ch} on {realm}"),
        None => format!("account {}", u.identity.account_name()),
    }
}

pub fn status_text(s: &UnitStatus) -> String {
    match s {
        UnitStatus::Off => "Settings not shared".into(),
        UnitStatus::Retired => "This computer is retired; its settings are not shared".into(),
        UnitStatus::Active => "Settings are syncing here".into(),
        UnitStatus::InSync { from: Some(from) } => format!("Settings up to date (from {from})"),
        UnitStatus::InSync { from: None } => "Settings up to date".into(),
        UnitStatus::Waiting { from } => {
            format!("New settings from {from} will be used when WoW closes")
        }
        UnitStatus::LocalDifferent(_) => "Changed here, not shared".into(),
        UnitStatus::NoSharedCopy => "Not shared yet".into(),
        UnitStatus::Paused => "Settings paused on this computer".into(),
        UnitStatus::NeedsChoice => "Choose which settings to keep".into(),
        UnitStatus::PolicyConflict => "Settings master was changed on two computers".into(),
        UnitStatus::MasterAbsent { name } => format!(
            "The settings master {name} was retired. Choose a new master to share settings again."
        ),
        UnitStatus::Diagnosed => "WoW Sync can't tell which add-on owns these settings".into(),
        UnitStatus::Suspect => {
            "This add-on's settings file looks damaged; it was not shared".into()
        }
        UnitStatus::CaptureFailed => "Settings could not be saved".into(),
        UnitStatus::NotShared => "Not shared (your choice)".into(),
        UnitStatus::WaitingForIdentity { who: Some(who) } => {
            format!("Waiting for {who} to log in on this computer")
        }
        UnitStatus::WaitingForIdentity { who: None } => {
            "Waiting for a character to log in on this computer".into()
        }
        UnitStatus::TooLarge { size: Some(n) } => format!(
            "Too large to share automatically ({})",
            crate::history_ui::size_text(*n)
        ),
        UnitStatus::TooLarge { size: None } => "Too large to share automatically".into(),
        UnitStatus::OwnedBySeveral { with } if with.is_empty() => {
            "Shared with another add-on, not synced".into()
        }
        UnitStatus::OwnedBySeveral { with } => {
            format!("Shared with {}, not synced", with.join(", "))
        }
        UnitStatus::WaitingForWow => "New settings will be used when WoW closes".into(),
        UnitStatus::WaitingForSharedCopy => "Waiting for the shared settings to arrive".into(),
    }
}

/// The buttons a unit offers beside its status (settings.md §9.1).
pub fn unit_controls(w: &World, row: &UnitRow) -> Vec<Control> {
    let mut out = Vec::new();
    let share = |out: &mut Vec<Control>| {
        out.push(match (row.can_share, row.latest_capture) {
            (true, Some(checkpoint)) => control(
                w,
                ActionKind::ShareSettings,
                "Share my settings",
                GroupAction::ShareSettings {
                    unit: row.unit.clone(),
                    checkpoint,
                },
            ),
            _ => off(
                "Share my settings",
                "There is nothing saved here to share yet.",
            ),
        });
    };
    match &row.status {
        UnitStatus::Active => share(&mut out),
        UnitStatus::LocalDifferent(_) => {
            share(&mut out);
            out.push(control(
                w,
                ActionKind::UseSharedSettings,
                "Use shared settings",
                GroupAction::UseSharedSettings {
                    unit: row.unit.clone(),
                },
            ));
        }
        UnitStatus::NeedsChoice => {
            let shown = row.shown_heads.clone();
            if shown.is_empty() {
                out.push(off(
                    "Choose which settings to keep",
                    "The competing copies are not known yet. Try again after the next check.",
                ));
            }
            for h in row.heads.iter().filter(|_| !shown.is_empty()) {
                out.push(control(
                    w,
                    ActionKind::ChooseSettings,
                    format!("Keep {}", h.label),
                    GroupAction::ChooseSettings {
                        unit: row.unit.clone(),
                        checkpoint: h.checkpoint,
                        shown_heads: shown.clone(),
                    },
                ));
            }
        }
        _ => {}
    }
    out
}

/// "Restore" buttons for older saved copies, with the sentence to confirm first.
pub fn restore_controls(w: &World, row: &UnitRow) -> Vec<(Control, String)> {
    let local = row.local_copies.iter().map(|c| (c, true));
    row.history
        .iter()
        .map(|c| (c, false))
        .chain(local)
        .map(|(c, mine)| {
            let confirm = if mine {
                format!(
                    "Restore this computer's {} for {}? The settings used now are kept as a recovery copy.",
                    c.label,
                    identity_words(&row.unit)
                )
            } else {
                format!(
                    "Restore {} for {}? Your current settings are kept as a recovery copy.",
                    c.label,
                    identity_words(&row.unit)
                )
            };
            let label = if mine {
                format!("Restore this computer's {}…", c.label)
            } else {
                format!("Restore {}…", c.label)
            };
            let button = if row.restore_possible {
                control(
                    w,
                    ActionKind::RestoreSettings,
                    label,
                    GroupAction::RestoreSettings {
                        unit: row.unit.clone(),
                        checkpoint: c.checkpoint,
                    },
                )
            } else {
                off(
                    label,
                    "Settings can't be restored here right now (sync is off, paused or blocked for this add-on).",
                )
            };
            (button, confirm)
        })
        .collect()
}

/// "Pause all settings" or, when it is on, "Resume all settings"; the per-add-on pause stays.
pub fn pause_all_control(w: &World) -> Control {
    if w.features.settings.paused_all {
        control(
            w,
            ActionKind::ResumeAllSettings,
            "Resume all settings",
            GroupAction::ResumeAllSettings,
        )
    } else {
        control(
            w,
            ActionKind::PauseAllSettings,
            "Pause all settings",
            GroupAction::PauseAllSettings,
        )
    }
}

pub fn pause_control(w: &World, folder: &FolderKey) -> Control {
    control(
        w,
        ActionKind::PauseSettings,
        "Pause settings",
        GroupAction::PauseSettings {
            folder: folder.clone(),
        },
    )
}

fn resume_label(o: ResumeOption) -> &'static str {
    match o {
        ResumeOption::Resume => "Resume",
        ResumeOption::KeepMineHere => "Keep mine here",
        ResumeOption::ShareMine | ResumeOption::ShareMineInstead => "Share mine",
        ResumeOption::GoBackToShared | ResumeOption::UseShared => "Use shared settings",
        ResumeOption::UseSharedKeepMineAsRecovery => "Use shared, keep mine as a recovery copy",
        ResumeOption::StayPaused => "Stay paused",
    }
}

/// The choices when resuming, in the order the preview gives them. "Stay paused" is the sheet's
/// Cancel, so it is not a command. Resume never shares: share choices say what to do instead.
pub fn resume_controls(
    w: &World,
    folder: &FolderKey,
    here_changed: bool,
    shared_newer: bool,
) -> Vec<Control> {
    resume_preview(here_changed, shared_newer)
        .options
        .into_iter()
        .filter(|o| *o != ResumeOption::StayPaused)
        .map(|o| match o {
            ResumeOption::ShareMine | ResumeOption::ShareMineInstead => off(
                resume_label(o),
                "Resume never shares. Resume first, then choose Share my settings.",
            ),
            _ => control(
                w,
                ActionKind::ResumeSettings,
                resume_label(o),
                GroupAction::ResumeSettings {
                    folder: folder.clone(),
                    choice: o,
                },
            ),
        })
        .collect()
}

pub fn computer_name(w: &World, e: &EnrollmentRef) -> String {
    if w.features.this_enrollment.as_ref() == Some(e) {
        return "this computer".into();
    }
    w.features
        .enrollments
        .iter()
        .find(|(_, r)| r == e)
        .and_then(|(id, _)| w.peers.iter().find(|p| p.id == *id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "another computer".into())
}

/// Choose, change or clear the settings master. Exceptions are kept as they are.
pub fn master_controls(w: &World) -> Vec<Control> {
    let d = &w.features.settings;
    let candidates = w
        .features
        .this_enrollment
        .iter()
        .map(|e| ("this computer".to_string(), e))
        .chain(w.features.enrollments.iter().map(|(id, e)| {
            let name = w
                .peers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "another computer".into());
            (name, e)
        }));
    let mut out: Vec<Control> = candidates
        .filter(|(_, e)| d.master.as_ref() != Some(*e))
        // This computer becomes master only through the confirmation (`make_master_control`).
        .filter(|(_, e)| w.features.this_enrollment.as_ref() != Some(*e))
        .map(|(name, e)| {
            control(
                w,
                ActionKind::SetSettingsPolicy,
                format!("Use {name} as the settings master"),
                GroupAction::SetSettingsPolicy {
                    master: Some(e.clone()),
                    exceptions: d.exceptions.clone(),
                    share_now: Vec::new(),
                },
            )
        })
        .collect();
    if out.is_empty() && d.master.is_none() && w.features.this_enrollment.is_none() {
        out.push(off(
            "Choose a settings master",
            "No other active computer is known yet. Try again after the next check.",
        ));
    }
    if d.master.is_some() {
        out.push(control(
            w,
            ActionKind::SetSettingsPolicy,
            "Stop sharing with a master",
            GroupAction::SetSettingsPolicy {
                master: None,
                exceptions: d.exceptions.clone(),
                share_now: Vec::new(),
            },
        ));
    }
    out
}

fn without(rules: &[PolicyRule], folder: &FolderKey) -> Vec<PolicyRule> {
    rules
        .iter()
        .filter(|r| &r.folder != folder)
        .cloned()
        .collect()
}

/// The settings master when it is still an active computer. A retired or unresolved master is not
/// one: nothing is shared from it and no replacement is chosen for the owner.
pub fn active_master(w: &World) -> Option<&EnrollmentRef> {
    let m = w.features.settings.master.as_ref()?;
    let active = w.features.this_enrollment.as_ref() == Some(m)
        || w.features.enrollments.iter().any(|(_, e)| e == m);
    active.then_some(m)
}

/// The sentence at the top of the Settings sync page. The settings size limit is approved
/// (Q-77, 32 MiB), so it never says sharing waits for one.
pub fn banner_text(w: &World) -> String {
    let d = &w.features.settings;
    if d.policy_conflict {
        return "The settings master was changed on two computers. Nothing is chosen for you; choose a master to share settings again.".into();
    }
    let name = w
        .features
        .master
        .clone()
        .unwrap_or_else(|| "The settings master".into());
    if d.master.is_some() && active_master(w).is_none() {
        return format!(
            "{name} was the settings master and is no longer an active computer, so settings have no active master and are not being shared. Nothing is chosen for you; choose a master to share settings again."
        );
    }
    if d.master.is_some() {
        if !d.enabled {
            return format!(
                "{name} is the settings master, but settings sync is off on this computer."
            );
        }
        return format!(
            "{name} shares its add-on settings with the group. Settings are never merged."
        );
    }
    if d.enabled {
        "No settings master is chosen, so settings are not shared yet.".into()
    } else {
        "Settings sync is off. Add-on files are synced; your add-on settings stay as they are on each computer.".into()
    }
}

/// Per add-on: don't share it, take it from another computer, or drop the exception.
pub fn exception_controls(w: &World, folder: &FolderKey) -> Vec<Control> {
    let d = &w.features.settings;
    let Some(master) = active_master(w).cloned() else {
        let why = if d.policy_conflict {
            "The settings master was changed on two computers. Choose one master first."
        } else if d.master.is_some() {
            "The settings master is no longer an active computer. Choose a new master first."
        } else {
            "Choose a settings master first; exceptions are changes to what it shares."
        };
        return vec![off("Add an exception…", why)];
    };
    let rest = without(&d.exceptions, folder);
    let mut out = Vec::new();
    let mut set = |label: String, rule: PolicyRule| {
        let mut exceptions = rest.clone();
        exceptions.push(rule);
        out.push(control(
            w,
            ActionKind::SetSettingsPolicy,
            label,
            GroupAction::SetSettingsPolicy {
                master: Some(master.clone()),
                exceptions,
                share_now: Vec::new(),
            },
        ));
    };
    set(
        "Don't share this add-on".into(),
        PolicyRule {
            folder: folder.clone(),
            rule: PolicyRuleKind::NotShared,
            computer_id: None,
            enrollment_id: None,
        },
    );
    let sources = w
        .features
        .this_enrollment
        .iter()
        .chain(w.features.enrollments.iter().map(|(_, e)| e))
        .filter(|e| **e != master);
    for e in sources {
        set(
            format!("Use {} for this add-on", computer_name(w, e)),
            PolicyRule {
                folder: folder.clone(),
                rule: PolicyRuleKind::Master,
                computer_id: Some(e.computer_id.clone()),
                enrollment_id: Some(e.enrollment_id.clone()),
            },
        );
    }
    if rest.len() != d.exceptions.len() {
        out.push(control(
            w,
            ActionKind::SetSettingsPolicy,
            "Remove this exception",
            GroupAction::SetSettingsPolicy {
                master: Some(master),
                exceptions: rest,
                share_now: Vec::new(),
            },
        ));
    }
    out
}

/// "Share settings for these accounts" before settings sync is on.
/// "Turn on settings sync" for the accounts the owner ticked. Only accounts the snapshot lists are
/// offered, and nothing is chosen for the owner.
pub fn enable_control(w: &World, chosen: &[String]) -> Control {
    let listed: Vec<String> = chosen
        .iter()
        .filter(|a| w.features.settings.available_accounts.contains(a))
        .cloned()
        .collect();
    if w.features.settings.available_accounts.is_empty() {
        return off(
            "Turn on settings sync",
            "No WoW account folder was found on this computer to include.",
        );
    }
    if listed.is_empty() {
        return off("Turn on settings sync", "Choose at least one account.");
    }
    control(
        w,
        ActionKind::EnableSettings,
        "Turn on settings sync",
        GroupAction::EnableSettings {
            included_accounts: listed,
        },
    )
}

/// How urgent a unit status is for the one-line summary of an add-on; higher is shown first.
fn urgency(s: &UnitStatus) -> u8 {
    match s {
        UnitStatus::NeedsChoice | UnitStatus::PolicyConflict | UnitStatus::MasterAbsent { .. } => 9,
        UnitStatus::CaptureFailed
        | UnitStatus::Suspect
        | UnitStatus::Diagnosed
        | UnitStatus::TooLarge { .. }
        | UnitStatus::OwnedBySeveral { .. } => 8,
        UnitStatus::LocalDifferent(_) => 7,
        UnitStatus::Waiting { .. }
        | UnitStatus::WaitingForIdentity { .. }
        | UnitStatus::WaitingForWow
        | UnitStatus::WaitingForSharedCopy => 6,
        UnitStatus::Paused => 5,
        UnitStatus::NoSharedCopy => 4,
        UnitStatus::Retired => 3,
        UnitStatus::NotShared | UnitStatus::Off => 2,
        UnitStatus::Active | UnitStatus::InSync { .. } => 1,
    }
}

/// An add-on with no row of its own that another add-on's row says shares its saved variables.
fn shared_by_other(w: &World, folder: &str) -> Option<UnitStatus> {
    let names: Vec<&str> = std::iter::once(folder)
        .chain(
            w.addons
                .iter()
                .filter(|a| {
                    a.key.eq_ignore_ascii_case(folder) || a.title.eq_ignore_ascii_case(folder)
                })
                .flat_map(|a| [a.key.as_str(), a.title.as_str()]),
        )
        .collect();
    w.features
        .settings
        .units
        .iter()
        .find_map(|u| match &u.status {
            UnitStatus::OwnedBySeveral { with }
                if with
                    .iter()
                    .any(|n| names.iter().any(|m| n.eq_ignore_ascii_case(m))) =>
            {
                Some(UnitStatus::OwnedBySeveral {
                    with: vec![u.addon.clone()],
                })
            }
            _ => None,
        })
}

/// The settings line of the Add-ons list: the most urgent status among the add-on's identities.
pub fn addon_settings_line(w: &World, folder: &str) -> String {
    let worst = w
        .features
        .settings
        .units
        .iter()
        .filter(|u| u.unit.folder.as_str().eq_ignore_ascii_case(folder))
        .map(|u| &u.status)
        .max_by_key(|s| urgency(s));
    match worst {
        Some(UnitStatus::WaitingForIdentity { .. }) => {
            status_text(&UnitStatus::WaitingForIdentity { who: None })
        }
        Some(s) => status_text(s),
        None if shared_by_other(w, folder).is_some() => {
            status_text(&shared_by_other(w, folder).unwrap_or(UnitStatus::Off))
        }
        None if w.features.settings.enabled => "Not shared yet".into(),
        None => "Settings not shared".into(),
    }
}

/// One line per identity for Add-on detail, in plain words ("Thrall on Argent Dawn: ...").
pub fn addon_settings_lines(w: &World, folder: &str) -> Vec<String> {
    let lines: Vec<String> = w
        .features
        .settings
        .units
        .iter()
        .filter(|u| u.unit.folder.as_str().eq_ignore_ascii_case(folder))
        .map(|u| format!("{}: {}", identity_words(&u.unit), status_text(&u.status)))
        .collect();
    if lines.is_empty() {
        vec![addon_settings_line(w, folder)]
    } else {
        lines
    }
}

/// The add-ons with saved settings waiting for the owner to pick one of two competing copies.
pub fn pending_choice_addons(w: &World) -> Vec<String> {
    let mut v: Vec<String> = w
        .features
        .settings
        .units
        .iter()
        .filter(|u| matches!(u.status, UnitStatus::NeedsChoice))
        .map(|u| u.addon.clone())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// What the headline says while a settings choice is pending: never "up to date".
pub fn pending_choice_headline(w: &World) -> Option<String> {
    match pending_choice_addons(w).len() {
        0 => None,
        1 => Some("Choose which saved settings to keep".into()),
        n => Some(format!("Choose which saved settings to keep ({n} add-ons)")),
    }
}

/// The guided offer to share add-on settings, shown only while nothing about settings is on here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    /// No settings policy exists: this computer may choose the master.
    Share,
    /// A master was chosen on another computer and this one has not confirmed.
    UseMaster { master: String },
}

pub const OFFER_SHARE_QUESTION: &str = "Share add-on settings between your computers?";

/// The start of the runtime's note when some saved settings were left out of a pass.
pub const SETTINGS_NOTE_PREFIX: &str = "some saved settings were not shared";

/// The note shown with the settings status: a capital and a closing full stop.
pub fn settings_note_text(w: &World) -> Option<String> {
    let n = w.settings_note.as_deref()?.trim();
    let mut c = n.chars();
    let first = c.next()?;
    let mut s: String = first.to_uppercase().chain(c).collect();
    if !s.ends_with('.') {
        s.push('.');
    }
    Some(s)
}

/// The consent text of settings.md §5.4, shown before anything is shared.
pub const CONSENT_TEXT: &str = "All addon settings on this computer are shared, including addons you install later. WoW's own settings, key bindings, macros and layout are not shared. Settings stay inside each account and character.";

impl Offer {
    pub fn question(&self) -> String {
        match self {
            Offer::Share => OFFER_SHARE_QUESTION.into(),
            Offer::UseMaster { master } => {
                let mut c = master.chars();
                let first: String = c
                    .next()
                    .map(|f| f.to_uppercase().collect())
                    .unwrap_or_default();
                format!(
                    "{first}{} is now the settings master. Use its add-on settings here?",
                    c.as_str()
                )
            }
        }
    }

    pub fn confirm_label(&self) -> &'static str {
        match self {
            Offer::Share => "Share settings",
            Offer::UseMaster { .. } => "Use settings",
        }
    }
}

/// The offer this computer would make, ignoring "Not now" and whether the headline is calm.
pub fn offer(w: &World) -> Option<Offer> {
    let d = &w.features.settings;
    if !w.enrolled
        || d.enabled
        || d.policy_conflict
        || d.available_accounts.is_empty()
        || !matches!(w.features.standing, crate::features::Standing::Active)
    {
        return None;
    }
    match (&d.master, active_master(w)) {
        (None, _) => Some(Offer::Share),
        (Some(_), Some(m)) => Some(Offer::UseMaster {
            master: computer_name(w, m),
        }),
        (Some(_), None) => None,
    }
}

/// The offer for the headline area: add-ons are up to date, no decision waits and the owner has
/// not said "Not now".
pub fn headline_offer(w: &World, dismissed: bool) -> Option<Offer> {
    if dismissed
        || !w.features.decisions.is_empty()
        || crate::model::headline(w).tone != crate::theme::Tone::Ok
    {
        return None;
    }
    offer(w)
}

/// A computer the consent can name as the master.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasterChoice {
    pub label: String,
    pub enrollment: EnrollmentRef,
}

/// Enrolled computers, this one first. This computer is the default.
pub fn master_choices(w: &World) -> Vec<MasterChoice> {
    let mine = w.features.this_enrollment.iter().map(|e| MasterChoice {
        label: format!("{} (this computer)", w.computer),
        enrollment: e.clone(),
    });
    let others = w.features.enrollments.iter().map(|(id, e)| MasterChoice {
        label: w
            .peers
            .iter()
            .find(|p| p.id == *id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "another computer".into()),
        enrollment: e.clone(),
    });
    mine.chain(others).collect()
}

/// One add-on whose settings here the runtime says differ from the group's or have no shared copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareCandidate {
    pub unit: SettingsUnit,
    pub line: String,
}

/// The units a new master could push now (runtime `to_share`). Becoming master never pushes them
/// by itself (settings.md §5.2).
pub fn share_candidates(w: &World) -> Vec<ShareCandidate> {
    w.features
        .settings
        .to_share
        .iter()
        .map(|s| {
            let why = if s.not_shared_yet {
                "has no shared copy yet"
            } else {
                "differs from the shared copy"
            };
            let name = w
                .addon(s.unit.folder.as_str())
                .map(|a| a.title.clone())
                .unwrap_or_else(|| s.unit.folder.as_str().to_string());
            ShareCandidate {
                unit: s.unit.clone(),
                line: format!("{name} ({}) {why}", identity_words(&s.unit)),
            }
        })
        .collect()
}

/// The first adoption: the group has no shared settings yet, so nothing here can overwrite a
/// shared copy (OD-23).
pub fn first_adoption(w: &World) -> bool {
    !w.features.settings.group_has_shared_settings
}

/// "Share these now" is ticked by default only on the first adoption; the owner's flip inverts it.
pub fn share_now_on(w: &World, flipped: bool) -> bool {
    first_adoption(w) != flipped
}

/// What a ticked box pushes: every candidate; nothing when unticked.
pub fn pushes(cands: &[ShareCandidate], on: bool) -> Vec<SettingsUnit> {
    if !on {
        return Vec::new();
    }
    cands.iter().map(|c| c.unit.clone()).collect()
}
/// Before settings sync is on nothing in WTF has been read, so the consent sheet can only say
/// what will happen: the installed add-ons' settings are shared once WoW Sync has read them.
pub fn will_share_line(w: &World) -> Option<String> {
    if w.features.settings.enabled {
        return None;
    }
    let mut names: Vec<&str> = w.addons.iter().map(|a| a.title.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let list = match names.len() {
        0 => String::new(),
        n if n <= 6 => format!(" ({})", names.join(", ")),
        n => format!(" ({} and {} more)", names[..6].join(", "), n - 6),
    };
    Some(format!(
        "The settings of the add-ons installed on this computer{list} will be shared once WoW Sync has read them."
    ))
}

/// "Make this computer the master": the confirmation's action. `share_now` is what the ticked
/// "Share these now" box pushes after the policy is recorded.
pub fn make_master_control(w: &World, share_now: Vec<SettingsUnit>) -> Control {
    let label = "Make this computer the master";
    let Some(mine) = w.features.this_enrollment.clone() else {
        return off(label, "This computer is not ready to be chosen yet.");
    };
    control(
        w,
        ActionKind::SetSettingsPolicy,
        label,
        GroupAction::SetSettingsPolicy {
            master: Some(mine),
            exceptions: w.features.settings.exceptions.clone(),
            share_now,
        },
    )
}

/// "Share settings" / "Use settings": the accounts left ticked and, for the choosing computer,
/// the master picked (`pick` is an index into `master_choices`; `None` is this computer). With
/// this computer as master, `share_now` pushes the ticked add-ons.
pub fn start_control(
    w: &World,
    o: &Offer,
    chosen: &[String],
    pick: Option<usize>,
    share_now: bool,
) -> Control {
    let label = o.confirm_label();
    let listed: Vec<String> = w
        .features
        .settings
        .available_accounts
        .iter()
        .filter(|a| chosen.contains(a))
        .cloned()
        .collect();
    if listed.is_empty() {
        return off(label, "Choose at least one account.");
    }
    let master = match o {
        Offer::UseMaster { .. } => None,
        Offer::Share => {
            let choices = master_choices(w);
            match choices.get(pick.unwrap_or(0)) {
                Some(c) => Some(c.enrollment.clone()),
                None => return off(label, "This computer is not ready to be chosen yet."),
            }
        }
    };
    let this_master = matches!(&master, Some(m) if w.features.this_enrollment.as_ref() == Some(m));
    let push = if this_master {
        pushes(&share_candidates(w), share_now)
    } else {
        Vec::new()
    };
    control(
        w,
        ActionKind::StartSettings,
        label,
        GroupAction::StartSettings {
            master,
            exceptions: w.features.settings.exceptions.clone(),
            included_accounts: listed,
            share_now: push,
            share_when_ready: this_master && share_now,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Capabilities, Standing, RETIRED_REASON};
    use wow_sync_core::ids::{ComputerId, EnrollmentId, Identity};

    const DEN: &str = "22222222222222222222222222222222";

    fn folder(n: &str) -> FolderKey {
        wow_sync_core::paths::validate_folder_name(n).unwrap()
    }

    fn den() -> EnrollmentRef {
        EnrollmentRef {
            computer_id: ComputerId::parse(DEN).unwrap(),
            enrollment_id: EnrollmentId::parse("33333333333333333333333333333333").unwrap(),
        }
    }

    fn unit(status: UnitStatus) -> UnitRow {
        UnitRow {
            unit: SettingsUnit {
                folder: folder("Details"),
                identity: Identity::account("ERIC").unwrap(),
            },
            addon: "Details".into(),
            status,
            live: true,
            latest_capture: Some(Sha256([1; 32])),
            can_share: true,
            heads: vec![],
            shown_heads: vec![],
            history: vec![],
            local_copies: vec![],
            restore_possible: true,
        }
    }

    fn world() -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.features.caps = Capabilities::runtime();
        w.peers.push(crate::world::Peer {
            id: DEN.into(),
            name: "Den PC".into(),
            state: wow_sync_core::status::MemberSummary::Usable,
            report_secs_ago: Some(5),
            game: String::new(),
            app: String::new(),
            confirmed: 0,
        });
        w.features.enrollments.push((DEN.into(), den()));
        w
    }

    #[test]
    fn status_words_follow_the_design() {
        assert_eq!(status_text(&UnitStatus::Off), "Settings not shared");
        assert_eq!(
            status_text(&UnitStatus::InSync {
                from: Some("Den PC".into())
            }),
            "Settings up to date (from Den PC)"
        );
        assert_eq!(
            status_text(&UnitStatus::InSync { from: None }),
            "Settings up to date"
        );
        assert_eq!(
            status_text(&UnitStatus::LocalDifferent(LocalReason::LocalChange)),
            "Changed here, not shared"
        );
        assert!(status_text(&UnitStatus::MasterAbsent {
            name: "Den PC".into()
        })
        .starts_with("The settings master Den PC was retired"));
        let u = unit(UnitStatus::Off).unit;
        assert_eq!(identity_words(&u), "account ERIC");
    }

    #[test]
    fn every_local_difference_offers_share_and_use_shared() {
        let w = world();
        let labels = |r| -> Vec<String> {
            unit_controls(&w, &unit(UnitStatus::LocalDifferent(r)))
                .into_iter()
                .map(|c| c.label)
                .collect()
        };
        assert_eq!(
            labels(LocalReason::LocalChange),
            ["Share my settings", "Use shared settings"]
        );
        assert_eq!(
            labels(LocalReason::EpochNotClean),
            ["Share my settings", "Use shared settings"]
        );
        let c = &unit_controls(
            &w,
            &unit(UnitStatus::LocalDifferent(LocalReason::LocalChange)),
        )[0];
        assert_eq!(
            c.action,
            Ok(GroupAction::ShareSettings {
                unit: unit(UnitStatus::Off).unit,
                checkpoint: Sha256([1; 32]),
            })
        );
    }

    #[test]
    fn a_choice_names_the_heads_it_was_shown() {
        let w = world();
        let mut r = unit(UnitStatus::NeedsChoice);
        r.heads = vec![
            Copy {
                checkpoint: Sha256([2; 32]),
                label: "Den PC's copy".into(),
            },
            Copy {
                checkpoint: Sha256([3; 32]),
                label: "Laptop's copy".into(),
            },
        ];
        r.shown_heads = vec![Sha256([2; 32]), Sha256([3; 32])];
        let cs = unit_controls(&w, &r);
        assert_eq!(cs.len(), 2);
        assert_eq!(
            cs[1].action,
            Ok(GroupAction::ChooseSettings {
                unit: r.unit.clone(),
                checkpoint: Sha256([3; 32]),
                shown_heads: vec![Sha256([2; 32]), Sha256([3; 32])],
            })
        );
    }

    #[test]
    fn resume_never_shares_and_stay_paused_is_not_a_command() {
        let w = world();
        let cs = resume_controls(&w, &folder("Details"), true, true);
        let labels: Vec<&str> = cs.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Use shared, keep mine as a recovery copy", "Share mine"]
        );
        assert!(cs[0].action.is_ok());
        assert_eq!(
            cs[1].action,
            Err("Resume never shares. Resume first, then choose Share my settings.".into())
        );
        let plain = resume_controls(&w, &folder("Details"), false, false);
        assert_eq!(
            plain[0].action,
            Ok(GroupAction::ResumeSettings {
                folder: folder("Details"),
                choice: ResumeOption::Resume
            })
        );
    }

    #[test]
    fn master_and_exceptions_keep_each_other() {
        let mut w = world();
        let cs = master_controls(&w);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].label, "Use Den PC as the settings master");
        assert_eq!(
            exception_controls(&w, &folder("Details"))[0].action,
            Err("Choose a settings master first; exceptions are changes to what it shares.".into())
        );
        w.features.settings.master = Some(den());
        let cs = exception_controls(&w, &folder("Details"));
        let Ok(GroupAction::SetSettingsPolicy {
            master, exceptions, ..
        }) = &cs[0].action
        else {
            panic!("policy");
        };
        assert_eq!(master, &Some(den()));
        assert_eq!(exceptions[0].rule, PolicyRuleKind::NotShared);
        assert!(!cs.iter().any(|c| c.label == "Remove this exception"));
        let existing = exceptions[0].clone();
        w.features.settings.exceptions = vec![existing.clone()];
        let cs = exception_controls(&w, &folder("Details"));
        let Ok(GroupAction::SetSettingsPolicy { exceptions, .. }) = &cs.last().unwrap().action
        else {
            panic!("policy");
        };
        assert!(exceptions.is_empty());
    }

    #[test]
    fn a_retired_computer_cannot_change_settings_and_enable_needs_an_account() {
        let mut w = world();
        w.features.standing = Standing::Retired {
            on: "1 Jan".into(),
            by: "Den PC".into(),
        };
        let r = unit(UnitStatus::LocalDifferent(LocalReason::EpochNotClean));
        let cs = unit_controls(&w, &r);
        assert_eq!(cs[0].label, "Share my settings");
        assert_eq!(cs[0].action, Err(RETIRED_REASON.into()));
        assert_eq!(master_controls(&w)[0].action, Err(RETIRED_REASON.into()));
        let mut w = world();
        w.features.settings.available_accounts = vec!["ERIC".into()];
        assert!(enable_control(&w, &[]).action.is_err());
        assert_eq!(
            enable_control(&w, &["ERIC".into()]).action,
            Ok(GroupAction::EnableSettings {
                included_accounts: vec!["ERIC".into()]
            })
        );
    }

    #[test]
    fn turning_on_offers_only_listed_accounts_from_the_snapshot() {
        let w = snapshot_world_with(|s| {
            s.settings.enabled = false;
            s.settings.available_accounts = vec!["ACC".into(), "ZED".into()];
        });
        assert_eq!(w.features.settings.available_accounts, ["ACC", "ZED"]);
        assert_eq!(
            enable_control(&w, &["ZED".into(), "NOPE".into()]).action,
            Ok(GroupAction::EnableSettings {
                included_accounts: vec!["ZED".into()]
            })
        );
        assert_eq!(
            enable_control(&w, &["NOPE".into()]).action,
            Err("Choose at least one account.".into())
        );
        let none = snapshot_world_with(|s| s.settings.available_accounts.clear());
        let why = enable_control(&none, &["ACC".into()]).action.unwrap_err();
        assert!(why.contains("No WoW account folder"));
    }

    #[test]
    fn restore_says_the_current_copy_is_kept() {
        let w = world();
        let mut r = unit(UnitStatus::InSync {
            from: Some("Den PC".into()),
        });
        r.history = vec![Copy {
            checkpoint: Sha256([4; 32]),
            label: "Tuesday's copy".into(),
        }];
        let rs = restore_controls(&w, &r);
        assert_eq!(rs[0].0.label, "Restore Tuesday's copy…");
        assert_eq!(
            rs[0].1,
            "Restore Tuesday's copy for account ERIC? Your current settings are kept as a recovery copy."
        );
        assert!(matches!(
            rs[0].0.action,
            Ok(GroupAction::RestoreSettings { .. })
        ));
    }

    fn snapshot_world_with(edit: impl FnOnce(&mut wow_sync_runtime::StatusSnapshot)) -> World {
        use wow_sync_runtime::{PausedFolder, PeerRow, PeerStanding};
        let mut s = wow_sync_runtime::StatusSnapshot::default();
        let this = EnrollmentRef {
            computer_id: ComputerId::parse("11111111111111111111111111111111").unwrap(),
            enrollment_id: EnrollmentId::parse("11111111111111111111111111111111").unwrap(),
        };
        s.peers = vec![
            PeerRow {
                enrollment: this.clone(),
                name: "Desk".into(),
                standing: PeerStanding::Active,
                retired_at_ms: None,
                is_this: true,
            },
            PeerRow {
                enrollment: den(),
                name: "Den PC".into(),
                standing: PeerStanding::Active,
                retired_at_ms: None,
                is_this: false,
            },
        ];
        s.settings.enabled = true;
        s.settings.master = Some(den());
        s.settings.master_name = Some("Den PC".into());
        s.settings.paused = vec![
            PausedFolder {
                folder: folder("Details"),
                here_changed: true,
                shared_newer: false,
            },
            PausedFolder {
                folder: folder("Bagnon"),
                here_changed: false,
                shared_newer: true,
            },
        ];
        edit(&mut s);
        let mut w = world();
        crate::features::apply_snapshot(&mut w.features, &s);
        w
    }

    fn snapshot_world() -> World {
        snapshot_world_with(|_| {})
    }

    #[test]
    fn the_snapshot_drives_master_choice_and_the_precise_resume_sheet() {
        let w = snapshot_world();
        assert_eq!(w.features.master.as_deref(), Some("Den PC"));
        let labels: Vec<String> = master_controls(&w).into_iter().map(|c| c.label).collect();
        assert_eq!(labels, ["Stop sharing with a master"]);
        let p = &w.features.settings.paused;
        let a = resume_controls(&w, &p[0].folder, p[0].here_changed, p[0].shared_newer);
        let b = resume_controls(&w, &p[1].folder, p[1].here_changed, p[1].shared_newer);
        let la: Vec<_> = a.iter().map(|c| c.label.clone()).collect();
        let lb: Vec<_> = b.iter().map(|c| c.label.clone()).collect();
        assert_ne!(la, lb, "each folder meets its own differences");
        assert_eq!(
            la,
            resume_controls(&w, &folder("Details"), true, false)
                .iter()
                .map(|c| c.label.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn two_competing_masters_choose_nothing_and_block_exceptions_with_their_reason() {
        let w = snapshot_world_with(|s| {
            s.settings.master = None;
            s.settings.master_name = None;
            s.settings.policy_conflict = true;
        });
        let c = exception_controls(&w, &folder("Details"));
        assert_eq!(c.len(), 1);
        assert!(c[0].action.as_ref().unwrap_err().contains("two computers"));
        assert!(
            master_controls(&w).iter().all(|c| c.action.is_ok()),
            "the owner may still choose one master"
        );
    }

    #[test]
    fn a_retired_master_is_not_shown_as_sharing_and_nothing_replaces_it() {
        use wow_sync_runtime::PeerStanding;
        let w = snapshot_world();
        assert_eq!(
            banner_text(&w),
            "Den PC shares its add-on settings with the group. Settings are never merged."
        );
        assert!(active_master(&w).is_some());
        let w = snapshot_world_with(|s| {
            for p in &mut s.peers {
                if !p.is_this {
                    p.standing = PeerStanding::Retired;
                }
            }
        });
        assert!(active_master(&w).is_none());
        let t = banner_text(&w);
        assert!(t.contains("no active master") && !t.contains("shares its add-on settings"));
        let c = exception_controls(&w, &folder("Details"));
        assert_eq!(c.len(), 1);
        assert!(c[0]
            .action
            .as_ref()
            .unwrap_err()
            .contains("no longer an active computer"));
        assert!(
            master_controls(&w)
                .iter()
                .all(|c| !matches!(&c.action, Ok(GroupAction::SetSettingsPolicy { master: Some(m), .. }) if *m == den())),
            "the retired master is never offered or elected"
        );
    }

    #[test]
    fn the_banner_never_waits_for_a_size_limit_and_names_a_missing_consent() {
        use wow_sync_runtime::SettingsUnitStatus as S;
        let (w, _) = controls_for(|_| {}, S::Disabled, true);
        let t = banner_text(&w);
        assert!(!t.contains("size limit"), "{t}");
        assert!(t.contains("shares its add-on settings"), "{t}");
        let (w, _) = controls_for(|_| {}, S::Disabled, false);
        assert!(banner_text(&w).contains("settings sync is off on this computer"));
    }

    #[test]
    fn a_fresh_consent_with_no_units_says_the_master_shares_and_no_limit_is_awaited() {
        let w = snapshot_world_with(|s| {
            s.settings.enabled = true;
            s.settings_units.clear();
        });
        assert!(w.features.settings.units.is_empty());
        let t = banner_text(&w);
        assert!(t.contains("shares its add-on settings"), "{t}");
        assert!(!t.contains("size limit"), "{t}");
        let off = snapshot_world_with(|s| s.settings.enabled = false);
        assert!(banner_text(&off).contains("settings sync is off on this computer"));
    }

    fn runtime_row(
        status: wow_sync_runtime::SettingsUnitStatus,
    ) -> wow_sync_runtime::SettingsUnitRow {
        let issuer = wow_sync_core::kinds::Issuer {
            computer_id: den().computer_id,
            enrollment_id: den().enrollment_id,
            issued_at_seq: 1,
        };
        let entry = |n: u8, head: bool| wow_sync_runtime::SettingsHistoryEntry {
            hash: Sha256([n; 32]),
            kind: wow_sync_core::kinds::RevisionKind::Auto,
            checkpoint: Sha256([n; 32]),
            issuer: issuer.clone(),
            written_unix_ms: 1_700_000_000_000,
            is_head: head,
            origin: wow_sync_runtime::HeadKind::SharedFrom,
            source: None,
            size_bytes: None,
            seq: 0,
        };
        wow_sync_runtime::SettingsUnitRow {
            unit: unit(UnitStatus::Off).unit,
            status,
            live: false,
            competing: vec![],
            shown_heads: vec![],
            history: vec![entry(4, false), entry(5, true)],
            restore_possible: true,
            latest_capture: Some(Sha256([9; 32])),
            can_share: true,
            waiting_for: None,
            size_bytes: None,
            shared_with: Vec::new(),
            in_sync_from: None,
            local_copies: Vec::new(),
            recovery_copies: Vec::new(),
            local_reason: None,
        }
    }

    fn controls_for(
        edit: impl FnOnce(&mut wow_sync_runtime::SettingsUnitRow),
        status: wow_sync_runtime::SettingsUnitStatus,
        enabled: bool,
    ) -> (World, UnitRow) {
        let mut r = runtime_row(status);
        edit(&mut r);
        let w = snapshot_world_with(|s| {
            s.settings.enabled = enabled;
            s.settings_units = vec![r];
        });
        let row = w.features.settings.units[0].clone();
        (w, row)
    }

    #[test]
    fn the_units_list_wires_share_choose_and_restore_from_the_snapshot() {
        use wow_sync_runtime::{HeadChoice, SettingsUnitStatus as S};
        let (w, row) = controls_for(|r| r.live = true, S::Active, true);
        assert_eq!(row.status, UnitStatus::Active);
        assert_eq!(
            unit_controls(&w, &row)[0].action,
            Ok(GroupAction::ShareSettings {
                unit: row.unit.clone(),
                checkpoint: Sha256([9; 32])
            })
        );
        let rs = restore_controls(&w, &row);
        assert_eq!(rs.len(), 1, "the current head is not offered for restore");
        assert_eq!(
            rs[0].0.action,
            Ok(GroupAction::RestoreSettings {
                unit: row.unit.clone(),
                checkpoint: Sha256([4; 32])
            })
        );
        let (w, row) = controls_for(
            |r| {
                r.competing = vec![
                    HeadChoice {
                        checkpoint: Sha256([4; 32]),
                        revisions: vec![Sha256([4; 32])],
                        source: None,
                        kind: wow_sync_runtime::HeadKind::SharedFrom,
                        written_unix_ms: 0,
                        size_bytes: None,
                    },
                    HeadChoice {
                        checkpoint: Sha256([5; 32]),
                        revisions: vec![Sha256([5; 32])],
                        source: None,
                        kind: wow_sync_runtime::HeadKind::SharedFrom,
                        written_unix_ms: 0,
                        size_bytes: None,
                    },
                ];
                r.shown_heads = vec![Sha256([4; 32]), Sha256([5; 32]), Sha256([6; 32])];
            },
            S::NeedsChoice,
            true,
        );
        let cs = unit_controls(&w, &row);
        assert_eq!(cs.len(), 2);
        assert_eq!(
            cs[1].action,
            Ok(GroupAction::ChooseSettings {
                unit: row.unit.clone(),
                checkpoint: Sha256([5; 32]),
                shown_heads: vec![Sha256([4; 32]), Sha256([5; 32]), Sha256([6; 32])],
            })
        );
    }

    #[test]
    fn units_that_cannot_share_or_restore_say_why() {
        use wow_sync_runtime::SettingsUnitStatus as S;
        let (w, row) = controls_for(
            |r| {
                r.can_share = false;
                r.restore_possible = false;
            },
            S::Active,
            true,
        );
        assert!(unit_controls(&w, &row)[0].action.is_err());
        assert!(restore_controls(&w, &row)[0].0.action.is_err());
        let (w, row) = controls_for(|r| r.restore_possible = false, S::Disabled, true);
        assert_eq!(row.status, UnitStatus::Off);
        assert_eq!(status_text(&row.status), "Settings not shared");
        assert!(unit_controls(&w, &row).is_empty());
        let (_, retired) = controls_for(|_| {}, S::Retired, true);
        assert_eq!(retired.status, UnitStatus::Retired);
    }

    fn offer_world(edit: impl FnOnce(&mut wow_sync_runtime::StatusSnapshot)) -> World {
        let mut w = snapshot_world_with(|s| {
            s.settings.enabled = false;
            s.settings.master = None;
            s.settings.master_name = None;
            s.settings.available_accounts = vec!["ACC".into(), "ZED".into()];
            edit(s);
        });
        w.view.headline = wow_sync_core::status::Headline::Ok;
        w.checking = false;
        w
    }

    #[test]
    fn the_offer_follows_policy_and_consent_and_never_shows_when_it_cannot_act() {
        let w = offer_world(|_| {});
        assert_eq!(offer(&w), Some(Offer::Share));
        assert_eq!(
            Offer::Share.question(),
            "Share add-on settings between your computers?"
        );
        let other = offer_world(|s| {
            s.settings.master = Some(den());
            s.settings.master_name = Some("Den PC".into());
        });
        let o = offer(&other).unwrap();
        assert_eq!(
            o.question(),
            "Den PC is now the settings master. Use its add-on settings here?"
        );
        assert_eq!(o.confirm_label(), "Use settings");
        assert_eq!(offer(&offer_world(|s| s.settings.enabled = true)), None);
        assert_eq!(
            offer(&offer_world(|s| s.settings.policy_conflict = true)),
            None
        );
        assert_eq!(
            offer(&offer_world(|s| s.settings.available_accounts.clear())),
            None
        );
        // A master that is no longer an active computer is not offered.
        let gone = offer_world(|s| {
            s.settings.master = Some(EnrollmentRef {
                computer_id: ComputerId::parse("44444444444444444444444444444444").unwrap(),
                enrollment_id: EnrollmentId::parse("55555555555555555555555555555555").unwrap(),
            });
        });
        assert_eq!(offer(&gone), None);
    }

    #[test]
    fn not_now_and_busy_headlines_hide_only_the_headline_offer() {
        let w = offer_world(|_| {});
        assert_eq!(headline_offer(&w, false), Some(Offer::Share));
        assert_eq!(headline_offer(&w, true), None);
        assert_eq!(offer(&w), Some(Offer::Share));
        let mut waiting = w.clone();
        waiting.view.headline = wow_sync_core::status::Headline::Waiting;
        assert_eq!(headline_offer(&waiting, false), None);
        assert_eq!(offer(&waiting), Some(Offer::Share));
    }

    #[test]
    fn the_consent_ticks_every_account_and_picks_this_computer_by_default() {
        let w = offer_world(|_| {});
        let choices = master_choices(&w);
        assert_eq!(choices[0].label, format!("{} (this computer)", w.computer));
        assert_eq!(choices[1].label, "Den PC");
        let all = ["ACC".to_string(), "ZED".to_string()];
        let c = start_control(&w, &Offer::Share, &all, None, false);
        assert_eq!(
            c.action,
            Ok(GroupAction::StartSettings {
                master: Some(choices[0].enrollment.clone()),
                exceptions: vec![],
                included_accounts: all.to_vec(),
                share_now: vec![],
                share_when_ready: false,
            })
        );
        let den_pick = start_control(&w, &Offer::Share, &["ZED".into()], Some(1), false);
        assert_eq!(
            den_pick.action,
            Ok(GroupAction::StartSettings {
                master: Some(den()),
                exceptions: vec![],
                included_accounts: vec!["ZED".into()],
                share_now: vec![],
                share_when_ready: false,
            })
        );
        assert_eq!(
            start_control(&w, &Offer::Share, &[], None, false).action,
            Err("Choose at least one account.".into())
        );
        assert!(CONSENT_TEXT.starts_with("All addon settings on this computer are shared"));
    }

    #[test]
    fn confirming_a_master_chosen_elsewhere_sets_no_master() {
        let w = offer_world(|s| s.settings.master = Some(den()));
        let o = offer(&w).unwrap();
        let c = start_control(&w, &o, &["ACC".into(), "NOPE".into()], None, false);
        assert_eq!(
            c.action,
            Ok(GroupAction::StartSettings {
                master: None,
                exceptions: vec![],
                included_accounts: vec!["ACC".into()],
                share_now: vec![],
                share_when_ready: false,
            })
        );
        let GroupAction::StartSettings {
            included_accounts, ..
        } = c.action.unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            GroupAction::StartSettings {
                master: None,
                exceptions: vec![],
                included_accounts: included_accounts.clone(),
                share_now: vec![],
                share_when_ready: false,
            }
            .to_command(),
            wow_sync_runtime::Command::EnableSettings { included_accounts }
        );
    }

    #[test]
    fn a_retired_computer_is_not_offered_settings() {
        let mut w = offer_world(|_| {});
        w.features.standing = Standing::Retired {
            on: "1 Jan".into(),
            by: "Den PC".into(),
        };
        assert_eq!(offer(&w), None);
    }

    #[test]
    fn a_pending_settings_choice_shows_in_the_headline_and_needs_attention() {
        let mut w = world();
        w.enrolled = true;
        w.view.headline = wow_sync_core::status::Headline::Ok;
        w.features.settings.enabled = true;
        assert_eq!(
            crate::model::headline(&w).text,
            "This computer is up to date"
        );
        assert!(crate::model::need_items(&w).is_empty());
        w.features.settings.units = vec![unit(UnitStatus::NeedsChoice)];
        let h = crate::model::headline(&w);
        assert_eq!(h.text, "Choose which saved settings to keep");
        assert_eq!(h.tone, crate::theme::Tone::Help);
        let items = crate::model::need_items(&w);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].target, crate::model::NeedTarget::SettingsChoice);
        assert!(items[0]
            .title
            .ends_with("choose which saved settings to keep"));
    }

    #[test]
    fn the_new_attention_statuses_read_in_plain_words() {
        assert_eq!(
            status_text(&UnitStatus::TooLarge {
                size: Some(5_242_880)
            }),
            "Too large to share automatically (5.0 MB)"
        );
        assert_eq!(
            status_text(&UnitStatus::OwnedBySeveral {
                with: vec!["Alpha".into(), "Beta".into()]
            }),
            "Shared with Alpha, Beta, not synced"
        );
        assert_eq!(
            status_text(&UnitStatus::WaitingForWow),
            "New settings will be used when WoW closes"
        );
        assert_eq!(
            status_text(&UnitStatus::WaitingForSharedCopy),
            "Waiting for the shared settings to arrive"
        );
        assert!(urgency(&UnitStatus::WaitingForSharedCopy) > urgency(&UnitStatus::NoSharedCopy));
        let mut w = world();
        w.features.settings.enabled = true;
        w.features.settings.units = vec![unit(UnitStatus::InSync { from: None }), {
            let mut u = unit(UnitStatus::WaitingForSharedCopy);
            u.unit.identity = Identity::account("ZED").unwrap();
            u
        }];
        let line = addon_settings_line(&w, "Details");
        assert_eq!(line, "Waiting for the shared settings to arrive");
        assert!(!line.contains("up to date"));
        assert!(urgency(&UnitStatus::TooLarge { size: None }) > urgency(&UnitStatus::NoSharedCopy));
    }

    #[test]
    fn the_cycle_note_reads_as_a_sentence_and_is_absent_without_one() {
        let mut w = world();
        assert_eq!(settings_note_text(&w), None);
        w.settings_note = Some(format!("{SETTINGS_NOTE_PREFIX}: Details is too large"));
        assert_eq!(
            settings_note_text(&w).as_deref(),
            Some("Some saved settings were not shared: Details is too large.")
        );
    }
    #[test]
    fn add_on_status_lines_say_the_most_urgent_thing_in_plain_words() {
        let mut w = world();
        assert_eq!(addon_settings_line(&w, "Details"), "Settings not shared");
        w.features.settings.enabled = true;
        assert_eq!(addon_settings_line(&w, "Details"), "Not shared yet");
        w.features.settings.units = vec![
            unit(UnitStatus::InSync {
                from: Some("Den PC".into()),
            }),
            {
                let mut u = unit(UnitStatus::WaitingForIdentity {
                    who: Some("character A1".into()),
                });
                u.unit.identity = Identity::account("ZED").unwrap();
                u
            },
        ];
        assert_eq!(
            addon_settings_line(&w, "Details"),
            "Waiting for a character to log in on this computer"
        );
        assert_eq!(
            addon_settings_lines(&w, "Details"),
            [
                "account ERIC: Settings up to date (from Den PC)",
                "account ZED: Waiting for character A1 to log in on this computer",
            ]
        );
        assert_eq!(addon_settings_line(&w, "Bagnon"), "Not shared yet");
    }

    #[test]
    fn two_add_ons_declaring_the_same_variables_both_read_as_shared_not_synced() {
        let mut w = world();
        w.features.settings.enabled = true;
        w.addons = vec![
            test_addon("alpha", "Alpha Tools"),
            test_addon("beta", "Beta"),
        ];
        let mut a = named(
            "Alpha Tools",
            UnitStatus::OwnedBySeveral {
                with: vec!["Beta".into()],
            },
            1,
        );
        a.unit.folder = folder("alpha");
        w.features.settings.units = vec![a];
        assert_eq!(
            addon_settings_line(&w, "alpha"),
            "Shared with Beta, not synced"
        );
        assert_eq!(
            addon_settings_line(&w, "beta"),
            "Shared with Alpha Tools, not synced"
        );
        assert_ne!(addon_settings_line(&w, "beta"), "Not shared yet");
    }

    #[test]
    fn rows_the_runtime_emits_for_two_blocked_declarers_both_read_as_shared_not_synced() {
        use wow_sync_runtime::SettingsUnitStatus as S;
        let mut w = world();
        w.features.settings.enabled = true;
        w.addons = vec![
            test_addon("checksharedone", "CheckSharedOne"),
            test_addon("checksharedtwo", "CheckSharedTwo"),
        ];
        let row = |folder_key: &str, other: &str| {
            let mut r = runtime_row(S::OwnedBySeveral);
            r.unit.folder = folder(folder_key);
            r.shared_with = vec![other.into()];
            r
        };
        w.features.settings.units = crate::features::settings_unit_rows(&[
            row("checksharedone", "CheckSharedTwo"),
            row("checksharedtwo", "CheckSharedOne"),
        ]);
        assert_eq!(
            addon_settings_line(&w, "checksharedone"),
            "Shared with CheckSharedTwo, not synced"
        );
        assert_eq!(
            addon_settings_line(&w, "checksharedtwo"),
            "Shared with CheckSharedOne, not synced"
        );
    }

    #[test]
    fn shared_checkpoint_entries_show_source_kind_size_and_tell_same_minute_copies_apart() {
        use wow_sync_runtime::HeadKind as K;
        let base = runtime_row(wow_sync_runtime::SettingsUnitStatus::InSync).history[0].clone();
        let mk = |n: u8, origin: K, source: &str, ms: u64, seq: u64| {
            let mut e = base.clone();
            e.hash = Sha256([n; 32]);
            e.checkpoint = Sha256([n; 32]);
            e.is_head = false;
            e.origin = origin;
            e.source = Some(source.into());
            e.size_bytes = Some(2048);
            e.written_unix_ms = ms;
            e.seq = seq;
            e
        };
        let t = 1_700_000_000_000;
        let copies = crate::features::history_copies(&[
            mk(1, K::MastersRevision, "Desk", t, 7),
            mk(2, K::SharedFrom, "Laptop", t + 3_000, 2),
            mk(3, K::SharedFrom, "Laptop", t + 3_000, 3),
        ]);
        assert!(
            copies[0].label.contains("settings master Desk"),
            "{copies:?}"
        );
        assert!(copies[0].label.contains("2 KB"), "{copies:?}");
        assert!(copies[1].label.contains("shared from Laptop"));
        assert!(copies[1].label.ends_with("earlier"), "{copies:?}");
        assert!(copies[2].label.ends_with("later"), "{copies:?}");
        assert_ne!(copies[0].label, copies[1].label);
        let secs = crate::world::date_time_seconds_text(t);
        let secs2 = crate::world::date_time_seconds_text(t + 3_000);
        assert_ne!(secs, secs2);
        assert!(copies[0].label.contains(&secs), "{copies:?}");
    }

    #[test]
    fn competing_choices_name_their_source_kind_time_and_size() {
        use crate::features::{head_label, recovery_label};
        use wow_sync_runtime::{HeadKind as H, RecoveryKind as R};
        let d = "25 November 14:32, 1 KB";
        let a = head_label(H::MastersRevision, Some("Desk"), d);
        let b = head_label(H::ThisComputersCopy, None, d);
        let c = head_label(H::SharedFrom, Some("Laptop"), d);
        assert!(a.contains("settings master Desk") && a.contains(d));
        assert!(b.contains("this computer"));
        assert!(c.contains("shared from Laptop"));
        assert!(a != b && b != c && a != c);
        let r1 = recovery_label(R::YieldedToMaster, Some("Desk"), d);
        let r2 = recovery_label(R::PeerReplaced, Some("Laptop"), d);
        assert!(r1 != r2 && r1.contains(d) && r2.contains("Laptop"));
    }

    #[test]
    fn pause_all_toggles_to_resume_all_and_keeps_the_per_add_on_pause() {
        let mut w = world();
        let c = pause_all_control(&w);
        assert_eq!(c.label, "Pause all settings");
        assert_eq!(c.action, Ok(GroupAction::PauseAllSettings));
        w.features.settings.paused_all = true;
        let c = pause_all_control(&w);
        assert_eq!(c.label, "Resume all settings");
        assert_eq!(c.action, Ok(GroupAction::ResumeAllSettings));
        assert_eq!(
            GroupAction::PauseAllSettings.to_command(),
            wow_sync_runtime::Command::PauseAllSettings
        );
        assert_eq!(
            GroupAction::ResumeAllSettings.to_command(),
            wow_sync_runtime::Command::ResumeAllSettings
        );
        assert!(pause_control(&w, &folder("bagnon")).action.is_ok());
    }

    fn test_addon(key: &str, title: &str) -> crate::world::Addon {
        crate::world::Addon {
            key: key.into(),
            title: title.into(),
            version: String::new(),
            target_version: None,
            folders: vec![],
            kind: crate::world::Kind::Available,
            candidates: vec![],
            why: String::new(),
            next: wow_sync_core::view::NextAction::Nothing,
            awaiting: vec![],
            waiting_on: vec![],
            waiting_secs: None,
        }
    }

    fn me() -> EnrollmentRef {
        EnrollmentRef {
            computer_id: ComputerId::parse("44444444444444444444444444444444").unwrap(),
            enrollment_id: EnrollmentId::parse("55555555555555555555555555555555").unwrap(),
        }
    }

    fn named(name: &str, status: UnitStatus, cp: u8) -> UnitRow {
        let mut r = unit(status);
        r.addon = name.into();
        r.unit.folder = folder(name);
        r.latest_capture = Some(Sha256([cp; 32]));
        r
    }

    #[test]
    fn a_damaged_settings_file_is_said_in_plain_words() {
        assert_eq!(
            status_text(&UnitStatus::Suspect),
            "This add-on's settings file looks damaged; it was not shared"
        );
    }

    #[test]
    fn share_these_now_lists_only_differing_or_unshared_units_and_is_off_unless_first_adoption() {
        let mut w = world();
        w.features.this_enrollment = Some(me());
        w.features.settings.group_has_shared_settings = true;
        w.features.settings.to_share = vec![
            ShareItem {
                unit: named("Bagnon", UnitStatus::Active, 1).unit,
                not_shared_yet: true,
            },
            ShareItem {
                unit: named("Details", UnitStatus::Active, 2).unit,
                not_shared_yet: false,
            },
        ];
        let c = share_candidates(&w);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].line, "bagnon (account ERIC) has no shared copy yet");
        assert!(c[1].line.ends_with("differs from the shared copy"));
        // The group already has shared settings: off by default.
        assert!(!first_adoption(&w));
        assert!(!share_now_on(&w, false) && share_now_on(&w, true));
        assert_eq!(pushes(&c, false), vec![]);
        assert_eq!(pushes(&c, true).len(), 2);
        // Nothing shared anywhere yet: first adoption, on by default.
        w.features.settings.group_has_shared_settings = false;
        assert!(first_adoption(&w) && share_now_on(&w, false) && !share_now_on(&w, true));
    }

    fn with_one_to_share(w: &mut World) {
        w.features.settings.to_share = vec![ShareItem {
            unit: named("Bagnon", UnitStatus::Active, 7).unit,
            not_shared_yet: true,
        }];
    }

    #[test]
    fn making_this_computer_master_pushes_only_what_the_box_ticks() {
        let mut w = world();
        w.features.this_enrollment = Some(me());
        with_one_to_share(&mut w);
        let c = make_master_control(&w, pushes(&share_candidates(&w), true));
        let Ok(GroupAction::SetSettingsPolicy {
            master, share_now, ..
        }) = c.action
        else {
            panic!("not a policy")
        };
        assert_eq!(master, Some(me()));
        assert_eq!(share_now, vec![named("Bagnon", UnitStatus::Active, 7).unit]);
        let none = make_master_control(&w, pushes(&share_candidates(&w), false));
        assert!(matches!(
            none.action,
            Ok(GroupAction::SetSettingsPolicy { ref share_now, .. }) if share_now.is_empty()
        ));
        // The direct list never makes this computer master without the confirmation.
        assert!(master_controls(&w).iter().all(|c| !matches!(
            &c.action,
            Ok(GroupAction::SetSettingsPolicy { master: Some(m), .. }) if *m == me()
        )));
    }

    #[test]
    fn the_guided_consent_asks_to_share_once_ready_only_for_this_master() {
        let w = offer_world(|_| {});
        let all = ["ACC".to_string()];
        let ready = |c: Control| match c.action {
            Ok(GroupAction::StartSettings {
                share_when_ready, ..
            }) => share_when_ready,
            other => panic!("{other:?}"),
        };
        assert!(first_adoption(&w));
        assert!(ready(start_control(&w, &Offer::Share, &all, None, true)));
        assert!(!ready(start_control(&w, &Offer::Share, &all, None, false)));
        assert!(!ready(start_control(
            &w,
            &Offer::Share,
            &all,
            Some(1),
            true
        )));
        let line = will_share_line(&w).unwrap();
        assert!(line.contains("will be shared once WoW Sync has read them"));
        let mut on = w.clone();
        on.features.settings.enabled = true;
        assert_eq!(will_share_line(&on), None);
    }

    #[test]
    fn the_guided_consent_pushes_only_when_this_computer_is_the_master() {
        let mut w = offer_world(|_| {});
        with_one_to_share(&mut w);
        let all = ["ACC".to_string()];
        let push = |c: Control| match c.action {
            Ok(GroupAction::StartSettings { share_now, .. }) => share_now,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            push(start_control(&w, &Offer::Share, &all, None, true)).len(),
            1
        );
        assert!(push(start_control(&w, &Offer::Share, &all, None, false)).is_empty());
        assert!(push(start_control(&w, &Offer::Share, &all, Some(1), true)).is_empty());
    }

    #[test]
    fn this_computers_replaced_copy_can_be_restored() {
        let w = world();
        let mut r = unit(UnitStatus::Active);
        r.local_copies = vec![Copy {
            checkpoint: Sha256([5; 32]),
            label: "copy from 1 Jan".into(),
        }];
        let rs = restore_controls(&w, &r);
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].0.label, "Restore this computer's copy from 1 Jan…");
        assert!(rs[0]
            .1
            .starts_with("Restore this computer's copy from 1 Jan for "));
        assert_eq!(
            rs[0].0.action,
            Ok(GroupAction::RestoreSettings {
                unit: r.unit.clone(),
                checkpoint: Sha256([5; 32]),
            })
        );
        assert!(rs[0].1.contains("kept as a recovery copy"));
    }

    #[test]
    fn the_runtimes_new_statuses_and_local_copies_reach_the_controls() {
        use wow_sync_runtime::SettingsUnitStatus as S;
        let map = |row: wow_sync_runtime::SettingsUnitRow| {
            crate::features::settings_unit_rows(&[row]).remove(0)
        };
        let mut r = runtime_row(S::LocalDifferent(LocalReason::LocalChange));
        r.local_copies = vec![wow_sync_runtime::SettingsHistoryEntry {
            hash: Sha256([8; 32]),
            ..runtime_row(S::Active).history[0].clone()
        }];
        let row = map(r);
        assert_eq!(
            row.status,
            UnitStatus::LocalDifferent(LocalReason::LocalChange)
        );
        let w = world();
        let labels: Vec<String> = unit_controls(&w, &row)
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["Share my settings", "Use shared settings"]);
        let restore = restore_controls(&w, &row);
        let mine = restore.last().unwrap();
        assert!(mine
            .0
            .label
            .starts_with("Restore this computer's copy from"));
        assert_eq!(
            mine.0.action,
            Ok(GroupAction::RestoreSettings {
                unit: row.unit.clone(),
                checkpoint: Sha256([8; 32]),
            })
        );
        assert_eq!(map(runtime_row(S::Suspect)).status, UnitStatus::Suspect);
        assert_eq!(
            map(runtime_row(S::NoSharedCopy)).status,
            UnitStatus::NoSharedCopy
        );
        let mut s = runtime_row(S::InSync);
        s.in_sync_from = Some("Den PC".into());
        assert_eq!(
            map(s).status,
            UnitStatus::InSync {
                from: Some("Den PC".into())
            }
        );
        assert_eq!(
            map(runtime_row(S::InSync)).status,
            UnitStatus::InSync { from: None }
        );
    }
}
