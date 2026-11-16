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
/// Observation of one path component, without following it and without hydrating it.
pub enum ReparseKind { None, CloudFile { tag: u32 }, Link { tag: u32 } /* symlink, junction/mount point */,
                       Other { tag: u32 } }
pub struct EntryMeta { pub is_dir: bool, pub size: u64, pub mtime_ns: i128, pub reparse: ReparseKind,
                       pub cloud: CloudAttrs }
pub struct CloudAttrs { pub recall_on_data_access: bool, pub recall_on_open: bool, pub offline: bool,
                        pub pinned: bool, pub unpinned: bool }
/// Pure: classify a Windows reparse tag. Cloud-files family: (tag & 0xFFFF0FFF) == 0x9000001A
/// (Dropbox test folder: 0x9000601A, 0x9000201A; S-dropbox). SYMLINK 0xA000000C and
/// MOUNT_POINT 0xA0000003 -> Link. Everything else -> Other.
pub fn classify_tag(tag: u32) -> ReparseKind;
/// Windows: open with FILE_READ_ATTRIBUTES + FILE_FLAG_OPEN_REPARSE_POINT |
/// FILE_FLAG_BACKUP_SEMANTICS (no data access, no recall) and
/// GetFileInformationByHandleEx(FileAttributeTagInfo / FileBasicInfo / FileStandardInfo).
/// Non-Windows: symlink_metadata; symlink -> Link.
pub fn entry_meta(path: &Path) -> Result<EntryMeta, FsError>;

pub enum ReparsePolicy {
    /// AddOns, install work area, local state: any reparse point is refused.
    RejectAll,
    /// Provider folder only: CloudFile tags are accepted as ordinary entries;
    /// Link and Other are refused. Data availability is decided separately (CloudAttrs).
    AllowCloudFiles,
}
pub struct Root { /* canonical absolute path + policy */ }
impl Root {
    /// The root and every component from the drive down are checked with `entry_meta`
    /// under `policy` (Link/Other anywhere -> ReparsePoint error).
    pub fn open(path: &Path, policy: ReparsePolicy) -> Result<Root, FsError>;
    pub fn path(&self) -> &Path;
    /// Join a '/'-separated relative path; rejects '..', absolute, drive, ':' and empty segments.
    pub fn join(&self, rel: &str) -> Result<PathBuf, FsError>;
    /// entry_meta of each component under the root, checked against the policy.
    pub fn check_components(&self, rel: &str) -> Result<EntryMeta /* of the last component */, FsError>;
}
/// Data must not be read: RECALL_ON_DATA_ACCESS or OFFLINE on a file, or RECALL_ON_OPEN on a directory.
pub fn is_cloud_only(c: &CloudAttrs, is_dir: bool) -> bool;
```

All file operations below go through a `Root`. Links, junctions and unknown
reparse points are never followed and never written through (A-23). Cloud
placeholders are accepted only under the provider root (M-01); reading a
cloud-only entry's data is refused with `CloudOnly` before any open for
data, so the app never hydrates (Q-6). Directories with `RECALL_ON_OPEN`
are not enumerated (reported `CloudOnly`). Whether `0x9000601A` means
"in sync" is unverified (S-dropbox); the design does not depend on it:
hashes, not attributes, prove content.

## 3. `fs_scan`

```rust
pub struct ScanOptions { pub max_file_bytes: u64 /* 512 MiB */, pub max_files_per_folder: u32 /* 100_000 */,
                         pub max_toc_bytes: u64 /* 256 KiB */, pub recent_ms: u64 /* 2000 (Q-15) */, pub now_unix_ms: i64 }
pub enum ScanStatus { Ok, Unreadable(String), Unsafe(String), Unstable(String) }
pub struct ScannedFile { pub rel_path: String /* '/'-sep */, pub size: u64, pub sha256: [u8; 32] }
pub struct TocFile { pub file_name: String, pub contents: Result<Vec<u8>, String> }
/// Metadata-only snapshot of a folder: every file AND directory, sorted by path.
pub struct Snapshot { pub entries: Vec<(String /* rel */, bool /* is_dir */, u64 /* size */, i128 /* mtime_ns */)> }
impl Snapshot { pub fn fingerprint(&self) -> [u8; 32]; pub fn newest_mtime_ns(&self) -> Option<i128>; }
pub struct ScannedFolder { pub name: String, pub status: ScanStatus, pub files: Vec<ScannedFile>,
                           pub tocs: Vec<TocFile> /* top-level *.toc files */, pub fingerprint: Option<[u8; 32]> }
pub struct AddonsScan { pub folders: Vec<ScannedFolder>, pub listing_stable: bool }
pub fn scan_addons(addons: &Root, opts: &ScanOptions) -> Result<AddonsScan, FsError>;
pub fn scan_folder(addons: &Root, name: &str, opts: &ScanOptions) -> ScannedFolder;
/// Test seam: `hook` runs after the first snapshot and after hashing (before the second snapshot).
pub fn scan_folder_with_hook(addons: &Root, name: &str, opts: &ScanOptions,
                             hook: &mut dyn FnMut(ScanPhase)) -> ScannedFolder;
