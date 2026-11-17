//! `.toc` header parsing and the single selection rule (domain.md §3).

use crate::variant::{FileObs, TocMapping};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdKind {
    Curse,
    Wago,
    #[serde(rename = "wowi")]
    WoWI,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalId {
    pub kind: IdKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TocInfo {
    pub interface: Vec<u32>,
    pub version: Option<String>,
    pub title: Option<String>,
    pub hard_deps: Vec<String>,
    pub ids: Vec<ExternalId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TocError {
    TooLarge,
    NotUtf8,
}

const TOC_LIMIT: usize = 256 * 1024;

pub fn parse_toc(bytes: &[u8]) -> Result<TocInfo, TocError> {
    if bytes.len() > TOC_LIMIT {
        return Err(TocError::TooLarge);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| TocError::NotUtf8)?;
    let mut info = TocInfo::default();
    for raw in text.lines() {
        let Some(rest) = raw.trim_start().strip_prefix("##") else {
            continue;
        };
        let Some((key, value)) = rest.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "interface" => {
                info.interface.extend(
                    value
                        .split(',')
                        .filter_map(|p| p.trim().parse::<u32>().ok()),
                );
            }
            "version" => {
                if info.version.is_none() {
                    info.version = Some(value.to_string());
                }
            }
            "title" => {
                if info.title.is_none() {
                    info.title = Some(value.to_string());
                }
            }
            "x-curse-project-id" => push_id(&mut info, IdKind::Curse, value),
            "x-wago-id" => push_id(&mut info, IdKind::Wago, value),
            "x-wowi-id" => push_id(&mut info, IdKind::WoWI, value),
            k if k == "dependencies" || k == "requireddeps" || k.starts_with("dep") => {
                info.hard_deps.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|d| !d.is_empty())
                        .map(str::to_string),
                )
            }
            _ => {}
        }
    }
    Ok(info)
}

