# R3-ui-build evidence: real app UI on the real runtime

Run 003, worker R3-ui-build, branch `run003/R3-ui-build`. Links: D-21 (approved prototype and UI
defaults), SPEC UI rules, ACCEPTANCE A-29, A-38, docs/design/ui/screens.md. The approved
prototype modules (pages, theme, components, setup) were copied into `crates/wow-sync-app` and
then edited: every fake-model read was replaced by a read of the runtime snapshot. The app has no
prototype scenario switcher and never shows invented data; unknown values show as absent or as a
"not available yet" sentence.

macOS: **BLOCKED** (no Mac). Windows 11 x64 dev PC only. Release exes were built with
`cargo build --release -p wow-sync-app` and copied to `D:\WoWSyncDemo\bin\` (`wow-sync.exe`,
`devkit.exe`). `wow-sync-proto.exe` was not touched. The slice exe that was still running from
an earlier demo could not be overwritten, so it was renamed to `wow-sync-slice-running.exe`
(the two older demo processes are not mine and were not stopped).

## Runtime work (priority 1)

New runtime modules `control.rs` (persisted pause, app settings, activity log reader),
`discover.rs` (read-only Dropbox and WoW discovery), `detail.rs` (typed `StatusSnapshot.detail`:
members, per-package title/versions/folders/awaiting, activity of the game, provider problem,
last/next check). `SliceService` gained `paused_since`, `set_paused`, `app_settings`,
`save_app_settings`, `activity_log`; quit-when-safe is a gate in the sync thread (the quit
happens only between cycles). Runtime and core cases are registered as `R3-ui-build:runtime` and
`R3-ui-build:core` in `tests/required-cases.toml`.

## Data gaps (docs/design/ui/screens.md, "Status after run-003")

- **Closed:** UG-1, UG-2, UG-3, UG-4, UG-5, UG-6, UG-7, UG-10, UG-11, UG-14, UG-17, UG-22, UG-24; UG-12 partly.
- **Deferred (designed placeholder text, nothing faked):** UG-8, UG-9 (decision and needs-help actions), UG-13 (restore), UG-15, UG-16, UG-18 (update check), UG-19 (OS notifications), UG-20, UG-21, UG-23 (start at sign-in), UG-25 (per-package apply scope only), UG-26 (core decision).
- The new group's name defaults to "My add-ons" (no editor yet). The computer name is display only.

## Screens

| Screen | Status | Evidence |
|---|---|---|
| Setup wizard (steps 1-6, create group) | works on real data, driven by UIA end to end | `shots/s6.png`, `shots/done.png`, mem harness enrols 9 labs through it |
| Overview | real data, light and dark | `overview-*.png`, `overview.uia.txt` |
| Add-ons list and detail | real data, light and dark | `addons-light.png`, `addon-detail-*.png`, `addons.uia.txt` |
| Computers | real data (peers need a second enrolled computer) | `computers-*.png` |
| Activity | real log, jargon rewritten; filters | `activity-light.png` |
| Needs attention | rendered from the real snapshot; decision and needs-help actions deferred | not reached with synthetic data |
| Paused state | persisted, header/banner/Activity | `paused-dark.png`, `paused.uia.txt` |
| WoW running | banner from real process detection | `wow-running-light.png` |
| App settings, About | start-at-sign-in and update check deferred | `settings-light.png` |
| First-close sheet | works; see RC-3 | `first-close-light.png`, `rc3-focus.txt` |
| Reconfirm | wired to the real reconfirm preview and enrol; not reached with synthetic data | unit/consent tests only |
| Quit-when-safe sheet | wired to the backend gate; not exercised by UIA | |

Capture artefact: `PrintWindow` shots show a black left strip and a clipped right edge; the
prototype shots have the same artefact, so it is the capture method, not the app (unconfirmed).
Shots are downscaled to 50% to stay under 1 MB.

## RC-3 focus return

`rc3-focus.txt`: after the first-close sheet opens, Esc returns focus to the control that opened
it (two different openers tested, including after Tab moved focus inside the sheet). The opener is
recorded before focus moves into the sheet. The Keep-running path shares `close_sheet` but was not
run (it needs a running apply). Esc on the first-close sheet now cancels; it no longer hides the window.

## A-29 memory (final binary, `mem/`)

Harness `tools/mem/run.ps1`, counter private working set summed over the app-owned process set,
limit 100 MiB, 3 runs per scenario. I changed the harness to drive the new setup wizard (UIA
names) and to match the new window title; I also acknowledge the one-time close sheet.

| Scenario | steady median per run (MiB) | whole-run max (MiB) | verdict |
|---|---|---|---|
| a. start hidden, idle 120 s | 28.40 / 28.74 / 28.76 | 30.31 | PASS |
| b. shown with input 60 s, hidden, idle 120 s | 35.67 / 34.68 / 34.98 | 37.73 | PASS |
| c. 20 show/hide cycles, idle 300 s | 35.42 / 35.56 / 34.93 | 36.36 | PASS |

Sync start (launch to first state change): 0.18 to 0.26 s (limit 10 s), 9 of 9. Findings: 0.
An earlier full run (before the hidden-start fix) had one finding: the window was briefly created
visible with `--start-hidden`. Fixed by `show: !hidden` in the window options; hidden start now
holds about 28 MiB. Private Bytes (committed, context only) is about 99 to 105 MiB; it is not the gated counter.

## A-38 negative controls (break, fail, restore)

1. `control::set_paused` made a no-op: `control::tests::pause_persists_across_a_restart_until_resumed` FAILED; restored, passes.
2. `backend::humane` match string broken: `backend::tests::refusals_read_as_plain_language` FAILED; restored, passes.
3. `world::plain_text` neutralised: `world::tests::activity_jargon_is_rewritten_and_last_shared_comes_from_the_log` FAILED; restored, passes.

## Known issues and skipped

- Overview "Shared folder" shows the full path, which can reach the window edge.
- Capture shots omit Needs attention, Reconfirm and quit-when-safe (no synthetic state reaches them without the deferred actions).
- OS notifications, start at sign-in, update check, restore and decision actions are deferred (above).
- `cargo xtask verify --tier native` result: see the final commit message and the reply.
