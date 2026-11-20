//! View-model for going back to an earlier release and for what it says afterwards
//! (rollback.md §1-§4, §8). Pure: the rows come from the snapshot (held releases), the words are
//! decided here and the only thing it can produce is a typed `GroupAction`.

use crate::computers_ui::{allow_again, AllowAgainRow};
use crate::features::{gate, ActionKind, GroupAction};
use crate::world::World;
use wow_sync_core::ids::Sha256;
use wow_sync_core::paths::FolderKey;

/// One held release of an add-on in its history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseRow {
    pub package: FolderKey,
    pub title: String,
    pub version: String,
    pub package_hash: Sha256,
    pub current: bool,
    /// Active rejections of this release; non-empty means it is blocked for the group.
    pub rejections: Vec<Sha256>,
    /// Names of the computers that hold this copy.
    pub held_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryButton {
    GoBack(GroupAction),
    AllowAgain(GroupAction),
    Disabled {
        label: &'static str,
        reason: String,
    },
    /// The current release has no recovery action.
    None,
}

pub const GO_BACK: &str = "Go back to this version…";
pub const ALLOW_AGAIN: &str = "Allow again…";

pub fn blocked_chip(r: &ReleaseRow) -> Option<String> {
    (!r.rejections.is_empty()).then(|| format!("Blocked: {}", r.version))
}

/// The right action for a history row, or why it is unavailable (rollback.md §8).
pub fn button(w: &World, r: &ReleaseRow) -> HistoryButton {
    if r.current {
        return HistoryButton::None;
    }
    if !r.rejections.is_empty() {
        let row = AllowAgainRow {
            title: r.title.clone(),
            version: r.version.clone(),
            rejections: r.rejections.clone(),
            on: String::new(),
        };
        return match allow_again(w, &row) {
            Ok(a) => HistoryButton::AllowAgain(a),
            Err(reason) => HistoryButton::Disabled {
                label: ALLOW_AGAIN,
                reason,
            },
        };
    }
    if let Err(reason) = gate(w, ActionKind::Rollback) {
        return HistoryButton::Disabled {
            label: GO_BACK,
            reason,
        };
    }
    if let Some(name) = &w.features.older_peer {
        return HistoryButton::Disabled {
            label: GO_BACK,
            reason: format!(
                "{name} runs an older WoW Sync that can't block versions. Update WoW Sync on {name} first."
            ),
        };
    }
    HistoryButton::GoBack(GroupAction::Rollback {
        package: r.package.clone(),
        recovered: r.package_hash,
    })
}

/// What the confirmation says before anything is queued.
pub fn confirm_text(installed: &str, r: &ReleaseRow) -> Vec<String> {
    vec![
        format!(
            "Go back to {} {}? Every computer in the group replaces version {installed} with {}.",
            r.title, r.version, r.version
        ),
        format!("Version {installed} is blocked for the group. You can allow it again later."),
        "WoW Sync applies this when WoW is closed. Nothing is changed while it is running.".into(),
    ]
}

// The service does not report a rollback outcome or recovery sizes yet; only tests build these.
#[allow(dead_code)]
/// How the settings saved with the earlier version fared (rollback.md §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsOutcome {
    Paired { units: Vec<String> },
    Partial { missing: Vec<String> },
    CodeOnly,
    NotApplicable,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub text: String,
    /// Partial and code-only are warnings, never shown as plain success.
    pub warning: bool,
}

#[allow(dead_code)]
pub fn summary(title: &str, back_to: &str, from: &str, o: &SettingsOutcome) -> Summary {
    match o {
        SettingsOutcome::Paired { units } => Summary {
            text: format!(
                "{title} is back to version {back_to}, with the settings you had with {back_to} ({}).",
                units.join(", ")
            ),
            warning: false,
        },
        SettingsOutcome::NotApplicable => Summary {
            text: format!("{title} is back to version {back_to}."),
            warning: false,
        },
        SettingsOutcome::Partial { missing } => Summary {
            text: format!(
                "{title} is back to version {back_to}. Your settings were not restored for {}, because no copy from version {back_to} is left. Your current settings are kept. They were made with {from} and may not work with {back_to}.",
                missing.join(", ")
            ),
            warning: true,
        },
        SettingsOutcome::CodeOnly => Summary {
            text: format!(
                "{title} is back to version {back_to}. Your settings were not restored, because no copy from version {back_to} is left. Your current settings are kept. They were made with {from} and may not work with {back_to}."
            ),
            warning: true,
        },
    }
}

