//! Port-trait implementations over the adapters, and the failure-injecting wrapper.

use crate::activity::ActivitySource;
use wow_sync_adapters::authority_store;
use wow_sync_adapters::fs_install::SettingsSlot as AdSettingsSlot;
use wow_sync_adapters::fs_install::{InstallArea, InstallLock, Slot as AdSlot};
use wow_sync_adapters::fs_safe::{ReparsePolicy, Root};
use wow_sync_adapters::fs_scan::{snapshot_folder, ScannedFile, Snapshot};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::{ObjectObs, ProviderRoot};
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::exec::{
    ApplyEffects, EffectError, EffectErrorKind, ObjectStat, PublishEffects, SettingsSlot, Slot,
    SlotObs,
};
use wow_sync_core::hash::tree_hash;
use wow_sync_core::ids::ComputerId;
use wow_sync_core::ids::EnrollmentId;
use wow_sync_core::ids::{GroupId, OpId, Sha256};
use wow_sync_core::inventory::FileEntry;
use wow_sync_core::layout;
use wow_sync_core::layout::blob;
use wow_sync_core::paths::{validate_rel_path, FolderKey};
use wow_sync_core::plan::{Activity, FolderApply, ObjectRef, ObjectSource};
use wow_sync_core::records::{
    decode, decode_point, encode, AnyPoint, ApplyJournal, PendingReport, RecoveryPoint,
};
use wow_sync_core::records::{ApplyJournalV2, PendingRecord, RecoveryPointV2, SettingsFileOp};
use wow_sync_core::report::{finalized_result, record_pending};

const OBJECT_LIMIT: u64 = 512 * 1024 * 1024;
const AUTHORITY_LIMIT: u64 = 16 * 1024 * 1024;
const SETTINGS_BLOB_LIMIT: u64 = 256 * 1024 * 1024;
const JOURNAL_LIMIT: usize = 1024 * 1024;
pub(crate) const PENDING_REPORT: &str = "pending-report.json";
const PENDING_LIMIT: u64 = 1024 * 1024;

/// The durable apply report (publication.md §4). An unreadable or corrupt store counts as
/// empty: losing a confirmation must not block applies, and the next apply rewrites it.
pub(crate) fn load_pending(state: &LocalState) -> Option<PendingReport> {
    state
        .read(PENDING_REPORT, PENDING_LIMIT)
        .ok()
        .flatten()
        .and_then(|b| decode(&b, PENDING_LIMIT as usize).ok())
}

/// Where a finalized apply is recorded before its journal is removed.
pub(crate) struct PendingSink<'a> {
    pub state: &'a LocalState,
    pub group: GroupId,
    pub enrollment: EnrollmentId,
}

pub(crate) fn map_err(e: FsError) -> EffectError {
    let kind = match e.kind {
        FsErrorKind::NotFound => EffectErrorKind::NotFound,
        FsErrorKind::AlreadyExists => EffectErrorKind::Exists,
        FsErrorKind::InUse => EffectErrorKind::InUse,
        FsErrorKind::CloudOnly => EffectErrorKind::CloudOnly,
        FsErrorKind::HashMismatch => EffectErrorKind::HashMismatch,
        FsErrorKind::OutsideRoot | FsErrorKind::ReparsePoint | FsErrorKind::NotPlainFile => {
            EffectErrorKind::Unsafe
        }
        FsErrorKind::TooLarge | FsErrorKind::Io | FsErrorKind::Unsupported => EffectErrorKind::Io,
    };
    EffectError::new(kind, e.to_string())
}

fn conv(s: &Slot) -> AdSlot<'_> {
    match s {
        Slot::Live { folder } => AdSlot::Live { folder },
        Slot::Staging { op, folder } => AdSlot::Staging {
            op: op.as_str(),
            folder,
        },
        Slot::Recovery { pkg, op, folder } => AdSlot::Recovery {
            pkg: pkg.as_str(),
            op: op.as_str(),
            folder,
        },
    }
}

fn conv_settings(s: &SettingsSlot) -> AdSettingsSlot<'_> {
    match s {
        SettingsSlot::Live { rel } => AdSettingsSlot::Live { rel },
        SettingsSlot::Staging { op, idx } => AdSettingsSlot::Staging {
            op: op.as_str(),
            idx: *idx,
        },
        SettingsSlot::Recovery { op, idx } => AdSettingsSlot::Recovery {
            op: op.as_str(),
            idx: *idx,
        },
    }
}

fn snap_obs(s: &Snapshot, now_ms: i64, recent_ms: u64) -> SlotObs {
    let threshold = (now_ms as i128 - recent_ms as i128) * 1_000_000;
    SlotObs {
        fingerprint: Sha256(s.fingerprint()),
        recently_modified: s.newest_mtime_ns().is_some_and(|m| m >= threshold),
    }
}

