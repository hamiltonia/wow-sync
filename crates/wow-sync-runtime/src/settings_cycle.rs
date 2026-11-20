//! The settings cycle bodies that are independent of the service skeleton (settings.md §10.2):
//! observing the scope on disk, capturing a unit into the local store, and composing a settings
//! apply for the existing executor. Bytes are only hashed and copied, never parsed or executed.

#![allow(dead_code)] // called by the cycle in a later step; the unit tests exercise it now

use std::collections::BTreeSet;
use std::path::Path;

use wow_sync_adapters::fs_safe::WtfScope;
use wow_sync_adapters::settings_store::SettingsStore;
use wow_sync_adapters::wtf::scan_wtf;
use wow_sync_adapters::FsError;
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{CheckpointRef, OpId, SettingsUnit, Sha256};
use wow_sync_core::plan::{ExpectedLive, PackageApply};
use wow_sync_core::records::{
    encode, CheckpointFile, Location, OpKind, RecoverySlot, RootState, SettingsCheckpointManifest,
    SettingsFileOp, SettingsRoot,
};
use wow_sync_core::settings::{
    settings_scope, DeclaringFolder, OwnedFile, SettingsScope, WtfEntry,
};

#[derive(Debug)]
pub enum CaptureError {
    Fs(FsError),
    /// The file changed between the scan and the copy: the capture is suspect, nothing is stored.
    Changed,
    Manifest(String),
}

impl From<FsError> for CaptureError {
    fn from(e: FsError) -> Self {
        Self::Fs(e)
    }
}

/// Step 3s: the settings scope of one variant. Nothing outside the two shapes is read.
pub fn observe_scope(
    variant_dir: &Path,
    decls: &[DeclaringFolder],
    included: &BTreeSet<String>,
    size_cap: Option<u64>,
) -> Result<SettingsScope, FsError> {
    let files = scan_wtf(variant_dir, included, size_cap)?;
    let entries: Vec<WtfEntry> = files
        .into_iter()
        .map(|f| WtfEntry {
            rel: f.rel,
            size: f.size,
            sha256: Sha256(f.sha256),
            unsafe_path: f.unsafe_path,
        })
        .collect();
    let nfc_included: BTreeSet<String> = included
        .iter()
        .map(|s| wow_sync_core::ids::nfc(s))
        .collect();
    Ok(settings_scope(decls, &entries, &nfc_included, size_cap))
}

/// Stores the unit's file as a blob and a one-file manifest; the manifest is written only after
/// the blob, so a failure never leaves a manifest without its bytes. The checkpoint reference is
/// the manifest's hash, so identical captures are not stored twice.
pub fn capture_unit(
    variant_dir: &Path,
    store: &SettingsStore,
    owned: &OwnedFile,
    size_cap: u64,
) -> Result<CheckpointRef, CaptureError> {
    let scope = WtfScope::open(variant_dir, false)?
        .ok_or_else(|| CaptureError::Manifest("WTF is missing".into()))?;
    let root = scope.root();
    let mut file = root.open_read(owned.rel.as_str(), size_cap)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut bytes)
        .map_err(|e| FsError::from_io(&root.path().join(owned.rel.as_str()), &e))?;
    if sha256(&bytes) != owned.sha256 || bytes.len() as u64 != owned.size {
        return Err(CaptureError::Changed);
    }
    let blob = store.put_blob(&bytes)?;
    let name = owned
        .rel
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    let manifest = SettingsCheckpointManifest {
        unit: owned.unit.clone(),
        files: vec![CheckpointFile {
            name,
            size: owned.size,
            blob: Sha256(blob),
        }],
    };
    let encoded = encode(&manifest);
    let h = store.put_manifest(&encoded, &[blob])?;
    Ok(Sha256(h))
}

/// A settings-only apply for the existing executor: one file, `expected_live` from the scan, the
/// blob to stage (`None` removes the live file) and one recovery slot. The checkpoint is the root.
pub fn plan_settings_apply(
    op: OpId,
    owned_now: Option<&OwnedFile>,
    unit: SettingsUnit,
    rel: wow_sync_core::paths::SettingsPath,
    staged_blob: Option<Sha256>,
    checkpoint: CheckpointRef,
    explanation: wow_sync_core::explain::Explanation,
) -> PackageApply {
    let mut p =
        PackageApply::code_apply(op, unit.folder.clone(), checkpoint, Vec::new(), explanation);
    p.kind = OpKind::Settings;
    p.settings_ops = vec![SettingsFileOp {
        unit: unit.clone(),
        live_rel: rel,
        expected_live: match owned_now {
            Some(o) => ExpectedLive::Tree {
                tree: o.sha256,
                fingerprint: o.sha256,
            },
            None => ExpectedLive::Absent,
        },
        staged: staged_blob,
        recovery_slot: RecoverySlot::new("settings/0").expect("fixed slot name"),
    }];
    p.settings_roots = vec![SettingsRoot {
        unit,
        state: RootState::Captured {
            checkpoint,
            location: Location::Local,
        },
    }];
    p
}

