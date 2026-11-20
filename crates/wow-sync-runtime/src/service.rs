//! `SliceService`: the thin-slice orchestration. Every decision (variant safety, admission,
//! target, plan, status wording) is made by `wow-sync-core`; this file gathers observations
//! through the adapters, calls core, and runs the resulting plans through the effects.

use crate::activity::{ActivitySource, WinprocActivity};
use crate::control::{Command, CommandError, CommandOutcome};
use crate::decisions::write_own_record;
use crate::decisions::{
    learn_from_directories, plan_own_index, route, write_record, Constraints, KeptQueue,
    KeptRecord, Plane, RouteCx, Routed,
};
use crate::detail::{
    build_package_rows, folder_titles, member_rows, render_with_names, DetailInput, StatusDetail,
};
use crate::effects::{
    load_pending, FaultPlan, Faulty, Hooks, PendingSink, RealEffects, PENDING_REPORT,
};
use crate::removal_cycle::{step_3a, step_3b, RemovalStore};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use wow_sync_adapters::fs_install::InstallArea;
use wow_sync_adapters::fs_safe::{ReparsePolicy, Root};
use wow_sync_adapters::fs_scan::{
    list_variant_folders, read_variant_files, scan_addons, DirKind, FileRead, ScanOptions,
    ScanStatus as AdScanStatus,
};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::{EntryState, ProviderRoot};
use wow_sync_adapters::sysinfo::{now_unix_ms, random_id_hex};
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::admit::{
    admit_member_states, apply_barrier, newest_state_name, MemberLatest, MemberObs, MemberView,
    StateFileObs,
};
use wow_sync_core::authority::{AuthorityRecord, Retired};
use wow_sync_core::decide::decision_items;
use wow_sync_core::decide::{DecideBehaviour, Standing};
use wow_sync_core::decision::CycleCx;
use wow_sync_core::decision::{DecideOutcome, DecisionEffect, FreshEvidence};
use wow_sync_core::exec::{
    apply_report, recover_interrupted, run_apply, run_publish, ApplyEffects, PublishOutcome,
};
use wow_sync_core::explain;
use wow_sync_core::hooks::RemovalHooks;
use wow_sync_core::hooks::{DecisionHooks, RetireHooks};
use wow_sync_core::ids::{ComputerId, EnrollmentId, GroupId, OpId, Sha256};
use wow_sync_core::inventory::{build_inventory, FolderScan, Inventory, ScanStatus, ScannedFile};
use wow_sync_core::kinds::EnrollmentRef;
use wow_sync_core::kinds::Issuer;
use wow_sync_core::layout;
use wow_sync_core::ledger::{derive, Status as LedgerStatus};
use wow_sync_core::package::ClientInfo;
use wow_sync_core::paths::FolderKey;
use wow_sync_core::plan::{
    own_publish_slot, plan_apply, plan_publish, publish_needed, ApplyGate, PlanBlock,
    PublishContext,
};
use wow_sync_core::plan::{Activity, ApplyPlan, PackageApply};
use wow_sync_core::records::{
    decode, decode_tree, encode, AdmissionWatermarks, ApplyJournal, ApplyReport, BasisEntry,
    CodeState, GroupRecord, LocalComputer, LocalEnrollment, MemberRecord, PendingReport,
    RecordError, RecoveryPoint, RootFormat,
};
use wow_sync_core::reject::{build_lift, LiftError, RejectBehaviour};
use wow_sync_core::removal::known_summary;
use wow_sync_core::removal::{removal_record, ObserveInput, RemovalBehaviour};
use wow_sync_core::report::{outstanding, record_pending, store_needs_reconcile};
use wow_sync_core::retire::OwnStanding;
use wow_sync_core::retire::RetireBehaviour;
use wow_sync_core::status::{
    applied_here, derive_status, game_halted, headline_sentence, journal_status, remember_applied,
    render_block, render_headline, render_member, render_result, Headline, StatusInput, StatusView,
};
use wow_sync_core::target::{
    compose, package_copies, GroupTarget, PackageCopies, TargetInput, TreeAvail,
};
use wow_sync_core::toc::TocCandidate;
use wow_sync_core::variant::{
    assess_layout, check_join, confirm_enrolled, confirm_variant, consent_disclosures, consent_gap,
    reconfirm_variant, validate_variant, ConfirmedVariant, ConsentAction, ConsentContext, DirObs,
    FileObs, GuidedConfirmation, LayoutAssessment, OtherVariants, RefuseReason, VariantObservation,
    VariantVerdict,
};

const SMALL: u64 = 64 * 1024;
const STATE_LIMIT: u64 = 16 * 1024 * 1024;
const TREE_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub install_root: PathBuf,
    pub variant_folder: String,
    pub shared_root: PathBuf,
    pub state_dir: PathBuf,
    pub display_name: String,
    pub app_version: String,
    /// Stability window for scans (production default 2000 ms).
    pub recent_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    NotEnrolled,
    AlreadyEnrolled,
    Refused(String),
    /// The enrolled client no longer matches its confirmation; guided reconfirmation is offered.
    NeedsReconfirmation,
    /// The game product or folder is not the enrolled one; reconfirming cannot fix that.
    VariantChanged,
    Provider(String),
    Local(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEnrolled => write!(f, "this installation is not enrolled"),
            Self::AlreadyEnrolled => write!(f, "this installation is already enrolled"),
            Self::Refused(s) => write!(f, "refused: {s}"),
            Self::NeedsReconfirmation => write!(
                f,
                "this game version changed since it was confirmed; confirm it again to continue"
            ),
            Self::VariantChanged => write!(
                f,
                "this installation is no longer the game version that was enrolled (its product \
                 or folder changed). Syncing is paused; restore the original install or set this \
                 computer up again"
            ),
            Self::Provider(s) => write!(f, "shared folder problem: {s}"),
            Self::Local(s) => write!(f, "local problem: {s}"),
        }
    }
}

impl std::error::Error for ServiceError {}

fn local(e: FsError) -> ServiceError {
    ServiceError::Local(e.to_string())
}

