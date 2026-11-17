//! Small durable and in-memory controls the GUI needs: computer-wide pause (UG-2), the
//! quit-when-safe gate (UG-17), app settings (UG-18/23 deferred) and the activity log (UG-7).
//! None of this touches the game folder or the shared folder.

use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::sysinfo::now_unix_ms;

const PAUSE_FILE: &str = "paused.txt";
const SETTINGS_FILE: &str = "app-settings.txt";
const SMALL: u64 = 16 * 1024;
const AUDIT_READ: u64 = 2 * 1024 * 1024;

/// When syncing was paused on this computer (unix ms), or `None` when it is running.
/// A pause lasts until the user resumes; it survives restarts. An unreadable file reads as
/// paused so a damaged pause is never silently dropped.
pub fn paused_since(state: &LocalState) -> Option<i64> {
    match state.read(PAUSE_FILE, SMALL) {
        Ok(None) => None,
        Ok(Some(b)) => Some(
            String::from_utf8_lossy(&b)
                .trim()
                .parse::<i64>()
                .unwrap_or(0),
        ),
        Err(_) => Some(0),
    }
}

pub fn set_paused(state: &LocalState, paused: bool) -> Result<(), String> {
    if paused {
        if paused_since(state).is_some() {
            return Ok(());
        }
        state
            .write_atomic(PAUSE_FILE, now_unix_ms().to_string().as_bytes())
            .map_err(|e| e.to_string())
    } else {
        let path = state.path().join(PAUSE_FILE);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppSettings {
    /// Closing the window quits WoW Sync instead of hiding it to the tray.
    pub closing_quits: bool,
    /// The one-time first-close sheet has been shown.
    pub first_close_seen: bool,
}

impl AppSettings {
    pub fn load(state: &LocalState) -> AppSettings {
        let mut s = AppSettings::default();
        let Ok(Some(b)) = state.read(SETTINGS_FILE, SMALL) else {
            return s;
        };
        for line in String::from_utf8_lossy(&b).lines() {
            match line.trim() {
                "closing_quits=1" => s.closing_quits = true,
                "first_close_seen=1" => s.first_close_seen = true,
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, state: &LocalState) -> Result<(), String> {
        let text = format!(
            "closing_quits={}\nfirst_close_seen={}\n",
            u8::from(self.closing_quits),
            u8::from(self.first_close_seen)
        );
        state
            .write_atomic(SETTINGS_FILE, text.as_bytes())
            .map_err(|e| e.to_string())
    }
}

/// What a quit request does right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopState {
    /// Nothing is being changed: quit immediately.
    Now,
    /// A step is in progress: quit at the next safe point, after it finishes.
    AfterCurrentStep,
}

/// Quit-when-safe. A step is one sync cycle (it may change game files); between steps is
/// always a safe point.
#[derive(Debug, Default)]
pub struct QuitGate {
    requested: bool,
    in_step: bool,
}

impl QuitGate {
    pub fn request(&mut self) -> StopState {
        self.requested = true;
        if self.in_step {
            StopState::AfterCurrentStep
        } else {
            StopState::Now
        }
    }

    /// The user chose "Keep running".
    pub fn cancel(&mut self) {
        self.requested = false;
    }

    pub fn requested(&self) -> bool {
        self.requested
    }

    /// False when a quit is pending: no new step may start.
    pub fn begin_step(&mut self) -> bool {
        if self.requested {
            return false;
        }
        self.in_step = true;
        true
    }

    /// True when the step that just ended was the last one: quit now.
    pub fn end_step(&mut self) -> bool {
        self.in_step = false;
        self.requested
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    /// `None` for lines written before timestamps were recorded.
    pub unix_ms: Option<i64>,
    pub kind: String,
    pub package: Option<String>,
    pub text: String,
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub fn log_activity(state: &LocalState, kind: &str, package: Option<&str>, text: &str) {
    let line = format!(
        "{}\t{}\t{}\t{}",
        now_unix_ms(),
        clean(kind),
        clean(package.unwrap_or("")),
        clean(text)
    );
    let _ = state.append_audit(&line);
}

fn parse_line(line: &str) -> Option<ActivityEntry> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.splitn(4, '\t').collect();
    if let [ts, kind, pkg, text] = parts.as_slice() {
        if let Ok(ms) = ts.parse::<i64>() {
            return Some(ActivityEntry {
                unix_ms: Some(ms),
                kind: kind.to_string(),
                package: (!pkg.is_empty()).then(|| pkg.to_string()),
                text: text.to_string(),
            });
        }
    }
    Some(ActivityEntry {
        unix_ms: None,
        kind: "note".into(),
        package: None,
        text: line.to_string(),
    })
}

/// Newest first, at most `limit` entries, from the two retained audit files.
pub fn read_activity(state: &LocalState, limit: usize) -> Vec<ActivityEntry> {
    let mut all = Vec::new();
    for name in ["audit.1.jsonl", "audit.jsonl"] {
        if let Ok(Some(b)) = state.read(name, AUDIT_READ) {
            all.extend(String::from_utf8_lossy(&b).lines().filter_map(parse_line));
        }
    }
    all.reverse();
    all.truncate(limit);
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;

    fn st(t: &TempDir) -> LocalState {
        LocalState::open_at(&t.join("state")).unwrap()
    }

    #[test]
    fn pause_persists_across_a_restart_until_resumed() {
        let t = TempDir::new();
        let a = st(&t);
        assert_eq!(paused_since(&a), None);
        set_paused(&a, true).unwrap();
        let since = paused_since(&a).expect("paused");
        // A new handle on the same directory is a restart.
        let b = st(&t);
        assert_eq!(paused_since(&b), Some(since));
        set_paused(&b, true).unwrap();
        assert_eq!(
            paused_since(&b),
            Some(since),
            "pausing twice keeps the time"
        );
        set_paused(&b, false).unwrap();
        assert_eq!(paused_since(&st(&t)), None);
    }

    #[test]
    fn a_damaged_pause_file_still_reads_as_paused() {
        let t = TempDir::new();
        let a = st(&t);
        a.write_atomic(PAUSE_FILE, b"not a number").unwrap();
        assert!(paused_since(&a).is_some());
    }

    #[test]
    fn quit_waits_for_the_safe_point() {
        let mut g = QuitGate::default();
        assert!(g.begin_step());
        assert_eq!(g.request(), StopState::AfterCurrentStep);
        assert!(!g.begin_step(), "no new step once a quit is pending");
        assert!(g.end_step(), "quit happens only when the step has ended");
    }

    #[test]
    fn quit_is_immediate_when_idle_and_cancel_keeps_running() {
        let mut g = QuitGate::default();
        assert_eq!(g.request(), StopState::Now);
        g.cancel();
        assert!(g.begin_step());
        g.request();
        g.cancel();
        assert!(
            !g.end_step(),
            "keep running means the step end does not quit"
        );
    }

    #[test]
    fn settings_default_off_and_round_trip() {
        let t = TempDir::new();
        let s = st(&t);
        assert_eq!(AppSettings::load(&s), AppSettings::default());
        assert!(!AppSettings::default().closing_quits);
        let v = AppSettings {
            closing_quits: true,
            first_close_seen: true,
        };
        v.save(&s).unwrap();
        assert_eq!(AppSettings::load(&s), v);
    }

    #[test]
    fn activity_is_newest_first_and_tolerates_old_lines() {
        let t = TempDir::new();
        let s = st(&t);
        s.append_audit("enrolled group g").unwrap();
        log_activity(&s, "applied", Some("Bagnon"), "added\tfrom\nPC");
        log_activity(&s, "published", None, "shared");
        let a = read_activity(&s, 10);
        assert_eq!(a.len(), 3);
        assert_eq!(a[0].kind, "published");
        assert_eq!(a[1].package.as_deref(), Some("Bagnon"));
        assert!(!a[1].text.contains('\n'));
        assert_eq!(a[2].unix_ms, None);
        assert_eq!(a[2].text, "enrolled group g");
        assert_eq!(read_activity(&s, 1).len(), 1);
    }
}
