//! Read-only detection of where the game and Dropbox live (UG-24). It only lists directories
//! and reads Dropbox's own `info.json`; nothing is written and the game is never started.

use std::path::{Path, PathBuf};
use wow_sync_adapters::probe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedInstall {
    pub root: PathBuf,
    /// Variant folders found under the root, e.g. `_retail_`, `_classic_era_`.
    pub variants: Vec<String>,
}

fn variant_folders(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = probe::subdir_names(root)
        .into_iter()
        .filter(|n| n.len() > 2 && n.starts_with('_') && n.ends_with('_'))
        .collect();
    out.sort();
    out
}

/// Candidate roots that exist and hold at least one variant folder.
pub fn find_wow(candidates: &[PathBuf]) -> Vec<DetectedInstall> {
    let mut out: Vec<DetectedInstall> = Vec::new();
    for c in candidates {
        let key = path_key(c);
        if out.iter().any(|d| path_key(&d.root) == key) {
            continue;
        }
        let variants = variant_folders(c);
        if !variants.is_empty() {
            out.push(DetectedInstall {
                root: c.clone(),
                variants,
            });
        }
    }
    out
}

/// Case-insensitive identity of a path for de-duplication (Windows paths are case-blind).
fn path_key(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// `Battle.net`'s product database is a few KiB; anything over this is not read.
pub const PRODUCT_DB_LIMIT: u64 = 4 * 1024 * 1024;

const LOCATIONS: [&str; 4] = [
    "World of Warcraft",
    "Games\\World of Warcraft",
    "Program Files (x86)\\World of Warcraft",
    "Program Files\\World of Warcraft",
];

/// One place that was looked at, and why it is on the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub source: String,
    pub path: PathBuf,
    pub result: ProbeResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    /// A game folder with this many variant folders.
    Found(usize),
    /// The folder exists but has no `_variant_` sub-folder, so it is not a game install.
    NoVariants,
    Missing,
}

impl ProbeResult {
    pub fn word(&self) -> &'static str {
        match self {
            ProbeResult::Found(_) => "found",
            ProbeResult::NoVariants => "no variants",
            ProbeResult::Missing => "missing",
        }
    }
}

/// Everything a detection run saw, so the app can log it.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub installs: Vec<DetectedInstall>,
    pub probes: Vec<Probe>,
    /// Fixed drive letters that were searched.
    pub drives: Vec<char>,
    /// Game paths recorded by Battle.net; `None` when its database is absent, unreadable or
    /// over the size limit.
    pub battlenet: Option<Vec<PathBuf>>,
}

/// The ordered list of places to look. Battle.net's own record comes first.
pub fn probe_list(
    battlenet: &[PathBuf],
    program_files: &[PathBuf],
    drives: &[char],
) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    for p in battlenet {
        v.push(("Battle.net".into(), p.clone()));
    }
    for p in program_files {
        v.push(("Program Files".into(), p.join("World of Warcraft")));
    }
    for d in drives {
        for l in LOCATIONS {
            v.push((format!("drive {d}"), PathBuf::from(format!("{d}:\\{l}"))));
        }
    }
    v
}

/// Looks at every listed place, read-only. Places that repeat (case-blind) are looked at once.
pub fn scan(list: Vec<(String, PathBuf)>) -> Discovery {
    let mut d = Discovery::default();
    let mut seen: Vec<String> = Vec::new();
    for (source, path) in list {
        let key = path_key(&path);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let variants = variant_folders(&path);
        let result = if !variants.is_empty() {
            ProbeResult::Found(variants.len())
        } else if probe::is_dir(&path) {
            ProbeResult::NoVariants
        } else {
            ProbeResult::Missing
        };
        if !variants.is_empty() {
            d.installs.push(DetectedInstall {
                root: path.clone(),
                variants,
            });
        }
        d.probes.push(Probe {
            source,
            path,
            result,
        });
    }
    d
}

/// Finds the game on this computer: Battle.net's recorded path, the usual Program Files
/// folders and four usual folders on every local fixed drive. Nothing is written.
pub fn discover_wow() -> Discovery {
    let drives = probe::fixed_drive_letters();
    let battlenet = probe::env_path("ProgramData").and_then(|p| {
        let db = p.join("Battle.net").join("Agent").join("product.db");
        probe::read_bytes_bounded(&db, PRODUCT_DB_LIMIT).map(|b| paths_from_product_db(&b))
    });
    // A path on a network or removable drive is not probed, same as the drive search.
    let usable: Vec<PathBuf> = battlenet
        .iter()
        .flatten()
        .filter(|p| {
            drive_letter(p).is_some_and(|c| drives.iter().any(|d| d.eq_ignore_ascii_case(&c)))
        })
        .cloned()
        .collect();
    let pf: Vec<PathBuf> = ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(|v| probe::env_path(v))
        .collect();
    let mut d = scan(probe_list(&usable, &pf, &drives));
    d.drives = drives;
    d.battlenet = battlenet;
    d
}

