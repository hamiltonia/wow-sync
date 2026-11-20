//! Typed, GUI-ready view of the sync state (UG-1, 3, 4, 6, 10, 11, 14). Everything here is
//! read from core results; nothing is decided here. Sentences name computers by the name
//! their owner chose, never by an internal ID (UG-22).

use std::collections::BTreeMap;
use wow_sync_core::admit::{MemberLatest, MemberView};
use wow_sync_core::explain::{render_named, Explanation};
use wow_sync_core::ids::{ComputerId, Sha256};
use wow_sync_core::inventory::{FolderScan, FolderState, Inventory};
use wow_sync_core::paths::FolderKey;
use wow_sync_core::status::{confirmations, MemberSummary, PackageStatus, StatusView};
use wow_sync_core::target::{GroupTarget, PackageTarget};
use wow_sync_core::toc::parse_toc;
use wow_sync_core::view::{next_action, NextAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRow {
    pub name: String,
    pub version: Option<String>,
    pub files: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    pub version: Option<String>,
    pub holders: Vec<String>,
    pub here: bool,
    pub folders: Vec<FolderRow>,
    pub differences: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRow {
    /// Internal folder key; shown only in the Details area.
    pub key: String,
    pub title: String,
    pub status: PackageStatus,
    pub sentence: String,
    pub next: NextAction,
    pub version_here: Option<String>,
    pub version_target: Option<String>,
    /// Computers the chosen copy comes from, by name.
    pub sources: Vec<String>,
    pub folders: Vec<FolderRow>,
    /// Computers that have not yet reported applying this package.
    pub awaiting: Vec<String>,
    pub confirmed_by: Vec<String>,
    pub candidates: Vec<CandidateRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    pub id: String,
    pub name: String,
    pub is_this: bool,
    pub state: MemberSummary,
    pub app_version: Option<String>,
    pub game_build: Option<String>,
    /// Local-clock milliseconds when this computer last saw new data from it.
    pub seen_locally_at: Option<i64>,
    /// Add-ons it has confirmed as applied / not yet confirmed.
    pub confirmed: u32,
    pub unconfirmed: Vec<String>,
}

/// Everything the GUI shows beyond the headline.
#[derive(Debug, Clone)]
pub struct StatusDetail {
    pub view: StatusView,
    pub packages: Vec<PackageRow>,
    pub members: Vec<MemberRow>,
    pub game: wow_sync_core::plan::Activity,
    pub group_id: String,
    pub computer_id: String,
    pub variant_label: String,
    pub variant_folder: String,
    pub app_version: String,
    pub game_build: Option<String>,
}

/// `|cAARRGGBB` and `|r` colour codes are display markup, not part of a name.
pub fn strip_colour_codes(s: &str) -> String {
    let mut out = String::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '|' && i + 1 < b.len() {
            match b[i + 1] {
                'c' | 'C' if i + 10 <= b.len() => {
                    i += 10;
                    continue;
                }
                'r' | 'R' => {
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out.trim().to_string()
}

/// Folder name (lowercase) to the title its own `.toc` gives, when it has a readable one.
pub fn folder_titles(scans: &[FolderScan]) -> BTreeMap<String, String> {
    use wow_sync_core::variant::FileObs;
    let mut out = BTreeMap::new();
    for s in scans {
        let want = format!("{}.toc", s.name).to_ascii_lowercase();
        let Some(c) = s
            .tocs
            .iter()
            .find(|c| c.file_name.to_ascii_lowercase() == want)
        else {
            continue;
        };
        let FileObs::Bytes(b) = &c.contents else {
            continue;
        };
        if let Ok(info) = parse_toc(b) {
            if let Some(t) = info.title {
                let t = strip_colour_codes(&t);
                if !t.is_empty() {
                    out.insert(s.name.to_ascii_lowercase(), t);
                }
            }
        }
    }
    out
}

pub struct DetailInput<'a> {
    pub view: &'a StatusView,
    pub target: &'a GroupTarget,
    pub inventory: &'a Inventory,
    pub views: &'a [MemberView],
    pub titles: &'a BTreeMap<String, String>,
    pub this_id: &'a ComputerId,
    /// The game is observed running now; only the next step depends on it.
    pub game_running: bool,
}

fn folder_row(f: &FolderState) -> FolderRow {
    FolderRow {
        name: f.name.clone(),
        version: f.toc.version.clone(),
        files: f.files,
        bytes: f.bytes,
    }
}

fn main_version(folders: &[FolderState], key: &FolderKey) -> Option<String> {
    folders
        .iter()
        .find(|f| &f.key == key)
        .or_else(|| folders.first())
        .and_then(|f| f.toc.version.clone())
}

fn namer<'a>(views: &'a [MemberView], this: &'a ComputerId) -> impl Fn(&ComputerId) -> String + 'a {
    move |c: &ComputerId| {
        if c == this {
            return "this computer".to_string();
        }
        views
            .iter()
            .find(|v| &v.computer == c)
            .map(|v| v.display_name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "another computer".to_string())
    }
}

pub fn render_with_names(e: &Explanation, views: &[MemberView], this: &ComputerId) -> String {
    render_named(e, &namer(views, this))
}

pub fn build_package_rows(i: &DetailInput) -> Vec<PackageRow> {
    let name_of = namer(i.views, i.this_id);
    let confirmed = confirmations(i.target, i.views);
    let mut rows = Vec::new();
    for (key, status, why) in &i.view.packages {
        let pt = i.target.packages.get(key);
        let local = i.inventory.get(key);
        let title = i
            .titles
            .get(&key.as_str().to_ascii_lowercase())
            .cloned()
            .or_else(|| local.map(|f| f.name.clone()))
            .or_else(|| match pt {
                Some(PackageTarget::Chosen { version, .. }) => version
                    .folders
                    .iter()
                    .find(|f| &f.key == key)
                    .map(|f| f.name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| key.to_string());
        let mut sources = Vec::new();
        let mut version_target = None;
        let mut folders: Vec<FolderRow> = local.map(folder_row).into_iter().collect();
        let mut candidates = Vec::new();
        match pt {
            Some(PackageTarget::Chosen {
                version,
                sources: s,
                ..
            }) => {
                sources = s.iter().map(&name_of).collect();
                version_target = main_version(&version.folders, key);
                folders = version.folders.iter().map(folder_row).collect();
            }
            Some(PackageTarget::NeedsDecision { candidates: c, .. }) => {
                let mut union: Vec<String> = Vec::new();
                for (v, _) in c {
                    for f in &v.folders {
                        if !union.contains(&f.name) {
                            union.push(f.name.clone());
                        }
                    }
                }
                for (v, holders) in c {
                    let missing: Vec<&String> = union
                        .iter()
                        .filter(|n| !v.folders.iter().any(|f| &f.name == *n))
                        .collect();
                    let differences = if missing.is_empty() {
                        "Has every folder.".to_string()
                    } else {
                        format!(
                            "Missing folders that other copies have: {}.",
                            missing
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                    candidates.push(CandidateRow {
                        version: main_version(&v.folders, key),
                        holders: holders.iter().map(&name_of).collect(),
                        here: holders.contains(i.this_id),
                        folders: v.folders.iter().map(folder_row).collect(),
                        differences,
                    });
                }
            }
            _ => {}
        }
        let version_here = local.and_then(|f| f.toc.version.clone());
        let confirmed_ids: Vec<ComputerId> = match status {
            PackageStatus::ConfirmedBy(c) => c.clone(),
            _ => confirmed.get(key).cloned().unwrap_or_default(),
        };
        let reportable = matches!(
            status,
            PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_) | PackageStatus::InSync
        );
        let awaiting = if reportable {
            i.views
                .iter()
                .filter(|v| matches!(v.latest, MemberLatest::Valid { .. }))
                .filter(|v| !confirmed_ids.contains(&v.computer))
                .map(|v| name_of(&v.computer))
                .collect()
        } else {
            Vec::new()
        };
        rows.push(PackageRow {
            key: key.to_string(),
            title,
            status: status.clone(),
            sentence: render_named(why, &name_of),
            next: next_action(status, why, i.game_running),
            version_here,
            version_target,
            sources,
            folders,
            awaiting,
            confirmed_by: confirmed_ids.iter().map(&name_of).collect(),
            candidates,
        });
    }
    rows
}

/// What this computer itself has confirmed: only packages that are applied or in sync here.
/// Available, blocked, undecided, interrupted and not-synced packages are not confirmed.
fn this_confirmed(packages: &[PackageRow]) -> u32 {
    packages
        .iter()
        .filter(|p| {
            matches!(
                p.status,
                PackageStatus::InSync | PackageStatus::AppliedHere | PackageStatus::ConfirmedBy(_)
            )
        })
        .count() as u32
}

pub fn member_rows(
    i: &DetailInput,
    this: (&str, &str, Option<String>),
    packages: &[PackageRow],
) -> Vec<MemberRow> {
    let (this_name, app_version, game_build) = this;
    let confirmed = confirmations(i.target, i.views);
    let synced = this_confirmed(packages);
    let mut out = vec![MemberRow {
        id: i.this_id.to_string(),
        name: this_name.to_string(),
        is_this: true,
        state: MemberSummary::Usable,
        app_version: Some(app_version.to_string()),
        game_build,
        seen_locally_at: None,
        confirmed: synced,
        unconfirmed: Vec::new(),
    }];
    for m in &i.view.members {
        let latest = i.views.iter().find(|v| v.computer == m.computer);
        let (app, build) = match latest.map(|v| &v.latest) {
            Some(MemberLatest::Valid { state, .. }) => (
                Some(state.app_version.clone()),
                state.client.build_version.clone(),
            ),
            _ => (None, None),
        };
        let mut unconfirmed = Vec::new();
        let mut done = 0;
        for ((key, _, _), p) in i.view.packages.iter().zip(packages) {
            if p.status == PackageStatus::NotSynced {
                continue;
            }
            let ok = confirmed
                .get(key)
                .map(|c| c.contains(&m.computer))
                .unwrap_or(false);
            if ok {
                done += 1;
            } else {
                unconfirmed.push(p.title.clone());
            }
        }
        out.push(MemberRow {
            id: m.computer.to_string(),
            name: m.display_name.clone(),
            is_this: false,
            state: m.state,
            app_version: app,
            game_build: build,
            seen_locally_at: m.seen_locally_at,
            confirmed: done,
            unconfirmed,
        });
    }
    out
}

/// One package the group has removed and that is still removed (the "Removed" section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedRow {
    pub title: String,
    pub folders: Vec<String>,
    /// Who removed it and how it was noticed, by the name its owner chose.
    pub provenance: String,
    /// Computers that have not yet acknowledged the removal.
    pub awaiting: Vec<String>,
}

fn how(e: wow_sync_core::kinds::RemovalEvidence) -> &'static str {
    use wow_sync_core::kinds::RemovalEvidence as E;
    match e {
        E::UserAction => "removed it",
        E::UserConfirmed => "confirmed it was removed",
        E::Observed => "noticed it was missing",
        E::FolderDrop => "noticed an upgrade dropped it",
        E::Rollback => "rolled it back",
    }
}

/// One effective retirement as a row of the Computers "Retired" group (ops.md §2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementRow {
    pub computer: ComputerId,
    pub enrollment: wow_sync_core::ids::EnrollmentId,
    /// The retired computer, by the name its owner chose.
    pub name: String,
    /// Who retired it, by name.
    pub retired_by: String,
    pub retired_by_id: ComputerId,
    /// Its records after this sequence number are ignored.
    pub cutoff_seq: u64,
    pub is_this: bool,
}

/// Where one enrolled computer stands in the group (ops.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStanding {
    Active,
    /// A retirement naming it is effective.
    Retired,
    /// A retirement naming it exists but is not yet settled; nothing may rely on either state.
    RetirementUnresolved,
}

/// One enrolled computer with the exact enrollment a retire or master choice must name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRow {
    pub enrollment: wow_sync_core::kinds::EnrollmentRef,
    /// By the name its owner chose; "another computer" when none is known.
    pub name: String,
    pub standing: PeerStanding,
    /// When the retirement was written (the retiring computer's clock, milliseconds).
    pub retired_at_ms: Option<u64>,
    pub is_this: bool,
}

pub fn peer_rows(
    members: &wow_sync_core::ledger::Membership,
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &BTreeMap<ComputerId, String>,
    this: &ComputerId,
) -> Vec<PeerRow> {
    use wow_sync_core::authority::{AuthorityRecord, RetStatus};
    let mut standing: BTreeMap<wow_sync_core::kinds::EnrollmentRef, PeerStanding> = BTreeMap::new();
    let mut when: BTreeMap<wow_sync_core::kinds::EnrollmentRef, u64> = BTreeMap::new();
    for e in ledger.entries() {
        let Some(AuthorityRecord::Retirement(r)) = ledger.record(&e.hash) else {
            continue;
        };
        match derived.retirement_status.get(&e.hash) {
            Some(RetStatus::Effective) => {
                standing.insert(r.retired.clone(), PeerStanding::Retired);
                when.insert(r.retired.clone(), r.written_unix_ms);
            }
            Some(RetStatus::Unresolved) => {
                standing
                    .entry(r.retired.clone())
                    .or_insert(PeerStanding::RetirementUnresolved);
            }
            _ => {}
        }
    }
    let mut out: Vec<PeerRow> = members
        .pairs()
        .map(|p| PeerRow {
            enrollment: p.clone(),
            name: if &p.computer_id == this {
                "this computer".to_string()
            } else {
                names
                    .get(&p.computer_id)
                    .cloned()
                    .unwrap_or_else(|| "another computer".into())
            },
            standing: standing.get(p).copied().unwrap_or(PeerStanding::Active),
            retired_at_ms: when.get(p).copied(),
            is_this: &p.computer_id == this,
        })
        .collect();
    out.sort_by(|a, b| {
        (!a.is_this, &a.name, &a.enrollment).cmp(&(!b.is_this, &b.name, &b.enrollment))
    });
    out
}

/// An older complete release of a package that a rollback can bring back (one row per release).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldRelease {
    pub package: FolderKey,
    pub title: String,
    /// Pass this to `Command::Rollback { recovered }`.
    pub release: wow_sync_core::ids::Sha256,
    pub label: String,
    /// Computers that hold this release, by name.
    pub holders: Vec<String>,
}

/// Effective retirements only; unresolved ones are reported through the hold, not as rows.
pub fn retirement_rows(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &BTreeMap<ComputerId, String>,
    this: &ComputerId,
) -> Vec<RetirementRow> {
    use wow_sync_core::authority::{AuthorityRecord, RetStatus};
    let name = |c: &ComputerId| -> String {
        if c == this {
            "this computer".to_string()
        } else {
            names
                .get(c)
                .cloned()
                .unwrap_or_else(|| "another computer".into())
        }
    };
    let mut out = Vec::new();
    for e in ledger.entries() {
        if derived.retirement_status.get(&e.hash) != Some(&RetStatus::Effective) {
            continue;
        }
        let Some(AuthorityRecord::Retirement(r)) = ledger.record(&e.hash) else {
            continue;
        };
        out.push(RetirementRow {
            computer: r.retired.computer_id.clone(),
            enrollment: r.retired.enrollment_id.clone(),
            name: name(&r.retired.computer_id),
            retired_by: name(&r.issuer.computer_id),
            retired_by_id: r.issuer.computer_id.clone(),
            cutoff_seq: r.cutoff_seq,
            is_this: &r.retired.computer_id == this,
        });
    }
    out
}

/// The non-installed releases of every package this computer has installed. Rejected and
/// excluded releases are already absent from `copies`.
pub fn held_releases(
    copies: &[wow_sync_core::target::PackageCopies],
    inventory: &Inventory,
    titles: &BTreeMap<String, String>,
    own: &wow_sync_core::ids::EnrollmentId,
    names: &BTreeMap<wow_sync_core::ids::EnrollmentId, String>,
) -> Vec<HeldRelease> {
    let mut out = Vec::new();
    for pc in copies {
        let installed = pc
            .copies
            .iter()
            .any(|c| c.holders.iter().any(|h| &h.enrollment_id == own));
        if !installed {
            continue;
        }
        let display = inventory.get(&pc.package.display);
        let title = titles
            .get(pc.package.key.as_str())
            .cloned()
            .or_else(|| display.map(|f| f.name.clone()))
            .unwrap_or_else(|| pc.package.display.to_string());
        for c in &pc.copies {
            if c.holders.iter().any(|h| &h.enrollment_id == own) {
                continue;
            }
            out.push(HeldRelease {
                package: pc.package.key.clone(),
                title: title.clone(),
                release: c.version.hash,
                label: c
                    .version
                    .folders
                    .iter()
                    .find(|f| f.key == pc.package.display)
                    .and_then(|f| f.toc.version.clone())
                    .unwrap_or_default(),
                holders: c
                    .holders
                    .iter()
                    .map(|h| {
                        names
                            .get(&h.enrollment_id)
                            .cloned()
                            .unwrap_or_else(|| "another computer".into())
                    })
                    .collect(),
            });
        }
    }
    out
}

/// "Review needed" (ops.md §1.7, UQ-1): a pending guided reconfirmation shows it, but never
/// over `NeedsHelp` or `Error`, and Paused or Working never hide it. A pending decision alone
/// does not raise it (Q-59).
pub fn review_needed(reconfirm_pending: bool, base: wow_sync_core::status::Headline) -> bool {
    use wow_sync_core::status::Headline as H;
    reconfirm_pending && !matches!(base, H::NeedsHelp | H::Error)
}

/// Live removals as rows, with provenance by computer name (UG-22). Nothing is decided here.
#[cfg(test)]
pub fn removed_rows(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &BTreeMap<ComputerId, String>,
    this: &ComputerId,
) -> Vec<RemovedRow> {
    removed_rows_keyed(ledger, derived, names, this)
        .into_iter()
        .map(|(row, _)| row)
        .collect()
}

/// The release a rollback rejected and that is still rejected (the "Allow again" list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedRelease {
    pub folders: Vec<String>,
    pub label: String,
    /// The rejected package; the id a later same-release candidate is compared with.
    pub release: Sha256,
    /// The record hashes `Command::AllowAgain { rejections }` takes.
    pub rejections: Vec<Sha256>,
    /// When the rejection was written (the rejecting computer's clock, milliseconds).
    pub rejected_at_ms: u64,
}

