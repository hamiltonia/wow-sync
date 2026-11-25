//! The removal cycle's local state and steps (removal.md §1.6, §5.6, §7): the persisted presence
//! baseline and observation, the gated steps 3a/3b, and the explicit user commands.
//!
//! Nothing here runs by itself yet: `service.rs` has no step skeleton to call these from, so the
//! functions are exercised by their tests and wait for that wiring.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::Path;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::settings_store::SettingsStore;
use wow_sync_adapters::FsError;
use wow_sync_core::decision::{AckItem, CleanupClass, CycleCx};
use wow_sync_core::explain::Explanation;
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{CheckpointRef, GroupId, OpId, SettingsUnit, Sha256};
use wow_sync_core::inventory::{FolderState, Inventory};
use wow_sync_core::kinds::{Ack, EnrollmentRef, Readd, ReaddEvidence, Removal, RemovalEvidence};
use wow_sync_core::ledger::Ledger;
use wow_sync_core::paths::{FolderKey, SettingsPath};
use wow_sync_core::plan::PackageApply;
use wow_sync_core::records::{Location, MissingReason, RootState, SettingsRoot};
use wow_sync_core::removal::{
    absent_acks, observe, plan_cleanup_ops, plan_remove_ops, plan_remove_ops_with,
    readd_for_restore, removal_record, Baseline, Observation, ObserveInput, Observed,
    ObservedRemoval,
};
use wow_sync_core::settings::{removal_targets, DeclaringFolder, OwnedFile, SettingsScope};

use crate::settings_cycle::{capture_unit, load_local, observe_scope, plan_settings_apply};

const LIMIT: u64 = 8 * 1024 * 1024;

fn baseline_name(group: &GroupId) -> String {
    format!("removal-baseline-{group}.json")
}

fn observation_name(group: &GroupId) -> String {
    format!("removal-observation-{group}.json")
}

/// The baseline and observation of one group, kept in local state so a restart neither forgets a
/// removal nor shortens the 30-second confirmation (E4).
pub struct RemovalStore<'a> {
    state: &'a LocalState,
    group: GroupId,
}

impl<'a> RemovalStore<'a> {
    pub fn new(state: &'a LocalState, group: GroupId) -> Self {
        Self { state, group }
    }

    /// A missing, unreadable or foreign-looking file is a fresh start: no baseline can only mean
    /// that nothing is judged missing, never that something is.
    pub fn load(&self) -> (Baseline, Observation) {
        let base = self
            .state
            .read(&baseline_name(&self.group), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| Baseline::from_bytes(&b))
            .unwrap_or_default();
        let obs = self
            .state
            .read(&observation_name(&self.group), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| Observation::from_bytes(&b))
            .unwrap_or_default();
        (base, obs)
    }

    pub fn save(&self, base: &Baseline, obs: &Observation) -> Result<(), FsError> {
        self.state
            .write_atomic(&observation_name(&self.group), &obs.to_bytes())?;
        self.state
            .write_atomic(&baseline_name(&self.group), &base.to_bytes())
    }

    /// A package the group removed and this computer applied is not a local disappearance:
    /// forgetting it keeps the next observation from turning the applied removal into a new one.
    pub fn forget_group_removed(
        &self,
        inventory: &Inventory,
        removed_by_group: impl Fn(&FolderKey) -> bool,
    ) -> Result<(), FsError> {
        let (mut base, mut obs) = self.load();
        let gone: Vec<String> = base
            .packages
            .iter()
            .filter(|(_, fs)| {
                fs.iter()
                    .all(|f| inventory.get(&f.key).is_none() && removed_by_group(&f.key))
            })
            .map(|(n, _)| n.clone())
            .collect();
        if gone.is_empty() {
            return Ok(());
        }
        for n in gone {
            base.packages.remove(&n);
            obs.gone.remove(&n);
        }
        self.save(&base, &obs)
    }

    /// Packages this computer just removed on the group's behalf, whatever the removal's state.
    pub fn forget_keys(&self, keys: &BTreeSet<FolderKey>) -> Result<(), FsError> {
        let (mut base, mut obs) = self.load();
        let gone: Vec<String> = base
            .packages
            .iter()
            .filter(|(_, fs)| fs.iter().all(|f| keys.contains(&f.key)))
            .map(|(n, _)| n.clone())
            .collect();
        if gone.is_empty() {
            return Ok(());
        }
        for n in gone {
            base.packages.remove(&n);
            obs.gone.remove(&n);
        }
        self.save(&base, &obs)
    }