fn drive_letter(p: &Path) -> Option<char> {
    let s = p.to_string_lossy();
    let mut c = s.chars();
    let l = c.next().filter(char::is_ascii_alphabetic)?;
    (c.next() == Some(':')).then_some(l)
}

/// Well-known install locations on this computer (Battle.net's record is not included).
pub fn wow_candidates() -> Vec<PathBuf> {
    let pf: Vec<PathBuf> = ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(|v| probe::env_path(v))
        .collect();
    probe_list(&[], &pf, &probe::fixed_drive_letters())
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

fn path_byte(b: u8) -> bool {
    b >= 0x20 && b != 0x7f && !b"<>\"|?*".contains(&b)
}

/// Absolute `X:\...\World of Warcraft` paths found anywhere in Battle.net's product database.
/// The file is untrusted binary data: it is only scanned for text, never parsed or run.
pub fn paths_from_product_db(bytes: &[u8]) -> Vec<PathBuf> {
    const NEEDLE: &[u8] = b"World of Warcraft";
    const MAX_PATHS: usize = 32;
    const MAX_LEN: usize = 400;
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i + NEEDLE.len() <= bytes.len() && out.len() < MAX_PATHS {
        if !bytes[i..i + NEEDLE.len()].eq_ignore_ascii_case(NEEDLE) {
            i += 1;
            continue;
        }
        let end = i + NEEDLE.len();
        let prev = i.checked_sub(1).map(|k| bytes[k]);
        let next_ok = bytes
            .get(end)
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b' '));
        let mut start = i;
        while start > 0 && end - start < MAX_LEN && path_byte(bytes[start - 1]) {
            start -= 1;
        }
        i = end;
        if !next_ok || !matches!(prev, Some(b'\\' | b'/')) {
            continue;
        }
        let seg = &bytes[start..end];
        let Some(j) = (0..seg.len().saturating_sub(2)).find(|&j| {
            seg[j].is_ascii_alphabetic() && seg[j + 1] == b':' && matches!(seg[j + 2], b'\\' | b'/')
        }) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&seg[j..]) else {
            continue;
        };
        let norm = normalize_win_path(text);
        if norm.split('\\').any(|c| c == "..") {
            continue;
        }
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&norm)) {
            out.push(norm);
        }
    }
    out.into_iter().map(PathBuf::from).collect()
}

fn normalize_win_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let c = if c == '/' { '\\' } else { c };
        if !(c == '\\' && out.ends_with('\\')) {
            out.push(c);
        }
    }
    out.trim_end_matches('\\').to_string()
}

/// What a person's folder choice means: an install root, or a `_variant_` folder inside one.
/// `variant` is the chosen folder's own name when a variant folder was picked.
pub fn install_from_choice(chosen: &Path) -> Option<(DetectedInstall, Option<String>)> {
    if let Some(d) = find_wow(&[chosen.to_path_buf()]).into_iter().next() {
        return Some((d, None));
    }
    let name = chosen.file_name()?.to_str()?.to_string();
    if !(name.len() > 2 && name.starts_with('_') && name.ends_with('_')) {
        return None;
    }
    let parent = chosen.parent()?;
    let d = find_wow(&[parent.to_path_buf()]).into_iter().next()?;
    d.variants.contains(&name).then_some((d, Some(name)))
}

/// The `"path"` values of Dropbox's `info.json`, in file order. A tiny reader: the file is
/// Dropbox's own and is treated as untrusted text.
pub fn dropbox_paths_from_info(text: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("\"path\"") {
        rest = &rest[i + 6..];
        let Some(colon) = rest.find(':') else { break };
        let after = rest[colon + 1..].trim_start();
        let Some(body) = after.strip_prefix('"') else {
            continue;
        };
        let mut s = String::new();
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some(o) => {
                        s.push('\\');
                        s.push(o);
                    }
                    None => break,
                },
                o => s.push(o),
            }
        }
        if !s.is_empty() {
            out.push(PathBuf::from(s));
        }
    }
    out
}

