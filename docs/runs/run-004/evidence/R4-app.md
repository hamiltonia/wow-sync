# Evidence: R4-app (run-004, phase D, the GUI for the run-004 features)

Branch `run004/R4-app`, base main `45a1c3e`. Environment: Windows 11 x64, native. macOS was not
available: every macOS row is NOT RUN. No GUI window was driven by the worker (see "Manual steps for
R4-check").

## 1. Control map (work item 1)

Read: `crates\wow-sync-runtime\src\{control.rs,lib.rs,service.rs}` and `crates\wow-sync-app\src\backend.rs`
on base `45a1c3e`.

**Finding that shapes everything below:** `wow-sync-runtime::lib.rs` does **not** re-export
`Command`, `CommandOutcome`, `CommandError` (they live in the private `mod control`), nor
`updates`, `notices`, `diag` (private, `dead_code` allowed). `SliceService::command` is `pub`, but
its parameter type cannot be named from the app crate, so the app **cannot send any group command
today**. R4-app does not own `lib.rs`. Every action is therefore built behind one seam
(`features::GroupAction` plus `backend::dispatch`) and is shown **disabled with a reason** until
R4-integrate exports the types and the app's `Capabilities` lists the action as wired (see
"Handover to R4-integrate").

Also absent on this base (the design lists them as step-1 additions, they are not in the code):
core `Headline::ReviewNeeded`, `StatusView.hold`, `StatusSnapshot.{hold, retirements, update}`,
`MemberRow.retired`, `NextAction::{ReviewSetup, JoinAgain, ChangeMaster}`,
`AppSettings.{update_check, notify_summaries, notify_needs_me}`, and the `CheckUpdates` /
`ExportDiagnostics` control variants. The app derives what it can from the types that do exist
(`HoldScope`, `AllHoldCause`, `RetiredInfo`, `DecisionItem`, `UpdateResult`, `diagnostics::*`) and
takes the rest through `features::FeatureData`, which R4-integrate fills.

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

(Work items follow below as they are completed.)
