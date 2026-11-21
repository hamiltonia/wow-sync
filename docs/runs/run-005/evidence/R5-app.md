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

## Notes

One edit to `about_ui.rs` was written with a .NET file write by mistake instead of the edit tool
(content correct, reviewed in the diff).
