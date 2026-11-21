//! The restore of a removed add-on's code (removal.md §7.4). A queued request becomes one
//! `restore` operation built from the removal record's manifest: trees come from the shared
//! folder or a local recovery point, the staged package must hash to the record's
//! `removed_hash`, and nothing is planned when any part is unreadable. The `user_restore`
//! re-add rides in the operation and is written only after it verifies.

use serde::{Deserialize, Serialize};
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_adapters::FsErrorKind;
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::ids::{GroupId, OpId, Sha256};
use wow_sync_core::inventory::{FolderStatus, Inventory};
use wow_sync_core::kinds::EnrollmentRef;
use wow_sync_core::layout;
use wow_sync_core::ledger::derive;
use wow_sync_core::plan::{ExpectedLive, PackageApply};
use wow_sync_core::records::{decode_tree, FolderTreeRecord};
use wow_sync_core::reject::RejectBehaviour;
use wow_sync_core::removal::{plan_restore, RestoreError, RestoreFolder};

use crate::decisions::Plane;

const TREE_LIMIT: u64 = 32 * 1024 * 1024;
const OBJECT_LIMIT: u64 = 512 * 1024 * 1024;

/// The exact warning when the add-on comes back without its settings (removal.md §7.4, R-10).
pub const CODE_ONLY_WARNING: &str =
    "Settings were not restored: no saved copy from before the removal";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub removal: Sha256,
}

/// Why a restore was not planned. Nothing was installed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreRefusal {
    UnknownRemoval,
    /// The version the removal took away was rejected by the group (§7.4 step 6).
    Rejected,
    /// The code cannot be read back in full; the reason names what is missing.
    Unavailable(String),
}

impl RestoreRefusal {
    pub fn text(&self) -> String {
        match self {
            Self::UnknownRemoval => "restore not applied: no such removal".into(),
            Self::Rejected => "restore not applied: the group rejected this version".into(),
            Self::Unavailable(why) => format!("restore unavailable: {why}"),
        }
    }
}

fn request_name(group: &GroupId) -> String {
    format!("restore-request-{group}.json")
}

fn outcome_name(group: &GroupId) -> String {
    format!("restore-outcome-{group}.json")
}

pub fn load_request(state: &LocalState, group: &GroupId) -> Option<RestoreRequest> {
    let b = state.read(&request_name(group), 64 * 1024).ok().flatten()?;
    serde_json::from_slice(&b).ok()
}

pub fn save_request(
    state: &LocalState,
    group: &GroupId,
    r: &RestoreRequest,
) -> Result<(), wow_sync_adapters::FsError> {
    state.write_atomic(
        &request_name(group),
        &serde_json::to_vec(r).expect("plain data serializes"),
    )
}

pub fn clear_request(state: &LocalState, group: &GroupId) {
    let _ = state.remove(&request_name(group));
}

/// How the latest queued restore ended, as the activity log words it.
#[allow(dead_code)] // read by the GUI surface once one exists
pub fn load_outcome(state: &LocalState, group: &GroupId) -> Option<String> {
    let b = state.read(&outcome_name(group), 64 * 1024).ok().flatten()?;
    String::from_utf8(b).ok()
}

pub fn save_outcome(state: &LocalState, group: &GroupId, text: &str) {
    let _ = state.write_atomic(&outcome_name(group), text.as_bytes());
}

pub fn clear_outcome(state: &LocalState, group: &GroupId) {
    let _ = state.remove(&outcome_name(group));
}

pub(crate) struct PlanArgs<'a> {
    pub removal: Sha256,
    pub group: &'a GroupId,
    pub own: &'a EnrollmentRef,
    pub inventory: &'a Inventory,
    pub plane: &'a Plane,
    pub provider: &'a ProviderRoot,
    pub variant_dir: &'a std::path::Path,
    pub op: OpId,
    pub seq: u64,
    pub now_unix_ms: u64,
}

