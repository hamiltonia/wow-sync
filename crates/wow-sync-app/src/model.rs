//! Pure view-model: wording, tones, counts, filters and accessible names derived from a
//! `World`. No GPUI types here, so it runs on every platform and is unit tested.

use crate::theme::Tone;
use crate::world::{absolute, rel, Addon, Busy, Candidate, Kind, World};
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
    // Core's `Headline` is used as is (RV-5), then three app-side promotions (ops.md §1.6-§1.7):
    // a retired or held self, and "Review needed". Pause is not part of core's status, so it only
    // replaces the headlines that say nothing is wrong; errors and needed help always show, and
    // none of the promotions hides them.
    let core_blocks_pause = matches!(w.view.headline, Headline::Error | Headline::NeedsHelp);
    let standing = crate::computers_ui::standing_headline(&w.features.standing);
    let (text, tone) = if !w.enrolled {
        ("Set up this computer".to_string(), Tone::Unknown)
    } else if let Some(r) = w.features.data_at_risk.first() {
        (r.text.clone(), Tone::Error)
    } else if blocked_by_test_mode(w) {
        ("Paused by test mode".to_string(), Tone::Help)
    } else if let (false, Some((h, _))) = (core_blocks_pause, &standing) {
        let held = matches!(w.features.standing, crate::features::Standing::Held);
        (h.clone(), if held { Tone::Waiting } else { Tone::Help })
    } else if w.features.review_needed && !core_blocks_pause {
        ("Review needed".to_string(), Tone::Help)
    } else if w.busy == Some(Busy::Pausing) {
        (
            "Pausing, finishing the current step".to_string(),
            Tone::Paused,
        )
    } else if w.pause_damaged && !core_blocks_pause {
        (
            "Syncing is held: the pause setting can't be read".to_string(),
            Tone::Help,
        )
    } else if w.is_paused() && !core_blocks_pause {
        ("Paused".to_string(), Tone::Paused)
    } else {
        match w.view.headline {
            Headline::Ok => ("This computer is up to date".to_string(), Tone::Ok),
            Headline::Working => (
                match &w.busy {
                    Some(Busy::Syncing { done, total }) if *total > 0 => {
                        format!("Syncing: applying {done} of {total} updates")
                    }
                    _ => match removing(w) {
                        0 => "Syncing".to_string(),
                        n => format!("Removing {}", plural(n, "add-on", "add-ons")),
                    },
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

fn removing(w: &World) -> usize {
    w.view
        .packages
        .iter()
        .filter(|(_, s, _)| *s == PackageStatus::Removing)
        .count()
}

/// The line under the headline when the headline alone is not enough (ops.md §1.6-§1.7).
pub fn headline_sub(w: &World) -> Option<String> {
    if !w.enrolled
        || blocked_by_test_mode(w)
        || !w.features.data_at_risk.is_empty()
        || matches!(w.view.headline, Headline::Error | Headline::NeedsHelp)
    {
        return None;
    }
    if let Some((_, sub)) = crate::computers_ui::standing_headline(&w.features.standing) {
        return Some(sub);
    }
    if w.features.review_needed {
        return Some("Confirm this computer's WoW folders so syncing can continue.".to_string());
    }
    w.features
        .restores
        .iter()
        .find(|p| matches!(p.state, wow_sync_runtime::RestoreState::Waiting { .. }))
        .map(|p| crate::removed_ui::plain(&p.text))
}

/// Seconds an unavailable add-on may wait before the headline stops promising it will arrive.
const UNAVAILABLE_GRACE_SECS: u64 = 30 * 60;

fn unavailable_sentence(w: &World) -> Option<String> {
    let un: Vec<&crate::world::Addon> = w
        .addons
        .iter()
        .filter(|a| a.kind == crate::world::Kind::Unavailable)
        .collect();
    let first = un.first()?;
    let mut who: Vec<&str> = un
        .iter()
        .flat_map(|a| a.waiting_on.iter().map(String::as_str))
        .collect();
    who.sort_unstable();
    who.dedup();
    let overdue = un
        .iter()
        .any(|a| a.waiting_secs.is_some_and(|s| s >= UNAVAILABLE_GRACE_SECS));
    let what = if un.len() == 1 {
        first.title.clone()
    } else {
        format!("{} add-ons", un.len())
    };
    Some(match (who.as_slice(), overdue) {
        ([], false) => format!("Waiting for the shared files for {what} to arrive"),
        ([], true) => format!(
            "The shared files for {what} have not arrived. Open WoW Sync on the computer that shares them"
        ),
        (names, false) => format!("Waiting for {} to finish sharing {what}", names.join(", ")),
        (names, true) => format!(
            "{} has not finished sharing {what}. Open WoW Sync on it and let it sync",
            names.join(", ")
        ),
    })
}
fn waiting_sentence(w: &World) -> String {
    if w.view.blocks.contains(&PlanBlock::GameRunning) {
        return match removing(w) {
            0 => "Waiting: WoW is running".into(),
            n => format!(
                "Waiting: WoW is running. {} will be removed when it closes",
                plural(n, "add-on", "add-ons")
            ),
        };
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
    if let Some(s) = unavailable_sentence(w) {
        return s;
    }
    if w.view.blocks.contains(&PlanBlock::Unstable) {
        return "Waiting for add-on files to stop changing".into();
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
        PackageStatus::Unavailable => {
            label("Waiting for shared files", Tone::Waiting, Icon::Hourglass)
        }
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
    let c = counts(w);
    // Waiting add-ons (including interrupted ones) are retried on resume, so "0 updates" would
    // be untrue; their number is shown as a count of add-ons, not as a promise of success.
    Some(match (c.ready, c.waiting) {
        (n, 0) => format!(
            "{} will be applied when you resume",
            plural(n, "update", "updates")
        ),
        (0, k) => format!(
            "{} waiting will be tried again when you resume",
            plural(k, "add-on", "add-ons")
        ),
        (n, k) => format!(
            "{} will be applied and {} waiting will be tried again when you resume",
            plural(n, "update", "updates"),
            plural(k, "add-on", "add-ons")
        ),
    })
}

/// A candidate's version label, or "not available" when the copy has none.
pub fn version_label(c: &Candidate) -> &str {
    if c.version.trim().is_empty() {
        "not available"
    } else {
        &c.version
    }
}

pub fn game_state(w: &World) -> &'static str {
    if w.game_running {
        "Running"
    } else {
        "Not running"
    }
}

// ---------------------------------------------------------------------------------------------
// Banners

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerAction {
    None,
    Resume,
    TryAgain,
    Reconfirm,
    TurnOffTestMode,
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub text: String,
    pub tone: Tone,
    pub action: BannerAction,
    pub action_label: &'static str,
}

pub fn blocked_by_test_mode(w: &World) -> bool {
    w.test_mode && w.test_mode_blocked
}

pub const TEST_MODE_HELP: &str = "Test mode is on, and this installation is not marked as a test folder, so syncing is paused. Nothing was changed.";

pub fn banners(w: &World) -> (Vec<Banner>, usize) {
    let mut all = Vec::new();
    if blocked_by_test_mode(w) {
        all.push(Banner {
            text: TEST_MODE_HELP.into(),
            tone: Tone::Help,
            action: BannerAction::TurnOffTestMode,
            action_label: "Turn off test mode",
        });
    } else if let Some(e) = &w.provider_error {
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
    if w.pause_damaged {
        all.push(Banner {
            text: "The pause setting on this computer can't be read, so syncing is held. Choose Resume syncing to clear it."
                .into(),
            tone: Tone::Error,
            action: BannerAction::Resume,
            action_label: "Resume syncing",
        });
    } else if w.is_paused() {
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
    Risk,
    /// Test mode refuses this folder; the button turns it off.
    TestMode,
    /// A refused restore of a removed add-on; its Try again and Dismiss are on the Removed list.
    Restore,
}

/// What to do about each data-at-risk condition, one sentence.
fn risk_reason(k: wow_sync_runtime::RiskKind) -> &'static str {
    use wow_sync_runtime::RiskKind as R;
    match k {
        R::LocalStateUnreadable => {
            "Restart WoW Sync. If this stays, the saved state on this computer may be damaged."
        }
        R::RecoveryCopyFailed => {
            "Check that this computer's disk has free space and that WoW Sync's folder can be written to."
        }
        R::StorageFull => "Free up space on this computer's disk.",
    }
}

pub fn need_items(w: &World) -> Vec<NeedItem> {
    let mut v = Vec::new();
    for r in &w.features.data_at_risk {
        v.push(NeedItem {
            title: r.text.clone(),
            reason: risk_reason(r.kind).into(),
            affects: "Your add-ons and settings may not be recoverable".into(),
            button: "See what to do",
            tone: Tone::Error,
            target: NeedTarget::Risk,
        });
    }
    for p in &w.features.restores {
        if matches!(p.state, wow_sync_runtime::RestoreState::Unavailable { .. }) {
            v.push(NeedItem {
                title: crate::removed_ui::plain(&p.text),
                reason: "Choose Try again, or Dismiss to stop showing this.".into(),
                affects: String::new(),
                button: "Open Removed",
                tone: Tone::Error,
                target: NeedTarget::Restore,
            });
        }
    }
    if blocked_by_test_mode(w) {
        v.push(NeedItem {
            title: "Test mode is blocking this folder".into(),
            reason:
                "Test mode only syncs folders marked as test folders, and this one is not marked."
                    .into(),
            affects: "Nothing syncs until test mode is turned off".into(),
            button: "Turn off test mode",
            tone: Tone::Help,
            target: NeedTarget::TestMode,
        });
    } else if let Some(e) = &w.provider_error {
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
                affects: String::new(),
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
                affects: String::new(),
                button: "Choose a copy",
                tone: Tone::Help,
                target: NeedTarget::Decision(a.key.as_str().into()),
            });
        }
    }
    for item in &w.features.decisions {
        let wow_sync_core::decision::DecisionScope::Folders(ks) = &item.scope else {
            continue;
        };
        let Some(first) = ks.first() else { continue };
        if !matches!(
            item.reason,
            wow_sync_core::decision::DecisionKind::NewVersionAfterDecision
                | wow_sync_core::decision::DecisionKind::NewerAfterRollback
        ) || w
            .addons
            .iter()
            .any(|a| a.key.as_str() == first.as_str() && matches!(a.kind, Kind::NeedsDecision(_)))
        {
            continue;
        }
        let id = wow_sync_core::decision::DecisionId(item.scope.clone());
        let title = crate::decisions_ui::scope_title(w, &item.scope);
        let local = w.features.local_only.iter().find(|l| l.decision == id);
        v.push(NeedItem {
            title: format!("{title}: a newer version is not shared"),
            reason: match local {
                Some(lo) => crate::decisions_ui::local_only_sentence(lo),
                None => {
                    format!("A new version of {title} appeared after you made a choice about it.")
                }
            },
            affects: String::new(),
            button: "Choose",
            tone: Tone::Help,
            target: NeedTarget::Decision(first.as_str().into()),
        });
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

/// D-21 wording: one sentence per `DecisionReason`, never about game builds, game versions
/// or Interface numbers (those never hold an add-on back).
fn decision_title(r: DecisionReason) -> &'static str {
    match r {
        DecisionReason::Ambiguous => "can't tell which copy is newer",
        DecisionReason::PartialCandidate => "one copy is missing folders",
        DecisionReason::NoCompatibleToc => "a copy has no readable add-on file list",
        DecisionReason::DifferentTocMapping => "the copies read their file lists differently",
        DecisionReason::UnvalidatedTocMapping => "a copy has client-specific files",
        DecisionReason::ChoicesDiffer => "computers chose different versions",
        DecisionReason::RollbackTargetsDiffer => "computers went back to different versions",
    }
}

pub fn decision_reason(_w: &World, a: &Addon, r: DecisionReason) -> String {
    match r {
        DecisionReason::Ambiguous => format!(
            "WoW Sync can't tell which copy is newer. Their version labels can't be compared safely ({} and {}).",
            a.candidates.first().map(version_label).unwrap_or("?"),
            a.candidates.get(1).map(version_label).unwrap_or("?")
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
        DecisionReason::ChoicesDiffer => {
            "Different computers chose different versions of this add-on.".into()
        }
        DecisionReason::RollbackTargetsDiffer => {
            "Different computers went back to different versions of this add-on.".into()
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
    pub details: Vec<String>,
}

/// "Last check" line: the real time of the last check, then the scheduler's next check or
/// "unavailable" (never a guessed interval).
pub fn last_check_text(w: &World) -> String {
    match (w.last_check_secs, &w.next_check_at) {
        (None, _) => "Not yet".to_string(),
        (Some(s), Some(at)) => format!("{}; next check about {at}", rel(s)),
        (Some(s), None) => format!("{}; next check unavailable", rel(s)),
    }
}

/// Core's rendered explanation as a sentence: first letter capitalised, one full stop.
fn as_sentence(s: &str) -> String {
    let s = s.trim().trim_end_matches('.');
    let mut c = s.chars();
    match c.next() {
        Some(f) => format!("{}{}.", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

pub fn addon_story(w: &World, key: &str) -> Option<Story> {
    let a = w.addon(key)?;
    let st = status_of(w, key)?.clone();
    let label = status_label(w, &st);
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
    // What happened and why come from core's `Explanation` (rendered by the runtime); the app
    // adds no sentences of its own (RV-1). Decisions use the D-21 wording per reason.
    let what = match &st {
        PackageStatus::NeedsDecision(r) => decision_reason(w, a, *r),
        _ => as_sentence(&a.why),
    };
    let why = match &st {
        PackageStatus::NeedsDecision(_) => a
            .candidates
            .iter()
            .map(|c| {
                format!(
                    "{} ({}): {}",
                    version_label(c),
                    c.holders.join(" and "),
                    c.differences
                )
            })
            .collect(),
        _ if !a.awaiting.is_empty() => {
            vec![format!(
                "Not yet reported as applied by {}.",
                a.awaiting.join(", ")
            )]
        }
        _ => Vec::new(),
    };
    let next = a.next.sentence().to_string();
    let next_button = match &st {
        PackageStatus::NeedsDecision(_) => {
            Some(("Choose a copy", Detail::Decision(key.to_string())))
        }
        PackageStatus::NeedsHelp(_) => {
            Some(("See what happened", Detail::NeedsHelp(key.to_string())))
        }
        _ => None,
    };
    let mut computers = Vec::new();
    computers.push((
        w.computer.clone(),
        match &st {
            PackageStatus::AppliedHere => "Updated here".to_string(),
            PackageStatus::Available => "Update ready, not applied yet".into(),
            _ => format!("Has {}", a.version),
        },
        label.tone,
        label.icon,
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
        (a.folders.len() > 1).then(|| "These folders are treated as one add-on.".to_string());
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

/// 32x32 RGBA: the brand art (D-22) plus a status overlay in the lower-right quadrant. The shape (not
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
    let white = [0xff, 0xff, 0xff, 0xff];
    let dark = [0x10, 0x14, 0x1c, 0xff];
    let base = crate::brand::tray_base();
    for (i, p) in base.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = ((i % N) as i32, (i / N) as i32);
        put(x, y, *p);
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

/// The Quit sheet text for a Quit request. Setup draws no sheets, so a Quit during setup
/// never opens one: it goes straight to quit-when-safe, and the app exits once any write
/// step in progress has finished.
pub fn quit_sheet_for(w: &World, setup_mode: bool) -> Option<String> {
    if setup_mode {
        None
    } else {
        sheet_quit_text(w)
    }
}

/// The accessible name of a sidebar entry.
pub fn nav_name(p: Page, needs: usize, current: bool) -> String {
    let mut name = p.title().to_string();
    if p == Page::Needs && needs > 0 {
        name = format!("{name}, {needs} items");
    }
    if current {
        name = format!("{name}, current page");
    }
    name
}

/// Where focus goes when a confirmation panel opens: the safe choice, never the one that acts.
pub fn focus_on_open(key: &str, opened: bool) -> Option<&'static str> {
    (opened && key == "retire-open").then_some("retire-cancel")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_the_retire_panel_focuses_cancel_and_nothing_else_moves_focus() {
        assert_eq!(focus_on_open("retire-open", true), Some("retire-cancel"));
        assert_eq!(focus_on_open("retire-open", false), None);
        assert_eq!(focus_on_open("retire-ack", true), None);
    }

    #[test]
    fn settings_sync_is_a_normal_entry_and_not_labelled_coming_later() {
        assert_eq!(nav_name(Page::SettingsSync, 0, false), "Settings sync");
        assert_eq!(
            nav_name(Page::SettingsSync, 0, true),
            "Settings sync, current page"
        );
        assert_eq!(nav_name(Page::Needs, 3, false), "Needs attention, 3 items");
    }

    fn enrolled() -> World {
        let mut w = World::empty("0.0.0");
        w.enrolled = true;
        w.computer = "Desk".into();
        w.group = "My add-ons".into();
        w.identity_label = "WoW Retail".into();
        w.view.headline = Headline::Ok;
        w
    }

    fn blocked() -> World {
        let mut w = enrolled();
        w.test_mode = true;
        w.test_mode_blocked = true;
        w.provider_error = Some(wow_sync_runtime::ServiceError::TestModeBlocked.to_string());
        w
    }

    #[test]
    fn the_header_says_test_mode_paused_sync_and_keeps_the_names() {
        let w = blocked();
        let h = headline(&w);
        assert_eq!(h.text, "Paused by test mode");
        assert_eq!(h.tone, Tone::Help);
        assert!(context_line(&w).contains("My add-ons"));
        assert!(context_line(&w).contains("Desk"));
    }

    #[test]
    fn error_text_alone_never_means_test_mode() {
        let mut w = enrolled();
        w.test_mode = true;
        w.provider_error = Some(wow_sync_runtime::ServiceError::TestModeBlocked.to_string());
        assert!(!blocked_by_test_mode(&w));
        w.test_mode_blocked = true;
        w.test_mode = false;
        assert!(!blocked_by_test_mode(&w));
    }

    #[test]
    fn test_mode_blocking_offers_to_turn_it_off_and_never_blames_dropbox() {
        let w = blocked();
        assert!(blocked_by_test_mode(&w));
        let (b, _) = banners(&w);
        assert_eq!(b[0].action, BannerAction::TurnOffTestMode);
        assert_eq!(b[0].action_label, "Turn off test mode");
        let items = need_items(&w);
        assert_eq!(items[0].target, NeedTarget::TestMode);
        assert_eq!(items[0].button, "Turn off test mode");
        for t in [
            b[0].text.clone(),
            items[0].title.clone(),
            items[0].reason.clone(),
            items[0].affects.clone(),
        ] {
            assert!(
                !t.contains("Dropbox") && !t.contains("TestModeBlocked"),
                "{t}"
            );
        }
    }

    #[test]
    fn a_real_shared_folder_error_is_not_mistaken_for_test_mode() {
        let mut w = enrolled();
        w.provider_error = Some("The shared folder is missing".into());
        assert!(!blocked_by_test_mode(&w));
        w.test_mode = true;
        assert!(!blocked_by_test_mode(&w));
        assert_eq!(banners(&w).0[0].action, BannerAction::TryAgain);
        assert!(!blocked_by_test_mode(&enrolled()));
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
        assert_eq!(headline(&w).text, "Can't read the shared folder");
        w.view.headline = Headline::Ok;
        assert_eq!(headline(&w).text, "Paused");
        w.paused_since = None;
        w.pause_damaged = true;
        assert!(headline(&w).text.contains("pause setting can't be read"));
    }

    #[test]
    fn a_refused_restore_is_a_needs_item_and_a_waiting_one_is_only_a_status_line() {
        use wow_sync_runtime::{RestoreProgress, RestoreState, RestoreWait};
        let mut w = enrolled();
        let p = |state, text: &str| RestoreProgress {
            package: wow_sync_core::paths::validate_folder_name("Alpha").unwrap(),
            title: "alpha".into(),
            state,
            text: text.into(),
        };
        w.features.restores = vec![p(
            RestoreState::Waiting {
                reason: RestoreWait::WowRunning,
            },
            "Restoring alpha: waiting for World of Warcraft to close.",
        )];
        assert!(need_items(&w).is_empty(), "waiting is not a problem");
        assert_eq!(
            headline_sub(&w).as_deref(),
            Some("Restoring alpha: waiting for World of Warcraft to close.")
        );
        w.features.restores = vec![p(
            RestoreState::Unavailable { text: "x".into() },
            "|cffff0000alpha|r can't be restored: a file is missing. Nothing was changed.",
        )];
        let items = need_items(&w);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].title,
            "alpha can't be restored: a file is missing. Nothing was changed."
        );
        assert_eq!(items[0].tone, Tone::Error);
        assert_eq!(items[0].target, NeedTarget::Restore);
        assert_ne!(items[0].target, NeedTarget::Risk, "not data at risk");
        assert!(headline_sub(&w).is_none());
    }

    #[test]
    fn data_at_risk_shows_in_headline_tray_and_needs_attention_whatever_the_toggles_say() {
        use wow_sync_runtime::{DataAtRisk, RiskKind};
        let mut w = enrolled();
        w.features.toggles = wow_sync_runtime::AppToggles {
            update_check: false,
            notify_summaries: false,
            notify_needs_me: false,
        };
        assert!(need_items(&w).is_empty());
        w.features.data_at_risk = vec![DataAtRisk::new(RiskKind::StorageFull)];
        let h = headline(&w);
        assert_eq!(
            h.text,
            "WoW Sync can't keep recovery copies: the disk is full"
        );
        assert_eq!(h.tone, Tone::Error);
        assert_eq!(tray_shape(&w), TrayShape::Cross);
        assert!(tray_tooltip(&w).contains("the disk is full"));
        assert_eq!(headline_sub(&w), None);
        let items = need_items(&w);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, h.text);
        assert_eq!(items[0].tone, Tone::Error);
        assert_eq!(items[0].target, NeedTarget::Risk);
    }

    #[test]
    fn a_local_only_version_stays_in_needs_attention_until_answered() {
        use wow_sync_core::decision::{
            DecisionId, DecisionItem, DecisionKind, DecisionReach, DecisionScope, EvidenceToken,
        };
        use wow_sync_core::ids::Sha256;
        let key = wow_sync_core::paths::validate_folder_name("Details").unwrap();
        let scope = DecisionScope::Folders(vec![key.clone()]);
        let mut w = enrolled();
        w.features.decisions = vec![DecisionItem {
            scope: scope.clone(),
            reason: DecisionKind::NewVersionAfterDecision,
            reach: DecisionReach::Group,
            options: vec![],
            evidence: EvidenceToken(Sha256([1; 32])),
        }];
        w.features.local_only = vec![wow_sync_runtime::LocalOnlyVersion {
            package: key.clone(),
            title: "Details".into(),
            scope: vec![key],
            version_label: Some("9.1".into()),
            decision: DecisionId(scope),
        }];
        let items = need_items(&w);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].reason,
            "Version 9.1 of Details is installed only on this computer and is not shared with the group."
        );
        assert_eq!(items[0].target, NeedTarget::Decision("details".into()));
        w.features.decisions.clear();
        assert!(need_items(&w).is_empty(), "answered: it goes away");
    }

    #[test]
    fn every_decision_reason_has_its_own_build_free_wording() {
        use wow_sync_core::target::DecisionReason as R;
        let w = enrolled();
        let a = Addon {
            key: "k".into(),
            title: "Demo".into(),
            version: "1".into(),
            target_version: None,
            folders: vec![],
            kind: Kind::Available,
            candidates: vec![],
            why: String::new(),
            next: wow_sync_core::view::NextAction::CloseGame,
            awaiting: vec![],
            waiting_on: vec![],
            waiting_secs: None,
        };
        let all = [
            R::Ambiguous,
            R::NoCompatibleToc,
            R::DifferentTocMapping,
            R::UnvalidatedTocMapping,
            R::PartialCandidate,
        ];
        let mut seen = std::collections::HashSet::new();
        for r in all {
            let text = format!("{} {}", decision_title(r), decision_reason(&w, &a, r));
            let low = text.to_lowercase();
            for bad in ["build", "game version", "interface", "supports"] {
                assert!(!low.contains(bad), "{r:?} mentions {bad}: {text}");
            }
            assert!(seen.insert(text), "{r:?} wording is not distinct");
        }
    }

    #[test]
    fn an_unavailable_add_on_names_who_has_not_shared_and_stops_promising_downloads() {
        let mut w = enrolled();
        w.addons.push(Addon {
            key: "questie".into(),
            title: "Questie".into(),
            version: String::new(),
            target_version: None,
            folders: vec![],
            kind: Kind::Unavailable,
            candidates: vec![],
            why: String::new(),
            next: wow_sync_core::view::NextAction::CloseGame,
            awaiting: vec![],
            waiting_on: vec!["Lenovo".into()],
            waiting_secs: Some(60),
        });
        let t = unavailable_sentence(&w).unwrap();
        assert_eq!(t, "Waiting for Lenovo to finish sharing Questie");
        assert!(!t.contains("Dropbox"));
        w.addons[0].waiting_secs = Some(UNAVAILABLE_GRACE_SECS);
        let t = unavailable_sentence(&w).unwrap();
        assert!(
            t.starts_with("Lenovo has not finished sharing Questie"),
            "{t}"
        );
        w.addons[0].waiting_on.clear();
        assert!(unavailable_sentence(&w)
            .unwrap()
            .contains("have not arrived"));
        w.addons[0].kind = Kind::InSync;
        assert_eq!(unavailable_sentence(&w), None);
    }

    #[test]
    fn last_check_text_uses_the_scheduler_or_says_unavailable() {
        let mut w = enrolled();
        assert_eq!(last_check_text(&w), "Not yet");
        w.last_check_secs = Some(30);
        assert!(last_check_text(&w).ends_with("next check unavailable"));
        w.next_check_at = Some("14:07".into());
        let t = last_check_text(&w);
        assert!(t.ends_with("next check about 14:07"), "{t}");
        assert!(!t.contains("min"), "no literal interval: {t}");
    }

    #[test]
    fn review_needed_retired_held_and_removing_have_their_own_headlines() {
        use crate::features::Standing;
        let mut w = enrolled();
        w.features.review_needed = true;
        assert_eq!(headline(&w).text, "Review needed");
        assert_eq!(headline(&w).tone, Tone::Help);
        assert_eq!(
            headline_sub(&w).as_deref(),
            Some("Confirm this computer's WoW folders so syncing can continue.")
        );
        assert_eq!(tray_tooltip(&w), "WoW Sync: Review needed");
        w.paused_since = Some("12:00".into());
        assert_eq!(headline(&w).text, "Review needed", "pause never hides it");
        for hard in [Headline::NeedsHelp, Headline::Error] {
            w.view.headline = hard;
            assert_ne!(headline(&w).text, "Review needed");
            assert_eq!(headline_sub(&w), None);
        }

        let mut w = enrolled();
        w.features.standing = Standing::Retired {
            on: String::new(),
            by: "Den PC".into(),
        };
        assert_eq!(headline(&w).text, "This computer was retired");
        assert_eq!(headline(&w).tone, Tone::Help);
        assert!(headline_sub(&w).unwrap().starts_with("Retired by Den PC."));
        w.features.standing = Standing::Held;
        assert_eq!(headline(&w).text, "Waiting for group history");
        assert_eq!(headline(&w).tone, Tone::Waiting);

        let mut w = world_with(
            PackageStatus::Removing,
            wow_sync_core::explain::Explanation::InSync,
        );
        w.view.headline = Headline::Working;
        assert_eq!(headline(&w).text, "Removing 1 add-on");
        w.view.headline = Headline::Waiting;
        w.view.blocks.push(PlanBlock::GameRunning);
        assert_eq!(
            headline(&w).text,
            "Waiting: WoW is running. 1 add-on will be removed when it closes"
        );
    }

    fn world_with(status: PackageStatus, why: wow_sync_core::explain::Explanation) -> World {
        use crate::world::Inputs;
        use wow_sync_runtime::{PackageRow, StatusDetail, StatusSnapshot};
        let key = wow_sync_core::paths::validate_folder_name("Demo").unwrap();
        let mut view = crate::world::placeholder_view();
        view.packages.push((key, status.clone(), why.clone()));
        let row = PackageRow {
            key: "demo".into(),
            title: "Demo".into(),
            status: status.clone(),
            sentence: wow_sync_core::explain::render(&why),
            next: wow_sync_core::view::next_action(&status, &why, false),
            version_here: Some("1".into()),
            version_target: Some("2".into()),
            sources: vec![],
            folders: vec![],
            awaiting: vec![],
            waiting_on: vec![],
            confirmed_by: vec![],
            candidates: vec![],
        };
        let snap = StatusSnapshot {
            enrolled: true,
            detail: Some(StatusDetail {
                view,
                packages: vec![row],
                members: vec![],
                game: wow_sync_core::plan::Activity::NotRunning,
                group_id: String::new(),
                computer_id: String::new(),
                variant_label: String::new(),
                variant_folder: String::new(),
                app_version: String::new(),
                game_build: None,
            }),
            ..Default::default()
        };
        World::from_snapshot(Inputs {
            snap: &snap,
            paused_since_ms: None,
            pause_damaged: false,
            activity: &[],
            syncing: false,
            last_check_ms: None,
            next_check_ms: None,
            setup: None,
            app_version: "x",
        })
    }

    #[test]
    fn interrupted_text_follows_core_and_differs_by_cause() {
        use wow_sync_core::explain::{render, Explanation as E};
        let a = E::Interrupted {
            package: "Demo".into(),
        };
        let b = E::InterruptedByGame {
            package: "Demo".into(),
        };
        let sa = addon_story(&world_with(PackageStatus::Interrupted, a.clone()), "demo").unwrap();
        let sb = addon_story(&world_with(PackageStatus::Interrupted, b.clone()), "demo").unwrap();
        assert_eq!(sa.what, as_sentence(&render(&a)));
        assert_eq!(sb.what, as_sentence(&render(&b)));
        assert_ne!(sa.what, sb.what);
        assert_eq!(
            sa.next,
            wow_sync_core::view::NextAction::FinishOnNextSync.sentence()
        );
        assert!(!sa.next.contains("WoW"), "{}", sa.next);
    }

    #[test]
    fn a_missing_candidate_version_reads_not_available_never_blank() {
        let w = enrolled();
        let cand = |v: &str| Candidate {
            version: v.into(),
            holders: vec!["Laptop".into()],
            differences: String::new(),
        };
        let a = Addon {
            key: "k".into(),
            title: "Demo".into(),
            version: "1".into(),
            target_version: None,
            folders: vec![],
            kind: Kind::Available,
            candidates: vec![cand("1.0.0"), cand("")],
            why: String::new(),
            next: wow_sync_core::view::NextAction::Nothing,
            awaiting: vec![],
            waiting_on: vec![],
            waiting_secs: None,
        };
        let text = decision_reason(&w, &a, DecisionReason::Ambiguous);
        assert!(text.contains("(1.0.0 and not available)"), "{text}");
        assert_eq!(version_label(&a.candidates[1]), "not available");
        assert_eq!(version_label(&cand("  ")), "not available");
    }

    #[test]
    fn the_paused_hint_never_promises_zero_while_an_update_waits() {
        let mut w = world_with(
            PackageStatus::Interrupted,
            wow_sync_core::explain::Explanation::Interrupted {
                package: "Demo".into(),
            },
        );
        w.paused_since = Some("12:00".into());
        let hint = paused_hint(&w).unwrap();
        assert!(!hint.starts_with("0 "), "{hint}");
        assert!(hint.contains("1 add-on waiting"), "{hint}");
        let mut ready = world_with(
            PackageStatus::Available,
            wow_sync_core::explain::Explanation::InSync,
        );
        ready.paused_since = Some("12:00".into());
        assert_eq!(
            paused_hint(&ready).unwrap(),
            "1 update will be applied when you resume"
        );
    }

    #[test]
    fn next_step_text_is_core_s_for_other_actions() {
        use wow_sync_core::explain::Explanation as E;
        use wow_sync_core::view::NextAction;
        let s = addon_story(
            &world_with(
                PackageStatus::Unavailable,
                E::Interrupted {
                    package: "Demo".into(),
                },
            ),
            "demo",
        )
        .unwrap();
        assert_eq!(s.next, NextAction::WaitForDropbox.sentence());
        let s = addon_story(
            &world_with(
                PackageStatus::Available,
                E::Interrupted {
                    package: "Demo".into(),
                },
            ),
            "demo",
        )
        .unwrap();
        assert_eq!(s.next, NextAction::ApplyOnNextSync.sentence());
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
        assert!(quit_sheet_for(&w, false).is_some());
    }

    #[test]
    fn a_quit_during_setup_never_opens_a_sheet_that_setup_cannot_draw() {
        let mut w = enrolled();
        w.busy = Some(Busy::Syncing { done: 0, total: 0 });
        assert!(quit_sheet_for(&w, true).is_none());
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
