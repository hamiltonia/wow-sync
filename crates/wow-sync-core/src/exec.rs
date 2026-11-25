//! Port traits and the executor for publish, apply and recovery (core-api.md §7, apply.md).
//!
//! Every decision about live files is made here from observations made through the ports;
//! recovery never relies on remembered per-folder progress.

use crate::explain::Explanation;
use crate::ids::{OpId, Sha256};
use crate::layout::parse_state_name;
use crate::paths::FolderKey;
use crate::plan::{
    Activity, ApplyPlan, ExpectedLive, FolderAction, FolderApply, ObjectRef, PackageApply,
    PlanBlock, PublishContext, PublishPlan,
};
use crate::records::{
    decode_journal, decode_point, derive_paired, encode, AnyJournal, AnyPoint, ApplyJournal,
    ApplyJournalV2, ApplyReport, BasisEntry, JournalFolder, JournalFolderV2, JournalPhase, OpKind,
    PendingRecord, RecoveryPoint, RecoveryPointV2, RecoverySlot, ReportKind, ReportResult,
    SettingsFileOp, TreeRef,
};
use std::collections::{BTreeMap, BTreeSet};

/// What a settings file is called in reasons and logs: the add-on and whether the file is
/// account-wide or a character's. Never the account, realm or character name (RV11-08).
fn settings_label(o: &SettingsFileOp) -> String {
    let kind = match o.unit.identity.realm_character() {
        None => "account",
        Some(_) => "character",
    };
    format!("{} {kind} settings", o.unit.folder.as_str())
}
/// A settings file location for the `/2` slot tables (settings.md §4.3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingsSlot {
    /// WTF-relative path of the live file.
    Live { rel: String },
    /// Staged blob `n` of an operation.
    Staging { op: OpId, idx: usize },
    /// The slot a live file moves to; `n` is the index in `settings_ops`.
    Recovery { op: OpId, idx: usize },
}