/// Effects over the real adapters for one install and one provider folder.
pub struct RealEffects<'a> {
    pub(crate) provider: &'a ProviderRoot,
    pub(crate) addons: &'a Root,
    pub(crate) area: &'a InstallArea,
    pub(crate) variant_dir: &'a std::path::Path,
    pub(crate) group: GroupId,
    pub(crate) computer: String,
    pub(crate) activity: &'a mut dyn ActivitySource,
    pub(crate) recent_ms: u64,
    lock: Option<InstallLock>,
    // The hash computed by the immediately preceding `stat_object`; never reused later.
    just_hashed: Option<(String, Sha256)>,
    pending: Option<PendingSink<'a>>,
}

impl<'a> RealEffects<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        provider: &'a ProviderRoot,
        addons: &'a Root,
        area: &'a InstallArea,
        variant_dir: &'a std::path::Path,
        group: GroupId,
        computer: String,
        activity: &'a mut dyn ActivitySource,
        recent_ms: u64,
    ) -> Self {
        Self {
            provider,
            addons,
            area,
            variant_dir,
            group,
            computer,
            activity,
            recent_ms,
            lock: None,
            just_hashed: None,
            pending: None,
        }
    }

    /// Record every finalized (`verified`) apply in the durable report store before its
    /// journal is removed, also when recovery finishes an interrupted apply.
    pub(crate) fn recording_into(mut self, sink: PendingSink<'a>) -> Self {
        self.pending = Some(sink);
        self
    }

    fn record_finalized(&self, op: &OpId) -> Result<(), EffectError> {
        let Some(sink) = &self.pending else {
            return Ok(());
        };
        let journals = self.area.read_journals().map_err(map_err)?;
        let Some((_, bytes)) = journals.iter().find(|(o, _)| o == op.as_str()) else {
            return Ok(());
        };
        let Some(rep) = decode::<ApplyJournal>(bytes, JOURNAL_LIMIT)
            .ok()
            .as_ref()
            .and_then(finalized_result)
        else {
            return Ok(());
        };
        let rec = record_pending(load_pending(sink.state), &sink.group, &sink.enrollment, rep);
        sink.state
            .write_atomic(PENDING_REPORT, &encode(&rec))
            .map_err(map_err)
    }

    pub(crate) fn hold_lock(&mut self) -> Result<(), EffectError> {
        if self.lock.is_none() {
            self.lock = Some(self.area.lock().map_err(map_err)?);
        }
        Ok(())
    }

    fn stage_from_recovery(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError> {
        let found = local_release(self.variant_dir, &f.name, &f.tree.tree)
            .ok_or_else(|| EffectError::new(EffectErrorKind::NotFound, "release not held"))?;
        let work = Root::open(&self.variant_dir.join(".wowsync"), ReparsePolicy::RejectAll)
            .map_err(map_err)?;
        for file in &f.tree.files {
            let rel = format!(
                "recovery/{}/{}/{}/{}",
                found.pkg,
                found.op,
                f.name,
                file.path.as_str()
            );
            let mut src = work.open_read(&rel, OBJECT_LIMIT).map_err(map_err)?;
            self.area
                .stage_file(
                    op.as_str(),
                    &f.name,
                    file.path.as_str(),
                    &mut src,
                    file.sha256.0,
                    file.size,
                )
                .map_err(map_err)?;
        }
        Ok(())
    }
}

/// A release of one folder held in a local recovery point.
pub(crate) struct LocalRelease {
    pkg: String,
    op: String,
    pub files: Vec<FileEntry>,
}

/// The folder `name` with tree `tree`, if some local recovery point still holds exactly that
/// content (the scan is verified against the tree hash, never trusted from the metadata).
pub(crate) fn local_release(
    variant_dir: &std::path::Path,
    name: &str,
    tree: &Sha256,
) -> Option<LocalRelease> {
    let area = InstallArea::open(variant_dir).ok()?;
    let work = Root::open(&variant_dir.join(".wowsync"), ReparsePolicy::RejectAll).ok()?;
    for pkg in work.list("recovery").ok()?.into_iter().filter(|e| e.is_dir) {
        let Ok(points) = area.list_recovery(&pkg.name) else {
            continue;
        };
        for (op, bytes) in points {
            let replaced = match decode_point(&bytes) {
                Ok(AnyPoint::V1(p)) => p.replaced,
                Ok(AnyPoint::V2(p)) => p.replaced,
                Err(_) => continue,
            };
            if !replaced.iter().any(|t| t.name == name && t.tree == *tree) {
                continue;
            }
            let slot = AdSlot::Recovery {
                pkg: &pkg.name,
                op: &op,
                folder: name,
            };
            let Ok(Some(files)) = area.hash_slot(slot) else {
                continue;
            };
            let Ok(mut files) = entries(files) else {
                continue;
            };
            files.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
            if tree_hash(&files) == *tree {
                return Some(LocalRelease {
                    pkg: pkg.name,
                    op,
                    files,
                });
            }
        }
    }
    None
}

