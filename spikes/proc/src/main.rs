//! S-proc spike: process detection by image path, Restart Manager, file stability,
//! launch-during-write race, polling cost. Windows only. Synthetic data only.
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::RestartManager::*;
use windows_sys::Win32::System::Threading::*;

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

// ---------- process enumeration by image path ----------

#[derive(Clone, Debug)]
struct Proc {
    pid: u32,
    exe: String,
    path: Option<PathBuf>,
}

fn list_procs() -> (Vec<Proc>, usize) {
    list_procs_with(false)
}

/// `names_only`: resolve the image path only for processes whose image name is a known
/// client/launcher name (much cheaper; misses renamed executables).
fn list_procs_with(names_only: bool) -> (Vec<Proc>, usize) {
    let mut out = Vec::new();
    let mut unreadable = 0;
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return (out, 0);
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
            let exe = String::from_utf16_lossy(&e.szExeFile[..len]);
            let mut path = None;
            let known = !names_only || {
                let l = exe.to_lowercase();
                CLIENT_NAMES.contains(&l.as_str()) || LAUNCHER_NAMES.contains(&l.as_str())
            };
            let h = if known { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, e.th32ProcessID) } else { std::ptr::null_mut() };
            if !h.is_null() {
                let mut buf = vec![0u16; 1024];
                let mut n = buf.len() as u32;
                if QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut n) != 0 {
                    path = Some(PathBuf::from(OsString::from_wide(&buf[..n as usize])));
                }
                CloseHandle(h);
            }
            if path.is_none() && known && e.th32ProcessID > 4 {
                unreadable += 1;
            }
            out.push(Proc { pid: e.th32ProcessID, exe, path });
            ok = Process32NextW(snap, &mut e);
        }
        CloseHandle(snap);
    }
    (out, unreadable)
}

fn norm(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\").to_lowercase();
    s.strip_prefix("\\\\?\\").unwrap_or(&s).trim_end_matches('\\').to_string()
}

#[derive(Debug, PartialEq, Clone)]
enum Kind {
    Client,
    Launcher,
    Other,
}

const CLIENT_NAMES: &[&str] = &["wow.exe", "wowclassic.exe", "wowt.exe", "wowb.exe", "wowclassict.exe", "wowclassicb.exe"];
const LAUNCHER_NAMES: &[&str] = &[
    "battle.net.exe",
    "agent.exe",
    "world of warcraft launcher.exe",
    "blizzarderror.exe",
    "blizzardbrowser.exe",
    "wowvoiceproxy.exe",
];

/// Returns (variant directory, kind) when the process image lies under `root`.
fn classify(p: &Proc, root: &Path) -> Option<(String, Kind)> {
    let path = norm(p.path.as_ref()?);
    let r = norm(root);
    let rest = path.strip_prefix(&r)?.strip_prefix('\\')?;
    let comps: Vec<&str> = rest.split('\\').collect();
    let variant = if comps.len() >= 2 { comps[0].to_string() } else { String::new() };
    let name = comps.last()?.to_string();
    let kind = if CLIENT_NAMES.contains(&name.as_str()) {
        Kind::Client
    } else if LAUNCHER_NAMES.contains(&name.as_str()) {
        Kind::Launcher
    } else {
        Kind::Other
    };
    Some((variant, kind))
}

fn clients_under(root: &Path) -> Vec<(u32, String)> {
    list_procs_with(true)
        .0
        .iter()
        .filter_map(|p| classify(p, root).filter(|(_, k)| *k == Kind::Client).map(|(v, _)| (p.pid, v)))
        .collect()
}

// ---------- stand-ins ----------

fn temp_root() -> PathBuf {
    std::env::temp_dir().join("wowsync-proc")
}

fn spawn_standin(dest: &Path, args: &[&str]) -> Child {
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    if !dest.exists() {
        fs::copy(std::env::current_exe().unwrap(), dest).unwrap();
    }
    Command::new(dest).args(args).stdout(Stdio::piped()).spawn().expect("spawn standin")
}

fn wait_ready(c: &mut Child) {
    let mut line = String::new();
    BufReader::new(c.stdout.as_mut().unwrap()).read_line(&mut line).unwrap();
    assert!(line.starts_with("ready"), "standin said {line:?}");
}

