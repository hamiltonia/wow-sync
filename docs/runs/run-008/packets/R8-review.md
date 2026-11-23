# R8-review: safety review of the run-008 fix

Role: review / claude-opus-5.5. Rules: [COMMON.md](COMMON.md). Read-only on
product code. You may run probes on synthetic data; revert them afterwards.

Review the R8-fix merge on `main` (the SHA named by the orchestrator)
against [../PLAN.md](../PLAN.md) goals 1–5. Look for:

- **Concurrency:** races between a cancelled inspection and a new one, or
  with an enroll or sync step. A stale preview reaching the wrong row. A
  thread or handle leak.
- **Quit:** can Quit now skip a write step that must finish (the quit-when-safe
  rule)? Can it still hang?
- **Root cause:** is the claimed root cause plausible and actually fixed? Do
  the regression tests fail without the fix?
- **Log privacy:** no profile paths, names or contents.
- **Release notes:** they match the behaviour.

Write `docs/runs/run-008/evidence/R8-review.md` with findings RV8-01, … and a
severity for each. Commit, then reply with the SHA and a verdict: are there
any open critical or major findings?
