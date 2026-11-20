//! Per-computer app state directory (`%LOCALAPPDATA%\WoWSync`): atomic named
//! files, bounded audit log and a single-instance guard.

use crate::fs_safe::{ReparsePolicy, Root};
use crate::{FsError, FsErrorKind};
use std::fs::File;
use std::io::Write;
use std::path::Path;

const AUDIT_LIMIT: u64 = 1024 * 1024;
pub const NOTICES_FILE: &str = "notices.json";
pub const NOTICES_LIMIT: u64 = 64 * 1024;
pub const UPDATE_CHECK_FILE: &str = "update_check.json";
pub const UPDATE_CHECK_LIMIT: u64 = 4096;

pub struct LocalState {
    root: Root,
}

pub struct InstanceGuard {
    _file: File,
}

impl LocalState {
    pub fn open_default() -> Result<LocalState, FsError> {
        match std::env::var_os("LOCALAPPDATA") {
            Some(base) => LocalState::open_at(&Path::new(&base).join("WoWSync")),
            None => Err(FsError::new(
                FsErrorKind::Unsupported,
                Path::new(""),
                "LOCALAPPDATA is not set",
            )),
        }
    }

    /// Creates `dir` (its parent must exist) if missing.
    pub fn open_at(dir: &Path) -> Result<LocalState, FsError> {
        match std::fs::create_dir(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(FsError::from_io(dir, &e)),
        }
        Ok(LocalState {
            root: Root::open(dir, ReparsePolicy::RejectAll)?,
        })
    }

    pub fn path(&self) -> &Path {
        self.root.path()
    }

    /// None = absent.
    pub fn read(&self, name: &str, limit: u64) -> Result<Option<Vec<u8>>, FsError> {
        let f = match self.root.open_read(name, limit) {
            Ok(f) => f,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let path = self.root.join(name)?;
        let mut b = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(f, limit + 1), &mut b)
            .map_err(|e| FsError::from_io(&path, &e))?;
        if b.len() as u64 > limit {
            return Err(FsError::new(
                FsErrorKind::TooLarge,
                &path,
                "file grew beyond limit",
            ));
        }
        Ok(Some(b))
    }