/// What the cycle remembers about one unit beyond `SettingsLocal`: whether the authority epoch is
/// clean, the sequence of the last capture, and why that capture was taken (settings.md §5).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnitEpoch {
    pub unit: SettingsUnit,
    pub epoch_clean: bool,
    pub capture_seq: u64,
    pub capture_kind: Option<wow_sync_core::settings::CaptureKind>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct EpochFile {
    units: Vec<UnitEpoch>,
}

fn epoch_name(group: &wow_sync_core::ids::GroupId) -> String {
    format!("settings-epoch-{group}.json")
}

/// Persisted in local state so a restart keeps an experiment held rather than publishing it.
pub struct EpochStore<'a> {
    state: &'a wow_sync_adapters::localstate::LocalState,
    group: wow_sync_core::ids::GroupId,
}

impl<'a> EpochStore<'a> {
    pub fn new(
        state: &'a wow_sync_adapters::localstate::LocalState,
        group: wow_sync_core::ids::GroupId,
    ) -> Self {
        Self { state, group }
    }

    /// Missing or damaged state means no unit is known clean: nothing publishes until a fresh
    /// activation capture proves it.
    pub fn load(&self) -> Vec<UnitEpoch> {
        let file: EpochFile = self
            .state
            .read(&epoch_name(&self.group), 4 * 1024 * 1024)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut seen = BTreeSet::new();
        file.units
            .into_iter()
            .filter(|u| seen.insert(u.unit.clone()))
            .collect()
    }

    pub fn save(&self, units: &[UnitEpoch]) -> Result<(), FsError> {
        let bytes = serde_json::to_vec(&EpochFile {
            units: units.to_vec(),
        })
        .expect("plain data serializes");
        self.state.write_atomic(&epoch_name(&self.group), &bytes)
    }
}

/// One revision of a unit's settings history (the `SettingsHistory` query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsHistoryEntry {
    pub hash: wow_sync_core::ids::Sha256,
    pub kind: wow_sync_core::kinds::RevisionKind,
    pub checkpoint: wow_sync_core::ids::CheckpointRef,
    pub issuer: wow_sync_core::kinds::Issuer,
    pub written_unix_ms: u64,
    /// Currently a head of the unit's graph: nothing newer builds on it.
    pub is_head: bool,
}

