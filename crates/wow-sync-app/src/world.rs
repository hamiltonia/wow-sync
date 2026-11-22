//! The UI's view of the real runtime. Everything here is built from a `StatusSnapshot` and the
//! small control reads (pause, activity); nothing is invented. Fields the runtime cannot supply
//! yet stay empty and the pages show a designed "not available yet" state instead.

use std::path::PathBuf;
use wow_sync_core::plan::Activity;
use wow_sync_core::status::{MemberSummary, PackageStatus, StatusView};
use wow_sync_core::target::DecisionReason;
use wow_sync_core::view::NextAction;
use wow_sync_runtime::{ActivityEntry, EnrollPreview, MemberRow, PackageRow, StatusSnapshot};

// ---------------------------------------------------------------------------------------------
// Time helpers (real clock; the local offset comes from Windows, UTC elsewhere)

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn rel(secs: i64) -> String {
    let secs = secs.max(0);
    let (m, h, d) = (secs / 60, secs / 3600, secs / 86400);
    if secs < 60 {
        "just now".into()
    } else if m < 60 {
        format!("{m} min ago")
    } else if h < 24 {
        format!("{h} {} ago", if h == 1 { "hour" } else { "hours" })
    } else if d < 21 {
        format!("{d} {} ago", if d == 1 { "day" } else { "days" })
    } else {
        format!("{} weeks ago", d / 7)
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Civil date from days since 1970-01-01 (proleptic Gregorian).
fn civil(days: i64) -> (i64, usize, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, (m - 1) as usize, d)
}

/// "21:02", "Today" style parts for a unix-ms instant in local time.
pub fn local_parts(unix_ms: i64) -> (i64, String) {
    let local = unix_ms / 1000 + i64::from(crate::win::local_offset_minutes()) * 60;
    let days = local.div_euclid(86_400);
    let rem = local.rem_euclid(86_400);
    (days, format!("{:02}:{:02}", rem / 3600, (rem % 3600) / 60))
}

/// The local date of a recorded time, e.g. "16 November" (a record's own clock, so day-level).
pub fn day_text(unix_ms: u64) -> String {
    let (days, _) = local_parts(i64::try_from(unix_ms).unwrap_or(i64::MAX / 2));
    let (_, m, d) = civil(days);
    format!("{d} {}", MONTHS[m])
}

/// Absolute local time of something `secs` ago, e.g. "21:02 on 16 November".
pub fn absolute(secs: i64) -> String {
    let at = now_ms() - secs.max(0) * 1000;
    let (days, hm) = local_parts(at);
    let (_, m, d) = civil(days);
    format!("{hm} on {d} {}", MONTHS[m])
}

fn day_label(days: i64, today: i64) -> String {
    match today - days {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        _ => {
            let (_, m, d) = civil(days);
            format!("{d} {}", MONTHS[m])
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Types the pages read

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    InSync,
    /// Indexes into `World::peers`.
    Confirmed(Vec<usize>),
    AppliedHere,
    Available,
    Unavailable,
    /// A plan-level block (WoW running, files changing) holds this update.
    Blocked,
    NeedsDecision(DecisionReason),
    NeedsHelp,
    Interrupted,
    NotSynced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub version: String,
    pub holders: Vec<String>,
    pub differences: String,
}

#[derive(Clone, Debug)]
pub struct Addon {
    pub key: String,
    pub title: String,
    pub version: String,
    pub target_version: Option<String>,
    pub folders: Vec<String>,
    pub kind: Kind,
    pub candidates: Vec<Candidate>,
    /// The runtime's own sentence (core's `Explanation`, rendered) for why this add-on is in
    /// its current state.
    pub why: String,
    /// Core's next action for this add-on (`core::view::next_action`).
    pub next: NextAction,
    /// Computers that have not yet reported applying this package.
    pub awaiting: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub state: MemberSummary,
    pub report_secs_ago: Option<i64>,
    pub game: String,
    pub app: String,
    pub confirmed: u32,
}

#[derive(Clone, Debug)]
pub struct ActivityItem {
    pub title: String,
    pub items: Vec<String>,
    pub problem: bool,
    pub day: String,
    pub when: String,
}

#[derive(Clone, Debug)]
pub struct Install {
    pub identity: String,
    pub folder: String,
    pub build: String,
    pub root: PathBuf,
    pub recognised: bool,
    /// False until the read-only check of this installation has come back.
    pub inspected: bool,
    pub refused: Option<String>,
    /// True when an outside fix (files, folders) could change a refusal.
    pub can_retry: bool,
    pub test_mode_blocked: bool,
    pub eligible: Vec<String>,
    pub blocked: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub identity: String,
}

#[derive(Clone, Debug, Default)]
pub struct SetupData {
    pub dropbox: Option<String>,
    /// Where the shared data lives: a folder inside Dropbox, or the developer's provider folder.
    pub shared_root: Option<PathBuf>,
    pub online_only: bool,
    pub installs: Vec<Install>,
    pub groups: Vec<Group>,
    pub computer: String,
}

#[derive(Clone, Debug)]
pub struct Reconfirm {
    pub what_changed: String,
    pub identity: String,
    pub eligible: Vec<String>,
    pub blocked: Vec<String>,
    pub digest: Option<wow_sync_core::ids::Sha256>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Busy {
    Syncing { done: u32, total: u32 },
    Pausing,
}

/// UG-18 (update check) is deferred; only the running version is real.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
}

#[derive(Clone, Debug)]
pub struct World {
    pub enrolled: bool,
    pub setup: Option<SetupData>,
    pub computer: String,
    pub group: String,
    pub identity_label: String,
    pub build: String,
    pub addons: Vec<Addon>,
    pub peers: Vec<Peer>,
    pub busy: Option<Busy>,
    pub paused_since: Option<String>,
    pub pause_damaged: bool,
    pub game_running: bool,
    pub provider_error: Option<String>,
    pub test_mode: bool,
    /// Test mode is what blocks this install (typed signal from the status snapshot).
    pub test_mode_blocked: bool,
    pub stale_secs: Option<i64>,
    pub reconfirm: Option<Reconfirm>,
    pub features: crate::features::FeatureData,
    pub activity: Vec<ActivityItem>,
    pub update: UpdateInfo,
    pub last_shared_secs: Option<i64>,
    pub last_check_secs: Option<i64>,
    /// Local clock time ("21:05") of the next scheduled check; `None` when none is scheduled.
    pub next_check_at: Option<String>,
    pub recovery_extra: u32,
    pub group_id: String,
    pub computer_id: String,
    pub variant_folder: String,
    pub shared_label: String,
    pub view: StatusView,
}

/// The real inputs besides the snapshot.
pub struct Inputs<'a> {
    pub snap: &'a StatusSnapshot,
    pub paused_since_ms: Option<i64>,
    pub pause_damaged: bool,
    pub activity: &'a [ActivityEntry],
    pub syncing: bool,
    pub last_check_ms: Option<i64>,
    pub next_check_ms: Option<i64>,
    pub setup: Option<SetupData>,
    pub app_version: &'a str,
}

pub fn placeholder_view() -> StatusView {
    StatusView {
        headline: wow_sync_core::status::Headline::Waiting,
        packages: Vec::new(),
        members: Vec::new(),
        last_publish: None,
        blocks: Vec::new(),
        retained_extra_recovery: 0,
    }
}

fn secs_since(now: i64, then_ms: i64) -> i64 {
    ((now - then_ms) / 1000).max(0)
}

fn kind_of(p: &PackageRow, peers: &[&MemberRow]) -> Kind {
    match &p.status {
        PackageStatus::InSync => Kind::InSync,
        PackageStatus::ConfirmedBy(ids) => Kind::Confirmed(
            ids.iter()
                .filter_map(|id| peers.iter().position(|m| m.id == id.as_str()))
                .collect(),
        ),
        PackageStatus::AppliedHere => Kind::AppliedHere,
        PackageStatus::Available => Kind::Available,
        PackageStatus::Unavailable => Kind::Unavailable,
        PackageStatus::NotSynced => Kind::NotSynced,
        PackageStatus::Interrupted => Kind::Interrupted,
        PackageStatus::NeedsHelp(_) => Kind::NeedsHelp,
        PackageStatus::NeedsDecision(r) => Kind::NeedsDecision(*r),
        PackageStatus::Blocked(_) => Kind::Blocked,
        PackageStatus::Removing => Kind::Available,
        PackageStatus::Held => Kind::Blocked,
    }
}

fn plain_text(t: &str) -> String {
    if t.starts_with("published seq ") {
        return "Shared this computer's add-on changes".into();
    }
    if t.starts_with("enrolled group ") {
        return "This computer joined the add-on group".into();
    }
    t.to_string()
}

fn activity_items(entries: &[ActivityEntry], now: i64) -> Vec<ActivityItem> {
    let (today, _) = local_parts(now);
    let entries: Vec<ActivityEntry> = entries
        .iter()
        .map(|e| ActivityEntry {
            text: plain_text(&e.text),
            ..e.clone()
        })
        .collect();
    let entries = &entries[..];
    let mut out: Vec<ActivityItem> = Vec::new();
    let mut batch_start: Option<i64> = None;
    for e in entries {
        let problem = e.kind == "problem";
        let Some(ms) = e.unix_ms else {
            out.push(ActivityItem {
                title: e.text.clone(),
                items: Vec::new(),
                problem,
                day: "Earlier".into(),
                when: String::new(),
            });
            batch_start = None;
            continue;
        };
        let joins = e.kind == "applied" || e.kind == "published";
        if let (true, Some(start), Some(last)) = (joins, batch_start, out.last_mut()) {
            if (start - ms).abs() < 90_000 && !last.problem && last.items.len() < 200 {
                if last.items.is_empty() {
                    last.items.push(last.title.clone());
                }
                last.items.push(e.text.clone());
                last.title = format!("{} changes", last.items.len());
                continue;
            }
        }
        let (days, hm) = local_parts(ms);
        out.push(ActivityItem {
            title: e.text.clone(),
            items: Vec::new(),
            problem,
            day: day_label(days, today),
            when: hm,
        });
        batch_start = joins.then_some(ms);
    }
    out
}

impl World {
    /// Before the first status arrives or when the service could not start.
    pub fn empty(app_version: &str) -> World {
        World {
            enrolled: false,
            setup: None,
            computer: String::new(),
            group: String::new(),
            identity_label: String::new(),
            build: String::new(),
            addons: Vec::new(),
            peers: Vec::new(),
            busy: None,
            paused_since: None,
            pause_damaged: false,
            game_running: false,
            provider_error: None,
            test_mode: false,
            test_mode_blocked: false,
            stale_secs: None,
            reconfirm: None,
            features: Default::default(),
            activity: Vec::new(),
            update: UpdateInfo {
                current: app_version.to_string(),
            },
            last_shared_secs: None,
            last_check_secs: None,
            next_check_at: None,
            recovery_extra: 0,
            group_id: String::new(),
            computer_id: String::new(),
            variant_folder: String::new(),
            shared_label: String::new(),
            view: placeholder_view(),
        }
    }

    /// Enrollment was saved but the first status has not been read yet; the install is enrolled,
    /// so setup must never be shown for it.
    pub fn confirm_enrolled(&mut self, enrolled_now: bool) {
        self.enrolled |= enrolled_now;
    }

    pub fn from_snapshot(i: Inputs) -> World {
        let now = now_ms();
        let mut w = World::empty(i.app_version);
        let s = i.snap;
        w.enrolled = s.enrolled;
        w.setup = i.setup;
        w.computer = s.computer_name.clone();
        w.group = s.group_name.clone();
        w.identity_label = s.identity_line.clone();
        w.provider_error = s.provider_error.clone();
        w.test_mode = s.test_mode;
        w.test_mode_blocked = s.test_mode_blocked;
        w.recovery_extra = s.retained_extra_recovery;
        w.reconfirm = s.reconfirm.as_ref().map(reconfirm_of);
        w.paused_since = i.paused_since_ms.map(|ms| {
            let (_, hm) = local_parts(ms);
            hm
        });
        w.pause_damaged = i.pause_damaged;
        w.busy = i.syncing.then_some(Busy::Syncing { done: 0, total: 0 });
        w.last_check_secs = i.last_check_ms.map(|ms| secs_since(now, ms));
        w.next_check_at = i
            .next_check_ms
            .filter(|_| !w.is_paused())
            .map(|ms| local_parts(ms).1);
        w.activity = activity_items(i.activity, now);
        // The service only remembers a publish made in this run; the log remembers older ones.
        let logged_publish = i
            .activity
            .iter()
            .filter(|e| e.kind == "published")
            .filter_map(|e| e.unix_ms)
            .max();
        w.last_shared_secs = logged_publish.map(|ms| secs_since(now, ms));
        if let Some(d) = &s.detail {
            w.view = d.view.clone();
            w.build = d.game_build.clone().unwrap_or_default();
            w.game_running = matches!(d.game, Activity::Running { .. });
            w.group_id = d.group_id.clone();
            w.computer_id = d.computer_id.clone();
            w.variant_folder = d.variant_folder.clone();
            w.last_shared_secs = d
                .view
                .last_publish
                .map(|ms| secs_since(now, ms))
                .or(w.last_shared_secs);
            let others: Vec<&MemberRow> = d.members.iter().filter(|m| !m.is_this).collect();
            w.peers = others
                .iter()
                .map(|m| Peer {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    state: m.state,
                    report_secs_ago: m.seen_locally_at.map(|ms| secs_since(now, ms)),
                    game: m.game_build.clone().unwrap_or_default(),
                    app: m.app_version.clone().unwrap_or_default(),
                    confirmed: m.confirmed,
                })
                .collect();
            w.addons = d
                .packages
                .iter()
                .map(|p| Addon {
                    key: p.key.clone(),
                    title: p.title.clone(),
                    version: p.version_here.clone().unwrap_or_default(),
                    target_version: p.version_target.clone(),
                    folders: p.folders.iter().map(|f| f.name.clone()).collect(),
                    kind: kind_of(p, &others),
                    candidates: p
                        .candidates
                        .iter()
                        .map(|c| Candidate {
                            version: c.version.clone().unwrap_or_default(),
                            holders: c.holders.clone(),
                            differences: c.differences.clone(),
                        })
                        .collect(),
                    why: p.sentence.clone(),
                    next: p.next,
                    awaiting: p.awaiting.clone(),
                })
                .collect();
        } else {
            w.view = placeholder_view();
        }
        w
    }

    pub fn synced_total(&self) -> u32 {
        self.addons
            .iter()
            .filter(|a| a.kind != Kind::NotSynced)
            .count() as u32
    }

    /// How many current add-ons peer `p` last confirmed (UG-4).
    pub fn confirmed_by(&self, p: usize) -> u32 {
        self.peers.get(p).map(|x| x.confirmed).unwrap_or(0)
    }

    pub fn addon(&self, key: &str) -> Option<&Addon> {
        self.addons.iter().find(|a| a.key.as_str() == key)
    }

    pub fn is_paused(&self) -> bool {
        self.paused_since.is_some() || self.pause_damaged
    }
}

fn reconfirm_of(p: &EnrollPreview) -> Reconfirm {
    Reconfirm {
        what_changed: p.scope_line.clone(),
        identity: p.identity_line.clone(),
        eligible: p.eligible.clone(),
        blocked: p.blocked.clone(),
        digest: p.digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ms: i64, kind: &str, text: &str) -> ActivityEntry {
        ActivityEntry {
            unix_ms: Some(ms),
            kind: kind.into(),
            package: None,
            text: text.into(),
        }
    }

    #[test]
    fn a_just_enrolled_install_is_never_shown_as_not_set_up() {
        let mut w = World::empty("0.0.0");
        w.confirm_enrolled(false);
        assert!(!w.enrolled);
        w.confirm_enrolled(true);
        assert!(w.enrolled);
        assert_ne!(crate::model::headline(&w).text, "Set up this computer");
    }

    #[test]
    fn empty_snapshot_shows_no_data() {
        let snap = StatusSnapshot::default();
        let w = World::from_snapshot(Inputs {
            snap: &snap,
            paused_since_ms: None,
            pause_damaged: false,
            activity: &[],
            syncing: false,
            last_check_ms: None,
            next_check_ms: None,
            setup: None,
            app_version: "1.2.3",
        });
        assert!(!w.enrolled);
        assert!(w.addons.is_empty() && w.peers.is_empty() && w.activity.is_empty());
        assert!(w.last_shared_secs.is_none() && w.last_check_secs.is_none());
        assert_eq!(w.update.current, "1.2.3");
        assert!(!w.is_paused());
    }

    #[test]
    fn paused_and_syncing_come_from_inputs() {
        let snap = StatusSnapshot::default();
        let w = World::from_snapshot(Inputs {
            snap: &snap,
            paused_since_ms: Some(now_ms()),
            pause_damaged: false,
            activity: &[],
            syncing: true,
            last_check_ms: Some(now_ms() - 5_000),
            next_check_ms: Some(now_ms() + 60_000),
            setup: None,
            app_version: "x",
        });
        assert!(w.is_paused());
        assert!(
            w.next_check_at.is_none(),
            "no check is scheduled while paused"
        );
        assert!(matches!(w.busy, Some(Busy::Syncing { .. })));
        assert!(w.last_check_secs.is_some_and(|s| (4..60).contains(&s)));
    }

    #[test]
    fn activity_batches_close_applies_and_keeps_problems_apart() {
        let t = now_ms() - 60_000;
        let items = activity_items(
            &[
                entry(t, "applied", "Updated A"),
                entry(t - 1_000, "applied", "Updated B"),
                entry(t - 2_000, "problem", "Could not read C"),
                entry(t - 3_000, "paused", "Paused"),
            ],
            now_ms(),
        );
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "2 changes");
        assert_eq!(items[0].items, ["Updated A", "Updated B"]);
        assert!(items[1].problem && items[1].items.is_empty());
        assert_eq!(items[2].title, "Paused");
    }

    #[test]
    fn activity_jargon_is_rewritten_and_last_shared_comes_from_the_log() {
        let t = now_ms() - 120_000;
        let log = [
            entry(t, "published", "published seq 7 (6 new objects)"),
            entry(
                t - 5_000,
                "audit",
                "enrolled group c711e3bc581b7b837690423711570789",
            ),
        ];
        let items = activity_items(&log, now_ms());
        assert_eq!(items[0].title, "Shared this computer's add-on changes");
        assert_eq!(items[1].title, "This computer joined the add-on group");
        let snap = StatusSnapshot::default();
        let w = World::from_snapshot(Inputs {
            snap: &snap,
            paused_since_ms: None,
            pause_damaged: false,
            activity: &log,
            syncing: false,
            last_check_ms: None,
            next_check_ms: None,
            setup: None,
            app_version: "x",
        });
        assert!(w.last_shared_secs.is_some_and(|s| (115..180).contains(&s)));
    }

    #[test]
    fn activity_without_a_time_is_not_given_one() {
        let items = activity_items(
            &[ActivityEntry {
                unix_ms: None,
                kind: "applied".into(),
                package: None,
                text: "Old line".into(),
            }],
            now_ms(),
        );
        assert_eq!(items[0].day, "Earlier");
        assert!(items[0].when.is_empty());
    }
}
