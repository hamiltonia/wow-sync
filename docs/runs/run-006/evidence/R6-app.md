# R6-app evidence (stage A)

Branch `run006/R6-app`, merged with main (url 2.5.9 fix) before verification.
Stage A2 (test-mode GUI) is NOT RUN: it waits for R6-gate's API.

## Built (PLAN goal 4)

- `crates/wow-sync-runtime/src/service.rs`: `refuse_text`, `layout_text`, `block_text` replace every
  `{:?}`-formatted reason that reached the user (setup refusal, layout, blocked add-ons,
  enroll/reconfirm/consent refusals). Each says what is wrong, then what to do.
  `EnrollPreview.retryable` is true only when an outside fix can change the result
  (flavor/build info missing or unreadable, product inactive, AddOns missing, duplicate folder).
- App: `Install.can_retry`. Step 3 shows "Try again" only then; otherwise "Trying again will not change
  this. Choose Back…". The card heading is the product label, or the folder name when refused
  (cause of the blank line: `identity_line` is empty for a refusal, so the heading was empty).
- Step 4 and re-confirmation: eligible and cannot-be-synced counts always shown, first 4 blocked
  reasons plus "and N more".
- `backend::humane` passes refusal text through without the `refused:` prefix.
- Not changed (out of setup scope, still debug-formatted internally): rollback outcome text
  (`rollback_flow.rs`, `service.rs` apply note), settings-cycle notes, update-source error.
- Variants covered: all RefuseReason (18 incl. Layout), LayoutReason (4), BlockReason (7).
  `NotSyntheticInstall` text is generic ("only works with test folders"); A2 refines it with the
  test-mode wording once R6-gate lands.
- `tests/required-cases.toml`: section R6-app, case `R6-app:plain-refusals`.

## Results

| Check | Result |
|---|---|
| `cargo xtask verify --tier offline` (after `git merge main`, Windows 11 x64) | PASS |
| `cargo xtask verify --tier native` | PASS |
| `service::reason_text_tests::*` (2 tests, every variant, no `{`, `}`, enum names or inner raw strings; Back/Try again rule) | PASS |
| A-38 control: `NotSyntheticInstall` text temporarily set to `format!("{r:?}. …")`; test failed with `shows "NotSynthetic"`; reverted, `git diff` clean for that text | PASS (control failed as required) |
| GUI screenshots, debug build, lab `D:\WoWSyncLab\R6-app` (synthetic) | PASS |

Screenshots (`evidence/R6-app/`): `setup-refused-retry.png` (AddOns missing: reason + Try again),
`setup-refused-back.png` (not a test folder: reason, no Try again, points to Back),
`setup-scope.png` (1 eligible, 2 cannot be synced with reasons).

## Limits

- Blocked reasons do not name the other game versions (reason carries no toc suffix list).
- "1 add-ons are eligible" plural wording is pre-existing.
- Screenshots show window edges over the desktop; the app was launched with `--dev` on lab data only.

## Stage A2: test-mode GUI

Built on R6-gate (main `37d489b`): `Command::SetTestMode(bool)`, `StatusSnapshot.test_mode`,
`ServiceError::TestModeBlocked`.

- **Settings** (App settings → Testing): "Test mode: only sync test folders", off by default, with
  one line saying only folders carrying the test marker sync while it is on. Persists through
  `Command::SetTestMode` (`Cmd::SetTestMode` in the backend).
- **Setup step 3**: a folder refused only because test mode is on (`NotSyntheticInstall`, direct or
  via layout) says so in a sentence and offers "Turn off test mode"; choosing it turns the mode off
  and re-inspects the folder. `EnrollPreview.test_mode_blocked` carries this; no "Try again" is
  shown because trying again alone cannot change it.
- **Needs attention / banner / Overview**: an enrolled install blocked by test mode
  (`provider_error` is `TestModeBlocked`) reads "Test mode is blocking this folder" with a
  "Turn off test mode" action, instead of the Dropbox "Start Dropbox, then choose Try again" text.
  The Overview shared-folder row says "paused by test mode".
