# `wow-sync-adapters` API sketch

Part of the [slice design](README.md). Owner: R2-adapters. The crate does not
depend on `wow-sync-core`; it reports observations and performs requested
effects inside authorized roots. It never decides ordering, completeness,
removal or safety (ARCHITECTURE §2.3). Windows-specific code is behind
`#[cfg(windows)]`; the crate must still compile on macOS (CI builds four
targets).

## 1. Errors

```rust
pub enum FsErrorKind { NotFound, AlreadyExists, OutsideRoot, ReparsePoint, NotPlainFile,
                       TooLarge, HashMismatch, InUse, Io, CloudOnly, Unsupported }
pub struct FsError { pub kind: FsErrorKind, pub path: PathBuf, pub detail: String }
```

## 2. `fs_safe`

```rust
pub struct Root { /* canonical absolute path */ }
impl Root {
    /// Directory must exist and must not be (or traverse) a reparse point.
    pub fn open(path: &Path) -> Result<Root, FsError>;
    pub fn path(&self) -> &Path;
    /// Join a '/'-separated relative path; rejects '..', absolute, drive, ':' and empty segments.
    pub fn join(&self, rel: &str) -> Result<PathBuf, FsError>;
    /// symlink_metadata of each component under the root; any reparse point -> ReparsePoint.
    pub fn check_no_reparse(&self, rel: &str) -> Result<(), FsError>;
}
pub fn is_reparse(meta: &std::fs::Metadata) -> bool; // windows: FILE_ATTRIBUTE_REPARSE_POINT; else is_symlink
pub fn is_cloud_only(meta: &std::fs::Metadata) -> bool; // windows: RECALL_ON_DATA_ACCESS | RECALL_ON_OPEN | OFFLINE
```

All file operations below go through a `Root`. Junctions, symlinks and other
reparse points are never followed and never written through (A-23).

## 3. `fs_scan`

```rust
pub struct ScanOptions { pub max_file_bytes: u64 /* 512 MiB */, pub max_files_per_folder: u32 /* 100_000 */,
                         pub max_toc_bytes: u64 /* 256 KiB */, pub recent_ms: u64 /* 2000 (Q-15) */, pub now_unix_ms: i64 }
pub enum ScanStatus { Ok, Unreadable(String), Unsafe(String), Unstable(String) }
pub struct ScannedFile { pub rel_path: String /* '/'-sep */, pub size: u64, pub sha256: [u8; 32] }
pub struct TocFile { pub file_name: String, pub contents: Result<Vec<u8>, String> }
pub struct ScannedFolder { pub name: String, pub status: ScanStatus, pub files: Vec<ScannedFile>,
                           pub tocs: Vec<TocFile> /* top-level *.toc files */ }
pub fn scan_addons(addons: &Root, opts: &ScanOptions) -> Result<Vec<ScannedFolder>, FsError>;
pub fn scan_folder(addons: &Root, name: &str, opts: &ScanOptions) -> ScannedFolder;
pub fn hash_folder(addons: &Root, name: &str) -> Result<Option<Vec<ScannedFile>>, FsError>; // None = absent

pub struct VariantFiles { pub folder_name: String, pub flavor_info: FileRead, pub build_info: FileRead,
                          pub addons_dir: DirKind }
pub enum FileRead { Missing, Unreadable(String), Bytes(Vec<u8>) }
pub enum DirKind { Missing, PlainDir, NotPlain }
pub fn read_variant_files(install_root: &Path, variant_folder: &str) -> VariantFiles; // reads ≤ 1 MiB each
pub fn list_variant_folders(install_root: &Path) -> Vec<String>; // dirs matching ^_[a-z0-9_]+_$ with .flavor.info
```

Scan rules: top-level non-directories in AddOns are ignored; a top-level
reparse point → `Unsafe`; a nested reparse point → folder `Unsafe`; any
file/dir read error → `Unreadable`; stability per file: metadata before,
stream hash, metadata after; `(len, mtime)` must be identical and `mtime`
older than `now - recent_ms`, otherwise `Unstable`. No exclusive-open probe
(S-proc showed it disturbs the client). Hidden/system files are included.

## 4. `fs_install` (install-side effects)

