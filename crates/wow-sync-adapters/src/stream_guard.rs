//! Guard against whole-file reads on the settings path (OD-30). A counting allocator measures,
//! per thread, the most memory held at once while a large saved-variables file is hashed,
//! stored, uploaded and staged; a regression to reading a file into memory shows up as a peak
//! near the file size.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write;

use crate::fs_install::{InstallArea, SettingsSlot};
use crate::provider::ProviderRoot;
use crate::settings_store::SettingsStore;
use crate::testkit::{SyntheticInstall, TempDir};
use crate::wtf::hash_wtf_file;

pub struct Counting;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static BIGGEST: Cell<usize> = const { Cell::new(0) };
}

fn grow(n: usize) {
    if ON.with(Cell::get) {
        let live = LIVE.with(|l| {
            l.set(l.get() + n as isize);
            l.get()
        });
        PEAK.with(|p| p.set(p.get().max(live)));
        BIGGEST.with(|b| b.set(b.get().max(n)));
    }
}

fn shrink(n: usize) {
    if ON.with(Cell::get) {
        LIVE.with(|l| l.set(l.get() - n as isize));
    }
}

// SAFETY: every call is forwarded unchanged to the system allocator; the counters are
// thread-local `Cell`s that never allocate.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        grow(l.size());
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        shrink(l.size());
        System.dealloc(p, l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        grow(l.size());
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        shrink(l.size());
        grow(new);
        System.realloc(p, l, new)
    }
}

/// Runs `f` and returns (peak bytes held at once, largest single allocation) on this thread.
fn measured<T>(f: impl FnOnce() -> T) -> (T, isize, usize) {
    LIVE.with(|l| l.set(0));
    PEAK.with(|p| p.set(0));
    BIGGEST.with(|b| b.set(0));
    ON.with(|o| o.set(true));
    let out = f();
    ON.with(|o| o.set(false));
    (out, PEAK.with(Cell::get), BIGGEST.with(Cell::get))
}

const FILE_BYTES: u64 = 24 * 1024 * 1024;
/// Far below the file size, far above the copy buffers and bookkeeping.
const PEAK_LIMIT: isize = 3 * 1024 * 1024;
const SINGLE_LIMIT: usize = 1024 * 1024;

fn write_big(path: &std::path::Path) {
    let mut f = std::fs::File::create(path).unwrap();
    let chunk = vec![b'x'; 1024 * 1024];
    for _ in 0..FILE_BYTES / chunk.len() as u64 {
        f.write_all(&chunk).unwrap();
    }
}

#[test]
fn a_large_settings_file_is_hashed_stored_uploaded_and_staged_in_fixed_buffers() {
    let t = TempDir::new();
    let inst = SyntheticInstall::new_forever(t.path(), "wow");
    let dir = inst.variant_dir.join("WTF/Account/ACC/SavedVariables");
    std::fs::create_dir_all(&dir).unwrap();
    write_big(&dir.join("Big.lua"));
    let rel = "Account/ACC/SavedVariables/Big.lua";
    let cap = 32 * 1024 * 1024;

    let (fact, peak, single) = measured(|| hash_wtf_file(&inst.variant_dir, rel, cap));
    let (size, hash) = fact.unwrap().unwrap();
    assert_eq!(size, FILE_BYTES);
    assert!(
        peak < PEAK_LIMIT && single < SINGLE_LIMIT,
        "hash: {peak} {single}"
    );

    let store = SettingsStore::open(&inst.variant_dir).unwrap();
    let scope = crate::fs_safe::WtfScope::open(&inst.variant_dir, false)
        .unwrap()
        .unwrap();
    let ((), peak, single) = measured(|| {
        let mut f = scope.root().open_read(rel, cap).unwrap();
        store.put_blob_stream(&mut f, hash, size).unwrap();
        let mut again = scope.root().open_read(rel, cap).unwrap();
        store.put_blob_stream(&mut again, hash, size).unwrap();
    });
    assert!(
        peak < PEAK_LIMIT && single < SINGLE_LIMIT,
        "store: {peak} {single}"
    );

    let shared = TempDir::new();
    let provider = ProviderRoot::open_or_create(shared.path()).unwrap();
    let ((), peak, single) = measured(|| {
        let (mut blob, n) = store.open_blob(&hash, cap).unwrap().unwrap();
        assert_eq!(n, size);
        provider
            .put_object_reader("objects/aa/big", &mut blob, hash, "COMPUTERAAAA")
            .unwrap();
    });
    assert!(
        peak < PEAK_LIMIT && single < SINGLE_LIMIT,
        "upload: {peak} {single}"
    );

    let area = InstallArea::open(&inst.variant_dir).unwrap();
    let ((), peak, single) = measured(|| {
        let mut src = provider.open_read("objects/aa/big", cap).unwrap();
        area.stage_settings("op1", 0, &mut src, hash, size).unwrap();
        let staged = area
            .settings_hash(SettingsSlot::Staging { op: "op1", idx: 0 })
            .unwrap();
        assert_eq!(staged, Some(hash));
    });
    assert!(
        peak < PEAK_LIMIT && single < SINGLE_LIMIT,
        "stage: {peak} {single}"
    );
}

#[test]
fn the_counter_sees_a_whole_file_read() {
    let t = TempDir::new();
    let p = t.path().join("f.bin");
    write_big(&p);
    let (_, peak, single) = measured(|| std::fs::read(&p).unwrap());
    assert!(peak >= FILE_BYTES as isize && single >= FILE_BYTES as usize);
}
