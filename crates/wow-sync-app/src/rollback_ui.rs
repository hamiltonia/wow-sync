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
    /// When this copy was first held, once the snapshot provides it. Pending snapshot field.
    pub held_on: Option<String>,
}

/// The local date a held release was set aside, when the snapshot has one for it.
pub fn held_on(
    dates: &std::collections::BTreeMap<Sha256, u64>,
    release: &Sha256,
) -> Option<String> {
    dates.get(release).map(|ms| crate::world::day_text(*ms))
}

/// Whether a rollback request to `release` of `package` is finished: that release is now the
/// installed one, or the service recorded an outcome for the add-on. (The snapshot has no
/// separate pending-request field.)
pub fn request_done(w: &World, package: &str, release: &Sha256) -> bool {
    w.features
        .history
        .iter()
        .any(|h| h.package.as_str() == package && h.package_hash == *release && h.current)
        || w.features
            .rollback_outcome
            .as_ref()
            .is_some_and(|o| o.package.as_str() == package)
}

/// The version installed on this computer, or `None` when it is not known.
pub fn installed_version(w: &World, package: &str) -> Option<String> {
    w.addon(package)
        .map(|a| a.version.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The history line for a release: version, holder, held date and state.
pub fn release_line(r: &ReleaseRow) -> String {
    let mut line = format!("Version {}", r.version);
    if !r.held_by.is_empty() {
        line.push_str(&format!(", held by {}", r.held_by));
    }
    if let Some(d) = &r.held_on {
        line.push_str(&format!(", held since {d}"));
    }
    if r.current {
        line.push_str(" (installed now)");
    }
    if let Some(chip) = blocked_chip(r) {
        line.push_str(&format!(" [{chip}]"));
    }
    line
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

/// The last rollback outcome the service recorded for an add-on, as the page says it. An applied
/// rollback needs no note; the new version is simply installed.
pub fn outcome_line(w: &World, package: &str) -> Option<String> {
    use wow_sync_runtime::RollbackOutcomeKind as K;
    let o = w.features.rollback_outcome.as_ref()?;
    if o.package.as_str() != package {
        return None;
    }
    let detail = o.text.trim();
    match o.kind {
        K::Applied => None,
        K::NotAvailable => Some(
            "The earlier version could not be read when WoW Sync tried to go back. Nothing was changed; you can ask again later."
                .to_string(),
        ),
        K::NotApplied => Some(if detail.is_empty() {
            "Going back did not complete. Nothing was changed.".to_string()
        } else {
            format!("Going back did not complete ({detail}). Nothing was changed.")
        }),
    }
}
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
pub fn confirm_text(installed: Option<&str>, r: &ReleaseRow) -> Vec<String> {
    if installed.is_some_and(|v| v == r.version) {
        return same_version_text(r);
    }
    let (replaces, blocked) = match installed {
        Some(v) => (
            format!("version {v}"),
            format!("Version {v} is blocked for the group. You can allow it again later."),
        ),
        None => (
            "the installed version".to_string(),
            "The installed version is blocked for the group. You can allow it again later.".into(),
        ),
    };
    vec![
        format!(
            "Go back to {} {}? Every computer in the group replaces {replaces} with {}.",
            r.title, r.version, r.version
        ),
        blocked,
        "WoW Sync applies this when WoW is closed. Nothing is changed while it is running.".into(),
    ]
}

/// The confirmation when the installed version and the earlier copy have the same version
/// number. The one place to adjust what this says about the group.
pub fn same_version_text(r: &ReleaseRow) -> Vec<String> {
    let earlier = match &r.held_on {
        Some(d) => format!("the earlier copy of {} set aside on {d}", r.version),
        None => format!("the earlier copy of {}", r.version),
    };
    vec![
        format!(
            "Go back to {} {}? Every computer in the group switches to {earlier}; the copy installed now is kept for recovery.",
            r.title, r.version
        ),
        "WoW Sync applies this when WoW is closed. Nothing is changed while it is running.".into(),
    ]
}

// The rollback outcome itself is reported (`rollback_outcome`, see `outcome_line`). The service
// does not yet report how the saved settings fared or recovery sizes; only tests build these.
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
            held_on: None,
        }
    }

    #[test]
    fn a_same_version_confirmation_states_the_group_switch_and_that_the_current_copy_is_kept() {
        let mut r = row("1.0.0", false, false);
        r.held_on = Some("3 Nov".into());
        let t = confirm_text(Some("1.0.0"), &r);
        assert!(t[0].contains(
            "Every computer in the group switches to the earlier copy of 1.0.0 set aside on 3 Nov; the copy installed now is kept for recovery."
        ));
        r.held_on = None;
        let t = confirm_text(Some("1.0.0"), &r);
        assert!(
            t[0].contains("switches to the earlier copy of 1.0.0;"),
            "{t:?}"
        );
        assert!(!t[0].contains("set aside"));
        let t = confirm_text(Some("2.0.0"), &r);
        assert!(t[0].contains("replaces version 2.0.0 with 1.0.0"), "{t:?}");
        assert!(t[0].contains("group"));
    }

    #[test]
    fn a_rollback_request_is_done_when_installed_or_an_outcome_is_set_and_not_before() {
        use wow_sync_runtime::{RollbackOutcome, RollbackOutcomeKind as K};
        let mut w = world();
        let rel = Sha256([5; 32]);
        let mut r = row("1.0.0", false, false);
        r.package_hash = rel;
        w.features.history = vec![r.clone()];
        assert!(!request_done(&w, "details", &rel), "still waiting");
        w.features.history[0].current = true;
        assert!(request_done(&w, "details", &rel));
        assert!(!request_done(&w, "other", &rel));
        w.features.history[0].current = false;
        w.features.rollback_outcome = Some(RollbackOutcome {
            package: key(),
            recovered: rel,
            kind: K::NotApplied,
            text: "x".into(),
        });
        assert!(request_done(&w, "details", &rel));
        assert!(!request_done(&w, "other", &rel));
    }

    #[test]
    fn a_rollback_outcome_is_told_on_its_own_add_on_page_only() {
        use wow_sync_runtime::{RollbackOutcome, RollbackOutcomeKind as K};
        let mut w = world();
        assert_eq!(outcome_line(&w, "details"), None);
        fn set(w: &mut World, kind: K, text: &str) {
            w.features.rollback_outcome = Some(RollbackOutcome {
                package: key(),
                recovered: Sha256([5; 32]),
                kind,
                text: text.into(),
            });
        }
        set(&mut w, K::NotAvailable, "not available");
        let t = outcome_line(&w, "details").unwrap();
        assert!(t.contains("could not be read") && t.contains("Nothing was changed"));
        assert_eq!(outcome_line(&w, "other"), None);
        set(&mut w, K::NotApplied, "not applied: interrupted");
        let t = outcome_line(&w, "details").unwrap();
        assert!(t.contains("did not complete (not applied: interrupted)"));
        set(&mut w, K::Applied, "done");
        assert_eq!(
            outcome_line(&w, "details"),
            None,
            "success needs no warning"
        );
    }

    #[test]
    fn a_held_date_is_mapped_in_local_time_and_absent_for_a_peer_only_release() {
        let mut dates = std::collections::BTreeMap::new();
        dates.insert(Sha256([5; 32]), 1_700_000_000_000u64);
        let d = held_on(&dates, &Sha256([5; 32])).unwrap();
        assert_eq!(d, crate::world::day_text(1_700_000_000_000));
        assert!(d.contains("Nov"), "{d}");
        assert_eq!(held_on(&dates, &Sha256([6; 32])), None);
        let mut r = row("1.0.0", false, false);
        r.held_on = Some(d.clone());
        assert!(release_line(&r).contains(&format!("held since {d}")));
    }

    #[test]
    fn the_installed_version_comes_from_the_add_on_and_the_line_shows_a_held_date() {
        let mut w = world();
        assert_eq!(installed_version(&w, "details"), None);
        w.addons.push(crate::world::Addon {
            key: "details".into(),
            title: "Details".into(),
            version: "2.0.0".into(),
            target_version: None,
            folders: vec![],
            kind: crate::world::Kind::InSync,
            candidates: vec![],
            why: String::new(),
            next: wow_sync_core::view::NextAction::Nothing,
            awaiting: vec![],
        });
        let v = installed_version(&w, "details");
        assert_eq!(v.as_deref(), Some("2.0.0"));
        let t = confirm_text(v.as_deref(), &row("1.0.0", false, false));
        assert!(
            t[0].contains("replaces version 2.0.0 with 1.0.0"),
            "{}",
            t[0]
        );
        let mut r = row("1.0.0", false, false);
        assert_eq!(release_line(&r), "Version 1.0.0, held by Den PC");
        r.held_on = Some("3 Nov".into());
        assert_eq!(
            release_line(&r),
            "Version 1.0.0, held by Den PC, held since 3 Nov"
        );
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
        let t = confirm_text(Some("2.1"), &row("2.0", false, false));
        assert!(t[0].starts_with("Go back to Details 2.0?"));
        assert!(t[0].contains("replaces version 2.1 with 2.0"));
        assert!(t[1].contains("2.1 is blocked"));
        assert!(t[2].contains("when WoW is closed"));
    }

    #[test]
    fn the_confirmation_never_repeats_a_summary_sentence_as_the_version() {
        let t = confirm_text(None, &row("2.0", false, false));
        assert!(t[0].contains("replaces the installed version with 2.0"));
        assert!(!t[0].contains("version version"));
        assert!(t[1].starts_with("The installed version is blocked"));
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
