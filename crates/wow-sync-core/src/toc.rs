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
    /// `## SavedVariables:` names (account-wide settings files), in header order.
    pub saved_variables: Vec<String>,
    /// `## SavedVariablesPerCharacter:` names, in header order.
    pub saved_variables_per_character: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TocError {
    TooLarge,
    NotUtf8,
}

/// The header window of a `.toc` (domain.md §3). A file longer than this is read from its
/// header only: the complete lines of the first `TOC_LIMIT` bytes.
pub const TOC_LIMIT: usize = 256 * 1024;

/// The complete lines of the first `TOC_LIMIT` bytes. A cut line is dropped whole, so a cut
/// multi-byte character never reaches the UTF-8 check. `None` = no complete line in the window.
fn header_window(bytes: &[u8]) -> Option<&[u8]> {
    let window = &bytes[..bytes.len().min(TOC_LIMIT + 1)];
    let end = window.iter().rposition(|b| *b == b'\n')?;
    Some(&window[..=end])
}

/// Parses a `.toc` header. Bytes beyond `TOC_LIMIT` are not read for meaning: only the complete
/// lines before the cut count. A cut file is accepted only when the window shows the header has
/// ended (a file-list line follows the `#` lines); otherwise directives may lie beyond the window
/// and it is `TooLarge`, never an incomplete success (D-29, RV7-04).
pub fn parse_toc(bytes: &[u8]) -> Result<TocInfo, TocError> {
    let oversized = bytes.len() > TOC_LIMIT;
    let bytes = if oversized {
        let window = header_window(bytes).ok_or(TocError::TooLarge)?;
        if !header_has_ended(window) {
            return Err(TocError::TooLarge);
        }
        window
    } else {
        bytes
    };
    let info = parse_header(bytes)?;
    if oversized && info == TocInfo::default() {
        return Err(TocError::TooLarge);
    }
    Ok(info)
}

/// A line that is neither blank nor a `#` line is a file entry: the header is over.
fn header_has_ended(window: &[u8]) -> bool {
    let window = window.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(window);
    window
        .split(|b| *b == b'\n')
        .map(|l| l.trim_ascii())
        .any(|l| !l.is_empty() && l[0] != b'#')
}

