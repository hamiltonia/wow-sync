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
    pub synthetic_marker: FileObs,   // <install root>/.wowsync-synthetic (slice gate, domain.md §2.1)
}
pub struct VariantIdentity { pub product: ProductCode, pub folder: String,
                             pub build_version: Option<String>, pub profile: Option<&'static Profile> }
pub struct Profile { pub product: &'static str, pub folder: &'static str, pub label: &'static str,
                     pub toc_suffixes: &'static [&'static str], pub mapping_id: &'static str /* "profile:wowf/1" */ }
pub const KNOWN_PROFILES: &[Profile];   // only wowf in the slice (domain.md §2)
pub enum VariantVerdict { Recognized(VariantIdentity), Unfamiliar(VariantIdentity), Refused(RefuseReason) }
pub fn validate_variant(obs: &VariantObservation) -> VariantVerdict;

pub enum TocMapping { Profile(&'static Profile), ValidatedPlain /* "validated-plain/1" */ }
pub struct OtherVariants { pub same_product_folders: Vec<String>, pub enrolled_elsewhere: bool }
pub enum LayoutAssessment {
    Supported { mapping: TocMapping, eligible: Vec<FolderKey>, blocked: Vec<(FolderKey, BlockReason)>, digest: Sha256 },
    Unsupported(LayoutReason),       // AddOnsLayout | DuplicateProductFolder | AlreadyEnrolled | NotSyntheticInstall (slice, Q-24)
}
pub fn assess_layout(id: &VariantIdentity, unfamiliar: bool, addons: &[FolderScan], others: &OtherVariants) -> LayoutAssessment;
pub struct GuidedConfirmation { pub identity: bool, pub scope: bool, pub digest: Sha256 }
/// Recognized + Supported: Ok (consent text still shown by the GUI).
/// Unfamiliar + Supported: Ok only if identity && scope && digest == assessment digest.
/// Anything else: Err. Consent alone (no Supported assessment) never succeeds (M-02).
pub fn confirm_variant(v: &VariantVerdict, a: &LayoutAssessment, c: Option<&GuidedConfirmation>)
    -> Result<ConfirmedVariant, RefuseReason>;
pub struct VariantValidation { /* serializable evidence stored in local-enrollment: kind, identity, mapping id, digest, counts */ }
```

`ConfirmedVariant` (identity + mapping) is the only value accepted by
enrollment and planning functions; it can only be constructed by
`confirm_variant` (private fields).

## 3. `toc`, `version`

```rust
pub struct TocInfo { pub interface: Vec<u32>, pub version: Option<String>, pub title: Option<String>,
                     pub hard_deps: Vec<String>, pub ids: Vec<ExternalId> }
pub struct ExternalId { pub kind: IdKind /* Curse | Wago | WoWI */, pub value: String }
pub fn parse_toc(bytes: &[u8]) -> Result<TocInfo, TocError>;  // TooLarge | NotUtf8
pub struct TocCandidate { pub file_name: String, pub contents: FileObs }
pub enum TocSelection {
    Selected { file_name: String, suffix: Option<String>, info: TocInfo },
    None { reason: String },
    Ambiguous { files: Vec<String> },
    Unvalidated { files: Vec<String> },   // unfamiliar client with suffixed tocs (domain.md §2.1)
    Error { file_name: String, reason: String },
}
pub fn select_toc(folder: &str, candidates: &[TocCandidate], mapping: &TocMapping) -> TocSelection;

pub enum Version { DottedNumeric{..}, Integer(u64), Revision(u64), Date(u32,u32,u32),
                   Tagged{ tag: String, v: Box<Version> }, Missing, Token(String), GitHash(String), Other(String) }
pub enum VersionOrder { Less, Equal, Greater, Ambiguous(String) }
pub fn classify(raw: Option<&str>) -> Version;
pub fn compare(a: &Version, b: &Version) -> VersionOrder;   // channels and describe rules: domain.md §4
```

## 4. `inventory`, `package`

```rust
/// What adapters report for one AddOns top-level entry (mirrors adapters::fs_scan::ScannedFolder).
pub struct FolderScan { pub name: String, pub status: ScanStatus, pub files: Vec<ScannedFile>,
                        pub tocs: Vec<TocCandidate>, pub fingerprint: Option<Sha256> }
pub enum ScanStatus { Ok, Unreadable(String), Unsafe(String) /* reparse/link */, Unstable(String) /* changed while read */ }
pub struct ScannedFile { pub rel_path: String, pub size: u64, pub sha256: Sha256 }

pub enum FolderStatus { Ok, Unreadable, Unsafe, Unsupported, Unstable }
pub struct FileEntry { pub path: RelPath, pub size: u64, pub sha256: Sha256 }
pub struct FolderState { pub name: String, pub key: FolderKey, pub status: FolderStatus,
                         pub tree: Option<Sha256>, pub fingerprint: Option<Sha256>, pub files: u32, pub bytes: u64,
                         pub toc: TocSummary /* kind, file, suffix, version, interface, deps, ids */,
                         pub file_entries: Vec<FileEntry> }
pub struct Inventory { pub folders: Vec<FolderState>, pub listing_stable: bool } // sorted by key
pub fn build_inventory(scans: Vec<FolderScan>, listing_stable: bool, v: &ConfirmedVariant) -> Inventory;

pub struct Package { pub key: FolderKey, pub display: FolderKey, pub members: Vec<FolderKey>,
                     pub edges: Vec<(FolderKey, FolderKey, EdgeKind /* SharedId|Dependency|NamePrefix|Family */)> }
pub fn group_packages(observations: &[&Inventory]) -> Vec<Package>;   // domain.md §5
pub struct PackageVersion { pub package: FolderKey, pub hash: Sha256, pub folders: Vec<FolderState>,
                            pub client: ClientInfo /* build_version, toc mapping id of the holder */ }
pub fn package_version(pkg: &Package, inv: &Inventory, client: &ClientInfo) -> Option<PackageVersion>;
pub enum SetRelation { Equal, AHasMore, BHasMore, Different }
pub enum PackageOrder { Same, Greater { set: SetRelation }, Less { set: SetRelation },
                        Ambiguous { reason: AmbiguityReason, set: SetRelation, evidence: Vec<FolderEvidence> } }
pub fn compare_package(a: &PackageVersion, b: &PackageVersion) -> PackageOrder;
```

## 5. `records`, `layout`, `admit`

Rust structs mirroring every schema in [publication.md](publication.md) §2
(`RootFormat`, `GroupRecord`, `MemberRecord`, `CodeState`, `FolderTreeRecord`,
`ApplyJournal`, `RecoveryPoint`, `AuditEntry`, `LocalComputer`,
`LocalEnrollment`, `AdmissionWatermarks`).

```rust
pub fn encode<T: Record>(r: &T) -> Vec<u8>;                     // pretty JSON, LF, trailing newline
pub fn decode<T: Record>(bytes: &[u8], limit: usize) -> Result<T, RecordError>;
// RecordError: TooLarge | Json(String) | SchemaMissing | UnknownSchema(String) | UnsupportedMajor(u32) | Invalid(String)
pub trait Record: Serialize + DeserializeOwned { const SCHEMA: &'static str; const MAJOR: u32; const LIMIT: usize; fn validate(&self) -> Result<(), String>; }
pub fn decode_tree(bytes: &[u8], name_hash: &Sha256) -> Result<FolderTreeRecord, RecordError>; // recomputes tree hash, validates paths

// layout: all return '/'-separated paths relative to <shared>/WoWSync or <variant>/.wowsync
pub fn group_record(g: &GroupId) -> String;            // "groups/<g>/group.json"
pub fn member_record(g: &GroupId, c: &ComputerId) -> String;
pub fn state_record(g: &GroupId, c: &ComputerId, e: &EnrollmentId, seq: u64) -> String;
pub fn parse_state_name(file_name: &str) -> Option<(EnrollmentId, u64)>;
pub fn tree_record(g: &GroupId, h: &Sha256) -> String;
pub fn blob(g: &GroupId, h: &Sha256) -> String;
pub fn staging_dir(op: &OpId) -> String;  pub fn recovery_dir(pkg: &FolderKey, op: &OpId) -> String;
pub fn journal(op: &OpId) -> String;

// admit (publication.md §6): the only place that decides which member state is used.
pub struct StateFileObs { pub name: String, pub size: u64, pub cloud_only: bool }
pub struct MemberObs { pub member: Result<MemberRecord, RecordError>, pub files: Vec<StateFileObs>,
                       pub newest_bytes: Option<Result<Vec<u8>, String>> /* bytes of the highest-seq name, if read */ }
pub fn newest_state_name(m: &MemberObs) -> Option<String>; // runtime reads exactly this file
pub fn admit_member_states(group: &GroupRecord, obs: &[MemberObs], marks: &AdmissionWatermarks)
    -> (Vec<MemberView>, AdmissionWatermarks);
```

## 6. `target`, `plan`

```rust
pub struct MemberView { pub computer: ComputerId, pub enrollment: EnrollmentId, pub display_name: String,
                        pub latest: MemberLatest }
pub enum MemberLatest { Valid { seq: u64, state: CodeState }, Unusable { seq: u64, reason: String },
                        Regressed { max_seq_seen: u64 }, InvalidMember(String), NoState }
pub struct TargetInput<'a> { pub group: &'a GroupRecord, pub variant: &'a ConfirmedVariant,
                             pub local: &'a Inventory, pub local_client: &'a ClientInfo,
                             pub local_id: &'a ComputerId, pub members: &'a [MemberView],
                             pub trees: &'a BTreeMap<Sha256, TreeAvail> /* Ok(record) | Missing | CloudOnly | Corrupt */ }
pub enum PackageTarget { InSync, Chosen { version: PackageVersion, sources: Vec<ComputerId>, why: Explanation },
                         NeedsDecision { reason: DecisionReason, candidates: Vec<(PackageVersion, Vec<ComputerId>)>, why: Explanation },
                         Unavailable { why: Explanation } }
// DecisionReason: Ambiguous | NoCompatibleToc | DifferentTocMapping
//               | UnvalidatedTocMapping | PartialCandidate          (domain.md §6)
// (D-21: DifferentClientBuild was removed; build and Interface numbers never gate a package.)
pub struct GroupTarget { pub packages: BTreeMap<FolderKey, PackageTarget> }
pub fn compute_target(input: &TargetInput) -> GroupTarget;

pub struct PublishContext { pub group: GroupId, pub computer: ComputerId, pub enrollment: EnrollmentId,
                            pub seq: u64, pub client: ClientInfo, pub apply_report: Option<ApplyReport>,
                            pub own_state_files: Vec<String> }
pub struct PublishPlan { pub objects: Vec<ObjectRef /* blob or tree, with local source */>,
                         pub state: CodeState, pub state_path: String, pub prune_states: Vec<String>,
                         pub source_fingerprints: Vec<(String /* folder */, Sha256)> }
pub fn plan_publish(inv: &Inventory, ctx: &PublishContext) -> Result<PublishPlan, PlanBlock>;

pub struct ApplyPlan { pub packages: Vec<PackageApply> }   // sorted by package key
/// One package = one operation (apply.md §2.2): own OpId, journal, staging, recovery dir.
pub struct PackageApply { pub op: OpId, pub package: FolderKey, pub target_hash: Sha256,
                          pub folders: Vec<FolderApply>, pub explanation: Explanation }
pub struct FolderApply { pub name: String, pub key: FolderKey, pub action: FolderAction,
                         pub tree: FolderTreeRecord, pub expected_live: ExpectedLive }
pub enum ExpectedLive { Absent, Tree { tree: Sha256, fingerprint: Sha256 } }   // apply.md §2.1
pub enum FolderAction { Install /* absent locally */, Replace /* present, different tree */ }
/// 
ew_op is called once per planned package (ids come from the runtime's RNG).
pub fn plan_apply(target: &GroupTarget, local: &Inventory, new_op: &mut dyn FnMut() -> OpId, gate: &ApplyGate) -> Result<ApplyPlan, PlanBlock>;
pub struct ApplyGate { pub activity: Activity, pub variant: ConfirmedVariant, pub any_journal: bool /* Q-25 */ }
pub enum Activity { NotRunning, Running { pids: Vec<u32> }, Unknown(String) }
pub enum PlanBlock { GameRunning, ActivityUnknown(String), Unstable, UnresolvedJournal, NothingToDo, Invalid(String) }
```

`plan_apply` includes only `Chosen` packages whose local folders are all
`Ok` with a fingerprint (stable this scan); `expected_live` comes from that
scan. It never plans a package whose target is not complete against the
group-wide membership (domain.md §6 `PartialCandidate`). `plan_publish` refuses when
`listing_stable` is false; unstable folders are published with status
`unstable` and no tree.

## 7. `exec` (port traits and executor)

```rust
pub enum Slot { Live { folder: String }, Staging { op: OpId, folder: String },
                Recovery { pkg: FolderKey, op: OpId, folder: String } }
pub struct SlotObs { pub fingerprint: Sha256, pub recently_modified: bool /* adapter: newest mtime within window */ }
pub enum ObjectStat { Missing, CloudOnly, Local { size: u64, mtime_ns: i128 } }

pub trait PublishEffects {
    fn stat_object(&mut self, rel: &str) -> Result<ObjectStat, EffectError>;
    fn hash_object(&mut self, rel: &str) -> Result<Sha256, EffectError>;            // Local objects only
    /// Copy source to temp in the provider, verify hash, rename over `rel`; returns (size, mtime_ns).
    fn put_object(&mut self, obj: &ObjectRef) -> Result<(u64, i128), EffectError>;
    fn observe_source(&mut self, folder: &str) -> Result<Option<SlotObs>, EffectError>;  // live AddOns folder
    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError>;   // temp + rename
    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError>;             // own member dir only
}
pub trait ApplyEffects {
    fn activity(&mut self) -> Activity;
    fn lock(&mut self) -> Result<(), EffectError>;
    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError>;   // temp + flush + rename
    fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError>; // core decodes
    fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError>;
    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError>; // copy blobs, verify each hash
    fn tree_of(&mut self, slot: &Slot) -> Result<Option<Sha256>, EffectError>;   // content hash; None = absent
    fn observe(&mut self, slot: &Slot) -> Result<Option<SlotObs>, EffectError>;  // metadata; None = absent
    fn rename(&mut self, from: &Slot, to: &Slot) -> Result<(), EffectError>;     // fails if `to` exists
    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError>;
    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<(String, Vec<u8>)>, EffectError>; // core decodes
    fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError>;
    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError>;
}
pub struct EffectError { pub kind: EffectErrorKind /* NotFound | Exists | InUse | CloudOnly | HashMismatch | Unsafe | Io | Crashed */,
                         pub detail: String }
pub enum ObjectCheck { VerifiedOk, Missing, Corrupt, Unverifiable }
/// stat; Local -> hash_object now (== expected ? VerifiedOk : Corrupt); CloudOnly -> Unverifiable;
/// Missing -> Missing. No metadata cache (retry 2, M-05).
pub fn check_object(fx: &mut impl PublishEffects, rel: &str, expect: &Sha256) -> Result<ObjectCheck, EffectError>;
pub fn run_publish(plan: &PublishPlan, ctx: &mut PublishContext, fx: &mut impl PublishEffects) -> PublishOutcome; // publication.md §4
pub fn run_apply(plan: &ApplyPlan, fx: &mut impl ApplyEffects) -> ApplyOutcome;      // apply.md §2
pub fn recover_interrupted(fx: &mut impl ApplyEffects) -> RecoveryOutcome;            // apply.md §4
pub struct ApplyOutcome { pub packages: Vec<(FolderKey, PackageResult)>, pub stopped: Option<PlanBlock> }
pub enum PackageResult { Applied { recovery: OpId }, Skipped(Explanation), Replanned(Explanation),
                         Interrupted(Explanation), RolledBack(Explanation), NeedsHelp(Explanation), Failed(Explanation) }
pub struct RecoveryOutcome { pub ops: Vec<(OpId, FolderKey, PackageResult)>, pub journals_remaining: bool }
/// Pure slot classification used by rollback/verify (apply.md §3.1).
pub enum SlotConfig { Untouched, Half, Done, Unknown }
pub fn classify_slots(install: bool, live: bool, staging: bool, recovery: bool) -> SlotConfig;
```

Every port method maps to exactly one adapter call or a fixed short
sequence ([adapters-api.md](adapters-api.md) §9); none needs private storage
access.

## 8. `status`, `explain`

```rust
pub enum Headline { Ok, Working, Waiting, NeedsHelp, Error }
pub enum PackageStatus { InSync, Available, AppliedHere, ConfirmedBy(Vec<ComputerId>),
                         NeedsDecision(DecisionReason), Unavailable, Interrupted, NeedsHelp(String), Blocked(PlanBlock) }
pub struct MemberStatus { pub computer: ComputerId, pub display_name: String, pub last_seq: Option<u64>,
                          pub reported_at_peer_clock: Option<i64>, pub seen_locally_at: Option<i64>,
                          pub state: MemberLatest /* summarized */ }
pub struct StatusView { pub headline: Headline, pub packages: Vec<(FolderKey, PackageStatus, Explanation)>,
                        pub members: Vec<MemberStatus>, pub last_publish: Option<i64>, pub blocks: Vec<PlanBlock>,
                        pub retained_extra_recovery: u32 /* pinned points beyond the normal two */ }
pub fn derive_status(input: &StatusInput) -> StatusView;
// An interruption is `InterruptedByGame` only when its journal records the game start
// (`ApplyJournal.halted_by_game`, collected by `game_halted`); otherwise `Interrupted` (neutral).
pub fn next_action(status: &PackageStatus, why: &Explanation, game_running: bool) -> NextAction;
// Interrupted: game running now => CloseGame ("Close WoW to let the update finish");
// otherwise FinishOnNextSync (no "close WoW" step).
pub fn confirmations(target: &GroupTarget, members: &[MemberView]) -> BTreeMap<FolderKey, Vec<ComputerId>>;

pub enum Explanation { /* closed set, each with structured fields, e.g. */
    NewerVersion { folder: String, from: String, to: String, source: ComputerId },
    Installed { source: ComputerId }, AmbiguousVersions { evidence: Vec<FolderEvidence> },
    PartialCandidate { missing: Vec<String> /* membership folders absent from the candidate */ }, NoCompatibleToc { folder: String },
    DifferentTocMapping { here: String, there: String }, UnvalidatedTocMapping { folder: String },
    GameRunning, ActivityUnknown(String), Unstable, NotAvailableYet { missing: Vec<Sha256> },
    MemberUnusable { computer: ComputerId, seq: u64, reason: String }, MemberRegressed { computer: ComputerId },
    DestinationChanged { folder: String }, InterruptedByGame { package: String },
    LiveMismatch { folder: String }, RecoveryMismatch { folder: String }, RestoreFailed { folder: String },
    AlreadyNewerHere, Unsupported { folder: String, why: String }, GroupedBy { edges: Vec<String> },
    /* … */ }
pub fn render(e: &Explanation) -> String;   // plain English, single line; no peer clock as "current"
```