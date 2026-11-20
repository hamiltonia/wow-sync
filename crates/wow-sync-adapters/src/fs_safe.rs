//! Authorized roots, containment, reparse/recall classification and enumeration.
//!
//! All classification comes from directory enumeration data. No handle to a
//! child is opened to decide whether it is safe to touch.

use crate::{FsError, FsErrorKind};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const ATTR_DIRECTORY: u32 = 0x0000_0010;
pub const ATTR_REPARSE_POINT: u32 = 0x0000_0400;
pub const ATTR_OFFLINE: u32 = 0x0000_1000;
/// Meaning valid for enumeration data only.
pub const ATTR_RECALL_ON_OPEN: u32 = 0x0004_0000;
pub const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

const TAG_SYMLINK: u32 = 0xA000_000C;
const TAG_MOUNT_POINT: u32 = 0xA000_0003;
const TAG_CLOUD_FAMILY: u32 = 0x9000_001A;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReparseKind {
    None,
    CloudFile {
        tag: u32,
    },
    /// Symlink, junction or mount point.
    Link {
        tag: u32,
    },
    Other {
        tag: u32,
    },
}

/// One entry as reported by enumerating its parent directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMeta {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_ns: i128,
    pub attrs: u32,
    pub reparse: ReparseKind,
}

/// Cloud-files family: `(tag & 0xFFFF0FFF) == 0x9000001A` (Dropbox: 0x9000601A, 0x9000201A).
pub fn classify_tag(tag: u32) -> ReparseKind {
    if tag & 0xFFFF_0FFF == TAG_CLOUD_FAMILY {
        ReparseKind::CloudFile { tag }
    } else if tag == TAG_SYMLINK || tag == TAG_MOUNT_POINT {
        ReparseKind::Link { tag }
    } else {
        ReparseKind::Other { tag }
    }
}

