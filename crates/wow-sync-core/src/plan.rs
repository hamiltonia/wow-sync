//! Publish and apply planning (core-api.md §6). Pure: no effects.

use crate::explain::Explanation;
use crate::hash::sha256;
use crate::ids::{ComputerId, EnrollmentId, GroupId, OpId, Sha256};
use crate::inventory::{FolderStatus, Inventory};
use crate::layout::{blob, parse_state_name, state_record, tree_record};
use crate::package::ClientInfo;
use crate::paths::{FolderKey, RelPath};
use crate::records::{
    encode, ApplyReport, AuthorityIndexHead, CodeState, FolderTreeRecord, OpKind, PackageLineage,
    PendingRecord, SettingsFileOp, SettingsRoot,
};
use crate::target::{GroupTarget, PackageTarget};
use crate::variant::ConfirmedVariant;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const KEEP_OWN_STATES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activity {
    NotRunning,
    Running { pids: Vec<u32> },
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanBlock {
    GameRunning,
    ActivityUnknown(String),
    Unstable,
    UnresolvedJournal,
    NothingToDo,
    DestinationChanged,
    RenameFailed(String),
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Blob,
    Tree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectSource {
    /// A file inside a live AddOns folder.
    Folder {
        folder: String,
        path: RelPath,
    },
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRef {
    pub rel: String,
    pub kind: ObjectKind,
    /// SHA-256 of the object's bytes.
    pub expect: Sha256,
    pub source: ObjectSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishContext {
    pub group: GroupId,
    pub computer: ComputerId,
    pub enrollment: EnrollmentId,
    pub product: String,
    pub app_version: String,
    pub now_unix_ms: u64,
    /// Already persisted by the runtime; must be greater than every own state number.
    pub seq: u64,
    pub client: ClientInfo,
    pub apply_report: Option<ApplyReport>,
    pub authority_index: Option<AuthorityIndexHead>,
    pub features: Vec<String>,
    /// The package ancestry to publish (`lineage::next_lineage`).
    pub lineage: Vec<PackageLineage>,
    pub own_state_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPlan {
    pub objects: Vec<ObjectRef>,
    pub state: CodeState,
    pub state_path: String,
    pub prune_states: Vec<String>,
    pub source_fingerprints: Vec<(String, Sha256)>,
}

/// The objects a publication of `inv` needs, folder by folder: each folder's new blobs, then its
/// tree record. Built one at a time so a large install never holds every object at once; a blob
/// or tree already produced for an earlier folder is not produced again.
pub fn publish_objects<'a>(
    inv: &'a Inventory,
    group: &'a GroupId,
) -> impl Iterator<Item = ObjectRef> + 'a {
    let mut seen: std::collections::HashSet<(bool, Sha256)> = std::collections::HashSet::new();
    inv.folders
        .iter()
        .filter(|f| f.transfer_eligible())
        .filter_map(|f| f.tree.map(|t| (f, t)))
        .flat_map(move |(f, tree)| {
            let mut out: Vec<ObjectRef> = Vec::new();
            for e in f.file_entries.iter() {
                if seen.insert((false, e.sha256)) {
                    out.push(ObjectRef {
                        rel: blob(group, &e.sha256),
                        kind: ObjectKind::Blob,
                        expect: e.sha256,
                        source: ObjectSource::Folder {
                            folder: f.name.clone(),
                            path: e.path.clone(),
                        },
                    });
                }
            }
            let mut bytes = encode(&FolderTreeRecord {
                tree,
                files: f.file_entries.to_vec(),
            });
            bytes.shrink_to_fit();
            if seen.insert((true, tree)) {
                out.push(ObjectRef {
                    rel: tree_record(group, &tree),
                    kind: ObjectKind::Tree,
                    expect: sha256(&bytes),
                    source: ObjectSource::Bytes(bytes),
                });
            }
            out
        })
}

/// How many objects `publish_objects` yields, without keeping any of them.
pub fn count_publish_objects(inv: &Inventory) -> u64 {
    let mut seen: std::collections::HashSet<(bool, Sha256)> = std::collections::HashSet::new();
    for f in inv.folders.iter().filter(|f| f.transfer_eligible()) {
        let Some(tree) = f.tree else { continue };
        for e in f.file_entries.iter() {
            seen.insert((false, e.sha256));
        }
        seen.insert((true, tree));
    }
    seen.len() as u64
}

/// The plan without its objects (`publish_objects`) and with a state that carries no local file
/// lists (they are never serialised), so a large install is not copied for a publication.
pub fn plan_publish_state(inv: &Inventory, ctx: &PublishContext) -> Result<PublishPlan, PlanBlock> {
    plan_publish_inner(inv, ctx, false)
}

pub fn plan_publish(inv: &Inventory, ctx: &PublishContext) -> Result<PublishPlan, PlanBlock> {
    plan_publish_inner(inv, ctx, true)
}

fn plan_publish_inner(
    inv: &Inventory,
    ctx: &PublishContext,
    with_objects: bool,
) -> Result<PublishPlan, PlanBlock> {
    if !inv.listing_stable {
        return Err(PlanBlock::Unstable);
    }
    let own: Vec<u64> = ctx
        .own_state_files
        .iter()
        .filter_map(|n| parse_state_name(n))
        .filter(|(e, _)| *e == ctx.enrollment)
        .map(|(_, s)| s)
        .collect();
    if own.iter().any(|s| *s >= ctx.seq) {
        return Err(PlanBlock::Invalid(
            "the next state number must be greater than every existing one".into(),
        ));
    }
    let mut objects = Vec::new();
    if with_objects {
        let mut blobs: BTreeMap<String, ObjectRef> = BTreeMap::new();
        let mut trees: BTreeMap<String, ObjectRef> = BTreeMap::new();
        for o in publish_objects(inv, &ctx.group) {
            match o.kind {
                ObjectKind::Blob => blobs.insert(o.rel.clone(), o),
                ObjectKind::Tree => trees.insert(o.rel.clone(), o),
            };
        }
        objects = blobs.into_values().chain(trees.into_values()).collect();
    }
    let fingerprints: Vec<(String, Sha256)> = inv
        .folders
        .iter()
        .filter(|f| f.transfer_eligible() && f.tree.is_some())
        .filter_map(|f| f.fingerprint.map(|fp| (f.name.clone(), fp)))
        .collect();
    let state = CodeState {
        group_id: ctx.group.clone(),
        computer_id: ctx.computer.clone(),
        enrollment_id: ctx.enrollment.clone(),
        seq: ctx.seq,
        product: ctx.product.clone(),
        app_version: ctx.app_version.clone(),
        written_unix_ms: ctx.now_unix_ms,
        client: ctx.client.clone(),
        inventory_complete: inv.folders.iter().all(|f| f.transfer_eligible()),
        folders: if with_objects {
            inv.folders.clone()
        } else {
            inv.folders.iter().map(|f| f.without_entries()).collect()
        },
        apply_report: ctx.apply_report.clone(),
        authority_index: ctx.authority_index,
        features: ctx.features.clone(),
        lineage: ctx.lineage.clone(),
    };
    let mut seqs = own;
    seqs.push(ctx.seq);
    seqs.sort_unstable_by(|a, b| b.cmp(a));
    let prune_states = seqs
        .iter()
        .skip(KEEP_OWN_STATES)
        .map(|s| state_record(&ctx.group, &ctx.computer, &ctx.enrollment, *s))
        .collect();
    Ok(PublishPlan {
        objects,
        state_path: state_record(&ctx.group, &ctx.computer, &ctx.enrollment, ctx.seq),
        state,
        prune_states,
        source_fingerprints: fingerprints,
    })
}

/// Which own state is the newest of this enrollment (compared against the inventory) and the
/// sequence number of the next one: above the persisted counter and above every own state
/// file of any enrollment, so a number is never reused (publication.md §4 step 6).
pub fn own_publish_slot(
    own_state_files: &[String],
    enrollment: &EnrollmentId,
    stored_next_seq: u64,
) -> (Option<String>, u64) {
    let parsed: Vec<(EnrollmentId, u64, &String)> = own_state_files
        .iter()
        .filter_map(|n| parse_state_name(n).map(|(e, s)| (e, s, n)))
        .collect();
    let newest = parsed
        .iter()
        .filter(|(e, _, _)| e == enrollment)
        .max_by_key(|(_, s, _)| *s)
        .map(|(_, _, n)| (*n).clone());
    let highest = parsed.iter().map(|(_, s, _)| *s).max();
    (newest, stored_next_seq.max(highest.map_or(1, |s| s + 1)))
}

/// Whether a new state must be published (M3): there is an apply report to carry, no previous
/// state of this enrollment is readable, or the previous state differs from the inventory in any
/// folder, status, tree, toc outcome or completeness, or the authority index head or the
/// capability list changed.
pub fn publish_needed(
    previous: Option<&CodeState>,
    inv: &Inventory,
    has_apply_report: bool,
    index: Option<&AuthorityIndexHead>,
    features: &[String],
) -> bool {
    if has_apply_report {
        return true;
    }
    let Some(st) = previous else { return true };
    if st.authority_index.as_ref() != index || st.features != features {
        return true;
    }
    let same = st.folders.len() == inv.folders.len()
        && st.folders.iter().zip(&inv.folders).all(|(a, b)| {
            a.key == b.key && a.status == b.status && a.tree == b.tree && a.toc.kind == b.toc.kind
        })
        && st.inventory_complete == inv.folders.iter().all(|f| f.transfer_eligible());
    !same
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderAction {
    Install,
    Replace,
    /// The live folder moves to its recovery slot and nothing is installed (`apply-journal/2`).
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpectedLive {
    Absent,
    Tree { tree: Sha256, fingerprint: Sha256 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderApply {
    pub name: String,
    pub key: FolderKey,
    pub action: FolderAction,
    pub tree: FolderTreeRecord,
    pub expected_live: ExpectedLive,
}

/// A live folder this operation moves to recovery (action `remove`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveApply {
    pub name: String,
    pub key: FolderKey,
    pub expected_live: ExpectedLive,
}

/// One package = one operation (apply.md §2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageApply {
    pub op: OpId,
    pub package: FolderKey,
    pub target_hash: Sha256,
    pub folders: Vec<FolderApply>,
    pub explanation: Explanation,
    pub kind: OpKind,
    pub removes: Vec<RemoveApply>,
    pub removals: Vec<Sha256>,
    pub settings_ops: Vec<SettingsFileOp>,
    pub pending_records: Vec<PendingRecord>,
    /// Every settings unit this operation applies to, captured or missing (contracts.md §2.3).
    pub settings_roots: Vec<SettingsRoot>,
}

impl PackageApply {
    /// A pure code apply, as the slice planner builds it.
    pub fn code_apply(
        op: OpId,
        package: FolderKey,
        target_hash: Sha256,
        folders: Vec<FolderApply>,
        explanation: Explanation,
    ) -> Self {
        Self {
            op,
            package,
            target_hash,
            folders,
            explanation,
            kind: OpKind::Apply,
            removes: Vec::new(),
            removals: Vec::new(),
            settings_ops: Vec::new(),
            pending_records: Vec::new(),
            settings_roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPlan {
    pub group_id: GroupId,
    pub started_unix_ms: u64,
    pub packages: Vec<PackageApply>,
    /// Chosen packages left out this cycle, with the reason.
    pub skipped: Vec<(FolderKey, Explanation)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyGate {
    pub activity: Activity,
    pub variant: ConfirmedVariant,
    /// Q-25: any journal of any operation blocks new applies.
    pub any_journal: bool,
    pub now_unix_ms: u64,
}

pub fn plan_apply(
    target: &GroupTarget,
    local: &Inventory,
    new_op: &mut dyn FnMut() -> OpId,
    gate: &ApplyGate,
) -> Result<ApplyPlan, PlanBlock> {
    match &gate.activity {
        Activity::Running { .. } => return Err(PlanBlock::GameRunning),
        Activity::Unknown(why) => return Err(PlanBlock::ActivityUnknown(why.clone())),
        Activity::NotRunning => {}
    }
    if gate.any_journal {
        return Err(PlanBlock::UnresolvedJournal);
    }
    if !local.listing_stable {
        return Err(PlanBlock::Unstable);
    }
    let mut plan = ApplyPlan {
        group_id: target.group_id.clone(),
        started_unix_ms: gate.now_unix_ms,
        packages: Vec::new(),
        skipped: Vec::new(),
    };
    for (key, t) in &target.packages {
        let PackageTarget::Chosen { version, why, .. } = t else {
            continue;
        };
        let mut folders = Vec::new();
        let mut blocked = None;
        for f in &version.folders {
            let Some(tree) = f.tree else {
                blocked = Some(Explanation::NoCompatibleToc {
                    folder: f.name.clone(),
                });
                break;
            };
            let rec = FolderTreeRecord {
                tree,
                files: f.file_entries.to_vec(),
            };
            if crate::hash::tree_hash(&rec.files) != tree {
                blocked = Some(Explanation::NotAvailableYet {
                    missing: vec![tree],
                });
                break;
            }
            match local.get(&f.key) {
                None => folders.push(FolderApply {
                    name: f.name.clone(),
                    key: f.key.clone(),
                    action: FolderAction::Install,
                    tree: rec,
                    expected_live: ExpectedLive::Absent,
                }),
                Some(lf) if lf.tree == Some(tree) => {}
                Some(lf) => match (lf.status, lf.tree, lf.fingerprint) {
                    (FolderStatus::Ok, Some(lt), Some(fp)) => folders.push(FolderApply {
                        name: lf.name.clone(),
                        key: f.key.clone(),
                        action: FolderAction::Replace,
                        tree: rec,
                        expected_live: ExpectedLive::Tree {
                            tree: lt,
                            fingerprint: fp,
                        },
                    }),
                    _ => {
                        blocked = Some(Explanation::Unstable);
                        break;
                    }
                },
            }
        }
        if let Some(why) = blocked {
            plan.skipped.push((key.clone(), why));
            continue;
        }
        if folders.is_empty() {
            continue;
        }
        folders.sort_by(|a, b| a.key.cmp(&b.key));
        plan.packages.push(PackageApply::code_apply(
            new_op(),
            key.clone(),
            version.hash,
            folders,
            why.clone(),
        ));
    }
    if plan.packages.is_empty() {
        return Err(if plan.skipped.is_empty() {
            PlanBlock::NothingToDo
        } else {
            PlanBlock::Unstable
        });
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::{run_publish, PublishOutcome};
    use crate::testkit::{addon, gate, Effect, FakeEffects, OpGen, Scenario, Step};

    fn suite(v: &str, tag: &str) -> Vec<crate::inventory::FolderScan> {
        vec![
            addon("Suite", Some(v), tag),
            addon("Other", Some("1.0"), "o"),
        ]
    }

    fn plan_for(sc: &Scenario, g: &ApplyGate) -> Result<ApplyPlan, PlanBlock> {
        let mut ops = OpGen::new();
        plan_apply(&sc.target(), &sc.local, &mut || ops.next_op(), g)
    }

    #[test]
    fn game_running_blocks_apply_plan() {
        let sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        assert!(plan_for(&sc, &gate(Activity::NotRunning)).is_ok());
        assert_eq!(
            plan_for(&sc, &gate(Activity::Running { pids: vec![4] })),
            Err(PlanBlock::GameRunning)
        );
    }

    #[test]
    fn unknown_activity_blocks_apply_plan() {
        let sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        assert_eq!(
            plan_for(&sc, &gate(Activity::Unknown("access denied".into()))),
            Err(PlanBlock::ActivityUnknown("access denied".into()))
        );
    }

    #[test]
    fn unstable_inventory_blocks_publish_and_apply() {
        let mut sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        sc.local.listing_stable = false;
        assert_eq!(
            plan_for(&sc, &gate(Activity::NotRunning)),
            Err(PlanBlock::Unstable)
        );
        assert_eq!(
            plan_publish(&sc.local, &sc.publish_ctx(1, &[])),
            Err(PlanBlock::Unstable)
        );
        // An unstable folder is published without a tree and a stable one is applied around it.
        let mut scans = suite("1.0", "a");
        scans[1].status = crate::inventory::ScanStatus::Unstable("changed".into());
        let sc = Scenario::new(scans).peer(2, suite("2.0", "b"));
        let p = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
        let other = p.state.folders.iter().find(|f| f.name == "Other").unwrap();
        assert_eq!((other.status, other.tree), (FolderStatus::Unstable, None));
        assert!(!p.state.inventory_complete);
        assert_eq!(p.source_fingerprints.len(), 1);
    }

    #[test]
    fn lazy_publish_yields_the_same_objects_as_the_full_plan() {
        let sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        let ctx = sc.publish_ctx(1, &[]);
        let full = plan_publish(&sc.local, &ctx).unwrap();
        let lean = plan_publish_state(&sc.local, &ctx).unwrap();
        assert!(lean.objects.is_empty());
        assert!(lean.state.folders.iter().all(|f| f.file_entries.is_empty()));
        let key = |o: &ObjectRef| (o.rel.clone(), o.expect);
        let mut a: Vec<_> = full.objects.iter().map(key).collect();
        let mut b: Vec<_> = publish_objects(&sc.local, &ctx.group)
            .map(|o| key(&o))
            .collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
        assert_eq!(count_publish_objects(&sc.local), a.len() as u64);
    }

    #[test]
    fn any_journal_blocks_apply_plan() {
        let sc = Scenario::new(suite("1.0", "a")).peer(2, suite("2.0", "b"));
        let mut g = gate(Activity::NotRunning);
        g.any_journal = true;
        assert_eq!(plan_for(&sc, &g), Err(PlanBlock::UnresolvedJournal));
    }

    #[test]
    fn extra_local_folders_are_left_alone() {
        let local = vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Other", Some("1.0"), "o"),
            addon("LocalOnly", Some("1.0"), "mine"),
        ];
        let sc = Scenario::new(local).peer(2, suite("2.0", "b"));
        let plan = plan_for(&sc, &gate(Activity::NotRunning)).unwrap();
        let names: Vec<&str> = plan
            .packages
            .iter()
            .flat_map(|p| p.folders.iter().map(|f| f.name.as_str()))
            .collect();
        assert_eq!(names, vec!["Suite"]);
        let f = &plan.packages[0].folders[0];
        assert_eq!(f.action, FolderAction::Replace);
        assert!(matches!(f.expected_live, ExpectedLive::Tree { .. }));
        assert_eq!(
            plan.packages[0].target_hash,
            sc.target().effective[&FolderKey::lossy("suite")].hash
        );
    }

    #[test]
    fn local_newer_or_same_is_not_downgraded() {
        let newer = Scenario::new(suite("2.0", "b")).peer(2, suite("1.0", "a"));
        assert_eq!(
            plan_for(&newer, &gate(Activity::NotRunning)),
            Err(PlanBlock::NothingToDo)
        );
        let same = Scenario::new(suite("1.0", "a")).peer(2, suite("1.0", "a"));
        assert_eq!(
            plan_for(&same, &gate(Activity::NotRunning)),
            Err(PlanBlock::NothingToDo)
        );
    }

    #[test]
    fn publish_uploads_missing_blobs_and_writes_state_last() {
        let sc = Scenario::new(suite("1.0", "a"));
        let plan = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
        assert!(
            plan.objects.iter().any(|o| o.kind == ObjectKind::Blob)
                && plan.objects.iter().any(|o| o.kind == ObjectKind::Tree)
        );
        let first_tree = plan
            .objects
            .iter()
            .position(|o| o.kind == ObjectKind::Tree)
            .unwrap();
        assert!(
            plan.objects[first_tree..]
                .iter()
                .all(|o| o.kind == ObjectKind::Tree),
            "blobs before trees"
        );
        let mut fx = FakeEffects::new();
        fx.set_live_from(&sc.local);
        let mut ctx = sc.publish_ctx(1, &[]);
        let out = run_publish(&plan, &mut ctx, &mut fx);
        assert!(matches!(out, PublishOutcome::Published { .. }), "{out:?}");
        let puts: Vec<usize> = fx
            .log
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Effect::Other(Step::PutObject)))
            .map(|(i, _)| i)
            .collect();
        let write = fx
            .log
            .iter()
            .position(|e| matches!(e, Effect::Other(Step::WriteRecord)))
            .unwrap();
        assert_eq!(puts.len(), plan.objects.len());
        assert!(puts.iter().all(|i| *i < write));
        assert!(fx.records.contains_key(&plan.state_path));
        // Second publish finds every object verified and uploads nothing.
        let before = fx.count(Step::PutObject);
        let mut ctx2 = sc.publish_ctx(2, &[&plan.state_path]);
        let plan2 = plan_publish(&sc.local, &ctx2).unwrap();
        run_publish(&plan2, &mut ctx2, &mut fx);
        assert_eq!(fx.count(Step::PutObject), before);
    }

    #[test]
    fn blocked_folders_are_never_published() {
        let mut bad = addon("Blocked", Some("9.9"), "secret");
        bad.status = crate::inventory::ScanStatus::Unreadable("locked".into());
        let scans = vec![addon("Good", Some("1.0"), "g"), bad];
        let sc = Scenario::new(scans);
        let p = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
        let blocked = sc
            .local
            .folders
            .iter()
            .find(|f| f.name == "Blocked")
            .unwrap();
        assert_eq!(blocked.status, FolderStatus::Unreadable);
        assert!(!blocked.transfer_eligible());
        let good = sc.local.folders.iter().find(|f| f.name == "Good").unwrap();
        assert!(good
            .file_entries
            .iter()
            .all(|e| p.objects.iter().any(|o| o.expect == e.sha256)));
        assert!(
            !blocked
                .file_entries
                .iter()
                .any(|e| p.objects.iter().any(|o| o.expect == e.sha256)),
            "blocked folder content must not be uploaded"
        );
        assert!(!p.objects.iter().any(|o| Some(o.expect) == blocked.tree));
        // Still listed in the state, visibly not complete.
        assert!(p.state.folders.iter().any(|f| f.name == "Blocked"));
        assert!(!p.state.inventory_complete);
    }

    #[test]
    fn unrecognised_folders_are_published_by_content() {
        let scans = vec![
            addon("Good", Some("1.0"), "g"),
            crate::testkit::addon_unvalidated("Odd", "9.9", "content"),
            crate::testkit::addon_unrecognised("Bare", "bare"),
        ];
        let sc = Scenario::new(scans);
        let p = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
        for name in ["Odd", "Bare"] {
            let f = sc.local.folders.iter().find(|f| f.name == name).unwrap();
            assert!(f.unrecognised() && f.transfer_eligible(), "{name}");
            assert!(
                f.file_entries
                    .iter()
                    .all(|e| p.objects.iter().any(|o| o.expect == e.sha256)),
                "{name} content is uploaded"
            );
            assert!(
                p.objects
                    .iter()
                    .filter(|o| o.kind == ObjectKind::Tree)
                    .count()
                    >= 3
            );
        }
        assert!(p.state.inventory_complete);
    }

    #[test]
    fn publish_needed_decisions() {
        let sc = Scenario::new(suite("1.0", "a"));
        let p = plan_publish(&sc.local, &sc.publish_ctx(1, &[])).unwrap();
        assert!(publish_needed(None, &sc.local, false, None, &[]));
        assert!(!publish_needed(Some(&p.state), &sc.local, false, None, &[]));
        assert!(publish_needed(Some(&p.state), &sc.local, true, None, &[]));
        let changed = Scenario::new(suite("1.1", "b"));
        assert!(publish_needed(
            Some(&p.state),
            &changed.local,
            false,
            None,
            &[]
        ));
        let more = Scenario::new(vec![
            addon("Suite", Some("1.0"), "a"),
            addon("Other", Some("1.0"), "o"),
            addon("Third", Some("1.0"), "t"),
        ]);
        assert!(publish_needed(
            Some(&p.state),
            &more.local,
            false,
            None,
            &[]
        ));
        // Completeness flips without a tree change.
        let mut unstable = Scenario::new(suite("1.0", "a"));
        unstable.local.folders[1].status = FolderStatus::Unstable;
        assert!(publish_needed(
            Some(&p.state),
            &unstable.local,
            false,
            None,
            &[]
        ));
        // A new index head or a changed capability list alone republishes.
        let head = AuthorityIndexHead {
            head: Sha256([4; 32]),
            total: 3,
        };
        assert!(publish_needed(
            Some(&p.state),
            &sc.local,
            false,
            Some(&head),
            &[]
        ));
        let caps = ["authority/1".to_string()];
        assert!(publish_needed(
            Some(&p.state),
            &sc.local,
            false,
            None,
            &caps
        ));
        let mut with = p.state.clone();
        with.authority_index = Some(head);
        with.features = caps.to_vec();
        assert!(!publish_needed(
            Some(&with),
            &sc.local,
            false,
            Some(&head),
            &caps
        ));
    }

    #[test]
    fn publish_prunes_to_three_own_states() {
        let sc = Scenario::new(suite("1.0", "a"));
        let own = [1u64, 2, 3, 4, 5]
            .map(|s| crate::layout::state_file_name(&crate::testkit::enrollment(0xA0), s));
        let other = crate::layout::state_file_name(&crate::testkit::enrollment(7), 1);
        let mut files: Vec<&str> = own.iter().map(String::as_str).collect();
        files.push(&other);
        files.push("junk.tmp");
        let plan = plan_publish(&sc.local, &sc.publish_ctx(6, &files)).unwrap();
        let pruned: Vec<u64> = plan
            .prune_states
            .iter()
            .map(|p| parse_state_name(p.rsplit('/').next().unwrap()).unwrap().1)
            .collect();
        assert_eq!(pruned, vec![3, 2, 1]);
        assert!(plan.prune_states.iter().all(|p| !p.contains(&other)));
    }

    #[test]
    fn seq_never_reused() {
        let sc = Scenario::new(suite("1.0", "a"));
        let name = crate::layout::state_file_name(&crate::testkit::enrollment(0xA0), 5);
        assert!(plan_publish(&sc.local, &sc.publish_ctx(5, &[&name])).is_err());
        assert!(plan_publish(&sc.local, &sc.publish_ctx(4, &[&name])).is_err());
        assert!(plan_publish(&sc.local, &sc.publish_ctx(6, &[&name])).is_ok());
        let _ = Sha256([0; 32]);
    }

    #[test]
    fn own_publish_slot_never_reuses_a_number() {
        use crate::layout::state_file_name;
        use crate::testkit::enrollment;
        let (mine, old) = (enrollment(1), enrollment(2));
        let files = vec![
            state_file_name(&mine, 3),
            state_file_name(&mine, 5),
            state_file_name(&old, 9),
            "notes.txt".to_string(),
        ];
        let (newest, seq) = own_publish_slot(&files, &mine, 2);
        assert_eq!(newest, Some(state_file_name(&mine, 5)));
        assert_eq!(seq, 10, "above every own state file of any enrollment");
        assert_eq!(own_publish_slot(&files, &mine, 12).1, 12);
        assert_eq!(own_publish_slot(&[], &mine, 1), (None, 1));
        assert_eq!(own_publish_slot(&[], &mine, 0), (None, 1));
    }
}
