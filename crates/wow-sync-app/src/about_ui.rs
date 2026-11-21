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
    /// The runtime's export; Save sends exactly this.
    pub export: wow_sync_core::diagnostics::Export,
    /// A file name no earlier save used; Save writes here and never over an earlier export.
    pub dest: PathBuf,
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
            (
                ActionKind::ExportDiagnostics,
                ActionResult::Exported {
                    text,
                    summary,
                    export,
                },
            ) => {
                self.preview = Some(Preview {
                    text: text.clone(),
                    summary: summary.clone(),
                    export: export.clone(),
                    dest: default_dest(),
                });
                self.message = None;
            }
            (ActionKind::SaveDiagnostics, ActionResult::Exported { .. }) => {
                let mut text = crate::features::result_text(kind, r);
                if let Some(p) = self.preview.as_mut() {
                    text = format!("{text} Saved as {}.", p.dest.display());
                    p.dest = default_dest();
                }
                self.message = Some(text);
            }
            (ActionKind::SaveDiagnostics, ActionResult::Changed) => {
                self.preview = None;
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

/// Where Save puts the file: the user's Documents folder when it exists, else the home folder,
/// under a name made from the current time. The runtime still refuses the shared folder and
/// install roots.
pub fn default_dest() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let docs = home.join("Documents");
    let dir = if docs.is_dir() { docs } else { home };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    fresh_dest(&dir, now)
}

/// `wow-sync-diagnostics-YYYYMMDD-HHMMSS-mmm.txt` (UTC) in `dir`; a numbered variant if that name
/// is somehow taken, so no existing file is ever the destination.
pub fn fresh_dest(dir: &std::path::Path, unix_ms: u64) -> PathBuf {
    let secs = unix_ms / 1000;
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    let stem = format!(
        "wow-sync-diagnostics-{y:04}{m:02}{d:02}-{:02}{:02}{:02}-{:03}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        unix_ms % 1000
    );
    let mut p = dir.join(format!("{stem}.txt"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem}-{n}.txt"));
        n += 1;
    }
    p
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((yoe + era * 400 + i64::from(m <= 2)), m, d)
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
                export: sample_export("x"),
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

    fn sample_export(text: &str) -> wow_sync_core::diagnostics::Export {
        wow_sync_core::diagnostics::Export {
            text: text.into(),
            replaced: wow_sync_core::diagnostics::Replaced {
                folder_paths: 1,
                computer_names: 0,
                addon_names: 0,
            },
        }
    }

    #[test]
    fn save_sends_the_exact_previewed_export_and_never_a_rebuilt_one() {
        use crate::features::GroupAction;
        use wow_sync_runtime::{Command, CommandOutcome};
        let shown = sample_export("what the preview showed");
        let r = ActionResult::from_outcome(Ok(CommandOutcome::Exported(shown.clone())));
        let mut d = DiagState::default();
        d.absorb(ActionKind::ExportDiagnostics, &r);
        let p = d.preview.clone().unwrap();
        assert_eq!(p.export, shown);
        let save = GroupAction::SaveDiagnostics {
            export: p.export,
            dest: PathBuf::from("out.txt"),
        };
        assert_eq!(
            save.to_command(),
            Command::SavePreviewedDiagnostics {
                export: shown,
                dest: PathBuf::from("out.txt"),
            }
        );
    }

    #[test]
    fn a_stale_preview_says_so_drops_the_preview_and_other_changes_keep_their_wording() {
        use wow_sync_core::decision::DecideError;
        use wow_sync_runtime::CommandError;
        let mut d = DiagState::default();
        d.absorb(
            ActionKind::ExportDiagnostics,
            &ActionResult::Exported {
                text: "x".into(),
                summary: "s".into(),
                export: sample_export("x"),
            },
        );
        let r = ActionResult::from_outcome(Err(CommandError::Decide(DecideError::Changed)));
        d.absorb(ActionKind::SaveDiagnostics, &r);
        assert_eq!(
            d.message.as_deref(),
            Some(
                "The details changed since the preview. Preview again before saving. Nothing was saved."
            )
        );
        assert!(d.preview.is_none());
        let other = crate::features::result_text(ActionKind::Decide, &r);
        assert!(other.starts_with("Something changed while you were choosing"));
    }

    #[test]
    fn each_save_gets_a_new_file_name_and_never_an_existing_one() {
        let dir = std::env::temp_dir().join(format!("wowsync-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = fresh_dest(&dir, 1_795_000_000_123);
        assert_eq!(
            first.file_name().unwrap().to_string_lossy(),
            "wow-sync-diagnostics-20261118-110640-123.txt"
        );
        std::fs::write(&first, "first export").unwrap();
        let again = fresh_dest(&dir, 1_795_000_000_123);
        assert_ne!(again, first, "same millisecond still gets a new name");
        let later = fresh_dest(&dir, 1_795_000_001_000);
        assert_ne!(later, first);
        std::fs::write(&again, "second export").unwrap();
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "first export");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_successful_save_moves_the_preview_to_a_new_destination() {
        let mut d = DiagState::default();
        let shown = sample_export("x");
        d.absorb(
            ActionKind::ExportDiagnostics,
            &ActionResult::Exported {
                text: "x".into(),
                summary: "s".into(),
                export: shown.clone(),
            },
        );
        let before = d.preview.clone().unwrap().dest;
        std::fs::create_dir_all(before.parent().unwrap()).ok();
        d.preview.as_mut().unwrap().dest = std::env::temp_dir().join("wowsync-taken.txt");
        std::fs::write(&d.preview.as_ref().unwrap().dest, "kept").unwrap();
        d.absorb(
            ActionKind::SaveDiagnostics,
            &ActionResult::Exported {
                text: "x".into(),
                summary: "s".into(),
                export: shown,
            },
        );
        assert_ne!(
            d.preview.as_ref().unwrap().dest,
            std::env::temp_dir().join("wowsync-taken.txt")
        );
        assert!(d.message.as_deref().unwrap().contains("wowsync-taken.txt"));
        let _ = std::fs::remove_file(std::env::temp_dir().join("wowsync-taken.txt"));
    }
}
