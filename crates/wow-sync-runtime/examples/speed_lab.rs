//! Speed lab (run 010): enrolls a synthetic group (isolated state and provider) on an install,
//! then times repeated sync passes with a warm hash cache and counts file-system touches.
//!
//!   speed_lab <install root> <variant folder> <shared dir> <state dir> [passes] [group id]
//!
//! Enrols into a throwaway copy (a new group, or group id as a new computer) and may write to the
//! install, the shared dir and the state dir: point it only at copies, never at a live install or group.

use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;
use wow_sync_adapters::fs_safe::{LISTED_ENTRIES, LISTINGS, STATS};
use wow_sync_runtime::core::ids::GroupId;
use wow_sync_runtime::core::plan::Activity;
use wow_sync_runtime::{
    EnrollRequest, HashCache, JoinChoice, ScanCtl, ScriptedActivity, ServiceConfig, SliceService,
};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (root, variant, shared, state) = (&a[0], &a[1], &a[2], &a[3]);
    let passes: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
    std::fs::create_dir_all(shared).unwrap();
    std::fs::create_dir_all(state).unwrap();
    let cfg = ServiceConfig {
        install_root: root.into(),
        variant_folder: variant.clone(),
        shared_root: shared.into(),
        state_dir: state.into(),
        display_name: "SPEEDLAB".into(),
        app_version: "0.0.0-lab".into(),
        recent_ms: 2000,
    };
    let (src, _act) = ScriptedActivity::new(Activity::NotRunning);
    let mut svc = if std::env::var_os("SPEED_LAB_REAL_ACTIVITY").is_some() {
        SliceService::new(cfg.clone()).unwrap()
    } else {
        SliceService::with_activity(cfg.clone(), Box::new(src)).unwrap()
    };
    let cache = HashCache::open(state.as_ref(), "speed-lab");
    svc.set_read_ctl(ScanCtl {
        cache: Some(cache.clone()),
        ..ScanCtl::default()
    });
    if !svc.is_enrolled() {
        let p = svc.inspect();
        svc.enroll(&EnrollRequest {
            join: match a.get(5) {
                Some(g) => JoinChoice::Existing(GroupId::parse(g).unwrap()),
                None => JoinChoice::New {
                    name: "SpeedLab".into(),
                },
            },
            identity_confirmed: true,
            scope_confirmed: true,
            digest: p.digest,
            display_name: "SPEEDLAB".into(),
        })
        .unwrap();
    }
    for i in 0..passes {
        cache.hits.store(0, Relaxed);
        cache.misses.store(0, Relaxed);
        let (l0, e0, s0) = (
            LISTINGS.load(Relaxed),
            LISTED_ENTRIES.load(Relaxed),
            STATS.load(Relaxed),
        );
        let t = Instant::now();
        svc.sync_once().unwrap();
        let ms = t.elapsed().as_millis();
        println!(
            "pass {i}: {ms} ms reused={} read={} listings={} listed_entries={} stats={}",
            cache.hits.load(Relaxed),
            cache.misses.load(Relaxed),
            LISTINGS.load(Relaxed) - l0,
            LISTED_ENTRIES.load(Relaxed) - e0,
            STATS.load(Relaxed) - s0,
        );
    }
}
