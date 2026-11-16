//! Test builders and the in-memory effect fake. Compiled for unit tests and for dependants that
//! enable the `testkit` feature; contains no OS access.

use crate::admit::{MemberLatest, MemberView};
use crate::exec::{
    ApplyEffects, EffectError, EffectErrorKind, ObjectStat, PublishEffects, Slot, SlotObs,
};
use crate::hash::sha256;
use crate::ids::{ComputerId, EnrollmentId, GroupId, OpId, Sha256};
use crate::inventory::{build_inventory, FolderScan, Inventory, ScanStatus, ScannedFile};
use crate::layout::blob;
use crate::package::ClientInfo;
use crate::paths::FolderKey;
use crate::plan::{
    plan_apply, Activity, ApplyGate, ApplyPlan, ExpectedLive, FolderAction, FolderApply, ObjectRef,
    ObjectSource, PublishContext,
};
use crate::records::{
    decode, encode, ApplyJournal, CodeState, FolderTreeRecord, GroupRecord, JournalFolder,
    JournalPhase, MemberRecord, Record, RecoveryPoint,
};
use crate::target::{compute_target, GroupTarget, TargetInput, TreeAvail};
use crate::toc::{IdKind, TocCandidate};
use crate::variant::{
    assess_layout, confirm_variant, validate_variant, ConfirmedVariant, DirObs, FileObs,
    OtherVariants, VariantObservation, VariantVerdict, SYNTHETIC_MARKER_LINE,
};
use std::collections::{BTreeMap, VecDeque};

pub const BUILD: &str = "1.15.8.12345";

fn hex_id(n: u8) -> String {
    format!("{n:02x}").repeat(16)
}

pub fn computer(n: u8) -> ComputerId {
    ComputerId::parse(&hex_id(n)).expect("valid id")
}

pub fn enrollment(n: u8) -> EnrollmentId {
    EnrollmentId::parse(&hex_id(n)).expect("valid id")
}

pub fn op(n: u8) -> OpId {
    OpId::parse(&hex_id(n)).expect("valid id")
}

pub fn group_id() -> GroupId {
    GroupId::parse(&hex_id(0x11)).expect("valid id")
}

pub fn group_record() -> GroupRecord {
    GroupRecord {
        group_id: group_id(),
        product: "wowf".into(),
        variant_label: "Forever".into(),
        display_name: "Test group".into(),
        created_by: computer(1),
        created_unix_ms: 1,
    }
}

pub fn variant_obs(product: &str, folder: &str) -> VariantObservation {
    VariantObservation {
        folder_name: folder.to_string(),
        flavor_info: FileObs::Bytes(format!("Product!STRING:0\n{product}\n").into_bytes()),
        build_info: FileObs::Bytes(
            format!("Product!STRING:0|Active!DEC:1|Version!STRING:0\n{product}|1|{BUILD}\n")
                .into_bytes(),
        ),
        addons_dir: DirObs::PlainDir,
        synthetic_marker: FileObs::Bytes(format!("{SYNTHETIC_MARKER_LINE}\n").into_bytes()),
    }
}

pub fn variant_obs_forever() -> VariantObservation {
    variant_obs("wowf", "_forever_")
}

pub fn confirmed_forever() -> ConfirmedVariant {
    let v = validate_variant(&variant_obs_forever());
    let VariantVerdict::Recognized(id) = &v else {
        panic!("{v:?}")
    };
    let a = assess_layout(id, false, &[], &OtherVariants::default());
    confirm_variant(&v, &a, None).expect("forever confirms")
}

pub fn client(build: &str) -> ClientInfo {
    ClientInfo {
        build_version: Some(build.to_string()),
        toc_mapping: "validated-plain/1".into(),
    }
}

