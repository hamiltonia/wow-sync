//! Typed, GUI-ready view of the sync state (UG-1, 3, 4, 6, 10, 11, 14). Everything here is
//! read from core results; nothing is decided here. Sentences name computers by the name
//! their owner chose, never by an internal ID (UG-22).

use std::collections::BTreeMap;
use wow_sync_core::admit::{MemberLatest, MemberView};
use wow_sync_core::explain::{render_named, Explanation};
use wow_sync_core::ids::ComputerId;
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
            next: next_action(status, why),
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

pub fn member_rows(
    i: &DetailInput,
    this: (&str, &str, Option<String>),
    packages: &[PackageRow],
) -> Vec<MemberRow> {
    let (this_name, app_version, game_build) = this;
    let confirmed = confirmations(i.target, i.views);
    let synced = packages
        .iter()
        .filter(|p| !matches!(p.status, PackageStatus::NotSynced))
        .count();
    let mut out = vec![MemberRow {
        id: i.this_id.to_string(),
        name: this_name.to_string(),
        is_this: true,
        state: MemberSummary::Usable,
        app_version: Some(app_version.to_string()),
        game_build,
        seen_locally_at: None,
        confirmed: synced as u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_codes_are_not_part_of_a_title() {
        assert_eq!(strip_colour_codes("|cff00ff00Bagnon|r"), "Bagnon");
        assert_eq!(strip_colour_codes("Plain"), "Plain");
        assert_eq!(strip_colour_codes("a|cff"), "a|cff");
    }
}
