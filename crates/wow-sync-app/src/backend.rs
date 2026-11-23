//! Background sync thread: owns the `SliceService`, publishes typed view data to the UI.
//!
//! Pause is persisted by the runtime (UG-2). Quit-when-safe goes through `QuitGate` (UG-17): a
//! sync cycle is one step, and the thread only reads commands between steps.

use crate::features::{ActionKind, ActionResult, GroupAction};
use crate::world::SetupData;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use wow_sync_runtime::{
    ActivityEntry, AppSettings, EnrollPreview, EnrollRequest, PauseState, QuitGate,
    ReconfirmRequest, ServiceConfig, SliceService, StatusSnapshot, StopState,
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
    /// Turn test mode on or off; the target is used only before enrollment.
    SetTestMode(bool, Option<Target>),
    Enroll(Target, EnrollRequest),
    Reconfirm(ReconfirmRequest),
    SaveSettings(AppSettings),
    /// The About toggles; stored in their own file by the service.
    SaveToggles(wow_sync_runtime::AppToggles),
    /// A group command from a feature screen; its result is published for the screen to show.
    Group(GroupAction),
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
    ActionResult::from_outcome(svc.command(a.to_command()))
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
    pub shared_root: Option<PathBuf>,
    /// What core currently offers the owner to decide (typed; the app renders the words).
    pub decisions: Vec<wow_sync_core::decision::DecisionItem>,
    pub group_result: Option<GroupResult>,
    /// The last test-mode change: a counter so a result shows once, the mode asked for, and
    /// whether it was applied.
    pub test_mode_result: Option<(u64, bool, bool)>,
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
fn log_status_changes(st: &StatusSnapshot) {
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

fn refresh(svc: &mut SliceService, shared: &SharedState, wake: &Sender<()>, checked: bool) {
    let st = svc.status();
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

fn run(
    target: Option<Target>,
    setup: Option<SetupData>,
    opts: BackendOptions,
    rx: Receiver<Cmd>,
    shared: SharedState,
    wake: Sender<()>,
) {
    let mut svc: Option<SliceService> = None;
    if let Some(t) = &target {
        match open_service(config_for(&opts, t), os_attach(&opts)) {
            Ok(mut s) => {
                s.inject_crash_after(opts.crash_after);
                svc = Some(s);
            }
            Err(e) => {
                crate::app::log_error(&format!("could not open the sync service: {e}"));
                publish(&shared, &wake, |g| g.message = e.to_string())
            }
        }
    }
    publish(&shared, &wake, |g| {
        g.setup = setup;
        g.started = true;
        g.shared_root = target.as_ref().map(|t| t.shared_root.clone());
    });
    let mut gate = QuitGate::default();
    let mut force = true;
    loop {
        let cmd = if force {
            None
        } else {
            let scheduled = svc
                .as_mut()
                .is_some_and(|s| !s.pause_state().holds_sync() && s.status().enrolled);
            let next = scheduled.then(|| now_ms() + opts.interval.as_millis() as i64);
            publish(&shared, &wake, |g| g.next_check_ms = next);
            match rx.recv_timeout(opts.interval) {
                Err(RecvTimeoutError::Disconnected) => return,
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
            }
        };
        force = false;
        let mut manual = false;
        match cmd {
            Some(Cmd::Inspect(t)) => {
                publish(&shared, &wake, |g| g.preview = None);
                let res = SliceService::new(config_for(&opts, &t)).map(|s| s.inspect());
                publish(&shared, &wake, |g| match res {
                    Ok(p) => {
                        g.preview = Some(p);
                        g.message.clear();
                    }
                    Err(e) => {
                        crate::app::log_error(&format!("inspect failed: {e}"));
                        g.preview = None;
                        g.message = e.to_string();
                    }
                });
                continue;
            }
            Some(Cmd::Enroll(t, req)) => {
                let _ = std::fs::create_dir_all(&t.shared_root);
                let res =
                    open_service(config_for(&opts, &t), os_attach(&opts)).and_then(|mut s| {
                        s.enroll(&req)?;
                        Ok(s)
                    });
                match res {
                    Ok(mut s) => {
                        if opts.save_target {
                            save_target(&opts.state_dir, &t);
                        }
                        s.inject_crash_after(opts.crash_after);
                        refresh(&mut s, &shared, &wake, false);
                        svc = Some(s);
                        publish(&shared, &wake, |g| {
                            g.message.clear();
                            g.enrolled_now = true;
                            g.shared_root = Some(t.shared_root.clone());
                        });
                        force = true;
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
                    let p = SliceService::new(config_for(&opts, &t)).map(|s| s.inspect());
                    publish(&shared, &wake, |g| {
                        g.preview = p.ok();
                        g.message = if failed {
                            TEST_MODE_FAILED.into()
                        } else {
                            String::new()
                        };
                    });
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
                    let result = dispatch(s, &a);
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
                    refresh(s, &shared, &wake, false);
                }
                continue;
            }
            Some(Cmd::Pause) => {
                if let Some(s) = svc.as_mut() {
                    let _ = s.set_paused(true);
                    refresh(s, &shared, &wake, false);
                }
                continue;
            }
            Some(Cmd::Resume) => {
                if let Some(s) = svc.as_ref() {
                    let _ = s.set_paused(false);
                }
            }
            Some(Cmd::SyncNow) => manual = true,
            Some(Cmd::CancelQuit) => {
                gate.cancel();
                publish(&shared, &wake, |g| g.quit_ready = false);
                continue;
            }
            Some(Cmd::QuitWhenSafe) => {
                // Commands are only read between cycles, so no step is running here.
                if gate.request() == StopState::Now {
                    publish(&shared, &wake, |g| g.quit_ready = true);
                    continue;
                }
            }
            None => {}
        }
        let Some(s) = svc.as_mut() else { continue };
        if !s.status().enrolled {
            continue;
        }
        if s.pause_state().holds_sync() && !manual {
            refresh(s, &shared, &wake, false);
            continue;
        }
        if !gate.begin_step() {
            continue;
        }
        publish(&shared, &wake, |g| g.busy = true);
        let res = s.sync_once();
        let crashed = matches!(&res, Ok(r) if r.crashed);
        if crashed {
            let _ = s.set_paused(true);
        }
        match res {
            Ok(_) => publish(&shared, &wake, |g| g.message.clear()),
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
            }
        }
        refresh(s, &shared, &wake, true);
        publish(&shared, &wake, |g| g.busy = false);
        if gate.end_step() {
            publish(&shared, &wake, |g| g.quit_ready = true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
