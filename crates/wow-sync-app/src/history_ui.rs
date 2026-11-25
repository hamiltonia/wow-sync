//! Change history for the Activity page and the add-on page (run-010 goal 1): turns the
//! runtime's change records into pass groups and per-add-on rows. Words only; no paths,
//! account names or file contents ever reach this text.

use wow_sync_runtime::{
    ChangeKind, ChangeReason, ChangeRecord, ChangeSource, FileCounts, VersionLabel,
};

/// One add-on change, ready to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeRow {
    pub name: String,
    /// Settings only: whose settings, in plain words ("account A", "A1 on Realm").
    pub identity: Option<String>,
    /// "1.2 → 1.3", "new", "removed"; may be empty for settings.
    pub what: String,
    pub files: String,
    pub size: String,
    pub reason: String,
    pub kind: ChangeKind,
    pub unix_ms: i64,
    /// Changed paths relative to the add-on folder, already capped.
    pub paths: Vec<String>,
    pub paths_more: u32,
    /// "from RYZENFISH", or "from this computer".
    pub from: String,
}

impl ChangeRow {
    /// The row with where the change came from, for lists that mix computers.
    pub fn line_with_source(&self) -> String {
        let mut parts: Vec<&str> = vec![self.name.as_str(), self.from.as_str()];
        for p in [&self.what, &self.files, &self.size, &self.reason] {
            if !p.is_empty() {
                parts.push(p);
            }
        }
        parts.join(" · ")
    }

    /// The whole row as one sentence, for the screen reader and the list.
    pub fn line(&self) -> String {
        let mut parts: Vec<&str> = vec![self.name.as_str()];
        if let Some(i) = &self.identity {
            parts.push(i);
        }
        for p in [&self.what, &self.files, &self.size, &self.reason] {
            if !p.is_empty() {
                parts.push(p);
            }
        }
        parts.join(" · ")
    }

    pub fn paths_text(&self) -> Vec<String> {
        let mut v = self.paths.clone();
        if self.paths_more > 0 {
            v.push(format!("and {} more", group_digits(self.paths_more.into())));
        }
        v
    }
}

pub fn group_digits(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "1.8 GB", "120 KB", "0 B": binary units, one decimal from MB up.
pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.0} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.1} GB", b / (KB * KB * KB))
    }
}

pub fn signed_size(delta: i64) -> String {
    match delta {
        0 => String::new(),
        d if d > 0 => format!("+{}", size_text(d.unsigned_abs())),
        d => format!("\u{2212}{}", size_text(d.unsigned_abs())),
    }
}

pub fn files_text(f: &FileCounts) -> String {
    let mut parts: Vec<String> = Vec::new();
    if f.added > 0 {
        parts.push(format!("+{}", group_digits(f.added.into())));
    }
    if f.changed > 0 {
        parts.push(format!("~{}", group_digits(f.changed.into())));
    }
    if f.removed > 0 {
        parts.push(format!("\u{2212}{}", group_digits(f.removed.into())));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{} files", parts.join(" "))
    }
}

fn version_text(v: &VersionLabel) -> String {
    match v {
        VersionLabel::New => "new".into(),
        VersionLabel::Removed => "removed".into(),
        VersionLabel::Toc(s) => s.clone(),
        VersionLabel::ContentId(s) if s.is_empty() => "unknown".into(),
        VersionLabel::ContentId(s) => format!("#{s}"),
    }
}

/// "old → new"; one word when the add-on appeared or went away.
pub fn what_text(r: &ChangeRecord) -> String {
    match (&r.old_version, &r.new_version) {
        (VersionLabel::New, VersionLabel::New) | (VersionLabel::Removed, VersionLabel::Removed) => {
            String::new()
        }
        (VersionLabel::New, new) => format!("new, {}", version_text(new)),
        (old, VersionLabel::Removed) => format!("removed, was {}", version_text(old)),
        (old, new) if old == new => version_text(new),
        (old, new) => format!("{} \u{2192} {}", version_text(old), version_text(new)),
    }
}

fn source_name(s: &ChangeSource) -> Option<&str> {
    match s {
        ChangeSource::ThisComputer => None,
        ChangeSource::Computer(n) => Some(n),
    }
}

