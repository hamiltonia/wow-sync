//! Deterministic fake scenarios S1..S16 (docs/design/ui/prototype.md). Everything here is
//! in-memory fake data. The prototype never reads WoW folders, the provider folder or Dropbox,
//! and never writes a file. Data the core does not expose yet (UG-n) lives in `Fake…` structs.

use crate::notify::{NotifEngine, NotifKind};
use std::collections::BTreeMap;
use wow_sync_core::explain::{AmbiguousCandidate, Explanation};
use wow_sync_core::ids::{ComputerId, GroupId, Sha256};
use wow_sync_core::package::{ClientInfo, FolderEvidence, PackageVersion};
use wow_sync_core::paths::{validate_folder_name, FolderKey};
use wow_sync_core::plan::PlanBlock;
use wow_sync_core::status::{
    derive_status, Headline, MemberSummary, PackageStatus, StatusInput, StatusView,
};
use wow_sync_core::target::{DecisionReason, EffectiveTarget, GroupTarget, PackageTarget};
use wow_sync_core::variant::{ProductCode, VariantIdentity, KNOWN_PROFILES};

/// "Now" on the fake clock is 21:10 on 16 November.
const NOW_MINUTES: i64 = 21 * 60 + 10;

pub const COMPUTER: &str = "Desk PC";
pub const GROUP: &str = "Main add-ons";
pub const FOREVER_FOLDER: &str = "_forever_";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioId {
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
}

pub const ALL_SCENARIOS: [ScenarioId; 16] = [
    ScenarioId::S1,
    ScenarioId::S2,
    ScenarioId::S3,
    ScenarioId::S4,
    ScenarioId::S5,
    ScenarioId::S6,
    ScenarioId::S7,
    ScenarioId::S8,
    ScenarioId::S9,
    ScenarioId::S10,
    ScenarioId::S11,
    ScenarioId::S12,
    ScenarioId::S13,
    ScenarioId::S14,
    ScenarioId::S15,
    ScenarioId::S16,
];

