//! Record schemas, encoding and decoding (publication.md §2).

use crate::hash::tree_hash;
use crate::ids::{CheckpointRef, ComputerId, EnrollmentId, GroupId, OpId, SettingsUnit, Sha256};
use crate::inventory::{FileEntry, FolderState, FolderStatus};
use crate::package::ClientInfo;
use crate::paths::{validate_folder_name, FolderKey};
use crate::plan::{ExpectedLive, FolderAction};
use crate::variant::VariantValidation;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const KIB: usize = 1024;
pub const MIB: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    TooLarge,
    Json(String),
    SchemaMissing,
    UnknownSchema(String),
    UnsupportedMajor(u32),
    Invalid(String),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "record too large"),
            Self::Json(e) => write!(f, "malformed record: {e}"),
            Self::SchemaMissing => write!(f, "record has no schema"),
            Self::UnknownSchema(s) => write!(f, "unknown record schema {s}"),
            Self::UnsupportedMajor(m) => {
                write!(f, "written by a newer or other version (major {m})")
            }
            Self::Invalid(e) => write!(f, "invalid record: {e}"),
        }
    }
}

pub trait Record: Serialize + DeserializeOwned {
    const SCHEMA: &'static str;
    /// The major `encode` writes unless `write_major` says otherwise.
    const MAJOR: u32;
    /// Majors `decode` accepts (contracts.md §2.1). Every slice record keeps `[1]`.
    const MAJORS: &'static [u32] = &[];
    const LIMIT: usize;
    fn validate(&self) -> Result<(), String>;
    /// The major chosen for this value on encode.
    fn write_major(&self) -> u32 {
        Self::MAJOR
    }
    /// A value decoded as `major` must not use content of a later major.
    fn validate_major(&self, _major: u32) -> Result<(), String> {
        Ok(())
    }
}

fn accepts_major<T: Record>(m: u32) -> bool {
    if T::MAJORS.is_empty() {
        m == T::MAJOR
    } else {
        T::MAJORS.contains(&m)
    }
}

/// Pretty JSON, LF, trailing newline, `schema` first.
pub fn encode<T: Record>(r: &T) -> Vec<u8> {
    let body = serde_json::to_string_pretty(r).expect("records serialize");
    let rest = body.strip_prefix("{\n").expect("records are objects");
    let mut out = format!(
        "{{\n  \"schema\": \"{}/{}\",\n{}",
        T::SCHEMA,
        r.write_major(),
        rest
    );
    out.push('\n');
    out.into_bytes()
}

pub fn decode<T: Record>(bytes: &[u8], limit: usize) -> Result<T, RecordError> {
    if bytes.len() > limit {
        return Err(RecordError::TooLarge);
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| RecordError::Json(e.to_string()))?;
    let obj = value
        .as_object_mut()
        .ok_or_else(|| RecordError::Json("not an object".into()))?;
    let schema = match obj.remove("schema") {
        Some(serde_json::Value::String(s)) => s,
        _ => return Err(RecordError::SchemaMissing),
    };
    let Some((name, major)) = schema.rsplit_once('/') else {
        return Err(RecordError::UnknownSchema(schema));
    };
    if name != T::SCHEMA {
        return Err(RecordError::UnknownSchema(schema));
    }
    let found = match major.parse::<u32>() {
        Ok(m) if accepts_major::<T>(m) => m,
        Ok(m) => return Err(RecordError::UnsupportedMajor(m)),
        Err(_) => return Err(RecordError::UnknownSchema(schema)),
    };
    let rec: T = serde_json::from_value(value).map_err(|e| RecordError::Json(e.to_string()))?;
    rec.validate().map_err(RecordError::Invalid)?;
    rec.validate_major(found).map_err(RecordError::Invalid)?;
    Ok(rec)
}