pub fn toc_bytes(version: Option<&str>, deps: &[&str], ids: &[(IdKind, &str)]) -> Vec<u8> {
    let mut s = String::from("## Interface: 11508\n");
    if let Some(v) = version {
        s.push_str(&format!("## Version: {v}\n"));
    }
    if !deps.is_empty() {
        s.push_str(&format!("## Dependencies: {}\n", deps.join(", ")));
    }
    for (k, v) in ids {
        let key = match k {
            IdKind::Curse => "X-Curse-Project-ID",
            IdKind::Wago => "X-Wago-ID",
            IdKind::WoWI => "X-WoWI-ID",
        };
        s.push_str(&format!("## {key}: {v}\n"));
    }
    s.into_bytes()
}

fn fingerprint_of(name: &str, files: &[ScannedFile]) -> Sha256 {
    let mut text = name.to_string();
    for f in files {
        text.push_str(&format!("|{}:{}:{}", f.rel_path, f.size, f.sha256));
    }
    sha256(text.as_bytes())
}

pub fn folder_scan(name: &str, files: &[(&str, &[u8])], tocs: &[&str]) -> FolderScan {
    let files: Vec<ScannedFile> = files
        .iter()
        .map(|(p, b)| ScannedFile {
            rel_path: (*p).to_string(),
            size: b.len() as u64,
            sha256: sha256(b),
        })
        .collect();
    FolderScan {
        name: name.to_string(),
        status: ScanStatus::Ok,
        tocs: tocs
            .iter()
            .map(|t| TocCandidate {
                file_name: (*t).to_string(),
                contents: FileObs::Bytes(b"## Interface: 11508\n".to_vec()),
            })
            .collect(),
        fingerprint: Some(fingerprint_of(name, &files)),
        files,
    }
}

pub fn addon_full(
    name: &str,
    version: Option<&str>,
    deps: &[&str],
    ids: &[(IdKind, &str)],
    tag: &str,
) -> FolderScan {
    let toc = toc_bytes(version, deps, ids);
    let toc_name = format!("{name}.toc");
    let mut scan = folder_scan(
        name,
        &[
            ("main.lua", tag.as_bytes()),
            (toc_name.as_str(), toc.as_slice()),
        ],
        &[],
    );
    scan.tocs = vec![TocCandidate {
        file_name: toc_name,
        contents: FileObs::Bytes(toc),
    }];
    scan
}

pub fn addon(name: &str, version: Option<&str>, tag: &str) -> FolderScan {
    addon_full(name, version, &[], &[], tag)
}

/// A folder whose only `.toc` carries a client suffix that no validated mapping covers.
pub fn addon_unvalidated(name: &str, version: &str, tag: &str) -> FolderScan {
    let toc = toc_bytes(Some(version), &[], &[]);
    let toc_name = format!("{name}_Foreign.toc");
    let mut scan = folder_scan(
        name,
        &[
            ("main.lua", tag.as_bytes()),
            (toc_name.as_str(), toc.as_slice()),
        ],
        &[],
    );
    scan.tocs = vec![TocCandidate {
        file_name: toc_name,
        contents: FileObs::Bytes(toc),
    }];
    scan
}

pub fn inventory(scans: Vec<FolderScan>) -> Inventory {
    build_inventory(scans, true, &confirmed_forever())
}

fn n_of(c: &ComputerId) -> u8 {
    u8::from_str_radix(&c.as_str()[..2], 16).expect("testkit computer id")
}

pub fn member_record(c: &ComputerId) -> MemberRecord {
    MemberRecord {
        group_id: group_id(),
        computer_id: c.clone(),
        enrollment_id: enrollment(n_of(c)),
        display_name: format!("PC{}", n_of(c)),
        product: "wowf".into(),
        variant_folder: "_forever_".into(),
        enrolled_unix_ms: 1,
        app_version: "0.0.0-test".into(),
        retired: None,
    }
}