fn parse_header(bytes: &[u8]) -> Result<TocInfo, TocError> {
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
            "savedvariables" => push_names(&mut info.saved_variables, value),
            "savedvariablespercharacter" => {
                push_names(&mut info.saved_variables_per_character, value)
            }
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

fn push_names(out: &mut Vec<String>, value: &str) {
    for n in value.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        if !out.iter().any(|o| o == n) {
            out.push(n.to_string());
        }
    }
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

/// The only `.toc` selection rule (domain.md §3). An unfamiliar client uses the plain rule: a
/// folder with any suffixed candidate is `Unvalidated`. A known profile selects by game-type
/// suffix, then family suffix, then the plain `.toc`; other suffixes are ignored (D-28, Q-3).
pub fn select_toc(folder: &str, candidates: &[TocCandidate], mapping: &TocMapping) -> TocSelection {
    match mapping {
        TocMapping::ValidatedPlain => select_plain(folder, candidates),
        TocMapping::Profile(p) => select_profile(folder, candidates, p),
    }
}

fn select_one(c: &TocCandidate, suffix: Option<String>) -> TocSelection {
    match &c.contents {
        FileObs::Bytes(b) => match parse_toc(b) {
            Ok(info) => TocSelection::Selected {
                file_name: c.file_name.clone(),
                suffix,
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
    }
}

fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.iter().all(|x| b.contains(x)) && b.iter().all(|x| a.contains(x))
}

/// One tier of the profile rule: candidates for the listed suffixes, in order. Nothing found
/// is `None`; several that agree on version, dependencies and ids select the first listed
/// suffix; several that differ are `Ambiguous` (domain.md §3 step 2).
fn select_tier(
    folder: &str,
    candidates: &[TocCandidate],
    suffixes: &[&str],
) -> Option<TocSelection> {
    let mut found: Vec<(usize, String, &TocCandidate)> = Vec::new();
    for c in candidates {
        let Some(Some(s)) = candidate_kind(folder, &c.file_name) else {
            continue;
        };
        if let Some(rank) = suffixes.iter().position(|x| x.eq_ignore_ascii_case(&s)) {
            found.push((rank, s, c));
        }
    }
    if found.is_empty() {
        return None;
    }
    found.sort_by(|a, b| (a.0, &a.2.file_name).cmp(&(b.0, &b.2.file_name)));
    let parsed: Vec<TocSelection> = found
        .iter()
        .map(|(_, s, c)| select_one(c, Some(s.clone())))
        .collect();
    if let Some(e) = parsed
        .iter()
        .find(|p| matches!(p, TocSelection::Error { .. }))
    {
        return Some(e.clone());
    }
    let infos: Vec<&TocInfo> = parsed
        .iter()
        .filter_map(|p| match p {
            TocSelection::Selected { info, .. } => Some(info),
            _ => None,
        })
        .collect();
    let first = infos[0];
    let agree = infos.iter().all(|i| {
        i.version == first.version
            && same_set(&i.hard_deps, &first.hard_deps)
            && same_set(&i.ids, &first.ids)
    });
    if agree {
        return parsed.into_iter().next();
    }
    Some(TocSelection::Ambiguous {
        files: found.iter().map(|f| f.2.file_name.clone()).collect(),
    })
}

fn select_profile(
    folder: &str,
    candidates: &[TocCandidate],
    profile: &crate::variant::Profile,
) -> TocSelection {
    for suffixes in [profile.toc_suffixes, profile.family_suffixes] {
        if let Some(sel) = select_tier(folder, candidates, suffixes) {
            return sel;
        }
    }
    let plain: Vec<&TocCandidate> = candidates
        .iter()
        .filter(|c| candidate_kind(folder, &c.file_name) == Some(None))
        .collect();
    match plain.as_slice() {
        [] => TocSelection::None {
            reason: "no .toc for this variant".to_string(),
        },
        [c] => select_one(c, None),
        many => {
            let mut files: Vec<String> = many.iter().map(|c| c.file_name.clone()).collect();
            files.sort();
            TocSelection::Ambiguous { files }
        }
    }
}

fn select_plain(folder: &str, candidates: &[TocCandidate]) -> TocSelection {
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

    fn big_toc(header: &str, filler_line: &str, total: usize) -> Vec<u8> {
        let mut b = header.as_bytes().to_vec();
        while b.len() < total {
            b.extend_from_slice(filler_line.as_bytes());
        }
        b
    }

    #[test]
    fn oversized_toc_is_read_from_its_header() {
        let b = big_toc(
            "## Interface: 11508\n## Title: Big\n## Version: 9.9.9\n## SavedVariables: BigDB\n",
            "Data\\File_0001.lua\n",
            1_200_000,
        );
        let sel = select_toc(
            "Big",
            &[TocCandidate {
                file_name: "Big.toc".into(),
                contents: FileObs::Bytes(b),
            }],
            &TocMapping::ValidatedPlain,
        );
        match sel {
            TocSelection::Selected { info, .. } => {
                assert_eq!(info.version.as_deref(), Some("9.9.9"));
                assert_eq!(info.saved_variables, vec!["BigDB".to_string()]);
                assert_eq!(info.interface, vec![11508]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cut_inside_a_multibyte_character_keeps_the_header() {
        // The window ends in the middle of a three-byte character on a line without a newline.
        let mut b = b"## Version: 2.0\n".to_vec();
        while b.len() < TOC_LIMIT - 1 {
            b.extend_from_slice("x\n".as_bytes());
        }
        b.truncate(TOC_LIMIT - 1);
        b.extend_from_slice("€€€€".as_bytes());
        assert!(b.len() > TOC_LIMIT);
        assert!(std::str::from_utf8(&b[..TOC_LIMIT]).is_err());
        let info = parse_toc(&b).unwrap();
        assert_eq!(info.version.as_deref(), Some("2.0"));
    }

    #[test]
    fn header_beyond_the_window_is_not_selected() {
        let b = big_toc("", "# a comment line\n", TOC_LIMIT + 4096);
        let mut b = b;
        b.extend_from_slice(b"## Version: 1\n");
        assert_eq!(parse_toc(&b), Err(TocError::TooLarge));
        let sel = select_toc(
            "F",
            &[TocCandidate {
                file_name: "F.toc".into(),
                contents: FileObs::Bytes(b),
            }],
            &TocMapping::ValidatedPlain,
        );
        assert!(matches!(sel, TocSelection::Error { .. }));
    }

    #[test]
    fn cut_header_without_a_file_line_is_not_selected() {
        // The window holds only directives: more of them (and the saved variables) may follow.
        let mut b = big_toc(
            "## Version: 1\n",
            "## X-Filler: padding\n",
            TOC_LIMIT + 4096,
        );
        b.extend_from_slice(b"## SavedVariables: Late\nMain.lua\n");
        assert_eq!(parse_toc(&b), Err(TocError::TooLarge));
        // A file-list line inside the window proves the header ended: the rest is file entries.
        let mut ok = b"## Version: 1\n".to_vec();
        ok.extend_from_slice(b"Main.lua\n");
        ok.extend(big_toc("", "## X-Filler: padding\n", TOC_LIMIT + 4096));
        assert_eq!(parse_toc(&ok).unwrap().version.as_deref(), Some("1"));
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

    fn profile_pick(folder: &str, cands: &[TocCandidate]) -> TocSelection {
        select_toc(folder, cands, &TocMapping::Profile(&KNOWN_PROFILES[0]))
    }

    fn picked(sel: &TocSelection) -> Option<&str> {
        match sel {
            TocSelection::Selected { file_name, .. } => Some(file_name),
            _ => None,
        }
    }

    #[test]
    fn plain_rule_blocks_suffixed_tocs_for_unfamiliar_clients() {
        let cands = [
            cand("F.toc", "## Version: 1\n"),
            cand("F_Forever.toc", "## Version: 1\n"),
        ];
        assert_eq!(
            select_toc("F", &cands, &TocMapping::ValidatedPlain),
            TocSelection::Unvalidated {
                files: vec!["F_Forever.toc".to_string()]
            }
        );
    }

    #[test]
    fn forever_profile_suffix_mixes_observed_on_the_dev_pc() {
        let v = |n: u32| format!("## Version: {n}\n## Interface: 1\n");
        // (files with their version, expected pick or None)
        type Row<'a> = (Vec<(&'a str, u32)>, Option<&'a str>);
        let table: Vec<Row> = vec![
            (vec![("F.toc", 1)], Some("F.toc")),
            (
                vec![("F.toc", 1), ("F_Camelot.toc", 2)],
                Some("F_Camelot.toc"),
            ),
            (
                vec![("F.toc", 1), ("F_Forever.toc", 2)],
                Some("F_Forever.toc"),
            ),
            (
                vec![("F_Forever.toc", 3), ("F_Camelot.toc", 3), ("F.toc", 1)],
                Some("F_Forever.toc"),
            ),
            (
                vec![("F.toc", 1), ("F_Mainline.toc", 2)],
                Some("F_Mainline.toc"),
            ),
            (
                vec![("F_Mainline.toc", 2), ("F_Camelot.toc", 3)],
                Some("F_Camelot.toc"),
            ),
            (vec![("F_Vanilla.toc", 1)], None),
            (
                vec![("F_Vanilla.toc", 1), ("F_Mists.toc", 1), ("F_TBC.toc", 1)],
                None,
            ),
            (vec![("F.toc", 1), ("F_TBC.toc", 2)], Some("F.toc")),
            (
                vec![
                    ("F.toc", 1),
                    ("F_Wrath.toc", 2),
                    ("F-Cata.toc", 3),
                    ("F_BCC.toc", 4),
                ],
                Some("F.toc"),
            ),
            (
                vec![("f.TOC", 1), ("F-CAMELOT.toc", 2)],
                Some("F-CAMELOT.toc"),
            ),
            (
                vec![("F-forever.toc", 5), ("F_Camelot.toc", 5)],
                Some("F-forever.toc"),
            ),
            (vec![("F_Classic.toc", 1), ("F.toc", 2)], Some("F.toc")),
        ];
        for (files, want) in table {
            let cands: Vec<TocCandidate> = files.iter().map(|(n, ver)| cand(n, &v(*ver))).collect();
            let sel = profile_pick("F", &cands);
            match want {
                Some(w) => assert_eq!(picked(&sel), Some(w), "{files:?}"),
                None => assert!(
                    matches!(sel, TocSelection::None { .. }),
                    "{files:?} {sel:?}"
                ),
            }
        }
    }

    #[test]
    fn forever_camelot_with_different_summaries_is_ambiguous() {
        for (a, b) in [
            ("## Version: 1\n", "## Version: 2\n"),
            (
                "## Version: 1\n## Dependencies: A\n",
                "## Version: 1\n## Dependencies: B\n",
            ),
            (
                "## Version: 1\n## X-Wago-ID: a\n",
                "## Version: 1\n## X-Wago-ID: b\n",
            ),
        ] {
            let sel = profile_pick(
                "F",
                &[
                    cand("F_Forever.toc", a),
                    cand("F_Camelot.toc", b),
                    cand("F.toc", a),
                ],
            );
            assert_eq!(
                sel,
                TocSelection::Ambiguous {
                    files: vec!["F_Forever.toc".to_string(), "F_Camelot.toc".to_string()]
                }
            );
        }
        // Dependencies compare as sets, so order and spacing do not matter.
        let sel = profile_pick(
            "F",
            &[
                cand("F_Forever.toc", "## Dependencies: A, B\n"),
                cand("F_Camelot.toc", "## Dependencies: B,A\n"),
            ],
        );
        assert_eq!(picked(&sel), Some("F_Forever.toc"));
    }

    #[test]
    fn profile_candidate_errors_fail_closed() {
        let bad = TocCandidate {
            file_name: "F_Camelot.toc".into(),
            contents: FileObs::Bytes(vec![b'#', b'#', 0xff]),
        };
        let sel = profile_pick("F", &[bad, cand("F.toc", "## Version: 1\n")]);
        assert!(matches!(sel, TocSelection::Error { .. }), "{sel:?}");
    }

    #[test]
    fn profile_reports_the_suffix_it_selected() {
        let sel = profile_pick("F", &[cand("F-camelot.toc", "## Version: 1\n")]);
        let TocSelection::Selected { suffix, .. } = sel else {
            panic!("{sel:?}")
        };
        assert_eq!(suffix.as_deref(), Some("camelot"));
    }

    #[test]
    fn unfamiliar_with_suffixed_tocs_is_unvalidated() {
        let sel = select_toc("F", &[cand("F-Mists.toc", "")], &TocMapping::ValidatedPlain);
        assert!(matches!(sel, TocSelection::Unvalidated { .. }));
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
