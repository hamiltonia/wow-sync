//! `SliceService`: the thin-slice orchestration. Every decision (variant safety, admission,
//! target, plan, status wording) is made by `wow-sync-core`; this file gathers observations
//! through the adapters, calls core, and runs the resulting plans through the effects.

use crate::activity::{ActivitySource, WinprocActivity};
use crate::control::{
    inventory_without, target_without, withheld_local_versions, Command, CommandError,
    CommandOutcome, LocalOnlyVersion,
};
use crate::decisions::{
    learn_from_directories, plan_own_index, route, write_record, Constraints, KeptQueue,
    KeptRecord, Plane, RouteCx, Routed,
};
use crate::decisions::{write_own_bytes, write_own_record};
use crate::detail::{
    build_package_rows, folder_titles, member_rows, render_with_names, unrecognised_note,
    DetailInput, StatusDetail,
};
use crate::effects::{
    load_pending, FaultPlan, Faulty, Hooks, PendingSink, RealEffects, PENDING_REPORT,
};
use crate::removal_cycle::{
    step_3a_with, step_3b, Gathered, RemovalStore, SettingsEnv, SettingsPass,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use wow_sync_adapters::fs_install::InstallArea;
use wow_sync_adapters::fs_safe::{ReparsePolicy, Root};
use wow_sync_adapters::fs_scan::{
    list_variant_folders, read_variant_files, scan_addons_ctl, DirKind, FileRead, ScanOptions,
    ScanStatus as AdScanStatus,
};
pub use wow_sync_adapters::fs_scan::{Interrupt, ScanCtl};
pub use wow_sync_adapters::hash_cache::HashCache;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::{EntryState, ProviderRoot};
use wow_sync_adapters::sysinfo::{now_unix_ms, random_id_hex};
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::admit::{
    admit_member_states, apply_barrier, newest_state_name, MemberLatest, MemberObs, MemberView,
    StateFileObs,
};
use wow_sync_core::authority::{decode_authority, AuthorityRecord, Retired};
use wow_sync_core::decide::decision_items;
use wow_sync_core::decide::{DecideBehaviour, Standing};
use wow_sync_core::decision::CycleCx;
use wow_sync_core::decision::{
    DecideError, DecideOutcome, DecisionEffect, DecisionId, DecisionScope, FreshEvidence,
};
use wow_sync_core::exec::{
    apply_report, recover_interrupted, run_apply, ApplyEffects, PackageResult, PublishOutcome,
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
    own_publish_slot, plan_apply, publish_needed, ApplyGate, PlanBlock, PublishContext,
};
use wow_sync_core::plan::{Activity, ApplyPlan, PackageApply};
use wow_sync_core::records::{
    decode, decode_tree, encode, AdmissionWatermarks, ApplyJournal, ApplyReport, BasisEntry,
    CodeState, GroupRecord, LocalComputer, LocalEnrollment, MemberRecord, PendingReport,
    RecordError, RecoveryPoint, RootFormat,
};
use wow_sync_core::reject::{build_lift, LiftError, RejectBehaviour};
use wow_sync_core::removal::known_summary;
use wow_sync_core::removal::{
    plan_remove_ops, removal_record_with_settings, ObserveInput, RemovalBehaviour,
};
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
    FileObs, GuidedConfirmation, LayoutAssessment, MarkerPolicy, OtherVariants, RefuseReason,
    VariantObservation, VariantVerdict,
};
use wow_sync_core::variant::{BlockReason, LayoutReason};

/// A refusal in words the owner can act on: what is wrong, then what to do.
/// `retry` is true only when something outside WoW Sync can change the answer.
pub fn refuse_text(r: &RefuseReason) -> (String, bool) {
    const BACK: &str = "Go Back and choose another folder.";
    let (text, retry) = match r {
        RefuseReason::FlavorMissing => (
            "This folder does not look like a game installation: its .flavor.info file is missing. \
             Start the game once from Battle.net so it is created, then choose Try again. If it is \
             the wrong folder, go Back and choose another.".to_string(),
            true,
        ),
        RefuseReason::FlavorUnreadable(_) => (
            "WoW Sync could not read this folder's .flavor.info file. Close anything that may be \
             using the game folder and check you have permission to read it, then choose Try again."
                .to_string(),
            true,
        ),
        RefuseReason::FlavorUnparseable => (
            format!("This folder's .flavor.info file is not in the expected format, so the game version cannot be told. {BACK}"),
            false,
        ),
        RefuseReason::InvalidProductCode => (
            format!("This folder names a game product WoW Sync does not recognise. {BACK}"),
            false,
        ),
        RefuseReason::BuildInfo(_) => (
            "WoW Sync could not read the .build.info file that describes the game version. Check \
             that the game is installed completely and that you can read the game folder, then \
             choose Try again.".to_string(),
            true,
        ),
        RefuseReason::ProductNotInBuildInfo => (
            format!("The game's .build.info file does not list this game version, so it cannot be checked. {BACK}"),
            false,
        ),
        RefuseReason::ProductAmbiguousInBuildInfo => (
            format!("The game's .build.info file lists this game version more than once, so WoW Sync cannot tell which entry is right. {BACK}"),
            false,
        ),
        RefuseReason::ProductInactive => (
            "The game version in this folder is not marked as installed and active. Finish the \
             install or update in Battle.net, then choose Try again.".to_string(),
            true,
        ),
        RefuseReason::FolderMismatch => (
            format!("This folder has the wrong name for the game version it contains, so it may be a copy or a renamed folder. {BACK}"),
            false,
        ),
        RefuseReason::AddOnsMissing => (
            "This installation has no Interface\\AddOns folder yet. Start the game once, or \
             create the folder, then choose Try again.".to_string(),
            true,
        ),
        RefuseReason::NotSyntheticInstall => (
            "Test mode is on, and this folder is not marked as a test folder, so WoW Sync will not use it. Turn off test mode to use this folder, or go Back and choose a test folder.".to_string(),
            false,
        ),
        RefuseReason::Layout(l) => return layout_text(l),
        RefuseReason::ConfirmationRequired => (
            "Confirm both the game version and the add-on list to continue. Nothing has been changed."
                .to_string(),
            false,
        ),
        RefuseReason::StaleConfirmation => (
            "The installation changed after you looked at it. Check it again and confirm. Nothing has been changed."
                .to_string(),
            false,
        ),
        RefuseReason::VariantChanged => (
            "This is not the game version that was set up for this group. Go Back and choose the original installation."
                .to_string(),
            false,
        ),
    };
    (text, retry)
}

pub fn layout_text(r: &LayoutReason) -> (String, bool) {
    match r {
        LayoutReason::AddOnsLayout => (
            "The AddOns folder is not an ordinary folder (it may be a shortcut or link), so WoW \
             Sync will not change it. Go Back and choose another installation.".into(),
            false,
        ),
        LayoutReason::DuplicateProductFolder => (
            "Another folder on this computer holds the same game version, so WoW Sync cannot tell \
             which one is yours. Go Back and choose one, or remove the extra copy and choose Try again."
                .into(),
            true,
        ),
        LayoutReason::AlreadyEnrolled => (
            "This game version is already set up on this computer. Go Back and choose another installation."
                .into(),
            false,
        ),
        LayoutReason::NotSyntheticInstall => (
            "Test mode is on, and this folder is not marked as a test folder, so WoW Sync will not use it. Turn off test mode to use this folder, or go Back and choose a test folder."
                .into(),
            false,
        ),
    }
}

/// Why one add-on folder is left out of syncing.
pub fn block_text(r: &BlockReason) -> &'static str {
    match r {
        BlockReason::UnsupportedName(_) => "its folder name cannot be used for syncing",
        BlockReason::CaseCollision => "another folder has the same name apart from capital letters",
        BlockReason::NotReadable(_) => "WoW Sync cannot read it",
    }
}

const SMALL: u64 = 64 * 1024;
const STATE_LIMIT: u64 = 16 * 1024 * 1024;
const TREE_LIMIT: u64 = 32 * 1024 * 1024;
/// A scan this young is reused within one sync pass when nothing in the pass changed the install.
const RECENT_SCAN_MS: u128 = 10_000;

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
    /// Test mode is on and this install has no synthetic marker: syncing is blocked until test
    /// mode is turned off (D-28). Nothing was changed or un-enrolled.
    TestModeBlocked,
    /// A read-only look at the install was abandoned because the app is quitting. Nothing was
    /// changed by the abandoned step.
    Cancelled,
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
            Self::TestModeBlocked => write!(
                f,
                "Test mode is on, and this installation does not carry the test marker, so \
                 syncing is paused. Nothing was changed; turn test mode off to continue"
            ),
            Self::Cancelled => write!(f, "stopped before it finished; nothing was changed"),
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
    /// True only when fixing something outside WoW Sync can change `problem`.
    pub retryable: bool,
    /// True when test mode is what refuses this folder, so turning it off changes the result.
    pub test_mode_blocked: bool,
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
    /// Every enrolled computer with its enrollment and standing (retire and master choices).
    pub peers: Vec<crate::detail::PeerRow>,
    /// Settings consent, accounts, master and paused folders.
    pub settings: crate::settings_cycle::SettingsSnapshot,
    /// Per settings unit: status, live, competing heads, history and whether Share is possible.
    pub settings_units: Vec<crate::settings_cycle::SettingsUnitRow>,
    /// Settings units waiting for the owner to choose between competing shared states. The
    /// headline never says the computer is up to date while this is above zero.
    pub settings_decisions_pending: u32,
    /// How much is held and why (ops.md §1.6). `None` shows no scope sentence.
    pub hold: wow_sync_core::status::HoldScope,
    /// Set when this computer's own enrollment is retired and the retirement is effective.
    pub this_retirement: Option<wow_sync_core::status::RetiredInfo>,
    /// Effective retirements of any computer, for the Computers "Retired" group.
    pub retirements: Vec<crate::detail::RetirementRow>,
    /// Older complete releases a rollback can restore, per package.
    pub held_releases: Vec<crate::detail::HeldRelease>,
    /// Packages the group removed and that are still removed.
    pub removed: Vec<crate::detail::RemovedRow>,
    /// Parallel to `removed`: the record hash `Command::RestoreRemoved { removal }` takes.
    pub removal_hashes: Vec<Sha256>,
    /// Releases a rollback rejected, with the hashes `Command::AllowAgain` takes.
    pub rejected_releases: Vec<crate::detail::RejectedRelease>,
    /// Name of a live computer too old to honour a rollback, when there is one.
    pub older_peer: Option<String>,
    /// Every known enrollment id with the name its computer's owner chose.
    pub enrollment_names: BTreeMap<EnrollmentId, String>,
    /// "Review needed" (ops.md §1.7): a guided reconfirmation is pending and neither
    /// `NeedsHelp` nor `Error` applies. The app shows it above Paused and Working.
    pub review_needed: bool,
    /// Newer local-only versions held back after a keep (Q-88): installed here, not shared.
    pub local_only_versions: Vec<LocalOnlyVersion>,
    /// Conditions that put the owner's data at risk. Always shown in the app (headline, tray,
    /// Needs attention), whatever the OS notification toggle says (Q-85, Q-86).
    pub data_at_risk: Vec<crate::notices::DataAtRisk>,
    /// The newest update-check result; `None` until an update service has run.
    pub update: Option<wow_sync_core::update::UpdateResult>,
    /// One entry per apply journal on disk, for the diagnostics export (ops.md §5.3 Holds).
    pub journal_states: Vec<wow_sync_core::diagnostics::JournalState>,
    /// How the latest rollback request ended when it did not complete (rollback.md §1: "not
    /// available, try later"); cleared when a new rollback is requested or one applies.
    pub rollback_outcome: Option<crate::rollback_flow::RollbackOutcome>,
    /// Restores of removed add-ons that have not finished: waiting, or refused with a reason
    /// that stays until the owner retries (`Command::RestoreRemoved`) or dismisses it
    /// (`Command::DismissRestoreIssue`).
    pub restores: Vec<crate::restore_flow::RestoreProgress>,
    /// Releases held in local recovery points, newest first per add-on (rollback.md §7).
    /// Restore one with `Command::Rollback { package, recovered: row.release }`.
    pub recovery_points: Vec<crate::detail::RecoveryRow>,
    /// When a held release was set aside, by release hash, for the rollback candidates in
    /// `held_releases`. Only releases this computer holds locally have a date: no shared record
    /// carries a per-release date (the code state has none), so a peer-only release has no entry.
    pub held_release_dates: std::collections::BTreeMap<wow_sync_core::ids::Sha256, u64>,
    /// Test mode is on for this computer (D-28): only install roots with the synthetic marker
    /// sync. Off by default; `Command::SetTestMode` changes it.
    pub test_mode: bool,
    /// Test mode is what stops this enrolled install from syncing: it is on and the install does
    /// not carry the marker. Turning test mode off clears it.
    pub test_mode_blocked: bool,
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
    /// Folders of a live keep whose package has a newer copy that only this computer holds
    /// (Q-88): neither replaced by the keep nor published until the owner answers.
    withheld: BTreeSet<FolderKey>,
    local_only: Vec<crate::control::LocalOnlyVersion>,
    own_states: Vec<String>,
    /// This enrollment's newest published state, if readable.
    own_latest: Option<CodeState>,
    titles: BTreeMap<String, String>,
    decls: Vec<wow_sync_core::settings::DeclaringFolder>,
    own: EnrollmentRef,
    standing: OwnStanding,
    /// Computer names from the member records, including computers the barrier hides.
    names: BTreeMap<ComputerId, String>,
}

