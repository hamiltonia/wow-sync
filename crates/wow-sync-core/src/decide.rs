//! Decision freshness (contracts.md §5.5, ops.md §1).
//!
//! `decide` compares the evidence token the owner was shown with the token of the recomputed
//! item. A mismatch writes nothing (`Changed`). The records an option would write are planned
//! when the item is built, because only the builder knows the issuer, sequence and clock.

use crate::authority::AuthorityRecord;
use crate::choice::{
    build_keep, build_resume, ChoiceView, Copy, CopyEntry, NewVersion, WriteCx, WriteError,
};
use crate::decision::{
    Consequence, DecideError, DecideOutcome, DecisionEffect, DecisionId, DecisionItem,
    DecisionKind, DecisionOption, DecisionReach, DecisionScope, EvidenceToken, FreshEvidence,
    OptionId, OptionLabel,
};
use crate::hash::sha256;
use crate::hooks::DecisionHooks;
use crate::ids::Sha256;
use crate::kinds::{ChoiceReason, CodeChoice, EnrollmentRef};
use crate::paths::FolderKey;
use crate::records::encode;
use std::collections::BTreeMap;

/// The wire bytes of any authority record.
pub fn encode_authority(r: &AuthorityRecord) -> Vec<u8> {
    match r {
        AuthorityRecord::Removal(x) => encode(x),
        AuthorityRecord::Readd(x) => encode(x),
        AuthorityRecord::Ack(x) => encode(x),
        AuthorityRecord::CodeChoice(x) => encode(x),
        AuthorityRecord::Rejection(x) => encode(x),
        AuthorityRecord::Lift(x) => encode(x),
        AuthorityRecord::SettingsPolicy(x) => encode(x),
        AuthorityRecord::SettingsRevision(x) => encode(x),
        AuthorityRecord::Retirement(x) => encode(x),
        AuthorityRecord::Part(x) => encode(x),
    }
}

/// One option with what choosing it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOption {
    pub option: DecisionOption,
    pub effect: DecisionEffect,
}

/// The token covers the item's own content and `basis`: the hashes of every ledger record and
/// copy the item was built from. A newly learned record or copy therefore changes it.
pub fn evidence_token(
    scope: &DecisionScope,
    reason: DecisionKind,
    reach: DecisionReach,
    options: &[DecisionOption],
    basis: &[Sha256],
) -> EvidenceToken {
    let mut b = basis.to_vec();
    b.sort();
    b.dedup();
    let text = format!("{scope:?}|{reason:?}|{reach:?}|{options:?}|{b:?}");
    EvidenceToken(sha256(text.as_bytes()))
}

/// An item with its planned effects, keyed by (item id, option id).
pub fn build_item(
    scope: DecisionScope,
    reason: DecisionKind,
    reach: DecisionReach,
    planned: Vec<PlannedOption>,
    basis: &[Sha256],
) -> (DecisionItem, Plans) {
    let options: Vec<DecisionOption> = planned.iter().map(|p| p.option.clone()).collect();
    let evidence = evidence_token(&scope, reason, reach, &options, basis);
    let id = DecisionId(scope.clone());
    let plans = planned
        .into_iter()
        .map(|p| ((id.clone(), p.option.id), p.effect))
        .collect();
    (
        DecisionItem {
            scope,
            reason,
            reach,
            options,
            evidence,
        },
        plans,
    )
}

pub type Plans = Vec<((DecisionId, OptionId), DecisionEffect)>;

/// The `NewVersionAfterDecision` item for one live keep (ops.md §1.5, Q-49): keep the current
/// copy (now also considering the new ones), take a new copy, or resume following the newest.
/// Every option writes exactly one code choice; none is chosen for the owner.
pub fn new_version_item(
    view: &ChoiceView,
    cx: &WriteCx,
    copies: &[Copy],
    nv: &NewVersion,
) -> Result<(DecisionItem, Plans), WriteError> {
    let shown: Vec<CopyEntry> = copies
        .iter()
        .map(|c| CopyEntry::from_copy(c, &nv.scope))
        .collect::<Result<_, _>>()?;
    let mut planned = Vec::new();
    let mut next = 0u32;
    let mut push = |label: OptionLabel,
                    holders: Vec<EnrollmentRef>,
                    installs: Vec<FolderKey>,
                    choice: CodeChoice| {
        planned.push(PlannedOption {
            option: DecisionOption {
                id: OptionId(next),
                label,
                holders,
                consequence: Consequence {
                    installs,
                    ..Consequence::default()
                },
                warning: None,
            },
            effect: DecisionEffect::Write(Box::new(AuthorityRecord::CodeChoice(choice))),
        });
        next += 1;
    };
    let reason = ChoiceReason::NewVersionAfterDecision;
    let kept = &copies[nv.kept];
    push(
        OptionLabel::KeepCurrent,
        kept.holders.clone(),
        vec![],
        build_keep(
            view,
            cx,
            reason,
            &nv.scope,
            &shown[nv.kept],
            &shown,
            &kept.source,
        )?,
    );
    for i in &nv.copies {
        let c = &copies[*i];
        let label = c
            .version
            .folders
            .iter()
            .find_map(|f| f.toc.version.clone())
            .unwrap_or_default();
        push(
            OptionLabel::Release {
                version_label: label,
            },
            c.holders.clone(),
            nv.scope.clone(),
            build_keep(view, cx, reason, &nv.scope, &shown[*i], &shown, &c.source)?,
        );
    }
    push(
        OptionLabel::ChangeDecision,
        vec![],
        vec![],
        build_resume(view, cx, reason, &nv.scope, &[])?,
    );
    let mut basis: Vec<Sha256> = copies.iter().map(|c| c.version.hash).collect();
    basis.push(nv.keep);
    Ok(build_item(
        DecisionScope::Folders(nv.scope.clone()),
        DecisionKind::NewVersionAfterDecision,
        DecisionReach::Group,
        planned,
        &basis,
    ))
}

/// Whether this computer may still publish group records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Standing {
    #[default]
    Active,
    /// Retired or held: a group record is refused; a local-only choice still works.
    Blocked,
}

#[derive(Debug, Clone, Default)]
pub struct DecideBehaviour {
    plans: BTreeMap<(DecisionId, OptionId), DecisionEffect>,
    standing: Standing,
}

impl DecideBehaviour {
    pub fn new(plans: Plans, standing: Standing) -> Self {
        Self {
            plans: plans.into_iter().collect(),
            standing,
        }
    }
}

impl DecisionHooks for DecideBehaviour {
    fn decide(
        &self,
        item: &DecisionId,
        option: &OptionId,
        shown: &EvidenceToken,
        now: &FreshEvidence,
    ) -> DecideOutcome {
        let Some(current) = now
            .items
            .iter()
            .find(|i| DecisionId(i.scope.clone()) == *item)
        else {
            // The item no longer exists: what the owner saw is stale.
            return DecideOutcome::Refused(DecideError::Changed);
        };
        if current.evidence != *shown {
            return DecideOutcome::Refused(DecideError::Changed);
        }
        if !current.options.iter().any(|o| o.id == *option) {
            return DecideOutcome::Refused(DecideError::UnknownOption);
        }
        let Some(effect) = self.plans.get(&(item.clone(), *option)) else {
            return DecideOutcome::Refused(DecideError::NotAllowed);
        };
        if self.standing == Standing::Blocked && matches!(effect, DecisionEffect::Write(_)) {
            return DecideOutcome::Refused(DecideError::NotAllowed);
        }
        DecideOutcome::Effect(effect.clone())
    }
}
