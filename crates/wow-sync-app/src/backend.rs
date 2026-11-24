//! Background sync thread: owns the `SliceService`, publishes typed view data to the UI.
//!
//! Pause is persisted by the runtime (UG-2). Quit-when-safe goes through `QuitGate` (UG-17): a
//! sync cycle is one step, and the thread only reads commands between steps.

use crate::features::{ActionKind, ActionResult, GroupAction};
use crate::world::SetupData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use wow_sync_runtime::{
    ActivityEntry, AppSettings, EnrollPreview, EnrollRequest, PauseState, QuitGate,
    ReconfirmRequest, ScanCtl, ServiceConfig, SliceService, StatusSnapshot, StopState,
};

/// Which install and shared folder a computer is set up with.
#[derive(Clone, Debug)]
pub struct Target {
    pub install_root: PathBuf,
    pub variant_folder: String,
    pub shared_root: PathBuf,
}

pub enum Cmd {
    SyncNow,
    Pause,
    Resume,
    QuitWhenSafe,
    CancelQuit,
    /// Read-only: describe this installation against the shared folder (setup step 4).
    Inspect(Target),
    /// Stop the read-only check that is running, if any.
    CancelInspect,
    /// Turn test mode on or off; the target is used only before enrollment.
    SetTestMode(bool, Option<Target>),
    Enroll(Target, EnrollRequest),
    Reconfirm(ReconfirmRequest),
    SaveSettings(AppSettings),
    /// The About toggles; stored in their own file by the service.
    SaveToggles(wow_sync_runtime::AppToggles),
    /// A group command from a feature screen; its result is published for the screen to show.
    Group(GroupAction),
    /// Re-read every add-on file once, ignoring the hash cache ("Check all files").
    CheckAllFiles,
}

/// The outcome of the last group command, with a counter so a screen shows each result once.
#[derive(Clone, Debug)]
pub struct GroupResult {
    pub seq: u64,
    pub kind: ActionKind,
    pub subject: Option<String>,
    pub result: ActionResult,
}

/// Runs a group action on the service. The one place the app turns a screen's choice into a
/// runtime command.
pub fn dispatch(svc: &mut SliceService, a: &GroupAction) -> ActionResult {
    let started = Instant::now();
    let out = svc.command(a.to_command());
    let (ok, line) = group_log_line(a.kind(), &out);
    let ms = started.elapsed().as_millis();
    if ok {
        crate::app::log(&format!("{line} ({ms} ms)"));
    } else {
        crate::app::log_warn(&format!("{line} ({ms} ms)"));
    }
    ActionResult::from_outcome(out)
}

/// One log line for what a group command did: its kind, whether it was accepted or refused and
/// why, and the hash of a record it wrote. Never a path, name or file content.
pub fn group_log_line(
    kind: ActionKind,
    out: &Result<wow_sync_runtime::CommandOutcome, wow_sync_runtime::CommandError>,
) -> (bool, String) {
    use wow_sync_runtime::{CommandError as E, CommandOutcome as O};
    let short = |h: &wow_sync_core::ids::Sha256| h.to_string().chars().take(12).collect::<String>();
    match out {
        Ok(O::Written(h)) => (
            true,
            format!("{kind:?}: accepted, wrote record {}", short(h)),
        ),
        Ok(O::Kept) => (
            true,
            format!("{kind:?}: accepted, kept until publishing resumes"),
        ),
        Ok(O::Local(_) | O::Recorded) => (
            true,
            format!("{kind:?}: accepted, changed only this computer"),
        ),
        Ok(O::ReEnrolled) => (true, format!("{kind:?}: accepted, joined again")),
        Ok(O::NothingToDo) => (true, format!("{kind:?}: nothing to do")),
        Ok(O::UpdateCheck(_)) => (true, format!("{kind:?}: update check requested")),
        Ok(O::Exported(e)) => (
            true,
            format!("{kind:?}: ok, {} line(s) of text", e.text.lines().count()),
        ),
        Err(E::Decide(wow_sync_core::decision::DecideError::Changed)) => (
            false,
            format!("{kind:?}: refused, the choice was out of date and the list was refreshed"),
        ),
        Err(E::Decide(e)) => (false, format!("{kind:?}: refused, {e:?}")),
        Err(E::Export(_)) => (false, format!("{kind:?}: refused or could not be saved")),
        Err(E::Service(_)) => (
            false,
            format!("{kind:?}: the sync service could not do that"),
        ),
        Err(E::Rollback(r)) => (false, format!("{kind:?}: refused, {r:?}")),
        Err(E::Settings(e)) => (false, format!("{kind:?}: refused, {e:?}")),
        Err(E::NotAllowed) => (
            false,
            format!("{kind:?}: refused, this computer cannot publish"),
        ),
        Err(E::UpdatesUnavailable) => (false, format!("{kind:?}: updates not available")),
        Err(other) => (false, format!("{kind:?}: refused, {}", variant_name(other))),
    }
}

fn variant_name(e: &wow_sync_runtime::CommandError) -> String {
    let d = format!("{e:?}");
    d.split(['(', ' ', '{'])
        .next()
        .unwrap_or("error")
        .to_string()
}

#[derive(Default)]
pub struct Shared {
    pub version: u64,
    pub started: bool,
    pub snapshot: StatusSnapshot,
    pub preview: Option<EnrollPreview>,
    pub paused_since: Option<i64>,
    pub pause_damaged: bool,
    pub activity: Vec<ActivityEntry>,
    pub settings: AppSettings,
    pub toggles: wow_sync_runtime::AppToggles,
    pub busy: bool,
    pub last_check_ms: Option<i64>,
    /// When the scheduler will next run a check on its own; `None` while none is scheduled
    /// (paused, not enrolled, or no sync thread service).
    pub next_check_ms: Option<i64>,
    pub message: String,
    pub quit_ready: bool,
    pub setup: Option<SetupData>,
    pub enrolled_now: bool,
    /// Set when the sync thread caught a panic: the phase it was in. Cleared by a good pass.
    pub fault: Option<String>,
    pub shared_root: Option<PathBuf>,
    /// What core currently offers the owner to decide (typed; the app renders the words).
    pub decisions: Vec<wow_sync_core::decision::DecisionItem>,
    pub group_result: Option<GroupResult>,
    /// The last test-mode change: a counter so a result shows once, the mode asked for, and
    /// whether it was applied.
    pub test_mode_result: Option<(u64, bool, bool)>,
    /// The installation `preview` describes; a preview is only used for its own target.
    pub preview_for: Option<(PathBuf, String)>,
    /// The read-only check that is running now, with its progress.
    pub inspecting: Option<Inspecting>,
    /// Identifies the check whose result may still be published; a cancelled or replaced
    /// check finds a different value and publishes nothing.
    pub inspect_seq: u64,
    /// How many checks have ended (finished, failed, cancelled or replaced).
    pub inspect_ended: u64,
    /// Set by the window the moment Quit is chosen, before the sync thread can read the
    /// command: read-only scans of the install stop at the next file. Never stops a write.
    pub abort_reads: Arc<AtomicBool>,
    /// Set by the window each time it queues a command, so a sync pass that is only reading
    /// stops at the next file and the command is answered at once.
    pub command_pending: Arc<AtomicBool>,
}

impl Shared {
    /// The digest of the check made for exactly `t`, never another folder's.
    pub fn digest_for(&self, t: &Target) -> Option<wow_sync_core::ids::Sha256> {
        let (root, folder) = self.preview_for.as_ref()?;
        if *root != t.install_root || *folder != t.variant_folder {
            return None;
        }
        self.preview.as_ref().and_then(|p| p.digest)
    }
}

/// A read-only check of one installation that has not finished.
#[derive(Clone)]
pub struct Inspecting {
    pub root: PathBuf,
    pub folder: String,
    pub files: Arc<AtomicU64>,
}

pub const TEST_MODE_FAILED: &str = "WoW Sync could not change test mode, so nothing was changed. Check that its data folder can be written to, then try again.";

