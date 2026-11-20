# R4-integrate evidence (stages A and B done; C and D DEFERRED)

Branch `run004/R4-integrate`. Status: **partial**. Stage A, the code-state `authority_index`,
and stage B (authority plane, compose with constraints, removal through the cycle) are built
and tested. Stages C and D are DEFERRED below; nothing here claims they work.

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

No new UI was added. There are still no control commands for the app to drive (stage C).

## Open items

- **Crash between observe-save and the removal-record write loses the removal.**
  `RemovalStore::observe` saves the matured observation, then the record is written. A crash
  between the two removes the observation without a record. Proposed fix: persist the pending
  `ObservedRemoval`s in local state before writing, replay them at the next cycle, and clear
  them once the record write succeeds. Owner: `removal_cycle.rs`/`service.rs`.
- **Peers' index chunks** (§1.12 step 1, pending-unreadable refs) are not read; directory
  listing plus verbatim mirrors cover it.
- **Level 5 (stale copy)** is not in `compose`; the `PartialCandidate` nuance for a no-tree
  constraint is not raised.
- **Package-level `Remove`:** any removed member folder makes the whole package `Remove` for
  status. Actual removal is per folder through step 3a.
- **A rejected or kept `NeedsDecision` for differing trees** carries `candidates: []`.

## DEFERRED (not built; each needs the named change)

- **Settings `PackageApply`s** (`service.rs`): `plan_settings` is not called;
  `CycleCx.roots` is `&[]`. Needs the settings roots from the live scan and the settings
  epoch/revision ledger view, then append after `plan_apply` handling `NothingToDo`.
- **§9.1 control surface** (`control.rs`, stage C): every command/query variant routed to
  `decisions`, `removal_cycle`, `settings_cycle`, `rollback_flow` (incl. `SettingsHistory`,
  rollback, allow-again).
- **Rollback inputs from live scans, cross-packet items, `TooLarge` split persistence**
  (probably `kinds.rs`), the stage D A-38 controls.

## Verification

Final tree: `cargo xtask verify --tier offline` PASS; `cargo xtask verify --tier native` PASS
(Windows, this machine).
