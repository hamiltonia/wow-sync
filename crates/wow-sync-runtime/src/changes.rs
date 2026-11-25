//! Builds change records from what an apply already holds: the planned new trees, the
//! inventory taken before the apply, the target and the ledger. Nothing here reads the disk.

use crate::history::{
    ChangeKind, ChangeReason, ChangeRecord, ChangeSource, FileCounts, VersionLabel,
    MAX_RECORD_PATHS,
};
use std::collections::{BTreeMap, BTreeSet};
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::explain::Explanation;
use wow_sync_core::ids::ComputerId;
use wow_sync_core::inventory::{FolderState, Inventory};
use wow_sync_core::kinds::{AuthorityKind, ChoiceAction, RevisionKind};
use wow_sync_core::ledger::Ledger;
use wow_sync_core::paths::FolderKey;
use wow_sync_core::plan::PackageApply;
use wow_sync_core::records::{OpKind, RootState};
use wow_sync_core::target::{GroupTarget, PackageTarget};

const UNKNOWN_COMPUTER: &str = "another computer";
const MAX_VERSION_CHARS: usize = 40;

pub(crate) struct BuildCx<'a> {
    pub inventory: &'a Inventory,
    pub target: &'a GroupTarget,
    pub names: &'a BTreeMap<ComputerId, String>,
    pub own: &'a ComputerId,
    pub titles: &'a BTreeMap<String, String>,
    pub ledger: &'a Ledger,
    pub pass_id: u64,
    pub now_ms: i64,
}

impl BuildCx<'_> {
    fn source_of(&self, c: &ComputerId) -> ChangeSource {
        if c == self.own {
            ChangeSource::ThisComputer
        } else {
            ChangeSource::Computer(self.friendly(c))
        }
    }

    fn friendly(&self, c: &ComputerId) -> String {
        self.names
            .get(c)
            .map_or_else(|| UNKNOWN_COMPUTER.to_string(), |n| clip(n))
    }

    /// The newest record of `kind` that is about any of `keys`.
    fn newest(&self, kind: AuthorityKind, keys: &BTreeSet<FolderKey>) -> Option<&AuthorityRecord> {
        self.ledger
            .entries()
            .filter(|e| e.kind == kind)
            .filter_map(|e| self.ledger.record(&e.hash).map(|r| (e, r)))
            .filter(|(_, r)| match r {
                AuthorityRecord::Removal(x) => x.folders.iter().any(|f| keys.contains(&f.key)),
                AuthorityRecord::CodeChoice(x) => x.folders.iter().any(|f| keys.contains(f)),
                AuthorityRecord::Rejection(x) => x.folders.iter().any(|f| keys.contains(f)),
                _ => false,
            })
            .map(|(_, r)| r)
            .max_by_key(|r| written_ms(r))
    }

    pub(crate) fn stamp(&self) -> (u64, i64) {
        (self.pass_id, self.now_ms)
    }

    fn author_of(&self, r: &AuthorityRecord) -> Option<ChangeSource> {
        r.issuer().map(|i| self.source_of(&i.computer_id))
    }
}

fn written_ms(r: &AuthorityRecord) -> u64 {
    match r {
        AuthorityRecord::Removal(x) => x.written_unix_ms,
        AuthorityRecord::CodeChoice(x) => x.written_unix_ms,
        AuthorityRecord::Rejection(x) => x.written_unix_ms,
        AuthorityRecord::SettingsRevision(x) => x.written_unix_ms,
        _ => 0,
    }
}

/// Eight hex characters of a content hash: enough to tell two releases apart.
fn short(h: &wow_sync_core::ids::Sha256) -> String {
    h.0[..4].iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn clip(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_VERSION_CHARS)
        .collect()
}

pub(crate) fn label(f: &FolderState) -> VersionLabel {
    match f.toc.version.as_deref().map(str::trim) {
        Some(v) if !v.is_empty() => VersionLabel::Toc(clip(v)),
        _ => VersionLabel::ContentId(f.tree.map(|t| short(&t)).unwrap_or_default()),
    }
}