macro_rules! record {
    ($t:ty, $schema:literal, $limit:expr, |$s:ident| $validate:block) => {
        record!($t, $schema, 1, $limit, |$s| $validate);
    };
    ($t:ty, $schema:literal, $major:literal, $limit:expr, |$s:ident| $validate:block) => {
        impl Record for $t {
            const SCHEMA: &'static str = $schema;
            const MAJOR: u32 = $major;
            const LIMIT: usize = $limit;
            fn validate(&$s) -> Result<(), String> $validate
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootFormat {
    pub created_unix_ms: u64,
}
record!(RootFormat, "wowsync.root", 64 * KIB, |self| { Ok(()) });

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRecord {
    pub group_id: GroupId,
    pub product: String,
    pub variant_label: String,
    pub display_name: String,
    pub created_by: ComputerId,
    pub created_unix_ms: u64,
}
record!(GroupRecord, "wowsync.group", 64 * KIB, |self| {
    if self.product.is_empty() {
        return Err("empty product".into());
    }
    Ok(())
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberRecord {
    pub group_id: GroupId,
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
    pub display_name: String,
    pub product: String,
    pub variant_folder: String,
    pub enrolled_unix_ms: u64,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired: Option<bool>,
    /// Enrollments this computer held before it re-enrolled; grow-only (contracts.md §1.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous_enrollments: Vec<EnrollmentId>,
}
record!(MemberRecord, "wowsync.member", 64 * KIB, |self| {
    if self.retired == Some(true) {
        return Err("retired members are not supported in the slice".into());
    }
    Ok(())
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    Applied,
    Skipped,
    Replanned,
    Interrupted,
    RolledBack,
    NeedsHelp,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasisEntry {
    pub computer_id: ComputerId,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportResult {
    pub package: FolderKey,
    pub result: ReportKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_hash: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyReport {
    pub op: OpId,
    pub basis: Vec<BasisEntry>,
    pub results: Vec<ReportResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeState {
    pub group_id: GroupId,
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
    pub seq: u64,
    pub product: String,
    pub app_version: String,
    pub written_unix_ms: u64,
    pub client: ClientInfo,
    pub inventory_complete: bool,
    pub folders: Vec<FolderState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_report: Option<ApplyReport>,
    /// Additive in `code-state/1` (contracts.md §1.6): the head of this enrollment's authority
    /// index chain and the number of refs it lists. Old readers ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_index: Option<AuthorityIndexHead>,
    /// Additive: the capabilities this app applies (contracts.md §1.13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityIndexHead {
    pub head: Sha256,
    pub total: u64,
}

/// The capabilities this build applies, in the order of contracts.md §1.13.
pub const FEATURES: &[&str] = &[
    "authority/1",
    "removal/1",
    "code-choice/1",
    "rejection/1",
    "settings/1",
    "retirement/1",
    "journal/2",
];

record!(CodeState, "wowsync.code-state", 16 * MIB, |self| {
    let mut seen = std::collections::BTreeSet::new();
    for f in &self.folders {
        if !seen.insert(f.key.clone()) {
            return Err(format!("duplicate folder {}", f.key));
        }
        match f.status {
            FolderStatus::Ok if f.tree.is_none() => {
                return Err(format!("folder {} is ok without a tree", f.name))
            }
            FolderStatus::Ok => {}
            _ if f.tree.is_some() => {
                return Err(format!("folder {} has a tree but is not ok", f.name))
            }
            _ => {}
        }
        if f.status != FolderStatus::Unsupported {
            match validate_folder_name(&f.name) {
                Ok(k) if k == f.key => {}
                _ => return Err(format!("folder name {:?} does not match its key", f.name)),
            }
        }
    }
    Ok(())
});

/// Context checks that a state record is the one its file name, group and member say it is.
pub fn validate_state_for(
    s: &CodeState,
    group: &GroupRecord,
    computer: &ComputerId,
    enrollment: &EnrollmentId,
    name_seq: u64,
) -> Result<(), String> {
    if s.group_id != group.group_id {
        return Err("state belongs to another group".into());
    }
    if s.product != group.product {
        return Err("state belongs to another product".into());
    }
    if &s.computer_id != computer {
        return Err("state belongs to another computer".into());
    }
    if &s.enrollment_id != enrollment {
        return Err("state belongs to another enrollment".into());
    }
    if s.seq != name_seq {
        return Err("state sequence disagrees with its file name".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderTreeRecord {
    pub tree: Sha256,
    pub files: Vec<FileEntry>,
}
record!(FolderTreeRecord, "wowsync.folder-tree", 32 * MIB, |self| {
    if self
        .files
        .windows(2)
        .any(|w| w[0].path.as_str().as_bytes() >= w[1].path.as_str().as_bytes())
    {
        return Err("files are not sorted and unique".into());
    }
    if tree_hash(&self.files) != self.tree {
        return Err("tree hash does not match its files".into());
    }
    Ok(())
});

/// `name_hash` is the hash in the object's file name; it must equal the recomputed tree hash.
pub fn decode_tree(bytes: &[u8], name_hash: &Sha256) -> Result<FolderTreeRecord, RecordError> {
    let rec: FolderTreeRecord = decode(bytes, FolderTreeRecord::LIMIT)?;
    if &rec.tree != name_hash {
        return Err(RecordError::Invalid(
            "tree hash does not match its name".into(),
        ));
    }
    Ok(rec)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalPhase {
    Staging,
    Committing,
    Committed,
    Verified,
    NeedsHelp,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalFolder {
    pub name: String,
    pub key: FolderKey,
    pub action: FolderAction,
    pub target_tree: Sha256,
    pub expected_live: ExpectedLive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyJournal {
    pub op: OpId,
    pub group_id: GroupId,
    pub started_unix_ms: u64,
    pub package: FolderKey,
    pub target_hash: Sha256,
    pub phase: JournalPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_phase: Option<JournalPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The executor stopped this update because the game started. Missing (old journals) means
    /// the cause is unknown.
    #[serde(default, skip_serializing_if = "is_false")]
    pub halted_by_game: bool,
    pub folders: Vec<JournalFolder>,
}

fn is_false(b: &bool) -> bool {
    !*b
}
record!(ApplyJournal, "wowsync.apply-journal", MIB, |self| {
    if self.phase == JournalPhase::RolledBack
        && !matches!(
            self.from_phase,
            Some(JournalPhase::Staging | JournalPhase::Committing)
        )
    {
        return Err("rolled_back needs from_phase staging or committing".into());
    }
    if self.folders.is_empty() {
        return Err("journal without folders".into());
    }
    if self
        .folders
        .iter()
        .any(|f| f.action == FolderAction::Remove)
    {
        return Err("a remove action needs apply-journal/2".into());
    }
    Ok(())
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeRef {
    pub name: String,
    pub tree: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPoint {
    pub op: OpId,
    pub package: FolderKey,
    pub created_unix_ms: u64,
    pub replaced: Vec<TreeRef>,
    pub installed: Vec<TreeRef>,
    pub reason: String,
    pub pinned: bool,
}
record!(RecoveryPoint, "wowsync.recovery-point", 64 * KIB, |self| {
    Ok(())
});

// ---------------------------------------------------------------- C-JRN `/2` (contracts.md §2)
//
// The `/2` shapes are separate Rust types next to the slice types so that slice code and its
// exhaustive matches stay unchanged. `decode_journal` and `decode_point` accept both majors.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    Apply,
    Remove,
    Restore,
    Rollback,
    Settings,
    SettingsCleanup,
}

/// Where a journal or recovery-point slot lives, relative to the variant's `.wowsync` folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecoverySlot(String);

impl RecoverySlot {
    pub fn new(s: &str) -> Result<Self, String> {
        crate::paths::validate_rel_path(s).map_err(|e| format!("{e:?}"))?;
        Ok(Self(s.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for RecoverySlot {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RecoverySlot {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        RecoverySlot::new(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsFileOp {
    pub unit: SettingsUnit,
    pub live_rel: crate::paths::SettingsPath,
    pub expected_live: ExpectedLive,
    pub staged: Option<Sha256>,
    pub recovery_slot: RecoverySlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Local,
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    Paused,
    UnknownOwner,
    SharedFile,
    Unreadable,
    TooLarge,
    IdentityCollision,
    NotIncluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RootState {
    Captured {
        checkpoint: CheckpointRef,
        location: Location,
    },
    Missing {
        reason: MissingReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRoot {
    pub unit: SettingsUnit,
    pub state: RootState,
}

impl SettingsRoot {
    pub fn captured(&self) -> bool {
        matches!(self.state, RootState::Captured { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paired {
    NotApplicable,
    Paired,
    Partial,
    CodeOnly,
    SettingsOnly,
}

/// The §2.3 table. `None` means no point is written: nothing was captured or replaced.
pub fn derive_paired(code_nonempty: bool, settings: &[SettingsRoot]) -> Option<Paired> {
    let captured = settings.iter().filter(|r| r.captured()).count();
    if code_nonempty {
        Some(if settings.is_empty() {
            Paired::NotApplicable
        } else if captured == settings.len() {
            Paired::Paired
        } else if captured == 0 {
            Paired::CodeOnly
        } else {
            Paired::Partial
        })
    } else if captured >= 1 {
        Some(Paired::SettingsOnly)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRecord {
    pub hash: Sha256,
    /// The canonical record bytes (UTF-8 JSON), fixed at journal creation.
    pub bytes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalFolderV2 {
    pub name: String,
    pub key: FolderKey,
    pub action: FolderAction,
    pub expected_live: ExpectedLive,
    pub staged: Option<Sha256>,
    pub recovery_slot: RecoverySlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyJournalV2 {
    pub op: OpId,
    pub group_id: GroupId,
    pub started_unix_ms: u64,
    pub kind: OpKind,
    pub package: FolderKey,
    pub target_hash: Option<Sha256>,
    pub phase: JournalPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_phase: Option<JournalPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub halted_by_game: bool,
    pub folders: Vec<JournalFolderV2>,
    #[serde(default)]
    pub removals: Vec<Sha256>,
    #[serde(default)]
    pub pending_records: Vec<PendingRecord>,
    #[serde(default)]
    pub settings_ops: Vec<SettingsFileOp>,
}

impl ApplyJournalV2 {
    /// A journal stays `/1` iff it is a pure code apply (contracts.md §2.2).
    pub fn is_v1_expressible(&self) -> bool {
        self.kind == OpKind::Apply
            && !self.folders.is_empty()
            && self
                .folders
                .iter()
                .all(|f| matches!(f.action, FolderAction::Install | FolderAction::Replace))
            && self.removals.is_empty()
            && self.settings_ops.is_empty()
            && self.pending_records.is_empty()
            && self.target_hash.is_some()
    }

    pub fn removal_hashes_pending(&self) -> Vec<Sha256> {
        self.pending_records.iter().map(|p| p.hash).collect()
    }
}

/// V-JRN, the parts that need no ledger.
fn validate_journal_v2(j: &ApplyJournalV2) -> Result<(), String> {
    if j.phase == JournalPhase::RolledBack
        && !matches!(
            j.from_phase,
            Some(JournalPhase::Staging | JournalPhase::Committing)
        )
    {
        return Err("rolled_back needs from_phase staging or committing".into());
    }
    let has = |a: FolderAction| j.folders.iter().any(|f| f.action == a);
    let (remove, installs) = (
        has(FolderAction::Remove),
        has(FolderAction::Install) || has(FolderAction::Replace),
    );
    let ops = !j.settings_ops.is_empty();
    let ok = match j.kind {
        OpKind::Apply | OpKind::Rollback => {
            (!j.folders.is_empty() || ops) && (j.removals.is_empty() != remove)
        }
        OpKind::Remove => !installs && remove && !j.removals.is_empty(),
        OpKind::Restore => !remove && j.removals.is_empty() && (!j.folders.is_empty() || ops),
        OpKind::Settings | OpKind::SettingsCleanup => {
            j.folders.is_empty() && j.removals.is_empty() && ops
        }
    };
    if !ok {
        return Err("journal kind and actions do not match".into());
    }
    for f in &j.folders {
        let staged_ok = (f.action == FolderAction::Remove) == f.staged.is_none();
        if !staged_ok {
            return Err("staged content only for install and replace".into());
        }
    }
    let mut keys = std::collections::BTreeSet::new();
    if !j.folders.iter().all(|f| keys.insert(f.key.clone())) {
        return Err("duplicate folder".into());
    }
    let mut units = std::collections::BTreeSet::new();
    if !j.settings_ops.iter().all(|o| units.insert(o.unit.clone())) {
        return Err("duplicate settings unit".into());
    }
    if !crate::kinds::strictly_sorted(&j.removals) {
        return Err("removals must be sorted and unique".into());
    }
    for p in &j.pending_records {
        let bytes = p.bytes.as_bytes();
        if crate::hash::sha256(bytes) != p.hash {
            return Err("pending record hash does not match its bytes".into());
        }
        crate::authority::decode_authority(bytes, &p.hash)
            .map_err(|e| format!("pending record: {e:?}"))?;
    }
    Ok(())
}

record!(
    ApplyJournalV2,
    "wowsync.apply-journal",
    2,
    4 * MIB,
    |self| { validate_journal_v2(self) }
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPointV2 {
    pub op: OpId,
    pub group_id: GroupId,
    pub kind: OpKind,
    pub package: FolderKey,
    pub code_package_hash: Option<Sha256>,
    pub created_unix_ms: u64,
    pub replaced: Vec<TreeRef>,
    pub installed: Vec<TreeRef>,
    #[serde(default)]
    pub removals: Vec<Sha256>,
    #[serde(default)]
    pub settings: Vec<SettingsRoot>,
    pub paired: Paired,
    pub reason: String,
    pub pinned: bool,
}
record!(
    RecoveryPointV2,
    "wowsync.recovery-point",
    2,
    256 * KIB,
    |self| {
        let code = !self.replaced.is_empty() || !self.installed.is_empty();
        match derive_paired(code, &self.settings) {
            Some(p) if p == self.paired => Ok(()),
            Some(_) => Err("paired does not match the code and settings parts".into()),
            None => Err("a point with nothing captured or replaced is never written".into()),
        }
    }
);

/// A journal of either major, as found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyJournal {
    V1(ApplyJournal),
    V2(ApplyJournalV2),
}

impl AnyJournal {
    pub fn op(&self) -> &OpId {
        match self {
            Self::V1(j) => &j.op,
            Self::V2(j) => &j.op,
        }
    }
}

pub fn decode_journal(bytes: &[u8]) -> Result<AnyJournal, RecordError> {
    match decode::<ApplyJournal>(bytes, ApplyJournal::LIMIT) {
        Err(RecordError::UnsupportedMajor(2)) => {
            decode::<ApplyJournalV2>(bytes, ApplyJournalV2::LIMIT).map(AnyJournal::V2)
        }
        other => other.map(AnyJournal::V1),
    }
}

pub fn encode_journal(j: &AnyJournal) -> Vec<u8> {
    match j {
        AnyJournal::V1(j) => encode(j),
        AnyJournal::V2(j) => encode(j),
    }
}

/// A recovery point of either major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyPoint {
    V1(RecoveryPoint),
    V2(RecoveryPointV2),
}

pub fn decode_point(bytes: &[u8]) -> Result<AnyPoint, RecordError> {
    match decode::<RecoveryPoint>(bytes, RecoveryPoint::LIMIT) {
        Err(RecordError::UnsupportedMajor(2)) => {
            decode::<RecoveryPointV2>(bytes, RecoveryPointV2::LIMIT).map(AnyPoint::V2)
        }
        other => other.map(AnyPoint::V1),
    }
}

/// Local-only pending report with outcomes the slice report cannot express
/// (contracts.md §2.4). The slice decoder rejects major 2 and starts a fresh report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalOutcomeKind {
    Removed,
    AbsentAcknowledged,
    Restored,
    SettingsApplied,
    SettingsKeptLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOutcome {
    pub package: FolderKey,
    pub kind: LocalOutcomeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingReportV2 {
    pub group_id: GroupId,
    pub enrollment_id: EnrollmentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ApplyReport>,
    #[serde(default)]
    pub outcomes: Vec<LocalOutcome>,
}
record!(PendingReportV2, "wowsync.pending-report", 2, MIB, |self| {
    let mut seen = std::collections::BTreeSet::new();
    if !self
        .outcomes
        .iter()
        .all(|o| seen.insert((o.package.clone(), o.kind as u8)))
    {
        return Err("duplicate outcome".into());
    }
    Ok(())
});

// ---------------------------------------------------------------- settings records (settings.md §3)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointFile {
    pub name: String,
    pub size: u64,
    pub blob: Sha256,
}

/// `wowsync.settings-checkpoint/1`. Its reference is `sha256(encode(self))`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsCheckpointManifest {
    pub unit: SettingsUnit,
    pub files: Vec<CheckpointFile>,
}
record!(
    SettingsCheckpointManifest,
    "wowsync.settings-checkpoint",
    64 * KIB,
    |self| {
        match self.files.as_slice() {
            [] => Ok(()),
            [f] => {
                let want = format!("{}.lua", self.unit.folder.as_str());
                if f.name.to_lowercase() == want {
                    Ok(())
                } else {
                    Err("the file name must be the unit's folder".into())
                }
            }
            _ => Err("a unit has at most one settings file".into()),
        }
    }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedResult {
    Applied,
    KeptLocal,
    Paused,
    Blocked,
    Diagnosed,
    Waiting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedUnit {
    pub unit: SettingsUnit,
    pub revisions: Vec<Sha256>,
    pub checkpoint: CheckpointRef,
    pub result: AppliedResult,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PausedSet {
    pub all: bool,
    pub folders: Vec<FolderKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsDiagnosis {
    pub unit: SettingsUnit,
    /// A closed kind name such as `SharedVariable`; never a path or a name.
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsState {
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
    pub seq: u64,
    pub applied: Vec<AppliedUnit>,
    pub paused: PausedSet,
    pub included_accounts: Vec<String>,
    pub diagnoses: Vec<SettingsDiagnosis>,
}
record!(SettingsState, "wowsync.settings-state", 4 * MIB, |self| {
    let mut units = std::collections::BTreeSet::new();
    if !self.applied.iter().all(|a| units.insert(a.unit.clone())) {
        return Err("a unit is reported once".into());
    }
    Ok(())
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalUnitState {
    pub unit: SettingsUnit,
    /// Newest first.
    pub captures: Vec<CheckpointRef>,
    pub basis: Vec<Sha256>,
    pub last_applied: Option<CheckpointRef>,
    pub pinned: bool,
    pub authority_since: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstAdoption {
    Adopt,
    KeepMine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsLocal {
    pub units: Vec<LocalUnitState>,
    pub paused_folders: Vec<FolderKey>,
    pub included_accounts: Vec<String>,
    pub consent: bool,
    pub first_adoption: Option<FirstAdoption>,
}
record!(SettingsLocal, "wowsync.settings-local", 16 * MIB, |self| {
    let mut units = std::collections::BTreeSet::new();
    if !self.units.iter().all(|a| units.insert(a.unit.clone())) {
        return Err("a unit is listed once".into());
    }
    Ok(())
});

/// The saved-file size cap (settings.md §1.2, Q-77 PENDING OWNER). No value is approved, so this
/// gate is `None` and settings sync on real data stays disabled; tests pass their own value.
pub const MAX_SETTINGS_FILE_BYTES: Option<u64> = None;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalComputer {
    pub computer_id: ComputerId,
    pub display_name: String,
    pub created_unix_ms: u64,
}
record!(LocalComputer, "wowsync.local-computer", 64 * KIB, |self| {
    Ok(())
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalEnrollment {
    pub group_id: GroupId,
    pub enrollment_id: EnrollmentId,
    pub shared_root: String,
    pub install_root: String,
    pub variant_folder: String,
    pub product: String,
    pub validation: VariantValidation,
    pub next_seq: u64,
}
record!(
    LocalEnrollment,
    "wowsync.local-enrollment",
    64 * KIB,
    |self| { Ok(()) }
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watermark {
    pub computer_id: ComputerId,
    pub enrollment_id: EnrollmentId,
    pub max_seq_seen: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admitted_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionWatermarks {
    pub group_id: GroupId,
    pub members: Vec<Watermark>,
}
record!(AdmissionWatermarks, "wowsync.admission", MIB, |self| {
    Ok(())
});

impl AdmissionWatermarks {
    pub fn empty(group_id: GroupId) -> Self {
        Self {
            group_id,
            members: Vec::new(),
        }
    }
    pub fn get(&self, c: &ComputerId, e: &EnrollmentId) -> Option<&Watermark> {
        self.members
            .iter()
            .find(|w| &w.computer_id == c && &w.enrollment_id == e)
    }
}

/// Local-only: the apply report not yet carried by a published state of this enrollment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingReport {
    pub group_id: GroupId,
    pub enrollment_id: EnrollmentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ApplyReport>,
}
record!(PendingReport, "wowsync.pending-report", MIB, |self| {
    if let Some(r) = &self.report {
        let mut seen = std::collections::BTreeSet::new();
        if !r.results.iter().all(|x| seen.insert(x.package.clone())) {
            return Err("duplicate package in pending report".into());
        }
    }
    Ok(())
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at_unix_ms: u64,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    pub detail: String,
}
record!(AuditEntry, "wowsync.audit", 16 * KIB, |self| { Ok(()) });

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::hash::sha256;
    use crate::ids::Identity;
    use crate::testkit::{golden, group_id, op, sample_journal};

    fn key(s: &str) -> FolderKey {
        FolderKey::lossy(s)
    }

    fn folder2(name: &str, action: FolderAction) -> JournalFolderV2 {
        JournalFolderV2 {
            name: name.into(),
            key: key(name),
            action,
            expected_live: ExpectedLive::Absent,
            staged: (action != FolderAction::Remove).then(|| sha256(name.as_bytes())),
            recovery_slot: RecoverySlot::new(&format!("recovery/{name}")).unwrap(),
        }
    }

    fn unit(folder: &str) -> SettingsUnit {
        SettingsUnit {
            folder: key(folder),
            identity: Identity::account("ACC").unwrap(),
        }
    }

    fn settings_op(folder: &str) -> SettingsFileOp {
        SettingsFileOp {
            unit: unit(folder),
            live_rel: crate::paths::account_settings_path("ACC", folder).unwrap(),
            expected_live: ExpectedLive::Absent,
            staged: Some(sha256(b"s")),
            recovery_slot: RecoverySlot::new(&format!("recovery/s-{folder}")).unwrap(),
        }
    }

    fn j2(kind: OpKind, actions: &[FolderAction], removals: usize, ops: bool) -> ApplyJournalV2 {
        let mut rem: Vec<Sha256> = (0..removals as u8).map(|n| sha256(&[n])).collect();
        rem.sort();
        ApplyJournalV2 {
            op: op(9),
            group_id: group_id(),
            started_unix_ms: 5,
            kind,
            package: key("suite"),
            target_hash: None,
            phase: JournalPhase::Staging,
            from_phase: None,
            reason: None,
            halted_by_game: false,
            folders: actions
                .iter()
                .enumerate()
                .map(|(i, a)| folder2(&format!("f{i}"), *a))
                .collect(),
            removals: rem,
            pending_records: Vec::new(),
            settings_ops: if ops {
                vec![settings_op("suite")]
            } else {
                vec![]
            },
        }
    }

    fn accepts(j: &ApplyJournalV2) -> bool {
        decode::<ApplyJournalV2>(&encode(j), ApplyJournalV2::LIMIT).is_ok()
    }

    #[test]
    fn journal_kind_action_table() {
        // T-R4D1-jrn-kind-actions
        use FolderAction::{Install as I, Remove as R, Replace as P};
        use OpKind::*;
        let cases: Vec<(ApplyJournalV2, bool)> = vec![
            (j2(Apply, &[I, P], 0, false), true),
            (j2(Apply, &[P, R], 1, false), true),
            (j2(Apply, &[P, R], 0, false), false),
            (j2(Apply, &[I], 1, false), false),
            (j2(Remove, &[R], 1, false), true),
            (j2(Remove, &[R, I], 1, false), false),
            (j2(Remove, &[R], 0, false), false),
            (j2(Remove, &[], 1, false), false),
            (j2(Restore, &[I, P], 0, false), true),
            (j2(Restore, &[R], 0, false), false),
            (j2(Restore, &[P], 1, false), false),
            (j2(Rollback, &[P, R], 1, false), true),
            (j2(Rollback, &[R], 0, false), false),
            (j2(Settings, &[], 0, true), true),
            (j2(Settings, &[], 0, false), false),
            (j2(Settings, &[I], 0, true), false),
            (j2(Settings, &[], 1, true), false),
            (j2(SettingsCleanup, &[], 0, true), true),
            (j2(SettingsCleanup, &[R], 0, true), false),
            (j2(Apply, &[], 0, true), true),
            (j2(Apply, &[], 0, false), false),
        ];
        for (i, (j, want)) in cases.iter().enumerate() {
            assert_eq!(accepts(j), *want, "case {i}: {:?} {:?}", j.kind, j.folders);
        }
    }

    #[test]
    fn v1_journal_with_a_remove_action_is_rejected() {
        let mut j = sample_journal(JournalPhase::Committing);
        j.folders[1].action = FolderAction::Remove;
        assert!(decode::<ApplyJournal>(&encode(&j), ApplyJournal::LIMIT).is_err());
    }

    #[test]
    fn dropping_a_folder_is_not_v1_expressible() {
        // T-R4D1-mixed-update-drop (shape), T-C05
        use FolderAction::{Remove as R, Replace as P};
        let pure = j2(OpKind::Apply, &[P], 0, false);
        let mut pure = pure;
        pure.target_hash = Some(sha256(b"t"));
        assert!(pure.is_v1_expressible());
        let mixed = j2(OpKind::Apply, &[P, R], 1, false);
        assert!(!mixed.is_v1_expressible());
        assert!(accepts(&mixed));
    }

    #[test]
    fn slice_decoder_reports_major_2_and_new_decoder_reads_both() {
        // T-C06-v2-under-slice
        let j = j2(OpKind::Settings, &[], 0, true);
        let bytes = encode(&j);
        assert!(matches!(
            decode::<ApplyJournal>(&bytes, ApplyJournal::LIMIT),
            Err(RecordError::UnsupportedMajor(2))
        ));
        assert!(matches!(decode_journal(&bytes), Ok(AnyJournal::V2(_))));
        let v1 = encode(&sample_journal(JournalPhase::Staging));
        assert!(matches!(decode_journal(&v1), Ok(AnyJournal::V1(_))));
    }

    #[test]
    fn pending_report_v2_is_replaced_by_the_slice_and_never_enters_a_state() {
        // T-C07-pending-report-v2
        use crate::testkit::{computer, enrollment};
        let v2 = PendingReportV2 {
            group_id: group_id(),
            enrollment_id: enrollment(1),
            report: None,
            outcomes: vec![LocalOutcome {
                package: key("suite"),
                kind: LocalOutcomeKind::Removed,
            }],
        };
        let bytes = encode(&v2);
        assert!(matches!(
            decode::<PendingReport>(&bytes, PendingReport::LIMIT),
            Err(RecordError::UnsupportedMajor(2))
        ));
        assert_eq!(decode::<PendingReportV2>(&bytes, MIB).unwrap(), v2);
        // The slice report is still major 1 and round-trips unchanged.
        let v1 = PendingReport {
            group_id: group_id(),
            enrollment_id: enrollment(1),
            report: Some(ApplyReport {
                op: op(1),
                basis: vec![BasisEntry {
                    computer_id: computer(1),
                    seq: 1,
                }],
                results: vec![ReportResult {
                    package: key("suite"),
                    result: ReportKind::Applied,
                    package_hash: None,
                }],
            }),
        };
        let b1 = encode(&v1);
        assert!(String::from_utf8_lossy(&b1).contains("pending-report/1"));
        assert_eq!(decode::<PendingReport>(&b1, MIB).unwrap(), v1);
        // No new outcome kind can be written as a slice report kind, so an old peer's decoder
        // never sees one in `code-state/1.apply_report`.
        for k in [
            "removed",
            "absent_acknowledged",
            "restored",
            "settings_applied",
            "settings_kept_local",
        ] {
            assert!(serde_json::from_str::<ReportKind>(&format!("\"{k}\"")).is_err());
        }
    }

    #[test]
    fn journal_v1_and_v2_golden_bytes() {
        // T-C05-journal-v1-golden, T-C01
        golden(
            "apply-journal-1.json",
            &encode(&sample_journal(JournalPhase::Staging)),
        );
        golden(
            "apply-journal-2-mixed.json",
            &encode(&j2(
                OpKind::Apply,
                &[FolderAction::Replace, FolderAction::Remove],
                1,
                true,
            )),
        );
        golden(
            "apply-journal-2-settings.json",
            &encode(&j2(OpKind::Settings, &[], 0, true)),
        );
    }

    fn captured(folder: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(folder),
            state: RootState::Captured {
                checkpoint: sha256(b"cp"),
                location: Location::Local,
            },
        }
    }

    fn missing(folder: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(folder),
            state: RootState::Missing {
                reason: MissingReason::NotIncluded,
            },
        }
    }

    #[test]
    fn paired_is_derived_for_every_row() {
        // T-R4D1-paired-derived
        assert_eq!(derive_paired(true, &[]), Some(Paired::NotApplicable));
        assert_eq!(derive_paired(true, &[captured("a")]), Some(Paired::Paired));
        assert_eq!(
            derive_paired(true, &[captured("a"), missing("b")]),
            Some(Paired::Partial)
        );
        assert_eq!(derive_paired(true, &[missing("a")]), Some(Paired::CodeOnly));
        assert_eq!(
            derive_paired(false, &[captured("a")]),
            Some(Paired::SettingsOnly)
        );
        assert_eq!(derive_paired(false, &[missing("a")]), None);
        assert_eq!(derive_paired(false, &[]), None);

        let mut p = RecoveryPointV2 {
            op: op(9),
            group_id: group_id(),
            kind: OpKind::Apply,
            package: key("suite"),
            code_package_hash: Some(sha256(b"t")),
            created_unix_ms: 1,
            replaced: vec![TreeRef {
                name: "Suite".into(),
                tree: sha256(b"old"),
            }],
            installed: vec![],
            removals: vec![],
            settings: vec![captured("a")],
            paired: Paired::Paired,
            reason: "apply".into(),
            pinned: true,
        };
        let ok = |p: &RecoveryPointV2| {
            decode::<RecoveryPointV2>(&encode(p), RecoveryPointV2::LIMIT).is_ok()
        };
        assert!(ok(&p));
        for wrong in [
            Paired::NotApplicable,
            Paired::Partial,
            Paired::CodeOnly,
            Paired::SettingsOnly,
        ] {
            p.paired = wrong;
            assert!(!ok(&p), "{wrong:?}");
        }
        golden(
            "recovery-point-2-paired.json",
            &encode(&RecoveryPointV2 {
                paired: Paired::Paired,
                ..p
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::TocSummary;
    use crate::testkit;

    fn code_state() -> CodeState {
        testkit::member_state(
            &testkit::computer(1),
            3,
            &testkit::inventory(vec![testkit::addon("Suite", Some("1.0"), "a")]),
        )
    }

    #[test]
    fn authority_index_and_features_are_additive_and_old_readers_ignore_them() {
        // contracts.md §1.6, §1.13 downgrade: the slice shape stays byte-identical
        let slice = code_state();
        let slice_bytes = encode(&slice);
        let text = String::from_utf8(slice_bytes.clone()).unwrap();
        assert!(!text.contains("authority_index") && !text.contains("features"));
        assert_eq!(decode::<CodeState>(&slice_bytes, MIB).unwrap(), slice);

        let mut modern = slice.clone();
        modern.authority_index = Some(AuthorityIndexHead {
            head: Sha256([5; 32]),
            total: 12,
        });
        modern.features = FEATURES.iter().map(|f| f.to_string()).collect();
        let bytes = encode(&modern);
        assert_eq!(decode::<CodeState>(&bytes, 16 * MIB).unwrap(), modern);

        #[derive(Deserialize)]
        struct SliceReader {
            seq: u64,
            folders: Vec<serde_json::Value>,
        }
        let old: SliceReader = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(old.seq, 3);
        assert_eq!(old.folders.len(), 1);
    }

    #[test]
    fn round_trip_all_schemas() {
        fn rt<T: Record + PartialEq + std::fmt::Debug>(r: T) {
            let bytes = encode(&r);
            assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
            let text = String::from_utf8(bytes.clone()).unwrap();
            assert!(
                text.starts_with(&format!("{{\n  \"schema\": \"{}/1\",", T::SCHEMA)),
                "{text}"
            );
            assert_eq!(decode::<T>(&bytes, T::LIMIT).unwrap(), r);
        }
        let s = code_state();
        rt(RootFormat { created_unix_ms: 1 });
        rt(testkit::group_record());
        rt(testkit::member_record(&testkit::computer(1)));
        let mut with_report = s.clone();
        with_report.apply_report = Some(ApplyReport {
            op: testkit::op(9),
            basis: vec![BasisEntry {
                computer_id: testkit::computer(2),
                seq: 4,
            }],
            results: vec![ReportResult {
                package: FolderKey::lossy("suite"),
                result: ReportKind::Applied,
                package_hash: Some(Sha256([7; 32])),
            }],
        });
        rt(s);
        rt(with_report);
        rt(FolderTreeRecord {
            tree: tree_hash(&[]),
            files: vec![],
        });
        rt(testkit::sample_journal(JournalPhase::Committing));
        rt(RecoveryPoint {
            op: testkit::op(1),
            package: FolderKey::lossy("suite"),
            created_unix_ms: 5,
            replaced: vec![TreeRef {
                name: "Suite".into(),
                tree: Sha256([1; 32]),
            }],
            installed: vec![],
            reason: "apply".into(),
            pinned: true,
        });
        rt(LocalComputer {
            computer_id: testkit::computer(1),
            display_name: "PC".into(),
            created_unix_ms: 1,
        });
        rt(LocalEnrollment {
            group_id: testkit::group_id(),
            enrollment_id: testkit::enrollment(1),
            shared_root: "S".into(),
            install_root: "I".into(),
            variant_folder: "_forever_".into(),
            product: "wowf".into(),
            validation: testkit::confirmed_forever().validation().clone(),
            next_seq: 4,
        });
        let mut marks = AdmissionWatermarks::empty(testkit::group_id());
        marks.members.push(Watermark {
            computer_id: testkit::computer(1),
            enrollment_id: testkit::enrollment(1),
            max_seq_seen: 3,
            admitted_seq: Some(3),
        });
        rt(marks);
        rt(AuditEntry {
            at_unix_ms: 1,
            event: "publish".into(),
            package: None,
            detail: "seq 3".into(),
        });
    }

    #[test]
    fn unknown_major_refused() {
        let text = String::from_utf8(encode(&testkit::group_record())).unwrap();
        let v2 = text.replace("wowsync.group/1", "wowsync.group/2");
        assert_eq!(
            decode::<GroupRecord>(v2.as_bytes(), MIB),
            Err(RecordError::UnsupportedMajor(2))
        );
        let other = text.replace("wowsync.group/1", "wowsync.member/1");
        assert!(matches!(
            decode::<GroupRecord>(other.as_bytes(), MIB),
            Err(RecordError::UnknownSchema(_))
        ));
        let none = text.replace("\"schema\"", "\"scheme\"");
        assert_eq!(
            decode::<GroupRecord>(none.as_bytes(), MIB),
            Err(RecordError::SchemaMissing)
        );
    }

    #[test]
    fn unknown_fields_ignored_within_major() {
        let text = String::from_utf8(encode(&testkit::group_record())).unwrap();
        let extra = text.replacen("{\n", "{\n  \"future_field\": {\"a\": 1},\n", 1);
        assert_eq!(
            decode::<GroupRecord>(extra.as_bytes(), MIB).unwrap(),
            testkit::group_record()
        );
    }

    #[test]
    fn oversized_record_refused_before_parse() {
        let mut bytes = encode(&testkit::group_record());
        assert!(decode::<GroupRecord>(&bytes, GroupRecord::LIMIT).is_ok());
        bytes.extend(std::iter::repeat_n(b' ', GroupRecord::LIMIT));
        assert_eq!(
            decode::<GroupRecord>(&bytes, GroupRecord::LIMIT),
            Err(RecordError::TooLarge)
        );
        // Garbage that is also oversized reports size, not a parse error.
        assert_eq!(
            decode::<GroupRecord>(&[b'{'; 100], 10),
            Err(RecordError::TooLarge)
        );
    }

    #[test]
    fn state_for_other_group_or_product_rejected() {
        let group = testkit::group_record();
        let s = code_state();
        let (c, e) = (s.computer_id.clone(), s.enrollment_id.clone());
        assert!(validate_state_for(&s, &group, &c, &e, 3).is_ok());
        let mut other_group = group.clone();
        other_group.group_id = GroupId::parse(&"ab".repeat(16)).unwrap();
        assert!(validate_state_for(&s, &other_group, &c, &e, 3).is_err());
        let mut other_product = group.clone();
        other_product.product = "wow_test".into();
        assert!(validate_state_for(&s, &other_product, &c, &e, 3).is_err());
    }

    #[test]
    fn state_name_must_match_body() {
        let group = testkit::group_record();
        let s = code_state();
        let (c, e) = (s.computer_id.clone(), s.enrollment_id.clone());
        assert!(validate_state_for(&s, &group, &c, &e, 4).is_err());
        assert!(validate_state_for(&s, &group, &testkit::computer(2), &e, 3).is_err());
        assert!(validate_state_for(&s, &group, &c, &testkit::enrollment(9), 3).is_err());
    }

    #[test]
    fn tree_record_hash_recomputed_on_read() {
        let inv = testkit::inventory(vec![testkit::addon("Suite", Some("1.0"), "a")]);
        let f = &inv.folders[0];
        let rec = FolderTreeRecord {
            tree: f.tree.unwrap(),
            files: f.file_entries.clone(),
        };
        let bytes = encode(&rec);
        assert_eq!(decode_tree(&bytes, &rec.tree).unwrap(), rec);
        assert!(decode_tree(&bytes, &Sha256([9; 32])).is_err());
        let tampered = FolderTreeRecord {
            tree: Sha256([9; 32]),
            files: rec.files.clone(),
        };
        assert!(matches!(
            decode_tree(&encode(&tampered), &tampered.tree),
            Err(RecordError::Invalid(_))
        ));
        let mut content_changed = rec.clone();
        content_changed.files[0].sha256 = Sha256([5; 32]);
        assert!(decode_tree(&encode(&content_changed), &rec.tree).is_err());
    }

    #[test]
    fn tree_with_unsafe_path_rejected() {
        let inv = testkit::inventory(vec![testkit::addon("Suite", Some("1.0"), "a")]);
        let rec = FolderTreeRecord {
            tree: inv.folders[0].tree.unwrap(),
            files: inv.folders[0].file_entries.clone(),
        };
        let text = String::from_utf8(encode(&rec)).unwrap();
        for bad in ["../evil.lua", "C:/x", "a/../b", "CON.lua", "/abs"] {
            let t = text.replace("main.lua", bad);
            assert!(decode_tree(t.as_bytes(), &rec.tree).is_err(), "{bad}");
        }
        let _ = TocSummary::empty(crate::inventory::TocKind::None);
    }
}
