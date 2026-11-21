//! Local recovery points per add-on (rollback.md §7): earlier versions this computer set aside,
//! restorable with the same group-wide rollback as a held release. Pure view-model; the rows come
//! from the snapshot and nothing here reads files.

use crate::features::GroupAction;
use crate::rollback_ui::{button, HistoryButton, ReleaseRow};
use crate::world::{day_text, World};
use wow_sync_core::ids::Sha256;
use wow_sync_core::paths::FolderKey;

/// One local recovery point, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub package: FolderKey,
    pub release: Sha256,
    pub version: Option<String>,
    /// Local date it was set aside.
    pub when: String,
    pub reason: String,
    pub pinned: bool,
}

pub fn entries(rows: &[wow_sync_runtime::RecoveryRow]) -> Vec<RecoveryEntry> {
    rows.iter()
        .map(|r| RecoveryEntry {
            package: r.package.clone(),
            release: r.release,
            version: r.version.clone().filter(|v| !v.trim().is_empty()),
            when: day_text(r.set_aside_unix_ms),
            reason: r.reason.clone(),
            pinned: r.pinned,
        })
        .collect()
}

/// The entries of one add-on that the held-release list does not already show.
pub fn entries_for<'a>(w: &'a World, key: &str, shown: &[ReleaseRow]) -> Vec<&'a RecoveryEntry> {
    w.features
        .recovery
        .iter()
        .filter(|e| e.package.as_str() == key)
        .filter(|e| shown.iter().all(|r| r.package_hash != e.release))
        .collect()
}

pub fn line(e: &RecoveryEntry) -> String {
    let mut s = match &e.version {
        Some(v) => format!("Version {v}"),
        None => "An earlier copy".to_string(),
    };
    s.push_str(&format!(", set aside here on {}", e.when));
    if e.pinned {
        s.push_str(" (kept until you decide)");
    }
    if !e.reason.trim().is_empty() {
        s.push_str(&format!(". {}", e.reason.trim()));
    }
    s
}

/// The history-row shape of an entry, so restoring shares the rollback gate and confirmation.
pub fn as_release(e: &RecoveryEntry, title: &str) -> ReleaseRow {
    ReleaseRow {
        package: e.package.clone(),
        title: title.to_string(),
        version: e
            .version
            .clone()
            .unwrap_or_else(|| "from earlier".to_string()),
        package_hash: e.release,
        current: false,
        rejections: vec![],
        held_by: "this computer".into(),
        held_on: Some(e.when.clone()),
    }
}

pub const RESTORE: &str = "Restore this version…";

/// The restore action, or why it is unavailable.
pub fn restore(w: &World, e: &RecoveryEntry, title: &str) -> Result<GroupAction, String> {
    match button(w, &as_release(e, title)) {
        HistoryButton::GoBack(a) => Ok(a),
        HistoryButton::Disabled { reason, .. } => Err(reason),
        HistoryButton::AllowAgain(_) | HistoryButton::None => {
            Err("This version can't be restored from here.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{ActionKind, Capabilities, Standing};
    use wow_sync_core::ids::Sha256;

    fn key() -> FolderKey {
        wow_sync_core::paths::validate_folder_name("Details").unwrap()
    }

    fn row(n: u8, version: Option<&str>) -> wow_sync_runtime::RecoveryRow {
        wow_sync_runtime::RecoveryRow {
            package: key(),
            id: format!("op{n}"),
            release: Sha256([n; 32]),
            version: version.map(Into::into),
            set_aside_unix_ms: 1_700_000_000_000,
            reason: "Replaced by a rollback".into(),
            pinned: n == 2,
        }
    }

    fn world(rows: &[wow_sync_runtime::RecoveryRow]) -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.features.caps = Capabilities::runtime();
        w.features.recovery = entries(rows);
        w
    }

    #[test]
    fn entries_list_the_add_ons_recovery_points_and_restore_is_a_group_rollback() {
        let w = world(&[row(1, Some("1.0.0")), row(2, None)]);
        let es = entries_for(&w, "details", &[]);
        assert_eq!(es.len(), 2);
        assert!(line(es[0]).starts_with("Version 1.0.0, set aside here on "));
        assert!(line(es[0]).ends_with("Replaced by a rollback"));
        assert!(line(es[1]).starts_with("An earlier copy"));
        assert!(line(es[1]).contains("kept until you decide"));
        assert_eq!(
            restore(&w, es[0], "Details"),
            Ok(GroupAction::Rollback {
                package: key(),
                recovered: Sha256([1; 32])
            })
        );
        let t = crate::rollback_ui::confirm_text(Some("2.0.0"), &as_release(es[0], "Details"));
        assert!(t[0].contains("replaces version 2.0.0 with 1.0.0"));
        assert!(t[0].contains("Every computer in the group"));
    }

    #[test]
    fn nothing_listed_for_other_add_ons_duplicates_or_when_none_are_kept() {
        let w = world(&[row(1, Some("1.0.0"))]);
        assert!(entries_for(&w, "other", &[]).is_empty());
        let held = ReleaseRow {
            package_hash: Sha256([1; 32]),
            ..as_release(&w.features.recovery[0], "Details")
        };
        assert!(entries_for(&w, "details", &[held]).is_empty());
        assert!(entries_for(&world(&[]), "details", &[]).is_empty());
    }

    #[test]
    fn a_retired_computer_cannot_restore_and_is_told_why() {
        let mut w = world(&[row(1, Some("1.0.0"))]);
        w.features.standing = Standing::Retired {
            on: "1 Jan".into(),
            by: "Den PC".into(),
        };
        let e = w.features.recovery[0].clone();
        assert_eq!(
            restore(&w, &e, "Details"),
            Err(crate::features::RETIRED_REASON.into())
        );
        let _ = ActionKind::Rollback;
    }
}