/// The folder that stands for a package in a label: its own key, else the first one.
fn pick<'a>(
    folders: impl Iterator<Item = &'a FolderState>,
    key: &FolderKey,
) -> Option<&'a FolderState> {
    let all: Vec<&FolderState> = folders.collect();
    all.iter()
        .find(|f| &f.key == key)
        .or_else(|| all.first())
        .copied()
}

fn display_name(cx: &BuildCx, p: &PackageApply) -> String {
    if let Some(t) = cx.titles.get(&p.package.as_str().to_ascii_lowercase()) {
        return clip(t);
    }
    p.folders
        .iter()
        .map(|f| (&f.key, &f.name))
        .chain(p.removes.iter().map(|r| (&r.key, &r.name)))
        .find(|(k, _)| **k == p.package)
        .or_else(|| {
            p.folders
                .first()
                .map(|f| (&f.key, &f.name))
                .or_else(|| p.removes.first().map(|r| (&r.key, &r.name)))
        })
        .map_or_else(|| p.package.as_str().to_string(), |(_, n)| n.clone())
}

fn old_label(cx: &BuildCx, p: &PackageApply) -> VersionLabel {
    let keys: Vec<&FolderKey> = p
        .folders
        .iter()
        .map(|f| &f.key)
        .chain(p.removes.iter().map(|r| &r.key))
        .collect();
    let present = keys
        .iter()
        .filter_map(|k| cx.inventory.get(k))
        .filter(|f| f.tree.is_some());
    match pick(present, &p.package) {
        Some(f) => label(f),
        None => VersionLabel::New,
    }
}

fn new_label(cx: &BuildCx, p: &PackageApply) -> VersionLabel {
    if let Some(PackageTarget::Chosen { version, .. }) = cx.target.packages.get(&p.package) {
        if version.hash == p.target_hash {
            if let Some(f) = pick(version.folders.iter(), &p.package) {
                return label(f);
            }
        }
    }
    if let Explanation::NewerVersion { to, .. } | Explanation::AppliedFrom { to: Some(to), .. } =
        &p.explanation
    {
        return VersionLabel::Toc(clip(to));
    }
    let tree = p
        .folders
        .iter()
        .find(|f| f.key == p.package)
        .or_else(|| p.folders.first())
        .map(|f| short(&f.tree.tree))
        .unwrap_or_default();
    VersionLabel::ContentId(tree)
}

struct Diff {
    counts: FileCounts,
    byte_delta: i64,
    paths: Vec<String>,
}

fn diff(cx: &BuildCx, p: &PackageApply) -> Diff {
    let multi = p.folders.len() + p.removes.len() > 1;
    let mut counts = FileCounts::default();
    let mut delta = 0i64;
    let mut paths = BTreeSet::new();
    let shown = |folder: &str, path: &str| {
        if multi {
            format!("{folder}/{path}")
        } else {
            path.to_string()
        }
    };
    let mut one = |folder: &str,
                   old: &[wow_sync_core::inventory::FileEntry],
                   new: &[wow_sync_core::inventory::FileEntry]| {
        let old_map: BTreeMap<&str, (&wow_sync_core::ids::Sha256, u64)> = old
            .iter()
            .map(|e| (e.path.as_str(), (&e.sha256, e.size)))
            .collect();
        let mut seen = BTreeSet::new();
        for e in new {
            delta += e.size as i64;
            seen.insert(e.path.as_str());
            match old_map.get(e.path.as_str()) {
                None => {
                    counts.added += 1;
                    paths.insert(shown(folder, e.path.as_str()));
                }
                Some((h, _)) if **h != e.sha256 => {
                    counts.changed += 1;
                    paths.insert(shown(folder, e.path.as_str()));
                }
                Some(_) => {}
            }
        }
        for (path, (_, size)) in &old_map {
            delta -= *size as i64;
            if !seen.contains(path) {
                counts.removed += 1;
                paths.insert(shown(folder, path));
            }
        }
    };
    for f in &p.folders {
        let old = cx
            .inventory
            .get(&f.key)
            .map_or(&[][..], |s| s.file_entries.as_slice());
        let new: &[_] = match f.action {
            wow_sync_core::plan::FolderAction::Remove => &[],
            _ => &f.tree.files,
        };
        one(&f.name, old, new);
    }
    for r in &p.removes {
        let old = cx
            .inventory
            .get(&r.key)
            .map_or(&[][..], |s| s.file_entries.as_slice());
        one(&r.name, old, &[]);
    }
    Diff {
        counts,
        byte_delta: delta,
        paths: paths.into_iter().collect(),
    }
}

