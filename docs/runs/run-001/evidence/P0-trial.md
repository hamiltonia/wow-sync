# P0-trial evidence

Run 001, packet P0-trial. Machine: Windows 11 Pro build 26300 x64, PowerShell.
Overall status: **PARTIAL / BLOCKED** (MSVC Build Tools could not be installed).

## Installed versions

| Tool | Version |
|---|---|
| rustup | 1.29.3 (edbc831ce 2026-11-02), via winget `Rustlang.Rustup` |
| rustc | 1.100.0 (0ef1bc386 2026-11-09), pinned in `rust-toolchain.toml` |
| cargo | 1.100.0 (3acc79c0c 2026-11-05) |
| MSVC / VS Build Tools | **Not installed** (see below) |
| Windows SDK | **Not installed** |

## Build Tools install: BLOCKED

Command (twice, same result):
`winget install --id Microsoft.VisualStudio.2022.BuildTools -e --silent --accept-source-agreements --accept-package-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`

Output (abbreviated): hash verified, `Starting package install...`,
`Installer failed with exit code: 1602`. Exit 1602 means the user cancelled or
elevation was not granted; this worker cannot satisfy UAC/interactive consent.
`vswhere.exe` still absent afterwards. Stopped per packet (two failed attempts).

## Step 6 results

| Command | Result | Reason |
|---|---|---|
| `cargo fmt --all --check` | PASS | exit 0, no diffs (first run auto-installed the pinned toolchain) |
| `cargo clippy --workspace -- -D warnings` | PASS | `Finished dev profile`, exit 0 |
| `cargo build --workspace` | PASS | exit 0; library crate produces an rlib, so no linker needed |
| `cargo test --workspace` | FAIL (blocked by environment) | `error: linker link.exe not found`, exit 101; the test harness binary needs the MSVC linker |
| `cargo build --workspace --target aarch64-pc-windows-msvc` | PASS | exit 0; build-only, NOT runtime proof; rlib only, linking untested |

Zero tests exist by design (no dummy tests). The `cargo test` failure is the
missing linker, not a code failure; it must be rerun after Build Tools are installed.

## Deviations / limitations

- Build Tools absent, so `cargo test` is unproven and any bin/cdylib linking is untested.
- The PASS results for build do not prove the MSVC toolchain works.
- Not run: macOS targets are pinned but not built here.
- `profile = "minimal"` set in `rust-toolchain.toml` (not specified by packet).
- Workspace package also sets `edition = "2021"` and `version = "0.0.0"`; crate has `publish = false`.
