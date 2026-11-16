# `wow-sync-core` API sketch

Part of the [slice design](README.md). Owner: R2-core. Sketches fix names,
ownership of decisions and signatures; implementers may add private helpers,
derive traits and split files, but must not move a decision out of core or
change the public names below without a design retry. All public types derive
`Debug, Clone, PartialEq, Eq`; records also `serde::{Serialize, Deserialize}`.

## 1. `ids`, `hash`, `paths`

```rust
pub struct ComputerId(String);   // 32 lowercase hex; ::parse(&str) -> Result<Self, IdError>
pub struct EnrollmentId(String); // same form
pub struct GroupId(String);
pub struct OpId(String);
pub struct Sha256(pub [u8; 32]); // Display/parse as 64 lowercase hex

pub struct Hasher(sha2::Sha256); // new(), update(&[u8]), finish() -> Sha256
pub fn sha256(bytes: &[u8]) -> Sha256;
pub fn tree_hash(files: &[FileEntry]) -> Sha256;           // publication.md §3
pub fn package_hash(folders: &[(FolderKey, Sha256)]) -> Sha256;

pub struct FolderKey(String);    // lowercased validated folder name
pub struct RelPath(String);      // validated, '/'-separated, relative to the folder
pub fn validate_folder_name(name: &str) -> Result<FolderKey, PathError>;
pub fn validate_rel_path(path: &str) -> Result<RelPath, PathError>;
pub enum PathError { Empty, NonAscii, ForbiddenChar(char), Reserved, TrailingDotOrSpace,
                     DotSegment, TooLong, Absolute }
```

## 2. `bpsv`, `variant`

```rust
pub struct Bpsv { pub columns: Vec<String>, pub rows: Vec<Vec<String>> }
pub fn parse_bpsv(bytes: &[u8]) -> Result<Bpsv, BpsvError>;

pub enum FileObs { Missing, Unreadable(String), Bytes(Vec<u8>) }
pub struct VariantObservation {
    pub folder_name: String,
    pub flavor_info: FileObs,
    pub build_info: FileObs,
    pub addons_dir: DirObs,          // Missing | PlainDir | NotPlain
}
pub struct VariantIdentity { pub product: ProductCode, pub folder: String,
                             pub build_version: Option<String>, pub profile: Option<&'static Profile> }
pub struct Profile { pub product: &'static str, pub folder: &'static str,
                     pub label: &'static str, pub toc_suffixes: &'static [&'static str] }
pub const KNOWN_PROFILES: &[Profile];   // only wowf in the slice (domain.md §2)
pub enum VariantVerdict { Recognized(VariantIdentity), Unfamiliar(VariantIdentity), Refused(RefuseReason) }
pub fn validate_variant(obs: &VariantObservation) -> VariantVerdict;
/// Ok only for Recognized, or Unfamiliar with user_confirmed == true.
pub fn confirm_variant(v: &VariantVerdict, user_confirmed: bool) -> Result<ConfirmedVariant, RefuseReason>;
```

`ConfirmedVariant` is the only value accepted by enrollment and planning
functions; it can only be constructed by `confirm_variant` (private field).

## 3. `toc`, `version`

```rust
pub struct TocInfo { pub interface: Vec<u32>, pub version: Option<String>, pub title: Option<String>,
                     pub hard_deps: Vec<String>, pub ids: Vec<ExternalId> }
pub struct ExternalId { pub kind: IdKind /* Curse | Wago | WoWI */, pub value: String }
pub fn parse_toc(bytes: &[u8]) -> Result<TocInfo, TocError>;  // TooLarge | NotUtf8
pub struct TocCandidate { pub file_name: String, pub contents: FileObs }
pub enum TocSelection {
    Selected { file_name: String, info: TocInfo },
    None { reason: String },
    Ambiguous { files: Vec<String> },
    Error { file_name: String, reason: String },
}
pub fn select_toc(folder: &str, candidates: &[TocCandidate], profile: Option<&Profile>) -> TocSelection;

pub enum Version { DottedNumeric{..}, Integer(u64), Revision(u64), Date(u32,u32,u32),
                   Tagged{ tag: String, v: Box<Version> }, Missing, Token(String), GitHash(String), Other(String) }
pub enum VersionOrder { Less, Equal, Greater, Ambiguous(String) }
pub fn classify(raw: Option<&str>) -> Version;
pub fn compare(a: &Version, b: &Version) -> VersionOrder;
```

