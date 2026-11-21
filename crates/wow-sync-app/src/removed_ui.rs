//! The "Removed" section (removal.md §9): what the group removed, who did it, who has not caught
//! up, and how to bring it back. Pure text so it can be tested without a window.

use crate::features::{gate, ActionKind, GroupAction};
use crate::world::World;
use wow_sync_runtime::RemovedRow;

/// Why the restore button is off when the snapshot gives no removal to name.
pub const RESTORE_UNAVAILABLE: &str =
    "This removal can't be named yet, so it can't be restored from here. Nothing was changed.";

/// Which parts of the Add-ons page are drawn (the empty-state sentence, the active list, the
/// Removed section). Removed and restores must not depend on the active count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddonsLayout {
    pub empty_state: bool,
    pub active_list: bool,
    pub removed_section: bool,
}

pub fn addons_layout(w: &World) -> AddonsLayout {
    let none = w.view.packages.is_empty();
    AddonsLayout {
        empty_state: none,
        active_list: !none,
        removed_section: !none || !w.features.removed.is_empty() || !w.features.restores.is_empty(),
    }
}

/// The restore in progress for the removed row at `index`, matched by its first folder.
pub fn progress_for(w: &World, index: usize) -> Option<&wow_sync_runtime::RestoreProgress> {
    let first = w.features.removed.get(index)?.folders.first()?;
    w.features
        .restores
        .iter()
        .find(|p| p.package.as_str().eq_ignore_ascii_case(first))
}

/// A restore's sentence as plain text (the runtime already strips colour codes; this keeps
/// markup from an add-on title out of the window whatever the source).
pub fn plain(text: &str) -> String {
    wow_sync_runtime::strip_colour_codes(text)
}

/// Dismiss a refused restore (it leaves Needs attention).
pub fn dismiss_action(p: &wow_sync_runtime::RestoreProgress) -> GroupAction {
    GroupAction::DismissRestoreIssue {
        package: p.package.clone(),
    }
}

/// What the confirmation says before a restore is sent (removal.md §7.4 step 2).
pub fn confirm_text(title: &str) -> String {
    format!(
        "This reinstalls {} on all computers in this group. If World of Warcraft is running, it waits until the game closes.",
        wow_sync_runtime::strip_colour_codes(title)
    )
}

