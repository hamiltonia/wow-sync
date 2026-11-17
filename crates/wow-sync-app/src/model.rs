//! Pure view-model: wording, tones, counts, filters and accessible names derived from a
//! `World`. No GPUI types here, so it runs on every platform and is unit tested.

use crate::theme::Tone;
use crate::world::{absolute, rel, Addon, ApplyScope, Busy, Kind, World};
use wow_sync_core::plan::PlanBlock;
use wow_sync_core::status::{Headline, MemberSummary, PackageStatus};
use wow_sync_core::target::DecisionReason;

/// Peers silent for at least this many days read "Not heard from" (Q-27 is open; provisional).
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
}

pub const NAV: [Page; 8] = [
    Page::Overview,
    Page::Computers,
    Page::Addons,
    Page::Needs,
    Page::Activity,
    Page::SettingsSync,
    Page::AppSettings,
    Page::About,
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
        w.identity_label, w.group, w.computer
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
    format!("WoW Sync: {}", headline(w).text)
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
            } else if w.apply_scope == ApplyScope::AllAppliesBlocked {
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

pub fn confirmed_names(w: &World, a: &Addon) -> Vec<String> {
    match &a.kind {
        Kind::Confirmed(ix) => ix
            .iter()
            .filter_map(|i| w.peers.get(*i).map(|p| p.name.clone()))
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
        let names = confirmed_names(w, a);
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
            PackageStatus::Unavailable | PackageStatus::Blocked(_) | PackageStatus::Interrupted => {
                c.waiting += 1
            }
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
    if c.needs_help > 0 && w.apply_scope == ApplyScope::AllAppliesBlocked {
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
        let tail = if n == 0 {
            "No updates are waiting.".to_string()
        } else {
            format!(
                "{} will be applied after you close WoW.",
                plural(n, "update", "updates")
            )
        };
        all.push(Banner {
            text: format!(
                "WoW is running. WoW Sync never changes game files while the game runs. {tail}"
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
            v.push(NeedItem {
                title: format!("{}: {}", a.title, decision_title(r)),
                reason: decision_reason(w, a, r),
                affects: if w.apply_scope == ApplyScope::AllAppliesBlocked {
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
        _ => "the copies are for different game builds",
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
            let who: Vec<&str> = a
                .candidates
                .iter()
                .filter(|c| !c.differences.is_empty())
                .map(|c| c.differences.as_str())
                .collect();
            if who.is_empty() {
                "One copy is missing folders that other copies have.".into()
            } else {
                format!("One copy is missing folders that other copies have. {}", who.join(" "))
            }
        }
        DecisionReason::DifferentTocMapping | DecisionReason::NoCompatibleToc => {
            "This copy came from a computer that reads add-on files by a different rule, or does not say it supports this game version.".into()
        }
        _ => "This add-on has game-version-specific files that WoW Sync does not handle yet.".into(),
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
    let src = a
        .source
        .clone()
        .unwrap_or_else(|| "another computer".into());
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
    let what = match &st {
        PackageStatus::AppliedHere => format!(
            "Updated to {to}, copied from {src}. No other computer has confirmed it yet."
        ),
        PackageStatus::ConfirmedBy(_) => format!(
            "Same as the group's version. Confirmed by {}.",
            confirmed_names(w, a).join(", ")
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
    };
    let why = match &st {
        PackageStatus::AppliedHere
        | PackageStatus::Available
        | PackageStatus::Blocked(_)
        | PackageStatus::Interrupted => {
            let mut v = if a.why.is_empty() {
                vec![format!("{src} has {to}. This computer has {}.", a.version)]
            } else {
                vec![a.why.clone()]
            };
            v.push("File dates and computer clocks are never used to decide.".into());
            v
        }
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
        PackageStatus::Available if w.apply_scope == ApplyScope::AllAppliesBlocked => (
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
            PackageStatus::AppliedHere => "Updated here".to_string(),
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
        PackageStatus::AppliedHere => {
            "Earlier copies are kept so this update can be undone.".to_string()
        }
        PackageStatus::NeedsDecision(_) => {
            "The competing copies are kept until the decision is made.".into()
        }
        _ => "No earlier copies are needed yet.".into(),
    };
    let details = vec![
        format!("Folder key: {}", a.key.as_str()),
        format!("Group ID: {}", w.group_id),
        format!("Computer ID: {}", w.computer_id),
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
        Some(
            "WoW Sync is applying changes right now. Quitting waits for the current step to finish safely."
                .into(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enrolled() -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.computer = "Desk".into();
        w.group = "My add-ons".into();
        w.identity_label = "WoW Retail".into();
        w.view.headline = Headline::Ok;
        w
    }

    #[test]
    fn headline_follows_the_runtime_state() {
        assert_eq!(
            headline(&World::empty("0.0.0")).text,
            "Set up this computer"
        );
        let mut w = enrolled();
        assert_eq!(headline(&w).text, "This computer is up to date");
        w.view.headline = Headline::Error;
        assert_eq!(headline(&w).text, "Can't read the shared folder");
        w.paused_since = Some("12:00".into());
        assert_eq!(headline(&w).text, "Paused");
    }

    #[test]
    fn nothing_invented_when_there_is_no_data() {
        let w = enrolled();
        assert!(addon_rows(&w, Filter::All).is_empty());
        assert!(!context_line(&w).contains("Classic"));
        assert!(banners(&w).0.is_empty());
        assert!(sheet_quit_text(&w).is_none());
    }

    #[test]
    fn quit_sheet_only_while_applying() {
        let mut w = enrolled();
        w.busy = Some(Busy::Syncing { done: 0, total: 0 });
        assert!(sheet_quit_text(&w).is_some());
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
        let mut w = enrolled();
        assert_eq!(tray_shape(&w), TrayShape::Check);
        assert!(tray_tooltip(&w).starts_with("WoW Sync: "));
        w.view.headline = Headline::Error;
        assert_eq!(tray_shape(&w), TrayShape::Cross);
    }
}