/// Active rejections, one row per rejected release, grouping every record that rejects it.
pub fn rejected_releases(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
) -> Vec<RejectedRelease> {
    let mut out: Vec<RejectedRelease> = Vec::new();
    for (hash, r) in wow_sync_core::reject::RejectBehaviour::new(derived).active(ledger) {
        if let Some(row) = out
            .iter_mut()
            .find(|x| x.release == r.rejected.package_hash)
        {
            row.rejections.push(hash);
            row.rejected_at_ms = row.rejected_at_ms.max(r.written_unix_ms);
            continue;
        }
        out.push(RejectedRelease {
            folders: r
                .rejected
                .folders
                .iter()
                .map(|t| t.key.to_string())
                .collect(),
            label: r.rejected.version_label.clone(),
            release: r.rejected.package_hash,
            rejections: vec![hash],
            rejected_at_ms: r.written_unix_ms,
        });
    }
    out
}

/// The first live computer whose software predates rollback (no `rejection/1`), by name.
/// It would keep installing a rejected release, so a rollback must say so first.
pub fn older_peer(views: &[wow_sync_core::admit::MemberView], peers: &[PeerRow]) -> Option<String> {
    use wow_sync_core::admit::MemberLatest;
    views
        .iter()
        .filter(|v| {
            !peers.iter().any(|p| {
                p.enrollment.enrollment_id == v.enrollment && p.standing == PeerStanding::Retired
            })
        })
        .find_map(|v| match &v.latest {
            MemberLatest::Valid { state, .. }
                if !state.features.iter().any(|f| f == "rejection/1") =>
            {
                Some(v.display_name.clone())
            }
            _ => None,
        })
}

