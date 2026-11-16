//! Install-side effects: lock, staging, slot renames, journals, recovery records.
//! Deletion happens only inside `<variant>/.wowsync/{staging,recovery,journal}`;
//! `AddOns` content is never deleted, only renamed away.

use crate::fs_safe::{validate_segment, ReparsePolicy, Root};
use crate::fs_scan::{ScanOptions, ScannedFile, Snapshot};
use crate::{FsError, FsErrorKind};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub struct InstallArea {
    addons: Root,
    work: Root,
}

pub struct InstallLock {
    _file: File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<'a> {
    Live {
        folder: &'a str,
    },
    Staging {
        op: &'a str,
        folder: &'a str,
    },
    Recovery {
        pkg: &'a str,
        op: &'a str,
        folder: &'a str,
    },
}

const JOURNAL_LIMIT: u64 = 4 * 1024 * 1024;

fn seg(path: &Path, s: &str) -> Result<(), FsError> {
    validate_segment(s)
        .map_err(|d| FsError::new(FsErrorKind::OutsideRoot, path, format!("{d}: {s:?}")))
}

pub(crate) fn hold_lock(path: &Path) -> Result<File, FsError> {
    let mut oo = OpenOptions::new();
    oo.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        oo.share_mode(0);
    }
    let f = oo.open(path).map_err(|e| {
        let k = FsError::from_io(path, &e);
        if k.kind == FsErrorKind::InUse {
            FsError::new(FsErrorKind::InUse, path, "lock is held by another instance")
        } else {
            k
        }
    })?;
    match f.try_lock() {
        Ok(()) => Ok(f),
        Err(std::fs::TryLockError::WouldBlock) => Err(FsError::new(
            FsErrorKind::InUse,
            path,
            "lock is held by another instance",
        )),
        Err(std::fs::TryLockError::Error(e)) => Err(FsError::from_io(path, &e)),
    }
}

impl InstallArea {
    /// Creates `<variant>/.wowsync/{staging,recovery,journal}` if needed.
    pub fn open(variant_dir: &Path) -> Result<InstallArea, FsError> {
        let addons = Root::open(
            &variant_dir.join("Interface").join("AddOns"),
            ReparsePolicy::RejectAll,
        )?;
        let work_dir = variant_dir.join(".wowsync");
        match std::fs::create_dir(&work_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(FsError::from_io(&work_dir, &e)),
        }
        let work = Root::open(&work_dir, ReparsePolicy::RejectAll)?;
        for d in ["staging", "recovery", "journal"] {
            work.ensure_dir(d)?;
        }
        Ok(InstallArea { addons, work })
    }