## 4. `inventory`, `package`

```rust
/// What adapters report for one AddOns top-level entry (mirrors adapters::fs_scan::ScannedFolder).
pub struct FolderScan { pub name: String, pub status: ScanStatus, pub files: Vec<ScannedFile>,
                        pub tocs: Vec<TocCandidate> }
pub enum ScanStatus { Ok, Unreadable(String), Unsafe(String) /* reparse/link */, Unstable(String) /* changed while read */ }
pub struct ScannedFile { pub rel_path: String, pub size: u64, pub sha256: Sha256 }

pub enum FolderStatus { Ok, Unreadable, Unsafe, Unsupported, Unstable }
pub struct FileEntry { pub path: RelPath, pub size: u64, pub sha256: Sha256 }
pub struct FolderState { pub name: String, pub key: FolderKey, pub status: FolderStatus,
                         pub tree: Option<Sha256>, pub files: u32, pub bytes: u64,
                         pub toc: TocSummary /* from TocSelection, without full info */ ,
                         pub file_entries: Vec<FileEntry> }
pub struct Inventory { pub folders: Vec<FolderState> } // sorted by key
pub fn build_inventory(scans: Vec<FolderScan>, v: &ConfirmedVariant) -> Inventory;
pub fn inventory_is_stable(inv: &Inventory) -> bool; // false if any Unstable -> no publish/apply this cycle

pub struct Package { pub key: FolderKey, pub display: FolderKey, pub members: Vec<FolderKey>,
                     pub edges: Vec<(FolderKey, FolderKey, EdgeKind)> }
pub fn group_packages(observations: &[&Inventory]) -> Vec<Package>;   // domain.md §5
pub struct PackageVersion { pub package: FolderKey, pub hash: Sha256, pub folders: Vec<FolderState> }
pub fn package_version(pkg: &Package, inv: &Inventory) -> Option<PackageVersion>; // None if no member present
pub enum PackageOrder { Same, Greater, Less, Ambiguous { reason: AmbiguityReason, evidence: Vec<FolderEvidence> } }
pub fn compare_package(a: &PackageVersion, b: &PackageVersion) -> PackageOrder;
```

## 5. `records`, `layout`

Rust structs mirroring every schema in [publication.md](publication.md) §2
(`RootFormat`, `GroupRecord`, `MemberRecord`, `CodeState`, `FolderTreeRecord`,
`ApplyJournal`, `RecoveryPoint`, `AuditEntry`, `LocalComputer`, `LocalEnrollment`).

```rust
pub fn encode<T: Record>(r: &T) -> Vec<u8>;                     // pretty JSON, LF, trailing newline
pub fn decode<T: Record>(bytes: &[u8], limit: usize) -> Result<T, RecordError>;
// RecordError: TooLarge | Json(String) | SchemaMissing | UnknownSchema(String) | UnsupportedMajor(u32) | Invalid(String)
pub trait Record: Serialize + DeserializeOwned { const SCHEMA: &'static str; const MAJOR: u32; const LIMIT: usize; fn validate(&self) -> Result<(), String>; }
pub fn validate_state_for_group(s: &CodeState, g: &GroupRecord, member: &MemberRecord) -> Result<(), RecordError>;

// layout: all return '/'-separated paths relative to <shared>/WoWSync or <variant>/.wowsync
pub fn group_record(g: &GroupId) -> String;            // "groups/<g>/group.json"
pub fn member_record(g: &GroupId, c: &ComputerId) -> String;
pub fn state_record(g: &GroupId, c: &ComputerId, e: &EnrollmentId, seq: u64) -> String;
pub fn parse_state_name(file_name: &str) -> Option<(EnrollmentId, u64)>;
pub fn tree_record(g: &GroupId, h: &Sha256) -> String;
pub fn blob(g: &GroupId, h: &Sha256) -> String;
pub fn staging_dir(op: &OpId) -> String;  pub fn recovery_dir(pkg: &FolderKey, op: &OpId) -> String;
pub fn journal(op: &OpId) -> String;
```