/// Restore for the row at `index`: the removal record the snapshot lists for the same row.
pub fn restore_action(w: &World, index: usize) -> Result<GroupAction, String> {
    gate(w, ActionKind::RestoreRemoved)?;
    match w.features.removal_hashes.get(index) {
        Some(removal) if index < w.features.removed.len() => Ok(GroupAction::RestoreRemoved {
            removal: *removal,
            title: wow_sync_runtime::strip_colour_codes(&w.features.removed[index].title),
        }),
        _ => Err(RESTORE_UNAVAILABLE.into()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedLine {
    pub title: String,
    pub provenance: String,
    pub folders: String,
    pub awaiting: Option<String>,
}

pub fn lines(rows: &[RemovedRow]) -> Vec<RemovedLine> {
    rows.iter()
        .map(|r| RemovedLine {
            title: r.title.clone(),
            provenance: r.provenance.clone(),
            folders: r.folders.join(", "),
            awaiting: if r.awaiting.is_empty() {
                None
            } else {
                Some(format!(
                    "Still to remove it: {}. It stays on those computers until they catch up.",
                    r.awaiting.join(", ")
                ))
            },
        })
        .collect()
}

pub fn intro(count: usize) -> &'static str {
    if count == 0 {
        "Nothing has been removed from this group."
    } else {
        "Removed add-ons keep their code and settings so they can be restored."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{result_text_named, ActionResult};
    use wow_sync_core::ids::Sha256;

    fn row(awaiting: &[&str]) -> RemovedRow {
        RemovedRow {
            title: "Demo".into(),
            folders: vec!["Demo".into(), "Demo_Options".into()],
            provenance: "Desk removed it".into(),
            awaiting: awaiting.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn lines_name_the_remover_and_who_is_still_awaited() {
        let l = lines(&[row(&["Laptop", "Den"])]);
        assert_eq!(l[0].folders, "Demo, Demo_Options");
        assert!(l[0].awaiting.as_deref().unwrap().contains("Laptop, Den"));
        assert_eq!(l[0].provenance, "Desk removed it");
    }

    #[test]
    fn restore_names_the_removal_of_the_same_row_and_a_retired_computer_cannot() {
        use crate::features::{Capabilities, Standing, RETIRED_REASON};
        let mut w = World::empty("0.0.0");
        w.features.caps = Capabilities::runtime();
        w.features.removed = vec![row(&[]), row(&["Laptop"])];
        w.features.removal_hashes = vec![Sha256([1; 32]), Sha256([2; 32])];
        assert_eq!(
            restore_action(&w, 1),
            Ok(GroupAction::RestoreRemoved {
                removal: Sha256([2; 32]),
                title: "Demo".into(),
            })
        );
        assert_eq!(restore_action(&w, 2), Err(RESTORE_UNAVAILABLE.into()));
        w.features.removal_hashes.truncate(1);
        assert_eq!(
            restore_action(&w, 1),
            Err(RESTORE_UNAVAILABLE.into()),
            "a row without a hash is never given another row's"
        );
        w.features.standing = Standing::Retired {
            on: String::new(),
            by: "Den".into(),
        };
        assert_eq!(restore_action(&w, 0), Err(RETIRED_REASON.into()));
    }

    #[test]
    fn the_confirmation_says_it_reinstalls_the_plain_title_on_all_computers() {
        let t = confirm_text("|cff33ff99Demo|r");
        assert!(t.starts_with("This reinstalls Demo on all computers in this group"));
        assert!(!t.contains("cff") && !t.contains('|'));
    }

    #[test]
    fn a_queued_restore_says_it_installs_everywhere_and_waits_for_the_game() {
        use wow_sync_runtime::CommandError;
        let q = result_text_named(
            ActionKind::RestoreRemoved,
            &ActionResult::Local,
            Some("Demo"),
        );
        assert!(q.starts_with("Demo will be reinstalled on all computers"));
        assert!(q.contains("running") && !q.contains("Saved on this computer"));
        let r = ActionResult::from_outcome(Err(CommandError::Service(
            "restore unavailable: a file is missing".into(),
        )));
        assert_eq!(
            result_text_named(ActionKind::RestoreRemoved, &r, Some("Demo")),
            "Demo can't be restored: a file is missing. Nothing was changed."
        );
        let other = ActionResult::from_outcome(Err(CommandError::Service("boom".into())));
        assert!(matches!(other, ActionResult::Failed(_)));
    }

    #[test]
    fn with_no_active_add_ons_the_removed_row_and_its_restore_are_still_there() {
        use crate::features::Capabilities;
        let mut w = World::empty("0.0.0");
        w.features.caps = Capabilities::runtime();
        assert_eq!(
            addons_layout(&w),
            AddonsLayout {
                empty_state: true,
                active_list: false,
                removed_section: false
            }
        );
        w.features.removed = vec![row(&[])];
        w.features.removal_hashes = vec![Sha256([1; 32])];
        let l = addons_layout(&w);
        assert!(l.empty_state && l.removed_section && !l.active_list);
        assert!(matches!(
            restore_action(&w, 0),
            Ok(GroupAction::RestoreRemoved { .. })
        ));
        assert_eq!(lines(&w.features.removed).len(), 1);
    }

    #[test]
    fn progress_belongs_to_the_row_of_its_first_folder_and_dismiss_names_that_folder() {
        use wow_sync_runtime::{RestoreProgress, RestoreState, RestoreWait};
        let mut w = World::empty("0.0.0");
        w.features.removed = vec![row(&[]), {
            let mut r = row(&[]);
            r.folders = vec!["Other".into()];
            r
        }];
        let p = RestoreProgress {
            package: wow_sync_core::paths::validate_folder_name("Other").unwrap(),
            title: "Other".into(),
            state: RestoreState::Waiting {
                reason: RestoreWait::Paused,
            },
            text: "Restoring Other: waiting for syncing to resume.".into(),
        };
        w.features.restores = vec![p.clone()];
        assert!(progress_for(&w, 0).is_none());
        assert_eq!(progress_for(&w, 1), Some(&p));
        assert_eq!(
            dismiss_action(&p),
            GroupAction::DismissRestoreIssue {
                package: p.package.clone()
            }
        );
        assert_eq!(
            result_text_named(ActionKind::DismissRestoreIssue, &ActionResult::Local, None),
            "Dismissed. Nothing was changed."
        );
    }

    #[test]
    fn nobody_awaited_means_no_waiting_line_and_empty_means_the_empty_text() {
        assert_eq!(lines(&[row(&[])])[0].awaiting, None);
        assert!(intro(0).starts_with("Nothing"));
        assert!(intro(1).contains("restored"));
        assert!(RESTORE_UNAVAILABLE.ends_with("Nothing was changed."));
    }
}