/// Files added, changed and removed between two listings of one folder, and their paths.
fn file_diff(
    old: &[wow_sync_core::inventory::FileEntry],
    new: &[wow_sync_core::inventory::FileEntry],
) -> (FileCounts, Vec<String>) {
    let old_map: BTreeMap<&str, &wow_sync_core::ids::Sha256> =
        old.iter().map(|e| (e.path.as_str(), &e.sha256)).collect();
    let mut counts = FileCounts::default();
    let mut paths = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for e in new {
        seen.insert(e.path.as_str());
        match old_map.get(e.path.as_str()) {
            None => counts.added += 1,
            Some(h) if **h != e.sha256 => counts.changed += 1,
            Some(_) => continue,
        }
        paths.insert(e.path.as_str().to_string());
    }
    for path in old_map.keys().filter(|p| !seen.contains(*p)) {
        counts.removed += 1;
        paths.insert((*path).to_string());
    }
    (counts, paths.into_iter().collect())
}

fn capped(mut paths: Vec<String>) -> (Vec<String>, u32) {
    let more = paths.len().saturating_sub(MAX_RECORD_PATHS) as u32;
    paths.truncate(MAX_RECORD_PATHS);
    (paths, more)
}

fn folder_names(p: &PackageApply) -> Vec<String> {
    let mut v: Vec<String> = p
        .folders
        .iter()
        .map(|f| f.name.clone())
        .chain(p.removes.iter().map(|r| r.name.clone()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn explained_source(cx: &BuildCx, p: &PackageApply) -> Option<ChangeSource> {
    match &p.explanation {
        Explanation::NewerVersion { source, .. }
        | Explanation::Installed { source }
        | Explanation::ContentLineage { source, .. }
        | Explanation::AppliedFrom { source, .. } => Some(cx.source_of(source)),
        _ => None,
    }
    .or_else(|| match cx.target.packages.get(&p.package) {
        Some(PackageTarget::Chosen { sources, .. }) => sources.first().map(|s| cx.source_of(s)),
        _ => None,
    })
}

/// The record this package's apply will write once it is verified; the duration is filled in
/// then. `None` for operations that are not a user-visible change.
pub(crate) fn prepare(p: &PackageApply, cx: &BuildCx) -> Option<ChangeRecord> {
    let keys: BTreeSet<FolderKey> = p
        .folders
        .iter()
        .map(|f| f.key.clone())
        .chain(p.removes.iter().map(|r| r.key.clone()))
        .chain(std::iter::once(p.package.clone()))
        .collect();
    let d = diff(cx, p);
    let old = old_label(cx, p);
    let mut new = new_label(cx, p);
    let (kind, source, reason) = match p.kind {
        OpKind::Apply => {
            let kind = if old == VersionLabel::New {
                ChangeKind::InstalledNew
            } else {
                ChangeKind::Applied
            };
            let content_source = explained_source(cx, p).unwrap_or(ChangeSource::ThisComputer);
            // A peer's rollback arrives as a plain apply of the recovered release.
            let rolled_back = (old != VersionLabel::New)
                .then(|| cx.newest(AuthorityKind::Rejection, &keys))
                .flatten()
                .filter(|r| {
                    matches!(r, AuthorityRecord::Rejection(x)
                        if x.recovered.package_hash == p.target_hash)
                })
                .and_then(|r| cx.author_of(r));
            if let Some(by) = rolled_back {
                let mut rec = base(
                    cx.stamp(),
                    ChangeKind::RolledBack,
                    display_name(cx, p),
                    folder_names(p),
                    by,
                    ChangeReason::Rollback,
                );
                let (paths, more) = capped(d.paths);
                rec.old_version = old;
                rec.new_version = new;
                rec.files = d.counts;
                rec.byte_delta = d.byte_delta;
                rec.paths = paths;
                rec.paths_more = more;
                return Some(rec);
            }
            let chosen = cx
                .newest(AuthorityKind::CodeChoice, &keys)
                .filter(|r| {
                    matches!(r, AuthorityRecord::CodeChoice(c) if c.action == ChoiceAction::Keep
                        && c.keep.as_ref().is_some_and(|k| k.package_hash == p.target_hash))
                })
                .or_else(|| {
                    matches!(p.explanation, Explanation::ChosenByGroupRecord)
                        .then(|| cx.newest(AuthorityKind::CodeChoice, &keys))
                        .flatten()
                });
            let reason = match chosen.and_then(|r| cx.author_of(r)) {
                Some(ChangeSource::ThisComputer) => ChangeReason::Choice(None),
                Some(ChangeSource::Computer(n)) => ChangeReason::Choice(Some(n)),
                None => ChangeReason::NewerRelease,
            };
            (kind, content_source, reason)
        }
        OpKind::Remove => {
            new = VersionLabel::Removed;
            let by = cx
                .newest(AuthorityKind::Removal, &keys)
                .and_then(|r| cx.author_of(r))
                .unwrap_or(ChangeSource::ThisComputer);
            (ChangeKind::Removed, by, ChangeReason::Removal)
        }
        OpKind::Restore => (
            ChangeKind::RecoveryRestored,
            ChangeSource::ThisComputer,
            ChangeReason::Recovery,
        ),
        OpKind::Rollback => {
            let by = cx
                .newest(AuthorityKind::Rejection, &keys)
                .and_then(|r| cx.author_of(r))
                .unwrap_or(ChangeSource::ThisComputer);
            (ChangeKind::RolledBack, by, ChangeReason::Rollback)
        }
        OpKind::Settings => {
            let revision = p.settings_roots.iter().find_map(|root| {
                let RootState::Captured { checkpoint, .. } = &root.state else {
                    return None;
                };
                cx.ledger
                    .entries()
                    .filter_map(|e| cx.ledger.record(&e.hash))
                    .find_map(|r| match r {
                        AuthorityRecord::SettingsRevision(s) if s.checkpoint == *checkpoint => {
                            Some(s)
                        }
                        _ => None,
                    })
            });
            let reason = match revision.map(|r| r.kind) {
                Some(RevisionKind::Push) => ChangeReason::PeerPush,
                _ => ChangeReason::SettingsMaster,
            };
            let by = revision
                .map(|r| cx.source_of(&r.issuer.computer_id))
                .unwrap_or(ChangeSource::ThisComputer);
            let mut names = folder_names(p);
            if names.is_empty() {
                names = p
                    .settings_ops
                    .iter()
                    .map(|o| display_of(cx.titles, cx.inventory, &o.unit.folder))
                    .collect();
                names.sort();
                names.dedup();
            }
            let name = names
                .first()
                .map_or_else(|| "Saved settings".to_string(), |n| format!("{n} settings"));
            let mut rec = base(
                cx.stamp(),
                ChangeKind::SettingsApplied,
                name,
                names,
                by,
                reason,
            );
            rec.old_version = VersionLabel::ContentId(String::new());
            rec.new_version = VersionLabel::ContentId(String::new());
            rec.files.changed = p.settings_ops.len() as u32;
            return Some(rec);
        }
        OpKind::SettingsCleanup => return None,
    };
    let (paths, more) = capped(d.paths);
    let mut rec = base(
        cx.stamp(),
        kind,
        display_name(cx, p),
        folder_names(p),
        source,
        reason,
    );
    rec.old_version = old;
    rec.new_version = new;
    rec.files = d.counts;
    rec.byte_delta = d.byte_delta;
    rec.paths = paths;
    rec.paths_more = more;
    Some(rec)
}

pub(crate) fn base(
    stamp: (u64, i64),
    kind: ChangeKind,
    name: String,
    folders: Vec<String>,
    source: ChangeSource,
    reason: ChangeReason,
) -> ChangeRecord {
    ChangeRecord {
        unix_ms: stamp.1,
        pass_id: stamp.0,
        kind,
        name,
        folders,
        source,
        old_version: VersionLabel::New,
        new_version: VersionLabel::New,
        files: FileCounts::default(),
        byte_delta: 0,
        reason,
        paths: Vec::new(),
        paths_more: 0,
        ms: 0,
        identity: None,
    }
}

/// The owner chose to keep this release on the group.
pub(crate) fn kept(
    stamp: (u64, i64),
    c: &wow_sync_core::kinds::CodeChoice,
) -> Option<ChangeRecord> {
    let keep = c.keep.as_ref().filter(|_| c.action == ChoiceAction::Keep)?;
    let mut names: Vec<String> = keep.folders.iter().map(|f| f.name.clone()).collect();
    names.sort();
    let name = names.first().cloned().unwrap_or_default();
    Some(base(
        stamp,
        ChangeKind::Kept,
        name,
        names,
        ChangeSource::ThisComputer,
        ChangeReason::Choice(None),
    ))
}

/// The name the owner knows a folder by: its title, else its folder name as installed.
pub(crate) fn display_of(
    titles: &BTreeMap<String, String>,
    inventory: &Inventory,
    key: &FolderKey,
) -> String {
    if let Some(t) = titles.get(&key.as_str().to_ascii_lowercase()) {
        return clip(t);
    }
    inventory
        .get(key)
        .map_or_else(|| key.as_str().to_string(), |f| f.name.clone())
}

/// An interrupted apply that crash recovery finished. Only the result is known: the version
/// before the interruption is not, and is shown as an empty content id. `source` is where the
/// install came from as saved when it began; when that is missing (an older version wrote the
/// journal) it is shown as an unknown other computer, never as this one.
pub(crate) fn recovered(
    stamp: (u64, i64),
    key: &FolderKey,
    titles: &BTreeMap<String, String>,
    now: &Inventory,
    source: Option<ChangeSource>,
) -> ChangeRecord {
    let name = display_of(titles, now, key);
    let mut rec = base(
        stamp,
        ChangeKind::Applied,
        name.clone(),
        vec![name],
        source.unwrap_or_else(|| ChangeSource::Computer(UNKNOWN_COMPUTER.to_string())),
        ChangeReason::Recovery,
    );
    rec.old_version = VersionLabel::ContentId(String::new());
    rec.new_version = now.get(key).map_or(VersionLabel::New, label);
    rec
}

/// A saved-settings revision this computer just published.
pub(crate) fn settings_published(
    stamp: (u64, i64),
    r: &wow_sync_core::kinds::SettingsRevision,
    display: String,
) -> ChangeRecord {
    let folder = display;
    let mut rec = base(
        stamp,
        ChangeKind::SettingsPublished,
        format!("{folder} settings"),
        vec![folder],
        ChangeSource::ThisComputer,
        match r.kind {
            RevisionKind::Push => ChangeReason::PeerPush,
            _ => ChangeReason::SettingsMaster,
        },
    );
    rec.old_version = VersionLabel::ContentId(String::new());
    rec.new_version = VersionLabel::ContentId(String::new());
    rec.files.changed = 1;
    rec
}

/// Records for add-on folders this pass published: new or changed against the previously
/// published state (file by file, using `prev_files`), minus the folders this same pass
/// installed or removed for a peer. An add-on gone from `present` that the previous state
/// held is a removal made on this computer. Many at once (a first publish) become one record.
pub(crate) fn published(
    stamp: (u64, i64),
    previous: Option<&wow_sync_core::records::CodeState>,
    now: &Inventory,
    present: &Inventory,
    earlier_files: &dyn Fn(
        &wow_sync_core::ids::Sha256,
    ) -> Option<Vec<wow_sync_core::inventory::FileEntry>>,
    skip: &BTreeSet<FolderKey>,
) -> Vec<ChangeRecord> {
    const MAX_SEPARATE: usize = 25;
    let mut out = Vec::new();
    for f in &now.folders {
        if !f.transfer_eligible() || skip.contains(&f.key) {
            continue;
        }
        let before = previous.and_then(|s| s.folders.iter().find(|x| x.key == f.key));
        if before.is_some_and(|b| b.tree == f.tree) {
            continue;
        }
        let mut rec = base(
            stamp,
            ChangeKind::Published,
            f.name.clone(),
            vec![f.name.clone()],
            ChangeSource::ThisComputer,
            ChangeReason::NewerRelease,
        );
        rec.old_version = before.map_or(VersionLabel::New, label);
        rec.new_version = label(f);
        let (old_files, old_bytes) = before.map_or((0, 0), |b| (b.files, b.bytes));
        match before.and_then(|b| b.tree).and_then(|h| earlier_files(&h)) {
            Some(old) => {
                let (counts, paths) = file_diff(&old, &f.file_entries);
                rec.files = counts;
                let (paths, more) = capped(paths);
                rec.paths = paths;
                rec.paths_more = more;
            }
            // The earlier tree is not at hand: only the net difference is known.
            _ => {
                rec.files.added = f.files.saturating_sub(old_files);
                rec.files.removed = old_files.saturating_sub(f.files);
                rec.files.changed = f.files.min(old_files);
            }
        }
        rec.byte_delta = f.bytes as i64 - old_bytes as i64;
        out.push(rec);
    }
    for b in previous.map_or(&[][..], |s| s.folders.as_slice()) {
        if !b.transfer_eligible()
            || skip.contains(&b.key)
            || present.folders.iter().any(|f| f.key == b.key)
        {
            continue;
        }
        let mut rec = base(
            stamp,
            ChangeKind::Removed,
            b.name.clone(),
            vec![b.name.clone()],
            ChangeSource::ThisComputer,
            ChangeReason::Removal,
        );
        rec.old_version = label(b);
        rec.new_version = VersionLabel::Removed;
        rec.files.removed = b.files;
        rec.byte_delta = -(b.bytes as i64);
        out.push(rec);
    }
    if out.len() <= MAX_SEPARATE {
        return out;
    }
    let mut all = base(
        stamp,
        ChangeKind::Published,
        format!("{} add-ons", out.len()),
        out.iter()
            .take(MAX_RECORD_PATHS)
            .flat_map(|r| r.folders.clone())
            .collect(),
        ChangeSource::ThisComputer,
        ChangeReason::NewerRelease,
    );
    all.old_version = VersionLabel::New;
    all.new_version = VersionLabel::New;
    all.files.added = out.iter().map(|r| r.files.added).sum();
    all.files.removed = out.iter().map(|r| r.files.removed).sum();
    all.files.changed = out.iter().map(|r| r.files.changed).sum();
    all.byte_delta = out.iter().map(|r| r.byte_delta).sum();
    all.paths_more = out.len().saturating_sub(MAX_RECORD_PATHS) as u32;
    vec![all]
}
