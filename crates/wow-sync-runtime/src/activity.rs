//! Game-activity observation behind an injectable source.

use std::path::Path;
use std::sync::{Arc, Mutex};
use wow_sync_adapters::winproc::{observe_variant, ProcessActivity};
use wow_sync_core::plan::Activity;

pub trait ActivitySource: Send {
    fn observe(&mut self, variant_dir: &Path) -> Activity;
}

/// Real process observation (Windows); other platforms report `Unknown`, which blocks writes.
pub struct WinprocActivity;

impl ActivitySource for WinprocActivity {
    fn observe(&mut self, variant_dir: &Path) -> Activity {
        match observe_variant(variant_dir).0 {
            ProcessActivity::NotRunning => Activity::NotRunning,
            ProcessActivity::Running { pids, .. } => Activity::Running { pids },
            ProcessActivity::Unknown(why) => Activity::Unknown(why),
        }
    }
}

/// Shared control of a [`ScriptedActivity`] from a test or the developer shell.
#[derive(Clone)]
pub struct ActivityHandle(Arc<Mutex<Activity>>);

impl ActivityHandle {
    pub fn set(&self, a: Activity) {
        *self.0.lock().expect("activity lock") = a;
    }
}

/// A source whose value is set from outside (headless tests).
pub struct ScriptedActivity(Arc<Mutex<Activity>>);

impl ScriptedActivity {
    pub fn new(initial: Activity) -> (ScriptedActivity, ActivityHandle) {
        let cell = Arc::new(Mutex::new(initial));
        (ScriptedActivity(cell.clone()), ActivityHandle(cell))
    }
}

impl ActivitySource for ScriptedActivity {
    fn observe(&mut self, _variant_dir: &Path) -> Activity {
        self.0.lock().expect("activity lock").clone()
    }
}
