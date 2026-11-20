# R4-integrate evidence (stages A, B, C0, rollback, settings commands and policy parts done)

Branch `run004/R4-integrate`. Status: **partial**. Stage A, the code-state `authority_index`,
stage B (authority plane, compose with constraints, removal through the cycle) and a subset of
stage C (`Decide`, `Retire`, `AllowAgain` commands, kept-record settling) are built and tested.
The rest of stage C, C0 (settings `PackageApply`) and stage D are DEFERRED below; nothing here
claims they work.

The orchestrator accepted two stage-B edits as within the approved intent: the core
`decision.rs` match-arm change and the `e2e.rs` service tests.

## Stage C (partial): control commands

`control.rs` declares `Command`, `CommandOutcome`, `CommandError`; `SliceService::command` runs
them against the freshly observed world and routes the effect through `decisions::route`
(retired → refused, held/paused → kept in the persisted `KeptQueue`, otherwise written).
`SliceService::offer_decisions` sets the items an answer is judged against. `sync_once` settles
the kept queue first (`settle_kept`): it is written once nothing blocks it, and discarded when
the computer is retired. The used sequence number is persisted before it can be handed out again.

| Command | Parameters | Result | Meaning for the UI |
|---|---|---|---|
| `Decide` | `item: DecisionId`, `option: OptionId`, `shown: EvidenceToken` | `Written(hash)`, `Kept`, `Local(LocalResolution)`; errors `Decide(Changed \| UnknownOption \| NotAllowed \| UnknownItem)` | `Changed`: "this changed since you looked", reload the item. `Kept`: "saved, applies when syncing resumes" |
| `Retire` | `subject: EnrollmentRef` | `Written(hash)`, `NothingToDo` (not a member or already retired); error `NotAllowed` (this computer is retired or held) | Retirement is group-wide; the subject can publish nothing new |
| `AllowAgain` | `rejections: Vec<Sha256>` | `Written(hash)`; errors `UnknownRejection(h)` (never existed or already lifted), `EmptySelection` | The rejected release may be installed again |

Tests: `e2e::tests::decide_command_is_checked_for_freshness_and_kept_while_paused` (local
answer, stale → `Changed`, unknown option, paused → `Kept`, written after resume),
`retire_command_writes_a_retirement_and_a_retired_computer_is_refused`,
`allow_again_lifts_only_an_active_rejection`.

A-38 controls for C (edit-tool mutation, named test failed, reverted, `git diff --exit-code`
clean):

| Control | Mutation | Failing test |
|---|---|---|
| Decide freshness check bypassed | core `decide.rs`: `if current.evidence != *shown` → `if false && ...` | `decide_command_is_checked_for_freshness_and_kept_while_paused` (got `Written`, wanted `Changed`) |
| Paused decision written at once | `decisions.rs` `route`: `Active if cx.paused` → `Active if false && ...` | same test (got `Written`, wanted `Kept`) |
| Rejection that is not active accepted | core `reject.rs` `build_lift`: `return Err(NotActive)` → `continue` | `allow_again_lifts_only_an_active_rejection` |

Two controls the orchestrator named do not exist: there is no rollback command (needs live
R1/R0 inputs, stage D) and no settings push command (settings step not built), so "unrouted
rollback" and "non-master push without explicit intent" are NOT RUN.

## Done

