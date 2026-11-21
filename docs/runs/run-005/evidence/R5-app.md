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

## Restore from Removed (removal.md §7.4, after merge of 9d3a144)

- Confirmation: the Removed list's Restore now expands to "This reinstalls X on all computers in this group.
  If World of Warcraft is running, it waits until the game closes." (X plain title, colour codes stripped)
  with Restore / Cancel. The add-on history restore (recovery points) sends a `Rollback`, not
  `RestoreRemoved`; its confirmation already says every computer, so it was left unchanged.
- Result: `Recorded` for RestoreRemoved reads "X will be reinstalled on all computers in this group. If World of
  Warcraft is running, it waits until the game closes." `CommandError::Service("restore unavailable: …")` reads
  "X can't be restored: <reason>. Nothing was changed." (`ActionResult::RestoreUnavailable`). The title travels
  as display-only `GroupAction::RestoreRemoved.title` and `GroupResult.subject`; it is never sent.
- Tests: `removed_ui::tests::the_confirmation_says_it_reinstalls_the_plain_title_on_all_computers`,
  `a_queued_restore_says_it_installs_everywhere_and_waits_for_the_game` (case `R5-app:restore-confirm-result`).
- A-38 controls (exit 101 each, reverted): queued text mapped to the Rollback arm instead; confirmation text
  changed to "on this computer".

## Restore progress (item 3, after merge of debc58f)

- `StatusSnapshot.restores` is kept in `FeatureData.restores`. Waiting: a muted line on the add-on's Removed row
  (matched by first folder) and the headline sub-line (after standing and review needed). Unavailable: an
  error-tone Needs item showing `text` (target `NeedTarget::Restore`, opens Add-ons) and, on the Removed row, the
  text with Try again (`RestoreRemoved` again) and Dismiss (`GroupAction::DismissRestoreIssue`, result "Dismissed.
  Nothing was changed."). It is not DataAtRisk. `title`/`text` pass through `strip_colour_codes` and are drawn as
  plain text.
- Tests: `model::tests::a_refused_restore_is_a_needs_item_and_a_waiting_one_is_only_a_status_line`,
  `removed_ui::tests::progress_belongs_to_the_row_of_its_first_folder_and_dismiss_names_that_folder`
  (case `R5-app:restore-progress`).
- A-38 control: filtering the Needs item on `Waiting` instead of `Unavailable` failed the test (exit 101), reverted.
- Not wired: the Needs page has only the navigate button; Try again and Dismiss live on the Removed row.

## Notes

One edit to `about_ui.rs` was written with a .NET file write by mistake instead of the edit tool
(content correct, reviewed in the diff).
