use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub enum LinkError {
    /// git could not be run, so the tracked-file list is unknown.
    GitUnavailable(String),
    Io(String),
}

/// Splits `git ls-files -z` output into tracked Markdown paths.
fn parse_ls_files(output: &[u8]) -> Vec<PathBuf> {
    output
        .split(|&b| b == 0)
        .filter_map(|p| std::str::from_utf8(p).ok())
        .filter(|p| p.ends_with(".md"))
        .map(PathBuf::from)
        .collect()
}

fn tracked_markdown(root: &Path) -> Result<Vec<PathBuf>, LinkError> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--", "*.md"])
        .current_dir(root)
        .output()
        .map_err(|e| LinkError::GitUnavailable(format!("cannot start git: {e}")))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(LinkError::GitUnavailable(format!(
            "git ls-files failed: {msg}"
        )));
    }
    Ok(parse_ls_files(&out.stdout)
        .into_iter()
        .map(|p| root.join(p))
        // Never follow symlinks or junctions; skip tracked files deleted from the work tree.
        .filter(|p| {
            fs::symlink_metadata(p)
                .map(|m| m.file_type().is_file())
                .unwrap_or(false)
        })
        .collect())
}

/// Checks git-tracked Markdown files under `root`; returns (files checked, "file: target" problems).
pub fn check_repo(root: &Path) -> Result<(usize, Vec<String>), LinkError> {
    let files = tracked_markdown(root)?;
    let mut problems = Vec::new();
    for f in &files {
        let text =
            fs::read_to_string(f).map_err(|e| LinkError::Io(format!("{}: {e}", f.display())))?;
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
    fn ls_files_output_is_split_on_nul_and_filtered_to_markdown() {
        let out = b"README.md\0src/a.rs\0docs/sp ace.md\0";
        assert_eq!(
            parse_ls_files(out),
            vec![PathBuf::from("README.md"), PathBuf::from("docs/sp ace.md")]
        );
    }

    #[test]
    fn untracked_markdown_is_not_scanned() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join(format!("xtask-links-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("cache")).unwrap();
        fs::write(root.join("tracked.md"), "[bad](gone.md)").unwrap();
        fs::write(root.join("cache").join("untracked.md"), "[bad](other.md)").unwrap();
        let git = |args: &[&str]| {
            let s = Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap();
            assert!(s.success());
        };
        git(&["init", "-q"]);
        git(&["add", "tracked.md"]);
        let (n, problems) = check_repo(&root).ok().unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(n, 1);
        assert_eq!(problems, vec!["tracked.md: gone.md"]);
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