/// As decoded from the provider: local-only inventory fields are absent.
pub fn member_state(c: &ComputerId, seq: u64, inv: &Inventory) -> CodeState {
    let mut folders = inv.folders.clone();
    for f in &mut folders {
        f.fingerprint = None;
        f.file_entries.clear();
    }
    CodeState {
        group_id: group_id(),
        computer_id: c.clone(),
        enrollment_id: enrollment(n_of(c)),
        seq,
        product: "wowf".into(),
        app_version: "0.0.0-test".into(),
        written_unix_ms: 1_000,
        client: client(BUILD),
        inventory_complete: inv
            .folders
            .iter()
            .all(|f| f.status == crate::inventory::FolderStatus::Ok),
        folders,
        apply_report: None,
    }
}

pub fn sample_journal(phase: JournalPhase) -> ApplyJournal {
    let from = (phase == JournalPhase::RolledBack).then_some(JournalPhase::Committing);
    ApplyJournal {
        op: op(9),
        group_id: group_id(),
        started_unix_ms: 5,
        package: FolderKey::lossy("suite"),
        target_hash: sha256(b"t"),
        phase,
        from_phase: from,
        reason: None,
        folders: vec![
            JournalFolder {
                name: "Suite".into(),
                key: FolderKey::lossy("suite"),
                action: FolderAction::Replace,
                target_tree: sha256(b"new"),
                expected_live: ExpectedLive::Tree {
                    tree: sha256(b"old"),
                    fingerprint: sha256(b"fp"),
                },
            },
            JournalFolder {
                name: "Suite_Data".into(),
                key: FolderKey::lossy("suite_data"),
                action: FolderAction::Install,
                target_tree: sha256(b"new2"),
                expected_live: ExpectedLive::Absent,
            },
        ],
    }
}

pub struct OpGen(u8);

impl OpGen {
    pub fn new() -> Self {
        Self(0x20)
    }
    pub fn next_op(&mut self) -> OpId {
        self.0 += 1;
        op(self.0)
    }
}

impl Default for OpGen {
    fn default() -> Self {
        Self::new()
    }
}

pub fn gate(activity: Activity) -> ApplyGate {
    ApplyGate {
        activity,
        variant: confirmed_forever(),
        any_journal: false,
        now_unix_ms: 1_000,
    }
}

/// This computer is `computer(1)`; peers are added with `peer`.
#[derive(Clone)]
pub struct Scenario {
    pub local: Inventory,
    pub local_client: ClientInfo,
    pub members: Vec<MemberView>,
    pub trees: BTreeMap<Sha256, TreeAvail>,
}

impl Scenario {
    pub fn new(local: Vec<FolderScan>) -> Self {
        Self {
            local: inventory(local),
            local_client: client(BUILD),
            members: Vec::new(),
            trees: BTreeMap::new(),
        }
    }

    pub fn peer(self, n: u8, scans: Vec<FolderScan>) -> Self {
        self.peer_client(n, scans, client(BUILD))
    }

    pub fn peer_client(mut self, n: u8, scans: Vec<FolderScan>, c: ClientInfo) -> Self {
        let inv = inventory(scans);
        for f in &inv.folders {
            if let Some(t) = f.tree {
                self.trees.insert(
                    t,
                    TreeAvail::Ok(FolderTreeRecord {
                        tree: t,
                        files: f.file_entries.clone(),
                    }),
                );
            }
        }
        let mut st = member_state(&computer(n), 1, &inv);
        st.client = c;
        self.members.push(MemberView {
            computer: computer(n),
            enrollment: enrollment(n),
            display_name: format!("PC{n}"),
            latest: MemberLatest::Valid {
                seq: 1,
                state: Box::new(st),
            },
        });
        self
    }

    pub fn map_state(mut self, n: u8, f: impl FnOnce(&mut CodeState)) -> Self {
        let m = self
            .members
            .iter_mut()
            .find(|m| m.computer == computer(n))
            .expect("peer exists");
        let MemberLatest::Valid { state, .. } = &mut m.latest else {
            panic!("peer state is not valid")
        };
        f(state);
        self
    }

    pub fn written(self, n: u8, ms: u64) -> Self {
        self.map_state(n, |s| s.written_unix_ms = ms)
    }

