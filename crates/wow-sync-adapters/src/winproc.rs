//! WoW client observation by image path (M-06). Pure classification plus a
//! Windows process snapshot. No process is started, opened for write or killed.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcEntry {
    pub pid: u32,
    /// Toolhelp `szExeFile`, available even for elevated or other-session processes.
    pub exe_name: String,
    /// `QueryFullProcessImageNameW` result or the reason it failed.
    pub image: Result<PathBuf, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessActivity {
    NotRunning,
    Running {
        pids: Vec<u32>,
        images: Vec<PathBuf>,
    },
    Unknown(String),
}

/// Lowercased, '\'-separated, verbatim prefix removed, `.`/`..` resolved.
fn normalize(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    let (unc, s) = if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        (true, rest.to_string())
    } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
        (false, rest.to_string())
    } else if let Some(rest) = s.strip_prefix("\\\\") {
        (true, rest.to_string())
    } else {
        (false, s)
    };
    let mut parts: Vec<String> = Vec::new();
    for seg in s.split('\\') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other.to_lowercase()),
        }
    }
    let joined = parts.join("\\");
    if unc {
        format!("\\\\{joined}")
    } else {
        joined
    }
}

/// Is `image` strictly inside `variant_dir` (case-insensitive, separator-delimited)?
pub fn image_is_under(variant_dir: &Path, image: &Path) -> bool {
    let dir = normalize(variant_dir);
    if dir.is_empty() {
        return false;
    }
    let img = normalize(image);
    img.len() > dir.len() + 1 && img.starts_with(&dir) && img.as_bytes()[dir.len()] == b'\\'
}

/// `wow*.exe` (case-insensitive) or any `.exe` listed from the variant folder.
pub fn is_possible_client(exe_name: &str, variant_exes: &[String]) -> bool {
    let n = exe_name.to_lowercase();
    (n.starts_with("wow") && n.ends_with(".exe"))
        || variant_exes.iter().any(|e| e.to_lowercase() == n)
}

/// Pure decision over a process snapshot; the second value counts inaccessible,
/// non-client entries (reported, not blocking).
pub fn classify(
    entries: &[ProcEntry],
    variant_dir: &Path,
    variant_exes: &[String],
) -> (ProcessActivity, u32) {
    let mut pids = Vec::new();
    let mut images = Vec::new();
    let mut unknown: Option<String> = None;
    let mut other_inaccessible = 0u32;
    for e in entries {
        match &e.image {
            Ok(img) => {
                if image_is_under(variant_dir, img) {
                    pids.push(e.pid);
                    images.push(img.clone());
                }
            }
            Err(_) => {
                if is_possible_client(&e.exe_name, variant_exes) {
                    unknown.get_or_insert_with(|| {
                        format!(
                            "possible client {} pid {} cannot be inspected",
                            e.exe_name, e.pid
                        )
                    });
                } else {
                    other_inaccessible += 1;
                }
            }
        }
    }
    if !pids.is_empty() {
        (
            ProcessActivity::Running { pids, images },
            other_inaccessible,
        )
    } else if let Some(msg) = unknown {
        (ProcessActivity::Unknown(msg), other_inaccessible)
    } else {
        (ProcessActivity::NotRunning, other_inaccessible)
    }
}