/// The valid revisions of `unit`, oldest first. Invalid and unreadable ones are not history.
pub fn settings_history(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    unit: &SettingsUnit,
) -> Vec<SettingsHistoryEntry> {
    use wow_sync_core::authority::AuthorityRecord;
    let graph = wow_sync_core::settings::graph_from_ledger(ledger, derived, unit);
    let mut out: Vec<SettingsHistoryEntry> = graph
        .nodes
        .keys()
        .filter_map(|h| match ledger.record(h) {
            Some(AuthorityRecord::SettingsRevision(r)) => Some(SettingsHistoryEntry {
                hash: *h,
                kind: r.kind,
                checkpoint: r.checkpoint,
                issuer: r.issuer.clone(),
                written_unix_ms: r.written_unix_ms,
                is_head: graph.heads.contains(h),
            }),
            _ => None,
        })
        .collect();
    out.sort_by_key(|e| (e.written_unix_ms, e.hash));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::explain::Explanation;
    use wow_sync_core::ids::{ComputerId, Identity};
    use wow_sync_core::layout;
    use wow_sync_core::paths::validate_folder_name;

    fn hex(c: char) -> String {
        std::iter::repeat_n(c, 32).collect()
    }

    fn put(base: &Path, rel: &str, bytes: &[u8]) {
        let p = rel.split('/').fold(base.to_path_buf(), |a, s| a.join(s));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn decls() -> Vec<DeclaringFolder> {
        vec![DeclaringFolder {
            folder: validate_folder_name("Details").unwrap(),
            saved_variables: vec!["DetailsDB".into()],
            saved_variables_per_character: vec![],
            toc_ambiguous: false,
        }]
    }

    fn included() -> BTreeSet<String> {
        ["ERIC".to_string()].into()
    }

    #[test]
    fn the_epoch_state_survives_a_restart_and_unreadable_state_is_not_clean() {
        use wow_sync_core::settings::CaptureKind;
        let t = TempDir::new();
        let state = wow_sync_adapters::localstate::LocalState::open_at(t.path()).unwrap();
        let g = wow_sync_core::ids::GroupId::parse(&hex('9')).unwrap();
        let unit = SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Details").unwrap(),
            identity: Identity::account("ERIC").unwrap(),
        };
        let units = vec![UnitEpoch {
            unit: unit.clone(),
            epoch_clean: false,
            capture_seq: 41,
            capture_kind: Some(CaptureKind::BeforeChange),
        }];
        let store = EpochStore::new(&state, g.clone());
        assert!(store.load().is_empty());
        store.save(&units).unwrap();
        assert_eq!(EpochStore::new(&state, g.clone()).load(), units);
        let other = wow_sync_core::ids::GroupId::parse(&hex('8')).unwrap();
        assert!(EpochStore::new(&state, other).load().is_empty());
        state
            .write_atomic(&epoch_name(&g), b"{\"units\": [ truncated")
            .unwrap();
        assert!(
            store.load().is_empty(),
            "damaged state claims nothing clean"
        );
    }

    #[test]
    fn the_scope_reads_only_owned_files_and_a_capture_stores_blob_then_manifest() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(
            &wtf,
            "Account/ERIC/SavedVariables/Details.lua",
            b"DetailsDB = {}",
        );
        put(&wtf, "Account/ERIC/config-cache.wtf", b"cfg");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(1024)).unwrap();
        assert_eq!(scope.owned.len(), 1);
        assert!(scope.sync_enabled);
        let store = SettingsStore::open(t.path()).unwrap();
        let cp = capture_unit(t.path(), &store, &scope.owned[0], 1024).unwrap();
        let manifest = t
            .path()
            .join(".wowsync")
            .join("settings")
            .join("manifests")
            .join(format!("{cp}.json"));
        assert!(manifest.exists());
        assert_eq!(
            layout::settings_manifest(&cp),
            format!("settings/manifests/{cp}.json")
        );
        let blob = sha256(b"DetailsDB = {}");
        let blob_path = t
            .path()
            .join(".wowsync")
            .join("settings")
            .join("blobs")
            .join(blob.to_string());
        assert_eq!(std::fs::read(&blob_path).unwrap(), b"DetailsDB = {}");
        // identical bytes give the same checkpoint and store nothing new
        let again = capture_unit(t.path(), &store, &scope.owned[0], 1024).unwrap();
        assert_eq!(again, cp);
        // the live file was never changed by capturing
        assert_eq!(
            std::fs::read(
                wtf.join("Account")
                    .join("ERIC")
                    .join("SavedVariables")
                    .join("Details.lua")
            )
            .unwrap(),
            b"DetailsDB = {}"
        );
    }

    #[test]
    fn without_a_cap_nothing_syncs_and_a_file_changed_after_the_scan_is_not_stored() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let off = observe_scope(
            t.path(),
            &decls(),
            &included(),
            wow_sync_core::records::MAX_SETTINGS_FILE_BYTES,
        )
        .unwrap();
        assert!(!off.sync_enabled);
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        put(
            &wtf,
            "Account/ERIC/SavedVariables/Details.lua",
            b"v2 changed",
        );
        let store = SettingsStore::open(t.path()).unwrap();
        let err = capture_unit(t.path(), &store, &scope.owned[0], 64).unwrap_err();
        assert!(matches!(err, CaptureError::Changed), "{err:?}");
        let blobs = t.path().join(".wowsync").join("settings").join("blobs");
        assert!(!blobs.exists() || std::fs::read_dir(blobs).unwrap().next().is_none());
    }

    #[test]
    fn a_failing_store_records_no_state() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        // a plain file where the blob directory must be makes the store write fail
        put(&t.path().join(".wowsync"), "settings/blobs", b"in the way");
        let store = SettingsStore::open(t.path()).unwrap();
        assert!(capture_unit(t.path(), &store, &scope.owned[0], 64).is_err());
        let manifests = t.path().join(".wowsync").join("settings").join("manifests");
        assert!(!manifests.exists());
    }

    #[test]
    fn the_apply_plan_is_settings_only_with_the_scanned_expectation() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua", b"v1");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(64)).unwrap();
        let o = &scope.owned[0];
        let target = sha256(b"v2");
        let p = plan_settings_apply(
            OpId::parse(&hex('7')).unwrap(),
            Some(o),
            o.unit.clone(),
            o.rel.clone(),
            Some(target),
            sha256(b"checkpoint"),
            Explanation::Installed {
                source: ComputerId::parse(&hex('3')).unwrap(),
            },
        );
        assert_eq!(p.kind, OpKind::Settings);
        assert!(p.folders.is_empty() && p.removes.is_empty());
        assert_eq!(p.settings_ops.len(), 1);
        assert_eq!(p.settings_ops[0].staged, Some(target));
        assert_eq!(
            p.settings_ops[0].expected_live,
            ExpectedLive::Tree {
                tree: o.sha256,
                fingerprint: o.sha256
            }
        );
        assert_eq!(
            p.settings_ops[0].unit.identity,
            Identity::account("ERIC").unwrap()
        );
        assert!(p.settings_roots[0].captured());
        let absent = plan_settings_apply(
            OpId::parse(&hex('8')).unwrap(),
            None,
            o.unit.clone(),
            o.rel.clone(),
            Some(target),
            sha256(b"checkpoint"),
            Explanation::Installed {
                source: ComputerId::parse(&hex('3')).unwrap(),
            },
        );
        assert_eq!(absent.settings_ops[0].expected_live, ExpectedLive::Absent);
    }
}