fn standin_main(args: &[String]) {
    match args[0].as_str() {
        "idle" => {
            println!("ready");
            std::io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(args[1].parse().unwrap()));
        }
        "hold" => {
            let share = match args[2].as_str() {
                "none" => 0,
                "read" => FILE_SHARE_READ,
                "rw" => FILE_SHARE_READ | FILE_SHARE_WRITE,
                _ => FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            };
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .share_mode(share)
                .open(&args[1])
                .expect("hold open");
            f.write_all(b"x").unwrap();
            println!("ready");
            std::io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(args[3].parse().unwrap()));
            drop(f);
        }
        _ => {}
    }
}

fn stop(mut c: Child) {
    let _ = c.kill();
    let _ = c.wait();
}

// ---------- Restart Manager ----------

fn rm_holders(file: &Path) -> Result<Vec<(u32, String)>, u32> {
    unsafe {
        let mut h = 0u32;
        let mut key = [0u16; 33];
        let r = RmStartSession(&mut h, 0, key.as_mut_ptr());
        if r != 0 {
            return Err(r);
        }
        let w = wide(file.as_os_str());
        let files = [w.as_ptr()];
        let mut res = RmRegisterResources(h, 1, files.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null());
        let mut out = Vec::new();
        if res == 0 {
            let (mut need, mut have, mut reasons) = (0u32, 0u32, 0u32);
            res = RmGetList(h, &mut need, &mut have, std::ptr::null_mut(), &mut reasons);
            if res == ERROR_MORE_DATA || (res == 0 && need > 0) {
                let mut v: Vec<RM_PROCESS_INFO> = vec![std::mem::zeroed(); need as usize];
                have = need;
                res = RmGetList(h, &mut need, &mut have, v.as_mut_ptr(), &mut reasons);
                if res == 0 {
                    for i in v.iter().take(have as usize) {
                        let n = i.strAppName.iter().position(|&c| c == 0).unwrap_or(0);
                        out.push((i.Process.dwProcessId, String::from_utf16_lossy(&i.strAppName[..n])));
                    }
                }
            }
        }
        RmEndSession(h);
        if res == 0 {
            Ok(out)
        } else {
            Err(res)
        }
    }
}

// ---------- exclusive-open probe ----------

