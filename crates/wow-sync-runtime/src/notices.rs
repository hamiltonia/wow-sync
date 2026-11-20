//! Notification delivery (ops.md §3): plans with `wow_sync_core::notice`, saves the ledger,
//! then delivers each notice once. A delivery failure is logged once per kind per session and
//! never retried; the ledger already records the notice as announced.

use crate::updates::ActivitySink;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::notify::{Notifier, RenderedNotice};
use wow_sync_core::notice::{
    plan_notices, GameActivity, Notice, NoticeEvent, NoticeKey, NoticeKind, NoticeLedger,
    NoticePolicy, NoticePrefs,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleOutcome {
    pub notices: Vec<Notice>,
    pub delivered: u32,
    pub failed: u32,
    /// The ledger could not be saved, so nothing was delivered this cycle.
    pub not_saved: bool,
}

pub struct NoticeRunner {
    store: Arc<LocalState>,
    notifier: Arc<dyn Notifier>,
    ledger: Mutex<NoticeLedger>,
    policy: NoticePolicy,
    reported: Mutex<BTreeSet<NoticeKind>>,
    activity: ActivitySink,
}

pub fn render(n: &Notice) -> RenderedNotice {
    RenderedNotice {
        title: n.title().to_string(),
        body: n.body(),
        urgent: n.kind == NoticeKind::DataAtRisk,
    }
}

impl NoticeRunner {
    /// An unreadable or invalid ledger starts empty.
    pub fn new(
        store: Arc<LocalState>,
        notifier: Arc<dyn Notifier>,
        policy: NoticePolicy,
        activity: ActivitySink,
    ) -> NoticeRunner {
        let ledger = store
            .read_notices()
            .ok()
            .flatten()
            .and_then(|b| NoticeLedger::decode(&b))
            .unwrap_or_default();
        NoticeRunner {
            store,
            notifier,
            ledger: Mutex::new(ledger),
            policy,
            reported: Mutex::new(BTreeSet::new()),
            activity,
        }
    }

    /// Called at the end of each sync cycle. `events` and `open_now` are described on
    /// [`NoticeEvent`] and [`plan_notices`].
    pub fn after_cycle(
        &self,
        events: &[NoticeEvent],
        open_now: &BTreeSet<NoticeKey>,
        game: GameActivity,
        prefs: NoticePrefs,
        now_unix_ms: i64,
    ) -> CycleOutcome {
        let mut ledger = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
        let (notices, next) = plan_notices(
            &ledger,
            events,
            open_now,
            game,
            prefs,
            self.policy,
            now_unix_ms,
        );
        let mut out = CycleOutcome {
            notices: Vec::new(),
            delivered: 0,
            failed: 0,
            not_saved: false,
        };
        if next == *ledger && notices.is_empty() {
            return out;
        }
        // Saved before delivery: a crash between the two loses a notification rather than
        // repeating it. A summary leaves the ledger unchanged but is still delivered.
        if next != *ledger && self.store.write_notices(&next.encode()).is_err() {
            out.not_saved = true;
            return out;
        }
        *ledger = next;
        drop(ledger);
        for n in &notices {
            match self.notifier.deliver(&render(n)) {
                Ok(()) => out.delivered += 1,
                Err(_) => {
                    out.failed += 1;
                    let first = self
                        .reported
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(n.kind);
                    if first {
                        (self.activity)("A notification could not be shown");
                    }
                }
            }
        }
        out.notices = notices;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wow_sync_adapters::notify::{NotifyError, RecordingNotifier};
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::notice::{fingerprint, NoticeScope};

    fn key(kind: NoticeKind, name: &str) -> NoticeKey {
        NoticeKey {
            kind,
            scope: NoticeScope::Addon(name.to_string()),
        }
    }

    fn event(kind: NoticeKind, name: &str) -> NoticeEvent {
        NoticeEvent {
            key: key(kind, name),
            fingerprint: fingerprint(name.as_bytes()),
            name: name.to_string(),
        }
    }

    fn open(evs: &[NoticeEvent]) -> BTreeSet<NoticeKey> {
        evs.iter()
            .filter(|e| e.key.kind != NoticeKind::Summary)
            .map(|e| e.key.clone())
            .collect()
    }

    struct Rig {
        _t: TempDir,
        store: Arc<LocalState>,
        rec: Arc<RecordingNotifier>,
        log: Arc<Mutex<Vec<String>>>,
    }

    fn rig() -> Rig {
        let t = TempDir::new();
        let store = Arc::new(LocalState::open_at(&t.path().join("state")).unwrap());
        Rig {
            _t: t,
            store,
            rec: Arc::new(RecordingNotifier::new()),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    impl Rig {
        fn runner(&self) -> NoticeRunner {
            let log = self.log.clone();
            NoticeRunner::new(
                self.store.clone(),
                self.rec.clone(),
                NoticePolicy::default(),
                Arc::new(move |l| log.lock().unwrap().push(l.to_string())),
            )
        }
    }

    const T0: i64 = 1_000_000_000;

    #[test]
    fn a_decision_is_announced_once_across_cycles_and_restarts() {
        let r = rig();
        let ev = [event(NoticeKind::ActionNeeded, "WeakAuras")];
        let run = r.runner();
        let prefs = NoticePrefs::default();
        let o = run.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0);
        assert_eq!(o.delivered, 1);
        assert_eq!(r.rec.sent().len(), 1);
        assert_eq!(
            r.rec.sent()[0].body,
            "WeakAuras needs your decision. Open WoW Sync to choose."
        );
        for i in 1..5 {
            let o = run.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0 + i);
            assert!(o.notices.is_empty());
        }
        let again = r.runner();
        let o = again.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0 + 99);
        assert!(o.notices.is_empty(), "the ledger survived a restart");
        assert_eq!(r.rec.sent().len(), 1);
    }

    #[test]
    fn the_ledger_is_saved_before_the_notice_is_delivered() {
        struct Probe {
            store: Arc<LocalState>,
            saw: Arc<AtomicUsize>,
        }
        impl Notifier for Probe {
            fn deliver(&self, _: &RenderedNotice) -> Result<(), NotifyError> {
                let saved = self.store.read_notices().unwrap();
                let ledger = NoticeLedger::decode(&saved.expect("saved before delivery")).unwrap();
                self.saw.store(ledger.open_len(), Ordering::SeqCst);
                Ok(())
            }
        }
        let r = rig();
        let saw = Arc::new(AtomicUsize::new(0));
        let run = NoticeRunner::new(
            r.store.clone(),
            Arc::new(Probe {
                store: r.store.clone(),
                saw: saw.clone(),
            }),
            NoticePolicy::default(),
            Arc::new(|_| {}),
        );
        let ev = [event(NoticeKind::ActionNeeded, "A")];
        run.after_cycle(
            &ev,
            &open(&ev),
            GameActivity::NotRunning,
            NoticePrefs::default(),
            T0,
        );
        assert_eq!(saw.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failed_delivery_is_logged_once_per_kind_and_never_retried() {
        let r = rig();
        r.rec.set_fail(true);
        let run = r.runner();
        let prefs = NoticePrefs::default();
        let ev = [event(NoticeKind::ActionNeeded, "A")];
        let o = run.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0);
        assert_eq!((o.delivered, o.failed), (0, 1));
        r.rec.set_fail(false);
        let o = run.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0 + 1);
        assert!(o.notices.is_empty());
        assert_eq!(r.rec.attempts(), 1, "no retry");
        assert!(r.rec.sent().is_empty());

        // A second failing notice of the same kind adds no second line; another kind does.
        r.rec.set_fail(true);
        let ev2 = [
            event(NoticeKind::ActionNeeded, "A"),
            event(NoticeKind::ActionNeeded, "B"),
        ];
        run.after_cycle(&ev2, &open(&ev2), GameActivity::NotRunning, prefs, T0 + 2);
        let ev3 = [
            event(NoticeKind::ActionNeeded, "A"),
            event(NoticeKind::ActionNeeded, "B"),
            event(NoticeKind::DataAtRisk, "C"),
        ];
        run.after_cycle(&ev3, &open(&ev3), GameActivity::NotRunning, prefs, T0 + 3);
        assert_eq!(
            r.log.lock().unwrap().len(),
            2,
            "one per kind: {:?}",
            r.log.lock().unwrap()
        );
    }

    #[test]
    fn nothing_is_delivered_when_the_ledger_cannot_be_saved() {
        let r = rig();
        std::fs::create_dir(r.store.path().join("notices.json")).unwrap();
        let run = r.runner();
        let ev = [event(NoticeKind::DataAtRisk, "X")];
        let o = run.after_cycle(
            &ev,
            &open(&ev),
            GameActivity::NotRunning,
            NoticePrefs::default(),
            T0,
        );
        assert!(o.not_saved);
        assert_eq!(r.rec.attempts(), 0);
    }

    #[test]
    fn a_corrupt_ledger_starts_empty() {
        let r = rig();
        r.store.write_notices(b"{ not a ledger").unwrap();
        let run = r.runner();
        let ev = [event(NoticeKind::ActionNeeded, "A")];
        let o = run.after_cycle(
            &ev,
            &open(&ev),
            GameActivity::NotRunning,
            NoticePrefs::default(),
            T0,
        );
        assert_eq!(o.delivered, 1);
    }

    #[test]
    fn while_the_game_runs_summaries_wait_and_data_risk_does_not() {
        let r = rig();
        let run = r.runner();
        let prefs = NoticePrefs::default();
        let ev = [
            event(NoticeKind::Summary, "Bagnon"),
            event(NoticeKind::DataAtRisk, "Disk"),
        ];
        let o = run.after_cycle(&ev, &open(&ev), GameActivity::Running, prefs, T0);
        assert_eq!(o.delivered, 1);
        let sent = r.rec.sent();
        assert!(sent[0].urgent, "data at risk is urgent");
        assert_eq!(sent[0].title, "WoW Sync: attention needed now");

        let still = [event(NoticeKind::DataAtRisk, "Disk")];
        let o = run.after_cycle(
            &still,
            &open(&still),
            GameActivity::NotRunning,
            prefs,
            T0 + 1,
        );
        assert_eq!(o.delivered, 1);
        let last = r.rec.sent().pop().unwrap();
        assert!(!last.urgent);
        assert_eq!(last.body, "While you played: Bagnon was updated.");
    }

    #[test]
    fn the_needs_me_switch_suppresses_delivery() {
        let r = rig();
        let run = r.runner();
        let prefs = NoticePrefs {
            summaries: true,
            needs_me: false,
        };
        let ev = [event(NoticeKind::ActionNeeded, "A")];
        let o = run.after_cycle(&ev, &open(&ev), GameActivity::NotRunning, prefs, T0);
        assert_eq!(o.delivered, 0);
        assert_eq!(r.rec.attempts(), 0);
    }
}
