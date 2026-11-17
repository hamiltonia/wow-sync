# R3-fix2 (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first. The findings to fix are in
`docs/runs/run-003/evidence/R3-check3.md`: RC3-1, RC3-2 and RC3-3.

**Owned paths:**
- `crates/wow-sync-core/src/{status.rs,view.rs}` and their tests.
- If a cause has to be recorded: `crates/wow-sync-core/src/journal*`, plus
  the matching adapter and runtime code.
- `crates/wow-sync-app/src/{world.rs,model.rs,pages.rs}`.
- `tests/required-cases.toml` (entries for cases you add).
- `docs/design/core-api.md` (the status and next-action table only).
- `docs/runs/run-003/evidence/R3-fix2.md` and `evidence/R3-fix2/**`.

File writes go through edit/create or `cargo fmt` only (incident I-1). Add
no new dependencies. Use only synthetic data. Stop processes only by the
PID you captured at launch.

## Goals

1. **RC3-1 (medium).** The interruption cause and the next step must be
   true.
   - Today `derive_status` picks `InterruptedByGame` from *current*
     `game_running`. Use `InterruptedByGame` only when there is recorded
     evidence that the game start caused the interruption. That evidence
     can be a cause the journal already records, or one you add when the
     executor stops for a game start.
   - Otherwise use neutral `Interrupted` wording.
   - `next_action` for an interruption:
     - if the game is running now, "Close WoW to let the update finish";
     - if not, no "close WoW" step; say it will finish or restore on the
       next sync, using core's existing sentence style.
   - Any journal format change must keep reading old journals; treat a
     missing cause as unknown.
   - Tests:
     - crash-interrupted with the game started later gives neutral wording
       plus "close WoW";
     - crash-interrupted with no game gives neutral wording and no
       "close WoW";
     - a recorded game-start interruption gives `InterruptedByGame`.
   - A-38 negative control: revert to the current-`game_running` rule and
     show a test fails, then restore.
2. **RC3-2 (low).** Show a missing candidate version as "not available",
   never as an empty string, in both the decision sentence and the card.
   Add a test.
3. **RC3-3 (low).** The paused hint must not promise "0 updates" while an
   interrupted or waiting update exists. Count it correctly, or leave the
   number out when it's unknown. Add a test.

## Done

- `cargo xtask verify --tier offline` and `--tier native` both pass.
- The evidence lists each finding with the change, the tests, the negative
  control and the command results.
- Commit on your branch. Do not push.
