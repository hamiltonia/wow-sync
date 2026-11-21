# R5-review (integration review, claude-opus-5.5): phase C

Read [COMMON.md](COMMON.md) and `AGENTS.md` first.

You are an independent integration reviewer. You did not write this code.
Do not trust the authors' or verifier's claims; read the code and re-run what
you rely on. The orchestrator writes the run report; you write the review
evidence.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\R5-review`, branch
  `run005/R5-review`, created from the `main` commit named in your prompt.
- **Owned path:** `docs\runs\run-005\evidence\R5-review.md` only.
- Product code, tests, tools and other docs are read-only. Do not fix
  anything; report each finding with file:line, impact and the smallest fix.
  Temporary mutations to confirm a suspected gap are allowed, but are never
  committed. Make and revert them with the edit tool, then show
  `git diff --exit-code`.
- **File writes:** use only the edit and create tools, with absolute paths.
  Do not use .NET file APIs or `Set-Content` on repository paths. Before
  committing, check that `git -C D:\GitHub\wow-sync status --short` is empty.
- Use synthetic data only. Never touch real WoW, Battle.net or Dropbox data,
  or `D:\WoWSyncDemo`. Stop only processes you started, by captured PID.

## Inputs

- Plan: [PLAN.md](../PLAN.md). Ledger: [USAGE.md](../USAGE.md). Handoff:
  [HANDOFF.md](../HANDOFF.md), including incident I-5 and the native-tier
  flake note.
- Design: `docs\design\` (contracts, ops, settings, removal, questions Q-77
  and Q-88). Owner decisions: `docs\OWNER-DECISIONS.md` D-26 and D-27.
- Evidence: `docs\runs\run-005\evidence\`. The R5-remove, R5-decide, R5-app,
  R5-release and R5-check evidence, plus any fix rechecks.
- Release: `.github\workflows\release.yml`, `docs\release\v0.1.0.md`,
  `LICENSE`, `docs\DEPENDENCIES.md`.

## Review

1. **E-06 security and data.** Walk the trust boundaries run-005 changed:
   - uninstall settings capture and restore (paths, scope, overwrite of
     newer data);
   - the Q-77 cap and the retained Q-24 marker gate;
   - the DataAtRisk inputs;
   - the diagnostics fingerprints and build commit (no private paths or
     SIDs);
   - the update check's 404 handling;
   - the release workflow: permissions, action pinning, secrets, and the
     tag/version check. Confirm `workflow_dispatch` never publishes.
   - the release zip contents.

   Confirm that malicious or partial input fails safely. Confirm that
   dependencies are locked and declared, and that `docs\DEPENDENCIES.md`
   matches the workspace.
2. **Product safety rules** (the AGENTS.md list), focused on run-005:
   - uninstall removes code and settings, and recovery restores both;
   - Q-88 (a): a local-only newer version is never shared and never
     overwritten, and the decision stays visible;
   - variant and account boundaries are kept in settings capture;
   - DataAtRisk is shown whatever the notification toggles say.

   Base this on the code and tests, not the evidence prose.
3. **E-08 repository.** Check that a new agent can find the current state,
   owner barriers and next safe steps from `AGENTS.md`. Look for:
   - stale status text, for example README and REPO-MAP still pointing at
     run-004;
   - drift between `tests\required-cases.toml` and the real test names;
   - evidence that claims more than was run.
4. **Test honesty.** Sample at least six run-005 required cases across core,
   runtime and app. Confirm each asserts the observable requirement and has
   its negative case. Re-run at least two A-38 controls of your own choice
   that R5-check did not use.
5. **Release readiness.** Say whether `v0.1.0` can be tagged. Name any
   claim in the release notes that is false or missing, and confirm that
   the known gaps are stated.
6. **Open items.** List what remains PENDING OWNER (macOS kit runs and any
   new question), and every DEFERRED item with its source.

Run `cargo xtask verify --tier offline` and `--tier native` once each.

## Output

Write `R5-review.md` with:
- a verdict per review area (PASS, FAIL or CONCERN);
- a findings table with columns ID (RV5-nn), severity, file:line, impact and
  smallest fix;
- the open-items list;
- exactly what you ran.

Commit on your branch with the prefix `run005(R5-review):` and the Copilot
trailer. Don't push. Budget: about 400 AIU. Stop at 600 at an area boundary
and say which areas were not reviewed.