fn prov(e: FsError) -> ServiceError {
    use wow_sync_adapters::FsErrorKind as K;
    ServiceError::Provider(match e.kind {
        K::NotPlainFile | K::NotFound => format!(
            "the shared folder at {} is missing or is not a normal folder. Check that Dropbox \
             is running and the folder is available, then try again",
            e.path.display()
        ),
        K::CloudOnly => format!(
            "{} is online-only; make it available offline in Dropbox, then try again",
            e.path.display()
        ),
        _ => e.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewKind {
    Recognized,
    Unfamiliar,
    Refused,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: GroupId,
    pub name: String,
    pub product: String,
}

/// Read-only description shown before enrollment (nothing has been written).
#[derive(Debug, Clone)]
pub struct EnrollPreview {
    pub kind: PreviewKind,
    pub identity_line: String,
    pub scope_line: String,
    pub eligible: Vec<String>,
    pub blocked: Vec<String>,
    pub digest: Option<Sha256>,
    pub needs_guided_confirmation: bool,
    pub groups: Vec<GroupInfo>,
    pub problem: Option<String>,
    /// Consent text shown before anything is written (domain.md §1 step 5, §2.1).
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReconfirmRequest {
    pub identity_confirmed: bool,
    pub scope_confirmed: bool,
    pub digest: Option<Sha256>,
}

#[derive(Debug, Clone)]
pub enum JoinChoice {
    New { name: String },
    Existing(GroupId),
}

#[derive(Debug, Clone)]
pub struct EnrollRequest {
    pub join: JoinChoice,
    pub identity_confirmed: bool,
    pub scope_confirmed: bool,
    pub digest: Option<Sha256>,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct PackageLine {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AppliedLine {
    pub package: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct StatusSnapshot {
    pub enrolled: bool,
    pub headline: String,
    pub group_name: String,
    pub computer_name: String,
    pub identity_line: String,
    pub packages: Vec<PackageLine>,
    pub members: Vec<String>,
    pub blocks: Vec<String>,
    pub recovery_note: Option<String>,
    pub last_publish: Option<String>,
    pub provider_error: Option<String>,
    pub activity: String,
    pub retained_extra_recovery: u32,
    /// Set when the enrolled client needs guided reconfirmation (shown by the GUI).
    pub reconfirm: Option<EnrollPreview>,
    /// Typed rows for the GUI; present whenever the shared state could be read.
    pub detail: Option<StatusDetail>,
}

impl StatusSnapshot {
    /// "Group on Computer (identity)", only when the parts are known.
    pub fn who_line(&self) -> Option<String> {
        if self.group_name.is_empty() && self.computer_name.is_empty() {
            return None;
        }
        let mut s = format!("{} on {}", self.group_name, self.computer_name);
        if !self.identity_line.is_empty() {
            s.push_str(&format!(" ({})", self.identity_line));
        }
        Some(s)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub recovered: Vec<String>,
    pub applied: Vec<AppliedLine>,
    pub apply_note: Option<String>,
    pub published: Option<String>,
    pub publish_note: Option<String>,
    pub crashed: bool,
    pub status: StatusSnapshot,
}

struct World {
    enrollment: LocalEnrollment,
    computer: LocalComputer,
    group: GroupRecord,
    variant: ConfirmedVariant,
    inventory: Inventory,
    views: Vec<MemberView>,
    target: GroupTarget,
    offers: Vec<(
        wow_sync_core::decision::DecisionItem,
        wow_sync_core::decide::Plans,
    )>,
    copies: Vec<PackageCopies>,
    own_states: Vec<String>,
    /// This enrollment's newest published state, if readable.
    own_latest: Option<CodeState>,
    titles: BTreeMap<String, String>,
    decls: Vec<wow_sync_core::settings::DeclaringFolder>,
    own: EnrollmentRef,
    standing: OwnStanding,
}

pub struct SliceService {
    cfg: ServiceConfig,
    activity: Box<dyn ActivitySource>,
    hooks: Hooks,
    fault: FaultPlan,
    state: LocalState,
    seen_locally: BTreeMap<ComputerId, i64>,
    seen_seq: BTreeMap<ComputerId, u64>,
    last_publish: Option<i64>,
    applied_here: Vec<(FolderKey, wow_sync_core::explain::Explanation)>,
    last_blocks: Vec<PlanBlock>,
    provider_error: Option<String>,
    plane: Plane,
    /// The approved saved-settings size cap. No value is approved (Q-77), so it is `None` and
    /// settings sync never touches real data; only tests set one.
    settings_cap: Option<u64>,
    /// The decisions currently offered to the owner and what answering each one does.
    offered: (
        Vec<wow_sync_core::decision::DecisionItem>,
        wow_sync_core::decide::Plans,
    ),
}

fn file_obs(r: FileRead) -> FileObs {
    match r {
        FileRead::Missing => FileObs::Missing,
        FileRead::Unreadable(s) => FileObs::Unreadable(s),
        FileRead::Bytes(b) => FileObs::Bytes(b),
    }
}

fn is_missing(e: &FsError) -> bool {
    e.kind == FsErrorKind::NotFound
}

/// Folders that any journal names (removal.md E5).
fn journaled_folders(area: &InstallArea) -> BTreeSet<FolderKey> {
    let (journals, _) = crate::control::split_journals(area.read_journals().unwrap_or_default());
    journals
        .iter()
        .flat_map(|j| j.folders.iter().map(|f| f.key.clone()))
        .collect()
}

impl SliceService {
    pub fn new(cfg: ServiceConfig) -> Result<SliceService, ServiceError> {
        Self::with_activity(cfg, Box::new(WinprocActivity))
    }

    pub fn with_activity(
        cfg: ServiceConfig,
        activity: Box<dyn ActivitySource>,
    ) -> Result<SliceService, ServiceError> {
        let state = LocalState::open_at(&cfg.state_dir).map_err(local)?;
        let plane = Plane::load(&state);
        Ok(SliceService {
            cfg,
            activity,
            hooks: Hooks::default(),
            fault: FaultPlan::default(),
            state,
            seen_locally: BTreeMap::new(),
            seen_seq: BTreeMap::new(),
            last_publish: None,
            applied_here: Vec::new(),
            last_blocks: Vec::new(),
            provider_error: None,
            plane,
            settings_cap: wow_sync_core::records::MAX_SETTINGS_FILE_BYTES,
            offered: Default::default(),
        })
    }

    /// Test-only: a settings size cap, standing in for the unapproved production value (Q-77).
    #[cfg(test)]
    pub(crate) fn set_settings_cap_for_tests(&mut self, cap: Option<u64>) {
        self.settings_cap = cap;
    }

    /// Replaces the offered decisions. The set is what `Command::Decide` is judged against, so
    /// an item whose evidence changed since the owner saw it is refused as changed.
    pub fn offer_decisions(
        &mut self,
        items: Vec<wow_sync_core::decision::DecisionItem>,
        plans: wow_sync_core::decide::Plans,
    ) {
        self.offered = (items, plans);
    }

    /// The decisions offered after the latest cycle.
    pub fn offered_decisions(&self) -> &[wow_sync_core::decision::DecisionItem] {
        &self.offered.0
    }

    /// The `SettingsHistory` query: the valid revisions of `unit` as of the latest observation,
    /// oldest first. It reads the plane only; nothing is written or applied.
    pub fn settings_history(
        &self,
        unit: &wow_sync_core::ids::SettingsUnit,
    ) -> Vec<crate::settings_cycle::SettingsHistoryEntry> {
        let Ok(Some(enrollment)) = self.read_enrollment() else {
            return Vec::new();
        };
        let derived = derive(
            &self.plane.ledger,
            &enrollment.group_id,
            &self.plane.members,
        );
        crate::settings_cycle::settings_history(&self.plane.ledger, &derived, unit)
    }

    /// Runs one group command (contracts.md §9.1) against the current observed state.
    pub fn command(&mut self, cmd: Command) -> Result<CommandOutcome, CommandError> {
        if cmd == Command::ReEnroll {
            return self.re_enroll();
        }
        let w = self
            .observe_world(false)
            .map_err(|e| CommandError::Service(e.to_string()))?;
        let provider = self
            .open_provider()
            .map_err(|e| CommandError::Service(e.to_string()))?;
        if w.standing == OwnStanding::Retired {
            return Err(CommandError::NotAllowed);
        }
        let gid = w.group.group_id.clone();
        if let Command::EnableSettings { included_accounts } = &cmd {
            let mut l = crate::settings_cycle::load_local(&self.state, &gid);
            l.consent = true;
            l.included_accounts = included_accounts.clone();
            l.included_accounts.sort();
            l.included_accounts.dedup();
            crate::settings_cycle::save_local(&self.state, &gid, &l)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        let now = now_unix_ms() as u64;
        if let Command::Rollback { package, recovered } = &cmd {
            let request = crate::rollback_flow::RollbackRequest {
                package: package.clone(),
                recovered: *recovered,
            };
            let mut audience: Vec<EnrollmentRef> = w
                .views
                .iter()
                .map(|v| EnrollmentRef {
                    computer_id: v.computer.clone(),
                    enrollment_id: v.enrollment.clone(),
                })
                .collect();
            audience.push(w.own.clone());
            crate::rollback_flow::plan_requested(&crate::rollback_flow::PlanArgs {
                request: &request,
                group: &gid,
                own: &w.own,
                audience,
                inventory: &w.inventory,
                copies: &w.copies,
                plane: &self.plane,
                provider: &provider,
                variant_dir: &self.variant_dir(),
                op: OpId::parse(&random_id_hex()).expect("random id"),
                seq: w.enrollment.next_seq,
                now_unix_ms: now,
            })
            .map_err(|why| match why {
                crate::rollback_flow::RollbackRefusal::UnknownRelease(h) => {
                    CommandError::UnknownCopy(h)
                }
                other => CommandError::Rollback(other),
            })?;
            crate::rollback_flow::save_request(&self.state, &gid, &request)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        let seq = w.enrollment.next_seq;
        let derived = derive(&self.plane.ledger, &gid, &self.plane.members);
        let held = BTreeSet::new();
        let effect = {
            let cx = CycleCx {
                group_id: &gid,
                own: &w.own,
                ledger: &self.plane.ledger,
                derived: &derived,
                members: &self.plane.members,
                now_unix_ms: now,
                held: &held,
                roots: &[],
            };
            let issuer = Issuer {
                computer_id: w.own.computer_id.clone(),
                enrollment_id: w.own.enrollment_id.clone(),
                issued_at_seq: seq,
            };
            match cmd {
                Command::Decide {
                    item,
                    option,
                    shown,
                } => {
                    let behaviour = DecideBehaviour::new(self.offered.1.clone(), Standing::Active);
                    let fresh = FreshEvidence {
                        items: self.offered.0.clone(),
                    };
                    match behaviour.decide(&item, &option, &shown, &fresh) {
                        DecideOutcome::Effect(e) => e,
                        DecideOutcome::Refused(why) => return Err(CommandError::Decide(why)),
                    }
                }
                Command::Retire { subject } => {
                    let mut admitted_seq = BTreeMap::new();
                    for e in cx.ledger.entries() {
                        if let Some(i) = &e.issuer {
                            let s = admitted_seq.entry(i.enrollment_id.clone()).or_insert(0);
                            *s = (*s).max(i.issued_at_seq);
                        }
                    }
                    let plan = RetireBehaviour {
                        own_next_seq: seq,
                        admitted_seq,
                        master: None,
                    }
                    .plan_retirement(&cx, &subject);
                    match plan.retirement {
                        Some(r) => DecisionEffect::Write(Box::new(AuthorityRecord::Retirement(r))),
                        None if w.standing != OwnStanding::Active => {
                            return Err(CommandError::NotAllowed)
                        }
                        None => return Ok(CommandOutcome::NothingToDo),
                    }
                }
                Command::AllowAgain { rejections } => {
                    let lift = build_lift(
                        &RejectBehaviour::new(&derived),
                        &self.plane.ledger,
                        &gid,
                        issuer,
                        &rejections,
                        known_summary(&self.plane.ledger, &gid),
                        now,
                    )
                    .map_err(|e| match e {
                        LiftError::NotActive(h) => CommandError::UnknownRejection(h),
                        LiftError::Empty => CommandError::EmptySelection,
                    })?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::Lift(lift)))
                }
                Command::RestoreRemoved { removal } => {
                    let valid = derived.status.get(&removal) == Some(&LedgerStatus::Valid);
                    let Some(AuthorityRecord::Removal(rm)) =
                        self.plane.ledger.record(&removal).filter(|_| valid)
                    else {
                        return Err(CommandError::UnknownRemoval(removal));
                    };
                    let readd = crate::removal_cycle::restore(
                        &self.plane.ledger,
                        &gid,
                        &w.own,
                        rm,
                        seq,
                        now,
                    );
                    DecisionEffect::Write(Box::new(AuthorityRecord::Readd(readd)))
                }
                // Dispatched before observation, above.
                Command::ReEnroll | Command::EnableSettings { .. } | Command::Rollback { .. } => {
                    return Err(CommandError::NotAllowed)
                }
                Command::SetSettingsPolicy { master, exceptions } => {
                    let rec = wow_sync_core::settings::build_policy(
                        &self.plane.ledger,
                        &derived,
                        &self.plane.members,
                        &gid,
                        issuer,
                        master,
                        exceptions,
                        now,
                    )
                    .map_err(CommandError::Settings)?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::SettingsPolicy(rec)))
                }
                Command::ShareSettings { unit, checkpoint } => {
                    let known_here =
                        wow_sync_adapters::settings_store::SettingsStore::open(&self.variant_dir())
                            .ok()
                            .and_then(|s| s.get_manifest(&checkpoint.0).ok().flatten())
                            .is_some();
                    if !known_here {
                        return Err(CommandError::UnknownCheckpoint(checkpoint));
                    }
                    let rec = wow_sync_core::settings::build_push(
                        &self.plane.ledger,
                        &derived,
                        &gid,
                        issuer,
                        unit,
                        checkpoint,
                        now,
                    )
                    .map_err(CommandError::Settings)?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::SettingsRevision(rec)))
                }
                Command::Keep { scope, copy, why } => {
                    let valid = wow_sync_core::choice::valid_hashes(&derived);
                    let view = wow_sync_core::choice::ChoiceView::new(&self.plane.ledger, &valid);
                    let pc = w
                        .copies
                        .iter()
                        .find(|p| {
                            !scope.is_empty() && scope.iter().all(|k| p.package.members.contains(k))
                        })
                        .ok_or(CommandError::UnknownCopy(copy))?;
                    let chosen = pc
                        .copies
                        .iter()
                        .find(|c| c.version.hash == copy)
                        .ok_or(CommandError::UnknownCopy(copy))?;
                    let shown: Vec<_> = pc
                        .copies
                        .iter()
                        .map(|c| wow_sync_core::choice::CopyEntry::from_copy(c, &scope))
                        .collect::<Result<_, _>>()
                        .map_err(CommandError::Write)?;
                    let kept = wow_sync_core::choice::CopyEntry::from_copy(chosen, &scope)
                        .map_err(CommandError::Write)?;
                    let rec = wow_sync_core::choice::build_keep(
                        &view,
                        &wow_sync_core::choice::WriteCx {
                            group: &gid,
                            issuer,
                            now_unix_ms: now,
                        },
                        why,
                        &scope,
                        &kept,
                        &shown,
                        &chosen.source,
                    )
                    .map_err(CommandError::Write)?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::CodeChoice(rec)))
                }
                Command::Resume {
                    scope,
                    allow_again,
                    why,
                } => {
                    let valid = wow_sync_core::choice::valid_hashes(&derived);
                    let view = wow_sync_core::choice::ChoiceView::new(&self.plane.ledger, &valid);
                    let rec = wow_sync_core::choice::build_resume(
                        &view,
                        &wow_sync_core::choice::WriteCx {
                            group: &gid,
                            issuer,
                            now_unix_ms: now,
                        },
                        why,
                        &scope,
                        &allow_again,
                    )
                    .map_err(CommandError::Write)?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::CodeChoice(rec)))
                }
            }
        };
        let routed = route(
            effect,
            &RouteCx {
                standing: w.standing,
                paused: self.pause_state().holds_sync(),
                folder_writable: true,
            },
        );
        match routed {
            Routed::Refused(why) => Err(CommandError::Decide(why)),
            Routed::Local(l) => Ok(CommandOutcome::Local(l)),
            Routed::WriteNow(rec) => {
                let h =
                    write_own_record(&mut self.plane, &provider, &gid, &w.own.computer_id, &rec)
                        .map_err(|e| CommandError::Service(e.to_string()))?;
                self.plane
                    .save(&self.state)
                    .map_err(|e| CommandError::Service(e.to_string()))?;
                self.reserve_seq(&w, seq)?;
                Ok(CommandOutcome::Written(h))
            }
            Routed::Keep(rec, why) => {
                let mut q = KeptQueue::load(&self.state, &gid);
                q.push(KeptRecord::reserve(&rec, seq, why));
                q.save(&self.state, &gid)
                    .map_err(|e| CommandError::Service(e.to_string()))?;
                self.reserve_seq(&w, seq)?;
                Ok(CommandOutcome::Kept)
            }
        }
    }

    /// Explicit re-enrollment of a retired (or held) computer: a new enrollment id for the same
    /// computer, recorded in `member.json` with the old ids kept (ops.md §2.3 item 6). The old
    /// enrollment stays retired and its records stay valid.
    fn re_enroll(&mut self) -> Result<CommandOutcome, CommandError> {
        let svc = |e: &dyn std::fmt::Display| CommandError::Service(e.to_string());
        let w = self.observe_world(false).map_err(|e| svc(&e))?;
        if w.standing == OwnStanding::Active {
            return Ok(CommandOutcome::NothingToDo);
        }
        let provider = self.open_provider().map_err(|e| svc(&e))?;
        let me = w.computer.computer_id.clone();
        let old = provider
            .read(&layout::member_record(&w.group.group_id, &me), SMALL)
            .map_err(|e| svc(&e))
            .and_then(|b| decode::<MemberRecord>(&b, SMALL as usize).map_err(|e| svc(&e)))?;
        let enrollment_id = EnrollmentId::parse(&random_id_hex()).expect("random id");
        let mut previous = old.previous_enrollments.clone();
        previous.push(w.enrollment.enrollment_id.clone());
        previous.sort();
        previous.dedup();
        let member = MemberRecord {
            enrollment_id: enrollment_id.clone(),
            enrolled_unix_ms: now_unix_ms() as u64,
            app_version: self.cfg.app_version.clone(),
            previous_enrollments: previous,
            ..old
        };
        provider
            .write_atomic(
                &layout::member_record(&w.group.group_id, &me),
                &encode(&member),
                me.as_str(),
            )
            .map_err(|e| svc(&e))?;
        let mut le = w.enrollment.clone();
        le.enrollment_id = enrollment_id;
        le.next_seq = 1;
        self.state
            .write_atomic("enrollment.json", &encode(&le))
            .map_err(|e| svc(&e))?;
        let _ = self.state.append_audit("re-enrolled this computer");
        Ok(CommandOutcome::ReEnrolled)
    }

    /// The sequence number a record used is never handed out again.
    fn reserve_seq(&self, w: &World, used: u64) -> Result<(), CommandError> {
        let mut le = w.enrollment.clone();
        le.next_seq = used + 1;
        self.state
            .write_atomic("enrollment.json", &encode(&le))
            .map_err(|e| CommandError::Service(e.to_string()))
    }

    /// Writes the records kept while publishing was blocked, once nothing blocks them; a
    /// retired computer discards them (ops.md §1, Q-60).
    fn settle_kept(&mut self, w: &World, provider: &ProviderRoot) {
        let gid = &w.group.group_id;
        let mut q = KeptQueue::load(&self.state, gid);
        if q.is_empty() {
            return;
        }
        let settled = q.settle(w.standing, self.pause_state().holds_sync(), true);
        for r in settled.to_write {
            match write_record(provider, gid, &w.own.computer_id, &r) {
                Ok(_) => self
                    .plane
                    .learn_bytes(r.hash, Some(&w.own.computer_id), &r.bytes),
                Err(_) => q.push(r),
            }
        }
        let _ = self.plane.save(&self.state);
        let _ = q.save(&self.state, gid);
    }

    /// Simulate a process crash after this many mutating effects in the next cycle.
    pub fn inject_crash_after(&mut self, n: Option<usize>) {
        self.fault.crash_after = n;
    }

    pub fn hooks_mut(&mut self) -> &mut Hooks {
        &mut self.hooks
    }

    pub fn config(&self) -> &ServiceConfig {
        &self.cfg
    }

    fn variant_dir(&self) -> PathBuf {
        self.cfg.install_root.join(&self.cfg.variant_folder)
    }

    fn addons_path(&self) -> PathBuf {
        self.variant_dir().join("Interface").join("AddOns")
    }

    fn observation(&self, folder: &str) -> VariantObservation {
        let f = read_variant_files(&self.cfg.install_root, folder);
        VariantObservation {
            folder_name: f.folder_name,
            flavor_info: file_obs(f.flavor_info),
            build_info: file_obs(f.build_info),
            addons_dir: match f.addons_dir {
                DirKind::Missing => DirObs::Missing,
                DirKind::PlainDir => DirObs::PlainDir,
                DirKind::NotPlain => DirObs::NotPlain,
            },
            synthetic_marker: file_obs(f.synthetic_marker),
        }
    }

    fn scan(&self) -> Result<(Vec<FolderScan>, bool), ServiceError> {
        let root = Root::open(&self.addons_path(), ReparsePolicy::RejectAll).map_err(local)?;
        let mut opts = ScanOptions::with_defaults(now_unix_ms());
        opts.recent_ms = self.cfg.recent_ms;
        let scan = scan_addons(&root, &opts).map_err(local)?;
        let folders = scan
            .folders
            .into_iter()
            .map(|f| FolderScan {
                name: f.name,
                status: match f.status {
                    AdScanStatus::Ok => ScanStatus::Ok,
                    AdScanStatus::Unreadable(s) => ScanStatus::Unreadable(s),
                    AdScanStatus::Unsafe(s) => ScanStatus::Unsafe(s),
                    AdScanStatus::Unstable(s) => ScanStatus::Unstable(s),
                },
                files: f
                    .files
                    .into_iter()
                    .map(|x| ScannedFile {
                        rel_path: x.rel_path,
                        size: x.size,
                        sha256: Sha256(x.sha256),
                    })
                    .collect(),
                tocs: f
                    .tocs
                    .into_iter()
                    .map(|t| TocCandidate {
                        file_name: t.file_name,
                        contents: match t.contents {
                            Ok(b) => FileObs::Bytes(b),
                            Err(e) => FileObs::Unreadable(e),
                        },
                    })
                    .collect(),
                fingerprint: f.fingerprint.map(Sha256),
            })
            .collect();
        Ok((folders, scan.listing_stable))
    }

    /// Sibling variant folders that validate to the same product (the core decides what counts).
    fn others(&self, product: &str, enrolled_elsewhere: bool) -> OtherVariants {
        let mut same = Vec::new();
        for f in list_variant_folders(&self.cfg.install_root) {
            if f == self.cfg.variant_folder {
                continue;
            }
            let v = validate_variant(&self.observation(&f));
            let id = match &v {
                VariantVerdict::Recognized(i) | VariantVerdict::Unfamiliar(i) => i,
                VariantVerdict::Refused(_) => continue,
            };
            if id.product.as_str() == product {
                same.push(f);
            }
        }
        OtherVariants {
            same_product_folders: same,
            enrolled_elsewhere,
        }
    }

    fn judge(
        &self,
        enrolled_elsewhere: bool,
    ) -> Result<(VariantVerdict, LayoutAssessment, Vec<FolderScan>, bool), ServiceError> {
        let verdict = validate_variant(&self.observation(&self.cfg.variant_folder));
        let (scans, stable) = match &verdict {
            VariantVerdict::Refused(_) => (Vec::new(), true),
            _ => self.scan()?,
        };
        let assessment = match &verdict {
            VariantVerdict::Recognized(id) => assess_layout(
                id,
                false,
                &scans,
                &self.others(id.product.as_str(), enrolled_elsewhere),
            ),
            VariantVerdict::Unfamiliar(id) => assess_layout(
                id,
                true,
                &scans,
                &self.others(id.product.as_str(), enrolled_elsewhere),
            ),
            VariantVerdict::Refused(_) => {
                assess_layout(&dummy_identity(), false, &[], &OtherVariants::default())
            }
        };
        Ok((verdict, assessment, scans, stable))
    }

    fn open_provider(&self) -> Result<ProviderRoot, ServiceError> {
        ProviderRoot::open_or_create(&self.cfg.shared_root).map_err(prov)
    }

    fn existing_groups(&self) -> Vec<GroupInfo> {
        // Inspection must not create the shared folder.
        if !self.cfg.shared_root.join("WoWSync").is_dir() {
            return Vec::new();
        }
        let Ok(p) = ProviderRoot::open_or_create(&self.cfg.shared_root) else {
            return Vec::new();
        };
        let Ok(entries) = p.list("groups") else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in entries.into_iter().filter(|e| e.is_dir) {
            let Ok(id) = GroupId::parse(&e.name) else {
                continue;
            };
            let Ok(bytes) = p.read(&layout::group_record(&id), SMALL) else {
                continue;
            };
            if let Ok(g) = decode::<GroupRecord>(&bytes, SMALL as usize) {
                out.push(GroupInfo {
                    id,
                    name: g.display_name,
                    product: g.product,
                });
            }
        }
        out
    }

    /// The learned records and membership, for tests.
    #[cfg(test)]
    pub(crate) fn plane(&self) -> &Plane {
        &self.plane
    }

    /// Read-only: writes nothing locally or in the shared folder.
    pub fn inspect(&self) -> EnrollPreview {
        let enrolled = self.read_enrollment().ok().flatten().is_some();
        let mut p = self.preview();
        if enrolled {
            p.problem = Some(ServiceError::AlreadyEnrolled.to_string());
        }
        p
    }

    fn preview(&self) -> EnrollPreview {
        let (verdict, assessment, _scans, _stable) = match self.judge(false) {
            Ok(x) => x,
            Err(e) => {
                return EnrollPreview {
                    kind: PreviewKind::Refused,
                    identity_line: String::new(),
                    scope_line: String::new(),
                    eligible: vec![],
                    blocked: vec![],
                    digest: None,
                    needs_guided_confirmation: false,
                    groups: vec![],
                    problem: Some(e.to_string()),
                    disclosures: vec![],
                }
            }
        };
        let mut p = EnrollPreview {
            kind: PreviewKind::Refused,
            identity_line: String::new(),
            scope_line: String::new(),
            eligible: vec![],
            blocked: vec![],
            digest: None,
            needs_guided_confirmation: false,
            groups: self.existing_groups(),
            problem: None,
            disclosures: vec![],
        };
        let (id, kind) = match &verdict {
            VariantVerdict::Recognized(i) => (i, PreviewKind::Recognized),
            VariantVerdict::Unfamiliar(i) => (i, PreviewKind::Unfamiliar),
            VariantVerdict::Refused(r) => {
                p.problem = Some(format!("This folder cannot be used: {r:?}"));
                return p;
            }
        };
        p.kind = kind.clone();
        p.needs_guided_confirmation = kind == PreviewKind::Unfamiliar;
        p.identity_line = format!(
            "{} - folder {}, build {}",
            id.display_label(),
            id.folder,
            id.build_version.as_deref().unwrap_or("unknown")
        );
        match assessment {
            LayoutAssessment::Supported {
                eligible,
                blocked,
                digest,
                mapping,
            } => {
                p.scope_line = format!(
                    "{} add-on folder(s) in {}/Interface/AddOns would sync; {} blocked",
                    eligible.len(),
                    self.cfg.variant_folder,
                    blocked.len()
                );
                p.eligible = eligible.iter().map(|k| k.to_string()).collect();
                p.blocked = blocked.iter().map(|(k, r)| format!("{k}: {r:?}")).collect();
                p.digest = Some(digest);
                p.disclosures = consent_disclosures(&ConsentContext {
                    identity: id,
                    mapping_id: mapping.id(),
                    install_root: &self.cfg.install_root.to_string_lossy(),
                    addons_path: &self.addons_path().to_string_lossy(),
                });
            }
            LayoutAssessment::Unsupported(r) => {
                p.problem = Some(format!("Unsupported layout: {r:?}"));
            }
        }
        p
    }

    /// Guided reconfirmation after the stored confirmation went stale (M1). Writes only the
    /// local enrollment record, and only for the same product and folder.
    pub fn reconfirm(&mut self, req: &ReconfirmRequest) -> Result<(), ServiceError> {
        let mut le = self.read_enrollment()?.ok_or(ServiceError::NotEnrolled)?;
        let (verdict, assessment, _s, _st) = self.judge(false)?;
        let guided = GuidedConfirmation {
            identity: req.identity_confirmed,
            scope: req.scope_confirmed,
            digest: req.digest.unwrap_or(Sha256([0; 32])),
        };
        let confirmed = reconfirm_variant(&verdict, &assessment, &guided, &le.validation)
            .map_err(|r| ServiceError::Refused(format!("{r:?}")))?;
        Self::check_consent(ConsentAction::Reconfirm, &verdict, &assessment, &guided)?;
        le.validation = confirmed.validation().clone();
        self.state
            .write_atomic("enrollment.json", &encode(&le))
            .map_err(local)?;
        let _ = self.state.append_audit("reconfirmed game version");
        Ok(())
    }

    /// Consent decided by core (mi-1): the shell enforces the same rule as the window.
    fn check_consent(
        action: ConsentAction,
        verdict: &VariantVerdict,
        assessment: &LayoutAssessment,
        guided: &GuidedConfirmation,
    ) -> Result<(), ServiceError> {
        let digest = match assessment {
            LayoutAssessment::Supported { digest, .. } => Some(digest),
            LayoutAssessment::Unsupported(_) => None,
        };
        let unfamiliar = matches!(verdict, VariantVerdict::Unfamiliar(_));
        match consent_gap(action, unfamiliar, guided, digest) {
            None => Ok(()),
            Some(g) => Err(ServiceError::Refused(format!(
                "{:?}",
                RefuseReason::from(g)
            ))),
        }
    }

    fn read_enrollment(&self) -> Result<Option<LocalEnrollment>, ServiceError> {
        match self.state.read("enrollment.json", SMALL).map_err(local)? {
            None => Ok(None),
            Some(b) => decode(&b, SMALL as usize)
                .map(Some)
                .map_err(|e| ServiceError::Local(format!("enrollment.json: {e}"))),
        }
    }

    fn read_computer(&self) -> Result<Option<LocalComputer>, ServiceError> {
        match self.state.read("computer.json", SMALL).map_err(local)? {
            None => Ok(None),
            Some(b) => decode(&b, SMALL as usize)
                .map(Some)
                .map_err(|e| ServiceError::Local(format!("computer.json: {e}"))),
        }
    }

    pub fn enroll(&mut self, req: &EnrollRequest) -> Result<GroupId, ServiceError> {
        if self.read_enrollment()?.is_some() {
            return Err(ServiceError::AlreadyEnrolled);
        }
        let (verdict, assessment, _s, _st) = self.judge(false)?;
        let guided = GuidedConfirmation {
            identity: req.identity_confirmed,
            scope: req.scope_confirmed,
            digest: req.digest.unwrap_or(Sha256([0; 32])),
        };
        let confirmed = confirm_variant(&verdict, &assessment, Some(&guided))
            .map_err(|r| ServiceError::Refused(format!("{r:?}")))?;
        Self::check_consent(ConsentAction::Enroll, &verdict, &assessment, &guided)?;
        let provider = self.open_provider()?;
        let now = now_unix_ms() as u64;
        let computer = match self.read_computer()? {
            Some(c) => c,
            None => LocalComputer {
                computer_id: ComputerId::parse(&random_id_hex()).expect("random id"),
                display_name: req.display_name.clone(),
                created_unix_ms: now,
            },
        };
        let group = match &req.join {
            JoinChoice::Existing(g) => {
                let bytes = provider
                    .read(&layout::group_record(g), SMALL)
                    .map_err(prov)?;
                let rec: GroupRecord = decode(&bytes, SMALL as usize)
                    .map_err(|e| ServiceError::Provider(format!("group record: {e}")))?;
                check_join(&confirmed, &rec.product)
                    .map_err(|e| ServiceError::Refused(e.to_string()))?;
                rec
            }
            JoinChoice::New { name } => GroupRecord {
                group_id: GroupId::parse(&random_id_hex()).expect("random id"),
                product: confirmed.product().to_string(),
                variant_label: confirmed.label().to_string(),
                display_name: name.clone(),
                created_by: computer.computer_id.clone(),
                created_unix_ms: now,
            },
        };
        let me = computer.computer_id.as_str().to_string();
        match provider.read("format.json", SMALL) {
            Ok(_) => {}
            Err(e) if is_missing(&e) => provider
                .write_atomic(
                    "format.json",
                    &encode(&RootFormat {
                        created_unix_ms: now,
                    }),
                    &me,
                )
                .map_err(prov)?,
            Err(e) => return Err(prov(e)),
        }
        if matches!(req.join, JoinChoice::New { .. }) {
            provider
                .write_atomic(&layout::group_record(&group.group_id), &encode(&group), &me)
                .map_err(prov)?;
        }
        let enrollment_id = EnrollmentId::parse(&random_id_hex()).expect("random id");
        let member = MemberRecord {
            group_id: group.group_id.clone(),
            computer_id: computer.computer_id.clone(),
            enrollment_id: enrollment_id.clone(),
            display_name: req.display_name.clone(),
            product: confirmed.product().to_string(),
            variant_folder: self.cfg.variant_folder.clone(),
            enrolled_unix_ms: now,
            app_version: self.cfg.app_version.clone(),
            retired: None,
            previous_enrollments: vec![],
        };
        provider
            .write_atomic(
                &layout::member_record(&group.group_id, &computer.computer_id),
                &encode(&member),
                &me,
            )
            .map_err(prov)?;
        self.state
            .write_atomic("computer.json", &encode(&computer))
            .map_err(local)?;
        let le = LocalEnrollment {
            group_id: group.group_id.clone(),
            enrollment_id,
            shared_root: self.cfg.shared_root.to_string_lossy().into_owned(),
            install_root: self.cfg.install_root.to_string_lossy().into_owned(),
            variant_folder: self.cfg.variant_folder.clone(),
            product: confirmed.product().to_string(),
            validation: confirmed.validation().clone(),
            next_seq: 1,
        };
        self.state
            .write_atomic("enrollment.json", &encode(&le))
            .map_err(local)?;
        let _ = self
            .state
            .append_audit(&format!("enrolled group {}", group.group_id));
        Ok(group.group_id)
    }

    fn client_info(v: &ConfirmedVariant) -> ClientInfo {
        ClientInfo {
            build_version: v.identity().build_version.clone(),
            toc_mapping: v.mapping().id().to_string(),
        }
    }

    fn rederive_variant(&self, e: &LocalEnrollment) -> Result<ConfirmedVariant, ServiceError> {
        let (verdict, assessment, _s, _st) = self.judge(false)?;
        confirm_enrolled(&verdict, &assessment, &e.validation).map_err(|r| match r {
            RefuseReason::StaleConfirmation => ServiceError::NeedsReconfirmation,
            RefuseReason::VariantChanged => ServiceError::VariantChanged,
            r => ServiceError::Refused(format!("{r:?}")),
        })
    }

    fn read_members(
        &self,
        provider: &ProviderRoot,
        group: &GroupId,
        me: &ComputerId,
    ) -> Result<Vec<MemberObs>, ServiceError> {
        let entries = match provider.list(&layout::members_dir(group)) {
            Ok(e) => e,
            Err(e) if is_missing(&e) => return Ok(vec![]),
            Err(e) => return Err(prov(e)),
        };
        let mut out = Vec::new();
        for e in entries.into_iter().filter(|e| e.is_dir) {
            let Ok(c) = ComputerId::parse(&e.name) else {
                continue;
            };
            if &c == me {
                continue;
            }
            let member = match provider.read(&layout::member_record(group, &c), SMALL) {
                Ok(b) => decode::<MemberRecord>(&b, SMALL as usize),
                Err(e) => Err(RecordError::Invalid(e.to_string())),
            };
            let files: Vec<StateFileObs> = match provider.list(&layout::state_dir(group, &c)) {
                Ok(list) => list
                    .into_iter()
                    .filter(|f| {
                        !f.is_dir && matches!(f.state, EntryState::Local | EntryState::CloudOnly)
                    })
                    .map(|f| StateFileObs {
                        name: f.name,
                        size: f.size,
                        cloud_only: f.state == EntryState::CloudOnly,
                    })
                    .collect(),
                Err(e) if is_missing(&e) => vec![],
                Err(e) => return Err(prov(e)),
            };
            let mut obs = MemberObs {
                computer: c.clone(),
                member,
                files,
                newest_bytes: None,
            };
            if let Some(name) = newest_state_name(&obs) {
                let cloud = obs.files.iter().any(|f| f.name == name && f.cloud_only);
                if !cloud {
                    let rel = format!("{}/{name}", layout::state_dir(group, &c));
                    obs.newest_bytes =
                        Some(provider.read(&rel, STATE_LIMIT).map_err(|e| e.to_string()));
                }
            }
            out.push(obs);
        }
        Ok(out)
    }

    fn observe_world(&mut self, persist: bool) -> Result<World, ServiceError> {
        let enrollment = self.read_enrollment()?.ok_or(ServiceError::NotEnrolled)?;
        let computer = self.read_computer()?.ok_or(ServiceError::NotEnrolled)?;
        let provider = self.open_provider()?;
        let group: GroupRecord = {
            let bytes = provider
                .read(&layout::group_record(&enrollment.group_id), SMALL)
                .map_err(prov)?;
            decode(&bytes, SMALL as usize)
                .map_err(|e| ServiceError::Provider(format!("group record: {e}")))?
        };
        let variant = self.rederive_variant(&enrollment)?;
        let (scans, stable) = self.scan()?;
        let titles = folder_titles(&scans);
        let decls = crate::settings_cycle::declaring_folders(&scans);
        let inventory = build_inventory(scans, stable, &variant);
        let marks = match self
            .state
            .read("admission.json", 1024 * 1024)
            .map_err(local)?
        {
            Some(b) => decode(&b, 1024 * 1024)
                .map_err(|e| ServiceError::Local(format!("admission.json: {e}")))?,
            None => AdmissionWatermarks::empty(group.group_id.clone()),
        };
        let obs = self.read_members(&provider, &group.group_id, &computer.computer_id)?;
        let own = EnrollmentRef {
            computer_id: computer.computer_id.clone(),
            enrollment_id: enrollment.enrollment_id.clone(),
        };
        let mut computers: Vec<ComputerId> = vec![computer.computer_id.clone()];
        self.plane.add_member(&own.computer_id, &own.enrollment_id);
        if let Ok(list) = provider.list(&layout::state_dir(&group.group_id, &computer.computer_id))
        {
            for f in list {
                if let Some((e, _)) = layout::parse_state_name(&f.name) {
                    self.plane.add_member(&computer.computer_id, &e);
                }
            }
        }
        for m in &obs {
            computers.push(m.computer.clone());
            if let Ok(rec) = &m.member {
                if rec.computer_id == m.computer {
                    self.plane.add_member(&m.computer, &rec.enrollment_id);
                    for p in &rec.previous_enrollments {
                        self.plane.add_member(&m.computer, p);
                    }
                }
            }
            for f in &m.files {
                if let Some((e, _)) = layout::parse_state_name(&f.name) {
                    self.plane.add_member(&m.computer, &e);
                }
            }
        }
        learn_from_directories(&mut self.plane, &provider, &group.group_id, &computers);
        let derived = derive(&self.plane.ledger, &group.group_id, &self.plane.members);
        if persist {
            self.plane.save(&self.state).map_err(local)?;
        }
        let standing = match derived.barrier.retired(&own.enrollment_id) {
            Retired::No => OwnStanding::Active,
            Retired::Unresolved => OwnStanding::Held,
            Retired::Yes => OwnStanding::Retired,
        };
        let (views, marks) = admit_member_states(&group, &obs, &marks);
        let (views, _retired) = apply_barrier(views, &derived.barrier);
        if persist {
            self.state
                .write_atomic("admission.json", &encode(&marks))
                .map_err(local)?;
        }
        let now = now_unix_ms();
        for v in &views {
            if let MemberLatest::Valid { seq, .. } = &v.latest {
                if self.seen_seq.get(&v.computer) != Some(seq) {
                    self.seen_seq.insert(v.computer.clone(), *seq);
                    self.seen_locally.insert(v.computer.clone(), now);
                }
            }
        }
        let mut trees = BTreeMap::new();
        let state_folders = views
            .iter()
            .filter_map(|v| match &v.latest {
                MemberLatest::Valid { state, .. } => Some(state.folders.iter()),
                _ => None,
            })
            .flatten()
            .chain(inventory.folders.iter());
        for f in state_folders {
            let Some(h) = f.tree else {
                continue;
            };
            if trees.contains_key(&h) {
                continue;
            }
            let avail = match provider.read(&layout::tree_record(&group.group_id, &h), TREE_LIMIT) {
                Ok(b) => match decode_tree(&b, &h) {
                    Ok(t) => TreeAvail::Ok(t),
                    Err(_) => TreeAvail::Corrupt,
                },
                Err(e) if e.kind == FsErrorKind::CloudOnly => TreeAvail::CloudOnly,
                Err(e) if is_missing(&e) => TreeAvail::Missing,
                Err(_) => TreeAvail::Corrupt,
            };
            trees.insert(h, avail);
        }
        if inventory.listing_stable {
            let hooks =
                RemovalBehaviour::new(&self.plane.ledger, &group.group_id, &self.plane.members);
            let _ = RemovalStore::new(&self.state, group.group_id.clone())
                .forget_group_removed(&inventory, |k| {
                    !hooks.merged_removals(&self.plane.ledger, k).live.is_empty()
                });
        }
        let held = {
            let identity = format!("{}|{}", enrollment.install_root, enrollment.variant_folder);
            let journaled = self.journaled();
            let (base, obs) = RemovalStore::new(&self.state, group.group_id.clone()).load();
            let seen = wow_sync_core::removal::observe(
                &base,
                &obs,
                &ObserveInput {
                    inventory: &inventory,
                    dir_identity: &identity,
                    variant: &enrollment.variant_folder,
                    now_unix_ms: now_unix_ms() as u64,
                    journaled: &journaled,
                },
            );
            let mut held: BTreeSet<FolderKey> = BTreeSet::new();
            for (name, _) in &seen.held {
                if let Some(fs) = seen.baseline.packages.get(name).or(base.packages.get(name)) {
                    held.extend(fs.iter().map(|f| f.key.clone()));
                }
            }
            held
        };
        let removal_hooks =
            RemovalBehaviour::new(&self.plane.ledger, &group.group_id, &self.plane.members)
                .with_inventory(inventory.clone(), enrollment.next_seq);
        let constraints = Constraints::for_derived(&derived);
        let input = TargetInput {
            group: &group,
            variant: &variant,
            local: &inventory,
            local_client: &Self::client_info(&variant),
            local_id: &computer.computer_id,
            members: &views,
            trees: &trees,
        };
        let target = compose(
            &input,
            &self.plane.ledger,
            &removal_hooks,
            &held,
            &constraints,
        )
        .target;
        let copies = package_copies(
            &input,
            &enrollment.enrollment_id,
            &self.plane.ledger,
            &constraints,
        );
        let offers = {
            let valid = wow_sync_core::choice::valid_hashes(&derived);
            let view = wow_sync_core::choice::ChoiceView::new(&self.plane.ledger, &valid);
            decision_items(
                &view,
                &wow_sync_core::choice::WriteCx {
                    group: &group.group_id,
                    issuer: Issuer {
                        computer_id: computer.computer_id.clone(),
                        enrollment_id: enrollment.enrollment_id.clone(),
                        issued_at_seq: enrollment.next_seq,
                    },
                    now_unix_ms: now_unix_ms() as u64,
                },
                &target,
                &copies,
            )
        };
        let own_states =
            match provider.list(&layout::state_dir(&group.group_id, &computer.computer_id)) {
                Ok(l) => l
                    .into_iter()
                    .filter(|e| !e.is_dir && e.state == EntryState::Local)
                    .map(|e| e.name)
                    .collect(),
                Err(_) => vec![],
            };
        let own_latest = own_publish_slot(&own_states, &enrollment.enrollment_id, 0)
            .0
            .and_then(|name| {
                let rel = format!(
                    "{}/{name}",
                    layout::state_dir(&group.group_id, &computer.computer_id)
                );
                provider
                    .read(&rel, STATE_LIMIT)
                    .ok()
                    .and_then(|b| decode::<CodeState>(&b, STATE_LIMIT as usize).ok())
            });
        Ok(World {
            enrollment,
            computer,
            group,
            variant,
            inventory,
            views,
            target,
            offers,
            copies,
            own_states,
            own_latest,
            titles,
            decls,
            own,
            standing,
        })
    }

    fn journals(
        &self,
        area: &InstallArea,
    ) -> (Vec<ApplyJournal>, Vec<crate::control::UnknownJournal>) {
        crate::control::split_journals(area.read_journals().unwrap_or_default())
    }

    /// Folders that any journal names (removal.md E5).
    fn journaled(&self) -> BTreeSet<FolderKey> {
        match InstallArea::open(&self.variant_dir()) {
            Ok(area) => journaled_folders(&area),
            Err(_) => BTreeSet::new(),
        }
    }

    fn snapshot(&mut self, w: &World, area: &InstallArea) -> StatusSnapshot {
        let (journals, unknown_journals) = self.journals(area);
        let (interrupted, needs_help) = journal_status(&journals);
        let mut retained: Vec<RecoveryPoint> = Vec::new();
        let keys: Vec<FolderKey> = w.target.packages.keys().cloned().collect();
        for k in &keys {
            for (_, b) in area.list_recovery(k.as_str()).unwrap_or_default() {
                if let Ok(rp) = decode::<RecoveryPoint>(&b, SMALL as usize) {
                    retained.push(rp);
                }
            }
        }
        let blocks: Vec<PlanBlock> = self
            .last_blocks
            .iter()
            .filter(|b| **b != PlanBlock::NothingToDo)
            .cloned()
            .collect();
        let now = now_unix_ms();
        let observed = self.activity.observe(&self.variant_dir());
        let activity = format!("{observed:?}");
        let pending = outstanding(
            load_pending(&self.state).as_ref(),
            &w.group.group_id,
            &w.enrollment.enrollment_id,
            None,
        );
        let reports: Vec<&ApplyReport> = w
            .own_latest
            .as_ref()
            .and_then(|s| s.apply_report.as_ref())
            .into_iter()
            .chain(pending.as_ref())
            .collect();
        let here = applied_here(&self.applied_here, &reports, &w.target);
        let sv: StatusView = derive_status(&StatusInput {
            target: &w.target,
            members: &w.views,
            applied_here: &here,
            interrupted: &interrupted,
            interrupted_by_game: &game_halted(&journals),
            needs_help: &needs_help,
            blocks: &blocks,
            retained: &retained,
            provider_error: self.provider_error.is_some(),
            working: false,
            last_publish_unix_ms: self.last_publish,
            seen_locally: &self.seen_locally,
        });
        let di = DetailInput {
            view: &sv,
            target: &w.target,
            inventory: &w.inventory,
            views: &w.views,
            titles: &w.titles,
            this_id: &w.computer.computer_id,
            game_running: matches!(observed, wow_sync_core::plan::Activity::Running { .. }),
        };
        let rows = build_package_rows(&di);
        let game_build = w.variant.identity().build_version.clone();
        let members = member_rows(
            &di,
            (
                &w.computer.display_name,
                &self.cfg.app_version,
                game_build.clone(),
            ),
            &rows,
        );
        let detail = StatusDetail {
            view: sv.clone(),
            packages: rows,
            members,
            game: observed.clone(),
            group_id: w.group.group_id.to_string(),
            computer_id: w.computer.computer_id.to_string(),
            variant_label: w.variant.identity().display_label(),
            variant_folder: w.enrollment.variant_folder.clone(),
            app_version: self.cfg.app_version.clone(),
            game_build,
        };
        StatusSnapshot {
            enrolled: true,
            headline: render_headline(&sv),
            group_name: w.group.display_name.clone(),
            computer_name: w.computer.display_name.clone(),
            identity_line: format!(
                "{} / {}",
                w.variant.identity().display_label(),
                w.enrollment.variant_folder
            ),
            packages: sv
                .packages
                .iter()
                .map(|p| PackageLine {
                    key: p.0.to_string(),
                    text: format!(
                        "{}: {}",
                        p.0.as_str(),
                        render_with_names(&p.2, &w.views, &w.computer.computer_id)
                    ),
                })
                .collect(),
            members: sv.members.iter().map(render_member).collect(),
            blocks: sv
                .blocks
                .iter()
                .map(render_block)
                .chain(unknown_journals.iter().map(|u| u.render()))
                .collect(),
            recovery_note: (!retained.is_empty() || !interrupted.is_empty()).then(|| {
                format!(
                    "{} recovery point(s) kept, {} interrupted update(s)",
                    retained.len(),
                    interrupted.len()
                )
            }),
            last_publish: sv
                .last_publish
                .map(|t| format!("Published {} s ago", ((now - t).max(0)) / 1000)),
            provider_error: self.provider_error.clone(),
            activity,
            retained_extra_recovery: sv.retained_extra_recovery,
            reconfirm: None,
            detail: Some(detail),
        }
    }

    /// Read-only view of the current state.
    pub fn status(&mut self) -> StatusSnapshot {
        let w = match self.observe_world(false) {
            Ok(w) => {
                self.provider_error = None;
                w
            }
            Err(ServiceError::NotEnrolled) => return StatusSnapshot::default(),
            Err(ServiceError::VariantChanged) => {
                self.provider_error = None;
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::NeedsHelp),
                    provider_error: Some(ServiceError::VariantChanged.to_string()),
                    ..Default::default()
                };
            }
            Err(ServiceError::NeedsReconfirmation) => {
                self.provider_error = None;
                let mut preview = self.preview();
                preview.groups.clear();
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::NeedsHelp),
                    provider_error: Some(ServiceError::NeedsReconfirmation.to_string()),
                    reconfirm: Some(preview),
                    ..Default::default()
                };
            }
            Err(e) => {
                self.provider_error = Some(e.to_string());
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::Error),
                    provider_error: Some(e.to_string()),
                    ..Default::default()
                };
            }
        };
        match InstallArea::open(&self.variant_dir()) {
            Ok(area) => self.snapshot(&w, &area),
            Err(e) => StatusSnapshot {
                enrolled: true,
                headline: headline_sentence(Headline::Error),
                provider_error: Some(e.to_string()),
                ..Default::default()
            },
        }
    }

    pub fn sync_once(&mut self) -> Result<SyncReport, ServiceError> {
        let mut report = SyncReport::default();
        let mut w = match self.observe_world(true) {
            Ok(w) => {
                self.provider_error = None;
                w
            }
            Err(ServiceError::NotEnrolled) => return Err(ServiceError::NotEnrolled),
            Err(e) => {
                self.provider_error = Some(e.to_string());
                return Err(e);
            }
        };
        let provider = self.open_provider()?;
        let addons = Root::open(&self.addons_path(), ReparsePolicy::RejectAll).map_err(local)?;
        let variant_dir = self.variant_dir();
        let area = InstallArea::open(&variant_dir).map_err(local)?;
        let group_id = w.group.group_id.clone();
        let me = w.computer.computer_id.as_str().to_string();
        let enrollment_id = w.enrollment.enrollment_id.clone();
        let mut new_report = None;
        let mut journals_remaining = false;
        self.last_blocks.clear();
        self.settle_kept(&w, &provider);
        if w.standing == OwnStanding::Active {
            let (items, plans): (Vec<_>, Vec<_>) =
                std::mem::take(&mut w.offers).into_iter().unzip();
            self.offer_decisions(items, plans.into_iter().flatten().collect());
        } else {
            self.offer_decisions(vec![], vec![]);
        }

        // Drop results the newest own state already carries (a crash after publishing but
        // before the store was cleared). A failure here is retried: `publish_step` refuses to
        // publish a newer state until the store is reconciled, so nothing is published twice.
        let stored = load_pending(&self.state);
        let still = outstanding(
            stored.as_ref(),
            &group_id,
            &enrollment_id,
            w.own_latest.as_ref(),
        );
        if stored.as_ref().and_then(|s| s.report.as_ref()) != still.as_ref() {
            let _ = self.state.write_atomic(
                PENDING_REPORT,
                &encode(&PendingReport {
                    group_id: group_id.clone(),
                    enrollment_id: enrollment_id.clone(),
                    report: still,
                }),
            );
        }

        {
            let real = RealEffects::new(
                &provider,
                &addons,
                &area,
                &variant_dir,
                group_id.clone(),
                me.clone(),
                &mut *self.activity,
                self.cfg.recent_ms,
            )
            .recording_into(PendingSink {
                state: &self.state,
                group: group_id.clone(),
                enrollment: enrollment_id.clone(),
            });
            let mut fx = Faulty::new(real, self.fault, &mut self.hooks);

            if let Err(e) = fx.lock() {
                report.crashed = fx.crashed();
                self.last_blocks
                    .push(PlanBlock::Invalid(format!("install lock: {}", e.detail)));
                drop(fx);
                report.status = self.snapshot(&w, &area);
                return Ok(report);
            }

            let rec = recover_interrupted(&mut fx);
            journals_remaining = journals_remaining || rec.journals_remaining;
            for (op, key, res) in &rec.ops {
                report
                    .recovered
                    .push(format!("{key} ({op}): {}", render_result(res)));
            }

            let mut applied_plan = None;
            let mut applied_outcome = None;
            if !fx.crashed() && !rec.journals_remaining {
                let gate = ApplyGate {
                    activity: fx.activity(),
                    variant: w.variant.clone(),
                    any_journal: false,
                    now_unix_ms: now_unix_ms() as u64,
                };
                let mut gen = || OpId::parse(&random_id_hex()).expect("random id");
                let game_blocks = !matches!(gate.activity, Activity::NotRunning);
                let mut journaled = journaled_folders(&area);
                if !rec.ops.is_empty() {
                    // Recovery changed the folders after the scan; judge nothing until the next one.
                    journaled.extend(w.inventory.folders.iter().map(|f| f.key.clone()));
                    journaled.extend(rec.ops.iter().map(|(_, k, _)| k.clone()));
                }
                let (extra, mut next_seq) = Self::removal_steps(
                    &self.state,
                    &mut self.plane,
                    &provider,
                    &w,
                    &journaled,
                    game_blocks,
                    &mut report,
                );
                let settings_ops = if w.standing == OwnStanding::Active && !game_blocks {
                    let out = crate::settings_cycle::settings_cycle(
                        &mut self.plane,
                        &crate::settings_cycle::StepArgs {
                            cap: self.settings_cap,
                            state: &self.state,
                            provider: &provider,
                            variant_dir: &self.cfg.install_root.join(&self.cfg.variant_folder),
                            group: &group_id,
                            own: &w.own,
                            inventory: &w.inventory,
                            decls: &w.decls,
                            game_blocks,
                        },
                        &mut next_seq,
                    );
                    out.applies
                } else {
                    Vec::new()
                };
                let rollback = if w.standing == OwnStanding::Active
                    && !game_blocks
                    && crate::rollback_flow::load_request(&self.state, &group_id).is_some()
                {
                    let request = crate::rollback_flow::load_request(&self.state, &group_id)
                        .expect("checked above");
                    let mut audience: Vec<EnrollmentRef> = w
                        .views
                        .iter()
                        .map(|v| EnrollmentRef {
                            computer_id: v.computer.clone(),
                            enrollment_id: v.enrollment.clone(),
                        })
                        .collect();
                    audience.push(w.own.clone());
                    let args = crate::rollback_flow::PlanArgs {
                        request: &request,
                        group: &group_id,
                        own: &w.own,
                        audience,
                        inventory: &w.inventory,
                        copies: &w.copies,
                        plane: &self.plane,
                        provider: &provider,
                        variant_dir: &variant_dir,
                        op: gen(),
                        seq: next_seq,
                        now_unix_ms: now_unix_ms() as u64,
                    };
                    match crate::rollback_flow::step(&self.state, &args, &w.enrollment) {
                        Ok(p) => Some(p),
                        Err(why) => {
                            report.apply_note = Some(format!("rollback not applied: {why:?}"));
                            None
                        }
                    }
                } else {
                    None
                };
                let mut planned = match plan_apply(&w.target, &w.inventory, &mut gen, &gate) {
                    Ok(p) => Some(p),
                    Err(PlanBlock::NothingToDo) => None,
                    Err(b) => {
                        report.apply_note = Some(render_block(&b));
                        self.last_blocks.push(b);
                        None
                    }
                };
                if !extra.is_empty() {
                    let p = planned.get_or_insert_with(|| ApplyPlan {
                        group_id: group_id.clone(),
                        started_unix_ms: gate.now_unix_ms,
                        packages: Vec::new(),
                        skipped: Vec::new(),
                    });
                    let mut all = extra;
                    all.append(&mut p.packages);
                    p.packages = all;
                }
                if let Some(rb) = rollback {
                    let p = planned.get_or_insert_with(|| ApplyPlan {
                        group_id: group_id.clone(),
                        started_unix_ms: gate.now_unix_ms,
                        packages: Vec::new(),
                        skipped: Vec::new(),
                    });
                    let mut mine: BTreeSet<FolderKey> =
                        rb.folders.iter().map(|f| f.key.clone()).collect();
                    mine.extend(rb.removes.iter().map(|r| r.key.clone()));
                    mine.insert(rb.package.clone());
                    p.packages.retain(|q| {
                        !mine.contains(&q.package)
                            && !q.folders.iter().any(|f| mine.contains(&f.key))
                            && !q.removes.iter().any(|r| mine.contains(&r.key))
                    });
                    p.packages.insert(0, rb);
                }
                if !settings_ops.is_empty() {
                    let p = planned.get_or_insert_with(|| ApplyPlan {
                        group_id: group_id.clone(),
                        started_unix_ms: gate.now_unix_ms,
                        packages: Vec::new(),
                        skipped: Vec::new(),
                    });
                    let taken: BTreeSet<FolderKey> = p
                        .packages
                        .iter()
                        .flat_map(|q| {
                            std::iter::once(q.package.clone())
                                .chain(q.removes.iter().map(|r| r.key.clone()))
                        })
                        .collect();
                    p.packages.extend(
                        settings_ops
                            .into_iter()
                            .filter(|s| !taken.contains(&s.package)),
                    );
                }
                match planned {
                    None => {}
                    Some(plan) => {
                        for (k, why) in &plan.skipped {
                            report.applied.push(AppliedLine {
                                package: k.to_string(),
                                text: format!("not applied: {}", explain::render(why)),
                            });
                        }
                        let outcome = run_apply(&plan, &mut fx);
                        for (k, r) in &outcome.packages {
                            report.applied.push(AppliedLine {
                                package: k.to_string(),
                                text: render_result(r),
                            });
                        }
                        remember_applied(&mut self.applied_here, &plan, &outcome);
                        let removed_here: BTreeSet<FolderKey> = plan
                            .packages
                            .iter()
                            .flat_map(|p| p.removes.iter().map(|r| r.key.clone()))
                            .collect();
                        if !removed_here.is_empty() && !fx.crashed() {
                            let _ = RemovalStore::new(&self.state, group_id.clone())
                                .forget_keys(&removed_here);
                        }
                        if let Some(b) = &outcome.stopped {
                            report.apply_note = Some(render_block(b));
                            self.last_blocks.push(b.clone());
                        }
                        applied_plan = Some(plan);
                        applied_outcome = Some(outcome);
                    }
                }
            }
            report.crashed = fx.crashed();

            if let (Some(plan), Some(outcome)) = (&applied_plan, &applied_outcome) {
                let basis: Vec<BasisEntry> = w
                    .views
                    .iter()
                    .filter_map(|v| match &v.latest {
                        MemberLatest::Valid { seq, .. } => Some(BasisEntry {
                            computer_id: v.computer.clone(),
                            seq: *seq,
                        }),
                        _ => None,
                    })
                    .collect();
                new_report = apply_report(plan, outcome, basis);
                std::thread::sleep(std::time::Duration::from_millis(self.cfg.recent_ms + 25));
            }

            if !report.crashed {
                // Finalize already recorded each applied package; add this cycle's full
                // results (with basis) so failures and rollbacks withdraw earlier claims.
                if let Some(rep) = new_report.take() {
                    let merged =
                        record_pending(load_pending(&self.state), &group_id, &enrollment_id, rep);
                    if self
                        .state
                        .write_atomic(PENDING_REPORT, &encode(&merged))
                        .is_err()
                    {
                        new_report = merged.report;
                    }
                }
                // Re-observe: an apply changed the folders, and a peer may have moved.
                let used = fx.effects_run();
                drop(fx);
                let fresh = self.observe_world(true)?;
                w = fresh;
                let store = match new_report.take() {
                    // Not saved locally: still publish it from memory this cycle.
                    Some(r) => Some(PendingReport {
                        group_id: group_id.clone(),
                        enrollment_id: enrollment_id.clone(),
                        report: Some(r),
                    }),
                    None => load_pending(&self.state),
                };
                let real = RealEffects::new(
                    &provider,
                    &addons,
                    &area,
                    &variant_dir,
                    group_id.clone(),
                    me.clone(),
                    &mut *self.activity,
                    self.cfg.recent_ms,
                );
                // The crash budget counts across the whole cycle.
                let mut fx = Faulty::new(real, self.fault, &mut self.hooks).starting_at(used);
                let mut last_publish = None;
                Self::publish_step(
                    &self.cfg,
                    &self.state,
                    &mut self.plane,
                    &provider,
                    &mut last_publish,
                    &w,
                    store,
                    journals_remaining,
                    &mut fx,
                    &mut report,
                );
                report.crashed = fx.crashed();
                if last_publish.is_some() {
                    self.last_publish = last_publish;
                }
            }
        }
        // The crash counter only matters for one cycle.
        self.fault.crash_after = None;
        if report.crashed {
            return Ok(report);
        }
        if let Ok(fresh) = self.observe_world(false) {
            w = fresh;
        }
        report.status = self.snapshot(&w, &area);
        self.log_cycle(&report);
        Ok(report)
    }

    /// Removal steps 3a, 3b and 6 (removal.md §1.6): observe and write matured removal records,
    /// write `absent` acknowledgements, and plan remove operations. Ledger writes never touch
    /// game files, so only the returned operations depend on `game_blocks_changes`. A computer
    /// whose retirement is settled or unresolved writes nothing (ops.md §2, Q-60).
    fn removal_steps(
        state: &LocalState,
        plane: &mut Plane,
        provider: &ProviderRoot,
        w: &World,
        journaled: &BTreeSet<FolderKey>,
        game_blocks_changes: bool,
        report: &mut SyncReport,
    ) -> (Vec<PackageApply>, u64) {
        if w.standing != OwnStanding::Active {
            return (Vec::new(), w.enrollment.next_seq);
        }
        let gid = &w.group.group_id;
        let now = now_unix_ms() as u64;
        let mut next_seq = w.enrollment.next_seq;
        let mut held: BTreeSet<FolderKey> = BTreeSet::new();
        let mut written = false;
        if w.inventory.listing_stable {
            let identity = format!(
                "{}|{}",
                w.enrollment.install_root, w.enrollment.variant_folder
            );
            let observed = match RemovalStore::new(state, gid.clone()).observe(&ObserveInput {
                inventory: &w.inventory,
                dir_identity: &identity,
                variant: &w.enrollment.variant_folder,
                now_unix_ms: now,
                journaled,
            }) {
                Ok(o) => o,
                Err(e) => {
                    report.apply_note = Some(format!("removal state could not be saved: {e}"));
                    return (Vec::new(), next_seq);
                }
            };
            for (name, _) in &observed.held {
                if let Some(fs) = observed.baseline.packages.get(name) {
                    held.extend(fs.iter().map(|f| f.key.clone()));
                }
            }
            let mut audience: Vec<EnrollmentRef> = w
                .views
                .iter()
                .map(|v| EnrollmentRef {
                    computer_id: v.computer.clone(),
                    enrollment_id: v.enrollment.clone(),
                })
                .collect();
            audience.push(w.own.clone());
            for r in &observed.removed {
                let rec = removal_record(gid, &w.own, audience.clone(), r, next_seq, now);
                match write_own_record(
                    plane,
                    provider,
                    gid,
                    &w.own.computer_id,
                    &AuthorityRecord::Removal(rec),
                ) {
                    Ok(_) => {
                        next_seq += 1;
                        written = true;
                    }
                    Err(e) => {
                        report.apply_note = Some(format!("removal record not written: {e}"));
                    }
                }
            }
        }
        let derived = derive(&plane.ledger, gid, &plane.members);
        let behaviour = RemovalBehaviour::new(&plane.ledger, gid, &plane.members)
            .with_inventory(w.inventory.clone(), next_seq);
        let acks = {
            let cx = CycleCx {
                group_id: gid,
                own: &w.own,
                ledger: &plane.ledger,
                derived: &derived,
                members: &plane.members,
                now_unix_ms: now,
                held: &held,
                roots: &[],
            };
            let items = behaviour.classify_due(&cx);
            step_3b(&cx, &items, next_seq)
        };
        for a in acks {
            if write_own_record(
                plane,
                provider,
                gid,
                &w.own.computer_id,
                &AuthorityRecord::Ack(a),
            )
            .is_ok()
            {
                next_seq += 1;
                written = true;
            }
        }
        if written {
            let _ = plane.save(state);
        }
        let derived = derive(&plane.ledger, gid, &plane.members);
        let cx = CycleCx {
            group_id: gid,
            own: &w.own,
            ledger: &plane.ledger,
            derived: &derived,
            members: &plane.members,
            now_unix_ms: now,
            held: &held,
            roots: &[],
        };
        let ops = step_3a(&cx, &w.inventory, next_seq, game_blocks_changes);
        (ops, next_seq)
    }

    #[allow(clippy::too_many_arguments)]
    fn publish_step<E: wow_sync_core::exec::PublishEffects>(
        cfg: &ServiceConfig,
        state: &LocalState,
        plane: &mut Plane,
        provider: &ProviderRoot,
        last_publish: &mut Option<i64>,
        w: &World,
        store: Option<PendingReport>,
        journals_remaining: bool,
        fx: &mut E,
        report: &mut SyncReport,
    ) {
        if journals_remaining {
            report.publish_note =
                Some("not published: an interrupted update needs attention".into());
            return;
        }
        if w.standing != OwnStanding::Active {
            report.publish_note = Some(match w.standing {
                OwnStanding::Retired => "not published: this computer is retired".into(),
                _ => "not published: retirement is unresolved".into(),
            });
            return;
        }
        let group_id = &w.group.group_id;
        let me = &w.computer.computer_id;
        let enrollment = &w.enrollment.enrollment_id;
        let apply_rep = outstanding(store.as_ref(), group_id, enrollment, w.own_latest.as_ref());
        let features: Vec<String> = wow_sync_core::records::FEATURES
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let published_head = w.own_latest.as_ref().and_then(|s| s.authority_index);
        let own_index = plan_own_index(plane, state, provider, group_id, &w.own, published_head);
        if let Some(n) = &own_index.note {
            report.publish_note = Some(n.clone());
        }
        let latest_seq = w.own_latest.as_ref().map_or(0, |s| s.seq);
        if !own_index.adopt.is_empty() {
            plane.mark_listed(&own_index.adopt, latest_seq);
        }
        let index_head = own_index.head;
        if !publish_needed(
            w.own_latest.as_ref(),
            &w.inventory,
            apply_rep.is_some(),
            index_head.as_ref(),
            &features,
        ) {
            return;
        }
        // Deduplication only looks at the newest own state, so the saved store must no longer
        // hold anything that state carries before a newer state may replace it (R3-M1).
        let saved = load_pending(state);
        if store_needs_reconcile(saved.as_ref(), group_id, enrollment, w.own_latest.as_ref()) {
            let reduced = PendingReport {
                group_id: group_id.clone(),
                enrollment_id: enrollment.clone(),
                report: outstanding(saved.as_ref(), group_id, enrollment, w.own_latest.as_ref()),
            };
            if state
                .write_atomic(PENDING_REPORT, &encode(&reduced))
                .is_err()
            {
                report.publish_note = Some(
                    "not published: the saved apply report could not be updated; will retry".into(),
                );
                return;
            }
        }
        let (_, seq) = own_publish_slot(&w.own_states, enrollment, w.enrollment.next_seq);
        let published_report = apply_rep.is_some();
        let mut ctx = PublishContext {
            group: group_id.clone(),
            computer: me.clone(),
            enrollment: w.enrollment.enrollment_id.clone(),
            product: w.enrollment.product.clone(),
            app_version: cfg.app_version.clone(),
            now_unix_ms: now_unix_ms() as u64,
            seq,
            client: Self::client_info(&w.variant),
            apply_report: apply_rep,
            own_state_files: w.own_states.clone(),
            authority_index: index_head,
            features,
        };
        let plan = match plan_publish(&w.inventory, &ctx) {
            Ok(p) => p,
            Err(b) => {
                report.publish_note = Some(format!("not published: {}", render_block(&b)));
                return;
            }
        };
        // The sequence number is persisted before anything is published so it is never reused.
        let mut le = w.enrollment.clone();
        le.next_seq = seq + 1;
        if state.write_atomic("enrollment.json", &encode(&le)).is_err() {
            report.publish_note = Some("not published: local state could not be saved".into());
            return;
        }
        match run_publish(&plan, &mut ctx, fx) {
            PublishOutcome::Published { uploaded, .. } => {
                *last_publish = Some(now_unix_ms());
                plane.mark_listed(&own_index.newly, seq);
                let _ = plane.save(state);
                report.published = Some(format!("published seq {seq} ({uploaded} new objects)"));
                if published_report {
                    // Published once. If this write is lost, `outstanding` drops it while this
                    // state is the newest, and the next publication first reconciles the store.
                    let _ = state.write_atomic(
                        PENDING_REPORT,
                        &encode(&PendingReport {
                            group_id: group_id.clone(),
                            enrollment_id: enrollment.clone(),
                            report: None,
                        }),
                    );
                }
            }
            PublishOutcome::Aborted(b) => {
                report.publish_note = Some(format!("not published: {}", render_block(&b)));
            }
            PublishOutcome::Failed(why) => {
                report.publish_note = Some(format!("publish failed: {why}"));
            }
        }
    }

    pub fn state_dir(&self) -> &Path {
        &self.cfg.state_dir
    }

    /// When syncing was paused on this computer, or `None` (UG-2). Persisted; lasts until resumed.
    pub fn paused_since(&self) -> Option<i64> {
        crate::control::paused_since(&self.state)
    }

    /// Running, paused since a time, or a damaged pause file that holds syncing.
    pub fn pause_state(&self) -> crate::control::PauseState {
        crate::control::pause_state(&self.state)
    }

    pub fn set_paused(&self, paused: bool) -> Result<(), ServiceError> {
        crate::control::set_paused(&self.state, paused).map_err(ServiceError::Local)?;
        crate::control::log_activity(
            &self.state,
            if paused { "paused" } else { "resumed" },
            None,
            if paused {
                "Syncing paused on this computer"
            } else {
                "Syncing resumed on this computer"
            },
        );
        Ok(())
    }

    pub fn app_settings(&self) -> crate::control::AppSettings {
        crate::control::AppSettings::load(&self.state)
    }

    pub fn save_app_settings(&self, s: &crate::control::AppSettings) -> Result<(), ServiceError> {
        s.save(&self.state).map_err(ServiceError::Local)
    }

    /// Newest first (UG-7).
    pub fn activity_log(&self, limit: usize) -> Vec<crate::control::ActivityEntry> {
        crate::control::read_activity(&self.state, limit)
    }

    fn log_cycle(&self, r: &SyncReport) {
        use crate::control::log_activity as log;
        for l in &r.recovered {
            log(&self.state, "recovered", None, l);
        }
        for a in &r.applied {
            log(&self.state, "applied", Some(&a.package), &a.text);
        }
        if let Some(p) = &r.published {
            log(&self.state, "published", None, p);
        }
    }
}

fn dummy_identity() -> wow_sync_core::variant::VariantIdentity {
    wow_sync_core::variant::VariantIdentity {
        product: wow_sync_core::variant::ProductCode::parse("wowf").expect("valid code"),
        folder: String::new(),
        build_version: None,
        profile: None,
    }
}
