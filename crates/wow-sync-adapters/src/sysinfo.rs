//! Clock, random identifiers and the two environment facts the diagnostics export reports.

/// The OS version as (major, minor, build), or `None` when it cannot be read. Windows asks
/// `RtlGetVersion` (not `GetVersionEx`, which reports a manifest-dependent value); macOS reads
/// `sw_vers -productVersion` and reports (major, minor, patch); other systems report `None`.
pub fn os_version() -> Option<(u32, u32, u32)> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct OsVersionInfo {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            csd: [u16; 128],
        }
        #[link(name = "ntdll")]
        extern "system" {
            fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
        }
        let mut info = OsVersionInfo {
            size: std::mem::size_of::<OsVersionInfo>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform: 0,
            csd: [0; 128],
        };
        // SAFETY: `info` is a correctly sized, initialised OSVERSIONINFOW for the call.
        let status = unsafe { RtlGetVersion(&mut info) };
        (status == 0).then_some((info.major, info.minor, info.build))
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| parse_version(&String::from_utf8_lossy(&out.stdout)))
            .flatten()
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        None
    }
}

/// "14.5" or "14.5.1" as (14, 5, 0) or (14, 5, 1); anything else is `None`.
#[cfg(any(test, target_os = "macos"))]
fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    parts.next().is_none().then_some((major, minor, patch))
}

/// This process's private memory in MiB, rounded up (the background budget is private resident
/// memory). Windows reads `PrivateUsage`; other systems report `None` because no honest
/// private-memory figure is available there without a new dependency.
pub fn process_private_mib() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        // SAFETY: zeroed is a valid initial value; the size passed matches the struct.
        let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        // SAFETY: `c` starts with a PROCESS_MEMORY_COUNTERS and is `cb` bytes long.
        let ok = unsafe {
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                (&mut c as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
                c.cb,
            )
        };
        (ok != 0).then(|| (c.PrivateUsage as u64).div_ceil(1024 * 1024))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn now_unix_ms() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

/// 16 random bytes as 32 lowercase hex characters. Panics if the OS random
/// source fails; an unpredictable id cannot be replaced by a weaker one.
pub fn random_id_hex() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("operating system random source failed");
    crate::hex(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_text_parses_and_rejects_garbage() {
        assert_eq!(parse_version("14.5\n"), Some((14, 5, 0)));
        assert_eq!(parse_version("15.1.2"), Some((15, 1, 2)));
        for bad in ["", "x.y", "14.", "1.2.3.4", "14.5-beta"] {
            assert_eq!(parse_version(bad), None, "{bad:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_reports_a_real_version_and_nonzero_private_memory() {
        let (major, _, build) = os_version().expect("RtlGetVersion works");
        assert!(major >= 10 && build > 0);
        assert!(process_private_mib().expect("memory is readable") >= 1);
    }

    #[test]
    fn random_ids_are_32_hex_and_distinct() {
        let ids: Vec<String> = (0..64).map(|_| random_id_hex()).collect();
        for id in &ids {
            assert_eq!(id.len(), 32);
            assert!(id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        }
        let mut s = ids.clone();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), ids.len());
        assert!(now_unix_ms() > 1_700_000_000_000);
    }
}
