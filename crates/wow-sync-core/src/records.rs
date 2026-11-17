//! Record schemas, encoding and decoding (publication.md §2).

use crate::hash::tree_hash;
use crate::ids::{ComputerId, EnrollmentId, GroupId, OpId, Sha256};
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
    const MAJOR: u32;
    const LIMIT: usize;
    fn validate(&self) -> Result<(), String>;
}

/// Pretty JSON, LF, trailing newline, `schema` first.
pub fn encode<T: Record>(r: &T) -> Vec<u8> {
    let body = serde_json::to_string_pretty(r).expect("records serialize");
    let rest = body.strip_prefix("{\n").expect("records are objects");
    let mut out = format!(
        "{{\n  \"schema\": \"{}/{}\",\n{}",
        T::SCHEMA,
        T::MAJOR,
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
    match major.parse::<u32>() {
        Ok(m) if m == T::MAJOR => {}
        Ok(m) => return Err(RecordError::UnsupportedMajor(m)),
        Err(_) => return Err(RecordError::UnknownSchema(schema)),
    }
    let rec: T = serde_json::from_value(value).map_err(|e| RecordError::Json(e.to_string()))?;
    rec.validate().map_err(RecordError::Invalid)?;
    Ok(rec)
}

macro_rules! record {
    ($t:ty, $schema:literal, $limit:expr, |$s:ident| $validate:block) => {
        impl Record for $t {
            const SCHEMA: &'static str = $schema;
            const MAJOR: u32 = 1;
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
}
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
