//! Platform-free core of WoW Sync: identities, validation, planning, execution and status.
//! All effects go through the port traits in [`exec`]; this crate does no OS I/O.
#![forbid(unsafe_code)]

pub mod admit;
mod arch;
pub mod authority;
pub mod bpsv;
pub mod choice;
#[cfg(test)]
mod choice_tests;
pub mod decide;
#[cfg(test)]
mod decide_tests;
pub mod decision;
pub mod diagnostics;
pub mod exec;
pub mod explain;
pub mod hash;
pub mod hooks;
pub mod ids;
pub mod inventory;
pub mod kinds;
pub mod layout;
pub mod ledger;
pub mod notice;
pub mod package;
pub mod paths;
pub mod plan;
pub mod records;
pub mod reject;
#[cfg(test)]
mod reject_tests;
pub mod removal;
#[cfg(test)]
mod removal_tests;
pub mod report;
pub mod retire;
#[cfg(test)]
mod retire_tests;
pub mod settings;
#[cfg(test)]
mod settings_tests;
pub mod status;
pub mod target;
pub mod toc;
pub mod update;
pub mod variant;
pub mod version;
pub mod view;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