/// Dropbox folders that exist, from Dropbox's info files and the default location.
pub fn find_dropbox(info_files: &[PathBuf], defaults: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if probe::is_dir(&p) && !out.contains(&p) {
            out.push(p);
        }
    };
    for f in info_files {
        if let Some(text) = probe::read_text_bounded(f, probe::INFO_JSON_LIMIT) {
            for p in dropbox_paths_from_info(&text) {
                push(p);
            }
        }
    }
    for d in defaults {
        push(d.clone());
    }
    out
}

pub fn dropbox_info_files() -> Vec<PathBuf> {
    ["APPDATA", "LOCALAPPDATA"]
        .iter()
        .filter_map(|v| probe::env_path(v))
        .map(|p| p.join("Dropbox").join("info.json"))
        .collect()
}

pub fn dropbox_defaults() -> Vec<PathBuf> {
    probe::env_path("USERPROFILE")
        .map(|p| vec![p.join("Dropbox")])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;

    #[test]
    fn finds_only_roots_with_variant_folders() {
        let t = TempDir::new();
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_classic_era_")).unwrap();
        std::fs::create_dir_all(wow.join("Data")).unwrap();
        let empty = t.join("Nothing");
        std::fs::create_dir_all(&empty).unwrap();
        let found = find_wow(&[wow.clone(), empty, t.join("missing"), wow.clone()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].variants, vec!["_classic_era_".to_string()]);
    }

    #[test]
    fn product_db_paths_are_extracted_from_untrusted_bytes() {
        let mut b: Vec<u8> = vec![0x0a, 0x03, b'w', b'o', b'w', 0x12, 0x1b];
        b.extend_from_slice(b"G:/Games/World of Warcraft");
        b.extend_from_slice(&[0x1a, 0x00, 0xff, 0xfe]);
        b.extend_from_slice(b"\x12\x20D:\\Blizzard\\World of Warcraft\\_retail_\x00");
        b.extend_from_slice(b"\x12\x20g:\\GAMES\\world of warcraft\x22");
        b.extend_from_slice(b"\x12\x20C:\\My World of Warcraft Beta\x00");
        b.extend_from_slice(b"\x12\x20\\\\server\\share\\World of Warcraft\x00");
        b.extend_from_slice(b"\x12\x20C:\\x\\..\\World of Warcraft\x00");
        let p = paths_from_product_db(&b);
        assert_eq!(
            p,
            vec![
                PathBuf::from("G:\\Games\\World of Warcraft"),
                PathBuf::from("D:\\Blizzard\\World of Warcraft"),
            ],
            "slash styles are normalised, repeats collapse case-blind, and anything that is not an absolute drive path is dropped"
        );
    }

    #[test]
    fn product_db_garbage_and_empty_input_find_nothing() {
        assert!(paths_from_product_db(&[]).is_empty());
        assert!(paths_from_product_db(&[0xff; 5000]).is_empty());
        assert!(paths_from_product_db(b"World of Warcraft").is_empty());
        assert!(paths_from_product_db(b"\\World of Warcraft").is_empty());
        let mut b = vec![0x80u8; 10];
        b.extend_from_slice(b"C:\\\xff\\World of Warcraft");
        assert!(paths_from_product_db(&b).is_empty(), "not UTF-8");
    }

    #[test]
    fn product_db_scan_is_bounded_on_a_huge_file() {
        let mut b = Vec::new();
        for _ in 0..200_000 {
            b.extend_from_slice(b"X:\\World of Warcraft\x00");
        }
        assert!(b.len() as u64 > 3 * 1024 * 1024);
        let p = paths_from_product_db(&b);
        assert_eq!(p, vec![PathBuf::from("X:\\World of Warcraft")]);
        let mut many = Vec::new();
        for i in 0..200u8 {
            many.extend_from_slice(format!("E:\\p{i}\\World of Warcraft\x00").as_bytes());
        }
        assert_eq!(paths_from_product_db(&many).len(), 32);
    }

    #[test]
    fn a_huge_or_missing_product_db_is_not_read() {
        let t = TempDir::new();
        let db = t.join("product.db");
        assert!(probe::read_bytes_bounded(&db, PRODUCT_DB_LIMIT).is_none());
        std::fs::write(&db, vec![b'a'; PRODUCT_DB_LIMIT as usize + 1]).unwrap();
        assert!(probe::read_bytes_bounded(&db, PRODUCT_DB_LIMIT).is_none());
    }

    #[test]
    fn the_probe_list_has_the_four_places_per_drive_and_scan_reports_each() {
        let l = probe_list(
            &[PathBuf::from("G:\\Games\\World of Warcraft")],
            &[],
            &['C', 'G'],
        );
        assert_eq!(l.len(), 1 + 8);
        assert_eq!(l[0].0, "Battle.net");
        for want in [
            "G:\\World of Warcraft",
            "G:\\Games\\World of Warcraft",
            "G:\\Program Files (x86)\\World of Warcraft",
            "G:\\Program Files\\World of Warcraft",
        ] {
            assert!(l.iter().any(|(_, p)| p == &PathBuf::from(want)), "{want}");
        }
        let t = TempDir::new();
        let ok = t.join("a");
        std::fs::create_dir_all(ok.join("_forever_")).unwrap();
        let bare = t.join("b");
        std::fs::create_dir_all(bare.join("Data")).unwrap();
        let d = scan(vec![
            ("x".into(), ok.clone()),
            (
                "y".into(),
                PathBuf::from(ok.to_string_lossy().to_uppercase()),
            ),
            ("z".into(), bare),
            ("w".into(), t.join("nope")),
        ]);
        assert_eq!(d.installs.len(), 1);
        let r: Vec<&ProbeResult> = d.probes.iter().map(|p| &p.result).collect();
        assert_eq!(
            r,
            vec![
                &ProbeResult::Found(1),
                &ProbeResult::NoVariants,
                &ProbeResult::Missing
            ],
            "the case-only repeat is one probe"
        );
    }

    #[test]
    fn a_chosen_folder_may_be_a_root_or_a_variant_folder() {
        let t = TempDir::new();
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_forever_")).unwrap();
        std::fs::create_dir_all(wow.join("_retail_")).unwrap();
        std::fs::create_dir_all(wow.join("Data")).unwrap();
        let (d, v) = install_from_choice(&wow).unwrap();
        assert_eq!((d.root.clone(), v), (wow.clone(), None));
        let (d, v) = install_from_choice(&wow.join("_forever_")).unwrap();
        assert_eq!(d.root, wow);
        assert_eq!(v.as_deref(), Some("_forever_"));
        assert!(install_from_choice(&wow.join("Data")).is_none());
        assert!(install_from_choice(&t.join("missing")).is_none());
        let lone = t.join("Other").join("_x_");
        std::fs::create_dir_all(&lone).unwrap();
        let (d, v) = install_from_choice(&lone).unwrap();
        assert_eq!(d.root, t.join("Other"));
        assert_eq!(v.as_deref(), Some("_x_"));
        let (d, v) = install_from_choice(&t.join("Other")).unwrap();
        assert_eq!((d.root, v), (t.join("Other"), None));
    }

    #[test]
    fn dropbox_info_paths_are_read_without_trusting_the_file() {
        let text = r#"{"personal": {"path": "C:\\Users\\x\\Dropbox", "host": 1}, "business": {"path": "D:\\Team"}}"#;
        let p = dropbox_paths_from_info(text);
        assert_eq!(
            p,
            vec![
                PathBuf::from("C:\\Users\\x\\Dropbox"),
                PathBuf::from("D:\\Team")
            ]
        );
        assert!(dropbox_paths_from_info("garbage {").is_empty());
    }

    #[test]
    fn an_oversized_dropbox_info_file_is_not_read() {
        let t = TempDir::new();
        let real = t.join("Dropbox");
        std::fs::create_dir_all(&real).unwrap();
        let info = t.join("info.json");
        let mut body = format!(
            "{{\"personal\": {{\"path\": \"{}\"}}}}",
            real.display().to_string().replace('\\', "\\\\")
        );
        assert_eq!(find_dropbox(std::slice::from_ref(&info), &[]).len(), 0);
        std::fs::write(&info, &body).unwrap();
        assert_eq!(find_dropbox(std::slice::from_ref(&info), &[]), vec![real]);
        body.push_str(&" ".repeat(probe::INFO_JSON_LIMIT as usize));
        std::fs::write(&info, &body).unwrap();
        assert!(find_dropbox(&[info], &[]).is_empty());
    }

    #[test]
    fn dropbox_lookup_keeps_only_existing_folders() {
        let t = TempDir::new();
        let real = t.join("Dropbox");
        std::fs::create_dir_all(&real).unwrap();
        let info = t.join("info.json");
        std::fs::write(
            &info,
            format!(
                "{{\"personal\": {{\"path\": \"{}\"}}}}",
                real.display().to_string().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        let found = find_dropbox(&[info], &[t.join("nope")]);
        assert_eq!(found, vec![real]);
    }
}
