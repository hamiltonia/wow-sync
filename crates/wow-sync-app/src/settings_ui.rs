//! View-model for settings sync: unit status, master and exceptions, share, pause and resume,
//! use shared, choose, restore (settings.md §4-§6, §9). Pure: every control is either a typed
//! `GroupAction` or the reason it is unavailable; nothing here writes.

// Fields and helpers wait for the snapshot's settings rows (pending snapshot field `settings`).
#![allow(dead_code)]

use crate::features::{gate, ActionKind, GroupAction};
use crate::world::World;
use wow_sync_core::ids::{SettingsUnit, Sha256};
use wow_sync_core::kinds::{EnrollmentRef, PolicyRule, PolicyRuleKind};
use wow_sync_core::paths::FolderKey;
use wow_sync_core::settings::{resume_preview, LocalReason, ResumeOption};

/// What the unit is doing here, one per row of settings.md §9.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitStatus {
    Off,
    InSync { from: String },
    Waiting { from: String },
    LocalDifferent(LocalReason),
    NoSharedCopy,
    Paused,
    NeedsChoice,
    PolicyConflict,
    MasterAbsent { name: String },
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
    /// The checkpoint of what is live here, the thing "Share my settings" publishes.
    pub live: Option<Sha256>,
    /// The competing heads of a `NeedsChoice` unit.
    pub heads: Vec<Copy>,
    /// Older saved copies that "Restore" can bring back.
    pub history: Vec<Copy>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsData {
    pub units: Vec<UnitRow>,
    pub exceptions: Vec<PolicyRule>,
    pub master: Option<EnrollmentRef>,
    pub accounts: Vec<String>,
    pub enabled: bool,
    pub paused: Vec<FolderKey>,
}

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
    match &row.status {
        UnitStatus::LocalDifferent(reason) => {
            out.push(match row.live {
                Some(checkpoint) => control(
                    w,
                    ActionKind::ShareSettings,
                    "Share my settings",
                    GroupAction::ShareSettings {
                        unit: row.unit.clone(),
                        checkpoint,
                    },
                ),
                None => off(
                    "Share my settings",
                    "There is nothing saved here to share yet.",
                ),
            });
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
            let shown: Vec<Sha256> = row.heads.iter().map(|h| h.checkpoint).collect();
            for h in &row.heads {
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
            (
                control(
                    w,
                    ActionKind::RestoreSettings,
                    format!("Restore {}…", c.label),
                    GroupAction::RestoreSettings {
                        unit: row.unit.clone(),
                        checkpoint: c.checkpoint,
                    },
                ),
                confirm,
            )
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
    let mut out: Vec<Control> = w
        .features
        .enrollments
        .iter()
        .filter(|(_, e)| d.master.as_ref() != Some(e))
        .map(|(id, e)| {
            let name = w
                .peers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "another computer".into());
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

/// Per add-on: don't share it, take it from another computer, or drop the exception.
pub fn exception_controls(w: &World, folder: &FolderKey) -> Vec<Control> {
    let d = &w.features.settings;
    let Some(master) = d.master.clone() else {
        return vec![off(
            "Add an exception…",
            "Choose a settings master first; exceptions are changes to what it shares.",
        )];
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
    for (_, e) in w.features.enrollments.iter().filter(|(_, e)| *e != master) {
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
            live: Some(Sha256([1; 32])),
            heads: vec![],
            history: vec![],
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
}
