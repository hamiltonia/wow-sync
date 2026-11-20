//! Platform defaults that have no Windows counterpart: the pure parts (paths, parsing) are shared
//! so they are tested on every platform; the parts that ask the operating system are
//! `cfg(not(windows))` and are exercised on macOS only.

use std::path::{Path, PathBuf};

/// `~/Library/Application Support/WoW Sync/state`, the standard macOS place for app data.
#[cfg_attr(windows, allow(dead_code))]
pub fn mac_state_dir(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("WoW Sync")
        .join("state")
}

/// Minutes east of UTC from `date +%z` output such as `+0530` or `-0800`.
#[cfg_attr(windows, allow(dead_code))]
pub fn parse_utc_offset(z: &str) -> Option<i32> {
    let z = z.trim();
    let (sign, digits) = match z.as_bytes().first()? {
        b'+' => (1, &z[1..]),
        b'-' => (-1, &z[1..]),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let h: i32 = digits[..2].parse().ok()?;
    let m: i32 = digits[2..].parse().ok()?;
    (h < 24 && m < 60).then_some(sign * (h * 60 + m))
}

/// The per-instance socket a second launch connects to, to ask the first to show its window.
#[cfg_attr(windows, allow(dead_code))]
pub fn show_socket_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("wowsync-{key}.sock"))
}

#[cfg(not(windows))]
pub fn host_name() -> Option<String> {
    let run = |prog: &str, args: &[&str]| {
        std::process::Command::new(prog)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    run("scutil", &["--get", "ComputerName"]).or_else(|| run("hostname", &[]))
}

#[cfg(not(windows))]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(not(windows))]
pub fn local_offset_minutes() -> i32 {
    std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .and_then(|o| parse_utc_offset(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mac_state_folder_is_under_application_support() {
        let p = mac_state_dir(Path::new("/Users/ann"));
        assert!(p.ends_with("Library/Application Support/WoW Sync/state"));
        assert!(p.starts_with("/Users/ann"));
    }

    #[test]
    fn utc_offsets_parse_and_bad_text_is_refused() {
        assert_eq!(parse_utc_offset("+0530\n"), Some(330));
        assert_eq!(parse_utc_offset("-0800"), Some(-480));
        assert_eq!(parse_utc_offset("+0000"), Some(0));
        for bad in ["", "0530", "+53", "+2460", "+2500", "+05:30", "-ab12"] {
            assert_eq!(parse_utc_offset(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_second_launch_socket_is_per_instance_key() {
        let a = show_socket_path(Path::new("/t"), "abc");
        assert_ne!(a, show_socket_path(Path::new("/t"), "abd"));
        assert!(a.to_string_lossy().ends_with("wowsync-abc.sock"));
    }
}
