# R3-macfix evidence

## Changes

- `crates/wow-sync-runtime/src/discover.rs`, test
  `finds_only_roots_with_variant_folders`: replaced
  `format!("{}\\_classic_era_", ...)` and `format!("{}\\Data", ...)` with
  `wow.join(...)`. On macOS the literal backslash created one directory with a
  backslash in its name instead of a subfolder.

## Audit (crates/**, xtask/**), no change needed

- Windows-style literals in `winproc.rs` (`image_is_under`, process lists),
  `variant.rs`, `paths.rs`, `records.rs`, `fs_safe.rs`, and the Dropbox
  `info.json` tests in `discover.rs` are pure string parsing or validation.
  No filesystem access.
- `.exe`/`ping.exe`/`SystemRoot` uses in `winproc.rs`, `testkit.rs`, and
  `devkit.rs` are already under `#[cfg(windows)]`.
- `provider.rs` `D:\Dropbox\...` is a constant in `--ignored` provider-tier tests.
- `backend.rs` `C:\Games\WoW` and `C:\Dropbox\WoW Sync` are opaque `PathBuf`
  values and are never created on disk.
- Not run on macOS (no macOS host). CI will report it.

## Results (Windows, local)

- `cargo test --workspace --no-fail-fast`: all test binaries pass, 0 failed.
- `cargo xtask verify --tier offline`: overall PASS, exit 0
  (fmt, clippy, test, required-cases, deny, markdown-links).
