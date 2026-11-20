//! Pure view-model: wording, tones, counts, filters and accessible names derived from a
//! `World`. No GPUI types here, so it runs on every platform and is unit tested.

use crate::scenario::{absolute, rel, Addon, Busy, DecisionPhase, FakeApplyScope, Kind, World};
use crate::theme::Tone;
use wow_sync_core::plan::PlanBlock;
use wow_sync_core::status::{Headline, MemberSummary, PackageStatus};
use wow_sync_core::target::DecisionReason;

/// Peers silent for at least this many days read "Not heard from" (Q-27 is open; fake value).
pub const STALE_DAYS: i64 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Check,
    Sync,
    Hourglass,
    Alert,
    Error,
    Clock,
    Dashed,
    Slashed,
    Pause,
    Chevron,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Icon::Check => "icons/check.svg",
            Icon::Sync => "icons/sync.svg",
            Icon::Hourglass => "icons/hourglass.svg",
            Icon::Alert => "icons/alert.svg",
            Icon::Error => "icons/error.svg",
            Icon::Clock => "icons/clock.svg",
            Icon::Dashed => "icons/dashed.svg",
            Icon::Slashed => "icons/slashed.svg",
            Icon::Pause => "icons/pause.svg",
            Icon::Chevron => "icons/chevron.svg",
        }
    }
}

pub fn tone_icon(t: Tone) -> Icon {
    match t {
        Tone::Ok => Icon::Check,
        Tone::Working => Icon::Sync,
        Tone::Waiting => Icon::Hourglass,
        Tone::Help => Icon::Alert,
        Tone::Error => Icon::Error,
        Tone::Unknown => Icon::Dashed,
        Tone::Paused => Icon::Pause,
    }
}

// ---------------------------------------------------------------------------------------------
// Pages

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Overview,
    Computers,
    Addons,
    Needs,
    Activity,
    SettingsSync,
    AppSettings,
    About,
    Scenarios,
}

pub const NAV: [Page; 9] = [
    Page::Overview,
    Page::Computers,
    Page::Addons,
    Page::Needs,
    Page::Activity,
    Page::SettingsSync,
    Page::AppSettings,
    Page::About,
    Page::Scenarios,
];

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Computers => "Computers",
            Page::Addons => "Add-ons",
            Page::Needs => "Needs attention",
            Page::Activity => "Activity",
            Page::SettingsSync => "Settings sync",
            Page::AppSettings => "App settings",
            Page::About => "About",
            Page::Scenarios => "Prototype scenarios",
        }
    }

    pub fn parse(s: &str) -> Option<Page> {
        let s = s.to_ascii_lowercase().replace('-', "");
        NAV.iter()
            .copied()
            .find(|p| p.title().to_ascii_lowercase().replace([' ', '-'], "") == s.replace(' ', ""))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Detail {
    None,
    Addon(String),
    Computer(usize),
    Decision(String),
    NeedsHelp(String),
    Reconfirm,
}

// ---------------------------------------------------------------------------------------------
// Headline, context, tray

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    pub text: String,
    pub tone: Tone,
    pub icon: Icon,
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

pub fn headline(w: &World) -> Head {
    let (text, tone) = if !w.enrolled {
        ("Set up this computer".to_string(), Tone::Unknown)
    } else if w.busy == Some(Busy::Pausing) {
        (
            "Pausing, finishing the current step".to_string(),
            Tone::Paused,
        )
    } else if w.is_paused() {
        ("Paused".to_string(), Tone::Paused)
    } else if w.reconfirm.is_some() {
        // The design gives reconfirm no headline; showing "up to date" while syncing is stopped
        // would be false (recorded as a design disagreement).
        (
            "Confirm the game version to continue syncing".to_string(),
            Tone::Help,
        )
    } else {
        match w.view.headline {
            Headline::Ok => ("This computer is up to date".to_string(), Tone::Ok),
            Headline::Working => (
                match &w.busy {
                    Some(Busy::Syncing { done, total }) if *total > 0 => {
                        format!("Syncing: applying {done} of {total} updates")
                    }
                    _ => "Syncing".to_string(),
                },
                Tone::Working,
            ),
            Headline::Waiting => (waiting_sentence(w), Tone::Waiting),
            Headline::NeedsHelp => ("An update needs your help".to_string(), Tone::Help),
            Headline::Error => ("Can't read the shared folder".to_string(), Tone::Error),
        }
    };
    Head {
        icon: tone_icon(tone),
        text,
        tone,
    }
}

fn waiting_sentence(w: &World) -> String {
    if w.view.blocks.contains(&PlanBlock::GameRunning) {
        return "Waiting: WoW is running".into();
    }
    let unavailable = w
        .view
        .packages
        .iter()
        .any(|(_, s, _)| *s == PackageStatus::Unavailable);
    if unavailable {
        return "Waiting for Dropbox to finish downloading".into();
    }
    if w.view.blocks.contains(&PlanBlock::Unstable) {
        return "Waiting for add-on files to stop changing".into();
    }
    let n = w
        .view
        .packages
        .iter()
        .filter(|(_, s, _)| matches!(s, PackageStatus::NeedsDecision(_)))
        .count();
    if n > 0 {
        return format!(
            "Waiting for your decision on {}",
            plural(n, "add-on", "add-ons")
        );
    }
    "Waiting".into()
}

pub fn context_line(w: &World) -> String {
    if !w.enrolled {
        return "Setup: nothing has been changed yet".into();
    }
    format!(
        "{} · \"{}\" group · this computer: {}",
        w.identity.display_label(),
        w.group,
        w.computer
    )
}

pub fn headline_a11y(w: &World) -> String {
    format!(
        "{}. {}",
        headline(w).text,
        context_line(w).replace(" · ", ", ").replace('"', "")
    )
}

pub fn tray_tooltip(w: &World) -> String {
    format!("Prototype: WoW Sync: {}", headline(w).text)
}

/// Window title for the page (screen readers announce the change).
pub fn window_title(page_title: &str) -> String {
    format!("{page_title} · WoW Sync")
}

// ---------------------------------------------------------------------------------------------
// Add-on rows

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    NotUpToDate,
    NeedsYou,
    NotSynced,
}

