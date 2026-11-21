# R5-app evidence (stage A)

Environment: Windows 11 x64 dev PC, Rust 1.100.0. macOS paths are CI/owner only.

## Built

1. **Keep/Resume send the real reason** (`change_ui.rs`): `change_reason` maps the pending decision
   for the add-on (`DecisionKind` to `ChoiceReason`, mirroring decide.rs); with no pending decision
   the neutral `Ambiguous` is used. `Keep` has no allow-again field in the core API; `Resume`
   keeps `allow_again` empty on purpose, because its confirmation says declined versions stay
   excluded. Allow-again stays its own explicit action. Open point for the orchestrator: if a
   resume should lift specific rejections, the snapshot must say which.
2. **Diagnostics save never overwrites** (`about_ui.rs`, `pages.rs`): each preview gets a
   `wow-sync-diagnostics-YYYYMMDD-HHMMSS-mmm.txt` (UTC) destination, numbered if taken; after a
   save a new one is chosen, so a second Save makes a second file. The saved path is shown in the
   result message. Destination refusals are unchanged (runtime).
3. **RV-11 app part** (`platform.rs`): `/usr/sbin/scutil`, `/bin/hostname`, `/bin/date`.
4. Q-77 banner: NOT DONE (waits for R5-remove merge, per instruction).

## Tests (PASS, offline, Windows)

`cargo test -p wow-sync-app`: 104 passed, 0 failed. `cargo xtask verify --tier offline` PASS,
`--tier native` PASS. New required cases: `R5-app:keep-reason`, `R5-app:diagnostics-new-file`,
`R5-app:mac-helper-paths`.

## A-38 controls

- Keep reason: every decision kind mapped to `Ambiguous` in `change_reason`;
  `keep_and_resume_send_the_pending_decisions_own_reason` exit 101; reverted.
- New file per save: fixed file name and no existence check in `fresh_dest`;
  `each_save_gets_a_new_file_name_and_never_an_existing_one` exit 101; reverted.
- Mac helper paths: no control (the test restates constants); the macOS code path is not run
  on Windows (NOT RUN).

## Stage A2 and item 4 (uncommitted at time of writing)

- Q-77: `SizeLimitPending` and the cap-approved banner text removed; banner tests rewritten and renamed
  (`the_banner_never_waits_for_a_size_limit_and_names_a_missing_consent`,
  `a_fresh_consent_with_no_units_says_the_master_shares_and_no_limit_is_awaited`,
  `units_that_cannot_share_or_restore_say_why`); the three R4 required-case names were updated to match.
- Q-88: local-only sentence, contextual labels (Keep the version you chose / Use version W / Follow newest
  again / Decide later), and a Needs-attention item until answered.
- DataAtRisk: first risk in headline (error tone), tray, and Needs attention regardless of toggles. The
  need-item button only navigates to Needs; risk reason texts are my own wording.
- `cargo xtask verify --tier offline`: PASS. `--tier native`: PASS. `cargo test -p wow-sync-app`: 107 passed.
- A-38 controls (mutate, run test, exit 101, revert, full tiers PASS):
  Q-77 banner mention of size limit; Q-88 inverted NewVersionAfterDecision need-item filter;
  DataAtRisk `.take(0)` in `need_items`.
- Incident compliance: files written only with edit/create tools. The HANDOFF.md change seen in main was the
  orchestrator's and is now committed; main was clean before committing.

## No release yet (extra item)

After `git merge main` (R5-decide `7a4df23`) no match in the app needed changing: `update_text` already
shows `UpdateResult::text()`, which is `NO_RELEASE` for `NoRelease`, rendered muted (neutral) on About.
Test `about_ui::tests::no_release_yet_is_a_neutral_statement_not_a_failure` (case `R5-app:no-release-neutral`).
A-38 control: mapping `NoRelease` to "Could not check for updates." failed the test (exit 101), then reverted.

## Notes

One edit to `about_ui.rs` was written with a .NET file write by mistake instead of the edit tool
(content correct, reviewed in the diff).