/// What to tell the owner after a test-mode change; never claims a state that was not saved.
pub fn test_mode_notice(on: bool, applied: bool) -> &'static str {
    match (applied, on) {
        (false, _) => TEST_MODE_FAILED,
        (true, true) => "Test mode is on",
        (true, false) => "Test mode is off",
    }
}

pub type SharedState = Arc<Mutex<Shared>>;

pub fn lock(s: &SharedState) -> MutexGuard<'_, Shared> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone)]
pub struct BackendOptions {
    pub interval: Duration,
    pub crash_after: Option<usize>,
    pub state_dir: PathBuf,
    pub display_name: String,
    pub app_version: String,
    pub recent_ms: u64,
    /// Remember the enrolled target for the next start (not in developer mode).
    pub save_target: bool,
    /// Attach the real notifier and the update source (not in developer mode).
    pub os_services: bool,
}

/// The hash cache is kept per installation: install root and variant folder.
fn cache_scope(t: &Target) -> String {
    format!("{}|{}", t.install_root.display(), t.variant_folder)
}

const CONFIG_FILE: &str = "target.txt";

pub fn load_target(state_dir: &Path) -> Option<Target> {
    let text = std::fs::read_to_string(state_dir.join(CONFIG_FILE)).ok()?;
    let get = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    Some(Target {
        install_root: PathBuf::from(get("install")?),
        variant_folder: get("variant")?,
        shared_root: PathBuf::from(get("shared")?),
    })
}

fn save_target(state_dir: &Path, t: &Target) {
    let text = format!(
        "install={}\nvariant={}\nshared={}\n",
        t.install_root.display(),
        t.variant_folder,
        t.shared_root.display()
    );
    let _ = std::fs::create_dir_all(state_dir);
    let _ = std::fs::write(state_dir.join(CONFIG_FILE), text);
}

/// Plain-language text for a refused setup or reconfirmation; never the internal reason name.
pub const LOST_ENROLLMENT: &str = "This computer's saved setup could not be loaded, so it is not syncing. Set it up again below; nothing was deleted.";

pub const ALREADY_SET_UP: &str =
    "This computer is already set up, so no second setup was made. Your settings are unchanged.";

fn humane(e: &wow_sync_runtime::ServiceError) -> String {
    let raw = match e {
        wow_sync_runtime::ServiceError::Refused(s) => s.clone(),
        _ => e.to_string(),
    };
    if raw.contains("ConfirmationRequired") {
        "Confirm both the game version and the add-on list to continue. Nothing has been changed."
            .into()
    } else if raw.contains("StaleConfirmation") {
        "The installation changed after you looked at it. Check it again and confirm. Nothing has been changed."
            .into()
    } else {
        raw
    }
}

fn config_for(o: &BackendOptions, t: &Target) -> ServiceConfig {
    ServiceConfig {
        install_root: t.install_root.clone(),
        variant_folder: t.variant_folder.clone(),
        shared_root: t.shared_root.clone(),
        state_dir: o.state_dir.clone(),
        display_name: o.display_name.clone(),
        app_version: o.app_version.clone(),
        recent_ms: o.recent_ms,
    }
}

pub fn spawn(
    target: Option<Target>,
    setup: Option<SetupData>,
    opts: BackendOptions,
    rx: Receiver<Cmd>,
    shared: SharedState,
    wake: Sender<()>,
) {
    std::thread::Builder::new()
        .name("sync".into())
        .spawn(move || run(target, setup, opts, rx, shared, wake))
        .expect("start sync thread");
}

fn publish<F: FnOnce(&mut Shared)>(shared: &SharedState, wake: &Sender<()>, f: F) {
    {
        let mut g = lock(shared);
        f(&mut g);
        g.version += 1;
    }
    let _ = wake.send(());
}

fn now_ms() -> i64 {
    crate::world::now_ms()
}

static LAST_LOGGED: Mutex<String> = Mutex::new(String::new());

/// Writes what is holding sync (plan blocks, a provider problem) once per change, so a
/// five-second check cycle does not fill the log. The lines are the plain-language sentences
/// the window shows; they carry no add-on or settings contents.
static LAST_ADDONS: Mutex<String> = Mutex::new(String::new());

/// Logs each add-on that is not synced or syncs by content, with the reason only, once per
/// change. Uses the folder key and the sentence the window shows; no file contents.
fn log_addon_notes(st: &StatusSnapshot) {
    let lines: Vec<String> = st
        .packages
        .iter()
        .filter(|p| p.text.contains("not synced") || p.text.contains("Unrecognised"))
        .map(|p| format!("add-on {}: {}", p.key, p.text.replace('\n', " ")))
        .collect();
    let sig = lines.join("\n");
    let mut last = LAST_ADDONS.lock().unwrap_or_else(|e| e.into_inner());
    if *last == sig {
        return;
    }
    let previous: Vec<String> = last.lines().map(str::to_owned).collect();
    *last = sig;
    for l in lines.iter().filter(|l| !previous.contains(l)) {
        crate::app::log_warn(l);
    }
}

fn log_status_changes(st: &StatusSnapshot) {
    log_addon_notes(st);
    let sig = format!("{:?}|{:?}", st.blocks, st.provider_error);
    let mut last = LAST_LOGGED.lock().unwrap_or_else(|e| e.into_inner());
    if *last == sig {
        return;
    }
    *last = sig;
    for b in &st.blocks {
        crate::app::log_warn(&format!("sync held: {b}"));
    }
    if let Some(e) = &st.provider_error {
        crate::app::log_error(&format!("shared folder problem: {e}"));
    }
}

/// Runs one phase so that a panic in it cannot end the sync thread: the panic is logged, the
/// phase is published as a fault the window shows, and the caller gets `None`.
fn guarded<T>(
    phase: &str,
    shared: &SharedState,
    wake: &Sender<()>,
    f: impl FnOnce() -> T,
) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(p) => {
            let why = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            crate::app::log_error(&format!("internal error while {phase}: {why}"));
            publish(shared, wake, |g| {
                g.fault = Some(phase.to_string());
                g.busy = false;
            });
            None
        }
    }
}

/// How long to wait before trying a pass again after an internal error.
fn fault_retry(state_dir: &Path) -> Duration {
    #[cfg(test)]
    {
        let ms = tests::retry_ms(state_dir);
        if ms > 0 {
            return Duration::from_millis(ms);
        }
    }
    Duration::from_secs(300)
}

fn refresh(svc: &mut SliceService, shared: &SharedState, wake: &Sender<()>, checked: bool) {
    if lock(shared).abort_reads.load(Ordering::Relaxed) {
        return;
    }
    let Some(st) = guarded("reading the add-on status", shared, wake, || svc.status()) else {
        return;
    };
    refresh_with(svc, shared, wake, checked, st);
}

fn refresh_with(
    svc: &SliceService,
    shared: &SharedState,
    wake: &Sender<()>,
    checked: bool,
    st: StatusSnapshot,
) {
    log_status_changes(&st);
    let decisions = svc.offered_decisions().to_vec();
    let (pause, act, settings) = (svc.pause_state(), svc.activity_log(300), svc.app_settings());
    let toggles = svc.toggles();
    publish(shared, wake, |g| {
        g.started = true;
        g.toggles = toggles;
        g.snapshot = st;
        g.decisions = decisions;
        g.paused_since = match pause {
            PauseState::Paused { since_ms } => Some(since_ms),
            _ => None,
        };
        g.pause_damaged = pause == PauseState::Damaged;
        g.activity = act;
        g.settings = settings;
        if checked {
            g.last_check_ms = Some(now_ms());
        }
    });
}

/// Publishes the saved pause state at once, so a click shows its effect without waiting for a
/// status read or a pass.
fn publish_pause(svc: &SliceService, shared: &SharedState, wake: &Sender<()>) {
    let pause = svc.pause_state();
    publish(shared, wake, |g| {
        g.paused_since = match pause {
            PauseState::Paused { since_ms } => Some(since_ms),
            _ => None,
        };
        g.pause_damaged = pause == PauseState::Damaged;
    });
}

