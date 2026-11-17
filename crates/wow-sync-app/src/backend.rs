//! Background sync thread: owns the `SliceService`, publishes typed view data to the UI.
//!
//! Pause is persisted by the runtime (UG-2). Quit-when-safe goes through `QuitGate` (UG-17): a
//! sync cycle is one step, and the thread only reads commands between steps.

use crate::world::SetupData;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use wow_sync_runtime::{
    ActivityEntry, AppSettings, EnrollPreview, EnrollRequest, QuitGate, ReconfirmRequest,
    ServiceConfig, SliceService, StatusSnapshot, StopState,
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
    Enroll(Target, EnrollRequest),
    Reconfirm(ReconfirmRequest),
    SaveSettings(AppSettings),
}

#[derive(Default)]
pub struct Shared {
    pub version: u64,
    pub started: bool,
    pub snapshot: StatusSnapshot,
    pub preview: Option<EnrollPreview>,
    pub paused_since: Option<i64>,
    pub activity: Vec<ActivityEntry>,
    pub settings: AppSettings,
    pub busy: bool,
    pub last_check_ms: Option<i64>,
    pub message: String,
    pub quit_ready: bool,
    pub setup: Option<SetupData>,
    pub enrolled_now: bool,
    pub shared_root: Option<PathBuf>,
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

fn refresh(svc: &mut SliceService, shared: &SharedState, wake: &Sender<()>, checked: bool) {
    let st = svc.status();
    let (paused, act, settings) = (
        svc.paused_since(),
        svc.activity_log(300),
        svc.app_settings(),
    );
    publish(shared, wake, |g| {
        g.started = true;
        g.snapshot = st;
        g.paused_since = paused;
        g.activity = act;
        g.settings = settings;
        if checked {
            g.last_check_ms = Some(now_ms());
        }
    });
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
        match SliceService::new(config_for(&opts, t)) {
            Ok(mut s) => {
                s.inject_crash_after(opts.crash_after);
                svc = Some(s);
            }
            Err(e) => publish(&shared, &wake, |g| g.message = e.to_string()),
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
                        g.preview = None;
                        g.message = e.to_string();
                    }
                });
                continue;
            }
            Some(Cmd::Enroll(t, req)) => {
                let _ = std::fs::create_dir_all(&t.shared_root);
                let res = SliceService::new(config_for(&opts, &t)).and_then(|mut s| {
                    s.enroll(&req)?;
                    Ok(s)
                });
                match res {
                    Ok(mut s) => {
                        if opts.save_target {
                            save_target(&opts.state_dir, &t);
                        }
                        s.inject_crash_after(opts.crash_after);
                        svc = Some(s);
                        publish(&shared, &wake, |g| {
                            g.message.clear();
                            g.enrolled_now = true;
                            g.shared_root = Some(t.shared_root.clone());
                        });
                        force = true;
                    }
                    Err(e) => publish(&shared, &wake, |g| g.message = e.to_string()),
                }
                continue;
            }
            Some(Cmd::Reconfirm(req)) => {
                if let Some(s) = svc.as_mut() {
                    let res = s.reconfirm(&req);
                    publish(&shared, &wake, |g| {
                        g.message = match res {
                            Ok(()) => String::new(),
                            Err(e) => e.to_string(),
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
        if s.paused_since().is_some() && !manual {
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
}
