# Work packet P0-trial: toolchain and minimal workspace

Run: 001 (approved, D-15). Role: implementation. Model: claude-sonnet-5.5 (D-04).

## Intent

Prove the dev PC can build and test a pinned Rust workspace, and measure what a
small worker task costs so the owner can set the run's credit ceiling (D-13).
Supports E-08 (pinned toolchain) and groundwork for A-39/A-43. Not a feature.

## Checkout

- Worktree: `D:\GitHub\wow-sync.worktrees\P0-trial`
- Branch: `run001/P0-trial` (from `main`)
- Do not edit `D:\GitHub\wow-sync` (main checkout) or any other worktree.

## Owned writes

- `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, `.gitignore`
- `crates/wow-sync-core/` (library skeleton only)
- `docs/runs/run-001/evidence/P0-trial.md` (your report)

Do not modify SPEC.md, ACCEPTANCE.md, ENGINEERING.md, AGENTS.md,
`docs/OWNER-DECISIONS.md`, PLAN.md, USAGE.md or packets.

## Allowed system changes (D-07)

- Install rustup (per-user) and the toolchain pinned in `rust-toolchain.toml`.
- Install Visual Studio 2022 Build Tools with the C++ workload (MSVC and a
  Windows SDK) via winget if the MSVC linker is missing. If installation needs
  interactive UAC/consent you cannot complete, **stop and report blocked**.
- Nothing else: no other global settings, no security changes.

## Work

1. Install prerequisites as above. Record exact versions installed.
2. `rust-toolchain.toml`: pin an exact stable version (not `stable`), components
   `rustfmt` and `clippy`; targets `x86_64-pc-windows-msvc`,
   `aarch64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.
3. Root `Cargo.toml` virtual workspace (`resolver = "2"`, MIT license in
   `[workspace.package]`, `rust-version` matching the pin) with member
   `crates/wow-sync-core`.
4. `wow-sync-core`: empty library crate. **Do not add dummy passing tests**
   (AGENTS.md bootstrap obligation); zero tests is the honest state.
5. No third-party dependencies.
6. Run and capture output of: `cargo fmt --all --check`,
   `cargo clippy --workspace -- -D warnings`, `cargo build --workspace`,
   `cargo test --workspace`, and
   `cargo build --workspace --target aarch64-pc-windows-msvc`
   (build-only; label it as not runtime proof). Report each honestly.
7. Commit on your branch with message prefix `run001(P0):` and the trailer
   `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
   Do not push; do not merge.

## Evidence file

`docs/runs/run-001/evidence/P0-trial.md`: installed tool versions, exact
commands, abbreviated real output, pass/fail/blocked per command, anything
skipped and why, known limitations. Never fabricate output.

## Stop conditions

- Do not spawn sub-agents.
- Two failed attempts at the same step: stop and report.
- Anything requiring credentials, paid services, or writes outside the owned
  paths and allowed installs: stop and report.

## Definition of done

Branch has the commit; evidence file exists; the orchestrator can rerun step 6
from a clean clone of the branch and get the same results.
