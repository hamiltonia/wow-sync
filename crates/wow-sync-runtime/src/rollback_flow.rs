//! The rollback flow (rollback.md §1, §9, R4D-27). The settings half reports what was really
//! restored. The code half turns a queued request into one `rollback` operation built from the
//! live scan (R1) and the recovered release's trees (R0).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::ids::{GroupId, OpId, SettingsUnit, Sha256};
use wow_sync_core::inventory::{FolderStatus, Inventory};
use wow_sync_core::kinds::{EnrollmentRef, Issuer, RejectionSettings};
use wow_sync_core::layout;
use wow_sync_core::ledger::derive;
use wow_sync_core::paths::FolderKey;
use wow_sync_core::plan::{ExpectedLive, PackageApply};
use wow_sync_core::records::{decode_tree, FolderTreeRecord, SettingsRoot};
use wow_sync_core::reject::{
    plan_rollback, LiveFolder, RollbackError, RollbackInput, TargetFolder,
};
use wow_sync_core::removal::known_summary;
use wow_sync_core::target::PackageCopies;

use crate::decisions::Plane;

const TREE_LIMIT: u64 = 32 * 1024 * 1024;

/// A rollback the owner asked for, kept until the apply gate lets it run. It names the release
/// by package hash only; everything else is read again from the live state when it runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub package: FolderKey,
    pub recovered: Sha256,
}

/// Why a rollback cannot be planned. Nothing is queued or written in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackRefusal {
    /// No release with this package hash is held for the package.
    UnknownRelease(Sha256),
    /// The release is what is installed now.
    SameRelease,
    /// Nothing of the package is installed, or it is not readable.
    NotInstalled,
    /// The recovered release's file list or files are not in the shared folder.
    NotAvailable,
}

fn request_name(group: &GroupId) -> String {
    format!("rollback-request-{group}.json")
}

pub fn load_request(state: &LocalState, group: &GroupId) -> Option<RollbackRequest> {
    let b = state.read(&request_name(group), 64 * 1024).ok().flatten()?;
    serde_json::from_slice(&b).ok()
}

pub fn save_request(
    state: &LocalState,
    group: &GroupId,
    r: &RollbackRequest,
) -> Result<(), wow_sync_adapters::FsError> {
    state.write_atomic(
        &request_name(group),
        &serde_json::to_vec(r).expect("plain data serializes"),
    )
}

pub fn clear_request(state: &LocalState, group: &GroupId) {
    let _ = state.remove(&request_name(group));
}

pub(crate) struct PlanArgs<'a> {
    pub request: &'a RollbackRequest,
    pub group: &'a GroupId,
    pub own: &'a EnrollmentRef,
    pub audience: Vec<EnrollmentRef>,
    pub inventory: &'a Inventory,
    pub copies: &'a [PackageCopies],
    pub plane: &'a Plane,
    pub provider: &'a ProviderRoot,
    pub variant_dir: &'a std::path::Path,
    pub op: OpId,
    pub seq: u64,
    pub now_unix_ms: u64,
}

/// Builds the queued rollback, if any, from the live world. The request is consumed either way:
/// a refused one is reported and dropped (the owner asks again against what is live then), a
/// planned one is carried by its operation journal. The record sequence it uses is reserved
/// before anything is applied.
pub(crate) fn step(
    state: &LocalState,
    a: &PlanArgs,
    enrollment: &wow_sync_core::records::LocalEnrollment,
) -> Result<PackageApply, RollbackRefusal> {
    let out = plan_requested(a);
    clear_request(state, a.group);
    if out.is_ok() {
        let mut le = enrollment.clone();
        le.next_seq = a.seq + 1;
        let _ = state.write_atomic("enrollment.json", &wow_sync_core::records::encode(&le));
    }
    out
}

