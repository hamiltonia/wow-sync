# R4-integrate evidence (partial: stage A done, stage B part 1 done)

Branch `run004/R4-integrate`, based on main cb9931e. Status: **partial**. The wiring
stages that remain are DEFERRED below; nothing here claims they work.

## Done

| Item | Change | Tests (offline tier) |
|---|---|---|
| Ledger fix (stage A) | `ledger.rs` `kind_rules`: a `SettingsRevision` parent needs the same unit, any kind | `ledger::tests::a_settings_revision_parent_needs_the_same_unit_and_any_kind`, `graph_from_ledger_keeps_equal_heads_and_marks_an_unknown_parent_pending`, `policy_heads_drop_a_policy_that_a_later_one_knows` |
| Kept queue persistence (A) | `decisions.rs` `KeptQueue::{load,save}`; items that do not hash to their name are dropped | `decisions::tests::a_kept_queue_survives_a_restart_and_a_tampered_item_is_dropped` |
| Settings epoch/seq/kind persistence (A) | `settings_cycle.rs` `EpochStore`; damaged state loads as "nothing clean" | `settings_cycle::tests::the_epoch_state_survives_a_restart_and_unreadable_state_is_not_clean` |
| Code-state `authority_index` + `features` (B part 1) | `records.rs`, `plan.rs`: additive fields (contracts §1, §2); `publish_needed` republishes when either differs; the service publishes `FEATURES` and `authority_index: None` | `records::tests::authority_index_and_features_are_additive_and_old_readers_ignore_them` (slice bytes unchanged, round trip, old-shape reader ignores the fields), `plan::tests::publish_needed_decisions` |

Golden fixtures: unchanged (new fields are skipped when empty/None).

Out-of-list edit: `crates/wow-sync-core/src/testkit.rs` gained two struct-literal fields
(`authority_index: None, features: Vec::new()`) because it builds `CodeState` and
`PublishContext`. Compile-required, no behaviour change.

## DEFERRED (not built; each needs the named change)

- **Authority plane in the cycle** (`service.rs`): read every member's `authority\*.json`
  into `Ledger`/`Membership` and persist them; publish own index chunks via
  `ledger::append_index_refs` and set `PublishContext.authority_index`; mirror peers' records
  (contracts §1.12); call `RemovalStore::observe`; append `plan_remove_ops`/`plan_settings`
  `PackageApply`s after `plan_apply` (handle `NothingToDo`); removal steps 3a/3b/6 in
  removal.md §1.6 order. Needs `observe_world`/`sync_once` changes; a new module is outside the
  file set, so the code belongs in `decisions.rs` or `removal_cycle.rs`.
- **`target::compose` with `ConstraintHooks`** (`target.rs`): `compose` takes only
  `RemovalHooks`; add a `&dyn ConstraintHooks` parameter, plus `PackageTarget::{Remove, Held}`,
  removal `Explanation`s and `PackageStatus` variants with explicit arms in core
  `status.rs`/`view.rs`, runtime `detail.rs`, proto `model.rs`/`scenario.rs`, app
  `model.rs`/`world.rs`. Suggested honest display: Remove = "Removing", Held = "Waiting:
  group history incomplete" (existing `Waiting`/Hourglass label).
- **§9.1 control surface** (`control.rs`): all command/query variants routed to `decisions`,
  `removal_cycle`, `settings_cycle`, `rollback_flow` (incl. `SettingsHistory`, rollback,
  allow-again).
- **Rollback inputs from live scans**, `TooLarge` split persistence (probably `kinds.rs`),
  cross-packet items.
- **A-38 controls**: none run in this packet. The 12 unrun settings controls and the four
  wiring controls remain open; the stage A ledger fix has a test that fails on the old rule
  but its mutation was not re-run for the table.

## Handovers to R4-app

None yet: no new `PackageStatus`/`PackageTarget`/`Explanation` variants were added, so no
proto/app mapping changed. No control commands exist for the app to drive.

## Verification

`cargo xtask verify --tier offline`: PASS at the stage B part 1 commit.
