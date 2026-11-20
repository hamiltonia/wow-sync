//! View-model for retiring a computer, joining again and allowing a rejected release again
//! (ops.md §2, rollback.md §8). Pure: it decides what the owner is told and which typed action a
//! confirmation produces; it writes nothing.

use crate::features::{gate, ActionKind, GroupAction, Standing};
use crate::world::World;
use wow_sync_core::ids::Sha256;

pub const STALE_AFTER_SECS: i64 = 72 * 3600;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetireSheet {
    pub title: String,
    pub facts: Vec<String>,
    /// Shown when the computer reported 72 hours ago or more, or never.
    pub stale_warning: Option<String>,
    /// Shown when the computer is the settings master.
    pub master_note: Option<String>,
    pub ack_label: String,
}

fn ago(secs: i64) -> String {
    let h = secs / 3600;
    if h < 1 {
        "less than an hour ago".into()
    } else if h < 48 {
        format!("{h} hour{} ago", if h == 1 { "" } else { "s" })
    } else {
        format!("{} days ago", h / 24)
    }
}

/// Why "Retire this computer…" is unavailable, or `Ok` when the sheet may open.
pub fn retire_gate(w: &World, peer: usize) -> Result<(), String> {
    if w.paused_since.is_some() {
        return Err("Resume syncing to retire a computer.".into());
    }
    gate(w, ActionKind::Retire)?;
    let Some(p) = w.peers.get(peer) else {
        return Err("This computer is no longer in the group.".into());
    };
    if !w.features.enrollments.iter().any(|(id, _)| *id == p.id) {
        return Err(
            "This computer's enrollment is not known yet. Try again after the next check.".into(),
        );
    }
    Ok(())
}

pub fn retire_sheet(w: &World, peer: usize) -> Result<RetireSheet, String> {
    retire_gate(w, peer)?;
    let p = &w.peers[peer];
    let n = &p.name;
    let stale = match p.report_secs_ago {
        Some(s) if s < STALE_AFTER_SECS => None,
        Some(s) => Some(format!(
            "{n} reported {}. Retire only computers you no longer use, or that are lost.",
            ago(s)
        )),
        None => Some(format!(
            "{n} has not reported. Retire only computers you no longer use, or that are lost."
        )),
    };
    let master_note = (w.features.master.as_deref() == Some(n.as_str())).then(|| {
        format!(
            "{n} is the settings master. After retiring it, settings are not shared automatically until you choose a new master. Nothing changes on the other computers' settings."
        )
    });
    Ok(RetireSheet {
        title: format!("Retire {n}?"),
        facts: vec![
            format!("{n} will stop sharing changes. Other computers stop waiting for it."),
            "Its history and recovery copies are kept. Nothing is deleted from the shared folder."
                .into(),
            "To use it again, join the group again from that computer.".into(),
        ],
        stale_warning: stale,
        master_note,
        ack_label: format!("I understand {n} stops syncing."),
    })
}

/// The retire action once the owner ticked the acknowledgement. Without it nothing is built.
pub fn retire_action(w: &World, peer: usize, acknowledged: bool) -> Result<GroupAction, String> {
    retire_gate(w, peer)?;
    if !acknowledged {
        return Err("Tick the box to confirm.".into());
    }
    let id = &w.peers[peer].id;
    let subject = w
        .features
        .enrollments
        .iter()
        .find(|(c, _)| c == id)
        .map(|(_, e)| e.clone())
        .ok_or_else(|| "This computer's enrollment is not known yet.".to_string())?;
    Ok(GroupAction::Retire { subject })
}

/// "Retired on D by X", or "Retired by X" when the date is not known.
fn retired_by(on: &str, by: &str) -> String {
    if on.is_empty() {
        format!("Retired by {by}")
    } else {
        format!("Retired on {on} by {by}")
    }
}

/// The sentence on the Computers page and Overview for a retired or held self.
pub fn standing_headline(s: &Standing) -> Option<(String, String)> {
    match s {
        Standing::Active => None,
        Standing::Retired { on, by } => Some((
            "This computer was retired".into(),
            format!(
                "{}. Your add-ons and settings here are kept. Join the group again to sync.",
                retired_by(on, by)
            ),
        )),
        Standing::Held => Some((
            "Waiting for group history".into(),
            "Nothing is updated on this computer until the group history is complete.".into(),
        )),
    }
}

/// "Join again…" is the one group action a retired computer has.
pub fn join_again(w: &World) -> Result<GroupAction, String> {
    if !matches!(w.features.standing, Standing::Retired { .. }) {
        return Err("This computer is not retired.".into());
    }
    gate(w, ActionKind::ReEnroll)?;
    Ok(GroupAction::ReEnroll)
}

/// A release the group rejected that the owner may allow again. Hashes come from the snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowAgainRow {
    pub title: String,
    pub version: String,
    pub rejections: Vec<Sha256>,
    /// The day the group rejected it; empty when not known.
    pub on: String,
}

pub fn allow_again_label(r: &AllowAgainRow) -> String {
    if r.on.is_empty() {
        format!("{} {} is blocked", r.title, r.version)
    } else {
        format!("{} {} is blocked since {}", r.title, r.version, r.on)
    }
}

pub fn allow_again(w: &World, row: &AllowAgainRow) -> Result<GroupAction, String> {
    gate(w, ActionKind::AllowAgain)?;
    if row.rejections.is_empty() {
        return Err("Nothing is blocked here.".into());
    }
    Ok(GroupAction::AllowAgain {
        rejections: row.rejections.clone(),
    })
}

