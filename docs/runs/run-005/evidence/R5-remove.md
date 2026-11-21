# R5-remove evidence

Branch `run005/R5-remove` (from main fe13e3d). Environment: Windows, local cargo.

## Done

- **Q-77 / D-26**: `MAX_SETTINGS_FILE_BYTES = Some(33_554_432)`. Tests: cap and cap+1 (core), no-cap-value stays disabled.
- **RV-04**: uninstall captures owned, unshared, unpaused settings files into the local settings store, moves them with the code (`Remove` op with settings_ops, `SettingsCleanup` op when the folder is already gone), writes `RemovalSettings::Checkpoints`, and reserves the matching ack (`paired`, `partial`, `kept_paused`, `code_only`, `not_applicable`). A restore of the addon puts the captured file back when the live file is absent (never overwrites).
- Executor (`exec.rs`) needed no change.

## Tests (offline tier PASS, native tier PASS)

- core lib 350 passed, runtime lib 152 passed.
- New: core `removal_tests` (3), `settings_tests` (cap, targets, no-cap), runtime e2e (3: uninstall+restore round trip incl. peer, over-cap not moved, exactly-at-cap captured).
- Required cases: `Q-77:cap`, `RV-04:uninstall-restore` in the R5-remove section. Two stale names removed from R4-core sections because their tests were replaced (`a_remove_op_with_owned_settings_waits_for_cleanup_and_carries_no_ack`, `without_an_approved_cap_real_data_sync_is_disabled`).

## A-38 controls (each: mutate, test exit 101, revert, no leftover diff)

1. Cap constant 33_554_431 -> `the_approved_cap_is_32_mib...` failed (exit 101).
2. `ack_settings_for` Paired condition `+ 1` -> `a_remove_op_moves_captured_settings_and_reserves_the_matching_ack` failed (exit 101).
3. Restore skipped (`|| true` on the restore guard) -> `an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both` failed (exit 101).

## Deviations and limits

- Unknown declarations give not_applicable, not code_only.
- No receiver-side manifest `.toc` ownership path.
- Captures are local only; a peer restores from its own capture, not from the origin's record.
- Capture needs consent, included accounts and a cap.
- An uninstall with owned files is deferred while WoW runs.
- Not tested at e2e level: paused (`kept_paused`) and WoW-running deferral; covered only by core planning tests.
- `settings_ui.rs:373` banner (R5-app) may change now that the cap is `Some`; not edited.

## File-write disclosure

No .NET file API was used. Once early on I used `Set-Content` on `removal_tests.rs` (content verified intact afterwards); all other writes used edit/create. `git -C D:\GitHub\wow-sync status --short` was empty before commit.

## Fix round: R5-check F1 (originating code survives an uninstall)

**Root cause.** In the cycle where A's own matured removal is recorded, `service.rs` composes the apply target before that record exists (`compose` at the `held` block, ~1930-1990). A's folder is absent, so `plan_apply` installs it again from the group's copy in the same cycle. The next cycle sees a live removal on a present folder that A has already acknowledged as absent, and `reinstall_held` (`removal.rs`, ~260) treats it as a deliberate reinstall, so it is never removed. A's list stays at "removing, not confirmed yet". Reproduced deterministically in `e2e::tests::an_uninstall_after_a_group_restore_converges_on_every_computer` (V-02 fixture: differing same-version copies, B keeps its own, A restores its displaced copy, then A's folder is deleted and the gap matures). It failed with A's code present before the fix. The earlier-recreated-tree history was not needed.

**Fix.** `removal_steps` returns the folder keys whose removal record it just wrote; `sync_once` drops any apply package for those keys from that cycle's plan (`service.rs`, next to `plan_apply`). Safety rules are untouched: remove ops, WoW-running block and offline-peer handling are not changed.

**Test:** `e2e::tests::an_uninstall_after_a_group_restore_converges_on_every_computer` (required case `R5-check-F1:uninstall-converges`). Asserts A and B code absent and one entry in Removed.
**A-38 control:** disabled the new filter (`true ||`): the test failed (exit 101); reverted, diff clean.

**Consequence for my earlier restore test.** It had passed only because the bug left A's code in place. A real uninstall leaves no computer holding the code, and the runtime's `RestoreRemoved` only writes the re-add; the design's §7.4 source (recovery point or manifest trees and blobs, `kind: restore` operation) is not implemented in the runtime. The test now puts the code back by hand and judges the settings half only. This gap needs an owner decision or a follow-up item: Restore from Removed cannot bring the code back when nobody holds it.

No .NET file API was used. Both tiers PASS.