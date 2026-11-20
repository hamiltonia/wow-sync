//! The "Removed" section (removal.md §9): what the group removed, who did it, who has not caught
//! up, and how to bring it back. Pure text so it can be tested without a window.

use wow_sync_runtime::RemovedRow;

/// Why the restore button is off. The snapshot row carries no removal hash yet, so the app
/// cannot name the removal to restore (handover to R4-integrate).
pub const RESTORE_UNAVAILABLE: &str =
    "Restoring from here is not available yet. Nothing was changed.";

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
    fn nobody_awaited_means_no_waiting_line_and_empty_means_the_empty_text() {
        assert_eq!(lines(&[row(&[])])[0].awaiting, None);
        assert!(intro(0).starts_with("Nothing"));
        assert!(intro(1).contains("restored"));
        assert!(RESTORE_UNAVAILABLE.ends_with("Nothing was changed."));
    }
}
