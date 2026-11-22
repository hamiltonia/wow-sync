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
    LocalResolution, OptionId, OptionLabel,
};
use crate::hash::sha256;
use crate::hooks::DecisionHooks;
use crate::ids::Sha256;
use crate::kinds::{ChoiceReason, CodeChoice, EnrollmentRef};
use crate::ledger::Ledger;
use crate::paths::FolderKey;
use crate::records::encode;
use crate::reject::{NewerAfterRollback, RejectBehaviour};
use crate::target::{DecisionReason, GroupTarget, PackageCopies, PackageTarget};
use std::collections::{BTreeMap, BTreeSet};

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
        .chain(nv.absent_kept.iter())
        .map(|c| CopyEntry::from_copy(c, &nv.scope))
        .collect::<Result<_, _>>()?;
    let kept_entry = match &nv.absent_kept {
        Some(_) => &shown[copies.len()],
        None => &shown[nv.kept],
    };
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
    let kept = nv.kept_copy(copies);
    push(
        OptionLabel::KeepCurrent,
        kept.holders.clone(),
        vec![],
        build_keep(
            view,
            cx,
            reason,
            &nv.scope,
            kept_entry,
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

/// The `NewerAfterRollback` items (rollback.md §3.1, Q-81): for each live rollback, one item per
/// package offering "Use <version>" for each raised copy (a keep that covers the rollback) and
/// "Stay" on the recovered version (a local dismissal of exactly these copies).
pub fn rollback_items(
    view: &ChoiceView,
    cx: &WriteCx,
    rejects: &RejectBehaviour,
    ledger: &Ledger,
    dismissed: &BTreeSet<Sha256>,
    packages: &[PackageCopies],
) -> Vec<(DecisionItem, Plans)> {
    let mut out = Vec::new();
    for pc in packages {
        for n in rejects.newer_after_rollbacks(ledger, &pc.package, &pc.copies) {
            let key = n.dismissal_key(&pc.copies);
            if dismissed.contains(&key) {
                continue;
            }
            if let Some(item) = rollback_item(view, cx, pc, &n, key) {
                out.push(item);
            }
        }
    }
    out
}

fn rollback_item(
    view: &ChoiceView,
    cx: &WriteCx,
    pc: &PackageCopies,
    n: &NewerAfterRollback,
    key: Sha256,
) -> Option<(DecisionItem, Plans)> {
    let shown: Vec<CopyEntry> = pc
        .copies
        .iter()
        .map(|c| CopyEntry::from_copy(c, &n.scope))
        .collect::<Result<_, _>>()
        .ok()?;
    let mut planned = Vec::new();
    for i in &n.copies {
        let c = &pc.copies[*i];
        let choice = build_keep(
            view,
            cx,
            ChoiceReason::NewerAfterRollback,
            &n.scope,
            &shown[*i],
            &shown,
            &c.source,
        )
        .ok()?;
        let label = c
            .version
            .folders
            .iter()
            .find_map(|f| f.toc.version.clone())
            .unwrap_or_default();
        planned.push(PlannedOption {
            option: DecisionOption {
                id: OptionId(planned.len() as u32),
                label: OptionLabel::Release {
                    version_label: label,
                },
                holders: c.holders.clone(),
                consequence: Consequence {
                    installs: n.scope.clone(),
                    ..Consequence::default()
                },
                warning: None,
            },
            effect: DecisionEffect::Write(Box::new(AuthorityRecord::CodeChoice(choice))),
        });
    }
    planned.push(PlannedOption {
        option: DecisionOption {
            id: OptionId(planned.len() as u32),
            label: OptionLabel::KeepCurrent,
            holders: pc.copies[n.recovered].holders.clone(),
            consequence: Consequence::default(),
            warning: None,
        },
        effect: DecisionEffect::Local(LocalResolution::Dismiss(key)),
    });
    let mut basis: Vec<Sha256> = pc.copies.iter().map(|c| c.version.hash).collect();
    basis.push(n.rejection);
    Some(build_item(
        DecisionScope::Folders(n.scope.clone()),
        DecisionKind::NewerAfterRollback,
        DecisionReach::Group,
        planned,
        &basis,
    ))
}

/// Every decision item the live state raises, each with the records its options would write:
/// `NewVersionAfterDecision` for live keeps (Q-49), and one item per package whose target needs
/// a decision for `Ambiguous`, `PartialCandidate`, `ChoicesDiffer` or `RollbackTargetsDiffer`.
/// A conflict item offers a keep of each copy and a keep on this computer only; none is chosen.
pub fn decision_items(
    view: &ChoiceView,
    cx: &WriteCx,
    target: &GroupTarget,
    packages: &[PackageCopies],
) -> Vec<(DecisionItem, Plans)> {
    let mut out = Vec::new();
    for pc in packages {
        for nv in view.new_versions(&pc.package, &pc.copies) {
            if let Ok(item) = new_version_item(view, cx, &pc.copies, &nv) {
                out.push(item);
            }
        }
        let Some(PackageTarget::NeedsDecision { reason, .. }) =
            target.packages.get(&pc.package.key)
        else {
            continue;
        };
        let (kind, why) = match reason {
            DecisionReason::Ambiguous => (DecisionKind::Ambiguous, ChoiceReason::Ambiguous),
            DecisionReason::PartialCandidate => (
                DecisionKind::PartialCandidate,
                ChoiceReason::PartialCandidate,
            ),
            DecisionReason::ChoicesDiffer => {
                (DecisionKind::ChoicesDiffer, ChoiceReason::ChoicesDiffer)
            }
            DecisionReason::RollbackTargetsDiffer => (
                DecisionKind::RollbackTargetsDiffer,
                ChoiceReason::RollbackTargetsDiffer,
            ),
            _ => continue,
        };
        if let Some(item) = conflict_item(view, cx, pc, kind, why) {
            out.push(item);
        }
    }
    out
}

fn conflict_item(
    view: &ChoiceView,
    cx: &WriteCx,
    pc: &PackageCopies,
    kind: DecisionKind,
    why: ChoiceReason,
) -> Option<(DecisionItem, Plans)> {
    let scope = pc.package.members.clone();
    let shown: Vec<CopyEntry> = pc
        .copies
        .iter()
        .map(|c| CopyEntry::from_copy(c, &scope))
        .collect::<Result<_, _>>()
        .ok()?;
    let mut planned = Vec::new();
    for (i, copy) in pc.copies.iter().enumerate() {
        let Ok(choice) = build_keep(view, cx, why, &scope, &shown[i], &shown, &copy.source) else {
            continue;
        };
        let label = copy
            .version
            .folders
            .iter()
            .find_map(|f| f.toc.version.clone())
            .unwrap_or_default();
        planned.push(PlannedOption {
            option: DecisionOption {
                id: OptionId(planned.len() as u32),
                label: OptionLabel::Release {
                    version_label: label,
                },
                holders: copy.holders.clone(),
                consequence: Consequence {
                    installs: scope.clone(),
                    ..Consequence::default()
                },
                warning: None,
            },
            effect: DecisionEffect::Write(Box::new(AuthorityRecord::CodeChoice(choice))),
        });
    }
    if planned.is_empty() {
        return None;
    }
    planned.push(PlannedOption {
        option: DecisionOption {
            id: OptionId(planned.len() as u32),
            label: OptionLabel::KeepOnThisComputer,
            holders: vec![],
            consequence: Consequence::default(),
            warning: None,
        },
        effect: DecisionEffect::Local(LocalResolution::KeepOnThisComputer(DecisionScope::Folders(
            scope.clone(),
        ))),
    });
    let mut basis: Vec<Sha256> = pc.copies.iter().map(|c| c.version.hash).collect();
    basis.extend(view.known_for(&scope).extra);
    Some(build_item(
        DecisionScope::Folders(scope),
        kind,
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
