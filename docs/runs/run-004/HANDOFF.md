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

## Owner decisions after close (D-26)

- Q-88: option (a), not yet implemented.
- Q-77: 32 MiB, not yet applied; real settings sync stays disabled until
  run-005 wires the value.
- The owner will run [MAC-OWNER.md](MAC-OWNER.md) on a Mac; results pending.
- A `v0.1.0` Windows pre-release is approved; publishing is a run-005 step.
- Run-005 ceiling: 10,000 AIU (stop at 9,500). **The run-005 plan is not
  yet approved**, so no workers may launch.

## Next safe steps (need an approved run-005 plan)

- RV-04: uninstall captures and removes settings; plan in
  [R4-integrate evidence](evidence/R4-integrate.md) "Pass 17".
- Independent GUI recheck of the V-02 group fix (`2cc8858`) and RV-01.
- RV-09, RV-10, RV-11 and the DataAtRisk producer.
- Implement Q-88 (a) and the Q-77 32 MiB cap.
- Build and publish the `v0.1.0` Windows pre-release with known-gaps notes.
