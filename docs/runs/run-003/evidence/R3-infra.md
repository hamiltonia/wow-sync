# R3-infra evidence (run-003)

Environment: Windows 11 x64 dev PC, Rust 1.100.0, worktree
`D:\GitHub\wow-sync.worktrees\R3-infra`, branch `run003/R3-infra`.
Related: Q-17, D-18, [VERIFY.md](../../../VERIFY.md).

## Built

1. **Native tier.** `cargo xtask verify --tier native` runs
   `cargo test --workspace` on Windows and fails if any `native` required case
   did not run, was ignored, or failed (or if `cargo test` fails). On other
   hosts it is BLOCKED (never PASS). `provider` stays BLOCKED. CI runs the
   native tier on the Windows job only (`.github/workflows/ci.yml`).
   Self-test `tests::missing_native_test_fails_the_tier` proves a missing
   native test fails the tier.
2. **Diagnostics.** When `cargo test` fails, xtask appends a `failed tests:`
   list with the test names (offline and native). Test:
   `tests::failed_test_names_are_printed`.
3. **Flaky test fix** (test code only, `winproc.rs`).
4. **Dependabot**: no bump possible (below).

## Commands and results

| Command | Result |
|---|---|
| `cargo xtask verify --tier native` | PASS (Windows 11): `[PASS] native/tier`, exit 0 |
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, test, required-cases, deny, markdown-links; exit 0 |
| `cargo test --workspace` | 271 passed, 0 failed (45 adapters lib + 3 ignored provider-tier; 172, 26, 26, 2 elsewhere) |
| macOS native tier | NOT RUN here (BLOCKED by design); CI only |

## Flaky test investigation (`wow-sync-adapters`)

- Default threads: 30/30 runs passed. `--test-threads=1`: 30/30 passed. A
  quiet machine does not reproduce the failure.
- Under CPU load it reproduced. Six concurrent loops of 60 runs: 216 of 360
  runs failed, in `winproc::tests::fake_client_under_variant_is_running` and
  `winproc::tests::process_outside_variant_is_not_running`, always with
  `left: Unknown("possible client ... cannot be inspected")`, `right:
  NotRunning`.
- Root cause: `observe_variant` correctly reports Unknown while any `wow*.exe`
  process is being created or torn down and cannot be inspected. Two tests
  start or kill fake `Wow.exe` processes concurrently in the same binary (and
  an exited-but-not-yet-released client is uninspectable for a moment after
  `kill`+`wait`), so one test's snapshot saw the other's transitional process.
  The original CI failure (`7da6d0f`) is consistent with this; it is not
  proven, since CI did not name the test.
- Fix (test code only, no production change): a mutex serializes the two
  process-observing tests, and a bounded settle helper (`observe_settled`, up
  to 5 s) waits while the result is Unknown before the `NotRunning` assertions.
  A process that stays uninspectable still fails. No retries of whole tests.
- After the fix, with 12 CPU burners and one test process: 300 runs of the
  `winproc` tests, 0 failures. The serialize-only intermediate version failed
  3 of 300 under the same load (foreign `Wow.exe` or `wow_*` binaries from
  other worktrees and teardown lag), which is why the settle helper was added.
- Not fixed (outside owned paths): `wow-sync-runtime` `e2e.rs`
  `fake_client_blocks_real_apply` runs a fake `Wow.exe` while other e2e tests
  call `observe_variant` in parallel. The same transient Unknown could affect
  them; suggest the same serialization there.
- A-38 negative control for the xtask self-test: NOT RUN (the test constructs
  the missing-test case directly against `native_check`).

## Dependabot alerts (open, 6)

| # | Package | Lockfile | From | Fix available | Status |
|---|---|---|---|---|---|
| 6 | glib | `Cargo.lock` | 0.18.5 | 0.20.0 | Not fixed: needs gtk-rs 0.20; see recommendation |
| 5 | glib | `spikes/gpui-pin/Cargo.lock` | 0.18.5 | 0.20.0 | Not fixed: same |
| 1 | glib | `spikes/gpui/Cargo.lock` | 0.18.5 | 0.20.0 | Not fixed: same |
| 4 | grid | `spikes/mem/Cargo.lock` | 0.18.0 | 1.0.1 | Not fixed: `gpui 0.2.2` pins `taffy =0.9.0` which needs `grid ^0.18` |
| 3 | grid | `spikes/a11y/Cargo.lock` | 0.18.0 | 1.0.1 | Not fixed: same |
| 2 | grid | `spikes/gpui/Cargo.lock` | 0.18.0 | 1.0.1 | Not fixed: same |

No alert was dismissed. `cargo update -p grid --precise 1.0.1` in
`spikes/mem` failed on the `taffy` requirement (nothing was changed).

Recommendations (owner decision):

- **glib (GHSA-wrw7-89jp-8q8g):** comes only from `tray-icon 0.26.1` →
  `gtk 0.18` / `libappindicator` (gtk3-rs, capped at glib 0.18; Linux-only, so
  not compiled into the Windows/macOS products). No compatible release exists.
  Options: wait for a `tray-icon` release off gtk3, or drop the Linux tray
  path. Keep the alert open until then.
- **grid (GHSA-38c5-483c-4qqp):** only the three `spikes/*` crates that use
  crates.io `gpui 0.2.2`; the product uses the Zed `v1.28.1` pin (D-18) and has
  no `grid` in its lockfile. If those spikes are obsolete, delete them (closes
  alerts 1–4) or move them to the pinned GPUI.

`cargo deny check` passes (offline tier above); `deny.toml` unchanged.

## Limits

- CI is not run (cannot push); the macOS job is unverified. The new native
  CI step is Windows-only (`if: runner.os == 'Windows'`), so macOS only runs
  offline as before. xtask native logic is `cfg!(windows)`-gated at run time
  but compiles everywhere.
- The flaky-test cause on CI is inferred, not observed.
