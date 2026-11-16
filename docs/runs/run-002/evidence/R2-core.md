# R2-core evidence

Worker: R2-core (claude-sonnet-5.5). Checkout `D:\GitHub\wow-sync.worktrees\R2-core`, branch `run002/R2-core`.
Environment: Windows, cargo 1.100.0, no network use beyond crates.io fetch (Cargo.lock updated).

## Result

`cargo xtask verify --tier offline` from the worktree root: **PASS** (fmt, clippy `-D warnings`, test,
required-cases, deny, markdown-links). `cargo test -p wow-sync-core`: 152 passed, 0 failed, 0 ignored.

Two cargo warnings are reported (`unused workspace dependency getrandom / windows-sys`). They come from the
root `Cargo.toml` written exactly as crates.md §3; the adapters crate (R2-adapters) is their consumer. They
do not fail the verify tier.

## Required cases (tests.md §3), all PASS

Every test name listed in tests.md §3 exists as an in-module unit test, is registered in
`tests/required-cases.toml` (block at the top, tier `offline`) and passes:
`R-02:core`, `A-46:core`, `A-08:core`, `toc:core`, `A-23:core`, `A-09:core`, `hash:core`, `records:core`,
`A-20:core-admit`, `A-07:core`, `A-04:core`, `A-21:core`, `A-22:core`, `A-20:core`, `A-24:core`, `A-43:core`.

Added beyond the table:

- F-01: the synthetic marker is enforced in `validate_variant` (`variant::tests::install_without_synthetic_marker_refused`).
- F-02: rollback re-verifies restored live content before the terminal record
  (`exec::tests::f02_crash_after_restoring_corrupt_copy_is_not_rolled_back`, registered under `A-22:core`;
  `restore_mismatch_needs_help` covers the direct case).

The exec crash sweeps (`crash_at_every_effect_recovers_to_old_or_new`,
`crash_during_recovery_at_every_effect_is_reentrant`) run every crash point of a two-package apply and every
crash point of the following recovery (also with a `Running` activity between), and check old-or-new live trees,
no leftover journal or staging, no pinned point, and a `NotRunning` check before every live rename.

## Design gaps (safer reading taken; design docs not edited)

- `s_addon.rs` table was not written; `version::tests::s_addon_labels_classify_as_expected` is self-contained.
- F-01 reason is `RefuseReason::NotSyntheticInstall` from `validate_variant`; `LayoutReason::{AddOnsLayout, NotSyntheticInstall}` are unreachable from `assess_layout`.
- toc mapping is always `validated-plain/1`; profile selection is treated like plain; any suffixed toc gives `TocKind::Unvalidated` (variant added).
  Consequently an unvalidated local folder yields `Ambiguous` (not `UnvalidatedTocMapping`) for a package with a validated peer candidate; `UnvalidatedTocMapping` is used when no local folder exists. Tests assert this.
- `assess_layout` skips unstable folders; `confirm_variant` accepts Recognized+Supported without a confirmation.
- Added fields: `GroupTarget{group_id, effective, local_hash}`, `MemberObs.computer`, `PublishContext{product, app_version, now_unix_ms}`, `ApplyGate.now_unix_ms`, `ApplyPlan{group_id, started_unix_ms, skipped}`, `ObjectRef.source`; added `PlanBlock::{DestinationChanged, RenameFailed(String)}`.
- `run_publish` cannot persist `next_seq` (runtime duty); `plan_publish` blocks a seq not above existing own seqs. Own-state pruning keeps the newest 3 including the new one, current enrollment only.
- Status: `NeedsDecision`, `Available`, `Interrupted` and `Blocked` map to the Waiting headline; `confirmations()` expects `members` to exclude this computer.
- `plan_apply` skips a whole package when a present local folder is not Ok/fingerprinted; if all are skipped it returns `Err(Unstable)`.
- After a `Crashed` effect error, `run_apply` keeps iterating; the remaining calls fail harmlessly.
- A rename error during rollback gives `Failed(RestoreFailed)` and leaves the journal in place so it is retried.
- Needs-help journal `reason` uses `code:folder` strings.
- `exec` tests live in `exec_tests.rs` (module path `exec::tests`, as required).

## Not implemented

- `testkit/s_addon.rs` real-metadata table (see above).
- Nothing else from the R2-core packet is outstanding. No platform/native tests belong to this crate.
