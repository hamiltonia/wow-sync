//! Small durable and in-memory controls the GUI needs: computer-wide pause (UG-2), the
//! quit-when-safe gate (UG-17), app settings (UG-18/23 deferred) and the activity log (UG-7).
//! None of this touches the game folder or the shared folder.

use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::sysinfo::now_unix_ms;

const PAUSE_FILE: &str = "paused.txt";
const SETTINGS_FILE: &str = "app-settings.txt";
const SMALL: u64 = 16 * 1024;
const AUDIT_READ: u64 = 2 * 1024 * 1024;

/// The pause control as stored on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseState {
    Running,
    /// Paused since this time (unix ms). Lasts until the user resumes.
    Paused {
        since_ms: i64,
    },
    /// The pause file exists but cannot be read or understood. Syncing stays held (a damaged
    /// pause is never silently dropped) and the GUI shows it as a problem, not a time.
    Damaged,
}

impl PauseState {
    pub fn holds_sync(self) -> bool {
        !matches!(self, PauseState::Running)
    }
}

pub fn pause_state(state: &LocalState) -> PauseState {
    match state.read(PAUSE_FILE, SMALL) {
        Ok(None) => PauseState::Running,
        Ok(Some(b)) => match String::from_utf8_lossy(&b).trim().parse::<i64>() {
            Ok(ms) if ms > 0 => PauseState::Paused { since_ms: ms },
            _ => PauseState::Damaged,
        },
        Err(_) => PauseState::Damaged,
    }
}

/// When syncing was paused on this computer (unix ms), or `None` when it is running or the
/// pause file is damaged (see [`pause_state`]).
pub fn paused_since(state: &LocalState) -> Option<i64> {
    match pause_state(state) {
        PauseState::Paused { since_ms } => Some(since_ms),
        _ => None,
    }
}