/// Retired computers keep history but have no status dot.
pub fn retired_line(c: &crate::features::RetiredComputer) -> String {
    format!("{}. History kept.", retired_by(&c.on, &c.by))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Capabilities, RETIRED_REASON};
    use wow_sync_core::ids::{ComputerId, EnrollmentId};
    use wow_sync_core::kinds::EnrollmentRef;

    const DEN: &str = "22222222222222222222222222222222";

    fn world(report: Option<i64>) -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.computer_id = "11111111111111111111111111111111".into();
        w.peers.push(crate::world::Peer {
            id: DEN.into(),
            name: "Den PC".into(),
            state: wow_sync_core::status::MemberSummary::Usable,
            report_secs_ago: report,
            game: String::new(),
            app: String::new(),
            confirmed: 0,
        });
        w.features.caps = Capabilities::runtime();
        w.features.enrollments.push((
            DEN.into(),
            EnrollmentRef {
                computer_id: ComputerId::parse(DEN).unwrap(),
                enrollment_id: EnrollmentId::parse("33333333333333333333333333333333").unwrap(),
            },
        ));
        w
    }

    #[test]
    fn the_sheet_says_what_retiring_does_and_keeps() {
        let s = retire_sheet(&world(Some(60)), 0).unwrap();
        assert_eq!(s.title, "Retire Den PC?");
        assert!(s.facts[1].contains("Nothing is deleted from the shared folder"));
        assert!(s.facts[2].contains("join the group again"));
        assert_eq!(s.stale_warning, None);
        assert_eq!(s.ack_label, "I understand Den PC stops syncing.");
    }

    #[test]
    fn a_silent_computer_gets_the_72_hour_warning_and_a_master_gets_its_note() {
        let mut w = world(Some(3 * 24 * 3600));
        w.features.master = Some("Den PC".into());
        let s = retire_sheet(&w, 0).unwrap();
        assert!(s
            .stale_warning
            .unwrap()
            .contains("Den PC reported 3 days ago"));
        assert!(s
            .master_note
            .unwrap()
            .contains("until you choose a new master"));
        let edge = retire_sheet(&world(Some(STALE_AFTER_SECS - 1)), 0).unwrap();
        assert_eq!(edge.stale_warning, None);
        let never = retire_sheet(&world(None), 0).unwrap();
        assert!(never.stale_warning.unwrap().contains("has not reported"));
    }

    #[test]
    fn retire_needs_the_tick_and_names_the_exact_enrollment() {
        let w = world(Some(60));
        assert!(retire_action(&w, 0, false).is_err());
        let GroupAction::Retire { subject } = retire_action(&w, 0, true).unwrap() else {
            panic!("a retire action");
        };
        assert_eq!(subject.computer_id.as_str(), DEN);
    }

    #[test]
    fn retire_is_unavailable_while_paused_retired_or_unknown() {
        let mut w = world(Some(60));
        w.paused_since = Some("today".into());
        assert_eq!(
            retire_gate(&w, 0).unwrap_err(),
            "Resume syncing to retire a computer."
        );
        let mut w = world(Some(60));
        w.features.standing = Standing::Retired {
            on: "12 Jan".into(),
            by: "Laptop".into(),
        };
        assert_eq!(retire_gate(&w, 0).unwrap_err(), RETIRED_REASON);
        let mut w = world(Some(60));
        w.features.enrollments.clear();
        assert!(retire_gate(&w, 0).is_err());
        assert!(retire_gate(&world(Some(60)), 5).is_err());
    }

    #[test]
    fn a_retired_self_reads_retired_and_a_held_self_claims_nothing() {
        let (h, s) = standing_headline(&Standing::Retired {
            on: "12 Jan".into(),
            by: "Laptop".into(),
        })
        .unwrap();
        assert_eq!(h, "This computer was retired");
        assert!(s.starts_with("Retired on 12 Jan by Laptop."));
        let (h, s) = standing_headline(&Standing::Held).unwrap();
        assert_eq!(h, "Waiting for group history");
        assert!(!s.contains(" by ") && !s.contains("Retired"));
        assert_eq!(standing_headline(&Standing::Active), None);
    }

    #[test]
    fn join_again_exists_only_for_a_retired_computer() {
        let mut w = world(Some(60));
        assert!(join_again(&w).is_err());
        w.features.standing = Standing::Retired {
            on: "12 Jan".into(),
            by: "Laptop".into(),
        };
        assert_eq!(join_again(&w).unwrap(), GroupAction::ReEnroll);
    }

    #[test]
    fn allow_again_carries_exactly_the_rejections_shown() {
        let w = world(Some(60));
        let row = AllowAgainRow {
            title: "Bagnon".into(),
            version: "2.1".into(),
            rejections: vec![Sha256([1; 32]), Sha256([2; 32])],
            on: String::new(),
        };
        assert_eq!(allow_again_label(&row), "Bagnon 2.1 is blocked");
        assert_eq!(
            allow_again_label(&AllowAgainRow {
                on: "12 January".into(),
                ..row.clone()
            }),
            "Bagnon 2.1 is blocked since 12 January"
        );
        assert_eq!(
            allow_again(&w, &row).unwrap(),
            GroupAction::AllowAgain {
                rejections: vec![Sha256([1; 32]), Sha256([2; 32])]
            }
        );
        let empty = AllowAgainRow {
            rejections: vec![],
            ..row
        };
        assert!(allow_again(&w, &empty).is_err());
    }
}