/// Live removals with the record hash `Command::RestoreRemoved { removal }` takes.
pub fn removed_rows_keyed(
    ledger: &wow_sync_core::ledger::Ledger,
    derived: &wow_sync_core::ledger::Derived,
    names: &BTreeMap<ComputerId, String>,
    this: &ComputerId,
) -> Vec<(RemovedRow, Sha256)> {
    use wow_sync_core::authority::AuthorityRecord;
    let name = |c: &ComputerId| -> String {
        if c == this {
            "this computer".to_string()
        } else {
            names
                .get(c)
                .cloned()
                .unwrap_or_else(|| "another computer".into())
        }
    };
    let mut out = Vec::new();
    for unit in wow_sync_core::removal::removal_units(ledger, derived) {
        for h in &unit.removals {
            let Some(AuthorityRecord::Removal(r)) = ledger.record(h) else {
                continue;
            };
            let awaiting = r
                .audience
                .iter()
                .filter(|e| {
                    r.folders
                        .iter()
                        .any(|m| wow_sync_core::removal::owes(ledger, derived, e, h, &m.key))
                })
                .map(|e| name(&e.computer_id))
                .collect();
            out.push((
                RemovedRow {
                    title: strip_colour_codes(&r.title),
                    folders: r.folders.iter().map(|m| m.name.clone()).collect(),
                    provenance: format!("{} {}", name(&r.issuer.computer_id), how(r.evidence)),
                    awaiting,
                },
                *h,
            ));
        }
    }
    out
}

