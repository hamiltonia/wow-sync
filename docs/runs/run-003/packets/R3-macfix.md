# R3-macfix (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first.

**Problem.** CI `verify-offline (macos-latest)` has failed since `ce15e24`,
the phase F merge. The failing test is
`wow-sync-runtime discover::tests::finds_only_roots_with_variant_folders`.
It builds paths with a literal `\\` separator. On macOS that makes a
directory whose name contains a backslash, not a subfolder.

Because `cargo test` stops at the first failing test binary, later macOS
test binaries have not run since then. They may hide more failures.

**Owned paths:**
- Test code and `#[cfg(windows)]` gating in `crates/**`, but no product
  behaviour change.
- `docs/runs/run-003/evidence/R3-macfix.md`.

## Goals

1. Fix the test to be portable: use `Path::join`, not literal separators.
2. Search `crates/**` and `xtask/**` for other Windows-only assumptions in
   tests, such as literal backslash separators, drive letters used as real
   paths, or `.exe` names. Fix each one, or gate it with `#[cfg(windows)]`
   and a reason.
   - Pure string-parsing tests on Windows-style inputs (for example Dropbox
     `info.json` text) are fine as they are.
3. Run `cargo test --workspace --no-fail-fast` locally on Windows.
4. Commit, then push only the branch `run003/R3-macfix` to origin; never
   push main. The orchestrator opens a draft PR to run CI (CI runs only on
   pull requests and main) and records the macOS result.

## Done

- `cargo xtask verify --tier offline` passes locally.
- The evidence lists each change, with the reason for each.
