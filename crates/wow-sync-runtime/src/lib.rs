//! Glue for the thin slice: implements the core port traits over the adapters and exposes
//! [`SliceService`] (inspect / enroll / sync_once / status). It makes no policy decision:
//! ordering, grouping, targets, plans and status all come from `wow-sync-core`.
#![forbid(unsafe_code)]

mod activity;
mod effects;
mod service;

#[cfg(test)]
mod e2e;

pub use activity::{ActivityHandle, ActivitySource, ScriptedActivity, WinprocActivity};
pub use effects::{FaultPlan, Faulty, Hooks, RealEffects};
pub use service::{
    AppliedLine, EnrollPreview, EnrollRequest, GroupInfo, JoinChoice, PackageLine, PreviewKind,
    ServiceConfig, ServiceError, SliceService, StatusSnapshot, SyncReport,
};

pub use wow_sync_core as core;