/// One retained recovery point and the release it holds (rollback.md §7). Only points whose
/// every replaced folder still hashes to its recorded tree are listed: the scan is verified,
/// never trusted from the metadata.
pub(crate) struct LocalPoint {
    pub package: FolderKey,
    pub op: String,
    pub created_unix_ms: u64,
    pub reason: String,
    pub pinned: bool,
    pub folders: Vec<(FolderKey, String, Sha256)>,
    /// The package hash of the held release; what a rollback request names it by.
    pub release: Sha256,
    pub version: Option<String>,
}

pub(crate) fn local_points(variant_dir: &std::path::Path) -> Vec<LocalPoint> {
    let mut out = Vec::new();
    let Ok(area) = InstallArea::open(variant_dir) else {
        return out;
    };
    let Ok(work) = Root::open(&variant_dir.join(".wowsync"), ReparsePolicy::RejectAll) else {
        return out;
    };
    let Ok(pkgs) = work.list("recovery") else {
        return out;
    };
    for pkg in pkgs.into_iter().filter(|e| e.is_dir) {
        let Ok(package) = wow_sync_core::paths::validate_folder_name(&pkg.name) else {
            continue;
        };
        let Ok(points) = area.list_recovery(&pkg.name) else {
            continue;
        };
        for (op, bytes) in points {
            let (replaced, created_unix_ms, reason, pinned) = match decode_point(&bytes) {
                Ok(AnyPoint::V1(p)) => (p.replaced, p.created_unix_ms, p.reason, p.pinned),
                Ok(AnyPoint::V2(p)) => (p.replaced, p.created_unix_ms, p.reason, p.pinned),
                Err(_) => continue,
            };
            let mut folders = Vec::new();
            for t in &replaced {
                let Ok(key) = wow_sync_core::paths::validate_folder_name(&t.name) else {
                    break;
                };
                let slot = AdSlot::Recovery {
                    pkg: &pkg.name,
                    op: &op,
                    folder: &t.name,
                };
                let held = match area.hash_slot(slot) {
                    Ok(Some(files)) => entries(files).ok(),
                    _ => None,
                };
                let Some(mut files) = held else { break };
                files.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
                if tree_hash(&files) != t.tree {
                    break;
                }
                folders.push((key, t.name.clone(), t.tree));
            }
            if folders.is_empty() || folders.len() != replaced.len() {
                continue;
            }
            let release = wow_sync_core::hash::package_hash(
                &folders
                    .iter()
                    .map(|(k, _, t)| (k.clone(), *t))
                    .collect::<Vec<_>>(),
            );
            let version = folders
                .iter()
                .find(|(k, _, _)| *k == package)
                .and_then(|(_, n, _)| {
                    use std::io::Read;
                    let mut f = work
                        .open_read(&format!("recovery/{}/{op}/{n}/{n}.toc", pkg.name), u64::MAX)
                        .ok()?;
                    let mut b = Vec::new();
                    let cap = wow_sync_core::toc::TOC_LIMIT as u64 + 1;
                    (&mut f).take(cap).read_to_end(&mut b).ok()?;
                    wow_sync_core::toc::parse_toc(&b).ok()?.version
                })
                .map(|v| v.chars().take(100).collect());
            out.push(LocalPoint {
                package: package.clone(),
                op,
                created_unix_ms,
                reason,
                pinned,
                folders,
                release,
                version,
            });
        }
    }
    out
}

impl PublishEffects for RealEffects<'_> {
    fn stat_object(&mut self, rel: &str) -> Result<ObjectStat, EffectError> {
        self.just_hashed = None;
        match self
            .provider
            .check_object(rel, OBJECT_LIMIT)
            .map_err(map_err)?
        {
            ObjectObs::Missing => Ok(ObjectStat::Missing),
            ObjectObs::CloudOnly => Ok(ObjectStat::CloudOnly),
            ObjectObs::Local {
                size,
                mtime_ns,
                sha256,
            } => {
                self.just_hashed = Some((rel.to_string(), Sha256(sha256)));
                Ok(ObjectStat::Local { size, mtime_ns })
            }
        }
    }

    fn hash_object(&mut self, rel: &str) -> Result<Sha256, EffectError> {
        if let Some((r, h)) = self.just_hashed.take() {
            if r == rel {
                return Ok(h);
            }
        }
        match self
            .provider
            .check_object(rel, OBJECT_LIMIT)
            .map_err(map_err)?
        {
            ObjectObs::Local { sha256, .. } => Ok(Sha256(sha256)),
            ObjectObs::Missing => Err(EffectError::new(EffectErrorKind::NotFound, rel)),
            ObjectObs::CloudOnly => Err(EffectError::new(EffectErrorKind::CloudOnly, rel)),
        }
    }

    fn put_object(&mut self, obj: &ObjectRef) -> Result<(u64, i128), EffectError> {
        match &obj.source {
            ObjectSource::Folder { folder, path } => {
                let src = self
                    .addons
                    .join(&format!("{folder}/{}", path.as_str()))
                    .map_err(map_err)?;
                self.provider
                    .put_object(&obj.rel, &src, obj.expect.0, &self.computer)
                    .map_err(map_err)
            }
            ObjectSource::Bytes(b) => self
                .provider
                .put_object_bytes(&obj.rel, b, obj.expect.0, &self.computer)
                .map_err(map_err),
        }
    }

    fn observe_source(&mut self, folder: &str) -> Result<Option<SlotObs>, EffectError> {
        let snap = snapshot_folder(self.addons, folder).map_err(map_err)?;
        let now = wow_sync_adapters::sysinfo::now_unix_ms();
        Ok(snap.map(|s| snap_obs(&s, now, self.recent_ms)))
    }

    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError> {
        self.provider
            .write_atomic(rel, bytes, &self.computer)
            .map_err(map_err)
    }

    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError> {
        self.provider.remove_file(rel).map_err(map_err)
    }
}

