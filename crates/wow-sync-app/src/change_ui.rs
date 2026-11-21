//! "Change decision" and local recovery on the Add-on page (ops.md §1.5, rollback.md §8). Pure:
//! every control is a typed `GroupAction` or the reason it is unavailable; nothing here writes.

use crate::features::{gate, ActionKind, GroupAction};
use crate::rollback_ui::ReleaseRow;
use crate::world::World;
use wow_sync_core::kinds::ChoiceReason;
use wow_sync_core::paths::FolderKey;

/// A button with the sentence to confirm first and what a screen reader announces for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeControl {
    pub label: String,
    pub name: String,
    pub confirm: String,
    pub action: Result<GroupAction, String>,
}

/// The reason recorded with a change made from the Add-on page. The snapshot does not yet carry
/// the reason of the original decision, so this is the neutral one.
const CHANGE_REASON: ChoiceReason = ChoiceReason::Ambiguous;

/// Keep one held release, or resume automatic updates, for `key`.
pub fn change_controls(
    w: &World,
    key: &str,
    title: &str,
    rows: &[ReleaseRow],
) -> Vec<ChangeControl> {
    let Ok(folder) = wow_sync_core::paths::validate_folder_name(key) else {
        return Vec::new();
    };
    let mut out: Vec<ChangeControl> = rows
        .iter()
        .filter(|r| !r.current && r.rejections.is_empty())
        .map(|r| keep_control(w, &folder, title, r))
        .collect();
    out.push(resume_control(w, &folder, title));
    out
}

fn keep_control(w: &World, folder: &FolderKey, title: &str, r: &ReleaseRow) -> ChangeControl {
    let holder = if r.held_by.is_empty() {
        String::new()
    } else {
        format!(", held by {}", r.held_by)
    };
    let consequence = format!(
        "Keep version {} for the whole group. {title} is not updated past it automatically; you are asked when a newer version arrives.",
        r.version
    );
    ChangeControl {
        label: format!("Keep version {}", r.version),
        name: format!(
            "Keep version {} of {title}{holder}. {consequence}",
            r.version
        ),
        confirm: consequence,
        action: gate(w, ActionKind::Keep).map(|()| GroupAction::Keep {
            scope: vec![folder.clone()],
            copy: r.package_hash,
            why: CHANGE_REASON,
        }),
    }
}

fn resume_control(w: &World, folder: &FolderKey, title: &str) -> ChangeControl {
    let consequence = format!(
        "Resume automatic updates for {title}? Versions you declined earlier stay excluded, so they are never installed automatically."
    );
    ChangeControl {
        label: "Resume automatic updates".into(),
        name: format!("Resume automatic updates for {title}. {consequence}"),
        confirm: consequence,
        action: gate(w, ActionKind::Resume).map(|()| GroupAction::Resume {
            scope: vec![folder.clone()],
            allow_again: Vec::new(),
            why: CHANGE_REASON,
        }),
    }
}

/// What the Recovery panel says when no earlier version is kept for the add-on.
pub const NO_EARLIER: &str =
    "No earlier version of this add-on is kept here. Older versions appear here when this computer or another holds one.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Capabilities, Standing, RETIRED_REASON};
    use wow_sync_core::ids::Sha256;

    fn row(version: &str, n: u8, current: bool) -> ReleaseRow {
        ReleaseRow {
            package: wow_sync_core::paths::validate_folder_name("Details").unwrap(),
            title: "Details".into(),
            version: version.into(),
            package_hash: Sha256([n; 32]),
            current,
            rejections: vec![],
            held_by: "Den PC".into(),
            held_on: None,
        }
    }

    fn world() -> World {
        let mut w = World::empty("0.0.0");
        w.features.caps = Capabilities::runtime();
        w
    }

    #[test]
    fn keep_names_the_release_and_resume_is_always_offered() {
        let w = world();
        let rows = [row("2.0", 2, true), row("1.0", 1, false)];
        let cs = change_controls(&w, "Details", "Details", &rows);
        assert_eq!(cs.len(), 2, "no keep for the installed release");
        assert_eq!(cs[0].label, "Keep version 1.0");
        assert!(cs[0].name.contains("1.0") && cs[0].name.contains("Den PC"));
        assert!(cs[0].name.contains("not updated past it"));
        let Ok(GroupAction::Keep { scope, copy, .. }) = &cs[0].action else {
            panic!("keep");
        };
        assert_eq!(scope[0].as_str(), "details");
        assert_eq!(*copy, Sha256([1; 32]));
        assert!(matches!(cs[1].action, Ok(GroupAction::Resume { .. })));
        assert!(cs[1].confirm.contains("declined earlier stay excluded"));
    }

    #[test]
    fn a_blocked_release_is_not_offered_and_a_retired_computer_cannot_change_a_decision() {
        let mut w = world();
        let mut blocked = row("1.5", 3, false);
        blocked.rejections = vec![Sha256([8; 32])];
        let cs = change_controls(&w, "Details", "Details", &[blocked]);
        assert_eq!(cs.len(), 1, "only resume");
        w.features.standing = Standing::Retired {
            on: "1 Jan".into(),
            by: "Den PC".into(),
        };
        let cs = change_controls(&w, "Details", "Details", &[row("1.0", 1, false)]);
        assert!(cs.iter().all(|c| c.action == Err(RETIRED_REASON.into())));
    }
}