pub struct SliceService {
    cfg: ServiceConfig,
    activity: Box<dyn ActivitySource>,
    progress: crate::history::ProgressHub,
    scan_total_hint: u64,
    hooks: Hooks,
    fault: FaultPlan,
    state: LocalState,
    seen_locally: BTreeMap<ComputerId, i64>,
    /// Decoded provider trees reused by the observations of one pass; emptied when the pass ends.
    tree_cache: BTreeMap<Sha256, (std::time::Instant, TreeAvail)>,
    seen_seq: BTreeMap<ComputerId, u64>,
    last_publish: Option<i64>,
    applied_here: Vec<(FolderKey, wow_sync_core::explain::Explanation)>,
    last_blocks: Vec<PlanBlock>,
    provider_error: Option<String>,
    plane: Plane,
    /// The saved-settings size cap (32 MiB, owner decision D-26); tests may change it.
    settings_cap: Option<u64>,
    /// Why units are not syncing, as of the latest settings cycle. Never logged.
    settings_facts: crate::settings_cycle::SettingsFacts,
    /// The text the latest diagnostics preview returned; a previewed save must match it.
    previewed: Option<String>,
    /// The decisions currently offered to the owner and what answering each one does.
    offered: (
        Vec<wow_sync_core::decision::DecisionItem>,
        wow_sync_core::decide::Plans,
    ),
    /// Data-at-risk conditions the latest apply produced (recovery copy failed, disk full).
    at_risk_apply: Vec<crate::notices::DataAtRisk>,
    /// Delivers notices at the end of a cycle; none until a notifier is attached.
    notices: Option<crate::notices::NoticeRunner>,
    /// Update checks; none until a release source is attached.
    updates: Option<crate::updates::UpdateService>,
    started_ms: i64,
    /// Read-only scans of the install stop at the next file once its flag is set (the app is
    /// quitting). Steps that change game files never consult it.
    read_ctl: ScanCtl,
    /// The newest complete local scan, for commands that need no fresh local evidence.
    last_scan: std::sync::Mutex<Option<(Vec<FolderScan>, bool)>>,
    /// Set for one observation: answer from last_scan instead of reading the install.
    reuse_scan: std::sync::atomic::AtomicBool,
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
            progress: crate::history::ProgressHub::new(),
            scan_total_hint: 0,
            hooks: Hooks::default(),
            fault: FaultPlan::default(),
            state,
            seen_locally: BTreeMap::new(),
            tree_cache: BTreeMap::new(),
            seen_seq: BTreeMap::new(),
            last_publish: None,
            applied_here: Vec::new(),
            last_blocks: Vec::new(),
            provider_error: None,
            plane,
            settings_cap: wow_sync_core::records::MAX_SETTINGS_FILE_BYTES,
            settings_facts: Default::default(),
            previewed: None,
            offered: Default::default(),
            at_risk_apply: Vec::new(),
            notices: None,
            updates: None,
            started_ms: now_unix_ms(),
            read_ctl: ScanCtl::default(),
            last_scan: Default::default(),
            reuse_scan: Default::default(),
        })
    }

    fn shared_state(&self) -> Result<std::sync::Arc<LocalState>, ServiceError> {
        LocalState::open_at(&self.cfg.state_dir)
            .map(std::sync::Arc::new)
            .map_err(local)
    }

    fn activity_sink(&self) -> Result<crate::updates::ActivitySink, ServiceError> {
        let store = self.shared_state()?;
        Ok(std::sync::Arc::new(move |text: &str| {
            crate::control::log_activity(&store, "update-check-failed", None, text)
        }))
    }

    /// Delivers the end-of-cycle notices through `notifier` (ops.md §3).
    pub fn attach_notifier(
        &mut self,
        notifier: std::sync::Arc<dyn wow_sync_adapters::notify::Notifier>,
        policy: wow_sync_core::notice::NoticePolicy,
    ) -> Result<(), ServiceError> {
        let sink = self.activity_sink()?;
        self.notices = Some(crate::notices::NoticeRunner::new(
            self.shared_state()?,
            notifier,
            policy,
            sink,
        ));
        Ok(())
    }

    /// Checks for updates through `source` on the automatic schedule and on request (ops.md §4).
    pub fn attach_updates(
        &mut self,
        source: std::sync::Arc<dyn wow_sync_adapters::update_http::ReleaseSource>,
        clock: crate::updates::Clock,
    ) -> Result<(), ServiceError> {
        let current = wow_sync_core::update::SemVer::parse(&self.cfg.app_version)
            .or_else(|| wow_sync_core::update::SemVer::parse(env!("CARGO_PKG_VERSION")))
            .ok_or_else(|| {
                ServiceError::Refused("the app version is not a release number".into())
            })?;
        let sink = self.activity_sink()?;
        self.updates = Some(crate::updates::UpdateService::new(
            self.shared_state()?,
            source,
            current,
            clock,
            sink,
        ));
        Ok(())
    }

    /// Production wiring: registers the toast identity (Windows), then attaches the OS notifier
    /// and the GitHub release source.
    pub fn attach_os_services(&mut self) -> Result<(), ServiceError> {
        use wow_sync_adapters::notify::{OsNotifier, APP_ID, APP_NAME};
        #[cfg(windows)]
        {
            // A failed registration only costs the app name on toasts.
            let _ = wow_sync_adapters::winproc::register_app_user_model_id(APP_ID, APP_NAME, None);
            let _ = wow_sync_adapters::winproc::set_process_app_user_model_id(APP_ID);
        }
        let source = wow_sync_adapters::update_http::GithubReleases::new()
            .map_err(|e| ServiceError::Refused(format!("update source unavailable: {e:?}")))?;
        self.attach_notifier(
            std::sync::Arc::new(OsNotifier::with_app_id(APP_ID)),
            wow_sync_core::notice::NoticePolicy::default(),
        )?;
        let _ = APP_NAME;
        self.attach_updates(std::sync::Arc::new(source), crate::updates::system_clock())
    }

    /// Blocks until no update check is in flight (tests and shutdown).
    pub fn wait_for_updates(&self) {
        if let Some(u) = &self.updates {
            u.wait_idle();
        }
    }

    fn data_at_risk(&self) -> Vec<crate::notices::DataAtRisk> {
        let mut v: Vec<_> = crate::notices::local_state_risk(&self.state)
            .into_iter()
            .collect();
        v.extend(self.at_risk_apply.iter().cloned());
        v
    }

    /// Notices and the update schedule at the end of a cycle (ops.md §3, §4.2).
    fn notify_cycle(&mut self, w: &World, report: &SyncReport) {
        let settings = self.toggles();
        let now = now_unix_ms();
        if let Some(u) = &self.updates {
            u.tick(settings.update_check, now - self.started_ms);
        }
        let Some(runner) = &self.notices else { return };
        use wow_sync_core::notice::{
            fingerprint, GameActivity, NoticeEvent, NoticeKey, NoticeKind, NoticePrefs, NoticeScope,
        };
        let mut events: Vec<NoticeEvent> = Vec::new();
        let mut open: BTreeSet<NoticeKey> = BTreeSet::new();
        for item in &self.offered.0 {
            let name = match &item.scope {
                wow_sync_core::decision::DecisionScope::Folders(keys) => keys
                    .iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                wow_sync_core::decision::DecisionScope::Settings(_) => "Saved settings".to_string(),
            };
            let key = NoticeKey {
                kind: NoticeKind::ActionNeeded,
                scope: NoticeScope::Addon(name.clone()),
            };
            open.insert(key.clone());
            events.push(NoticeEvent {
                key,
                fingerprint: fingerprint(format!("{:?}", item.reason).as_bytes()),
                name,
            });
        }
        for risk in self.data_at_risk() {
            open.insert(risk.key());
            events.push(risk.event());
        }
        if w.standing == OwnStanding::Retired {
            let name = w.computer.display_name.clone();
            let key = NoticeKey {
                kind: NoticeKind::ActionNeeded,
                scope: NoticeScope::Computer(name.clone()),
            };
            open.insert(key.clone());
            events.push(NoticeEvent {
                key,
                fingerprint: fingerprint(b"retired"),
                name,
            });
        }
        for a in &report.applied {
            events.push(NoticeEvent {
                key: NoticeKey {
                    kind: NoticeKind::Summary,
                    scope: NoticeScope::Addon(a.package.clone()),
                },
                fingerprint: fingerprint(a.text.as_bytes()),
                name: a.package.clone(),
            });
        }
        let mut update_tag = None;
        if let Some(u) = &self.updates {
            let (event, still_open) = u.notice_event();
            if let Some(k) = still_open {
                open.insert(k);
            }
            if let Some(e) = event {
                if let NoticeScope::Update(tag) = &e.key.scope {
                    update_tag = Some(tag.clone());
                }
                events.push(e);
            }
        }
        let game = match self.activity.observe(&self.variant_dir()) {
            Activity::NotRunning => GameActivity::NotRunning,
            Activity::Running { .. } => GameActivity::Running,
            Activity::Unknown(_) => GameActivity::Unknown,
        };
        let out = runner.after_cycle(
            &events,
            &open,
            game,
            NoticePrefs {
                summaries: settings.notify_summaries,
                needs_me: settings.notify_needs_me,
            },
            now,
        );
        if let (Some(tag), Some(u)) = (update_tag, &self.updates) {
            if out.notices.iter().any(|n| n.kind == NoticeKind::AppUpdate) {
                u.note_notified(&tag);
            }
        }
    }

    pub(crate) fn redaction_context(
        &self,
        snap: &StatusSnapshot,
    ) -> wow_sync_core::diagnostics::RedactionContext {
        let env = |a: &str, b: &str| {
            std::env::var(a)
                .or_else(|_| std::env::var(b))
                .unwrap_or_default()
        };
        let mut names: Vec<String> = snap
            .detail
            .iter()
            .flat_map(|d| d.members.iter().map(|m| m.name.clone()))
            .collect();
        if !snap.computer_name.is_empty() {
            names.push(snap.computer_name.clone());
        }
        names.sort();
        names.dedup();
        let (wtf_accounts, wtf_realms, wtf_chars) =
            crate::settings_cycle::wtf_names(&self.variant_dir());
        let mut computers = vec![env("COMPUTERNAME", "HOSTNAME")];
        computers.retain(|c| !c.is_empty());
        wow_sync_core::diagnostics::RedactionContext {
            home_dir: env("USERPROFILE", "HOME"),
            os_user_name: env("USERNAME", "USER"),
            computer_names: computers,
            display_names: names,
            install_roots: vec![self.cfg.install_root.to_string_lossy().into_owned()],
            shared_root: self.cfg.shared_root.to_string_lossy().into_owned(),
            dropbox_root: self.cfg.shared_root.to_string_lossy().into_owned(),
            local_state_root: self.cfg.state_dir.to_string_lossy().into_owned(),
            wtf_account_names: wtf_accounts,
            wtf_realm_names: wtf_realms,
            wtf_character_names: wtf_chars,
        }
    }

    fn diagnostics_command(&mut self, cmd: &Command) -> Result<CommandOutcome, CommandError> {
        // Diagnostics need no fresh local evidence: both looks below use the newest complete scan.
        let snap = self.status_with(true);
        let wants_fingerprints = matches!(
            cmd,
            Command::ExportDiagnostics { options } if options.include_fingerprints
        );
        let fingerprints = match wants_fingerprints
            .then(|| self.observe_for_command(true))
            .transpose()
        {
            Ok(Some(w)) => crate::diag::fingerprints_from(&snap, &w.inventory),
            _ => Default::default(),
        };
        let s = self.toggles();
        let input = crate::diag::input_from(
            &snap,
            wow_sync_core::diagnostics::Toggles {
                update_check: s.update_check,
                notify_summaries: s.notify_summaries,
                notify_needs_me: s.notify_needs_me,
            },
            &self.activity_log(50),
            now_unix_ms(),
            &fingerprints,
        );
        let ctx = self.redaction_context(&snap);
        let result = match cmd {
            Command::ExportDiagnostics { options } => {
                crate::diag::export_diagnostics(&input, &ctx, *options)
                    .map_err(crate::diag::ExportError::Refused)
            }
            Command::SavePreviewedDiagnostics { export, dest } => {
                if self.previewed.as_deref() != Some(export.text.as_str()) {
                    return Err(CommandError::Decide(
                        wow_sync_core::decision::DecideError::Changed,
                    ));
                }
                let forbidden = vec![self.cfg.shared_root.clone(), self.cfg.install_root.clone()];
                crate::diag::save_export(export, dest, &forbidden)
                    .map(|()| export.clone())
                    .map_err(crate::diag::ExportError::Save)
            }
            _ => return Err(CommandError::NotAllowed),
        };
        if let (Command::ExportDiagnostics { .. }, Ok(export)) = (cmd, &result) {
            self.previewed = Some(export.text.clone());
        }
        result.map(CommandOutcome::Exported).map_err(|e| {
            crate::control::log_activity(
                &self.state,
                "export-refused",
                None,
                "Diagnostics export was refused",
            );
            CommandError::Export(e)
        })
    }

    /// Test-only: a settings size cap other than the production 32 MiB (`None` turns settings
    /// sync off).
    #[cfg(test)]
    pub(crate) fn set_settings_cap_for_tests(&mut self, cap: Option<u64>) {
        self.settings_cap = cap;
    }

    /// Drops the offers this computer already answered with a record kept for later (paused or
    /// held): offering them again would let a second answer conflict with the first.
    fn hide_answered(&self, w: &mut World) {
        let queued = self.queued_scopes(&w.group.group_id);
        Self::hide_scopes(w, &queued);
    }

    fn queued_scopes(&self, gid: &GroupId) -> Vec<Vec<FolderKey>> {
        KeptQueue::load(&self.state, gid)
            .items()
            .iter()
            .filter_map(|k| decode_authority(&k.bytes, &k.hash).ok())
            .filter_map(|r| match r {
                AuthorityRecord::CodeChoice(c) => Some(c.folders),
                _ => None,
            })
            .collect()
    }

    fn hide_scopes(w: &mut World, scopes: &[Vec<FolderKey>]) {
        w.offers.retain(|(i, _)| match &i.scope {
            DecisionScope::Folders(f) => {
                let mut f = f.clone();
                f.sort();
                !scopes.contains(&f)
            }
            _ => true,
        });
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
        match &cmd {
            Command::CheckUpdates => {
                return match &self.updates {
                    Some(u) => Ok(CommandOutcome::UpdateCheck(u.check_now())),
                    None => Err(CommandError::UpdatesUnavailable),
                };
            }
            Command::ExportDiagnostics { .. } | Command::SavePreviewedDiagnostics { .. } => {
                return self.diagnostics_command(&cmd);
            }
            Command::SetTestMode(on) => {
                crate::control::set_test_mode(&self.state, *on).map_err(CommandError::Service)?;
                crate::control::log_activity(
                    &self.state,
                    if *on { "test-mode-on" } else { "test-mode-off" },
                    None,
                    if *on {
                        "Test mode turned on: only folders with the test marker can sync"
                    } else {
                        "Test mode turned off"
                    },
                );
                return Ok(CommandOutcome::Recorded);
            }
            _ => {}
        }
        if cmd == Command::ReEnroll {
            return self.re_enroll();
        }
        // Commands that only touch the ledger answer from the newest complete scan; the others
        // look again through the hash cache (no full read, a pending full check stays queued).
        let ledger_only = matches!(
            cmd,
            Command::Retire { .. } | Command::AllowAgain { .. } | Command::SetSettingsPolicy { .. }
        );
        // A decision looks again only at the folders it is about: its evidence covers nothing
        // else, so the rest of the tree comes from the newest complete scan.
        let only = match &cmd {
            Command::Decide {
                item: DecisionId(DecisionScope::Folders(keys)),
                ..
            } => Some(keys.iter().map(|k| k.as_str().to_string()).collect()),
            _ => None,
        };
        let mut w = self
            .observe_for_command_in(ledger_only, only)
            .map_err(|e| CommandError::Service(e.to_string()))?;
        self.hide_answered(&mut w);
        let provider = self
            .open_provider()
            .map_err(|e| CommandError::Service(e.to_string()))?;
        if w.standing == OwnStanding::Retired {
            return Err(CommandError::NotAllowed);
        }
        let gid = w.group.group_id.clone();
        if let Command::DismissRestoreIssue { package } = &cmd {
            let derived = derive(&self.plane.ledger, &gid, &self.plane.members);
            let shown = crate::restore_flow::progress(
                &self.state,
                &gid,
                &self.plane.ledger,
                &derived,
                false,
                false,
            );
            return Ok(
                if shown.iter().any(|p| {
                    &p.package == package
                        && matches!(
                            p.state,
                            crate::restore_flow::RestoreState::Unavailable { .. }
                        )
                }) {
                    crate::restore_flow::clear_issue(&self.state, &gid);
                    CommandOutcome::Recorded
                } else {
                    CommandOutcome::NothingToDo
                },
            );
        }
        if let Command::EnableSettings { included_accounts } = &cmd {
            if included_accounts.is_empty() {
                return Err(CommandError::EmptySelection);
            }
            let mut l = crate::settings_cycle::load_local(&self.state, &gid);
            l.consent = true;
            l.included_accounts = included_accounts.clone();
            l.included_accounts.sort();
            l.included_accounts.dedup();
            crate::settings_cycle::save_local(&self.state, &gid, &l)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        if matches!(cmd, Command::ShareSettingsWhenReady) {
            let mut l = crate::settings_cycle::load_local(&self.state, &gid);
            if !l.consent || !self.is_settings_master(&gid, &w.own, None) {
                return Err(CommandError::NotAllowed);
            }
            l.share_when_ready = true;
            crate::settings_cycle::save_local(&self.state, &gid, &l)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        if matches!(cmd, Command::PauseAllSettings | Command::ResumeAllSettings) {
            let mut l = crate::settings_cycle::load_local(&self.state, &gid);
            let want = matches!(cmd, Command::PauseAllSettings);
            if l.paused_all == want {
                return Ok(CommandOutcome::NothingToDo);
            }
            l.paused_all = want;
            crate::settings_cycle::save_local(&self.state, &gid, &l)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        if let Command::PauseSettings { folder } | Command::ResumeSettings { folder, .. } = &cmd {
            use wow_sync_core::settings::ResumeOption::*;
            if !w.inventory.folders.iter().any(|f| &f.key == folder) {
                return Err(CommandError::UnknownFolder(folder.clone()));
            }
            let mut l = crate::settings_cycle::load_local(&self.state, &gid);
            let paused = l.paused_folders.contains(folder);
            let mut use_shared = false;
            match &cmd {
                Command::ResumeSettings { choice, .. } => {
                    if !paused || *choice == StayPaused {
                        return Ok(CommandOutcome::NothingToDo);
                    }
                    match choice {
                        ShareMine | ShareMineInstead => return Err(CommandError::UseShareSettings),
                        UseShared | GoBackToShared | UseSharedKeepMineAsRecovery => {
                            use_shared = true
                        }
                        Resume | KeepMineHere | StayPaused => {}
                    }
                    l.paused_folders.retain(|f| f != folder);
                }
                _ => {
                    if paused {
                        return Ok(CommandOutcome::NothingToDo);
                    }
                    l.paused_folders.push(folder.clone());
                    l.paused_folders.sort();
                }
            }
            if use_shared {
                let mut req = crate::settings_cycle::load_use_shared(&self.state, &gid);
                for u in l.units.iter().filter(|u| &u.unit.folder == folder) {
                    if !req.contains(&u.unit) {
                        req.push(u.unit.clone());
                    }
                }
                crate::settings_cycle::save_use_shared(&self.state, &gid, &req)
                    .map_err(|e| CommandError::Service(e.to_string()))?;
            }
            crate::settings_cycle::save_local(&self.state, &gid, &l)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        if let Command::UseSharedSettings { unit } = &cmd {
            let d = derive(&self.plane.ledger, &gid, &self.plane.members);
            let g = wow_sync_core::settings::graph_from_ledger(&self.plane.ledger, &d, unit);
            if g.pending || g.head_states.len() != 1 {
                return Err(CommandError::NoSharedSettings(unit.clone()));
            }
            let mut req = crate::settings_cycle::load_use_shared(&self.state, &gid);
            if !req.contains(unit) {
                req.push(unit.clone());
            }
            crate::settings_cycle::save_use_shared(&self.state, &gid, &req)
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
            crate::rollback_flow::clear_outcome(&self.state, &gid);
            crate::rollback_flow::save_request(&self.state, &gid, &request)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            return Ok(CommandOutcome::Recorded);
        }
        let seq = w.enrollment.next_seq;
        if let Command::RestoreRemoved { removal } = &cmd {
            let derived = derive(&self.plane.ledger, &gid, &self.plane.members);
            let valid = derived.status.get(removal) == Some(&LedgerStatus::Valid);
            if let Some(AuthorityRecord::Removal(rm)) =
                self.plane.ledger.record(removal).filter(|_| valid)
            {
                // Code still installed here only needs the re-add; otherwise the code is put back
                // by a queued restore operation (removal.md §7.4).
                if rm.folders.iter().any(|m| w.inventory.get(&m.key).is_none()) {
                    let planned =
                        crate::restore_flow::plan_requested(&crate::restore_flow::PlanArgs {
                            removal: *removal,
                            group: &gid,
                            own: &w.own,
                            inventory: &w.inventory,
                            plane: &self.plane,
                            provider: &provider,
                            variant_dir: &self.variant_dir(),
                            op: OpId::parse(&random_id_hex()).expect("random id"),
                            seq,
                            now_unix_ms: now,
                        });
                    if let Err(why) = planned {
                        // A refusal stays listed (replacing an older reason) until a restore
                        // succeeds or the owner dismisses it; the error is still returned.
                        return Err(match why {
                            crate::restore_flow::RestoreRefusal::UnknownRemoval => {
                                CommandError::UnknownRemoval(*removal)
                            }
                            other => {
                                crate::restore_flow::clear_request(&self.state, &gid);
                                crate::restore_flow::save_issue(
                                    &self.state,
                                    &gid,
                                    &crate::restore_flow::RestoreIssue {
                                        removal: *removal,
                                        reason: other.reason(),
                                    },
                                );
                                CommandError::Service(other.text())
                            }
                        });
                    }
                    crate::restore_flow::clear_issue(&self.state, &gid);
                    crate::restore_flow::save_request(
                        &self.state,
                        &gid,
                        &crate::restore_flow::RestoreRequest { removal: *removal },
                    )
                    .map_err(|e| CommandError::Service(e.to_string()))?;
                    return Ok(CommandOutcome::Recorded);
                }
            }
        }
        let derived = derive(&self.plane.ledger, &gid, &self.plane.members);
        if let Command::ShareSettingsNow { units } = &cmd {
            let folders: Vec<_> = units.iter().map(|u| u.folder.clone()).collect();
            if !self.is_settings_master(&gid, &w.own, Some(&folders)) {
                return Err(CommandError::NotAllowed);
            }
            if w.standing != OwnStanding::Active {
                return Err(CommandError::Service(
                    "this computer cannot share settings right now".into(),
                ));
            }
            let mut next = seq;
            let wrote = self.push_units(
                &provider,
                &gid,
                &w.own,
                units,
                &mut next,
                now,
                &w.titles,
                &w.inventory,
            )?;
            if !wrote {
                return Ok(CommandOutcome::NothingToDo);
            }
            self.plane
                .save(&self.state)
                .map_err(|e| CommandError::Service(e.to_string()))?;
            self.reserve_seq(&w, next - 1)?;
            return Ok(CommandOutcome::Recorded);
        }
        let held = BTreeSet::new();
        let mut policy_parts: Vec<(wow_sync_core::ids::Sha256, Vec<u8>)> = Vec::new();
        let decided = match &cmd {
            Command::Decide { item, .. } => Some(item.clone()),
            _ => None,
        };
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
                    // Judged against what the group says now (ledger just reloaded from the
                    // provider, peers' records included): an item that no longer exists, or whose
                    // evidence changed, is stale. There is no fallback to the cached offers.
                    let Some((now, plans)) = w
                        .offers
                        .iter()
                        .find(|(i, _)| DecisionId(i.scope.clone()) == item)
                    else {
                        return Err(CommandError::Decide(DecideError::Changed));
                    };
                    let outcome = DecideBehaviour::new(plans.clone(), Standing::Active).decide(
                        &item,
                        &option,
                        &shown,
                        &FreshEvidence {
                            items: vec![now.clone()],
                        },
                    );
                    match outcome {
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
                Command::ReEnroll
                | Command::EnableSettings { .. }
                | Command::ShareSettingsNow { .. }
                | Command::ShareSettingsWhenReady
                | Command::PauseSettings { .. }
                | Command::PauseAllSettings
                | Command::ResumeAllSettings
                | Command::ResumeSettings { .. }
                | Command::UseSharedSettings { .. }
                | Command::Rollback { .. }
                | Command::DismissRestoreIssue { .. } => return Err(CommandError::NotAllowed),
                Command::SetSettingsPolicy { master, exceptions } => {
                    let (rec, parts) = wow_sync_core::settings::build_policy(
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
                    policy_parts = parts;
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
                    self.share_checkpoint(
                        &provider,
                        w.own.computer_id.as_str(),
                        &checkpoint,
                        &unit,
                    )?;
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
                Command::ChooseSettings {
                    unit,
                    checkpoint,
                    shown_heads,
                } => {
                    let rec = wow_sync_core::settings::build_choice(
                        &self.plane.ledger,
                        &derived,
                        &gid,
                        issuer,
                        unit.clone(),
                        checkpoint,
                        &shown_heads,
                        now,
                    )
                    .map_err(CommandError::Settings)?;
                    let _ = self.share_checkpoint(
                        &provider,
                        w.own.computer_id.as_str(),
                        &checkpoint,
                        &unit,
                    );
                    DecisionEffect::Write(Box::new(AuthorityRecord::SettingsRevision(rec)))
                }
                Command::RestoreSettings { unit, checkpoint } => {
                    let rec = self.restore_revision(
                        &provider,
                        w.own.computer_id.as_str(),
                        &derived,
                        &gid,
                        issuer,
                        unit,
                        checkpoint,
                        now,
                    )?;
                    DecisionEffect::Write(Box::new(AuthorityRecord::SettingsRevision(rec)))
                }
                Command::RestoreRemovedSettings { removal, unit } => {
                    let valid = derived.status.get(&removal) == Some(&LedgerStatus::Valid);
                    let Some(AuthorityRecord::Removal(rm)) =
                        self.plane.ledger.record(&removal).filter(|_| valid)
                    else {
                        return Err(CommandError::UnknownRemoval(removal));
                    };
                    let checkpoint = wow_sync_core::settings::removed_unit_checkpoint(rm, &unit)
                        .map_err(CommandError::Settings)?;
                    let rec = self.restore_revision(
                        &provider,
                        w.own.computer_id.as_str(),
                        &derived,
                        &gid,
                        issuer,
                        unit,
                        checkpoint,
                        now,
                    )?;
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
                // Answered before the group state is read.
                Command::CheckUpdates
                | Command::SetTestMode(_)
                | Command::ExportDiagnostics { .. }
                | Command::SavePreviewedDiagnostics { .. } => return Err(CommandError::NotAllowed),
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
        let result = match routed {
            Routed::Refused(why) => Err(CommandError::Decide(why)),
            Routed::Local(l) => {
                if let wow_sync_core::decision::LocalResolution::Dismiss(h) = &l {
                    self.plane.ledger.dismiss(*h);
                    self.plane
                        .save(&self.state)
                        .map_err(|e| CommandError::Service(e.to_string()))?;
                }
                Ok(CommandOutcome::Local(l))
            }
            Routed::WriteNow(rec) => {
                if let AuthorityRecord::SettingsRevision(r) = &*rec {
                    // The file as last seen is the stale one this selection supersedes; an edit
                    // made after it is a new change (RV11-17). Remembered before the write so a
                    // failure to save it refuses the selection instead of losing the guard.
                    let mut local = crate::settings_cycle::load_local(&self.state, &gid);
                    if let Some(u) = local.units.iter_mut().find(|u| u.unit == r.unit) {
                        u.pending_selection = u.captures.first().copied();
                        crate::settings_cycle::save_local(&self.state, &gid, &local).map_err(
                            |_| {
                                CommandError::Service(
                                    "the settings choice could not be remembered".into(),
                                )
                            },
                        )?;
                    }
                }
                // Parts first: a policy whose parts are missing stays pending for every reader.
                for (_, bytes) in &policy_parts {
                    write_own_bytes(&mut self.plane, &provider, &gid, &w.own.computer_id, bytes)
                        .map_err(|e| CommandError::Service(e.to_string()))?;
                }
                let h =
                    write_own_record(&mut self.plane, &provider, &gid, &w.own.computer_id, &rec)
                        .map_err(|e| CommandError::Service(e.to_string()))?;
                self.plane
                    .save(&self.state)
                    .map_err(|e| CommandError::Service(e.to_string()))?;
                self.reserve_seq(&w, seq)?;
                if matches!(&*rec, AuthorityRecord::SettingsRevision(_)) {
                    self.record_settings_write(&provider, &w.titles, &w.inventory, &h);
                }
                if let AuthorityRecord::CodeChoice(c) = &*rec {
                    if let Some(k) =
                        crate::changes::kept((now_unix_ms() as u64, now_unix_ms()), c, &w.inventory)
                    {
                        crate::history::record_change(&self.state, &k);
                    }
                }
                Ok(CommandOutcome::Written(h))
            }
            Routed::Keep(rec, why) => {
                // The kept policy names these parts, so they must outlive the wait; they are
                // inert files without an issuer until the policy itself is written.
                for (_, bytes) in &policy_parts {
                    write_own_bytes(&mut self.plane, &provider, &gid, &w.own.computer_id, bytes)
                        .map_err(|e| CommandError::Service(e.to_string()))?;
                }
                let mut q = KeptQueue::load(&self.state, &gid);
                q.push(KeptRecord::reserve(&rec, seq, why));
                q.save(&self.state, &gid)
                    .map_err(|e| CommandError::Service(e.to_string()))?;
                self.reserve_seq(&w, seq)?;
                Ok(CommandOutcome::Kept)
            }
        };
        // An answered item is no longer offered: a second answer to the same stale view is
        // refused as changed instead of writing a record that does not know the first.
        if let (Some(id), Ok(_)) = (&decided, &result) {
            self.offered
                .0
                .retain(|i| DecisionId(i.scope.clone()) != *id);
        }
        result
    }

    /// Makes `checkpoint` reachable by peers: uploaded from this computer's store, or already in
    /// the shared folder. A checkpoint known nowhere is refused.
    /// Publishes the newest capture of each listed unit as a push (never an automatic revision).
    /// Units without a capture are skipped. `next` is the next issuer sequence number.
    #[allow(clippy::too_many_arguments)]
    fn push_units(
        &mut self,
        provider: &ProviderRoot,
        gid: &wow_sync_core::ids::GroupId,
        own: &EnrollmentRef,
        units: &[wow_sync_core::ids::SettingsUnit],
        next: &mut u64,
        now: u64,
        titles: &BTreeMap<String, String>,
        inventory: &Inventory,
    ) -> Result<bool, CommandError> {
        let derived = derive(&self.plane.ledger, gid, &self.plane.members);
        let local = crate::settings_cycle::load_local(&self.state, gid);
        let mut wrote = false;
        for unit in units {
            let Some(cp) = local
                .units
                .iter()
                .find(|u| &u.unit == unit)
                .and_then(|u| u.captures.first().copied())
            else {
                continue;
            };
            self.share_checkpoint(provider, own.computer_id.as_str(), &cp, unit)?;
            let Ok(rec) = wow_sync_core::settings::build_push(
                &self.plane.ledger,
                &derived,
                gid,
                Issuer {
                    computer_id: own.computer_id.clone(),
                    enrollment_id: own.enrollment_id.clone(),
                    issued_at_seq: *next,
                },
                unit.clone(),
                cp,
                now,
            ) else {
                continue;
            };
            let h = write_own_record(
                &mut self.plane,
                provider,
                gid,
                &own.computer_id,
                &AuthorityRecord::SettingsRevision(rec),
            )
            .map_err(|e| CommandError::Service(e.to_string()))?;
            self.record_settings_write(provider, titles, inventory, &h);
            *next += 1;
            wrote = true;
        }
        Ok(wrote)
    }

    /// Carries out a recorded `ShareSettingsWhenReady` at the start of a pass: the previous
    /// pass captured the units, so each one without a shared copy is pushed once.
    fn share_when_ready(&mut self) {
        let Ok(w) = self.observe_for_command(true) else {
            return;
        };
        let gid = w.group.group_id.clone();
        let mut l = crate::settings_cycle::load_local(&self.state, &gid);
        if !l.share_when_ready || !l.consent || self.settings_facts.units.is_empty() {
            return;
        }
        let units: Vec<_> = self
            .settings_facts
            .units
            .iter()
            .filter(|(_, f)| matches!(f, crate::settings_cycle::UnitFact::NoSharedCopy))
            .map(|(u, _)| u.clone())
            .collect();
        l.share_when_ready = false;
        if crate::settings_cycle::save_local(&self.state, &gid, &l).is_err() {
            return;
        }
        if !units.is_empty() {
            let _ = self.command(Command::ShareSettingsNow { units });
        }
    }
    /// Whether this computer is the effective settings master (for each of `folders`, or for the
    /// group's default master when `None`). Only a master shares in bulk (design §5.2); a single
    /// explicit push (`ShareSettings`) stays allowed to every active computer.
    fn is_settings_master(
        &self,
        gid: &GroupId,
        own: &EnrollmentRef,
        folders: Option<&[wow_sync_core::paths::FolderKey]>,
    ) -> bool {
        use wow_sync_core::settings::{authority_for, policy_heads, Authority};
        let derived = derive(&self.plane.ledger, gid, &self.plane.members);
        let heads = policy_heads(&self.plane.ledger, &derived);
        let me = |m: &EnrollmentRef| {
            m.computer_id == own.computer_id && m.enrollment_id == own.enrollment_id
        };
        match folders {
            Some(fs) => fs
                .iter()
                .all(|f| matches!(authority_for(&heads, f), Authority::Holder(h) if me(&h))),
            None => matches!(heads.as_slice(), [one] if one.master.as_ref().is_some_and(me)),
        }
    }
    /// The Activity row for a settings revision this computer just wrote by command.
    fn record_settings_write(
        &self,
        provider: &ProviderRoot,
        titles: &BTreeMap<String, String>,
        inventory: &Inventory,
        hash: &wow_sync_core::ids::Sha256,
    ) {
        let Some(AuthorityRecord::SettingsRevision(r)) = self.plane.ledger.record(hash) else {
            return;
        };
        let Some(info) = crate::settings_cycle::revision_info(&self.plane.ledger, provider, r)
        else {
            return;
        };
        let at = now_unix_ms();
        let derived = derive(&self.plane.ledger, &r.group_id, &self.plane.members);
        let rec = crate::changes::settings_published(
            (at as u64, at),
            crate::settings_cycle::issued_by_master(&self.plane.ledger, &derived, r),
            r.kind,
            crate::changes::display_of(titles, inventory, &r.unit.folder),
            r.unit.folder.as_str(),
            &info,
        );
        crate::history::record_change(&self.state, &rec);
    }

    fn share_checkpoint(
        &self,
        provider: &ProviderRoot,
        me: &str,
        checkpoint: &wow_sync_core::ids::Sha256,
        unit: &wow_sync_core::ids::SettingsUnit,
    ) -> Result<(), CommandError> {
        let store =
            wow_sync_adapters::settings_store::SettingsStore::open(&self.variant_dir()).ok();
        let local = store
            .as_ref()
            .and_then(|s| s.get_manifest(&checkpoint.0).ok().flatten())
            .is_some();
        if local {
            let store = store.as_ref().expect("store");
            return crate::settings_cycle::upload_checkpoint(provider, store, checkpoint, me)
                .map_err(CommandError::Service);
        }
        if crate::settings_cycle::manifest_blob(provider, checkpoint, unit).is_some() {
            return Ok(());
        }
        Err(CommandError::UnknownCheckpoint(*checkpoint))
    }

    /// The `restore` revision of `unit` to `checkpoint`, parented on this computer's basis.
    #[allow(clippy::too_many_arguments)]
    fn restore_revision(
        &self,
        provider: &ProviderRoot,
        me: &str,
        derived: &wow_sync_core::ledger::Derived,
        gid: &GroupId,
        issuer: Issuer,
        unit: wow_sync_core::ids::SettingsUnit,
        checkpoint: wow_sync_core::ids::Sha256,
        now: u64,
    ) -> Result<wow_sync_core::kinds::SettingsRevision, CommandError> {
        let basis = crate::settings_cycle::load_local(&self.state, gid)
            .units
            .into_iter()
            .find(|u| u.unit == unit)
            .map(|u| u.basis)
            .unwrap_or_default();
        let rec = wow_sync_core::settings::build_restore(
            &self.plane.ledger,
            derived,
            gid,
            issuer,
            unit.clone(),
            checkpoint,
            &basis,
            now,
        )
        .map_err(CommandError::Settings)?;
        self.share_checkpoint(provider, me, &checkpoint, &unit)?;
        Ok(rec)
    }

    /// Explicit re-enrollment of a retired (or held) computer: a new enrollment id for the same
    /// computer, recorded in `member.json` with the old ids kept (ops.md §2.3 item 6). The old
    /// enrollment stays retired and its records stay valid.
    fn re_enroll(&mut self) -> Result<CommandOutcome, CommandError> {
        let svc = |e: &dyn std::fmt::Display| CommandError::Service(e.to_string());
        let w = self.observe_for_command(true).map_err(|e| svc(&e))?;
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

    /// Uses `ctl` for every read-only scan of this service: setting its cancel flag abandons the
    /// scan at the next file.
    pub fn set_read_ctl(&mut self, ctl: ScanCtl) {
        self.read_ctl = ctl;
    }

    /// Whether this computer has an enrollment record (or one that cannot be read), without
    /// looking at the add-on folders.
    pub fn is_enrolled(&self) -> bool {
        !matches!(self.read_enrollment(), Ok(None))
    }

    /// True only when the saved enrollment exists and loads.
    pub fn enrollment_loads(&self) -> bool {
        matches!(self.read_enrollment(), Ok(Some(_)))
    }

    /// `None` = cancelled through `ctl`.
    #[allow(clippy::type_complexity)]
    fn scan_ctl(&self, ctl: &ScanCtl) -> Result<Option<(Vec<FolderScan>, bool)>, ServiceError> {
        if self.reuse_scan.load(std::sync::atomic::Ordering::Relaxed) {
            let last = self
                .last_scan
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if last.is_some() {
                return Ok(last);
            }
        }
        let scanned = self.scan_install(ctl)?;
        if let Some(s) = &scanned {
            *self.last_scan.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
        }
        Ok(scanned)
    }

    fn scan_install(&self, ctl: &ScanCtl) -> Result<Option<(Vec<FolderScan>, bool)>, ServiceError> {
        let root = Root::open(&self.addons_path(), ReparsePolicy::RejectAll).map_err(local)?;
        let mut opts = ScanOptions::with_defaults(now_unix_ms());
        opts.recent_ms = self.cfg.recent_ms;
        let only = ctl.only.is_some();
        let last = if only {
            self.last_scan
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            None
        };
        if only && last.is_none() {
            // Nothing to carry over: read everything.
            let full = ScanCtl {
                only: None,
                ..ctl.clone()
            };
            return self.scan_install(&full);
        }
        let Some(scan) = scan_addons_ctl(&root, &opts, &mut |_| {}, ctl).map_err(local)? else {
            return Ok(None);
        };
        let listing = scan.listing.clone();
        let mut folders: Vec<FolderScan> = scan
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
                        rel_path: x.rel_path.into(),
                        size: x.size,
                        sha256: Sha256(x.sha256),
                    })
                    .collect::<Vec<_>>()
                    .into(),
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
        if let Some((carried, _)) = last {
            // Folders outside the scope keep what the last complete scan saw, in listing order.
            let mut merged = Vec::with_capacity(listing.len());
            for name in &listing {
                let fresh = folders
                    .iter()
                    .position(|f| f.name.eq_ignore_ascii_case(name));
                match fresh {
                    Some(i) => merged.push(folders.swap_remove(i)),
                    None => match carried.iter().find(|f| f.name == *name) {
                        Some(f) => merged.push(f.clone()),
                        // A folder that appeared since: read everything.
                        None => {
                            let full = ScanCtl {
                                only: None,
                                ..ctl.clone()
                            };
                            return self.scan_install(&full);
                        }
                    },
                }
            }
            folders = merged;
        }
        Ok(Some((folders, scan.listing_stable)))
    }

    fn marker_policy(&self) -> MarkerPolicy {
        MarkerPolicy::from_test_mode(crate::control::test_mode(&self.state))
    }

    /// Sibling variant folders that validate to the same product (the core decides what counts).
    fn others(&self, product: &str, enrolled_elsewhere: bool) -> OtherVariants {
        let policy = self.marker_policy();
        let mut same = Vec::new();
        for f in list_variant_folders(&self.cfg.install_root) {
            if f == self.cfg.variant_folder {
                continue;
            }
            let v = validate_variant(&self.observation(&f), policy);
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
        self.judge_ctl(enrolled_elsewhere, &self.read_ctl)?
            .ok_or(ServiceError::Cancelled)
    }

    #[allow(clippy::type_complexity)]
    fn judge_ctl(
        &self,
        enrolled_elsewhere: bool,
        ctl: &ScanCtl,
    ) -> Result<Option<(VariantVerdict, LayoutAssessment, Vec<FolderScan>, bool)>, ServiceError>
    {
        let verdict = validate_variant(
            &self.observation(&self.cfg.variant_folder),
            self.marker_policy(),
        );
        let (scans, stable) = match &verdict {
            VariantVerdict::Refused(_) => (Vec::new(), true),
            _ => match self.scan_ctl(ctl)? {
                Some(x) => x,
                None => return Ok(None),
            },
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
        Ok(Some((verdict, assessment, scans, stable)))
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
        self.inspect_ctl(&ScanCtl::default())
            .expect("an inspection without a cancel request completes")
    }

    /// Like `inspect`; `None` = cancelled through `ctl`, with nothing reported.
    pub fn inspect_ctl(&self, ctl: &ScanCtl) -> Option<EnrollPreview> {
        let enrolled = self.read_enrollment().ok().flatten().is_some();
        let mut p = self.preview(ctl)?;
        if enrolled {
            p.problem = Some(ServiceError::AlreadyEnrolled.to_string());
        }
        Some(p)
    }

    fn preview(&self, ctl: &ScanCtl) -> Option<EnrollPreview> {
        let (verdict, assessment, _scans, _stable) = match self.judge_ctl(false, ctl) {
            Ok(Some(x)) => x,
            Ok(None) => return None,
            Err(e) => {
                return Some(EnrollPreview {
                    kind: PreviewKind::Refused,
                    identity_line: String::new(),
                    scope_line: String::new(),
                    eligible: vec![],
                    blocked: vec![],
                    digest: None,
                    needs_guided_confirmation: false,
                    groups: vec![],
                    problem: Some(e.to_string()),
                    retryable: true,
                    test_mode_blocked: false,
                    disclosures: vec![],
                })
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
            retryable: false,
            test_mode_blocked: false,
            disclosures: vec![],
        };
        let (id, kind) = match &verdict {
            VariantVerdict::Recognized(i) => (i, PreviewKind::Recognized),
            VariantVerdict::Unfamiliar(i) => (i, PreviewKind::Unfamiliar),
            VariantVerdict::Refused(r) => {
                let (text, retry) = refuse_text(r);
                p.problem = Some(text);
                p.retryable = retry;
                p.test_mode_blocked = matches!(r, RefuseReason::NotSyntheticInstall);
                return Some(p);
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
                p.blocked = blocked
                    .iter()
                    .map(|(k, r)| format!("{k}: {}", block_text(r)))
                    .collect();
                p.digest = Some(digest);
                p.disclosures = consent_disclosures(&ConsentContext {
                    identity: id,
                    mapping_id: mapping.id(),
                    install_root: &self.cfg.install_root.to_string_lossy(),
                    addons_path: &self.addons_path().to_string_lossy(),
                });
            }
            LayoutAssessment::Unsupported(r) => {
                let (text, retry) = layout_text(&r);
                p.problem = Some(text);
                p.retryable = retry;
                p.test_mode_blocked = matches!(r, LayoutReason::NotSyntheticInstall);
            }
        }
        Some(p)
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
            .map_err(|r| ServiceError::Refused(refuse_text(&r).0))?;
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
            Some(g) => Err(ServiceError::Refused(refuse_text(&RefuseReason::from(g)).0)),
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
            .map_err(|r| ServiceError::Refused(refuse_text(&r).0))?;
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

    /// Re-judges the install once and hands back the scan it was based on, so callers need not
    /// hash the add-ons a second time.
    #[allow(clippy::type_complexity)]
    fn rederive_variant(
        &self,
        e: &LocalEnrollment,
        ctl: &ScanCtl,
    ) -> Result<(ConfirmedVariant, Vec<FolderScan>, bool), ServiceError> {
        let (verdict, assessment, scans, stable) =
            self.judge_ctl(false, ctl)?.ok_or(ServiceError::Cancelled)?;
        let variant =
            confirm_enrolled(&verdict, &assessment, &e.validation).map_err(|r| match r {
                RefuseReason::StaleConfirmation => ServiceError::NeedsReconfirmation,
                RefuseReason::VariantChanged => ServiceError::VariantChanged,
                RefuseReason::NotSyntheticInstall => ServiceError::TestModeBlocked,
                r => ServiceError::Refused(refuse_text(&r).0),
            })?;
        Ok((variant, scans, stable))
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

    /// Names for a snapshot that cannot read the install: from local records and, when it can be
    /// read, the group record. Anything unreadable stays blank.
    fn blocked_names(&self) -> (String, String, String) {
        let Ok(Some(e)) = self.read_enrollment() else {
            return Default::default();
        };
        let computer = self
            .read_computer()
            .ok()
            .flatten()
            .map(|c| c.display_name)
            .unwrap_or_default();
        let group = self
            .open_provider()
            .ok()
            .and_then(|p| p.read(&layout::group_record(&e.group_id), SMALL).ok())
            .and_then(|b| decode::<GroupRecord>(&b, SMALL as usize).ok())
            .map(|g| g.display_name)
            .unwrap_or_default();
        (group, computer, e.variant_folder)
    }

    /// Like `observe_world`, but when the newest complete scan is younger than `RECENT_SCAN_MS`
    /// and has no unstable folder, the install is not scanned again: nothing this pass did
    /// changed it. `scanned_at` is moved forward when a scan was actually made.
    fn observe_world_recent(
        &mut self,
        persist: bool,
        scanned_at: &mut std::time::Instant,
    ) -> Result<World, ServiceError> {
        let settled = self
            .last_scan
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|(folders, stable)| {
                *stable
                    && !folders
                        .iter()
                        .any(|f| matches!(f.status, ScanStatus::Unstable(_)))
            });
        let reuse = settled && scanned_at.elapsed().as_millis() < RECENT_SCAN_MS;
        self.reuse_scan
            .store(reuse, std::sync::atomic::Ordering::Relaxed);
        let w = self.observe_world(persist);
        self.reuse_scan
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if !reuse {
            *scanned_at = std::time::Instant::now();
        }
        w
    }

    fn observe_world(&mut self, persist: bool) -> Result<World, ServiceError> {
        let ctl = self.read_ctl.clone();
        self.observe_world_ctl(persist, &ctl)
    }

    /// The observation that answers a user command or a status request: it never does a full
    /// read (the hash cache serves unchanged files, size and mtime per D-31) and leaves a
    /// pending `check all files` request for the next pass. `reuse_local` answers from the
    /// newest complete scan without touching the install, for commands that need no fresh
    /// local evidence.
    fn observe_for_command(&mut self, reuse_local: bool) -> Result<World, ServiceError> {
        self.observe_for_command_in(reuse_local, None)
    }

    /// Like observe_for_command; with only, just those folders are looked at again and the
    /// rest come from the newest complete scan.
    fn observe_for_command_in(
        &mut self,
        reuse_local: bool,
        only: Option<Vec<String>>,
    ) -> Result<World, ServiceError> {
        let ctl = ScanCtl {
            routine: true,
            only: only.map(std::sync::Arc::new),
            ..self.read_ctl.clone()
        };
        self.reuse_scan
            .store(reuse_local, std::sync::atomic::Ordering::Relaxed);
        let w = self.observe_world_ctl(false, &ctl);
        self.reuse_scan
            .store(false, std::sync::atomic::Ordering::Relaxed);
        w
    }

    fn observe_world_ctl(&mut self, persist: bool, ctl: &ScanCtl) -> Result<World, ServiceError> {
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
        let (variant, scans, stable) = self.rederive_variant(&enrollment, ctl)?;
        let titles = folder_titles(&scans);
        let decls = crate::settings_cycle::declaring_folders(&scans, variant.mapping());
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
        let names: BTreeMap<ComputerId, String> = obs
            .iter()
            .filter_map(|m| match &m.member {
                Ok(rec) if rec.computer_id == m.computer => {
                    Some((m.computer.clone(), rec.display_name.clone()))
                }
                _ => None,
            })
            .collect();
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
            if let Some((at, w)) = self.tree_cache.get(&h) {
                if at.elapsed().as_millis() < RECENT_SCAN_MS {
                    trees.insert(h, w.clone());
                    continue;
                }
            }
            let avail = match provider.read(&layout::tree_record(&group.group_id, &h), TREE_LIMIT) {
                Ok(b) => match decode_tree(&b, &h) {
                    Ok(t) => TreeAvail::Ok(t.into()),
                    Err(_) => TreeAvail::Corrupt,
                },
                Err(e) if e.kind == FsErrorKind::CloudOnly => TreeAvail::CloudOnly,
                Err(e) if is_missing(&e) => TreeAvail::Missing,
                Err(_) => TreeAvail::Corrupt,
            };
            if matches!(avail, TreeAvail::Ok(_)) {
                self.tree_cache
                    .insert(h, (std::time::Instant::now(), avail.clone()));
            }
            trees.insert(h, avail);
        }
        self.tree_cache
            .retain(|_, (at, _)| at.elapsed().as_millis() < RECENT_SCAN_MS);
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
        let input = TargetInput {
            group: &group,
            variant: &variant,
            local: &inventory,
            local_client: &Self::client_info(&variant),
            local_id: &computer.computer_id,
            members: &views,
            trees: &trees,
            local_prev: own_latest
                .as_ref()
                .filter(|s| s.group_id == group.group_id && s.computer_id == computer.computer_id),
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
        let (offers, withheld, local_only) = {
            let valid = wow_sync_core::choice::valid_hashes(&derived);
            let view = wow_sync_core::choice::ChoiceView::new(&self.plane.ledger, &valid);
            let rejects = wow_sync_core::reject::RejectBehaviour::new(&derived);
            let (withheld, local_only) = withheld_local_versions(
                &view,
                &rejects,
                &self.plane.ledger,
                &copies,
                &computer.computer_id,
                &titles,
            );
            let cx = wow_sync_core::choice::WriteCx {
                group: &group.group_id,
                issuer: Issuer {
                    computer_id: computer.computer_id.clone(),
                    enrollment_id: enrollment.enrollment_id.clone(),
                    issued_at_seq: enrollment.next_seq,
                },
                now_unix_ms: now_unix_ms() as u64,
            };
            let mut offers = decision_items(&view, &cx, &target, &copies);
            offers.extend(wow_sync_core::decide::rollback_items(
                &view,
                &cx,
                &rejects,
                &self.plane.ledger,
                self.plane.ledger.dismissed(),
                &copies,
            ));
            (offers, withheld, local_only)
        };
        Ok(World {
            enrollment,
            computer,
            group,
            variant,
            inventory,
            views,
            target,
            offers,
            withheld,
            local_only,
            copies,
            own_states,
            own_latest,
            titles,
            decls,
            own,
            standing,
            names,
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
        let mut s = self.snapshot_inner(w, area);
        s.test_mode = self.test_mode();
        s
    }

    fn snapshot_inner(&mut self, w: &World, area: &InstallArea) -> StatusSnapshot {
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
        let recovery_points =
            crate::detail::recovery_rows(&self.cfg.install_root.join(&self.cfg.variant_folder));
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
        let derived = derive(&self.plane.ledger, &w.group.group_id, &self.plane.members);
        let mut names = w.names.clone();
        names.insert(
            w.computer.computer_id.clone(),
            w.computer.display_name.clone(),
        );
        let enrollment_names: BTreeMap<EnrollmentId, String> = self
            .plane
            .members
            .pairs()
            .filter_map(|p| {
                names
                    .get(&p.computer_id)
                    .map(|n| (p.enrollment_id.clone(), n.clone()))
            })
            .collect();
        let mut settings = crate::settings_cycle::settings_snapshot(
            &crate::settings_cycle::load_local(&self.state, &w.group.group_id),
            &self.plane.ledger,
            &derived,
            &enrollment_names,
        );
        settings.available_accounts = if w.standing == OwnStanding::Active {
            crate::settings_cycle::available_accounts(&self.variant_dir())
        } else {
            Vec::new()
        };
        let retirements = crate::detail::retirement_rows(
            &self.plane.ledger,
            &derived,
            &names,
            &w.computer.computer_id,
        );
        let peers = crate::detail::peer_rows(
            &self.plane.members,
            &self.plane.ledger,
            &derived,
            &names,
            &w.computer.computer_id,
        );
        let this_retirement = retirements
            .iter()
            .find(|r| r.enrollment == w.own.enrollment_id)
            .map(|r| wow_sync_core::status::RetiredInfo {
                enrollment: r.enrollment.clone(),
                retired_by: r.retired_by_id.clone(),
                cutoff_seq: r.cutoff_seq,
            });
        let settings_units = crate::settings_cycle::settings_units(
            &crate::settings_cycle::load_local(&self.state, &w.group.group_id),
            &self.plane.ledger,
            &derived,
            this_retirement.is_some(),
            self.settings_cap.is_some(),
            &self.settings_facts,
            &names,
            &w.own,
        );
        settings.to_share = settings_units
            .iter()
            .filter(|r| {
                r.can_share
                    && matches!(
                        r.status,
                        crate::settings_cycle::SettingsUnitStatus::NoSharedCopy
                            | crate::settings_cycle::SettingsUnitStatus::LocalDifferent(_)
                    )
            })
            .filter_map(|r| {
                Some(crate::settings_cycle::ShareOffer {
                    unit: r.unit.clone(),
                    checkpoint: r.latest_capture?,
                    not_shared_yet: r.status
                        == crate::settings_cycle::SettingsUnitStatus::NoSharedCopy,
                })
            })
            .collect();
        let unresolved: Vec<FolderKey> = interrupted
            .iter()
            .cloned()
            .chain(needs_help.iter().map(|(k, _)| k.clone()))
            .collect();
        let hold = wow_sync_core::status::hold_scope(
            &blocks,
            &unresolved,
            &wow_sync_core::status::HoldInputs {
                paused: crate::control::pause_state(&self.state).holds_sync(),
                retired: w.standing == OwnStanding::Retired,
                retirement_unresolved: w.standing == OwnStanding::Held,
                waiting_addons: self.offered.0.iter().map(|i| i.scope.clone()).collect(),
            },
        );
        let held_releases = crate::detail::held_releases(
            &w.copies,
            &w.inventory,
            &w.titles,
            &w.enrollment.enrollment_id,
            &enrollment_names,
        );
        let keyed = crate::detail::removed_rows_keyed(
            &self.plane.ledger,
            &derived,
            &names,
            &w.computer.computer_id,
        );
        let removal_hashes: Vec<Sha256> = keyed.iter().map(|(_, h)| *h).collect();
        let removed: Vec<crate::detail::RemovedRow> = keyed.into_iter().map(|(r, _)| r).collect();
        let rejected_releases = crate::detail::rejected_releases(&self.plane.ledger, &derived);
        let older_peer = crate::detail::older_peer(&w.views, &peers);
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
        let notes: BTreeMap<String, String> = rows
            .iter()
            .filter_map(|r| Some((r.key.clone(), unrecognised_note(&r.folders)?)))
            .collect();
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
        let settings_decisions_pending = settings_units
            .iter()
            .filter(|r| r.status == crate::settings_cycle::SettingsUnitStatus::NeedsChoice)
            .count() as u32;
        StatusSnapshot {
            test_mode: false,
            test_mode_blocked: false,
            enrolled: true,
            // A pending settings choice is never "up to date".
            headline: if settings_decisions_pending > 0 && sv.headline == Headline::Ok {
                "Saved settings need your choice".to_string()
            } else {
                render_headline(&sv)
            },
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
                    text: {
                        let line = format!(
                            "{}: {}",
                            p.0.as_str(),
                            render_with_names(&p.2, &w.views, &w.computer.computer_id)
                        );
                        match notes.get(p.0.as_str()) {
                            Some(n) => format!("{line} {n}"),
                            None => line,
                        }
                    },
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
            peers,
            settings,
            settings_decisions_pending,

            settings_units,
            hold,
            this_retirement,
            retirements,
            held_releases,
            removed,
            removal_hashes,
            rejected_releases,
            older_peer,
            enrollment_names,
            review_needed: false,
            local_only_versions: w.local_only.clone(),
            data_at_risk: self.data_at_risk(),
            update: self.updates.as_ref().and_then(|u| u.last_result()),
            journal_states: {
                use wow_sync_core::diagnostics::JournalState as J;
                std::iter::repeat_n(J::Pending, interrupted.len())
                    .chain(std::iter::repeat_n(J::NeedsHelp, needs_help.len()))
                    .chain(std::iter::repeat_n(J::Other, unknown_journals.len()))
                    .collect()
            },
            rollback_outcome: crate::rollback_flow::load_outcome(&self.state, &w.group.group_id),
            restores: crate::restore_flow::progress(
                &self.state,
                &w.group.group_id,
                &self.plane.ledger,
                &derived,
                matches!(observed, wow_sync_core::plan::Activity::Running { .. }),
                crate::control::pause_state(&self.state).holds_sync(),
            ),
            held_release_dates: recovery_points
                .iter()
                .map(|r| (r.release, r.set_aside_unix_ms))
                .fold(std::collections::BTreeMap::new(), |mut m, (h, d)| {
                    let e = m.entry(h).or_insert(d);
                    *e = (*e).max(d);
                    m
                }),
            recovery_points,
        }
    }

    /// Read-only view of the current state.
    pub fn status(&mut self) -> StatusSnapshot {
        self.status_with(false)
    }

    /// `reuse_local`: answer from the newest complete scan instead of reading the install.
    fn status_with(&mut self, reuse_local: bool) -> StatusSnapshot {
        let mut s = self.status_inner(reuse_local);
        s.test_mode = self.test_mode();
        s
    }

    /// Whether test mode is on for this computer (`Command::SetTestMode`).
    pub fn test_mode(&self) -> bool {
        crate::control::test_mode(&self.state)
    }

    fn status_inner(&mut self, reuse_local: bool) -> StatusSnapshot {
        let w = match self.observe_for_command(reuse_local) {
            Ok(w) => {
                self.provider_error = None;
                w
            }
            Err(ServiceError::NotEnrolled) => return StatusSnapshot::default(),
            Err(ServiceError::Cancelled) => {
                return StatusSnapshot {
                    enrolled: true,
                    ..Default::default()
                }
            }
            Err(ServiceError::VariantChanged) => {
                self.provider_error = None;
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::NeedsHelp),
                    provider_error: Some(ServiceError::VariantChanged.to_string()),
                    ..Default::default()
                };
            }
            Err(ServiceError::TestModeBlocked) => {
                self.provider_error = None;
                let (group_name, computer_name, identity_line) = self.blocked_names();
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::NeedsHelp),
                    group_name,
                    computer_name,
                    identity_line,
                    provider_error: Some(ServiceError::TestModeBlocked.to_string()),
                    test_mode_blocked: true,
                    ..Default::default()
                };
            }
            Err(ServiceError::NeedsReconfirmation) => {
                self.provider_error = None;
                let ctl = self.read_ctl.clone();
                let Some(mut preview) = self.preview(&ctl) else {
                    return StatusSnapshot {
                        enrolled: true,
                        ..Default::default()
                    };
                };
                preview.groups.clear();
                return StatusSnapshot {
                    enrolled: true,
                    headline: headline_sentence(Headline::NeedsHelp),
                    provider_error: Some(ServiceError::NeedsReconfirmation.to_string()),
                    reconfirm: Some(preview),
                    // The NeedsHelp sentence stands in for the unreadable state here; no real
                    // NeedsHelp or Error is known, so the reconfirmation shows (ops.md §1.7).
                    review_needed: crate::detail::review_needed(true, Headline::Waiting),
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
        let report = self.sync_pass();
        // Decoded trees are as large as the add-on file lists; nothing reuses them after a pass.
        self.tree_cache.clear();
        report
    }

    fn sync_pass(&mut self) -> Result<SyncReport, ServiceError> {
        self.share_when_ready();
        let mut report = SyncReport::default();
        self.at_risk_apply.clear();
        let pass_id = now_unix_ms() as u64;
        let _pass = crate::tracking::PassGuard::start(&self.progress, pass_id);
        // The counter is per pass; the hint is what the first look of the last pass counted.
        self.read_ctl
            .files
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let scan_total = self.scan_total_hint;
        let observed = {
            let _tick = crate::tracking::ScanTicker::start(
                &self.progress,
                pass_id,
                self.read_ctl.files.clone(),
                scan_total,
            );
            self.observe_world(true)
        };
        let mut w = match observed {
            Ok(w) => {
                self.provider_error = None;
                self.scan_total_hint = self
                    .read_ctl
                    .files
                    .load(std::sync::atomic::Ordering::Relaxed);
                w
            }
            Err(ServiceError::NotEnrolled) => return Err(ServiceError::NotEnrolled),
            Err(ServiceError::Cancelled) => return Err(ServiceError::Cancelled),
            Err(e) => {
                self.provider_error = Some(e.to_string());
                return Err(e);
            }
        };
        let provider = self.open_provider()?;
        let addons_dir = self.addons_path();
        let addons = Root::open(&addons_dir, ReparsePolicy::RejectAll).map_err(local)?;
        let variant_dir = self.variant_dir();
        let area = InstallArea::open(&variant_dir)
            .map_err(local)?
            .with_hash_cache(self.read_ctl.cache.clone());
        let mut scanned_at = std::time::Instant::now();
        let mut quiet_cycle = false;
        let group_id = w.group.group_id.clone();
        let me = w.computer.computer_id.as_str().to_string();
        let enrollment_id = w.enrollment.enrollment_id.clone();
        let mut new_report = None;
        let mut journals_remaining = false;
        self.last_blocks.clear();
        let answered = self.queued_scopes(&w.group.group_id);
        self.settle_kept(&w, &provider);
        Self::hide_scopes(&mut w, &answered);
        self.hide_answered(&mut w);
        crate::tracking::phase(&self.progress, pass_id, crate::history::Phase::Deciding);
        let mut decided: BTreeSet<FolderKey> = answered.iter().flatten().cloned().collect();
        if w.standing == OwnStanding::Active {
            let (items, plans): (Vec<_>, Vec<_>) =
                std::mem::take(&mut w.offers).into_iter().unzip();
            for it in &items {
                if let DecisionScope::Folders(keys) = &it.scope {
                    decided.extend(keys.iter().cloned());
                }
            }
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
                let svd = self.cfg.install_root.join(&self.cfg.variant_folder);
                let senv = crate::removal_cycle::SettingsEnv {
                    cap: self.settings_cap,
                    variant_dir: &svd,
                    decls: &w.decls,
                };
                let (extra, mut next_seq, just_removed) = Self::removal_steps(
                    &self.state,
                    &mut self.plane,
                    &provider,
                    &w,
                    &journaled,
                    game_blocks,
                    &senv,
                    &mut report,
                );
                let mut settings_info = BTreeMap::new();
                let settings_ops = if w.standing == OwnStanding::Active && !game_blocks {
                    let hub = self.progress.clone();
                    let tick = |done: u64,
                                total: u64,
                                current: Option<&str>,
                                from: Option<&ComputerId>| {
                        let source = from
                            .and_then(|c| w.names.get(c))
                            .map(|n| crate::changes::clip(n));
                        crate::tracking::settings_step(&hub, pass_id, done, total, current, source);
                    };
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
                            tick: &tick,
                            settle: &|| {
                                std::thread::sleep(std::time::Duration::from_millis(
                                    self.cfg.recent_ms,
                                ))
                            },
                        },
                        &mut next_seq,
                    );
                    self.settings_facts = out.facts;
                    if !out.notes.is_empty() && report.apply_note.is_none() {
                        report.apply_note = Some(format!(
                            "some saved settings were not shared: {}",
                            out.notes.join("; ")
                        ));
                    }
                    for (h, info) in &out.published {
                        if let Some(AuthorityRecord::SettingsRevision(r)) =
                            self.plane.ledger.record(h)
                        {
                            let rec = crate::changes::settings_published(
                                (pass_id, now_unix_ms()),
                                crate::settings_cycle::issued_by_master(
                                    &self.plane.ledger,
                                    &derive(&self.plane.ledger, &group_id, &self.plane.members),
                                    r,
                                ),
                                r.kind,
                                crate::changes::display_of(&w.titles, &w.inventory, &r.unit.folder),
                                r.unit.folder.as_str(),
                                info,
                            );
                            crate::history::record_change(&self.state, &rec);
                            crate::tracking::recorded(
                                &self.progress,
                                pass_id,
                                crate::history::Phase::Settings,
                                rec,
                            );
                        }
                    }
                    settings_info = out.apply_info;
                    out.applies
                } else {
                    if w.standing == OwnStanding::Active {
                        self.settings_facts.wow_running = true;
                    }
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
                        Ok(p) => {
                            crate::rollback_flow::clear_outcome(&self.state, &group_id);
                            Some(p)
                        }
                        Err(why) => {
                            report.apply_note = Some(format!("rollback not applied: {why:?}"));
                            None
                        }
                    }
                } else {
                    None
                };
                crate::restore_flow::prune(
                    &self.state,
                    &group_id,
                    &self.plane.ledger,
                    &derive(&self.plane.ledger, &group_id, &self.plane.members),
                );
                let mut restore_pending: Option<(FolderKey, Sha256, String)> = None;
                let restore = if w.standing == OwnStanding::Active
                    && !game_blocks
                    && rollback.is_none()
                    && crate::restore_flow::load_request(&self.state, &group_id).is_some()
                {
                    let request = crate::restore_flow::load_request(&self.state, &group_id)
                        .expect("checked above");
                    let args = crate::restore_flow::PlanArgs {
                        removal: request.removal,
                        group: &group_id,
                        own: &w.own,
                        inventory: &w.inventory,
                        plane: &self.plane,
                        provider: &provider,
                        variant_dir: &variant_dir,
                        op: gen(),
                        seq: next_seq,
                        now_unix_ms: now_unix_ms() as u64,
                    };
                    match crate::restore_flow::step(&self.state, &args, &w.enrollment) {
                        Ok(p) => {
                            let keys: BTreeSet<FolderKey> =
                                p.folders.iter().map(|f| f.key.clone()).collect();
                            let captured = crate::removal_cycle::SettingsPass::open(
                                &self.state,
                                &group_id,
                                &senv,
                                game_blocks,
                            )
                            .captured_units(&keys);
                            // The note is shown only once the restore has verified and applied.
                            restore_pending = Some((
                                p.package.clone(),
                                request.removal,
                                crate::restore_flow::settings_note(
                                    crate::restore_flow::expected_units(
                                        &self.plane.ledger,
                                        &request.removal,
                                    ),
                                    captured,
                                ),
                            ));
                            Some(p)
                        }
                        Err(why) => {
                            report.apply_note = Some(why.text());
                            None
                        }
                    }
                } else {
                    None
                };
                let rollback = rollback.or(restore);
                let apply_target = target_without(&w.target, &w.withheld);
                let mut planned = match plan_apply(&apply_target, &w.inventory, &mut gen, &gate) {
                    Ok(p) => Some(p),
                    Err(PlanBlock::NothingToDo) => None,
                    Err(b) => {
                        report.apply_note = Some(render_block(&b));
                        self.last_blocks.push(b);
                        None
                    }
                };
                // The target was composed before this cycle's removal record existed: it must not
                // put back the folder whose removal was just recorded, or the uninstall is
                // mistaken for a deliberate reinstall and never completes.
                if let Some(p) = planned.as_mut() {
                    p.packages.retain(|q| {
                        !just_removed.contains(&q.package)
                            && !q.folders.iter().any(|f| just_removed.contains(&f.key))
                    });
                    if p.packages.is_empty() && p.skipped.is_empty() {
                        planned = None;
                    }
                }
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
                        let prepared = {
                            let cx = crate::changes::BuildCx {
                                inventory: &w.inventory,
                                target: &apply_target,
                                names: &w.names,
                                own: &w.computer.computer_id,
                                titles: &w.titles,
                                ledger: &self.plane.ledger,
                                pass_id,
                                now_ms: now_unix_ms(),
                                settings: &settings_info,
                            };
                            plan.packages
                                .iter()
                                .enumerate()
                                .filter_map(|(i, p)| {
                                    crate::changes::prepare(p, &cx).map(|r| (i, r))
                                })
                                .collect::<Vec<_>>()
                        };
                        let progress = crate::tracking::ApplyProgress::new(
                            &self.progress,
                            pass_id,
                            &self.state,
                            &plan,
                            prepared,
                        )
                        .with_addons(addons_dir.clone());
                        fx.inner_mut().set_reporting(progress.clone());
                        progress.announce(&decided);
                        for (k, why) in &plan.skipped {
                            report.applied.push(AppliedLine {
                                package: k.to_string(),
                                text: format!("not applied: {}", explain::render(why)),
                            });
                        }
                        let outcome = run_apply(&plan, &mut fx);
                        progress.complete_keys(
                            outcome
                                .packages
                                .iter()
                                .filter(|(_, r)| matches!(r, PackageResult::Applied { .. }))
                                .map(|(k, _)| k),
                        );
                        self.at_risk_apply = crate::notices::risks_from_outcome(&outcome);
                        for (k, r) in &outcome.packages {
                            report.applied.push(AppliedLine {
                                package: k.to_string(),
                                text: render_result(r),
                            });
                        }
                        remember_applied(&mut self.applied_here, &plan, &outcome);
                        if let Some((pkg, removal, note)) = restore_pending.take() {
                            crate::restore_flow::settle(
                                &self.state,
                                &group_id,
                                &removal,
                                outcome
                                    .packages
                                    .iter()
                                    .find(|(k, _)| *k == pkg)
                                    .map(|(_, r)| r),
                                note,
                                &mut report.apply_note,
                            );
                        }
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
                let fresh = if applied_plan.is_some() || !rec.ops.is_empty() {
                    // Game files changed: the cycle goes on to publish what it did, so this
                    // look is not abandoned.
                    // Not cancellable, but still uses the hash cache: files this apply installed
                    // were seeded into it after the rename (when size and mtime match a fresh
                    // snapshot); any other recently written file is re-read.
                    let keep = ScanCtl {
                        cache: self.read_ctl.cache.clone(),
                        ..ScanCtl::default()
                    };
                    let w = self.observe_world_ctl(true, &keep)?;
                    scanned_at = std::time::Instant::now();
                    w
                } else {
                    quiet_cycle = true;
                    self.observe_world_recent(true, &mut scanned_at)?
                };
                w = fresh;
                let saved = crate::history::load_apply_sources(&self.state);
                for (_, key, res) in &rec.ops {
                    if matches!(res, PackageResult::Applied { .. }) {
                        let recovered = crate::changes::recovered(
                            (pass_id, now_unix_ms()),
                            key,
                            &w.titles,
                            &w.inventory,
                            saved.get(key.as_str()).cloned(),
                        );
                        crate::history::record_change(&self.state, &recovered);
                        crate::tracking::recorded(
                            &self.progress,
                            pass_id,
                            crate::history::Phase::Installing,
                            recovered,
                        );
                    }
                }
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
                )
                .yielding_to(self.read_ctl.clone());
                // The crash budget counts across the whole cycle.
                let mut fx = Faulty::new(real, self.fault, &mut self.hooks).starting_at(used);
                let mut last_publish = None;
                crate::tracking::phase(&self.progress, pass_id, crate::history::Phase::Publishing);
                let applied_keys: BTreeSet<FolderKey> = applied_plan
                    .iter()
                    .flat_map(|p| p.packages.iter())
                    .flat_map(|q| {
                        std::iter::once(q.package.clone())
                            .chain(q.folders.iter().map(|f| f.key.clone()))
                            .chain(q.removes.iter().map(|r| r.key.clone()))
                    })
                    .collect();
                // An install that crash recovery finished was not made here either.
                let applied_keys: BTreeSet<FolderKey> = applied_keys
                    .into_iter()
                    .chain(rec.ops.iter().map(|(_, k, _)| k.clone()))
                    .collect();
                let publishing = crate::tracking::PublishProgress::new(&self.progress, pass_id);
                fx.inner_mut().set_publishing(publishing.clone());
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
                    (&publishing, (pass_id, now_unix_ms()), &applied_keys),
                );
                report.crashed = fx.crashed();
                if last_publish.is_some() {
                    self.last_publish = last_publish;
                    quiet_cycle = false;
                }
            }
        }
        // The crash counter only matters for one cycle.
        self.fault.crash_after = None;
        if report.crashed {
            return Ok(report);
        }
        // Nothing changed since the look that just ran: a second look would only repeat it.
        let last_look = match if quiet_cycle {
            Ok(None)
        } else {
            self.observe_world_recent(false, &mut scanned_at).map(Some)
        } {
            Ok(fresh) => {
                if let Some(fresh) = fresh {
                    w = fresh;
                }
                None
            }
            Err(ServiceError::Cancelled) => None,
            Err(e) => Some(e.to_string()),
        };
        self.notify_cycle(&w, &report);
        report.status = self.snapshot(&w, &area);
        if let Some(e) = last_look {
            // Show the failed last look in this pass, not the next one.
            self.provider_error = Some(e.clone());
            report.status.headline = headline_sentence(Headline::Error);
            report.status.provider_error = Some(e);
        }
        self.log_cycle(&report);
        Ok(report)
    }

    /// Removal steps 3a, 3b and 6 (removal.md §1.6): observe and write matured removal records,
    /// write `absent` acknowledgements, and plan remove operations. Ledger writes never touch
    /// game files, so only the returned operations depend on `game_blocks_changes`. A computer
    /// whose retirement is settled or unresolved writes nothing (ops.md §2, Q-60).
    #[allow(clippy::too_many_arguments)]
    fn removal_steps(
        state: &LocalState,
        plane: &mut Plane,
        provider: &ProviderRoot,
        w: &World,
        journaled: &BTreeSet<FolderKey>,
        game_blocks_changes: bool,
        senv: &SettingsEnv,
        report: &mut SyncReport,
    ) -> (Vec<PackageApply>, u64, BTreeSet<FolderKey>) {
        if w.standing != OwnStanding::Active {
            return (Vec::new(), w.enrollment.next_seq, BTreeSet::new());
        }
        let gid = &w.group.group_id;
        let now = now_unix_ms() as u64;
        let mut next_seq = w.enrollment.next_seq;
        let mut pass = SettingsPass::open(state, gid, senv, game_blocks_changes);
        let mut held: BTreeSet<FolderKey> = BTreeSet::new();
        let mut recorded: BTreeSet<FolderKey> = BTreeSet::new();
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
                    return (Vec::new(), next_seq, BTreeSet::new());
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
                // Owned settings are captured with the record; with the game running they
                // cannot be taken, so the removal stays pending until it is closed.
                let keys: BTreeSet<FolderKey> = r.folders.iter().map(|f| f.key.clone()).collect();
                let Ok(taken) = pass.take(&keys) else {
                    continue;
                };
                let rec = removal_record_with_settings(
                    gid,
                    &w.own,
                    audience.clone(),
                    r,
                    next_seq,
                    now,
                    taken.roots,
                );
                match write_own_record(
                    plane,
                    provider,
                    gid,
                    &w.own.computer_id,
                    &AuthorityRecord::Removal(rec),
                ) {
                    Ok(_) => {
                        RemovalStore::new(state, gid.clone()).clear_pending(&r.package);
                        next_seq += 1;
                        written = true;
                        recorded.extend(keys.iter().cloned());
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
        // Settings of removals owed here: what is captured now for folders about to be removed,
        // and what an earlier cycle captured but has not yet cleaned up.
        let mut gathered = Gathered::default();
        if !game_blocks_changes {
            let probe = CycleCx {
                group_id: gid,
                own: &w.own,
                ledger: &plane.ledger,
                derived: &derived,
                members: &plane.members,
                now_unix_ms: now,
                held: &held,
                roots: &[],
            };
            let folders: BTreeSet<FolderKey> = plan_remove_ops(&probe, &w.inventory, next_seq)
                .iter()
                .flat_map(|o| o.removes.iter().map(|r| r.key.clone()))
                .collect();
            if !folders.is_empty() {
                if let Ok(g) = pass.take(&folders) {
                    gathered.extend(g);
                }
            }
            if !behaviour.classify_due(&probe).is_empty() {
                gathered.extend(pass.pending_cleanup());
            }
        }
        let acks = {
            let cx = CycleCx {
                group_id: gid,
                own: &w.own,
                ledger: &plane.ledger,
                derived: &derived,
                members: &plane.members,
                now_unix_ms: now,
                held: &held,
                roots: &gathered.roots,
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
            roots: &gathered.roots,
        };
        let mut ops = step_3a_with(
            &cx,
            &w.inventory,
            next_seq,
            game_blocks_changes,
            &gathered.files,
        );
        // Each planned operation reserves an acknowledgement at its own sequence number.
        next_seq += ops.len() as u64;
        ops.extend(pass.restore_ops(&w.inventory, &w.own, &|k| {
            !behaviour.merged_removals(&plane.ledger, k).live.is_empty()
        }));
        (ops, next_seq, recorded)
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
        track: (
            &crate::tracking::PublishProgress,
            (u64, i64),
            &BTreeSet<FolderKey>,
        ),
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
        let publishable = inventory_without(&w.inventory, &w.withheld);
        if !publish_needed(
            w.own_latest.as_ref(),
            &publishable,
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
        let peer_states: Vec<&CodeState> = w
            .views
            .iter()
            .filter(|m| m.computer != *me)
            .filter_map(|m| match &m.latest {
                MemberLatest::Valid { state, .. } if state.computer_id == m.computer => {
                    Some(&**state)
                }
                _ => None,
            })
            .collect();
        let client = Self::client_info(&w.variant);
        let lineage = wow_sync_core::lineage::next_lineage(
            &publishable,
            &client,
            w.own_latest.as_ref(),
            &peer_states,
        );
        let mut ctx = PublishContext {
            group: group_id.clone(),
            computer: me.clone(),
            enrollment: w.enrollment.enrollment_id.clone(),
            product: w.enrollment.product.clone(),
            app_version: cfg.app_version.clone(),
            now_unix_ms: now_unix_ms() as u64,
            seq,
            client,
            apply_report: apply_rep,
            own_state_files: w.own_states.clone(),
            authority_index: index_head,
            features,
            lineage,
        };
        let plan = match wow_sync_core::plan::plan_publish_state(&publishable, &ctx) {
            Ok(p) => p,
            Err(b) => {
                report.publish_note = Some(format!("not published: {}", render_block(&b)));
                return;
            }
        };
        // The sequence number is persisted before anything is published so it is never reused.
        track
            .0
            .set_total(wow_sync_core::plan::count_publish_objects(&publishable));
        let mut le = w.enrollment.clone();
        le.next_seq = seq + 1;
        if state.write_atomic("enrollment.json", &encode(&le)).is_err() {
            report.publish_note = Some("not published: local state could not be saved".into());
            return;
        }
        let group_for_objects = ctx.group.clone();
        let objects = wow_sync_core::plan::publish_objects(&publishable, &group_for_objects);
        match wow_sync_core::exec::run_publish_objects(&plan, objects, &mut ctx, fx) {
            PublishOutcome::Published { uploaded, .. } => {
                // The earlier file lists are read only for the folders that changed.
                let earlier = |h: &Sha256| {
                    provider
                        .read(&layout::tree_record(group_id, h), TREE_LIMIT)
                        .ok()
                        .and_then(|b| decode_tree(&b, h).ok())
                        .map(|t| t.files)
                };
                for rec in crate::changes::published(
                    track.1,
                    w.own_latest.as_ref(),
                    &publishable,
                    &w.inventory,
                    &earlier,
                    track.2,
                ) {
                    crate::history::record_change(state, &rec);
                    track.0.recorded(rec);
                }
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
            PublishOutcome::Yielded => {
                report.publish_note = Some(
                    "publishing paused to answer a command; it continues in the next pass".into(),
                );
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

    pub fn toggles(&self) -> crate::control::AppToggles {
        crate::control::AppToggles::load(&self.state)
    }

    pub fn save_toggles(&self, t: &crate::control::AppToggles) -> Result<(), ServiceError> {
        t.save(&self.state).map_err(ServiceError::Local)
    }

    pub fn save_app_settings(&self, s: &crate::control::AppSettings) -> Result<(), ServiceError> {
        s.save(&self.state).map_err(ServiceError::Local)
    }

    /// Newest first (UG-7).
    /// The progress stream; clone it to subscribe from another thread.
    pub fn progress_hub(&self) -> crate::history::ProgressHub {
        self.progress.clone()
    }

    pub fn history(&self, limit: usize) -> Vec<crate::history::HistoryItem> {
        crate::history::history(&self.state, limit)
    }

    pub fn changes_by_pass(&self, limit: usize) -> Vec<crate::history::PassGroup> {
        crate::history::changes_by_pass(&self.state, limit)
    }

    pub fn package_changes(
        &self,
        package: &str,
        limit: usize,
    ) -> Vec<crate::history::ChangeRecord> {
        crate::history::changes_for_package(&self.state, package, limit)
    }

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

#[cfg(test)]
mod reason_text_tests {
    use super::*;

    #[test]
    fn test_mode_refusal_is_flagged_so_the_offer_can_be_shown() {
        for r in [
            RefuseReason::NotSyntheticInstall,
            RefuseReason::Layout(LayoutReason::NotSyntheticInstall),
        ] {
            let (text, retry) = refuse_text(&r);
            assert!(!retry);
            assert!(text.contains("Turn off test mode"), "{text}");
            assert!(text.contains("Back"), "{text}");
        }
    }

    fn all_refusals() -> Vec<RefuseReason> {
        let s = || "RAW_INTERNAL".to_string();
        vec![
            RefuseReason::FlavorMissing,
            RefuseReason::FlavorUnreadable(s()),
            RefuseReason::FlavorUnparseable,
            RefuseReason::InvalidProductCode,
            RefuseReason::BuildInfo(s()),
            RefuseReason::ProductNotInBuildInfo,
            RefuseReason::ProductAmbiguousInBuildInfo,
            RefuseReason::ProductInactive,
            RefuseReason::FolderMismatch,
            RefuseReason::AddOnsMissing,
            RefuseReason::NotSyntheticInstall,
            RefuseReason::Layout(LayoutReason::AddOnsLayout),
            RefuseReason::Layout(LayoutReason::DuplicateProductFolder),
            RefuseReason::Layout(LayoutReason::AlreadyEnrolled),
            RefuseReason::Layout(LayoutReason::NotSyntheticInstall),
            RefuseReason::ConfirmationRequired,
            RefuseReason::StaleConfirmation,
            RefuseReason::VariantChanged,
        ]
    }

    fn plain(text: &str, what: &str) {
        for bad in [
            "{",
            "}",
            "RAW_INTERNAL",
            "Reason",
            "::",
            "Layout(",
            "NotSynthetic",
        ] {
            assert!(!text.contains(bad), "{what} shows {bad:?}: {text}");
        }
        assert!(text.ends_with('.'), "{what} is not a sentence: {text}");
    }

    #[test]
    fn every_refusal_reads_as_a_sentence_that_says_what_to_do() {
        for r in all_refusals() {
            let (text, retry) = refuse_text(&r);
            plain(&text, &format!("{r:?}"));
            let consent = matches!(
                r,
                RefuseReason::ConfirmationRequired | RefuseReason::StaleConfirmation
            );
            if retry {
                assert!(text.contains("Try again"), "{r:?}: {text}");
            } else if !consent {
                assert!(text.contains("Back"), "{r:?} points nowhere: {text}");
                assert!(!text.contains("Try again"), "{r:?}: {text}");
            }
        }
    }

    #[test]
    fn every_block_reason_reads_plainly() {
        let s = || "RAW_INTERNAL".to_string();
        let all = [
            BlockReason::UnsupportedName(s()),
            BlockReason::CaseCollision,
            BlockReason::NotReadable(s()),
        ];
        let mut seen = std::collections::HashSet::new();
        for r in all {
            let t = block_text(&r);
            for bad in ["{", "}", "RAW_INTERNAL", "Toc", "Reason", "::"] {
                assert!(!t.contains(bad), "{r:?} shows {bad:?}: {t}");
            }
            assert!(seen.insert(t), "{r:?} wording is not distinct");
        }
    }
}
