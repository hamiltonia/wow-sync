# R2-adapters evidence

Worker: R2-adapters (claude-sonnet-5.5). Branch `run002/R2-adapters`.
Environment: Windows 11 x64 dev PC, Rust 1.100.0 (pinned), temp directories only.
Scope: `crates/wow-sync-adapters`, root `Cargo.toml` (crates.md §3), `tests/required-cases.toml`.

## Commands and results

| Command | Result |
| --- | --- |
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) |
| `cargo test -p wow-sync-adapters` | PASS: 45 passed, 0 failed, 3 ignored (5 consecutive runs) |
| `cargo test -p wow-sync-adapters -- --ignored` with `WOWSYNC_PROVIDER_TEST_ROOT=D:\Dropbox\Gaming Sync\WoW Sync Test\r2-adapters-run` | run once, see Provider |

## Required cases

Offline tier (Windows, run by xtask): all PASS.

| Case | Tests | Result |
| --- | --- | --- |
| A-23:adapters | 6 (fs_safe x4, fs_install x2) | PASS |
| scan:adapters | 9 (fs_scan) | PASS |
| A-22:adapters | 4 (fs_install) | PASS |
| A-20:adapters | 7 (provider) | PASS |
| local:adapters | 7 (localstate, sysinfo, winproc) | PASS |
| fixture:adapters | 3 (testkit) | PASS |

Native tier (Windows 11; xtask `native` is BLOCKED by design Q-17, so these ran
through plain `cargo test -p wow-sync-adapters`):

| Case | Tests | Result |
| --- | --- | --- |
| A-23:adapters-native | junction_inside_root_is_refused, nested_junction_marks_folder_unsafe, junction_under_provider_root_is_refused | PASS |
| A-22:adapters-native | rename_fails_while_live_file_is_open, locked_file_marks_folder_unreadable | PASS |
| A-21:adapters-native | fake_client_under_variant_is_running, process_outside_variant_is_not_running (copied ping.exe, stopped by PID) | PASS |

Provider tier (`D:\Dropbox\Gaming Sync\WoW Sync Test`, synthetic data, one run):

| Test | Result |
| --- | --- |
| dropbox_test_root_round_trip | PASS (158 synthetic objects; listed Local; cleaned up) |
| dropbox_hydrated_tagged_entries_accepted | FAILED in the run (strict form required a cloud reparse tag within 30 s; Dropbox applied none). Post-merge change: the no-tag path now `panic!("BLOCKED: ...")`, so libtest reports failure, never PASS. Not re-run (one-run limit). Remains BLOCKED. |
| dropbox_online_only_entry_refused | BLOCKED: no owner-prepared online-only file (`WOWSYNC_PROVIDER_ONLINE_ONLY` unset). Post-merge change: the unset and not-online-only paths now `panic!("BLOCKED: ...")` instead of printing and returning (which would have reported PASS). Not re-run. Remains BLOCKED. |

Change record: per docs/VERIFY.md a tier that cannot run is BLOCKED, never PASS; no early-return-as-pass paths remain in the crate's tests.

Cleanup: the Dropbox test folder was verified empty afterward.
Not run: macOS; native/provider tiers inside xtask; Dropbox cloud-only/hydration semantics (Q-23).

## Design gaps and interpretations

- `scan_addons_with_hook` added as a test seam.
- `Entry.Unexpected` is undefined; used for names failing segment validation.
- Provider `list` classifies link/other reparse as `UnsafeReparse` before `.tmp`/conflicted checks.
- `remove_file`, `remove_journal`, `remove_staging`, `remove_recovery` are idempotent.
- Unavailable entries in AddOns map to `ScanStatus::Unsafe`; TooLarge/read errors map to `Unreadable`.
- Unstable folders still return files and fingerprint; Unreadable/Unsafe return none.
- `put_object(src)` checks only the final component of `src`; the runtime must supply an authorized path.
- `validate_segment` is stricter than the design (rejects `.`, trailing dot/space, device names, wildcards, control characters).
- `PermissionDenied` maps to `InUse`.
- `InstallArea::lock` uses an exclusive handle plus `try_lock`; a stale lock file is acquirable.
- `random_id_hex` panics if the OS RNG fails.
- Non-Windows code paths are untested; 8.3 short-name comparison in winproc has residual risk.
- Cargo warns about unused workspace deps serde/serde_json until R2-core lands.
- Extra tests not in tests.md: `open_read_enforces_limit_and_refuses_directories`, `read_write_atomic_round_trip_and_bounds` (not in required cases).
