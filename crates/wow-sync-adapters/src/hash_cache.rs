//! Per-install hash cache (D-31): `(relative path, size, mtime) -> sha256`, kept in the state
//! directory so a routine scan re-reads only new or changed files. Modification time is only a
//! hint for "this file was not touched since I hashed it"; it never orders anything. A lost or
//! damaged cache costs one full read, never correctness.

use crate::localstate::LocalState;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAGIC: &str = "wowsync.hashcache/1";
/// The cache's own memory budget. The background process may use 100 MiB in all, so the map
/// stays far below that: with the per-entry estimate below this holds about 60 000 files, more
/// than a large retail AddOns folder; beyond it new files are simply hashed every time.
const MAX_BYTES: usize = 12 * 1024 * 1024;
/// A full file is at most about as large as the map it fills.
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Estimated heap cost of one entry: the key string, the entry and the table slot.
fn entry_cost(key: &str) -> usize {
    key.len() + 16 + std::mem::size_of::<String>() + std::mem::size_of::<Entry>() + 16
}

struct Entry {
    size: u64,
    mtime_ns: i128,
    sha256: [u8; 32],
    generation: u64,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, Entry>,
    bytes: usize,
    generation: u64,
    dirty: bool,
}

pub struct HashCache {
    dir: std::path::PathBuf,
    file: String,
    marker: String,
    inner: Mutex<Inner>,
    /// Set until a complete scan has re-read everything ("Check all files").
    full: AtomicBool,
    pub hits: AtomicU64,
    pub misses: AtomicU64,
}

