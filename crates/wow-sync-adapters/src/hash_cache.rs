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
/// More entries than any supported install holds (the folder limit is 100 000 files per
/// add-on); beyond it new files are simply hashed every time.
const MAX_ENTRIES: usize = 400_000;
const MAX_FILE_BYTES: u64 = 96 * 1024 * 1024;

struct Entry {
    size: u64,
    mtime_ns: i128,
    sha256: [u8; 32],
    generation: u64,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, Entry>,
    generation: u64,
    dirty: bool,
}

pub struct HashCache {
    dir: std::path::PathBuf,
    file: String,
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
        let map = LocalState::open_at(state_dir)
            .ok()
            .and_then(|s| s.read(&file, MAX_FILE_BYTES).ok().flatten())
            .and_then(|b| String::from_utf8(b).ok())
            .map(|t| parse(&t))
            .unwrap_or_default();
        Arc::new(HashCache {
            dir: state_dir.to_path_buf(),
            file,
            inner: Mutex::new(Inner {
                map,
                generation: 0,
                dirty: false,
            }),
            full: AtomicBool::new(false),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    /// The next complete scan re-reads every file and replaces the cached hashes.
    pub fn request_full(&self) {
        self.full.store(true, Ordering::Relaxed);
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

    pub(crate) fn lookup(&self, key: &str, size: u64, mtime_ns: i128) -> Option<[u8; 32]> {
        if self.full_requested() {
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
        if g.map.len() >= MAX_ENTRIES && !g.map.contains_key(key) {
            return;
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
    pub(crate) fn finish(&self, complete: bool) {
        let text = {
            let mut g = self.lock();
            if complete {
                let generation = g.generation;
                let before = g.map.len();
                g.map.retain(|_, e| e.generation == generation);
                if g.map.len() != before {
                    g.dirty = true;
                }
                self.full.store(false, Ordering::Relaxed);
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
        if map.len() >= MAX_ENTRIES {
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