#[cfg(windows)]
mod sys {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::GetLongPathNameW;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// Expands 8.3 short names so path comparison is reliable; falls back to `p`.
    pub fn long_path(p: &Path) -> PathBuf {
        if !p.to_string_lossy().contains('~') {
            return p.to_path_buf();
        }
        let mut w: Vec<u16> = p.as_os_str().encode_wide().collect();
        w.push(0);
        let mut buf = vec![0u16; 1024];
        // SAFETY: `w` is NUL-terminated; `buf` is writable for `buf.len()` units.
        let n =
            unsafe { GetLongPathNameW(w.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n == 0 || n >= buf.len() {
            p.to_path_buf()
        } else {
            PathBuf::from(OsString::from_wide(&buf[..n]))
        }
    }

    fn image_of(pid: u32) -> Result<PathBuf, String> {
        // SAFETY: OpenProcess has no memory preconditions; the handle is closed below.
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if h.is_null() {
            // SAFETY: thread-local error read.
            return Err(format!("OpenProcess failed: {}", unsafe { GetLastError() }));
        }
        let mut buf = vec![0u16; 32 * 1024];
        let mut len = buf.len() as u32;
        // SAFETY: `buf`/`len` describe a writable buffer; `h` is a valid process handle.
        let ok = unsafe { QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) };
        // SAFETY: `h` is valid and closed exactly once.
        let err = if ok == 0 {
            Some(unsafe { GetLastError() })
        } else {
            None
        };
        unsafe { CloseHandle(h) };
        match err {
            Some(e) => Err(format!("QueryFullProcessImageNameW failed: {e}")),
            None => Ok(long_path(Path::new(&OsString::from_wide(
                &buf[..len as usize],
            )))),
        }
    }

    pub fn snapshot() -> Result<Vec<ProcEntry>, String> {
        // SAFETY: no pointer arguments.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snap == INVALID_HANDLE_VALUE {
            // SAFETY: thread-local error read.
            return Err(format!("CreateToolhelp32Snapshot failed: {}", unsafe {
                GetLastError()
            }));
        }
        // SAFETY: zeroed PROCESSENTRY32W is valid; dwSize is set before use.
        let mut pe: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut out = Vec::new();
        // SAFETY: `snap` is a valid snapshot handle and `pe` is initialized.
        let mut more = unsafe { Process32FirstW(snap, &mut pe) } != 0;
        while more {
            let len = pe
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(pe.szExeFile.len());
            let exe_name = OsString::from_wide(&pe.szExeFile[..len])
                .to_string_lossy()
                .into_owned();
            out.push(ProcEntry {
                pid: pe.th32ProcessID,
                exe_name,
                image: image_of(pe.th32ProcessID),
            });
            // SAFETY: as above.
            more = unsafe { Process32NextW(snap, &mut pe) } != 0;
        }
        // SAFETY: closed exactly once.
        unsafe { CloseHandle(snap) };
        Ok(out)
    }
}

#[cfg(all(windows, any(test, feature = "testkit")))]
pub(crate) fn long_path(p: &Path) -> PathBuf {
    sys::long_path(p)
}

/// Observes whether a client runs from `variant_dir`.
#[cfg(windows)]
pub fn observe_variant(variant_dir: &Path) -> (ProcessActivity, u32) {
    let dir = sys::long_path(variant_dir);
    let exes: Vec<String> = crate::fs_safe::list_dir(&dir)
        .map(|v| {
            v.into_iter()
                .filter(|e| !e.is_dir && e.name.to_lowercase().ends_with(".exe"))
                .map(|e| e.name)
                .collect()
        })
        .unwrap_or_default();
    match sys::snapshot() {
        Ok(entries) => classify(&entries, &dir, &exes),
        Err(e) => (ProcessActivity::Unknown(e), 0),
    }
}

