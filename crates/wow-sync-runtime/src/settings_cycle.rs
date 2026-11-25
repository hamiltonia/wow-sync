//! The settings cycle bodies that are independent of the service skeleton (settings.md §10.2):
//! observing the scope on disk, capturing a unit into the local store, and composing a settings
//! apply for the existing executor. Bytes are only hashed and copied, never parsed or executed.

#![allow(dead_code)] // called by the cycle in a later step; the unit tests exercise it now

use std::collections::BTreeSet;
use std::path::Path;

use crate::decisions::{write_own_record, Plane};
use wow_sync_adapters::authority_store;
use wow_sync_adapters::fs_safe::WtfScope;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_adapters::settings_store::SettingsStore;
use wow_sync_adapters::sysinfo::{now_unix_ms, random_id_hex};
use wow_sync_adapters::wtf::scan_wtf;
use wow_sync_adapters::FsError;
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{CheckpointRef, ComputerId, GroupId, OpId, SettingsUnit, Sha256};
use wow_sync_core::kinds::{EnrollmentRef, Issuer};
use wow_sync_core::layout;
use wow_sync_core::ledger::derive;
use wow_sync_core::plan::{ExpectedLive, PackageApply};
use wow_sync_core::records::{
    decode, encode, CheckpointFile, KeptCopy, LocalUnitState, Location, OpKind, RecoverySlot,
    RootState, SettingsCheckpointManifest, SettingsFileOp, SettingsLocal, SettingsRoot,
};
use wow_sync_core::settings::{
    classify_capture, settings_scope, CaptureClass, DeclaringFolder, FileFact, LocalReason,
    OwnedFile, SettingsScope, SettleScans, WtfEntry,
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
    match store.put_blob_stream(&mut file, owned.sha256.0, owned.size) {
        Ok(()) => {}
        Err(e) if e.kind == wow_sync_adapters::FsErrorKind::HashMismatch => {
            return Err(CaptureError::Changed)
        }
        Err(e) => return Err(e.into()),
    }
    let blob = owned.sha256.0;
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

/// What a peer's captured checkpoint of one settings file is made of (single-file units).
pub(crate) fn manifest_blob(
    provider: &ProviderRoot,
    checkpoint: &CheckpointRef,
    unit: &SettingsUnit,
) -> Option<Sha256> {
    manifest_file(provider, checkpoint, unit).map(|(blob, _)| blob)
}

/// The file's content hash and size in a checkpoint of a single-file unit.
pub(crate) fn manifest_file(
    provider: &ProviderRoot,
    checkpoint: &CheckpointRef,
    unit: &SettingsUnit,
) -> Option<(Sha256, u64)> {
    let bytes = provider
        .read(&layout::settings_manifest(checkpoint), 64 * 1024)
        .ok()?;
    if sha256(&bytes) != *checkpoint {
        return None;
    }
    let m: SettingsCheckpointManifest = decode(&bytes, 64 * 1024).ok()?;
    if &m.unit != unit {
        return None;
    }
    let first = m.files.first()?;
    let (blob, size) = (first.blob, first.size);
    provider
        .open_read(&layout::settings_blob(&blob), BLOB_LIMIT)
        .ok()?;
    Some((blob, size))
}

/// Whether the group's single current settings policy names the revision's issuer as master.
pub(crate) fn issued_by_master(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    r: &wow_sync_core::kinds::SettingsRevision,
) -> bool {
    match wow_sync_core::settings::policy_heads(ledger, derived).as_slice() {
        [one] => one
            .master
            .as_ref()
            .is_some_and(|m| m.enrollment_id == r.issuer.enrollment_id),
        _ => false,
    }
}

/// The unit-level version of a checkpoint (its id) and the total size of all its files.
pub(crate) fn manifest_unit(
    provider: &ProviderRoot,
    checkpoint: &CheckpointRef,
    unit: &SettingsUnit,
) -> Option<(Sha256, u64)> {
    let bytes = provider
        .read(&layout::settings_manifest(checkpoint), 64 * 1024)
        .ok()?;
    if sha256(&bytes) != *checkpoint {
        return None;
    }
    let m: SettingsCheckpointManifest = decode(&bytes, 64 * 1024).ok()?;
    if &m.unit != unit || m.files.is_empty() {
        return None;
    }
    Some((*checkpoint, m.files.iter().map(|f| f.size).sum()))
}

/// What one settings change replaced and brought, for its history record.
#[derive(Debug, Clone)]
pub(crate) struct SettingsChangeInfo {
    pub old: Option<(Sha256, u64)>,
    pub new: (Sha256, u64),
    /// "account-wide" or the character, in plain words. Shown in the Activity detail only.
    pub identity: String,
}

/// What a written revision replaced and brought, for its history record.
pub(crate) fn revision_info(
    ledger: &wow_sync_core::ledger::Ledger,
    provider: &ProviderRoot,
    r: &wow_sync_core::kinds::SettingsRevision,
) -> Option<SettingsChangeInfo> {
    let new = manifest_unit(provider, &r.checkpoint, &r.unit)?;
    let old = r.parents.iter().find_map(|p| match ledger.record(p) {
        Some(AuthorityRecord::SettingsRevision(pr)) => {
            manifest_unit(provider, &pr.checkpoint, &r.unit)
        }
        _ => None,
    });
    Some(SettingsChangeInfo {
        old,
        new,
        identity: identity_text(&r.unit),
    })
}

pub(crate) fn identity_text(unit: &SettingsUnit) -> String {
    match unit.identity.realm_character() {
        None => "account-wide".into(),
        Some((realm, ch)) => format!("{ch} on {realm}"),
    }
}

/// Why units of this computer are not syncing, as of the last cycle. Never logged.
#[derive(Debug, Clone, Default)]
pub struct SettingsFacts {
    /// The game was running at the last cycle: settings were neither read nor replaced.
    pub wow_running: bool,
    pub units: std::collections::BTreeMap<SettingsUnit, UnitFact>,
    /// Older shared states whose saved copy is gone from the shared folder and from this
    /// computer (cleanup removed it): they are not offered for restore.
    pub unavailable: BTreeSet<CheckpointRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitFact {
    /// The account or character folder is missing here; the identity in plain words.
    Waiting(String),
    TooLarge(u64),
    /// The other add-ons that declare the same saved variables.
    Several(Vec<String>),
    /// The file looks incomplete (empty, vanished next to a `.bak`, or still changing): it is
    /// neither captured nor published nor replaced until a later look finds it complete.
    Suspect,
    /// This computer's file is the shared copy; the computer that wrote it, when known.
    InSync(Option<ComputerId>),
    /// This computer's file differs from the shared copy and is not shared.
    LocalDifferent(LocalReason),
    /// Nothing has ever been shared for this unit.
    NoSharedCopy,
    /// A shared revision exists but its saved copy has not arrived (or is gone) yet.
    WaitingForCopy,
}

/// Whether the account (and character) folder of `identity` exists on this computer.
fn identity_present(variant_dir: &Path, unit: &SettingsUnit) -> bool {
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return false;
    };
    let mut rel = format!("Account/{}", unit.identity.account_name());
    if let Some((realm, ch)) = unit.identity.realm_character() {
        rel = format!("{rel}/{realm}/{ch}");
    }
    scope.root().check_components(&rel).is_ok()
}

/// A stored one-file checkpoint's file as the capture classification sees it.
fn local_file_fact(store: &SettingsStore, cp: &CheckpointRef, unit: &SettingsUnit) -> FileFact {
    let bytes = store.get_manifest(&cp.0).ok().flatten()?;
    let m: SettingsCheckpointManifest = decode(&bytes, 64 * 1024).ok()?;
    if &m.unit != unit {
        return None;
    }
    m.files.first().map(|f| (f.size, f.blob))
}

/// The file's size and hash as of now: `None` when it is absent, an unmatchable fact when it
/// cannot be read (so it never compares equal to an earlier scan).
fn read_fact(variant_dir: &Path, rel: &str, cap: u64) -> FileFact {
    let unreadable = Some((u64::MAX, Sha256([0; 32])));
    match wow_sync_adapters::wtf::hash_wtf_file(variant_dir, rel, cap) {
        Ok(None) => None,
        Ok(Some((size, h))) => Some((size, Sha256(h))),
        Err(_) => unreadable,
    }
}

/// WoW writes `<file>.bak` beside a saved-variables file it is about to replace.
fn bak_exists(variant_dir: &Path, rel: &str) -> bool {
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return false;
    };
    scope.root().check_components(&format!("{rel}.bak")).is_ok()
}

