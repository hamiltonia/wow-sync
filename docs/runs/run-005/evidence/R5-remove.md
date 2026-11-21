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

## Round 3: Restore of removed add-ons (removal.md §7.4)

**Built.** Core `removal::plan_restore` (+ `RestoreFolder`, `RestoreError`): checks every staged tree against the record's manifest and `package_hash == removed_hash`, then builds one `OpKind::Restore` apply (`Install`/`Replace`, no removes) whose only pending record is the `user_restore` re-add (`known` = current summary), so the re-add is written at finalize after verification. Runtime `restore_flow.rs`: request/outcome persistence, source selection and refusals, `step` reserving the record sequence. `service.rs` (small): `RestoreRemoved` on a computer missing a removed folder dry-plans (refusal returns `CommandError::Service("restore unavailable: ...")` or `UnknownRemoval`) then queues and returns `Recorded`; `sync_once` plans the request next to the rollback block (Active and WoW not running). A computer that still holds the folders keeps the old direct re-add (`Written`). `SettingsPass::has_capture` added.

**Steps 1-6.** 1 source: provider tree record, else a local recovery point holding that exact tree (`local_release`; any kind, because `LocalPoint` has no kind field and the content is hash-verified anyway); blobs staged by the existing `stage_folder` with the same recovery fallback; a missing tree/blob with no local copy gives `Unavailable` at command time and nothing is queued. 3 `kind: restore` operation with journal and G through the existing executor; prior live state of a `Replace` becomes a recovery point. 4 settings: the paired capture comes back the next cycle via the existing capture book (removal no longer stands); no capture gives the exact R-10 warning as the cycle's `apply_note`. 5 re-add in the operation. 6 an active rejection naming a manifest tree refuses the restore.

**Tests (real behavior).** `e2e::tests::an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both` is a true round trip again (no manual put-back; originating A has no recovery point, so it also covers the provider-manifest source). Added `a_restore_without_a_recovery_point_installs_the_code_from_the_shared_folder`, `a_restore_from_the_local_recovery_point_works_without_the_shared_files`, `a_restore_with_a_missing_blob_is_unavailable_and_leaves_nothing_partial`, `a_restore_waits_while_wow_runs_and_installs_when_it_closes`, `a_restore_of_a_rejected_version_is_refused`, `a_restore_with_no_settings_copy_warns_that_settings_were_not_restored`; core `removal_tests::a_restore_installs_exactly_the_removed_folders_and_reserves_the_readd`, `a_restore_whose_package_hash_is_not_the_removed_hash_is_refused`. `restoring_a_removed_addon_brings_it_back_and_it_is_not_removed_again` now expects `Recorded` and a later `user_restore` record. Required cases `RV-04:restore-code-from-manifest`, `RV-04:restore-refusals`.

**A-38 controls (mutate, exit 101, revert, clean).** (1) `if false && package_hash(&pairs) != removal.removed_hash` in `plan_restore`: the hash-mismatch core test failed. (2) `if false && missing && ...` on the blob check in `restore_flow`: the missing-blob test failed.

**Not done / for others.** (a) The app does not show "This reinstalls X on all computers in this group" (`features.rs`/`removed_ui.rs` not mine). (b) The app's result text for `Recorded` on `RestoreRemoved` reads "Saved on this computer."; it should say the restore is queued until WoW is closed. (c) The refusal reason surfaces as `CommandError::Service(text)`; a typed error or a status field for `restore_flow::load_outcome` (refusal found at cycle time) needs `control.rs`/status changes. (d) Paired vs partial is reported only as paired-or-warning; a partial-capture warning is not separate. (e) `removal.md` §7.4 got an as-built paragraph; contracts.md unchanged.

No .NET file API was used.

## Round 4: restore progress in the status snapshot

**API.** `wow_sync_runtime::StatusSnapshot.restores: Vec<RestoreProgress>`; `RestoreProgress { package: FolderKey, title: String, state: RestoreState, text: String }`; `RestoreState::{Waiting { reason: RestoreWait }, Unavailable { text: String }}`; `RestoreWait::{WowRunning, Paused, NextCycle}` (all exported from the crate root); `Command::DismissRestoreIssue { package: FolderKey }` (returns `Recorded`, or `NothingToDo` when nothing is shown). Title is the folder key with colour codes stripped (lowercase, as in the removed list). Texts: "Restoring X: waiting for World of Warcraft to close." / "...waiting for syncing to be resumed." / "...it will run on the next sync." / "X can't be restored: <reason>. Nothing was changed."

**Behavior.** The request now persists until `restore_flow::prune` sees the removal no longer stands (re-add written), so `Waiting` stays accurate across cycles. A cycle-time refusal is saved as `restore-issue-{group}.json`, clears the request, and stays until retry (`RestoreRemoved` clears it before re-planning) or dismissal. Cycle note: `settings_note` gives the code-only R-10 warning (0 captured), "Settings were partly restored: N of M ..." or "Settings are restored with the code". `public_api.rs` updated (new variant, 21 commands). Only runtime files touched; no app files.

**Tests.** `a_queued_restore_is_shown_as_waiting_for_wow_then_for_the_next_sync_then_vanishes` (WowRunning, NextCycle, disappearance), `a_queued_restore_is_shown_as_waiting_while_syncing_is_paused`, `a_restore_refused_at_cycle_time_stays_listed_until_dismissed_or_retried`, `a_retried_restore_replaces_the_shown_refusal`, `the_settings_note_tells_code_only_partial_and_paired_restores_apart`. Required case `RV-04:restore-progress`. Runtime lib 173 pass; offline and native tiers PASS.

**A-38 control.** Mutating `progress` to ignore `stands` (`true || stands(..)`) made the first test fail (exit 101); reverted, diff clean. A first mutation of `prune`'s request check passed because `progress` filters independently, so `prune` has no direct test.

No .NET file API was used.