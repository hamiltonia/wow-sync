# R4-check (verification, gpt-6.1-sol): phase E

Read [COMMON.md](COMMON.md) and `AGENTS.md` first.

You are an independent verifier. Do not trust the authors' completion
claims; re-run the checks yourself. The authors' evidence is in
`docs\runs\run-004\evidence\` (R4-integrate, R4-core-*, R4-notify-diag,
R4-app, R4-mac-kit).

- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-check`, on branch
  `run004/R4-check`, created from `main` at the commit the orchestrator
  names in your prompt.
- **Owned paths:** `docs\runs\run-004\evidence\R4-check.md` and
  `docs\runs\run-004\evidence\R4-check\` (under 1 MB; screenshots only if
  small and free of personal data).
- Don't change product code, tests, tools or other docs. Temporary
  mutations for negative controls are allowed but are never committed:
  make and revert them with the edit tool and show `git status` clean.
- **Data:** synthetic data only, under a neutral lab root such as
  `D:\WoWSyncLab\R4-check`. Never read or write real WoW, Battle.net or
  Dropbox data, and never touch `D:\WoWSyncDemo`. Stop only processes you
  launched, by their captured PID.

## Checks

1. **Tiers and controls:** run `cargo xtask verify --tier offline` and
   `--tier native`. Pick at least six A-38 controls recorded by the authors,
   spread across removal, settings, rollback, decisions/retire, notify-diag
   and the app. Re-apply each, confirm the named test fails, and revert.
   Add at least two controls of your own on safety boundaries: the variant
   or account boundary, a retired computer publishing, and an offline peer
   resurrecting a rejected release.
2. **GUI drive (Windows):** build the release app and devkit
   (`cargo build --release -p wow-sync-app`). Use devkit to build a
   synthetic lab with two or more computers sharing one provider folder,
   as `tools\mem\run.ps1` does. Then drive the real window by keyboard and
   UI Automation through:
   - the review-needed headline;
   - a decision (choose, keep, resume);
   - retire and allow again;
   - rollback and its "not available" refusal;
   - settings master, exception, push, pause/resume, use-shared and restore;
   - the Removed section and restore;
   - About update states (the update source must be offline or faked;
     don't call GitHub);
   - **Export diagnostics…** with both options, including a refused save
     into the shared folder.

   Also do every manual step in the R4-app evidence that can be done on
   Windows, and check that a Windows notification appears with the
   app name after startup.

   For each item, record PASS, FAIL, BLOCKED or NOT RUN, with the observed
   text. Where an action is disabled, record its stated reason, and say
   whether the disabling is honest (a documented gap) or a defect.
3. **Recovery without a terminal:** confirm that each supported recovery
   (restore a removed addon, roll back, restore settings) can be completed
   from the GUI alone.
4. **Memory:** run `tools\mem\run.ps1 -Runs 1 -LabRoot D:\WoWSyncLab\R4-check\mem
   -OutDir docs\runs\run-004\evidence\R4-check\mem`. Compare with the
   100 MiB aggregate gate and the ten-second sync-start rule.
5. **CI:** confirm with `gh run list` / `gh run view` that the latest `main`
   CI run is green on every job, including macOS. Link the run.
6. **Records:** spot-check that the claims in `docs\runs\run-004\USAGE.md`
   ledger rows and the PLAN phase table have evidence. List the macOS rows
   that remain NOT RUN for the owner (`docs\runs\run-004\MAC-OWNER.md`).

## Done

- Commit the evidence with the prefix `run004(R4-check):` and the Copilot
  trailer. Don't push.
- Give a verdict per check: PASS, FAIL or BLOCKED.
- List the findings, each with a severity (blocker, major, minor) and the
  exact reproduction.