/// The units whose captured file is suspect (settings.md §2.1, Q-71): every changed file is
/// looked at a second time after `settle()` (the quiet wait) and must be identical, non-empty
/// and not a vanished file next to a `.bak`. Unchanged files need no second look.
pub(crate) fn suspect_units(
    variant_dir: &Path,
    store: &SettingsStore,
    local: &SettingsLocal,
    owned: &[OwnedFile],
    cap: u64,
    settle: &dyn Fn(),
) -> BTreeSet<SettingsUnit> {
    let prev_of = |unit: &SettingsUnit| -> FileFact {
        local
            .units
            .iter()
            .find(|u| &u.unit == unit)
            .and_then(|u| u.captures.first())
            .and_then(|c| local_file_fact(store, c, unit))
    };
    let looks: Vec<(&OwnedFile, FileFact)> = owned
        .iter()
        .map(|o| (o, prev_of(&o.unit)))
        .filter(|(o, prev)| o.size == 0 || *prev != Some((o.size, o.sha256)))
        .collect();
    let mut out = BTreeSet::new();
    if looks.is_empty() {
        return out;
    }
    settle();
    for (o, prev) in looks {
        let second = read_fact(variant_dir, o.rel.as_str(), cap);
        let scans = SettleScans {
            first: Some((o.size, o.sha256)),
            second,
            hashed: second,
            wow_running_at_start: false,
            wow_running_at_end: false,
            bak_present: bak_exists(variant_dir, o.rel.as_str()),
        };
        if classify_capture(prev, &scans) == CaptureClass::Suspect {
            out.insert(o.unit.clone());
        }
    }
    out
}

/// The computer that wrote the (single) current head revision of a unit's graph.
fn heads_issuer(
    ledger: &wow_sync_core::ledger::Ledger,
    graph: &wow_sync_core::settings::RevisionGraph,
) -> Option<ComputerId> {
    let h = graph.heads.first()?;
    match ledger.record(h) {
        Some(AuthorityRecord::SettingsRevision(r)) => Some(r.issuer.computer_id.clone()),
        _ => None,
    }
}

/// Removes the store copies of checkpoints that dropped out of retention, unless something still
/// names them (another unit's capture, a displaced copy, an applied state or a removal's
/// captured settings). Store data only; never a live file.
fn prune_store(
    store: &SettingsStore,
    dropped: &BTreeSet<CheckpointRef>,
    local: &SettingsLocal,
    ledger: &wow_sync_core::ledger::Ledger,
) {
    if dropped.is_empty() {
        return;
    }
    let mut retained: BTreeSet<CheckpointRef> = BTreeSet::new();
    for u in &local.units {
        retained.extend(u.captures.iter().copied());
        retained.extend(u.kept.iter().map(|k| k.checkpoint));
        retained.extend(u.last_applied);
    }
    for e in ledger.entries() {
        if let Some(AuthorityRecord::Removal(rm)) = ledger.record(&e.hash) {
            if let wow_sync_core::kinds::RemovalSettings::Checkpoints { roots } = &rm.settings {
                for r in roots {
                    if let RootState::Captured { checkpoint, .. } = &r.state {
                        retained.insert(*checkpoint);
                    }
                }
            }
        }
    }
    let blobs_of = |cp: &CheckpointRef| -> Vec<Sha256> {
        store
            .get_manifest(&cp.0)
            .ok()
            .flatten()
            .and_then(|b| decode::<SettingsCheckpointManifest>(&b, 64 * 1024).ok())
            .map(|m| m.files.iter().map(|f| f.blob).collect())
            .unwrap_or_default()
    };
    let in_use: BTreeSet<Sha256> = retained.iter().flat_map(blobs_of).collect();
    for cp in dropped.iter().filter(|c| !retained.contains(c)) {
        for b in blobs_of(cp).into_iter().filter(|b| !in_use.contains(b)) {
            let _ = store.remove_blob(&b.0);
        }
        let _ = store.remove_manifest(&cp.0);
    }
}

/// Shared settings copies stay at least this long after their file was written, whatever
/// the ledger says: an upload that is still in flight, or a revision this computer has not
/// read yet, is never mistaken for garbage.
const GC_MIN_AGE_MS: i128 = 30 * 24 * 3600 * 1000;
/// The shared folder is swept at most this often.
const GC_EVERY_MS: u64 = 24 * 3600 * 1000;

/// The checkpoints the shared folder must keep (OD-30): per unit the current head(s) and the two
/// revisions before the newest (three states), and every captured copy a removal still names.
pub(crate) fn retained_checkpoints(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
) -> Option<BTreeSet<CheckpointRef>> {
    let mut units: BTreeSet<SettingsUnit> = BTreeSet::new();
    for e in ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = ledger.record(&e.hash) {
            units.insert(r.unit.clone());
        }
    }
    if units.is_empty() {
        return None;
    }
    let mut keep = BTreeSet::new();
    for u in &units {
        let history = settings_history(ledger, derived, u);
        keep.extend(history.iter().filter(|h| h.is_head).map(|h| h.checkpoint));
        keep.extend(
            history
                .iter()
                .rev()
                .take(KEEP_CAPTURES)
                .map(|h| h.checkpoint),
        );
    }
    for e in ledger.entries() {
        if let Some(AuthorityRecord::Removal(rm)) = ledger.record(&e.hash) {
            if let wow_sync_core::kinds::RemovalSettings::Checkpoints { roots } = &rm.settings {
                for r in roots {
                    if let RootState::Captured { checkpoint, .. } = &r.state {
                        keep.insert(*checkpoint);
                    }
                }
            }
        }
    }
    Some(keep)
}

