//! OS notifications (ops.md §3.3). One delivery attempt per notice; failure is reported to the
//! caller, never retried here.

use std::fmt;

/// Display application id registered for toasts (see `winproc::register_app_user_model_id`).
pub const APP_ID: &str = "WoWSync.App";
pub const APP_NAME: &str = "WoW Sync";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedNotice {
    pub title: String,
    pub body: String,
    /// Data-at-risk notices ask the OS for a persistent presentation.
    pub urgent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyError(pub String);

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NotifyError {}

pub trait Notifier: Send + Sync {
    fn deliver(&self, notice: &RenderedNotice) -> Result<(), NotifyError>;
}

/// Delivers through the platform notification service.
pub struct OsNotifier {
    app_id: String,
}

impl OsNotifier {
    pub fn new() -> OsNotifier {
        OsNotifier {
            app_id: APP_ID.to_string(),
        }
    }

    pub fn with_app_id(app_id: &str) -> OsNotifier {
        OsNotifier {
            app_id: app_id.to_string(),
        }
    }
}

impl Default for OsNotifier {
    fn default() -> Self {
        OsNotifier::new()
    }
}

impl Notifier for OsNotifier {
    fn deliver(&self, notice: &RenderedNotice) -> Result<(), NotifyError> {
        let mut n = notify_rust::Notification::new();
        n.summary(&notice.title).body(&notice.body);
        #[cfg(windows)]
        n.app_id(&self.app_id);
        #[cfg(not(windows))]
        let _ = &self.app_id;
        if notice.urgent {
            n.timeout(notify_rust::Timeout::Never);
        }
        n.show().map(|_| ()).map_err(|e| NotifyError(e.to_string()))
    }
}

#[cfg(any(test, feature = "testkit"))]
pub use recording::RecordingNotifier;

#[cfg(any(test, feature = "testkit"))]
mod recording {
    use super::*;
    use std::sync::Mutex;

    /// Records notices; `set_fail(true)` makes every delivery return an error.
    #[derive(Default)]
    pub struct RecordingNotifier {
        sent: Mutex<Vec<RenderedNotice>>,
        attempts: Mutex<u32>,
        fail: Mutex<bool>,
    }

    impl RecordingNotifier {
        pub fn new() -> RecordingNotifier {
            RecordingNotifier::default()
        }

        pub fn set_fail(&self, fail: bool) {
            *self.fail.lock().unwrap() = fail;
        }

        pub fn sent(&self) -> Vec<RenderedNotice> {
            self.sent.lock().unwrap().clone()
        }

        pub fn attempts(&self) -> u32 {
            *self.attempts.lock().unwrap()
        }
    }

    impl Notifier for RecordingNotifier {
        fn deliver(&self, notice: &RenderedNotice) -> Result<(), NotifyError> {
            *self.attempts.lock().unwrap() += 1;
            if *self.fail.lock().unwrap() {
                return Err(NotifyError("delivery refused".into()));
            }
            self.sent.lock().unwrap().push(notice.clone());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-27-11 manual row: `cargo test -p wow-sync-adapters --lib manual_toast -- --ignored`.
    /// Shows one real notification under a scoped test identity and removes it afterwards.
    #[test]
    #[ignore = "manual: shows a real OS notification"]
    fn manual_toast_shows_the_app_name() {
        let id = "WoWSync.ManualToastTest";
        crate::winproc::register_app_user_model_id(id, "WoW Sync", None).unwrap();
        let n = OsNotifier::with_app_id(id);
        let r = n.deliver(&RenderedNotice {
            title: "WoW Sync".into(),
            body: "Manual notification check (synthetic).".into(),
            urgent: false,
        });
        std::thread::sleep(std::time::Duration::from_secs(3));
        crate::winproc::unregister_app_user_model_id(id).unwrap();
        r.unwrap();
    }

    #[test]
    fn recorder_keeps_order_and_counts_failed_attempts() {
        let r = RecordingNotifier::new();
        let a = RenderedNotice {
            title: "a".into(),
            body: "1".into(),
            urgent: false,
        };
        let b = RenderedNotice {
            title: "b".into(),
            body: "2".into(),
            urgent: true,
        };
        r.deliver(&a).unwrap();
        r.set_fail(true);
        assert!(r.deliver(&b).is_err());
        r.set_fail(false);
        r.deliver(&b).unwrap();
        assert_eq!(r.sent(), vec![a, b]);
        assert_eq!(r.attempts(), 3);
    }
}