    pub fn make_unusable(&mut self, n: u8) {
        let m = self
            .members
            .iter_mut()
            .find(|m| m.computer == computer(n))
            .expect("peer exists");
        m.latest = MemberLatest::Unusable {
            seq: 1,
            reason: "malformed".into(),
        };
    }

    pub fn peer_tree(&self, n: u8, key: &str) -> Sha256 {
        let m = self
            .members
            .iter()
            .find(|m| m.computer == computer(n))
            .expect("peer exists");
        let MemberLatest::Valid { state, .. } = &m.latest else {
            panic!("peer state is not valid")
        };
        state
            .folders
            .iter()
            .find(|f| f.key == FolderKey::lossy(key))
            .and_then(|f| f.tree)
            .expect("folder has a tree")
    }

    pub fn target(&self) -> GroupTarget {
        compute_target(&TargetInput {
            group: &group_record(),
            variant: &confirmed_forever(),
            local: &self.local,
            local_client: &self.local_client,
            local_id: &computer(1),
            members: &self.members,
            trees: &self.trees,
        })
    }

    pub fn publish_ctx(&self, seq: u64, own_files: &[&str]) -> PublishContext {
        PublishContext {
            group: group_id(),
            computer: computer(1),
            enrollment: enrollment(0xA0),
            product: "wowf".into(),
            app_version: "0.0.0-test".into(),
            now_unix_ms: 1_000,
            seq,
            client: self.local_client.clone(),
            apply_report: None,
            own_state_files: own_files
                .iter()
                .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
                .collect(),
        }
    }
}

/// Two packages (Alpha+Alpha_Data, Beta+Beta_Data); each has one replace and one install folder.
pub fn apply_fixture() -> (Scenario, ApplyPlan, FakeEffects) {
    let local = vec![
        addon("Alpha", Some("1.0"), "a1"),
        addon("Beta", Some("1.0"), "b1"),
        addon("Extra", Some("1.0"), "x"),
    ];
    let peer = vec![
        addon("Alpha", Some("2.0"), "a2"),
        addon("Alpha_Data", Some("2.0"), "ad2"),
        addon("Beta", Some("2.0"), "b2"),
        addon("Beta_Data", Some("2.0"), "bd2"),
    ];
    let sc = Scenario::new(local).peer(2, peer);
    let mut ops = OpGen::new();
    let plan = plan_apply(
        &sc.target(),
        &sc.local,
        &mut || ops.next_op(),
        &gate(Activity::NotRunning),
    )
    .expect("fixture plans");
    let mut fx = FakeEffects::new();
    fx.set_live_from(&sc.local);
    fx.provide_plan_blobs(&plan);
    (sc, plan, fx)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Activity,
    Lock,
    WriteJournal,
    ReadJournals,
    RemoveJournal,
    StageFolder,
    TreeOf,
    Observe,
    Rename,
    WriteRecovery,
    ListRecovery,
    RemoveRecovery,
    DiscardStaging,
    StatObject,
    HashObject,
    PutObject,
    ObserveSource,
    WriteRecord,
    RemoveOwnRecord,
}