/// The checkpoints of every settings revision record on the shared folder that is younger than the
/// age gate, whether or not this computer has read it yet (a peer's record may still be unknown
/// here). Gives up (None) when a recent record cannot be listed or read.
fn recent_revision_checkpoints(
    provider: &ProviderRoot,
    group: &GroupId,
    cutoff_ns: i128,
) -> Option<BTreeSet<CheckpointRef>> {
    let mut out = BTreeSet::new();
    let members = provider.list(&layout::members_dir(group)).ok()?;
    for m in members.iter().filter(|m| m.is_dir) {
        let Ok(c) = ComputerId::parse(&m.name) else {
            continue;
        };
        for e in provider.list(&layout::authority_dir(group, &c)).ok()? {
            if e.is_dir || e.mtime_ns < cutoff_ns {
                continue;
            }
            let Some(h) = e
                .name
                .strip_suffix(".json")
                .and_then(|s| Sha256::parse(s).ok())
            else {
                continue;
            };
            let bytes = provider
                .read(
                    &layout::authority_record(group, &c, &h),
                    wow_sync_core::kinds::MAX_RECORD_BYTES as u64,
                )
                .ok()?;
            if let Ok(AuthorityRecord::SettingsRevision(r)) =
                wow_sync_core::authority::decode_authority(&bytes, &h)
            {
                out.insert(r.checkpoint);
            }
        }
    }
    Some(out)
}

/// Deletes shared settings manifests and blobs that no kept checkpoint names and that are older
/// than `GC_MIN_AGE_MS`. Gives up (`None`) when a kept manifest cannot be read, because then the
/// blobs it names are unknown. Returns the number of files removed.
pub(crate) fn sweep_provider(
    provider: &ProviderRoot,
    group: &GroupId,
    keep: &BTreeSet<CheckpointRef>,
    now_ms: u64,
) -> Option<usize> {
    let cutoff_ns = (now_ms as i128 - GC_MIN_AGE_MS) * 1_000_000;
    let mut keep = keep.clone();
    keep.extend(recent_revision_checkpoints(provider, group, cutoff_ns)?);
    let mut keep_blobs: BTreeSet<String> = BTreeSet::new();
    let mut keep_manifests: BTreeSet<String> = BTreeSet::new();
    for cp in &keep {
        keep_manifests.insert(format!("{cp}.json"));
        match provider.read(&layout::settings_manifest(cp), 64 * 1024) {
            Ok(b) => {
                let m: SettingsCheckpointManifest = decode(&b, 64 * 1024).ok()?;
                keep_blobs.extend(m.files.iter().map(|f| f.blob.to_string()));
            }
            Err(e) if e.kind == wow_sync_adapters::FsErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    let mut removed = 0;
    let mut sweep = |dir: &str, keep: &BTreeSet<String>| -> Option<()> {
        let entries = match provider.list(dir) {
            Ok(v) => v,
            Err(e) if e.kind == wow_sync_adapters::FsErrorKind::NotFound => return Some(()),
            Err(_) => return None,
        };
        for e in entries {
            let stale = e.mtime_ns < cutoff_ns;
            let plain = !e.is_dir && e.state == wow_sync_adapters::provider::EntryState::Local;
            let doomed = plain && stale && !keep.contains(&e.name);
            if doomed && provider.remove_file(&format!("{dir}/{}", e.name)).is_ok() {
                removed += 1;
            }
        }
        Some(())
    };
    sweep("settings/manifests", &keep_manifests)?;
    sweep("settings/blobs", &keep_blobs)?;
    Some(removed)
}

/// Older shared states (outside the kept three) whose copy no longer exists on the shared folder or
/// in this computer's store, so a restore of them could not work.
fn unavailable_checkpoints(
    a: &StepArgs,
    store: &SettingsStore,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
) -> BTreeSet<CheckpointRef> {
    let mut units: BTreeSet<SettingsUnit> = BTreeSet::new();
    for e in ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = ledger.record(&e.hash) {
            units.insert(r.unit.clone());
        }
    }
    let mut out = BTreeSet::new();
    for u in &units {
        let history = settings_history(ledger, derived, u);
        for h in history
            .iter()
            .rev()
            .skip(KEEP_CAPTURES)
            .filter(|h| !h.is_head)
        {
            let local = store.get_manifest(&h.checkpoint.0).ok().flatten().is_some();
            if !local && manifest_blob(a.provider, &h.checkpoint, u).is_none() {
                out.insert(h.checkpoint);
            }
        }
    }
    out
}

/// The daily sweep of the shared folder; a failure changes nothing and is tried again later.
fn gc_provider(
    a: &StepArgs,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
) {
    let now = now_unix_ms() as u64;
    let stamp = format!("settings-gc-{}.txt", a.group);
    let last = a
        .state
        .read(&stamp, 64)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.trim().parse::<u64>().ok());
    if last.is_some_and(|l| now.saturating_sub(l) < GC_EVERY_MS) {
        return;
    }
    let Some(keep) = retained_checkpoints(ledger, derived) else {
        return;
    };
    if sweep_provider(a.provider, a.group, &keep, now).is_some() {
        let _ = a.state.write_atomic(&stamp, now.to_string().as_bytes());
    }
}
/// Copies the checkpoint (blob first, then manifest) to the shared folder, write-if-absent.
pub(crate) fn upload_checkpoint(
    provider: &ProviderRoot,
    store: &SettingsStore,
    checkpoint: &CheckpointRef,
    me: &str,
) -> Result<(), String> {
    let manifest = store
        .get_manifest(&checkpoint.0)
        .map_err(|e| e.to_string())?
        .ok_or("the checkpoint is not in the local store")?;
    let m: SettingsCheckpointManifest = decode(&manifest, 64 * 1024).map_err(|e| e.to_string())?;
    for f in &m.files {
        let rel = layout::settings_blob(&f.blob);
        if matches!(
            provider.check_object(&rel, BLOB_LIMIT),
            Ok(wow_sync_adapters::provider::ObjectObs::Local { sha256, .. }) if sha256 == f.blob.0
        ) {
            // Reused: its age restarts, so a sweep by a peer that has not read the new revision
            // yet cannot take it (RV11-12).
            provider.touch(&rel).map_err(|e| e.to_string())?;
            continue;
        }
        let (mut blob, _) = store
            .open_blob(&f.blob.0, BLOB_LIMIT)
            .map_err(|e| e.to_string())?
            .ok_or("the blob is not in the local store")?;
        provider
            .put_object_reader(&rel, &mut blob, f.blob.0, me)
            .map_err(|e| e.to_string())?;
    }
    authority_store::put_record(
        provider,
        &layout::settings_manifest(checkpoint),
        &manifest,
        64 * 1024,
        me,
    )
    .map_err(|e| e.to_string())?;
    provider
        .touch(&layout::settings_manifest(checkpoint))
        .map_err(|e| e.to_string())?;
    Ok(())
}

const BLOB_LIMIT: u64 = 256 * 1024 * 1024;
/// Captures kept per unit (current + two previous) and displaced copies kept per unit.
const KEEP_CAPTURES: usize = 3;
const LOCAL_LIMIT: usize = 16 * 1024 * 1024;

fn local_name(group: &GroupId) -> String {
    format!("settings-local-{group}.json")
}

/// This computer's settings sync consent, included accounts and per-unit bookkeeping. Damaged
/// or missing state means no consent: nothing is read or applied until the owner confirms again.
pub fn load_local(state: &LocalState, group: &GroupId) -> SettingsLocal {
    state
        .read(&local_name(group), LOCAL_LIMIT as u64)
        .ok()
        .flatten()
        .and_then(|b| decode::<SettingsLocal>(&b, LOCAL_LIMIT).ok())
        .unwrap_or(SettingsLocal {
            units: Vec::new(),
            paused_folders: Vec::new(),
            included_accounts: Vec::new(),
            consent: false,
            first_adoption: None,
            share_when_ready: false,
        })
}