/// A visible temporary exception, with its size (three-states rule: recovery data is shown).
#[allow(dead_code)]
pub fn exception_line(what: &str, bytes: u64) -> String {
    let size = if bytes >= 1 << 20 {
        format!("{:.1} MB", bytes as f64 / (1u64 << 20) as f64)
    } else {
        format!("{} KB", bytes.div_ceil(1024).max(1))
    };
    format!("{what} is kept temporarily ({size}) until it can be resolved.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Capabilities, Standing, RETIRED_REASON};

    fn key() -> FolderKey {
        wow_sync_core::paths::validate_folder_name("Details").unwrap()
    }

    fn row(version: &str, current: bool, blocked: bool) -> ReleaseRow {
        ReleaseRow {
            package: key(),
            title: "Details".into(),
            version: version.into(),
            package_hash: Sha256([5; 32]),
            current,
            rejections: if blocked {
                vec![Sha256([8; 32])]
            } else {
                vec![]
            },
            held_by: "Den PC".into(),
        }
    }

    fn world() -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.features.caps = Capabilities::runtime();
        w
    }

    #[test]
    fn each_state_gets_the_right_action() {
        let w = world();
        assert_eq!(button(&w, &row("2.1", true, false)), HistoryButton::None);
        assert_eq!(
            button(&w, &row("2.0", false, false)),
            HistoryButton::GoBack(GroupAction::Rollback {
                package: key(),
                recovered: Sha256([5; 32]),
            })
        );
        assert_eq!(
            button(&w, &row("2.1", false, true)),
            HistoryButton::AllowAgain(GroupAction::AllowAgain {
                rejections: vec![Sha256([8; 32])]
            })
        );
        assert_eq!(
            blocked_chip(&row("2.1", false, true)).unwrap(),
            "Blocked: 2.1"
        );
        assert_eq!(blocked_chip(&row("2.0", false, false)), None);
    }

    #[test]
    fn go_back_is_disabled_with_a_reason_when_it_cannot_be_planned() {
        let mut w = world();
        w.features.older_peer = Some("Laptop".into());
        let HistoryButton::Disabled { label, reason } = button(&w, &row("2.0", false, false))
        else {
            panic!("disabled");
        };
        assert_eq!(label, GO_BACK);
        assert_eq!(
            reason,
            "Laptop runs an older WoW Sync that can't block versions. Update WoW Sync on Laptop first."
        );
        let mut w = world();
        w.features.standing = Standing::Retired {
            on: "1 Jan".into(),
            by: "Den PC".into(),
        };
        let HistoryButton::Disabled { reason, .. } = button(&w, &row("2.0", false, false)) else {
            panic!("disabled");
        };
        assert_eq!(reason, RETIRED_REASON);
        let HistoryButton::Disabled { label, .. } = button(&w, &row("2.1", false, true)) else {
            panic!("allow again is group-writing too");
        };
        assert_eq!(label, ALLOW_AGAIN);
    }

    #[test]
    fn partial_and_code_only_are_warnings_and_paired_is_not() {
        let paired = summary(
            "Details",
            "2.0",
            "2.1",
            &SettingsOutcome::Paired {
                units: vec!["Thrall on Argent Dawn".into()],
            },
        );
        assert!(!paired.warning);
        assert!(paired.text.contains("with the settings you had with 2.0"));
        let partial = summary(
            "Details",
            "2.0",
            "2.1",
            &SettingsOutcome::Partial {
                missing: vec!["Thrall on Argent Dawn".into()],
            },
        );
        assert!(partial.warning && partial.text.contains("not restored for Thrall"));
        let code = summary("Details", "2.0", "2.1", &SettingsOutcome::CodeOnly);
        assert!(code.warning);
        assert_ne!(code.text, partial.text);
        assert_ne!(paired.text, code.text);
        let none = summary("Details", "2.0", "2.1", &SettingsOutcome::NotApplicable);
        assert!(!none.warning && !none.text.contains("settings"));
    }

    #[test]
    fn the_confirmation_says_what_is_blocked_and_when_it_applies() {
        let t = confirm_text("2.1", &row("2.0", false, false));
        assert!(t[0].starts_with("Go back to Details 2.0?"));
        assert!(t[1].contains("2.1 is blocked"));
        assert!(t[2].contains("when WoW is closed"));
    }

    #[test]
    fn a_temporary_exception_shows_its_size() {
        assert_eq!(
            exception_line("A recovery copy of Details", 3 * 1024 * 1024),
            "A recovery copy of Details is kept temporarily (3.0 MB) until it can be resolved."
        );
        assert!(exception_line("x", 10).contains("1 KB"));
    }
}
