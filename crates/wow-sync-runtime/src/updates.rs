//! Update checks (ops.md §4): a single-flight background worker around [`ReleaseSource`], with
//! the attempt time persisted before any request is made. A check never downloads, installs or
//! touches game or install files; it only records a result.

// Not yet reachable from `lib.rs` (R4-integrate re-exports it); drop this allowance then.
#![allow(dead_code)]

use std::sync::{Arc, Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::update_http::{FetchError, ReleaseSource};
use wow_sync_core::notice::{fingerprint, NoticeEvent, NoticeKey, NoticeKind, NoticeScope};
use wow_sync_core::update::{
    due, release_page_url, SemVer, UpdateCheckState, UpdateFailKind, UpdateResult, ValidatedTag,
    CHECKING, CHECK_INTERVAL_MS, MANUAL_DEBOUNCE_MS,
};

pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;
pub type ActivitySink = Arc<dyn Fn(&str) + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    })
}

pub fn fail_kind(e: FetchError) -> UpdateFailKind {
    match e {
        FetchError::Offline => UpdateFailKind::Offline,
        FetchError::Timeout => UpdateFailKind::Timeout,
        FetchError::Tls => UpdateFailKind::Tls,
        FetchError::Redirect => UpdateFailKind::Redirect,
        FetchError::Http(_) => UpdateFailKind::Http,
        FetchError::TooLarge => UpdateFailKind::TooLarge,
        FetchError::Other => UpdateFailKind::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Scheduled,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Started {
    /// A new request was started.
    Started,
    /// Joined the request already in flight; both callers see its result.
    Joined,
    /// A manual check within a minute of the previous one: the last result stands.
    Debounced,
    /// An automatic check is not due, or checks are off.
    NotDue,
    /// The attempt time could not be saved, so no request was sent.
    NotSaved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateView {
    pub checking: bool,
    /// About text: "Checking for updates…", or the last result's text; `None` before any result.
    pub text: Option<String>,
    pub available_tag: Option<String>,
    pub release_url: Option<String>,
    pub last_attempt_unix_ms: Option<i64>,
}

struct Shared {
    check: UpdateCheckState,
    in_flight: Option<Trigger>,
    last_manual_ms: Option<i64>,
}

struct Inner {
    shared: Mutex<Shared>,
    idle: Condvar,
    source: Arc<dyn ReleaseSource>,
    store: Arc<LocalState>,
    current: SemVer,
    clock: Clock,
    activity: ActivitySink,
}

pub struct UpdateService {
    inner: Arc<Inner>,
}

/// Clears the flight even if the worker panics.
struct FlightGuard(Arc<Inner>);

impl Drop for FlightGuard {
    fn drop(&mut self) {
        let mut s = self.0.shared.lock().unwrap_or_else(|e| e.into_inner());
        s.in_flight = None;
        self.0.idle.notify_all();
    }
}

impl UpdateService {
    pub fn new(
        store: Arc<LocalState>,
        source: Arc<dyn ReleaseSource>,
        current: SemVer,
        clock: Clock,
        activity: ActivitySink,
    ) -> UpdateService {
        let check = store
            .read_update_check()
            .ok()
            .flatten()
            .and_then(|b| UpdateCheckState::decode(&b))
            .unwrap_or_default();
        UpdateService {
            inner: Arc::new(Inner {
                shared: Mutex::new(Shared {
                    check,
                    in_flight: None,
                    last_manual_ms: None,
                }),
                idle: Condvar::new(),
                source,
                store,
                current,
                clock,
                activity,
            }),
        }
    }

    /// The automatic schedule (§4.2). `ready_for_ms`: how long the app has been ready.
    pub fn tick(&self, enabled: bool, ready_for_ms: i64) -> Started {
        let now = (self.inner.clock)();
        {
            let s = self.lock();
            if s.in_flight.is_some() {
                return Started::Joined;
            }
            if !due(
                enabled,
                ready_for_ms,
                s.check.last_attempt_unix_ms,
                now,
                CHECK_INTERVAL_MS,
            ) {
                return Started::NotDue;
            }
        }
        self.start(Trigger::Scheduled, now)
    }

    /// "Check for updates": works even with the automatic check off.
    pub fn check_now(&self) -> Started {
        self.start(Trigger::Manual, (self.inner.clock)())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.inner.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn start(&self, trigger: Trigger, now: i64) -> Started {
        let mut s = self.lock();
        if let Some(flying) = s.in_flight {
            if trigger == Trigger::Manual && flying == Trigger::Scheduled {
                s.in_flight = Some(Trigger::Manual);
                s.last_manual_ms = Some(now);
            }
            return Started::Joined;
        }
        if trigger == Trigger::Manual {
            if let Some(t) = s.last_manual_ms {
                if now >= t && now - t < MANUAL_DEBOUNCE_MS {
                    return Started::Debounced;
                }
            }
        }
        // The attempt is durable before the request: a worker that dies mid-request must not
        // be retried at once after a restart.
        let mut next = s.check.clone();
        next.last_attempt_unix_ms = Some(now);
        if self.inner.store.write_update_check(&next.encode()).is_err() {
            return Started::NotSaved;
        }
        s.check = next;
        s.in_flight = Some(trigger);
        if trigger == Trigger::Manual {
            s.last_manual_ms = Some(now);
        }
        drop(s);

        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let _guard = FlightGuard(inner.clone());
            let fetched = inner.source.fetch_latest();
            let at = (inner.clock)();
            let mut s = inner.shared.lock().unwrap_or_else(|e| e.into_inner());
            let next = match &fetched {
                Ok(bytes) => s.check.with_result(Ok(bytes), inner.current, at),
                Err(e) => s.check.with_result(Err(fail_kind(*e)), inner.current, at),
            };
            if let Some(UpdateResult::Failed { kind, .. }) = &next.last_result {
                (inner.activity)(&kind.activity_line());
            }
            let _ = inner.store.write_update_check(&next.encode());
            s.check = next;
        });
        Started::Started
    }

    /// Blocks until no check is in flight (tests and shutdown).
    pub fn wait_idle(&self) {
        let mut s = self.lock();
        while s.in_flight.is_some() {
            s = self.inner.idle.wait(s).unwrap_or_else(|e| e.into_inner());
        }
    }

    pub fn view(&self) -> UpdateView {
        let s = self.lock();
        let checking = s.in_flight.is_some();
        let (available_tag, release_url) = match &s.check.last_result {
            Some(UpdateResult::Available { tag, .. }) => (
                Some(tag.clone()),
                ValidatedTag::parse(tag).map(|t| release_page_url(&t)),
            ),
            _ => (None, None),
        };
        UpdateView {
            checking,
            text: if checking {
                Some(CHECKING.to_string())
            } else {
                s.check.last_result.as_ref().map(UpdateResult::text)
            },
            available_tag,
            release_url,
            last_attempt_unix_ms: s.check.last_attempt_unix_ms,
        }
    }

    /// The notification event for a newer release not yet notified, and the key that stays open
    /// while any release is available.
    pub fn notice_event(&self) -> (Option<NoticeEvent>, Option<NoticeKey>) {
        let s = self.lock();
        let Some(UpdateResult::Available { tag, .. }) = &s.check.last_result else {
            return (None, None);
        };
        let key = NoticeKey {
            kind: NoticeKind::AppUpdate,
            scope: NoticeScope::Update(tag.clone()),
        };
        let event = (s.check.notified_tag.as_deref() != Some(tag.as_str())).then(|| NoticeEvent {
            key: key.clone(),
            fingerprint: fingerprint(tag.as_bytes()),
            name: format!("Version {}", tag.strip_prefix('v').unwrap_or(tag)),
        });
        (event, Some(key))
    }

    /// Records that `tag` was announced, so it is not announced again.
    pub fn note_notified(&self, tag: &str) {
        let mut s = self.lock();
        if ValidatedTag::parse(tag).is_none() {
            return;
        }
        let mut next = s.check.clone();
        next.notified_tag = Some(tag.to_string());
        if self.inner.store.write_update_check(&next.encode()).is_ok() {
            s.check = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::mpsc;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_adapters::update_http::FakeReleases;

    const CUR: &str = "0.1.0";

    fn release(tag: &str) -> Vec<u8> {
        format!(r#"{{"tag_name":"{tag}","draft":false,"prerelease":false}}"#).into_bytes()
    }

    struct Rig {
        _t: TempDir,
        store: Arc<LocalState>,
        now: Arc<AtomicI64>,
        log: Arc<Mutex<Vec<String>>>,
    }

    fn rig() -> Rig {
        let t = TempDir::new();
        let store = Arc::new(LocalState::open_at(&t.path().join("state")).unwrap());
        Rig {
            _t: t,
            store,
            now: Arc::new(AtomicI64::new(1_000_000_000)),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    impl Rig {
        fn service(&self, src: Arc<FakeReleases>) -> UpdateService {
            let now = self.now.clone();
            let log = self.log.clone();
            UpdateService::new(
                self.store.clone(),
                src,
                SemVer::parse(CUR).unwrap(),
                Arc::new(move || now.load(Ordering::SeqCst)),
                Arc::new(move |l| log.lock().unwrap().push(l.to_string())),
            )
        }

        fn state(&self) -> UpdateCheckState {
            UpdateCheckState::decode(&self.store.read_update_check().unwrap().unwrap()).unwrap()
        }
    }

    #[test]
    fn scheduled_check_waits_for_readiness_the_setting_and_the_interval() {
        let r = rig();
        let src = Arc::new(FakeReleases::new(vec![
            Ok(release("v0.1.0")),
            Ok(release("v0.1.0")),
        ]));
        let s = r.service(src.clone());
        assert_eq!(s.tick(true, 59_999), Started::NotDue, "not ready");
        assert_eq!(s.tick(false, 120_000), Started::NotDue, "setting off");
        assert_eq!(src.calls(), 0);
        assert_eq!(s.tick(true, 60_000), Started::Started);
        s.wait_idle();
        assert_eq!(src.calls(), 1);
        assert_eq!(
            s.view().text.as_deref(),
            Some("You have the newest version.")
        );
        r.now.fetch_add(CHECK_INTERVAL_MS - 1, Ordering::SeqCst);
        assert_eq!(
            s.tick(true, 120_000),
            Started::NotDue,
            "inside the interval"
        );
        r.now.fetch_add(1, Ordering::SeqCst);
        assert_eq!(s.tick(true, 120_000), Started::Started);
        s.wait_idle();
        assert_eq!(src.calls(), 2);
    }

    #[test]
    fn a_newer_release_is_available_and_the_page_url_is_exact() {
        let r = rig();
        let s = r.service(Arc::new(FakeReleases::new(vec![Ok(release("v0.2.0"))])));
        assert_eq!(s.check_now(), Started::Started);
        s.wait_idle();
        let v = s.view();
        assert_eq!(v.text.as_deref(), Some("Version 0.2.0 is available"));
        assert_eq!(
            v.release_url.as_deref(),
            Some("https://github.com/hamiltonia/wow-sync/releases/tag/v0.2.0")
        );
        assert!(r.log.lock().unwrap().is_empty());
    }

    #[test]
    fn failures_show_only_the_fixed_text_and_log_one_activity_line() {
        let r = rig();
        let s = r.service(Arc::new(FakeReleases::new(vec![
            Err(FetchError::Offline),
            Ok(b"not json at all".to_vec()),
        ])));
        s.check_now();
        s.wait_idle();
        assert_eq!(
            s.view().text.as_deref(),
            Some("Could not check for updates.")
        );
        r.now.fetch_add(MANUAL_DEBOUNCE_MS, Ordering::SeqCst);
        s.check_now();
        s.wait_idle();
        assert_eq!(
            s.view().text.as_deref(),
            Some("Could not check for updates.")
        );
        assert_eq!(
            *r.log.lock().unwrap(),
            vec![
                "Update check failed: offline".to_string(),
                "Update check failed: unreadable response".to_string()
            ]
        );
        assert!(s.view().available_tag.is_none());
    }

    #[test]
    fn a_manual_check_during_a_scheduled_one_joins_it_with_one_request() {
        let r = rig();
        let src = Arc::new(FakeReleases::new(vec![Ok(release("v0.3.0"))]));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let go_rx = Mutex::new(go_rx);
        let entered_tx = Mutex::new(entered_tx);
        src.set_hook(move |_| {
            entered_tx.lock().unwrap().send(()).unwrap();
            go_rx.lock().unwrap().recv().unwrap();
        });
        let s = r.service(src.clone());
        assert_eq!(s.tick(true, 120_000), Started::Started);
        entered_rx.recv().unwrap();
        assert!(s.view().checking);
        assert_eq!(s.check_now(), Started::Joined);
        assert_eq!(
            s.tick(true, 120_000),
            Started::Joined,
            "no second scheduled flight"
        );
        assert_eq!(s.view().text.as_deref(), Some("Checking for updates…"));
        go_tx.send(()).unwrap();
        s.wait_idle();
        assert_eq!(src.calls(), 1, "one request served both");
        assert_eq!(s.view().text.as_deref(), Some("Version 0.3.0 is available"));
    }

    #[test]
    fn manual_checks_are_debounced_to_one_a_minute() {
        let r = rig();
        let src = Arc::new(FakeReleases::new(vec![
            Ok(release("v0.1.0")),
            Ok(release("v0.1.0")),
        ]));
        let s = r.service(src.clone());
        assert_eq!(s.check_now(), Started::Started);
        s.wait_idle();
        r.now.fetch_add(MANUAL_DEBOUNCE_MS - 1, Ordering::SeqCst);
        assert_eq!(s.check_now(), Started::Debounced);
        assert_eq!(src.calls(), 1);
        assert_eq!(
            s.view().text.as_deref(),
            Some("You have the newest version.")
        );
        r.now.fetch_add(1, Ordering::SeqCst);
        assert_eq!(s.check_now(), Started::Started);
        s.wait_idle();
        assert_eq!(src.calls(), 2);
    }

    #[test]
    fn the_attempt_is_saved_before_the_request_so_a_killed_worker_is_not_retried_at_once() {
        let r = rig();
        let src = Arc::new(FakeReleases::new(vec![Ok(release("v9.9.9"))]));
        let store = r.store.clone();
        let seen_before_request = Arc::new(Mutex::new(None));
        let seen = seen_before_request.clone();
        src.set_hook(move |_| {
            let saved = UpdateCheckState::decode(&store.read_update_check().unwrap().unwrap());
            *seen.lock().unwrap() = Some(saved.unwrap().last_attempt_unix_ms);
            panic!("worker killed mid-request");
        });
        let s = r.service(src.clone());
        assert_eq!(s.tick(true, 120_000), Started::Started);
        s.wait_idle();
        assert_eq!(
            *seen_before_request.lock().unwrap(),
            Some(Some(1_000_000_000))
        );
        assert_eq!(r.state().last_attempt_unix_ms, Some(1_000_000_000));
        assert!(r.state().last_result.is_none(), "no result was recorded");

        // Restart: a new service on the same files makes no immediate request.
        let again = Arc::new(FakeReleases::new(vec![Ok(release("v9.9.9"))]));
        let s2 = r.service(again.clone());
        r.now.fetch_add(60_000, Ordering::SeqCst);
        assert_eq!(s2.tick(true, 120_000), Started::NotDue);
        assert_eq!(again.calls(), 0);
        r.now.fetch_add(CHECK_INTERVAL_MS, Ordering::SeqCst);
        assert_eq!(
            s2.tick(true, 120_000),
            Started::Started,
            "retried after a full interval"
        );
        s2.wait_idle();
    }

    #[test]
    fn no_request_is_sent_when_the_attempt_cannot_be_saved() {
        let r = rig();
        std::fs::create_dir(r.store.path().join("update_check.json")).unwrap();
        let src = Arc::new(FakeReleases::new(vec![Ok(release("v9.9.9"))]));
        let s = r.service(src.clone());
        assert_eq!(s.tick(true, 120_000), Started::NotSaved);
        assert_eq!(s.check_now(), Started::NotSaved);
        assert_eq!(src.calls(), 0);
        assert!(!s.view().checking);
        assert!(s.view().text.is_none());
    }

    #[test]
    fn notice_event_is_raised_once_per_release_tag() {
        let r = rig();
        let s = r.service(Arc::new(FakeReleases::new(vec![Ok(release("v0.2.0"))])));
        assert_eq!(s.notice_event(), (None, None));
        s.check_now();
        s.wait_idle();
        let (ev, open) = s.notice_event();
        let ev = ev.unwrap();
        assert_eq!(ev.key.kind, NoticeKind::AppUpdate);
        assert_eq!(ev.key.scope, NoticeScope::Update("v0.2.0".into()));
        assert_eq!(ev.name, "Version 0.2.0");
        assert_eq!(open, Some(ev.key));
        s.note_notified("not a tag");
        assert!(s.notice_event().0.is_some(), "an invalid tag is ignored");
        s.note_notified("v0.2.0");
        let (ev, open) = s.notice_event();
        assert!(
            ev.is_none() && open.is_some(),
            "still open, not announced again"
        );
        // Survives a restart.
        let s2 = r.service(Arc::new(FakeReleases::new(vec![])));
        assert!(s2.notice_event().0.is_none());
    }

    #[test]
    fn a_check_leaves_every_file_outside_the_state_directory_untouched() {
        let r = rig();
        let install = r._t.path().join("install");
        std::fs::create_dir(&install).unwrap();
        std::fs::write(install.join("WoWSync.exe"), b"binary").unwrap();
        let list = |d: &std::path::Path| {
            let mut v: Vec<_> = std::fs::read_dir(d)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            v.sort();
            v
        };
        let before = list(&install);
        let s = r.service(Arc::new(FakeReleases::new(vec![Ok(release("v5.0.0"))])));
        s.check_now();
        s.wait_idle();
        assert_eq!(s.view().text.as_deref(), Some("Version 5.0.0 is available"));
        assert_eq!(list(&install), before);
        assert_eq!(
            std::fs::read(install.join("WoWSync.exe")).unwrap(),
            b"binary"
        );
    }

    #[test]
    fn update_code_has_no_filesystem_or_install_access() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let adapters = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("wow-sync-adapters")
            .join("src");
        let core = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("wow-sync-core")
            .join("src");
        let banned = [
            concat!("std::", "fs"),
            concat!("current", "_exe"),
            concat!("File", "::create"),
            concat!("std::", "process"),
            concat!("Command", "::new"),
        ];
        for p in [
            dir.join("updates.rs"),
            adapters.join("update_http.rs"),
            core.join("update.rs"),
        ] {
            let text = std::fs::read_to_string(&p).unwrap();
            // Only the part before the test module is the shipped code.
            let shipped = text.split("#[cfg(test)]").next().unwrap();
            for b in banned {
                assert!(!shipped.contains(b), "{} uses {b}", p.display());
            }
        }
    }
}
