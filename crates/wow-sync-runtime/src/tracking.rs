//! Live progress and change recording for one pass. Cheap to call: events are throttled and
//! every call is a few field updates.

use crate::history::{
    record_change, ChangeKind, ChangeRecord, ChangeSource, IncomingSummary, Phase, ProgressEvent,
    ProgressHub,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_core::paths::FolderKey;
use wow_sync_core::plan::ApplyPlan;

/// Summarise an incoming apply from this many add-ons or this many bytes.
const SUMMARY_ADDONS: u32 = 2;
const SUMMARY_BYTES: u64 = 8 * 1024 * 1024;

fn blank(pass_id: u64, phase: Phase) -> ProgressEvent {
    ProgressEvent {
        seq: 0,
        pass_id,
        phase,
        current: None,
        items_done: 0,
        items_total: 0,
        bytes_done: 0,
        bytes_total: 0,
        source: None,
        incoming: None,
        decision_satisfied: Vec::new(),
        completed: None,
    }
}

/// Emits a Scanning event (items_done 0) on creation and Idle on drop, so every exit of a
/// pass, early returns included, ends the stream.
pub(crate) struct PassGuard {
    hub: ProgressHub,
    pass_id: u64,
}

impl PassGuard {
    pub(crate) fn start(hub: &ProgressHub, pass_id: u64) -> Self {
        hub.emit(blank(pass_id, Phase::Scanning));
        Self {
            hub: hub.clone(),
            pass_id,
        }
    }
}

impl Drop for PassGuard {
    fn drop(&mut self) {
        self.hub.emit(blank(self.pass_id, Phase::Idle));
    }
}

pub(crate) fn phase(hub: &ProgressHub, pass_id: u64, p: Phase) {
    hub.emit(blank(pass_id, p));
}

/// Polls a scan's file counter and reports it while a scan runs.
pub(crate) struct ScanTicker {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ScanTicker {
    pub(crate) fn start(
        hub: &ProgressHub,
        pass_id: u64,
        files: Arc<AtomicU64>,
        total: u64,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (flag, hub) = (stop.clone(), hub.clone());
        let handle = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                let done = files.load(Ordering::Relaxed);
                if done > 0 {
                    let mut e = blank(pass_id, Phase::Scanning);
                    e.items_done = done;
                    e.items_total = total.max(done);
                    hub.emit_throttled(e);
                }
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for ScanTicker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

struct Item {
    record: ChangeRecord,
    key: FolderKey,
    started: Option<Instant>,
    done: bool,
    /// All bytes of the package's new folders, and how many of them are counted as copied.
    bytes: u64,
    counted: u64,
}

struct Inner {
    items: BTreeMap<String, Item>,
    total: u64,
    bytes_total: u64,
    items_done: u64,
    bytes_done: u64,
}

/// One apply's progress and records. Clone it into the effects; all clones share state.
#[derive(Clone)]
pub(crate) struct ApplyProgress {
    hub: ProgressHub,
    pass_id: u64,
    state_dir: PathBuf,
    inner: Arc<Mutex<Inner>>,
}

impl ApplyProgress {
    /// `prepared` pairs each planned package with the record its apply will write.
    pub(crate) fn new(
        hub: &ProgressHub,
        pass_id: u64,
        state: &LocalState,
        plan: &ApplyPlan,
        prepared: Vec<(usize, ChangeRecord)>,
    ) -> Self {
        let mut items = BTreeMap::new();
        let mut bytes_total = 0u64;
        for (i, record) in prepared {
            let p = &plan.packages[i];
            let bytes: u64 = p
                .folders
                .iter()
                .flat_map(|f| f.tree.files.iter())
                .map(|f| f.size)
                .sum();
            bytes_total += bytes;
            items.insert(
                p.op.as_str().to_string(),
                Item {
                    record,
                    key: p.package.clone(),
                    started: None,
                    done: false,
                    bytes,
                    counted: 0,
                },
            );
        }
        let total = items.len() as u64;
        Self {
            hub: hub.clone(),
            pass_id,
            state_dir: state.path().to_path_buf(),
            inner: Arc::new(Mutex::new(Inner {
                items,
                total,
                bytes_total,
                items_done: 0,
                bytes_done: 0,
            })),
        }
    }

    fn event(
        &self,
        g: &Inner,
        current: Option<String>,
        source: Option<ChangeSource>,
    ) -> ProgressEvent {
        let removal = |i: &Item| i.record.kind == ChangeKind::Removed;
        let removing = match &current {
            Some(name) => g
                .items
                .values()
                .any(|i| i.record.name == *name && removal(i)),
            None => !g.items.is_empty() && g.items.values().all(removal),
        };
        let phase = if removing {
            Phase::Removing
        } else {
            Phase::Installing
        };
        let mut e = blank(self.pass_id, phase);
        e.current = current;
        e.source = source;
        e.items_done = g.items_done;
        e.items_total = g.total;
        e.bytes_done = g.bytes_done;
        e.bytes_total = g.bytes_total;
        e
    }

    /// The first event of the apply: what is coming, and which decisions it satisfies.
    pub(crate) fn announce(&self, decided: &BTreeSet<FolderKey>) {
        let Ok(g) = self.inner.lock() else { return };
        let mut e = self.event(&g, None, None);
        let sources: BTreeSet<String> = g
            .items
            .values()
            .map(|i| format!("{:?}", i.record.source))
            .collect();
        if g.total >= u64::from(SUMMARY_ADDONS) || g.bytes_total >= SUMMARY_BYTES {
            // `None` when the add-ons come from more than one computer.
            let source = if sources.len() == 1 {
                g.items.values().next().map(|i| i.record.source.clone())
            } else {
                None
            };
            e.incoming = Some(IncomingSummary {
                addons: g.total as u32,
                source,
                bytes: g.bytes_total,
            });
            e.source = e.incoming.as_ref().and_then(|i| i.source.clone());
        }
        e.decision_satisfied = g
            .items
            .values()
            .filter(|i| decided.contains(&i.key))
            .map(|i| i.record.name.clone())
            .collect();
        drop(g);
        self.hub.emit(e);
    }

    /// A package's folder is about to be staged. The package's first event is never dropped,
    /// so the headline names it at once.
    pub(crate) fn staging(&self, op: &str) {
        let Ok(mut g) = self.inner.lock() else { return };
        let Some(item) = g.items.get_mut(op) else {
            return;
        };
        let first = item.started.is_none();
        item.started.get_or_insert_with(Instant::now);
        // A retry from a recovery copy starts the count again.
        let retried = std::mem::take(&mut item.counted);
        let (name, source) = (item.record.name.clone(), item.record.source.clone());
        g.bytes_done = g.bytes_done.saturating_sub(retried);
        let e = self.event(&g, Some(name), Some(source));
        drop(g);
        if first {
            self.hub.emit(e);
        } else {
            self.hub.emit_throttled(e);
        }
    }

    /// `bytes` of the package's files were copied into staging.
    pub(crate) fn copied(&self, op: &str, bytes: u64) {
        let Ok(mut g) = self.inner.lock() else { return };
        let Some(item) = g.items.get_mut(op) else {
            return;
        };
        item.counted += bytes;
        let (name, source) = (item.record.name.clone(), item.record.source.clone());
        g.bytes_done += bytes;
        let e = self.event(&g, Some(name), Some(source));
        drop(g);
        self.hub.emit_throttled(e);
    }

    /// The package is verified and finalized: write its record and announce it. Idempotent.
    pub(crate) fn complete(&self, op: &str) {
        let Ok(mut g) = self.inner.lock() else { return };
        let Some(item) = g.items.get_mut(op) else {
            return;
        };
        if item.done {
            return;
        }
        item.done = true;
        let mut rec = item.record.clone();
        rec.ms = item.started.map_or(0, |s| {
            s.elapsed().as_millis().min(u128::from(u32::MAX)) as u32
        });
        // The change landed now, not when the apply was planned.
        rec.unix_ms = wow_sync_adapters::sysinfo::now_unix_ms();
        let rest = item.bytes.saturating_sub(item.counted);
        item.counted = item.bytes;
        g.bytes_done += rest;
        g.items_done += 1;
        let mut e = self.event(&g, Some(rec.name.clone()), Some(rec.source.clone()));
        drop(g);
        if let Ok(state) = LocalState::open_at(&self.state_dir) {
            record_change(&state, &rec);
        }
        e.completed = Some(rec);
        self.hub.emit(e);
    }

    /// Completes every package whose apply the outcome reports as applied.
    pub(crate) fn complete_keys<'a>(&self, applied: impl Iterator<Item = &'a FolderKey>) {
        let applied: BTreeSet<&FolderKey> = applied.collect();
        let ops: Vec<String> = match self.inner.lock() {
            Ok(g) => g
                .items
                .iter()
                .filter(|(_, i)| applied.contains(&i.key))
                .map(|(o, _)| o.clone())
                .collect(),
            Err(_) => return,
        };
        for op in ops {
            self.complete(&op);
        }
    }
}

/// Counts publishing steps against a known total.
#[derive(Clone)]
pub(crate) struct PublishProgress {
    hub: ProgressHub,
    pass_id: u64,
    total: Arc<AtomicU64>,
    done: Arc<AtomicU64>,
}

impl PublishProgress {
    pub(crate) fn new(hub: &ProgressHub, pass_id: u64) -> Self {
        Self {
            hub: hub.clone(),
            pass_id,
            total: Arc::default(),
            done: Arc::default(),
        }
    }

    pub(crate) fn set_total(&self, n: u64) {
        self.total.store(n, Ordering::Relaxed);
    }

    pub(crate) fn step(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        let mut e = blank(self.pass_id, Phase::Publishing);
        e.items_done = done;
        e.items_total = self.total.load(Ordering::Relaxed).max(done);
        self.hub.emit_throttled(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{ChangeReason, FileCounts, VersionLabel};
    use wow_sync_core::paths::validate_folder_name;

    fn item(name: &str, source: ChangeSource) -> (String, Item) {
        let record = ChangeRecord {
            unix_ms: 1,
            pass_id: 1,
            kind: ChangeKind::Applied,
            name: format!("{name} Display"),
            folders: vec![name.into()],
            source,
            old_version: VersionLabel::New,
            new_version: VersionLabel::Toc("1".into()),
            files: FileCounts::default(),
            byte_delta: 0,
            reason: ChangeReason::NewerRelease,
            paths: vec![],
            paths_more: 0,
            ms: 0,
            identity: None,
        };
        let key = validate_folder_name(name).unwrap();
        (
            format!("op-{name}"),
            Item {
                record,
                key,
                started: None,
                done: false,
                bytes: 0,
                counted: 0,
            },
        )
    }

    fn announced(items: Vec<(String, Item)>, decided: &[&str]) -> ProgressEvent {
        let hub = ProgressHub::new();
        let rx = hub.subscribe();
        let total = items.len() as u64;
        let p = ApplyProgress {
            hub,
            pass_id: 1,
            state_dir: PathBuf::new(),
            inner: Arc::new(Mutex::new(Inner {
                items: items.into_iter().collect(),
                total,
                bytes_total: 0,
                items_done: 0,
                bytes_done: 0,
            })),
        };
        let decided = decided
            .iter()
            .map(|n| validate_folder_name(n).unwrap())
            .collect();
        p.announce(&decided);
        rx.try_recv().unwrap()
    }

    fn from(c: &str) -> ChangeSource {
        ChangeSource::Computer(c.into())
    }

    #[test]
    fn a_summary_names_the_one_source_and_none_when_several_computers_send() {
        let one = announced(vec![item("Alpha", from("a")), item("Beta", from("a"))], &[]);
        assert_eq!(one.incoming.unwrap().source, Some(from("a")));
        let mixed = announced(vec![item("Alpha", from("a")), item("Beta", from("b"))], &[]);
        let inc = mixed.incoming.unwrap();
        assert_eq!(inc.addons, 2);
        assert_eq!(inc.source, None);
        assert_eq!(mixed.source, None);
        let mixed_with_self = announced(
            vec![
                item("Alpha", from("a")),
                item("Beta", ChangeSource::ThisComputer),
            ],
            &[],
        );
        assert_eq!(mixed_with_self.incoming.unwrap().source, None);
    }

    #[test]
    fn a_removal_pass_reports_removing_and_an_install_pass_installing() {
        let mut rm = item("Alpha", from("a"));
        rm.1.record.kind = ChangeKind::Removed;
        assert_eq!(announced(vec![rm], &[]).phase, Phase::Removing);
        let add = announced(vec![item("Alpha", from("a"))], &[]);
        assert_eq!(add.phase, Phase::Installing);
    }

    #[test]
    fn the_next_package_is_named_at_once_and_bytes_follow_the_copied_files() {
        let mut first = item("Alpha", from("a"));
        first.1.bytes = 100;
        let mut second = item("Beta", from("a"));
        second.1.bytes = 300;
        let (op_a, op_b) = (first.0.clone(), second.0.clone());
        let hub = ProgressHub::with_min_interval(Duration::from_secs(60));
        let rx = hub.subscribe();
        let p = ApplyProgress {
            hub,
            pass_id: 1,
            state_dir: PathBuf::new(),
            inner: Arc::new(Mutex::new(Inner {
                items: vec![first, second].into_iter().collect(),
                total: 2,
                bytes_total: 400,
                items_done: 0,
                bytes_done: 0,
            })),
        };
        p.staging(&op_a);
        p.copied(&op_a, 40);
        p.complete(&op_a);
        p.staging(&op_b);
        p.copied(&op_b, 100);
        let got: Vec<_> = rx.try_iter().collect();
        let last = got.last().unwrap();
        assert_eq!(last.current.as_deref(), Some("Beta Display"), "{got:?}");
        assert_eq!(last.bytes_done, 100, "{got:?}");
        // The unthrottled start event names Beta even though a completion just went out.
        assert!(got
            .iter()
            .any(|e| e.current.as_deref() == Some("Beta Display") && e.bytes_done == 100));
        assert!(got
            .iter()
            .any(|e| e.current.as_deref() == Some("Alpha Display") && e.bytes_done == 100));
    }

    #[test]
    fn decision_satisfied_carries_display_names_of_decided_packages_only() {
        let ev = announced(
            vec![item("Alpha", from("a")), item("Beta", from("a"))],
            &["Beta", "Gamma"],
        );
        assert_eq!(ev.decision_satisfied, vec!["Beta Display".to_string()]);
        let none = announced(vec![item("Alpha", from("a"))], &[]);
        assert!(none.decision_satisfied.is_empty());
    }
}
