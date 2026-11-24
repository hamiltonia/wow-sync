# R10-review: review of run-010

Role: review / claude-opus-5.5. Rules: [COMMON.md](COMMON.md). Read-only on
product code. You may run probes on synthetic data; revert them afterwards.

Review the R10-history, R10-speed and R10-app merges on `main` (the SHA named
by the orchestrator) against [../PLAN.md](../PLAN.md). Look for:

- **History accuracy:**
  - wrong source computer, version or reason (for example a choice
    attributed to the wrong computer, or "newer release" for a rollback);
  - counts or bytes computed from the wrong trees;
  - records lost on a crash or restart;
  - unbounded growth;
  - legacy lines broken.
- **Privacy:** absolute paths, profile, account or character names in
  history, progress, tray or logs.
- **Progress:** stale or wrong headlines, a hidden pending decision that is
  still pending, a progress emission that blocks or slows the pass, races with
  quit-when-safe.
- **Scan and cache:**
  - can the single shared scan go stale across phases within a pass (for
    example files written by the apply and then used by publish)?
  - can seeding the cache after an install hide a later change, or a failed
    or partial install?
  - is mtime used as ordering evidence?
  - memory.
- **Release notes** match the behaviour.

Write `docs/runs/run-010/evidence/R10-review.md` with findings RV10-01, …
and a severity for each. Commit, then reply with the SHA and a verdict: are
there any open critical or major findings?
