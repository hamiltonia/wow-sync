//! Platform-free core of WoW Sync: identities, validation, planning, execution and status.
//! All effects go through the port traits in [`exec`]; this crate does no OS I/O.
#![forbid(unsafe_code)]

pub mod admit;
mod arch;
pub mod bpsv;
pub mod exec;
pub mod explain;
pub mod hash;
pub mod ids;
pub mod inventory;
pub mod layout;
pub mod package;
pub mod paths;
pub mod plan;
pub mod records;
pub mod status;
pub mod target;
pub mod toc;
pub mod variant;
pub mod version;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
