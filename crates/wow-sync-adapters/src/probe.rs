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
    fn subdir_names_lists_only_directories() {
        let t = TempDir::new();
        std::fs::create_dir(t.join("_retail_")).unwrap();
        std::fs::write(t.join("file.txt"), b"x").unwrap();
        assert_eq!(subdir_names(t.path()), vec!["_retail_".to_string()]);
        assert!(subdir_names(&t.join("missing")).is_empty());
    }
}
