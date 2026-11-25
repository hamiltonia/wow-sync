//! Change history (run-010 goal 1) and live progress events (goal 2): public types and
//! signatures. Records hold names, versions, counts and sizes only: never absolute paths,
//! profile/account/character names or file contents.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use wow_sync_adapters::localstate::LocalState;

/// Most relative paths kept per record; the rest are counted in `paths_more`.
pub const MAX_RECORD_PATHS: usize = 50;
/// Records kept on disk (oldest dropped first).
pub const MAX_RECORDS: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSource {
    ThisComputer,
    Computer(String),
}

/// Why the change happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionLabel {
    /// Not present before (a new add-on).
    New,
    /// Not present after (a removal).
    Removed,
    Toc(String),
    ContentId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileCounts {
    pub added: u32,
    pub changed: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
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

const CHANGES_FILE: &str = "changes.jsonl";
/// The file is rewritten to its newest [`MAX_RECORDS`] lines once it grows past this.
const TRIM_BYTES: u64 = 4 * 1024 * 1024;

/// Appends a record to the bounded, crash-safe store in the state dir: one JSON line, synced.
/// A line torn by a crash is skipped when reading and never joined to the next one.
pub fn record_change(state: &LocalState, record: &ChangeRecord) {
    let Ok(mut line) = serde_json::to_string(record) else {
        return;
    };
    line.push('\n');
    let path = state.path().join(CHANGES_FILE);
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(&path)
    else {
        return;
    };
    let size = f.metadata().map(|m| m.len()).unwrap_or(0);
    if size > 0 {
        let mut last = [0u8; 1];
        let torn = f.seek(SeekFrom::End(-1)).is_ok()
            && f.read_exact(&mut last).is_ok()
            && last[0] != b'\n';
        if torn {
            line.insert(0, '\n');
        }
    }
    if f.write_all(line.as_bytes()).is_err() || f.sync_all().is_err() {
        return;
    }
    if size + line.len() as u64 > TRIM_BYTES {
        trim(state);
    }
}

fn trim(state: &LocalState) {
    let Ok(text) = std::fs::read(state.path().join(CHANGES_FILE)) else {
        return;
    };
    let text = String::from_utf8_lossy(&text);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| serde_json::from_str::<ChangeRecord>(l).is_ok())
        .collect();
    let keep = &lines[lines.len().saturating_sub(MAX_RECORDS)..];
    let mut out = keep.join("\n");
    out.push('\n');
    let _ = state.write_atomic(CHANGES_FILE, out.as_bytes());
}

/// Oldest first, at most [`MAX_RECORDS`]; unreadable lines are skipped.
fn read_all(state: &LocalState) -> Vec<ChangeRecord> {
    let Ok(bytes) = std::fs::read(state.path().join(CHANGES_FILE)) else {
        return Vec::new();
    };
    let mut all: Vec<ChangeRecord> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if all.len() > MAX_RECORDS {
        all.drain(..all.len() - MAX_RECORDS);
    }
    all
}

/// Newest first, at most `limit`.
pub fn recent_changes(state: &LocalState, limit: usize) -> Vec<ChangeRecord> {
    let mut all = read_all(state);
    all.reverse();
    all.truncate(limit);
    all
}

/// Newest first, at most `limit`, for one add-on (display name or any of its folders, case
/// insensitive).
pub fn changes_for_package(state: &LocalState, package: &str, limit: usize) -> Vec<ChangeRecord> {
    let mut all = read_all(state);
    all.reverse();
    all.retain(|r| {
        r.name.eq_ignore_ascii_case(package)
            || r.folders.iter().any(|f| f.eq_ignore_ascii_case(package))
    });
    all.truncate(limit);
    all
}

fn group(records: Vec<ChangeRecord>, limit: usize) -> Vec<PassGroup> {
    let mut groups: Vec<PassGroup> = Vec::new();
    for r in records {
        match groups.iter_mut().find(|g| g.pass_id == r.pass_id) {
            Some(g) => {
                g.byte_delta += r.byte_delta;
                g.unix_ms = g.unix_ms.max(r.unix_ms);
                g.records.push(r);
            }
            None => {
                if groups.len() == limit {
                    continue;
                }
                groups.push(PassGroup {
                    pass_id: r.pass_id,
                    unix_ms: r.unix_ms,
                    byte_delta: r.byte_delta,
                    records: vec![r],
                });
            }
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.unix_ms));
    groups
}

/// Records grouped by pass, newest pass first, at most `limit` groups. Records inside a group
/// are in the order they were written.
pub fn changes_by_pass(state: &LocalState, limit: usize) -> Vec<PassGroup> {
    let mut all = read_all(state);
    all.reverse();
    let mut groups = group(all, limit);
    for g in &mut groups {
        g.records.reverse();
    }
    groups
}

/// Structured records and older `audit.jsonl` lines together, newest first. Legacy lines
/// without a timestamp are the oldest and come last.
pub fn history(state: &LocalState, limit: usize) -> Vec<HistoryItem> {
    let records = recent_changes(state, limit);
    let covered = |e: &crate::control::ActivityEntry| {
        let Some(t) = e.unix_ms else { return false };
        match (e.kind.as_str(), e.text.as_str()) {
            ("applied", "applied") => records.iter().any(|r| {
                (r.unix_ms - t).abs() <= 60_000
                    && e.package.as_deref().is_some_and(|p| {
                        r.folders.iter().any(|f| f.eq_ignore_ascii_case(p))
                            || r.name.eq_ignore_ascii_case(p)
                    })
            }),
            // A publish that shared nothing new is not a change.
            ("published", t) => t.ends_with("(0 new objects)"),
            _ => false,
        }
    };
    let mut items: Vec<(Option<i64>, HistoryItem)> = records
        .iter()
        .cloned()
        .map(|r| (Some(r.unix_ms), HistoryItem::Change(r)))
        .collect();
    items.extend(
        crate::control::read_activity(state, limit)
            .into_iter()
            .filter(|e| !covered(e))
            .map(|e| (e.unix_ms, HistoryItem::Legacy(e))),
    );
    // Stable: equal times keep the structured record before the legacy line.
    items.sort_by_key(|i| std::cmp::Reverse(i.0));
    items.truncate(limit);
    items.into_iter().map(|(_, i)| i).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Scanning,
    Deciding,
    Staging,
    Installing,
    /// Used instead of `Installing` while a package is being removed.
    Removing,
    Publishing,
    Idle,
}