/// True if any recall/offline indication is set, for files and directories alike.
pub fn is_unavailable(enum_attrs: u32) -> bool {
    enum_attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReparsePolicy {
    /// Any reparse point or unavailable entry is refused.
    RejectAll,
    /// Cloud-file tags are accepted when the entry is not unavailable.
    AllowCloudFiles,
}

/// Checks one entry against a policy.
pub fn check_entry(meta: &EntryMeta, policy: ReparsePolicy, path: &Path) -> Result<(), FsError> {
    match meta.reparse {
        ReparseKind::None => {}
        ReparseKind::CloudFile { .. } if policy == ReparsePolicy::AllowCloudFiles => {}
        other => {
            return Err(FsError::new(
                FsErrorKind::ReparsePoint,
                path,
                format!("reparse point refused: {other:?}"),
            ))
        }
    }
    if is_unavailable(meta.attrs) {
        return Err(FsError::new(
            FsErrorKind::CloudOnly,
            path,
            format!(
                "entry is not locally available (attributes 0x{:x})",
                meta.attrs
            ),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn ns_from_unix(d: std::io::Result<std::time::SystemTime>) -> i128 {
    match d {
        Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_nanos() as i128,
            Err(e) => -(e.duration().as_nanos() as i128),
        },
        Err(_) => 0,
    }
}

#[cfg(windows)]
mod sys {
    use super::*;
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
        FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
    };

    /// NUL-terminated UTF-16 path, with the verbatim prefix for long absolute paths.
    pub fn wide_path(p: &Path) -> Vec<u16> {
        let mut v: Vec<u16> = p.as_os_str().encode_wide().collect();
        let is_verbatim = v.starts_with(&[92, 92, 63, 92]);
        let is_drive = v.len() >= 3 && v[1] == 58 && v[2] == 92;
        let is_unc = v.len() > 2 && v[0] == 92 && v[1] == 92;
        if v.len() >= 240 && !is_verbatim {
            if is_drive {
                let mut w: Vec<u16> = "\\\\?\\".encode_utf16().collect();
                w.extend(v);
                v = w;
            } else if is_unc {
                let mut w: Vec<u16> = "\\\\?\\UNC\\".encode_utf16().collect();
                w.extend(&v[2..]);
                v = w;
            }
        }
        v.push(0);
        v
    }

    fn filetime_ns(hi: u32, lo: u32) -> i128 {
        let t = ((hi as u64) << 32 | lo as u64) as i128;
        (t - 116_444_736_000_000_000) * 100
    }

    fn entry(d: &WIN32_FIND_DATAW) -> EntryMeta {
        let len = d
            .cFileName
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(d.cFileName.len());
        let name = OsString::from_wide(&d.cFileName[..len])
            .to_string_lossy()
            .into_owned();
        let attrs = d.dwFileAttributes;
        let reparse = if attrs & ATTR_REPARSE_POINT != 0 {
            classify_tag(d.dwReserved0)
        } else {
            ReparseKind::None
        };
        EntryMeta {
            name,
            is_dir: attrs & ATTR_DIRECTORY != 0,
            size: (d.nFileSizeHigh as u64) << 32 | d.nFileSizeLow as u64,
            mtime_ns: filetime_ns(
                d.ftLastWriteTime.dwHighDateTime,
                d.ftLastWriteTime.dwLowDateTime,
            ),
            attrs,
            reparse,
        }
    }

    fn os_error(path: &Path, code: u32) -> FsError {
        FsError::from_io(path, &std::io::Error::from_raw_os_error(code as i32))
    }

    fn find_first(pattern: &[u16], data: &mut WIN32_FIND_DATAW) -> Result<isize, u32> {
        // SAFETY: `pattern` is NUL-terminated and `data` is a valid WIN32_FIND_DATAW.
        let h = unsafe {
            FindFirstFileExW(
                pattern.as_ptr(),
                FindExInfoBasic,
                data as *mut WIN32_FIND_DATAW as *mut c_void,
                FindExSearchNameMatch,
                std::ptr::null(),
                FIND_FIRST_EX_LARGE_FETCH,
            )
        };
        if h == INVALID_HANDLE_VALUE {
            // SAFETY: plain thread-local error read.
            Err(unsafe { GetLastError() })
        } else {
            Ok(h as isize)
        }
    }

    pub fn list_dir(dir: &Path) -> Result<Vec<EntryMeta>, FsError> {
        let mut pattern = dir.as_os_str().to_owned();
        let s = pattern.to_string_lossy();
        if !(s.ends_with('\\') || s.ends_with('/')) {
            pattern.push("\\");
        }
        pattern.push("*");
        let wide = wide_path(Path::new(&pattern));
        // SAFETY: zeroed WIN32_FIND_DATAW is a valid initial value (plain integers/arrays).
        let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
        let h = match find_first(&wide, &mut data) {
            Ok(h) => h,
            Err(2) => return Ok(Vec::new()),
            Err(267) => {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    dir,
                    "not a directory",
                ))
            }
            Err(code) => return Err(os_error(dir, code)),
        };
        let mut out = Vec::new();
        loop {
            let e = entry(&data);
            if e.name != "." && e.name != ".." {
                out.push(e);
            }
            // SAFETY: `h` is a live search handle and `data` is valid.
            if unsafe { FindNextFileW(h as _, &mut data) } == 0 {
                break;
            }
        }
        // SAFETY: `h` came from FindFirstFileExW and is closed exactly once.
        unsafe { FindClose(h as _) };
        Ok(out)
    }

    pub fn stat_entry(path: &Path) -> Result<EntryMeta, FsError> {
        if let Some(name) = path.file_name() {
            let n = name.to_string_lossy();
            if n.contains('*') || n.contains('?') {
                return Err(FsError::new(
                    FsErrorKind::Io,
                    path,
                    "wildcard in entry name",
                ));
            }
        }
        let wide = wide_path(path);
        // SAFETY: see list_dir.
        let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
        match find_first(&wide, &mut data) {
            Ok(h) => {
                // SAFETY: valid search handle closed once.
                unsafe { FindClose(h as _) };
                Ok(entry(&data))
            }
            Err(3) | Err(2) => Err(FsError::new(FsErrorKind::NotFound, path, "not found")),
            Err(code) => Err(os_error(path, code)),
        }
    }
}

#[cfg(not(windows))]
mod sys {
    use super::*;

    fn meta_of(name: String, m: &std::fs::Metadata) -> EntryMeta {
        let is_link = m.file_type().is_symlink();
        EntryMeta {
            name,
            is_dir: m.is_dir(),
            size: if m.is_dir() { 0 } else { m.len() },
            mtime_ns: ns_from_unix(m.modified()),
            attrs: 0,
            reparse: if is_link {
                ReparseKind::Link { tag: TAG_SYMLINK }
            } else {
                ReparseKind::None
            },
        }
    }

    pub fn list_dir(dir: &Path) -> Result<Vec<EntryMeta>, FsError> {
        let rd = std::fs::read_dir(dir).map_err(|e| FsError::from_io(dir, &e))?;
        let mut out = Vec::new();
        for e in rd {
            let e = e.map_err(|e| FsError::from_io(dir, &e))?;
            let m = std::fs::symlink_metadata(e.path()).map_err(|x| FsError::from_io(dir, &x))?;
            out.push(meta_of(e.file_name().to_string_lossy().into_owned(), &m));
        }
        Ok(out)
    }

    pub fn stat_entry(path: &Path) -> Result<EntryMeta, FsError> {
        let m = std::fs::symlink_metadata(path).map_err(|e| FsError::from_io(path, &e))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(meta_of(name, &m))
    }
}

/// Enumerates `dir`. No handle to any child is opened.
pub fn list_dir(dir: &Path) -> Result<Vec<EntryMeta>, FsError> {
    sys::list_dir(dir)
}

/// The entry for one path, as its parent's record reports it.
pub fn stat_entry(path: &Path) -> Result<EntryMeta, FsError> {
    if path.parent().is_none() {
        return Ok(EntryMeta {
            name: path.to_string_lossy().into_owned(),
            is_dir: true,
            size: 0,
            mtime_ns: 0,
            attrs: ATTR_DIRECTORY,
            reparse: ReparseKind::None,
        });
    }
    sys::stat_entry(path)
}

const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// One path segment: rejects empty, `.`/`..`, separators, drive colons, wildcards,
/// control characters, trailing dot/space and Windows device names.
pub fn validate_segment(seg: &str) -> Result<(), String> {
    if seg.is_empty() {
        return Err("empty segment".into());
    }
    if seg == "." || seg == ".." {
        return Err(format!("segment '{seg}' not allowed"));
    }
    if let Some(c) = seg.chars().find(|c| {
        matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || (*c as u32) < 0x20
    }) {
        return Err(format!("character {c:?} not allowed in segment"));
    }
    if seg.ends_with('.') || seg.ends_with(' ') {
        return Err("segment ends with dot or space".into());
    }
    let base = seg.split('.').next().unwrap_or("").to_ascii_lowercase();
    if RESERVED.contains(&base.as_str()) {
        return Err(format!("reserved device name '{seg}'"));
    }
    Ok(())
}

/// Splits and validates a '/'-separated relative path.
pub fn split_rel(rel: &str) -> Result<Vec<&str>, String> {
    if rel.is_empty() {
        return Err("empty relative path".into());
    }
    let segs: Vec<&str> = rel.split('/').collect();
    for s in &segs {
        validate_segment(s)?;
    }
    Ok(segs)
}

#[derive(Debug, Clone)]
pub struct Root {
    path: PathBuf,
    policy: ReparsePolicy,
}

impl Root {
    /// Every component from the drive down is checked under `policy`; the path must
    /// be absolute, free of `..`, and name a directory.
    pub fn open(path: &Path, policy: ReparsePolicy) -> Result<Root, FsError> {
        if !path.is_absolute() {
            return Err(FsError::new(
                FsErrorKind::OutsideRoot,
                path,
                "root path must be absolute",
            ));
        }
        let mut cur = PathBuf::new();
        for c in path.components() {
            match c {
                Component::Prefix(_) | Component::RootDir => cur.push(c.as_os_str()),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(FsError::new(
                        FsErrorKind::OutsideRoot,
                        path,
                        "'..' in root path",
                    ))
                }
                Component::Normal(n) => {
                    cur.push(n);
                    let m = stat_entry(&cur)?;
                    check_entry(&m, policy, &cur)?;
                    if !m.is_dir {
                        return Err(FsError::new(
                            FsErrorKind::NotPlainFile,
                            &cur,
                            "not a directory",
                        ));
                    }
                }
            }
        }
        Ok(Root { path: cur, policy })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn policy(&self) -> ReparsePolicy {
        self.policy
    }

    fn segs<'a>(&self, rel: &'a str) -> Result<Vec<&'a str>, FsError> {
        split_rel(rel).map_err(|d| {
            FsError::new(
                FsErrorKind::OutsideRoot,
                &self.path,
                format!("{d}: {rel:?}"),
            )
        })
    }

    /// Joins a '/'-separated relative path.
    pub fn join(&self, rel: &str) -> Result<PathBuf, FsError> {
        let mut p = self.path.clone();
        for s in self.segs(rel)? {
            p.push(s);
        }
        Ok(p)
    }

    /// Like `join` but "" names the root itself.
    pub fn join_dir(&self, rel_dir: &str) -> Result<PathBuf, FsError> {
        if rel_dir.is_empty() {
            Ok(self.path.clone())
        } else {
            self.join(rel_dir)
        }
    }

    /// `stat_entry` of each component under the root, checked against the policy.
    pub fn check_components(&self, rel: &str) -> Result<EntryMeta, FsError> {
        let segs = self.segs(rel)?;
        let mut cur = self.path.clone();
        let mut last = None;
        for (i, s) in segs.iter().enumerate() {
            cur.push(s);
            let m = stat_entry(&cur)?;
            check_entry(&m, self.policy, &cur)?;
            if i + 1 < segs.len() && !m.is_dir {
                return Err(FsError::new(
                    FsErrorKind::NotFound,
                    &cur,
                    "parent is not a directory",
                ));
            }
            last = Some(m);
        }
        last.ok_or_else(|| FsError::new(FsErrorKind::OutsideRoot, &self.path, "empty path"))
    }

    /// The only way to open a file for reading.
    pub fn open_read(&self, rel: &str, limit: u64) -> Result<File, FsError> {
        let meta = self.check_components(rel)?;
        let path = self.join(rel)?;
        if meta.is_dir {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                &path,
                "is a directory",
            ));
        }
        if meta.size > limit {
            return Err(FsError::new(
                FsErrorKind::TooLarge,
                &path,
                format!("{} bytes exceeds limit {limit}", meta.size),
            ));
        }
        let mut oo = OpenOptions::new();
        oo.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const FILE_FLAG_OPEN_NO_RECALL: u32 = 0x0010_0000;
            const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
            oo.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL);
        }
        let f = oo.open(&path).map_err(|e| FsError::from_io(&path, &e))?;
        let m = f.metadata().map_err(|e| FsError::from_io(&path, &e))?;
        if !m.is_file() {
            return Err(FsError::new(
                FsErrorKind::NotPlainFile,
                &path,
                "not a regular file",
            ));
        }
        Ok(f)
    }

    /// Raw enumeration of a checked directory ("" is the root). Children are not
    /// policy-checked; callers apply `check_entry`.
    pub fn list(&self, rel_dir: &str) -> Result<Vec<EntryMeta>, FsError> {
        let path = self.join_dir(rel_dir)?;
        if !rel_dir.is_empty() {
            let m = self.check_components(rel_dir)?;
            if !m.is_dir {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    &path,
                    "not a directory",
                ));
            }
        }
        list_dir(&path)
    }

    /// Creates missing directories one component at a time, re-checking each.
    pub fn ensure_dir(&self, rel_dir: &str) -> Result<PathBuf, FsError> {
        let mut cur = self.path.clone();
        if rel_dir.is_empty() {
            return Ok(cur);
        }
        for s in self.segs(rel_dir)? {
            cur.push(s);
            let m = match stat_entry(&cur) {
                Ok(m) => m,
                Err(e) if e.kind == FsErrorKind::NotFound => {
                    match std::fs::create_dir(&cur) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(FsError::from_io(&cur, &e)),
                    }
                    stat_entry(&cur)?
                }
                Err(e) => return Err(e),
            };
            check_entry(&m, self.policy, &cur)?;
            if !m.is_dir {
                return Err(FsError::new(
                    FsErrorKind::NotPlainFile,
                    &cur,
                    "not a directory",
                ));
            }
        }
        Ok(cur)
    }

    /// Writes `rel` through a same-directory temp file: create_new, fill, flush,
    /// `sync_all`, rename over the target. The temp file is removed on failure.
    pub(crate) fn write_atomic_with(
        &self,
        rel: &str,
        tmp_name: &str,
        fill: &mut dyn FnMut(&mut File) -> Result<(), FsError>,
    ) -> Result<PathBuf, FsError> {
        let segs = self.segs(rel)?;
        validate_segment(tmp_name)
            .map_err(|d| FsError::new(FsErrorKind::OutsideRoot, &self.path, d))?;
        let (name, parents) = segs
            .split_last()
            .expect("split_rel returns at least one segment");
        let dir = self.ensure_dir(&parents.join("/"))?;
        let target = dir.join(name);
        match stat_entry(&target) {
            Ok(m) => {
                if m.is_dir {
                    return Err(FsError::new(
                        FsErrorKind::NotPlainFile,
                        &target,
                        "target is a directory",
                    ));
                }
                let ok = match m.reparse {
                    ReparseKind::None => true,
                    ReparseKind::CloudFile { .. } => self.policy == ReparsePolicy::AllowCloudFiles,
                    _ => false,
                };
                if !ok {
                    return Err(FsError::new(
                        FsErrorKind::ReparsePoint,
                        &target,
                        "target is a reparse point",
                    ));
                }
            }
            Err(e) if e.kind == FsErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let tmp = dir.join(tmp_name);
        let result = (|| {
            let mut f = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&tmp)
                .map_err(|e| FsError::from_io(&tmp, &e))?;
            fill(&mut f)?;
            f.flush().map_err(|e| FsError::from_io(&tmp, &e))?;
            f.sync_all().map_err(|e| FsError::from_io(&tmp, &e))?;
            drop(f);
            std::fs::rename(&tmp, &target).map_err(|e| FsError::from_io(&target, &e))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result.map(|()| target)
    }

    /// Recursively deletes a directory (or file) under this root. Reparse points and
    /// unavailable entries anywhere inside are refused, never followed or deleted.
    pub(crate) fn remove_tree(&self, rel: &str) -> Result<(), FsError> {
        let top = self.join(rel)?;
        let meta = match self.check_components(rel) {
            Ok(m) => m,
            Err(e) if e.kind == FsErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        if !meta.is_dir {
            return remove_file_retry(&top);
        }
        let mut entries = walk_tree(&list_dir, &top, self.policy, u32::MAX)?;
        entries.reverse();
        for (r, m) in entries {
            let p = r.split('/').fold(top.clone(), |acc, s| acc.join(s));
            if m.is_dir {
                std::fs::remove_dir(&p).map_err(|e| FsError::from_io(&p, &e))?;
            } else {
                remove_file_retry(&p)?;
            }
        }
        std::fs::remove_dir(&top).map_err(|e| FsError::from_io(&top, &e))
    }
}

/// The settings scope of one install: `<variant>/WTF`, and only files below `Account/`
/// (settings.md §1). Paths are validated per segment, so nothing can leave the scope.
#[derive(Debug, Clone)]
pub struct WtfScope {
    root: Root,
}

impl WtfScope {
    /// `None` when `WTF` does not exist and `create` is false. With `create`, a missing `WTF`
    /// directory is made inside the variant folder (never above it).
    pub fn open(variant_dir: &Path, create: bool) -> Result<Option<WtfScope>, FsError> {
        let dir = variant_dir.join("WTF");
        if create {
            match std::fs::create_dir(&dir) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(FsError::from_io(&dir, &e)),
            }
        }
        match Root::open(&dir, ReparsePolicy::RejectAll) {
            Ok(root) => Ok(Some(WtfScope { root })),
            Err(e) if e.kind == FsErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// A settings file path: validated segments, below `Account/`, at least a file name deep.
    pub fn check_rel(rel: &str) -> Result<(), FsError> {
        let bad = |d: String| FsError::new(FsErrorKind::OutsideRoot, Path::new("WTF"), d);
        let segs = split_rel(rel).map_err(|d| bad(format!("{d}: {rel:?}")))?;
        if segs.len() < 2 || segs[0] != "Account" {
            return Err(bad(format!("{rel:?} is not a settings file path")));
        }
        Ok(())
    }

    pub fn root(&self) -> &Root {
        &self.root
    }
}

fn remove_file_retry(p: &Path) -> Result<(), FsError> {
    match std::fs::remove_file(p) {
        Ok(()) => Ok(()),
        Err(e) => {
            #[cfg(windows)]
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                if let Ok(m) = std::fs::metadata(p) {
                    let mut perm = m.permissions();
                    if perm.readonly() {
                        #[allow(clippy::permissions_set_readonly_false)]
                        perm.set_readonly(false);
                        let _ = std::fs::set_permissions(p, perm);
                        if std::fs::remove_file(p).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
            Err(FsError::from_io(p, &e))
        }
    }
}

/// Recursively enumerates `base` through `lister`. Every entry is checked against
/// `policy` from its parent's enumeration data before anything is opened or
/// enumerated; an unavailable or refused directory is not listed. Returns
/// (rel path with '/', entry) sorted by rel path.
pub(crate) fn walk_tree(
    lister: &dyn Fn(&Path) -> Result<Vec<EntryMeta>, FsError>,
    base: &Path,
    policy: ReparsePolicy,
    max_files: u32,
) -> Result<Vec<(String, EntryMeta)>, FsError> {
    let mut out: Vec<(String, EntryMeta)> = Vec::new();
    let mut files = 0u64;
    let mut stack: Vec<(PathBuf, String)> = vec![(base.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        for e in lister(&dir)? {
            let path = dir.join(&e.name);
            check_entry(&e, policy, &path)?;
            let rel = format!("{prefix}{}", e.name);
            if e.is_dir {
                stack.push((path, format!("{rel}/")));
            } else {
                files += 1;
                if files > max_files as u64 {
                    return Err(FsError::new(
                        FsErrorKind::TooLarge,
                        base,
                        format!("more than {max_files} files"),
                    ));
                }
            }
            out.push((rel, e));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Streams `r` into SHA-256; more than `limit` bytes is `TooLarge`.
pub(crate) fn sha256_stream(
    path: &Path,
    r: &mut dyn Read,
    limit: u64,
) -> Result<(u64, [u8; 32]), FsError> {
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = r.read(&mut buf).map_err(|e| FsError::from_io(path, &e))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            return Err(FsError::new(
                FsErrorKind::TooLarge,
                path,
                format!("exceeds limit {limit}"),
            ));
        }
        h.update(&buf[..n]);
    }
    Ok((total, h.finalize().into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TempDir;
    use std::cell::RefCell;

    #[test]
    fn join_rejects_traversal_and_absolute() {
        let t = TempDir::new();
        let root = Root::open(t.path(), ReparsePolicy::RejectAll).unwrap();
        for bad in [
            "", "..", "a/../b", "../x", "/abs", "C:/x", "a//b", "a/", "a\\b", "a:b", ".", "a/./b",
            "bad.", "nul", "NUL.txt", "a*b", "x\u{1}y",
        ] {
            let e = root.join(bad).unwrap_err();
            assert_eq!(e.kind, FsErrorKind::OutsideRoot, "{bad:?}");
        }
        let ok = root.join("Addon/sub dir/file.lua").unwrap();
        assert!(ok.starts_with(t.path()));
        assert!(Root::open(Path::new("relative"), ReparsePolicy::RejectAll).is_err());
        let with_parent = t.path().join("..").join("x");
        assert_eq!(
            Root::open(&with_parent, ReparsePolicy::RejectAll)
                .unwrap_err()
                .kind,
            FsErrorKind::OutsideRoot
        );
    }

    #[test]
    fn classify_tag_cloud_vs_link_vs_other() {
        assert_eq!(
            classify_tag(0x9000_601A),
            ReparseKind::CloudFile { tag: 0x9000_601A }
        );
        assert_eq!(
            classify_tag(0x9000_201A),
            ReparseKind::CloudFile { tag: 0x9000_201A }
        );
        assert_eq!(
            classify_tag(0x9000_001A),
            ReparseKind::CloudFile { tag: 0x9000_001A }
        );
        assert_eq!(
            classify_tag(0xA000_000C),
            ReparseKind::Link { tag: 0xA000_000C }
        );
        assert_eq!(
            classify_tag(0xA000_0003),
            ReparseKind::Link { tag: 0xA000_0003 }
        );
        assert_eq!(
            classify_tag(0x8000_0017),
            ReparseKind::Other { tag: 0x8000_0017 }
        );
        assert_eq!(
            classify_tag(0x9000_0019),
            ReparseKind::Other { tag: 0x9000_0019 }
        );
        assert_eq!(classify_tag(0), ReparseKind::Other { tag: 0 });
    }

    #[test]
    fn unavailable_attribute_combinations() {
        let bits = [
            ATTR_OFFLINE,
            ATTR_RECALL_ON_OPEN,
            ATTR_RECALL_ON_DATA_ACCESS,
        ];
        for dir in [false, true] {
            let base = if dir { ATTR_DIRECTORY } else { 0x20 };
            assert!(!is_unavailable(base));
            assert!(!is_unavailable(base | ATTR_REPARSE_POINT | 0x80000));
            for mask in 1u32..8 {
                let mut a = base;
                for (i, b) in bits.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        a |= b;
                    }
                }
                assert!(is_unavailable(a), "attrs 0x{a:x} dir={dir}");
                let m = EntryMeta {
                    name: "x".into(),
                    is_dir: dir,
                    size: 0,
                    mtime_ns: 0,
                    attrs: a,
                    reparse: ReparseKind::None,
                };
                for p in [ReparsePolicy::RejectAll, ReparsePolicy::AllowCloudFiles] {
                    assert_eq!(
                        check_entry(&m, p, Path::new("x")).unwrap_err().kind,
                        FsErrorKind::CloudOnly
                    );
                }
            }
        }
    }

    #[test]
    fn unavailable_directory_not_enumerated() {
        let listed: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
        let meta = |name: &str, dir: bool, attrs: u32| EntryMeta {
            name: name.into(),
            is_dir: dir,
            size: 0,
            mtime_ns: 0,
            attrs,
            reparse: ReparseKind::None,
        };
        let lister = |p: &Path| -> Result<Vec<EntryMeta>, FsError> {
            listed.borrow_mut().push(p.to_path_buf());
            if p == Path::new("base") {
                Ok(vec![
                    meta("ok", true, ATTR_DIRECTORY),
                    meta("cold", true, ATTR_DIRECTORY | ATTR_OFFLINE),
                ])
            } else {
                Ok(vec![])
            }
        };
        let e = walk_tree(
            &lister,
            Path::new("base"),
            ReparsePolicy::AllowCloudFiles,
            100,
        )
        .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::CloudOnly);
        assert!(listed.borrow().iter().all(|p| !p.ends_with("cold")));
        // A directory that is fine is enumerated.
        let ok = walk_tree(
            &|p: &Path| {
                if p == Path::new("base") {
                    Ok(vec![meta("d", true, ATTR_DIRECTORY)])
                } else {
                    Ok(vec![meta("f", false, 0x20)])
                }
            },
            Path::new("base"),
            ReparsePolicy::RejectAll,
            100,
        )
        .unwrap();
        assert_eq!(
            ok.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
            ["d", "d/f"]
        );
        // File count limit.
        let e = walk_tree(
            &|_: &Path| Ok(vec![meta("a", false, 0), meta("b", false, 0)]),
            Path::new("b"),
            ReparsePolicy::RejectAll,
            1,
        )
        .unwrap_err();
        assert_eq!(e.kind, FsErrorKind::TooLarge);
    }

    #[test]
    fn open_read_enforces_limit_and_refuses_directories() {
        let t = TempDir::new();
        std::fs::create_dir(t.path().join("d")).unwrap();
        std::fs::write(t.path().join("d").join("f.bin"), [7u8; 10]).unwrap();
        let root = Root::open(t.path(), ReparsePolicy::RejectAll).unwrap();
        assert!(root.open_read("d/f.bin", 10).is_ok());
        assert_eq!(
            root.open_read("d/f.bin", 9).unwrap_err().kind,
            FsErrorKind::TooLarge
        );
        assert_eq!(
            root.open_read("d", 100).unwrap_err().kind,
            FsErrorKind::NotPlainFile
        );
        assert_eq!(
            root.open_read("d/missing", 100).unwrap_err().kind,
            FsErrorKind::NotFound
        );
        assert_eq!(
            root.open_read("d/f.bin/x", 100).unwrap_err().kind,
            FsErrorKind::NotFound
        );
    }

    #[cfg(windows)]
    fn mklink_junction(link: &Path, target: &Path) {
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "mklink failed: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    #[cfg(windows)]
    #[test]
    fn junction_inside_root_is_refused() {
        let t = TempDir::new();
        let outside = t.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), b"x").unwrap();
        let rootdir = t.path().join("root");
        std::fs::create_dir_all(&rootdir).unwrap();
        mklink_junction(&rootdir.join("jn"), &outside);
        let root = Root::open(&rootdir, ReparsePolicy::RejectAll).unwrap();
        let m = stat_entry(&rootdir.join("jn")).unwrap();
        assert!(matches!(m.reparse, ReparseKind::Link { .. }), "{m:?}");
        assert_eq!(
            root.check_components("jn").unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            root.open_read("jn/secret.txt", 10).unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert_eq!(
            root.ensure_dir("jn/new").unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
        assert!(!outside.join("new").exists());
        // The junction itself cannot be a root, nor can anything below it.
        assert_eq!(
            Root::open(&rootdir.join("jn"), ReparsePolicy::RejectAll)
                .unwrap_err()
                .kind,
            FsErrorKind::ReparsePoint
        );
        // AllowCloudFiles still refuses links.
        let prov = Root::open(&rootdir, ReparsePolicy::AllowCloudFiles).unwrap();
        assert_eq!(
            prov.check_components("jn").unwrap_err().kind,
            FsErrorKind::ReparsePoint
        );
    }
}
