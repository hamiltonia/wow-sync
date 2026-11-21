# R4-review (integration review, opus-5.5): phase E

Read [COMMON.md](COMMON.md) and `AGENTS.md` first.

You are an independent integration reviewer. You did not write this code.
Do not trust the authors' or verifier's claims; read the code and re-run
what you rely on. The orchestrator writes the run report; you write the
review evidence.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-review`, branch
  `run004/R4-review`, created from the `main` commit named in your prompt.
- **Owned path:** `docs\runs\run-004\evidence\R4-review.md` only.
- Read-only for product code, tests, tools and other docs. Do not fix;
  report findings with file:line, impact and the smallest fix. Temporary
  mutations to confirm a suspected gap are allowed but never committed;
  make and revert them with the edit tool and show `git diff --exit-code`.
- Synthetic data only; never touch real WoW, Battle.net, Dropbox data or
  `D:\WoWSyncDemo`. Stop only processes you started, by captured PID.

## Inputs

- Plan: [PLAN.md](../PLAN.md); ledger: [USAGE.md](../USAGE.md).
- Design: `docs\design\` (contracts, ops, settings, removal, rollback,
  questions). Owner decisions: `docs\OWNER-DECISIONS.md`.
- Evidence: `docs\runs\run-004\evidence\` (R4-core-*, R4-integrate,
  R4-notify-diag, R4-app, R4-mac-kit, R4-check with its two rechecks).

## Review

1. **E-06 security and data.** Walk the trust boundaries the run-004 code
   added or changed: provider folder input (rollback requests, removal and
   settings records, recovery metadata), local recovery restore, settings
   consent/account listing, diagnostics export (redaction, destination
   refusal, journal content), update check, notifications, and the macOS
   kit scripts. Confirm malicious or partial input fails safely, no
   imported Lua/addon content is executed, no private data reaches logs,
   evidence or CI, and dependencies stay locked and declared.
2. **Product safety rules** (AGENTS.md list) against the integrated code:
   variant/account boundaries, retired computers cannot publish, no silent
   master election, rollback rejection is group-wide and offline peers
   cannot resurrect it, WoW running blocks game-file changes, three-state
   retention with visible exceptions, uninstall removes code and settings
   with recovery. Use the code and tests, not the evidence prose.
3. **E-08 repository.** Can a new agent find current state, owner barriers
   and next safe steps from `AGENTS.md` without this conversation? Check
   for stale or contradictory status text, required-case inventory drift
   (`tests\required-cases.toml` versus real test names), and evidence that
   claims more than was run.
4. **Test honesty.** Sample at least six required cases across runtime,
   core and app. Confirm each test asserts the observable requirement, has
   its negative case, and is not trivially passing. Re-run at least two
   A-38 controls of your own choice not used by R4-check.
5. **Open items.** List what remains PENDING OWNER (Q-77, Q-88, macOS
   runs, first GitHub release) and every DEFERRED item with its source,
   so the run report is complete.

Run `cargo xtask verify --tier offline` and `--tier native` once.

## Output

`R4-review.md`: verdict per review area (PASS, FAIL, CONCERN), a findings
table (ID RV-nn, severity, file:line, impact, smallest fix), the open-items
list, and exactly what you ran. Commit on your branch; don't push.
Budget: about 1,000 AIU. Stop at an area boundary if you run short and
say which areas were not reviewed.