| Item | Change | Tests (offline tier) |
|---|---|---|
| Ledger fix (A) | `ledger.rs` `kind_rules`: a `SettingsRevision` parent needs the same unit, any kind | `ledger::tests::a_settings_revision_parent_needs_the_same_unit_and_any_kind`, `graph_from_ledger_keeps_equal_heads_and_marks_an_unknown_parent_pending`, `policy_heads_drop_a_policy_that_a_later_one_knows` |
| Kept queue persistence (A) | `decisions.rs` `KeptQueue::{load,save}`; items that do not hash to their name are dropped | `decisions::tests::a_kept_queue_survives_a_restart_and_a_tampered_item_is_dropped` |
| Settings epoch/seq/kind persistence (A) | `settings_cycle.rs` `EpochStore`; damaged state loads as "nothing clean" | `settings_cycle::tests::the_epoch_state_survives_a_restart_and_unreadable_state_is_not_clean` |
| Code-state `authority_index` + `features` | `records.rs`, `plan.rs`: additive fields (contracts §1, §2); `publish_needed` republishes when either differs | `records::tests::authority_index_and_features_are_additive_and_old_readers_ignore_them`, `plan::tests::publish_needed_decisions` |
| B1 authority plane, read side | Every member's `authority\*.json` is read into `Ledger`/`Membership` and persisted; `RemovalStore::observe` runs in the cycle; a retired or unresolved computer writes nothing | `e2e::tests::matured_removal_is_written_learned_and_survives_a_restart`, `records_from_non_members_and_forged_copies_are_not_admitted`, `a_retired_computer_publishes_and_writes_nothing` |
| B2 publish and mirror | Own records are listed in the published `authority_index` chain; peers' valid records are mirrored (§1.12), only the own directory is repaired | `e2e::tests::own_records_are_listed_in_the_published_authority_index`, `a_peer_mirrors_valid_records_and_only_repairs_its_own_copy` |
| B3a new variants | `PackageTarget::{Remove, Held}`, `Explanation::{RemovedByGroup, WaitingForGroupHistory}`, `PackageStatus::{Removing, Held}` with explicit arms (table below) | compiles everywhere; covered by the B3b tests |
| B3b compose + constraints | `target::compose` screens candidates by `rejected`/`excluded` (§4.1 level 2) and applies the per-folder constraint (level 6, §4.4); removal and local-hold overlays. The service passes `RemovalBehaviour` and `decisions::Constraints` | core: `target::contract_tests::a_live_keep_pins_the_folder_against_a_newer_candidate`, `a_keep_names_the_tree_that_installs_even_when_older`, `constraints_naming_different_trees_need_a_decision`, `a_rejected_release_is_never_a_candidate`, `removal_and_local_hold_overlay_the_target`, `neutral_hooks_leave_the_slice_target_unchanged` |
| B3b service e2e | Through `SliceService::sync_once` with a shared provider folder | `e2e::tests::a_removal_published_by_a_peer_is_applied_and_an_offline_peer_cannot_resurrect_it`, `a_computer_that_lost_a_folder_waits_for_the_group_instead_of_reinstalling`, `a_live_keep_pins_a_folder_against_a_newer_candidate`, `a_rejected_release_is_never_installed`, `apply_report_survives_crash_before_publish` (regression, see below) |

Orchestrator-approved compose parameter: `target::compose(input, ledger, removal, held,
constraints)` gained `constraints: &dyn ConstraintHooks` as its fifth parameter. `compose` is
not a frozen §9.4 hook trait. `Neutral` keeps the previous behaviour; it is used only in the
test that asserts the slice target is unchanged.

Removal steps run in removal.md §1.6 order in `removal_steps` (observe, removal records,
step 3b acknowledgements, step 3a plan, then `plan_remove_ops` appended in `sync_once`).
`DecisionReason::{ChoicesDiffer, RollbackTargetsDiffer}` and `Explanation::{CheckingAgainstBlocked,
OnlyBlockedVersions, ConcurrentDecisions, ChosenByGroupRecord}` were added for compose.

Regression fixed while wiring: after an interrupted apply was recovered in the same cycle, the
pre-recovery scan seeded the removal baseline with a folder the recovery then rolled back,
which held the reinstall. The cycle now judges nothing for one cycle when recovery ran
(`service.rs`: journaled set widened). `apply_report_survives_crash_before_publish` caught it.

Golden fixtures: unchanged (new code-state fields are skipped when empty/None).

Out-of-list edit: `crates/wow-sync-core/src/testkit.rs` gained two struct-literal fields
(`authority_index: None, features: Vec::new()`). Compile-required, no behaviour change.
`explain.rs`, `decision.rs` (From arms) and `decisions.rs` (`Constraints` hooks) are in the
widened file set.

