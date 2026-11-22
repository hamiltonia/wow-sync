//! View-model of the Decisions screen (ops.md §1, UQ-2). Pure: it turns core's typed
//! `DecisionItem`s into words and turns a choice back into a typed `GroupAction`. The app never
//! invents an option: only ids core offered can be sent, with the evidence token that was shown.

use crate::features::{gate, hold_sentence, ActionKind, GroupAction, Standing};
use crate::world::World;
use wow_sync_core::decision::{
    Consequence, DecisionId, DecisionItem, DecisionKind, DecisionOption, DecisionReach,
    DecisionScope, EvidenceToken, OptionId, OptionLabel, OptionWarning, UnusableKind,
};
use wow_sync_core::kinds::EnrollmentRef;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionRow {
    pub id: OptionId,
    pub label: String,
    /// The verb on the button: "Keep this", or what the choice does when it is not a keep.
    pub button: String,
    /// "Held by Den PC and This computer"; empty when no one holds it.
    pub holders: String,
    /// What would happen, one plain sentence per effect.
    pub effects: Vec<String>,
    /// Shown before the choice; the owner must tick it (`needs_confirm`) to continue.
    pub warning: Option<String>,
    pub needs_confirm: bool,
}

impl OptionRow {
    /// Name for assistive technology: the release, who holds it and what choosing it does, so the
    /// identical "Keep this" buttons on one page are told apart.
    pub fn choose_name(&self) -> String {
        let mut parts = vec![if self.button == self.label {
            self.label.clone()
        } else {
            format!("{}: {}", self.button, self.label)
        }];
        if !self.holders.is_empty() {
            parts.push(self.holders.clone());
        }
        parts.extend(self.effects.iter().cloned());
        parts.join(". ")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionCard {
    pub id: DecisionId,
    pub title: String,
    pub why: String,
    /// Who the choice affects.
    pub reach: String,
    pub options: Vec<OptionRow>,
    pub evidence: EvidenceToken,
    /// No option exists: the only way forward is something other than a choice.
    pub no_options_note: Option<String>,
    /// Core never offers "Decide later" as an option; it is always available and changes nothing.
    pub hold_line: Option<String>,
}

fn name_of(w: &World, who: &EnrollmentRef) -> String {
    if who.computer_id.as_str() == w.computer_id {
        return "This computer".into();
    }
    w.peers
        .iter()
        .find(|p| p.id == who.computer_id.as_str())
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "A computer that is no longer listed".into())
}

fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

fn folders_text(keys: &[wow_sync_core::paths::FolderKey]) -> String {
    join_and(
        &keys
            .iter()
            .map(|k| k.as_str().to_string())
            .collect::<Vec<_>>(),
    )
}

pub fn scope_title(w: &World, s: &DecisionScope) -> String {
    match s {
        DecisionScope::Folders(keys) => {
            let names: Vec<String> = keys
                .iter()
                .map(|k| {
                    w.addon(k.as_str())
                        .map(|a| a.title.clone())
                        .unwrap_or_else(|| k.as_str().to_string())
                })
                .collect();
            join_and(&names)
        }
        DecisionScope::Settings(u) => format!("{} settings", u.folder.as_str()),
    }
}

/// The sentence for a newer version that exists only on this computer (Q-88, D-26).
pub fn local_only_sentence(lo: &wow_sync_runtime::LocalOnlyVersion) -> String {
    match &lo.version_label {
        Some(v) => format!(
            "Version {v} of {} is installed only on this computer and is not shared with the group.",
            lo.title
        ),
        None => format!(
            "A newer version of {} is installed only on this computer and is not shared with the group.",
            lo.title
        ),
    }
}

fn why_text(w: &World, item: &DecisionItem) -> String {
    let who = scope_title(w, &item.scope);
    match item.reason {
        DecisionKind::Ambiguous => {
            format!("Computers in the group have different copies of {who}, and WoW Sync can't tell which is newer.")
        }
        DecisionKind::UnusableMetadata(k) => {
            let what = match k {
                UnusableKind::NoCompatibleToc => "doesn't say which game version it is for",
                UnusableKind::Unreadable => "can't be read",
                UnusableKind::Unstable => "kept changing while WoW Sync read it",
                UnusableKind::Unsafe => "has files WoW Sync won't copy",
            };
            format!("This copy of {who} can't be used: it {what}. Reinstall or fix it on the computer that has it.")
        }
        DecisionKind::DifferentTocMapping | DecisionKind::UnvalidatedTocMapping => {
            format!("{who} is for a game version WoW Sync doesn't know how to match yet. It is waiting for a WoW Sync update that knows this game version.")
        }
        DecisionKind::PartialCandidate => {
            format!(
                "A copy of {who} is only partly available, so it can't be installed safely yet."
            )
        }
        DecisionKind::NewVersionAfterDecision => {
            let id = DecisionId(item.scope.clone());
            match w.features.local_only.iter().find(|l| l.decision == id) {
                Some(lo) => format!(
                    "{} It stays installed here. Keep the version you chose, use the new one for the whole group, follow the newest again, or decide later.",
                    local_only_sentence(lo)
                ),
                None => format!("A new version of {who} appeared after you made a choice about it."),
            }
        }
        DecisionKind::ChoicesDiffer => {
            format!("Two computers made different choices for {who} at the same time.")
        }
        DecisionKind::RollbackTargetsDiffer => {
            format!("Two computers went back to different versions of {who} at the same time.")
        }
        DecisionKind::RecoveredUnavailable => {
            format!("The version of {who} that you went back to is no longer available.")
        }
        DecisionKind::LookalikeOfRejected => {
            format!("A version of {who} looks like one the group rejected.")
        }
        DecisionKind::NewerAfterRollback => {
            let id = DecisionId(item.scope.clone());
            match w.features.local_only.iter().find(|l| l.decision == id) {
                Some(lo) => format!(
                    "{} The group went back to an older version, so it is not shared yet. It stays installed here until you choose: use the newer version for the whole group, or stay on the version you went back to.",
                    local_only_sentence(lo)
                ),
                None => format!(
                    "A newer version of {who} appeared after the group went back. Nothing changes until you choose: use the newer version, or stay on the version you went back to."
                ),
            }
        }
    }
}

pub fn reach_text(r: DecisionReach) -> &'static str {
    match r {
        DecisionReach::Group => "Your choice applies to every computer in the group.",
        DecisionReach::ThisComputer => "This only affects this computer.",
    }
}

