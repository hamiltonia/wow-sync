//! Minimal `.toc` metadata reader (declarations only; never loads addon code).

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TocDecl {
    pub account: BTreeSet<String>,
    pub character: BTreeSet<String>,
}

pub fn parse_toc(src: &[u8]) -> TocDecl {
    let src = src.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(src);
    let text = String::from_utf8_lossy(src);
    let mut decl = TocDecl::default();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("##") else {
            continue;
        };
        let Some((key, value)) = rest.split_once(':') else {
            continue;
        };
        let target = match key.trim().to_ascii_lowercase().as_str() {
            "savedvariables" => &mut decl.account,
            "savedvariablespercharacter" => &mut decl.character,
            _ => continue,
        };
        target.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(String::from),
        );
    }
    decl
}

/// Declarations of one addon folder across all of its flavor `.toc` files.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AddonDecl {
    pub union: TocDecl,
    pub intersection: TocDecl,
    pub toc_count: usize,
}

impl AddonDecl {
    pub fn from_tocs(tocs: &[TocDecl]) -> Self {
        let mut d = AddonDecl {
            toc_count: tocs.len(),
            ..Default::default()
        };
        for (i, t) in tocs.iter().enumerate() {
            d.union.account.extend(t.account.iter().cloned());
            d.union.character.extend(t.character.iter().cloned());
            if i == 0 {
                d.intersection = t.clone();
            } else {
                d.intersection.account.retain(|v| t.account.contains(v));
                d.intersection.character.retain(|v| t.character.contains(v));
            }
        }
        d
    }

    pub fn flavor_dependent(&self) -> bool {
        self.union != self.intersection
    }
}
