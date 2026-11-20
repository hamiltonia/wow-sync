# Evidence: R4-notify-diag (run-004, phase D)

Branch `run004/R4-notify-diag`, base main `22522f9`. Environment: Windows, native.
macOS was not available: every macOS-only row is NOT RUN.

## Verify

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) |
| `cargo xtask verify --tier native` | PASS |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok (duplicate-version warnings only; `deny.toml` untouched) |

## T-row results

| Row | Result | Test(s) |
|---|---|---|
| T-27-1..10 | PASSED (Windows) | `notice::tests::*`, `notices::tests::*` |
| T-27-11 | Windows: toast API accepted the notification under a scoped test AUMID (`manual_toast_shows_the_app_name`, `--ignored`); the displayed app name was not visually confirmed by the worker. macOS: NOT RUN (R4-mac-kit). | manual |
| T-28-1, T-R4D14 | PASSED | `update::tests::parse_latest_*`, `the_release_link_*` |
| T-28-2 | PASSED | `update::tests::due_follows_*` |
| T-28-3..5, 28-11, 28-12 | PASSED | `updates::tests::*` |
| T-28-6 | PASSED | `update_http::tests::request_has_exactly_*` |
| T-28-7 | PASSED (arch guard with plant controls) | `update_http::tests::only_update_http_names_*` |
| T-28-8 | PASSED (static scan; e2e listing in `a_check_leaves_every_file_*`) | `updates::tests::*` |
| T-28-9, T-28-10 | PASSED, Windows only (native tier) | `update_http::tls_tests::*` |
| T-28-13 | Windows: one anonymous GET reached GitHub over TLS and returned HTTP 404 (no release published yet), handled as a failure kind. macOS: NOT RUN. | manual `manual_live_release_fetch_is_one_anonymous_get` |
| T-33-1..7, T-R4D12 | PASSED | `diagnostics::tests::*` |
| T-33-8, T-33-9 | PASSED | `diag::tests::*` |
| T-33-10 | NOT RUN (GUI; R4-app) | n/a |

## A-38 controls (applied with the edit tool on committed code, reverted, `git diff --exit-code` = 0)

| Control | Result |
|---|---|
| T-28-12: pre-request attempt write removed | 2 tests FAILED |
| T-27-9: ledger save removed | 3 tests FAILED (exit 101) |
| T-28-7: planted `use ureq::Agent` in `diag.rs` | guard FAILED, names `diag.rs:4` |
| T-33-2: output scan made a no-op | `a_string_forced_into_a_typed_field_is_refused_whole` FAILED |

## Dependencies (E-06)

- `notify-rust =4.19.0`: native notifications (WinRT toast, macOS).
- `ureq =2.12.1` (no default features, `native-tls`) and `native-tls =0.2.18`: schannel/Security.framework TLS, so no bundled crypto stack and the OS trust store is used.
- `windows-sys` features `Win32_UI_Shell`, `Win32_System_Registry`: AUMID registration.
- Lock side effects: new transitive crates (schannel, security-framework, openssl-sys family via native-tls, url/idna/icu, `windows` 0.62, tauri-winrt-notification); some crates moved windows-sys 0.59 to 0.61; tempfile's getrandom 0.3 to 0.4. Deny passes.
- macOS build was not run locally; the dependencies are cross-platform and not Windows-gated.

## Deferred / notes

- `runtime\src\{updates,notices,diag}.rs` carry a module-level `#![allow(dead_code)]` because `lib.rs` is not mine. R4-integrate must re-export them and drop the allowance.
- T-33-10, the GUI strings, and macOS rows are for R4-app / R4-mac-kit.

## Handover

R4-integrate:
- Call `NoticeRunner::after_cycle(events, open_now, game, prefs, now_ms)` at the end of each cycle.
- Own an `UpdateService`: `tick(enabled, ready_for_ms)` on the schedule, `check_now()` for CheckUpdates, `view()` for About, `notice_event()` into the planner events, `note_notified(tag)` after the notice.
- ExportDiagnostics handler: `diag::export_diagnostics` for the preview, `diag::save_export` / `export_and_save` with the shared folder and install roots as `forbidden`.
- At startup: `winproc::register_app_user_model_id(notify::APP_ID, notify::APP_NAME, None)` then `set_process_app_user_model_id(notify::APP_ID)`; build `OsNotifier::with_app_id(APP_ID)`.
- Production transport: `update_http::GithubReleases::new()`.

R4-app strings/states: About shows "Could not check for updates." / "Checking for updates…" / "You have the newest version." / "Version X is available" (`UpdateResult::text`); export sheet uses `diagnostics::{NEVER_INCLUDED, SHARED_FOLDER_WARNING, REFUSED_TEXT}` and `Replaced::summary`.