    /// One observation pass; the new baseline and observation are persisted before the result is
    /// returned, so a crash cannot lose a first sighting. A removal this pass matured is also
    /// kept as pending *before* the observation is consumed and returned again by every later
    /// pass until `clear_pending`, so a crash (or a failed write) before its record is written
    /// does not lose it. A pending removal whose folder is installed again is dropped.
    pub fn observe(&self, input: &ObserveInput) -> Result<Observed, FsError> {
        let (base, obs) = self.load();
        let mut out = observe(&base, &obs, input);
        let before = self.load_pending();
        let mut pending: Vec<PendingRemoval> = before
            .iter()
            .filter(|p| {
                p.folders
                    .iter()
                    .all(|f| input.inventory.get(&f.key).is_none())
            })
            .cloned()
            .collect();
        for r in &out.removed {
            if !pending.iter().any(|p| p.package == r.package) {
                pending.push(PendingRemoval::from(r));
            }
        }
        if pending != before {
            self.save_pending(&pending)?;
        }
        self.save(&out.baseline, &out.observation)?;
        out.removed = pending.into_iter().map(ObservedRemoval::from).collect();
        Ok(out)
    }

    fn pending_name(&self) -> String {
        format!("removal-pending-{}.json", self.group)
    }

    fn load_pending(&self) -> Vec<PendingRemoval> {
        self.state
            .read(&self.pending_name(), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save_pending(&self, p: &[PendingRemoval]) -> Result<(), FsError> {
        if p.is_empty() {
            return self.state.remove(&self.pending_name());
        }
        self.state.write_atomic(
            &self.pending_name(),
            &serde_json::to_vec(p).expect("plain data serializes"),
        )
    }

    /// The removal record for `package` is written; it is no longer pending.
    pub fn clear_pending(&self, package: &str) {
        let mut p = self.load_pending();
        p.retain(|r| r.package != package);
        let _ = self.save_pending(&p);
    }
}

/// An observed removal kept across a restart until its record is written.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PendingRemoval {
    package: String,
    folders: Vec<FolderState>,
    evidence: RemovalEvidence,
}

impl From<&ObservedRemoval> for PendingRemoval {
    fn from(r: &ObservedRemoval) -> Self {
        Self {
            package: r.package.clone(),
            folders: r.folders.clone(),
            evidence: r.evidence,
        }
    }
}

impl From<PendingRemoval> for ObservedRemoval {
    fn from(r: PendingRemoval) -> Self {
        Self {
            package: r.package,
            folders: r.folders,
            evidence: r.evidence,
        }
    }
}

// ---------------------------------------------------------------- steps

/// Step 3b: absent acknowledgements for items with nothing to clean. Items that wait for a
/// settings cleanup get none (T-R4D5-absent-ack-waits-cleanup). Ledger writes never touch game
/// files, so a running game does not block them.
pub fn step_3b(cx: &CycleCx, items: &[(AckItem, CleanupClass)], next_seq: u64) -> Vec<Ack> {
    absent_acks(cx, items, next_seq)
}

/// Step 3a: remove operations. Game files change here, so a running game, an unstable listing
/// or an unknown activity plans nothing.
pub fn step_3a(
    cx: &CycleCx,
    inv: &Inventory,
    next_seq: u64,
    game_blocks_changes: bool,
) -> Vec<PackageApply> {
    if game_blocks_changes {
        return Vec::new();
    }
    plan_remove_ops(cx, inv, next_seq)
}

// ---------------------------------------------------------------- settings of removed addons

/// What the settings side of a removal pass needs from the service.
pub struct SettingsEnv<'a> {
    pub cap: Option<u64>,
    pub variant_dir: &'a Path,
    /// The declarations of the folders installed now.
    pub decls: &'a [DeclaringFolder],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredDecl {
    folder: FolderKey,
    account: Vec<String>,
    character: Vec<String>,
}

/// A saved file this computer captured for an uninstall and has not yet put back.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Captured {
    unit: SettingsUnit,
    rel: SettingsPath,
    checkpoint: CheckpointRef,
    blob: Sha256,
}

