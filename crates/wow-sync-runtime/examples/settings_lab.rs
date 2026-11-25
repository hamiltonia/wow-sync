//! Settings lab (run 011): drives the real runtime commands of one isolated computer per call.
//!
//!   settings_lab <lab dir> <A|B> <command> [args]
//!
//! The lab dir holds `<X>/` (install root with `_forever_`), `<X>-state/` and `shared/`. Console
//! output replaces account, realm and character names with `acct<N>` / `char<N>`; the numbers
//! come from the sorted names under `<lab>/base`. Set `LAB_WOW_RUNNING=1` to report the game as
//! running. Only the lab dir is touched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use wow_sync_runtime::core::ids::{GroupId, Identity, SettingsUnit};
use wow_sync_runtime::core::paths::validate_folder_name;
use wow_sync_runtime::core::plan::Activity;
use wow_sync_runtime::core::settings::ResumeOption;
use wow_sync_runtime::{
    Command, EnrollRequest, JoinChoice, ScriptedActivity, ServiceConfig, SliceService,
};

fn names(base: &Path) -> (Vec<String>, Vec<(String, String, String)>) {
    let acc_dir = base.join("_forever_/WTF/Account");
    let mut accts: Vec<String> = std::fs::read_dir(&acc_dir)
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    accts.sort();
    let mut chars = Vec::new();
    for a in &accts {
        let mut realms: Vec<String> = std::fs::read_dir(acc_dir.join(a))
            .map(|d| {
                d.flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n != "SavedVariables")
                    .collect()
            })
            .unwrap_or_default();
        realms.sort();
        for r in realms {
            let mut cs: Vec<String> = std::fs::read_dir(acc_dir.join(a).join(&r))
                .map(|d| {
                    d.flatten()
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            cs.sort();
            for c in cs {
                chars.push((a.clone(), r.clone(), c));
            }
        }
    }
    (accts, chars)
}

struct Anon {
    accts: Vec<String>,
    chars: Vec<(String, String, String)>,
}

impl Anon {
    fn unit(&self, u: &SettingsUnit) -> String {
        let a = self
            .accts
            .iter()
            .position(|x| x == u.identity.account_name())
            .map_or("acct?".to_string(), |i| format!("acct{i}"));
        match u.identity.realm_character() {
            None => format!("{} {a}", u.folder.as_str()),
            Some((r, c)) => {
                let i = self
                    .chars
                    .iter()
                    .position(|x| x.0 == u.identity.account_name() && x.1 == r && x.2 == c)
                    .map_or("?".to_string(), |i| i.to_string());
                format!("{} {a}/char{i}", u.folder.as_str())
            }
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let lab = PathBuf::from(&a[0]);
    let who = a[1].as_str();
    let cmd = a[2].as_str();
    let rest = &a[3..];
    let root = lab.join(who);
    let cfg = ServiceConfig {
        install_root: root.clone(),
        variant_folder: "_forever_".into(),
        shared_root: lab.join("shared"),
        state_dir: lab.join(format!("{who}-state")),
        display_name: format!("Lab {who}"),
        app_version: "0.0.0-lab".into(),
        recent_ms: 0,
    };
    std::fs::create_dir_all(&cfg.shared_root).unwrap();
    std::fs::create_dir_all(&cfg.state_dir).unwrap();
    let running = std::env::var("LAB_WOW_RUNNING").is_ok_and(|v| v == "1");
    let (src, _act) = ScriptedActivity::new(if running {
        Activity::Running { pids: vec![4242] }
    } else {
        Activity::NotRunning
    });
    let mut svc = SliceService::with_activity(cfg.clone(), Box::new(src)).unwrap();
    let (accts, chars) = names(&lab.join("base"));
    let anon = Anon { accts, chars };
    let units = |svc: &mut SliceService| svc.status().settings_units;
    let pick = |svc: &mut SliceService, idx: &str| -> SettingsUnit {
        let i: usize = idx.parse().unwrap();
        units(svc)[i].unit.clone()
    };
    match cmd {
        "enroll" => {
            let p = svc.inspect();
            let join = match rest.first().map(String::as_str) {
                Some("join") => JoinChoice::Existing(GroupId::parse(&rest[1]).unwrap()),
                _ => JoinChoice::New {
                    name: "R11 lab".into(),
                },
            };
            let g = svc
                .enroll(&EnrollRequest {
                    join,
                    identity_confirmed: true,
                    scope_confirmed: true,
                    digest: p.digest,
                    display_name: cfg.display_name.clone(),
                })
                .unwrap();
            println!("group {}", g.as_str());
        }
        "sync" => {
            let n: usize = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            for i in 0..n {
                let t = std::time::Instant::now();
                match svc.sync_once() {
                    Ok(r) => {
                        println!(
                            "sync {i}: {} ms headline={:?} apply_note={:?} crashed={}",
                            t.elapsed().as_millis(),
                            r.status.headline,
                            r.apply_note,
                            r.crashed
                        );
                        for l in &r.status.blocks {
                            println!("  block: {l}");
                        }
                    }
                    Err(e) => println!("sync {i}: ERROR {e}"),
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
        "status" => {
            if std::env::var("LAB_SYNC_FIRST").is_ok_and(|v| v == "1") {
                let _ = svc.sync_once();
            }
            let s = svc.status();
            println!("headline: {}", s.headline);
            for p in &s.peers {
                println!("peer {} this={} {:?}", p.name, p.is_this, p.standing);
            }
            let st = &s.settings;
            println!(
                "settings enabled={} master={:?} conflict={} paused={:?} accounts_included={} available={}",
                st.enabled,
                st.master_name,
                st.policy_conflict,
                st.paused.iter().map(|p| p.folder.as_str().to_string()).collect::<Vec<_>>(),
                st.accounts.len(),
                st.available_accounts.len()
            );
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            for (i, u) in s.settings_units.iter().enumerate() {
                *by.entry(format!("{:?}", u.status)).or_default() += 1;
                if rest.first().is_some_and(|x| x == "-v") {
                    println!(
                        "u{i} {} {:?} live={} hist={} share={} restore={} compet={}",
                        anon.unit(&u.unit),
                        u.status,
                        u.live,
                        u.history.len(),
                        u.can_share,
                        u.restore_possible,
                        u.competing.len()
                    );
                }
            }
            println!("units {} by status {:?}", s.settings_units.len(), by);
            println!(
                "to_share {} group_has_shared={}",
                st.to_share.len(),
                st.group_has_shared_settings
            );
            for l in s
                .blocks
                .iter()
                .filter(|l| l.to_lowercase().contains("setting"))
            {
                println!("block: {l}");
            }
        }
        "policy" => {
            let s = svc.status();
            let me = s
                .peers
                .iter()
                .find(|p| p.is_this)
                .map(|p| p.enrollment.clone());
            let other = s
                .peers
                .iter()
                .find(|p| !p.is_this)
                .map(|p| p.enrollment.clone());
            let master = match rest[0].as_str() {
                "me" => me,
                "other" => other,
                _ => None,
            };
            println!(
                "{:?}",
                svc.command(Command::SetSettingsPolicy {
                    master,
                    exceptions: vec![]
                })
            );
        }
        "enable" => {
            let inc: Vec<String> = rest
                .iter()
                .map(|i| anon.accts[i.parse::<usize>().unwrap()].clone())
                .collect();
            println!(
                "{:?}",
                svc.command(Command::EnableSettings {
                    included_accounts: inc
                })
            );
        }
        "pause" => {
            let f = validate_folder_name(&rest[0]).unwrap();
            println!("{:?}", svc.command(Command::PauseSettings { folder: f }));
        }
        "resume" => {
            let f = validate_folder_name(&rest[0]).unwrap();
            let choice = match rest[1].as_str() {
                "resume" => ResumeOption::Resume,
                "keep" => ResumeOption::KeepMineHere,
                "goback" => ResumeOption::GoBackToShared,
                "useshared" => ResumeOption::UseShared,
                "sharedrecover" => ResumeOption::UseSharedKeepMineAsRecovery,
                _ => ResumeOption::StayPaused,
            };
            println!(
                "{:?}",
                svc.command(Command::ResumeSettings { folder: f, choice })
            );
        }
        "share" => {
            let rows = units(&mut svc);
            let r = &rows[rest[0].parse::<usize>().unwrap()];
            println!(
                "{:?}",
                svc.command(Command::ShareSettings {
                    unit: r.unit.clone(),
                    checkpoint: r.latest_capture.unwrap(),
                })
            );
        }
        "sharenow" => {
            let _ = svc.sync_once();
            let units: Vec<_> = svc
                .status()
                .settings
                .to_share
                .iter()
                .map(|o| o.unit.clone())
                .collect();
            println!("sharing {}", units.len());
            println!("{:?}", svc.command(Command::ShareSettingsNow { units }));
        }
        "usesh" => {
            let u = pick(&mut svc, &rest[0]);
            println!("{:?}", svc.command(Command::UseSharedSettings { unit: u }));
        }
        "restore" => {
            let rows = units(&mut svc);
            let r = &rows[rest[0].parse::<usize>().unwrap()];
            let h = &r.history[rest[1].parse::<usize>().unwrap()];
            println!(
                "{:?}",
                svc.command(Command::RestoreSettings {
                    unit: r.unit.clone(),
                    checkpoint: h.checkpoint,
                })
            );
        }
        "choose" => {
            let rows = units(&mut svc);
            let r = &rows[rest[0].parse::<usize>().unwrap()];
            let c = &r.competing[rest[1].parse::<usize>().unwrap()];
            println!(
                "{:?}",
                svc.command(Command::ChooseSettings {
                    unit: r.unit.clone(),
                    checkpoint: c.checkpoint,
                    shown_heads: r.shown_heads.clone(),
                })
            );
        }
        "units" => {
            for (i, u) in units(&mut svc).iter().enumerate() {
                println!("u{i} {}", anon.unit(&u.unit));
            }
        }
        _ => {
            let _ = Identity::account("x");
            panic!("unknown command")
        }
    }
}
