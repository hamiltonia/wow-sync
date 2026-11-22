# R6-gate evidence

Worktree `D:\GitHub\wow-sync.worktrees\R6-gate`, branch `run006/R6-gate`, merged with main
after the `url` 2.5.9 fix (`d4fad8f`). Environment: Windows dev PC, synthetic temp installs.

## Verification (actual runs, after the last code change)

| Tier | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, test, required-cases, deny (no exception needed), markdown-links |
| `cargo xtask verify --tier native` | PASS |

## Goals

1. **Gate (D-28).** `MarkerPolicy {NotRequired (default), Required}`; `validate_variant(obs, policy)`
   checks the marker only under `Required`. Tests: `unmarked_install_is_accepted_by_default`,
   `required_policy_refuses_missing_wrong_or_bom_prefixed_markers` (a valid BOM-prefixed marker is
   still accepted, as the existing parser did), e2e `install_without_marker_enrolls_and_syncs_by_default`.
2. **`wowf` toc profile.** Tiers `Forever`, `Camelot`, family `Mainline`, plain; `mapping_id`
   `profile:wowf/2`; unfamiliar clients keep `validated-plain/1`. Table test of suffix mixes designed
   from the dev-PC survey (folder names and toc header lines only; nothing copied). An enrolled
   `wowf` with the stored `validated-plain/1` gets `StaleConfirmation` → `NeedsReconfirmation`,
   reconfirms, and loses nothing (core and e2e tests). `declaring_folders` now uses the variant's
   mapping instead of the hard-coded plain rule.
3. **First run loses nothing.** e2e tests: populated join (older version replaced with a GUI
   recovery point and restored byte for byte through `Command::Rollback`; local-only add-on kept and
   published; same label with different content untouched on both sides), populated create (no
   local change, no recovery points), join with a group-removed add-on the joiner already has
   (kept untouched, removal still listed).
4. **Test mode API.** See below.

## Test-mode API (for R6-app)

- `Command::SetTestMode(bool)` → `CommandOutcome::Recorded`. Works before enrollment and while blocked.
- `StatusSnapshot.test_mode: bool` (set in every status path).
- `SliceService::test_mode()`; `control::test_mode(&LocalState)` / `control::set_test_mode`.
- Persisted in the local state file `test-mode.txt` (missing = off; unreadable = on).
- On: only marker-carrying installs validate. An enrolled unmarked install stops with
  `ServiceError::TestModeBlocked` and the headline NeedsHelp (`provider_error` carries the text);
  nothing is un-enrolled or changed. Off resumes.

## Refusal and layout reasons visible to R6-app

- `RefuseReason`: `FlavorMissing`, `FlavorUnreadable`, `FlavorUnparseable`, `InvalidProductCode`,
  `BuildInfo`, `ProductNotInBuildInfo`, `ProductAmbiguousInBuildInfo`, `ProductInactive`,
  `FolderMismatch`, `AddOnsMissing`, `NotSyntheticInstall` (test mode only), `Layout(..)`,
  `ConfirmationRequired`, `StaleConfirmation`, `VariantChanged`.
- `LayoutReason`: `AddOnsLayout`, `DuplicateProductFolder`, `AlreadyEnrolled`, `NotSyntheticInstall`.
- `BlockReason` (codes `name`, `case`, `unreadable`, `unvalidated`, `notoc`, `tocerror`, `ambiguous`):
  `UnsupportedName`, `CaseCollision`, `NotReadable`, `UnvalidatedTocMapping` (unfamiliar clients
  only), `NoToc` (for `wowf`: no Forever/Camelot/Mainline/plain toc), `TocError`, `TocAmbiguous`.
- `ServiceError`: `NotEnrolled`, `AlreadyEnrolled`, `Refused`, `NeedsReconfirmation`,
  `VariantChanged`, `TestModeBlocked`, `Provider`, `Local`.
- The preview refusal text still uses `{r:?}` (service.rs); R6-app owns humanizing it.

## A-38 controls (each: mutate in one call, run test and see it fail, revert, `git diff --exit-code` = 0)

| Goal | Mutation | Failing test |
|---|---|---|
| 1 | `variant.rs` marker check made unconditional | `unmarked_install_is_accepted_by_default` |
| 2 | `family_suffixes: &[]` | `forever_profile_suffix_mixes_observed_on_the_dev_pc` |
| 3 | `package.rs` same-label-different-content treated as newer | `first_run_join_of_a_populated_install_loses_nothing` (`eps-a` ≠ `eps-b`) |
| 4 | `control::test_mode` always false for an existing file | `test_mode_persists_and_blocks_unmarked_installs_both_ways` |

A first attempt at control 3 had its edit and revert-check in one batch and produced a syntax-broken
mutation; it was reverted and redone in separate calls. No commit was involved.

## Findings

- A peer still publishing `validated-plain/1` against a local `profile:wowf/2` is a
  `DifferentTocMapping` holder until it upgrades and republishes; nothing is lost meanwhile.
- A joiner's pre-existing copy of an add-on the group already removed is kept (not deleted);
  the removal stays listed. No first-cycle delete without a recovery point was found.
- Core tests that asserted old behavior were updated: `inventory` (suffixed `wowf` folder is now
  `Selected`) and `target::foreign_suffix_only_candidate_not_installed` (now `NoCompatibleToc`).
- Files outside the packet touched: `crates/wow-sync-runtime/tests/public_api.rs` (exhaustive
  command match, count 22), `tests/required-cases.toml` (renamed the removed tests in their old
  cases), `docs/design/tests.md`.
- Optional owner client check of the Forever toc order remains unrun (Q-3); source is the
  warcraft.wiki.gg "TOC format" page (read 2026-11-22).
- Blocked: nothing.