/// Ownership evidence for folders that are gone (their declarations) and the captures taken for
/// removals. Declarations are metadata read from `.toc` files; no saved-variable file is read
/// for them.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct SettingsBook {
    decls: Vec<StoredDecl>,
    captures: Vec<Captured>,
}

const BOOK_DECLS: usize = 1024;
const BOOK_CAPTURES: usize = 256;

fn book_name(group: &GroupId) -> String {
    format!("removal-settings-{group}.json")
}

struct Active {
    scope: SettingsScope,
    store: SettingsStore,
    paused: BTreeSet<FolderKey>,
    cap: u64,
}

#[derive(Debug, Default)]
pub struct Gathered {
    pub roots: Vec<SettingsRoot>,
    pub files: Vec<OwnedFile>,
}

impl Gathered {
    pub fn extend(&mut self, other: Gathered) {
        for r in other.roots {
            if !self.roots.iter().any(|x| x.unit == r.unit) {
                self.roots.push(r);
            }
        }
        for f in other.files {
            if !self.files.iter().any(|x| x.unit == f.unit) {
                self.files.push(f);
            }
        }
    }
}

/// Owned files of removed folders cannot be taken while the game runs; the removal waits.
#[derive(Debug, PartialEq, Eq)]
pub struct Deferred;

/// One removal pass's view of settings. The scan of `WTF` happens only when something needs it.
pub struct SettingsPass<'a> {
    state: &'a LocalState,
    group: GroupId,
    env: &'a SettingsEnv<'a>,
    game_blocks: bool,
    book: SettingsBook,
    active: Option<Option<Active>>,
}

