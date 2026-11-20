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

## 7. Verify

Both run on the final tree (Windows 11 x64, native, worktree `D:\GitHub\wow-sync.worktrees\R4-app`,
branch `run004/R4-app`, main not re-merged after `979b8de`):

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo xtask verify --tier native`: PASS (`native/tier`).
- macOS: NOT RUN.


