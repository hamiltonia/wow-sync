# Evidence: R4-core-removal (run-004, phase C step 2)

Branch `run004/R4-core-removal`, base main b8707ae. Synthetic data only.

## Lost-work incident

While running an A-38 mutation control I reverted a mutation with
`git checkout -- removal.rs` while the implementation was still uncommitted.
That wiped the whole core `removal.rs`. The owner approved re-implementing it
under new rules: commit as soon as a file compiles, mutate only committed code,
apply and revert with the edit tool, confirm with `git diff --exit-code`, and
rerun both verify tiers after the final change (earlier results are void).

Recovery commits: `e0234aa` (tests, runtime cycle, detail rows, toml section
preserved), `39c49a5` (core `removal.rs` re-implemented). The final commit
adds this file.

Result seen before the reset: the no-resurrection mutation
(`candidate_is_stale` forced to false) failed
`an_offline_peer_that_still_owes_cannot_resurrect_a_removed_package`, exit 101.
Rerun on committed code: see mutation 1 below, same result.

## Implemented

- Core `removal.rs`: merge (`Live = Rem - Bnd`, re-add coverage, connected
  removal units), obligations (owes, eligibility, stale candidates, pending
  gate, reinstall hold), ack/plan (`due_items`, absent acks in batches of
  256, `plan_remove_ops`), explicit actions (`known_summary`,
  `readd_for_restore`, `dependency_removed`), observation (`observe`:
  E1, E4, E5, E6, folder drop; E2/E3/E8 approximated) and `RemovalBehaviour`
  behind the frozen `RemovalHooks`.
- Runtime `removal_cycle.rs`: `RemovalStore`, `step_3a` (blocked while the
  game runs), `step_3b`, and the commands (remove, confirm/keep unclear,
  restore, add back, continue without).
- Runtime `detail.rs`: `removed_rows`.

## Tests (offline tier)

22 core (`removal_tests`), 3 runtime (`removal_cycle::tests`), 1 detail
(`removed_tests`) = 26 tests. Toml cases `R4-core-removal:merge`,
`:stale-and-owed`, `:ack-and-plan`, `:observation`, `:commands-and-rows`.

## Mutation controls (committed code, edit-tool apply/revert)

| # | Mutation | Failing tests | Exit | Restored |
|---|---|---|---|---|
| 1 | `candidate_is_stale` always false (no resurrection) | `an_offline_peer_that_still_owes_cannot_resurrect_a_removed_package` | 101 | diff exit 0 |
| 2 | `re_covers` always true (removal wins) | 3 tests (`removal_wins_over_...`, `conflicting_removals_merge_...`, `restore_readd_covers_...`) | 101 | diff exit 0 |
| 3 | `re_covers` always false (explicit add-back) | 4 tests (`restore_readd_covers_...`, `a_readd_discharges_...`, `conflicting_removals_merge_...`, `a_removal_alone_is_live_...`) | 101 | diff exit 0 |
| 4 | units never merge | `conflicting_removals_merge_into_one_unit_and_both_stay_live` | 101 | diff exit 0 |
| 5 | E4 gap check always true | 4 observation tests | 101 | diff exit 0 |
| 6 | waiting items also absent-acked | `due_items_include_the_origin_and_split_by_cleanup_class` | 101 | diff exit 0 |

## Verification

On the final tree (core/runtime source identical to 39c49a5/e0234aa):
`cargo xtask verify --tier offline` PASS (fmt, clippy, test, required-cases,
deny, markdown-links); `cargo xtask verify --tier native` PASS.

## Deferred / not done

- Cycle skeleton calling steps 3a/3b/6 and building `RemovalBehaviour`:
  `crates/wow-sync-runtime/src/service.rs` (not owned). Needs a skeleton that
  calls `removal_cycle::step_3a`/`step_3b` with the game-running gate and
  persists via `RemovalStore`. `removal_cycle.rs` carries `allow(dead_code)`
  until then.
- `PackageTarget::{Remove, Held}`, removal `Explanation` and `PackageStatus`
  variants: `crates/wow-sync-core/src/target.rs`, `explain.rs`, `status.rs`
  (step-1, not owned). `removal.rs` uses its own `HoldReason` and a placeholder
  `Explanation::Installed`.
- Settings-cleanup op planning (`OpKind::SettingsCleanup`): needs live settings
  file state; belongs with `settings.rs` (R4-core-settings). Removal classes
  `WaitForCleanup` and omits the ack when a captured root exists.
- `T-R4D21-readd-over-rollback`: rejection-constraint lifting lives in
  `reject.rs`/`ConstraintHooks` (R4-core-settings). Not tested here.
- Approximations: E2/E3/E8 in `observe`, E6 snapshot replaced by
  `known_unusable`.
- GUI items (T32, keyboard flows, wording, Removed section commands): R4-app.
- T33 provider tier: BLOCKED. Native T-C11: NOT RUN.