impl<'a> SettingsPass<'a> {
    pub fn open(
        state: &'a LocalState,
        group: &GroupId,
        env: &'a SettingsEnv<'a>,
        game_blocks: bool,
    ) -> Self {
        let mut book: SettingsBook = state
            .read(&book_name(group), LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut changed = false;
        for d in env.decls {
            let new = StoredDecl {
                folder: d.folder.clone(),
                account: d.saved_variables.clone(),
                character: d.saved_variables_per_character.clone(),
            };
            match book.decls.iter_mut().find(|s| s.folder == d.folder) {
                Some(s) if s.account == new.account && s.character == new.character => {}
                Some(s) => {
                    *s = new;
                    changed = true;
                }
                None => {
                    book.decls.push(new);
                    changed = true;
                }
            }
        }
        if book.decls.len() > BOOK_DECLS {
            book.decls
                .retain(|s| env.decls.iter().any(|d| d.folder == s.folder));
            changed = true;
        }
        let pass = Self {
            state,
            group: group.clone(),
            env,
            game_blocks,
            book,
            active: None,
        };
        if changed {
            pass.save();
        }
        pass
    }

    /// How many units of `folders` have a settings copy taken at removal on this computer.
    pub fn captured_units(&self, folders: &BTreeSet<FolderKey>) -> usize {
        self.book
            .captures
            .iter()
            .filter(|c| folders.contains(&c.unit.folder))
            .count()
    }

    fn save(&self) {
        let _ = self.state.write_atomic(
            &book_name(&self.group),
            &serde_json::to_vec(&self.book).expect("plain data serializes"),
        );
    }

    fn ensure(&mut self) {
        if self.active.is_some() {
            return;
        }
        self.active = Some(self.build());
    }

    fn build(&self) -> Option<Active> {
        let cap = self.env.cap?;
        let local = load_local(self.state, &self.group);
        // An uninstall removes the add-on's owned saved files whether or not settings sync was
        // ever turned on (R-04). Accounts the owner chose for sharing bound the scope; with no
        // choice made, every account on this computer is searched, by ownership declarations only.
        let included: BTreeSet<String> = if local.consent && !local.included_accounts.is_empty() {
            local.included_accounts.iter().cloned().collect()
        } else {
            crate::settings_cycle::available_accounts(self.env.variant_dir)
                .into_iter()
                .collect()
        };
        if included.is_empty() {
            return None;
        }
        let mut decls: Vec<DeclaringFolder> = self.env.decls.to_vec();
        for s in &self.book.decls {
            if !decls.iter().any(|d| d.folder == s.folder) {
                decls.push(DeclaringFolder {
                    folder: s.folder.clone(),
                    saved_variables: s.account.clone(),
                    saved_variables_per_character: s.character.clone(),
                    toc_ambiguous: false,
                });
            }
        }
        let scope = observe_scope(self.env.variant_dir, &decls, &included, Some(cap)).ok()?;
        let store = SettingsStore::open(self.env.variant_dir).ok()?;
        Some(Active {
            scope,
            store,
            paused: local.paused_folders.iter().cloned().collect(),
            cap,
        })
    }

    /// The owned files of `folders` and a root for each unit seen: captured into the local store
    /// when it is safe to move the file, otherwise missing with the reason. With the game running
    /// and a file to take, nothing is captured and the caller waits.
    pub fn take(&mut self, folders: &BTreeSet<FolderKey>) -> Result<Gathered, Deferred> {
        self.ensure();
        let Some(Some(a)) = &self.active else {
            return Ok(Gathered::default());
        };
        let t = removal_targets(&a.scope, folders, &a.paused);
        if self.game_blocks && !t.files.is_empty() {
            return Err(Deferred);
        }
        let mut out = Gathered::default();
        for (unit, reason) in t.missing {
            out.roots.push(SettingsRoot {
                unit,
                state: RootState::Missing { reason },
            });
        }
        let mut taken = Vec::new();
        for f in t.files {
            match capture_unit(self.env.variant_dir, &a.store, &f, a.cap) {
                Ok(checkpoint) => {
                    out.roots.push(SettingsRoot {
                        unit: f.unit.clone(),
                        state: RootState::Captured {
                            checkpoint,
                            location: Location::Local,
                        },
                    });
                    taken.push(Captured {
                        unit: f.unit.clone(),
                        rel: f.rel.clone(),
                        checkpoint,
                        blob: f.sha256,
                    });
                    out.files.push(f);
                }
                Err(_) => out.roots.push(SettingsRoot {
                    unit: f.unit,
                    state: RootState::Missing {
                        reason: MissingReason::Unreadable,
                    },
                }),
            }
        }
        if !taken.is_empty() {
            for c in taken {
                self.book.captures.retain(|x| x.unit != c.unit);
                self.book.captures.push(c);
            }
            let excess = self.book.captures.len().saturating_sub(BOOK_CAPTURES);
            self.book.captures.drain(..excess);
            self.save();
        }
        Ok(out)
    }

    /// Captures whose file is still in place unchanged: the cleanup they wait for is not done.
    pub fn pending_cleanup(&mut self) -> Gathered {
        let mut out = Gathered::default();
        if self.book.captures.is_empty() {
            return out;
        }
        self.ensure();
        let Some(Some(a)) = &self.active else {
            return out;
        };
        for c in &self.book.captures {
            if let Some(f) = a
                .scope
                .owned
                .iter()
                .find(|o| o.unit == c.unit && o.sha256 == c.blob)
            {
                out.roots.push(SettingsRoot {
                    unit: c.unit.clone(),
                    state: RootState::Captured {
                        checkpoint: c.checkpoint,
                        location: Location::Local,
                    },
                });
                out.files.push(f.clone());
            }
        }
        out
    }

    /// Puts a captured file back once its addon is installed again and no removal stands: the
    /// removal was undone. The live file is never overwritten; a newer file of the user's wins
    /// and the capture is dropped.
    pub fn restore_ops(
        &mut self,
        inv: &Inventory,
        own: &EnrollmentRef,
        removal_stands: &dyn Fn(&FolderKey) -> bool,
    ) -> Vec<PackageApply> {
        let mut out = Vec::new();
        if self.book.captures.is_empty() || self.game_blocks || !inv.listing_stable {
            return out;
        }
        if !self
            .book
            .captures
            .iter()
            .any(|c| inv.get(&c.unit.folder).is_some() && !removal_stands(&c.unit.folder))
        {
            return out;
        }
        self.ensure();
        let Some(Some(a)) = &self.active else {
            return out;
        };
        let mut keep = Vec::new();
        let mut changed = false;
        for c in self.book.captures.clone() {
            let folder = &c.unit.folder;
            if inv.get(folder).is_none() || removal_stands(folder) {
                keep.push(c);
                continue;
            }
            if a.scope.owned.iter().any(|o| o.unit == c.unit) {
                changed = true;
                continue;
            }
            let declares = self.env.decls.iter().any(|d| {
                &d.folder == folder
                    && if c.unit.identity.realm_character().is_none() {
                        !d.saved_variables.is_empty()
                    } else {
                        !d.saved_variables_per_character.is_empty()
                    }
            });
            if a.scope.blocked.contains(&c.unit) || !declares {
                keep.push(c);
                continue;
            }
            let seed = format!(
                "wowsync-restore-settings-v1\n{}\n{}",
                c.rel.as_str(),
                c.blob
            );
            let op = OpId::parse(&sha256(seed.as_bytes()).to_string()[..32]).expect("hex");
            out.push(plan_settings_apply(
                op,
                None,
                c.unit.clone(),
                c.rel.clone(),
                Some(c.blob),
                c.checkpoint,
                Explanation::Installed {
                    source: own.computer_id.clone(),
                },
            ));
            keep.push(c);
        }
        if changed {
            self.book.captures = keep;
            self.save();
        }
        out
    }
}

/// Step 3a with settings: remove operations that also move the captured settings files, and the
/// cleanup operations for removals whose folders are already gone. Cleanup acknowledgements
/// follow the remove operations' sequence numbers.
pub fn step_3a_with(
    cx: &CycleCx,
    inv: &Inventory,
    next_seq: u64,
    game_blocks_changes: bool,
    files: &[OwnedFile],
) -> Vec<PackageApply> {
    if game_blocks_changes {
        return Vec::new();
    }
    let mut ops = plan_remove_ops_with(cx, inv, next_seq, files);
    let cleanup = plan_cleanup_ops(cx, inv, next_seq + ops.len() as u64, files);
    ops.extend(cleanup);
    ops
}

// ---------------------------------------------------------------- commands

fn issuer_audience(own: &EnrollmentRef, mut audience: Vec<EnrollmentRef>) -> Vec<EnrollmentRef> {
    if !audience.contains(own) {
        audience.push(own.clone());
    }
    audience
}

fn as_observed(package: &str, folders: &[FolderState], e: RemovalEvidence) -> ObservedRemoval {
    ObservedRemoval {
        package: package.to_string(),
        folders: folders.to_vec(),
        evidence: e,
    }
}

/// "Remove from group" (and "Remove again" after a reinstall): an explicit removal of what is
/// installed here.
pub fn remove_from_group(
    group: &GroupId,
    own: &EnrollmentRef,
    audience: Vec<EnrollmentRef>,
    title: &str,
    folders: &[FolderState],
    own_seq: u64,
    now_unix_ms: u64,
) -> Removal {
    let o = as_observed(title, folders, RemovalEvidence::UserAction);
    removal_record(
        group,
        own,
        issuer_audience(own, audience),
        &o,
        own_seq,
        now_unix_ms,
    )
}

/// The user confirms that an unclear disappearance was a removal.
pub fn confirm_unclear(
    group: &GroupId,
    own: &EnrollmentRef,
    audience: Vec<EnrollmentRef>,
    removed: &ObservedRemoval,
    own_seq: u64,
    now_unix_ms: u64,
) -> Removal {
    let o = as_observed(
        &removed.package,
        &removed.folders,
        RemovalEvidence::UserConfirmed,
    );
    removal_record(
        group,
        own,
        issuer_audience(own, audience),
        &o,
        own_seq,
        now_unix_ms,
    )
}

/// The user says an unclear disappearance is not a removal: no record is written and the package
/// leaves the baseline, so it is not asked about again.
pub fn keep_unclear(base: &mut Baseline, obs: &mut Observation, package: &str) {
    base.packages.remove(package);
    obs.gone.remove(package);
}

/// "Restore": re-add a removed package for the group (explicit only, Q-55).
pub fn restore(
    ledger: &Ledger,
    group: &GroupId,
    own: &EnrollmentRef,
    removal: &Removal,
    own_seq: u64,
    now_unix_ms: u64,
) -> Readd {
    readd_for_restore(
        ledger,
        group,
        own,
        removal,
        ReaddEvidence::UserRestore,
        own_seq,
        now_unix_ms,
    )
}

/// "Add back": the user reinstalled it on purpose and wants it kept.
pub fn add_back(
    ledger: &Ledger,
    group: &GroupId,
    own: &EnrollmentRef,
    removal: &Removal,
    own_seq: u64,
    now_unix_ms: u64,
) -> Readd {
    readd_for_restore(
        ledger,
        group,
        own,
        removal,
        ReaddEvidence::UserAddBack,
        own_seq,
        now_unix_ms,
    )
}

/// "Continue without it": an unreadable removal stops holding installs, on this computer only.
pub fn continue_without(ledger: &mut Ledger, removal: Sha256) {
    ledger.dismiss(removal);
}

/// Folder keys that a game-file operation must not touch while a journal names them (E5).
pub fn journaled_keys(names: impl IntoIterator<Item = FolderKey>) -> BTreeSet<FolderKey> {
    names.into_iter().collect()
}

/// Synthetic data without the core testkit (the runtime does not enable that feature).
#[cfg(test)]
pub(crate) mod fixtures {
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::{ComputerId, EnrollmentId, GroupId};
    use wow_sync_core::inventory::{FolderState, FolderStatus, Inventory, TocKind, TocSummary};
    use wow_sync_core::paths::validate_folder_name;