impl HashCache {
    /// Loads the cache for `scope` (the install root and variant folder) from `state_dir`.
    /// A missing, damaged or oversized file is an empty cache.
    pub fn open(state_dir: &Path, scope: &str) -> Arc<HashCache> {
        let tag = crate::hex(&Sha256::digest(scope.as_bytes())[..8]);
        let file = format!("hashcache-{tag}.txt");
        let marker = format!("hashcache-{tag}.full");
        let pending = LocalState::open_at(state_dir)
            .ok()
            .and_then(|s| s.read(&marker, 16).ok().flatten())
            .is_some();
        let map = LocalState::open_at(state_dir)
            .ok()
            .and_then(|s| s.read(&file, MAX_FILE_BYTES).ok().flatten())
            .and_then(|b| String::from_utf8(b).ok())
            .map(|t| parse(&t))
            .unwrap_or_default();
        Arc::new(HashCache {
            dir: state_dir.to_path_buf(),
            file,
            marker,
            inner: Mutex::new(Inner {
                bytes: map.keys().map(|k| entry_cost(k)).sum(),
                map,
                generation: 0,
                dirty: false,
            }),
            full: AtomicBool::new(pending),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    /// Deletes the caches of other roots or variants: only the one for `scope` is in use.
    pub fn prune_stale(state_dir: &Path, scope: &str) -> usize {
        let tag = crate::hex(&Sha256::digest(scope.as_bytes())[..8]);
        let keep = [
            format!("hashcache-{tag}.txt"),
            format!("hashcache-{tag}.full"),
        ];
        let Ok(dir) = std::fs::read_dir(state_dir) else {
            return 0;
        };
        let mut removed = 0;
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_cache = name.starts_with("hashcache-")
                && (name.ends_with(".txt") || name.ends_with(".full"));
            if is_cache
                && !keep.contains(&name)
                && e.file_type().is_ok_and(|t| t.is_file())
                && std::fs::remove_file(e.path()).is_ok()
            {
                removed += 1;
            }
        }
        removed
    }

    /// The next complete scan re-reads every file and replaces the cached hashes.
    pub fn request_full(&self) {
        self.full.store(true, Ordering::Relaxed);
        // Kept on disk so a restart, quit or reopen before the full pass ends does not lose it.
        if let Ok(s) = LocalState::open_at(&self.dir) {
            let _ = s.write_atomic(&self.marker, b"1");
        }
    }

    pub fn full_requested(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }

    pub fn len(&self) -> usize {
        self.lock().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn begin(&self) {
        self.lock().generation += 1;
    }

    /// `routine`: a read for a user command, which never acts on the "check all files" request.
    pub(crate) fn lookup(
        &self,
        key: &str,
        size: u64,
        mtime_ns: i128,
        routine: bool,
    ) -> Option<[u8; 32]> {
        if !routine && self.full_requested() {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let mut g = self.lock();
        let generation = g.generation;
        let hit = match g.map.get_mut(key) {
            Some(e) if e.size == size && e.mtime_ns == mtime_ns => {
                e.generation = generation;
                Some(e.sha256)
            }
            _ => None,
        };
        let counter = if hit.is_some() {
            &self.hits
        } else {
            &self.misses
        };
        counter.fetch_add(1, Ordering::Relaxed);
        hit
    }

    pub(crate) fn store(&self, key: &str, size: u64, mtime_ns: i128, sha256: [u8; 32]) {
        if key.contains(['\t', '\n', '\r']) {
            return;
        }
        let mut g = self.lock();
        let generation = g.generation;
        let cost = entry_cost(key);
        let known = g.map.contains_key(key);
        if !known && g.bytes + cost > MAX_BYTES {
            return;
        }
        if !known {
            g.bytes += cost;
        }
        g.map.insert(
            key.to_string(),
            Entry {
                size,
                mtime_ns,
                sha256,
                generation,
            },
        );
        g.dirty = true;
    }

    /// Ends a scan: a complete scan drops entries for files it did not see, then the cache is
    /// saved if it changed. A cancelled scan keeps what it hashed so the next one continues.
    /// A `routine` scan leaves a pending "check all files" request for the next pass.
    pub(crate) fn finish(&self, complete: bool, routine: bool) {
        let text = {
            let mut g = self.lock();
            if complete {
                let generation = g.generation;
                let before = g.map.len();
                g.map.retain(|_, e| e.generation == generation);
                if g.map.len() != before {
                    g.bytes = g.map.keys().map(|k| entry_cost(k)).sum();
                    g.dirty = true;
                }
                if !routine {
                    self.full.store(false, Ordering::Relaxed);
                    if let Ok(s) = LocalState::open_at(&self.dir) {
                        let _ = s.remove(&self.marker);
                    }
                }
            }
            if !g.dirty {
                return;
            }
            g.dirty = false;
            render(&g.map)
        };
        if let Ok(s) = LocalState::open_at(&self.dir) {
            if s.write_atomic(&self.file, text.as_bytes()).is_err() {
                self.lock().dirty = true;
            }
        }
    }
}

fn render(map: &HashMap<String, Entry>) -> String {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let mut out = String::with_capacity(map.len() * 120 + 32);
    out.push_str(MAGIC);
    out.push('\n');
    for k in keys {
        let e = &map[k];
        out.push_str(&format!(
            "{}\t{}\t{}\t{k}\n",
            e.size,
            e.mtime_ns,
            crate::hex(&e.sha256)
        ));
    }
    out
}

fn parse(text: &str) -> HashMap<String, Entry> {
    let mut lines = text.lines();
    if lines.next() != Some(MAGIC) {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    let mut bytes = 0usize;
    for line in lines {
        let mut p = line.splitn(4, '\t');
        let (Some(size), Some(mtime), Some(hex), Some(key)) =
            (p.next(), p.next(), p.next(), p.next())
        else {
            return HashMap::new();
        };
        let (Ok(size), Ok(mtime_ns), Some(sha256)) =
            (size.parse::<u64>(), mtime.parse::<i128>(), unhex(hex))
        else {
            return HashMap::new();
        };
        bytes += entry_cost(key);
        if bytes > MAX_BYTES {
            break;
        }
        map.insert(
            key.to_string(),
            Entry {
                size,
                mtime_ns,
                sha256,
                generation: 0,
            },
        );
    }
    map
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_stays_within_its_memory_budget_and_a_full_file_reloads_within_the_read_cap() {
        let dir = std::env::temp_dir().join(format!("wowsync-hc-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = HashCache::open(&dir, "scope");
        for i in 0..400_000u32 {
            c.store(
                &format!("AddOns/SomeAddon/Libs/Sub/File{i:06}.lua"),
                1,
                i as i128,
                [1; 32],
            );
        }
        let held = c.lock().bytes;
        assert!(held <= MAX_BYTES, "{held}");
        assert!(c.len() > 40_000 && c.len() < 400_000, "{}", c.len());
        c.begin();
        c.finish(false, false);
        let file = std::fs::metadata(dir.join(&c.file)).unwrap().len();
        assert!(file <= MAX_FILE_BYTES, "{file}");
        let reopened = HashCache::open(&dir, "scope");
        assert_eq!(reopened.len(), c.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_check_all_files_request_survives_a_reopen_until_a_full_pass_completes() {
        let dir = std::env::temp_dir().join(format!("wowsync-hc-full-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = HashCache::open(&dir, "scope");
        assert!(!c.full_requested());
        c.request_full();
        let again = HashCache::open(&dir, "scope");
        assert!(again.full_requested());
        again.begin();
        again.finish(true, true);
        assert!(
            HashCache::open(&dir, "scope").full_requested(),
            "a routine pass keeps it"
        );
        again.begin();
        again.finish(false, false);
        assert!(
            HashCache::open(&dir, "scope").full_requested(),
            "a cancelled pass keeps it"
        );
        again.begin();
        again.finish(true, false);
        assert!(!again.full_requested());
        assert!(!HashCache::open(&dir, "scope").full_requested());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caches_of_roots_no_longer_in_use_are_removed_and_the_active_one_is_kept() {
        let dir = std::env::temp_dir().join(format!("wowsync-hc-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for scope in ["old-root|_retail_", "active|_retail_"] {
            let c = HashCache::open(&dir, scope);
            c.store("a", 1, 1, [2; 32]);
            c.finish(false, false);
        }
        std::fs::write(dir.join("enrollment.json"), b"{}").unwrap();
        assert_eq!(HashCache::prune_stale(&dir, "active|_retail_"), 1);
        assert_eq!(HashCache::open(&dir, "active|_retail_").len(), 1);
        assert_eq!(HashCache::open(&dir, "old-root|_retail_").len(), 0);
        assert!(dir.join("enrollment.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
