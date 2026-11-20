//! Observation of WTF settings files (settings.md §1.1, §1.3). Only the two documented shapes
//! under `WTF\Account` are enumerated; only their size and SHA-256 are read, never parsed or
//! executed. Reparse points anywhere on the way mark the entry unsafe and nothing under them is
//! read. Other files under `WTF` are not opened.

use std::collections::BTreeSet;
use std::path::Path;

use crate::fs_safe::{check_entry, sha256_stream, EntryMeta, ReparsePolicy, Root, WtfScope};
use crate::{FsError, FsErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WtfFile {
    /// `/`-separated, relative to `WTF`.
    pub rel: String,
    pub size: u64,
    pub sha256: [u8; 32],
    /// A component is a link, junction or other reparse point; no content was read.
    pub unsafe_path: bool,
}

fn is_settings_name(name: &str) -> bool {
    name.ends_with(".lua") && !name.to_ascii_lowercase().starts_with("blizzard_")
}

/// Lists `Account/<A>/SavedVariables/*.lua` and `Account/<A>/<Realm>/<Char>/SavedVariables/*.lua`
/// for the included accounts (`included` holds the exact folder names). `size_cap` is the saved
/// file cap; a larger file is reported with its size but is not read (hash is all zero).
pub fn scan_wtf(
    variant_dir: &Path,
    included: &BTreeSet<String>,
    size_cap: Option<u64>,
) -> Result<Vec<WtfFile>, FsError> {
    let Some(scope) = WtfScope::open(variant_dir, false)? else {
        return Ok(Vec::new());
    };
    let root = scope.root();
    let mut out = Vec::new();
    let accounts = match root.list("Account") {
        Ok(a) => a,
        Err(e) if e.kind == FsErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    let mut accounts: Vec<EntryMeta> = accounts.into_iter().filter(|m| m.is_dir).collect();
    accounts.sort_by(|a, b| a.name.cmp(&b.name));
    for a in accounts {
        if !included.contains(&a.name) {
            continue;
        }
        let base = format!("Account/{}", a.name);
        if unsafe_entry(root, &base) {
            out.push(unsafe_file(&format!("{base}/SavedVariables")));
            continue;
        }
        let mut children = root.list(&base)?;
        children.sort_by(|x, y| x.name.cmp(&y.name));
        for c in children.into_iter().filter(|c| c.is_dir) {
            let cp = format!("{base}/{}", c.name);
            if c.name == "SavedVariables" {
                collect(root, &cp, size_cap, &mut out)?;
                continue;
            }
            // a realm folder: its children are characters
            if unsafe_entry(root, &cp) {
                out.push(unsafe_file(&format!("{cp}/SavedVariables")));
                continue;
            }
            let mut chars = root.list(&cp)?;
            chars.sort_by(|x, y| x.name.cmp(&y.name));
            for ch in chars.into_iter().filter(|c| c.is_dir) {
                let chp = format!("{cp}/{}", ch.name);
                let sv = format!("{chp}/SavedVariables");
                if unsafe_entry(root, &chp) {
                    out.push(unsafe_file(&sv));
                    continue;
                }
                match root.list(&chp) {
                    Ok(inner) if inner.iter().any(|m| m.name == "SavedVariables") => {
                        collect(root, &sv, size_cap, &mut out)?
                    }
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Ok(out)
}

fn unsafe_entry(root: &Root, rel: &str) -> bool {
    match root.check_components(rel) {
        Ok(m) => check_entry(&m, ReparsePolicy::RejectAll, Path::new(rel)).is_err(),
        Err(_) => true,
    }
}

fn unsafe_file(rel: &str) -> WtfFile {
    WtfFile {
        rel: rel.to_string(),
        size: 0,
        sha256: [0; 32],
        unsafe_path: true,
    }
}

fn collect(
    root: &Root,
    dir: &str,
    size_cap: Option<u64>,
    out: &mut Vec<WtfFile>,
) -> Result<(), FsError> {
    if unsafe_entry(root, dir) {
        out.push(unsafe_file(dir));
        return Ok(());
    }
    let mut files = root.list(dir)?;
    files.sort_by(|a, b| a.name.cmp(&b.name));
    for f in files {
        if f.is_dir || !is_settings_name(&f.name) {
            continue;
        }
        let rel = format!("{dir}/{}", f.name);
        if check_entry(&f, ReparsePolicy::RejectAll, Path::new(&rel)).is_err() {
            out.push(unsafe_file(&rel));
            continue;
        }
        if size_cap.is_some_and(|c| f.size > c) {
            out.push(WtfFile {
                rel,
                size: f.size,
                sha256: [0; 32],
                unsafe_path: false,
            });
            continue;
        }
        let limit = size_cap.unwrap_or(u64::MAX);
        let mut file = root.open_read(&rel, limit)?;
        let path = root.join(&rel)?;
        let (size, sha256) = sha256_stream(&path, &mut file, limit)?;
        out.push(WtfFile {
            rel,
            size,
            sha256,
            unsafe_path: false,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TempDir;
    use std::fs;

    fn put(base: &Path, rel: &str, bytes: &[u8]) {
        let p = rel.split('/').fold(base.to_path_buf(), |a, s| a.join(s));
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    fn included(n: &[&str]) -> BTreeSet<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn lists_only_the_two_shapes_with_size_and_hash() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        put(
            &wtf,
            "Account/ERIC/SavedVariables/Details.lua",
            b"DetailsDB = {}",
        );
        put(
            &wtf,
            "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua",
            b"x = 1",
        );
        put(&wtf, "Account/ERIC/SavedVariables/Details.lua.bak", b"old");
        put(&wtf, "Account/ERIC/SavedVariables/Blizzard_Foo.lua", b"ui");
        put(&wtf, "Account/ERIC/config-cache.wtf", b"cfg");
        put(
            &wtf,
            "Account/ERIC/Argent Dawn/Thrall/layout-local.txt",
            b"lay",
        );
        put(&wtf, "Account/OTHER/SavedVariables/Details.lua", b"other");
        put(&wtf, "Config.wtf", b"c");
        let got = scan_wtf(t.path(), &included(&["ERIC"]), Some(1024)).unwrap();
        let rels: Vec<&str> = got.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(
            rels,
            vec![
                "Account/ERIC/Argent Dawn/Thrall/SavedVariables/Details.lua",
                "Account/ERIC/SavedVariables/Details.lua"
            ]
        );
        let acct = got
            .iter()
            .find(|f| f.rel.ends_with("ERIC/SavedVariables/Details.lua"))
            .unwrap();
        assert_eq!(acct.size, 14);
        let expect = {
            use sha2::{Digest, Sha256};
            let d: [u8; 32] = Sha256::digest(b"DetailsDB = {}").into();
            d
        };
        assert_eq!(acct.sha256, expect);
    }

    #[test]
    fn an_oversized_file_is_listed_but_not_read_and_no_wtf_is_empty() {
        let t = TempDir::new();
        assert!(scan_wtf(t.path(), &included(&["ERIC"]), Some(8))
            .unwrap()
            .is_empty());
        let wtf = t.path().join("WTF");
        put(&wtf, "Account/ERIC/SavedVariables/Big.lua", &[b'a'; 64]);
        let got = scan_wtf(t.path(), &included(&["ERIC"]), Some(8)).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].size, 64);
        assert_eq!(got[0].sha256, [0; 32]);
        assert!(!got[0].unsafe_path);
    }

    #[test]
    fn a_file_that_would_run_code_is_only_hashed() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        let marker = t.path().join("executed.txt");
        let lua = format!("os.execute('echo x > \"{}\"')", marker.display());
        put(&wtf, "Account/ERIC/SavedVariables/Evil.lua", lua.as_bytes());
        let got = scan_wtf(t.path(), &included(&["ERIC"]), Some(4096)).unwrap();
        assert_eq!(got.len(), 1);
        assert!(!marker.exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_at_an_account_is_unsafe_and_nothing_beneath_it_is_read() {
        let t = TempDir::new();
        let wtf = t.path().join("WTF");
        let outside = t.path().join("outside");
        put(&outside, "SavedVariables/Leak.lua", b"secret");
        fs::create_dir_all(wtf.join("Account")).unwrap();
        let link = wtf.join("Account").join("ERIC");
        let st = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(st.status.success(), "mklink failed");
        let got = scan_wtf(t.path(), &included(&["ERIC"]), Some(4096)).unwrap();
        assert!(got.iter().all(|f| f.unsafe_path), "{got:?}");
        assert!(!got.iter().any(|f| f.rel.contains("Leak")));
        // remove the link itself, never its target
        fs::remove_dir(&link).unwrap();
        assert!(outside.join("SavedVariables").join("Leak.lua").exists());
    }
}