## 6. `target`, `plan`

```rust
pub struct MemberView { pub computer: ComputerId, pub enrollment: EnrollmentId, pub display_name: String,
                        pub latest: MemberLatest }
pub enum MemberLatest { Valid { seq: u64, state: CodeState }, Unavailable(String), Invalid(String), None }
pub struct TargetInput<'a> { pub group: &'a GroupRecord, pub local: &'a Inventory,
                             pub local_id: &'a ComputerId, pub members: &'a [MemberView] }
pub enum PackageTarget { Chosen { version: PackageVersion, sources: Vec<ComputerId> },
                         NeedsDecision { candidates: Vec<(PackageVersion, Vec<ComputerId>)>, why: Explanation } }
pub struct GroupTarget { pub packages: BTreeMap<FolderKey, PackageTarget> }
pub fn compute_target(input: &TargetInput) -> GroupTarget;

pub struct PublishPlan { pub blobs: Vec<BlobUpload>, pub trees: Vec<(Sha256, FolderTreeRecord)>,
                         pub state: CodeState, pub state_path: String, pub prune_states: Vec<String> }
pub struct BlobUpload { pub sha256: Sha256, pub local: LocalFileRef /* folder + RelPath */, pub size: u64 }
pub fn plan_publish(inv: &Inventory, ctx: &PublishContext, existing_blobs: &BTreeSet<Sha256>,
                    existing_trees: &BTreeSet<Sha256>, own_state_files: &[String]) -> Result<PublishPlan, PlanBlock>;

pub struct ApplyPlan { pub op: OpId, pub packages: Vec<PackageApply> }   // sorted by package key
pub struct PackageApply { pub package: FolderKey, pub target_hash: Sha256,
                          pub folders: Vec<FolderApply>, pub explanation: Explanation }
pub struct FolderApply { pub name: String, pub key: FolderKey, pub action: FolderAction,
                         pub tree: FolderTreeRecord }
pub enum FolderAction { Install /* absent locally */, Replace /* present, different tree */ }
pub fn plan_apply(target: &GroupTarget, local: &Inventory, op: OpId, gate: &ApplyGate) -> Result<ApplyPlan, PlanBlock>;
pub struct ApplyGate { pub activity: Activity, pub inventory_stable: bool, pub variant: ConfirmedVariant }
pub enum Activity { NotRunning, Running { pids: Vec<u32> }, Unknown(String) }
pub enum PlanBlock { GameRunning, ActivityUnknown(String), Unstable, NothingToDo, Invalid(String) }
```

`plan_apply` skips packages whose local version is `Same` or `Greater` than
the target (no downgrade) and folders whose tree already equals the target;
local folders of the package that are absent from the target are left alone
(Q-2). `plan_publish` refuses when the inventory is unstable.

## 7. `exec` (port traits and executor)

