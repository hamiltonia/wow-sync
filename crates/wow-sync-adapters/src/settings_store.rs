//! Local settings blob, manifest and metadata store under `<variant>\.wowsync\settings\`
//! (settings.md §3.1, §7). Every write is temp + rename, so a failure leaves nothing partial.
//! Nothing here touches `WTF` or any live file.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::fs_safe::{ReparsePolicy, Root};
use crate::{FsError, FsErrorKind};

pub fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub struct SettingsStore {
    root: Root,
}

/// Copy buffer for every settings stream (hash, store, stage, upload): memory never scales with
/// the file.
pub const STREAM_BUF: usize = 256 * 1024;
pub const LOCAL_FILE: &str = "settings/local.json";
const LOCAL_LIMIT: u64 = 16 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 64 * 1024;

impl SettingsStore {
    /// `variant_dir` is the variant folder; the store lives in its `.wowsync` folder.
    pub fn open(variant_dir: &Path) -> Result<Self, FsError> {
        let dir = variant_dir.join(".wowsync");
        match std::fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(FsError::from_io(&dir, &e)),
        }
        Ok(Self {
            root: Root::open(&dir, ReparsePolicy::RejectAll)?,
        })
    }

    fn read_rel(&self, rel: &str, limit: u64) -> Result<Option<Vec<u8>>, FsError> {
        let f = match self.root.open_read(rel, limit) {
            Ok(f) => f,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let path = self.root.join(rel)?;
        let mut b = Vec::new();
        f.take(limit)
            .read_to_end(&mut b)
            .map_err(|e| FsError::from_io(&path, &e))?;
        Ok(Some(b))
    }

    fn write_rel(&self, rel: &str, bytes: &[u8]) -> Result<(), FsError> {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let tmp = format!(".{name}.{}.tmp", crate::sysinfo::random_id_hex());
        let p = self.root.path().to_path_buf();
        self.root
            .write_atomic_with(rel, &tmp, &mut |f| {
                f.write_all(bytes).map_err(|e| FsError::from_io(&p, &e))
            })
            .map(|_| ())
    }

    fn blob_name(h: &[u8; 32]) -> String {
        format!("settings/blobs/{}", hex(h))
    }

    fn manifest_name(h: &[u8; 32]) -> String {
        format!("settings/manifests/{}.json", hex(h))
    }

    /// Streams `src` into the blob named `expect`, write-if-absent. Only a fixed buffer is ever
    /// held, so a 32 MiB file costs the same memory as a small one. A source whose size or hash
    /// differs from what was declared (it changed while being read) stores nothing.
    pub fn put_blob_stream(
        &self,
        src: &mut dyn Read,
        expect: [u8; 32],
        size: u64,
    ) -> Result<(), FsError> {
        if self.blob_matches(&expect)? {
            return Ok(());
        }
        let p = self.root.path().to_path_buf();
        let tmp = format!(".blob.{}.tmp", crate::sysinfo::random_id_hex());
        self.root
            .write_atomic_with(&Self::blob_name(&expect), &tmp, &mut |f| {
                let mut h = Sha256::new();
                let mut buf = vec![0u8; STREAM_BUF];
                let mut total = 0u64;
                loop {
                    let n = src.read(&mut buf).map_err(|e| FsError::from_io(&p, &e))?;
                    if n == 0 {
                        break;
                    }
                    total += n as u64;
                    if total > size {
                        break;
                    }
                    h.update(&buf[..n]);
                    f.write_all(&buf[..n])
                        .map_err(|e| FsError::from_io(&p, &e))?;
                }
                let got: [u8; 32] = h.finalize().into();
                if total != size || got != expect {
                    return Err(FsError::new(
                        FsErrorKind::HashMismatch,
                        &p,
                        "the settings file changed while it was being saved",
                    ));
                }
                Ok(())
            })
            .map(|_| ())
    }

    /// Stores small bytes under their hash. Whole-buffer convenience for tests and tiny data;
    /// saved-variables files go through `put_blob_stream`.
    pub fn put_blob(&self, bytes: &[u8]) -> Result<[u8; 32], FsError> {
        let h = digest(bytes);
        self.put_blob_stream(&mut &bytes[..], h, bytes.len() as u64)?;
        Ok(h)
    }

    /// True when the blob exists and its bytes hash to its name (read in fixed chunks).
    fn blob_matches(&self, h: &[u8; 32]) -> Result<bool, FsError> {
        let name = Self::blob_name(h);
        let mut f = match self.root.open_read(&name, u64::MAX / 2) {
            Ok(f) => f,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        let path = self.root.join(&name)?;
        let (_, got) = crate::fs_safe::sha256_stream(&path, &mut f, u64::MAX)?;
        Ok(&got == h)
    }

    /// Opens a stored blob for streaming, with its size; `None` when absent. The reader's
    /// consumer verifies the hash while it copies.
    pub fn open_blob(&self, h: &[u8; 32], limit: u64) -> Result<Option<(File, u64)>, FsError> {
        let name = Self::blob_name(h);
        let f = match self.root.open_read(&name, limit) {
            Ok(f) => f,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let size = f
            .metadata()
            .map_err(|e| FsError::from_io(self.root.path(), &e))?
            .len();
        Ok(Some((f, size)))
    }

    /// `None` when absent. A stored blob whose bytes do not hash to its name is corrupt.
    /// Reads the whole blob, so it exists for tests only.
    #[cfg(test)]
    pub fn get_blob(&self, h: &[u8; 32], limit: u64) -> Result<Option<Vec<u8>>, FsError> {
        let name = Self::blob_name(h);
        let Some(b) = self.read_rel(&name, limit)? else {
            return Ok(None);
        };
        if &digest(&b) != h {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                self.root.path(),
                "stored settings blob does not match its hash",
            ));
        }
        Ok(Some(b))
    }

    fn has_blob(&self, h: &[u8; 32]) -> Result<bool, FsError> {
        match self.root.check_components(&Self::blob_name(h)) {
            Ok(m) => Ok(!m.is_dir),
            Err(e) if e.kind == FsErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// A manifest is stored only after its blobs exist; a missing blob refuses the write.
    pub fn put_manifest(&self, bytes: &[u8], blobs: &[[u8; 32]]) -> Result<[u8; 32], FsError> {
        for b in blobs {
            if !self.has_blob(b)? {
                return Err(FsError::new(
                    FsErrorKind::NotFound,
                    self.root.path(),
                    "a manifest blob is missing",
                ));
            }
        }
        let h = digest(bytes);
        self.write_rel(&Self::manifest_name(&h), bytes)?;
        Ok(h)
    }

    pub fn get_manifest(&self, h: &[u8; 32]) -> Result<Option<Vec<u8>>, FsError> {
        let Some(b) = self.read_rel(&Self::manifest_name(h), MANIFEST_LIMIT)? else {
            return Ok(None);
        };
        if &digest(&b) != h {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                self.root.path(),
                "stored settings manifest does not match its hash",
            ));
        }
        Ok(Some(b))
    }

    /// Pruning removes only store data, never a live file.
    pub fn remove_blob(&self, h: &[u8; 32]) -> Result<(), FsError> {
        self.root.remove_tree(&Self::blob_name(h))
    }

    pub fn remove_manifest(&self, h: &[u8; 32]) -> Result<(), FsError> {
        self.root.remove_tree(&Self::manifest_name(h))
    }

    pub fn read_local(&self) -> Result<Option<Vec<u8>>, FsError> {
        self.read_rel(LOCAL_FILE, LOCAL_LIMIT)
    }

    pub fn write_local(&self, bytes: &[u8]) -> Result<(), FsError> {
        self.write_rel(LOCAL_FILE, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TempDir;

    #[test]
    fn blobs_round_trip_deduplicate_and_detect_corruption() {
        let t = TempDir::new();
        let s = SettingsStore::open(t.path()).unwrap();
        let h = s.put_blob(b"DB = {}").unwrap();
        assert_eq!(s.put_blob(b"DB = {}").unwrap(), h);
        assert_eq!(s.get_blob(&h, 100).unwrap().unwrap(), b"DB = {}");
        assert_eq!(s.get_blob(&digest(b"other"), 100).unwrap(), None);
        let p = t
            .path()
            .join(".wowsync")
            .join("settings")
            .join("blobs")
            .join(hex(&h));
        std::fs::write(&p, b"tampered").unwrap();
        assert!(s.get_blob(&h, 100).is_err());
        s.remove_blob(&h).unwrap();
        assert_eq!(s.get_blob(&h, 100).unwrap(), None);
    }

    #[test]
    fn a_manifest_without_its_blob_is_refused_and_leaves_nothing() {
        let t = TempDir::new();
        let s = SettingsStore::open(t.path()).unwrap();
        let missing = digest(b"never stored");
        let err = s.put_manifest(b"{\"files\":[]}", &[missing]).unwrap_err();
        assert_eq!(err.kind, FsErrorKind::NotFound);
        assert_eq!(s.get_manifest(&digest(b"{\"files\":[]}")).unwrap(), None);
        let b = s.put_blob(b"x").unwrap();
        let m = s.put_manifest(b"{\"files\":[1]}", &[b]).unwrap();
        assert_eq!(s.get_manifest(&m).unwrap().unwrap(), b"{\"files\":[1]}");
        s.remove_manifest(&m).unwrap();
        assert_eq!(s.get_manifest(&m).unwrap(), None);
    }

    #[test]
    fn local_metadata_survives_a_restart_and_a_failed_write_keeps_the_previous() {
        let t = TempDir::new();
        let s = SettingsStore::open(t.path()).unwrap();
        assert_eq!(s.read_local().unwrap(), None);
        s.write_local(b"{\"a\":1}").unwrap();
        let s2 = SettingsStore::open(t.path()).unwrap();
        assert_eq!(s2.read_local().unwrap().unwrap(), b"{\"a\":1}");
        s2.write_local(b"{\"a\":2}").unwrap();
        assert_eq!(s.read_local().unwrap().unwrap(), b"{\"a\":2}");
        // no temporary file remains next to the metadata
        let dir = t.path().join(".wowsync").join("settings");
        let names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["local.json".to_string()]);
    }
}
