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
    /// Consent is given but no size limit is approved (Q-77), so nothing is read or applied.
    SizeLimitPending,
    /// Syncing here right now.
    Active,
    InSync {
        from: String,
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
    pub restore_possible: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsData {
    pub units: Vec<UnitRow>,
    pub exceptions: Vec<PolicyRule>,
    pub master: Option<EnrollmentRef>,
    pub accounts: Vec<String>,
    pub enabled: bool,
    /// Competing master choices exist; nothing is chosen for the owner.
    pub policy_conflict: bool,
    /// Paused folders with what a resume would meet.
    pub paused: Vec<wow_sync_runtime::PausedFolder>,
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
        UnitStatus::SizeLimitPending => {
            "Settings sharing is off until a size limit is approved".into()
        }
        UnitStatus::Active => "Settings are syncing here".into(),
        UnitStatus::InSync { from } => format!("Settings up to date (from {from})"),
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
        UnitStatus::Suspect => "Settings may be incomplete after WoW stopped".into(),
        UnitStatus::CaptureFailed => "Settings could not be saved".into(),
        UnitStatus::NotShared => "Not shared (your choice)".into(),
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
        UnitStatus::LocalDifferent(reason) => {
            share(&mut out);
            if *reason != LocalReason::LocalChange {
                out.push(control(
                    w,
                    ActionKind::UseSharedSettings,
                    "Use shared settings",
                    GroupAction::UseSharedSettings {
                        unit: row.unit.clone(),
                    },
                ));
            }
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
    row.history
        .iter()
        .map(|c| {
            let confirm = format!(
                "Restore {} for {}? Your current settings are kept as a recovery copy.",
                c.label,
                identity_words(&row.unit)
            );
            let label = format!("Restore {}…", c.label);
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
        .map(|(name, e)| {
            control(
                w,
                ActionKind::SetSettingsPolicy,
                format!("Use {name} as the settings master"),
                GroupAction::SetSettingsPolicy {
                    master: Some(e.clone()),
                    exceptions: d.exceptions.clone(),
                },
            )
        })
        .collect();
    if out.is_empty() && d.master.is_none() {
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

/// The sentence at the top of the Settings sync page.
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
        if w.features
            .settings
            .units
            .iter()
            .any(|u| u.status == UnitStatus::SizeLimitPending)
        {
            return format!(
                "{name} is the settings master, but settings sharing is off until a size limit is approved."
            );
        }
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
            },
        ));
    }
    out
}

/// "Share settings for these accounts" before settings sync is on.
// Waits for the list of accounts to choose from (not in the snapshot).
#[allow(dead_code)]
pub fn enable_control(w: &World, chosen: &[String]) -> Control {
    if chosen.is_empty() {
        return off("Turn on settings sync", "Choose at least one account.");
    }
    control(
        w,
        ActionKind::EnableSettings,
        "Turn on settings sync",
        GroupAction::EnableSettings {
            included_accounts: chosen.to_vec(),
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
                from: "Den PC".into()
            }),
            "Settings up to date (from Den PC)"
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
    fn local_difference_offers_share_and_only_some_offer_use_shared() {
        let w = world();
        let labels = |r| -> Vec<String> {
            unit_controls(&w, &unit(UnitStatus::LocalDifferent(r)))
                .into_iter()
                .map(|c| c.label)
                .collect()
        };
        assert_eq!(labels(LocalReason::LocalChange), ["Share my settings"]);
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
        let Ok(GroupAction::SetSettingsPolicy { master, exceptions }) = &cs[0].action else {
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
        let w = world();
        assert!(enable_control(&w, &[]).action.is_err());
        assert_eq!(
            enable_control(&w, &["ERIC".into()]).action,
            Ok(GroupAction::EnableSettings {
                included_accounts: vec!["ERIC".into()]
            })
        );
    }

    #[test]
    fn restore_says_the_current_copy_is_kept() {
        let w = world();
        let mut r = unit(UnitStatus::InSync {
            from: "Den PC".into(),
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
        assert_eq!(
            labels,
            [
                "Use this computer as the settings master",
                "Stop sharing with a master"
            ]
        );
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
    fn the_banner_does_not_claim_sharing_without_consent_or_a_size_limit() {
        use wow_sync_runtime::SettingsUnitStatus as S;
        let (w, _) = controls_for(|_| {}, S::Disabled, true);
        assert_eq!(
            banner_text(&w),
            "Den PC is the settings master, but settings sharing is off until a size limit is approved."
        );
        let (w, _) = controls_for(|_| {}, S::Disabled, false);
        assert!(banner_text(&w).contains("settings sync is off on this computer"));
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
                    },
                    HeadChoice {
                        checkpoint: Sha256([5; 32]),
                        revisions: vec![Sha256([5; 32])],
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
    fn units_that_cannot_share_or_restore_say_why_and_a_missing_cap_is_not_invented() {
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
        assert_eq!(row.status, UnitStatus::SizeLimitPending);
        assert_eq!(
            status_text(&row.status),
            "Settings sharing is off until a size limit is approved"
        );
        assert!(unit_controls(&w, &row).is_empty());
        let (_, off_row) = controls_for(|_| {}, S::Disabled, false);
        assert_eq!(off_row.status, UnitStatus::Off);
        let (_, retired) = controls_for(|_| {}, S::Retired, true);
        assert_eq!(retired.status, UnitStatus::Retired);
    }
}
