//! Status derivation (core-api.md §8, R-12, A-24). A published record is never confirmation.

use crate::admit::{MemberLatest, MemberView};
use crate::explain::{render, Explanation};
use crate::ids::ComputerId;
use crate::inventory::FolderStatus;
use crate::paths::FolderKey;
use crate::plan::PlanBlock;
use crate::records::{CodeState, RecoveryPoint, ReportKind};
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
    /// Packages applied here and verified, not yet confirmed by a peer.
    pub applied_here: &'a [FolderKey],
    pub interrupted: &'a [FolderKey],
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
                    Explanation::InterruptedByGame {
                        package: key.as_str().to_string(),
                    },
                )
            } else {
                match t {
                    PackageTarget::NeedsDecision { reason, why, .. } => {
                        (PackageStatus::NeedsDecision(*reason), why.clone())
                    }
                    PackageTarget::Unavailable { why } => (PackageStatus::Unavailable, why.clone()),
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
                        _ if input.applied_here.contains(key) => {
                            (PackageStatus::AppliedHere, Explanation::AppliedHere)
                        }
                        _ => (PackageStatus::InSync, Explanation::InSync),
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
    } else if any(|s| {
        matches!(
            s,
            PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_)
        )
    }) && !any(|s| {
        !matches!(
            s,
            PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_)
        )
    }) || packages.is_empty()
    {
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
            applied_here: &[pk()],
            ..input(&t, &sc.members, &seen)
        });
        assert_eq!(v.packages[0].1, PackageStatus::AppliedHere);
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
                Explanation::AmbiguousVersions { evidence },
            ) => assert!(!evidence.is_empty()),
            other => panic!("{other:?}"),
        }
        assert_eq!(v.headline, Headline::Waiting);
        assert!(render_package(&v.packages[0]).contains("cannot tell which version is newer"));
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
}