```rust
pub trait PublishEffects {
    fn blob_exists(&mut self, h: &Sha256) -> Result<bool, EffectError>;
    fn upload_blob(&mut self, src: &LocalFileRef, h: &Sha256, size: u64) -> Result<(), EffectError>; // verifies hash
    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError>;               // temp + rename
    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError>;                         // own member dir only
}
pub trait ApplyEffects {
    fn activity(&mut self) -> Activity;
    fn lock(&mut self) -> Result<(), EffectError>;
    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError>;
    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError>; // copy blobs, verify each hash
    fn verify_staged(&mut self, op: &OpId, f: &FolderApply) -> Result<bool, EffectError>;
    fn move_live_to_recovery(&mut self, op: &OpId, pkg: &FolderKey, folder: &str) -> Result<(), EffectError>;
    fn move_staged_to_live(&mut self, op: &OpId, folder: &str) -> Result<(), EffectError>;
    fn move_recovery_to_live(&mut self, op: &OpId, pkg: &FolderKey, folder: &str) -> Result<(), EffectError>;
    fn move_live_to_staging(&mut self, op: &OpId, folder: &str) -> Result<(), EffectError>;
    fn hash_live_folder(&mut self, folder: &str) -> Result<Option<Sha256>, EffectError>; // None = absent
    fn exists(&mut self, area: Area, op: &OpId, folder: &str) -> Result<bool, EffectError>;
    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError>;
    fn prune_recovery(&mut self, pkg: &FolderKey, keep: &[OpId]) -> Result<(), EffectError>;
    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<RecoveryPoint>, EffectError>;
    fn read_journals(&mut self) -> Result<Vec<ApplyJournal>, EffectError>;
    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError>;
}
pub enum Area { Live, Staging, Recovery }
pub struct EffectError { pub kind: EffectErrorKind, pub detail: String }
pub fn run_publish(plan: &PublishPlan, fx: &mut impl PublishEffects) -> PublishOutcome;
pub fn run_apply(plan: &ApplyPlan, fx: &mut impl ApplyEffects) -> ApplyOutcome;   // apply.md §2
pub fn recover_interrupted(fx: &mut impl ApplyEffects) -> RecoveryOutcome;         // apply.md §4
pub struct ApplyOutcome { pub packages: Vec<(FolderKey, PackageResult)>, pub stopped: Option<PlanBlock> }
pub enum PackageResult { Applied { recovery: OpId }, Skipped(Explanation), RolledBack(Explanation),
                         NeedsHelp(Explanation), Failed(Explanation) }
```

## 8. `status`, `explain`

```rust
pub enum Headline { Ok, Working, Waiting, NeedsHelp, Error }
pub enum PackageStatus { InSync, Available /* target newer, not yet applied */, AppliedHere,
                         ConfirmedBy(Vec<ComputerId>), NeedsDecision, Unavailable, Blocked(PlanBlock) }
pub struct MemberStatus { pub computer: ComputerId, pub display_name: String, pub last_seq: Option<u64>,
                          pub reported_at_peer_clock: Option<i64>, pub seen_locally_at: Option<i64>,
                          pub state: MemberLatest /* summarized */ }
pub struct StatusView { pub headline: Headline, pub packages: Vec<(FolderKey, PackageStatus, Explanation)>,
                        pub members: Vec<MemberStatus>, pub last_publish: Option<i64>, pub blocks: Vec<PlanBlock> }
pub fn derive_status(input: &StatusInput) -> StatusView;
pub fn confirmations(target: &GroupTarget, members: &[MemberView]) -> BTreeMap<FolderKey, Vec<ComputerId>>;

pub enum Explanation { /* closed set, each with structured fields, e.g. */
    NewerVersion { folder: String, from: String, to: String, source: ComputerId },
    Installed { source: ComputerId }, AmbiguousVersions { evidence: Vec<FolderEvidence> },
    GameRunning, ActivityUnknown, Unstable, NotAvailableYet { missing: Vec<Sha256> },
    RolledBackLaunch, HashMismatch { folder: String }, AlreadyNewerHere, Unsupported { folder: String, why: String },
    /* … */ }
pub fn render(e: &Explanation) -> String;   // plain English, single line; no peer clock as "current"
```