pub enum ScanPhase { AfterFirstSnapshot, AfterHashing }
/// Metadata only; None = absent.
pub fn snapshot_folder(addons: &Root, name: &str) -> Result<Option<Snapshot>, FsError>;
pub fn hash_folder(addons: &Root, name: &str) -> Result<Option<Vec<ScannedFile>>, FsError>; // None = absent

pub struct VariantFiles { pub folder_name: String, pub flavor_info: FileRead, pub build_info: FileRead,
                          pub addons_dir: DirKind }
pub enum FileRead { Missing, Unreadable(String), Bytes(Vec<u8>) }
pub enum DirKind { Missing, PlainDir, NotPlain }
pub fn read_variant_files(install_root: &Path, variant_folder: &str) -> VariantFiles; // reads ≤ 1 MiB each
pub fn list_variant_folders(install_root: &Path) -> Vec<String>; // dirs matching ^_[a-z0-9_]+_$ with .flavor.info
```

Scan rules (B-02, complete-collection stability):

1. `scan_addons`: list AddOns top level (L1); scan each folder; list again
   (L2). `listing_stable = (L1 == L2)` by names and kinds. Top-level
   non-directories are ignored; a top-level reparse point → `Unsafe`.
2. `scan_folder`: snapshot S1 (all entries, metadata only); hash every file
   (stream, ≤ `max_file_bytes`); snapshot S2. Stable only if `S1 == S2`
   (same membership, sizes and mtimes of every file and directory) **and**
   `newest_mtime_ns < now - recent_ms`. Otherwise `Unstable` with the first
   difference. `fingerprint = S2.fingerprint()`.
3. Nested reparse point → `Unsafe`; read error → `Unreadable`.
4. No exclusive-open probe (S-proc showed it disturbs the client); hidden
   and system files are included.

Residual (stated, not hidden): a writer that changes a file and restores
identical size and mtime between S1 and S2 is not detected; content hashes
at stage/verify time ([apply.md](apply.md) §2.2) still protect the apply.
## 4. `fs_install` (install-side effects)

```rust
pub struct InstallArea { /* addons: Root(RejectAll), work: Root(RejectAll) at <variant>/.wowsync */ }
impl InstallArea {
    /// Creates <variant>/.wowsync/{staging,recovery,journal} if needed; same volume as AddOns.
    pub fn open(variant_dir: &Path) -> Result<InstallArea, FsError>;
    /// Exclusive lock file <variant>/.wowsync/lock (create_new + held handle; stale lock = held by no process).
    /// Excludes other WoW Sync instances only; not other writers (CurseForge).
    pub fn lock(&self) -> Result<InstallLock, FsError>;
    /// Copies src into staging via a temp name, hashes the copy, renames into place;
    /// HashMismatch if the copy differs from `expect`. Refuses CloudOnly sources.
    pub fn stage_file(&self, op: &str, folder: &str, rel: &str, src: &Path, expect: [u8; 32], size: u64) -> Result<(), FsError>;
    /// Content of any slot (live, staging, recovery); None = absent.
    pub fn hash_slot(&self, slot: Slot) -> Result<Option<Vec<ScannedFile>>, FsError>;
    pub fn snapshot_slot(&self, slot: Slot) -> Result<Option<Snapshot>, FsError>;
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
is checked first and the error preserved. Which recovery points may be
removed is decided by core (pinning, [apply.md](apply.md) §2.4).

## 5. `provider`

```rust
pub struct ProviderRoot { /* Root(AllowCloudFiles) at <shared>/WoWSync */ }
pub enum EntryState { Local, CloudOnly, Temp, Conflicted, Unexpected, UnsafeReparse }
pub struct Entry { pub name: String, pub is_dir: bool, pub size: u64, pub mtime_ns: i128, pub state: EntryState }
impl ProviderRoot {
    /// Creates <shared>/WoWSync if missing (shared must exist). Cloud-file reparse tags on
    /// <shared> and its parents are accepted (S-dropbox), links/junctions/unknown tags refused (Q-23).
    pub fn open_or_create(shared: &Path) -> Result<ProviderRoot, FsError>;
    pub fn list(&self, rel_dir: &str) -> Result<Vec<Entry>, FsError>; // NotFound -> empty vec
    /// Refuses CloudOnly (FsErrorKind::CloudOnly; never hydrates, Q-6) and files > limit.
    pub fn read(&self, rel: &str, limit: u64) -> Result<Vec<u8>, FsError>;
    /// <target>.<computer>.<rand>.tmp in same dir, write, flush (sync_all), rename over target.
    pub fn write_atomic(&self, rel: &str, bytes: &[u8], computer: &str) -> Result<(), FsError>;
    /// Observation for publish: Missing | CloudOnly | Local{size, mtime_ns, sha256} (hashes Local objects).
    pub fn check_object(&self, rel: &str, max_bytes: u64) -> Result<ObjectObs, FsError>;
    /// Copies src via temp, verifies sha256 of the temp file, renames over `rel` (replacing an
    /// existing object, which by content addressing must hold the same bytes). Returns the
    /// final (size, mtime_ns) for the verified-objects cache.
    pub fn put_object(&self, rel: &str, src: &Path, expect: [u8; 32], computer: &str) -> Result<(u64, i128), FsError>;
    pub fn put_object_bytes(&self, rel: &str, bytes: &[u8], expect: [u8; 32], computer: &str) -> Result<(u64, i128), FsError>;
    pub fn path_of(&self, rel: &str) -> Result<PathBuf, FsError>; // for staging copies, CloudOnly checked by stage_file
    pub fn remove_file(&self, rel: &str) -> Result<(), FsError>;  // caller restricts to own member dir
    pub fn remove_stale_temps(&self, rel_dir: &str, computer: &str, older_than_ms: u64, now: i64) -> Result<u32, FsError>; // own temps only
}
pub enum ObjectObs { Missing, CloudOnly, Local { size: u64, mtime_ns: i128, sha256: [u8; 32] } }
```

Classification: names ending `.tmp` → `Temp`; names containing
`conflicted copy` (case-insensitive) → `Conflicted`; link/unknown reparse →
`UnsafeReparse` (never read); cloud-only attributes (§2) → `CloudOnly`;
else `Local` (including hydrated cloud placeholders). Readers skip
`Temp`/`Conflicted`/`UnsafeReparse`; the runtime reports them in diagnostics.
No Dropbox API, no hydration, no credentials.

## 6. `winproc` (M-06)

```rust
pub struct ProcEntry { pub pid: u32, pub exe_name: String /* Toolhelp szExeFile, always available */,
                       pub image: Result<PathBuf, String> /* QueryFullProcessImageNameW or error */ }
pub enum ProcessActivity { NotRunning, Running { pids: Vec<u32>, images: Vec<PathBuf> },
                           Unknown(String) }
/// Pure: is `image` inside `variant_dir` (case-insensitive, normalized to '\', trailing '\')?
pub fn image_is_under(variant_dir: &Path, image: &Path) -> bool;
/// Pure: names of possible clients = every "*.exe" directly in the variant folder (listed by
/// the caller) plus any name matching `wow*.exe` (case-insensitive).
pub fn is_possible_client(exe_name: &str, variant_exes: &[String]) -> bool;
/// Pure decision over a snapshot:
/// - any accessible image under variant_dir                     -> Running
/// - any inaccessible entry whose exe_name is a possible client -> Unknown("possible client <name> pid <pid> cannot be inspected")
/// - otherwise                                                  -> NotRunning
/// Inaccessible entries with other names are counted and reported, not blocking.
pub fn classify(entries: &[ProcEntry], variant_dir: &Path, variant_exes: &[String]) -> (ProcessActivity, u32);
pub fn observe_variant(variant_dir: &Path) -> (ProcessActivity, u32 /* inaccessible, non-client */);
```

Windows: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` gives every pid and
`szExeFile` even for elevated/other-session processes; per pid
`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW`
(S-proc). Snapshot failure → `Unknown`. Non-Windows → `Unknown("unsupported
platform")`, which blocks writes in core.

Supported detection boundary (stated in status/diagnostics): a client is
detected if its image path is readable and under the variant folder, or if
it is uninspectable and its executable name is `wow*.exe` or equals an
`.exe` in the variant folder. A renamed client copied outside the variant
folder and run elevated is not detected (limitation; Q-16).
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
| `FolderScan`, `listing_stable` | `fs_scan::scan_addons` (field-for-field, incl. `fingerprint`) |
| `OtherVariants` | runtime lists sibling `_*_` folders + `LocalState` enrollments |
| `ApplyEffects::activity` | `winproc::observe_variant` → `Activity` |
| `lock`, `write_journal`, `read_journals`, `remove_journal` | `InstallArea::{lock, write_journal, read_journals, remove_journal}` |
| `stage_folder` | for each tree file: `ProviderRoot::path_of(blob)` → `InstallArea::stage_file` (CloudOnly → `NotAvailableYet`) |
| `tree_of(slot)` | `InstallArea::hash_slot` + `core::hash::tree_hash` |
| `observe(slot)` | `InstallArea::snapshot_slot` → `SlotObs{fingerprint, recently_modified = newest mtime within window}` |
| `rename` | `InstallArea::rename(Slot, Slot)` |
| `write_recovery_point`, `list_recovery`, `remove_recovery`, `discard_staging` | `InstallArea::{write_recovery_meta, list_recovery, remove_recovery, remove_staging}` |
| `PublishEffects::stat_object`, `hash_object` | `ProviderRoot::check_object` (stat part / hash part) |
| `put_object` | `ProviderRoot::put_object` (blob from AddOns file) or `put_object_bytes` (tree record) |
| `observe_source` | `fs_scan::snapshot_folder` on the live AddOns folder |
| `write_record`, `remove_own_record` | `ProviderRoot::write_atomic`, `remove_file` |
| `MemberObs` | `ProviderRoot::list` + `read` (member record, then only `newest_state_name`) |
| `AdmissionWatermarks`, `VerifiedObjects` | `LocalState::read` / `write_atomic` (`admission.json`, `verified-objects.json`) |
| clock / ids | `sysinfo` |
| local records, audit | `LocalState` |