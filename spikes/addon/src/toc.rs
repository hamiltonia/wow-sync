//! Text-only `.toc` parsing. Nothing is executed; values are untrusted data.

use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct Toc {
    /// Lower-cased directive name -> raw value (last occurrence wins).
    pub fields: BTreeMap<String, String>,
}

pub fn parse_toc(text: &str) -> Toc {
    let mut toc = Toc::default();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("##") else {
            continue;
        };
        let Some((k, v)) = rest.split_once(':') else {
            continue;
        };
        toc.fields
            .insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
    }
    toc
}

impl Toc {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    pub fn interfaces(&self) -> Vec<u32> {
        self.get("interface")
            .map(|v| v.split(',').filter_map(|p| p.trim().parse().ok()).collect())
            .unwrap_or_default()
    }

    pub fn deps(&self) -> Vec<String> {
        let mut out = Vec::new();
        for k in [
            "dependencies",
            "requireddeps",
            "dep1",
            "optionaldeps",
            "optionaldependencies",
        ] {
            if let Some(v) = self.get(k) {
                out.extend(
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty()),
                );
            }
        }
        out
    }

    pub fn load_on_demand(&self) -> bool {
        matches!(self.get("loadondemand"), Some("1"))
    }

    pub fn saved_variables(&self) -> bool {
        self.fields.keys().any(|k| k.starts_with("savedvariables"))
    }

    /// `X-*` ids relevant for cross-folder identity (key, value).
    pub fn package_ids(&self) -> Vec<(String, String)> {
        ["x-curse-project-id", "x-wowi-id", "x-wago-id", "x-github"]
            .iter()
            .filter_map(|k| self.get(k).map(|v| (k.to_string(), v.to_string())))
            .collect()
    }
}

/// Splits `Folder_Suffix.toc` / `Folder-Suffix.toc` into the flavor suffix.
/// Returns `None` for the plain `Folder.toc`, `Some(None)` is never produced;
/// a non-matching file name returns `Err`.
pub fn toc_suffix(folder: &str, file: &str) -> Result<Option<String>, ()> {
    let lf = file.to_ascii_lowercase();
    let base = lf.strip_suffix(".toc").ok_or(())?;
    let lfolder = folder.to_ascii_lowercase();
    if base == lfolder {
        return Ok(None);
    }
    let rest = base.strip_prefix(&lfolder).ok_or(())?;
    match rest.chars().next() {
        Some('_') | Some('-') => Ok(Some(file[folder.len() + 1..file.len() - 4].to_string())),
        _ => Err(()),
    }
}

/// Documented client suffixes (warcraft.wiki.gg TOC format). Camelot/Forever
/// are not documented and are treated as "unknown suffix".
pub const DOCUMENTED_SUFFIXES: &[&str] = &[
    "mainline",
    "vanilla",
    "classic",
    "tbc",
    "bcc",
    "wrath",
    "wotlkc",
    "cata",
    "mists",
    "mop",
    "xptr",
    "plunderstorm",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fields_and_lists() {
        let t = parse_toc(
            "\u{feff}## Interface: 110000, 50500\n## Version: @project-version@\n## X-Curse-Project-ID: 42\n## SavedVariables: A\n## Dependencies: Foo, Bar\n## LoadOnDemand: 1\nfile.lua\n",
        );
        assert_eq!(t.interfaces(), vec![110000, 50500]);
        assert_eq!(t.get("VERSION"), Some("@project-version@"));
        assert_eq!(t.deps(), vec!["Foo", "Bar"]);
        assert!(t.load_on_demand() && t.saved_variables());
        assert_eq!(t.package_ids().len(), 1);
    }

    #[test]
    fn suffix_selection() {
        assert_eq!(toc_suffix("Foo", "Foo.toc"), Ok(None));
        assert_eq!(
            toc_suffix("Foo", "Foo_Vanilla.toc"),
            Ok(Some("Vanilla".into()))
        );
        assert_eq!(
            toc_suffix("Foo", "Foo-Classic.toc"),
            Ok(Some("Classic".into()))
        );
        assert!(toc_suffix("Foo", "Foobar.toc").is_err());
        assert!(toc_suffix("Foo", "Foo.txt").is_err());
    }
}