type Attach = fn(&mut SliceService) -> Result<(), wow_sync_runtime::ServiceError>;

/// Opens the service and, when `attach` is given, attaches the OS notifier and update source.
/// A failed attach costs notifications and update checks only; sync still runs.
fn open_service(
    cfg: ServiceConfig,
    attach: Option<Attach>,
) -> Result<SliceService, wow_sync_runtime::ServiceError> {
    let mut s = SliceService::new(cfg)?;
    if let Some(f) = attach {
        if let Err(e) = f(&mut s) {
            crate::app::log(&format!("OS services not attached: {e}"));
        }
    }
    Ok(s)
}

fn os_attach(o: &BackendOptions) -> Option<Attach> {
    o.os_services
        .then_some(SliceService::attach_os_services as Attach)
}

/// The saved setup is gone or unreadable: say so and offer setup, instead of waiting forever.
fn report_lost(shared: &SharedState, wake: &Sender<()>, computer: &str) {
    crate::app::log_warn("saved setup could not be loaded; showing setup");
    publish(shared, wake, |g| {
        g.fault = None;
        g.enrolled_now = false;
        g.snapshot.enrolled = false;
        g.message = LOST_ENROLLMENT.into();
    });
    detect_later(computer.to_string(), shared.clone(), wake.clone(), false);
}

/// Looks for the game and Dropbox off the sync thread, once the first status has been read,
/// and only for a computer that is not set up. An enrolled computer never pays for the
/// all-drives search, so sync starts as soon as it is ready.
fn detect_later(computer: String, shared: SharedState, wake: Sender<()>, wait_for_status: bool) {
    let _ = std::thread::Builder::new()
        .name("wow-sync-detect".into())
        .spawn(move || {
            for _ in 0..if wait_for_status { 300 } else { 0 } {
                let g = lock(&shared);
                if !g.snapshot.headline.is_empty() {
                    if g.snapshot.enrolled {
                        return;
                    }
                    break;
                }
                drop(g);
                std::thread::sleep(Duration::from_millis(200));
            }
            let data = crate::detect(&computer);
            publish(&shared, &wake, |g| g.setup = Some(data));
        });
}

/// Runs the read-only check of an installation off the command loop, one at a time, so Quit,
/// Back and a new selection are never queued behind a long scan. A check that was cancelled
/// or replaced publishes nothing.
#[derive(Default)]
pub struct Inspector {
    running: Option<ScanCtl>,
    seq: u64,
}

impl Inspector {
    pub fn start(
        &mut self,
        cfg: ServiceConfig,
        t: Target,
        shared: &SharedState,
        wake: &Sender<()>,
    ) {
        if let Some(old) = self.running.take() {
            old.cancel.store(true, Ordering::Relaxed);
        }
        self.seq += 1;
        let (seq, ctl) = (
            self.seq,
            ScanCtl {
                cache: Some(wow_sync_runtime::HashCache::open(
                    &cfg.state_dir,
                    &cache_scope(&t),
                )),
                ..ScanCtl::default()
            },
        );
        self.running = Some(ctl.clone());
        publish(shared, wake, |g| {
            g.preview = None;
            g.preview_for = None;
            g.message.clear();
            g.inspect_seq = seq;
            g.inspecting = Some(Inspecting {
                root: t.install_root.clone(),
                folder: t.variant_folder.clone(),
                files: ctl.files.clone(),
            });
        });
        let (owned_shared, owned_wake) = (shared.clone(), wake.clone());
        let spawned = std::thread::Builder::new()
            .name("inspect".into())
            .spawn(move || run_inspect(cfg, t, ctl, seq, owned_shared, owned_wake));
        if let Err(e) = spawned {
            crate::app::log_error(&format!("inspect could not start: {e}"));
            self.running = None;
            publish(shared, wake, |g| {
                g.inspecting = None;
                g.message = format!("The check could not start: {e}");
            });
        }
    }

    pub fn cancel(&mut self, shared: &SharedState, wake: &Sender<()>) {
        if let Some(ctl) = self.running.take() {
            ctl.cancel.store(true, Ordering::Relaxed);
            publish(shared, wake, |g| {
                g.inspect_seq = 0;
                g.inspecting = None;
            });
        }
    }
}

fn inspect_once(
    cfg: ServiceConfig,
    ctl: &ScanCtl,
) -> Result<Option<EnrollPreview>, wow_sync_runtime::ServiceError> {
    #[cfg(test)]
    if cfg.variant_folder == "_hold_" {
        while !ctl.cancelled() {
            std::thread::sleep(Duration::from_millis(5));
        }
        return Ok(None);
    }
    #[cfg(test)]
    if cfg.variant_folder == "_late_" {
        // A check that ends with a result after it was cancelled or replaced.
        while !ctl.cancelled() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let cfg = ServiceConfig {
            variant_folder: "_retail_".into(),
            ..cfg
        };
        return Ok(Some(SliceService::new(cfg)?.inspect()));
    }
    #[cfg(test)]
    if cfg.variant_folder == "_panic_" {
        panic!("test: the check fails unexpectedly");
    }
    Ok(SliceService::new(cfg)?.inspect_ctl(ctl))
}

/// Counts a check as over, however it ended.
struct Ended(SharedState);

impl Drop for Ended {
    fn drop(&mut self) {
        lock(&self.0).inspect_ended += 1;
    }
}

/// Stops the progress ticker however the check ends, including by a panic.
struct StopTicker(Arc<AtomicBool>);

impl Drop for StopTicker {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

fn run_inspect(
    cfg: ServiceConfig,
    t: Target,
    ctl: ScanCtl,
    seq: u64,
    shared: SharedState,
    wake: Sender<()>,
) {
    let place = t.install_root.join(&t.variant_folder);
    let place = place.display();
    let _ended = Ended(shared.clone());
    crate::app::log(&format!("inspect start {place}"));
    let started = Instant::now();
    let ticking = Arc::new(AtomicBool::new(true));
    let _stop = StopTicker(ticking.clone());
    {
        let (ticking, shared, wake) = (ticking.clone(), shared.clone(), wake.clone());
        let _ = std::thread::Builder::new()
            .name("inspect-progress".into())
            .spawn(move || {
                while ticking.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(250));
                    if !ticking.load(Ordering::Relaxed) {
                        break;
                    }
                    publish(&shared, &wake, |_| {});
                }
            });
    }
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inspect_once(cfg, &ctl)))
        .unwrap_or_else(|_| {
            Err(wow_sync_runtime::ServiceError::Local(
                "the check stopped unexpectedly".into(),
            ))
        });
    ticking.store(false, Ordering::Relaxed);
    let ms = started.elapsed().as_millis();
    let files = ctl.files.load(Ordering::Relaxed);
    let result = match res {
        Ok(Some(p)) => Ok(p),
        Ok(None) => {
            crate::app::log(&format!("inspect cancelled {place}"));
            return;
        }
        Err(e) => Err(e),
    };
    let mut line = None;
    publish(&shared, &wake, |g| {
        if g.inspect_seq != seq {
            return;
        }
        g.inspecting = None;
        match result {
            Ok(p) => {
                line = Some(format!(
                    "inspect done {place} in {ms} ms: {} add-on(s) eligible, {} blocked, {files} file(s) read",
                    p.eligible.len(),
                    p.blocked.len()
                ));
                g.preview = Some(p);
                g.preview_for = Some((t.install_root.clone(), t.variant_folder.clone()));
                g.message.clear();
            }
            Err(e) => {
                crate::app::log_error(&format!("inspect failed {place} in {ms} ms: {e}"));
                g.preview = None;
                g.preview_for = None;
                g.message = e.to_string();
            }
        }
    });
    match line {
        Some(l) => crate::app::log(&l),
        None => {
            if lock(&shared).inspect_seq != seq {
                crate::app::log(&format!("inspect cancelled {place}"));
            }
        }
    }
}