#[cfg(test)]
mod older_peer_tests {
    use super::*;
    use crate::removal_cycle::fixtures::{addon, computer, enrollment, group_id, inventory};
    use wow_sync_core::package::ClientInfo;
    use wow_sync_core::records::{CodeState, FEATURES};

    fn view(n: u8, features: &[&str]) -> MemberView {
        let inv = inventory(vec![addon("Alpha", Some("1"), "a")]);
        MemberView {
            computer: computer(n),
            enrollment: enrollment(n),
            display_name: format!("pc{n}"),
            latest: MemberLatest::Valid {
                seq: 1,
                state: Box::new(CodeState {
                    group_id: group_id(),
                    computer_id: computer(n),
                    enrollment_id: enrollment(n),
                    seq: 1,
                    product: "wowf".into(),
                    app_version: "0.0.0".into(),
                    written_unix_ms: 1,
                    client: ClientInfo {
                        build_version: Some("1.15.8".into()),
                        toc_mapping: "validated-plain/1".into(),
                    },
                    inventory_complete: true,
                    folders: inv.folders,
                    apply_report: None,
                    authority_index: None,
                    features: features.iter().map(|f| f.to_string()).collect(),
                }),
            },
        }
    }

    fn peer(n: u8, standing: PeerStanding) -> PeerRow {
        PeerRow {
            enrollment: wow_sync_core::kinds::EnrollmentRef {
                computer_id: computer(n),
                enrollment_id: enrollment(n),
            },
            name: format!("pc{n}"),
            standing,
            retired_at_ms: None,
            is_this: false,
        }
    }

