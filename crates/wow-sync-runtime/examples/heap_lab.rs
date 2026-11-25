//! Heap lab (run 011): runs sync passes against an existing lab install and prints the live and
//! peak heap bytes after each pass, so growth can be attributed.
//!
//!   heap_lab <install root> <shared dir> <state dir> <display name> <passes>
//!
//! Point it at a copy of a state directory; it never touches anything else.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};
use wow_sync_runtime::core::plan::Activity;
use wow_sync_runtime::{HashCache, ScanCtl, ScriptedActivity, ServiceConfig, SliceService};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to the system allocator and only adds counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let now = LIVE.fetch_add(l.size(), Relaxed) + l.size();
            PEAK.fetch_max(now, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = System.realloc(p, l, n);
        if !q.is_null() {
            if n >= l.size() {
                let now = LIVE.fetch_add(n - l.size(), Relaxed) + (n - l.size());
                PEAK.fetch_max(now, Relaxed);
            } else {
                LIVE.fetch_sub(l.size() - n, Relaxed);
            }
        }
        q
    }
}

#[global_allocator]
static A: Counting = Counting;

fn mib(b: usize) -> f64 {
    b as f64 / 1048576.0
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let passes: usize = a[4].parse().unwrap();
    let cfg = ServiceConfig {
        install_root: (&a[0]).into(),
        variant_folder: "_forever_".into(),
        shared_root: (&a[1]).into(),
        state_dir: (&a[2]).into(),
        display_name: a[3].clone(),
        app_version: "0.0.0-lab".into(),
        recent_ms: 2000,
    };
    let (src, _act) = ScriptedActivity::new(Activity::NotRunning);
    let mut svc = SliceService::with_activity(cfg, Box::new(src)).unwrap();
    let cache = HashCache::open(a[2].as_ref(), "heap-lab");
    svc.set_read_ctl(ScanCtl {
        cache: Some(cache),
        ..ScanCtl::default()
    });
    println!(
        "start live={:.1} peak={:.1} MiB",
        mib(LIVE.load(Relaxed)),
        mib(PEAK.load(Relaxed))
    );
    for i in 0..passes {
        PEAK.store(LIVE.load(Relaxed), Relaxed);
        let t = Instant::now();
        let r = svc.sync_once().unwrap();
        let st = svc.status();
        println!(
            "pass {i}: {} ms live={:.1} peak={:.1} MiB headline={:?} settings_units={}",
            t.elapsed().as_millis(),
            mib(LIVE.load(Relaxed)),
            mib(PEAK.load(Relaxed)),
            r.status.headline,
            st.settings_units.len()
        );
        drop(r);
        std::thread::sleep(Duration::from_millis(300));
    }
}