    /// Exclusive lock excluding other WoW Sync instances only; a stale lock file with
    /// no holder is acquirable.
    pub fn lock(&self) -> Result<InstallLock, FsError> {
        let path = self.work.join("lock")?;
        if let Ok(m) = self.work.check_components("lock") {
            if m.is_dir {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    &path,
                    "lock is a directory",
                ));
            }
        }
        Ok(InstallLock {
            _file: hold_lock(&path)?,
        })
    }

    fn locate(&self, slot: Slot<'_>) -> Result<(&Root, String), FsError> {
        let p = self.work.path();
        match slot {
            Slot::Live { folder } => {
                seg(p, folder)?;
                Ok((&self.addons, folder.to_string()))
            }
            Slot::Staging { op, folder } => {
                seg(p, op)?;
                seg(p, folder)?;
                Ok((&self.work, format!("staging/{op}/{folder}")))
            }
            Slot::Recovery { pkg, op, folder } => {
                seg(p, pkg)?;
                seg(p, op)?;
                seg(p, folder)?;
                Ok((&self.work, format!("recovery/{pkg}/{op}/{folder}")))
            }
        }
    }

    /// Copies `src` into staging via a temp name, hashes the written copy and renames
    /// it into place; `HashMismatch` if the copy differs from `expect` or `size`.
    pub fn stage_file(
        &self,
        op: &str,
        folder: &str,
        rel: &str,
        src: &mut dyn Read,
        expect: [u8; 32],
        size: u64,
    ) -> Result<(), FsError> {
        let p = self.work.path();
        seg(p, op)?;
        seg(p, folder)?;
        let rel_in_work = format!("staging/{op}/{folder}/{rel}");
        let tmp_name = format!(".wowsync-{}.tmp", crate::sysinfo::random_id_hex());
        let mut written = (0u64, [0u8; 32]);
        let target = self
            .work
            .write_atomic_with(&rel_in_work, &tmp_name, &mut |f| {
                let mut h = Sha256::new();
                let mut buf = vec![0u8; 64 * 1024];
                let mut total = 0u64;
                loop {
                    let n = src.read(&mut buf).map_err(|e| FsError::from_io(p, &e))?;
                    if n == 0 {
                        break;
                    }
                    total += n as u64;
                    if total > size {
                        return Err(FsError::new(
                            FsErrorKind::HashMismatch,
                            p,
                            "source is larger than declared size",
                        ));
                    }
                    h.update(&buf[..n]);
                    f.write_all(&buf[..n])
                        .map_err(|e| FsError::from_io(p, &e))?;
                }
                written = (total, h.finalize().into());
                if written.0 != size || written.1 != expect {
                    return Err(FsError::new(
                        FsErrorKind::HashMismatch,
                        p,
                        format!("staged copy of {rel} does not match the expected size/hash"),
                    ));
                }
                Ok(())
            })?;
        // Hash the copy as it lies on disk, not only the bytes that were written.
        let mut f = File::open(&target).map_err(|e| FsError::from_io(&target, &e))?;
        let (n, h) = crate::fs_safe::sha256_stream(&target, &mut f, size)?;
        if n != size || h != expect {
            drop(f);
            let _ = std::fs::remove_file(&target);
            return Err(FsError::new(
                FsErrorKind::HashMismatch,
                &target,
                "re-read of staged file differs",
            ));
        }
        Ok(())
    }

    /// Content of any slot; None = absent.
    pub fn hash_slot(&self, slot: Slot<'_>) -> Result<Option<Vec<ScannedFile>>, FsError> {
        let (root, rel) = self.locate(slot)?;
        crate::fs_scan::tree_hash(root, &rel, &ScanOptions::with_defaults(0))
    }

    pub fn snapshot_slot(&self, slot: Slot<'_>) -> Result<Option<Snapshot>, FsError> {
        let (root, rel) = self.locate(slot)?;
        crate::fs_scan::tree_snapshot(
            root,
            &rel,
            ScanOptions::with_defaults(0).max_files_per_folder,
        )
    }

    pub fn exists(&self, slot: Slot<'_>) -> Result<bool, FsError> {
        let (root, rel) = self.locate(slot)?;
        match root.check_components(&rel) {
            Ok(_) => Ok(true),
            Err(e) if e.kind == FsErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Single same-volume directory rename; fails if the destination exists.
    pub fn rename(&self, from: Slot<'_>, to: Slot<'_>) -> Result<(), FsError> {
        let (fr, frel) = self.locate(from)?;
        let (tr, trel) = self.locate(to)?;
        let src = fr.join(&frel)?;
        let meta = fr.check_components(&frel)?;
        if !meta.is_dir {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                &src,
                "slot is not a directory",
            ));
        }
        let dst = tr.join(&trel)?;
        match tr.check_components(&trel) {
            Ok(_) => {
                return Err(FsError::new(
                    FsErrorKind::AlreadyExists,
                    &dst,
                    "destination exists",
                ))
            }
            Err(e) if e.kind == FsErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let parent_rel = trel.rsplit_once('/').map(|x| x.0).unwrap_or("");
        tr.ensure_dir(parent_rel)?;
        std::fs::rename(&src, &dst).map_err(|e| FsError::from_io(&src, &e))
    }

    /// Temp file, flush, fsync, rename.
    pub fn write_journal(&self, op: &str, bytes: &[u8]) -> Result<(), FsError> {
        seg(self.work.path(), op)?;
        let tmp = format!(".{}.{}.tmp", op, crate::sysinfo::random_id_hex());
        let p = self.work.path().to_path_buf();
        self.work
            .write_atomic_with(&format!("journal/{op}.json"), &tmp, &mut |f| {
                f.write_all(bytes).map_err(|e| FsError::from_io(&p, &e))
            })
            .map(|_| ())
    }

    /// (op, bytes) for every complete journal, sorted by op; temp files are ignored.
    pub fn read_journals(&self) -> Result<Vec<(String, Vec<u8>)>, FsError> {
        let mut out = Vec::new();
        for e in self.work.list("journal")? {
            let Some(op) = e.name.strip_suffix(".json") else {
                continue;
            };
            if e.is_dir || op.starts_with('.') {
                continue;
            }
            out.push((
                op.to_string(),
                read_all(&self.work, &format!("journal/{}", e.name), JOURNAL_LIMIT)?,
            ));
        }
        out.sort();
        Ok(out)
    }

    /// Idempotent.
    pub fn remove_journal(&self, op: &str) -> Result<(), FsError> {
        seg(self.work.path(), op)?;
        self.work.remove_tree(&format!("journal/{op}.json"))
    }

    /// Writes `recovery/<pkg>/<op>/recovery.json`.
    pub fn write_recovery_meta(&self, pkg: &str, op: &str, bytes: &[u8]) -> Result<(), FsError> {
        let w = self.work.path();
        seg(w, pkg)?;
        seg(w, op)?;
        let tmp = format!(".recovery.{}.tmp", crate::sysinfo::random_id_hex());
        let p = w.to_path_buf();
        self.work
            .write_atomic_with(
                &format!("recovery/{pkg}/{op}/recovery.json"),
                &tmp,
                &mut |f| f.write_all(bytes).map_err(|e| FsError::from_io(&p, &e)),
            )
            .map(|_| ())
    }

    /// (op, recovery.json bytes) for each recovery point of `pkg`; empty if none.
    pub fn list_recovery(&self, pkg: &str) -> Result<Vec<(String, Vec<u8>)>, FsError> {
        seg(self.work.path(), pkg)?;
        let dir = format!("recovery/{pkg}");
        let entries = match self.work.list(&dir) {
            Ok(v) => v,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut out = Vec::new();
        for e in entries.into_iter().filter(|e| e.is_dir) {
            match read_all(
                &self.work,
                &format!("{dir}/{}/recovery.json", e.name),
                JOURNAL_LIMIT,
            ) {
                Ok(b) => out.push((e.name, b)),
                Err(x) if x.kind == FsErrorKind::NotFound => {}
                Err(x) => return Err(x),
            }
        }
        out.sort();
        Ok(out)
    }

    /// Deletes `recovery/<pkg>/<op>` (and an emptied `<pkg>`). Idempotent.
    pub fn remove_recovery(&self, pkg: &str, op: &str) -> Result<(), FsError> {
        let w = self.work.path();
        seg(w, pkg)?;
        seg(w, op)?;
        self.work.remove_tree(&format!("recovery/{pkg}/{op}"))?;
        if let Ok(rest) = self.work.list(&format!("recovery/{pkg}")) {
            if rest.is_empty() {
                let _ = self.work.remove_tree(&format!("recovery/{pkg}"));
            }
        }
        Ok(())
    }

    /// Deletes `staging/<op>`. Idempotent.
    pub fn remove_staging(&self, op: &str) -> Result<(), FsError> {
        seg(self.work.path(), op)?;
        self.work.remove_tree(&format!("staging/{op}"))
    }
}

fn read_all(root: &Root, rel: &str, limit: u64) -> Result<Vec<u8>, FsError> {
    let f = root.open_read(rel, limit)?;
    let path = root.join(rel)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{SyntheticInstall, TempDir};

    fn setup() -> (TempDir, SyntheticInstall, InstallArea) {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        inst.add_addon("Alpha", "Alpha.toc", "## T\n", &[("a.lua", b"live")]);
        let area = InstallArea::open(&inst.variant_dir).unwrap();
        (t, inst, area)
    }

    fn sha(b: &[u8]) -> [u8; 32] {
        Sha256::digest(b).into()
    }

    fn stage(area: &InstallArea, op: &str, folder: &str, rel: &str, bytes: &[u8]) {
        area.stage_file(
            op,
            folder,
            rel,
            &mut &bytes[..],
            sha(bytes),
            bytes.len() as u64,
        )
        .unwrap();
    }

    #[test]
    fn deletion_only_inside_work_area() {
        let (_t, inst, area) = setup();
        stage(&area, "op1", "Alpha", "a.lua", b"new");
        area.write_recovery_meta("Alpha", "op0", b"{}").unwrap();
        // Traversal out of the work area is rejected for every deleting call.
        for bad in ["..", "../..", "a/b", "..\\x", ""] {
            assert_eq!(
                area.remove_staging(bad).unwrap_err().kind,
                FsErrorKind::OutsideRoot,
                "{bad:?}"
            );
            assert_eq!(
                area.remove_recovery(bad, "op0").unwrap_err().kind,
                FsErrorKind::OutsideRoot
            );
            assert_eq!(
                area.remove_recovery("Alpha", bad).unwrap_err().kind,
                FsErrorKind::OutsideRoot
            );
            assert_eq!(
                area.remove_journal(bad).unwrap_err().kind,
                FsErrorKind::OutsideRoot
            );
        }
        assert!(inst.addons.join("Alpha").join("a.lua").exists());
        // Legitimate deletions work, are scoped and idempotent.
        area.remove_staging("op1").unwrap();
        area.remove_staging("op1").unwrap();
        assert!(!inst
            .variant_dir
            .join(".wowsync")
            .join("staging")
            .join("op1")
            .exists());
        assert!(inst.variant_dir.join(".wowsync").join("staging").exists());
        assert_eq!(area.list_recovery("Alpha").unwrap().len(), 1);
        area.remove_recovery("Alpha", "op0").unwrap();
        assert!(area.list_recovery("Alpha").unwrap().is_empty());
        // Live folders are not reachable through any removal API.
        area.remove_staging("Alpha").unwrap();
        assert_eq!(
            std::fs::read(inst.addons.join("Alpha").join("a.lua")).unwrap(),
            b"live"
        );
    }

    #[test]
    fn stage_file_rejects_hash_mismatch() {
        let (_t, inst, area) = setup();
        let e = area
            .stage_file(
                "op1",
                "Alpha",
                "x.lua",
                &mut &b"abc"[..],
                sha(b"different"),
                3,
            )
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::HashMismatch);
        let e = area
            .stage_file("op1", "Alpha", "x.lua", &mut &b"abc"[..], sha(b"abc"), 4)
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::HashMismatch);
        let e = area
            .stage_file("op1", "Alpha", "x.lua", &mut &b"abcd"[..], sha(b"abcd"), 3)
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::HashMismatch);
        // Neither the target nor a temp file is left behind.
        let staged = inst
            .variant_dir
            .join(".wowsync")
            .join("staging")
            .join("op1")
            .join("Alpha");
        assert_eq!(
            std::fs::read_dir(&staged).map(|d| d.count()).unwrap_or(0),
            0
        );
        stage(&area, "op1", "Alpha", "sub/x.lua", b"abc");
        assert_eq!(
            std::fs::read(staged.join("sub").join("x.lua")).unwrap(),
            b"abc"
        );
        // Escaping relative paths are refused.
        let e = area
            .stage_file(
                "op1",
                "Alpha",
                "../../escape",
                &mut &b"abc"[..],
                sha(b"abc"),
                3,
            )
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::OutsideRoot);
    }

    #[test]
    fn rename_refuses_existing_destination() {
        let (_t, inst, area) = setup();
        stage(&area, "op1", "Alpha", "a.lua", b"new");
        let e = area
            .rename(
                Slot::Staging {
                    op: "op1",
                    folder: "Alpha",
                },
                Slot::Live { folder: "Alpha" },
            )
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(inst.addons.join("Alpha").join("a.lua")).unwrap(),
            b"live"
        );
        // The proper sequence: live -> recovery, staging -> live.
        area.rename(
            Slot::Live { folder: "Alpha" },
            Slot::Recovery {
                pkg: "Alpha",
                op: "op1",
                folder: "Alpha",
            },
        )
        .unwrap();
        area.rename(
            Slot::Staging {
                op: "op1",
                folder: "Alpha",
            },
            Slot::Live { folder: "Alpha" },
        )
        .unwrap();
        assert_eq!(
            std::fs::read(inst.addons.join("Alpha").join("a.lua")).unwrap(),
            b"new"
        );
        let rec = Slot::Recovery {
            pkg: "Alpha",
            op: "op1",
            folder: "Alpha",
        };
        assert!(area.exists(rec).unwrap());
        assert_eq!(
            area.rename(
                Slot::Staging {
                    op: "none",
                    folder: "Alpha"
                },
                Slot::Live { folder: "Beta" }
            )
            .unwrap_err()
            .kind,
            FsErrorKind::NotFound
        );
        assert!(!area.exists(Slot::Live { folder: "Beta" }).unwrap());
        // Restore into an absent live name.
        area.rename(rec, Slot::Live { folder: "Beta" }).unwrap();
        assert_eq!(
            std::fs::read(inst.addons.join("Beta").join("a.lua")).unwrap(),
            b"live"
        );
    }

    #[test]
    fn second_lock_fails_while_held() {
        let (_t, inst, area) = setup();
        let l = area.lock().unwrap();
        assert_eq!(area.lock().err().unwrap().kind, FsErrorKind::InUse);
        let other = InstallArea::open(&inst.variant_dir).unwrap();
        assert_eq!(other.lock().err().unwrap().kind, FsErrorKind::InUse);
        drop(l);
        // A stale lock file (no holder) can be acquired.
        assert!(inst.variant_dir.join(".wowsync").join("lock").exists());
        let _again = area.lock().unwrap();
    }

    #[test]
    fn journal_write_is_atomic_and_listed() {
        let (_t, inst, area) = setup();
        assert!(area.read_journals().unwrap().is_empty());
        area.write_journal("op2", b"{\"v\":2}").unwrap();
        area.write_journal("op1", b"{\"v\":1}").unwrap();
        area.write_journal("op1", b"{\"v\":3}").unwrap();
        // A crashed writer's temp file is never reported as a journal.
        let jd = inst.variant_dir.join(".wowsync").join("journal");
        std::fs::write(jd.join(".op9.deadbeef.tmp"), b"partial").unwrap();
        let j = area.read_journals().unwrap();
        assert_eq!(
            j,
            vec![
                ("op1".to_string(), b"{\"v\":3}".to_vec()),
                ("op2".to_string(), b"{\"v\":2}".to_vec())
            ]
        );
        area.remove_journal("op1").unwrap();
        area.remove_journal("op1").unwrap();
        assert_eq!(area.read_journals().unwrap().len(), 1);
        // A failed write (target is a directory) leaves the old state and no temp.
        std::fs::create_dir(jd.join("op7.json")).unwrap();
        assert!(area.write_journal("op7", b"x").is_err());
        let temps = std::fs::read_dir(&jd)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temps, 1);
    }

    #[test]
    fn hash_and_snapshot_any_slot() {
        let (_t, _inst, area) = setup();
        stage(&area, "op1", "Alpha", "a.lua", b"s1");
        stage(&area, "op1", "Alpha", "d/b.lua", b"s2");
        for slot in [
            Slot::Live { folder: "Alpha" },
            Slot::Staging {
                op: "op1",
                folder: "Alpha",
            },
        ] {
            let h = area.hash_slot(slot).unwrap().unwrap();
            assert!(!h.is_empty());
            assert!(area.snapshot_slot(slot).unwrap().is_some());
        }
        let staged = area
            .hash_slot(Slot::Staging {
                op: "op1",
                folder: "Alpha",
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            staged
                .iter()
                .map(|f| (f.rel_path.as_str(), f.sha256))
                .collect::<Vec<_>>(),
            vec![("a.lua", sha(b"s1")), ("d/b.lua", sha(b"s2"))]
        );
        let live = area
            .hash_slot(Slot::Live { folder: "Alpha" })
            .unwrap()
            .unwrap();
        assert_eq!(live.len(), 2);
        let none = Slot::Recovery {
            pkg: "P",
            op: "o",
            folder: "Alpha",
        };
        assert!(area.hash_slot(none).unwrap().is_none());
        assert!(area.snapshot_slot(none).unwrap().is_none());
        assert_eq!(
            area.hash_slot(Slot::Live { folder: ".." })
                .unwrap_err()
                .kind,
            FsErrorKind::OutsideRoot
        );
        // Recovery metadata round trip.
        area.write_recovery_meta("Alpha", "op1", b"{\"r\":1}")
            .unwrap();
        area.write_recovery_meta("Alpha", "op2", b"{\"r\":2}")
            .unwrap();
        assert_eq!(area.list_recovery("Alpha").unwrap().len(), 2);
        assert!(area.list_recovery("Other").unwrap().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn rename_fails_while_live_file_is_open() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_t, inst, area) = setup();
        // Open without FILE_SHARE_DELETE, as an addon-reading process would.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1 | 2)
            .open(inst.addons.join("Alpha").join("a.lua"))
            .unwrap();
        let rec = Slot::Recovery {
            pkg: "Alpha",
            op: "op1",
            folder: "Alpha",
        };
        let e = area
            .rename(Slot::Live { folder: "Alpha" }, rec)
            .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::InUse, "{e}");
        assert!(inst.addons.join("Alpha").join("a.lua").exists());
        assert!(!area.exists(rec).unwrap());
        drop(held);
        area.rename(Slot::Live { folder: "Alpha" }, rec).unwrap();
        assert!(area.exists(rec).unwrap());
    }
}
