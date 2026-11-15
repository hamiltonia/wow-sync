use std::fs;
use std::path::{Path, PathBuf};

/// Removes inline code spans (any backtick run length, CommonMark matching) from one line.
/// An unmatched backtick run is kept as literal text.
fn strip_code_spans(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let run_len = |from: usize| chars[from..].iter().take_while(|&&c| c == '`').count();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let n = run_len(i);
        let mut j = i + n;
        let mut close = None;
        while j < chars.len() {
            if chars[j] == '`' {
                let m = run_len(j);
                if m == n {
                    close = Some(j + m);
                    break;
                }
                j += m;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => i = end,
            None => {
                out.extend(&chars[i..i + n]);
                i += n;
            }
        }
    }
    out
}

/// Extracts inline Markdown link targets, ignoring fenced code blocks and inline code spans.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let stripped = strip_code_spans(line);
        let mut rest = stripped.as_str();
        while let Some(i) = rest.find("](") {
            let after = &rest[i + 2..];
            if let Some(end) = after.find(')') {
                let target = after[..end].trim();
                let target = target.split_whitespace().next().unwrap_or("");
                out.push(target.trim_matches(['<', '>']).to_string());
                rest = &after[end..];
            } else {
                break;
            }
        }
    }
    out
}

fn is_external(target: &str) -> bool {
    target.is_empty()
        || target.starts_with('#')
        || target.contains("://")
        || target.starts_with("mailto:")
}

/// Returns the relative-link targets in `text` that do not exist relative to `dir`.
pub fn broken_links(text: &str, dir: &Path) -> Vec<String> {
    extract_links(text)
        .into_iter()
        .filter(|t| !is_external(t))
        .filter(|t| {
            let path = t.split('#').next().unwrap_or("");
            !path.is_empty() && !dir.join(path).exists()
        })
        .collect()
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name != "target" && name != ".git" {
                collect_markdown(&path, out)?;
            }
        } else if name.ends_with(".md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Checks every Markdown file under `root`; returns (files checked, "file: target" problems).
pub fn check_repo(root: &Path) -> std::io::Result<(usize, Vec<String>)> {
    let mut files = Vec::new();
    collect_markdown(root, &mut files)?;
    let mut problems = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f)?;
        let dir = f.parent().unwrap_or(root);
        for t in broken_links(&text, dir) {
            let rel = f.strip_prefix(root).unwrap_or(f);
            problems.push(format!("{}: {t}", rel.display()));
        }
    }
    Ok((files.len(), problems))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_links_and_skips_fenced_code() {
        let text = "see [a](A.md) and [b](docs/B.md#x \"t\")\n```\n[c](C.md)\n```\n[d](<D.md>)";
        assert_eq!(extract_links(text), vec!["A.md", "docs/B.md#x", "D.md"]);
    }

    #[test]
    fn inline_code_spans_are_not_links() {
        assert!(extract_links("check `](target)` here").is_empty());
        assert!(extract_links("double ``a ` ](target)`` ok").is_empty());
        assert_eq!(extract_links("lone ` tick [a](A.md)"), vec!["A.md"]);
    }

    #[test]
    fn real_link_next_to_code_span_is_still_reported() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let text = "`](target)` and [bad](missing.md) then `x`";
        assert_eq!(broken_links(text, dir), vec!["missing.md"]);
    }

    #[test]
    fn flags_only_missing_relative_targets() {
        // The xtask manifest directory exists, so a link to it resolves.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let text = "[ok](Cargo.toml#frag) [web](https://e.com/x) [anchor](#top) [bad](missing.md)";
        assert_eq!(broken_links(text, dir), vec!["missing.md"]);
    }
}