    /// Temp file, flush, fsync, rename.
    pub fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), FsError> {
        let p = self.root.path().to_path_buf();
        let tmp = format!(".{name}.{}.tmp", crate::sysinfo::random_id_hex());
        self.root
            .write_atomic_with(name, &tmp, &mut |f| {
                f.write_all(bytes).map_err(|e| FsError::from_io(&p, &e))
            })
            .map(|_| ())
    }

    /// Removes a named plain file; already absent is success.
    pub fn remove(&self, name: &str) -> Result<(), FsError> {
        let path = self.root.join(name)?;
        match self.root.check_components(name) {
            Ok(m) if m.is_dir => {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    &path,
                    "is a directory",
                ))
            }
            Ok(_) => {}
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        }
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(FsError::from_io(&path, &e)),
        }
    }

    /// Appends one line to `audit.jsonl`; rotates at 1 MiB to `audit.1.jsonl`
    /// (two files are kept).
    pub fn append_audit(&self, line: &str) -> Result<(), FsError> {
        self.append_audit_limited(line, AUDIT_LIMIT)
    }

    pub(crate) fn append_audit_limited(&self, line: &str, limit: u64) -> Result<(), FsError> {
        let path = self.root.join("audit.jsonl")?;
        if line.contains('\n') || line.contains('\r') {
            return Err(FsError::new(
                FsErrorKind::Io,
                &path,
                "audit line must not contain line breaks",
            ));
        }
        let current = match self.root.check_components("audit.jsonl") {
            Ok(m) if m.is_dir => {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    &path,
                    "audit.jsonl is a directory",
                ))
            }
            Ok(m) => m.size,
            Err(e) if e.kind == FsErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        if current > 0 && current + line.len() as u64 + 1 > limit {
            let rotated = self.root.join("audit.1.jsonl")?;
            // Refuse to rotate over anything that is not a plain file.
            match self.root.check_components("audit.1.jsonl") {
                Ok(m) if m.is_dir => {
                    return Err(FsError::new(
                        FsErrorKind::NotPlainFile,
                        &rotated,
                        "is a directory",
                    ))
                }
                Ok(_)
                | Err(FsError {
                    kind: FsErrorKind::NotFound,
                    ..
                }) => {}
                Err(e) => return Err(e),
            }
            std::fs::rename(&path, &rotated).map_err(|e| FsError::from_io(&path, &e))?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| FsError::from_io(&path, &e))?;
        f.write_all(format!("{line}\n").as_bytes())
            .map_err(|e| FsError::from_io(&path, &e))?;
        f.sync_all().map_err(|e| FsError::from_io(&path, &e))
    }

    /// Exclusive lock on `instance.lock`; held until the guard drops.
    pub fn acquire_instance(&self) -> Result<InstanceGuard, FsError> {
        let path = self.root.join("instance.lock")?;
        Ok(InstanceGuard {
            _file: crate::fs_install::hold_lock(&path)?,
        })
    }

    /// The notification ledger (`notices.json`, ops.md §3.2). None = absent.
    pub fn read_notices(&self) -> Result<Option<Vec<u8>>, FsError> {
        self.read(NOTICES_FILE, NOTICES_LIMIT)
    }

    pub fn write_notices(&self, bytes: &[u8]) -> Result<(), FsError> {
        self.write_bounded(NOTICES_FILE, NOTICES_LIMIT, bytes)
    }

    /// The update-check state (`update_check.json`, ops.md §4.2). None = absent.
    pub fn read_update_check(&self) -> Result<Option<Vec<u8>>, FsError> {
        self.read(UPDATE_CHECK_FILE, UPDATE_CHECK_LIMIT)
    }

    pub fn write_update_check(&self, bytes: &[u8]) -> Result<(), FsError> {
        self.write_bounded(UPDATE_CHECK_FILE, UPDATE_CHECK_LIMIT, bytes)
    }

    fn write_bounded(&self, name: &str, limit: u64, bytes: &[u8]) -> Result<(), FsError> {
        if bytes.len() as u64 > limit {
            return Err(FsError::new(
                FsErrorKind::TooLarge,
                &self.root.path().join(name),
                "state file would exceed its limit",
            ));
        }
        self.write_atomic(name, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TempDir;

    #[test]
    fn notice_and_update_state_files_round_trip_within_their_limits() {
        let t = TempDir::new();
        let ls = LocalState::open_at(&t.path().join("state")).unwrap();
        assert_eq!(ls.read_notices().unwrap(), None);
        assert_eq!(ls.read_update_check().unwrap(), None);
        ls.write_notices(b"{\"n\":1}").unwrap();
        ls.write_update_check(b"{\"u\":1}").unwrap();
        assert_eq!(ls.read_notices().unwrap().unwrap(), b"{\"n\":1}");
        assert_eq!(ls.read_update_check().unwrap().unwrap(), b"{\"u\":1}");
        let big = vec![b'x'; UPDATE_CHECK_LIMIT as usize + 1];
        assert_eq!(
            ls.write_update_check(&big).unwrap_err().kind,
            FsErrorKind::TooLarge
        );
        assert_eq!(ls.read_update_check().unwrap().unwrap(), b"{\"u\":1}");
        let big = vec![b'x'; NOTICES_LIMIT as usize + 1];
        assert!(ls.write_notices(&big).is_err());
    }

    #[test]
    fn remove_deletes_a_file_and_tolerates_absence() {
        let t = TempDir::new();
        let ls = LocalState::open_at(&t.path().join("state")).unwrap();
        ls.write_atomic("a.txt", b"x").unwrap();
        ls.remove("a.txt").unwrap();
        assert_eq!(ls.read("a.txt", 10).unwrap(), None);
        ls.remove("a.txt").unwrap();
        std::fs::create_dir(ls.path().join("d")).unwrap();
        assert!(ls.remove("d").is_err(), "a directory is not removed");
    }

    #[test]
    fn audit_rotates_at_limit_keeping_two() {
        let t = TempDir::new();
        let ls = LocalState::open_at(&t.path().join("state")).unwrap();
        let line = "x".repeat(99);
        for _ in 0..10 {
            ls.append_audit_limited(&line, 1000).unwrap();
        }
        let size = |n: &str| {
            std::fs::metadata(ls.path().join(n))
                .map(|m| m.len())
                .unwrap_or(0)
        };
        assert_eq!(size("audit.jsonl"), 1000);
        assert_eq!(size("audit.1.jsonl"), 0);
        ls.append_audit_limited("marker-a", 1000).unwrap();
        assert_eq!(size("audit.1.jsonl"), 1000);
        assert_eq!(
            std::fs::read_to_string(ls.path().join("audit.jsonl")).unwrap(),
            "marker-a\n"
        );
        for _ in 0..30 {
            ls.append_audit_limited(&line, 1000).unwrap();
        }
        let names: Vec<String> = std::fs::read_dir(ls.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("audit"))
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(size("audit.jsonl") <= 1000 && size("audit.1.jsonl") <= 1000);
        // Newlines would forge records.
        assert!(ls.append_audit("a\nb").is_err());
        ls.append_audit("public path works").unwrap();
    }

    #[test]
    fn second_instance_guard_fails() {
        let t = TempDir::new();
        let dir = t.path().join("state");
        let a = LocalState::open_at(&dir).unwrap();
        let b = LocalState::open_at(&dir).unwrap();
        let g = a.acquire_instance().unwrap();
        assert_eq!(b.acquire_instance().err().unwrap().kind, FsErrorKind::InUse);
        assert_eq!(a.acquire_instance().err().unwrap().kind, FsErrorKind::InUse);
        drop(g);
        let _g2 = b.acquire_instance().unwrap();
    }

    #[test]
    fn read_write_atomic_round_trip_and_bounds() {
        let t = TempDir::new();
        let ls = LocalState::open_at(&t.path().join("state")).unwrap();
        assert_eq!(ls.read("admission.json", 100).unwrap(), None);
        ls.write_atomic("admission.json", b"{\"a\":1}").unwrap();
        ls.write_atomic("admission.json", b"{\"a\":22}").unwrap();
        assert_eq!(
            ls.read("admission.json", 100).unwrap().unwrap(),
            b"{\"a\":22}"
        );
        assert_eq!(
            ls.read("admission.json", 3).unwrap_err().kind,
            FsErrorKind::TooLarge
        );
        assert!(ls.write_atomic("../x", b"x").is_err());
        assert!(ls.read("a/../../x", 10).is_err());
        let leftovers = std::fs::read_dir(ls.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }
}