pub fn save_local(state: &LocalState, group: &GroupId, l: &SettingsLocal) -> Result<(), FsError> {
    state.write_atomic(&local_name(group), &encode(l))
}

/// One paused folder with what a resume would meet (settings.md resume sheet inputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedFolder {
    pub folder: wow_sync_core::paths::FolderKey,
    /// This computer's saved settings differ from the last shared state it applied.
    pub here_changed: bool,
    /// The shared settings moved on to a revision this computer has not applied.
    pub shared_newer: bool,
}

/// The settings facts the GUI needs: consent, accounts, who is master and what is paused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub enabled: bool,
    pub accounts: Vec<String>,
    /// The single policy's master; `None` when there is no policy, no master or a conflict.
    pub master: Option<EnrollmentRef>,
    pub master_name: Option<String>,
    pub exceptions: Vec<wow_sync_core::kinds::PolicyRule>,
    /// Several competing policies exist: nothing is chosen for the owner.
    pub policy_conflict: bool,
    pub paused: Vec<PausedFolder>,
    /// `WTF\Account` folders this computer has that consent could include (V-05). Link or
    /// reparse-point entries are not offered; nothing under them is read.
    pub available_accounts: Vec<String>,
    /// The add-ons "Share these now" would push: this computer's saved settings that differ from
    /// the shared copy or have none. Filled by the service; the master confirmation lists them
    /// and `Command::ShareSettingsNow { units }` pushes the chosen ones. Nothing is shared by
    /// becoming master.
    pub to_share: Vec<ShareOffer>,
    /// Some computer has already shared settings revisions in this group.
    pub group_has_shared_settings: bool,
}

/// One add-on's settings that could be shared now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareOffer {
    pub unit: SettingsUnit,
    pub checkpoint: wow_sync_core::ids::CheckpointRef,
    /// Nothing has ever been shared for this unit (otherwise it replaces the shared copy).
    pub not_shared_yet: bool,
}

/// The account folder names under `<variant>\WTF\Account`, sorted. A missing `WTF` or `Account`
/// folder, or an unreadable one, gives an empty list.
pub fn available_accounts(variant_dir: &std::path::Path) -> Vec<String> {
    use wow_sync_adapters::fs_safe::{ReparseKind, WtfScope};
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return Vec::new();
    };
    let Ok(entries) = scope.root().list("Account") else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .into_iter()
        .filter(|e| e.is_dir && e.reparse == ReparseKind::None)
        .map(|e| e.name)
        .collect();
    names.sort();
    names
}

