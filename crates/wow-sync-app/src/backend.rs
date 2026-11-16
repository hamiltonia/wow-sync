//! Background sync thread: owns the `SliceService`, publishes view data to the UI.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use wow_sync_runtime::{
    EnrollPreview, EnrollRequest, ServiceConfig, SliceService, StatusSnapshot, SyncReport,
};

pub enum Cmd {
    SyncNow,
    Enroll(EnrollRequest),
    Quit,
}

#[derive(Default)]
pub struct Shared {
    pub version: u64,
    pub preview: Option<EnrollPreview>,
    pub status: StatusSnapshot,
    pub message: String,
    pub report_lines: Vec<String>,
    pub busy: bool,
    pub paused_after_crash: bool,
}

pub type SharedState = Arc<Mutex<Shared>>;

pub fn lock(s: &SharedState) -> MutexGuard<'_, Shared> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct BackendOptions {
    pub interval: Duration,
    pub crash_after: Option<usize>,
}

fn lines(r: &SyncReport) -> Vec<String> {
    let mut v = Vec::new();
    v.extend(r.recovered.iter().map(|l| format!("Recovered {l}")));
    v.extend(
        r.applied
            .iter()
            .map(|l| format!("{}: {}", l.package, l.text)),
    );
    v.extend(r.apply_note.clone());
    v.extend(r.published.clone());
    v.extend(r.publish_note.clone());
    if r.crashed {
        v.push("Stopped mid-way (simulated crash for the developer check)".into());
    }
    v
}

pub fn spawn(
    cfg: ServiceConfig,
    opts: BackendOptions,
    rx: Receiver<Cmd>,
    shared: SharedState,
    wake: Sender<()>,
) {
    std::thread::Builder::new()
        .name("sync".into())
        .spawn(move || run(cfg, opts, rx, shared, wake))
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

fn run(
    cfg: ServiceConfig,
    opts: BackendOptions,
    rx: Receiver<Cmd>,
    shared: SharedState,
    wake: Sender<()>,
) {
    let mut svc = match SliceService::new(cfg) {
        Ok(s) => s,
        Err(e) => {
            publish(&shared, &wake, |g| g.message = e.to_string());
            return;
        }
    };
    svc.inject_crash_after(opts.crash_after);
    let mut paused = false;
    let mut force = true;
    loop {
        let cmd = if force {
            None
        } else {
            match rx.recv_timeout(opts.interval) {
                Ok(Cmd::Quit) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
            }
        };
        force = false;
        let mut manual = false;
        match cmd {
            Some(Cmd::Enroll(req)) => {
                let res = svc.enroll(&req);
                publish(&shared, &wake, |g| {
                    g.message = match res {
                        Ok(_) => "Enrolled. Syncing will start now.".into(),
                        Err(e) => e.to_string(),
                    }
                });
                paused = false;
            }
            Some(Cmd::SyncNow) => {
                manual = true;
                paused = false;
            }
            Some(Cmd::Quit) => return,
            None => {}
        }
        let enrolled = svc.status().enrolled;
        if !enrolled {
            let p = svc.inspect();
            publish(&shared, &wake, |g| {
                g.preview = Some(p);
                g.status = StatusSnapshot::default();
            });
            continue;
        }
        if paused && !manual {
            let st = svc.status();
            publish(&shared, &wake, |g| g.status = st);
            continue;
        }
        publish(&shared, &wake, |g| g.busy = true);
        match svc.sync_once() {
            Ok(r) => {
                let l = lines(&r);
                let crashed = r.crashed;
                let st = if crashed { svc.status() } else { r.status };
                paused = crashed;
                publish(&shared, &wake, |g| {
                    g.busy = false;
                    g.status = st;
                    g.paused_after_crash = crashed;
                    if !l.is_empty() {
                        g.report_lines = l;
                    }
                    g.message.clear();
                });
            }
            Err(e) => {
                let st = svc.status();
                publish(&shared, &wake, |g| {
                    g.busy = false;
                    g.status = st;
                    g.message = e.to_string();
                });
            }
        }
    }
}
