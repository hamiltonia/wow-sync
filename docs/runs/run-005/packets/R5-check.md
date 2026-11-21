# R5-check (verification, gpt-6.1-sol): phase B

Read [COMMON.md](COMMON.md) and `AGENTS.md` first.

You are an independent verifier. Don't trust the authors' completion claims;
re-run the checks yourself. The authors' evidence is in
`docs\runs\run-005\evidence\`: R5-remove, R5-decide, R5-app and R5-release.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\R5-check`, on branch
  `run005/R5-check`, created from `main` at the commit the orchestrator names
  in your prompt.
- **Owned paths:** `docs\runs\run-005\evidence\R5-check.md`,
  `docs\runs\run-005\evidence\R5-check\` (under 1 MB) and your section of
  `tests\required-cases.toml` (leave it empty unless you add a test, which you
  shouldn't).
- **Don't change** product code, tests, tools or other docs. Temporary
  mutations for negative controls are allowed, but never commit them. Make and
  revert them with the edit tool, then show `git status` clean.
- **File writes:** use only the edit and create tools, with absolute paths
  under your worktree or lab root. Don't use .NET file APIs or `Set-Content`
  for repository files. Before committing, check that
  `git -C D:\GitHub\wow-sync status --short` is empty. If it isn't, stop and
  report.
- **Data:** use synthetic data only, under `D:\WoWSyncLab\R5-check`. Never
  read or write real WoW, Battle.net or Dropbox data, and never touch
  `D:\WoWSyncDemo`. Stop only processes you launched, by their captured PID.
- **Network:** don't call GitHub's API from the app. `gh` may be used only for
  check 6 (read-only).

## Checks

1. **Tiers and flake:** run `cargo xtask verify --tier offline` once and
   `--tier native` **three times**, saving each full log under your evidence
   folder. One native run on main failed intermittently, cause unknown (see
   HANDOFF "Intermittent native tier"). If any run fails, record the failing
   step and test output exactly.

2. **A-38 controls:** re-apply at least five of the authors' recorded
   controls, one from each of these areas:
   - RV-04 uninstall/restore;
   - Q-77 cap;
   - Q-88 local-only;
   - DataAtRisk;
   - the app's keep-reason or diagnostics-new-file control.

   Confirm that each named test fails, then revert.

   Add at least two controls of your own:
   - a uninstall that removes code but leaves settings must fail a test;
   - a settings file of exactly 33,554,432 bytes vs one byte more. Check the
     boundary is where Q-77 says, and that the marker gate still blocks
     unmarked data.

3. **GUI drive (Windows):** build the release app and devkit
   (`cargo build --release -p wow-sync-app`). Use devkit to build a synthetic
   lab of two or more computers sharing one provider folder, as
   `tools\mem\run.ps1` does. Drive the real window by keyboard and UI
   Automation through the following, recording PASS, FAIL, BLOCKED or NOT RUN
   with the observed text for each:
   - **V-02 group fix** (`2cc8858`) and RV-01 as described in
     `docs\runs\run-004\REPORT.md`.
   - **Keep and Resume** on a pending decision. Confirm the result reflects
     the decision's real reason, not "ambiguous".
   - **Q-88:** after a keep, a newer version installed only on this computer
     shows as installed here only and not shared. The decision offers Keep /
     Use / Follow newest again / Decide later, and stays in Needs attention
     until answered. The kept version must not be reinstalled over it.
   - **RV-04:** uninstall an addon that has settings. Both code and settings
     go away; restore from Removed brings back both.
   - **Q-77:** no "size limit pending" banner appears. A synthetic marked
     settings file over 32 MiB is refused for that unit, with an honest
     message.
   - **DataAtRisk:** induce at least one risk kind in the lab, such as an
     unreadable local state file in the lab root. It must show in the
     headline, tray and Needs attention, even with notifications turned off.
   - **Export diagnostics:** two saves give two distinct files, and neither
     overwrites the other. The export includes file fingerprints and the
     build commit (RV-09). Saving into the shared folder is still refused.
   - **About:** the version reads 0.1.0. With a faked or offline update
     source that returns 404, the text is "No release has been published
     yet.", not an error.

4. **Recovery without a terminal:** confirm that restore-removed (code and
   settings), rollback and restore-settings can each be completed from the
   GUI alone.

5. **Memory:** run
   `tools\mem\run.ps1 -Runs 1 -LabRoot D:\WoWSyncLab\R5-check\mem -OutDir docs\runs\run-005\evidence\R5-check\mem`.
   Compare the result with the 100 MiB aggregate gate and the ten-second
   sync-start rule.

6. **Release artifact smoke test:**
   1. Download the artifact of the `release` workflow run the orchestrator
      names, with `gh run download <id> -D D:\WoWSyncLab\R5-check\artifact`.
   2. Check the zip name `wow-sync-0.1.0-windows-x64.zip`, its contents (exe,
      LICENSE, README, release notes) and that its SHA-256 matches the
      `.sha256` file.
   3. Launch the extracted exe against a synthetic lab with
      `WOWSYNC_*` lab settings, as in `tools\mem\run.ps1`. Confirm the window
      opens, About shows 0.1.0, and the diagnostics export carries the
      workflow's commit SHA.
   4. Stop it by PID.

   Also review `.github/workflows/release.yml`:
   - actions are pinned by SHA;
   - `contents: write` is granted only on the publish job;
   - the tag is checked against the version;
   - `workflow_dispatch` never publishes.

7. **Release notes accuracy:** check each claim in `docs\release\v0.1.0.md`
   against the code and evidence. Each claim must hold, and the known gaps
   (Q-24 gate, no macOS build, unsigned, no installer) must be stated.

8. **CI:** confirm the latest `main` CI run is green on every job, and link
   it.

## Done

- Commit the evidence with the prefix `run005(R5-check):` and the Copilot
  trailer. Don't push.
- Give a verdict per check: PASS, FAIL or BLOCKED.
- List the findings, each with a severity (blocker, major or minor) and the
  exact reproduction.
- Budget: about 800 AIU. Stop and report at 1,100.