```rust
pub struct InstallArea { /* addons: Root, work: Root (<variant>/.wowsync) */ }
impl InstallArea {
    /// Creates <variant>/.wowsync/{staging,recovery,journal} if needed; same volume as AddOns.
    pub fn open(variant_dir: &Path) -> Result<InstallArea, FsError>;
    /// Exclusive lock file <variant>/.wowsync/lock (create_new + held handle; stale lock = held by no process).
    pub fn lock(&self) -> Result<InstallLock, FsError>;
    pub fn stage_file(&self, op: &str, folder: &str, rel: &str, src: &Path, expect: [u8; 32], size: u64) -> Result<(), FsError>;
    pub fn hash_staged(&self, op: &str, folder: &str) -> Result<Vec<ScannedFile>, FsError>;
    /// Single same-volume directory rename; fails if destination exists.
    pub fn rename(&self, from: Slot, to: Slot) -> Result<(), FsError>;
    pub fn exists(&self, slot: Slot) -> Result<bool, FsError>;
    pub fn write_journal(&self, op: &str, bytes: &[u8]) -> Result<(), FsError>;  // temp + flush + rename
    pub fn read_journals(&self) -> Result<Vec<(String, Vec<u8>)>, FsError>;
    pub fn remove_journal(&self, op: &str) -> Result<(), FsError>;
    pub fn write_recovery_meta(&self, pkg: &str, op: &str, bytes: &[u8]) -> Result<(), FsError>;
    pub fn list_recovery(&self, pkg: &str) -> Result<Vec<(String, Vec<u8>)>, FsError>;
    pub fn remove_recovery(&self, pkg: &str, op: &str) -> Result<(), FsError>; // only under work/recovery
    pub fn remove_staging(&self, op: &str) -> Result<(), FsError>;              // only under work/staging
}
pub enum Slot<'a> { Live { folder: &'a str }, Staging { op: &'a str, folder: &'a str },
                    Recovery { pkg: &'a str, op: &'a str, folder: &'a str } }
```

Layout under `<variant>\.wowsync\`: `staging\<op>\<folder>\…`,
`recovery\<pkg>\<op>\<folder>\…` plus `recovery\<pkg>\<op>\recovery.json`,
`journal\<op>.json`, `lock`. Deletion is permitted only inside `staging` and
`recovery`, never in `AddOns` (removal from `AddOns` is always a rename into
recovery). Renames use `std::fs::rename` (MoveFileExW without
`MOVEFILE_REPLACE_EXISTING` semantics for directories); destination existence
is checked first and the error preserved.

## 5. `provider`

```rust
pub struct ProviderRoot { /* Root at <shared>/WoWSync */ }
pub enum EntryState { Local, CloudOnly, Temp, Conflicted, Unexpected }
pub struct Entry { pub name: String, pub is_dir: bool, pub size: u64, pub state: EntryState }
impl ProviderRoot {
    /// Creates <shared>/WoWSync if missing (shared must exist).
    pub fn open_or_create(shared: &Path) -> Result<ProviderRoot, FsError>;
    pub fn list(&self, rel_dir: &str) -> Result<Vec<Entry>, FsError>; // NotFound -> empty vec
    /// Refuses CloudOnly (FsErrorKind::CloudOnly; never hydrates, Q-6) and files > limit.
    pub fn read(&self, rel: &str, limit: u64) -> Result<Vec<u8>, FsError>;
    /// <target>.<computer>.<rand>.tmp in same dir, write, flush (sync_all), rename over target.
    pub fn write_atomic(&self, rel: &str, bytes: &[u8], computer: &str) -> Result<(), FsError>;
    /// Write-if-absent: copies src via temp, verifies sha256 of the temp file, renames; if target
    /// exists and is Local, verifies its hash (mismatch -> HashMismatch, left in place).
    pub fn put_blob(&self, rel: &str, src: &Path, expect: [u8; 32], computer: &str) -> Result<(), FsError>;
    pub fn exists(&self, rel: &str) -> Result<Option<EntryState>, FsError>;
    pub fn path_of(&self, rel: &str) -> Result<PathBuf, FsError>; // for staging copies, CloudOnly checked by caller
    pub fn remove_file(&self, rel: &str) -> Result<(), FsError>;  // caller restricts to own member dir
    pub fn remove_stale_temps(&self, rel_dir: &str, computer: &str, older_than_ms: u64, now: i64) -> Result<u32, FsError>; // own temps only
}
```

Classification: names ending `.tmp` → `Temp`; names containing
`conflicted copy` (case-insensitive) → `Conflicted`; cloud attributes →
`CloudOnly`; else `Local`. Readers skip `Temp`/`Conflicted`; the runtime
reports them in diagnostics. No Dropbox API, no hydration, no credentials.

## 6. `winproc`

```rust
pub enum ProcessActivity { NotRunning, Running { pids: Vec<u32>, images: Vec<PathBuf> },
                           Unknown(String) }
