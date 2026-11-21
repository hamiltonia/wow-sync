# R5-app: GUI fixes now, then Q-88 and DataAtRisk screens

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R5-app`, branch `run005/R5-app`.

## Stage A (start now)

1. **Keep sends the real reason.** `app/src/change_ui.rs:21` hard-codes
   `ChoiceReason::Ambiguous` and Keep sends an empty `allow_again`. Send the
   pending decision's actual reason and allow-again scope from the decision
   view (contracts.md §5, decide.rs mapping `DecisionReason` → `ChoiceReason`).
   Test that each decision kind sends its own reason; A-38 control.
2. **Diagnostics save never overwrites silently.** Today it writes a fixed
   Documents path and overwrites the previous export. Use a new
   timestamped file name per save (no new dependency), show the saved path,
   and keep the existing destination refusals. Test two saves give two
   files and the first is unchanged.
3. **RV-11 (app part).** `app/src/platform.rs` uses absolute macOS helper
   paths (`/usr/sbin/scutil`, `/bin/hostname`, `/bin/date`).
4. **Q-77 banner.** With the cap now set (R5-remove), the missing-cap banner
   in `settings_ui.rs` must not show. Adjust it and its tests once
   R5-remove merges; until then, leave it.

Reply with a SHA after stage A. Do not wait idle for stage A2.

## Stage A2 (after R5-decide merges; the orchestrator sends the API note)

5. **Q-88 (a) in the GUI.** The New-version-after-keep decision says W is
   installed only on this computer and not shared, offers Keep V / Use W /
   Follow newest again / Decide later (contracts.md:1019), and stays in
   Needs attention until answered.
6. **DataAtRisk.** Show it in the headline, tray and Needs attention,
   whatever the notification toggles (Q-86), with interaction.md:147 copy.

Tests for each; one A-38 control per item.

## Files you own

`crates/wow-sync-app/**`. Your required-cases section. No core, runtime or
adapter files: if the API is missing something, report it.

## Done

Both tiers pass; evidence `docs/runs/run-005/evidence/R5-app.md`; reply
with SHA per stage. Budget about 1,300 AIU total; stop and report at 1,700.
