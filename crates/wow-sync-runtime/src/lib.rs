//! Glue for the thin slice: implements the core port traits over the adapters and exposes
//! [`SliceService`] (inspect / enroll / sync_once / status). It makes no policy decision:
//! ordering, grouping, targets, plans and status all come from `wow-sync-core`.
#![forbid(unsafe_code)]

mod activity;
mod control;
mod decisions;
mod detail;
mod diag;
mod discover;
mod effects;
mod history;
mod notices;
mod removal_cycle;
mod restore_flow;
mod rollback_flow;
mod service;
mod settings_cycle;
mod updates;

#[cfg(test)]
mod e2e;

pub use activity::{ActivityHandle, ActivitySource, ScriptedActivity, WinprocActivity};
pub use control::{
    log_activity, pause_state, paused_since, read_activity, set_paused, ActivityEntry, AppSettings,
    AppToggles, Command, CommandError, CommandOutcome, LocalOnlyVersion, PauseState, QuitGate,
    StopState,
};
pub use detail::{
    strip_colour_codes, CandidateRow, FolderRow, HeldRelease, MemberRow, PackageRow, PeerRow,
    PeerStanding, RecoveryRow, RejectedRelease, RemovedRow, RetirementRow, StatusDetail,
};
pub use diag::{ExportError, SaveError};
pub use discover::{
    discover_wow, dropbox_defaults, dropbox_info_files, dropbox_paths_from_info, find_dropbox,
    find_wow, install_from_choice, is_dropbox_folder, paths_from_product_db, probe_list, scan,
    wow_candidates, DetectedInstall, Discovery, Probe, ProbeResult, PRODUCT_DB_LIMIT,
};
pub use effects::{FaultPlan, Faulty, Hooks, RealEffects};
pub use history::{
    changes_by_pass, changes_for_package, history, recent_changes, record_change, ChangeKind,
    ChangeReason, ChangeRecord, ChangeSource, FileCounts, HistoryItem, IncomingSummary, PassGroup,
    Phase, ProgressEvent, ProgressHub, VersionLabel, MAX_RECORDS, MAX_RECORD_PATHS,
};
pub use notices::{CycleOutcome, DataAtRisk, NoticeRunner, RiskKind};
pub use restore_flow::{RestoreProgress, RestoreState, RestoreWait};
pub use rollback_flow::{RollbackOutcome, RollbackOutcomeKind, RollbackRefusal};
pub use service::{
    AppliedLine, EnrollPreview, EnrollRequest, GroupInfo, HashCache, Interrupt, JoinChoice,
    PackageLine, PreviewKind, ReconfirmRequest, ScanCtl, ServiceConfig, ServiceError, SliceService,
    StatusSnapshot, SyncReport,
};
pub use settings_cycle::{
    HeadChoice, PausedFolder, SettingsHistoryEntry, SettingsSnapshot, SettingsUnitRow,
    SettingsUnitStatus,
};
pub use updates::{system_clock, Clock, Started, UpdateService, UpdateView};

pub use wow_sync_core as core;