fn entries(files: Vec<ScannedFile>) -> Result<Vec<FileEntry>, EffectError> {
    files
        .into_iter()
        .map(|f| {
            validate_rel_path(&f.rel_path)
                .map(|path| FileEntry {
                    path,
                    size: f.size,
                    sha256: Sha256(f.sha256),
                })
                .map_err(|e| EffectError::new(EffectErrorKind::Unsafe, e.to_string()))
        })
        .collect()
}

impl ApplyEffects for RealEffects<'_> {
    fn activity(&mut self) -> Activity {
        self.activity.observe(self.variant_dir)
    }

    fn lock(&mut self) -> Result<(), EffectError> {
        self.hold_lock()
    }

    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError> {
        self.area
            .write_journal(j.op.as_str(), &encode(j))
            .map_err(map_err)
    }

    fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.area.read_journals().map_err(map_err)
    }

    fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError> {
        // If recording fails the journal stays, so finalize is retried later.
        self.record_finalized(op)?;
        self.area.remove_journal(op.as_str()).map_err(map_err)
    }

    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError> {
        for file in &f.tree.files {
            let rel = blob(&self.group, &file.sha256);
            let mut src = match self.provider.open_read(&rel, OBJECT_LIMIT) {
                Ok(s) => s,
                // A file the shared folder no longer holds may still lie in a local recovery
                // point (rollback.md §1 step 4); the staged copy is hash-verified either way.
                Err(e) if e.kind == FsErrorKind::NotFound => {
                    return self.stage_from_recovery(op, f);
                }
                Err(e) => return Err(map_err(e)),
            };
            if let Err(e) = self.area.stage_file(
                op.as_str(),
                &f.name,
                file.path.as_str(),
                &mut src,
                file.sha256.0,
                file.size,
            ) {
                let e = map_err(e);
                // A shared copy that fails its hash may still be held intact in a local recovery
                // point; that copy is verified the same way.
                if e.kind == EffectErrorKind::HashMismatch {
                    return self.stage_from_recovery(op, f).map_err(|_| e);
                }
                return Err(e);
            }
        }
        Ok(())
    }

    fn tree_of(&mut self, slot: &Slot) -> Result<Option<Sha256>, EffectError> {
        match self.area.hash_slot(conv(slot)).map_err(map_err)? {
            None => Ok(None),
            Some(files) => Ok(Some(tree_hash(&entries(files)?))),
        }
    }

    fn observe(&mut self, slot: &Slot) -> Result<Option<SlotObs>, EffectError> {
        let snap = self.area.snapshot_slot(conv(slot)).map_err(map_err)?;
        let now = wow_sync_adapters::sysinfo::now_unix_ms();
        Ok(snap.map(|s| snap_obs(&s, now, self.recent_ms)))
    }

    fn rename(&mut self, from: &Slot, to: &Slot) -> Result<(), EffectError> {
        self.area.rename(conv(from), conv(to)).map_err(map_err)
    }

    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError> {
        self.area
            .write_recovery_meta(rp.package.as_str(), rp.op.as_str(), &encode(rp))
            .map_err(map_err)
    }

    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.area.list_recovery(pkg.as_str()).map_err(map_err)
    }

    fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError> {
        self.area
            .remove_recovery(pkg.as_str(), op.as_str())
            .map_err(map_err)?;
        self.area
            .remove_settings_recovery(op.as_str())
            .map_err(map_err)
    }

    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.area.remove_staging(op.as_str()).map_err(map_err)
    }

    fn write_journal_v2(&mut self, j: &ApplyJournalV2) -> Result<(), EffectError> {
        self.area
            .write_journal(j.op.as_str(), &encode(j))
            .map_err(map_err)
    }

    fn write_recovery_point_v2(&mut self, rp: &RecoveryPointV2) -> Result<(), EffectError> {
        self.area
            .write_recovery_meta(rp.package.as_str(), rp.op.as_str(), &encode(rp))
            .map_err(map_err)
    }

    fn write_pending_record(&mut self, p: &PendingRecord) -> Result<(), EffectError> {
        let me = ComputerId::parse(&self.computer)
            .map_err(|e| EffectError::new(EffectErrorKind::Unsafe, e.to_string()))?;
        let rel = layout::authority_record(&self.group, &me, &p.hash);
        authority_store::put_record(
            self.provider,
            &rel,
            p.bytes.as_bytes(),
            AUTHORITY_LIMIT,
            &self.computer,
        )
        .map(|_| ())
        .map_err(map_err)
    }

    fn settings_stage(
        &mut self,
        op: &OpId,
        idx: usize,
        o: &SettingsFileOp,
    ) -> Result<(), EffectError> {
        let Some(h) = o.staged else {
            return Err(EffectError::new(
                EffectErrorKind::Unsafe,
                "a settings file with nothing staged",
            ));
        };
        let rel = layout::settings_blob(&h);
        let Ok(mut src) = self.provider.open_read(&rel, SETTINGS_BLOB_LIMIT) else {
            // A checkpoint captured on this computer (an undone uninstall) is in the local store.
            let local = wow_sync_adapters::settings_store::SettingsStore::open(self.variant_dir)
                .map_err(map_err)?
                .get_blob(&h.0, SETTINGS_BLOB_LIMIT)
                .map_err(map_err)?
                .ok_or_else(|| {
                    EffectError::new(EffectErrorKind::Io, "the settings copy is not available")
                })?;
            let size = local.len() as u64;
            return self
                .area
                .stage_settings(op.as_str(), idx, &mut local.as_slice(), h.0, size)
                .map_err(map_err);
        };
        let size = src
            .metadata()
            .map_err(|e| EffectError::new(EffectErrorKind::Io, e.to_string()))?
            .len();
        self.area
            .stage_settings(op.as_str(), idx, &mut src, h.0, size)
            .map_err(map_err)
    }

    fn settings_hash(&mut self, s: &SettingsSlot) -> Result<Option<Sha256>, EffectError> {
        Ok(self
            .area
            .settings_hash(conv_settings(s))
            .map_err(map_err)?
            .map(Sha256))
    }

    fn settings_rename(
        &mut self,
        from: &SettingsSlot,
        to: &SettingsSlot,
    ) -> Result<(), EffectError> {
        self.area
            .settings_rename(conv_settings(from), conv_settings(to))
            .map_err(map_err)
    }

    fn discard_settings_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.area
            .remove_settings_staging(op.as_str())
            .map_err(map_err)
    }
}

