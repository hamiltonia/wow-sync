//! Builds a validated local inventory from adapter scan results (domain.md §5).

use crate::hash::tree_hash;
use crate::ids::Sha256;
use crate::paths::{validate_folder_name, validate_rel_path, FolderKey, RelPath};
use crate::toc::{select_toc, ExternalId, TocCandidate, TocSelection};
use crate::variant::ConfirmedVariant;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What adapters report for one AddOns top-level entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderScan {
    pub name: String,
    pub status: ScanStatus,
    pub files: Vec<ScannedFile>,
    pub tocs: Vec<TocCandidate>,
    pub fingerprint: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanStatus {
    Ok,
    Unreadable(String),
    /// Reparse point / link.
    Unsafe(String),
    /// Changed while it was read.
    Unstable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub rel_path: String,
    pub size: u64,
    pub sha256: Sha256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderStatus {
    Ok,
    Unreadable,
    Unsafe,
    Unsupported,
    Unstable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: RelPath,
    pub size: u64,
    pub sha256: Sha256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TocKind {
    Selected,
    None,
    Ambiguous,
    Error,
    /// A suffixed candidate exists and no suffix order is validated (Q-3, Q-20).
    Unvalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocSummary {
    pub kind: TocKind,
    pub file: Option<String>,
    pub suffix: Option<String>,
    pub version: Option<String>,
    pub interface: Vec<u32>,
    pub hard_deps: Vec<String>,
    pub ids: Vec<ExternalId>,
}

impl TocSummary {
    pub fn empty(kind: TocKind) -> Self {
        Self {
            kind,
            file: None,
            suffix: None,
            version: None,
            interface: vec![],
            hard_deps: vec![],
            ids: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderState {
    pub name: String,
    pub key: FolderKey,
    pub status: FolderStatus,
    pub tree: Option<Sha256>,
    #[serde(skip)]
    pub fingerprint: Option<Sha256>,
    pub files: u32,
    pub bytes: u64,
    pub toc: TocSummary,
    /// Local only; peers' file lists come from tree records.
    #[serde(skip)]
    pub file_entries: Vec<FileEntry>,
}

/// The status label of an add-on without a recognised `.toc` (R-05, D-29).
pub const UNRECOGNISED_LABEL: &str = "Unrecognised: synced by content";

impl FolderState {
    /// Only these folders are published or installed (R-03, A-46, D-29): readable, safe and
    /// stable. A folder whose `.toc` is not recognised is eligible too and synced by content
    /// (`unrecognised`); unreadable, unsafe, unstable and unsupported folders stay "not synced".
    pub fn transfer_eligible(&self) -> bool {
        self.status == FolderStatus::Ok
    }

    /// Eligible, but no `.toc` was selected: the folder is synced by content lineage (R-05).
    pub fn unrecognised(&self) -> bool {
        self.status == FolderStatus::Ok && self.toc.kind != TocKind::Selected
    }

    /// Why the folder is unrecognised, as a short phrase; `None` for a recognised folder.
    pub fn unrecognised_reason(&self) -> Option<&'static str> {
        if !self.unrecognised() {
            return None;
        }
        Some(match self.toc.kind {
            TocKind::None => "no description (.toc) file for this game version",
            TocKind::Ambiguous => {
                "several description (.toc) files and no way to tell which applies"
            }
            TocKind::Unvalidated => "description (.toc) files only for other game versions",
            _ => "its description (.toc) file cannot be read",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub folders: Vec<FolderState>,
    pub listing_stable: bool,
}

impl Inventory {
    pub fn get(&self, key: &FolderKey) -> Option<&FolderState> {
        self.folders.iter().find(|f| &f.key == key)
    }
}

fn unusable(name: &str, key: FolderKey, status: FolderStatus) -> FolderState {
    FolderState {
        name: name.to_string(),
        key,
        status,
        tree: None,
        fingerprint: None,
        files: 0,
        bytes: 0,
        toc: TocSummary::empty(TocKind::None),
        file_entries: Vec::new(),
    }
}

fn summarize(sel: TocSelection) -> TocSummary {
    match sel {
        TocSelection::Selected {
            file_name,
            suffix,
            info,
        } => TocSummary {
            kind: TocKind::Selected,
            file: Some(file_name),
            suffix,
            version: info.version,
            interface: info.interface,
            hard_deps: info.hard_deps,
            ids: info.ids,
        },
        TocSelection::None { .. } => TocSummary::empty(TocKind::None),
        TocSelection::Ambiguous { .. } => TocSummary::empty(TocKind::Ambiguous),
        TocSelection::Unvalidated { .. } => TocSummary::empty(TocKind::Unvalidated),
        TocSelection::Error { .. } => TocSummary::empty(TocKind::Error),
    }
}

fn build_ok(scan: FolderScan, key: FolderKey, v: &ConfirmedVariant) -> FolderState {
    let mut entries = Vec::with_capacity(scan.files.len());
    let mut seen = BTreeMap::new();
    for f in &scan.files {
        let Ok(path) = validate_rel_path(&f.rel_path) else {
            return unusable(&scan.name, key, FolderStatus::Unsupported);
        };
        if seen.insert(path.as_str().to_lowercase(), ()).is_some() {
            return unusable(&scan.name, key, FolderStatus::Unsupported);
        }
        entries.push(FileEntry {
            path,
            size: f.size,
            sha256: f.sha256,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let bytes = entries.iter().map(|e| e.size).sum();
    FolderState {
        name: scan.name.clone(),
        key,
        status: FolderStatus::Ok,
        tree: Some(tree_hash(&entries)),
        fingerprint: scan.fingerprint,
        files: entries.len() as u32,
        bytes,
        toc: summarize(select_toc(&scan.name, &scan.tocs, v.mapping())),
        file_entries: entries,
    }
}

pub fn build_inventory(
    scans: Vec<FolderScan>,
    listing_stable: bool,
    v: &ConfirmedVariant,
) -> Inventory {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for s in &scans {
        *counts.entry(s.name.to_lowercase()).or_default() += 1;
    }
    let mut folders: Vec<FolderState> = scans
        .into_iter()
        .map(|scan| {
            let Ok(key) = validate_folder_name(&scan.name) else {
                let key = FolderKey::lossy(&scan.name);
                return unusable(&scan.name, key, FolderStatus::Unsupported);
            };
            if counts.get(&scan.name.to_lowercase()).copied().unwrap_or(0) > 1 {
                return unusable(&scan.name, key, FolderStatus::Unsupported);
            }
            match &scan.status {
                ScanStatus::Ok => build_ok(scan, key, v),
                ScanStatus::Unreadable(_) => unusable(&scan.name, key, FolderStatus::Unreadable),
                ScanStatus::Unsafe(_) => unusable(&scan.name, key, FolderStatus::Unsafe),
                ScanStatus::Unstable(_) => unusable(&scan.name, key, FolderStatus::Unstable),
            }
        })
        .collect();
    folders.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| a.name.cmp(&b.name)));
    Inventory {
        folders,
        listing_stable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit;

    #[test]
    fn eligibility_follows_status_and_unrecognised_folders_say_why() {
        let v = testkit::confirmed_forever();
        let mut locked = testkit::addon("Locked", Some("1.0"), "l");
        locked.status = ScanStatus::Unreadable("denied".into());
        let inv = build_inventory(
            vec![
                testkit::addon("Plain", Some("1.0"), "p"),
                testkit::addon_unrecognised("Bare", "b"),
                testkit::addon_unvalidated("Foreign", "1.0", "f"),
                locked,
            ],
            true,
            &v,
        );
        let f = |n: &str| inv.folders.iter().find(|f| f.name == n).unwrap();
        assert!(f("Plain").transfer_eligible() && !f("Plain").unrecognised());
        assert_eq!(f("Plain").unrecognised_reason(), None);
        for n in ["Bare", "Foreign"] {
            assert!(f(n).transfer_eligible() && f(n).unrecognised(), "{n}");
        }
        assert!(f("Bare")
            .unrecognised_reason()
            .is_some_and(|r| r.contains(".toc")));
        assert!(!f("Locked").transfer_eligible() && !f("Locked").unrecognised());
    }

    #[test]
    fn case_collision_marks_both_unsupported() {
        let v = testkit::confirmed_forever();
        let inv = build_inventory(
            vec![
                testkit::folder_scan("Addon", &[("a.lua", b"1")], &["Addon.toc"]),
                testkit::folder_scan("ADDON", &[("a.lua", b"2")], &["ADDON.toc"]),
                testkit::folder_scan("Other", &[("a.lua", b"3")], &["Other.toc"]),
            ],
            true,
            &v,
        );
        let st = |n: &str| inv.folders.iter().find(|f| f.name == n).unwrap().status;
        assert_eq!(st("Addon"), FolderStatus::Unsupported);
        assert_eq!(st("ADDON"), FolderStatus::Unsupported);
        assert_eq!(st("Other"), FolderStatus::Ok);
        assert!(inv
            .folders
            .iter()
            .filter(|f| f.status == FolderStatus::Unsupported)
            .all(|f| f.tree.is_none()));
    }

    #[test]
    fn inventory_hashes_sorts_and_summarizes() {
        let v = testkit::confirmed_forever();
        let mut unsafe_scan = testkit::folder_scan("Z", &[], &[]);
        unsafe_scan.status = ScanStatus::Unsafe("junction".into());
        let mut bad_path = testkit::folder_scan("Evil", &[("a.lua", b"1")], &["Evil.toc"]);
        bad_path.files[0].rel_path = "../x".into();
        let inv = build_inventory(
            vec![
                unsafe_scan,
                testkit::folder_scan("B", &[("b.lua", b"bb"), ("a.lua", b"a")], &["B.toc"]),
                bad_path,
                testkit::folder_scan("Suf", &[("a.lua", b"1")], &["Suf.toc", "Suf_Forever.toc"]),
            ],
            true,
            &v,
        );
        let keys: Vec<&str> = inv.folders.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["b", "evil", "suf", "z"]);
        let b = &inv.folders[0];
        assert_eq!((b.files, b.bytes, b.status), (2, 3, FolderStatus::Ok));
        assert_eq!(b.file_entries[0].path.as_str(), "a.lua");
        assert_eq!(b.toc.kind, TocKind::Selected);
        assert_eq!(inv.folders[1].status, FolderStatus::Unsupported);
        assert_eq!(inv.folders[2].toc.kind, TocKind::Selected);
        assert_eq!(inv.folders[3].status, FolderStatus::Unsafe);
        assert!(inv.folders[3].tree.is_none());
    }
}
