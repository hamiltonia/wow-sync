# R2-fix5 evidence (run-002, D-17)

Branch `run002/R2-fix5`, created from `50e5029`. This fixes the macOS CI failure
in `offline/required-cases`:
"R2-fix4:slice: listed test 'e2e::tests::pending_clear_failure_does_not_duplicate_report' did not run".

## Change (test-only, no product change)

- `crates/wow-sync-runtime/src/e2e.rs`:
  - The R3-M1 regression test now compiles on
    `cfg(any(windows, target_os = "macos"))`, the CI matrix.
  - A test-only `ReplaceBlock` makes `pending-report.json` readable but impossible
    to replace:
    - **Windows:** unchanged; a `FILE_SHARE_READ`-only handle.
    - **macOS:** `/usr/bin/chflags uchg`. The user immutable flag is settable by
      the file owner without root. It is cleared with `chflags nouchg` on drop,
      including during unwind, so temp-dir cleanup still works.
  - No assertion was changed or weakened. The test still independently asserts
    that `LocalState::write_atomic` fails, so an ineffective block on any platform
    fails the test instead of passing vacuously.
- `docs/design/tests.md`: updated the `R2-fix4:slice` description.
- The test stays in the offline tier. `tests/required-cases.toml` is unchanged.

## Results (Windows 11 x64, Rust 1.100.0)

| Command | Result |
|---|---|
| `cargo test -q -p wow-sync-runtime --lib -- --exact e2e::tests::pending_clear_failure_does_not_duplicate_report` | pass |
| Same test with the `service.rs` reconcile gate disabled (A-38, restored afterwards) | EXIT=101, panicked `e2e.rs:888` |
| `cargo clippy -q -p wow-sync-runtime --lib --tests --target aarch64-apple-darwin -- -D warnings` | exit 0. Type-check and lint only; nothing was executed on macOS |
| `cargo xtask verify --tier offline` | exit 0, all PASS |
| `cargo test -q --workspace` | exit 0: adapters 45 (+3 ignored), app 2, core 172, runtime 26, xtask 24 |

## Unverified

- **The macOS path was not run.** These points are reasoned, not observed:
  - XNU denies rename(2) onto a target with `UF_IMMUTABLE` (EPERM), because
    replacing the target requires delete permission on it, which is refused
    for immutable files.
  - Reads still succeed.
  - GitHub `macos-latest` runners run as a non-root user, and the owner may set
    and clear `uchg` either way.
  - The temp directory is on APFS, which supports file flags.
- If any of these assumptions is wrong, the test's own `write_atomic(..).is_err()`
  assertion fails on macOS CI. It does not hide the problem.
- **Linux:** the test is not compiled. Linux is not in the CI matrix, and
  `chattr +i` needs `CAP_LINUX_IMMUTABLE`. If Linux is added to the matrix,
  `required-cases` will report the test as not run.
- **GUI:** not run.
