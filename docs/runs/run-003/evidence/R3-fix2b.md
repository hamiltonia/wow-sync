# R3-fix2b evidence

Branch `run003/R3-fix2b` (from main 025a1cd). Finding RC4-1 from
[R3-check4](R3-check4.md). Links: R-12, A-25, A-38.

## RC4-1 — recovery must not record a game cause (DONE)

Cause: `recover_one`, `rollback`, `fail_restore` and `verify_and_finish` called
`record_game_halt`, so a game blocking a *recovery retry* marked a cause-less crash
journal `halted_by_game`.

Change (`core/src/exec.rs`): only `run_apply` records the cause — the commit-loop
gates, and the verify gate when `verify_and_finish` was called from the apply
itself. Recovery paths just return `Interrupted` and leave the journal untouched.
So a crash journal stays "unknown" (neutral wording); the next step is still
"Close WoW" while the game runs (existing `next_action` rule). A journal that already
records a true cause keeps it.

Tests:
- New `exec::tests::recovery_blocked_by_a_running_game_does_not_record_a_cause`
  (crash journals in Committing and Committed; recovery with the game running →
  `Interrupted`, `halted_by_game` still false, `game_halted` empty). PASS.
- Existing genuine-launch tests still PASS:
  `a_game_start_halt_is_recorded_and_status_blames_the_game_from_it`,
  `no_live_mutation_after_launch_mid_commit`, `verify_with_game_running_waits`.

A-38 negative control: re-added `record_game_halt` in `recover_one`'s Committing
gate → `cargo test -p wow-sync-core` exit 101, FAILED
`exec::tests::recovery_blocked_by_a_running_game_does_not_record_a_cause`.
Restored afterwards.

## Results (Windows 11 x64, Rust 1.100.0)

- `cargo test -p wow-sync-core` PASS (182 tests).
- `cargo xtask verify --tier offline` PASS, exit 0.
- `cargo xtask verify --tier native` PASS, exit 0.

## Limits

No real-app GUI re-check was run; no files outside `core/src/{exec.rs,exec_tests.rs}`,
`tests/required-cases.toml` and this evidence changed.
