# R7-check: independent verification of A-47, A-48 and the log

Role: verification / gpt-6.1-sol. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R7-check`, branch `run007/R7-check`.
Write only your evidence and your required-cases section. Do not change
product code. Report defects as findings G1, G2, …, each with a severity
(critical / major / minor) and reproduction steps.

## Build under test

- The `main` SHA named by the orchestrator, built with `cargo build --release`
  in your worktree.
- The dispatch artifact named by the orchestrator: download it, check its
  `.sha256`, unzip it and smoke-test it.

## Checks

1. **A-47 discovery.**
   - Make a synthetic install `D:\WoWSyncLab\R7-check\G\Games\World of Warcraft\_forever_`
     (use the existing testkit/devkit layout).
   - Map it with `subst` to a free letter beyond F; record the letter. Run
     setup with `--state` in your lab and show that step 3 lists it.
   - Remove the `subst` letter afterwards.
   - Show that a folder outside every probe is not listed, and that step 3
     shows the not-found text and "Choose folder…".
   - Drive the picker if you can automate it (UI Automation). Otherwise
     verify the root/variant normalization through the code path the app
     uses, and label that honestly.
   - Show that a non-WoW folder gets the plain explanation.
   - Optional: if you can create a network mapping to an unreachable share
     without credentials, show that setup does not stall. Otherwise mark it
     NOT RUN and say why.
   - Read `%ProgramData%\Battle.net\Agent\product.db` on this PC (read-only).
     Show that discovery reports `D:\Games\World of Warcraft` from it.
2. **A-48 on lab copies.** Two simulated computers with `--state`, a test
   provider under `D:\Dropbox\Gaming Sync\WoW Sync Test\R7-check` (delete it
   at the end), and synthetic add-ons:
   - a 1 MB+ `.toc` whose header comes first: recognised, with its version;
   - no `.toc`;
   - an ambiguous `.toc` choice.
   For the unrecognised ones:
   - only on A: it propagates;
   - identical on both: nothing to do;
   - changed on A only after a shared release: A's copy wins with no prompt;
   - changed on both: one ChooseCopy decision, alternatives preserved,
     resolved through the GUI;
   - removal, then restore through the GUI.
   Also show that no timestamp decides: touch mtimes only, and nothing
   changes.
3. **Real data (D-14, D-28).** Take a hashed backup of
   `D:\Games\World of Warcraft\_forever_\Interface\AddOns` and `..\WTF`.
   - Enroll the real install into an isolated `--state` with a test provider.
   - Show that QuestieDB is recognised and published, and that there are 0
     blocked add-ons, or list any with their reasons.
   - Restore and prove exact manifest equality, as R6-check did.
   - Never start or kill WoW.
4. **Log.**
   - The default log exists under the state dir's `logs\`.
   - It records the discovery probes, the product.db result, the setup steps
     and the blocked/unrecognised reasons.
   - It contains no user-profile path, no add-on or settings contents and no
     names.
   - It rotates at 1 MiB with one previous file (drive the size with the
     dev flags or repeated runs).
   - `WOWSYNC_LOG` overrides the path.
   - "Open data folder" opens the state dir.
5. **Regression.** Both verify tiers pass on the SHA under test.

## Done

Write `docs/runs/run-007/evidence/R7-check.md` with PASS / FAIL / BLOCKED /
NOT RUN for each check and the environment. Commit it with redaction (`~`
for profile paths). Reply with the SHA and the findings. Budget about 700
AIU; stop and report at 900.