/// Account, realm and character folder names under `<variant>\WTF\Account`, each sorted and
/// deduplicated, for the diagnostics second-defence scan (ops.md §5.3). `SavedVariables` is
/// not a realm. Unreadable or linked folders are skipped.
pub fn wtf_names(variant_dir: &std::path::Path) -> (Vec<String>, Vec<String>, Vec<String>) {
    use wow_sync_adapters::fs_safe::{ReparseKind, WtfScope};
    let accounts = available_accounts(variant_dir);
    let Ok(Some(scope)) = WtfScope::open(variant_dir, false) else {
        return (accounts, Vec::new(), Vec::new());
    };
    let dirs = |rel: &str| -> Vec<String> {
        scope
            .root()
            .list(rel)
            .map(|v| {
                v.into_iter()
                    .filter(|e| e.is_dir && e.reparse == ReparseKind::None)
                    .map(|e| e.name)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (mut realms, mut chars) = (Vec::new(), Vec::new());
    for a in &accounts {
        for r in dirs(&format!("Account/{a}"))
            .into_iter()
            .filter(|r| !r.eq_ignore_ascii_case("SavedVariables"))
        {
            chars.extend(dirs(&format!("Account/{a}/{r}")));
            realms.push(r);
        }
    }
    for v in [&mut realms, &mut chars] {
        v.sort();
        v.dedup();
    }
    (accounts, realms, chars)
}

pub fn settings_snapshot(
    local: &SettingsLocal,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &std::collections::BTreeMap<wow_sync_core::ids::EnrollmentId, String>,
) -> SettingsSnapshot {
    let heads = wow_sync_core::settings::policy_heads(ledger, derived);
    let (master, exceptions, policy_conflict) = match heads.as_slice() {
        [one] => (one.master.clone(), one.rules.clone(), false),
        [] => (None, Vec::new(), false),
        _ => (None, Vec::new(), true),
    };
    let mut paused: Vec<PausedFolder> = Vec::new();
    for folder in &local.paused_folders {
        let (mut here_changed, mut shared_newer) = (false, false);
        for u in local.units.iter().filter(|u| &u.unit.folder == folder) {
            let graph = wow_sync_core::settings::graph_from_ledger(ledger, derived, &u.unit);
            let latest = u.captures.first();
            here_changed |= latest.is_some()
                && latest != u.last_applied.as_ref()
                && (u.last_applied.is_some() || !graph.heads.is_empty());
            shared_newer |= !graph.pending
                && graph.head_states.len() == 1
                && graph.heads.iter().any(|h| !u.basis.contains(h));
        }
        paused.push(PausedFolder {
            folder: folder.clone(),
            here_changed,
            shared_newer,
        });
    }
    SettingsSnapshot {
        enabled: local.consent,
        accounts: local.included_accounts.clone(),
        master_name: master
            .as_ref()
            .and_then(|m| names.get(&m.enrollment_id).cloned()),
        master,
        exceptions,
        policy_conflict,
        paused,
        available_accounts: Vec::new(),
        to_share: Vec::new(),
        group_has_shared_settings: ledger.entries().any(|e| {
            derived.status.get(&e.hash) == Some(&wow_sync_core::ledger::Status::Valid)
                && matches!(
                    ledger.record(&e.hash),
                    Some(wow_sync_core::authority::AuthorityRecord::SettingsRevision(
                        _
                    ))
                )
        }),
    }
}

fn use_shared_name(group: &GroupId) -> String {
    format!("settings-use-shared-{group}.json")
}

/// The units the owner asked to "Use shared settings" for, until a cycle acts on them.
pub fn load_use_shared(state: &LocalState, group: &GroupId) -> Vec<SettingsUnit> {
    state
        .read(&use_shared_name(group), LOCAL_LIMIT as u64)
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_use_shared(
    state: &LocalState,
    group: &GroupId,
    units: &[SettingsUnit],
) -> Result<(), FsError> {
    if units.is_empty() {
        return state.remove(&use_shared_name(group));
    }
    state.write_atomic(
        &use_shared_name(group),
        &serde_json::to_vec(units).expect("plain data serializes"),
    )
}

/// The folders whose selected `.toc` declares saved variables (metadata only, never Lua).
pub fn declaring_folders(
    scans: &[wow_sync_core::inventory::FolderScan],
    mapping: &wow_sync_core::variant::TocMapping,
) -> Vec<DeclaringFolder> {
    use wow_sync_core::toc::{select_toc, TocSelection};
    let mut out = Vec::new();
    for s in scans {
        if s.status != wow_sync_core::inventory::ScanStatus::Ok {
            continue;
        }
        let Ok(folder) = wow_sync_core::paths::validate_folder_name(&s.name) else {
            continue;
        };
        if let TocSelection::Selected { info, .. } = select_toc(&s.name, &s.tocs, mapping) {
            out.push(DeclaringFolder {
                folder,
                saved_variables: info.saved_variables,
                saved_variables_per_character: info.saved_variables_per_character,
                toc_ambiguous: false,
            });
        }
    }
    out
}

/// Reports (done, total, add-on name, origin computer) as settings units are applied.
pub(crate) type ProgressTick<'a> = dyn Fn(u64, u64, Option<&str>, Option<&ComputerId>) + 'a;

/// What one settings cycle needs from the service. `cap` is the saved-file size limit (32 MiB,
/// D-26); with `None` (tests only) nothing is read, captured, published or applied.
pub(crate) struct StepArgs<'a> {
    pub cap: Option<u64>,
    pub state: &'a LocalState,
    pub provider: &'a ProviderRoot,
    pub variant_dir: &'a Path,
    pub group: &'a GroupId,
    pub own: &'a EnrollmentRef,
    pub inventory: &'a wow_sync_core::inventory::Inventory,
    pub decls: &'a [DeclaringFolder],
    pub game_blocks: bool,
    /// Reports progress: units done, units in all, the add-on being worked on.
    /// Progress: units done, units total, the add-on, and the computer a revision being applied
    /// came from (None for local capture or publish).
    pub tick: &'a ProgressTick<'a>,
    /// The quiet wait between the two scans a changed file must survive (Q-71).
    pub settle: &'a dyn Fn(),
}

#[derive(Debug, Default)]
pub(crate) struct StepOutcome {
    pub applies: Vec<PackageApply>,
    pub notes: Vec<String>,
    /// Revisions this cycle wrote, in order, with what each replaced and brought.
    pub published: Vec<(Sha256, SettingsChangeInfo)>,
    /// What each planned apply replaces and brings, by operation.
    pub apply_info: std::collections::BTreeMap<OpId, SettingsChangeInfo>,
    pub facts: SettingsFacts,
    /// Units this cycle looked at, for the progress line.
    pub considered: usize,
}

fn unit_rel(unit: &SettingsUnit, name: &str) -> Option<wow_sync_core::paths::SettingsPath> {
    use wow_sync_core::paths::{account_settings_path, character_settings_path};
    match unit.identity.realm_character() {
        None => account_settings_path(unit.identity.account_name(), name).ok(),
        Some((realm, ch)) => {
            character_settings_path(unit.identity.account_name(), realm, ch, name).ok()
        }
    }
}

/// Steps 3s, 5s and 7s for every unit this computer may touch (settings.md §10.3): capture the
/// owned files, publish the authority's changes inside its epoch, and plan the settings applies
/// the target asks for. Returns the operations; every ledger write is already persisted.
pub(crate) fn settings_cycle(plane: &mut Plane, a: &StepArgs, next_seq: &mut u64) -> StepOutcome {
    use wow_sync_core::settings::{
        authority_for, build_auto, graph_from_ledger, policy_heads, publish_decision, target_case,
        Authority, CaptureKind, Case, Publish, PublishInput, TargetInput,
    };
    let mut out = StepOutcome::default();
    let Some(cap) = a.cap else {
        return out;
    };
    if a.game_blocks {
        return out;
    }
    let mut local = load_local(a.state, a.group);
    if !local.consent {
        return out;
    }
    let included: BTreeSet<String> = local.included_accounts.iter().cloned().collect();
    let derived = derive(&plane.ledger, a.group, &plane.members);
    let heads = policy_heads(&plane.ledger, &derived);
    if heads.is_empty() {
        return out;
    }
    let Ok(scope) = observe_scope(a.variant_dir, a.decls, &included, Some(cap)) else {
        out.notes.push("settings could not be read".into());
        return out;
    };
    let Ok(store) = SettingsStore::open(a.variant_dir) else {
        out.notes
            .push("the settings store could not be opened".into());
        return out;
    };
    let display = |k: &wow_sync_core::paths::FolderKey| {
        a.inventory
            .folders
            .iter()
            .find(|f| &f.key == k)
            .map_or_else(
                || k.as_str().to_string(),
                |f| crate::detail::strip_colour_codes(&f.name),
            )
    };
    for d in &scope.diagnoses {
        match d {
            wow_sync_core::settings::ScopeDiagnosis::SharedVariable { folders, .. } => {
                for u in scope.blocked.iter().filter(|u| folders.contains(&u.folder)) {
                    let others = folders
                        .iter()
                        .filter(|f| **f != u.folder)
                        .map(&display)
                        .collect();
                    out.facts.units.insert(u.clone(), UnitFact::Several(others));
                }
            }
            wow_sync_core::settings::ScopeDiagnosis::TooLarge { file, size } => {
                for u in &scope.blocked {
                    let same = unit_rel(u, &display(&u.folder))
                        .is_some_and(|r| r.as_str().eq_ignore_ascii_case(file));
                    if same {
                        out.facts
                            .units
                            .entry(u.clone())
                            .or_insert(UnitFact::TooLarge(*size));
                    }
                }
            }
            _ => {}
        }
    }
    let epochs = EpochStore::new(a.state, a.group.clone());
    let mut eps = epochs.load();
    let mut units: BTreeSet<SettingsUnit> = scope.owned.iter().map(|o| o.unit.clone()).collect();
    for e in plane.ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = plane.ledger.record(&e.hash) {
            if included.contains(r.unit.identity.account_name()) {
                units.insert(r.unit.clone());
            }
        }
    }
    let mut written = false;
    let mut dirty = false;
    let mut use_shared = load_use_shared(a.state, a.group);
    let mut requests_changed = false;
    let total = units.len() as u64;
    out.considered = units.len();
    let suspect = suspect_units(a.variant_dir, &store, &local, &scope.owned, cap, a.settle);
    let mut dropped: BTreeSet<CheckpointRef> = BTreeSet::new();
    for (n, unit) in units.into_iter().enumerate() {
        (a.tick)(n as u64, total, None, None);
        if scope.blocked.contains(&unit) {
            continue;
        }
        let Some(name) = a
            .inventory
            .folders
            .iter()
            .find(|f| f.key == unit.folder)
            .map(|f| f.name.clone())
        else {
            continue;
        };
        (a.tick)(
            n as u64,
            total,
            Some(&crate::detail::strip_colour_codes(&name)),
            None,
        );
        let Some(rel) = unit_rel(&unit, &name) else {
            continue;
        };
        let owned = scope.owned.iter().find(|o| o.unit == unit);
        if owned.is_none() && !identity_present(a.variant_dir, &unit) {
            // The app never creates an account, realm or character folder: WoW does.
            let who = match unit.identity.realm_character() {
                None => "this account".to_string(),
                Some((realm, ch)) => format!("{ch} on {realm}"),
            };
            out.facts.units.insert(unit.clone(), UnitFact::Waiting(who));
            continue;
        }
        let suspect_now = suspect.contains(&unit)
            || (owned.is_none()
                && local
                    .units
                    .iter()
                    .any(|u| u.unit == unit && !u.captures.is_empty())
                && bak_exists(a.variant_dir, rel.as_str()));
        if suspect_now {
            out.facts.units.insert(unit.clone(), UnitFact::Suspect);
            continue;
        }
        let live = match owned {
            Some(o) => match capture_unit(a.variant_dir, &store, o, cap) {
                Ok(c) => Some(c),
                Err(_) => {
                    out.notes
                        .push(format!("{name}: settings changed while being read"));
                    continue;
                }
            },
            None => None,
        };
        let li = match local.units.iter().position(|u| u.unit == unit) {
            Some(i) => i,
            None => {
                local.units.push(LocalUnitState {
                    unit: unit.clone(),
                    captures: Vec::new(),
                    basis: Vec::new(),
                    last_applied: None,
                    pinned: false,
                    authority_since: None,
                    kept: Vec::new(),
                });
                local.units.len() - 1
            }
        };
        let ei = match eps.iter().position(|u| u.unit == unit) {
            Some(i) => i,
            None => {
                eps.push(UnitEpoch {
                    unit: unit.clone(),
                    epoch_clean: false,
                    capture_seq: 0,
                    capture_kind: None,
                });
                eps.len() - 1
            }
        };
        let graph = graph_from_ledger(&plane.ledger, &derived, &unit);
        let authority = authority_for(&heads, &unit.folder);
        let is_authority = matches!(&authority, Authority::Holder(h) if h == a.own);
        let paused = local.paused_folders.contains(&unit.folder);
        dirty = true;

        // Becoming the authority starts a new epoch: the activation capture never publishes.
        // Becoming master publishes nothing by itself (design §5.2, Q-75): sharing is the
        // explicit "Share these now" / per-add-on share.
        let mut activation = false;
        if is_authority && local.units[li].authority_since.is_none() {
            let since = eps[ei].capture_seq;
            local.units[li].authority_since = Some(since);
            eps[ei].capture_seq = since + 1;
            eps[ei].capture_kind = Some(CaptureKind::Activation);
            eps[ei].epoch_clean = wow_sync_core::settings::epoch_clean(
                live,
                local.units[li].last_applied,
                !graph.nodes.is_empty(),
            );
            if let Some(l) = live {
                local.units[li].captures.insert(0, l);
            }
            activation = true;
        }
        if !is_authority && local.units[li].authority_since.is_some() {
            local.units[li].authority_since = None;
            eps[ei].epoch_clean = false;
        }

        let previous_seq = eps[ei].capture_seq;
        let changed =
            !activation && live.is_some() && live != local.units[li].captures.first().copied();
        if changed {
            eps[ei].capture_seq += 1;
            eps[ei].capture_kind = Some(CaptureKind::SessionEnd);
            if let Some(l) = live {
                local.units[li].captures.insert(0, l);
                if local.units[li].captures.len() > KEEP_CAPTURES {
                    for old in local.units[li].captures.split_off(KEEP_CAPTURES) {
                        dropped.insert(old);
                    }
                }
            }
        }
        let case = target_case(&TargetInput {
            graph: &graph,
            authority: &authority,
            is_authority,
            paused,
            basis: &local.units[li].basis,
            basis_checkpoint: local.units[li].last_applied,
            live,
            epoch_clean: eps[ei].epoch_clean,
        });
        // An explicit "Use shared settings" turns a held local difference into a yield; every
        // other case ignores it, and the request is spent once the unit was evaluated.
        let case = match (&case, use_shared.contains(&unit), graph.head_states.len()) {
            (Case::LocalDifferent(_), true, 1) => Case::Yield {
                to: *graph.head_states.keys().next().expect("one head state"),
                capture_first: true,
            },
            _ => case,
        };
        if !paused && use_shared.contains(&unit) {
            use_shared.retain(|u| u != &unit);
            requests_changed = true;
        }
        let fact = match &case {
            Case::InSync { .. } => Some(UnitFact::InSync(heads_issuer(&plane.ledger, &graph))),
            Case::FirstAdoption(to) if live == Some(*to) => {
                Some(UnitFact::InSync(heads_issuer(&plane.ledger, &graph)))
            }
            Case::LocalDifferent(r) => Some(UnitFact::LocalDifferent(*r)),
            Case::NoSharedCopy if live.is_some() => Some(UnitFact::NoSharedCopy),
            _ => None,
        };
        if let Some(f) = fact {
            out.facts.units.insert(unit.clone(), f);
        }
        match case {
            Case::InSync { basis } => {
                local.units[li].basis = basis;
                local.units[li].last_applied = live;
            }
            Case::NoSharedCopy | Case::LocalDifferent(_) if is_authority && changed => {
                let decision = publish_decision(&PublishInput {
                    authority: authority.clone(),
                    own: a.own.clone(),
                    paused,
                    complete: true,
                    kind: CaptureKind::SessionEnd,
                    changed_in_session: true,
                    epoch_clean: eps[ei].epoch_clean,
                    authority_since: local.units[li].authority_since,
                    compared_with_local_seq: previous_seq,
                });
                let Some(cp) = live.filter(|_| decision == Publish::Auto) else {
                    continue;
                };
                let rec = match build_auto(
                    &plane.ledger,
                    &derived,
                    a.group,
                    Issuer {
                        computer_id: a.own.computer_id.clone(),
                        enrollment_id: a.own.enrollment_id.clone(),
                        issued_at_seq: *next_seq,
                    },
                    unit.clone(),
                    cp,
                    &local.units[li].basis,
                    now_unix_ms() as u64,
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = e;
                        out.notes
                            .push(format!("{name}: not shared (could not be prepared)"));
                        continue;
                    }
                };
                if let Err(e) =
                    upload_checkpoint(a.provider, &store, &cp, a.own.computer_id.as_str())
                {
                    let _ = e;
                    out.notes
                        .push(format!("{name}: not shared (could not be uploaded)"));
                    continue;
                }
                match write_own_record(
                    plane,
                    a.provider,
                    a.group,
                    &a.own.computer_id,
                    &AuthorityRecord::SettingsRevision(rec),
                ) {
                    Ok(h) => {
                        *next_seq += 1;
                        written = true;
                        let old = local.units[li].basis.iter().find_map(|b| {
                            match plane.ledger.record(b) {
                                Some(AuthorityRecord::SettingsRevision(r)) => {
                                    manifest_unit(a.provider, &r.checkpoint, &unit)
                                }
                                _ => None,
                            }
                        });
                        if let Some(o) = owned {
                            out.published.push((
                                h,
                                SettingsChangeInfo {
                                    old,
                                    new: manifest_unit(a.provider, &cp, &unit)
                                        .unwrap_or((o.sha256, o.size)),
                                    identity: identity_text(&unit),
                                },
                            ));
                        }
                        local.units[li].basis = vec![h];
                        local.units[li].last_applied = Some(cp);
                        out.facts.units.insert(
                            unit.clone(),
                            UnitFact::InSync(Some(a.own.computer_id.clone())),
                        );
                    }
                    Err(_) => out
                        .notes
                        .push(format!("{name}: not shared (could not be recorded)")),
                }
            }
            Case::FirstAdoption(to) if live == Some(to) => {
                local.units[li].basis = graph.heads.clone();
                local.units[li].last_applied = live;
            }
            Case::FirstAdoption(to) | Case::Yield { to, .. } => {
                let Some(blob) = manifest_blob(a.provider, &to, &unit) else {
                    out.facts
                        .units
                        .insert(unit.clone(), UnitFact::WaitingForCopy);
                    out.notes
                        .push(format!("{name}: shared settings are not available yet"));
                    continue;
                };
                // The replaced copy stays restorable (RV11-01): a peer's own edit that the
                // shared copy overwrites is pinned as a displaced copy.
                let is_shared = |l: wow_sync_core::ids::CheckpointRef| {
                    plane.ledger.entries().any(|e| {
                        matches!(plane.ledger.record(&e.hash),
                            Some(AuthorityRecord::SettingsRevision(r)) if r.checkpoint == l && r.unit == unit)
                    })
                };
                if let Some(l) = live.filter(|l| {
                    *l != to && Some(*l) != local.units[li].last_applied && !is_shared(*l)
                }) {
                    let kept = &mut local.units[li].kept;
                    if !kept.iter().any(|k| k.checkpoint == l) {
                        kept.insert(
                            0,
                            KeptCopy {
                                checkpoint: l,
                                at_unix_ms: now_unix_ms() as u64,
                            },
                        );
                        if kept.len() > KEEP_CAPTURES {
                            for old in kept.split_off(KEEP_CAPTURES) {
                                dropped.insert(old.checkpoint);
                            }
                        }
                    }
                }
                let origin =
                    plane
                        .ledger
                        .entries()
                        .find_map(|e| match plane.ledger.record(&e.hash) {
                            Some(AuthorityRecord::SettingsRevision(r))
                                if r.checkpoint == to && r.unit == unit =>
                            {
                                Some(r.issuer.computer_id.clone())
                            }
                            _ => None,
                        });
                (a.tick)(
                    n as u64,
                    total,
                    Some(&crate::detail::strip_colour_codes(&name)),
                    origin.as_ref().filter(|c| **c != a.own.computer_id),
                );
                let op = OpId::parse(&random_id_hex()).expect("random id");
                if let Some((nb, ns)) = manifest_unit(a.provider, &to, &unit) {
                    out.apply_info.insert(
                        op.clone(),
                        SettingsChangeInfo {
                            old: live
                                .and_then(|l| manifest_unit(a.provider, &l, &unit))
                                .or_else(|| owned.map(|o| (o.sha256, o.size))),
                            new: (nb, ns),
                            identity: identity_text(&unit),
                        },
                    );
                }
                out.applies.push(plan_settings_apply(
                    op,
                    owned,
                    unit.clone(),
                    rel,
                    Some(blob),
                    to,
                    wow_sync_core::explain::Explanation::Applied {
                        package: format!("{name} settings"),
                    },
                ));
            }
            _ => {}
        }
    }
    if requests_changed {
        let _ = save_use_shared(a.state, a.group, &use_shared);
    }
    prune_store(&store, &dropped, &local, &plane.ledger);
    gc_provider(a, &plane.ledger, &derived);
    out.facts.unavailable = unavailable_checkpoints(a, &store, &plane.ledger, &derived);
    if dirty {
        let _ = save_local(a.state, a.group, &local);
        let _ = epochs.save(&eps);
    }
    if written {
        let _ = plane.save(a.state);
    }
    out
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

/// Why a unit's settings are or are not syncing here, most blocking reason first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsUnitStatus {
    /// This computer is retired: it records no settings intent.
    Retired,
    /// No consent on this computer (or no size cap): nothing is read or applied.
    Disabled,
    /// A policy exception keeps this folder's settings on each computer.
    NotShared,
    /// Competing policies disagree about this folder; nothing is chosen for the owner.
    PolicyConflict,
    Paused,
    /// Competing shared revisions exist; the owner picks one (`Command::ChooseSettings`).
    NeedsChoice,
    /// The account or character folder this unit belongs to does not exist on this computer
    /// yet. The app never creates it; the settings are applied once WoW creates the folder.
    /// `SettingsUnitRow::waiting_for` has the identity in plain words.
    WaitingForIdentity,
    /// The file is larger than the saved-file size limit; it is never synced
    /// (`SettingsUnitRow::size_bytes` has its size).
    TooLarge,
    /// Several add-ons declare the same saved variables; none of their files is synced
    /// (`SettingsUnitRow::shared_with` names the other add-ons).
    OwnedBySeveral,
    /// WoW is running; settings are neither read nor replaced until it closes.
    WaitingForWow,
    /// The file looks incomplete or is still changing; nothing is captured, shared or replaced
    /// until a later look finds it complete.
    Suspect,
    /// This computer's file differs from the shared copy and is not shared; nothing replaces it
    /// unless the owner chooses "Use shared settings" (`Command::UseSharedSettings`).
    LocalDifferent(LocalReason),
    /// Nothing has been shared for this unit yet.
    NoSharedCopy,
    /// The shared revision's saved copy is not in the shared folder yet (or is gone); the unit
    /// applies by itself once it arrives.
    WaitingForSharedCopy,
    /// This computer's file is the shared copy (`SettingsUnitRow::in_sync_from`).
    InSync,
    Active,
}

/// One of the competing shared states Choose offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadChoice {
    /// The value `Command::ChooseSettings { checkpoint }` takes.
    pub checkpoint: wow_sync_core::ids::CheckpointRef,
    /// The revisions currently at this state (equal heads are one state).
    pub revisions: Vec<wow_sync_core::ids::Sha256>,
}

