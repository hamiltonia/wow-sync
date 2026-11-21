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

    pub(crate) fn reason(&self) -> String {
        match self {
            Self::UnknownRemoval => "this removal is no longer current".into(),
            Self::Rejected => "the group rejected this version".into(),
            Self::Unavailable(why) => why.clone(),
        }
    }
}

/// Why a restore could not be planned when its cycle ran. Kept until the owner retries or
/// dismisses it, so a restore that did nothing is never mistaken for one that worked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreIssue {
    pub removal: Sha256,
    pub reason: String,
}

/// What a restore is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreWait {
    WowRunning,
    /// Syncing is paused on this computer.
    Paused,
    /// Nothing blocks it; the next sync cycle runs it.
    NextCycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreState {
    Waiting {
        reason: RestoreWait,
    },
    /// The cycle that tried could not restore it. Nothing was changed.
    Unavailable {
        text: String,
    },
}

/// One restore the owner asked for that has not finished (removal.md §7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreProgress {
    /// The first folder of the removed add-on; `Command::DismissRestoreIssue` takes it.
    pub package: wow_sync_core::paths::FolderKey,
    /// The add-on's title: untrusted text with colour codes stripped.
    pub title: String,
    pub state: RestoreState,
    /// One plain sentence for the GUI.
    pub text: String,
}

fn request_name(group: &GroupId) -> String {
    format!("restore-request-{group}.json")
}

