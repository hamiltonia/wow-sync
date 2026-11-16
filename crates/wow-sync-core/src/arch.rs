//! Architecture guard (A-43): core stays platform-free.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    const FORBIDDEN: &[&str] = &[
        "std::fs",
        "std::process",
        "std::net",
        "std::env",
        "std::thread",
        "SystemTime",
        "Instant::now",
        "File::",
        "getrandom",
        "windows_sys",
        "gpui",
    ];

    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn core_sources_use_no_os_io_or_clock() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        sources(&root.join("src"), &mut files);
        assert!(files.len() > 10, "source scan found {} files", files.len());
        let mut hits = Vec::new();
        for f in &files {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name == "arch.rs" || name == "testkit.rs" {
                continue;
            }
            let text = std::fs::read_to_string(f).expect("read source");
            for t in FORBIDDEN {
                if text.contains(t) {
                    hits.push(format!("{name}: {t}"));
                }
            }
        }
        assert!(hits.is_empty(), "forbidden platform use in core: {hits:?}");

        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("read manifest");
        let deps: Vec<&str> = manifest
            .split("[dependencies]")
            .nth(1)
            .unwrap_or_default()
            .lines()
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with('['))
            .filter_map(|l| l.split(['=', '.', ' ']).next())
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(deps, vec!["serde", "serde_json", "sha2"]);
    }
}
