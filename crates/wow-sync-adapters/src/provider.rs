//! Provider-folder adapter: bounded reads, atomic writes, listings with cloud
//! state. No provider API, no hydration, no credentials.

use crate::fs_safe::{
    check_entry, is_unavailable, sha256_stream, stat_entry, validate_segment, ReparseKind,
    ReparsePolicy, Root,
};
use crate::{FsError, FsErrorKind};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub struct ProviderRoot {
    root: Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    Local,
    CloudOnly,
    Temp,
    Conflicted,
    /// Name cannot be addressed safely (invalid segment); never read.
    Unexpected,
    UnsafeReparse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_ns: i128,
    pub state: EntryState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectObs {
    Missing,
    CloudOnly,
    Local {
        size: u64,
        mtime_ns: i128,
        sha256: [u8; 32],
    },
}

fn tmp_name(target: &str, computer: &str) -> String {
    format!(
        "{target}.{computer}.{}.tmp",
        crate::sysinfo::random_id_hex()
    )
}

fn check_computer(root: &Path, computer: &str) -> Result<(), FsError> {
    validate_segment(computer)
        .and_then(|()| {
            if computer.contains('.') {
                Err("'.' in computer id".to_string())
            } else {
                Ok(())
            }
        })
        .map_err(|d| FsError::new(FsErrorKind::OutsideRoot, root, format!("{d}: {computer:?}")))
}

impl ProviderRoot {
    /// Creates `<shared>/WoWSync` if missing; `shared` must exist.
    pub fn open_or_create(shared: &Path) -> Result<ProviderRoot, FsError> {
        let shared_root = Root::open(shared, ReparsePolicy::AllowCloudFiles)?;
        shared_root.ensure_dir("WoWSync")?;
        let root = Root::open(&shared.join("WoWSync"), ReparsePolicy::AllowCloudFiles)?;
        Ok(ProviderRoot { root })
    }

    pub fn path(&self) -> &Path {
        self.root.path()
    }

    /// Entries of `rel_dir` ("" = the provider root); a missing directory is empty.
    pub fn list(&self, rel_dir: &str) -> Result<Vec<Entry>, FsError> {
        let metas = match self.root.list(rel_dir) {
            Ok(v) => v,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut out: Vec<Entry> = metas
            .into_iter()
            .map(|m| {
                let lower = m.name.to_lowercase();
                let state = if matches!(
                    m.reparse,
                    ReparseKind::Link { .. } | ReparseKind::Other { .. }
                ) {
                    EntryState::UnsafeReparse
                } else if validate_segment(&m.name).is_err() && !lower.ends_with(".tmp") {
                    EntryState::Unexpected
                } else if lower.ends_with(".tmp") {
                    EntryState::Temp
                } else if lower.contains("conflicted copy") {
                    EntryState::Conflicted
                } else if is_unavailable(m.attrs) {
                    EntryState::CloudOnly
                } else {
                    EntryState::Local
                };
                Entry {
                    name: m.name,
                    is_dir: m.is_dir,
                    size: m.size,
                    mtime_ns: m.mtime_ns,
                    state,
                }
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Refuses cloud-only entries (never hydrates) and files larger than `limit`.
    pub fn read(&self, rel: &str, limit: u64) -> Result<Vec<u8>, FsError> {
        let f = self.root.open_read(rel, limit)?;
        let path = self.root.join(rel)?;
        let mut b = Vec::new();
        f.take(limit + 1)
            .read_to_end(&mut b)
            .map_err(|e| FsError::from_io(&path, &e))?;
        if b.len() as u64 > limit {
            return Err(FsError::new(
                FsErrorKind::TooLarge,
                &path,
                "file grew beyond limit",
            ));
        }
        Ok(b)
    }

    /// `<target>.<computer>.<rand>.tmp` in the same directory, write, `sync_all`, rename over target.
    pub fn write_atomic(&self, rel: &str, bytes: &[u8], computer: &str) -> Result<(), FsError> {
        check_computer(self.root.path(), computer)?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let p = self.root.path().to_path_buf();
        self.root
            .write_atomic_with(rel, &tmp_name(name, computer), &mut |f| {
                f.write_all(bytes).map_err(|e| FsError::from_io(&p, &e))
            })
            .map(|_| ())
    }

    /// Observation for publish; hashes the bytes of local objects every time.
    pub fn check_object(&self, rel: &str, max_bytes: u64) -> Result<ObjectObs, FsError> {
        let meta = match self.root.check_components(rel) {
            Ok(m) => m,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(ObjectObs::Missing),
            Err(e) if e.kind == FsErrorKind::CloudOnly => return Ok(ObjectObs::CloudOnly),
            Err(e) => return Err(e),
        };
        let path = self.root.join(rel)?;
        if meta.is_dir {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                &path,
                "object is a directory",
            ));
        }
        let mut f = self.root.open_read(rel, max_bytes)?;
        let (size, sha256) = sha256_stream(&path, &mut f, max_bytes)?;
        Ok(ObjectObs::Local {
            size,
            mtime_ns: meta.mtime_ns,
            sha256,
        })
    }

    pub fn open_read(&self, rel: &str, limit: u64) -> Result<File, FsError> {
        self.root.open_read(rel, limit)
    }

    /// Copies `src` via a temp file, verifies the temp file's SHA-256 against `expect`,
    /// then renames over `rel`. The final `src` component must be a plain, locally
    /// available file; the caller supplies a path obtained from an authorized root.
    pub fn put_object(
        &self,
        rel: &str,
        src: &Path,
        expect: [u8; 32],
        computer: &str,
    ) -> Result<(u64, i128), FsError> {
        let meta = stat_entry(src)?;
        check_entry(&meta, ReparsePolicy::RejectAll, src)?;
        if meta.is_dir {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                src,
                "source is a directory",
            ));
        }
        let mut oo = std::fs::OpenOptions::new();
        oo.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            oo.custom_flags(0x0010_0000 | 0x0020_0000);
        }
        let mut f = oo.open(src).map_err(|e| FsError::from_io(src, &e))?;
        self.put_stream(rel, &mut f, expect, computer)
    }

    pub fn put_object_bytes(
        &self,
        rel: &str,
        bytes: &[u8],
        expect: [u8; 32],
        computer: &str,
    ) -> Result<(u64, i128), FsError> {
        let mut r = bytes;
        self.put_stream(rel, &mut r, expect, computer)
    }

    /// Streams `src` into `rel` through a temp file verified against `expect`; only a fixed
    /// buffer is held.
    pub fn put_object_reader(
        &self,
        rel: &str,
        src: &mut dyn Read,
        expect: [u8; 32],
        computer: &str,
    ) -> Result<(u64, i128), FsError> {
        self.put_stream(rel, src, expect, computer)
    }

    fn put_stream(
        &self,
        rel: &str,
        src: &mut dyn Read,
        expect: [u8; 32],
        computer: &str,
    ) -> Result<(u64, i128), FsError> {
        check_computer(self.root.path(), computer)?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let p = self.root.path().to_path_buf();
        let target = self
            .root
            .write_atomic_with(rel, &tmp_name(name, computer), &mut |f| {
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    let n = src.read(&mut buf).map_err(|e| FsError::from_io(&p, &e))?;
                    if n == 0 {
                        break;
                    }
                    f.write_all(&buf[..n])
                        .map_err(|e| FsError::from_io(&p, &e))?;
                }
                // Verify what actually reached the temp file before it can be renamed.
                f.sync_all().map_err(|e| FsError::from_io(&p, &e))?;
                std::io::Seek::seek(f, std::io::SeekFrom::Start(0))
                    .map_err(|e| FsError::from_io(&p, &e))?;
                let (_, h) = sha256_stream(&p, f, u64::MAX)?;
                if h != expect {
                    return Err(FsError::new(
                        FsErrorKind::HashMismatch,
                        &p,
                        format!("object {rel} does not match expected sha256"),
                    ));
                }
                Ok(())
            })?;
        let m = stat_entry(&target)?;
        Ok((m.size, m.mtime_ns))
    }

    /// The caller restricts this to its own member directory.
    pub fn remove_file(&self, rel: &str) -> Result<(), FsError> {
        let m = match self.root.check_components(rel) {
            Ok(m) => m,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let path = self.root.join(rel)?;
        if m.is_dir {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                &path,
                "is a directory",
            ));
        }
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(FsError::from_io(&path, &e)),
        }
    }

    /// Removes this computer's own temp files (`*.<computer>.<rand>.tmp`) whose mtime
    /// is older than `older_than_ms` before `now`. Returns the number removed.
    pub fn remove_stale_temps(
        &self,
        rel_dir: &str,
        computer: &str,
        older_than_ms: u64,
        now: i64,
    ) -> Result<u32, FsError> {
        check_computer(self.root.path(), computer)?;
        let marker = format!(".{computer}.");
        let cutoff_ns = (now as i128 - older_than_ms as i128) * 1_000_000;
        let mut removed = 0;
        for e in self.list(rel_dir)? {
            if e.state != EntryState::Temp
                || e.is_dir
                || !e.name.contains(&marker)
                || e.mtime_ns >= cutoff_ns
            {
                continue;
            }
            let rel = if rel_dir.is_empty() {
                e.name.clone()
            } else {
                format!("{rel_dir}/{}", e.name)
            };
            if self.remove_file(&rel).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{set_mtime, TempDir};
    use sha2::{Digest, Sha256};
    use std::time::{Duration, SystemTime};

    const C: &str = "comp1";

    fn sha(b: &[u8]) -> [u8; 32] {
        Sha256::digest(b).into()
    }

    fn setup() -> (TempDir, ProviderRoot) {
        let t = TempDir::new();
        let p = ProviderRoot::open_or_create(t.path()).unwrap();
        (t, p)
    }

    fn temps_in(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn write_atomic_replaces_without_partial_file() {
        let (_t, p) = setup();
        p.write_atomic("members/m1/state.json", b"one", C).unwrap();
        assert_eq!(p.read("members/m1/state.json", 100).unwrap(), b"one");
        p.write_atomic("members/m1/state.json", b"two-longer", C)
            .unwrap();
        assert_eq!(p.read("members/m1/state.json", 100).unwrap(), b"two-longer");
        let dir = p.path().join("members").join("m1");
        assert!(temps_in(&dir).is_empty());
        // Failure (target is a directory) keeps the old state and leaves no temp.
        std::fs::create_dir(dir.join("blocked")).unwrap();
        assert_eq!(
            p.write_atomic("members/m1/blocked", b"x", C)
                .unwrap_err()
                .kind,
            FsErrorKind::NotPlainFile
        );
        assert!(temps_in(&dir).is_empty());
        assert!(p
            .write_atomic("members/m1/state.json", b"x", "bad/id")
            .is_err());
        assert!(p.write_atomic("../escape", b"x", C).is_err());
        assert_eq!(p.read("members/m1/state.json", 100).unwrap(), b"two-longer");
        // A temp left by a crashed writer is invisible as the target and listed as Temp.
        std::fs::write(dir.join(format!("state.json.{C}.abc.tmp")), b"par").unwrap();
        assert_eq!(p.read("members/m1/state.json", 100).unwrap(), b"two-longer");
    }

    #[test]
    fn put_object_verifies_temp_before_rename() {
        let (t, p) = setup();
        let bytes = b"blob bytes";
        let e = p
            .put_object_bytes("objects/aa/obj", bytes, sha(b"other"), C)
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::HashMismatch);
        assert!(!p.path().join("objects").join("aa").join("obj").exists());
        assert!(temps_in(&p.path().join("objects").join("aa")).is_empty());
        let (size, mtime) = p
            .put_object_bytes("objects/aa/obj", bytes, sha(bytes), C)
            .unwrap();
        assert_eq!(size, bytes.len() as u64);
        assert!(mtime > 0);
        // From a file path.
        let src = t.path().join("src.bin");
        std::fs::write(&src, b"from file").unwrap();
        p.put_object("objects/bb/obj2", &src, sha(b"from file"), C)
            .unwrap();
        assert_eq!(p.read("objects/bb/obj2", 100).unwrap(), b"from file");
        assert_eq!(
            p.put_object("objects/bb/obj3", &src, sha(b"x"), C)
                .unwrap_err()
                .kind,
            FsErrorKind::HashMismatch
        );
        assert!(!p.path().join("objects").join("bb").join("obj3").exists());
        assert_eq!(
            p.put_object("objects/bb/obj4", &t.path().join("nope"), sha(b"x"), C)
                .unwrap_err()
                .kind,
            FsErrorKind::NotFound
        );
    }

    #[test]
    fn put_object_replaces_corrupt_object() {
        let (_t, p) = setup();
        let good = b"the real bytes";
        p.write_atomic("objects/ab/obj", b"corrupted!", C).unwrap();
        assert_ne!(
            p.check_object("objects/ab/obj", 1000).unwrap(),
            ObjectObs::Local {
                size: good.len() as u64,
                mtime_ns: 0,
                sha256: sha(good)
            }
        );
        p.put_object_bytes("objects/ab/obj", good, sha(good), C)
            .unwrap();
        match p.check_object("objects/ab/obj", 1000).unwrap() {
            ObjectObs::Local { sha256, size, .. } => {
                assert_eq!(sha256, sha(good));
                assert_eq!(size, good.len() as u64);
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn check_object_reports_wrong_hash() {
        let (_t, p) = setup();
        assert_eq!(
            p.check_object("objects/x/missing", 100).unwrap(),
            ObjectObs::Missing
        );
        // Same size, different content: the hash differs, size alone proves nothing.
        p.write_atomic("objects/x/o", b"AAAA", C).unwrap();
        let ObjectObs::Local {
            sha256: a,
            size: sa,
            ..
        } = p.check_object("objects/x/o", 100).unwrap()
        else {
            panic!()
        };
        p.write_atomic("objects/x/o", b"BBBB", C).unwrap();
        let ObjectObs::Local {
            sha256: b,
            size: sb,
            ..
        } = p.check_object("objects/x/o", 100).unwrap()
        else {
            panic!()
        };
        assert_eq!(sa, sb);
        assert_ne!(a, b);
        assert_eq!(b, sha(b"BBBB"));
        assert_eq!(
            p.check_object("objects/x/o", 3).unwrap_err().kind,
            FsErrorKind::TooLarge
        );
        assert_eq!(
            p.check_object("objects/x", 100).unwrap_err().kind,
            FsErrorKind::NotPlainFile
        );
        // Same size and mtime but changed bytes are still caught (no cache).
        let path = p.path().join("objects").join("x").join("o");
        let mt = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::fs::write(&path, b"CCCC").unwrap();
        set_mtime(&path, mt);
        let ObjectObs::Local { sha256: c, .. } = p.check_object("objects/x/o", 100).unwrap() else {
            panic!()
        };
        assert_eq!(c, sha(b"CCCC"));
    }

    #[test]
    fn temp_and_conflicted_entries_classified() {
        let (_t, p) = setup();
        let dir = p.path().join("d");
        std::fs::create_dir(&dir).unwrap();
        for n in [
            "ok.json",
            "x.comp1.ab.tmp",
            "state (Bob's Conflicted Copy 2026-11-15).json",
            "UPPER.TMP",
            "bad:name",
        ] {
            if n.contains(':') && cfg!(windows) {
                continue;
            }
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        std::fs::create_dir(dir.join("sub")).unwrap();
        let by: std::collections::BTreeMap<String, Entry> = p
            .list("d")
            .unwrap()
            .into_iter()
            .map(|e| (e.name.clone(), e))
            .collect();
        assert_eq!(by["ok.json"].state, EntryState::Local);
        assert_eq!(by["x.comp1.ab.tmp"].state, EntryState::Temp);
        assert_eq!(by["UPPER.TMP"].state, EntryState::Temp);
        assert_eq!(
            by["state (Bob's Conflicted Copy 2026-11-15).json"].state,
            EntryState::Conflicted
        );
        assert_eq!(by["sub"].state, EntryState::Local);
        assert!(by["sub"].is_dir && !by["ok.json"].is_dir);
        assert_eq!(by["ok.json"].size, 1);
        if !cfg!(windows) {
            assert_eq!(by["bad:name"].state, EntryState::Unexpected);
        }
        assert!(p.list("does/not/exist").unwrap().is_empty());
        assert_eq!(p.list("").unwrap().len(), 1);
    }

    #[test]
    fn read_refuses_oversized_file() {
        let (_t, p) = setup();
        p.write_atomic("f/big", &[1u8; 11], C).unwrap();
        assert_eq!(p.read("f/big", 10).unwrap_err().kind, FsErrorKind::TooLarge);
        assert_eq!(p.read("f/big", 11).unwrap().len(), 11);
        assert_eq!(
            p.read("f/none", 11).unwrap_err().kind,
            FsErrorKind::NotFound
        );
        assert_eq!(
            p.open_read("f/big", 10).unwrap_err().kind,
            FsErrorKind::TooLarge
        );
        assert_eq!(p.read("f", 11).unwrap_err().kind, FsErrorKind::NotPlainFile);
    }

    #[test]
    fn remove_stale_temps_only_own() {
        let (_t, p) = setup();
        let dir = p.path().join("m");
        std::fs::create_dir(&dir).unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        let mine_old = dir.join("a.comp1.01.tmp");
        let mine_new = dir.join("b.comp1.02.tmp");
        let theirs_old = dir.join("a.comp2.03.tmp");
        let real = dir.join("a.json");
        for f in [&mine_old, &mine_new, &theirs_old, &real] {
            std::fs::write(f, b"x").unwrap();
        }
        for f in [&mine_old, &theirs_old, &real] {
            set_mtime(f, old);
        }
        let now = crate::sysinfo::now_unix_ms();
        assert_eq!(p.remove_stale_temps("m", "comp1", 60_000, now).unwrap(), 1);
        assert!(!mine_old.exists());
        assert!(mine_new.exists() && theirs_old.exists() && real.exists());
        assert_eq!(p.remove_stale_temps("m", "comp1", 60_000, now).unwrap(), 0);
        assert_eq!(p.remove_stale_temps("absent", "comp1", 0, now).unwrap(), 0);
        p.remove_file("m/a.json").unwrap();
        p.remove_file("m/a.json").unwrap();
        assert!(!real.exists());
        assert_eq!(
            p.remove_file("m").unwrap_err().kind,
            FsErrorKind::NotPlainFile
        );
    }

    #[cfg(windows)]
    #[test]
    fn junction_under_provider_root_is_refused() {
        let (t, p) = setup();
        let outside = t.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("secret.json"), b"s").unwrap();
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(p.path().join("members"))
            .arg(&outside)
            .output()
            .unwrap();
        assert!(out.status.success());
        let listed = p.list("").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].state, EntryState::UnsafeReparse);
        assert_eq!(
            p.list("members").unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            p.read("members/secret.json", 100).unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            p.write_atomic("members/new.json", b"x", C)
                .unwrap_err()
                .kind,
            FsErrorKind::ReparsePoint
        );
        assert!(!outside.join("new.json").exists());
        assert_eq!(
            p.put_object_bytes("members/o", b"x", sha(b"x"), C)
                .unwrap_err()
                .kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            p.check_object("members/secret.json", 100).unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            p.remove_file("members/secret.json").unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert!(outside.join("secret.json").exists());
        // A junction as the shared folder itself or as WoWSync is refused too.
        let jshared = t.path().join("jshared");
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&jshared)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            ProviderRoot::open_or_create(&jshared).err().unwrap().kind,
            FsErrorKind::ReparsePoint
        );
        assert!(!outside.join("WoWSync").exists());
    }

    // Provider-tier tests: run with `cargo test -p wow-sync-adapters -- --ignored` and
    // `WOWSYNC_PROVIDER_TEST_ROOT` set below the Dropbox test folder.
    const ALLOWED: &str = "D:\\Dropbox\\Gaming Sync\\WoW Sync Test";

    fn provider_root() -> std::path::PathBuf {
        let v = std::env::var("WOWSYNC_PROVIDER_TEST_ROOT")
            .expect("BLOCKED: WOWSYNC_PROVIDER_TEST_ROOT is not set");
        let p = std::path::PathBuf::from(&v);
        assert!(
            crate::winproc::image_is_under(Path::new(ALLOWED), &p.join("x")),
            "refusing provider test root outside {ALLOWED}: {v}"
        );
        assert!(!v.contains(".."));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn run_subdir() -> std::path::PathBuf {
        provider_root().join(format!("r2-adapters-{}", crate::sysinfo::random_id_hex()))
    }

    fn wait_for_cloud_tag(path: &Path) -> Option<u32> {
        for _ in 0..30 {
            if let Ok(m) = stat_entry(path) {
                if let ReparseKind::CloudFile { tag } = m.reparse {
                    return Some(tag);
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        None
    }

    fn cleanup(sub: &Path) {
        for _ in 0..20 {
            if std::fs::remove_dir_all(sub).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        eprintln!("cleanup of {} incomplete", sub.display());
    }

    #[test]
    #[ignore = "provider tier: needs WOWSYNC_PROVIDER_TEST_ROOT under the Dropbox test folder"]
    fn dropbox_test_root_round_trip() {
        let sub = run_subdir();
        std::fs::create_dir_all(&sub).unwrap();
        let result = std::panic::catch_unwind(|| {
            let p = ProviderRoot::open_or_create(&sub).unwrap();
            let data = TempDir::new();
            let files = crate::testkit::synthetic_dataset(data.path(), 7, 20 * 1024 * 1024);
            let mut expected = Vec::new();
            for (i, f) in files.iter().enumerate() {
                let b = std::fs::read(f).unwrap();
                let rel = format!("objects/{:02}/o{i:04}", i % 8);
                p.put_object(&rel, f, sha(&b), C).unwrap();
                expected.push((rel, sha(&b), b.len() as u64));
            }
            p.write_atomic("members/m1/state.json", b"{\"round\":\"trip\"}", C)
                .unwrap();
            for (rel, h, n) in &expected {
                match p.check_object(rel, 1 << 20).unwrap() {
                    ObjectObs::Local { sha256, size, .. } => assert_eq!((sha256, size), (*h, *n)),
                    other => panic!("{rel}: {other:?}"),
                }
                assert_eq!(p.read(rel, 1 << 20).unwrap().len() as u64, *n);
            }
            assert_eq!(
                p.read("members/m1/state.json", 100).unwrap(),
                b"{\"round\":\"trip\"}"
            );
            let states: Vec<_> = p
                .list("members/m1")
                .unwrap()
                .into_iter()
                .map(|e| (e.name, e.state))
                .collect();
            println!(
                "round trip: {} objects, member dir listing {states:?}",
                expected.len()
            );
            println!(
                "cloud tag after write: {:?}",
                wait_for_cloud_tag(&sub.join("WoWSync").join("members"))
            );
            assert!(p
                .list("objects")
                .unwrap()
                .iter()
                .all(|e| e.state == EntryState::Local));
        });
        cleanup(&sub);
        if let Err(p) = result {
            std::panic::resume_unwind(p);
        }
        assert!(!sub.exists(), "cleanup left {}", sub.display());
    }

    #[test]
    #[ignore = "provider tier: needs WOWSYNC_PROVIDER_TEST_ROOT under the Dropbox test folder"]
    fn dropbox_hydrated_tagged_entries_accepted() {
        let sub = run_subdir();
        std::fs::create_dir_all(&sub).unwrap();
        let result = std::panic::catch_unwind(|| {
            let p = ProviderRoot::open_or_create(&sub).unwrap();
            p.write_atomic("tagged/f.json", b"{\"x\":1}", C).unwrap();
            let dir = sub.join("WoWSync").join("tagged");
            let Some(tag) = wait_for_cloud_tag(&dir.join("f.json")) else {
                panic!("BLOCKED: Dropbox applied no cloud reparse tag within 30 s");
            };
            println!(
                "file tag 0x{tag:x}; dir tag {:?}",
                stat_entry(&dir).map(|m| m.reparse)
            );
            let entries = p.list("tagged").unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].state, EntryState::Local);
            assert_eq!(p.read("tagged/f.json", 100).unwrap(), b"{\"x\":1}");
            let top = p.list("").unwrap();
            assert!(top
                .iter()
                .any(|e| e.name == "tagged" && e.state == EntryState::Local));
        });
        cleanup(&sub);
        if let Err(p) = result {
            std::panic::resume_unwind(p);
        }
        assert!(!sub.exists(), "cleanup left {}", sub.display());
    }

    #[test]
    #[ignore = "provider tier: needs an owner-prepared online-only file"]
    fn dropbox_online_only_entry_refused() {
        let Ok(v) = std::env::var("WOWSYNC_PROVIDER_ONLINE_ONLY") else {
            panic!("BLOCKED: no online-only entry (WOWSYNC_PROVIDER_ONLINE_ONLY unset)");
        };
        let path = std::path::PathBuf::from(v);
        assert!(
            crate::winproc::image_is_under(Path::new(ALLOWED), &path),
            "refusing online-only path outside {ALLOWED}"
        );
        let meta = stat_entry(&path).unwrap();
        assert!(
            is_unavailable(meta.attrs),
            "BLOCKED: {} is not an online-only entry",
            path.display()
        );
        let parent = path.parent().unwrap();
        let root = Root::open(parent, ReparsePolicy::AllowCloudFiles).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            root.open_read(&name, u64::MAX).unwrap_err().kind,
            FsErrorKind::CloudOnly
        );
    }
}
