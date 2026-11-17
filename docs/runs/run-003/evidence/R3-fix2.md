# R3-fix2 evidence

Branch `run003/R3-fix2` (from main 5deeaac). Findings RC3-1..RC3-3 from
[R3-check3](R3-check3.md). Links: R-12, A-24, A-25, A-38.

## RC3-1 — interruption cause and next step (DONE)

Change:
- `ApplyJournal.halted_by_game` (new, `serde(default)`, omitted when false). Old
  journals decode as "cause unknown". The executor records it (best-effort journal
  write) only when its game gate stops an update after the journal exists (commit
  renames, verify, recovery, rollback). Any later `transition` clears it.
- `status::game_halted(journals)` collects journals that record it. `StatusInput`
  now takes `interrupted_by_game` instead of `game_running`; `derive_status` uses
  `InterruptedByGame` only for those, otherwise neutral `Interrupted`.
- `view::next_action(status, why, game_running)`: interrupted + game running now →
  `CloseGame` ("Close WoW to let the update finish."); otherwise new
  `FinishOnNextSync` ("Nothing to do. It will be finished or restored on the next sync.").

Tests (all PASS):
- `status::tests::interruption_blames_the_game_only_on_recorded_evidence`
- `status::tests::only_journals_that_record_the_game_halt_blame_the_game`
- `status::tests::an_old_journal_without_the_cause_field_still_loads_as_unknown`
- `exec::tests::a_game_start_halt_is_recorded_and_status_blames_the_game_from_it`
- `view::tests::an_interruption_asks_to_close_wow_only_while_the_game_runs`
- `model::tests::interrupted_text_follows_core_and_differs_by_cause` (updated)

Mapping to the packet's three cases: crash-interrupted + game started later →
unflagged journal → neutral text, game running → "Close WoW" (view test, running);
crash-interrupted, no game → neutral, no "close WoW" (view test, idle);
recorded game-start interruption → `InterruptedByGame` (exec/status tests).

A-38 negative control (mutations reverted afterwards): `game_halted` ignoring the
recorded flag (the old "any interruption is the game's" behaviour) and
`next_action` returning `CloseGame` for every interruption:
`cargo test -p wow-sync-core` → exit 101, FAILED
`status::tests::only_journals_that_record_the_game_halt_blame_the_game` and
`view::tests::an_interruption_asks_to_close_wow_only_while_the_game_runs`.

## RC3-2 — missing candidate version (DONE)

`model::version_label` returns "not available" for a missing/blank label; used in
the decision sentence, the per-candidate line and (`pages.rs`) the candidate card.
Test `model::tests::a_missing_candidate_version_reads_not_available_never_blank`.
Control: `version_label` always returning the raw label → exit 101, test FAILED.

## RC3-3 — paused hint (DONE)

`model::paused_hint` counts waiting (incl. interrupted) add-ons: "1 add-on waiting
will be tried again when you resume" (plus the ready count when both exist); the
"N updates will be applied" form is kept only when nothing waits.
Test `model::tests::the_paused_hint_never_promises_zero_while_an_update_waits`.
Control: ignoring `waiting` → exit 101, test FAILED.

## Commands and results (Windows 11 x64, Rust 1.100.0)

- `cargo test -p wow-sync-core` PASS (181 + new). App/runtime/proto tests PASS.
- Mutations above: exit 101 as stated; reverted (`git diff` shows none committed).
- `cargo xtask verify --tier offline` PASS (fmt, clippy, test, required-cases, deny,
  markdown-links), exit 0.
- `cargo xtask verify --tier native` PASS, exit 0 (after the mutations were reverted).

## Deviations / limits

- Minimal exceptions outside the packet's owned paths: `core/src/{exec.rs,
  exec_tests.rs,records.rs,report.rs,testkit.rs}` (journal field and executor
  recording, as the packet allows for a recorded cause), `runtime/src/{service.rs,
  detail.rs}` (pass the new inputs), `proto/src/scenario.rs` (one-line `StatusInput`
  adaptation, behaviour unchanged).
- Real-app GUI re-verification of the three screens was NOT RUN; the precise
  game-start-during-commit race on a real install is also NOT RUN (covered by the
  fake-effects executor test only).
