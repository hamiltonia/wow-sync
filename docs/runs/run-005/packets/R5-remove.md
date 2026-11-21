# R5-remove: uninstall removes and recovers settings (RV-04); Q-77 cap

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R5-remove`, branch `run005/R5-remove`.

## Why

SPEC/AGENTS: "Uninstall removes active code and settings; recovery must
preserve and restore both." R4-review RV-04 found the settings half unbuilt:
`core/src/removal.rs` builds removals with `RemovalSettings::None`, and no
planner emits `OpKind::SettingsCleanup`. D-26 sets Q-77 to 32 MiB.

## Goals

1. **Q-77.** `core/src/records.rs` `MAX_SETTINGS_FILE_BYTES` becomes
   `Some(33_554_432)` (32 MiB). Update tests that assert `None`, including
   `core/src/settings_tests.rs` and anything in runtime/app that depends on
   the cap being absent (for example a "missing cap" path). The app banner
   for a missing cap (`app/src/settings_ui.rs:373`) is R5-app's file: if it
   needs a change, report it, do not edit it. A file above the cap is a
   `TooLarge` diagnosis for that unit and is not synced; prove it with a
   test at cap and cap+1 bytes.
2. **RV-04.** Build the plan in
   [R4-integrate evidence](../../run-004/evidence/R4-integrate.md),
   "RV-04 plan", following removal.md §1.6 step 6, §6 and its contract
   table:
   - core `settings.rs`: `owned_scope` (pure; ownership from the folder's
     `SavedVariables` declarations, live `.toc` or the presence baseline;
     never execute Lua) and `checkpoint`;
   - core `removal.rs`: `RemovalSettings::Checkpoints { roots }` when
     settings apply; plan `SettingsCleanup` after the code move
     (same-volume moves to `.wowsync\recovery\…\settings\…`, guard before
     each move, ack `paired|partial|code_only|kept_paused|not_applicable`
     after verification);
   - runtime removal step: gather settings ops for removed folders; WoW
     running blocks; paused settings are kept (`kept_paused`);
   - recovery: restoring a removed add-on restores code **and** settings
     (`RestoreRemoved`), and `RestoreRemovedSettings` still works alone.
   - Three-states rule: settings recovery points count against the same
     current-plus-two retention as other settings states (A-15). Visible
     temporary exceptions for unresolved recovery data stay visible.
3. **Tests (positive and negative):** uninstall on A removes code and
   settings on A and on peer B and records the checkpoint; restore returns
   both byte-identical; the next cycle does not remove again; paused
   settings are kept; WoW running blocks; unknown ownership is not moved; a
   shared `SavedVariables` file declared by another installed add-on is not
   moved. One A-38 control per goal.

## Files you own

- `crates/wow-sync-core/src/{removal.rs,settings.rs,exec.rs,records.rs}` and
  their `*_tests.rs` files, `core/src/journal*.rs` if needed.
- `crates/wow-sync-runtime/src/{removal*.rs,settings_cycle.rs,effects.rs}`,
  runtime `e2e.rs` (new tests only, in a clearly marked block).
- `crates/wow-sync-runtime/src/service.rs`: small, local edits only.
  R5-decide also edits it.
- `docs/design/removal.md`, `docs/design/settings.md`: only to record
  as-built notes or a contradiction you found, never to change intent.
- Your required-cases section.

Do not edit app crates, `control.rs`, `notices.rs`, `diag.rs`, `choice.rs`,
`target.rs`, `decide.rs`. If you need one, stop and report.

## Done

Both tiers pass; evidence at `docs/runs/run-005/evidence/R5-remove.md`;
reply with SHA. Budget about 1,800 AIU; stop and report at 2,200.
