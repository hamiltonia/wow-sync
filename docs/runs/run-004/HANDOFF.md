# Run 004 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-21, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-004 is **closed**. Report: [REPORT.md](REPORT.md). Ledger:
  [USAGE.md](USAGE.md). Plan: [PLAN.md](PLAN.md).
- All run-004 worker branches are merged; worktrees under
  `D:\GitHub\wow-sync.worktrees\` for R4-integrate and R4-app may remain and
  can be removed.
- Both verify tiers pass on `main`; CI is green on Windows and macOS.

## Open owner barriers

1. Q-88 (newer local install after a keep).
2. Q-77 (settings-file size cap). Real settings sync stays disabled.
3. macOS owner run of [MAC-OWNER.md](MAC-OWNER.md).
4. First GitHub release for the update notice.

## Next safe steps (need an approved run-005 plan and ceiling)

- RV-04: uninstall captures and removes settings; plan in
  [R4-integrate evidence](evidence/R4-integrate.md) "Pass 17".
- Independent GUI recheck of the V-02 group fix (`2cc8858`) and RV-01.
- RV-09, RV-10, RV-11 and the DataAtRisk producer.
- Implement the Q-88 answer once given.
