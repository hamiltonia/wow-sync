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
    /// "Not now" was chosen on the guided "Share add-on settings?" offer: it stays hidden until
    /// the owner opens the Settings sync page.
    pub settings_offer_dismissed: bool,
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
                "settings_offer_dismissed=1" => s.settings_offer_dismissed = true,
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, state: &LocalState) -> Result<(), String> {
        let text = format!(
            "closing_quits={}\nfirst_close_seen={}\nsettings_offer_dismissed={}\n",
            u8::from(self.closing_quits),
            u8::from(self.first_close_seen),
            u8::from(self.settings_offer_dismissed)
        );
        state
            .write_atomic(SETTINGS_FILE, text.as_bytes())
            .map_err(|e| e.to_string())
    }
}

const TEST_MODE_FILE: &str = "test-mode.txt";

/// Whether test mode is on for this computer (D-28). Off by default. When on, only install
/// roots carrying the synthetic marker validate. A file that exists but cannot be read counts as
/// on, the stricter reading.
pub fn test_mode(state: &LocalState) -> bool {
    match state.read(TEST_MODE_FILE, SMALL) {
        Ok(None) => false,
        Ok(Some(b)) => String::from_utf8_lossy(&b).trim() != "0",
        Err(_) => true,
    }
}

pub fn set_test_mode(state: &LocalState, on: bool) -> Result<(), String> {
    if on {
        state
            .write_atomic(TEST_MODE_FILE, b"1\n")
            .map_err(|e| e.to_string())
    } else {
        state.remove(TEST_MODE_FILE).map_err(|e| e.to_string())
    }
}

const TOGGLES_FILE: &str = "toggles.txt";

/// The three automatic behaviours the owner can switch off (ops.md §3.2, §4.2). Kept apart from
/// [`AppSettings`] so saving the window settings never resets them. A missing or damaged file
/// means all on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppToggles {
    /// Check for updates automatically.
    pub update_check: bool,
    /// Notify when changes were applied.
    pub notify_summaries: bool,
    /// Notify when something needs the owner (action needed, data at risk, app update).
    pub notify_needs_me: bool,
}

impl Default for AppToggles {
    fn default() -> Self {
        AppToggles {
            update_check: true,
            notify_summaries: true,
            notify_needs_me: true,
        }
    }
}