fn unsupported(what: &str) -> EffectError {
    EffectError::new(
        EffectErrorKind::Unsafe,
        format!("{what} is not supported here"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    Live {
        folder: String,
    },
    Staging {
        op: OpId,
        folder: String,
    },
    Recovery {
        pkg: FolderKey,
        op: OpId,
        folder: String,
    },
}

impl Slot {
    pub fn is_live(&self) -> bool {
        matches!(self, Slot::Live { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotObs {
    pub fingerprint: Sha256,
    /// The newest entry changed within the quiescence window.
    pub recently_modified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStat {
    Missing,
    CloudOnly,
    Local { size: u64, mtime_ns: i128 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectErrorKind {
    NotFound,
    Exists,
    InUse,
    CloudOnly,
    HashMismatch,
    Unsafe,
    Io,
    Crashed,
    /// A waiting user command asked the pass to stop at a safe point; nothing is half-done.
    Yielded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectError {
    pub kind: EffectErrorKind,
    pub detail: String,
}

impl EffectError {
    pub fn new(kind: EffectErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

pub trait PublishEffects {
    fn stat_object(&mut self, rel: &str) -> Result<ObjectStat, EffectError>;
    /// Local objects only.
    fn hash_object(&mut self, rel: &str) -> Result<Sha256, EffectError>;
    /// Copy the source to a temp file in the provider, verify its hash, rename over `rel`.
    fn put_object(&mut self, obj: &ObjectRef) -> Result<(u64, i128), EffectError>;
    fn observe_source(&mut self, folder: &str) -> Result<Option<SlotObs>, EffectError>;
    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError>;
    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError>;
}

pub trait ApplyEffects {
    fn activity(&mut self) -> Activity;
    fn lock(&mut self) -> Result<(), EffectError>;
    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError>;
    /// File name (`<op>.json`) and bytes; core decodes.
    fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError>;
    fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError>;
    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError>;
    /// Content hash of the slot; `None` = absent.
    fn tree_of(&mut self, slot: &Slot) -> Result<Option<Sha256>, EffectError>;
    fn observe(&mut self, slot: &Slot) -> Result<Option<SlotObs>, EffectError>;
    /// Fails if `to` exists.
    fn rename(&mut self, from: &Slot, to: &Slot) -> Result<(), EffectError>;
    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError>;
    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<(String, Vec<u8>)>, EffectError>;
    fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError>;
    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError>;

    // `/2` operations (contracts.md §2.2). The defaults refuse, so an implementation that does
    // not support them fails an operation before any live change.
    fn write_journal_v2(&mut self, _j: &ApplyJournalV2) -> Result<(), EffectError> {
        Err(unsupported("apply-journal/2"))
    }
    fn write_recovery_point_v2(&mut self, _rp: &RecoveryPointV2) -> Result<(), EffectError> {
        Err(unsupported("recovery-point/2"))
    }
    /// Write one reserved record to `authority\` (write-if-absent, same bytes) and index it.
    fn write_pending_record(&mut self, _p: &PendingRecord) -> Result<(), EffectError> {
        Err(unsupported("pending records"))
    }
    /// Copy the blob named by `o.staged` to the staging slot of file `idx`.
    fn settings_stage(
        &mut self,
        _op: &OpId,
        _idx: usize,
        _o: &SettingsFileOp,
    ) -> Result<(), EffectError> {
        Err(unsupported("settings files"))
    }
    /// Content hash of a settings file slot; `None` = absent.
    fn settings_hash(&mut self, _s: &SettingsSlot) -> Result<Option<Sha256>, EffectError> {
        Err(unsupported("settings files"))
    }
    /// Fails if `to` exists.
    fn settings_rename(
        &mut self,
        _from: &SettingsSlot,
        _to: &SettingsSlot,
    ) -> Result<(), EffectError> {
        Err(unsupported("settings files"))
    }
    fn discard_settings_staging(&mut self, _op: &OpId) -> Result<(), EffectError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectCheck {
    VerifiedOk,
    Missing,
    Corrupt,
    Unverifiable,
}

/// Always re-hashes a local object; size and mtime are never trusted (M-05).
pub fn check_object(
    fx: &mut impl PublishEffects,
    rel: &str,
    expect: &Sha256,
) -> Result<ObjectCheck, EffectError> {
    match fx.stat_object(rel)? {
        ObjectStat::Missing => Ok(ObjectCheck::Missing),
        ObjectStat::CloudOnly => Ok(ObjectCheck::Unverifiable),
        ObjectStat::Local { .. } => Ok(if fx.hash_object(rel)? == *expect {
            ObjectCheck::VerifiedOk
        } else {
            ObjectCheck::Corrupt
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    Published {
        state_path: String,
        uploaded: u32,
    },
    Aborted(PlanBlock),
    Failed(String),
    /// Stopped between objects for a waiting command; no state record was written, and the
    /// objects already in place are reused by the next attempt.
    Yielded,
}

pub fn run_publish(
    plan: &PublishPlan,
    ctx: &mut PublishContext,
    fx: &mut impl PublishEffects,
) -> PublishOutcome {
    debug_assert_eq!(plan.state.seq, ctx.seq);
    let mut uploaded = 0;
    for o in &plan.objects {
        match check_object(fx, &o.rel, &o.expect) {
            Ok(ObjectCheck::VerifiedOk) => continue,
            Ok(_) => {}
            Err(e) if e.kind == EffectErrorKind::Yielded => return PublishOutcome::Yielded,
            Err(e) => return PublishOutcome::Failed(format!("checking {}: {e}", o.rel)),
        }
        if let Err(e) = fx.put_object(o) {
            if e.kind == EffectErrorKind::Yielded {
                return PublishOutcome::Yielded;
            }
            return PublishOutcome::Failed(format!("uploading {}: {e}", o.rel));
        }
        uploaded += 1;
    }
    for (folder, fp) in &plan.source_fingerprints {
        match fx.observe_source(folder) {
            Ok(Some(obs)) if obs.fingerprint == *fp => {}
            Ok(_) => return PublishOutcome::Aborted(PlanBlock::Unstable),
            Err(e) => return PublishOutcome::Failed(format!("re-reading {folder}: {e}")),
        }
    }
    if let Err(e) = fx.write_record(&plan.state_path, &encode(&plan.state)) {
        return PublishOutcome::Failed(format!("writing the state record: {e}"));
    }
    for old in &plan.prune_states {
        let _ = fx.remove_own_record(old);
    }
    PublishOutcome::Published {
        state_path: plan.state_path.clone(),
        uploaded,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageResult {
    Applied { recovery: OpId },
    Skipped(Explanation),
    Replanned(Explanation),
    Interrupted(Explanation),
    RolledBack(Explanation),
    NeedsHelp(Explanation),
    Failed(Explanation),
}

impl PackageResult {
    pub fn report_kind(&self) -> ReportKind {
        match self {
            Self::Applied { .. } => ReportKind::Applied,
            Self::Skipped(_) => ReportKind::Skipped,
            Self::Replanned(_) => ReportKind::Replanned,
            Self::Interrupted(_) => ReportKind::Interrupted,
            Self::RolledBack(_) => ReportKind::RolledBack,
            Self::NeedsHelp(_) => ReportKind::NeedsHelp,
            Self::Failed(_) => ReportKind::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub packages: Vec<(FolderKey, PackageResult)>,
    pub stopped: Option<PlanBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryOutcome {
    pub ops: Vec<(OpId, FolderKey, PackageResult)>,
    pub journals_remaining: bool,
}

/// `apply_report` for the next published state; only verified-and-finalized packages count as applied.
pub fn apply_report(
    plan: &ApplyPlan,
    outcome: &ApplyOutcome,
    basis: Vec<BasisEntry>,
) -> Option<ApplyReport> {
    let op = plan.packages.first()?.op.clone();
    let results: Vec<ReportResult> = outcome
        .packages
        .iter()
        .map(|(key, r)| ReportResult {
            package: key.clone(),
            result: r.report_kind(),
            package_hash: plan
                .packages
                .iter()
                .find(|p| &p.package == key)
                .map(|p| p.target_hash),
        })
        .collect();
    Some(ApplyReport { op, basis, results })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotConfig {
    Untouched,
    Half,
    Done,
    Unknown,
}

/// Pure slot classification (apply.md §3.1).
pub fn classify_slots(install: bool, live: bool, staging: bool, recovery: bool) -> SlotConfig {
    if install {
        return match (live, staging, recovery) {
            (_, true, false) => SlotConfig::Untouched,
            (true, false, false) => SlotConfig::Done,
            _ => SlotConfig::Unknown,
        };
    }
    match (live, staging, recovery) {
        (_, true, false) => SlotConfig::Untouched,
        (false, true, true) => SlotConfig::Half,
        (true, false, true) => SlotConfig::Done,
        _ => SlotConfig::Unknown,
    }
}

struct Halt {
    block: PlanBlock,
    why: Explanation,
}

fn gate(fx: &mut impl ApplyEffects, package: &str) -> Result<(), Box<Halt>> {
    match fx.activity() {
        Activity::NotRunning => Ok(()),
        Activity::Running { .. } => Err(Box::new(Halt {
            block: PlanBlock::GameRunning,
            why: Explanation::InterruptedByGame {
                package: package.to_string(),
            },
        })),
        Activity::Unknown(why) => Err(Box::new(Halt {
            block: PlanBlock::ActivityUnknown(why.clone()),
            why: Explanation::ActivityUnknown(why),
        })),
    }
}

fn live(f: &str) -> Slot {
    Slot::Live {
        folder: f.to_string(),
    }
}

fn staging(op: &OpId, f: &str) -> Slot {
    Slot::Staging {
        op: op.clone(),
        folder: f.to_string(),
    }
}

fn recovery(j: &ApplyJournal, f: &str) -> Slot {
    Slot::Recovery {
        pkg: j.package.clone(),
        op: j.op.clone(),
        folder: f.to_string(),
    }
}

fn present(fx: &mut impl ApplyEffects, s: &Slot) -> Result<bool, EffectError> {
    Ok(fx.observe(s)?.is_some())
}

fn journal_for(plan: &ApplyPlan, p: &PackageApply) -> ApplyJournal {
    ApplyJournal {
        op: p.op.clone(),
        group_id: plan.group_id.clone(),
        started_unix_ms: plan.started_unix_ms,
        package: p.package.clone(),
        target_hash: p.target_hash,
        phase: JournalPhase::Staging,
        from_phase: None,
        reason: None,
        halted_by_game: false,
        folders: p
            .folders
            .iter()
            .map(|f| JournalFolder {
                name: f.name.clone(),
                key: f.key.clone(),
                action: f.action,
                target_tree: f.tree.tree,
                expected_live: f.expected_live.clone(),
            })
            .collect(),
    }
}

fn transition(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournal,
    phase: JournalPhase,
    from: Option<JournalPhase>,
    reason: Option<String>,
) -> Result<(), EffectError> {
    j.phase = phase;
    j.from_phase = from;
    j.reason = reason;
    j.halted_by_game = false;
    fx.write_journal(j)
}

/// Records on disk that the game start stopped this update, so status can say so later.
fn record_game_halt(fx: &mut impl ApplyEffects, j: &mut ApplyJournal, h: &Halt) {
    if matches!(h.why, Explanation::InterruptedByGame { .. }) && !j.halted_by_game {
        j.halted_by_game = true;
        let _ = fx.write_journal(j);
    }
}

fn needs_help(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournal,
    reason: &str,
    why: Explanation,
) -> PackageResult {
    let _ = transition(
        fx,
        j,
        JournalPhase::NeedsHelp,
        None,
        Some(reason.to_string()),
    );
    PackageResult::NeedsHelp(why)
}

fn needs_help_explanation(j: &ApplyJournal) -> Explanation {
    explanation_for(&j.package, j.reason.as_deref())
}

fn explanation_for(package: &FolderKey, reason: Option<&str>) -> Explanation {
    let package = package.as_str().to_string();
    let reason = reason.unwrap_or_default().to_string();
    let (code, folder) = reason.split_once(':').unwrap_or((reason.as_str(), ""));
    let folder = folder.to_string();
    match code {
        "live_mismatch" => Explanation::LiveMismatch { folder },
        "recovery_mismatch" => Explanation::RecoveryMismatch { folder },
        "restore_mismatch" => Explanation::RestoreFailed { folder },
        "unexpected_staging" => Explanation::UnexpectedStaging { package },
        _ => Explanation::UnknownInterruptedState { package },
    }
}

fn failed(package: &str, detail: impl std::fmt::Display) -> PackageResult {
    PackageResult::Failed(Explanation::StagingFailed {
        package: package.to_string(),
        detail: detail.to_string(),
    })
}

pub fn run_apply(plan: &ApplyPlan, fx: &mut impl ApplyEffects) -> ApplyOutcome {
    let mut out = ApplyOutcome {
        packages: Vec::new(),
        stopped: None,
    };
    if let Err(e) = fx.lock() {
        out.stopped = Some(PlanBlock::Invalid(format!(
            "could not take the install lock: {e}"
        )));
        return out;
    }
    let mut ordered: Vec<&PackageApply> = plan.packages.iter().collect();
    ordered.sort_by(|a, b| a.package.cmp(&b.package));
    for p in ordered {
        let (result, stop) = apply_package(plan, p, fx);
        out.packages.push((p.package.clone(), result));
        if stop.is_some() {
            out.stopped = stop;
            break;
        }
    }
    out
}

type Step = (PackageResult, Option<PlanBlock>);

fn abort_staging(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournal,
    result: PackageResult,
    stop: Option<PlanBlock>,
) -> Step {
    if transition(
        fx,
        j,
        JournalPhase::RolledBack,
        Some(JournalPhase::Staging),
        None,
    )
    .is_ok()
    {
        let _ = cleanup(fx, j);
    }
    (result, stop)
}

fn interrupted(why: Explanation, block: PlanBlock) -> Step {
    (PackageResult::Interrupted(why), Some(block))
}

fn apply_package(plan: &ApplyPlan, p: &PackageApply, fx: &mut impl ApplyEffects) -> Step {
    if !journal_v2_for(plan, p).is_v1_expressible() {
        return apply_package_v2(plan, p, fx);
    }
    let name = p.package.as_str().to_string();
    let mut j = journal_for(plan, p);
    let mut folders: Vec<&FolderApply> = p.folders.iter().collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key));

    if let Err(e) = fx.write_journal(&j) {
        return (failed(&name, e), None);
    }
    for f in &folders {
        if let Err(e) = fx.stage_folder(&p.op, f) {
            let result = match e.kind {
                EffectErrorKind::NotFound | EffectErrorKind::CloudOnly => {
                    PackageResult::Skipped(Explanation::NotAvailableYet {
                        missing: Vec::new(),
                    })
                }
                EffectErrorKind::HashMismatch => {
                    PackageResult::Failed(Explanation::StagedMismatch {
                        folder: f.name.clone(),
                    })
                }
                _ => failed(&name, e),
            };
            return abort_staging(fx, &mut j, result, None);
        }
        match fx.tree_of(&staging(&p.op, &f.name)) {
            Ok(Some(t)) if t == f.tree.tree => {}
            Ok(_) => {
                return abort_staging(
                    fx,
                    &mut j,
                    PackageResult::Failed(Explanation::StagedMismatch {
                        folder: f.name.clone(),
                    }),
                    None,
                )
            }
            Err(e) => return abort_staging(fx, &mut j, failed(&name, e), None),
        }
    }

    let mut fingerprints: Vec<Option<Sha256>> = Vec::new();
    for f in &folders {
        let obs = match fx.observe(&live(&f.name)) {
            Ok(o) => o,
            Err(e) => return abort_staging(fx, &mut j, failed(&name, e), None),
        };
        let unchanged = match (&f.expected_live, &obs) {
            (ExpectedLive::Absent, None) => Some(None),
            (ExpectedLive::Tree { tree, fingerprint }, Some(o))
                if o.fingerprint == *fingerprint && !o.recently_modified =>
            {
                match fx.tree_of(&live(&f.name)) {
                    Ok(Some(t)) if t == *tree => Some(Some(o.fingerprint)),
                    Ok(_) => None,
                    Err(e) => return abort_staging(fx, &mut j, failed(&name, e), None),
                }
            }
            _ => None,
        };
        match unchanged {
            Some(fp) => fingerprints.push(fp),
            None => {
                return abort_staging(
                    fx,
                    &mut j,
                    PackageResult::Replanned(Explanation::DestinationChanged {
                        folder: f.name.clone(),
                    }),
                    None,
                )
            }
        }
    }

    // Extra check before the recovery point so that a running game leaves no trace at all.
    if let Err(h) = gate(fx, &name) {
        return abort_staging(fx, &mut j, PackageResult::Skipped(h.why), Some(h.block));
    }
    let rp = RecoveryPoint {
        op: p.op.clone(),
        package: p.package.clone(),
        created_unix_ms: plan.started_unix_ms,
        replaced: folders
            .iter()
            .filter_map(|f| match &f.expected_live {
                ExpectedLive::Tree { tree, .. } => Some(TreeRef {
                    name: f.name.clone(),
                    tree: *tree,
                }),
                ExpectedLive::Absent => None,
            })
            .collect(),
        installed: folders
            .iter()
            .filter(|f| f.action == FolderAction::Install)
            .map(|f| TreeRef {
                name: f.name.clone(),
                tree: f.tree.tree,
            })
            .collect(),
        reason: "apply".to_string(),
        pinned: true,
    };
    if let Err(e) = fx.write_recovery_point(&rp) {
        return abort_staging(fx, &mut j, failed(&name, e), None);
    }
    if let Err(e) = transition(fx, &mut j, JournalPhase::Committing, None, None) {
        return abort_staging(fx, &mut j, failed(&name, e), None);
    }

    for (i, f) in folders.iter().enumerate() {
        let (l, s, r) = (
            live(&f.name),
            staging(&p.op, &f.name),
            recovery(&j, &f.name),
        );
        if f.action == FolderAction::Replace {
            match fx.observe(&l) {
                Ok(Some(o)) if Some(o.fingerprint) == fingerprints[i] => {}
                Ok(_) => {
                    return interrupted(
                        Explanation::DestinationChanged {
                            folder: f.name.clone(),
                        },
                        PlanBlock::DestinationChanged,
                    )
                }
                Err(e) => {
                    return interrupted(
                        Explanation::InterruptedByError {
                            package: name,
                            detail: e.to_string(),
                        },
                        PlanBlock::RenameFailed(e.to_string()),
                    )
                }
            }
            if let Err(h) = gate(fx, &name) {
                record_game_halt(fx, &mut j, &h);
                return interrupted(h.why, h.block);
            }
            if let Err(e) = fx.rename(&l, &r) {
                return interrupted(
                    Explanation::InterruptedByError {
                        package: name,
                        detail: e.to_string(),
                    },
                    PlanBlock::RenameFailed(e.to_string()),
                );
            }
        }
        if let Err(h) = gate(fx, &name) {
            record_game_halt(fx, &mut j, &h);
            return interrupted(h.why, h.block);
        }
        if let Err(e) = fx.rename(&s, &l) {
            return interrupted(
                Explanation::InterruptedByError {
                    package: name,
                    detail: e.to_string(),
                },
                PlanBlock::RenameFailed(e.to_string()),
            );
        }
    }
    if let Err(e) = transition(fx, &mut j, JournalPhase::Committed, None, None) {
        return interrupted(
            Explanation::InterruptedByError {
                package: name,
                detail: e.to_string(),
            },
            PlanBlock::RenameFailed(e.to_string()),
        );
    }
    let result = verify_and_finish(fx, &mut j);
    // The game started during this update; recovery retries never set the cause.
    if let PackageResult::Interrupted(Explanation::InterruptedByGame { .. }) = &result {
        record_game_halt(
            fx,
            &mut j,
            &Halt {
                block: PlanBlock::GameRunning,
                why: Explanation::InterruptedByGame {
                    package: name.clone(),
                },
            },
        );
    }
    (result, None).with_stop()
}

trait WithStop {
    fn with_stop(self) -> Step;
}

impl WithStop for (PackageResult, Option<PlanBlock>) {
    /// A result that is waiting for the game stops the run.
    fn with_stop(self) -> Step {
        let stop = match &self.0 {
            PackageResult::Interrupted(Explanation::InterruptedByGame { .. }) => {
                Some(PlanBlock::GameRunning)
            }
            PackageResult::Interrupted(Explanation::ActivityUnknown(w)) => {
                Some(PlanBlock::ActivityUnknown(w.clone()))
            }
            _ => self.1,
        };
        (self.0, stop)
    }
}

enum VerifyFail {
    Live(String),
    Recovery(String),
    Unknown,
    Io(EffectError),
}

fn verify_slots(fx: &mut impl ApplyEffects, j: &ApplyJournal) -> Result<(), VerifyFail> {
    for f in &j.folders {
        let install = f.action == FolderAction::Install;
        let (l, s, r) = (live(&f.name), staging(&j.op, &f.name), recovery(j, &f.name));
        let io = VerifyFail::Io;
        let cfg = classify_slots(
            install,
            present(fx, &l).map_err(io)?,
            present(fx, &s).map_err(VerifyFail::Io)?,
            present(fx, &r).map_err(VerifyFail::Io)?,
        );
        if cfg != SlotConfig::Done {
            return Err(VerifyFail::Unknown);
        }
        if fx.tree_of(&l).map_err(VerifyFail::Io)? != Some(f.target_tree) {
            return Err(VerifyFail::Live(f.name.clone()));
        }
        if let ExpectedLive::Tree { tree, .. } = &f.expected_live {
            if fx.tree_of(&r).map_err(VerifyFail::Io)? != Some(*tree) {
                return Err(VerifyFail::Recovery(f.name.clone()));
            }
        }
    }
    Ok(())
}

/// Phase `committed`: gate, verify, then finalize (apply.md §2.2 steps 6-7, §3.3).
fn verify_and_finish(fx: &mut impl ApplyEffects, j: &mut ApplyJournal) -> PackageResult {
    let name = j.package.as_str().to_string();
    if let Err(h) = gate(fx, &name) {
        return PackageResult::Interrupted(h.why);
    }
    match verify_slots(fx, j) {
        Ok(()) => {}
        Err(VerifyFail::Live(f)) => {
            return needs_help(
                fx,
                j,
                &format!("live_mismatch:{f}"),
                Explanation::LiveMismatch { folder: f },
            )
        }
        Err(VerifyFail::Recovery(f)) => {
            return needs_help(
                fx,
                j,
                &format!("recovery_mismatch:{f}"),
                Explanation::RecoveryMismatch { folder: f },
            )
        }
        Err(VerifyFail::Unknown) => {
            return needs_help(
                fx,
                j,
                "unknown_state",
                Explanation::UnknownInterruptedState { package: name },
            )
        }
        Err(VerifyFail::Io(e)) => {
            return PackageResult::Interrupted(Explanation::InterruptedByError {
                package: name,
                detail: e.to_string(),
            })
        }
    }
    if let Err(e) = transition(fx, j, JournalPhase::Verified, None, None) {
        return PackageResult::Interrupted(Explanation::InterruptedByError {
            package: name,
            detail: e.to_string(),
        });
    }
    finalize(fx, j)
}

enum PointEntry {
    V1(RecoveryPoint),
    V2(RecoveryPointV2),
}

impl PointEntry {
    fn pinned(&self) -> bool {
        match self {
            Self::V1(p) => p.pinned,
            Self::V2(p) => p.pinned,
        }
    }
    fn created(&self) -> u64 {
        match self {
            Self::V1(p) => p.created_unix_ms,
            Self::V2(p) => p.created_unix_ms,
        }
    }
    /// The retention budget a point counts against: all code, or one settings unit.
    fn class(&self) -> String {
        match self {
            Self::V2(p) if p.kind == OpKind::Settings => match p.settings.first() {
                Some(r) => format!("settings:{:?}", r.unit),
                None => "settings".to_string(),
            },
            _ => "code".to_string(),
        }
    }
    fn unpin(&mut self, fx: &mut impl ApplyEffects) -> Result<(), EffectError> {
        match self {
            Self::V1(p) => {
                p.pinned = false;
                fx.write_recovery_point(p)
            }
            Self::V2(p) => {
                p.pinned = false;
                fx.write_recovery_point_v2(p)
            }
        }
    }
}

fn decode_points(
    fx: &mut impl ApplyEffects,
    pkg: &FolderKey,
) -> Result<Vec<(OpId, PointEntry)>, EffectError> {
    let mut out = Vec::new();
    for (_, bytes) in fx.list_recovery(pkg)? {
        match decode_point(&bytes) {
            Ok(AnyPoint::V1(rp)) => out.push((rp.op.clone(), PointEntry::V1(rp))),
            Ok(AnyPoint::V2(rp)) => out.push((rp.op.clone(), PointEntry::V2(rp))),
            Err(_) => {}
        }
    }
    Ok(out)
}

fn journal_ops(fx: &mut impl ApplyEffects) -> Result<BTreeSet<String>, EffectError> {
    Ok(fx
        .read_journals()?
        .into_iter()
        .map(|(n, _)| op_name(&n))
        .collect())
}

fn op_name(file: &str) -> String {
    let base = file.rsplit('/').next().unwrap_or(file);
    base.strip_suffix(".json").unwrap_or(base).to_string()
}

/// Unpin this operation's point, prune to the retention limit, discard staging (apply.md §2.5).
fn settle_points(
    fx: &mut impl ApplyEffects,
    pkg: &FolderKey,
    op: &OpId,
) -> Result<(), EffectError> {
    let mut points = decode_points(fx, pkg)?;
    for (o, rp) in &mut points {
        if o == op && rp.pinned() {
            rp.unpin(fx)?;
        }
    }
    let journals = journal_ops(fx)?;
    let mut prunable: Vec<&(OpId, PointEntry)> = points
        .iter()
        .filter(|(o, rp)| !rp.pinned() && (o == op || !journals.contains(o.as_str())))
        .collect();
    prunable.sort_by(|a, b| (b.1.created(), b.0.as_str()).cmp(&(a.1.created(), a.0.as_str())));
    // Code and each settings unit keep their own two previous points: a settings apply never
    // evicts a code rollback point, nor the reverse (A-17).
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (o, rp) in prunable {
        let n = seen.entry(rp.class()).or_insert(0);
        *n += 1;
        if *n > KEEP_RECOVERY {
            fx.remove_recovery(pkg, o)?;
        }
    }
    Ok(())
}

/// Apply.md §2.5 (verified): unpin, prune, discard staging, remove the journal last.
fn finalize(fx: &mut impl ApplyEffects, j: &ApplyJournal) -> PackageResult {
    let name = j.package.as_str().to_string();
    let result = (|| -> Result<(), EffectError> {
        settle_points(fx, &j.package, &j.op)?;
        fx.discard_staging(&j.op)?;
        fx.remove_journal(&j.op)
    })();
    match result {
        Ok(()) => PackageResult::Applied {
            recovery: j.op.clone(),
        },
        Err(e) => PackageResult::Failed(Explanation::FinalizeFailed {
            package: name,
            detail: e.to_string(),
        }),
    }
}

pub const KEEP_RECOVERY: usize = 2;

/// Apply.md §2.5 (rolled_back). Journal removal is last; each step is skipped if already done.
fn cleanup(fx: &mut impl ApplyEffects, j: &mut ApplyJournal) -> PackageResult {
    let name = j.package.as_str().to_string();
    let rolled = PackageResult::RolledBack(Explanation::RolledBack {
        package: name.clone(),
    });
    let err = |e: EffectError| {
        PackageResult::Failed(Explanation::FinalizeFailed {
            package: name.clone(),
            detail: e.to_string(),
        })
    };
    if j.from_phase == Some(JournalPhase::Committing) {
        for f in &j.folders {
            match fx.tree_of(&staging(&j.op, &f.name)) {
                Ok(None) => {}
                Ok(Some(t)) if t == f.target_tree => {}
                Ok(Some(_)) => {
                    return needs_help(
                        fx,
                        j,
                        "unexpected_staging",
                        Explanation::UnexpectedStaging {
                            package: name.clone(),
                        },
                    )
                }
                Err(e) => return err(e),
            }
        }
    }
    if let Err(e) = fx.discard_staging(&j.op) {
        return err(e);
    }
    let mut any_recovery = false;
    if j.from_phase == Some(JournalPhase::Committing) {
        for f in &j.folders {
            match present(fx, &recovery(j, &f.name)) {
                Ok(p) => any_recovery |= p,
                Err(e) => return err(e),
            }
        }
    }
    if !any_recovery {
        if let Err(e) = fx.remove_recovery(&j.package, &j.op) {
            return err(e);
        }
    }
    if let Err(e) = fx.remove_journal(&j.op) {
        return err(e);
    }
    rolled
}

pub fn recover_interrupted(fx: &mut impl ApplyEffects) -> RecoveryOutcome {
    let Ok(raw) = fx.read_journals() else {
        return RecoveryOutcome {
            ops: Vec::new(),
            journals_remaining: true,
        };
    };
    let mut items: Vec<(String, Result<AnyJournal, ()>)> = raw
        .into_iter()
        .map(|(n, b)| (op_name(&n), decode_journal(&b).map_err(|_| ())))
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let mut ops = Vec::new();
    for (file_op, parsed) in items {
        let zero = OpId::parse(&"0".repeat(32)).expect("valid id");
        match parsed {
            Err(()) => {
                let op = OpId::parse(&file_op).unwrap_or(zero);
                ops.push((
                    op,
                    FolderKey::lossy("unknown"),
                    PackageResult::NeedsHelp(Explanation::UnknownInterruptedState {
                        package: file_op,
                    }),
                ));
            }
            Ok(AnyJournal::V1(mut j)) => {
                let result = recover_one(fx, &mut j);
                ops.push((j.op.clone(), j.package.clone(), result));
            }
            Ok(AnyJournal::V2(mut j)) => {
                let result = recover_one_v2(fx, &mut j);
                ops.push((j.op.clone(), j.package.clone(), result));
            }
        }
    }
    let journals_remaining = fx.read_journals().map(|l| !l.is_empty()).unwrap_or(true);
    RecoveryOutcome {
        ops,
        journals_remaining,
    }
}

fn recover_one(fx: &mut impl ApplyEffects, j: &mut ApplyJournal) -> PackageResult {
    let name = j.package.as_str().to_string();
    match j.phase {
        JournalPhase::Staging => {
            if let Err(e) = transition(
                fx,
                j,
                JournalPhase::RolledBack,
                Some(JournalPhase::Staging),
                None,
            ) {
                return failed(&name, e);
            }
            cleanup(fx, j)
        }
        JournalPhase::Committing => match gate(fx, &name) {
            Err(h) => PackageResult::Interrupted(h.why),
            Ok(()) => rollback(fx, j),
        },
        JournalPhase::Committed => verify_and_finish(fx, j),
        JournalPhase::Verified => finalize(fx, j),
        JournalPhase::RolledBack => cleanup(fx, j),
        JournalPhase::NeedsHelp => PackageResult::NeedsHelp(needs_help_explanation(j)),
    }
}

/// Apply.md §3.2, with the F-02 re-verification of restored live content before the terminal record.
fn rollback(fx: &mut impl ApplyEffects, j: &mut ApplyJournal) -> PackageResult {
    let name = j.package.as_str().to_string();
    let folders = j.folders.clone();
    for f in folders.iter().rev() {
        let install = f.action == FolderAction::Install;
        let (l, s, r) = (live(&f.name), staging(&j.op, &f.name), recovery(j, &f.name));
        let cfg = match (present(fx, &l), present(fx, &s), present(fx, &r)) {
            (Ok(a), Ok(b), Ok(c)) => classify_slots(install, a, b, c),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return failed(&name, e),
        };
        let old = match &f.expected_live {
            ExpectedLive::Tree { tree, .. } => Some(*tree),
            ExpectedLive::Absent => None,
        };
        match cfg {
            SlotConfig::Untouched => {}
            SlotConfig::Unknown => {
                return needs_help(
                    fx,
                    j,
                    "unknown_state",
                    Explanation::UnknownInterruptedState { package: name },
                )
            }
            SlotConfig::Half => {
                if let Some(res) = restore(fx, &name, &f.name, &r, &l, old) {
                    return fail_restore(fx, j, res, &f.name);
                }
            }
            SlotConfig::Done => {
                match fx.tree_of(&l) {
                    Ok(Some(t)) if t == f.target_tree => {}
                    Ok(_) => {
                        return needs_help(
                            fx,
                            j,
                            &format!("live_mismatch:{}", f.name),
                            Explanation::LiveMismatch {
                                folder: f.name.clone(),
                            },
                        )
                    }
                    Err(e) => return failed(&name, e),
                }
                if let Err(h) = gate(fx, &name) {
                    return PackageResult::Interrupted(h.why);
                }
                if let Err(e) = fx.rename(&l, &s) {
                    return PackageResult::Failed(Explanation::RestoreFailed {
                        folder: format!("{} ({e})", f.name),
                    });
                }
                if !install {
                    if let Some(res) = restore(fx, &name, &f.name, &r, &l, old) {
                        return fail_restore(fx, j, res, &f.name);
                    }
                }
            }
        }
    }
    // F-02: whatever the slot configuration was, the live content must be the original before
    // the terminal record is written; a copy that was corrupted or changed stays as evidence.
    for f in &folders {
        let l = live(&f.name);
        let ok = match (&f.expected_live, fx.tree_of(&l)) {
            (_, Err(e)) => return failed(&name, e),
            (ExpectedLive::Tree { tree, .. }, Ok(t)) => t == Some(*tree),
            (ExpectedLive::Absent, Ok(t)) => t.is_none(),
        };
        if !ok {
            return needs_help(
                fx,
                j,
                &format!("restore_mismatch:{}", f.name),
                Explanation::RestoreFailed {
                    folder: f.name.clone(),
                },
            );
        }
    }
    if let Err(e) = transition(
        fx,
        j,
        JournalPhase::RolledBack,
        Some(JournalPhase::Committing),
        None,
    ) {
        return failed(&name, e);
    }
    cleanup(fx, j)
}

enum RestoreFail {
    Gate(Box<Halt>),
    Rename(EffectError),
    Mismatch,
    Io(EffectError),
}

/// Gate, move the recovery copy back, and check that live content is the original.
fn restore(
    fx: &mut impl ApplyEffects,
    name: &str,
    folder: &str,
    r: &Slot,
    l: &Slot,
    old: Option<Sha256>,
) -> Option<RestoreFail> {
    if let Err(h) = gate(fx, name) {
        return Some(RestoreFail::Gate(h));
    }
    if let Err(e) = fx.rename(r, l) {
        return Some(RestoreFail::Rename(e));
    }
    match fx.tree_of(l) {
        Ok(t) if t == old => None,
        Ok(_) => Some(RestoreFail::Mismatch),
        Err(e) => {
            let _ = folder;
            Some(RestoreFail::Io(e))
        }
    }
}

fn fail_restore(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournal,
    res: RestoreFail,
    folder: &str,
) -> PackageResult {
    match res {
        RestoreFail::Gate(h) => PackageResult::Interrupted(h.why),
        RestoreFail::Rename(e) => PackageResult::Failed(Explanation::RestoreFailed {
            folder: format!("{folder} ({e})"),
        }),
        RestoreFail::Io(e) => failed(j.package.as_str(), e),
        RestoreFail::Mismatch => needs_help(
            fx,
            j,
            &format!("restore_mismatch:{folder}"),
            Explanation::RestoreFailed {
                folder: folder.to_string(),
            },
        ),
    }
}

// ---------------------------------------------------------------- `/2` operations (contracts.md §2.2)
//
// One journal may mix install, replace and remove folder actions and settings file moves. Every
// action is an `Item` with three slots and follows the same slot tables (apply.md §3, removal.md
// §5.3, settings.md §4.3); the journal phase decides the direction on recovery.

#[derive(Debug, Clone)]
enum Tgt {
    Dir(Slot),
    File(SettingsSlot),
}

#[derive(Debug, Clone)]
struct Item {
    label: String,
    live: Tgt,
    staging: Tgt,
    recovery: Tgt,
    /// Live content before the operation; `None` = absent.
    old: Option<Sha256>,
    /// Live content after the operation; `None` = absent (a remove).
    new: Option<Sha256>,
    fingerprint: Option<Sha256>,
}

impl Item {
    fn classify(&self, live: bool, staging: bool, recovery: bool) -> SlotConfig {
        if self.new.is_none() {
            return match (live, recovery) {
                (true, false) => SlotConfig::Untouched,
                (false, true) => SlotConfig::Done,
                _ => SlotConfig::Unknown,
            };
        }
        classify_slots(self.old.is_none(), live, staging, recovery)
    }
}

fn items_of(j: &ApplyJournalV2) -> Vec<Item> {
    let mut folders: Vec<&JournalFolderV2> = j.folders.iter().collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key));
    let mut out = Vec::new();
    for f in folders {
        let (old, fingerprint) = match &f.expected_live {
            ExpectedLive::Tree { tree, fingerprint } => (Some(*tree), Some(*fingerprint)),
            ExpectedLive::Absent => (None, None),
        };
        if old.is_none() && f.staged.is_none() {
            continue;
        }
        out.push(Item {
            label: f.name.clone(),
            live: Tgt::Dir(live(&f.name)),
            staging: Tgt::Dir(staging(&j.op, &f.name)),
            recovery: Tgt::Dir(Slot::Recovery {
                pkg: j.package.clone(),
                op: j.op.clone(),
                folder: f.name.clone(),
            }),
            old,
            new: f.staged,
            fingerprint,
        });
    }
    for (idx, o) in j.settings_ops.iter().enumerate() {
        let old = match &o.expected_live {
            ExpectedLive::Tree { tree, .. } => Some(*tree),
            ExpectedLive::Absent => None,
        };
        if old.is_none() && o.staged.is_none() {
            continue;
        }
        out.push(Item {
            label: settings_label(o),
            live: Tgt::File(SettingsSlot::Live {
                rel: o.live_rel.as_str().to_string(),
            }),
            staging: Tgt::File(SettingsSlot::Staging {
                op: j.op.clone(),
                idx,
            }),
            recovery: Tgt::File(SettingsSlot::Recovery {
                op: j.op.clone(),
                idx,
            }),
            old,
            new: o.staged,
            fingerprint: None,
        });
    }
    out
}

fn t_present(fx: &mut impl ApplyEffects, t: &Tgt) -> Result<bool, EffectError> {
    match t {
        Tgt::Dir(s) => present(fx, s),
        Tgt::File(s) => Ok(fx.settings_hash(s)?.is_some()),
    }
}

fn t_hash(fx: &mut impl ApplyEffects, t: &Tgt) -> Result<Option<Sha256>, EffectError> {
    match t {
        Tgt::Dir(s) => fx.tree_of(s),
        Tgt::File(s) => fx.settings_hash(s),
    }
}

fn t_rename(fx: &mut impl ApplyEffects, from: &Tgt, to: &Tgt) -> Result<(), EffectError> {
    match (from, to) {
        (Tgt::Dir(a), Tgt::Dir(b)) => fx.rename(a, b),
        (Tgt::File(a), Tgt::File(b)) => fx.settings_rename(a, b),
        _ => Err(unsupported("a move between folder and file slots")),
    }
}

fn slots_of(fx: &mut impl ApplyEffects, it: &Item) -> Result<SlotConfig, EffectError> {
    let l = t_present(fx, &it.live)?;
    let s = t_present(fx, &it.staging)?;
    let r = t_present(fx, &it.recovery)?;
    Ok(it.classify(l, s, r))
}

/// The live content is what the plan saw. `strict` also demands quiescence and re-hashes.
fn live_unchanged(
    fx: &mut impl ApplyEffects,
    it: &Item,
    strict: bool,
) -> Result<bool, EffectError> {
    match (&it.live, it.old) {
        (Tgt::Dir(s), None) => Ok(fx.observe(s)?.is_none()),
        (Tgt::Dir(s), Some(tree)) => match fx.observe(s)? {
            Some(o)
                if Some(o.fingerprint) == it.fingerprint && !(strict && o.recently_modified) =>
            {
                Ok(!strict || fx.tree_of(s)? == Some(tree))
            }
            _ => Ok(false),
        },
        (Tgt::File(s), old) => Ok(fx.settings_hash(s)? == old),
    }
}

fn recovery_slot(op: &OpId, leaf: &str) -> RecoverySlot {
    RecoverySlot::new(&format!("r/{op}/{leaf}")).expect("validated operation id and key")
}

fn journal_v2_for(plan: &ApplyPlan, p: &PackageApply) -> ApplyJournalV2 {
    let mut folders: Vec<JournalFolderV2> = p
        .folders
        .iter()
        .map(|f| JournalFolderV2 {
            name: f.name.clone(),
            key: f.key.clone(),
            action: f.action,
            expected_live: f.expected_live.clone(),
            staged: Some(f.tree.tree),
            recovery_slot: recovery_slot(&p.op, f.key.as_str()),
        })
        .collect();
    folders.extend(p.removes.iter().map(|r| JournalFolderV2 {
        name: r.name.clone(),
        key: r.key.clone(),
        action: FolderAction::Remove,
        expected_live: r.expected_live.clone(),
        staged: None,
        recovery_slot: recovery_slot(&p.op, r.key.as_str()),
    }));
    let mut removals = p.removals.clone();
    removals.sort();
    removals.dedup();
    ApplyJournalV2 {
        op: p.op.clone(),
        group_id: plan.group_id.clone(),
        started_unix_ms: plan.started_unix_ms,
        kind: p.kind,
        package: p.package.clone(),
        target_hash: match p.kind {
            OpKind::Settings | OpKind::SettingsCleanup => None,
            _ => Some(p.target_hash),
        },
        phase: JournalPhase::Staging,
        from_phase: None,
        reason: None,
        halted_by_game: false,
        folders,
        removals,
        pending_records: p.pending_records.clone(),
        settings_ops: p.settings_ops.clone(),
    }
}

fn recovery_point_v2(
    plan: &ApplyPlan,
    p: &PackageApply,
    j: &ApplyJournalV2,
) -> Option<RecoveryPointV2> {
    let mut folders: Vec<&JournalFolderV2> = j.folders.iter().collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key));
    let replaced: Vec<TreeRef> = folders
        .iter()
        .filter_map(|f| match &f.expected_live {
            ExpectedLive::Tree { tree, .. } => Some(TreeRef {
                name: f.name.clone(),
                tree: *tree,
            }),
            ExpectedLive::Absent => None,
        })
        .collect();
    let installed: Vec<TreeRef> = folders
        .iter()
        .filter(|f| f.action == FolderAction::Install)
        .filter_map(|f| {
            f.staged.map(|tree| TreeRef {
                name: f.name.clone(),
                tree,
            })
        })
        .collect();
    let code = !replaced.is_empty() || !installed.is_empty();
    let paired = derive_paired(code, &p.settings_roots)?;
    Some(RecoveryPointV2 {
        op: j.op.clone(),
        group_id: plan.group_id.clone(),
        kind: j.kind,
        package: j.package.clone(),
        code_package_hash: if code { j.target_hash } else { None },
        created_unix_ms: plan.started_unix_ms,
        replaced,
        installed,
        removals: j.removals.clone(),
        settings: p.settings_roots.clone(),
        paired,
        reason: match j.kind {
            OpKind::Apply => "apply",
            OpKind::Remove => "remove",
            OpKind::Restore => "restore",
            OpKind::Rollback => "rollback",
            OpKind::Settings => "settings",
            OpKind::SettingsCleanup => "settings_cleanup",
        }
        .to_string(),
        pinned: true,
    })
}

fn transition_v2(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournalV2,
    phase: JournalPhase,
    from: Option<JournalPhase>,
    reason: Option<String>,
) -> Result<(), EffectError> {
    j.phase = phase;
    j.from_phase = from;
    j.reason = reason;
    j.halted_by_game = false;
    fx.write_journal_v2(j)
}

fn record_game_halt_v2(fx: &mut impl ApplyEffects, j: &mut ApplyJournalV2, h: &Halt) {
    if matches!(h.why, Explanation::InterruptedByGame { .. }) && !j.halted_by_game {
        j.halted_by_game = true;
        let _ = fx.write_journal_v2(j);
    }
}

fn needs_help_v2(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournalV2,
    reason: &str,
    why: Explanation,
) -> PackageResult {
    let _ = transition_v2(
        fx,
        j,
        JournalPhase::NeedsHelp,
        None,
        Some(reason.to_string()),
    );
    PackageResult::NeedsHelp(why)
}

fn abort_staging_v2(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournalV2,
    result: PackageResult,
    stop: Option<PlanBlock>,
) -> Step {
    if transition_v2(
        fx,
        j,
        JournalPhase::RolledBack,
        Some(JournalPhase::Staging),
        None,
    )
    .is_ok()
    {
        let _ = cleanup_v2(fx, j);
    }
    (result, stop)
}

fn stage_failure(name: &str, folder: &str, e: EffectError) -> PackageResult {
    match e.kind {
        EffectErrorKind::NotFound | EffectErrorKind::CloudOnly => {
            PackageResult::Skipped(Explanation::NotAvailableYet {
                missing: Vec::new(),
            })
        }
        EffectErrorKind::HashMismatch => PackageResult::Failed(Explanation::StagedMismatch {
            folder: folder.to_string(),
        }),
        _ => failed(name, e),
    }
}

fn interrupted_by(name: &str, e: EffectError) -> Step {
    interrupted(
        Explanation::InterruptedByError {
            package: name.to_string(),
            detail: e.to_string(),
        },
        PlanBlock::RenameFailed(e.to_string()),
    )
}

fn apply_package_v2(plan: &ApplyPlan, p: &PackageApply, fx: &mut impl ApplyEffects) -> Step {
    let name = p.package.as_str().to_string();
    let mut j = journal_v2_for(plan, p);
    let items = items_of(&j);
    let mut folders: Vec<&FolderApply> = p.folders.iter().collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key));

    if let Err(e) = fx.write_journal_v2(&j) {
        return (failed(&name, e), None);
    }
    for f in &folders {
        if let Err(e) = fx.stage_folder(&p.op, f) {
            return abort_staging_v2(fx, &mut j, stage_failure(&name, &f.name, e), None);
        }
        match fx.tree_of(&staging(&p.op, &f.name)) {
            Ok(Some(t)) if t == f.tree.tree => {}
            Ok(_) => {
                return abort_staging_v2(
                    fx,
                    &mut j,
                    PackageResult::Failed(Explanation::StagedMismatch {
                        folder: f.name.clone(),
                    }),
                    None,
                )
            }
            Err(e) => return abort_staging_v2(fx, &mut j, failed(&name, e), None),
        }
    }
    for (idx, o) in p.settings_ops.iter().enumerate() {
        let Some(want) = o.staged else { continue };
        if let Err(e) = fx.settings_stage(&p.op, idx, o) {
            let label = settings_label(o);
            return abort_staging_v2(fx, &mut j, stage_failure(&name, &label, e), None);
        }
        let slot = SettingsSlot::Staging {
            op: p.op.clone(),
            idx,
        };
        match fx.settings_hash(&slot) {
            Ok(Some(h)) if h == want => {}
            Ok(_) => {
                return abort_staging_v2(
                    fx,
                    &mut j,
                    PackageResult::Failed(Explanation::StagedMismatch {
                        folder: settings_label(o),
                    }),
                    None,
                )
            }
            Err(e) => return abort_staging_v2(fx, &mut j, failed(&name, e), None),
        }
    }

    for it in &items {
        match live_unchanged(fx, it, true) {
            Ok(true) => {}
            Ok(false) => {
                return abort_staging_v2(
                    fx,
                    &mut j,
                    PackageResult::Replanned(Explanation::DestinationChanged {
                        folder: it.label.clone(),
                    }),
                    None,
                )
            }
            Err(e) => return abort_staging_v2(fx, &mut j, failed(&name, e), None),
        }
    }

    if let Err(h) = gate(fx, &name) {
        return abort_staging_v2(fx, &mut j, PackageResult::Skipped(h.why), Some(h.block));
    }
    if let Some(rp) = recovery_point_v2(plan, p, &j) {
        if let Err(e) = fx.write_recovery_point_v2(&rp) {
            return abort_staging_v2(fx, &mut j, failed(&name, e), None);
        }
    }
    if let Err(e) = transition_v2(fx, &mut j, JournalPhase::Committing, None, None) {
        return abort_staging_v2(fx, &mut j, failed(&name, e), None);
    }

    for it in &items {
        if it.old.is_some() {
            match live_unchanged(fx, it, false) {
                Ok(true) => {}
                Ok(false) => {
                    return interrupted(
                        Explanation::DestinationChanged {
                            folder: it.label.clone(),
                        },
                        PlanBlock::DestinationChanged,
                    )
                }
                Err(e) => return interrupted_by(&name, e),
            }
            if let Err(h) = gate(fx, &name) {
                record_game_halt_v2(fx, &mut j, &h);
                return interrupted(h.why, h.block);
            }
            if let Err(e) = t_rename(fx, &it.live, &it.recovery) {
                return interrupted_by(&name, e);
            }
        }
        if it.new.is_some() {
            if let Err(h) = gate(fx, &name) {
                record_game_halt_v2(fx, &mut j, &h);
                return interrupted(h.why, h.block);
            }
            if let Err(e) = t_rename(fx, &it.staging, &it.live) {
                return interrupted_by(&name, e);
            }
        }
    }
    if let Err(e) = transition_v2(fx, &mut j, JournalPhase::Committed, None, None) {
        return interrupted_by(&name, e);
    }
    let result = verify_and_finish_v2(fx, &mut j);
    if let PackageResult::Interrupted(Explanation::InterruptedByGame { .. }) = &result {
        record_game_halt_v2(
            fx,
            &mut j,
            &Halt {
                block: PlanBlock::GameRunning,
                why: Explanation::InterruptedByGame {
                    package: name.clone(),
                },
            },
        );
    }
    (result, None).with_stop()
}

fn verify_items(fx: &mut impl ApplyEffects, items: &[Item]) -> Result<(), VerifyFail> {
    for it in items {
        if slots_of(fx, it).map_err(VerifyFail::Io)? != SlotConfig::Done {
            return Err(VerifyFail::Unknown);
        }
        if let Some(new) = it.new {
            if t_hash(fx, &it.live).map_err(VerifyFail::Io)? != Some(new) {
                return Err(VerifyFail::Live(it.label.clone()));
            }
        }
        if let Some(old) = it.old {
            if t_hash(fx, &it.recovery).map_err(VerifyFail::Io)? != Some(old) {
                return Err(VerifyFail::Recovery(it.label.clone()));
            }
        }
    }
    Ok(())
}

fn verify_and_finish_v2(fx: &mut impl ApplyEffects, j: &mut ApplyJournalV2) -> PackageResult {
    let name = j.package.as_str().to_string();
    if let Err(h) = gate(fx, &name) {
        return PackageResult::Interrupted(h.why);
    }
    match verify_items(fx, &items_of(j)) {
        Ok(()) => {}
        Err(VerifyFail::Live(f)) => {
            return needs_help_v2(
                fx,
                j,
                &format!("live_mismatch:{f}"),
                Explanation::LiveMismatch { folder: f },
            )
        }
        Err(VerifyFail::Recovery(f)) => {
            return needs_help_v2(
                fx,
                j,
                &format!("recovery_mismatch:{f}"),
                Explanation::RecoveryMismatch { folder: f },
            )
        }
        Err(VerifyFail::Unknown) => {
            return needs_help_v2(
                fx,
                j,
                "unknown_state",
                Explanation::UnknownInterruptedState { package: name },
            )
        }
        Err(VerifyFail::Io(e)) => {
            return PackageResult::Interrupted(Explanation::InterruptedByError {
                package: name,
                detail: e.to_string(),
            })
        }
    }
    if let Err(e) = transition_v2(fx, j, JournalPhase::Verified, None, None) {
        return PackageResult::Interrupted(Explanation::InterruptedByError {
            package: name,
            detail: e.to_string(),
        });
    }
    finalize_v2(fx, j)
}

/// After verify: write the reserved records (idempotent), unpin and prune, discard staging, and
/// remove the journal last. Every step may run again after a crash.
fn finalize_v2(fx: &mut impl ApplyEffects, j: &ApplyJournalV2) -> PackageResult {
    let result = (|| -> Result<(), EffectError> {
        for p in &j.pending_records {
            fx.write_pending_record(p)?;
        }
        settle_points(fx, &j.package, &j.op)?;
        fx.discard_staging(&j.op)?;
        fx.discard_settings_staging(&j.op)?;
        fx.remove_journal(&j.op)
    })();
    match result {
        Ok(()) => PackageResult::Applied {
            recovery: j.op.clone(),
        },
        Err(e) => PackageResult::Failed(Explanation::FinalizeFailed {
            package: j.package.as_str().to_string(),
            detail: e.to_string(),
        }),
    }
}

/// Rolled back: the reserved records are discarded and never written.
fn cleanup_v2(fx: &mut impl ApplyEffects, j: &mut ApplyJournalV2) -> PackageResult {
    let name = j.package.as_str().to_string();
    let rolled = PackageResult::RolledBack(Explanation::RolledBack {
        package: name.clone(),
    });
    let err = |e: EffectError| {
        PackageResult::Failed(Explanation::FinalizeFailed {
            package: name.clone(),
            detail: e.to_string(),
        })
    };
    let items = items_of(j);
    let committing = j.from_phase == Some(JournalPhase::Committing);
    if committing {
        for it in &items {
            match t_hash(fx, &it.staging) {
                Ok(None) => {}
                Ok(Some(t)) if Some(t) == it.new => {}
                Ok(Some(_)) => {
                    return needs_help_v2(
                        fx,
                        j,
                        "unexpected_staging",
                        Explanation::UnexpectedStaging {
                            package: name.clone(),
                        },
                    )
                }
                Err(e) => return err(e),
            }
        }
    }
    if let Err(e) = fx.discard_staging(&j.op) {
        return err(e);
    }
    if let Err(e) = fx.discard_settings_staging(&j.op) {
        return err(e);
    }
    let mut any_recovery = false;
    if committing {
        for it in &items {
            match t_present(fx, &it.recovery) {
                Ok(p) => any_recovery |= p,
                Err(e) => return err(e),
            }
        }
    }
    if !any_recovery {
        if let Err(e) = fx.remove_recovery(&j.package, &j.op) {
            return err(e);
        }
    }
    if let Err(e) = fx.remove_journal(&j.op) {
        return err(e);
    }
    rolled
}

fn recover_one_v2(fx: &mut impl ApplyEffects, j: &mut ApplyJournalV2) -> PackageResult {
    let name = j.package.as_str().to_string();
    match j.phase {
        JournalPhase::Staging => {
            if let Err(e) = transition_v2(
                fx,
                j,
                JournalPhase::RolledBack,
                Some(JournalPhase::Staging),
                None,
            ) {
                return failed(&name, e);
            }
            cleanup_v2(fx, j)
        }
        JournalPhase::Committing => match gate(fx, &name) {
            Err(h) => PackageResult::Interrupted(h.why),
            Ok(()) => rollback_v2(fx, j),
        },
        JournalPhase::Committed => verify_and_finish_v2(fx, j),
        JournalPhase::Verified => finalize_v2(fx, j),
        JournalPhase::RolledBack => cleanup_v2(fx, j),
        JournalPhase::NeedsHelp => {
            PackageResult::NeedsHelp(explanation_for(&j.package, j.reason.as_deref()))
        }
    }
}

fn restore_item(fx: &mut impl ApplyEffects, name: &str, it: &Item) -> Option<RestoreFail> {
    if let Err(h) = gate(fx, name) {
        return Some(RestoreFail::Gate(h));
    }
    if let Err(e) = t_rename(fx, &it.recovery, &it.live) {
        return Some(RestoreFail::Rename(e));
    }
    match t_hash(fx, &it.live) {
        Ok(t) if t == it.old => None,
        Ok(_) => Some(RestoreFail::Mismatch),
        Err(e) => Some(RestoreFail::Io(e)),
    }
}

fn fail_restore_v2(
    fx: &mut impl ApplyEffects,
    j: &mut ApplyJournalV2,
    res: RestoreFail,
    label: &str,
) -> PackageResult {
    match res {
        RestoreFail::Gate(h) => PackageResult::Interrupted(h.why),
        RestoreFail::Rename(e) => PackageResult::Failed(Explanation::RestoreFailed {
            folder: format!("{label} ({e})"),
        }),
        RestoreFail::Io(e) => failed(j.package.as_str(), e),
        RestoreFail::Mismatch => needs_help_v2(
            fx,
            j,
            &format!("restore_mismatch:{label}"),
            Explanation::RestoreFailed {
                folder: label.to_string(),
            },
        ),
    }
}

/// Each action is rolled back from its observed slots, in reverse order, so a second crash never
/// repeats a move (removal.md §5.3).
fn rollback_v2(fx: &mut impl ApplyEffects, j: &mut ApplyJournalV2) -> PackageResult {
    let name = j.package.as_str().to_string();
    let items = items_of(j);
    for it in items.iter().rev() {
        let cfg = match slots_of(fx, it) {
            Ok(c) => c,
            Err(e) => return failed(&name, e),
        };
        match cfg {
            SlotConfig::Untouched => {}
            SlotConfig::Unknown => {
                return needs_help_v2(
                    fx,
                    j,
                    "unknown_state",
                    Explanation::UnknownInterruptedState { package: name },
                )
            }
            SlotConfig::Half => {
                if let Some(res) = restore_item(fx, &name, it) {
                    return fail_restore_v2(fx, j, res, &it.label);
                }
            }
            SlotConfig::Done => {
                if let Some(new) = it.new {
                    match t_hash(fx, &it.live) {
                        Ok(Some(t)) if t == new => {}
                        Ok(_) => {
                            return needs_help_v2(
                                fx,
                                j,
                                &format!("live_mismatch:{}", it.label),
                                Explanation::LiveMismatch {
                                    folder: it.label.clone(),
                                },
                            )
                        }
                        Err(e) => return failed(&name, e),
                    }
                    if let Err(h) = gate(fx, &name) {
                        return PackageResult::Interrupted(h.why);
                    }
                    if let Err(e) = t_rename(fx, &it.live, &it.staging) {
                        return PackageResult::Failed(Explanation::RestoreFailed {
                            folder: format!("{} ({e})", it.label),
                        });
                    }
                }
                if it.old.is_some() {
                    if let Some(res) = restore_item(fx, &name, it) {
                        return fail_restore_v2(fx, j, res, &it.label);
                    }
                }
            }
        }
    }
    for it in &items {
        match t_hash(fx, &it.live) {
            Err(e) => return failed(&name, e),
            Ok(t) if t == it.old => {}
            Ok(_) => {
                return needs_help_v2(
                    fx,
                    j,
                    &format!("restore_mismatch:{}", it.label),
                    Explanation::RestoreFailed {
                        folder: it.label.clone(),
                    },
                )
            }
        }
    }
    if let Err(e) = transition_v2(
        fx,
        j,
        JournalPhase::RolledBack,
        Some(JournalPhase::Committing),
        None,
    ) {
        return failed(&name, e);
    }
    cleanup_v2(fx, j)
}

/// Used by status: the op names present in raw journal listings.
pub fn journal_file_ops(files: &[(String, Vec<u8>)]) -> Vec<String> {
    files.iter().map(|(n, _)| op_name(n)).collect()
}

#[allow(dead_code)]
fn state_seq(name: &str) -> Option<u64> {
    parse_state_name(name).map(|(_, s)| s)
}

#[cfg(test)]
#[path = "exec_tests.rs"]
mod tests;