fn issue_name(group: &GroupId) -> String {
    format!("restore-issue-{group}.json")
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

pub fn load_issue(state: &LocalState, group: &GroupId) -> Option<RestoreIssue> {
    let b = state.read(&issue_name(group), 64 * 1024).ok().flatten()?;
    serde_json::from_slice(&b).ok()
}

pub(crate) fn save_issue(state: &LocalState, group: &GroupId, i: &RestoreIssue) {
    let _ = state.write_atomic(
        &issue_name(group),
        &serde_json::to_vec(i).expect("plain data serializes"),
    );
}

/// A new request replaces an earlier refusal.
pub fn clear_issue(state: &LocalState, group: &GroupId) {
    let _ = state.remove(&issue_name(group));
}

fn first_key(rm: &wow_sync_core::kinds::Removal) -> Option<wow_sync_core::paths::FolderKey> {
    rm.folders.iter().map(|m| m.key.clone()).min()
}

/// Whether the removal still stands (no re-add has lifted it). A request or issue about a
/// removal that no longer stands is finished or moot.
pub(crate) fn stands(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    removal: &Sha256,
) -> bool {
    wow_sync_core::removal::removal_units(ledger, derived)
        .iter()
        .any(|u| u.removals.contains(removal))
}

/// The cycle note on how the settings of a restore fare (removal.md §7.4 step 4). `captured` is
/// how many units of the removed folders have a saved copy on this computer; the removal's
/// roots say how many there were. Paired, partial and code-only read differently.
pub(crate) fn expected_units(ledger: &wow_sync_core::ledger::Ledger, removal: &Sha256) -> usize {
    match ledger.record(removal) {
        Some(AuthorityRecord::Removal(rm)) => match &rm.settings {
            wow_sync_core::kinds::RemovalSettings::Checkpoints { roots } => roots.len(),
            wow_sync_core::kinds::RemovalSettings::None => 0,
        },
        _ => 0,
    }
}

pub(crate) fn settings_note(expected: usize, captured: usize) -> String {
    if captured == 0 {
        CODE_ONLY_WARNING.to_string()
    } else if captured < expected {
        format!(
            "Settings were partly restored: {captured} of {expected} saved copies from before the removal"
        )
    } else {
        "Settings are restored with the code".to_string()
    }
}

/// Records how a planned restore ended: the settings note appears only after the code applied; a
/// failure (for example content that does not verify) becomes a refusal that stays until the
/// owner retries or dismisses it, instead of being planned again every cycle. Interrupted or
/// replanned operations stay queued.
pub(crate) fn settle(
    state: &LocalState,
    group: &GroupId,
    removal: &Sha256,
    result: Option<&wow_sync_core::exec::PackageResult>,
    note: String,
    apply_note: &mut Option<String>,
) {
    use wow_sync_core::exec::PackageResult as R;
    match result {
        Some(R::Applied { .. }) => *apply_note = Some(note),
        Some(R::Failed(e) | R::NeedsHelp(e)) => {
            clear_request(state, group);
            save_issue(
                state,
                group,
                &RestoreIssue {
                    removal: *removal,
                    reason: wow_sync_core::explain::render(e),
                },
            );
        }
        _ => {}
    }
}

/// Drops a request or refusal whose removal no longer stands: its re-add was written (finished)
/// or someone else lifted the removal (moot).
pub(crate) fn prune(
    state: &LocalState,
    group: &GroupId,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
) {
    if load_request(state, group).is_some_and(|r| !stands(ledger, derived, &r.removal)) {
        clear_request(state, group);
    }
    if load_issue(state, group).is_some_and(|i| !stands(ledger, derived, &i.removal)) {
        clear_issue(state, group);
    }
}

/// The restores in flight, for the status snapshot. A queued restore is listed until its
/// re-add is written; a cycle-time refusal until it is retried or dismissed.
pub(crate) fn progress(
    state: &LocalState,
    group: &GroupId,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    game_running: bool,
    paused: bool,
) -> Vec<RestoreProgress> {
    let removal_of = |h: &Sha256| match ledger.record(h) {
        Some(AuthorityRecord::Removal(rm)) => Some(rm),
        _ => None,
    };
    let mut out = Vec::new();
    if let Some(req) = load_request(state, group) {
        if let Some(rm) = removal_of(&req.removal).filter(|_| stands(ledger, derived, &req.removal))
        {
            if let Some(package) = first_key(rm) {
                let title = crate::detail::strip_colour_codes(&rm.title);
                let (reason, why) = if game_running {
                    (
                        RestoreWait::WowRunning,
                        "waiting for World of Warcraft to close",
                    )
                } else if paused {
                    (RestoreWait::Paused, "waiting for syncing to be resumed")
                } else {
                    (RestoreWait::NextCycle, "it will run on the next sync")
                };
                out.push(RestoreProgress {
                    package,
                    text: format!("Restoring {title}: {why}."),
                    title,
                    state: RestoreState::Waiting { reason },
                });
            }
        }
    }
    if let Some(i) = load_issue(state, group) {
        if let Some(rm) = removal_of(&i.removal).filter(|_| stands(ledger, derived, &i.removal)) {
            if let Some(package) = first_key(rm) {
                let title = crate::detail::strip_colour_codes(&rm.title);
                let reason = crate::detail::strip_colour_codes(&i.reason);
                out.push(RestoreProgress {
                    package,
                    text: format!("{title} can't be restored: {reason}. Nothing was changed."),
                    title,
                    state: RestoreState::Unavailable { text: reason },
                });
            }
        }
    }
    out
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

/// Plans the queued restore. A refusal consumes the request and is kept as an issue; a planned
/// one stays queued until its re-add is written, so an operation that did not finish is
/// planned again. The record sequence is reserved.
pub(crate) fn step(
    state: &LocalState,
    a: &PlanArgs,
    enrollment: &wow_sync_core::records::LocalEnrollment,
) -> Result<PackageApply, RestoreRefusal> {
    let out = plan_requested(a);
    match &out {
        Ok(_) => clear_issue(state, a.group),
        Err(why) => {
            clear_request(state, a.group);
            save_issue(
                state,
                a.group,
                &RestoreIssue {
                    removal: a.removal,
                    reason: why.reason(),
                },
            );
        }
    }
    if out.is_ok() {
        let mut le = enrollment.clone();
        le.next_seq = a.seq + 1;
        let _ = state.write_atomic("enrollment.json", &wow_sync_core::records::encode(&le));
    }
    out
}
