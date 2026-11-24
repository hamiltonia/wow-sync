//! Change history (run-010 goal 1) and live progress events (goal 2): public types and
//! signatures. Records hold names, versions, counts and sizes only: never absolute paths,
//! profile/account/character names or file contents.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use wow_sync_adapters::localstate::LocalState;

/// Most relative paths kept per record; the rest are counted in `paths_more`.
pub const MAX_RECORD_PATHS: usize = 50;
/// Records kept on disk (oldest dropped first).
pub const MAX_RECORDS: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Applied,
    InstalledNew,
    Removed,
    RolledBack,
    Kept,
    Published,
    SettingsApplied,
    SettingsPublished,
    RecoveryRestored,
}

impl ChangeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeKind::Applied => "applied",
            ChangeKind::InstalledNew => "installed-new",
            ChangeKind::Removed => "removed",
            ChangeKind::RolledBack => "rolled-back",
            ChangeKind::Kept => "kept",
            ChangeKind::Published => "published",
            ChangeKind::SettingsApplied => "settings-applied",
            ChangeKind::SettingsPublished => "settings-published",
            ChangeKind::RecoveryRestored => "recovery-restored",
        }
    }
}

/// Where a change came from. Friendly names only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeSource {
    ThisComputer,
    Computer(String),
}

/// Why the change happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeReason {
    NewerRelease,
    /// The owner's choice, made on the named computer (`None` = this computer).
    Choice(Option<String>),
    SettingsMaster,
    PeerPush,
    Removal,
    Rollback,
    Recovery,
}

/// A version label: the TOC `## Version` when present, else a short content id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionLabel {
    /// Not present before (a new add-on).
    New,
    /// Not present after (a removal).
    Removed,
    Toc(String),
    ContentId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileCounts {
    pub added: u32,
    pub changed: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRecord {
    pub unix_ms: i64,
    /// Identifies the pass; records with the same id are grouped together.
    pub pass_id: u64,
    pub kind: ChangeKind,
    /// Add-on display name (or "Saved settings").
    pub name: String,
    /// The add-on's folders, names only.
    pub folders: Vec<String>,
    pub source: ChangeSource,
    pub old_version: VersionLabel,
    pub new_version: VersionLabel,
    pub files: FileCounts,
    /// New size minus old size, in bytes (may be negative).
    pub byte_delta: i64,
    pub reason: ChangeReason,
    /// Changed paths relative to the add-on folder, at most [`MAX_RECORD_PATHS`].
    pub paths: Vec<String>,
    /// How many further paths were not listed.
    pub paths_more: u32,
    /// Duration of this package's apply, in milliseconds (0 for older records).
    pub ms: u32,
}

/// One pass's records, summarised for the Activity page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassGroup {
    pub pass_id: u64,
    pub unix_ms: i64,
    pub records: Vec<ChangeRecord>,
    pub byte_delta: i64,
}

/// An entry of the unified history: a structured record, or a legacy `audit.jsonl` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryItem {
    Change(ChangeRecord),
    Legacy(crate::control::ActivityEntry),
}

/// Appends a record to the bounded, crash-safe store in the state dir.
pub fn record_change(_state: &LocalState, _record: &ChangeRecord) {}

/// Newest first, at most `limit`.
pub fn recent_changes(_state: &LocalState, _limit: usize) -> Vec<ChangeRecord> {
    Vec::new()
}

/// Newest first, at most `limit`, for one add-on (display name).
pub fn changes_for_package(
    _state: &LocalState,
    _package: &str,
    _limit: usize,
) -> Vec<ChangeRecord> {
    Vec::new()
}

/// Records grouped by pass, newest pass first, at most `limit` groups.
pub fn changes_by_pass(_state: &LocalState, _limit: usize) -> Vec<PassGroup> {
    Vec::new()
}

/// Structured records and older `audit.jsonl` lines together, newest first.
pub fn history(_state: &LocalState, _limit: usize) -> Vec<HistoryItem> {
    Vec::new()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Scanning,
    Deciding,
    Staging,
    Installing,
    Publishing,
    Idle,
}

/// What is about to arrive, shown before a large apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingSummary {
    pub addons: u32,
    pub source: ChangeSource,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    /// Increases by one per emitted event.
    pub seq: u64,
    pub pass_id: u64,
    pub phase: Phase,
    /// The add-on being worked on, if any.
    pub current: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub source: Option<ChangeSource>,
    /// Set on the first event of an apply that is large enough to summarise.
    pub incoming: Option<IncomingSummary>,
    /// Packages that just started applying; the app drops their stale
    /// "Waiting for your decision" text.
    pub decision_satisfied: Vec<String>,
    /// Set on the event emitted the moment a package's record is written.
    pub completed: Option<ChangeRecord>,
}

/// Each pass emits a first event with phase Scanning and items_done 0.
/// Cloneable progress stream. The service emits; the app subscribes. Emission never blocks:
/// subscribers whose receiver was dropped are removed.
#[derive(Clone, Default)]
pub struct ProgressHub {
    inner: Arc<Mutex<HubInner>>,
}

#[derive(Default)]
struct HubInner {
    subscribers: Vec<Sender<ProgressEvent>>,
    latest: Option<ProgressEvent>,
}

impl ProgressHub {
    pub fn new() -> ProgressHub {
        ProgressHub::default()
    }

    /// Receives every event emitted after this call.
    pub fn subscribe(&self) -> Receiver<ProgressEvent> {
        let (tx, rx) = channel();
        if let Ok(mut g) = self.inner.lock() {
            g.subscribers.push(tx);
        }
        rx
    }

    /// The most recent event, for a late subscriber.
    pub fn latest(&self) -> Option<ProgressEvent> {
        self.inner.lock().ok().and_then(|g| g.latest.clone())
    }

    pub fn emit(&self, event: ProgressEvent) {
        if let Ok(mut g) = self.inner.lock() {
            g.subscribers.retain(|s| s.send(event.clone()).is_ok());
            g.latest = Some(event);
        }
    }
}