fn run(
    target: Option<Target>,
    setup: Option<SetupData>,
    opts: BackendOptions,
    rx: Receiver<Cmd>,
    shared: SharedState,
    wake: Sender<()>,
) {
    let abort = lock(&shared).abort_reads.clone();
    let interrupt = wow_sync_runtime::Interrupt {
        armed: Arc::new(AtomicBool::new(false)),
        pending: lock(&shared).command_pending.clone(),
    };
    let cache: std::cell::RefCell<Option<Arc<wow_sync_runtime::HashCache>>> = Default::default();
    let read_ctl = |t: &Target| {
        let c = wow_sync_runtime::HashCache::open(&opts.state_dir, &cache_scope(t));
        *cache.borrow_mut() = Some(c.clone());
        ScanCtl {
            cancel: abort.clone(),
            cache: Some(c),
            interrupt: Some(interrupt.clone()),
            ..ScanCtl::default()
        }
    };
    let open = |t: &Target| -> Result<SliceService, wow_sync_runtime::ServiceError> {
        let mut s = open_service(config_for(&opts, t), os_attach(&opts))?;
        // Every service, however it is opened, gets the hash cache, quit cancel and command
        // interrupt.
        s.set_read_ctl(read_ctl(t));
        #[cfg(test)]
        tests::count_open(&opts.state_dir);
        s.inject_crash_after(opts.crash_after);
        Ok(s)
    };
    let mut svc: Option<SliceService> = None;
    // The target the service is for; it changes only when setup finishes.
    let mut active: Option<Target> = target.clone();
    let saved = target.is_some() && setup.is_none();
    let mut open_failed = false;
    if let Some(t) = &target {
        wow_sync_runtime::HashCache::prune_stale(&opts.state_dir, &cache_scope(t));
        match open(t) {
            Ok(s) => svc = Some(s),
            Err(e) => {
                crate::app::log_error(&format!("could not open the sync service: {e}"));
                open_failed = true;
                publish(&shared, &wake, |g| g.message = e.to_string());
            }
        }
    }
    // "Enrolled at start" only when the saved enrollment really loads. If the service could
    // not be opened, the saved enrollment file decides, and the open is retried below.
    let enrolled_at_start = saved
        && match &svc {
            Some(s) => s.enrollment_loads(),
            None => opts.state_dir.join("enrollment.json").is_file(),
        };
    let lost = saved && !enrolled_at_start;
    publish(&shared, &wake, |g| {
        g.setup = setup.clone();
        g.started = true;
        g.shared_root = target.as_ref().map(|t| t.shared_root.clone());
        if enrolled_at_start {
            g.enrolled_now = true;
            g.snapshot.enrolled = true;
            g.snapshot.computer_name = opts.display_name.clone();
            if open_failed {
                g.fault = Some("opening the sync service".into());
            }
        }
        if lost {
            g.message = LOST_ENROLLMENT.into();
        }
    });
    if lost {
        crate::app::log_warn("saved setup could not be loaded; showing setup");
    }
    if setup.is_none() && !enrolled_at_start {
        detect_later(
            opts.display_name.clone(),
            shared.clone(),
            wake.clone(),
            !lost,
        );
    }
    // Consecutive failed retries since the fault appeared; the delay doubles up to 4x.
    let mut fault_streak: u32 = 0;
    let mut gate = QuitGate::default();
    let mut inspector = Inspector::default();
    let mut force = true;
    let mut carried: Option<Cmd> = None;
    loop {
        // A command that arrived during a pass, or while it ran, is answered before any
        // further pass starts.
        let queued = carried.take().or_else(|| rx.try_recv().ok());
        let cmd = if queued.is_some() {
            queued
        } else if force {
            None
        } else {
            let scheduled = svc
                .as_mut()
                .is_some_and(|s| !s.pause_state().holds_sync() && s.is_enrolled());
            let next = scheduled.then(|| now_ms() + opts.interval.as_millis() as i64);
            publish(&shared, &wake, |g| g.next_check_ms = next);
            let wait = if lock(&shared).fault.is_some() {
                fault_retry(&opts.state_dir) * (1 << fault_streak.min(2))
            } else {
                fault_streak = 0;
                opts.interval
            };
            match rx.recv_timeout(wait) {
                Err(RecvTimeoutError::Disconnected) => return,
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => {
                    if lock(&shared).fault.is_some() {
                        fault_streak += 1;
                    }
                    None
                }
            }
        };
        force = false;
        let mut manual = false;
        match cmd {
            Some(Cmd::Inspect(t)) => {
                inspector.start(config_for(&opts, &t), t, &shared, &wake);
                continue;
            }
            Some(Cmd::CancelInspect) => {
                inspector.cancel(&shared, &wake);
                continue;
            }
            Some(Cmd::Enroll(t, req)) => {
                inspector.cancel(&shared, &wake);
                if svc.as_ref().is_some_and(|s| s.enrollment_loads()) {
                    crate::app::log_warn("enrollment refused: this computer is already set up");
                    publish(&shared, &wake, |g| g.message = ALREADY_SET_UP.into());
                    continue;
                }
                let _ = std::fs::create_dir_all(&t.shared_root);
                let res = open(&t).and_then(|mut s| {
                    s.enroll(&req)?;
                    Ok(s)
                });
                match res {
                    Ok(mut s) => {
                        if opts.save_target {
                            save_target(&opts.state_dir, &t);
                        }
                        wow_sync_runtime::HashCache::prune_stale(&opts.state_dir, &cache_scope(&t));
                        active = Some(t.clone());
                        s.inject_crash_after(opts.crash_after);
                        refresh(&mut s, &shared, &wake, false);
                        svc = Some(s);
                        publish(&shared, &wake, |g| {
                            g.message.clear();
                            g.fault = None;
                            g.enrolled_now = true;
                            g.shared_root = Some(t.shared_root.clone());
                        });
                        force = true;
                    }
                    Err(wow_sync_runtime::ServiceError::Cancelled) => {
                        crate::app::log("enrollment stopped because the app is quitting");
                        publish(&shared, &wake, |g| {
                            g.message = "Setup was stopped before it finished. Nothing was changed; choose Set up to try again.".into();
                        });
                    }
                    Err(e) => {
                        crate::app::log_error(&format!("enrollment failed: {e}"));
                        publish(&shared, &wake, |g| g.message = humane(&e))
                    }
                }
                continue;
            }
            Some(Cmd::Reconfirm(req)) => {
                if let Some(s) = svc.as_mut() {
                    let res = s.reconfirm(&req);
                    if let Err(e) = &res {
                        crate::app::log_error(&format!("reconfirmation failed: {e}"));
                    }
                    publish(&shared, &wake, |g| {
                        g.message = match res {
                            Ok(()) => String::new(),
                            Err(e) => humane(&e),
                        }
                    });
                    force = true;
                }
                continue;
            }
            Some(Cmd::SaveSettings(st)) => {
                if let Some(s) = svc.as_ref() {
                    let _ = s.save_app_settings(&st);
                    publish(&shared, &wake, |g| g.settings = st);
                }
                continue;
            }
            Some(Cmd::SetTestMode(on, t)) => {
                let cmd = wow_sync_runtime::Command::SetTestMode(on);
                let mut failed = false;
                if let Some(s) = svc.as_mut() {
                    failed = s.command(cmd).is_err();
                    refresh(s, &shared, &wake, false);
                } else if let Some(t) = t {
                    failed = match SliceService::new(config_for(&opts, &t)) {
                        Ok(mut s) => s.command(cmd).is_err(),
                        Err(_) => true,
                    };
                    // The folder is judged again, so setup shows the real state either way.
                    publish(&shared, &wake, |g| {
                        g.message = if failed {
                            TEST_MODE_FAILED.into()
                        } else {
                            String::new()
                        };
                    });
                    inspector.start(config_for(&opts, &t), t, &shared, &wake);
                }
                publish(&shared, &wake, |g| {
                    let seq = g.test_mode_result.as_ref().map_or(1, |r| r.0 + 1);
                    g.test_mode_result = Some((seq, on, !failed));
                });
                continue;
            }
            Some(Cmd::SaveToggles(t)) => {
                if let Some(s) = svc.as_ref() {
                    let _ = s.save_toggles(&t);
                    publish(&shared, &wake, |g| g.toggles = t);
                }
                continue;
            }
            Some(Cmd::Group(a)) => {
                if let Some(s) = svc.as_mut() {
                    let done = guarded("handling a command", &shared, &wake, || dispatch(s, &a));
                    let panicked = done.is_none();
                    let result = done.unwrap_or_else(|| {
                        ActionResult::Failed(
                            "WoW Sync hit an internal error and could not finish that. Export diagnostics and report it.".into(),
                        )
                    });
                    let kind = a.kind();
                    let subject = a.subject();
                    publish(&shared, &wake, |g| {
                        let seq = g.group_result.as_ref().map_or(1, |r| r.seq + 1);
                        g.group_result = Some(GroupResult {
                            seq,
                            kind,
                            subject,
                            result,
                        });
                    });
                    // A panic may have left the in-memory state half-updated: start again
                    // from what is on disk.
                    if panicked {
                        svc = active.as_ref().and_then(|t| open(t).ok());
                    }
                    if let Some(s) = svc.as_mut() {
                        refresh(s, &shared, &wake, false);
                    }
                } else {
                    crate::app::log_warn(&format!("{:?}: not done, sync is not set up", a.kind()));
                    publish(&shared, &wake, |g| {
                        let seq = g.group_result.as_ref().map_or(1, |r| r.seq + 1);
                        g.group_result = Some(GroupResult {
                            seq,
                            kind: a.kind(),
                            subject: None,
                            result: ActionResult::Failed("this computer is not set up yet".into()),
                        });
                    });
                }
                continue;
            }
            Some(Cmd::CheckAllFiles) => {
                match cache.borrow().as_ref() {
                    Some(c) => {
                        c.request_full();
                        #[cfg(test)]
                        tests::count_full(&opts.state_dir);
                        crate::app::log(
                            "check all files requested: the next pass re-reads every add-on file",
                        );
                    }
                    None => crate::app::log_warn("check all files: nothing is set up to check"),
                }
                manual = true;
            }
            Some(Cmd::Pause) => {
                if let Some(s) = svc.as_mut() {
                    match s.set_paused(true) {
                        Ok(()) => crate::app::log("pause: syncing paused"),
                        Err(e) => {
                            crate::app::log_warn(&format!("pause: could not save it ({e:?})"))
                        }
                    }
                    publish_pause(s, &shared, &wake);
                    refresh(s, &shared, &wake, false);
                }
                continue;
            }
            Some(Cmd::Resume) => {
                if let Some(s) = svc.as_ref() {
                    match s.set_paused(false) {
                        Ok(()) => crate::app::log("resume: syncing resumed"),
                        Err(e) => {
                            crate::app::log_warn(&format!("resume: could not save it ({e:?})"))
                        }
                    }
                    publish_pause(s, &shared, &wake);
                }
            }
            Some(Cmd::SyncNow) => manual = true,
            Some(Cmd::CancelQuit) => {
                abort.store(false, Ordering::Relaxed);
                gate.cancel();
                publish(&shared, &wake, |g| g.quit_ready = false);
                // A scan cut short by Quit left a blank status; read a fresh one.
                force = true;
                continue;
            }
            Some(Cmd::QuitWhenSafe) => {
                // A read-only check or scan is abandoned; only a write step that is already
                // running delays quit, and it finishes whole because commands are read between
                // steps.
                abort.store(true, Ordering::Relaxed);
                inspector.cancel(&shared, &wake);
                if gate.request() == StopState::Now {
                    publish(&shared, &wake, |g| g.quit_ready = true);
                    continue;
                }
            }
            None => {}
        }
        if svc.is_none() {
            let retry = lock(&shared).fault.is_some() && !abort.load(Ordering::Relaxed);
            let Some(t) = active.clone().filter(|_| retry) else {
                continue;
            };
            match open(&t) {
                Ok(s) => {
                    crate::app::log("the sync service opened on a retry");
                    let loads = s.enrollment_loads();
                    svc = Some(s);
                    if loads {
                        fault_streak = 0;
                        publish(&shared, &wake, |g| {
                            g.fault = None;
                            g.message.clear();
                        });
                        force = true;
                    } else {
                        report_lost(&shared, &wake, &opts.display_name);
                    }
                }
                Err(e) => {
                    crate::app::log_error(&format!("could not open the sync service: {e}"));
                    publish(&shared, &wake, |g| g.message = e.to_string());
                }
            }
            continue;
        }
        let Some(s) = svc.as_mut() else { continue };
        if !s.is_enrolled() {
            if lock(&shared).enrolled_now {
                report_lost(&shared, &wake, &opts.display_name);
            }
            continue;
        }
        if s.pause_state().holds_sync() && !manual {
            refresh(s, &shared, &wake, false);
            continue;
        }
        // From here until the read-only part of the pass is over, a new command stops the scan
        // (the cache keeps what it hashed) and is answered before the pass starts again.
        interrupt.pending.store(false, Ordering::Relaxed);
        if let Ok(c) = rx.try_recv() {
            carried = Some(c);
            force = true;
            continue;
        }
        if !gate.begin_step() {
            continue;
        }
        publish(&shared, &wake, |g| g.busy = true);
        interrupt.armed.store(true, Ordering::Relaxed);
        let pass_started = Instant::now();
        let res = guarded("running a sync pass", &shared, &wake, || {
            #[cfg(test)]
            {
                let mut hook = tests::PANIC_NEXT_PASS.lock().unwrap();
                if let Some(i) = hook.iter().position(|p| *p == opts.state_dir) {
                    hook.remove(i);
                    drop(hook);
                    panic!("injected test panic");
                }
            }
            s.sync_once()
        });
        interrupt.armed.store(false, Ordering::Relaxed);
        let Some(res) = res else {
            // Start again from what is on disk, not from half-updated memory.
            svc = active.as_ref().and_then(|t| open(t).ok());
            publish(&shared, &wake, |g| g.busy = false);
            if gate.end_step() {
                publish(&shared, &wake, |g| g.quit_ready = true);
            }
            continue;
        };
        publish(&shared, &wake, |g| g.fault = None);
        let crashed = matches!(&res, Ok(r) if r.crashed);
        if crashed {
            let _ = s.set_paused(true);
        }
        if res.is_ok() {
            if let Some(c) = cache.borrow().as_ref() {
                let (reused, read) = (
                    c.hits.swap(0, Ordering::Relaxed),
                    c.misses.swap(0, Ordering::Relaxed),
                );
                let ms = pass_started.elapsed().as_millis();
                if read > 0 || ms > 2000 {
                    crate::app::log(&format!(
                        "sync pass took {ms} ms: {reused} file(s) reused from the hash cache, {read} read"
                    ));
                }
            }
        }
        match res {
            Ok(r) if !r.crashed && !abort.load(Ordering::Relaxed) => {
                publish(&shared, &wake, |g| {
                    g.message.clear();
                    g.fault = None;
                });
                let mut st = r.status;
                st.test_mode = s.test_mode();
                refresh_with(s, &shared, &wake, true, st);
            }
            Ok(_) => {
                publish(&shared, &wake, |g| {
                    g.message.clear();
                    g.fault = None;
                });
                refresh(s, &shared, &wake, true);
            }
            Err(wow_sync_runtime::ServiceError::Cancelled) if abort.load(Ordering::Relaxed) => {
                crate::app::log("sync cycle stopped because the app is quitting");
            }
            Err(wow_sync_runtime::ServiceError::Cancelled) => {
                crate::app::log(&format!(
                    "sync pass paused after {} ms to answer a command; it starts again right after",
                    pass_started.elapsed().as_millis()
                ));
                force = true;
            }
            Err(e) => {
                crate::app::log_error(&format!("sync cycle failed: {e}"));
                let has_provider = s.status().provider_error.is_some();
                publish(&shared, &wake, |g| {
                    g.message = if has_provider {
                        String::new()
                    } else {
                        e.to_string()
                    }
                });
                refresh(s, &shared, &wake, true);
            }
        }
        publish(&shared, &wake, |g| g.busy = false);
        if gate.end_step() {
            publish(&shared, &wake, |g| g.quit_ready = true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab_backend(tag: &str) -> (PathBuf, SharedState, Sender<Cmd>, Target) {
        let base = std::env::temp_dir().join(format!("wowsync-r8-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("wow").join("_retail_")).unwrap();
        // The local state rejects symlinked paths and macOS temp dirs sit behind one.
        let base = std::fs::canonicalize(&base).unwrap();
        let opts = BackendOptions {
            interval: Duration::from_secs(60),
            crash_after: None,
            state_dir: base.join("state"),
            display_name: "Desk".into(),
            app_version: "0.0.0".into(),
            recent_ms: 0,
            save_target: false,
            os_services: false,
        };
        std::fs::create_dir_all(&opts.state_dir).unwrap();
        let t = Target {
            install_root: base.join("wow"),
            variant_folder: "_retail_".into(),
            shared_root: base.join("shared"),
        };
        let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
        let (tx, rx) = std::sync::mpsc::channel();
        let (wake, _woken) = std::sync::mpsc::channel();
        spawn(
            None,
            Some(SetupData::default()),
            opts,
            rx,
            shared.clone(),
            wake,
        );
        (base, shared, tx, t)
    }

    fn held(t: &Target) -> Target {
        Target {
            variant_folder: "_hold_".into(),
            ..t.clone()
        }
    }

    fn wait_for(what: &str, limit: Duration, mut ok: impl FnMut() -> bool) -> Duration {
        let start = Instant::now();
        while !ok() {
            assert!(start.elapsed() < limit, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
        start.elapsed()
    }

    #[test]
    fn quit_does_not_wait_behind_a_running_read_only_inspection() {
        let (base, shared, tx, t) = lab_backend("quit");
        tx.send(Cmd::Inspect(held(&t))).unwrap();
        wait_for("the check to start", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_some()
        });
        tx.send(Cmd::QuitWhenSafe).unwrap();
        let took = wait_for("quit to be ready", Duration::from_secs(5), || {
            lock(&shared).quit_ready
        });
        assert!(took < Duration::from_secs(5), "quit took {took:?}");
        wait_for("the check to be abandoned", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_none()
        });
        let g = lock(&shared);
        assert!(g.preview.is_none() && g.preview_for.is_none());
        drop(g);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_cancelled_or_replaced_inspection_never_publishes_for_the_wrong_target() {
        let (base, shared, tx, t) = lab_backend("replace");
        tx.send(Cmd::Inspect(held(&t))).unwrap();
        wait_for("the first check", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_some()
        });
        tx.send(Cmd::CancelInspect).unwrap();
        wait_for("cancel", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_none()
        });
        tx.send(Cmd::Inspect(held(&t))).unwrap();
        wait_for("the second check", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_some()
        });
        tx.send(Cmd::Inspect(t.clone())).unwrap();
        wait_for("the replacing check", Duration::from_secs(5), || {
            lock(&shared).preview.is_some()
        });
        let g = lock(&shared);
        assert_eq!(
            g.preview_for,
            Some((t.install_root.clone(), "_retail_".to_string())),
            "the preview belongs to the folder that was asked for last"
        );
        assert!(g.inspecting.is_none());
        drop(g);
        std::thread::sleep(Duration::from_millis(100));
        let g = lock(&shared);
        assert_eq!(
            g.preview_for.as_ref().map(|p| p.1.as_str()),
            Some("_retail_")
        );
        drop(g);
        let _ = std::fs::remove_dir_all(&base);
    }

    fn ended(shared: &SharedState) -> u64 {
        lock(shared).inspect_ended
    }

    #[test]
    fn a_check_that_ends_with_a_result_after_cancel_or_replace_publishes_nothing() {
        let late = |t: &Target| Target {
            variant_folder: "_late_".into(),
            ..t.clone()
        };
        let (base, shared, tx, t) = lab_backend("late");
        tx.send(Cmd::Inspect(late(&t))).unwrap();
        wait_for("the check", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_some()
        });
        tx.send(Cmd::CancelInspect).unwrap();
        wait_for("it to end", Duration::from_secs(5), || ended(&shared) == 1);
        {
            let g = lock(&shared);
            assert!(g.preview.is_none() && g.preview_for.is_none() && g.inspecting.is_none());
        }
        // Replaced by a newer check instead of cancelled.
        tx.send(Cmd::Inspect(late(&t))).unwrap();
        wait_for("the second", Duration::from_secs(5), || {
            lock(&shared).inspecting.is_some()
        });
        tx.send(Cmd::Inspect(held(&t))).unwrap();
        wait_for("it to end", Duration::from_secs(5), || ended(&shared) == 2);
        {
            let g = lock(&shared);
            assert!(
                g.preview.is_none(),
                "an old result must not fill the new check"
            );
            assert!(
                g.inspecting.is_some(),
                "the newer check is still the live one"
            );
        }
        tx.send(Cmd::CancelInspect).unwrap();
        wait_for("the last", Duration::from_secs(5), || ended(&shared) == 3);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_check_that_panics_ends_as_an_error_and_stops_its_ticker() {
        let (base, shared, tx, t) = lab_backend("panic");
        let boom = Target {
            variant_folder: "_panic_".into(),
            ..t
        };
        tx.send(Cmd::Inspect(boom)).unwrap();
        wait_for("the check to end", Duration::from_secs(5), || {
            ended(&shared) == 1
        });
        {
            let g = lock(&shared);
            assert!(g.inspecting.is_none(), "the row is no longer checking");
            assert!(g.preview.is_none());
            assert!(g.message.contains("stopped unexpectedly"), "{}", g.message);
        }
        let v = lock(&shared).version;
        std::thread::sleep(Duration::from_millis(700));
        assert_eq!(
            lock(&shared).version,
            v,
            "the progress ticker is still publishing"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn enroll_uses_only_the_digest_of_the_folder_it_was_checked_for() {
        let (base, _shared, _tx, t) = lab_backend("digest");
        let mut g = Shared::default();
        let preview = SliceService::new(config_for(&lab_opts(&base), &t))
            .unwrap()
            .inspect();
        let preview = EnrollPreview {
            digest: Some(wow_sync_core::ids::Sha256([7; 32])),
            ..preview
        };
        g.preview = Some(preview);
        g.preview_for = Some((t.install_root.clone(), "_forever_".into()));
        assert_eq!(g.digest_for(&t), None, "another folder's digest is ignored");
        g.preview_for = Some((t.install_root.clone(), t.variant_folder.clone()));
        assert!(g.digest_for(&t).is_some());
        g.preview_for = Some((base.join("elsewhere"), t.variant_folder.clone()));
        assert_eq!(g.digest_for(&t), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    fn lab_opts(base: &Path) -> BackendOptions {
        BackendOptions {
            interval: Duration::from_secs(60),
            crash_after: None,
            state_dir: base.join("state2"),
            display_name: "Desk".into(),
            app_version: "0.0.0".into(),
            recent_ms: 0,
            save_target: false,
            os_services: false,
        }
    }

    #[test]
    fn keep_running_after_quit_leaves_a_stopped_setup_actionable_and_clears_the_flag() {
        let (base, shared, tx, t) = lab_backend("keep");
        let variant = t.install_root.join("_retail_");
        std::fs::create_dir_all(variant.join("Interface").join("AddOns").join("A")).unwrap();
        std::fs::write(
            variant.join("Interface/AddOns/A/A.toc"),
            "## Interface: 11508\n",
        )
        .unwrap();
        std::fs::write(
            t.install_root.join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Product!STRING:0|Version!STRING:0\nus|1|wow|1.0.0.1\n",
        )
        .unwrap();
        std::fs::write(
            variant.join(".flavor.info"),
            "Product Flavor!STRING:0\nwow\n",
        )
        .unwrap();
        // Quit was chosen (flag set) while Set up was scanning.
        lock(&shared).abort_reads.store(true, Ordering::Relaxed);
        tx.send(Cmd::Enroll(
            t.clone(),
            EnrollRequest {
                join: wow_sync_runtime::JoinChoice::New { name: "X".into() },
                identity_confirmed: true,
                scope_confirmed: true,
                digest: None,
                display_name: "Desk".into(),
            },
        ))
        .unwrap();
        wait_for("a message", Duration::from_secs(10), || {
            !lock(&shared).message.is_empty()
        });
        assert!(
            lock(&shared).message.contains("Setup was stopped"),
            "{}",
            lock(&shared).message
        );
        assert!(!lock(&shared).enrolled_now);
        // Keep running: the flag is cleared and the page is actionable again.
        tx.send(Cmd::CancelQuit).unwrap();
        wait_for("the flag to clear", Duration::from_secs(5), || {
            !lock(&shared).abort_reads.load(Ordering::Relaxed)
        });
        assert!(!lock(&shared).quit_ready);
        let _ = std::fs::remove_dir_all(&base);
    }

    pub(super) static PANIC_NEXT_PASS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    // Per state folder, so tests running in parallel cannot change each other's timing or counts.
    static RETRY_MS: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());
    static OPENS: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

    fn slot(v: &mut Vec<(PathBuf, u64)>, k: &Path) -> usize {
        v.iter().position(|(p, _)| p == k).unwrap_or_else(|| {
            v.push((k.to_path_buf(), 0));
            v.len() - 1
        })
    }

    pub(super) fn retry_ms(k: &Path) -> u64 {
        let mut v = RETRY_MS.lock().unwrap();
        let i = slot(&mut v, k);
        v[i].1
    }

    fn set_retry(base: &Path, ms: u64) {
        let mut v = RETRY_MS.lock().unwrap();
        let i = slot(&mut v, &base.join("state"));
        v[i].1 = ms;
    }

    static FULLS: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

    pub(super) fn count_full(k: &Path) {
        let mut v = FULLS.lock().unwrap();
        let i = slot(&mut v, k);
        v[i].1 += 1;
    }

    pub(super) fn count_open(k: &Path) {
        let mut v = OPENS.lock().unwrap();
        let i = slot(&mut v, k);
        v[i].1 += 1;
    }

    fn opens(base: &Path) -> u64 {
        let mut v = OPENS.lock().unwrap();
        let i = slot(&mut v, &base.join("state"));
        v[i].1
    }

    fn panic_next_pass(base: &Path) {
        PANIC_NEXT_PASS.lock().unwrap().push(base.join("state"));
    }

    fn cache_files(base: &Path) -> usize {
        std::fs::read_dir(base.join("state"))
            .map(|d| {
                d.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with("hashcache-"))
                    .count()
            })
            .unwrap_or(0)
    }

    fn clear_caches(base: &Path) {
        for e in std::fs::read_dir(base.join("state")).unwrap().flatten() {
            if e.file_name().to_string_lossy().starts_with("hashcache-") {
                std::fs::remove_file(e.path()).unwrap();
            }
        }
    }

    #[test]
    fn a_service_opened_from_a_saved_enrollment_or_after_a_panic_still_uses_the_hash_cache() {
        let (base, shared1, tx1, t) = enrolled_lab("ctlrestart");
        wait_for("the first pass", Duration::from_secs(20), || {
            let g = lock(&shared1);
            !g.busy && g.snapshot.detail.is_some()
        });
        assert!(cache_files(&base) > 0, "enroll path writes the cache");
        tx1.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || {
            lock(&shared1).quit_ready
        });
        drop(tx1);
        clear_caches(&base);
        let mut opts = lab_opts(&base);
        opts.state_dir = base.join("state");
        let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
        let (tx, rx) = std::sync::mpsc::channel();
        let (wake, _w) = std::sync::mpsc::channel();
        set_retry(&base, 500);
        spawn(Some(t.clone()), None, opts, rx, shared.clone(), wake);
        wait_for("restart pass", Duration::from_secs(20), || {
            let g = lock(&shared);
            !g.busy && g.snapshot.detail.is_some()
        });
        assert!(cache_files(&base) > 0, "restart path writes the cache");
        // Check all files must find the cache after a restart.
        tx.send(Cmd::CheckAllFiles).unwrap();
        wait_for("check all files", Duration::from_secs(10), || {
            FULLS
                .lock()
                .unwrap()
                .iter()
                .any(|(p, n)| *p == base.join("state") && *n > 0)
        });
        wait_for("its pass", Duration::from_secs(20), || !lock(&shared).busy);
        clear_caches(&base);
        panic_next_pass(&base);
        tx.send(Cmd::SyncNow).unwrap();
        wait_for("the fault", Duration::from_secs(10), || {
            lock(&shared).fault.is_some()
        });
        wait_for("the retry", Duration::from_secs(20), || {
            lock(&shared).fault.is_none()
        });
        wait_for("the cache again", Duration::from_secs(20), || {
            cache_files(&base) > 0
        });
        tx.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || lock(&shared).quit_ready);
        set_retry(&base, 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn after_a_panic_the_service_is_reopened_from_disk() {
        let (base, shared, tx, _t) = enrolled_lab("reopen");
        set_retry(&base, 1000);
        wait_for("the first pass", Duration::from_secs(20), || {
            let g = lock(&shared);
            !g.busy && g.snapshot.detail.is_some()
        });
        let before = opens(&base);
        panic_next_pass(&base);
        tx.send(Cmd::SyncNow).unwrap();
        wait_for("the fault", Duration::from_secs(10), || {
            lock(&shared).fault.is_some()
        });
        wait_for("the reopen", Duration::from_secs(10), || {
            opens(&base) > before
        });
        wait_for("the retry", Duration::from_secs(20), || {
            lock(&shared).fault.is_none()
        });
        tx.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || lock(&shared).quit_ready);
        set_retry(&base, 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_saved_target_without_a_loadable_enrollment_offers_setup_and_does_not_enroll() {
        let (base, shared1, tx1, t) = enrolled_lab("lostenr");
        wait_for("the first pass", Duration::from_secs(20), || {
            !lock(&shared1).busy
        });
        tx1.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || {
            lock(&shared1).quit_ready
        });
        drop(tx1);
        std::fs::remove_file(base.join("state").join("enrollment.json")).unwrap();
        let mut opts = lab_opts(&base);
        opts.state_dir = base.join("state");
        let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
        let (_tx, rx) = std::sync::mpsc::channel();
        let (wake, _w) = std::sync::mpsc::channel();
        spawn(Some(t.clone()), None, opts, rx, shared.clone(), wake);
        wait_for("setup offered", Duration::from_secs(20), || {
            lock(&shared).setup.is_some()
        });
        let g = lock(&shared);
        assert!(!g.enrolled_now && !g.snapshot.enrolled);
        assert_eq!(g.message, LOST_ENROLLMENT);
        drop(g);
        assert!(!base.join("state").join("enrollment.json").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    fn install_files(t: &Target) {
        let variant = t.install_root.join("_retail_");
        std::fs::create_dir_all(variant.join("Interface").join("AddOns").join("A")).unwrap();
        std::fs::write(
            variant.join("Interface/AddOns/A/A.toc"),
            "## Interface: 11508\n",
        )
        .unwrap();
        std::fs::write(
            t.install_root.join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Product!STRING:0|Version!STRING:0\nus|1|wow|1.0.0.1\n",
        )
        .unwrap();
        std::fs::write(
            variant.join(".flavor.info"),
            "Product Flavor!STRING:0\nwow\n",
        )
        .unwrap();
    }

    fn enroll_req(base: &Path, t: &Target) -> EnrollRequest {
        let digest = SliceService::new(config_for(&lab_opts(base), t))
            .unwrap()
            .inspect()
            .digest;
        EnrollRequest {
            join: wow_sync_runtime::JoinChoice::New { name: "X".into() },
            identity_confirmed: true,
            scope_confirmed: true,
            digest,
            display_name: "Desk".into(),
        }
    }

    fn enrolled_lab(tag: &str) -> (PathBuf, SharedState, Sender<Cmd>, Target) {
        let (base, shared, tx, t) = lab_backend(tag);
        install_files(&t);
        tx.send(Cmd::Enroll(t.clone(), enroll_req(&base, &t)))
            .unwrap();
        wait_for("enrollment", Duration::from_secs(20), || {
            let g = lock(&shared);
            g.enrolled_now || !g.message.is_empty()
        });
        assert!(lock(&shared).enrolled_now, "{}", lock(&shared).message);
        (base, shared, tx, t)
    }

    #[test]
    fn a_panic_in_a_pass_is_shown_keeps_commands_working_and_is_retried() {
        let (base, shared, tx, _t) = enrolled_lab("faultpass");
        set_retry(&base, 2000);
        wait_for("the first pass", Duration::from_secs(20), || {
            let g = lock(&shared);
            !g.busy && g.snapshot.detail.is_some()
        });
        panic_next_pass(&base);
        tx.send(Cmd::SyncNow).unwrap();
        wait_for("the fault", Duration::from_secs(10), || {
            lock(&shared).fault.is_some()
        });
        assert_eq!(lock(&shared).fault.as_deref(), Some("running a sync pass"));
        assert!(!lock(&shared).busy);
        wait_for(
            "the retry to clear the fault",
            Duration::from_secs(20),
            || lock(&shared).fault.is_none(),
        );
        tx.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || lock(&shared).quit_ready);
        set_retry(&base, 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn setup_finishing_for_an_enrolled_computer_is_refused_not_repeated() {
        let (base, shared, tx, t) = enrolled_lab("twice");
        wait_for("the first pass", Duration::from_secs(20), || {
            !lock(&shared).busy
        });
        lock(&shared).message.clear();
        tx.send(Cmd::Enroll(t.clone(), enroll_req(&base, &t)))
            .unwrap();
        wait_for("a message", Duration::from_secs(10), || {
            !lock(&shared).message.is_empty()
        });
        assert_eq!(lock(&shared).message, ALREADY_SET_UP);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_restart_with_a_saved_enrollment_shows_the_main_view_not_setup() {
        let (base, shared1, tx1, t) = enrolled_lab("restart");
        wait_for("the first pass", Duration::from_secs(20), || {
            !lock(&shared1).busy
        });
        tx1.send(Cmd::QuitWhenSafe).unwrap();
        wait_for("quit", Duration::from_secs(10), || {
            lock(&shared1).quit_ready
        });
        drop(tx1);
        let mut opts = lab_opts(&base);
        opts.state_dir = base.join("state");
        let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
        let (_tx, rx) = std::sync::mpsc::channel();
        let (wake, _w) = std::sync::mpsc::channel();
        spawn(Some(t.clone()), None, opts, rx, shared.clone(), wake);
        wait_for("started", Duration::from_secs(10), || lock(&shared).started);
        let g = lock(&shared);
        assert!(g.enrolled_now, "a saved enrollment is never shown as setup");
        assert!(g.setup.is_none());
        let mut w = crate::world::World::empty("0.0.0");
        w.confirm_enrolled(g.enrolled_now);
        w.checking = true;
        drop(g);
        assert!(w.enrolled);
        assert_eq!(crate::model::headline(&w).text, "Checking add-ons…");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_failed_test_mode_change_is_reported_and_never_claims_a_state() {
        let failed = test_mode_notice(false, false);
        assert_eq!(failed, TEST_MODE_FAILED);
        assert_eq!(test_mode_notice(true, false), TEST_MODE_FAILED);
        assert!(!failed.contains("Test mode is off"));
        assert_eq!(test_mode_notice(false, true), "Test mode is off");
        assert_eq!(test_mode_notice(true, true), "Test mode is on");
    }

    #[test]
    fn refusals_read_as_plain_language() {
        use wow_sync_runtime::ServiceError;
        let t = humane(&ServiceError::Refused("ConfirmationRequired".into()));
        assert!(t.contains("Confirm both") && !t.contains("ConfirmationRequired"));
        let s = humane(&ServiceError::Refused("StaleConfirmation".into()));
        assert!(s.contains("Check it again") && !s.contains("Stale"));
    }

    #[test]
    fn target_round_trips_and_partial_files_are_ignored() {
        let dir = std::env::temp_dir().join(format!("wowsync-target-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_target(&dir).is_none());
        let t = Target {
            install_root: PathBuf::from("C:\\Games\\WoW"),
            variant_folder: "_retail_".into(),
            shared_root: PathBuf::from("C:\\Dropbox\\WoW Sync"),
        };
        save_target(&dir, &t);
        let back = load_target(&dir).expect("saved target loads");
        assert_eq!(back.install_root, t.install_root);
        assert_eq!(back.variant_folder, t.variant_folder);
        assert_eq!(back.shared_root, t.shared_root);
        std::fs::write(dir.join(CONFIG_FILE), "install=C:\\x\nvariant=\nshared=y\n").unwrap();
        assert!(
            load_target(&dir).is_none(),
            "an empty field is not a target"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_group_result_gets_a_log_line_without_paths() {
        use wow_sync_runtime::{CommandError as E, CommandOutcome as O};
        let (ok, l) = group_log_line(ActionKind::Decide, &Ok(O::NothingToDo));
        assert!(ok && l.contains("nothing to do"), "{l}");
        let (ok, l) = group_log_line(ActionKind::Decide, &Err(E::NotAllowed));
        assert!(!ok && l.contains("refused"), "{l}");
        let stale = Err(E::Decide(wow_sync_core::decision::DecideError::Changed));
        let (ok, l) = group_log_line(ActionKind::Decide, &stale);
        assert!(
            !ok && l.contains("out of date") && l.contains("refreshed"),
            "{l}"
        );
        let (ok, l) = group_log_line(ActionKind::Decide, &Err(E::UpdatesUnavailable));
        assert!(!ok && !l.contains('\\') && !l.contains('/'), "{l}");
    }

    static ATTACHED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn counting_attach(_: &mut SliceService) -> Result<(), wow_sync_runtime::ServiceError> {
        ATTACHED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(wow_sync_runtime::ServiceError::Refused("test".into()))
    }

    #[test]
    fn opening_the_service_attaches_os_services_and_survives_a_failed_attach() {
        let base = std::env::temp_dir().join(format!("wowsync-attach-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        // The local state rejects symlinked paths and macOS temp dirs sit behind one.
        let base = std::fs::canonicalize(&base).unwrap();
        let mut opts = BackendOptions {
            interval: Duration::from_secs(1),
            crash_after: None,
            state_dir: base.join("state"),
            display_name: "Desk".into(),
            app_version: "0.0.0".into(),
            recent_ms: 0,
            save_target: false,
            os_services: false,
        };
        let t = Target {
            install_root: base.join("wow"),
            variant_folder: "_retail_".into(),
            shared_root: base.join("shared"),
        };
        for d in [&opts.state_dir, &t.install_root, &t.shared_root] {
            std::fs::create_dir_all(d).unwrap();
        }
        assert!(
            os_attach(&opts).is_none(),
            "developer mode attaches nothing"
        );
        opts.os_services = true;
        let attach = os_attach(&opts);
        assert!(attach.is_some(), "startup attaches the OS services");
        let before = ATTACHED.load(std::sync::atomic::Ordering::SeqCst);
        let svc = open_service(config_for(&opts, &t), Some(counting_attach));
        assert!(svc.is_ok(), "a failed attach does not stop sync");
        assert_eq!(
            ATTACHED.load(std::sync::atomic::Ordering::SeqCst),
            before + 1
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