/// Running if any process image path is inside `variant_dir` (case-insensitive prefix
/// match after normalizing to `\`, with trailing `\`). Launchers outside the variant dir
/// do not count. Inaccessible processes are counted in `inaccessible` but do not block (limitation).
pub fn observe_variant(variant_dir: &Path) -> (ProcessActivity, u32 /* inaccessible */);
```

Windows: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`, per pid
`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW`
(S-proc). Snapshot failure → `Unknown`. Non-Windows → `Unknown("unsupported
platform")`, which blocks writes in core.

## 7. `sysinfo`, `localstate`

```rust
pub fn now_unix_ms() -> i64;
pub fn random_id_hex() -> String;   // 16 bytes from getrandom, 32 lowercase hex

pub struct LocalState { /* Root at %LOCALAPPDATA%\WoWSync, or an explicit dir (tests) */ }
impl LocalState {
    pub fn open_default() -> Result<LocalState, FsError>;
    pub fn open_at(dir: &Path) -> Result<LocalState, FsError>;
    pub fn read(&self, name: &str, limit: u64) -> Result<Option<Vec<u8>>, FsError>;
    pub fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), FsError>;
    /// Appends one line to audit.jsonl; rotates at 1 MiB to audit.1.jsonl (keeps 2 files).
    pub fn append_audit(&self, line: &str) -> Result<(), FsError>;
    /// Single-instance guard: exclusive lock on <dir>/instance.lock.
    pub fn acquire_instance(&self) -> Result<InstanceGuard, FsError>;
}
```

## 8. `testkit` (feature `testkit`)

```rust
pub struct TempDir { /* under std::env::temp_dir()/wowsync-test-<random>; removed on Drop */ }
pub struct SyntheticInstall { pub root: PathBuf, pub variant_dir: PathBuf, pub addons: PathBuf }
impl SyntheticInstall {
    /// <tmp>/<name>/ with .build.info (Product wowf, Active 1), _forever_/.flavor.info (wowf),
    /// _forever_/Interface/AddOns. Never touches real WoW folders.
    pub fn new_forever(parent: &Path, name: &str) -> SyntheticInstall;
    pub fn with_product(parent: &Path, name: &str, folder: &str, product: &str) -> SyntheticInstall;
    pub fn add_addon(&self, folder: &str, toc_name: &str, toc_body: &str, files: &[(&str, &[u8])]);
    pub fn set_old_mtimes(&self);   // move all mtimes beyond the stability window
}
/// Deterministic synthetic dataset for provider tests; total bytes ≤ cap (Q-8).
pub fn synthetic_dataset(dir: &Path, seed: u64, total_bytes: u64) -> Vec<PathBuf>;
/// Windows: copies %SystemRoot%\System32\ping.exe to <variant_dir>\Wow.exe and starts it
/// (`-n 30 127.0.0.1`); Drop kills it. Stand-in for "WoW running" without real binaries.
#[cfg(windows)] pub struct FakeClient { /* Child */ }
#[cfg(windows)] impl FakeClient { pub fn start(variant_dir: &Path) -> FakeClient; }
```

## 9. Mapping to core port traits (R2-slice)

| Core port / input | Adapter call |
|---|---|
| `VariantObservation` | `fs_scan::read_variant_files` |
| `FolderScan` | `fs_scan::scan_addons` (field-for-field) |
| `ApplyEffects::activity` | `winproc::observe_variant` → `Activity` |
| `lock`, `write_journal`, `read_journals` | `InstallArea::lock`, `write_journal`, `read_journals` |
| `stage_folder` | for each tree file: `ProviderRoot::path_of(blob)` → `InstallArea::stage_file` (CloudOnly → `NotAvailableYet`) |
| `verify_staged`, `hash_live_folder` | `InstallArea::hash_staged`, `fs_scan::hash_folder` + `core::hash::tree_hash` |
| `move_*` | `InstallArea::rename(Slot, Slot)` |
| `write_recovery_point`, `list_recovery`, `prune_recovery` | `InstallArea::{write_recovery_meta, list_recovery, remove_recovery}` |
| `PublishEffects::blob_exists`, `upload_blob` | `ProviderRoot::exists`, `put_blob` |
| `write_record`, `remove_own_record` | `ProviderRoot::write_atomic`, `remove_file` |
| clock / ids | `sysinfo` |
| local records, audit | `LocalState` |
