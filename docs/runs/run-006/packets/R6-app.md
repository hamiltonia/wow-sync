# R6-app: plain-language setup and test-mode GUI

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R6-app`, branch `run006/R6-app`.

## Why

The owner ran `v0.1.0` on a real install and the setup screen (step 3 of 6,
"Game installation") showed:
- a card with a blank first line, then `_forever_`;
- in orange, "Can't be synced: This folder cannot be used:
  NotSyntheticInstall";
- "This installation was refused. You can go Back and choose another, or
  Try again after fixing it.";
- a "Try again" button that can never succeed.

The owner reported: "got stuck on this page, no reason given as to why". The
raw text comes from `format!("This folder cannot be used: {r:?}")` in
`crates/wow-sync-runtime/src/service.rs` (~line 1618).

## Goals

1. **Stage A: start now (PLAN goal 4).**
   - Replace every `{:?}`-formatted reason that reaches the user with a
     plain sentence: what is wrong, then what to do. Cover all
     `RefuseReason`, `LayoutReason` and `BlockReason` variants and any
     similar debug strings shown during setup.
     - Search `app` and `runtime` for `{:?}` / `{r:?}` / `{e:?}` in text the
       user can see.
     - Keep internal codes only in diagnostics export, if you keep them
       anywhere.
   - "Try again" is shown only when an outside fix can change the result,
     for example files unreadable or AddOns missing. When it is not shown,
     the text points to Back.
   - The variant card always shows a heading. Use the product label from
     `VariantIdentity::display_label` when known, otherwise the folder
     name. Find why the first line is blank and fix it.
   - The setup scope step lists blocked add-ons with readable reasons, for
     example "has files only for other game versions (Classic, TBC)",
     collapsed when long. Eligible and blocked counts stay visible.
   - Tests: model or unit tests that no user-facing reason string contains
     a Rust enum identifier or `{`/`}` debug output, plus one A-38 control.
     Also a GUI screenshot of the refusal and scope steps, taken with a
     devkit or synthetic install under `D:\WoWSyncLab\R6-app`.
2. **Stage A2: after the orchestrator says R6-gate's API is merged (PLAN
   goal 5).**
   - Add a Settings toggle, "Test mode: only sync test folders", with a
     one-line explanation.
   - Wire it to R6-gate's command and snapshot field.
   - Setup and status explain the test-mode refusal or block in plain
     words.
   - When a refusal reason comes from test mode, setup offers "Turn off
     test mode".
   - Run `git merge main` first.

## Files you own

- `crates/wow-sync-app/src/**` except `bin/devkit.rs`, which belongs to
  R6-release.
- `crates/wow-sync-proto/src/**`, only if its model mirrors the app model
  text you change.
- `crates/wow-sync-runtime/src/service.rs`: only the refusal and problem
  text function near line 1618, plus small helpers. R6-gate also edits
  this file.
- Your required-cases section.

## Done

Stage A: both tiers pass, then reply with the SHA and stay available for A2.
Stage A2: the same, plus evidence at `docs/runs/run-006/evidence/R6-app.md`.
Budget about 1,200 AIU; stop and report at 1,600.