fn label_text(w: &World, l: &OptionLabel, kind: DecisionKind) -> String {
    if kind == DecisionKind::NewVersionAfterDecision {
        match l {
            OptionLabel::Release { version_label } => {
                return format!("Use version {version_label}")
            }
            OptionLabel::KeepCurrent => return "Keep the version you chose".into(),
            OptionLabel::ChangeDecision => return "Follow newest again".into(),
            _ => {}
        }
    }
    if kind == DecisionKind::NewerAfterRollback {
        match l {
            OptionLabel::Release { version_label } => {
                return format!("Use version {version_label}")
            }
            OptionLabel::KeepCurrent => return "Stay on the version you went back to".into(),
            _ => {}
        }
    }
    match l {
        OptionLabel::Release { version_label } => format!("Keep version {version_label}"),
        OptionLabel::KeepCurrent => "Keep what is installed now".into(),
        OptionLabel::AllowAgain => "Allow this version again".into(),
        OptionLabel::ChangeDecision => "Change the earlier choice".into(),
        OptionLabel::RemoveEverywhere => "Remove it everywhere".into(),
        OptionLabel::KeepOnThisComputer => "Keep it on this computer only".into(),
        OptionLabel::UseSettingsFrom(id) => {
            let name = w
                .peers
                .iter()
                .find(|p| p.id == id.as_str())
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "that computer".into());
            format!("Use the settings from {name}")
        }
        OptionLabel::KeepMySettings => "Keep my settings".into(),
    }
}

