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