/// Stop the cycle after this many mutating effects (a simulated process crash).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaultPlan {
    pub crash_after: Option<usize>,
}

/// Test and developer hooks: simulate an external writer between two effects.
#[derive(Default)]
pub struct Hooks {
    pub after_stage: Option<Box<dyn FnMut() + Send>>,
    pub after_put: Option<Box<dyn FnMut() + Send>>,
}

/// Wraps effects, counts mutating calls and makes every later call fail with
/// `Crashed` once the planned count is reached. State already written is kept.
pub struct Faulty<'h, E> {
    inner: E,
    plan: FaultPlan,
    hooks: &'h mut Hooks,
    count: usize,
    crashed: bool,
}

impl<'h, E> Faulty<'h, E> {
    pub fn new(inner: E, plan: FaultPlan, hooks: &'h mut Hooks) -> Self {
        Self {
            inner,
            plan,
            hooks,
            count: 0,
            crashed: false,
        }
    }

    pub fn starting_at(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    pub fn crashed(&self) -> bool {
        self.crashed
    }

    pub fn effects_run(&self) -> usize {
        self.count
    }

    pub fn inner_mut(&mut self) -> &mut E {
        &mut self.inner
    }

    fn crash_error() -> EffectError {
        EffectError::new(EffectErrorKind::Crashed, "simulated crash")
    }

    fn read_gate(&self) -> Result<(), EffectError> {
        if self.crashed {
            Err(Self::crash_error())
        } else {
            Ok(())
        }
    }

    fn effect_gate(&mut self) -> Result<(), EffectError> {
        self.read_gate()?;
        if self.plan.crash_after.is_some_and(|k| self.count >= k) {
            self.crashed = true;
            return Err(Self::crash_error());
        }
        self.count += 1;
        Ok(())
    }
}

impl<E: PublishEffects> PublishEffects for Faulty<'_, E> {
    fn stat_object(&mut self, rel: &str) -> Result<ObjectStat, EffectError> {
        self.read_gate()?;
        self.inner.stat_object(rel)
    }
    fn hash_object(&mut self, rel: &str) -> Result<Sha256, EffectError> {
        self.read_gate()?;
        self.inner.hash_object(rel)
    }
    fn put_object(&mut self, obj: &ObjectRef) -> Result<(u64, i128), EffectError> {
        self.effect_gate()?;
        let r = self.inner.put_object(obj)?;
        if let Some(h) = self.hooks.after_put.as_mut() {
            h();
        }
        Ok(r)
    }
    fn observe_source(&mut self, folder: &str) -> Result<Option<SlotObs>, EffectError> {
        self.read_gate()?;
        self.inner.observe_source(folder)
    }
    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_record(rel, bytes)
    }
    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.remove_own_record(rel)
    }
}