pub fn reason_text(r: &ChangeRecord) -> String {
    let from = source_name(&r.source);
    match &r.reason {
        ChangeReason::NewerRelease => "newer release".into(),
        ChangeReason::Choice(None) => match from {
            Some(n) => format!("you chose {n}'s copy"),
            None => "you chose this computer's copy".into(),
        },
        ChangeReason::Choice(Some(n)) => format!("a choice made on {n}"),
        ChangeReason::SettingsMaster => "settings master".into(),
        ChangeReason::SharedFromHere => "shared from this computer".into(),
        ChangeReason::PeerPush => match from {
            Some(n) => format!("pushed from {n}"),
            None => "pushed to this computer".into(),
        },
        ChangeReason::Removal => match from {
            Some(n) => format!("removed on {n}"),
            None => "removed on this computer".into(),
        },
        ChangeReason::Rollback => match from {
            Some(n) => format!("rolled back on {n} for the group"),
            None => "rolled back on this computer for the group".into(),
        },
        ChangeReason::Recovery => "restored from recovery".into(),
    }
}

/// One add-on's own recent changes (newest first), matched by folder key or display name.
pub fn rows_for(
    records: &[ChangeRecord],
    key: &str,
    title: &str,
    limit: usize,
) -> Vec<(i64, ChangeRow)> {
    records
        .iter()
        .filter(|r| r.name == title || r.folders.iter().any(|f| f == key))
        .take(limit)
        .map(|r| (r.unix_ms, row_of(r)))
        .collect()
}