impl AppToggles {
    pub fn load(state: &LocalState) -> AppToggles {
        let mut s = AppToggles::default();
        let Ok(Some(b)) = state.read(TOGGLES_FILE, SMALL) else {
            return s;
        };
        for line in String::from_utf8_lossy(&b).lines() {
            match line.trim() {
                "update_check=0" => s.update_check = false,
                "notify_summaries=0" => s.notify_summaries = false,
                "notify_needs_me=0" => s.notify_needs_me = false,
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, state: &LocalState) -> Result<(), String> {
        let text = format!(
            "update_check={}\nnotify_summaries={}\nnotify_needs_me={}\n",
            u8::from(self.update_check),
            u8::from(self.notify_summaries),
            u8::from(self.notify_needs_me),
        );
        state
            .write_atomic(TOGGLES_FILE, text.as_bytes())
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
    /// Dismisses the refusal shown for a restore of the add-on whose first folder is `package`
    /// (`StatusSnapshot::restores` entry in state `Unavailable`). Retrying with
    /// `RestoreRemoved` replaces it as well.
    DismissRestoreIssue {
        package: wow_sync_core::paths::FolderKey,
    },
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
    /// Choose the settings master and the per-folder exceptions (settings.md §5.2). Explicit only:
    /// the whole policy is replaced, nothing elects a master. Applying it needs this
    /// computer's consent (`EnableSettings`).
    SetSettingsPolicy {
        master: Option<wow_sync_core::kinds::EnrollmentRef>,
        exceptions: Vec<wow_sync_core::kinds::PolicyRule>,
    },
    /// "Share my settings": publish this computer's captured settings of `unit` as a push. Any
    /// computer may, master or not; `checkpoint` must already be in the local settings store.
    ShareSettings {
        unit: wow_sync_core::ids::SettingsUnit,
        checkpoint: wow_sync_core::ids::Sha256,
    },
    /// "Share these now": the explicit push of the settings of each listed unit
    /// (`SettingsSnapshot::to_share`), one revision per unit that has a capture here. Becoming
    /// master shares nothing by itself; this is the only way a first copy is published for
    /// several add-ons at once. Units without a capture are skipped; refused for a retired or
    /// held computer.
    ShareSettingsNow {
        units: Vec<wow_sync_core::ids::SettingsUnit>,
    },
    /// Consent to settings sync on this computer for these accounts (settings.md §5.4). Local
    /// only; nothing of WTF is read until it is recorded.
    EnableSettings { included_accounts: Vec<String> },
    /// The consent step's ticked Share these now: the units only exist once settings sync is
    /// on, so this records the owner's choice and the next sync pushes every captured add-on
    /// that has no shared copy yet (one push each), then forgets it. Needs consent first.
    ShareSettingsWhenReady,
    /// "Pause settings" for one addon folder on this computer (settings.md §6): its units are
    /// neither published nor replaced until resumed. Local only; code is not paused.
    PauseSettings {
        folder: wow_sync_core::paths::FolderKey,
    },
    /// "Pause all settings" on this computer (settings.md §6, T-S12): no unit, including those
    /// of add-ons found later, is published or replaced until `ResumeAllSettings`. Persisted,
    /// local only; code keeps syncing. Per-folder pauses are kept as they are.
    PauseAllSettings,
    /// Lifts "Pause all settings". Never publishes; units whose file differs from the shared
    /// copy follow the normal rules (a local difference is shown, nothing is replaced unasked).
    ResumeAllSettings,
    /// Resume the settings of ``folder`` with one of the choices the resume preview offered
    /// (settings.md §6). Resume never publishes: the share choices are refused here and are the
    /// `ShareSettings` command. `StayPaused` changes nothing.
    ResumeSettings {
        folder: wow_sync_core::paths::FolderKey,
        choice: wow_sync_core::settings::ResumeOption,
    },
    /// "Use shared settings" for one unit that shows "Changed here, not shared": the next
    /// settings cycle replaces the file with the shared copy and keeps the current file as a
    /// recovery copy (settings.md §4.2). Refused when the unit has no single shared state.
    UseSharedSettings {
        unit: wow_sync_core::ids::SettingsUnit,
    },
    /// "Use <computer>'s settings": resolve competing settings choices (settings.md §4.4). The
    /// `choice` revision names every head; `shown_heads` are the heads the owner saw, and a unit
    /// whose heads changed since is refused (`Changed`) and shown again. Refused when the unit is
    /// not competing or `checkpoint` is not one of its head states.
    ChooseSettings {
        unit: wow_sync_core::ids::SettingsUnit,
        checkpoint: wow_sync_core::ids::Sha256,
        shown_heads: Vec<wow_sync_core::ids::Sha256>,
    },
    /// "Restore" an earlier settings copy of `unit` from its history (settings.md §3.3). The
    /// checkpoint must be known here or already shared.
    RestoreSettings {
        unit: wow_sync_core::ids::SettingsUnit,
        checkpoint: wow_sync_core::ids::Sha256,
    },
    /// "Undo removal" for the settings of one unit of a removed addon: restores the checkpoint the
    /// valid `removal` captured for `unit`, to that same unit only (settings.md §8). The code is
    /// restored by `RestoreRemoved`.
    RestoreRemovedSettings {
        removal: wow_sync_core::ids::Sha256,
        unit: wow_sync_core::ids::SettingsUnit,
    },
    /// "Go back to this release": install the held release `recovered` (its package hash) of
    /// `package` and reject what is installed now for the group. The request is checked against
    /// the live state now and queued; the operation runs when the game is closed and writes the
    /// rejection together with the install (rollback.md §1).
    Rollback {
        package: wow_sync_core::paths::FolderKey,
        recovered: wow_sync_core::ids::Sha256,
    },
    /// "Check for updates": works with the automatic check off (ops.md §4.2). The result is
    /// read from `StatusSnapshot::update` once the check finishes.
    CheckUpdates,
    /// Builds the diagnostics export for the preview (ops.md §5). Nothing is saved.
    ExportDiagnostics {
        options: wow_sync_core::diagnostics::ExportOptions,
    },
    /// Saves exactly the previewed `export` text to `dest` (ops.md §5.1 step 3, T-33-10); nothing
    /// is rebuilt from live state. `export` must be the text the latest `ExportDiagnostics`
    /// returned, otherwise it is refused as `Decide(Changed)` ("look again"). The shared folder
    /// and the install roots are refused.
    SavePreviewedDiagnostics {
        export: wow_sync_core::diagnostics::Export,
        dest: std::path::PathBuf,
    },
    /// Test mode on or off for this computer (D-28), persisted locally and off by default. On,
    /// only install roots carrying the synthetic marker validate: an enrolled install without it
    /// stops syncing with a visible reason (nothing is un-enrolled or changed); off resumes.
    /// Works before enrollment and while the install is blocked.
    SetTestMode(bool),
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
    /// A local choice was recorded; nothing was written to the group.
    Recorded,
    /// The command had no effect to make (for example the subject is already retired).
    NothingToDo,
    /// What `CheckUpdates` did: started a check, joined one in flight, or was debounced.
    UpdateCheck(crate::updates::Started),
    /// The export text for the preview (`ExportDiagnostics`) or the text that was saved
    /// (`SavePreviewedDiagnostics`).
    Exported(wow_sync_core::diagnostics::Export),
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
    /// The settings record could not be built.
    Settings(wow_sync_core::settings::SettingsWriteError),
    /// The checkpoint is not in this computer's settings store.
    UnknownCheckpoint(wow_sync_core::ids::Sha256),
    /// The rollback cannot be planned from what is live (unknown release, same release, nothing
    /// installed, files not in the shared folder). Nothing was queued.
    Rollback(crate::rollback_flow::RollbackRefusal),
    /// No installed addon folder has this key.
    UnknownFolder(wow_sync_core::paths::FolderKey),
    /// The unit has no single shared settings state to use (none shared yet, or competing
    /// choices that need a decision first).
    NoSharedSettings(wow_sync_core::ids::SettingsUnit),
    /// Resume never publishes: "Share mine" is the `ShareSettings` command.
    UseShareSettings,
    /// No update service is attached, so there is nothing to check.
    UpdatesUnavailable,
    /// The export was refused (private data found) or could not be saved.
    Export(crate::diag::ExportError),
    Service(String),
}

/// A newer copy of an add-on that exists only on this computer after a keep (Q-88, D-26). It
/// stays installed and unshared, and the `NewVersionAfterDecision` item stays open each cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOnlyVersion {
    pub package: wow_sync_core::paths::FolderKey,
    /// The add-on's display title.
    pub title: String,
    /// The folders the keep covers.
    pub scope: Vec<wow_sync_core::paths::FolderKey>,
    /// The version label of the local copy, when its metadata has one.
    pub version_label: Option<String>,
    /// The decision that is open for it.
    pub decision: wow_sync_core::decision::DecisionId,
}

/// The folders to hold back and the local-only versions behind them (Q-88).
pub(crate) fn withheld_local_versions(
    view: &wow_sync_core::choice::ChoiceView,
    rejects: &wow_sync_core::reject::RejectBehaviour,
    ledger: &wow_sync_core::ledger::Ledger,
    packages: &[wow_sync_core::target::PackageCopies],
    own: &wow_sync_core::ids::ComputerId,
    titles: &std::collections::BTreeMap<String, String>,
) -> (
    std::collections::BTreeSet<wow_sync_core::paths::FolderKey>,
    Vec<LocalOnlyVersion>,
) {
    let mut withheld = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for pc in packages {
        let mut raised: Vec<(Vec<wow_sync_core::paths::FolderKey>, usize)> = Vec::new();
        for nv in view.new_versions(&pc.package, &pc.copies) {
            if let Some(&first) = nv.local_only(&pc.copies, own).first() {
                raised.push((nv.scope.clone(), first));
            }
        }
        for n in rejects.newer_after_rollbacks(ledger, &pc.package, &pc.copies) {
            if ledger.dismissed().contains(&n.dismissal_key(&pc.copies)) {
                continue;
            }
            if let Some(&first) = n.local_only(&pc.copies, own).first() {
                raised.push((n.scope.clone(), first));
            }
        }
        for (scope, first) in raised {
            let Some(copy) = pc.copies.get(first) else {
                continue;
            };
            let name = copy
                .version
                .folders
                .iter()
                .find(|f| f.key == pc.package.display)
                .map_or_else(
                    || pc.package.display.as_str().to_string(),
                    |f| f.name.clone(),
                );
            rows.push(LocalOnlyVersion {
                package: pc.package.key.clone(),
                title: titles.get(&name).cloned().unwrap_or(name),
                scope: scope.clone(),
                version_label: copy
                    .version
                    .folders
                    .iter()
                    .find_map(|f| f.toc.version.clone()),
                decision: wow_sync_core::decision::DecisionId(
                    wow_sync_core::decision::DecisionScope::Folders(scope.clone()),
                ),
            });
            withheld.extend(scope);
        }
    }
    (withheld, rows)
}

/// The group target without the packages whose newer local copy must not be replaced.
pub(crate) fn target_without(
    target: &wow_sync_core::target::GroupTarget,
    withheld: &std::collections::BTreeSet<wow_sync_core::paths::FolderKey>,
) -> wow_sync_core::target::GroupTarget {
    use wow_sync_core::target::PackageTarget;
    let mut t = target.clone();
    t.packages.retain(|k, p| {
        let touches = match p {
            PackageTarget::Chosen { version, .. } => {
                version.folders.iter().any(|f| withheld.contains(&f.key))
            }
            _ => false,
        };
        !withheld.contains(k) && !touches
    });
    t
}

/// The local inventory without the withheld folders: what may be published.
pub(crate) fn inventory_without(
    inv: &wow_sync_core::inventory::Inventory,
    withheld: &std::collections::BTreeSet<wow_sync_core::paths::FolderKey>,
) -> wow_sync_core::inventory::Inventory {
    let mut i = inv.clone();
    i.folders.retain(|f| !withheld.contains(&f.key));
    i
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
            settings_offer_dismissed: true,
        };
        v.save(&s).unwrap();
        assert_eq!(AppSettings::load(&s), v);
        s.write_atomic(SETTINGS_FILE, b"closing_quits=1\nfirst_close_seen=1\n")
            .unwrap();
        assert!(!AppSettings::load(&s).settings_offer_dismissed);
    }

    #[test]
    fn toggles_default_on_persist_apart_from_the_window_settings() {
        let t = TempDir::new();
        let s = st(&t);
        let on = AppToggles::default();
        assert!(on.update_check && on.notify_summaries && on.notify_needs_me);
        assert_eq!(AppToggles::load(&s), on);
        let v = AppToggles {
            update_check: false,
            notify_summaries: false,
            notify_needs_me: true,
        };
        v.save(&s).unwrap();
        AppSettings {
            closing_quits: true,
            first_close_seen: true,
            settings_offer_dismissed: false,
        }
        .save(&s)
        .unwrap();
        assert_eq!(AppToggles::load(&s), v, "saving window settings keeps them");
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
