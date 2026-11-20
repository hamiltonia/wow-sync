//! Authority-record and ledger-segment storage in the shared folder (contracts.md §1.10, §9.1).
//!
//! Records are immutable and content-addressed: the file name is the SHA-256 of the exact bytes.
//! This module only moves bytes; every decision about their meaning is made in `wow-sync-core`.

use crate::provider::ProviderRoot;
use crate::{FsError, FsErrorKind};
use std::path::Path;

/// Writes `bytes` to `rel`. An identical existing copy is left alone; a damaged own copy
/// (different bytes under a content-addressed name) is repaired through a temp file, verified,
/// then renamed over the damaged one. Returns whether anything was written.
pub fn put_record(
    root: &ProviderRoot,
    rel: &str,
    bytes: &[u8],
    limit: u64,
    computer: &str,
) -> Result<bool, FsError> {
    match root.read(rel, limit) {
        Ok(existing) if existing == bytes => return Ok(false),
        Ok(_) | Err(FsError {
            kind: FsErrorKind::TooLarge,
            ..
        }) => {}
        Err(e) if e.kind == FsErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    root.write_atomic(rel, bytes, computer)?;
    let back = root.read(rel, limit)?;
    if back != bytes {
        return Err(FsError::new(
            FsErrorKind::HashMismatch,
            Path::new(rel),
            "the copy does not match after writing",
        ));
    }
    Ok(true)
}

/// Lists the file names in `rel_dir`; a missing directory is an empty list.
pub fn list_names(root: &ProviderRoot, rel_dir: &str) -> Result<Vec<String>, FsError> {
    match root.list(rel_dir) {
        Ok(v) => Ok(v.into_iter().map(|e| e.name).collect()),
        Err(e) if e.kind == FsErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