pub fn row_of(r: &ChangeRecord) -> ChangeRow {
    let what = match r.kind {
        ChangeKind::Kept => {
            let v = what_text(r);
            if v.is_empty() {
                "kept".into()
            } else {
                format!("kept {v}")
            }
        }
        ChangeKind::RolledBack => {
            let v = what_text(r);
            if v.is_empty() {
                "rolled back".into()
            } else {
                format!("rolled back, {v}")
            }
        }
        ChangeKind::Published | ChangeKind::SettingsPublished => {
            let v = what_text(r);
            if v.is_empty() {
                "shared".into()
            } else {
                format!("shared, {v}")
            }
        }
        _ => what_text(r),
    };
    ChangeRow {
        name: r.name.clone(),
        identity: r.identity.clone(),
        what,
        files: files_text(&r.files),
        size: signed_size(r.byte_delta),
        reason: reason_text(r),
        kind: r.kind,
        unix_ms: r.unix_ms,
        paths: r.paths.clone(),
        paths_more: r.paths_more,
        from: match source_name(&r.source) {
            Some(n) => format!("from {n}"),
            None => "from this computer".into(),
        },
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!(
        "{} {}",
        group_digits(n as u64),
        if n == 1 { one } else { many }
    )
}

/// "from RYZENFISH", "from 2 computers", or nothing when it all happened on this computer.
fn from_phrase(records: &[&ChangeRecord]) -> String {
    let mut names: Vec<&str> = records
        .iter()
        .filter_map(|r| source_name(&r.source))
        .collect();
    names.sort_unstable();
    names.dedup();
    match names.as_slice() {
        [] => String::new(),
        [one] => format!(" from {one}"),
        many => format!(" from {} computers", many.len()),
    }
}

/// "15 add-ons updated from RYZENFISH (+1.8 GB), 2 add-ons removed".
pub fn group_title(records: &[ChangeRecord]) -> String {
    let of = |ks: &[ChangeKind]| -> Vec<&ChangeRecord> {
        records.iter().filter(|r| ks.contains(&r.kind)).collect()
    };
    let updated = of(&[ChangeKind::Applied, ChangeKind::InstalledNew]);
    let removed = of(&[ChangeKind::Removed]);
    let rolled = of(&[ChangeKind::RolledBack]);
    let kept = of(&[ChangeKind::Kept]);
    let restored = of(&[ChangeKind::RecoveryRestored]);
    let shared = of(&[ChangeKind::Published]);
    let settings = of(&[ChangeKind::SettingsApplied, ChangeKind::SettingsPublished]);
    let delta = |rs: &[&ChangeRecord]| -> String {
        let d: i64 = rs.iter().map(|r| r.byte_delta).sum();
        match signed_size(d) {
            s if s.is_empty() => String::new(),
            s => format!(" ({s})"),
        }
    };
    let mut parts: Vec<String> = Vec::new();
    let one_name = |rs: &[&ChangeRecord]| rs.first().map(|r| r.name.clone()).unwrap_or_default();
    if !updated.is_empty() {
        let what = if updated.len() == 1 {
            one_name(&updated)
        } else {
            plural(updated.len(), "add-on", "add-ons")
        };
        parts.push(format!(
            "{what} updated{}{}",
            from_phrase(&updated),
            delta(&updated)
        ));
    }
    if !removed.is_empty() {
        let what = if removed.len() == 1 {
            one_name(&removed)
        } else {
            plural(removed.len(), "add-on", "add-ons")
        };
        parts.push(format!("{what} removed{}", from_phrase(&removed)));
    }
    if !restored.is_empty() {
        parts.push(format!(
            "{} restored from recovery",
            plural(restored.len(), "add-on", "add-ons")
        ));
    }
    if !rolled.is_empty() {
        parts.push(format!(
            "{} rolled back",
            plural(rolled.len(), "add-on", "add-ons")
        ));
    }
    if !kept.is_empty() {
        parts.push(format!(
            "kept your choice for {}",
            plural(kept.len(), "add-on", "add-ons")
        ));
    }
    if !shared.is_empty() {
        parts.push(format!(
            "{} shared from this computer",
            plural(shared.len(), "add-on", "add-ons")
        ));
    }
    if !settings.is_empty() {
        let pushed = settings
            .iter()
            .all(|r| r.kind == ChangeKind::SettingsPublished);
        parts.push(if pushed {
            "saved settings shared".into()
        } else {
            format!("saved settings updated{}", from_phrase(&settings))
        });
    }
    if parts.is_empty() {
        "Changes".into()
    } else {
        let mut t = parts.join(", ");
        if let Some(f) = t.get(..1) {
            t = f.to_uppercase() + &t[1..];
        }
        t
    }
}

/// "Updated 15 add-ons from RYZENFISH — see Activity": what the headline says right after a
/// pass that changed this computer. `None` when the pass changed nothing here.
pub fn done_note(records: &[ChangeRecord]) -> Option<String> {
    let updated: Vec<&ChangeRecord> = records
        .iter()
        .filter(|r| matches!(r.kind, ChangeKind::Applied | ChangeKind::InstalledNew))
        .collect();
    let removed = records
        .iter()
        .filter(|r| r.kind == ChangeKind::Removed)
        .count();
    let mut parts: Vec<String> = Vec::new();
    if !updated.is_empty() {
        let what = if updated.len() == 1 {
            updated[0].name.clone()
        } else {
            plural(updated.len(), "add-on", "add-ons")
        };
        parts.push(format!("Updated {what}{}", from_phrase(&updated)));
    }
    if removed > 0 {
        let what = plural(removed, "add-on", "add-ons");
        parts.push(if parts.is_empty() {
            format!("Removed {what}")
        } else {
            format!("removed {what}")
        });
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} \u{2014} see Activity", parts.join(", ")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn rec(kind: ChangeKind, name: &str, src: Option<&str>) -> ChangeRecord {
        ChangeRecord {
            unix_ms: 1_000,
            pass_id: 1,
            kind,
            name: name.into(),
            folders: vec![name.into()],
            source: match src {
                Some(s) => ChangeSource::Computer(s.into()),
                None => ChangeSource::ThisComputer,
            },
            old_version: VersionLabel::Toc("1.0".into()),
            new_version: VersionLabel::Toc("1.1".into()),
            files: FileCounts {
                added: 2,
                changed: 3,
                removed: 1,
            },
            byte_delta: 1_536,
            reason: ChangeReason::NewerRelease,
            paths: vec!["Core.lua".into()],
            paths_more: 0,
            ms: 0,
            identity: None,
        }
    }

    #[test]
    fn row_text_covers_versions_files_size_and_reason() {
        let r = row_of(&rec(ChangeKind::Applied, "Bagnon", Some("RYZENFISH")));
        assert_eq!(
            r.line(),
            "Bagnon · 1.0 \u{2192} 1.1 · +2 ~3 \u{2212}1 files · +2 KB · newer release"
        );
    }

    #[test]
    fn an_add_on_pages_rows_name_where_each_change_came_from() {
        let r = row_of(&rec(ChangeKind::Applied, "Bagnon", Some("RYZENFISH")));
        assert_eq!(
            r.line_with_source(),
            "Bagnon · from RYZENFISH · 1.0 \u{2192} 1.1 · +2 ~3 \u{2212}1 files · +2 KB · newer release"
        );
        let own = row_of(&rec(ChangeKind::Applied, "Bagnon", None));
        assert!(own.line_with_source().contains("from this computer"));
    }

    #[test]
    fn new_removed_and_content_id_versions() {
        let mut n = rec(ChangeKind::InstalledNew, "Nav", Some("A"));
        n.old_version = VersionLabel::New;
        n.new_version = VersionLabel::ContentId("ab12".into());
        assert_eq!(row_of(&n).what, "new, #ab12");
        let mut x = rec(ChangeKind::Removed, "Nav", Some("A"));
        x.old_version = VersionLabel::Toc("2.0".into());
        x.new_version = VersionLabel::Removed;
        x.byte_delta = -5 * 1024 * 1024;
        x.reason = ChangeReason::Removal;
        let r = row_of(&x);
        assert_eq!(r.what, "removed, was 2.0");
        assert_eq!(r.size, "\u{2212}5.0 MB");
        assert_eq!(r.reason, "removed on A");
    }

    #[test]
    fn a_recovered_install_with_no_old_content_id_says_unknown() {
        let mut r = rec(ChangeKind::Applied, "Bagnon", Some("A"));
        r.old_version = VersionLabel::ContentId(String::new());
        r.new_version = VersionLabel::Toc("1.2.0".into());
        assert_eq!(row_of(&r).what, "unknown \u{2192} 1.2.0");
        assert!(!row_of(&r).line().contains('#'));
    }

    #[test]
    fn every_reason_has_plain_words() {
        let mut r = rec(ChangeKind::Applied, "X", Some("RYZENFISH"));
        let cases = [
            (ChangeReason::NewerRelease, "newer release"),
            (ChangeReason::Choice(None), "you chose RYZENFISH's copy"),
            (
                ChangeReason::Choice(Some("DESK".into())),
                "a choice made on DESK",
            ),
            (ChangeReason::SettingsMaster, "settings master"),
            (ChangeReason::SharedFromHere, "shared from this computer"),
            (ChangeReason::PeerPush, "pushed from RYZENFISH"),
            (ChangeReason::Removal, "removed on RYZENFISH"),
            (
                ChangeReason::Rollback,
                "rolled back on RYZENFISH for the group",
            ),
            (ChangeReason::Recovery, "restored from recovery"),
        ];
        for (reason, want) in cases {
            r.reason = reason;
            assert_eq!(reason_text(&r), want);
        }
        r.source = ChangeSource::ThisComputer;
        r.reason = ChangeReason::Choice(None);
        assert_eq!(reason_text(&r), "you chose this computer's copy");
    }

    #[test]
    fn group_title_names_count_source_and_size() {
        let mut v: Vec<ChangeRecord> = (0..15)
            .map(|i| {
                rec(
                    ChangeKind::Applied,
                    &format!("Spoken{i}"),
                    Some("RYZENFISH"),
                )
            })
            .collect();
        for r in &mut v {
            r.byte_delta = 128 * 1024 * 1024;
        }
        assert_eq!(
            group_title(&v),
            "15 add-ons updated from RYZENFISH (+1.9 GB)"
        );
        v.push(rec(ChangeKind::Removed, "Old", Some("RYZENFISH")));
        assert!(
            group_title(&v).ends_with(", 1 add-on removed from RYZENFISH")
                || group_title(&v).ends_with(", Old removed from RYZENFISH")
        );
        assert_eq!(
            group_title(&[rec(ChangeKind::Applied, "Bagnon", None)]),
            "Bagnon updated (+2 KB)"
        );
        assert_eq!(
            group_title(&[rec(ChangeKind::SettingsPublished, "Saved settings", None)]),
            "Saved settings shared"
        );
    }

    #[test]
    fn done_note_is_none_when_nothing_changed_here() {
        assert_eq!(done_note(&[rec(ChangeKind::Published, "A", None)]), None);
        let v: Vec<ChangeRecord> = (0..15)
            .map(|i| rec(ChangeKind::Applied, &format!("S{i}"), Some("RYZENFISH")))
            .collect();
        assert_eq!(
            done_note(&v).as_deref(),
            Some("Updated 15 add-ons from RYZENFISH \u{2014} see Activity")
        );
    }

    #[test]
    fn a_settings_row_names_whose_settings_changed() {
        let mut r = rec(ChangeKind::SettingsApplied, "Details", Some("Den PC"));
        let row = row_of(&r);
        assert_eq!(row.identity, None);
        assert!(!row.line().contains("account"));
        r.identity = Some("account A".into());
        let row = row_of(&r);
        assert!(row.line().starts_with("Details · account A"));
    }

    #[test]
    fn file_paths_are_capped_with_and_n_more() {
        let mut r = rec(ChangeKind::Applied, "A", None);
        r.paths_more = 1_200;
        let t = row_of(&r).paths_text();
        assert_eq!(t.last().unwrap(), "and 1,200 more");
    }
}
