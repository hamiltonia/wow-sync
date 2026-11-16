//! Port-trait implementations over the adapters, and the failure-injecting wrapper.

use crate::activity::ActivitySource;
use wow_sync_adapters::fs_install::{InstallArea, InstallLock, Slot as AdSlot};
use wow_sync_adapters::fs_safe::Root;
use wow_sync_adapters::fs_scan::{snapshot_folder, ScannedFile, Snapshot};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::{ObjectObs, ProviderRoot};
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::exec::{
    ApplyEffects, EffectError, EffectErrorKind, ObjectStat, PublishEffects, Slot, SlotObs,
};
use wow_sync_core::hash::tree_hash;
use wow_sync_core::ids::EnrollmentId;
use wow_sync_core::ids::{GroupId, OpId, Sha256};
use wow_sync_core::inventory::FileEntry;
use wow_sync_core::layout::blob;
use wow_sync_core::paths::{validate_rel_path, FolderKey};
use wow_sync_core::plan::{Activity, FolderApply, ObjectRef, ObjectSource};
use wow_sync_core::records::{decode, encode, ApplyJournal, PendingReport, RecoveryPoint};
use wow_sync_core::report::{finalized_result, record_pending};

const OBJECT_LIMIT: u64 = 512 * 1024 * 1024;
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
            let mut src = self
                .provider
                .open_read(&rel, OBJECT_LIMIT)
                .map_err(map_err)?;
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
            .map_err(map_err)
    }

    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.area.remove_staging(op.as_str()).map_err(map_err)
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
}
