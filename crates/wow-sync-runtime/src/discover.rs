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
        if out.iter().any(|d| &d.root == c) {
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

/// Well-known install locations on this computer.
pub fn wow_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(p) = probe::env_path(var) {
            v.push(p.join("World of Warcraft"));
        }
    }
    for drive in ["C", "D", "E", "F"] {
        v.push(PathBuf::from(format!("{drive}:\\World of Warcraft")));
        v.push(PathBuf::from(format!("{drive}:\\Games\\World of Warcraft")));
    }
    v
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
        std::fs::create_dir_all(format!("{}\\_classic_era_", wow.display())).unwrap();
        std::fs::create_dir_all(format!("{}\\Data", wow.display())).unwrap();
        let empty = t.join("Nothing");
        std::fs::create_dir_all(&empty).unwrap();
        let found = find_wow(&[wow.clone(), empty, t.join("missing"), wow.clone()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].variants, vec!["_classic_era_".to_string()]);
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