## A-38 controls (wiring)

Each mutation was applied with the edit tool to committed code, the named test failed
(exit 101), the code was reverted, and `git diff --exit-code` was clean.

| Control | Mutation | Failing test |
|---|---|---|
| Removal not applied through the cycle | `service.rs` `sync_once`: `if !extra.is_empty()` → `if false && ...` (the step-3a operations are never appended to the plan) | `e2e::tests::a_removal_published_by_a_peer_is_applied_and_an_offline_peer_cannot_resurrect_it` |
| Non-member record admitted | `service.rs` `observe_world`: every issuer found in the ledger is added to the membership | `e2e::tests::records_from_non_members_and_forged_copies_are_not_admitted` |
| Retired computer publishing | `service.rs` `publish_step`: `if w.standing != Active` → `if false && ...` | `e2e::tests::a_retired_computer_publishes_and_writes_nothing` |

Not re-run for this table: the 12 unrun settings controls (R4-core-settings owns them) and the
stage A ledger-rule mutation.

## Handovers to R4-app (new variants and their existing display)

| Where | Mapping |
|---|---|
| core `status.rs` | `Remove` → `PackageStatus::Removing`; `Held` → `PackageStatus::Held` |
| core `view.rs` | `Removing` → next `ApplyOnNextSync`; `Held` → next `Nothing` |
| proto and app `model.rs` `status_label` | `Removing` → "Removing" (Working tone, `Icon::Sync`); `Held` → the existing "Waiting" label (Waiting tone, hourglass) |
| proto and app `counts` | both count as `waiting` |
| proto `detail` text | sentences for `Removing` and `Held` |
| app `world.rs` `kind_of` | `Removing` → `Kind::Available`; `Held` → `Kind::Blocked` |
| `detail.rs`, proto `scenario.rs` | no edit needed |
| proto and app `model.rs` `decision_title`/`decision_reason` | arms for `ChoicesDiffer` and `RollbackTargetsDiffer` (NeedsDecision with `candidates: []`; the detail view renders an empty candidate list) |

No new UI was added. Commands go through `SliceService::command` (`control.rs`). The orchestrator
accepted the core `decision.rs` match-arm change and the `e2e.rs` service tests.

### Command handover

| Command | Parameters | UI meaning | Errors |
|---|---|---|---|
| `Decide` | item, choice, freshness | answer an offered decision; stale → `Changed` | `Decide`, `NotAllowed` |
| `Retire` / `ReEnroll` | computer / none | retire; explicit re-enrollment of a retired computer (`previous_enrollments`) | `NotAllowed` |
| `AllowAgain` | rejection | allow a rejected release again | `UnknownRejection` |
| `Keep` | scope, copy, why | pin a copy for a typed scope (Q-50 exclusions inherited) | `UnknownCopy`, `EmptySelection`, `Write` |
| `Resume` | scope, allow_again, why | release a keep | `Write` |
| `RestoreRemoved` | removal | bring back code and settings of a removed addon | `UnknownRemoval` |
| `SetSettingsPolicy` | master, exceptions | record master and per-addon exceptions; any number of rules (over the inline bound they go into parts, written before the policy; no `TooManyRules` any more) | `Settings(NotLive/BadRule)` |
| `ShareSettings` | unit, checkpoint | explicit push of a stored checkpoint | `UnknownCheckpoint`, `Settings(NoPolicy/NotShared/HistoryIncomplete)` |
| `PauseSettings` | folder | "Pause settings for this addon": its unit applies and captures nothing until resumed | `UnknownFolder`; `NothingToDo` if already paused; `Recorded` |
| `ResumeSettings` | folder, `ResumeOption` | resume with the chosen option. `UseShared`/`GoBackToShared`/`UseSharedKeepMineAsRecovery` unpause and queue use-shared; `Resume`/`KeepMineHere` unpause; `StayPaused` → `NothingToDo`; `ShareMine`/`ShareMineInstead` are refused (a push is explicit) | `UnknownFolder`, `UseShareSettings`; `NothingToDo`; `Recorded` |
| `UseSharedSettings` | unit | "Use the shared settings here": replaces a local difference at the next cycle, keeping mine as a recovery copy | `NoSharedSettings` (no single head); `Recorded` |
| `ChooseSettings` | unit, checkpoint, shown_heads | resolve competing settings: one `Choice` revision naming every head; `shown_heads` must equal the current heads | `Settings(Changed)` (show the choice again), `Settings(NotAHead)`, `Settings(NotCompeting)`; `Written` |
| `RestoreSettings` | unit, checkpoint | "Restore these settings": a `Restore` revision of a shared copy (uploads it) | `UnknownCheckpoint`, `Settings(NoPolicy/NotShared/HistoryIncomplete)`; `Written` |
| `RestoreRemovedSettings` | removal, unit | undo a removal for one unit's settings: a `Restore` revision of the checkpoint the removal captured | `UnknownRemoval`, `Settings(NotRemovedUnit)`; `Written` |
| `EnableSettings` (= "Confirm settings here", settings.md §10.1) | included_accounts | consent; no account included is refused | `EmptySelection`; `Recorded` |
| query `settings_history(unit)` | unit | graph nodes, oldest first | none |
| query `offered_decisions()` | none | decision items from live state | none |

