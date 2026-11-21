# R5-release: v0.1.0 pre-release workflow, version, notes, RV-10

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R5-release`, branch `run005/R5-release`.

D-26 approves a `v0.1.0` GitHub pre-release with the Windows build, marked
pre-release. **You do not publish.** The orchestrator pushes the tag after
R5-check passes. You must not push, tag or call the GitHub API.

## Goals

1. **Version 0.1.0.** Workspace `version` and every internal path-dependency
   `version = "0.0.0"` become `0.1.0`; update `Cargo.lock` with
   `cargo update -w` (no other dependency changes). The About screen and
   diagnostics then report 0.1.0.
2. **`.github/workflows/release.yml`:**
   - triggers: `push` of tags `v*`, and `workflow_dispatch` (build and upload
     an artifact only, never publish);
   - `windows-latest`, toolchain from `rust-toolchain.toml`, run
     `cargo xtask verify --tier offline` and `--tier native` first;
   - `cargo build --release --locked -p wow-sync-app --bin wow-sync`, with
     `WOWSYNC_BUILD_COMMIT=${{ github.sha }}` (RV-09);
   - zip `wow-sync.exe`, `LICENSE*`, `README.md` and the release notes into
     `wow-sync-<version>-windows-x64.zip`, plus a SHA-256 file;
   - upload as a workflow artifact on every run;
   - on a tag push only: check the tag equals `v` + the workspace version,
     then create a **pre-release** with `gh release create --prerelease
     --notes-file …` using `GITHUB_TOKEN`. Grant `contents: write` only to
     that job. Pin actions by SHA like `ci.yml`. No other secrets.
3. **Release notes** `docs/release/v0.1.0.md`: what it is (an unsigned
   Windows preview; SmartScreen warning; no installer), what works, and the
   known gaps, read from `docs/runs/run-004/REPORT.md` and run-005 scope:
   the Q-24 synthetic-marker gate (it does not sync real WoW data yet), no
   macOS build, settings sync limits, open items. Plain and honest.
4. **RV-10.** Add `--locked` to xtask's cargo clippy/test/build invocations.
   Add a `docs/DEPENDENCIES.md` with licence, maintenance status and need
   for each direct third-party dependency in the workspace `Cargo.toml`,
   checked against the crates' own metadata in `~/.cargo/registry` (no
   network). Link it from `docs/REPO-MAP.md` (one row).
5. **Update notice with only a pre-release.** GitHub's `releases/latest`
   ignores pre-releases, so after publishing the app still gets 404. Read
   `adapters/src/update_http.rs`, `core/src/update.rs` and the runtime/app
   handling, and record in your evidence what the user sees in that case.
   Do not change product code. If it shows an error rather than "no
   release yet", say so plainly so the orchestrator can route it.

## Files you own

`Cargo.toml` (versions only), every crate `Cargo.toml` (versions only),
`Cargo.lock`, `.github/workflows/release.yml`, `xtask/**`,
`docs/release/**`, `docs/DEPENDENCIES.md`, one row in `docs/REPO-MAP.md`,
your required-cases section.

## Done

Both tiers pass; `actionlint`-style review by reading the workflow (no
network needed); evidence `docs/runs/run-005/evidence/R5-release.md`; reply
with SHA. Budget about 500 AIU; stop and report at 700.