- Tests: `model::tests::test_mode_blocking_offers_to_turn_it_off_and_never_blames_dropbox`,
  `model::tests::a_real_shared_folder_error_is_not_mistaken_for_test_mode`,
  `service::reason_text_tests::test_mode_refusal_is_flagged_so_the_offer_can_be_shown`.
  Required case `R6-app:test-mode-gui` (offline).

| Check | Result |
|---|---|
| New tests (3) | PASS |
| A-38 control: `blocked_by_test_mode` prefix mutated to `"Zzz"`; blocking test failed (`model.rs:1397`); reverted, `git diff` clean for that line | PASS (control failed as required) |
| GUI, debug build, lab `D:\WoWSyncLab\R6-app` (synthetic; unmarked install, state `st9`/`st10`) | PASS |
| `cargo xtask verify --tier offline` / `--tier native` (Windows 11 x64, after `git merge main` at `37d489b`) | PASS / PASS |

Screenshots (`evidence/R6-app/`): `settings-test-mode-off.png`, `settings-test-mode-on.png` (toggle
clicked; `test-mode.txt` appeared in the state dir), `setup-test-mode-blocked.png`,
`setup-test-mode-turned-off.png` (after the button: "Can be synced", file removed),
`needs-attention-test-mode.png`, `overview-test-mode-banner.png`,
`overview-test-mode-turned-off.png` (banner action: status recovered, "Test mode is off").

## Review fixes (RV6-06, RV6-07, RV6-04)

- RV6-06: `StatusSnapshot.test_mode_blocked` (typed) is set by core's blocked snapshot, which now
  also carries group name, computer name and identity (best effort, blank on read errors). The app
  reads `World.test_mode_blocked` instead of matching error text; the header reads "Paused by test
  mode" with the group and computer names. Screenshot: `header-test-mode.png` (synthetic lab,
  `st10`, unmarked install).
- RV6-07: the backend reports whether `SetTestMode` was applied; the notice ("Test mode is off/on")
  appears only after success, a failure shows "WoW Sync could not change test mode, so nothing
  was changed..." and the status is re-read. No optimistic flip of the shown state.
- RV6-04 (app part): the stale "synthetic installs only" comment in `main.rs` is reworded.

| Check | Result |
|---|---|
| New tests: `the_header_says_test_mode_paused_sync_and_keeps_the_names`, `error_text_alone_never_means_test_mode`, `a_failed_test_mode_change_is_reported_and_never_claims_a_state`, extended `test_mode_persists_and_blocks_unmarked_installs_both_ways`; all in `R6-app:test-mode-gui` | PASS |
| A-38 control: snapshot `test_mode_blocked: true` -> `false` makes the e2e test fail (e2e.rs:704); reverted, passes | PASS |
| Failure path of `SetTestMode` through a real unwritable state dir | NOT RUN (only the notice mapping is unit tested) |
| `cargo xtask verify --tier offline` / `--tier native` | PASS / PASS |
## Setup finish (R6-check G1) and G3 investigation

- G1 cause: after enrollment the shared status stayed at its empty default (`enrolled: false`)
  until the first sync finished, so Finish setup reset the UI into the not-enrolled Welcome.
  Fix: the backend refreshes the status right after a successful enroll, and the app treats the
  install as enrolled once the enroll was acknowledged (`World::confirm_enrolled`). Finish goes to
  Overview showing the status (first sync in progress). Case `R6-app:setup-finish`:
  `world::tests::a_just_enrolled_install_is_never_shown_as_not_set_up` PASS.
- G3: tried on the synthetic lab (`st10`, enrolled, debug build): hide the window with
  `ShowWindow(SW_HIDE)`, then `SW_RESTORE` + `SetForegroundWindow`, then a physical click on
  Pause syncing. Result: PASS, `paused.txt` was written, so hiding and re-showing does not stop
  dispatch. The single-instance wake path uses the same `set_visible` code. No defect found; the
  verifier's failure is likely tooling (focus/injection, the 1x1 IPC window it mentioned). NOT RUN:
  the UIA Invoke pattern, and the release build with a real tray.
- Checks: `cargo xtask verify --tier offline` / `--tier native`: PASS / PASS.
## Observations for others

- (Fixed in the review-fix section above: the blocked header now shows the names.) The Settings toggle sits below the fold at the default window size; screenshots were taken after
  scrolling.
