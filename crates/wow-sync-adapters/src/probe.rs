//! Read-only probes for detecting where the game and Dropbox live (UG-24): environment
//! reads, directory listings and bounded text reads. Nothing here writes or makes a decision.

use std::io::Read;
use std::path::{Path, PathBuf};

/// Dropbox's `info.json` is a few hundred bytes; anything larger is not trusted.
pub const INFO_JSON_LIMIT: u64 = 64 * 1024;

/// An environment variable that holds a path, if set.
pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Names of the sub-directories of `root`; empty when it cannot be listed.
pub fn subdir_names(root: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

/// Letters of the local fixed drives (never network, removable or optical ones, so a dead
/// share cannot stall detection). Empty off Windows.
#[cfg(windows)]
pub fn fixed_drive_letters() -> Vec<char> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_FIXED: u32 = 3;
    // SAFETY: both calls only read system state; the root string is NUL-terminated.
    let mask = unsafe { GetLogicalDrives() };
    (0..26u32)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| (b'A' + i as u8) as char)
        .filter(|c| {
            let root: Vec<u16> = format!("{c}:\\").encode_utf16().chain([0]).collect();
            unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_FIXED }
        })
        .collect()
}

#[cfg(not(windows))]
pub fn fixed_drive_letters() -> Vec<char> {
    Vec::new()
}

/// The bytes of a plain file of at most `limit` bytes, as untrusted data. `None` when it is
/// absent, unreadable, not a regular file or larger than the limit.
pub fn read_bytes_bounded(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let f = std::fs::File::open(path).ok()?;
    if !f.metadata().ok()?.is_file() {
        return None;
    }
    let mut b = Vec::new();
    f.take(limit + 1).read_to_end(&mut b).ok()?;
    (b.len() as u64 <= limit).then_some(b)
}

/// The text of a plain file of at most `limit` bytes. `None` when it is absent, unreadable,
/// not a regular file or larger than the limit.
pub fn read_text_bounded(path: &Path, limit: u64) -> Option<String> {
    let f = std::fs::File::open(path).ok()?;
    if !f.metadata().ok()?.is_file() {
        return None;
    }
    let mut b = Vec::new();
    f.take(limit + 1).read_to_end(&mut b).ok()?;
    if b.len() as u64 > limit {
        return None;
    }
    Some(String::from_utf8_lossy(&b).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TempDir;

    #[test]
    fn bounded_read_refuses_a_file_over_the_limit() {
        let t = TempDir::new();
        let p = t.join("info.json");
        std::fs::write(&p, vec![b'a'; 100]).unwrap();
        assert_eq!(read_text_bounded(&p, 100).map(|s| s.len()), Some(100));
        assert!(read_text_bounded(&p, 99).is_none());
        assert!(read_text_bounded(&t.join("missing"), 100).is_none());
        assert!(
            read_text_bounded(t.path(), 100).is_none(),
            "a directory is not text"
        );
    }

    #[test]
    fn bounded_byte_read_keeps_arbitrary_bytes_and_refuses_oversize() {
        let t = TempDir::new();
        let p = t.join("product.db");
        std::fs::write(&p, [0u8, 255, 7, 0, 1]).unwrap();
        assert_eq!(read_bytes_bounded(&p, 5), Some(vec![0u8, 255, 7, 0, 1]));
        assert!(read_bytes_bounded(&p, 4).is_none());
        assert!(read_bytes_bounded(&t.join("missing"), 5).is_none());
        assert!(read_bytes_bounded(t.path(), 5).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn the_system_drive_is_a_fixed_drive() {
        let sys = std::env::var("SystemDrive").unwrap_or_default();
        let letter = sys.chars().next().unwrap().to_ascii_uppercase();
        assert!(fixed_drive_letters().contains(&letter));
    }

    #[test]
    fn subdir_names_lists_only_directories() {
        let t = TempDir::new();
        std::fs::create_dir(t.join("_retail_")).unwrap();
        std::fs::write(t.join("file.txt"), b"x").unwrap();
        assert_eq!(subdir_names(t.path()), vec!["_retail_".to_string()]);
        assert!(subdir_names(&t.join("missing")).is_empty());
    }
}