fn warning_text(wn: &OptionWarning) -> String {
    match wn {
        OptionWarning::OlderThanInstalled => {
            "This is older than what is installed now. Installing it replaces the newer version here; the newer one is kept for recovery.".into()
        }
        OptionWarning::DropsFolders(f) => format!(
            "This version doesn't include {}. Those folders would be removed.",
            folders_text(f)
        ),
        OptionWarning::ReplacesSettings => {
            "This replaces the current settings. The current copy is kept for recovery.".into()
        }
    }
}

fn effects(c: &Consequence) -> Vec<String> {
    let mut v = Vec::new();
    if !c.installs.is_empty() {
        v.push(format!("Installs {}.", folders_text(&c.installs)));
    }
    if !c.removals.is_empty() {
        v.push(format!("Removes {}.", folders_text(&c.removals)));
    }
    if !c.settings_moves.is_empty() {
        let names: Vec<String> = c
            .settings_moves
            .iter()
            .map(|u| u.folder.as_str().to_string())
            .collect();
        v.push(format!("Moves settings of {}.", join_and(&names)));
    }
    v
}

fn row(w: &World, o: &DecisionOption, kind: DecisionKind) -> OptionRow {
    let holders: Vec<String> = o.holders.iter().map(|h| name_of(w, h)).collect();
    let label = label_text(w, &o.label, kind);
    let button = match (kind, &o.label) {
        (
            DecisionKind::NewVersionAfterDecision | DecisionKind::NewerAfterRollback,
            OptionLabel::Release { .. },
        ) => "Use this".into(),
        (DecisionKind::NewerAfterRollback, OptionLabel::KeepCurrent) => "Stay".into(),
        (DecisionKind::NewVersionAfterDecision, OptionLabel::ChangeDecision) => label.clone(),
        _ => "Keep this".into(),
    };
    OptionRow {
        id: o.id,
        label,
        button,
        holders: if holders.is_empty() {
            String::new()
        } else {
            format!("Held by {}", join_and(&holders))
        },
        effects: effects(&o.consequence),
        warning: o.warning.as_ref().map(warning_text),
        needs_confirm: o.warning.is_some(),
    }
}

pub fn card(w: &World, item: &DecisionItem) -> DecisionCard {
    let waiting = matches!(
        item.reason,
        DecisionKind::DifferentTocMapping | DecisionKind::UnvalidatedTocMapping
    );
    let no_options_note = item.options.is_empty().then(|| {
        if waiting {
            "Waiting for a WoW Sync update that knows this game version. Nothing to choose.".into()
        } else {
            "There is nothing to choose here. It is resolved on the computer that has the copy."
                .into()
        }
    });
    DecisionCard {
        id: DecisionId(item.scope.clone()),
        title: scope_title(w, &item.scope),
        why: why_text(w, item),
        reach: reach_text(item.reach).into(),
        options: item
            .options
            .iter()
            .map(|o| row(w, o, item.reason))
            .collect(),
        evidence: item.evidence,
        no_options_note,
        hold_line: hold_sentence(w),
    }
}

/// Every decision core currently offers; a pending decision never reads as "fully synced".
pub fn cards(w: &World) -> Vec<DecisionCard> {
    w.features.decisions.iter().map(|i| card(w, i)).collect()
}

/// Whether choosing is possible now, with the reason it is not.
pub fn choosing_gate(w: &World) -> Result<(), String> {
    gate(w, ActionKind::Decide)
}

/// The sentence shown with a decision for a computer that cannot publish right now.
pub fn standing_note(w: &World) -> Option<String> {
    match &w.features.standing {
        Standing::Active => None,
        Standing::Retired { .. } => Some(crate::features::RETIRED_REASON.into()),
        Standing::Held => {
            Some("Saved choices are shared if this computer is still in the group.".into())
        }
    }
}