/// Everything the Settings screen needs for one unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitRow {
    pub unit: SettingsUnit,
    pub status: SettingsUnitStatus,
    /// Settings of this unit are being synced here right now.
    pub live: bool,
    /// Empty unless the unit needs a choice and this computer may make it.
    pub competing: Vec<HeadChoice>,
    /// The value `Command::ChooseSettings { shown_heads }` takes: every current head revision.
    pub shown_heads: Vec<wow_sync_core::ids::Sha256>,
    /// Oldest first; the `checkpoint` of an entry is what `Command::RestoreSettings` takes.
    pub history: Vec<SettingsHistoryEntry>,
    pub restore_possible: bool,
    /// The value `Command::ShareSettings { checkpoint }` takes: this computer's latest capture.
    pub latest_capture: Option<wow_sync_core::ids::CheckpointRef>,
    pub can_share: bool,
    /// Set with `WaitingForIdentity`: the missing identity in plain words, for the detail row
    /// only (it names an account or character, so it is never logged).
    pub waiting_for: Option<String>,
    /// With `TooLarge`: the file size in bytes.
    pub size_bytes: Option<u64>,
    /// With `OwnedBySeveral`: the display names of the other add-ons that declare the variable.
    pub shared_with: Vec<String>,
    /// With `InSync`: the computer whose copy this is, when known.
    pub in_sync_from: Option<String>,
    /// This computer's own copies that shared settings replaced, newest first. `checkpoint` is
    /// what `Command::RestoreSettings` takes; `hash` repeats it and `kind`/`issuer` are
    /// placeholders (these were never revisions). Kept as the newest three per unit, apart from
    /// code recovery points.
    pub local_copies: Vec<SettingsHistoryEntry>,
    /// With `LocalDifferent`: why the difference is held.
    pub local_reason: Option<LocalReason>,
}

