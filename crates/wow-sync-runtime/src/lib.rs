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
mod notices;
mod removal_cycle;
mod rollback_flow;
mod service;
mod settings_cycle;
mod updates;

#[cfg(test)]
mod e2e;

pub use activity::{ActivityHandle, ActivitySource, ScriptedActivity, WinprocActivity};
pub use control::{
    log_activity, pause_state, paused_since, read_activity, set_paused, ActivityEntry, AppSettings,
    PauseState, QuitGate, StopState,
};
pub use detail::{
    strip_colour_codes, CandidateRow, FolderRow, MemberRow, PackageRow, StatusDetail,
};
pub use discover::{
    dropbox_defaults, dropbox_info_files, dropbox_paths_from_info, find_dropbox, find_wow,
    wow_candidates, DetectedInstall,
};
pub use effects::{FaultPlan, Faulty, Hooks, RealEffects};
pub use service::{
    AppliedLine, EnrollPreview, EnrollRequest, GroupInfo, JoinChoice, PackageLine, PreviewKind,
    ReconfirmRequest, ServiceConfig, ServiceError, SliceService, StatusSnapshot, SyncReport,
};

pub use wow_sync_core as core;