    #[test]
    fn a_live_computer_without_rejection_support_is_the_older_peer() {
        let peers = [peer(2, PeerStanding::Active), peer(3, PeerStanding::Active)];
        let current = view(2, FEATURES);
        let old = view(3, &["authority/1"]);
        assert_eq!(older_peer(std::slice::from_ref(&current), &peers), None);
        assert_eq!(older_peer(&[current, old], &peers), Some("pc3".to_string()));
        let none_at_all = view(3, &[]);
        assert_eq!(
            older_peer(&[none_at_all], &peers),
            Some("pc3".to_string()),
            "a slice that predates features counts as older"
        );
    }

    #[test]
    fn a_retired_computer_is_never_the_older_peer() {
        let peers = [peer(3, PeerStanding::Retired)];
        assert_eq!(older_peer(&[view(3, &[])], &peers), None);
    }
}

#[cfg(test)]
mod removed_tests {
    use super::*;
    use crate::removal_cycle::fixtures::{addon, computer, enrollment, group_id, inventory};
    use wow_sync_core::hash::sha256;
    use wow_sync_core::kinds::{AuthorityKind, EnrollmentRef, RemovalEvidence};
    use wow_sync_core::ledger::{derive, Ledger, Membership};
    use wow_sync_core::records::encode;
    use wow_sync_core::removal::removal_record;