pub(crate) fn plan_requested(a: &PlanArgs) -> Result<PackageApply, RestoreRefusal> {
    let derived = derive(&a.plane.ledger, a.group, &a.plane.members);
    let Some(AuthorityRecord::Removal(rm)) = a
        .plane
        .ledger
        .record(&a.removal)
        .filter(|_| derived.status.get(&a.removal).is_some_and(|s| s.is_valid()))
    else {
        return Err(RestoreRefusal::UnknownRemoval);
    };
    let rejected = RejectBehaviour::new(&derived)
        .active(&a.plane.ledger)
        .iter()
        .any(|(_, r)| {
            r.rejected.folders.iter().any(|e| {
                rm.folders
                    .iter()
                    .any(|m| m.key == e.key && m.tree == e.tree)
            })
        });
    if rejected {
        return Err(RestoreRefusal::Rejected);
    }
    let mut folders = Vec::new();
    for m in &rm.folders {
        let local = || {
            crate::effects::local_release(a.variant_dir, &m.name, &m.tree).map(|l| {
                FolderTreeRecord {
                    tree: m.tree,
                    files: l.files,
                }
            })
        };
        let record = a
            .provider
            .read(&layout::tree_record(a.group, &m.tree), TREE_LIMIT)
            .ok()
            .and_then(|b| decode_tree(&b, &m.tree).ok())
            .or_else(local)
            .ok_or_else(|| {
                RestoreRefusal::Unavailable(format!(
                    "the file list of {} is not in the shared folder",
                    m.name
                ))
            })?;
        for file in &record.files {
            let missing = matches!(
                a.provider.open_read(&layout::blob(a.group, &file.sha256), OBJECT_LIMIT),
                Err(e) if e.kind == FsErrorKind::NotFound
            );
            if missing && crate::effects::local_release(a.variant_dir, &m.name, &m.tree).is_none() {
                return Err(RestoreRefusal::Unavailable(format!(
                    "a file of {} is not in the shared folder or in a local copy",
                    m.name
                )));
            }
        }
        let live = match a.inventory.get(&m.key) {
            None => None,
            Some(f) => match (f.status, f.tree, f.fingerprint) {
                (FolderStatus::Ok, Some(tree), Some(fingerprint)) => {
                    Some(ExpectedLive::Tree { tree, fingerprint })
                }
                _ => {
                    return Err(RestoreRefusal::Unavailable(format!(
                        "{} is installed here but cannot be read",
                        m.name
                    )))
                }
            },
        };
        folders.push(RestoreFolder {
            key: m.key.clone(),
            name: m.name.clone(),
            record,
            live,
        });
    }
    plan_restore(
        &a.plane.ledger,
        a.own,
        rm,
        folders,
        a.op.clone(),
        a.seq,
        a.now_unix_ms,
    )
    .map_err(|e| match e {
        RestoreError::Empty => RestoreRefusal::Unavailable("the removal lists no folders".into()),
        RestoreError::ManifestMismatch | RestoreError::HashMismatch => {
            RestoreRefusal::Unavailable("the files do not match the removed version".into())
        }
    })
}

/// Plans the queued restore and consumes the request either way: a refusal is kept as the
/// outcome, a planned one is carried by its operation. The record sequence is reserved.
pub(crate) fn step(
    state: &LocalState,
    a: &PlanArgs,
    enrollment: &wow_sync_core::records::LocalEnrollment,
) -> Result<PackageApply, RestoreRefusal> {
    let out = plan_requested(a);
    clear_request(state, a.group);
    match &out {
        Ok(_) => clear_outcome(state, a.group),
        Err(why) => save_outcome(state, a.group, &why.text()),
    }
    if out.is_ok() {
        let mut le = enrollment.clone();
        le.next_seq = a.seq + 1;
        let _ = state.write_atomic("enrollment.json", &wow_sync_core::records::encode(&le));
    }
    out
}