impl Step {
    fn mutating(self) -> bool {
        matches!(
            self,
            Step::WriteJournal
                | Step::RemoveJournal
                | Step::StageFolder
                | Step::Rename
                | Step::WriteRecovery
                | Step::RemoveRecovery
                | Step::DiscardStaging
                | Step::PutObject
                | Step::WriteRecord
                | Step::RemoveOwnRecord
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Activity(bool),
    Rename { from: Slot, to: Slot },
    WriteJournal(JournalPhase),
    RemoveJournal,
    StageFolder,
    WriteRecovery { pinned: bool },
    RemoveRecovery,
    DiscardStaging,
    Other(Step),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dir {
    pub tree: Sha256,
    pub fingerprint: Sha256,
    pub recent: bool,
}

#[derive(Debug, Clone)]
pub struct FakeObj {
    pub hash: Sha256,
    pub corrupt: bool,
    pub cloud_only: bool,
}

#[derive(Debug, Clone)]
pub enum Mutation {
    LiveTree(String, Option<Sha256>),
    LiveFingerprint(String),
    LiveRecent(String),
    RecoveryTree(String, Sha256),
    CorruptObject(String),
}

#[derive(Clone)]
pub struct FakeEffects {
    pub group: GroupId,
    pub live: BTreeMap<String, Dir>,
    pub staging: BTreeMap<(String, String), Dir>,
    pub recovery: BTreeMap<(String, String, String), Dir>,
    pub points: BTreeMap<(String, String), Vec<u8>>,
    pub journals: BTreeMap<String, Vec<u8>>,
    pub objects: BTreeMap<String, FakeObj>,
    pub records: BTreeMap<String, Vec<u8>>,
    pub log: Vec<Effect>,
    pub activity_script: VecDeque<Activity>,
    /// Report `Running` once this many renames have happened.
    pub launch_after_renames: Option<usize>,
    pub fail_on: Option<(Step, usize, EffectErrorKind)>,
    pub mutate_at: Option<(Step, usize, Mutation)>,
    pub crash_after: Option<usize>,
    pub stage_tree_override: Option<Sha256>,
    pub mutations: usize,
    pub renames: usize,
    pub crashed: bool,
    counts: BTreeMap<Step, usize>,
}

impl Default for FakeEffects {
    fn default() -> Self {
        Self::new()
    }
}

fn live_slot(f: &str) -> Slot {
    Slot::Live {
        folder: f.to_string(),
    }
}

impl FakeEffects {
    pub fn new() -> Self {
        Self {
            group: group_id(),
            live: BTreeMap::new(),
            staging: BTreeMap::new(),
            recovery: BTreeMap::new(),
            points: BTreeMap::new(),
            journals: BTreeMap::new(),
            objects: BTreeMap::new(),
            records: BTreeMap::new(),
            log: Vec::new(),
            activity_script: VecDeque::new(),
            launch_after_renames: None,
            fail_on: None,
            mutate_at: None,
            crash_after: None,
            stage_tree_override: None,
            mutations: 0,
            renames: 0,
            crashed: false,
            counts: BTreeMap::new(),
        }
    }

    pub fn set_live_from(&mut self, inv: &Inventory) {
        for f in inv.folders.iter().filter(|f| f.tree.is_some()) {
            let fingerprint = f.fingerprint.unwrap_or_else(|| sha256(f.name.as_bytes()));
            self.live.insert(
                f.name.clone(),
                Dir {
                    tree: f.tree.expect("checked"),
                    fingerprint,
                    recent: false,
                },
            );
        }
    }

    pub fn provide_plan_blobs(&mut self, plan: &ApplyPlan) {
        for p in &plan.packages {
            for f in &p.folders {
                for e in &f.tree.files {
                    self.objects.insert(
                        blob(&self.group, &e.sha256),
                        FakeObj {
                            hash: e.sha256,
                            corrupt: false,
                            cloud_only: false,
                        },
                    );
                }
            }
        }
    }

    pub fn restart(&mut self) {
        self.crashed = false;
        self.crash_after = None;
        self.mutations = 0;
    }

    pub fn count(&self, s: Step) -> usize {
        self.counts.get(&s).copied().unwrap_or(0)
    }

    pub fn live_trees(&self) -> BTreeMap<String, Sha256> {
        self.live.iter().map(|(k, d)| (k.clone(), d.tree)).collect()
    }

    pub fn journal_phase(&self, op: &OpId) -> Option<JournalPhase> {
        self.journals
            .get(op.as_str())
            .and_then(|b| decode::<ApplyJournal>(b, ApplyJournal::LIMIT).ok())
            .map(|j| j.phase)
    }

    pub fn journal(&self, op: &OpId) -> Option<ApplyJournal> {
        self.journals
            .get(op.as_str())
            .and_then(|b| decode::<ApplyJournal>(b, ApplyJournal::LIMIT).ok())
    }

    pub fn points_for(&self, pkg: &FolderKey) -> Vec<RecoveryPoint> {
        self.points
            .iter()
            .filter(|((p, _), _)| p == pkg.as_str())
            .filter_map(|(_, b)| decode::<RecoveryPoint>(b, RecoveryPoint::LIMIT).ok())
            .collect()
    }

    pub fn renames_logged(&self) -> Vec<(Slot, Slot)> {
        self.log
            .iter()
            .filter_map(|e| {
                if let Effect::Rename { from, to } = e {
                    Some((from.clone(), to.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn dir(&self, s: &Slot) -> Option<&Dir> {
        match s {
            Slot::Live { folder } => self.live.get(folder),
            Slot::Staging { op, folder } => self.staging.get(&(op.to_string(), folder.clone())),
            Slot::Recovery { pkg, op, folder } => {
                self.recovery
                    .get(&(pkg.to_string(), op.to_string(), folder.clone()))
            }
        }
    }

    fn take(&mut self, s: &Slot) -> Option<Dir> {
        match s {
            Slot::Live { folder } => self.live.remove(folder),
            Slot::Staging { op, folder } => self.staging.remove(&(op.to_string(), folder.clone())),
            Slot::Recovery { pkg, op, folder } => {
                self.recovery
                    .remove(&(pkg.to_string(), op.to_string(), folder.clone()))
            }
        }
    }

    fn put(&mut self, s: &Slot, d: Dir) {
        match s {
            Slot::Live { folder } => self.live.insert(folder.clone(), d),
            Slot::Staging { op, folder } => {
                self.staging.insert((op.to_string(), folder.clone()), d)
            }
            Slot::Recovery { pkg, op, folder } => self
                .recovery
                .insert((pkg.to_string(), op.to_string(), folder.clone()), d),
        };
    }

    fn apply_mutation(&mut self, m: &Mutation) {
        match m {
            Mutation::LiveTree(f, Some(t)) => {
                self.live.insert(
                    f.clone(),
                    Dir {
                        tree: *t,
                        fingerprint: sha256(&[&t.0[..], b"ext"].concat()),
                        recent: false,
                    },
                );
            }
            Mutation::LiveTree(f, None) => {
                self.live.remove(f);
            }
            Mutation::LiveFingerprint(f) => {
                if let Some(d) = self.live.get_mut(f) {
                    d.fingerprint = sha256(&[&d.fingerprint.0[..], b"bump"].concat());
                }
            }
            Mutation::LiveRecent(f) => {
                if let Some(d) = self.live.get_mut(f) {
                    d.recent = true;
                }
            }
            Mutation::RecoveryTree(f, t) => {
                for ((_, _, folder), d) in self.recovery.iter_mut() {
                    if folder == f {
                        d.tree = *t;
                    }
                }
            }
            Mutation::CorruptObject(rel) => {
                if let Some(o) = self.objects.get_mut(rel) {
                    o.corrupt = true;
                }
            }
        }
    }

    fn enter(&mut self, step: Step) -> Result<(), EffectError> {
        let n = {
            let c = self.counts.entry(step).or_default();
            *c += 1;
            *c
        };
        if self.crashed {
            return Err(EffectError::new(EffectErrorKind::Crashed, "crashed"));
        }
        if let Some((s, k, m)) = self.mutate_at.clone() {
            if s == step && k == n {
                self.apply_mutation(&m);
                self.mutate_at = None;
            }
        }
        if step.mutating() {
            if let Some(k) = self.crash_after {
                if self.mutations >= k {
                    self.crashed = true;
                    return Err(EffectError::new(EffectErrorKind::Crashed, "crashed"));
                }
            }
        }
        if let Some((s, k, kind)) = self.fail_on {
            if s == step && k == n {
                return Err(EffectError::new(kind, "injected"));
            }
        }
        if step.mutating() {
            self.mutations += 1;
        }
        Ok(())
    }
}

impl ApplyEffects for FakeEffects {
    fn activity(&mut self) -> Activity {
        *self.counts.entry(Step::Activity).or_default() += 1;
        let a = if self.crashed {
            Activity::Unknown("crashed".into())
        } else if self.launch_after_renames.is_some_and(|n| self.renames >= n) {
            Activity::Running { pids: vec![1] }
        } else if self.activity_script.len() > 1 {
            self.activity_script.pop_front().expect("non-empty")
        } else {
            self.activity_script
                .front()
                .cloned()
                .unwrap_or(Activity::NotRunning)
        };
        self.log.push(Effect::Activity(a == Activity::NotRunning));
        a
    }

    fn lock(&mut self) -> Result<(), EffectError> {
        self.enter(Step::Lock)
    }

    fn write_journal(&mut self, j: &ApplyJournal) -> Result<(), EffectError> {
        self.enter(Step::WriteJournal)?;
        self.log.push(Effect::WriteJournal(j.phase));
        self.journals.insert(j.op.to_string(), encode(j));
        Ok(())
    }

    fn read_journals(&mut self) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.enter(Step::ReadJournals)?;
        Ok(self
            .journals
            .iter()
            .map(|(k, v)| (format!("{k}.json"), v.clone()))
            .collect())
    }

    fn remove_journal(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.enter(Step::RemoveJournal)?;
        self.log.push(Effect::RemoveJournal);
        self.journals.remove(op.as_str());
        Ok(())
    }

    fn stage_folder(&mut self, op: &OpId, f: &FolderApply) -> Result<(), EffectError> {
        self.enter(Step::StageFolder)?;
        self.log.push(Effect::StageFolder);
        for e in &f.tree.files {
            match self.objects.get(&blob(&self.group, &e.sha256)) {
                None => return Err(EffectError::new(EffectErrorKind::NotFound, "blob missing")),
                Some(o) if o.cloud_only => {
                    return Err(EffectError::new(
                        EffectErrorKind::CloudOnly,
                        "blob not downloaded",
                    ))
                }
                Some(o) if o.corrupt || o.hash != e.sha256 => {
                    return Err(EffectError::new(
                        EffectErrorKind::HashMismatch,
                        "blob corrupt",
                    ))
                }
                Some(_) => {}
            }
        }
        let tree = self.stage_tree_override.unwrap_or(f.tree.tree);
        self.staging.insert(
            (op.to_string(), f.name.clone()),
            Dir {
                tree,
                fingerprint: sha256(&tree.0),
                recent: false,
            },
        );
        Ok(())
    }

    fn tree_of(&mut self, slot: &Slot) -> Result<Option<Sha256>, EffectError> {
        self.enter(Step::TreeOf)?;
        Ok(self.dir(slot).map(|d| d.tree))
    }

    fn observe(&mut self, slot: &Slot) -> Result<Option<SlotObs>, EffectError> {
        self.enter(Step::Observe)?;
        Ok(self.dir(slot).map(|d| SlotObs {
            fingerprint: d.fingerprint,
            recently_modified: d.recent,
        }))
    }

    fn rename(&mut self, from: &Slot, to: &Slot) -> Result<(), EffectError> {
        self.enter(Step::Rename)?;
        self.log.push(Effect::Rename {
            from: from.clone(),
            to: to.clone(),
        });
        if self.dir(from).is_none() {
            return Err(EffectError::new(EffectErrorKind::NotFound, "source absent"));
        }
        if self.dir(to).is_some() {
            return Err(EffectError::new(
                EffectErrorKind::Exists,
                "destination exists",
            ));
        }
        let d = self.take(from).expect("checked");
        self.put(to, d);
        self.renames += 1;
        Ok(())
    }

    fn write_recovery_point(&mut self, rp: &RecoveryPoint) -> Result<(), EffectError> {
        self.enter(Step::WriteRecovery)?;
        self.log.push(Effect::WriteRecovery { pinned: rp.pinned });
        self.points
            .insert((rp.package.to_string(), rp.op.to_string()), encode(rp));
        Ok(())
    }

    fn list_recovery(&mut self, pkg: &FolderKey) -> Result<Vec<(String, Vec<u8>)>, EffectError> {
        self.enter(Step::ListRecovery)?;
        Ok(self
            .points
            .iter()
            .filter(|((p, _), _)| p == pkg.as_str())
            .map(|((_, o), b)| (format!("{o}.json"), b.clone()))
            .collect())
    }

    fn remove_recovery(&mut self, pkg: &FolderKey, op: &OpId) -> Result<(), EffectError> {
        self.enter(Step::RemoveRecovery)?;
        self.log.push(Effect::RemoveRecovery);
        self.points.remove(&(pkg.to_string(), op.to_string()));
        self.recovery
            .retain(|(p, o, _), _| !(p == pkg.as_str() && o == op.as_str()));
        Ok(())
    }

    fn discard_staging(&mut self, op: &OpId) -> Result<(), EffectError> {
        self.enter(Step::DiscardStaging)?;
        self.log.push(Effect::DiscardStaging);
        self.staging.retain(|(o, _), _| o != op.as_str());
        Ok(())
    }
}

impl PublishEffects for FakeEffects {
    fn stat_object(&mut self, rel: &str) -> Result<ObjectStat, EffectError> {
        self.enter(Step::StatObject)?;
        Ok(match self.objects.get(rel) {
            None => ObjectStat::Missing,
            Some(o) if o.cloud_only => ObjectStat::CloudOnly,
            Some(_) => ObjectStat::Local {
                size: 1,
                mtime_ns: 1,
            },
        })
    }

    fn hash_object(&mut self, rel: &str) -> Result<Sha256, EffectError> {
        self.enter(Step::HashObject)?;
        match self.objects.get(rel) {
            None => Err(EffectError::new(EffectErrorKind::NotFound, rel)),
            Some(o) if o.corrupt => Ok(sha256(b"corrupt")),
            Some(o) => Ok(o.hash),
        }
    }

    fn put_object(&mut self, obj: &ObjectRef) -> Result<(u64, i128), EffectError> {
        self.enter(Step::PutObject)?;
        self.log.push(Effect::Other(Step::PutObject));
        if let ObjectSource::Folder { folder, .. } = &obj.source {
            if !self.live.contains_key(folder) {
                return Err(EffectError::new(
                    EffectErrorKind::NotFound,
                    "source folder gone",
                ));
            }
        }
        self.objects.insert(
            obj.rel.clone(),
            FakeObj {
                hash: obj.expect,
                corrupt: false,
                cloud_only: false,
            },
        );
        Ok((1, 1))
    }

    fn observe_source(&mut self, folder: &str) -> Result<Option<SlotObs>, EffectError> {
        self.enter(Step::ObserveSource)?;
        Ok(self.live.get(folder).map(|d| SlotObs {
            fingerprint: d.fingerprint,
            recently_modified: d.recent,
        }))
    }

    fn write_record(&mut self, rel: &str, bytes: &[u8]) -> Result<(), EffectError> {
        self.enter(Step::WriteRecord)?;
        self.log.push(Effect::Other(Step::WriteRecord));
        self.records.insert(rel.to_string(), bytes.to_vec());
        Ok(())
    }

    fn remove_own_record(&mut self, rel: &str) -> Result<(), EffectError> {
        self.enter(Step::RemoveOwnRecord)?;
        self.log.push(Effect::Other(Step::RemoveOwnRecord));
        self.records.remove(rel);
        Ok(())
    }
}

pub fn live_of(fx: &FakeEffects, names: &[&str]) -> Vec<Option<Sha256>> {
    names
        .iter()
        .map(|n| fx.dir(&live_slot(n)).map(|d| d.tree))
        .collect()
}
