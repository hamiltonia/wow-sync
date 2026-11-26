# Run 011 handoff

State: **closed** (D-33). Checkout `D:\GitHub\wow-sync`, branch `main`.
`v0.4.0` is tagged from main after CI passed. The product code was verified
as dispatch 45749110502 (`7a65a11`).

- **Release:** tag `v0.4.0` at `19fbfbc`, published by release run 45784547521
  as a pre-release. `wow-sync-0.4.0-windows-x64.zip` SHA-256 is
  `55681ebe4e086f8a9ff4f67746b9b2c8c14c0087e79ba0c785cc12d1ae591fee`. Since
  `7a65a11`, the only code changes are a test module (`e2e.rs`) and a lint
  attribute that applies only to tests.

- **Report:** [REPORT.md](REPORT.md). **Usage:** [USAGE.md](USAGE.md).
- **Owner review list:** [DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md)
  (OD-01 to OD-64), waiting for the owner's review. The report lists the
  highlights.
- **Workers:** R11-runtime, R11-app, R11-review and R11-check are finished.
  Their worktrees and local branches are removed.
- **Open items for a future run:** long-lived screen-reader memory (OD-64),
  the headline while WoW runs (OD-59), saving "Not now", and the RV11-19
  residual.
- **Next:** the owner reviews the decisions, then decides whether to turn on
  settings in the real group. Any new run needs a plan and approval.