Intent is recorded only for the policy/push commands; settings apply is wired in C0 (below).

### C0 settings apply and rollback (this turn)

| Command | Parameters | UI meaning | Errors / outcome |
|---|---|---|---|
| `EnableSettings` | included_accounts | consent to settings sync on this computer; local only | `Recorded` |
| `Rollback` | package, recovered (package hash of a held copy) | "Go back to this release": queued, applied when the game is closed; the rejection is written with the install | `UnknownCopy` (release not held), `Rollback(SameRelease/NotInstalled/NotAvailable)`; `Recorded` |

Settings step (`settings_cycle.rs`): applies nothing in production (cap `None`, Q-77); consent,
policy heads and an unblocked unit are required. Simplifications: no session-end or settle
window (Q-71 unbuilt); a unit file first seen after authority counts as an activation capture;
`Explanation::Applied{"<Folder> settings"}` is reused; capture history limited to 8. A fix in
`target_case` handling: `FirstAdoption(c_h)` with live already equal to `c_h` now settles the
basis, otherwise `Yield` is never reached.

Rollback (`rollback_flow.rs`, `service.rs`): `command` dry-runs `plan_requested` against the live
scan (R1 = inventory of the package's folders; R0 = the tree records of the chosen copy read
from the shared folder) and queues `rollback-request-<group>.json`. `sync_once` rebuilds the
input when Active and the game is not running, reserves the record sequence, puts the rollback
`PackageApply` first and drops colliding packages. The request is consumed either way (a
refused one is reported in `apply_note`). R0 is staged from the provider or, when its blobs
are gone, from a local recovery point (`NotAvailable` only if neither has it). Settings of a rollback are
`NotApplicable` (code only); `actual_settings_result` stays unwired.

### A-38 (item 5)