    fn hex(n: u8) -> String {
        format!("{n:02x}").repeat(16)
    }

    pub fn computer(n: u8) -> ComputerId {
        ComputerId::parse(&hex(n)).unwrap()
    }

    pub fn enrollment(n: u8) -> EnrollmentId {
        EnrollmentId::parse(&hex(n)).unwrap()
    }

    pub fn group_id() -> GroupId {
        GroupId::parse(&hex(0x11)).unwrap()
    }

    pub fn addon(name: &str, _version: Option<&str>, tag: &str) -> FolderState {
        FolderState {
            name: name.into(),
            key: validate_folder_name(name).unwrap(),
            status: FolderStatus::Ok,
            tree: Some(sha256(format!("{name}:{tag}").as_bytes())),
            fingerprint: Some(sha256(format!("fp:{name}:{tag}").as_bytes())),
            files: 1,
            bytes: 1,
            toc: TocSummary::empty(TocKind::Selected),
            file_entries: Default::default(),
        }
    }

    pub fn inventory(folders: Vec<FolderState>) -> Inventory {
        Inventory {
            folders,
            listing_stable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{addon, computer, enrollment, group_id, inventory};
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::hash::sha256;
    use wow_sync_core::kinds::{AckResult, AuthorityKind};
    use wow_sync_core::ledger::{derive, Membership};
    use wow_sync_core::records::{decode, encode};
    use wow_sync_core::removal::{HoldReason, OBSERVE_GAP_MS};

    fn eref(n: u8) -> EnrollmentRef {
        EnrollmentRef {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
        }
    }

    fn input<'a>(
        inv: &'a Inventory,
        now: u64,
        none: &'a BTreeSet<FolderKey>,
        dir: &'a str,
    ) -> ObserveInput<'a> {
        ObserveInput {
            inventory: inv,
            dir_identity: dir,
            variant: "classic_era",
            now_unix_ms: now,
            journaled: none,
        }
    }

