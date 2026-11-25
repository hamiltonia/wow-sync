# R11-runtime: settings sync runtime, lab proof and fixes

Role: implementation, claude-sonnet-5.5. Read [COMMON.md](COMMON.md).
Owns `wow-sync-runtime`, `wow-sync-core`, `wow-sync-adapters`. Do not edit
`wow-sync-app` except for compile fixes forced by your API change; report
any such edit.

## Step 1: lab trial first (before building anything)

1. Build a two-computer lab (`D:\WoWSyncLab\R11-runtime\`): computers A and
   B, each with a copy of the dev add-ons and `WTF` per COMMON, one
   synthetic provider root, separate `--state` dirs. Make B differ: one
   character folder missing on B, a few settings files edited with
   synthetic changes.
2. Drive it headlessly through the existing runtime API / devkit (the same
   commands the GUI uses: `SetPolicy`, `ConfirmSettingsHere`,
   `ShareSettings`, `PauseSettings`, `ResumeSettings`, `ChooseSettings`,
   `RestoreSettings`, `UseSharedSettings`).
3. Walk A-11 to A-15, the missing character, an over-cap file, a file
   declared by two add-ons, uninstall / re-add, and restore. Record each
   step's observed result.
4. Commit a "Trial" section in evidence listing every defect with
   severity, then reply with that list and your plan before large changes.
   Small obvious fixes may go in straight away.

## Step 2: what to build and fix

- Fix the trial defects.
- **Interface for R11-app** (commit alone first, report SHA), as needed:
  - a query giving the guided-offer state: settings off with no policy;
    a policy exists but this computer has not confirmed; nothing to offer;
    with the master's friendly name and the accounts it reported;
  - a query for a "Not now" dismissal flag persisted in local state;
  - per-add-on settings status already exists (§9.1); add any missing
    plain-word reason, especially "waiting for <identity> to appear on this
    computer" for a missing account or character folder.
- **History and progress (run-010 types):** settings captures, publishes,
  applies, yields, pushes, choices and restores produce `ChangeRecord`s
  with source, add-on, identity in plain words, reason, file count and size
  change. A `Phase` (or equivalent) for the settings step with counts.
- **Logs:** one line per settings phase and per unit changed. No account,
  realm or character names in the log; use the add-on and "account-wide" or
  "per-character" with a count.
- **Clean-up (goal 4):** correct the stale Q-77 "settings off / no cap
  approved" comments listed in PLAN.md.
- **Safety:** re-read every write path under `WTF`: only the two §1.1
  shapes, only into existing folders, temp + rename, recovery copy before
  replace, WoW-running gate. Add a test for any gap you find.

## Done when

- Every trial defect is fixed or recorded as a product decision.
- An automated end-to-end test in `e2e.rs` covers the A-11 to A-15
  observable outcomes on synthetic data shaped like the real data
  (account-wide plus per-character files, a missing character).
- A final lab re-run of step 1 is recorded in evidence.
- Memory: private working set ≤ 100 MiB during the lab capture and apply.