| Control | Mutation | Failing test |
|---|---|---|
| Non-master push accepted without explicit intent | `build_push` emits `RevisionKind::Auto` | `e2e::tests::settings_policy_and_push_commands_record_explicit_intent_only` (failed; reverted, `git diff --exit-code` clean) |
| Settings yield dropped | `target_case` pattern `FirstAdoption(to) \| Yield{..}` → `FirstAdoption(to)` | `e2e::tests::a_non_master_routine_write_yields_to_the_master_and_keeps_a_recovery_copy` (failed; reverted, clean) |
| Unrouted rollback | rollback plan not inserted into the apply plan (`rollback.filter(\|_\| false)`) | `e2e::tests::a_rollback_installs_the_recovered_release_and_an_offline_peer_cannot_bring_the_rejected_one_back` (failed at the v1 assertion; reverted, clean) |
| R0 staged from local recovery | fallback returns `Err(NotFound)` | `e2e::tests::a_rollback_stages_the_recovered_release_from_a_local_recovery_point` (failed; reverted, clean) |
| Pending removal not replayed | `observe` returns only newly matured removals | `e2e::tests::a_removal_whose_record_could_not_be_written_is_replayed_on_the_next_cycle` (failed; reverted, clean) |
| Settings pause ignored | `paused = ... && false` in `settings_cycle.rs` | `e2e::tests::pausing_settings_holds_a_units_updates_until_resume_and_resume_never_publishes` (failed; reverted, clean) |
| Choose skips the freshness check | `shown != now_heads && false` in `build_choice` | `e2e::tests::competing_settings_choices_are_resolved_by_one_choice_that_names_every_head` (failed at "a head the owner was not shown"; reverted, clean) |
| Policy parts not written | `write_own_bytes` loop filtered to nothing in `service.rs` | `e2e::tests::a_policy_with_thousands_of_exceptions_is_written_in_parts_and_read_back_whole` (failed; reverted, clean) |

## Open items

- **Policy parts (§1.9 writer).** `kinds.rs` was not changed (`pack_items` already existed).
  `settings::build_policy` now returns `(policy, parts)` and writes no `TooManyRules`; the
  service writes the parts first. When the policy is kept (held/paused) the parts are written at
  once anyway: they are issuer-less and inert until the policy is written. Reader proof: the
  policy stays non-Valid while a part is missing and becomes Valid when it arrives. The same
  split is not applied to other partable fields (`known`, `parents`, `considered`); only the
  settings-policy writer was in scope.
- **Crash between observe-save and the removal-record write (FIXED, `afa7f59`).**
  `RemovalStore::observe` now persists matured removals in `removal-pending-<group>.json`
  before consuming the observation and returns them every pass until `clear_pending` (called
  after the record write). A pending removal whose folder is installed again is dropped.
  Tests: `a_removal_whose_record_could_not_be_written_is_replayed_on_the_next_cycle`,
  `a_pending_removal_is_dropped_when_the_folder_is_installed_again`. A-38: returning only newly
  matured removals (no replay) made the first test fail (`e2e.rs:1317`); reverted, clean.
  Residual: a crash between the record write and `clear_pending` may write a duplicate removal.
- **R0 local source (DONE, `fb6c8b4`).** rollback.md §1 step 4 allows R0 from a local recovery
  point; `effects.rs` falls back to `.wowsync/recovery` when the provider blob is missing.
  Test `a_rollback_stages_the_recovered_release_from_a_local_recovery_point` (R0 gone from the
  shared folder → A succeeds; B without a recovery point gets `NotAvailable`, nothing changes).
  A-38: fallback forced to `Err(NotFound)` failed the test; reverted, clean.
- **Peers' index chunks** (§1.12 step 1, pending-unreadable refs) are not read; directory
  listing plus verbatim mirrors cover it.
- **Level 5 (stale copy)** is not in `compose`; the `PartialCandidate` nuance for a no-tree
  constraint is not raised.
- **Package-level `Remove`:** any removed member folder makes the whole package `Remove` for
  status. Actual removal is per folder through step 3a.
- **A rejected or kept `NeedsDecision` for differing trees** carries `candidates: []`.

## DEFERRED (not built; each needs the named change)

- **Not modelled:** "Pause all settings" (per-folder only); a capture command producing a
  checkpoint (`ShareSettings`/`RestoreSettings` take a checkpoint already in the local store);
  a removal that captures settings (the core `removal_record` hard-codes `RemovalSettings::None`,
  so only a hand-crafted record carries roots; the settings return as a `Restore` revision, and
  the file-level return follows the usual §4.2 target).
- **`ShareSettings` does not upload its checkpoint** (Choose, Restore and removal-restore do);
  a peer cannot fetch a plain push's blob until something uploads it.