pub fn set_paused(state: &LocalState, paused: bool) -> Result<(), String> {
    if paused {
        if pause_state(state).holds_sync() {
            return Ok(());
        }
        state
            .write_atomic(PAUSE_FILE, now_unix_ms().to_string().as_bytes())
            .map_err(|e| e.to_string())
    } else {
        state.remove(PAUSE_FILE).map_err(|e| e.to_string())
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

/// A journal or recovery point this version cannot decode (contracts.md §2.2, R4D-1). It is
/// listed to the user and never deleted; core keeps blocking applies while it exists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnknownJournal {
    pub file: String,
}

impl UnknownJournal {
    pub fn render(&self) -> String {
        format!(
            "An interrupted-update record ({}) was written by a different version and is kept; \
             updates stay on hold until it is resolved",
            self.file
        )
    }
}

/// Splits stored journal files into the slice journals the status view understands and the
/// files nothing here can read. `/2` journals that decode are owned by their operation's flow.
pub fn split_journals(
    files: Vec<(String, Vec<u8>)>,
) -> (
    Vec<wow_sync_core::records::ApplyJournal>,
    Vec<UnknownJournal>,
) {
    use wow_sync_core::records::{decode_journal, AnyJournal};
    let (mut known, mut unknown) = (Vec::new(), Vec::new());
    for (file, bytes) in files {
        match decode_journal(&bytes) {
            Ok(AnyJournal::V1(j)) => known.push(j),
            Ok(AnyJournal::V2(_)) => {}
            Err(_) => unknown.push(UnknownJournal { file }),
        }
    }
    (known, unknown)
}

/// Group commands the GUI sends to the service (contracts.md §9.1). Each is routed through the
/// owner's standing: a retired computer is refused, a held or paused one keeps the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// An answer to an offered decision. `shown` is the evidence token the owner saw; a changed
    /// item is refused with `DecideError::Changed` and nothing is written.
    Decide {
        item: wow_sync_core::decision::DecisionId,
        option: wow_sync_core::decision::OptionId,
        shown: wow_sync_core::decision::EvidenceToken,
    },
    /// Retire an enrolled computer (this one or another).
    Retire {
        subject: wow_sync_core::kinds::EnrollmentRef,
    },
    /// "Allow again": lift exactly these active rejections.
    AllowAgain {
        rejections: Vec<wow_sync_core::ids::Sha256>,
    },
    /// Restore a removed addon for the whole group (explicit only, Q-55): `removal` is the hash of
    /// a valid removal record. The re-add covers it, so no later cycle removes the addon again.
    RestoreRemoved { removal: wow_sync_core::ids::Sha256 },
    /// "Join again…": explicit re-enrollment of this computer after it was retired.
    ReEnroll,
    /// Keep one release of the folders in `scope` (typed folder keys, UQ-2). `copy` is the package
    /// hash of a release currently held in the group; `why` is the reason the owner was shown.
    Keep {
        scope: Vec<wow_sync_core::paths::FolderKey>,
        copy: wow_sync_core::ids::Sha256,
        why: wow_sync_core::kinds::ChoiceReason,
    },
    /// Resume following the group for `scope`. `allow_again` names inherited exclusions to lift
    /// (Q-50); one that is not inherited is refused.
    Resume {
        scope: Vec<wow_sync_core::paths::FolderKey>,
        allow_again: Vec<wow_sync_core::ids::Sha256>,
        why: wow_sync_core::kinds::ChoiceReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// A group record was written (its hash).
    Written(wow_sync_core::ids::Sha256),
    /// Valid and reserved: it is written when publishing resumes (held, paused).
    Kept,
    /// Only this computer changed.
    Local(wow_sync_core::decision::LocalResolution),
    /// Following a retirement (or a held status) this computer joined again under a new
    /// enrollment; the old enrollment stays retired.
    ReEnrolled,
    /// The command had no effect to make (for example the subject is already retired).
    NothingToDo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Decide(wow_sync_core::decision::DecideError),
    /// This computer is retired or held and cannot publish.
    NotAllowed,
    /// A named rejection is not active (never existed, or already allowed again).
    UnknownRejection(wow_sync_core::ids::Sha256),
    EmptySelection,
    /// The choice could not be written (empty scope, a folder outside it, an exclusion that is
    /// not inherited).
    Write(wow_sync_core::choice::WriteError),
    /// No release with this package hash is currently held for the scope.
    UnknownCopy(wow_sync_core::ids::Sha256),
    /// No valid removal record has this hash.
    UnknownRemoval(wow_sync_core::ids::Sha256),
    Service(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;

    #[test]
    fn an_undecodable_journal_is_listed_by_name_and_the_known_ones_are_kept() {
        // T-C06b-unknown-journal-listed (control and view text; service uses the same split)
        let (known, unknown) = split_journals(vec![
            (
                "b".repeat(32),
                b"{\"schema\":\"wowsync.apply-journal/9\"}".to_vec(),
            ),
            ("c".repeat(32), b"not json".to_vec()),
        ]);
        assert!(known.is_empty());
        let files: Vec<&str> = unknown.iter().map(|u| u.file.as_str()).collect();
        assert_eq!(files, vec!["b".repeat(32), "c".repeat(32)]);
        assert!(unknown[0].render().contains(&"b".repeat(32)));
    }

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
    fn a_damaged_pause_file_holds_sync_and_is_an_error_not_a_time() {
        let t = TempDir::new();
        let a = st(&t);
        a.write_atomic(PAUSE_FILE, b"not a number").unwrap();
        assert_eq!(pause_state(&a), PauseState::Damaged);
        assert!(pause_state(&a).holds_sync());
        assert_eq!(
            paused_since(&a),
            None,
            "never reported as paused since 1970"
        );
        a.write_atomic(PAUSE_FILE, b"0").unwrap();
        assert_eq!(pause_state(&a), PauseState::Damaged);
        set_paused(&a, true).unwrap();
        assert_eq!(
            pause_state(&a),
            PauseState::Damaged,
            "pause does not overwrite it"
        );
        set_paused(&a, false).unwrap();
        assert_eq!(
            pause_state(&a),
            PauseState::Running,
            "resume clears a damaged pause"
        );
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