fn push_id(info: &mut TocInfo, kind: IdKind, value: &str) {
    if !value.is_empty() {
        info.ids.push(ExternalId {
            kind,
            value: value.to_string(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocCandidate {
    pub file_name: String,
    pub contents: FileObs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TocSelection {
    Selected {
        file_name: String,
        suffix: Option<String>,
        info: TocInfo,
    },
    None {
        reason: String,
    },
    Ambiguous {
        files: Vec<String>,
    },
    Unvalidated {
        files: Vec<String>,
    },
    Error {
        file_name: String,
        reason: String,
    },
}

/// `Some(None)` = plain `F.toc`, `Some(Some(s))` = suffixed, `None` = not a candidate.
fn candidate_kind(folder: &str, file: &str) -> Option<Option<String>> {
    let lf = file.to_ascii_lowercase();
    let lfolder = folder.to_ascii_lowercase();
    let stem = lf.strip_suffix(".toc")?;
    if stem == lfolder {
        return Some(None);
    }
    let rest = stem.strip_prefix(&lfolder)?;
    let suffix = rest.strip_prefix('_').or_else(|| rest.strip_prefix('-'))?;
    if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_alphanumeric()) {
        let original = &file[file.len() - 4 - suffix.len()..file.len() - 4];
        return Some(Some(original.to_string()));
    }
    None
}

/// The only `.toc` selection rule. In the slice every client uses the plain rule: a folder
/// with any suffixed candidate is `Unvalidated`, whatever the mapping says (Q-3, Q-20).
pub fn select_toc(
    folder: &str,
    candidates: &[TocCandidate],
    _mapping: &TocMapping,
) -> TocSelection {
    let mut plain: Vec<&TocCandidate> = Vec::new();
    let mut suffixed: Vec<String> = Vec::new();
    for c in candidates {
        match candidate_kind(folder, &c.file_name) {
            Some(None) => plain.push(c),
            Some(Some(_)) => suffixed.push(c.file_name.clone()),
            None => {}
        }
    }
    if !suffixed.is_empty() {
        suffixed.sort();
        return TocSelection::Unvalidated { files: suffixed };
    }
    match plain.as_slice() {
        [] => TocSelection::None {
            reason: "no .toc file for this folder".to_string(),
        },
        [c] => match &c.contents {
            FileObs::Bytes(b) => match parse_toc(b) {
                Ok(info) => TocSelection::Selected {
                    file_name: c.file_name.clone(),
                    suffix: None,
                    info,
                },
                Err(e) => TocSelection::Error {
                    file_name: c.file_name.clone(),
                    reason: format!("{e:?}"),
                },
            },
            FileObs::Missing => TocSelection::Error {
                file_name: c.file_name.clone(),
                reason: "missing".to_string(),
            },
            FileObs::Unreadable(r) => TocSelection::Error {
                file_name: c.file_name.clone(),
                reason: r.clone(),
            },
        },
        many => {
            let mut files: Vec<String> = many.iter().map(|c| c.file_name.clone()).collect();
            files.sort();
            TocSelection::Ambiguous { files }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::KNOWN_PROFILES;

    fn cand(name: &str, text: &str) -> TocCandidate {
        TocCandidate {
            file_name: name.to_string(),
            contents: FileObs::Bytes(text.as_bytes().to_vec()),
        }
    }

    #[test]
    fn parses_header_keys_with_bom_and_crlf() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            b"## Interface: 11507, 16001,x\r\n## version: 12.1.23\r\n## Title: |cffff0000Core|r\r\n## Dependencies: A, B ,\r\n## RequiredDeps: C\r\n## DepOther: D\r\n## X-Curse-Project-ID: 3358\r\n## X-Wago-ID: abc\r\n## X-WoWI-ID: 77\r\n## SavedVariables: Foo\r\nfile.lua\r\n",
        );
        let i = parse_toc(&bytes).unwrap();
        assert_eq!(i.interface, vec![11507, 16001]);
        assert_eq!(i.version.as_deref(), Some("12.1.23"));
        assert_eq!(i.title.as_deref(), Some("|cffff0000Core|r"));
        assert_eq!(i.hard_deps, vec!["A", "B", "C", "D"]);
        assert_eq!(
            i.ids,
            vec![
                ExternalId {
                    kind: IdKind::Curse,
                    value: "3358".into()
                },
                ExternalId {
                    kind: IdKind::Wago,
                    value: "abc".into()
                },
                ExternalId {
                    kind: IdKind::WoWI,
                    value: "77".into()
                },
            ]
        );
    }

    #[test]
    fn non_utf8_toc_is_error_not_empty() {
        assert_eq!(parse_toc(&[b'#', b'#', 0xff, 0xfe]), Err(TocError::NotUtf8));
        let sel = select_toc(
            "F",
            &[TocCandidate {
                file_name: "F.toc".into(),
                contents: FileObs::Bytes(vec![0xff, 0xfe]),
            }],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Error { .. }));
    }

    #[test]
    fn oversized_toc_is_error() {
        assert_eq!(
            parse_toc(&vec![b'a'; TOC_LIMIT + 1]),
            Err(TocError::TooLarge)
        );
        let sel = select_toc(
            "F",
            &[TocCandidate {
                file_name: "F.toc".into(),
                contents: FileObs::Bytes(vec![b'a'; TOC_LIMIT + 1]),
            }],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Error { .. }));
    }

    #[test]
    fn plain_only_folder_selected() {
        let sel = select_toc(
            "Addon",
            &[
                cand("ADDON.TOC", "## Version: 1.2\n"),
                cand("notes.toc", "## Version: 9\n"),
            ],
            &TocMapping::ValidatedPlain,
        );
        match sel {
            TocSelection::Selected {
                file_name,
                suffix,
                info,
            } => {
                assert_eq!(file_name, "ADDON.TOC");
                assert_eq!(suffix, None);
                assert_eq!(info.version.as_deref(), Some("1.2"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn suffixed_tocs_unvalidated_for_recognized_client() {
        let profile = TocMapping::Profile(&KNOWN_PROFILES[0]);
        let cands = [
            cand("F.toc", "## Version: 1\n"),
            cand("F_Forever.toc", "## Version: 1\n"),
        ];
        for mapping in [&profile, &TocMapping::ValidatedPlain] {
            let sel = select_toc("F", &cands, mapping);
            assert_eq!(
                sel,
                TocSelection::Unvalidated {
                    files: vec!["F_Forever.toc".to_string()]
                }
            );
        }
    }

    #[test]
    fn unfamiliar_with_suffixed_tocs_is_unvalidated() {
        let sel = select_toc("F", &[cand("F-Mists.toc", "")], &TocMapping::ValidatedPlain);
        assert!(matches!(sel, TocSelection::Unvalidated { .. }));
    }

    #[test]
    fn equal_summary_suffixed_tocs_still_unvalidated() {
        let body = "## Version: 3.1\n## Interface: 1\n";
        let sel = select_toc(
            "F",
            &[
                cand("F_Forever.toc", body),
                cand("F_Camelot.toc", body),
                cand("F.toc", body),
            ],
            &TocMapping::Profile(&KNOWN_PROFILES[0]),
        );
        assert_eq!(
            sel,
            TocSelection::Unvalidated {
                files: vec!["F_Camelot.toc".to_string(), "F_Forever.toc".to_string()]
            }
        );
    }

    #[test]
    fn only_foreign_suffix_is_unvalidated() {
        let sel = select_toc(
            "F",
            &[cand("F_Vanilla.toc", "## Version: 1\n")],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Unvalidated { .. }));
    }

    #[test]
    fn no_candidate_is_none() {
        let sel = select_toc(
            "F",
            &[cand("Other.toc", ""), cand("F_x y.toc", "")],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::None { .. }));
        assert!(matches!(
            select_toc("F", &[], &TocMapping::ValidatedPlain),
            TocSelection::None { .. }
        ));
    }

    #[test]
    fn unreadable_candidate_is_error() {
        let sel = select_toc(
            "F",
            &[TocCandidate {
                file_name: "F.toc".into(),
                contents: FileObs::Unreadable("locked".into()),
            }],
            &TocMapping::ValidatedPlain,
        );
        assert_eq!(
            sel,
            TocSelection::Error {
                file_name: "F.toc".into(),
                reason: "locked".into()
            }
        );
        let sel = select_toc(
            "F",
            &[TocCandidate {
                file_name: "F.toc".into(),
                contents: FileObs::Missing,
            }],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Error { .. }));
    }

    #[test]
    fn duplicate_plain_candidates_are_ambiguous() {
        let sel = select_toc(
            "F",
            &[cand("F.toc", ""), cand("f.TOC", "")],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Ambiguous { .. }));
    }
}
