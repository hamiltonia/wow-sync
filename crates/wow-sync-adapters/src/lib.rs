//! Effect adapters for WoW Sync. They report observations and perform requested
//! effects inside authorized roots; they make no policy decisions.

use std::fmt;
use std::path::{Path, PathBuf};

pub mod authority_store;
pub mod fs_install;
pub mod fs_safe;
pub mod fs_scan;
pub mod localstate;
pub mod notify;
pub mod probe;
pub mod provider;
pub mod settings_store;
pub mod sysinfo;
pub mod update_http;
pub mod winproc;
pub mod wtf;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsErrorKind {
    NotFound,
    AlreadyExists,
    OutsideRoot,
    ReparsePoint,
    NotPlainFile,
    TooLarge,
    HashMismatch,
    InUse,
    Io,
    CloudOnly,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsError {
    pub kind: FsErrorKind,
    pub path: PathBuf,
    pub detail: String,
}

impl FsError {
    pub fn new(kind: FsErrorKind, path: &Path, detail: impl Into<String>) -> FsError {
        FsError {
            kind,
            path: path.to_path_buf(),
            detail: detail.into(),
        }
    }

    /// Maps an I/O error to the closest kind. Sharing violations and access
    /// denials are reported as `InUse` (another process holds the entry).
    pub fn from_io(path: &Path, err: &std::io::Error) -> FsError {
        use std::io::ErrorKind;
        let kind = match err.kind() {
            ErrorKind::NotFound => FsErrorKind::NotFound,
            ErrorKind::AlreadyExists => FsErrorKind::AlreadyExists,
            ErrorKind::PermissionDenied => FsErrorKind::InUse,
            _ => match err.raw_os_error() {
                Some(32) | Some(33) => FsErrorKind::InUse,
                _ => FsErrorKind::Io,
            },
        };
        FsError::new(kind, path, err.to_string())
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at {}: {}",
            self.kind,
            self.path.display(),
            self.detail
        )
    }
}

impl std::error::Error for FsError {}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}
