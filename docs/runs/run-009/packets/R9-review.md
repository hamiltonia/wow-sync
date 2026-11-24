# R9-review: safety review of run-009

Role: review / claude-opus-5.5. Rules: [COMMON.md](COMMON.md). Read-only on
product code. You may run probes on synthetic data; revert them afterwards.

Review the R9-decide and R9-app merges on `main` (the SHA named by the
orchestrator) against [../PLAN.md](../PLAN.md) goals 1–7. Look for:

- **Decisions:**
  - Can the conflict fix merge competing intentional choices silently? It
    must not (SPEC): the owner picks.
  - Can a keep now be written that excludes the copy it keeps, or that fails
    to cover a conflicting record?
  - Does the stale-evidence fix refuse legitimate choices?
  - Rollback, `NewVersionAfterDecision` and retired-computer rules.
- **Hash cache:**
  - Can a changed file be missed: same size and same mtime after an updater,
    clock changes, or FAT/2-s mtime granularity?
  - Is the cache trusted across a different install root or a moved folder?
  - Is mtime used anywhere as ordering evidence?
  - Memory limit.
- **Command handling during a pass:** races with apply or write steps,
  quit-when-safe, and stale UI.
- **Headline and timeouts:** wrong wording or blame, and a decision hidden.
- **Log privacy and member-version compatibility** with `v0.3.1` peers.
- **Release notes** match the behaviour.

Write `docs/runs/run-009/evidence/R9-review.md` with findings RV9-01, … and a
severity for each. Commit, then reply with the SHA and a verdict: are there
any open critical or major findings?