    fn eref(n: u8) -> EnrollmentRef {
        EnrollmentRef {
            computer_id: computer(n),
            enrollment_id: enrollment(n),
        }
    }

    #[test]
    fn removed_rows_name_who_removed_it_and_who_has_not_acknowledged() {
        let mut members = Membership::default();
        for n in [1, 2] {
            members.add(&computer(n), &enrollment(n));
        }
        let o = wow_sync_core::removal::ObservedRemoval {
            package: "|cff00ff00Alpha|r".into(),
            folders: inventory(vec![addon("Alpha", Some("1"), "a")]).folders,
            evidence: RemovalEvidence::Observed,
        };
        let r = removal_record(&group_id(), &eref(2), vec![eref(1), eref(2)], &o, 5, 1);
        let bytes = encode(&r);
        let mut ledger = Ledger::new();
        ledger.learn(
            sha256(&bytes),
            AuthorityKind::Removal,
            vec![r.folders[0].key.clone()],
            Some(&computer(2)),
            Some(&bytes),
        );
        let d = derive(&ledger, &group_id(), &members);
        let names = BTreeMap::from([(computer(2), "Desk".to_string())]);
        let rows = removed_rows(&ledger, &d, &names, &computer(1));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Alpha");
        assert_eq!(rows[0].provenance, "Desk noticed it was missing");
        assert_eq!(rows[0].folders, vec!["Alpha".to_string()]);
        assert_eq!(
            rows[0].awaiting,
            vec!["this computer".to_string(), "Desk".to_string()]
        );
        // From the other computer's view the names swap; an unknown computer is not an ID.
        let rows = removed_rows(&ledger, &d, &BTreeMap::new(), &computer(2));
        assert_eq!(rows[0].provenance, "this computer noticed it was missing");
        assert!(rows[0].awaiting.contains(&"another computer".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_core::plan::PlanBlock;
    use wow_sync_core::target::DecisionReason;

    fn row(status: PackageStatus) -> PackageRow {
        PackageRow {
            key: "k".into(),
            title: "t".into(),
            status,
            sentence: String::new(),
            next: NextAction::Nothing,
            version_here: None,
            version_target: None,
            sources: Vec::new(),
            folders: Vec::new(),
            awaiting: Vec::new(),
            confirmed_by: Vec::new(),
            candidates: Vec::new(),
        }
    }

    #[test]
    fn this_computer_confirms_only_applied_or_in_sync_packages() {
        let rows = [
            row(PackageStatus::InSync),
            row(PackageStatus::AppliedHere),
            row(PackageStatus::ConfirmedBy(Vec::new())),
            row(PackageStatus::Available),
            row(PackageStatus::Blocked(PlanBlock::GameRunning)),
            row(PackageStatus::NeedsDecision(DecisionReason::Ambiguous)),
            row(PackageStatus::NeedsHelp("x".into())),
            row(PackageStatus::Interrupted),
            row(PackageStatus::Unavailable),
            row(PackageStatus::NotSynced),
        ];
        assert_eq!(this_confirmed(&rows), 3);
    }

    #[test]
    fn colour_codes_are_not_part_of_a_title() {
        assert_eq!(strip_colour_codes("|cff00ff00Bagnon|r"), "Bagnon");
        assert_eq!(strip_colour_codes("Plain"), "Plain");
        assert_eq!(strip_colour_codes("a|cff"), "a|cff");
    }
}