/// What is about to arrive, shown before a large apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingSummary {
    pub addons: u32,
    /// `None` when the incoming add-ons come from more than one computer.
    pub source: Option<ChangeSource>,
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

/// Smallest gap between two throttled events: about two per second.
const MIN_INTERVAL: Duration = Duration::from_millis(500);

/// Cloneable progress stream. The service emits; the app subscribes. Each pass emits a first
/// event with phase Scanning and items_done 0, and ends with one of phase Idle. Emission never
/// blocks: subscribers whose receiver was dropped are removed.
#[derive(Clone)]
pub struct ProgressHub {
    inner: Arc<Mutex<HubInner>>,
}

impl Default for ProgressHub {
    fn default() -> Self {
        ProgressHub::new()
    }
}

struct HubInner {
    subscribers: Vec<Sender<ProgressEvent>>,
    latest: Option<ProgressEvent>,
    seq: u64,
    last_emit: Option<Instant>,
    min_interval: Duration,
}

impl ProgressHub {
    pub fn new() -> ProgressHub {
        ProgressHub::with_min_interval(MIN_INTERVAL)
    }

    /// A hub whose throttled events are at least `min_interval` apart.
    pub fn with_min_interval(min_interval: Duration) -> ProgressHub {
        ProgressHub {
            inner: Arc::new(Mutex::new(HubInner {
                subscribers: Vec::new(),
                latest: None,
                seq: 0,
                last_emit: None,
                min_interval,
            })),
        }
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

    /// Delivers the event now. Its `seq` is replaced by the next number.
    pub fn emit(&self, event: ProgressEvent) {
        if let Ok(mut g) = self.inner.lock() {
            Self::send(&mut g, event);
        }
    }

    /// Like [`emit`](Self::emit), but drops the event when another was delivered less than the
    /// minimum interval ago. For frequent updates inside one phase; returns whether it was sent.
    pub fn emit_throttled(&self, event: ProgressEvent) -> bool {
        let Ok(mut g) = self.inner.lock() else {
            return false;
        };
        if g.last_emit.is_some_and(|t| t.elapsed() < g.min_interval) {
            return false;
        }
        Self::send(&mut g, event);
        true
    }

    fn send(g: &mut HubInner, mut event: ProgressEvent) {
        g.seq += 1;
        event.seq = g.seq;
        g.last_emit = Some(Instant::now());
        g.subscribers.retain(|s| s.send(event.clone()).is_ok());
        g.latest = Some(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str, ms: i64) -> ChangeRecord {
        ChangeRecord {
            unix_ms: ms,
            pass_id: 1,
            kind: ChangeKind::Applied,
            name: name.into(),
            folders: vec![name.into()],
            source: ChangeSource::ThisComputer,
            old_version: VersionLabel::New,
            new_version: VersionLabel::Toc("1".into()),
            files: FileCounts::default(),
            byte_delta: 0,
            reason: ChangeReason::NewerRelease,
            paths: vec![],
            paths_more: 0,
            ms: 0,
        }
    }

    fn ev(phase: Phase) -> ProgressEvent {
        ProgressEvent {
            seq: 0,
            pass_id: 1,
            phase,
            current: None,
            items_done: 0,
            items_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            source: None,
            incoming: None,
            decision_satisfied: vec![],
            completed: None,
        }
    }

    #[test]
    fn throttled_events_are_dropped_inside_the_interval_but_forced_ones_are_not() {
        let hub = ProgressHub::with_min_interval(Duration::from_secs(60));
        let rx = hub.subscribe();
        hub.emit(ev(Phase::Scanning));
        assert!(!hub.emit_throttled(ev(Phase::Installing)));
        hub.emit(ev(Phase::Idle));
        let got: Vec<_> = rx.try_iter().map(|e| (e.seq, e.phase)).collect();
        assert_eq!(got, vec![(1, Phase::Scanning), (2, Phase::Idle)]);
    }

    #[test]
    fn a_torn_last_line_is_skipped_and_the_next_record_is_not_joined_to_it() {
        let dir = wow_sync_adapters::testkit::TempDir::new();
        let st = LocalState::open_at(dir.path()).unwrap();
        record_change(&st, &rec("one", 1));
        let p = st.path().join(CHANGES_FILE);
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"{\"unix_ms\":2,\"pass").unwrap();
        drop(f);
        record_change(&st, &rec("three", 3));
        let names: Vec<_> = recent_changes(&st, 10)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, vec!["three", "one"]);
    }

    #[test]
    fn a_record_without_a_duration_still_reads() {
        let dir = wow_sync_adapters::testkit::TempDir::new();
        let st = LocalState::open_at(dir.path()).unwrap();
        let mut v = serde_json::to_value(rec("old", 1)).unwrap();
        v.as_object_mut().unwrap().remove("ms");
        std::fs::write(st.path().join(CHANGES_FILE), format!("{v}\n")).unwrap();
        assert_eq!(recent_changes(&st, 10)[0].ms, 0);
    }
}
