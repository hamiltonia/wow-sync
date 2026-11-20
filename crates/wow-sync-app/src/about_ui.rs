//! About and diagnostics view-model: the update state text, the notification toggles and the
//! diagnostics export sheet state. Pure text and state so it is testable without a window.

use crate::features::{ActionKind, ActionResult};
use std::path::PathBuf;
use wow_sync_core::diagnostics::{ExportOptions, NEVER_INCLUDED, SHARED_FOLDER_WARNING};
use wow_sync_core::update::UpdateResult;
use wow_sync_runtime::AppToggles;

/// The Updates panel's status line (screens.md "About and updates").
pub fn update_text(u: Option<&UpdateResult>) -> String {
    match u {
        None => "No update check has finished yet.".into(),
        Some(r) => r.text(),
    }
}

/// Applies the toggle named by a control key; `None` for a key that is not a toggle.
pub fn toggled(t: AppToggles, key: &str) -> Option<AppToggles> {
    let mut n = t;
    match key {
        "update-check" => n.update_check = !t.update_check,
        "notify-summaries" => n.notify_summaries = !t.notify_summaries,
        "notify-needs-me" => n.notify_needs_me = !t.notify_needs_me,
        _ => return None,
    }
    Some(n)
}

/// What the export sheet is showing. A preview belongs to the options it was built with.
#[derive(Clone, Debug, Default)]
pub struct DiagState {
    pub options: ExportOptions,
    pub preview: Option<Preview>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub text: String,
    pub summary: String,
}

impl DiagState {
    /// Flips an option. The old preview is dropped, so a save is always of what was shown.
    pub fn toggle(&mut self, key: &str) -> bool {
        match key {
            "diag-names" => self.options.include_folder_names = !self.options.include_folder_names,
            "diag-fingerprints" => {
                self.options.include_fingerprints = !self.options.include_fingerprints
            }
            _ => return false,
        }
        self.preview = None;
        self.message = None;
        true
    }

    /// Takes the result of a preview or save.
    pub fn absorb(&mut self, kind: ActionKind, r: &ActionResult) {
        match (kind, r) {
            (ActionKind::ExportDiagnostics, ActionResult::Exported { text, summary }) => {
                self.preview = Some(Preview {
                    text: text.clone(),
                    summary: summary.clone(),
                });
                self.message = None;
            }
            (ActionKind::SaveDiagnostics, ActionResult::Exported { .. }) => {
                self.message = Some(crate::features::result_text(kind, r));
            }
            (
                ActionKind::ExportDiagnostics | ActionKind::SaveDiagnostics,
                ActionResult::ExportRefused(t),
            ) => {
                if kind == ActionKind::ExportDiagnostics {
                    self.preview = None;
                }
                self.message = Some(t.clone());
            }
            _ => {}
        }
    }
}

/// What is never in the file, then the shared-folder warning (shown on the sheet).
pub fn warnings() -> Vec<String> {
    let mut v = vec![format!("Never included: {}.", NEVER_INCLUDED.join("; "))];
    v.push(SHARED_FOLDER_WARNING.to_string());
    v
}

/// Where Save puts the file: the user's Documents folder when it exists, else the home folder.
/// The runtime still refuses the shared folder and install roots.
pub fn default_dest() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let docs = home.join("Documents");
    let dir = if docs.is_dir() { docs } else { home };
    dir.join("wow-sync-diagnostics.txt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_core::update::UpdateFailKind;

    #[test]
    fn update_states_use_core_text_and_an_empty_state() {
        assert!(update_text(None).starts_with("No update check"));
        let a = UpdateResult::Available {
            tag: "v1.2.0".into(),
            checked: 1,
        };
        assert_eq!(update_text(Some(&a)), "Version 1.2.0 is available");
        let f = UpdateResult::Failed {
            at: 1,
            kind: UpdateFailKind::Offline,
        };
        assert_eq!(update_text(Some(&f)), UpdateResult::text(&f));
        assert_ne!(update_text(Some(&f)), update_text(Some(&a)));
    }

    #[test]
    fn a_toggle_flips_only_its_own_flag_and_unknown_keys_do_nothing() {
        let t = AppToggles::default();
        let n = toggled(t, "notify-summaries").unwrap();
        assert!(!n.notify_summaries && n.update_check && n.notify_needs_me);
        assert_eq!(toggled(t, "closing-quits"), None);
    }

    #[test]
    fn changing_an_option_drops_the_preview_and_a_refusal_is_shown_not_saved() {
        let mut d = DiagState::default();
        assert!(d.options.include_folder_names && !d.options.include_fingerprints);
        d.absorb(
            ActionKind::ExportDiagnostics,
            &ActionResult::Exported {
                text: "x".into(),
                summary: "Replaced: 1".into(),
            },
        );
        assert!(d.preview.is_some());
        assert!(d.toggle("diag-fingerprints"));
        assert!(d.preview.is_none(), "a save is only of what was shown");
        assert!(d.options.include_fingerprints);

        d.absorb(
            ActionKind::SaveDiagnostics,
            &ActionResult::ExportRefused("That place is not allowed.".into()),
        );
        assert_eq!(d.message.as_deref(), Some("That place is not allowed."));
        assert!(warnings()[1].contains("shared Dropbox folder"));
    }
}