/// Builds the action for choosing `option` on `card`. The warning (if any) must be confirmed; an
/// option core did not offer is refused. The evidence token is the one the card showed.
pub fn choose(
    w: &World,
    card: &DecisionCard,
    option: OptionId,
    confirmed: bool,
) -> Result<GroupAction, String> {
    choosing_gate(w)?;
    let Some(o) = card.options.iter().find(|o| o.id == option) else {
        return Err("That choice is not offered.".into());
    };
    if o.needs_confirm && !confirmed {
        return Err("Confirm that you understand the warning first.".into());
    }
    Ok(GroupAction::Decide {
        item: card.id.clone(),
        option,
        shown: card.evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Capabilities, RETIRED_REASON};
    use wow_sync_core::ids::{ComputerId, EnrollmentId, Sha256};
    use wow_sync_core::paths::validate_folder_name;

    const THIS: &str = "11111111111111111111111111111111";
    const DEN: &str = "22222222222222222222222222222222";

    fn eref(c: &str) -> EnrollmentRef {
        EnrollmentRef {
            computer_id: ComputerId::parse(c).unwrap(),
            enrollment_id: EnrollmentId::parse("33333333333333333333333333333333").unwrap(),
        }
    }

    fn item(options: Vec<DecisionOption>, reason: DecisionKind) -> DecisionItem {
        DecisionItem {
            scope: DecisionScope::Folders(vec![validate_folder_name("WeakAuras").unwrap()]),
            reason,
            reach: DecisionReach::Group,
            options,
            evidence: EvidenceToken(Sha256([7; 32])),
        }
    }

    fn opt(
        id: u32,
        label: OptionLabel,
        holder: &str,
        warn: Option<OptionWarning>,
    ) -> DecisionOption {
        DecisionOption {
            id: OptionId(id),
            label,
            holders: vec![eref(holder)],
            consequence: Consequence::default(),
            warning: warn,
        }
    }

    fn world(items: Vec<DecisionItem>) -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.computer_id = THIS.into();
        w.peers.push(crate::world::Peer {
            id: DEN.into(),
            name: "Den PC".into(),
            state: wow_sync_core::status::MemberSummary::Usable,
            report_secs_ago: Some(60),
            game: String::new(),
            app: String::new(),
            confirmed: 0,
        });
        w.features.decisions = items;
        w.features.caps = Capabilities::runtime();
        w
    }

    fn release(id: u32, v: &str, holder: &str) -> DecisionOption {
        opt(
            id,
            OptionLabel::Release {
                version_label: v.into(),
            },
            holder,
            None,
        )
    }

    #[test]
    fn a_new_version_after_keep_says_it_is_only_here_and_offers_the_four_answers() {
        let id = DecisionId(DecisionScope::Folders(vec![validate_folder_name(
            "WeakAuras",
        )
        .unwrap()]));
        let mut w = world(vec![item(
            vec![
                opt(0, OptionLabel::KeepCurrent, DEN, None),
                release(1, "9.1", THIS),
                opt(2, OptionLabel::ChangeDecision, THIS, None),
            ],
            DecisionKind::NewVersionAfterDecision,
        )]);
        w.features.local_only = vec![wow_sync_runtime::LocalOnlyVersion {
            package: validate_folder_name("WeakAuras").unwrap(),
            title: "WeakAuras".into(),
            scope: vec![validate_folder_name("WeakAuras").unwrap()],
            version_label: Some("9.1".into()),
            decision: id,
        }];
        let c = cards(&w).remove(0);
        assert!(c.why.starts_with(
            "Version 9.1 of WeakAuras is installed only on this computer and is not shared with the group."
        ));
        let labels: Vec<_> = c.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Keep the version you chose",
                "Use version 9.1",
                "Follow newest again"
            ]
        );
        let buttons: Vec<_> = c.options.iter().map(|o| o.button.as_str()).collect();
        assert_eq!(buttons, ["Keep this", "Use this", "Follow newest again"]);
    }

    #[test]
    fn identical_keep_buttons_have_distinct_names_with_release_holder_and_consequence() {
        let w = world(vec![item(
            vec![release(1, "5.1", DEN), release(2, "5.0", THIS)],
            DecisionKind::Ambiguous,
        )]);
        let c = cards(&w).remove(0);
        let a = c.options[0].choose_name();
        let b = c.options[1].choose_name();
        assert_ne!(a, b);
        assert!(a.contains("Keep version 5.1") && a.contains("Held by Den PC"));
        assert!(b.contains("Keep version 5.0") && b.contains("Held by This computer"));
        assert!(
            c.options[0].effects.iter().all(|e| a.contains(e.as_str())),
            "the consequence is part of the name"
        );
    }

    #[test]
    fn a_card_names_the_holders_and_who_the_choice_affects() {
        let w = world(vec![item(
            vec![release(1, "5.1", DEN), release(2, "5.0", THIS)],
            DecisionKind::Ambiguous,
        )]);
        let c = cards(&w).remove(0);
        assert_eq!(c.title, "WeakAuras".to_lowercase());
        assert_eq!(c.options[0].label, "Keep version 5.1");
        assert_eq!(c.options[0].holders, "Held by Den PC");
        assert_eq!(c.options[1].holders, "Held by This computer");
        assert_eq!(
            c.reach,
            "Your choice applies to every computer in the group."
        );
        assert!(c.why.contains("different copies"));
    }

    #[test]
    fn choosing_sends_the_shown_evidence_and_only_an_offered_option() {
        let w = world(vec![item(
            vec![release(1, "5.1", DEN)],
            DecisionKind::Ambiguous,
        )]);
        let c = cards(&w).remove(0);
        let a = choose(&w, &c, OptionId(1), false).unwrap();
        assert_eq!(
            a,
            GroupAction::Decide {
                item: c.id.clone(),
                option: OptionId(1),
                shown: EvidenceToken(Sha256([7; 32])),
            }
        );
        assert!(choose(&w, &c, OptionId(99), true).is_err(), "not offered");
    }

    #[test]
    fn a_warning_must_be_confirmed_before_the_choice_is_built() {
        let w = world(vec![item(
            vec![opt(
                1,
                OptionLabel::Release {
                    version_label: "4.0".into(),
                },
                DEN,
                Some(OptionWarning::OlderThanInstalled),
            )],
            DecisionKind::Ambiguous,
        )]);
        let c = cards(&w).remove(0);
        assert!(c.options[0].needs_confirm);
        assert!(c.options[0].warning.as_ref().unwrap().contains("older"));
        assert!(choose(&w, &c, OptionId(1), false).is_err());
        assert!(choose(&w, &c, OptionId(1), true).is_ok());
    }

    #[test]
    fn items_without_options_say_so_and_offer_nothing_to_send() {
        let w = world(vec![
            item(vec![], DecisionKind::UnvalidatedTocMapping),
            item(
                vec![],
                DecisionKind::UnusableMetadata(UnusableKind::Unreadable),
            ),
        ]);
        let cs = cards(&w);
        assert!(cs[0]
            .no_options_note
            .as_ref()
            .unwrap()
            .contains("WoW Sync update"));
        assert!(cs[1].why.contains("can't be read"));
        assert!(cs.iter().all(|c| c.options.is_empty()));
        assert!(choose(&w, &cs[0], OptionId(1), true).is_err());
    }

    #[test]
    fn a_retired_computer_cannot_choose_and_is_told_why() {
        let mut w = world(vec![item(
            vec![release(1, "5.1", DEN)],
            DecisionKind::Ambiguous,
        )]);
        w.features.standing = Standing::Retired {
            on: "12 Jan".into(),
            by: "Den PC".into(),
        };
        let c = cards(&w).remove(0);
        assert_eq!(
            choose(&w, &c, OptionId(1), true).unwrap_err(),
            RETIRED_REASON
        );
        assert_eq!(standing_note(&w).unwrap(), RETIRED_REASON);
    }

    #[test]
    fn settings_decisions_and_this_computer_reach_read_plainly() {
        let mut it = item(vec![], DecisionKind::ChoicesDiffer);
        it.reach = DecisionReach::ThisComputer;
        let w = world(vec![it]);
        let c = cards(&w).remove(0);
        assert_eq!(c.reach, "This only affects this computer.");
    }
}