/// Ok(true) = nobody has it open; Ok(false) = sharing violation; Err = other.
fn exclusive_probe(file: &Path) -> Result<bool, u32> {
    unsafe {
        let w = wide(file.as_os_str());
        let h = CreateFileW(w.as_ptr(), GENERIC_READ, 0, std::ptr::null(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, std::ptr::null_mut());
        if h == INVALID_HANDLE_VALUE {
            let e = GetLastError();
            return if e == ERROR_SHARING_VIOLATION || e == ERROR_LOCK_VIOLATION { Ok(false) } else { Err(e) };
        }
        CloseHandle(h);
        Ok(true)
    }
}

fn meta(file: &Path) -> Option<(u64, std::time::SystemTime)> {
    fs::metadata(file).ok().and_then(|m| Some((m.len(), m.modified().ok()?)))
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn cpu_secs() -> f64 {
    unsafe {
        let (mut c, mut e, mut k, mut u) = (std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed());
        GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u);
        let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) as f64 / 1e7;
        t(k) + t(u)
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

// ---------- 2a ----------

fn t2a() {
    println!("== 2a process detection by image path ==");
    let t = temp_root().join("t2a");
    let _ = fs::remove_dir_all(&t);
    let a = t.join("installA");
    let cases = [
        ("A forever Wow.exe (match)", a.join("_forever_").join("Wow.exe")),
        ("A classic WowClassic.exe (match, other variant)", a.join("_classic_era_").join("WowClassic.exe")),
        ("A launcher (match, launcher kind)", a.join("World of Warcraft Launcher.exe")),
        ("sibling prefix installA_evil Wow.exe (must NOT match)", t.join("installA_evil").join("_forever_").join("Wow.exe")),
        ("other install B Wow.exe (must NOT match)", t.join("installB").join("_forever_").join("Wow.exe")),
        ("A non-client helper tool.exe (Other)", a.join("_forever_").join("tool.exe")),
    ];
    let mut kids = Vec::new();
    for (label, p) in &cases {
        let mut c = spawn_standin(p, &["idle", "20"]);
        wait_ready(&mut c);
        kids.push((label, c));
    }
    let (procs, unreadable) = list_procs();
    println!("enumerated {} processes, {} not openable with QUERY_LIMITED (system/protected)", procs.len(), unreadable);
    for (label, c) in &kids {
        let p = procs.iter().find(|p| p.pid == c.id()).unwrap();
        let by_path = classify(p, &a);
        let by_name = CLIENT_NAMES.contains(&p.exe.to_lowercase().as_str());
        println!("  {label}: pid {} name-only-match={by_name} path-match-under-A={by_path:?}", c.id());
    }
    let hits = clients_under(&a);
    println!("clients_under(installA) = {hits:?}");
    let name_only = procs.iter().filter(|p| CLIENT_NAMES.contains(&p.exe.to_lowercase().as_str())).count();
    println!("name-only detection would flag {name_only} processes (includes decoys); path detection flags {}", hits.len());
    // case-insensitive root
    let upper = PathBuf::from(a.to_string_lossy().to_uppercase());
    println!("case-insensitive root match count = {}", clients_under(&upper).len());
    // real processes: any running under the real WoW root? (read-only enumeration, no touching)
    let real = Path::new(r"D:\Games\World of Warcraft");
    let real_hits: Vec<_> = list_procs().0.iter().filter_map(|p| classify(p, real).map(|(v, k)| (p.exe.clone(), v, format!("{k:?}")))).collect();
    println!("real install root: {} process(es) with image under it at this moment: {:?}", real_hits.len(), real_hits);
    for (_, c) in kids {
        stop(c);
    }
}

// ---------- 2b ----------

fn t2b() {
    println!("== 2b Restart Manager ==");
    let t = temp_root().join("t2b");
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    let hold_exe = t.join("_forever_").join("Wow.exe");
    for share in ["none", "read", "rw", "rwd"] {
        let f = t.join(format!("held_{share}.dat"));
        let mut c = spawn_standin(&hold_exe, &["hold", f.to_str().unwrap(), share, "15"]);
        wait_ready(&mut c);
        let t0 = Instant::now();
        let r = rm_holders(&f);
        let dt = t0.elapsed();
        let probe = exclusive_probe(&f);
        println!("  held share={share} holder pid={} -> RM={r:?} ({:.1} ms) exclusive_probe={probe:?}", c.id(), ms(dt));
        stop(c);
        thread::sleep(Duration::from_millis(100));
        println!("    after holder exit: RM={:?} probe={:?}", rm_holders(&f), exclusive_probe(&f));
    }
    // control: unheld file, nonexistent file, and directory-level resource
    let f = t.join("unheld.dat");
    fs::write(&f, b"x").unwrap();
    println!("  control unheld RM={:?}", rm_holders(&f));
    println!("  control missing RM={:?}", rm_holders(&t.join("missing.dat")));
    // read-only handle held by the *current* process with all sharing (typical antivirus/indexer)
    let g = t.join("reader.dat");
    fs::write(&g, b"x").unwrap();
    let h = fs::OpenOptions::new().read(true).share_mode(7).open(&g).unwrap();
    println!("  held read-only by self with full sharing: RM={:?} probe={:?}", rm_holders(&g), exclusive_probe(&g));
    drop(h);
}

// ---------- 2c ----------

fn probe_ok(f: &Path) -> bool {
    matches!(exclusive_probe(f), Ok(true))
}

/// Declares "stable" per rule; returns the instant of declaration.
fn detector(f: PathBuf, use_meta: bool, use_probe: bool, poll: Duration, quiesce: Duration, deadline: Instant) -> Option<Instant> {
    let mut last = meta(&f);
    let mut since = Instant::now();
    loop {
        thread::sleep(poll);
        let now = Instant::now();
        if now > deadline {
            return None;
        }
        let m = meta(&f);
        if m != last {
            last = m;
            since = now;
        }
        let meta_ok = !use_meta || now.duration_since(since) >= quiesce;
        let probe_ok = !use_probe || probe_ok(&f);
        if meta_ok && probe_ok {
            return Some(Instant::now());
        }
    }
}

fn t2c() {
    println!("== 2c file stability heuristic ==");
    let t = temp_root().join("t2c");
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    let poll = Duration::from_millis(100);
    let quiesce = Duration::from_millis(500);
    for scenario in ["hold-and-append", "open-write-close-bursts", "silent-hold"] {
        let f = t.join(format!("{scenario}.dat"));
        fs::write(&f, b"0").unwrap();
        let t_end: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let writer = {
            let (f, t_end, sc) = (f.clone(), t_end.clone(), scenario);
            thread::spawn(move || {
                let mut fails = 0u32;
                match sc {
                    "hold-and-append" => {
                        let mut h = fs::OpenOptions::new().append(true).share_mode(3).open(&f).unwrap();
                        for _ in 0..8 {
                            h.write_all(b"x").unwrap();
                            thread::sleep(Duration::from_millis(200));
                        }
                    }
                    "open-write-close-bursts" => {
                        for _ in 0..6 {
                            loop {
                                match fs::OpenOptions::new().append(true).share_mode(1).open(&f) {
                                    Ok(mut h) => {
                                        h.write_all(b"x").unwrap();
                                        break;
                                    }
                                    Err(_) => {
                                        fails += 1;
                                        thread::sleep(Duration::from_millis(1));
                                    }
                                }
                            }
                            thread::sleep(Duration::from_millis(300));
                        }
                    }
                    _ => {
                        let mut h = fs::OpenOptions::new().append(true).share_mode(3).open(&f).unwrap();
                        h.write_all(b"x").unwrap();
                        thread::sleep(Duration::from_millis(1800));
                    }
                }
                *t_end.lock().unwrap() = Some(Instant::now());
                fails
            })
        };
        thread::sleep(Duration::from_millis(50));
        let deadline = Instant::now() + Duration::from_secs(6);
        let dets: Vec<(&str, thread::JoinHandle<Option<Instant>>)> = [("meta-only", true, false), ("probe-only", false, true), ("meta+probe", true, true)]
            .iter()
            .map(|&(n, m, p)| {
                let f = f.clone();
                (n, thread::spawn(move || detector(f, m, p, poll, quiesce, deadline)))
            })
            .collect();
        let fails = writer.join().unwrap();
        let end = t_end.lock().unwrap().unwrap();
        println!("  scenario {scenario}: writer open failures caused by detectors/other = {fails}");
        for (n, d) in dets {
            match d.join().unwrap() {
                Some(i) => {
                    let delta = if i >= end { ms(i - end) } else { -ms(end - i) };
                    let verdict = if delta < 0.0 { "PREMATURE (false idle)" } else { "ok" };
                    println!("    {n}: declared stable at {delta:+.0} ms relative to writer final close -> {verdict}");
                }
                None => println!("    {n}: never declared stable"),
            }
        }
    }

    println!("  -- can the probe itself disturb a writer? (writer: open-append-close loop, share=read, 3 s) --");
    let f = t.join("disturb.dat");
    for variant in ["no probe", "exclusive probe hot loop", "exclusive probe every 100 ms", "Restart Manager every 5 ms"] {
        fs::write(&f, b"0").unwrap();
        let stopf = Arc::new(AtomicBool::new(false));
        let prober = {
            let (f, s) = (f.clone(), stopf.clone());
            let v = variant.to_string();
            thread::spawn(move || {
                let mut n = 0u64;
                while !s.load(Ordering::Relaxed) {
                    match v.as_str() {
                        "exclusive probe hot loop" => {
                            let _ = exclusive_probe(&f);
                        }
                        "exclusive probe every 100 ms" => {
                            let _ = exclusive_probe(&f);
                            thread::sleep(Duration::from_millis(100));
                        }
                        "Restart Manager every 5 ms" => {
                            let _ = rm_holders(&f);
                            thread::sleep(Duration::from_millis(5));
                        }
                        _ => thread::sleep(Duration::from_millis(50)),
                    }
                    n += 1;
                }
                n
            })
        };
        let (mut attempts, mut fails) = (0u64, 0u64);
        let end = Instant::now() + Duration::from_secs(3);
        while Instant::now() < end {
            attempts += 1;
            match fs::OpenOptions::new().append(true).share_mode(1).open(&f) {
                Ok(mut h) => {
                    let _ = h.write_all(b"x");
                }
                Err(_) => fails += 1,
            }
        }
        stopf.store(true, Ordering::Relaxed);
        let n = prober.join().unwrap();
        println!("    {variant}: writer attempts={attempts} failed opens={fails} (probe iterations={n})");
    }
}

// ---------- 2d ----------

struct RaceRes {
    safe: u32,
    ambiguous: u32,
    violating: u32,
    aborted: bool,
    max_check_to_commit_us: u128,
}

fn race_trial(mode: &str, offset_ms: u64, work: &Path, client_exe: &Path, root: &Path) -> RaceRes {
    let _ = fs::remove_dir_all(work);
    fs::create_dir_all(work).unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    let stopw = Arc::new(AtomicBool::new(false));
    let watcher = if mode == "poll 1000 ms flag" {
        let (flag, stopw, root) = (flag.clone(), stopw.clone(), root.to_path_buf());
        Some(thread::spawn(move || {
            while !stopw.load(Ordering::Relaxed) {
                if !clients_under(&root).is_empty() {
                    flag.store(true, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_millis(1000));
            }
        }))
    } else {
        None
    };
    let (work2, mode2, root2, flag2) = (work.to_path_buf(), mode.to_string(), root.to_path_buf(), flag.clone());
    let applier = thread::spawn(move || {
        let mut commits = Vec::new();
        let mut aborted = false;
        let mut max_gap = 0u128;
        for i in 0..8 {
            let stage = work2.join(format!("f{i}.stage"));
            fs::write(&stage, b"new").unwrap();
            thread::sleep(Duration::from_millis(25));
            let chk = Instant::now();
            let blocked = match mode2.as_str() {
                "per-commit recheck" => !clients_under(&root2).is_empty(),
                "poll 1000 ms flag" => flag2.load(Ordering::Relaxed),
                _ => false,
            };
            if blocked {
                aborted = true;
                break;
            }
            let t0 = Instant::now();
            fs::rename(&stage, work2.join(format!("f{i}.dat"))).unwrap();
            let t1 = Instant::now();
            max_gap = max_gap.max(t1.duration_since(chk).as_micros());
            commits.push((t0, t1));
        }
        (commits, aborted, max_gap)
    });
    thread::sleep(Duration::from_millis(offset_ms));
    let before = Instant::now();
    let child = spawn_standin(client_exe, &["idle", "5"]);
    let after = Instant::now();
    let (commits, aborted, gap) = applier.join().unwrap();
    stopw.store(true, Ordering::Relaxed);
    stop(child);
    if let Some(w) = watcher {
        let _ = w.join();
    }
    let mut r = RaceRes { safe: 0, ambiguous: 0, violating: 0, aborted, max_check_to_commit_us: gap };
    for (t0, t1) in commits {
        if t1 < before {
            r.safe += 1;
        } else if t0 > after {
            r.violating += 1;
        } else {
            r.ambiguous += 1;
        }
    }
    r
}

fn t2d() {
    println!("== 2d launch-during-write race (8-file staged write, ~25 ms/file, client stand-in spawned at offsets 0..~200 ms) ==");
    let t = temp_root().join("t2d");
    let _ = fs::remove_dir_all(&t);
    let root = t.join("install");
    let exe = root.join("_forever_").join("WowClassic.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    println!("  (commit classes: safe = rename finished before client process was created; violating = rename began after CreateProcess returned; ambiguous = overlapping)");
    for mode in ["no check", "poll 1000 ms flag", "per-commit recheck"] {
        let (mut s, mut a, mut v, mut ab, mut gap) = (0, 0, 0, 0, 0u128);
        let trials = 20;
        let mut tr_viol = 0;
        for k in 0..trials {
            let r = race_trial(mode, 5 + k * 10, &t.join("work"), &exe, &root);
            s += r.safe;
            a += r.ambiguous;
            v += r.violating;
            ab += r.aborted as u32;
            gap = gap.max(r.max_check_to_commit_us);
            if r.violating > 0 || r.ambiguous > 0 {
                tr_viol += 1;
            }
        }
        println!("  mode '{mode}': {trials} trials: commits safe={s} ambiguous={a} violating={v}; trials aborted={ab}; trials with any non-safe commit={tr_viol}; max check->rename-complete window={gap} us");
    }
}

// ---------- 3 ----------

fn t3() {
    println!("== 3 notification approach: polling latency and CPU ==");
    let t = temp_root().join("t3");
    let _ = fs::remove_dir_all(&t);
    let root = t.join("install");
    let exe = root.join("_forever_").join("Wow.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let mut rng = Rng(Instant::now().elapsed().as_nanos() as u64 ^ 0x9E3779B97F4A7C15 ^ std::process::id() as u64);
    for interval in [1000u64, 250, 100] {
        let mut lat = Vec::new();
        for _ in 0..10 {
            let stopf = Arc::new(AtomicBool::new(false));
            let found: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
            let th = {
                let (s, fnd, root) = (stopf.clone(), found.clone(), root.clone());
                thread::spawn(move || {
                    while !s.load(Ordering::Relaxed) {
                        if !clients_under(&root).is_empty() {
                            *fnd.lock().unwrap() = Some(Instant::now());
                            return;
                        }
                        thread::sleep(Duration::from_millis(interval));
                    }
                })
            };
            thread::sleep(Duration::from_millis(rng.next(interval) + 20));
            let c = spawn_standin(&exe, &["idle", "5"]);
            let after = Instant::now();
            let _ = th.join_timeout(&stopf, Duration::from_secs(3));
            if let Some(i) = *found.lock().unwrap() {
                lat.push(ms(i.duration_since(after)));
            }
            stop(c);
        }
        lat.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = lat.iter().sum::<f64>() / lat.len().max(1) as f64;
        println!("  poll {interval} ms: detection latency after CreateProcess returned: n={} min={:.0} mean={:.0} max={:.0} ms", lat.len(), lat.first().unwrap_or(&0.0), mean, lat.last().unwrap_or(&0.0));
    }
    // enumeration cost and CPU
    let n = list_procs().0.len();
    let t0 = Instant::now();
    for _ in 0..50 {
        let _ = clients_under(&root);
    }
    println!("  one names-only scan + path resolution over {n} processes: {:.2} ms avg (50 runs)", ms(t0.elapsed()) / 50.0);
    let t0 = Instant::now();
    for _ in 0..50 {
        let _ = list_procs();
    }
    println!("  full scan (resolve path of every process): {:.2} ms avg", ms(t0.elapsed()) / 50.0);
    for (interval, secs, full) in [(1000u64, 30u64, false), (100, 10, false), (1000, 30, true)] {
        let c0 = cpu_secs();
        let w0 = Instant::now();
        let mut iters = 0;
        while w0.elapsed() < Duration::from_secs(secs) {
            if full {
                let _ = list_procs();
            } else {
                let _ = clients_under(&root);
            }
            iters += 1;
            thread::sleep(Duration::from_millis(interval));
        }
        let cpu = cpu_secs() - c0;
        let kind = if full { "full scan" } else { "names-only scan" };
        println!("  CPU polling ({kind}) every {interval} ms for {secs} s: {iters} polls, {:.3} s CPU = {:.3}% of one core (this process, user+kernel)", cpu, cpu / w0.elapsed().as_secs_f64() * 100.0);
    }
}

trait JoinTimeout {
    fn join_timeout(self, stop: &AtomicBool, d: Duration) -> bool;
}
impl JoinTimeout for thread::JoinHandle<()> {
    fn join_timeout(self, stop: &AtomicBool, d: Duration) -> bool {
        let end = Instant::now() + d;
        while !self.is_finished() && Instant::now() < end {
            thread::sleep(Duration::from_millis(5));
        }
        stop.store(true, Ordering::Relaxed);
        self.join().is_ok()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: wowsync-proc-spike suite|t2a|t2b|t2c|t2d|t3 (idle/hold are stand-in modes)");
        return;
    }
    match args[0].as_str() {
        "idle" | "hold" => standin_main(&args),
        "t2a" => t2a(),
        "t2b" => t2b(),
        "t2c" => t2c(),
        "t2d" => t2d(),
        "t3" => t3(),
        "suite" => {
            t2a();
            t2b();
            t2c();
            t2d();
            t3();
        }
        _ => eprintln!("unknown"),
    }
}