impl ScenarioId {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_uppercase();
        ALL_SCENARIOS.iter().copied().find(|i| i.code() == s)
    }

    pub fn code(self) -> String {
        format!(
            "S{}",
            ALL_SCENARIOS.iter().position(|i| *i == self).unwrap() + 1
        )
    }

    pub fn next(self) -> Self {
        let i = ALL_SCENARIOS.iter().position(|i| *i == self).unwrap();
        ALL_SCENARIOS[(i + 1) % ALL_SCENARIOS.len()]
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::S1 => "First run, recognised client",
            Self::S2 => "First run, unfamiliar client and refused layout",
            Self::S3 => "All good",
            Self::S4 => "Syncing",
            Self::S5 => "WoW running",
            Self::S6 => "Waiting for Dropbox",
            Self::S7 => "Add-on decisions; other add-ons keep syncing",
            Self::S8 => "Shared folder error",
            Self::S9 => "Silent and late computers",
            Self::S10 => "Paused",
            Self::S11 => "Empty group",
            Self::S12 => "Reconfirm needed: add-on file rule changed",
            Self::S13 => "Update available",
            Self::S14 => "Large collection",
            Self::S15 => "Game patched: build-only change",
            Self::S16 => "Interrupted update needs help; all applies here stopped",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::S1 => "Setup: Empty, Working, Success. Set up fakes 2 s of work.",
            Self::S2 => "Setup: unfamiliar client needs the identity checkbox; one layout is refused.",
            Self::S3 => "42 add-ons, one computer confirms; Success.",
            Self::S4 => "5 updates apply over 6 s; Working.",
            Self::S5 => "Game blocks changes; notifications are held. Button: Simulate WoW closed.",
            Self::S6 => "Dropbox is still downloading; Waiting.",
            Self::S7 => "Three decisions; Details! still updates; Keep this copy fakes saved, waiting, done.",
            Self::S8 => "Dropbox is not running; Try again recovers after 2 s.",
            Self::S9 => "Den PC silent for 12 days; button Simulate: Den PC reports.",
            Self::S10 => "Paused since 21:04; Resume continues in S4.",
            Self::S11 => "Enrolled, nothing to show yet.",
            Self::S12 => "The add-on file rule changed; Reconfirm; syncing stays stopped.",
            Self::S13 => "Version 0.2.0 is available; Check for updates.",
            Self::S14 => "300 add-ons and 4 computers for scrolling and focus checks.",
            Self::S15 => "Button: Simulate game patched changes only the build; no reconfirm.",
            Self::S16 => "An interrupted update needs help; no updates are applied here.",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Time helpers (fake clock)

pub fn rel(secs: i64) -> String {
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

/// Absolute local time of something `secs` ago, e.g. "21:02 on 16 November".
pub fn absolute(secs: i64) -> String {
    let mut m = NOW_MINUTES - secs / 60;
    let mut day = 16;
    while m < 0 {
        m += 1440;
        day -= 1;
    }
    format!("{:02}:{:02} on {day} November", m / 60, m % 60)
}

pub fn cid(n: u32) -> ComputerId {
    ComputerId::parse(&format!("{n:032x}")).expect("32 hex characters")
}

// ---------------------------------------------------------------------------------------------
// Fake data types

/// UG-25: which continuation sentence applies. Core has no typed field for it yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeApplyScope {
    PerPackage,
    AllAppliesBlocked,
}

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

/// UG-8: the candidate copies of a decision with who holds each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeCandidate {
    pub version: String,
    pub holders: Vec<String>,
    pub differences: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionPhase {
    Saving,
    Saved(usize),
    Done(usize),
}

/// UG-3: display title, version and folders (core only has the folder key today).
#[derive(Clone, Debug)]
pub struct Addon {
    pub key: FolderKey,
    pub title: String,
    pub version: String,
    pub target_version: Option<String>,
    pub folders: Vec<String>,
    pub kind: Kind,
    pub source: Option<String>,
    pub candidates: Vec<FakeCandidate>,
    pub updated_secs_ago: Option<i64>,
}

/// UG-4/5/6: what a peer last confirmed and when this computer received its report.
#[derive(Clone, Debug)]
pub struct Peer {
    pub name: String,
    pub state: MemberSummary,
    pub report_secs_ago: Option<i64>,
    pub game: String,
}

#[derive(Clone, Debug)]
pub struct ActivityItem {
    pub title: String,
    pub items: Vec<String>,
    pub problem: bool,
    pub day: String,
    pub when: String,
}

/// UG-24: setup data (paths come from command-line options today).
#[derive(Clone, Debug)]
pub struct FakeInstall {
    pub identity: String,
    pub folder: String,
    pub build: String,
    pub recognised: bool,
    pub refused: Option<String>,
    pub eligible: Vec<String>,
    pub blocked: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct FakeGroup {
    pub name: String,
    pub identity: String,
    pub last_activity: String,
}

#[derive(Clone, Debug)]
pub struct SetupData {
    pub dropbox: Option<String>,
    pub online_only: bool,
    pub installs: Vec<FakeInstall>,
    pub groups: Vec<FakeGroup>,
}

/// UG-16: what changed since the confirmation was given.
#[derive(Clone, Debug)]
pub struct FakeReconfirm {
    pub what_changed: String,
    pub identity: String,
    pub eligible: Vec<String>,
    pub newly_eligible: Vec<String>,
    pub blocked: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Busy {
    Syncing { done: u32, total: u32 },
    Pausing,
}

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
    pub available: Option<String>,
    pub checking: bool,
    pub checked_secs_ago: Option<i64>,
}

/// Timed steps the app schedules; applying one is pure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    FinishSyncing,
    ApplyDetails,
    FinishSync,
    SetupDone,
    TryAgainDone,
    UpdateChecked,
    DecisionSaved(String),
    DecisionApplied(String),
    NeedsHelpResolved,
}

#[derive(Clone, Debug)]
pub struct World {
    pub scenario: ScenarioId,
    pub enrolled: bool,
    pub setup: Option<SetupData>,
    pub computer: String,
    pub group: String,
    pub identity: VariantIdentity,
    pub build: String,
    pub addons: Vec<Addon>,
    pub peers: Vec<Peer>,
    pub apply_scope: FakeApplyScope,
    pub busy: Option<Busy>,
    pub paused_since: Option<String>,
    pub game_running: bool,
    pub unstable_block: bool,
    pub provider_error: Option<String>,
    pub stale_secs: Option<i64>,
    pub reconfirm: Option<FakeReconfirm>,
    pub activity: Vec<ActivityItem>,
    pub notifs: NotifEngine,
    pub update: UpdateInfo,
    pub decisions: BTreeMap<String, DecisionPhase>,
    pub last_shared_secs: Option<i64>,
    pub last_check_secs: Option<i64>,
    pub recovery_extra: u32,
    pub needs_help_resolved: bool,
    pub view: StatusView,
}

// ---------------------------------------------------------------------------------------------
// Construction

const NAMES: [&str; 42] = [
    "Deadly Boss Mods",
    "WeakAuras",
    "Details!",
    "Bagnon",
    "Questie",
    "Trail Markers",
    "Mailbox Helper",
    "Auction Lens",
    "Bag Sorter Lite",
    "Chat Tidy",
    "Cooldown Strip",
    "Damage Meter Mini",
    "Dungeon Notes",
    "Flight Timer",
    "Gear Compare",
    "Guild Roster Plus",
    "Healer Frames",
    "Inspect Plus",
    "Interrupt Alert",
    "Keybind Helper",
    "Loot Council",
    "Map Pins",
    "Minimap Buttons",
    "Nameplate Lite",
    "Party Buffs",
    "Profession Tracker",
    "Quest Log Plus",
    "Raid Cooldowns",
    "Reputation Bars",
    "Roll Tracker",
    "Scroll Frames",
    "Spell Queue",
    "Stat Summary",
    "Talent Notes",
    "Threat Meter Lite",
    "Tooltip Extras",
    "Unit Frames Slim",
    "Vendor Price",
    "Zone Info",
    "Camera Tweaks",
    "Action Bar Keys",
    "Combat Text Lite",
];

fn slug(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn addon(i: usize, title: &str, kind: Kind) -> Addon {
    let key = validate_folder_name(&format!("{}-{i}", slug(title))).expect("valid folder key");
    let version = match title {
        "Deadly Boss Mods" => "11.0.3".to_string(),
        "WeakAuras" => "5.18.1".to_string(),
        "Details!" => "9.2.4".to_string(),
        _ => format!("1.{}.{}", i % 9, i % 7),
    };
    let folders = if title == "Deadly Boss Mods" {
        vec![
            "DBM-Core".into(),
            "DBM-StatusBarTimers".into(),
            "DBM-GUI".into(),
        ]
    } else {
        vec![title.replace(' ', "")]
    };
    Addon {
        key,
        title: title.to_string(),
        version,
        target_version: None,
        folders,
        kind,
        source: None,
        candidates: Vec::new(),
        updated_secs_ago: None,
    }
}

fn confirmed_all(i: usize, name: &str, peers: &[usize]) -> Addon {
    addon(i, name, Kind::Confirmed(peers.to_vec()))
}

fn base_addons(laptop_confirms: &[usize]) -> Vec<Addon> {
    let mut v: Vec<Addon> = NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| confirmed_all(i, n, laptop_confirms))
        .collect();
    v.push(addon(100, "Legacy Suffixed Pack", Kind::NotSynced));
    v.push(addon(101, "Tooltip Skin Suffixed", Kind::NotSynced));
    v
}

fn set_kind(w: &mut [Addon], name: &str, kind: Kind) -> usize {
    let i = w.iter().position(|a| a.title == name).expect(name);
    w[i].kind = kind;
    i
}

fn peer(name: &str, state: MemberSummary, ago: Option<i64>) -> Peer {
    Peer {
        name: name.into(),
        state,
        report_secs_ago: ago,
        game: "World of Warcraft: Forever (wowf)".into(),
    }
}

pub fn forever_identity() -> VariantIdentity {
    VariantIdentity {
        product: ProductCode::parse("wowf").expect("wowf"),
        folder: FOREVER_FOLDER.into(),
        build_version: Some("fake-build-A".into()),
        profile: KNOWN_PROFILES.first(),
    }
}

fn unfamiliar_identity() -> VariantIdentity {
    VariantIdentity {
        product: ProductCode::parse("wowx").expect("wowx"),
        folder: "_example_".into(),
        build_version: Some("fake-build-X".into()),
        profile: None,
    }
}

fn act(day: &str, when: &str, title: &str, items: &[&str]) -> ActivityItem {
    ActivityItem {
        title: title.into(),
        items: items.iter().map(|s| s.to_string()).collect(),
        problem: false,
        day: day.into(),
        when: when.into(),
    }
}

fn three_batches() -> Vec<ActivityItem> {
    vec![
        act(
            "Today",
            "21:05",
            "Updated 3 add-ons from Laptop",
            &[
                "Deadly Boss Mods 11.0.2 to 11.0.3",
                "WeakAuras 5.18.0 to 5.18.1",
                "Questie 6.1.0 to 6.1.1",
            ],
        ),
        act(
            "Today",
            "18:40",
            "Updated 2 add-ons from Laptop",
            &["Bagnon 10.2.1 to 10.2.2", "Details! 9.2.3 to 9.2.4"],
        ),
        act(
            "Yesterday",
            "22:15",
            "Updated 1 add-on from Laptop",
            &["Raid Cooldowns 1.4.0 to 1.4.1"],
        ),
    ]
}

fn setup_data(unfamiliar: bool) -> SetupData {
    let eligible: Vec<String> = (0..38).map(|i| format!("AddOn-{:02}", i + 1)).collect();
    let blocked = vec![
        (
            "Legacy-Pack".to_string(),
            "has a game-version-specific .toc file".to_string(),
        ),
        (
            "Skin-Pack".to_string(),
            "has a game-version-specific .toc file".to_string(),
        ),
    ];
    let mut installs = vec![FakeInstall {
        identity: forever_identity().display_label(),
        folder: FOREVER_FOLDER.into(),
        build: "fake-build-A".into(),
        recognised: true,
        refused: None,
        eligible: eligible.clone(),
        blocked: blocked.clone(),
    }];
    if unfamiliar {
        installs = vec![
            FakeInstall {
                identity: unfamiliar_identity().display_label(),
                folder: "_example_".into(),
                build: "fake-build-X".into(),
                recognised: false,
                refused: None,
                eligible: eligible[..12].to_vec(),
                blocked: blocked.clone(),
            },
            FakeInstall {
                identity: "Unfamiliar game version (wowy)".into(),
                folder: "_sample_".into(),
                build: "fake-build-Y".into(),
                recognised: false,
                refused: Some("its AddOns folder is missing".into()),
                eligible: Vec::new(),
                blocked: Vec::new(),
            },
        ];
    }
    SetupData {
        dropbox: Some("Dropbox (fake folder)".into()),
        online_only: unfamiliar,
        installs,
        groups: if unfamiliar {
            Vec::new()
        } else {
            vec![FakeGroup {
                name: GROUP.into(),
                identity: forever_identity().display_label(),
                last_activity: "5 min ago".into(),
            }]
        },
    }
}

impl World {
    pub fn new(id: ScenarioId) -> World {
        use ScenarioId::*;
        let laptop = 0usize;
        let mut addons = base_addons(&[laptop]);
        let mut peers = vec![peer("Laptop", MemberSummary::Usable, Some(300))];
        let mut w = World {
            scenario: id,
            enrolled: !matches!(id, S1 | S2),
            setup: None,
            computer: COMPUTER.into(),
            group: GROUP.into(),
            identity: forever_identity(),
            build: "fake-build-A".into(),
            addons: Vec::new(),
            peers: Vec::new(),
            apply_scope: FakeApplyScope::PerPackage,
            busy: None,
            paused_since: None,
            game_running: false,
            unstable_block: false,
            provider_error: None,
            stale_secs: None,
            reconfirm: None,
            activity: three_batches(),
            notifs: NotifEngine::default(),
            update: UpdateInfo {
                current: "0.1.0".into(),
                available: None,
                checking: false,
                checked_secs_ago: Some(7200),
            },
            decisions: BTreeMap::new(),
            last_shared_secs: Some(720),
            last_check_secs: Some(60),
            recovery_extra: 0,
            needs_help_resolved: false,
            view: placeholder_view(),
        };
        // S3 shape: 40 confirmed, 1 applied here, 1 in sync.
        let applied = |a: &mut Vec<Addon>| {
            set_kind(a, "Combat Text Lite", Kind::AppliedHere);
            set_kind(a, "Camera Tweaks", Kind::InSync);
            let i = a
                .iter()
                .position(|x| x.title == "Combat Text Lite")
                .unwrap();
            a[i].updated_secs_ago = Some(240);
            a[i].source = Some("Laptop".into());
            a[i].target_version = Some("1.5.3".into());
        };
        match id {
            S1 | S2 => {
                addons.clear();
                peers.clear();
                w.setup = Some(setup_data(id == S2));
                w.activity.clear();
            }
            S3 | S15 | S13 => {
                applied(&mut addons);
                // The detail page needs a "what changed" story for at least one add-on.
                let i = set_kind(&mut addons, "Deadly Boss Mods", Kind::AppliedHere);
                addons[i].updated_secs_ago = Some(300);
                addons[i].source = Some("Laptop".into());
                addons[i].target_version = Some("11.0.4".into());
                addons[i].version = "11.0.3".into();
                if id == S13 {
                    w.update.available = Some("0.2.0".into());
                    w.update.checked_secs_ago = Some(300);
                    w.notifs.raise(
                        NotifKind::AppUpdate,
                        "update-0.2.0",
                        "WoW Sync 0.2.0 is available. Open WoW Sync to download it.",
                    );
                }
            }
            S4 | S10 => {
                applied(&mut addons);
                for n in [
                    "Bagnon",
                    "Questie",
                    "Raid Cooldowns",
                    "Zone Info",
                    "Spell Queue",
                ] {
                    let i = set_kind(&mut addons, n, Kind::Available);
                    addons[i].source = Some("Laptop".into());
                    addons[i].target_version = Some(format!("{}.1", addons[i].version));
                }
                if id == S4 {
                    w.busy = Some(Busy::Syncing { done: 2, total: 5 });
                } else {
                    w.paused_since = Some("21:04".into());
                    w.activity
                        .insert(0, act("Today", "21:04", "Syncing paused", &[]));
                }
            }
            S5 => {
                applied(&mut addons);
                w.game_running = true;
                for n in ["Bagnon", "Questie"] {
                    let i = set_kind(&mut addons, n, Kind::Blocked);
                    addons[i].source = Some("Laptop".into());
                    addons[i].target_version = Some(format!("{}.1", addons[i].version));
                }
                set_kind(&mut addons, "Raid Cooldowns", Kind::Interrupted);
                w.activity.insert(
                    0,
                    act("Today", "21:02", "Waiting: WoW is running (2 updates)", &[]),
                );
                w.notifs.set_game_running(true);
                w.notifs.raise(
                    NotifKind::Summary,
                    "batch-a",
                    "2 add-ons updated from Laptop: WeakAuras, Details!.",
                );
                w.notifs.raise(
                    NotifKind::Summary,
                    "batch-b",
                    "1 add-on updated from Laptop: Spell Queue.",
                );
            }
            S6 => {
                applied(&mut addons);
                w.unstable_block = true;
                let i = set_kind(&mut addons, "Bagnon", Kind::Blocked);
                addons[i].source = Some("Laptop".into());
                for n in ["Questie", "Raid Cooldowns"] {
                    set_kind(&mut addons, n, Kind::Unavailable);
                }
            }
            S7 => {
                peers.push(peer("Den PC", MemberSummary::Usable, Some(1080)));
                let a = set_kind(
                    &mut addons,
                    "WeakAuras",
                    Kind::NeedsDecision(DecisionReason::Ambiguous),
                );
                addons[a].candidates = vec![
                    FakeCandidate {
                        version: "5.18.1".into(),
                        holders: vec!["This computer".into(), "Laptop".into()],
                        differences:
                            "Same folders. Version labels cannot be compared with the other copy."
                                .into(),
                    },
                    FakeCandidate {
                        version: "5.18.1-beta".into(),
                        holders: vec!["Den PC".into()],
                        differences:
                            "Same folders. Version labels cannot be compared with the other copy."
                                .into(),
                    },
                ];
                let b = set_kind(
                    &mut addons,
                    "Bagnon",
                    Kind::NeedsDecision(DecisionReason::PartialCandidate),
                );
                addons[b].candidates = vec![
                    FakeCandidate {
                        version: "10.2.2".into(),
                        holders: vec!["Laptop".into()],
                        differences: "Missing folders that other copies have: BagnonOptions."
                            .into(),
                    },
                    FakeCandidate {
                        version: "10.2.1".into(),
                        holders: vec!["This computer".into(), "Den PC".into()],
                        differences: "Has every folder.".into(),
                    },
                ];
                let q = set_kind(
                    &mut addons,
                    "Questie",
                    Kind::NeedsDecision(DecisionReason::DifferentTocMapping),
                );
                addons[q].candidates = vec![
                    FakeCandidate {
                        version: "6.1.1".into(),
                        holders: vec!["Laptop".into()],
                        differences:
                            "Came from a computer that reads add-on files by a different rule."
                                .into(),
                    },
                    FakeCandidate {
                        version: "6.1.0".into(),
                        holders: vec!["This computer".into()],
                        differences: "Uses this computer's add-on file rule.".into(),
                    },
                ];
                let d = set_kind(&mut addons, "Details!", Kind::Available);
                addons[d].source = Some("Laptop".into());
                addons[d].target_version = Some("9.2.5".into());
                for a in addons.iter_mut() {
                    if let Kind::Confirmed(v) = &mut a.kind {
                        v.push(1);
                    }
                }
                w.last_check_secs = Some(60);
                w.notifs.raise(
                    NotifKind::Action,
                    "decisions",
                    "3 add-ons need your decision.",
                );
            }
            S8 => {
                applied(&mut addons);
                w.provider_error = Some("Dropbox is not running".into());
                w.stale_secs = Some(3 * 3600);
                w.activity.insert(
                    0,
                    act("Today", "18:10", "Could not read the shared folder", &[]),
                );
                w.activity[0].problem = true;
            }
            S9 => {
                peers.push(peer("Den PC", MemberSummary::Usable, Some(12 * 86400)));
                peers.push(peer("Spare PC", MemberSummary::NoState, None));
                peers.push(peer("Old PC", MemberSummary::Regressed, Some(3 * 86400)));
                for a in addons.iter_mut() {
                    if let Kind::Confirmed(v) = &mut a.kind {
                        v.push(1);
                    }
                }
                let n = NAMES.len();
                for name in [NAMES[n - 1], NAMES[n - 2]] {
                    let i = addons.iter().position(|a| a.title == name).unwrap();
                    addons[i].kind = Kind::Confirmed(vec![0]);
                }
            }
            S11 => {
                addons.clear();
                peers.clear();
                w.activity.clear();
                w.last_shared_secs = None;
                w.last_check_secs = None;
            }
            S12 => {
                applied(&mut addons);
                let eligible: Vec<String> =
                    (0..43).map(|i| format!("AddOn-{:02}", i + 1)).collect();
                w.reconfirm = Some(FakeReconfirm {
                    what_changed: "WoW Sync now uses a different rule for which add-on files this game version loads. 5 more folders would sync.".into(),
                    identity: forever_identity().display_label(),
                    newly_eligible: eligible[38..].to_vec(),
                    eligible,
                    blocked: vec![(
                        "Legacy-Pack".to_string(),
                        "has a game-version-specific .toc file".to_string(),
                    )],
                });
            }
            S14 => {
                addons.clear();
                for i in 0..300usize {
                    let name = format!("{} {:03}", NAMES[i % NAMES.len()], i + 1);
                    let kind = match i % 17 {
                        0 => Kind::Available,
                        3 => Kind::Unavailable,
                        7 => Kind::NeedsDecision(DecisionReason::Ambiguous),
                        11 => Kind::NotSynced,
                        13 => Kind::InSync,
                        _ => Kind::Confirmed(vec![0]),
                    };
                    let mut a = addon(i, &name, kind);
                    if matches!(a.kind, Kind::Available) {
                        a.source = Some("Laptop".into());
                    }
                    addons.push(a);
                }
                peers.push(peer("Den PC", MemberSummary::Usable, Some(3 * 3600)));
                peers.push(peer("Spare PC", MemberSummary::Usable, Some(86400)));
                peers.push(peer("Old PC", MemberSummary::NoState, None));
            }
            S16 => {
                applied(&mut addons);
                w.apply_scope = FakeApplyScope::AllAppliesBlocked;
                set_kind(&mut addons, "Deadly Boss Mods", Kind::NeedsHelp);
                for n in ["Bagnon", "Questie", "Raid Cooldowns"] {
                    let i = set_kind(&mut addons, n, Kind::Available);
                    addons[i].source = Some("Laptop".into());
                    addons[i].target_version = Some(format!("{}.1", addons[i].version));
                }
                let mut a = act(
                    "Today",
                    "20:58",
                    "Deadly Boss Mods: an update stopped unexpectedly",
                    &[],
                );
                a.problem = true;
                w.activity.insert(0, a);
            }
        }
        w.addons = addons;
        w.peers = peers;
        w.recompute();
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
        self.addons
            .iter()
            .filter(|a| matches!(&a.kind, Kind::Confirmed(v) if v.contains(&p)))
            .count() as u32
    }

    /// Runs the core `derive_status` over the fake inputs and patches in per-peer confirmation
    /// (core derives confirmation from peer reports, which the prototype does not fake).
    pub fn recompute(&mut self) {
        let mut target = GroupTarget {
            group_id: GroupId::parse(&format!("{:032x}", 7)).expect("group id"),
            packages: BTreeMap::new(),
            effective: BTreeMap::new(),
            local_hash: BTreeMap::new(),
        };
        let mut interrupted = Vec::new();
        let mut needs_help = Vec::new();
        let mut applied_here = Vec::new();
        for a in &self.addons {
            let why = match &a.kind {
                Kind::Available | Kind::Blocked => Explanation::NewerVersion {
                    folder: a.title.clone(),
                    from: a.version.clone(),
                    to: a
                        .target_version
                        .clone()
                        .unwrap_or_else(|| format!("{}.1", a.version)),
                    source: cid(1),
                },
                _ => Explanation::InSync,
            };
            let pkg = match &a.kind {
                Kind::Available | Kind::Blocked => PackageTarget::Chosen {
                    version: PackageVersion {
                        package: a.key.clone(),
                        hash: Sha256([2; 32]),
                        folders: Vec::new(),
                        client: ClientInfo {
                            build_version: Some(self.build.clone()),
                            toc_mapping: "validated-plain/1".into(),
                        },
                    },
                    sources: vec![cid(1)],
                    why,
                },
                Kind::NeedsDecision(r) => PackageTarget::NeedsDecision {
                    reason: *r,
                    candidates: Vec::new(),
                    why: self.decision_explanation(a, *r),
                },
                Kind::Unavailable => PackageTarget::Unavailable {
                    why: Explanation::NotAvailableYet {
                        missing: vec![Sha256([3; 32])],
                    },
                },
                Kind::NotSynced => PackageTarget::NotSynced {
                    why: Explanation::Unsupported {
                        folder: a.title.clone(),
                        why: "game-version-specific setup".into(),
                    },
                },
                _ => PackageTarget::InSync,
            };
            target.packages.insert(a.key.clone(), pkg);
            if pkg_effective(&a.kind) {
                target.effective.insert(
                    a.key.clone(),
                    EffectiveTarget {
                        hash: Sha256([1; 32]),
                        folders: Vec::new(),
                    },
                );
            }
            match &a.kind {
                Kind::Interrupted => interrupted.push(a.key.clone()),
                Kind::NeedsHelp => needs_help.push((a.key.clone(), "unexpected state".to_string())),
                Kind::AppliedHere => applied_here.push((
                    a.key.clone(),
                    Explanation::NewerVersion {
                        folder: a.title.clone(),
                        from: a.version.clone(),
                        to: a
                            .target_version
                            .clone()
                            .unwrap_or_else(|| a.version.clone()),
                        source: cid(1),
                    },
                )),
                _ => {}
            }
        }
        let mut blocks = Vec::new();
        if self.game_running {
            blocks.push(PlanBlock::GameRunning);
        }
        if self.unstable_block {
            blocks.push(PlanBlock::Unstable);
        }
        let seen = BTreeMap::new();
        let mut view = derive_status(&StatusInput {
            target: &target,
            members: &[],
            applied_here: &applied_here,
            interrupted: &interrupted,
            interrupted_by_game: if self.game_running { &interrupted } else { &[] },
            needs_help: &needs_help,
            blocks: &blocks,
            retained: &[],
            provider_error: self.provider_error.is_some(),
            working: matches!(self.busy, Some(Busy::Syncing { .. })),
            last_publish_unix_ms: self.last_shared_secs.map(|s| NOW_MINUTES * 60 - s),
            seen_locally: &seen,
        });
        for (i, a) in self.addons.iter().enumerate() {
            if let Kind::Confirmed(ps) = &a.kind {
                if let Some(slot) = view.packages.iter_mut().find(|(k, _, _)| *k == a.key) {
                    let ids: Vec<ComputerId> = ps.iter().map(|p| cid(*p as u32 + 1)).collect();
                    slot.1 = PackageStatus::ConfirmedBy(ids.clone());
                    slot.2 = Explanation::ConfirmedBy { computers: ids };
                }
            }
            let _ = i;
        }
        view.members = self
            .peers
            .iter()
            .enumerate()
            .map(|(i, p)| wow_sync_core::status::MemberStatus {
                computer: cid(i as u32 + 1),
                display_name: p.name.clone(),
                last_seq: if p.state == MemberSummary::NoState {
                    None
                } else {
                    Some(10 + i as u64)
                },
                reported_at_peer_clock: None,
                seen_locally_at: p.report_secs_ago.map(|s| NOW_MINUTES * 60 - s),
                state: p.state,
            })
            .collect();
        view.retained_extra_recovery = self.recovery_extra;
        self.view = view;
    }

    fn decision_explanation(&self, a: &Addon, r: DecisionReason) -> Explanation {
        match r {
            DecisionReason::Ambiguous => Explanation::AmbiguousVersions {
                evidence: vec![FolderEvidence {
                    folder: a.title.clone(),
                    a_label: a.candidates.first().map(|c| c.version.clone()),
                    b_label: a.candidates.get(1).map(|c| c.version.clone()),
                    outcome: "labels cannot be compared safely".into(),
                }],
                candidates: a
                    .candidates
                    .iter()
                    .map(|c| AmbiguousCandidate {
                        label: c.version.clone(),
                        sources: Vec::new(),
                        here: c.holders.iter().any(|h| h == "This computer"),
                    })
                    .collect(),
            },
            DecisionReason::PartialCandidate => Explanation::PartialCandidate {
                missing: vec!["BagnonOptions".into()],
            },
            _ => Explanation::DifferentTocMapping {
                here: "fake-rule-A".into(),
                there: "fake-rule-B".into(),
            },
        }
    }

    pub fn addon(&self, key: &str) -> Option<&Addon> {
        self.addons.iter().find(|a| a.key.as_str() == key)
    }

    pub fn decisions_open(&self) -> usize {
        self.addons
            .iter()
            .filter(|a| matches!(a.kind, Kind::NeedsDecision(_)))
            .count()
    }

    pub fn is_paused(&self) -> bool {
        self.paused_since.is_some()
    }

    // ---- actions (pure state changes; timers live in the app) ----

    pub fn pause(&mut self) {
        self.paused_since = Some("21:10".into());
        self.activity
            .insert(0, act("Today", "21:10", "Syncing paused", &[]));
    }

    pub fn resume(&mut self) {
        self.paused_since = None;
        self.activity
            .insert(0, act("Today", "21:10", "Syncing resumed", &[]));
    }

    pub fn simulate_wow_closed(&mut self) {
        self.game_running = false;
        self.notifs.set_game_running(false);
        let open: Vec<String> = self.open_condition_keys();
        self.notifs.release(&open);
        for a in self.addons.iter_mut() {
            if matches!(a.kind, Kind::Blocked | Kind::Interrupted) {
                a.kind = Kind::Available;
                a.source = Some("Laptop".into());
            }
        }
        self.activity.insert(
            0,
            act(
                "Today",
                "21:11",
                "WoW closed: updates can now be applied",
                &[],
            ),
        );
        self.recompute();
    }

    pub fn simulate_den_reports(&mut self) {
        let Some(den) = self.peers.iter().position(|p| p.name == "Den PC") else {
            return;
        };
        self.peers[den].report_secs_ago = Some(0);
        for a in self.addons.iter_mut() {
            if let Kind::Confirmed(v) = &mut a.kind {
                if !v.contains(&den) {
                    v.push(den);
                }
            }
        }
        self.activity.insert(
            0,
            act("Today", "21:10", "Den PC reported again after 12 days", &[]),
        );
        self.recompute();
    }

    pub fn simulate_game_patched(&mut self) {
        self.build = "fake-build-B".into();
        self.identity.build_version = Some("fake-build-B".into());
        self.activity.insert(
            0,
            act(
                "Today",
                "21:10",
                "Game build changed to fake-build-B; syncing continues",
                &[],
            ),
        );
        self.recompute();
    }

    pub fn open_condition_keys(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.decisions_open() > 0 {
            v.push("decisions".to_string());
        }
        v
    }

    /// Fake "Sync now". Re-raises the open conditions; the notification engine suppresses
    /// unchanged ones.
    pub fn sync_now(&mut self) {
        self.last_check_secs = Some(0);
        if self.decisions_open() > 0 {
            let n = self.decisions_open();
            let text = if n == 1 {
                "1 add-on needs your decision.".to_string()
            } else {
                format!("{n} add-ons need your decision.")
            };
            self.notifs.raise(NotifKind::Action, "decisions", &text);
        }
        if self.provider_error.is_some() {
            self.notifs.raise(
                NotifKind::Urgent,
                "shared-folder",
                "WoW Sync can't read the shared folder. Start Dropbox, then try again.",
            );
        }
    }

    pub fn choose_copy(&mut self, key: &str, copy: usize) {
        self.decisions
            .insert(key.to_string(), DecisionPhase::Saving);
        let _ = copy;
    }

    pub fn step(&mut self, s: Step) {
        match s {
            Step::FinishSyncing => {
                let mut n = 0;
                for a in self.addons.iter_mut() {
                    if a.kind == Kind::Available {
                        a.kind = Kind::AppliedHere;
                        a.updated_secs_ago = Some(0);
                        a.version = a
                            .target_version
                            .clone()
                            .unwrap_or_else(|| a.version.clone());
                        n += 1;
                    }
                }
                self.busy = None;
                self.last_check_secs = Some(0);
                if n > 0 {
                    self.activity.insert(
                        0,
                        act(
                            "Today",
                            "21:11",
                            &format!("Updated {n} add-ons from Laptop"),
                            &[
                                "Bagnon",
                                "Questie",
                                "Raid Cooldowns",
                                "Zone Info",
                                "Spell Queue",
                            ],
                        ),
                    );
                    self.notifs.raise(
                        NotifKind::Summary,
                        "batch-now",
                        "5 add-ons updated from Laptop: Bagnon, Questie and 3 more.",
                    );
                }
                self.recompute();
            }
            Step::ApplyDetails => {
                if let Some(a) = self.addons.iter_mut().find(|a| a.title == "Details!") {
                    a.kind = Kind::AppliedHere;
                    a.updated_secs_ago = Some(0);
                    a.version = "9.2.5".into();
                }
                self.activity.insert(
                    0,
                    act(
                        "Today",
                        "21:10",
                        "Updated 1 add-on from Laptop",
                        &["Details! 9.2.4 to 9.2.5"],
                    ),
                );
                self.notifs.raise(
                    NotifKind::Summary,
                    "batch-details",
                    "1 add-on updated from Laptop: Details!.",
                );
                self.recompute();
            }
            Step::FinishSync => {
                self.busy = None;
                self.last_check_secs = Some(0);
                self.recompute();
            }
            Step::SetupDone => {
                self.enrolled = true;
                self.setup = None;
                *self = World::finish_setup_world(self.scenario);
            }
            Step::TryAgainDone => {
                self.provider_error = None;
                self.stale_secs = None;
                self.activity.insert(
                    0,
                    act("Today", "21:11", "Shared folder is readable again", &[]),
                );
                self.recompute();
            }
            Step::UpdateChecked => {
                self.update.checking = false;
                self.update.checked_secs_ago = Some(0);
            }
            Step::DecisionSaved(key) => {
                if let Some(DecisionPhase::Saving) = self.decisions.get(&key) {
                    self.decisions.insert(key, DecisionPhase::Saved(0));
                }
            }
            Step::DecisionApplied(key) => {
                if let Some(a) = self.addons.iter_mut().find(|a| a.key.as_str() == key) {
                    a.kind = Kind::AppliedHere;
                    a.updated_secs_ago = Some(0);
                }
                self.decisions.insert(key, DecisionPhase::Done(0));
                self.activity.insert(
                    0,
                    act(
                        "Today",
                        "21:11",
                        "You chose a copy; it will be used on every computer",
                        &[],
                    ),
                );
                self.recompute();
            }
            Step::NeedsHelpResolved => {
                self.needs_help_resolved = true;
                for a in self.addons.iter_mut() {
                    if a.kind == Kind::NeedsHelp {
                        a.kind = Kind::InSync;
                    }
                }
                self.apply_scope = FakeApplyScope::PerPackage;
                self.recompute();
            }
        }
    }

    fn finish_setup_world(from: ScenarioId) -> World {
        let _ = from;
        World::new(ScenarioId::S3)
    }
}

fn pkg_effective(k: &Kind) -> bool {
    !matches!(
        k,
        Kind::NeedsDecision(_) | Kind::Unavailable | Kind::NotSynced
    )
}

fn placeholder_view() -> StatusView {
    StatusView {
        headline: Headline::Ok,
        packages: Vec::new(),
        members: Vec::new(),
        last_publish: None,
        blocks: Vec::new(),
        retained_extra_recovery: 0,
    }
}
