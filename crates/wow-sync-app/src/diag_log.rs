//! The diagnostic log: one plain-text line per event, on by default, in `<state>\logs`.
//!
//! Lines are written synchronously and unbuffered (open, append, close), so the log holds no
//! memory and loses nothing in a crash. The file is rotated to `wow-sync.log.1` at 1 MiB.
//! The user-profile prefix is written as `~`, and line breaks in a message are flattened so
//! add-on folder names cannot forge lines. Callers pass folder names and reasons only: never
//! add-on or settings contents, and never account, realm or character names.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ROTATE_AT: u64 = 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn word(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

struct Sink {
    path: PathBuf,
    state: PathBuf,
    profile: Option<String>,
}

static SINK: OnceLock<Mutex<Sink>> = OnceLock::new();

/// Where the log goes: `WOWSYNC_LOG` when set, otherwise `<state>\logs\wow-sync.log`.
pub fn log_path(state: &Path, env: Option<&str>) -> PathBuf {
    match env.filter(|s| !s.is_empty()) {
        Some(p) => PathBuf::from(p),
        None => state.join("logs").join("wow-sync.log"),
    }
}

/// Starts logging to the default location for `state`. Safe to call once; later calls are
/// ignored.
pub fn init(state: &Path) {
    let path = log_path(state, std::env::var("WOWSYNC_LOG").ok().as_deref());
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let profile = std::env::var("USERPROFILE").ok().filter(|s| !s.is_empty());
    let _ = SINK.set(Mutex::new(Sink {
        path,
        state: state.to_path_buf(),
        profile,
    }));
}

/// The folder holding this instance's state and logs, once logging has started.
pub fn state_dir() -> Option<PathBuf> {
    SINK.get().map(|m| lock(m).state.clone())
}

fn lock(m: &Mutex<Sink>) -> std::sync::MutexGuard<'_, Sink> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn write(level: Level, msg: &str) {
    match SINK.get() {
        Some(m) => {
            let s = lock(m);
            append(&s.path, s.profile.as_deref(), level, msg);
        }
        None => {
            // Before `init` only an explicit override is honoured.
            if let Ok(p) = std::env::var("WOWSYNC_LOG") {
                let profile = std::env::var("USERPROFILE").ok();
                append(Path::new(&p), profile.as_deref(), level, msg);
            }
        }
    }
}

fn append(path: &Path, profile: Option<&str>, level: Level, msg: &str) {
    rotate_if_needed(path, ROTATE_AT);
    let line = format_line(unix_now(), std::process::id(), level, msg, profile);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Moves a full log to `<name>.1`, replacing the one before it.
pub fn rotate_if_needed(path: &Path, limit: u64) {
    let full = std::fs::metadata(path).is_ok_and(|m| m.len() >= limit);
    if !full {
        return;
    }
    let mut older = path.as_os_str().to_owned();
    older.push(".1");
    let older = PathBuf::from(older);
    let _ = std::fs::remove_file(&older);
    let _ = std::fs::rename(path, &older);
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `2026-11-23T16:45:43Z pid=1234 INFO message\n`
pub fn format_line(unix: u64, pid: u32, level: Level, msg: &str, profile: Option<&str>) -> String {
    let flat: String = msg
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    format!(
        "{} pid={pid} {} {}\n",
        utc(unix),
        level.word(),
        redact(&flat, profile)
    )
}

/// Replaces the user-profile folder with `~`, in either slash style and any letter case.
pub fn redact(msg: &str, profile: Option<&str>) -> String {
    let Some(p) = profile.map(|p| p.trim_end_matches(['\\', '/'])) else {
        return msg.to_string();
    };
    if p.is_empty() {
        return msg.to_string();
    }
    let fwd = p.replace('\\', "/");
    let mut out = msg.to_string();
    for needle in [p.to_string(), fwd] {
        out = replace_ci(&out, &needle, "~");
    }
    out
}

fn replace_ci(hay: &str, needle: &str, with: &str) -> String {
    let (h, n) = (hay.to_lowercase(), needle.to_lowercase());
    // Lower-casing can change byte lengths for non-ASCII text; then match exactly instead.
    if h.len() != hay.len() || n.len() != needle.len() {
        return hay.replace(needle, with);
    }
    let mut out = String::with_capacity(hay.len());
    let mut i = 0;
    while let Some(k) = h[i..].find(&n) {
        out.push_str(&hay[i..i + k]);
        out.push_str(with);
        i += k + n.len();
    }
    out.push_str(&hay[i..]);
    out
}

/// UTC `YYYY-MM-DDTHH:MM:SSZ` from Unix seconds.
pub fn utc(unix: u64) -> String {
    let (days, rem) = ((unix / 86_400) as i64, unix % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wowsync-log-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn timestamps_are_utc() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(951_782_400 + 86_399), "2000-02-29T23:59:59Z");
        assert_eq!(utc(1_795_452_343), "2026-11-23T16:45:43Z");
    }

    #[test]
    fn a_line_has_time_pid_level_and_no_line_breaks_from_the_message() {
        let l = format_line(0, 42, Level::Warn, "a\r\n2026 pid=1 INFO forged", None);
        assert_eq!(l.matches('\n').count(), 1);
        assert!(l.starts_with("1970-01-01T00:00:00Z pid=42 WARN a"));
    }

    #[test]
    fn the_profile_prefix_becomes_a_tilde_in_both_slash_styles() {
        let p = Some("C:\\Users\\Someone");
        assert_eq!(
            redact("x C:\\Users\\Someone\\AppData\\y and c:/users/someone/z", p),
            "x ~\\AppData\\y and ~/z"
        );
        assert_eq!(redact("D:\\Games", p), "D:\\Games");
        assert_eq!(redact("keep", None), "keep");
        assert_eq!(redact("keep", Some("")), "keep");
    }

    #[test]
    fn the_default_path_is_in_the_state_dir_and_the_env_overrides_it() {
        let s = Path::new("state");
        assert_eq!(log_path(s, None), s.join("logs").join("wow-sync.log"));
        assert_eq!(log_path(s, Some("")), log_path(s, None));
        assert_eq!(log_path(s, Some("own.log")), PathBuf::from("own.log"));
    }

    #[test]
    fn a_full_log_rotates_to_one_previous_file() {
        let d = temp("rot");
        let p = d.join("wow-sync.log");
        let mut old = p.as_os_str().to_owned();
        old.push(".1");
        let old = PathBuf::from(old);
        std::fs::write(&p, vec![b'a'; 100]).unwrap();
        std::fs::write(&old, b"stale").unwrap();
        rotate_if_needed(&p, 101);
        assert!(p.exists(), "under the limit stays");
        rotate_if_needed(&p, 100);
        assert!(!p.exists());
        assert_eq!(std::fs::read(&old).unwrap().len(), 100);
        append(&p, None, Level::Info, "fresh");
        assert!(std::fs::read_to_string(&p).unwrap().contains("INFO fresh"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
