//! AddOns scanning with hashing and a complete-collection stability check (B-02),
//! plus variant metadata reads.

use crate::fs_safe::{
    check_entry, list_dir, sha256_stream, stat_entry, walk_tree, EntryMeta, ReparseKind,
    ReparsePolicy, Root,
};
use crate::{FsError, FsErrorKind};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub struct ScanOptions {
    /// 512 MiB.
    pub max_file_bytes: u64,
    /// 100_000.
    pub max_files_per_folder: u32,
    /// A `.toc` is read from its start up to this many bytes (+1 to mark a cut); keep it equal
    /// to the core `TOC_LIMIT`. 256 KiB.
    pub max_toc_bytes: u64,
    /// 2000 (Q-15).
    pub recent_ms: u64,
    pub now_unix_ms: i64,
}

impl ScanOptions {
    pub fn with_defaults(now_unix_ms: i64) -> ScanOptions {
        ScanOptions {
            max_file_bytes: 512 * 1024 * 1024,
            max_files_per_folder: 100_000,
            max_toc_bytes: 256 * 1024,
            recent_ms: 2000,
            now_unix_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanStatus {
    Ok,
    Unreadable(String),
    Unsafe(String),
    Unstable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// '/'-separated.
    pub rel_path: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocFile {
    pub file_name: String,
    pub contents: Result<Vec<u8>, String>,
}

/// Metadata-only snapshot of a folder: every file and directory, sorted by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// (rel path, is_dir, size, mtime_ns)
    pub entries: Vec<(String, bool, u64, i128)>,
}

impl Snapshot {
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        for (p, d, s, m) in &self.entries {
            h.update((p.len() as u32).to_le_bytes());
            h.update(p.as_bytes());
            h.update([*d as u8]);
            h.update(s.to_le_bytes());
            h.update(m.to_le_bytes());
        }
        h.finalize().into()
    }

    pub fn newest_mtime_ns(&self) -> Option<i128> {
        self.entries.iter().map(|e| e.3).max()
    }

    fn first_difference(&self, other: &Snapshot) -> Option<String> {
        use std::collections::BTreeMap;
        let a: BTreeMap<_, _> = self.entries.iter().map(|e| (&e.0, e)).collect();
        let b: BTreeMap<_, _> = other.entries.iter().map(|e| (&e.0, e)).collect();
        for (k, v) in &a {
            match b.get(k) {
                None => return Some(format!("{k} removed during scan")),
                Some(w) if w != v => return Some(format!("{k} changed during scan")),
                _ => {}
            }
        }
        b.keys()
            .find(|k| !a.contains_key(*k))
            .map(|k| format!("{k} added during scan"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFolder {
    pub name: String,
    pub status: ScanStatus,
    pub files: Vec<ScannedFile>,
    /// Top-level `*.toc` files.
    pub tocs: Vec<TocFile>,
    pub fingerprint: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonsScan {
    pub folders: Vec<ScannedFolder>,
    pub listing_stable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase {
    AfterFirstSnapshot,
    AfterHashing,
}

fn snapshot_of(addons: &Root, name: &str, max_files: u32) -> Result<Option<Snapshot>, FsError> {
    tree_snapshot(addons, name, max_files)
}

/// Snapshot of `rel` under `root`; None = absent. Refuses links and unavailable entries.
pub(crate) fn tree_snapshot(
    root: &Root,
    rel: &str,
    max_files: u32,
) -> Result<Option<Snapshot>, FsError> {
    let meta = match root.check_components(rel) {
        Ok(m) => m,
        Err(e) if e.kind == FsErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let base = root.join(rel)?;
    if !meta.is_dir {
        return Err(FsError::new(
            FsErrorKind::NotPlainFile,
            &base,
            "not a directory",
        ));
    }
    let walked = walk_tree(&list_dir, &base, root.policy(), max_files)?;
    Ok(Some(Snapshot {
        entries: walked
            .into_iter()
            .map(|(p, m)| (p, m.is_dir, m.size, m.mtime_ns))
            .collect(),
    }))
}

/// Hashes every file of `rel` under `root`; None = absent.
pub(crate) fn tree_hash(
    root: &Root,
    rel: &str,
    opts: &ScanOptions,
) -> Result<Option<Vec<ScannedFile>>, FsError> {
    let Some(snap) = tree_snapshot(root, rel, opts.max_files_per_folder)? else {
        return Ok(None);
    };
    let mut files = Vec::new();
    for (p, is_dir, _, _) in &snap.entries {
        if *is_dir {
            continue;
        }
        let full = format!("{rel}/{p}");
        let mut f = root.open_read(&full, opts.max_file_bytes)?;
        let path = root.join(&full)?;
        let (size, sha256) = sha256_stream(&path, &mut f, opts.max_file_bytes)?;
        files.push(ScannedFile {
            rel_path: p.clone(),
            size,
            sha256,
        });
    }
    Ok(Some(files))
}

/// Metadata only; None = absent.
pub fn snapshot_folder(addons: &Root, name: &str) -> Result<Option<Snapshot>, FsError> {
    snapshot_of(
        addons,
        name,
        ScanOptions::with_defaults(0).max_files_per_folder,
    )
}

pub fn hash_folder(addons: &Root, name: &str) -> Result<Option<Vec<ScannedFile>>, FsError> {
    tree_hash(addons, name, &ScanOptions::with_defaults(0))
}

fn status_of(e: &FsError) -> ScanStatus {
    match e.kind {
        FsErrorKind::ReparsePoint | FsErrorKind::CloudOnly | FsErrorKind::OutsideRoot => {
            ScanStatus::Unsafe(e.to_string())
        }
        _ => ScanStatus::Unreadable(e.to_string()),
    }
}

fn failed(name: &str, status: ScanStatus) -> ScannedFolder {
    ScannedFolder {
        name: name.to_string(),
        status,
        files: Vec::new(),
        tocs: Vec::new(),
        fingerprint: None,
    }
}

pub fn scan_folder(addons: &Root, name: &str, opts: &ScanOptions) -> ScannedFolder {
    scan_folder_with_hook(addons, name, opts, &mut |_| {})
}

/// Test seam: `hook` runs after the first snapshot and after hashing.
pub fn scan_folder_with_hook(
    addons: &Root,
    name: &str,
    opts: &ScanOptions,
    hook: &mut dyn FnMut(ScanPhase),
) -> ScannedFolder {
    scan_folder_ctl(addons, name, opts, hook, &ScanCtl::default())
        .expect("a scan without a cancel request completes")
}

/// Shared between a scan and whoever watches it: `cancel` stops the scan at the next file,
/// `files` counts the files hashed so far (progress for a long read-only check).
#[derive(Clone, Default)]
pub struct ScanCtl {
    pub cancel: Arc<AtomicBool>,
    pub files: Arc<AtomicU64>,
    /// Pause after each file, so tests can make a scan slow enough to interrupt. Zero otherwise.
    pub per_file_delay_ms: u64,
}

impl ScanCtl {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn slowed(ms: u64) -> ScanCtl {
        ScanCtl {
            per_file_delay_ms: ms,
            ..ScanCtl::default()
        }
    }
}

/// `None` = cancelled before the folder was complete.
fn scan_folder_ctl(
    addons: &Root,
    name: &str,
    opts: &ScanOptions,
    hook: &mut dyn FnMut(ScanPhase),
    ctl: &ScanCtl,
) -> Option<ScannedFolder> {
    let s1 = match snapshot_of(addons, name, opts.max_files_per_folder) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Some(failed(
                name,
                ScanStatus::Unreadable("folder disappeared".into()),
            ))
        }
        Err(e) => return Some(failed(name, status_of(&e))),
    };
    hook(ScanPhase::AfterFirstSnapshot);
    let mut files = Vec::new();
    for (p, is_dir, _, _) in &s1.entries {
        if *is_dir {
            continue;
        }
        if ctl.cancelled() {
            return None;
        }
        let rel = format!("{name}/{p}");
        let hashed = addons
            .open_read(&rel, opts.max_file_bytes)
            .and_then(|mut f| {
                let path = addons.join(&rel)?;
                sha256_stream(&path, &mut f, opts.max_file_bytes)
            });
        match hashed {
            Ok((size, sha256)) => {
                files.push(ScannedFile {
                    rel_path: p.clone(),
                    size,
                    sha256,
                });
                ctl.files.fetch_add(1, Ordering::Relaxed);
                if ctl.per_file_delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(ctl.per_file_delay_ms));
                }
            }
            Err(e) => return Some(failed(name, status_of(&e))),
        }
    }
    let mut tocs = Vec::new();
    for (p, is_dir, _, _) in &s1.entries {
        if *is_dir || p.contains('/') || !p.to_ascii_lowercase().ends_with(".toc") {
            continue;
        }
        tocs.push(TocFile {
            file_name: p.clone(),
            contents: read_bounded(addons, &format!("{name}/{p}"), opts.max_toc_bytes),
        });
    }
    hook(ScanPhase::AfterHashing);
    let s2 = match snapshot_of(addons, name, opts.max_files_per_folder) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Some(failed(
                name,
                ScanStatus::Unstable("folder removed during scan".into()),
            ))
        }
        Err(e) => return Some(failed(name, status_of(&e))),
    };
    let fingerprint = Some(s2.fingerprint());
    let status = if let Some(d) = s1.first_difference(&s2) {
        ScanStatus::Unstable(d)
    } else if let Some(newest) = s2
        .newest_mtime_ns()
        .filter(|n| *n >= (opts.now_unix_ms as i128 - opts.recent_ms as i128) * 1_000_000)
    {
        ScanStatus::Unstable(format!(
            "modified within the last {} ms (mtime {newest} ns)",
            opts.recent_ms
        ))
    } else {
        ScanStatus::Ok
    };
    Some(ScannedFolder {
        name: name.to_string(),
        status,
        files,
        tocs,
        fingerprint,
    })
}

/// Reads at most `limit + 1` bytes from the start of the file: a longer file is never loaded
/// whole, and the extra byte tells the parser the file was cut.
fn read_bounded(root: &Root, rel: &str, limit: u64) -> Result<Vec<u8>, String> {
    let f = root.open_read(rel, u64::MAX).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.take(limit + 1)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

fn top_level(addons: &Root) -> Result<Vec<EntryMeta>, FsError> {
    let mut v: Vec<EntryMeta> = addons
        .list("")?
        .into_iter()
        .filter(|e| e.is_dir || e.reparse != ReparseKind::None)
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

fn kinds(v: &[EntryMeta]) -> Vec<(String, bool, bool)> {
    v.iter()
        .map(|e| (e.name.clone(), e.is_dir, e.reparse != ReparseKind::None))
        .collect()
}

pub fn scan_addons(addons: &Root, opts: &ScanOptions) -> Result<AddonsScan, FsError> {
    scan_addons_with_hook(addons, opts, &mut |_| {})
}

/// Like `scan_addons`; `hook` runs with the folder name after each folder was scanned.
pub fn scan_addons_with_hook(
    addons: &Root,
    opts: &ScanOptions,
    hook: &mut dyn FnMut(&str),
) -> Result<AddonsScan, FsError> {
    Ok(scan_addons_ctl(addons, opts, hook, &ScanCtl::default())?
        .expect("a scan without a cancel request completes"))
}

/// Like `scan_addons_with_hook`, but `ctl` can cancel it (`Ok(None)`) and shows its progress.
pub fn scan_addons_ctl(
    addons: &Root,
    opts: &ScanOptions,
    hook: &mut dyn FnMut(&str),
    ctl: &ScanCtl,
) -> Result<Option<AddonsScan>, FsError> {
    let l1 = top_level(addons)?;
    let mut folders = Vec::new();
    for e in &l1 {
        if ctl.cancelled() {
            return Ok(None);
        }
        let f = if e.reparse != ReparseKind::None || !e.is_dir {
            let why = check_entry(e, ReparsePolicy::RejectAll, &addons.path().join(&e.name))
                .err()
                .map(|x| x.to_string())
                .unwrap_or_else(|| "reparse point".into());
            failed(&e.name, ScanStatus::Unsafe(why))
        } else {
            match check_entry(e, addons.policy(), &addons.path().join(&e.name)) {
                Ok(()) => match scan_folder_ctl(addons, &e.name, opts, &mut |_| {}, ctl) {
                    Some(f) => f,
                    None => return Ok(None),
                },
                Err(x) => failed(&e.name, status_of(&x)),
            }
        };
        hook(&e.name);
        folders.push(f);
    }
    let l2 = top_level(addons)?;
    Ok(Some(AddonsScan {
        folders,
        listing_stable: kinds(&l1) == kinds(&l2),
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRead {
    Missing,
    Unreadable(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirKind {
    Missing,
    PlainDir,
    NotPlain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantFiles {
    pub folder_name: String,
    pub flavor_info: FileRead,
    pub build_info: FileRead,
    pub addons_dir: DirKind,
    /// `<install root>/.wowsync-synthetic`
    pub synthetic_marker: FileRead,
}

const VARIANT_FILE_LIMIT: u64 = 1024 * 1024;

fn read_file(root: &Root, rel: &str) -> FileRead {
    match root.open_read(rel, VARIANT_FILE_LIMIT) {
        Ok(mut f) => {
            let mut b = Vec::new();
            match f.by_ref().take(VARIANT_FILE_LIMIT + 1).read_to_end(&mut b) {
                Ok(_) if b.len() as u64 <= VARIANT_FILE_LIMIT => FileRead::Bytes(b),
                Ok(_) => FileRead::Unreadable("larger than 1 MiB".into()),
                Err(e) => FileRead::Unreadable(e.to_string()),
            }
        }
        Err(e) if e.kind == FsErrorKind::NotFound => FileRead::Missing,
        Err(e) => FileRead::Unreadable(e.to_string()),
    }
}

/// Reads at most 1 MiB per file. `variant_folder` must be a single plain segment.
pub fn read_variant_files(install_root: &Path, variant_folder: &str) -> VariantFiles {
    let mk = |f: FileRead, d: DirKind| VariantFiles {
        folder_name: variant_folder.to_string(),
        flavor_info: f.clone(),
        build_info: f.clone(),
        addons_dir: d,
        synthetic_marker: f,
    };
    let root = match Root::open(install_root, ReparsePolicy::RejectAll) {
        Ok(r) => r,
        Err(e) if e.kind == FsErrorKind::NotFound => {
            return mk(FileRead::Missing, DirKind::Missing)
        }
        Err(e) => return mk(FileRead::Unreadable(e.to_string()), DirKind::NotPlain),
    };
    if let Err(e) = crate::fs_safe::validate_segment(variant_folder) {
        return mk(FileRead::Unreadable(e), DirKind::NotPlain);
    }
    let addons_dir = match root.check_components(&format!("{variant_folder}/Interface/AddOns")) {
        Ok(m) if m.is_dir => DirKind::PlainDir,
        Ok(_) => DirKind::NotPlain,
        Err(e) if e.kind == FsErrorKind::NotFound => DirKind::Missing,
        Err(_) => DirKind::NotPlain,
    };
    VariantFiles {
        folder_name: variant_folder.to_string(),
        flavor_info: read_file(&root, &format!("{variant_folder}/.flavor.info")),
        build_info: read_file(&root, ".build.info"),
        addons_dir,
        synthetic_marker: read_file(&root, ".wowsync-synthetic"),
    }
}

fn is_variant_name(n: &str) -> bool {
    let b = n.as_bytes();
    b.len() >= 3
        && b[0] == b'_'
        && b[b.len() - 1] == b'_'
        && b[1..b.len() - 1]
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// Plain directories matching `^_[a-z0-9_]+_$` that contain a plain `.flavor.info`.
pub fn list_variant_folders(install_root: &Path) -> Vec<String> {
    let Ok(root) = Root::open(install_root, ReparsePolicy::RejectAll) else {
        return Vec::new();
    };
    let Ok(entries) = root.list("") else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .into_iter()
        .filter(|e| {
            e.is_dir
                && e.reparse == ReparseKind::None
                && is_variant_name(&e.name)
                && stat_entry(&install_root.join(&e.name).join(".flavor.info"))
                    .map(|m| !m.is_dir && m.reparse == ReparseKind::None)
                    .unwrap_or(false)
        })
        .map(|e| e.name)
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{set_mtime, SyntheticInstall, TempDir};
    use std::time::{Duration, SystemTime};

    const FAR_FUTURE_MS: i64 = 4_000_000_000_000;

    fn opts() -> ScanOptions {
        ScanOptions::with_defaults(FAR_FUTURE_MS)
    }

    fn setup_fresh() -> (TempDir, SyntheticInstall, Root) {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        inst.add_addon(
            "Alpha",
            "Alpha.toc",
            "## Version: 1\n",
            &[("a.lua", b"one"), ("sub/b.lua", b"two")],
        );
        let root = Root::open(&inst.addons, ReparsePolicy::RejectAll).unwrap();
        (t, inst, root)
    }

    /// Same install with mtimes moved out of the stability window.
    fn setup() -> (TempDir, SyntheticInstall, Root) {
        let (t, inst, root) = setup_fresh();
        inst.set_old_mtimes();
        (t, inst, root)
    }

    fn sha(b: &[u8]) -> [u8; 32] {
        Sha256::digest(b).into()
    }

    #[test]
    fn a_cancelled_scan_stops_at_the_next_file_and_reports_progress() {
        let (_t, _i, root) = setup();
        let ctl = ScanCtl::default();
        let done = scan_addons_ctl(&root, &opts(), &mut |_| {}, &ctl).unwrap();
        assert!(done.is_some());
        assert_eq!(ctl.files.load(Ordering::Relaxed), 3, "three files hashed");
        let ctl = ScanCtl::default();
        let mut hook = |p: ScanPhase| {
            if p == ScanPhase::AfterFirstSnapshot {
                ctl.cancel.store(true, Ordering::Relaxed);
            }
        };
        assert!(scan_folder_ctl(&root, "Alpha", &opts(), &mut hook, &ctl).is_none());
        assert_eq!(ctl.files.load(Ordering::Relaxed), 0, "no file was read");
        let ctl = ScanCtl::default();
        ctl.cancel.store(true, Ordering::Relaxed);
        assert!(scan_addons_ctl(&root, &opts(), &mut |_| {}, &ctl)
            .unwrap()
            .is_none());
    }

    #[test]
    fn scan_hashes_files_and_reads_tocs() {
        let (_t, _i, root) = setup();
        let f = scan_folder(&root, "Alpha", &opts());
        assert_eq!(f.status, ScanStatus::Ok, "{:?}", f.status);
        let got: Vec<(&str, u64, [u8; 32])> = f
            .files
            .iter()
            .map(|x| (x.rel_path.as_str(), x.size, x.sha256))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Alpha.toc", 14, sha(b"## Version: 1\n")),
                ("a.lua", 3, sha(b"one")),
                ("sub/b.lua", 3, sha(b"two")),
            ]
        );
        assert_eq!(f.tocs.len(), 1);
        assert_eq!(f.tocs[0].file_name, "Alpha.toc");
        assert_eq!(f.tocs[0].contents, Ok(b"## Version: 1\n".to_vec()));
        assert!(f.fingerprint.is_some());
        // An oversized toc is read from its start only: the limit plus one byte, so the parser
        // can tell it was cut. The whole file still goes into the tree hash.
        let mut o = opts();
        o.max_toc_bytes = 4;
        let f = scan_folder(&root, "Alpha", &o);
        assert_eq!(f.status, ScanStatus::Ok);
        assert_eq!(f.tocs[0].contents, Ok(b"## Ve".to_vec()));
        assert_eq!(f.files[0].size, 14);
        assert_eq!(f.files[0].sha256, sha(b"## Version: 1\n"));
        // Oversized file makes the folder unreadable.
        let mut o = opts();
        o.max_file_bytes = 2;
        assert!(matches!(
            scan_folder(&root, "Alpha", &o).status,
            ScanStatus::Unreadable(_)
        ));
        assert_eq!(hash_folder(&root, "Alpha").unwrap().unwrap().len(), 3);
        assert_eq!(hash_folder(&root, "Nope").unwrap(), None);
    }

    #[test]
    fn oversized_toc_is_read_as_a_bounded_prefix() {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        let mut toc = b"## Interface: 11508\n## Version: 3\n".to_vec();
        while toc.len() < 1_100_000 {
            toc.extend_from_slice(b"Db\\Row_000000.lua\n");
        }
        inst.add_addon("Big", "Big.toc", "", &[("a.lua", b"one")]);
        std::fs::write(inst.addons.join("Big").join("Big.toc"), &toc).unwrap();
        inst.set_old_mtimes();
        let root = Root::open(&inst.addons, ReparsePolicy::RejectAll).unwrap();
        let f = scan_folder(&root, "Big", &opts());
        assert_eq!(f.status, ScanStatus::Ok, "{:?}", f.status);
        let read = f.tocs[0].contents.as_ref().unwrap();
        assert_eq!(read.len() as u64, opts().max_toc_bytes + 1);
        assert_eq!(&read[..], &toc[..read.len()]);
        let hashed = f.files.iter().find(|x| x.rel_path == "Big.toc").unwrap();
        assert_eq!(hashed.size, toc.len() as u64);
        assert_eq!(hashed.sha256, sha(&toc));
    }

    #[test]
    fn top_level_files_ignored() {
        let (_t, inst, root) = setup();
        std::fs::write(inst.addons.join("stray.txt"), b"x").unwrap();
        let s = scan_addons(&root, &opts()).unwrap();
        assert_eq!(s.folders.len(), 1);
        assert_eq!(s.folders[0].name, "Alpha");
        assert!(s.listing_stable);
    }

    #[test]
    fn recently_modified_file_is_unstable() {
        let (_t, inst, root) = setup_fresh();
        let now = crate::sysinfo::now_unix_ms();
        let f = scan_folder(&root, "Alpha", &ScanOptions::with_defaults(now));
        assert!(
            matches!(f.status, ScanStatus::Unstable(_)),
            "{:?}",
            f.status
        );
        inst.set_old_mtimes();
        let f = scan_folder(&root, "Alpha", &ScanOptions::with_defaults(now));
        assert_eq!(f.status, ScanStatus::Ok, "{:?}", f.status);
    }

    #[test]
    fn file_changed_after_hash_marks_unstable() {
        let (_t, inst, root) = setup();
        let p = inst.addons.join("Alpha").join("a.lua");
        let f = scan_folder_with_hook(&root, "Alpha", &opts(), &mut |ph| {
            if ph == ScanPhase::AfterHashing {
                std::fs::write(&p, b"changed!").unwrap();
                set_mtime(&p, SystemTime::now() - Duration::from_secs(10));
            }
        });
        assert!(
            matches!(&f.status, ScanStatus::Unstable(d) if d.contains("a.lua")),
            "{:?}",
            f.status
        );
        // A change before hashing (between snapshot and hash) is also caught by size/mtime.
        let f = scan_folder_with_hook(&root, "Alpha", &opts(), &mut |ph| {
            if ph == ScanPhase::AfterFirstSnapshot {
                std::fs::write(&p, b"changed again, longer").unwrap();
            }
        });
        assert!(
            matches!(f.status, ScanStatus::Unstable(_)),
            "{:?}",
            f.status
        );
    }

    #[test]
    fn file_added_or_removed_during_scan_marks_unstable() {
        let (_t, inst, root) = setup();
        let dir = inst.addons.join("Alpha");
        let f = scan_folder_with_hook(&root, "Alpha", &opts(), &mut |ph| {
            if ph == ScanPhase::AfterHashing {
                std::fs::write(dir.join("new.lua"), b"n").unwrap();
            }
        });
        assert!(
            matches!(&f.status, ScanStatus::Unstable(d) if d.contains("new.lua")),
            "{:?}",
            f.status
        );
        let f = scan_folder_with_hook(&root, "Alpha", &opts(), &mut |ph| {
            if ph == ScanPhase::AfterFirstSnapshot {
                std::fs::remove_file(dir.join("a.lua")).unwrap();
            }
        });
        assert!(
            matches!(
                f.status,
                ScanStatus::Unreadable(_) | ScanStatus::Unstable(_)
            ),
            "{:?}",
            f.status
        );
        let f = scan_folder_with_hook(&root, "Alpha", &opts(), &mut |ph| {
            if ph == ScanPhase::AfterHashing {
                std::fs::create_dir(dir.join("emptydir")).unwrap();
            }
        });
        assert!(
            matches!(f.status, ScanStatus::Unstable(_)),
            "{:?}",
            f.status
        );
    }

    #[test]
    fn top_level_listing_change_detected() {
        let (_t, inst, root) = setup();
        let mut added = false;
        let s = scan_addons_with_hook(&root, &opts(), &mut |_| {
            if !added {
                std::fs::create_dir(inst.addons.join("Beta")).unwrap();
                added = true;
            }
        })
        .unwrap();
        assert!(!s.listing_stable);
        let s = scan_addons(&root, &opts()).unwrap();
        assert!(s.listing_stable);
        assert_eq!(s.folders.len(), 2);
    }

    #[test]
    fn fingerprint_covers_dirs_size_and_mtime() {
        let (_t, inst, root) = setup();
        let base = snapshot_folder(&root, "Alpha")
            .unwrap()
            .unwrap()
            .fingerprint();
        assert_eq!(
            base,
            snapshot_folder(&root, "Alpha")
                .unwrap()
                .unwrap()
                .fingerprint()
        );
        let dir = inst.addons.join("Alpha");
        let old = SystemTime::now() - Duration::from_secs(500);
        // mtime only
        set_mtime(&dir.join("a.lua"), old);
        let f1 = snapshot_folder(&root, "Alpha")
            .unwrap()
            .unwrap()
            .fingerprint();
        assert_ne!(base, f1);
        // size only (restore mtime)
        std::fs::write(dir.join("a.lua"), b"onex").unwrap();
        set_mtime(&dir.join("a.lua"), old);
        let f2 = snapshot_folder(&root, "Alpha")
            .unwrap()
            .unwrap()
            .fingerprint();
        assert_ne!(f1, f2);
        // directory membership
        std::fs::create_dir(dir.join("emptydir")).unwrap();
        let f3 = snapshot_folder(&root, "Alpha")
            .unwrap()
            .unwrap()
            .fingerprint();
        assert_ne!(f2, f3);
        let s = snapshot_folder(&root, "Alpha").unwrap().unwrap();
        assert!(s.entries.iter().any(|e| e.0 == "emptydir" && e.1));
        assert!(s.newest_mtime_ns().is_some());
        assert!(snapshot_folder(&root, "Nope").unwrap().is_none());
    }

    #[test]
    fn read_variant_files_reports_missing_and_bytes() {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        let v = read_variant_files(&inst.root, "_forever_");
        assert!(matches!(v.build_info, FileRead::Bytes(_)));
        assert!(matches!(v.flavor_info, FileRead::Bytes(_)));
        assert_eq!(v.addons_dir, DirKind::PlainDir);
        assert_eq!(v.folder_name, "_forever_");
        std::fs::remove_file(inst.variant_dir.join(".flavor.info")).unwrap();
        std::fs::remove_dir_all(&inst.addons).unwrap();
        std::fs::write(inst.root.join(".build.info"), vec![0u8; 1024 * 1024 + 1]).unwrap();
        let v = read_variant_files(&inst.root, "_forever_");
        assert_eq!(v.flavor_info, FileRead::Missing);
        assert_eq!(v.addons_dir, DirKind::Missing);
        assert!(matches!(v.build_info, FileRead::Unreadable(_)));
        let v = read_variant_files(&inst.root, "_nope_");
        assert_eq!(v.flavor_info, FileRead::Missing);
        assert_eq!(v.addons_dir, DirKind::Missing);
        let v = read_variant_files(&t.path().join("absent"), "_forever_");
        assert_eq!(v.build_info, FileRead::Missing);
        let v = read_variant_files(&inst.root, "..");
        assert!(matches!(v.flavor_info, FileRead::Unreadable(_)));
        // AddOns that is a file is not a plain directory.
        std::fs::write(&inst.addons, b"x").unwrap();
        assert_eq!(
            read_variant_files(&inst.root, "_forever_").addons_dir,
            DirKind::NotPlain
        );
    }

    #[test]
    fn list_variant_folders_matches_pattern() {
        let t = TempDir::new();
        let root = t.path().join("wow");
        for (n, flavor) in [
            ("_forever_", true),
            ("_classic_era_", true),
            ("_retail_", false),
            ("_Upper_", true),
            ("Interface", true),
            ("_x", true),
            ("__", true),
            ("_a-b_", true),
        ] {
            std::fs::create_dir_all(root.join(n)).unwrap();
            if flavor {
                std::fs::write(root.join(n).join(".flavor.info"), "x").unwrap();
            }
        }
        std::fs::write(root.join("_file_"), "x").unwrap();
        assert_eq!(
            list_variant_folders(&root),
            vec!["_classic_era_".to_string(), "_forever_".to_string()]
        );
        assert!(list_variant_folders(&t.path().join("missing")).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn nested_junction_marks_folder_unsafe() {
        let (t, inst, root) = setup();
        let outside = t.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.lua"), b"s").unwrap();
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(inst.addons.join("Alpha").join("jn"))
            .arg(&outside)
            .output()
            .unwrap();
        assert!(out.status.success());
        let f = scan_folder(&root, "Alpha", &opts());
        assert!(matches!(f.status, ScanStatus::Unsafe(_)), "{:?}", f.status);
        assert!(f.files.is_empty() && f.fingerprint.is_none());
        // A top-level junction is reported Unsafe and does not hide the other folders.
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(inst.addons.join("Linked"))
            .arg(&outside)
            .output()
            .unwrap();
        assert!(out.status.success());
        let s = scan_addons(&root, &opts()).unwrap();
        let linked = s.folders.iter().find(|f| f.name == "Linked").unwrap();
        assert!(matches!(linked.status, ScanStatus::Unsafe(_)));
        assert!(s.listing_stable);
        // The junction target is never read through.
        assert!(hash_folder(&root, "Alpha").is_err());
        assert!(hash_folder(&root, "Linked").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn locked_file_marks_folder_unreadable() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_t, inst, root) = setup();
        let p = inst.addons.join("Alpha").join("a.lua");
        // Exclusive handle with no sharing at all, like an application holding a file.
        let _held = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&p)
            .unwrap();
        let f = scan_folder(&root, "Alpha", &opts());
        assert!(
            matches!(&f.status, ScanStatus::Unreadable(d) if d.contains("InUse")),
            "{:?}",
            f.status
        );
        assert!(f.files.is_empty());
    }
}