pub const FILTERS: [Filter; 4] = [
    Filter::All,
    Filter::NotUpToDate,
    Filter::NeedsYou,
    Filter::NotSynced,
];

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::NotUpToDate => "Not up to date",
            Filter::NeedsYou => "Needs you",
            Filter::NotSynced => "Not synced here",
        }
    }
}

pub fn up_to_date(s: &PackageStatus) -> bool {
    matches!(
        s,
        PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_)
    )
}

fn needs_you(s: &PackageStatus) -> bool {
    matches!(
        s,
        PackageStatus::NeedsDecision(_) | PackageStatus::NeedsHelp(_)
    )
}

pub fn filter_matches(f: Filter, s: &PackageStatus) -> bool {
    match f {
        Filter::All => true,
        Filter::NotUpToDate => !up_to_date(s) && !matches!(s, PackageStatus::NotSynced),
        Filter::NeedsYou => needs_you(s),
        Filter::NotSynced => matches!(s, PackageStatus::NotSynced),
    }
}

pub fn filter_count(w: &World, f: Filter) -> usize {
    w.view
        .packages
        .iter()
        .filter(|(_, s, _)| filter_matches(f, s))
        .count()
}

pub fn status_of<'a>(w: &'a World, key: &str) -> Option<&'a PackageStatus> {
    w.view
        .packages
        .iter()
        .find(|(k, _, _)| k.as_str() == key)
        .map(|(_, s, _)| s)
}

fn rank(s: &PackageStatus) -> u8 {
    match s {
        PackageStatus::NeedsHelp(_) | PackageStatus::NeedsDecision(_) => 0,
        PackageStatus::Available
        | PackageStatus::Unavailable
        | PackageStatus::Blocked(_)
        | PackageStatus::Interrupted => 1,
        _ => 2,
    }
}

#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    pub tone: Tone,
    pub icon: Icon,
}

fn label(text: &str, tone: Tone, icon: Icon) -> Label {
    Label {
        text: text.into(),
        tone,
        icon,
    }
}