    #[test]
    fn persisted_observation_keeps_the_first_sighting_across_a_restart() {
        // T-R4D2-observe-persisted
        let dir = TempDir::new();
        let state = LocalState::open_at(&dir.join("state")).unwrap();
        let none = BTreeSet::new();
        let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
        let gone = inventory(vec![]);
        let seed = Baseline {
            variant: "classic_era".into(),
            dir_identity: "d1".into(),
            packages: Default::default(),
        };
        RemovalStore::new(&state, group_id())
            .save(&seed, &Observation::default())
            .unwrap();
        RemovalStore::new(&state, group_id())
            .observe(&input(&inv, 0, &none, "d1"))
            .unwrap();
        let first = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000, &none, "d1"))
            .unwrap();
        assert_eq!(first.held[0].1, HoldReason::RemovalObserved);
        // A new store (a restart) must still know the first sighting.
        let second = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000 + OBSERVE_GAP_MS - 1, &none, "d1"))
            .unwrap();
        assert!(second.removed.is_empty());
        let third = RemovalStore::new(&state, group_id())
            .observe(&input(&gone, 1_000 + OBSERVE_GAP_MS, &none, "d1"))
            .unwrap();
        assert_eq!(third.removed.len(), 1);
        // Corrupt files are a fresh start that judges nothing.
        state
            .write_atomic(&baseline_name(&group_id()), b"{ not json")
            .unwrap();
        let (b, o) = RemovalStore::new(&state, group_id()).load();
        assert!(b.packages.is_empty() && o.gone.is_empty());
    }

    #[test]
    fn game_running_blocks_remove_ops_but_not_absent_acks_and_waiting_items_get_none() {
        // T-R4D5-absent-ack-waits-cleanup
        let mut ledger = Ledger::new();
        let mut members = Membership::default();
        members.add(&computer(1), &enrollment(1));
        members.add(&computer(2), &enrollment(2));
        let r = wow_sync_core::removal::removal_record(
            &group_id(),
            &eref(2),
            vec![eref(1), eref(2)],
            &ObservedRemoval {
                package: "Alpha".into(),
                folders: inventory(vec![addon("Alpha", Some("1"), "a")]).folders,
                evidence: RemovalEvidence::UserAction,
            },
            5,
            1,
        );
        let bytes = encode(&r);
        ledger.learn(
            sha256(&bytes),
            AuthorityKind::Removal,
            vec![r.folders[0].key.clone()],
            Some(&computer(2)),
            Some(&bytes),
        );
        let d = derive(&ledger, &group_id(), &members);
        let g = group_id();
        let own = eref(1);
        let held = BTreeSet::new();
        let cx = CycleCx {
            group_id: &g,
            own: &own,
            ledger: &ledger,
            derived: &d,
            members: &members,
            now_unix_ms: 9,
            held: &held,
            roots: &[],
        };
        let present = inventory(vec![addon("Alpha", Some("1"), "a")]);
        assert!(step_3a(&cx, &present, 3, true).is_empty());
        assert_eq!(step_3a(&cx, &present, 3, false).len(), 1);
        let gone = inventory(vec![]);
        let items = wow_sync_core::removal::due_items(&cx, Some(&gone));
        let acks = step_3b(&cx, &items, 3);
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].result, AckResult::Absent);
        let waiting: Vec<(AckItem, CleanupClass)> = items
            .into_iter()
            .map(|(i, _)| (i, CleanupClass::WaitForCleanup))
            .collect();
        assert!(step_3b(&cx, &waiting, 3).is_empty());
    }

    #[test]
    fn commands_write_the_documented_records_and_keep_or_dismiss_locally() {
        // T-R4D4-commands
        let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
        let own = eref(1);
        let r = remove_from_group(
            &group_id(),
            &own,
            vec![eref(2)],
            "Alpha",
            &inv.folders,
            4,
            7,
        );
        assert_eq!(r.evidence, RemovalEvidence::UserAction);
        assert_eq!(r.audience, vec![eref(1), eref(2)]);
        let again = confirm_unclear(
            &group_id(),
            &own,
            vec![],
            &as_observed("Alpha", &inv.folders, RemovalEvidence::Observed),
            5,
            8,
        );
        assert_eq!(again.evidence, RemovalEvidence::UserConfirmed);
        assert_eq!(again.audience, vec![own.clone()]);
        let ledger = Ledger::new();
        let re = restore(&ledger, &group_id(), &own, &r, 6, 9);
        assert_eq!(re.evidence, ReaddEvidence::UserRestore);
        let ab = add_back(&ledger, &group_id(), &own, &r, 6, 9);
        assert_eq!(ab.evidence, ReaddEvidence::UserAddBack);
        assert!(decode::<Readd>(&encode(&ab), usize::MAX).is_ok());
        let mut base = Baseline {
            variant: "v".into(),
            dir_identity: "d".into(),
            packages: [("alpha".to_string(), inv.folders.clone())].into(),
        };
        let mut obs = Observation::default();
        obs.gone.insert("alpha".into(), (1, 2));
        keep_unclear(&mut base, &mut obs, "alpha");
        assert!(base.packages.is_empty() && obs.gone.is_empty());
        let mut l = Ledger::new();
        let h = sha256(b"x");
        continue_without(&mut l, h);
        assert!(l.dismissed().contains(&h));
    }
}
