# Evidence: R4-app (run-004, phase D, the GUI for the run-004 features)

Branch `run004/R4-app`, base main `45a1c3e`. Environment: Windows 11 x64, native. macOS was not
available: every macOS row is NOT RUN. No GUI window was driven by the worker (see "Manual steps for
R4-check").

## 1. Control map (work item 1)

Read: `crates\wow-sync-runtime\src\{control.rs,lib.rs,service.rs}` and `crates\wow-sync-app\src\backend.rs`
on base `45a1c3e`.

**Finding on the base `45a1c3e` (superseded):** `wow-sync-runtime::lib.rs` did not export
`Command`, `CommandOutcome` or `CommandError`, so the app could not send any command. Main
`979b8de` now exports them (and `RollbackRefusal`, `SettingsHistoryEntry`), and this branch merged
it (`59f7327`). The table below is the original map; section 2 says what is wired now. The
`Capabilities::runtime()` set in `pull` marks every exported command as wired.

Still absent when this was written (R4-integrate adds them): core `Headline::ReviewNeeded`,
`StatusView.hold`, `StatusSnapshot.{hold, retirements, update}`, `MemberRow.retired`,
`AppSettings.{update_check, notify_summaries, notify_needs_me}`, and the `CheckUpdates` /
`ExportDiagnostics` / `SaveDiagnostics` commands. The app takes the data it cannot read through
`features::FeatureData`, which R4-integrate fills.

| GUI need | Existing variant / query | Callable from the app now? | App behaviour |
|---|---|---|---|
| Choose an option on the Decision page | `Command::Decide { item, option, shown }` | No (type not exported) | button built from `DecisionItem`; disabled with reason until wired |
| Offered decisions | `SliceService::offered_decisions()` (pub) | **Yes** | read in `backend::refresh`, shown on the Decision page |
| Keep / Resume (Change decision, Resume automatic updates) | `Command::Keep`, `Command::Resume` | No | `GroupAction::{Keep,Resume}`, disabled until wired |
| Retire a computer | `Command::Retire { subject: EnrollmentRef }` | No | sheet and gating built; **Retire** disabled until wired |
| Join again | `Command::ReEnroll` | No | **Join again…** offered on retired/held headline; disabled until wired |
| Allow again (rejected release) | `Command::AllowAgain { rejections }` | No | disabled until wired |
| Go back to this release | `Command::Rollback { package, recovered }` (+ `CommandError::Rollback(NotAvailable...)`) | No | confirmation + result texts built; disabled until wired |
| Restore a removed addon | `Command::RestoreRemoved { removal }` | No | disabled until wired |
| Settings master and exceptions | `Command::SetSettingsPolicy { master, exceptions }` | No | disabled until wired |
| Share my settings | `Command::ShareSettings { unit, checkpoint }` | No | disabled until wired |
| Consent to settings sync | `Command::EnableSettings { included_accounts }` | No | disabled until wired |
| Settings history | `SliceService::settings_history(unit)` (pub) | Yes, but its entry type is in a private module | not read; handover |
| Pause / resume all settings, Use shared settings, Restore settings, settings confirm | **no variant** | n/a | disabled: "needs a WoW Sync update" (handover) |
| Check for updates, update state | `UpdateService` (private, not wired) | No | About shows the stored result when given; button disabled (handover) |
| Notification toggles | `AppSettings` has no such fields | No | toggles shown disabled (handover) |
| Export diagnostics | `diag::export_diagnostics` (private) | No | sheet model built on `core::diagnostics`; **Export diagnostics…** disabled (handover) |
| Pause / resume syncing, Sync now, quit | `Cmd::{Pause,Resume,SyncNow,QuitWhenSafe}` (existing) | Yes | unchanged |

## 2. Per-item results

Every command goes through one seam: `features::GroupAction` → `to_command` → `Cmd::Group` →
`SliceService::command` → `ActionResult` text in the notice line. `features::gate` refuses first:
retired computer (publishing actions), no runtime command (`NEEDS_UPDATE`), not wired (`NOT_WIRED`).

| Item | Commit | Scope | Tests (all offline, all pass) |
|---|---|---|---|
| 1 | `e5f60af` | Control map (section 1) | n/a |
| 2 (foundation only) | `f10ce28`, `ceafb99` | `features.rs`: actions, gate, results, hold sentences | `features::tests::*` (case `R4-app:feature-gating`) |
| 3 Decisions | `ceafb99` | `decisions_ui.rs`, decision card on the Decision page; warning needs a tick | 6 in `decisions_ui::tests` (case `R4-app:decisions`) |
| 4 Computers | `ffc9f65` | `computers_ui.rs`: retire panel (names the enrollment, 72 h warning, master note), standing headline, Join again, Retired group, Allow again | 7 in `computers_ui::tests` (case `R4-app:computers-retire-join-allow`) |
| 5 Rollback | `4c86684` | `rollback_ui.rs`, history list on the add-on page: Go back (confirmation), Allow again, "Blocked: X" chip, older-peer reason | 5 in `rollback_ui::tests` (case `R4-app:rollback`) |
| 6 Settings | see git log (`item 6`) | `settings_ui.rs`, Settings sync page: master, per-add-on exceptions, share, use shared, choose, restore + confirm, pause, resume | 7 in `settings_ui::tests` (case `R4-app:settings`) |
| 2 (headline), 7 (removed), 8 (About, export) | not done | wait for the snapshot (`review_needed`, removed rows, update flags) | none |

Negative cases: warning not confirmed (3); retire without the tick, retire while paused, no
enrollment (4); older peer, retired, blocked release (5); retired computer cannot share or change
master, share when nothing is saved, enable with no account, exception with no master, share via
Resume is refused with a reason, "Stay paused" is not a command (6).

## 3. A-38 mutation controls

Each mutation was made and reverted with the edit tool only, run sequentially, and the tree was
clean afterwards.

| Item | Mutation | Result |
|---|---|---|
| 3 | `if false && o.needs_confirm && !confirmed` in `decisions_ui::choose` | caught: `a_warning_must_be_confirmed_before_the_choice_is_built` FAILED |
| 4 | `if false && !acknowledged` in `computers_ui::retire_action` | caught: `retire_needs_the_tick_and_names_the_exact_enrollment` FAILED |
| 5 | `if false && !r.rejections.is_empty()` in `rollback_ui::button` | caught: `each_state_gets_the_right_action`, `go_back_is_disabled_with_a_reason_when_it_cannot_be_planned` FAILED |
| 6 | `if true \|\| *reason != LocalReason::LocalChange` in `settings_ui::unit_controls` | caught: `local_difference_offers_share_and_only_some_offer_use_shared` FAILED |

## 4. Pending snapshot fields (mapping to `FeatureData`)

| App field | Needed from the snapshot | Used by |
|---|---|---|
| `features.decisions` | already fed from `offered_decisions()` | item 3 |
| `features.standing` | retirement info (`Retired{on,by}`) and `hold` | item 4, gating |
| `features.hold` | `hold` | hold sentences |
| `features.enrollments` | peer id → `EnrollmentRef` | retire, master, exceptions |
| `features.retired` | retired rows | Computers "Retired" group |
| `features.allow_again`, `features.history` | held/rejected rollback releases per add-on | items 4, 5 |
| `features.older_peer` | live member with an older WoW Sync (Q-84) | item 5 |
| `features.master` | settings master name | item 4 note, item 6 banner |
| `features.settings` | units (status, live checkpoint, heads, history), exceptions, master `EnrollmentRef`, accounts, enabled, paused folders | item 6 |
| (not started) | removed rows, id→name map, `review_needed`, `update`, notification and update flags | items 2, 7, 8 |

Dead-code allowances: `#![allow(dead_code)]` in `features.rs`, `rollback_ui.rs` and `settings_ui.rs`
cover fields the snapshot will feed (and `rollback_ui::summary`, `exception_line`, not yet shown).
Remove them when the mapping is connected.

## 5. Manual steps for R4-check

No GUI window was driven by the worker. On Windows, with a demo group:

1. Decision page with a warning option: Choose stays disabled until the tick is set.
2. Computers → a peer → Retire: the panel names the computer, warns if silent 72 h or more, notes
   the master; the Retire button needs the tick. **Cancel does not get default keyboard focus**: the
   panel is inline, not a modal sheet (ops.md wants Cancel focused). Check it, and tell the owner if
   it matters.
3. Retired self: the Computers page shows "This computer was retired" and **Join again…**; every
   publishing control is disabled with the retired reason.
4. Add-on page → Recovery copies: Go back to an older version shows the confirmation; a blocked
   release shows "Blocked: X" and **Allow again…**.
5. Settings sync: Change master…, Add an exception…, Share my settings, Use shared settings,
   choose, Restore (confirm first), Pause settings, Resume… Share on Resume is disabled with its
   reason. Tab through and check focus rings and screen-reader names. macOS: NOT RUN.

## 6. Handover to R4-integrate

- Fill `FeatureData` from the snapshot (table in section 4) in `app.rs::pull`; leave
  `Capabilities::runtime()` as it is.
- Add `Command::CheckUpdates`, `ExportDiagnostics`, `SaveDiagnostics` and the `AppSettings`
  notification and update flags. `features::ActionKind::{CheckUpdates, ExportDiagnostics,
  SaveNoticePrefs}` stay disabled ("needs a WoW Sync update") until `has_runtime_command` is true
  for them.
- Settings pause/resume exist (`PauseSettings`, `ResumeSettings`); the resume sheet currently
  passes `here_changed=false, shared_newer=false` because the snapshot does not give them. Provide
  both flags per paused folder.
- The Settings page lists add-ons from `w.addons`; exceptions need `FolderKey`-valid add-on keys.

## 6b. Later commits (after snapshot merge `fba5b63`)

| Item | SHA | Notes |
|---|---|---|
| Merge main (snapshot) | `e2f3208` | |
| Connect snapshot | `8f74af9` | Computers, history, Settings fed from real snapshot. |
| Item 2 headline/tray | `9b0abb1` | `review_needed`, retired-self, Removing/Waiting. |
| Item 7 removed | `fc82979`, fix `0754875` | Restore disabled (`RESTORE_UNAVAILABLE`). |
| Item 8 About/diagnostics | `a702bcc` | updates, toggles, export/save. |
| B macOS behaviour | `18785ca` | `platform.rs`, socket single-instance, hide/show, paths, offset. |
| A attach_os_services | `e9f4765`, fix `ab0579e` | `open_service` calls attach at startup and Enroll. |

A-38 controls (all caught by tests; reverted; tree clean):

| Case | Mutation | Result |
|---|---|---|
| snapshot-connect | `.filter(\|r\| !r.is_this)` removed | caught |
| headline | dropped `!core_blocks_pause` guard | caught |
| removed | `\|\| true` on awaiting line | caught |
| about-diagnostics | preview not cleared on option toggle | caught |
| attach-os-services | `os_services && false` | caught |

macOS items (second launch raise, hide/show, host name, state dir, `date +%z` offset): code
cfg-gated and NOT compiled/run on Windows; only pure helpers (`mac_state_dir`, `parse_utc_offset`,
`show_socket_path`, visibility request) are tested (case `macos-platform-logic`). Rest NOT RUN.

Manual steps for R4-check on macOS: launch twice (second raises first); close window (hides to
menu bar; tray Show restores); confirm computer name, `~/Library/Application Support/WoW Sync`,
and local time. On Windows: inline retire panel Cancel is not default-focused (known gap);
confirm notifications fire after startup (os services attached unless developer mode).

Remaining snapshot gaps (handover to R4-integrate): active-peer enrollments (Retire/master
disabled); settings units/master/accounts/paused flags; `rejections` hashes and `older_peer`;
retirement/held dates; removal hash in `RemovedRow` (Restore disabled); resume-sheet
`here_changed`/`shared_newer`; DataAtRisk notice producer; diagnostics journals,
`process_memory_mib`, `os_version`; Keep/Resume screens; diagnostics save uses a default path (no
file picker). `settings_ui.rs`, `rollback_ui.rs` keep `allow(dead_code)`.

## 6c. Snapshot data connected (main `685f687`)

| Area | SHA | Wired |
|---|---|---|
| Peers / Retire | `364d25c` | `peers` to retire targets (Active, not this), retirement dates |
| Settings | `70fbf26` (contains a mistaken mutation line), fixed in `5a8fa41`; test via snapshot `723c5cb` | master (incl. this computer), exceptions, conflict, resume sheet flags from `paused` |
| Removed restore | `f1b7ada` | `removal_hashes` to `RestoreRemoved` |
| Allow again / older peer | `9c4c06d` | `rejected_releases`, blocked history, `older_peer` |

A-38 (all caught, reverted, tree clean): peers (accepting non-Active), settings (dropping
`policy_conflict`), removed (`get(0)` instead of `get(index)`), allow-again (filter always true).

Still disabled, with the reason shown in the GUI: Turn on settings sync (no list of accounts to
choose from; `enable_control` unused); Keep and Resume (no screen). Narrow `dead_code` allows remain
on `UnitStatus`, `enable_control`, and the rollback outcome/summary types.

## 6d. Settings units connected (main `4b7c31b`)

Commit `16e6bf5`: `settings_units` to `SettingsData.units` (`features::settings_unit_rows`).
Share needs `can_share` plus `latest_capture`; Choose uses `competing` and `shown_heads`; Restore
uses non-head `history` entries and is gated by `restore_possible`. A `Disabled` row with consent on
is shown as "Settings sharing is off until a size limit is approved" (Q-77, no cap invented); with
consent off it is "Settings not shared". A-38: showing head entries as restorable (caught by
`the_units_list_wires_share_choose_and_restore_from_the_snapshot`), reverted, tree clean.

Still disabled: Share/Restore on any unit whose `can_share`/`restore_possible` is false (no capture,
no history, consent off, paused/blocked, or retired); every unit while the production cap is unset;
Turn on settings sync (no account list); Keep/Resume screens; Cancel is not the default focus in the
retire panel; diagnostics journals, `process_memory_mib`, `os_version` empty and no save file picker.

## 6e. R4-check findings fixed (merge of main `5f9a125`)

| Finding | Commit | A-38 mutation (caught, reverted, tree clean) |
|---|---|---|
| V-04 results visible on their own page (`notice.rs`) | `2aa5f36` | `for_page` ignoring the page |
| V-02 Keep/Resume change decision + local recovery (`change_ui.rs`) | `001e668` | offering a rejected release for Keep |
| V-03 retired master not shown as sharing (`settings_ui::banner_text/active_master`) | `b42f65a` | `active_master` always `Some` |
| V-07 distinct "Keep this" names (`OptionRow::choose_name`) | `d499e13` | dropping holders from the name |
| V-05 Turn on settings sync with `available_accounts` | `e0cc0cb` | not filtering to listed accounts |
| V-10 nav entry no "Coming later" | `a0cea75` | restoring the suffix |
| V-09 rollback confirmation names the installed version once | `0d20de9` | `version` repeated before the name |
| V-08 Cancel default focus in retire panel | `e0ae641` | focusing Retire instead |

Notes for R4-integrate / R4-check: Keep uses `ChoiceReason::Ambiguous` and an empty `allow_again`
because the snapshot has no original reason or inherited exclusions (handover gap). A retired
master shows "no active master"; no replacement is elected. Manual: open Computers > retire panel
and confirm Cancel is focused; Settings sync page with sync off lists account toggles.


Both run on the final tree (Windows 11 x64, native, worktree `D:\GitHub\wow-sync.worktrees\R4-app`,
branch `run004/R4-app`, main not re-merged after `979b8de`):

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo xtask verify --tier native`: PASS (`native/tier`).
- macOS: NOT RUN.

## 6f. Recheck round 2 and pass 15 (merge of main `c149a30`)

| Finding | Commit | A-38 control (caught) |
|---|---|---|
| V-04 app part: queued rollback worded "Requested. It will happen on the next sync…" | `643ce5b` | yes |
| V-03 residual: banner from the missing global cap (`MAX_SETTINGS_FILE_BYTES`, app-side) | `b38e017` | yes |
| V-09 app part: installed version named in the confirmation | `b124522` | yes |
| V-02 GUI: `recovery_points` listed per add-on with Restore and confirmation | `afe940b` | yes (package filter ignored) |
| V-04 outcome: `rollback_outcome` shown on its package's page | `d650b44` | yes (`Applied` warns) |
| V-09 date: `held_release_dates` mapped to `held_on`, local time | `8c4de99` | yes (map lookup ignored) |

Notes: a peer-only release has no held date, so none is shown. The Keep reason for a recovered
release is `ChoiceReason::Ambiguous` with empty `allow_again` because the snapshot lacks the
original reason; this stays a handover gap. macOS: NOT RUN.