#[allow(clippy::too_many_arguments)]
pub fn settings_units(
    local: &SettingsLocal,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    retired_self: bool,
    cap_enabled: bool,
    facts: &SettingsFacts,
    names: &std::collections::BTreeMap<ComputerId, String>,
    own: &wow_sync_core::kinds::EnrollmentRef,
) -> Vec<SettingsUnitRow> {
    use wow_sync_core::authority::AuthorityRecord;
    use wow_sync_core::settings::{authority_for, graph_from_ledger, policy_heads, Authority};
    let mut units: std::collections::BTreeSet<SettingsUnit> =
        local.units.iter().map(|u| u.unit.clone()).collect();
    for e in ledger.entries() {
        if let Some(AuthorityRecord::SettingsRevision(r)) = ledger.record(&e.hash) {
            units.insert(r.unit.clone());
        }
    }
    units.extend(facts.units.keys().cloned());
    let heads = policy_heads(ledger, derived);
    units
        .into_iter()
        .map(|unit| {
            let authority = authority_for(&heads, &unit.folder);
            let graph = graph_from_ledger(ledger, derived, &unit);
            let mut history = settings_history(ledger, derived, &unit);
            history.retain(|h| !facts.unavailable.contains(&h.checkpoint));
            let latest_capture = local
                .units
                .iter()
                .find(|u| u.unit == unit)
                .and_then(|u| u.captures.first().copied());
            let paused = local.paused_folders.contains(&unit.folder);
            let blocked = matches!(authority, Authority::NotShared | Authority::Conflict);
            let actionable =
                !retired_self && local.consent && cap_enabled && !heads.is_empty() && !blocked;
            let competing: Vec<HeadChoice> = if actionable && graph.head_states.len() > 1 {
                graph
                    .head_states
                    .iter()
                    .map(|(c, r)| HeadChoice {
                        checkpoint: *c,
                        revisions: r.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let fact = facts.units.get(&unit);
            let status = if retired_self {
                SettingsUnitStatus::Retired
            } else if !local.consent || !cap_enabled {
                SettingsUnitStatus::Disabled
            } else if authority == Authority::NotShared {
                SettingsUnitStatus::NotShared
            } else if authority == Authority::Conflict {
                SettingsUnitStatus::PolicyConflict
            } else if facts.wow_running {
                SettingsUnitStatus::WaitingForWow
            } else if let Some(UnitFact::TooLarge(_)) = fact {
                SettingsUnitStatus::TooLarge
            } else if let Some(UnitFact::Several(_)) = fact {
                SettingsUnitStatus::OwnedBySeveral
            } else if let Some(UnitFact::Suspect) = fact {
                SettingsUnitStatus::Suspect
            } else if paused {
                SettingsUnitStatus::Paused
            } else if !competing.is_empty() {
                SettingsUnitStatus::NeedsChoice
            } else if let Some(UnitFact::Waiting(_)) = fact {
                SettingsUnitStatus::WaitingForIdentity
            } else if let Some(UnitFact::WaitingForCopy) = fact {
                SettingsUnitStatus::WaitingForSharedCopy
            } else if let Some(UnitFact::LocalDifferent(r)) = fact {
                SettingsUnitStatus::LocalDifferent(*r)
            } else if let Some(UnitFact::NoSharedCopy) = fact {
                SettingsUnitStatus::NoSharedCopy
            } else if let Some(UnitFact::InSync(_)) = fact {
                SettingsUnitStatus::InSync
            } else {
                SettingsUnitStatus::Active
            };
            let local_copies = local
                .units
                .iter()
                .find(|u| u.unit == unit)
                .map(|u| {
                    u.kept
                        .iter()
                        .map(|k| SettingsHistoryEntry {
                            hash: k.checkpoint,
                            kind: wow_sync_core::kinds::RevisionKind::Auto,
                            checkpoint: k.checkpoint,
                            issuer: wow_sync_core::kinds::Issuer {
                                computer_id: own.computer_id.clone(),
                                enrollment_id: own.enrollment_id.clone(),
                                issued_at_seq: 0,
                            },
                            written_unix_ms: k.at_unix_ms,
                            is_head: false,
                        })
                        .collect()
                })
                .unwrap_or_default();
            SettingsUnitRow {
                shown_heads: if competing.is_empty() {
                    Vec::new()
                } else {
                    graph.heads.clone()
                },
                live: matches!(
                    status,
                    SettingsUnitStatus::Active | SettingsUnitStatus::InSync
                ),
                restore_possible: actionable && !history.is_empty(),
                can_share: actionable
                    && latest_capture.is_some()
                    && !matches!(status, SettingsUnitStatus::Suspect),
                in_sync_from: match fact {
                    Some(UnitFact::InSync(Some(c))) if *c != own.computer_id => {
                        names.get(c).cloned()
                    }
                    _ => None,
                },
                local_copies,
                local_reason: match fact {
                    Some(UnitFact::LocalDifferent(r)) => Some(*r),
                    _ => None,
                },
                waiting_for: match fact {
                    Some(UnitFact::Waiting(w)) => Some(w.clone()),
                    _ => None,
                },
                size_bytes: match fact {
                    Some(UnitFact::TooLarge(s)) => Some(*s),
                    _ => None,
                },
                shared_with: match fact {
                    Some(UnitFact::Several(v)) => v.clone(),
                    _ => Vec::new(),
                },
                latest_capture,
                competing,
                history,
                status,
                unit,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// OD-30: the live code of the settings path reads files in fixed buffers only. The
    /// allocator guard in `wow-sync-adapters` measures the streams; this keeps the callers from
    /// reading a file whole.
    #[test]
    fn the_settings_path_never_reads_a_whole_file_into_memory() {
        for (name, src) in [
            ("settings_cycle", include_str!("settings_cycle.rs")),
            ("effects", include_str!("effects.rs")),
            ("service", include_str!("service.rs")),
        ] {
            let live = src.split("#[cfg(test)]").next().unwrap();
            for whole in ["read_to_end(", "fs::read(", ".get_blob(", "read_to_string("] {
                assert!(
                    !live.contains(whole),
                    "{name} reads a whole file with {whole}"
                );
            }
        }
    }
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
        let off = observe_scope(t.path(), &decls(), &included(), None).unwrap();
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
    fn a_file_still_changing_across_the_settle_wait_is_suspect_and_a_stable_one_is_not() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        let rel = "Account/ERIC/SavedVariables/Details.lua";
        put(&wtf, rel, b"DetailsDB = { v = 1 }");
        let scope = observe_scope(t.path(), &decls(), &included(), Some(1024)).unwrap();
        let store = SettingsStore::open(t.path()).unwrap();
        let local = SettingsLocal {
            units: vec![],
            paused_folders: vec![],
            included_accounts: vec![],
            consent: true,
            first_adoption: None,
            share_when_ready: false,
        };
        let stable = suspect_units(t.path(), &store, &local, &scope.owned, 1024, &|| {});
        assert!(stable.is_empty(), "{stable:?}");
        let live = wtf.join("Account/ERIC/SavedVariables/Details.lua");
        let busy = suspect_units(t.path(), &store, &local, &scope.owned, 1024, &|| {
            std::fs::write(&live, b"DetailsDB = { v = 2 }").unwrap();
        });
        assert_eq!(busy.len(), 1, "a file that changed during the wait waits");
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