#[cfg(not(windows))]
pub fn observe_variant(_variant_dir: &Path) -> (ProcessActivity, u32) {
    (ProcessActivity::Unknown("unsupported platform".into()), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pid: u32, exe: &str, image: Result<&str, &str>) -> ProcEntry {
        ProcEntry {
            pid,
            exe_name: exe.into(),
            image: image.map(PathBuf::from).map_err(String::from),
        }
    }

    const DIR: &str = "D:\\Games\\World of Warcraft\\_forever_";

    #[test]
    fn image_prefix_requires_separator() {
        let d = Path::new(DIR);
        assert!(image_is_under(
            d,
            Path::new("D:\\Games\\World of Warcraft\\_forever_\\Wow.exe")
        ));
        assert!(image_is_under(
            d,
            Path::new("d:/games/world of warcraft/_FOREVER_/sub/x.exe")
        ));
        assert!(image_is_under(
            d,
            Path::new("\\\\?\\D:\\Games\\World of Warcraft\\_forever_\\Wow.exe")
        ));
        assert!(!image_is_under(
            d,
            Path::new("D:\\Games\\World of Warcraft\\_forever_x\\Wow.exe")
        ));
        assert!(!image_is_under(
            d,
            Path::new("D:\\Games\\World of Warcraft\\_forever_")
        ));
        assert!(!image_is_under(
            d,
            Path::new("D:\\Games\\World of Warcraft\\_retail_\\Wow.exe")
        ));
        assert!(!image_is_under(
            d,
            Path::new("D:\\Games\\World of Warcraft\\_forever_\\..\\_retail_\\Wow.exe")
        ));
        assert!(!image_is_under(Path::new(""), Path::new("D:\\x.exe")));
    }

    #[test]
    fn inaccessible_possible_client_is_unknown() {
        let list = [
            entry(10, "explorer.exe", Ok("C:\\Windows\\explorer.exe")),
            entry(20, "WowClassic.exe", Err("access denied")),
        ];
        let (a, n) = classify(&list, Path::new(DIR), &[]);
        assert_eq!(
            a,
            ProcessActivity::Unknown(
                "possible client WowClassic.exe pid 20 cannot be inspected".into()
            )
        );
        assert_eq!(n, 0);
        // An exe name that exists in the variant folder is also a possible client.
        let list = [entry(30, "Launcher.exe", Err("denied"))];
        assert!(matches!(
            classify(&list, Path::new(DIR), &["launcher.EXE".to_string()]).0,
            ProcessActivity::Unknown(_)
        ));
        assert_eq!(
            classify(&list, Path::new(DIR), &[]).0,
            ProcessActivity::NotRunning
        );
    }

    #[test]
    fn inaccessible_unrelated_process_ignored() {
        let list = [
            entry(4, "System", Err("denied")),
            entry(5, "svchost.exe", Err("denied")),
            entry(6, "notepad.exe", Ok("C:\\Windows\\notepad.exe")),
        ];
        assert_eq!(
            classify(&list, Path::new(DIR), &[]),
            (ProcessActivity::NotRunning, 2)
        );
        assert_eq!(
            classify(&[], Path::new(DIR), &[]),
            (ProcessActivity::NotRunning, 0)
        );
    }

    #[test]
    fn accessible_image_under_variant_is_running() {
        let wow = format!("{DIR}\\Wow.exe");
        let list = [
            entry(7, "other.exe", Ok("C:\\x\\other.exe")),
            entry(8, "Wow.exe", Ok(&wow)),
            entry(9, "wow.exe", Err("denied")),
        ];
        let (a, _) = classify(&list, Path::new(DIR), &[]);
        assert_eq!(
            a,
            ProcessActivity::Running {
                pids: vec![8],
                images: vec![PathBuf::from(&wow)]
            }
        );
    }

    /// A fake client that is starting or exiting cannot be inspected for a moment, which
    /// correctly reports Unknown. Tests that observe processes therefore run one at a time.
    #[cfg(windows)]
    static PROCESS_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A process that is starting or exiting briefly cannot be inspected, which correctly
    /// reports Unknown; wait (bounded) for that to settle. A process that stays
    /// uninspectable still fails the calling assertion.
    #[cfg(windows)]
    fn observe_settled(dir: &Path) -> ProcessActivity {
        let mut a = observe_variant(dir).0;
        for _ in 0..50 {
            if !matches!(a, ProcessActivity::Unknown(_)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            a = observe_variant(dir).0;
        }
        a
    }

    #[cfg(windows)]
    #[test]
    fn fake_client_under_variant_is_running() {
        use crate::testkit::{FakeClient, SyntheticInstall, TempDir};
        let _serial = PROCESS_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "inst");
        assert_eq!(
            observe_settled(&inst.variant_dir),
            ProcessActivity::NotRunning
        );
        let client = FakeClient::start(&inst.variant_dir);
        let (a, _) = observe_variant(&inst.variant_dir);
        match a {
            ProcessActivity::Running { pids, images } => {
                assert!(pids.contains(&client.pid()), "{pids:?}");
                assert!(images.iter().all(|i| image_is_under(&inst.variant_dir, i)));
            }
            other => panic!("expected Running, got {other:?}"),
        }
        drop(client);
        assert_eq!(
            observe_settled(&inst.variant_dir),
            ProcessActivity::NotRunning
        );
    }

    #[cfg(windows)]
    #[test]
    fn process_outside_variant_is_not_running() {
        use crate::testkit::{SyntheticInstall, TempDir};
        let _serial = PROCESS_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "inst");
        let launcher = inst.root.join("Launcher.exe");
        let ping = Path::new(&std::env::var("SystemRoot").unwrap()).join("System32\\ping.exe");
        std::fs::copy(ping, &launcher).unwrap();
        let mut child = std::process::Command::new(&launcher)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let a = observe_settled(&inst.variant_dir);
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(a, ProcessActivity::NotRunning);
    }
}
