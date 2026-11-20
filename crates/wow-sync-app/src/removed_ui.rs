//! The "Removed" section (removal.md §9): what the group removed, who did it, who has not caught
//! up, and how to bring it back. Pure text so it can be tested without a window.

use crate::features::{gate, ActionKind, GroupAction};
use crate::world::World;
use wow_sync_runtime::RemovedRow;

/// Why the restore button is off when the snapshot gives no removal to name.
pub const RESTORE_UNAVAILABLE: &str =
    "This removal can't be named yet, so it can't be restored from here. Nothing was changed.";

/// Restore for the row at `index`: the removal record the snapshot lists for the same row.
pub fn restore_action(w: &World, index: usize) -> Result<GroupAction, String> {
    gate(w, ActionKind::RestoreRemoved)?;
    match w.features.removal_hashes.get(index) {
        Some(removal) if index < w.features.removed.len() => {
            Ok(GroupAction::RestoreRemoved { removal: *removal })
        }
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
                removal: Sha256([2; 32])
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
    fn nobody_awaited_means_no_waiting_line_and_empty_means_the_empty_text() {
        assert_eq!(lines(&[row(&[])])[0].awaiting, None);
        assert!(intro(0).starts_with("Nothing"));
        assert!(intro(1).contains("restored"));
        assert!(RESTORE_UNAVAILABLE.ends_with("Nothing was changed."));
    }
}
