//! Status derivation (core-api.md §8, R-12, A-24). A published record is never confirmation.

use crate::admit::{MemberLatest, MemberView};
use crate::exec::{ApplyOutcome, PackageResult};
use crate::explain::{render, Explanation};
use crate::ids::ComputerId;
use crate::inventory::FolderStatus;
use crate::paths::FolderKey;
use crate::plan::{ApplyPlan, PlanBlock};
use crate::records::{
    ApplyJournal, ApplyReport, CodeState, JournalPhase, RecoveryPoint, ReportKind,
};
use crate::target::{DecisionReason, GroupTarget, PackageTarget};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Headline {
    Ok,
    Working,
    Waiting,
    NeedsHelp,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageStatus {
    InSync,
    Available,
    AppliedHere,
    ConfirmedBy(Vec<ComputerId>),
    NeedsDecision(DecisionReason),
    Unavailable,
    /// A folder of the package is blocked for this client and is not synchronized.
    NotSynced,
    Interrupted,
    NeedsHelp(String),
    Blocked(PlanBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberSummary {
    NoState,
    Usable,
    Unusable,
    Regressed,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberStatus {
    pub computer: ComputerId,
    pub display_name: String,
    pub last_seq: Option<u64>,
    pub reported_at_peer_clock: Option<u64>,
    pub seen_locally_at: Option<i64>,
    pub state: MemberSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub headline: Headline,
    pub packages: Vec<(FolderKey, PackageStatus, Explanation)>,
    pub members: Vec<MemberStatus>,
    pub last_publish: Option<i64>,
    pub blocks: Vec<PlanBlock>,
    /// Pinned or retained points beyond the normal two per package.
    pub retained_extra_recovery: u32,
}

pub struct StatusInput<'a> {
    pub target: &'a GroupTarget,
    /// Members other than this computer.
    pub members: &'a [MemberView],
    /// Packages applied here and verified, not yet confirmed by a peer, with their plan explanation.
    pub applied_here: &'a [(FolderKey, Explanation)],
    pub interrupted: &'a [FolderKey],
    /// The game is observed running now; an interruption is blamed on it only then.
    pub game_running: bool,
    pub needs_help: &'a [(FolderKey, String)],
    pub blocks: &'a [PlanBlock],
    pub retained: &'a [RecoveryPoint],
    pub provider_error: bool,
    pub working: bool,
    pub last_publish_unix_ms: Option<i64>,
    /// When this computer last saw each peer's newest file (local clock).
    pub seen_locally: &'a BTreeMap<ComputerId, i64>,
}

fn folders_match(state: &CodeState, target: &GroupTarget, key: &FolderKey) -> bool {
    let Some(eff) = target.effective.get(key) else {
        return false;
    };
    eff.folders.iter().all(|(fk, tree)| {
        state
            .folders
            .iter()
            .any(|f| &f.key == fk && f.status == FolderStatus::Ok && f.tree == Some(*tree))
    })
}

/// A peer confirms only with its own later `Applied` report for the exact package content.
pub fn confirmations(
    target: &GroupTarget,
    members: &[MemberView],
) -> BTreeMap<FolderKey, Vec<ComputerId>> {
    let mut out: BTreeMap<FolderKey, Vec<ComputerId>> = BTreeMap::new();
    for m in members {
        let MemberLatest::Valid { state, .. } = &m.latest else {
            continue;
        };
        let Some(rep) = &state.apply_report else {
            continue;
        };
        for r in &rep.results {
            let Some(eff) = target.effective.get(&r.package) else {
                continue;
            };
            if r.result == ReportKind::Applied
                && r.package_hash == Some(eff.hash)
                && folders_match(state, target, &r.package)
            {
                out.entry(r.package.clone())
                    .or_default()
                    .push(m.computer.clone());
            }
        }
    }
    out
}

fn summarize(l: &MemberLatest) -> (MemberSummary, Option<u64>, Option<u64>) {
    match l {
        MemberLatest::Valid { seq, state } => (
            MemberSummary::Usable,
            Some(*seq),
            Some(state.written_unix_ms),
        ),
        MemberLatest::Unusable { seq, .. } => (MemberSummary::Unusable, Some(*seq), None),
        MemberLatest::Regressed { max_seq_seen } => {
            (MemberSummary::Regressed, Some(*max_seq_seen), None)
        }
        MemberLatest::InvalidMember(_) => (MemberSummary::Invalid, None, None),
        MemberLatest::NoState => (MemberSummary::NoState, None, None),
    }
}

pub fn derive_status(input: &StatusInput) -> StatusView {
    let confirmed = confirmations(input.target, input.members);
    let mut packages = Vec::new();
    for (key, t) in &input.target.packages {
        let (status, why) =
            if let Some((_, reason)) = input.needs_help.iter().find(|(k, _)| k == key) {
                (
                    PackageStatus::NeedsHelp(reason.clone()),
                    Explanation::UnknownInterruptedState {
                        package: key.as_str().to_string(),
                    },
                )
            } else if input.interrupted.contains(key) {
                (
                    PackageStatus::Interrupted,
                    if input.game_running {
                        Explanation::InterruptedByGame {
                            package: key.as_str().to_string(),
                        }
                    } else {
                        Explanation::Interrupted {
                            package: key.as_str().to_string(),
                        }
                    },
                )
            } else {
                match t {
                    PackageTarget::NeedsDecision { reason, why, .. } => {
                        (PackageStatus::NeedsDecision(*reason), why.clone())
                    }
                    PackageTarget::Unavailable { why } => (PackageStatus::Unavailable, why.clone()),
                    PackageTarget::NotSynced { why } => (PackageStatus::NotSynced, why.clone()),
                    PackageTarget::Chosen { why, .. } => match input.blocks.first() {
                        Some(b) => (PackageStatus::Blocked(b.clone()), why.clone()),
                        None => (PackageStatus::Available, why.clone()),
                    },
                    PackageTarget::InSync => match confirmed.get(key) {
                        Some(c) if !c.is_empty() => (
                            PackageStatus::ConfirmedBy(c.clone()),
                            Explanation::ConfirmedBy {
                                computers: c.clone(),
                            },
                        ),
                        _ => match input.applied_here.iter().find(|(k, _)| k == key) {
                            Some((k, plan_why)) => {
                                (PackageStatus::AppliedHere, applied_explanation(k, plan_why))
                            }
                            None => (PackageStatus::InSync, Explanation::InSync),
                        },
                    },
                }
            };
        packages.push((key.clone(), status, why));
    }
    let any = |f: fn(&PackageStatus) -> bool| packages.iter().any(|(_, s, _)| f(s));
    let headline = if any(|s| matches!(s, PackageStatus::NeedsHelp(_))) {
        Headline::NeedsHelp
    } else if input.provider_error {
        Headline::Error
    } else if input.working {
        Headline::Working
    } else if !any(|s| {
        !matches!(
            s,
            PackageStatus::InSync
                | PackageStatus::AppliedHere
                | PackageStatus::ConfirmedBy(_)
                | PackageStatus::NotSynced
        )
    }) {
        Headline::Ok
    } else {
        Headline::Waiting
    };
    let mut per_pkg: BTreeMap<&FolderKey, u32> = BTreeMap::new();
    for rp in input.retained {
        *per_pkg.entry(&rp.package).or_default() += 1;
    }
    let members = input
        .members
        .iter()
        .map(|m| {
            let (state, last_seq, peer) = summarize(&m.latest);
            MemberStatus {
                computer: m.computer.clone(),
                display_name: m.display_name.clone(),
                last_seq,
                reported_at_peer_clock: peer,
                seen_locally_at: input.seen_locally.get(&m.computer).copied(),
                state,
            }
        })
        .collect();
    StatusView {
        headline,
        packages,
        members,
        last_publish: input.last_publish_unix_ms,
        blocks: input.blocks.to_vec(),
        retained_extra_recovery: per_pkg.values().map(|n| n.saturating_sub(2)).sum(),
    }
}

/// Source and versions of a package this computer has just applied (G3), from its plan explanation.
pub fn applied_explanation(key: &FolderKey, plan: &Explanation) -> Explanation {
    match plan {
        Explanation::NewerVersion {
            folder,
            from,
            to,
            source,
        } => Explanation::AppliedFrom {
            package: folder.clone(),
            from: Some(from.clone()),
            to: Some(to.clone()),
            source: source.clone(),
        },
        Explanation::Installed { source } => Explanation::AppliedFrom {
            package: key.as_str().to_string(),
            from: None,
            to: None,
            source: source.clone(),
        },
        _ => Explanation::AppliedHere,
    }
}

/// The headline sentence (G4: "Waiting — WoW is running" while the game blocks changes).
pub fn render_headline(v: &StatusView) -> String {
    match v.headline {
        Headline::Ok => "This computer is up to date".to_string(),
        Headline::Working => "Working".to_string(),
        Headline::Waiting if v.blocks.contains(&PlanBlock::GameRunning) => {
            "Waiting \u{2014} WoW is running".to_string()
        }
        Headline::Waiting => "Waiting".to_string(),
        Headline::NeedsHelp => "Something needs your help".to_string(),
        Headline::Error => "Something needs attention".to_string(),
    }
}

/// Display only; a peer's own clock is labelled and never implies the member is current.
pub fn render_member(m: &MemberStatus) -> String {
    let state = match m.state {
        MemberSummary::NoState => "no data published yet",
        MemberSummary::Usable => "data received",
        MemberSummary::Unusable => "latest data could not be used",
        MemberSummary::Regressed => "data went backwards and was not used",
        MemberSummary::Invalid => "enrollment record is not valid",
    };
    let mut s = format!("{}: {state}", m.display_name);
    if let Some(t) = m.reported_at_peer_clock {
        s.push_str(&format!("; reported at {t} by its own clock (peer clock)"));
    }
    if let Some(t) = m.seen_locally_at {
        s.push_str(&format!("; last seen here at {t}"));
    }
    s
}

pub fn render_package(p: &(FolderKey, PackageStatus, Explanation)) -> String {
    format!("{}: {}", p.0.as_str(), render(&p.2))
}

/// Plain sentence for a headline outside a full status view (e.g. enrollment needs attention).
pub fn headline_sentence(h: Headline) -> String {
    render_headline(&StatusView {
        headline: h,
        packages: Vec::new(),
        members: Vec::new(),
        last_publish: None,
        blocks: Vec::new(),
        retained_extra_recovery: 0,
    })
}

pub fn render_block(b: &PlanBlock) -> String {
    match b {
        PlanBlock::GameRunning => "WoW is running; nothing was changed".into(),
        PlanBlock::ActivityUnknown(w) => {
            format!("Cannot tell whether the game is running ({w}); nothing was changed")
        }
        PlanBlock::Unstable => "Add-on files are still changing; waiting".into(),
        PlanBlock::UnresolvedJournal => "An earlier update is unresolved".into(),
        PlanBlock::NothingToDo => "Nothing to do".into(),
        PlanBlock::DestinationChanged => "An add-on changed while updating; will retry".into(),
        PlanBlock::RenameFailed(w) => format!("An add-on folder could not be replaced: {w}"),
        PlanBlock::Invalid(w) => format!("Invalid plan: {w}"),
    }
}

pub fn render_result(r: &PackageResult) -> String {
    match r {
        PackageResult::Applied { .. } => "applied".into(),
        PackageResult::Skipped(e) => format!("skipped - {}", render(e)),
        PackageResult::Replanned(e) => format!("will retry - {}", render(e)),
        PackageResult::Interrupted(e) => format!("interrupted - {}", render(e)),
        PackageResult::RolledBack(e) => format!("rolled back - {}", render(e)),
        PackageResult::NeedsHelp(e) => format!("needs help - {}", render(e)),
        PackageResult::Failed(e) => format!("failed - {}", render(e)),
    }
}

/// Journals present on disk split into interrupted packages and packages needing help
/// (apply.md §4: a `needs_help` journal waits for the user; any other phase is resumable).
pub type JournalSplit = (Vec<FolderKey>, Vec<(FolderKey, String)>);

pub fn journal_status(journals: &[ApplyJournal]) -> JournalSplit {
    let mut interrupted = Vec::new();
    let mut needs_help = Vec::new();
    for j in journals {
        if j.phase == JournalPhase::NeedsHelp {
            needs_help.push((j.package.clone(), j.reason.clone().unwrap_or_default()));
        } else {
            interrupted.push(j.package.clone());
        }
    }
    (interrupted, needs_help)
}

/// Remember the plan explanation of every package this outcome applied (newest wins).
pub fn remember_applied(
    remembered: &mut Vec<(FolderKey, Explanation)>,
    plan: &ApplyPlan,
    outcome: &ApplyOutcome,
) {
    for (k, r) in &outcome.packages {
        if matches!(r, PackageResult::Applied { .. }) {
            remembered.retain(|(a, _)| a != k);
            if let Some(p) = plan.packages.iter().find(|p| &p.package == k) {
                remembered.push((k.clone(), p.explanation.clone()));
            }
        }
    }
}

/// Packages shown as applied here (apply.md §6): those remembered this session with their
/// plan explanation, plus the `applied` results of this computer's own reports (latest
/// published or still pending) for the current target content, so a restart keeps them.
pub fn applied_here(
    remembered: &[(FolderKey, Explanation)],
    reports: &[&ApplyReport],
    target: &GroupTarget,
) -> Vec<(FolderKey, Explanation)> {
    let mut out = remembered.to_vec();
    for rep in reports {
        for r in &rep.results {
            let current = target.effective.get(&r.package).map(|e| e.hash);
            if r.result == ReportKind::Applied
                && r.package_hash.is_some()
                && r.package_hash == current
                && !out.iter().any(|(k, _)| k == &r.package)
            {
                out.push((r.package.clone(), Explanation::AppliedHere));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{ExpectedLive, FolderAction};
    use crate::records::{ApplyReport, ReportResult};
    use crate::testkit::{addon, op, Scenario};

    fn pk() -> FolderKey {
        FolderKey::lossy("suite")
    }

    fn base() -> Scenario {
        Scenario::new(vec![addon("Suite", Some("1.0"), "a")])
            .peer(2, vec![addon("Suite", Some("1.0"), "a")])
    }

    fn input<'a>(
        t: &'a GroupTarget,
        members: &'a [MemberView],
        seen: &'a BTreeMap<ComputerId, i64>,
    ) -> StatusInput<'a> {
        StatusInput {
            target: t,
            members,
            applied_here: &[],
            interrupted: &[],
            game_running: false,
            needs_help: &[],
            blocks: &[],
            retained: &[],
            provider_error: false,
            working: false,
            last_publish_unix_ms: None,
            seen_locally: seen,
        }
    }

    fn with_report(sc: Scenario, kind: ReportKind, hash_ok: bool) -> Scenario {
        sc.map_state(2, move |s: &mut CodeState| {
            let t = s.folders.first().and_then(|f| f.tree);
            let _ = t;
            s.apply_report = Some(ApplyReport {
                op: op(1),
                basis: vec![],
                results: vec![ReportResult {
                    package: pk(),
                    result: kind,
                    package_hash: if hash_ok {
                        Some(crate::hash::package_hash(&[(
                            FolderKey::lossy("suite"),
                            s.folders[0].tree.unwrap(),
                        )]))
                    } else {
                        None
                    },
                }],
            });
        })
    }

    #[test]
    fn published_is_not_confirmed() {
        let sc = base();
        let t = sc.target();
        let seen = BTreeMap::new();
        let v = derive_status(&StatusInput {
            applied_here: &[(
                pk(),
                Explanation::NewerVersion {
                    folder: "Suite".into(),
                    from: "1.0".into(),
                    to: "2.0".into(),
                    source: sc.members[0].computer.clone(),
                },
            )],
            ..input(&t, &sc.members, &seen)
        });
        assert_eq!(v.packages[0].1, PackageStatus::AppliedHere);
        let text = render_package(&v.packages[0]);
        assert!(
            text.contains("1.0")
                && text.contains("2.0")
                && text.contains("another computer")
                && !text.contains(sc.members[0].computer.as_str()),
            "post-apply status keeps source and versions without IDs: {text}"
        );
        assert!(confirmations(&t, &sc.members).is_empty());
    }

    #[test]
    fn confirmation_requires_peer_apply_report_and_trees() {
        let sc = with_report(base(), ReportKind::Applied, true);
        let t = sc.target();
        assert_eq!(
            confirmations(&t, &sc.members).get(&pk()).map(Vec::len),
            Some(1)
        );
        let seen = BTreeMap::new();
        assert!(matches!(
            derive_status(&input(&t, &sc.members, &seen)).packages[0].1,
            PackageStatus::ConfirmedBy(_)
        ));
        // Wrong package hash is not confirmation.
        let bad = with_report(base(), ReportKind::Applied, false);
        assert!(confirmations(&bad.target(), &bad.members).is_empty());
        // Report present but the peer's published trees differ from the target.
        let sc2 = Scenario::new(vec![addon("Suite", Some("1.0"), "a")])
            .peer(2, vec![addon("Suite", Some("1.0"), "a")]);
        let sc2 = sc2.map_state(2, |s: &mut CodeState| {
            let h = crate::hash::package_hash(&[(
                FolderKey::lossy("suite"),
                s.folders[0].tree.unwrap(),
            )]);
            s.apply_report = Some(ApplyReport {
                op: op(1),
                basis: vec![],
                results: vec![ReportResult {
                    package: pk(),
                    result: ReportKind::Applied,
                    package_hash: Some(h),
                }],
            });
            s.folders[0].tree = Some(crate::hash::sha256(b"other"));
        });
        assert!(confirmations(&sc2.target(), &sc2.members).is_empty());
    }

    #[test]
    fn interrupted_or_needs_help_report_not_confirmed() {
        for kind in [
            ReportKind::Interrupted,
            ReportKind::NeedsHelp,
            ReportKind::RolledBack,
            ReportKind::Failed,
            ReportKind::Skipped,
            ReportKind::Replanned,
        ] {
            let sc = with_report(base(), kind, true);
            assert!(
                confirmations(&sc.target(), &sc.members).is_empty(),
                "{kind:?}"
            );
        }
        let sc = base();
        let t = sc.target();
        let seen = BTreeMap::new();
        let nh = [(pk(), "live_mismatch:Suite".to_string())];
        let v = derive_status(&StatusInput {
            needs_help: &nh,
            ..input(&t, &sc.members, &seen)
        });
        assert!(matches!(v.packages[0].1, PackageStatus::NeedsHelp(_)));
        assert_eq!(v.headline, Headline::NeedsHelp);
        let ints = [pk()];
        let v = derive_status(&StatusInput {
            interrupted: &ints,
            ..input(&t, &sc.members, &seen)
        });
        assert_eq!(v.packages[0].1, PackageStatus::Interrupted);
    }

    #[test]
    fn peer_clock_never_makes_member_current() {
        let sc = base().written(2, 4_000_000_000_000);
        let t = sc.target();
        let mut seen = BTreeMap::new();
        seen.insert(sc.members[0].computer.clone(), 12);
        let v = derive_status(&input(&t, &sc.members, &seen));
        let m = &v.members[0];
        assert_eq!(m.reported_at_peer_clock, Some(4_000_000_000_000));
        assert_eq!(m.seen_locally_at, Some(12));
        let text = render_member(m).to_lowercase();
        assert!(
            text.contains("peer clock")
                && !text.contains("current")
                && !text.contains("up to date"),
            "{text}"
        );
    }

    #[test]
    fn needs_decision_shows_evidence() {
        let sc = Scenario::new(vec![addon("Suite", Some("1.0"), "a")])
            .peer(2, vec![addon("Suite", Some("1.0"), "b")]);
        let t = sc.target();
        let seen = BTreeMap::new();
        let v = derive_status(&input(&t, &sc.members, &seen));
        match &v.packages[0] {
            (
                _,
                PackageStatus::NeedsDecision(DecisionReason::Ambiguous),
                Explanation::AmbiguousVersions {
                    evidence,
                    candidates,
                },
            ) => {
                assert!(!evidence.is_empty());
                assert_eq!(candidates.len(), 2);
                assert!(candidates.iter().any(|c| c.here && c.sources.is_empty()));
                assert!(candidates
                    .iter()
                    .any(|c| !c.here && c.sources == vec![sc.members[0].computer.clone()]));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(v.headline, Headline::Waiting);
        let text = render_package(&v.packages[0]);
        assert!(
            text.contains("cannot tell which version is newer"),
            "{text}"
        );
        assert!(
            text.contains("this computer")
                && text.contains("another computer")
                && !text.contains(sc.members[0].computer.as_str()),
            "each candidate's holder is named without IDs: {text}"
        );
        assert!(text.contains("version 1.0 on"), "{text}");
    }

    #[test]
    fn interruption_blames_the_game_only_when_it_is_running() {
        let sc = base();
        let t = sc.target();
        let seen = BTreeMap::new();
        let ints = [pk()];
        let text = |game_running| {
            let v = derive_status(&StatusInput {
                interrupted: &ints,
                game_running,
                ..input(&t, &sc.members, &seen)
            });
            render_package(&v.packages[0])
        };
        assert!(text(true).contains("WoW started"));
        let calm = text(false);
        assert!(!calm.contains("WoW"), "{calm}");
        assert!(calm.contains("interrupted"));
    }

    #[test]
    fn headline_names_running_game_while_waiting() {
        let sc = Scenario::new(vec![addon("Suite", Some("1.0"), "a")])
            .peer(2, vec![addon("Suite", Some("2.0"), "b")]);
        let t = sc.target();
        let seen = BTreeMap::new();
        let blocks = [PlanBlock::GameRunning];
        let v = derive_status(&StatusInput {
            blocks: &blocks,
            ..input(&t, &sc.members, &seen)
        });
        assert_eq!(v.headline, Headline::Waiting);
        assert_eq!(render_headline(&v), "Waiting \u{2014} WoW is running");
        let v = derive_status(&input(&t, &sc.members, &seen));
        assert_eq!(render_headline(&v), "Waiting");
    }

    #[test]
    fn blocked_local_folder_is_not_synced_not_up_to_date() {
        let sc = Scenario::new(vec![
            addon("Good", Some("1.0"), "g"),
            crate::testkit::addon_unvalidated("Odd", "1.0", "o"),
        ]);
        let t = sc.target();
        let seen = BTreeMap::new();
        let v = derive_status(&input(&t, &sc.members, &seen));
        let odd = v.packages.iter().find(|p| p.0.as_str() == "odd").unwrap();
        assert_eq!(odd.1, PackageStatus::NotSynced);
        let text = render_package(odd);
        assert!(
            text.contains("not synced") && !text.contains("up to date"),
            "{text}"
        );
        let good = v.packages.iter().find(|p| p.0.as_str() == "good").unwrap();
        assert_eq!(good.1, PackageStatus::InSync);
        assert!(!t.effective.contains_key(&FolderKey::lossy("odd")));
        assert_eq!(v.headline, Headline::Ok);
    }

    #[test]
    fn headline_priority() {
        let sc = base();
        let t = sc.target();
        let seen = BTreeMap::new();
        let nh = [(pk(), "x".to_string())];
        let mut i = input(&t, &sc.members, &seen);
        assert_eq!(derive_status(&i).headline, Headline::Ok);
        i.working = true;
        assert_eq!(derive_status(&i).headline, Headline::Working);
        i.provider_error = true;
        assert_eq!(derive_status(&i).headline, Headline::Error);
        i.needs_help = &nh;
        assert_eq!(derive_status(&i).headline, Headline::NeedsHelp);
        let _ = (ExpectedLive::Absent, FolderAction::Install);
    }

    #[test]
    fn applied_here_is_rebuilt_from_own_report_after_restart() {
        let sc = base();
        let t = sc.target();
        let hash = t.effective.get(&pk()).unwrap().hash;
        let rep = |kind, h| ApplyReport {
            op: op(1),
            basis: vec![],
            results: vec![ReportResult {
                package: pk(),
                result: kind,
                package_hash: Some(h),
            }],
        };
        let ok = rep(ReportKind::Applied, hash);
        let derived = applied_here(&[], &[&ok], &t);
        assert_eq!(derived, vec![(pk(), Explanation::AppliedHere)]);
        let seen = BTreeMap::new();
        let v = derive_status(&StatusInput {
            applied_here: &derived,
            ..input(&t, &sc.members, &seen)
        });
        assert_eq!(v.packages[0].1, PackageStatus::AppliedHere);
        // Other content, or anything but `applied`, is not applied here.
        let other = rep(ReportKind::Applied, crate::hash::sha256(b"other"));
        let rolled = rep(ReportKind::RolledBack, hash);
        assert!(applied_here(&[], &[&other, &rolled], &t).is_empty());
        // A remembered explanation is kept and not duplicated.
        let remembered = [(pk(), Explanation::InSync)];
        assert_eq!(applied_here(&remembered, &[&ok], &t), remembered.to_vec());
    }

    #[test]
    fn needs_help_journal_is_not_reported_as_interrupted() {
        let mut nh = crate::testkit::sample_journal(JournalPhase::NeedsHelp);
        nh.package = FolderKey::lossy("broken");
        nh.reason = Some("live_mismatch:Broken".into());
        let mut j = vec![nh];
        for p in [
            JournalPhase::Staging,
            JournalPhase::Committing,
            JournalPhase::Committed,
            JournalPhase::Verified,
            JournalPhase::RolledBack,
        ] {
            j.push(crate::testkit::sample_journal(p));
        }
        let (interrupted, needs_help) = journal_status(&j);
        assert_eq!(
            needs_help,
            vec![(FolderKey::lossy("broken"), "live_mismatch:Broken".into())]
        );
        assert_eq!(interrupted, vec![pk(); 5]);
    }
}