/// Short list label for a core `PackageStatus` in the current context (docs: Add-on status).
pub fn status_label(w: &World, s: &PackageStatus) -> Label {
    let paused = w.is_paused();
    match s {
        PackageStatus::InSync => label("Up to date", Tone::Ok, Icon::Check),
        PackageStatus::ConfirmedBy(_) => label("Up to date", Tone::Ok, Icon::Check),
        PackageStatus::AppliedHere => label("Updated here", Tone::Ok, Icon::Check),
        PackageStatus::Available => {
            if paused {
                label("Waiting: syncing is paused", Tone::Paused, Icon::Pause)
            } else if w.apply_scope == FakeApplyScope::AllAppliesBlocked {
                label(
                    "Not applied: an earlier update needs help",
                    Tone::Waiting,
                    Icon::Hourglass,
                )
            } else if matches!(w.busy, Some(Busy::Syncing { .. })) {
                label("Updating…", Tone::Working, Icon::Sync)
            } else {
                label("Update ready", Tone::Working, Icon::Sync)
            }
        }
        PackageStatus::Blocked(PlanBlock::GameRunning) => {
            label("Waiting for WoW to close", Tone::Waiting, Icon::Hourglass)
        }
        PackageStatus::Blocked(_) => label("Waiting", Tone::Waiting, Icon::Hourglass),
        PackageStatus::Removing => label("Removing", Tone::Working, Icon::Sync),
        PackageStatus::Held => label("Waiting", Tone::Waiting, Icon::Hourglass),
        PackageStatus::Unavailable => label("Waiting for Dropbox", Tone::Waiting, Icon::Hourglass),
        PackageStatus::Interrupted => label("Update interrupted", Tone::Waiting, Icon::Hourglass),
        PackageStatus::NeedsDecision(_) => label("Needs your decision", Tone::Help, Icon::Alert),
        PackageStatus::NeedsHelp(_) => label("Needs help", Tone::Help, Icon::Alert),
        PackageStatus::NotSynced => label("Not synced here", Tone::Unknown, Icon::Slashed),
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub key: String,
    pub title: String,
    pub version: String,
    pub label: Label,
    pub confirmed_by: String,
    pub a11y: String,
}

pub fn peer_index(id: &str) -> usize {
    usize::from_str_radix(id, 16).unwrap_or(1).saturating_sub(1)
}

pub fn confirmed_names(w: &World, s: &PackageStatus) -> Vec<String> {
    match s {
        PackageStatus::ConfirmedBy(ids) => ids
            .iter()
            .filter_map(|id| w.peers.get(peer_index(id.as_str())).map(|p| p.name.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn addon_rows(w: &World, f: Filter) -> Vec<Row> {
    let mut rows: Vec<(u8, Row)> = Vec::new();
    for (key, st, _) in &w.view.packages {
        if !filter_matches(f, st) {
            continue;
        }
        let Some(a) = w.addon(key.as_str()) else {
            continue;
        };
        let names = confirmed_names(w, st);
        let confirmed_by = if names.is_empty() {
            if matches!(st, PackageStatus::NotSynced) {
                "—".to_string()
            } else {
                "Not confirmed yet".to_string()
            }
        } else {
            names.join(", ")
        };
        let label = status_label(w, st);
        let a11y = format!(
            "{}, {}, version {}, {}",
            a.title,
            label.text.to_lowercase(),
            a.version,
            if names.is_empty() {
                "not confirmed yet".to_string()
            } else {
                format!("confirmed by {}", confirmed_by)
            }
        );
        rows.push((
            rank(st),
            Row {
                key: key.as_str().to_string(),
                title: a.title.clone(),
                version: a.version.clone(),
                label,
                confirmed_by,
                a11y,
            },
        ));
    }
    rows.sort_by_key(|r| (r.0, r.1.title.to_lowercase()));
    rows.into_iter().map(|(_, r)| r).collect()
}

pub fn list_a11y(name: &str, count: usize, filter: Option<Filter>) -> String {
    match filter {
        Some(f) => format!("{name}, {count} items, filter {}", f.label()),
        None => format!("{name}, {count} items"),
    }
}

// ---------------------------------------------------------------------------------------------
// Counts and overview text

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub up_to_date: usize,
    pub ready: usize,
    pub waiting: usize,
    pub needs_decision: usize,
    pub needs_help: usize,
    pub not_synced: usize,
}

pub fn counts(w: &World) -> Counts {
    let mut c = Counts::default();
    for (_, s, _) in &w.view.packages {
        match s {
            PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_) => {
                c.up_to_date += 1
            }
            PackageStatus::Available => c.ready += 1,
            PackageStatus::Unavailable
            | PackageStatus::Blocked(_)
            | PackageStatus::Interrupted
            | PackageStatus::Removing
            | PackageStatus::Held => c.waiting += 1,
            PackageStatus::NeedsDecision(_) => c.needs_decision += 1,
            PackageStatus::NeedsHelp(_) => c.needs_help += 1,
            PackageStatus::NotSynced => c.not_synced += 1,
        }
    }
    c
}

pub fn counts_sentence(w: &World) -> String {
    let c = counts(w);
    if w.view.packages.is_empty() {
        return "No add-ons found yet".into();
    }
    let mut parts = vec![format!("{} up to date", c.up_to_date)];
    if c.ready > 0 {
        parts.push(format!(
            "{} {} ready",
            c.ready,
            if c.ready == 1 { "update" } else { "updates" }
        ));
    }
    if c.waiting > 0 {
        parts.push(format!("{} waiting", c.waiting));
    }
    if c.needs_decision > 0 {
        parts.push(format!("{} needs your decision", c.needs_decision));
    }
    if c.needs_help > 0 {
        parts.push(format!("{} needs help", c.needs_help));
    }
    if c.not_synced > 0 {
        parts.push(format!("{} not synced here", c.not_synced));
    }
    let mut s = parts.join(", ");
    if let Some(stale) = w.stale_secs {
        s.push_str(&format!(" (as of {})", rel(stale)));
    }
    s
}

pub fn paused_hint(w: &World) -> Option<String> {
    if !w.is_paused() {
        return None;
    }
    let n = counts(w).ready;
    Some(format!(
        "{} will be applied when you resume",
        plural(n, "update", "updates")
    ))
}

pub fn game_state(w: &World) -> &'static str {
    if w.game_running {
        "Running"
    } else {
        "Not running"
    }
}

/// The one-sentence continuation for the case shown (Continuation scope). Core does not
/// export which case applies (UG-25), so the prototype reads the fake typed scope.
pub fn continuation(w: &World) -> Option<String> {
    let c = counts(w);
    if c.needs_help > 0 && w.apply_scope == FakeApplyScope::AllAppliesBlocked {
        return Some(
            "No updates are applied on this computer until this is resolved. Sharing this computer's add-ons continues."
                .into(),
        );
    }
    if c.needs_decision > 0 {
        let names = decision_names(w);
        return Some(if names.len() == 1 {
            format!("Only {} is affected. Other add-ons keep syncing.", names[0])
        } else {
            format!(
                "Only these {} add-ons are affected. Other add-ons keep syncing.",
                names.len()
            )
        });
    }
    None
}

pub fn decision_names(w: &World) -> Vec<String> {
    w.addons
        .iter()
        .filter(|a| matches!(a.kind, Kind::NeedsDecision(_)))
        .map(|a| a.title.clone())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Banners

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerAction {
    None,
    Resume,
    TryAgain,
    Reconfirm,
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub text: String,
    pub tone: Tone,
    pub action: BannerAction,
    pub action_label: &'static str,
}

pub fn banners(w: &World) -> (Vec<Banner>, usize) {
    let mut all = Vec::new();
    if let Some(e) = &w.provider_error {
        all.push(Banner {
            text: format!("{e}. Start Dropbox, then choose Try again."),
            tone: Tone::Error,
            action: BannerAction::TryAgain,
            action_label: "Try again",
        });
    }
    if w.reconfirm.is_some() {
        all.push(Banner {
            text: "Confirm the game version to continue syncing".into(),
            tone: Tone::Help,
            action: BannerAction::Reconfirm,
            action_label: "Confirm…",
        });
    }
    if w.is_paused() {
        all.push(Banner {
            text: "Syncing is paused on this computer. Updates from other computers are not applied here, and changes here are not shared."
                .into(),
            tone: Tone::Paused,
            action: BannerAction::Resume,
            action_label: "Resume syncing",
        });
    }
    if w.game_running {
        let n = counts(w).ready + counts(w).waiting;
        all.push(Banner {
            text: format!(
                "WoW is running. WoW Sync never changes game files while the game runs. {} will be applied after you close WoW.",
                plural(n.max(2), "update", "updates")
            ),
            tone: Tone::Waiting,
            action: BannerAction::None,
            action_label: "",
        });
    }
    let more = all.len().saturating_sub(2);
    all.truncate(2);
    (all, more)
}

// ---------------------------------------------------------------------------------------------
// Needs attention

#[derive(Clone, Debug)]
pub struct NeedItem {
    pub title: String,
    pub reason: String,
    pub affects: String,
    pub button: &'static str,
    pub tone: Tone,
    pub target: NeedTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeedTarget {
    Error,
    Reconfirm,
    NeedsHelp(String),
    Decision(String),
    Computer(usize),
}

pub fn need_items(w: &World) -> Vec<NeedItem> {
    let mut v = Vec::new();
    if let Some(e) = &w.provider_error {
        v.push(NeedItem {
            title: "Can't read the shared folder".into(),
            reason: format!("{e}. Start Dropbox, then choose Try again."),
            affects: "Nothing can sync until this is fixed".into(),
            button: "See what to do",
            tone: Tone::Error,
            target: NeedTarget::Error,
        });
    }
    if w.reconfirm.is_some() {
        v.push(NeedItem {
            title: "Confirm the game version to keep syncing".into(),
            reason: "The add-on file rule for this game version changed.".into(),
            affects: "All syncing on this computer stays stopped".into(),
            button: "Confirm…",
            tone: Tone::Help,
            target: NeedTarget::Reconfirm,
        });
    }
    for a in &w.addons {
        if a.kind == Kind::NeedsHelp {
            v.push(NeedItem {
                title: format!("{}: an update stopped unexpectedly", a.title),
                reason: "The old and new files are both kept.".into(),
                affects: "All updates on this computer".into(),
                button: "See what happened",
                tone: Tone::Help,
                target: NeedTarget::NeedsHelp(a.key.as_str().into()),
            });
        }
    }
    for a in &w.addons {
        if let Kind::NeedsDecision(r) = a.kind {
            let open = !matches!(
                w.decisions.get(a.key.as_str()),
                Some(DecisionPhase::Done(_))
            );
            if !open {
                continue;
            }
            v.push(NeedItem {
                title: format!("{}: {}", a.title, decision_title(r)),
                reason: decision_reason(w, a, r),
                affects: if w.apply_scope == FakeApplyScope::AllAppliesBlocked {
                    format!("{} only (your choice is kept, but no updates are applied until the interrupted update is resolved)", a.title)
                } else {
                    format!("{} only", a.title)
                },
                button: "Choose a copy",
                tone: Tone::Help,
                target: NeedTarget::Decision(a.key.as_str().into()),
            });
        }
    }
    for (i, p) in w.peers.iter().enumerate() {
        if matches!(
            p.state,
            MemberSummary::Unusable | MemberSummary::Regressed | MemberSummary::Invalid
        ) {
            v.push(NeedItem {
                title: format!("{}'s latest report could not be used", p.name),
                reason: peer_problem(p.state, &p.name),
                affects: "What this computer confirmed is not counted".into(),
                button: "See details",
                tone: Tone::Help,
                target: NeedTarget::Computer(i),
            });
        }
    }
    v
}

fn decision_title(r: DecisionReason) -> &'static str {
    match r {
        DecisionReason::Ambiguous => "can't tell which copy is newer",
        DecisionReason::PartialCandidate => "one copy is missing folders",
        DecisionReason::NoCompatibleToc => "a copy has no readable add-on file list",
        DecisionReason::DifferentTocMapping => "the copies read their file lists differently",
        DecisionReason::UnvalidatedTocMapping => "a copy has client-specific files",
    }
}

pub fn decision_reason(_w: &World, a: &Addon, r: DecisionReason) -> String {
    match r {
        DecisionReason::Ambiguous => format!(
            "WoW Sync can't tell which copy is newer. Their version labels can't be compared safely ({} and {}).",
            a.candidates.first().map(|c| c.version.as_str()).unwrap_or("?"),
            a.candidates.get(1).map(|c| c.version.as_str()).unwrap_or("?")
        ),
        DecisionReason::PartialCandidate => {
            "Laptop's copy is missing folders that other copies have: BagnonOptions.".into()
        }
        DecisionReason::NoCompatibleToc => {
            "A copy of this add-on has no readable .toc file for this computer's WoW client, so WoW Sync can't tell which files to use."
                .into()
        }
        DecisionReason::DifferentTocMapping => {
            "The computers that hold this add-on read its .toc files by a different rule than this computer, so WoW Sync can't tell which files apply here."
                .into()
        }
        DecisionReason::UnvalidatedTocMapping => {
            "This add-on has client-specific .toc files that WoW Sync does not handle yet.".into()
        }
    }
}

fn peer_problem(s: MemberSummary, name: &str) -> String {
    match s {
        MemberSummary::Regressed => format!(
            "{name}'s report went backwards (older than one we already had) and was ignored. This can happen when a computer is restored from a backup. Open WoW Sync on {name}."
        ),
        MemberSummary::Invalid => format!("{name}'s enrollment record is not valid, so its reports are ignored."),
        _ => format!("{name}'s latest report could not be used."),
    }
}

// ---------------------------------------------------------------------------------------------
// Computers

#[derive(Clone, Debug)]
pub struct PeerView {
    pub name: String,
    pub tone: Tone,
    pub icon: Icon,
    pub status: String,
    pub confirmed: String,
    pub received: String,
    pub stale: bool,
    pub a11y: String,
}

pub fn peer_view(w: &World, i: usize) -> PeerView {
    let p = &w.peers[i];
    let total = w.synced_total();
    let confirmed = w.confirmed_by(i);
    let age = p.report_secs_ago;
    let stale = matches!(p.state, MemberSummary::Usable)
        && age.map(|a| a >= STALE_DAYS * 86400).unwrap_or(false);
    let received = match age {
        Some(a) => format!("Report received {}", rel(a)),
        None => "No report received".to_string(),
    };
    let (tone, icon, status, conf) = match p.state {
        MemberSummary::NoState => (
            Tone::Unknown,
            Icon::Dashed,
            "Has not shared anything yet".to_string(),
            "Joined, but has not shared anything yet.".to_string(),
        ),
        MemberSummary::Usable if stale => (
            Tone::Unknown,
            Icon::Clock,
            format!(
                "Not heard from for {}",
                rel(age.unwrap()).trim_end_matches(" ago")
            ),
            format!(
                "Last confirmed {confirmed} of {total} add-ons, {}. What it has now is not known.",
                rel(age.unwrap())
            ),
        ),
        MemberSummary::Usable => {
            let all = total > 0 && confirmed == total;
            (
                if all { Tone::Ok } else { Tone::Unknown },
                if all { Icon::Check } else { Icon::Dashed },
                "Report received".to_string(),
                format!(
                    "Confirmed {confirmed} of {total} current add-ons, as of {}.",
                    rel(age.unwrap_or(0))
                ),
            )
        }
        MemberSummary::Unusable => (
            Tone::Waiting,
            Icon::Hourglass,
            "Its latest report could not be used".to_string(),
            peer_problem(p.state, &p.name),
        ),
        MemberSummary::Regressed => (
            Tone::Help,
            Icon::Alert,
            "Its report went backwards and was ignored".to_string(),
            peer_problem(p.state, &p.name),
        ),
        MemberSummary::Invalid => (
            Tone::Error,
            Icon::Error,
            "Its enrollment record is not valid".to_string(),
            peer_problem(p.state, &p.name),
        ),
    };
    let a11y = format!(
        "{}, {}, {}{}",
        p.name,
        status.to_lowercase(),
        conf.to_lowercase(),
        age.map(|a| format!(" (received {})", absolute(a)))
            .unwrap_or_default()
    );
    PeerView {
        name: p.name.clone(),
        tone,
        icon,
        status,
        confirmed: conf,
        received,
        stale,
        a11y,
    }
}

pub fn unconfirmed_addons(w: &World, peer: usize) -> Vec<(String, String)> {
    w.addons
        .iter()
        .filter(|a| {
            a.kind != Kind::NotSynced && !matches!(&a.kind, Kind::Confirmed(v) if v.contains(&peer))
        })
        .map(|a| {
            (
                a.key.as_str().to_string(),
                format!(
                    "{} {}",
                    a.title,
                    a.target_version.as_ref().unwrap_or(&a.version)
                ),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Add-on detail ("Why this version")

#[derive(Clone, Debug)]
pub struct Story {
    pub title: String,
    pub label: Label,
    pub versions: String,
    pub what: String,
    pub why: Vec<String>,
    pub next: String,
    pub next_button: Option<(&'static str, Detail)>,
    pub computers: Vec<(String, String, Tone, Icon)>,
    pub folders: String,
    pub grouped: Option<String>,
    pub recovery: String,
    pub details: Vec<String>,
}

pub fn addon_story(w: &World, key: &str) -> Option<Story> {
    let a = w.addon(key)?;
    let st = status_of(w, key)?.clone();
    let label = status_label(w, &st);
    let src = a.source.clone().unwrap_or_else(|| "Laptop".into());
    let to = a
        .target_version
        .clone()
        .unwrap_or_else(|| a.version.clone());
    let versions = match &st {
        PackageStatus::Available | PackageStatus::Blocked(_) | PackageStatus::Interrupted => {
            format!(
                "On this computer: {} · Group's chosen version: {}",
                a.version, to
            )
        }
        PackageStatus::NeedsDecision(_) => {
            format!(
                "On this computer: {} · Group's chosen version: not decided yet",
                a.version
            )
        }
        PackageStatus::AppliedHere => {
            format!("On this computer: {to} · Group's chosen version: {to}")
        }
        _ => format!(
            "On this computer: {} · Group's chosen version: {}",
            a.version, a.version
        ),
    };
    let ago = a
        .updated_secs_ago
        .map(rel)
        .unwrap_or_else(|| "earlier".into());
    let what = match &st {
        PackageStatus::AppliedHere => format!(
            "Updated from {} to {}, copied from {src}. No other computer has confirmed it yet.",
            a.version_before(),
            to
        ),
        PackageStatus::ConfirmedBy(_) => format!(
            "Same as the group's version. Confirmed by {}.",
            confirmed_names(w, &st).join(", ")
        ),
        PackageStatus::InSync => "Same as the group's chosen version.".into(),
        PackageStatus::Available => format!("Version {to} from {src} will be applied on the next sync."),
        PackageStatus::Blocked(PlanBlock::GameRunning) => "WoW is running; nothing was changed.".into(),
        PackageStatus::Blocked(_) => "Add-on files are still changing; waiting.".into(),
        PackageStatus::Unavailable => format!("{src}'s copy has not finished downloading through Dropbox yet."),
        PackageStatus::Interrupted => {
            "The update was interrupted because WoW started. It will finish after WoW closes.".into()
        }
        PackageStatus::NeedsDecision(r) => decision_reason(w, a, *r),
        PackageStatus::NeedsHelp(_) => {
            "An update stopped in an unexpected state. The old and new files are both kept.".into()
        }
        PackageStatus::NotSynced => {
            "This folder uses a game-version-specific setup that is not supported yet, so it is never copied.".into()
        }
        PackageStatus::Removing => "The group removed this add-on. It is moved to recovery here, not deleted for good.".into(),
        PackageStatus::Held => "Waiting for the group's removal history before changing this add-on.".into(),
    };
    let why = match &st {
        PackageStatus::AppliedHere
        | PackageStatus::Available
        | PackageStatus::Blocked(_)
        | PackageStatus::Interrupted => vec![
            format!(
                "{src} has {to}. This computer had {}. {to} is newer by its version number.",
                a.version_before()
            ),
            "File dates and computer clocks are never used to decide.".into(),
        ],
        PackageStatus::ConfirmedBy(_) | PackageStatus::InSync => {
            vec!["Every computer that reported has the same files.".into()]
        }
        PackageStatus::NeedsDecision(_) => a
            .candidates
            .iter()
            .map(|c| {
                format!(
                    "{} ({}): {}",
                    c.version,
                    c.holders.join(" and "),
                    c.differences
                )
            })
            .collect(),
        PackageStatus::Unavailable => {
            vec!["The files are not on this computer yet, so nothing can be compared.".into()]
        }
        _ => vec!["Nothing was compared yet.".into()],
    };
    let (next, next_button) = match &st {
        PackageStatus::NeedsDecision(_) => (
            "Choose which copy to keep. Only this add-on is affected; other add-ons keep syncing."
                .to_string(),
            Some(("Choose a copy", Detail::Decision(key.to_string()))),
        ),
        PackageStatus::NeedsHelp(_) => (
            "See what happened and choose how to continue.".to_string(),
            Some(("See what happened", Detail::NeedsHelp(key.to_string()))),
        ),
        PackageStatus::Blocked(PlanBlock::GameRunning) | PackageStatus::Interrupted => {
            ("Close WoW to let the update finish.".to_string(), None)
        }
        PackageStatus::Unavailable => (
            "Wait for Dropbox to finish downloading, or make the folder available offline."
                .to_string(),
            None,
        ),
        PackageStatus::Available if w.apply_scope == FakeApplyScope::AllAppliesBlocked => (
            "Not applied: an earlier update needs help. Resolve that first.".to_string(),
            None,
        ),
        PackageStatus::Available => (
            "Nothing to do. It will be applied on the next sync.".to_string(),
            None,
        ),
        _ => ("Nothing to do.".to_string(), None),
    };
    let mut computers = Vec::new();
    computers.push((
        w.computer.clone(),
        match &st {
            PackageStatus::AppliedHere => format!("Updated here {ago}"),
            PackageStatus::Available => "Update ready, not applied yet".into(),
            PackageStatus::NeedsDecision(_) => format!("Has {}", a.version),
            _ => format!("Has {}", a.version),
        },
        Tone::Ok,
        Icon::Check,
    ));
    for (i, p) in w.peers.iter().enumerate() {
        let pv = peer_view(w, i);
        let confirmed = matches!(&a.kind, Kind::Confirmed(v) if v.contains(&i));
        let holds = a.candidates.iter().any(|c| c.holders.contains(&p.name));
        let (line, tone, icon) = if confirmed {
            (
                format!(
                    "Confirmed, report received {}",
                    rel(p.report_secs_ago.unwrap_or(0))
                ),
                Tone::Ok,
                Icon::Check,
            )
        } else if holds {
            (
                format!(
                    "Holds a copy; report received {}",
                    rel(p.report_secs_ago.unwrap_or(0))
                ),
                Tone::Unknown,
                Icon::Dashed,
            )
        } else if pv.stale {
            (
                format!("Not confirmed yet; {}", pv.status.to_lowercase()),
                Tone::Unknown,
                Icon::Clock,
            )
        } else {
            ("Not confirmed yet".into(), Tone::Unknown, Icon::Dashed)
        };
        computers.push((p.name.clone(), line, tone, icon));
    }
    let grouped =
        (a.folders.len() > 1).then(|| "Grouped because they share a name prefix.".to_string());
    let recovery = match &st {
        PackageStatus::AppliedHere => "2 earlier copies kept (normal).".to_string(),
        PackageStatus::NeedsDecision(_) => "1 extra copy kept until the decision is made.".into(),
        _ => "No earlier copies are needed yet.".into(),
    };
    let details = vec![
        format!("Folder key: {}", a.key.as_str()),
        "Package hash: 0202020202020202… (fake)".to_string(),
        "Computer IDs: 00000000000000000000000000000001 (fake)".to_string(),
    ];
    Some(Story {
        title: a.title.clone(),
        label,
        versions,
        what,
        why,
        next,
        next_button,
        computers,
        folders: a.folders.join(", "),
        grouped,
        recovery,
        details,
    })
}

impl Addon {
    fn version_before(&self) -> String {
        match (&self.kind, &self.target_version) {
            (Kind::AppliedHere, Some(t)) if *t != self.version => self.version.clone(),
            (Kind::AppliedHere, Some(_)) => previous_version(&self.version),
            (_, _) => self.version.clone(),
        }
    }
}

fn previous_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(str::to_string).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u32>() {
            *last = n.saturating_sub(1).to_string();
        }
    }
    parts.join(".")
}

// ---------------------------------------------------------------------------------------------
// Tray

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayShape {
    SetupDot,
    Check,
    Arrows,
    Hourglass,
    Exclamation,
    Cross,
    Bars,
}

pub fn tray_shape(w: &World) -> TrayShape {
    if !w.enrolled {
        return TrayShape::SetupDot;
    }
    match headline(w).tone {
        Tone::Ok => TrayShape::Check,
        Tone::Working => TrayShape::Arrows,
        Tone::Waiting => TrayShape::Hourglass,
        Tone::Help | Tone::Unknown => TrayShape::Exclamation,
        Tone::Error => TrayShape::Cross,
        Tone::Paused => TrayShape::Bars,
    }
}

/// 32x32 RGBA: a base mark plus a status overlay in the lower-right quadrant. The shape (not
/// the colour) carries the meaning, with a contrasting outline for light and dark taskbars.
pub fn tray_rgba(shape: TrayShape) -> Vec<u8> {
    const N: usize = 32;
    let mut px = vec![0u8; N * N * 4];
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if (0..N as i32).contains(&x) && (0..N as i32).contains(&y) {
            let i = (y as usize * N + x as usize) * 4;
            px[i..i + 4].copy_from_slice(&c);
        }
    };
    let blue = [0x2b, 0x7f, 0xff, 0xff];
    let white = [0xff, 0xff, 0xff, 0xff];
    let dark = [0x10, 0x14, 0x1c, 0xff];
    for y in 0..N as i32 {
        for x in 0..N as i32 {
            let (dx, dy) = ((x - 15).abs().max(0), (y - 15).abs());
            if dx <= 14 && dy <= 14 && !(dx >= 13 && dy >= 13) {
                put(x, y, blue);
            }
        }
    }
    // A stylised "W".
    let w_pts = [(5, 7), (9, 21), (13, 11), (17, 21), (21, 7)];
    for pair in w_pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        for t in 0..=40 {
            let (x, y) = (x0 + (x1 - x0) * t / 40, y0 + (y1 - y0) * t / 40);
            put(x, y, white);
            put(x + 1, y, white);
        }
    }
    let (cx, cy, r) = (23.0f32, 23.0f32, 8.5f32);
    for y in 0..N as i32 {
        for x in 0..N as i32 {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d <= r + 1.0 {
                put(x, y, dark);
            }
            if d <= r {
                put(x, y, white);
            }
        }
    }
    let ink = match shape {
        TrayShape::SetupDot => [0x2b, 0x7f, 0xff, 0xff],
        _ => [0x10, 0x14, 0x1c, 0xff],
    };
    macro_rules! line {
        ($x0:expr, $y0:expr, $x1:expr, $y1:expr) => {{
            let (x0, y0, x1, y1): (i32, i32, i32, i32) = ($x0, $y0, $x1, $y1);
            for t in 0..=30 {
                let (x, y) = (x0 + (x1 - x0) * t / 30, y0 + (y1 - y0) * t / 30);
                for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
                    put(x + ox, y + oy, ink);
                }
            }
        }};
    }
    match shape {
        TrayShape::SetupDot => {
            for y in 21..=26 {
                for x in 21..=26 {
                    put(x, y, ink);
                }
            }
        }
        TrayShape::Check => {
            line!(19, 23, 22, 26);
            line!(22, 26, 28, 19);
        }
        TrayShape::Arrows => {
            for y in 0..N as i32 {
                for x in 0..N as i32 {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    let on_ring = (4.0..=6.0).contains(&d);
                    let gap = x > 24 && y < 22;
                    if on_ring && !gap {
                        put(x, y, ink);
                    }
                }
            }
            line!(25, 15, 28, 20);
            line!(28, 20, 22, 20);
        }
        TrayShape::Hourglass => {
            line!(19, 18, 28, 18);
            line!(19, 29, 28, 29);
            line!(20, 18, 24, 23);
            line!(27, 18, 23, 23);
            line!(20, 29, 24, 24);
            line!(27, 29, 23, 24);
        }
        TrayShape::Exclamation => {
            for y in 17..=24 {
                for x in 22..=24 {
                    put(x, y, ink);
                }
            }
            for y in 26..=28 {
                for x in 22..=24 {
                    put(x, y, ink);
                }
            }
        }
        TrayShape::Cross => {
            line!(19, 19, 28, 28);
            line!(28, 19, 19, 28);
        }
        TrayShape::Bars => {
            for y in 18..=28 {
                for x in [20, 21, 26, 27] {
                    put(x, y, ink);
                }
            }
        }
    }
    px
}

// ---------------------------------------------------------------------------------------------
// Misc wording

pub fn sheet_quit_text(w: &World) -> Option<String> {
    if matches!(w.busy, Some(Busy::Syncing { .. })) {
        let name = w
            .addons
            .iter()
            .find(|a| a.kind == Kind::Available)
            .map(|a| a.title.clone())
            .unwrap_or_else(|| "an add-on".into());
        Some(format!(
            "WoW Sync is updating {name}. Quitting waits for the current step to finish safely."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{ScenarioId, ALL_SCENARIOS};

    #[test]
    fn every_decision_reason_has_distinct_build_free_wording() {
        let w = World::new(ScenarioId::S7);
        let a = w.addons.first().expect("S7 has add-ons");
        let all = [
            DecisionReason::Ambiguous,
            DecisionReason::PartialCandidate,
            DecisionReason::NoCompatibleToc,
            DecisionReason::DifferentTocMapping,
            DecisionReason::UnvalidatedTocMapping,
        ];
        let mut seen = std::collections::HashSet::new();
        for r in all {
            let text = format!("{} {}", decision_title(r), decision_reason(&w, a, r));
            let low = text.to_lowercase();
            for bad in [
                "build",
                "game version",
                "game-version",
                "interface",
                "supports",
            ] {
                assert!(!low.contains(bad), "{r:?} mentions {bad}: {text}");
            }
            assert!(seen.insert(text), "{r:?} wording is not distinct");
        }
        for a in &w.addons {
            for c in &a.candidates {
                let low = c.differences.to_lowercase();
                assert!(!low.contains("build") && !low.contains("game version"));
            }
        }
    }
    #[test]
    fn every_scenario_builds_with_expected_headline() {
        let want = [
            (ScenarioId::S1, "Set up this computer"),
            (ScenarioId::S3, "This computer is up to date"),
            (ScenarioId::S4, "Syncing: applying 2 of 5 updates"),
            (ScenarioId::S5, "Waiting: WoW is running"),
            (ScenarioId::S6, "Waiting for Dropbox to finish downloading"),
            (ScenarioId::S7, "Waiting for your decision on 3 add-ons"),
            (ScenarioId::S8, "Can't read the shared folder"),
            (ScenarioId::S10, "Paused"),
            (ScenarioId::S16, "An update needs your help"),
        ];
        for id in ALL_SCENARIOS {
            let w = World::new(id);
            assert!(!headline(&w).text.is_empty(), "{}", id.code());
            assert!(!headline_a11y(&w).is_empty(), "{}", id.code());
        }
        for (id, text) in want {
            assert_eq!(headline(&World::new(id)).text, text, "{}", id.code());
        }
    }

    #[test]
    fn forever_is_never_labelled_classic() {
        for id in ALL_SCENARIOS {
            let w = World::new(id);
            assert!(!window_title(&headline(&w).text).contains("Classic"));
            assert!(!context_line(&w).contains("Classic"), "{}", id.code());
        }
    }

    #[test]
    fn decisions_are_isolated_to_the_affected_addons() {
        let w = World::new(ScenarioId::S7);
        assert_eq!(decision_names(&w).len(), 3);
        assert!(counts_sentence(&w).contains("3 needs your decision"));
        assert!(banners(&w).0.len() <= 2);
    }

    #[test]
    fn tray_shape_differs_by_state_not_only_colour() {
        let shapes = [
            TrayShape::SetupDot,
            TrayShape::Check,
            TrayShape::Arrows,
            TrayShape::Hourglass,
            TrayShape::Exclamation,
            TrayShape::Cross,
            TrayShape::Bars,
        ];
        for (i, a) in shapes.iter().enumerate() {
            for b in &shapes[i + 1..] {
                let (pa, pb) = (tray_rgba(*a), tray_rgba(*b));
                assert_eq!(pa.len(), 32 * 32 * 4);
                assert_ne!(pa, pb, "{a:?} vs {b:?}");
            }
        }
        assert_eq!(tray_shape(&World::new(ScenarioId::S10)), TrayShape::Bars);
        assert_eq!(tray_shape(&World::new(ScenarioId::S8)), TrayShape::Cross);
        assert!(tray_tooltip(&World::new(ScenarioId::S3)).starts_with("Prototype: WoW Sync"));
    }
}
