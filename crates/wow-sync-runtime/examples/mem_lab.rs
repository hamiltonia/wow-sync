//! Memory lab (run 010): builds a large synthetic install and syncs it between two isolated
//! synthetic computers, so a separate sampler can watch the apply's memory.
//!
//!   mem_lab gen <parent> <name> <addons> <files per addon> [seed]
//!   mem_lab run <install root> <shared dir> <state dir> <display name> <passes> [group id]
//!
//! File sizes are 15-200 KiB of pseudo-random bytes. Only the given directories are touched.

use std::time::{Duration, Instant, SystemTime};
use wow_sync_adapters::testkit::{set_old_mtimes_under, SyntheticInstall};
use wow_sync_runtime::core::ids::GroupId;
use wow_sync_runtime::core::plan::Activity;
use wow_sync_runtime::{
    EnrollRequest, HashCache, JoinChoice, ScanCtl, ScriptedActivity, ServiceConfig, SliceService,
};

fn next(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn generate(parent: &str, name: &str, addons: u32, per: u32, seed: u64) {
    let inst = SyntheticInstall::new_forever(parent.as_ref(), name);
    let mut s = seed | 1;
    let mut total = 0u64;
    for a in 0..addons {
        let dir = inst.addons.join(format!("Lab{a:04}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("Lab{a:04}.toc")),
            format!("## Interface: 11500\n## Title: Lab{a}\n## Version: 1.{a}\nmain.lua\n"),
        )
        .unwrap();
        for f in 0..per {
            let size = 15_360 + next(&mut s) % (200 * 1024 - 15_360);
            let mut buf = vec![0u8; size as usize];
            for c in buf.chunks_mut(8) {
                let v = next(&mut s).to_le_bytes();
                c.copy_from_slice(&v[..c.len()]);
            }
            total += size;
            std::fs::write(dir.join(format!("f{f:04}.lua")), &buf).unwrap();
        }
    }
    set_old_mtimes_under(&inst.root);
    println!(
        "generated {} addons, {} files, {} MiB",
        addons,
        addons * (per + 1),
        total >> 20
    );
}

fn run(a: &[String]) {
    let (root, shared, state, name) = (&a[0], &a[1], &a[2], &a[3]);
    let passes: usize = a[4].parse().unwrap();
    std::fs::create_dir_all(shared).unwrap();
    std::fs::create_dir_all(state).unwrap();
    let cfg = ServiceConfig {
        install_root: root.into(),
        variant_folder: "_forever_".into(),
        shared_root: shared.into(),
        state_dir: state.into(),
        display_name: name.clone(),
        app_version: "0.0.0-lab".into(),
        recent_ms: 2000,
    };
    let (src, _act) = ScriptedActivity::new(Activity::NotRunning);
    let mut svc = SliceService::with_activity(cfg, Box::new(src)).unwrap();
    let cache = HashCache::open(state.as_ref(), "mem-lab");
    svc.set_read_ctl(ScanCtl {
        cache: Some(cache),
        ..ScanCtl::default()
    });
    if !svc.is_enrolled() {
        let p = svc.inspect();
        let join = match a.get(5) {
            Some(g) => JoinChoice::Existing(GroupId::parse(g).unwrap()),
            None => JoinChoice::New {
                name: "MemLab".into(),
            },
        };
        let g = svc
            .enroll(&EnrollRequest {
                join,
                identity_confirmed: true,
                scope_confirmed: true,
                digest: p.digest,
                display_name: name.clone(),
            })
            .unwrap();
        println!("group {}", g.as_str());
    }
    for i in 0..passes {
        let t = Instant::now();
        let r = svc.sync_once().unwrap();
        println!(
            "pass {i}: {} ms status={:?}",
            t.elapsed().as_millis(),
            r.status.headline
        );
        std::thread::sleep(Duration::from_millis(300));
    }
    let _ = SystemTime::now();
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a[0].as_str() {
        "gen" => generate(
            &a[1],
            &a[2],
            a[3].parse().unwrap(),
            a[4].parse().unwrap(),
            a.get(5).and_then(|s| s.parse().ok()).unwrap_or(7),
        ),
        "run" => run(&a[1..]),
        _ => panic!("gen | run"),
    }
}