impl<E: ApplyEffects> ApplyEffects for Faulty<'_, E> {
    fn activity(&mut self) -> Activity {
        if self.crashed {
            return Activity::Unknown("simulated crash".into());
        }
        self.inner.activity()
    }
    fn lock(&mut self) -> Result<(), EffectError> {
        self.read_gate()?;
        self.inner.lock()
    }
    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_journal(j)
    }
    fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.read_gate()?;
        self.inner.read_journals()
    }
    fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.remove_journal(op)
    }
    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.stage_folder(op, f)?;
        if let Some(h) = self.hooks.after_stage.as_mut() {
            h();
        }
        Ok(())
    }
    fn tree_of(&mut self, slot: &Slot) -> Result<Option<Sha256>, EffectError> {
        self.read_gate()?;
        self.inner.tree_of(slot)
    }
    fn observe(&mut self, slot: &Slot) -> Result<Option<SlotObs>, EffectError> {
        self.read_gate()?;
        self.inner.observe(slot)
    }
    fn rename(&mut self, from: &Slot, to: &Slot) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.rename(from, to)
    }
    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_recovery_point(rp)
    }
    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.read_gate()?;
        self.inner.list_recovery(pkg)
    }
    fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.remove_recovery(pkg, op)
    }
    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.discard_staging(op)
    }
    fn write_journal_v2(&mut self, j: &ApplyJournalV2) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_journal_v2(j)
    }
    fn write_recovery_point_v2(&mut self, rp: &RecoveryPointV2) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_recovery_point_v2(rp)
    }
    fn write_pending_record(&mut self, p: &PendingRecord) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.write_pending_record(p)
    }
    fn settings_stage(
        &mut self,
        op: &OpId,
        idx: usize,
        o: &SettingsFileOp,
    ) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.settings_stage(op, idx, o)
    }
    fn settings_hash(&mut self, s: &SettingsSlot) -> Result<Option<Sha256>, EffectError> {
        self.read_gate()?;
        self.inner.settings_hash(s)
    }
    fn settings_rename(
        &mut self,
        from: &SettingsSlot,
        to: &SettingsSlot,
    ) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.settings_rename(from, to)
    }
    fn discard_settings_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.effect_gate()?;
        self.inner.discard_settings_staging(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::ScriptedActivity;
    use std::path::PathBuf;
    use wow_sync_adapters::testkit::{SyntheticInstall, TempDir};
    use wow_sync_core::exec::{recover_interrupted, run_apply, PackageResult};
    use wow_sync_core::explain::Explanation;
    use wow_sync_core::hash::{sha256, tree_hash};
    use wow_sync_core::ids::{EnrollmentId, Identity, SettingsUnit};
    use wow_sync_core::inventory::FileEntry;
    use wow_sync_core::kinds::{Ack, AckItem, AckResult, AckSettings, Issuer};
    use wow_sync_core::paths::{account_settings_path, validate_rel_path};
    use wow_sync_core::plan::{ApplyPlan, ExpectedLive, FolderAction, PackageApply, RemoveApply};
    use wow_sync_core::records::{
        FolderTreeRecord, Location, OpKind, RecoverySlot, RootState, SettingsRoot,
    };

    fn fk(s: &str) -> FolderKey {
        wow_sync_core::paths::validate_folder_name(s).unwrap()
    }

    const SETTINGS_REL: &str = "Account/ACC/SavedVariables/Alpha.lua";

    fn hex(c: char) -> String {
        c.to_string().repeat(32)
    }

    /// A synthetic install, provider folder and the plan inputs, all under one temp dir.
    struct Lab {
        _t: TempDir,
        inst: SyntheticInstall,
        provider: ProviderRoot,
        addons: Root,
        area: InstallArea,
        group: GroupId,
    }

    impl Lab {
        fn new() -> Lab {
            let t = TempDir::new();
            let inst = SyntheticInstall::new_forever(t.path(), "wow");
            inst.add_addon("Alpha", "Alpha.toc", "## T\n", &[("a.lua", b"old")]);
            inst.add_addon("AlphaX", "AlphaX.toc", "## T\n", &[("x.lua", b"x")]);
            let settings = inst.variant_dir.join("WTF").join(SETTINGS_REL);
            std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
            std::fs::write(&settings, b"old settings").unwrap();
            inst.set_old_mtimes();
            wow_sync_adapters::testkit::set_old_mtimes_under(&inst.variant_dir.join("WTF"));
            std::fs::create_dir_all(t.join("dropbox")).unwrap();
            let provider = ProviderRoot::open_or_create(&t.join("dropbox")).unwrap();
            let addons = Root::open(
                &inst.variant_dir.join("Interface").join("AddOns"),
                wow_sync_adapters::fs_safe::ReparsePolicy::RejectAll,
            )
            .unwrap();
            let area = InstallArea::open(&inst.variant_dir).unwrap();
            Lab {
                _t: t,
                inst,
                provider,
                addons,
                area,
                group: GroupId::parse(&hex('1')).unwrap(),
            }
        }

        fn fx<'a>(&'a self, act: &'a mut ScriptedActivity) -> RealEffects<'a> {
            RealEffects::new(
                &self.provider,
                &self.addons,
                &self.area,
                &self.inst.variant_dir,
                self.group.clone(),
                hex('2'),
                act,
                0,
            )
        }

        fn put_blob(&self, rel: &str, bytes: &[u8]) {
            self.provider.write_atomic(rel, bytes, &hex('2')).unwrap();
        }

        fn live_file(&self, rel: &str) -> Option<String> {
            std::fs::read_to_string(self.inst.variant_dir.join(rel)).ok()
        }

        fn expected_dir(&self, name: &str) -> ExpectedLive {
            let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
            let mut fx = self.fx(&mut act);
            let slot = Slot::Live {
                folder: name.into(),
            };
            ExpectedLive::Tree {
                tree: fx.tree_of(&slot).unwrap().unwrap(),
                fingerprint: fx.observe(&slot).unwrap().unwrap().fingerprint,
            }
        }

        /// A `/2` plan of the given kind over the lab's old state.
        fn plan(&self, kind: OpKind) -> ApplyPlan {
            let file = |p: &str, b: &[u8]| FileEntry {
                path: validate_rel_path(p).unwrap(),
                size: b.len() as u64,
                sha256: sha256(b),
            };
            let new_a = file("a.lua", b"new");
            self.put_blob(&layout::blob(&self.group, &new_a.sha256), b"new");
            let new_settings = sha256(b"new settings");
            self.put_blob(&layout::settings_blob(&new_settings), b"new settings");
            let mut p = PackageApply::code_apply(
                OpId::parse(&hex('7')).unwrap(),
                fk("alpha"),
                sha256(b"target"),
                vec![],
                Explanation::Installed {
                    source: ComputerId::parse(&hex('3')).unwrap(),
                },
            );
            p.kind = kind;
            let unit = SettingsUnit {
                folder: fk("alpha"),
                identity: Identity::account("ACC").unwrap(),
            };
            p.settings_ops = vec![wow_sync_core::records::SettingsFileOp {
                unit: unit.clone(),
                live_rel: account_settings_path("ACC", "Alpha").unwrap(),
                expected_live: ExpectedLive::Tree {
                    tree: sha256(b"old settings"),
                    fingerprint: sha256(b"unused"),
                },
                staged: Some(new_settings),
                recovery_slot: RecoverySlot::new("r/0").unwrap(),
            }];
            p.settings_roots = vec![SettingsRoot {
                unit,
                state: RootState::Captured {
                    checkpoint: sha256(b"checkpoint"),
                    location: Location::Local,
                },
            }];
            if kind == OpKind::Apply {
                p.folders = vec![wow_sync_core::plan::FolderApply {
                    name: "Alpha".into(),
                    key: fk("alpha"),
                    action: FolderAction::Replace,
                    tree: FolderTreeRecord {
                        tree: tree_hash(std::slice::from_ref(&new_a)),
                        files: vec![new_a],
                    },
                    expected_live: self.expected_dir("Alpha"),
                }];
                p.removes = vec![RemoveApply {
                    name: "AlphaX".into(),
                    key: fk("alphax"),
                    expected_live: self.expected_dir("AlphaX"),
                }];
                p.removals = vec![sha256(b"removal")];
                let ack = Ack {
                    group_id: self.group.clone(),
                    issuer: Issuer {
                        computer_id: ComputerId::parse(&hex('3')).unwrap(),
                        enrollment_id: EnrollmentId::parse(&hex('4')).unwrap(),
                        issued_at_seq: 4,
                    },
                    items: vec![AckItem {
                        removal: sha256(b"removal"),
                        folders: vec![fk("alphax")],
                    }],
                    result: AckResult::Absent,
                    removed: vec![],
                    settings: AckSettings::NotApplicable,
                    written_unix_ms: 8,
                };
                let bytes = encode(&ack);
                p.pending_records = vec![PendingRecord {
                    hash: sha256(&bytes),
                    bytes: String::from_utf8(bytes).unwrap(),
                }];
            }
            ApplyPlan {
                group_id: self.group.clone(),
                started_unix_ms: 5,
                packages: vec![p],
                skipped: vec![],
            }
        }

        fn record_path(&self, plan: &ApplyPlan) -> PathBuf {
            let h = plan.packages[0].pending_records[0].hash;
            self.provider.path().join(layout::authority_record(
                &self.group,
                &ComputerId::parse(&hex('2')).unwrap(),
                &h,
            ))
        }

        /// (Alpha code, AlphaX present, settings file, reserved record present).
        fn state(&self, plan: &ApplyPlan) -> (Option<String>, bool, Option<String>, bool) {
            (
                self.live_file("Interface/AddOns/Alpha/a.lua"),
                self.inst.addons.join("AlphaX").exists(),
                self.live_file(&format!("WTF/{SETTINGS_REL}")),
                !plan.packages[0].pending_records.is_empty() && self.record_path(plan).exists(),
            )
        }

        fn leftovers(&self) -> Vec<String> {
            let w = self.inst.variant_dir.join(".wowsync");
            let mut out = Vec::new();
            for d in ["journal", "staging", "settings-staging"] {
                if let Ok(rd) = std::fs::read_dir(w.join(d)) {
                    out.extend(rd.flatten().map(|e| format!("{d}/{:?}", e.file_name())));
                }
            }
            out
        }
    }

    fn old_state() -> (Option<String>, bool, Option<String>, bool) {
        (Some("old".into()), true, Some("old settings".into()), false)
    }

    #[test]
    fn a_v2_apply_with_a_remove_and_settings_runs_through_the_real_effects() {
        let lab = Lab::new();
        let plan = lab.plan(OpKind::Apply);
        let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
        let out = run_apply(&plan, &mut lab.fx(&mut act));
        assert!(
            matches!(out.packages[0].1, PackageResult::Applied { .. }),
            "{:?}",
            out.packages
        );
        assert_eq!(
            lab.state(&plan),
            (Some("new".into()), false, Some("new settings".into()), true)
        );
        assert_eq!(
            std::fs::read(lab.record_path(&plan)).unwrap(),
            plan.packages[0].pending_records[0].bytes.as_bytes()
        );
        assert!(lab.leftovers().is_empty(), "{:?}", lab.leftovers());
    }

    #[test]
    fn a_settings_only_apply_changes_just_the_file_and_keeps_the_old_copy_for_recovery() {
        let lab = Lab::new();
        let plan = lab.plan(OpKind::Settings);
        let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
        let out = run_apply(&plan, &mut lab.fx(&mut act));
        assert!(
            matches!(out.packages[0].1, PackageResult::Applied { .. }),
            "{:?}",
            out.packages
        );
        let mut want = old_state();
        want.2 = Some("new settings".into());
        assert_eq!(lab.state(&plan), want);
        assert!(lab.leftovers().is_empty(), "{:?}", lab.leftovers());
        let kept = lab
            .inst
            .variant_dir
            .join(".wowsync")
            .join("settings-recovery");
        assert!(kept.is_dir(), "the replaced file is kept for recovery");
    }

    #[test]
    fn a_crash_at_every_effect_recovers_to_exactly_old_or_new_with_the_real_adapters() {
        // T-C06c at runtime level
        let clean = Lab::new();
        let plan = clean.plan(OpKind::Apply);
        let mut hooks = Hooks::default();
        let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
        let mut f = Faulty::new(clean.fx(&mut act), FaultPlan::default(), &mut hooks);
        assert!(matches!(
            run_apply(&plan, &mut f).packages[0].1,
            PackageResult::Applied { .. }
        ));
        let n = f.effects_run();
        assert!(n > 8, "a mixed operation has many effects, got {n}");
        let new_state = clean.state(&plan);
        for k in 0..=n {
            let lab = Lab::new();
            let plan = lab.plan(OpKind::Apply);
            {
                let mut hooks = Hooks::default();
                let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
                let plan_k = FaultPlan {
                    crash_after: Some(k),
                };
                let mut f = Faulty::new(lab.fx(&mut act), plan_k, &mut hooks);
                let _ = run_apply(&plan, &mut f);
            }
            for _ in 0..6 {
                let (mut act, _h) = ScriptedActivity::new(Activity::NotRunning);
                let r = recover_interrupted(&mut lab.fx(&mut act));
                assert!(
                    r.ops.iter().all(|(_, _, res)| !matches!(
                        res,
                        PackageResult::NeedsHelp(_) | PackageResult::Failed(_)
                    )),
                    "crash after {k}: {r:?}"
                );
                if !r.journals_remaining {
                    break;
                }
            }
            let got = lab.state(&plan);
            assert!(
                got == old_state() || got == new_state,
                "crash after {k}: neither old nor new: {got:?}"
            );
            assert!(
                lab.leftovers().is_empty(),
                "crash after {k}: {:?}",
                lab.leftovers()
            );
        }
    }
}