/// R1 is what is installed now for the package, R0 the recovered release read from the shared
/// folder. The result is the single operation that installs R0 and carries the records.
pub(crate) fn plan_requested(a: &PlanArgs) -> Result<PackageApply, RollbackRefusal> {
    let unknown = RollbackRefusal::UnknownRelease(a.request.recovered);
    let pc = a
        .copies
        .iter()
        .find(|p| p.package.key == a.request.package)
        .ok_or(unknown.clone())?;
    let chosen = pc
        .copies
        .iter()
        .find(|c| c.version.hash == a.request.recovered)
        .ok_or(unknown)?;
    let mut r1 = Vec::new();
    for k in &pc.package.members {
        let Some(f) = a.inventory.get(k) else {
            continue;
        };
        match (f.status, f.tree, f.fingerprint) {
            (FolderStatus::Ok, Some(tree), Some(fingerprint)) => r1.push(LiveFolder {
                key: f.key.clone(),
                name: f.name.clone(),
                tree,
                expected_live: ExpectedLive::Tree { tree, fingerprint },
            }),
            _ => return Err(RollbackRefusal::NotInstalled),
        }
    }
    let mut r0 = Vec::new();
    for f in &chosen.version.folders {
        let Some(tree) = f.tree else {
            return Err(RollbackRefusal::NotAvailable);
        };
        let record = a
            .provider
            .read(&layout::tree_record(a.group, &tree), TREE_LIMIT)
            .ok()
            .and_then(|b| decode_tree(&b, &tree).ok())
            .or_else(|| {
                crate::effects::local_release(a.variant_dir, &f.name, &tree).map(|l| {
                    FolderTreeRecord {
                        tree,
                        files: l.files,
                    }
                })
            })
            .ok_or(RollbackRefusal::NotAvailable)?;
        r0.push(TargetFolder {
            key: f.key.clone(),
            name: f.name.clone(),
            record,
        });
    }
    let display = a.inventory.get(&pc.package.display);
    let title = display.map_or_else(|| pc.package.display.to_string(), |f| f.name.clone());
    let rejected_label = display
        .and_then(|f| f.toc.version.clone())
        .map(|v| v.chars().take(100).collect())
        .unwrap_or_default();
    let added: BTreeSet<&FolderKey> = r0
        .iter()
        .map(|t| &t.key)
        .filter(|k| !r1.iter().any(|f| &f.key == *k))
        .collect();
    let derived = derive(&a.plane.ledger, a.group, &a.plane.members);
    let mut readd_covers = Vec::new();
    for e in a.plane.ledger.entries() {
        if let Some(AuthorityRecord::Removal(r)) = a.plane.ledger.record(&e.hash) {
            if derived.status.get(&e.hash).is_some_and(|s| s.is_valid())
                && r.folders.iter().any(|f| added.contains(&f.key))
            {
                readd_covers.push(e.hash);
            }
        }
    }
    plan_rollback(&RollbackInput {
        op: a.op.clone(),
        package: a.request.package.clone(),
        group_id: a.group.clone(),
        issuer: Issuer {
            computer_id: a.own.computer_id.clone(),
            enrollment_id: a.own.enrollment_id.clone(),
            issued_at_seq: a.seq,
        },
        title,
        rejected_label,
        r1,
        r0,
        considered: Vec::new(),
        settings: RejectionSettings::NotApplicable,
        known: known_summary(&a.plane.ledger, a.group),
        audience: a.audience.clone(),
        readd_covers,
        now_unix_ms: a.now_unix_ms,
    })
    .map_err(|e| match e {
        RollbackError::SameRelease => RollbackRefusal::SameRelease,
        RollbackError::EmptyRelease => RollbackRefusal::NotInstalled,
    })
}

/// `roots` are the recovery point's settings roots; `restored` the units whose files were really
/// restored by this rollback. A root that was captured but not restored counts as not restored.
#[allow(dead_code)] // the settings half of rollback is wired with the settings step
pub fn actual_settings_result(
    roots: &[SettingsRoot],
    restored: &BTreeSet<SettingsUnit>,
) -> RejectionSettings {
    if roots.is_empty() {
        return RejectionSettings::NotApplicable;
    }
    let done = roots
        .iter()
        .filter(|r| r.captured() && restored.contains(&r.unit))
        .count();
    if done == roots.len() {
        RejectionSettings::Paired
    } else if done == 0 {
        RejectionSettings::CodeOnly
    } else {
        RejectionSettings::Partial
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::Identity;
    use wow_sync_core::paths::validate_folder_name;
    use wow_sync_core::records::{Location, MissingReason, RootState};

    fn unit(f: &str) -> SettingsUnit {
        SettingsUnit {
            folder: validate_folder_name(f).unwrap(),
            identity: Identity::account("ERIC").unwrap(),
        }
    }

    fn captured(f: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(f),
            state: RootState::Captured {
                checkpoint: sha256(f.as_bytes()),
                location: Location::Local,
            },
        }
    }

    fn missing(f: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(f),
            state: RootState::Missing {
                reason: MissingReason::Paused,
            },
        }
    }

    #[test]
    fn the_result_reports_what_was_restored_not_what_was_expected() {
        let all: BTreeSet<SettingsUnit> = [unit("a"), unit("b")].into();
        let roots = vec![captured("a"), captured("b")];
        assert_eq!(
            actual_settings_result(&roots, &all),
            RejectionSettings::Paired
        );
        // both were expected, only one was restored
        let one: BTreeSet<SettingsUnit> = [unit("a")].into();
        assert_eq!(
            actual_settings_result(&roots, &one),
            RejectionSettings::Partial
        );
        assert_eq!(
            actual_settings_result(&roots, &BTreeSet::new()),
            RejectionSettings::CodeOnly
        );
        // a missing root is never counted, even if its unit is in the restored set
        let mixed = vec![captured("a"), missing("b")];
        assert_eq!(
            actual_settings_result(&mixed, &all),
            RejectionSettings::Partial
        );
        assert_eq!(
            actual_settings_result(&[], &all),
            RejectionSettings::NotApplicable
        );
    }
}