- **Cross-packet items** and stage D A-38 controls not listed above.

## Pass 11: snapshot data, notify/update/diagnostics wiring, handover paths

Commits: item 2 `10413c5` (runtime `StatusSnapshot` data), item 3 `ac9cf40` (wiring).

### Snapshot (`wow_sync_runtime::StatusSnapshot`, from `SliceService::status()`)
New fields: `hold`, `this_retirement`, `retirements: Vec<RetirementRow>`,
`held_releases: Vec<HeldRelease>`, `removed: Vec<RemovedRow>`, `enrollment_names`,
`review_needed: bool` (ops.md §1.7 precedence; never hides needs-help/error), `update:
Option<UpdateResult>`. By orchestrator decision the core `Headline`/`StatusView`/`MemberStatus`
are unchanged; R4-app derives the "review needed" headline from `review_needed`.

### Notify/update/diagnostics wiring (item 3)
- **Orchestrator-visible deviation:** the three toggles are NOT on `AppSettings`. Adding fields
  broke the struct literal at `crates\wow-sync-app\src\app.rs:686` (not my file). They live in
  `wow_sync_runtime::AppToggles { update_check, notify_summaries, notify_needs_me }` (all default
  on, own file `toggles.txt`, so saving the window settings never resets them). Access:
  `SliceService::{toggles, save_toggles}`. If you want them on `AppSettings`, R4-app must change
  app.rs:686 first.
- New commands: `Command::CheckUpdates` -> `CommandOutcome::UpdateCheck(Started)`;
  `Command::ExportDiagnostics{options: ExportOptions}` -> `CommandOutcome::Exported(Export)`
  (preview text); `Command::SaveDiagnostics{options, dest: PathBuf}` -> `Exported(Export)`, refused
  with `CommandError::Export(ExportError)` when `dest` is under the shared folder or install
  roots (also logged as an `export-refused` activity line). `CommandError::UpdatesUnavailable`
  when no update service is attached.
- Service hooks: `attach_os_services()` at startup (Windows: AUMID registration, `OsNotifier`,
  `GithubReleases`); `attach_notifier`, `attach_updates` for fakes; `NoticeRunner::after_cycle`
  runs at each cycle end (ActionNeeded from decisions and retirement, summaries from applied
  changes, AppUpdate once per tag).
- **Fix in R4-notify-diag `notices.rs`:** `after_cycle` returned early on an unchanged ledger, so
  summary-only notices were never delivered; it now returns early only when nothing changed and
  nothing is to notify.
- Tests (offline): the three e2e tests plus `control::tests::toggles_default_on_persist_apart_from_the_window_settings`.

### A-38 (pass 11)
| Item | Mutation | Failing test |
|---|---|---|
| 2 | `retired: false` in `HoldInputs` | `the_snapshot_reports_retirements_and_the_retired_computers_hold` |
| 3 | empty `forbidden` list for SaveDiagnostics | `diagnostics_preview_and_save_match_and_never_land_in_the_shared_or_install_folders` |

Both reverted; `git diff --exit-code` clean.

### Handover paths (R4-app)
`wow_sync_runtime::{Command, CommandOutcome, CommandError, RollbackRefusal, SettingsHistoryEntry,
StatusSnapshot, HeldRelease, RemovedRow, RetirementRow, AppSettings, AppToggles, ExportError,
SaveError, Started, UpdateService, UpdateView, NoticeRunner, CycleOutcome, Clock, system_clock}`;
core types via `wow_sync_runtime::core::{status::{HoldScope, RetiredInfo}, update::UpdateResult,
diagnostics::ExportOptions}`.

### New open items
No DataAtRisk notice producer; diagnostics `journals` empty, `process_memory_mib`/`os_version`
None, dropbox root empty; peers' index chunks not read; `held_releases` lists every non-installed
complete copy.

## Verification

Final tree: `cargo xtask verify --tier offline` PASS; `cargo xtask verify --tier native` PASS
(Windows, this machine).
