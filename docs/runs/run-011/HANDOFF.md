# Run 011 handoff

State: **in progress** (D-33). Checkout `D:\GitHub\wow-sync`, branch `main`.
Run-010 is closed and `v0.3.3` is published.

- **Plan:** [PLAN.md](PLAN.md). **Packets:** [packets/](packets/).
- **Owner review list:** [DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md)
  (OD-01 to OD-40). The owner is away, so every judgment call is logged there.
- **Done and merged to main:**
  - R11-runtime: settings statuses, local copies, explicit share and share
    when ready, separate retention, Suspect files, settle wait, streaming
    (OD-30), provider cleanup (OD-34), lazy code publish (memory, OD-39).
  - R11-app: guided offer, consent, status wording, GUI recovery restore,
    Activity rows, version 0.4.0.
  - R11-review: round 1, recheck and final recheck (`a73837f`). No critical
    or major finding is open; RV11-16 (minor, thin memory margin) is
    handled by the three-run gate in R11-check (OD-40).
- **CI:** green on `3cd15bb`, including macOS. Release dispatch
  45601785797 succeeded on `3cd15bb`.
- **In progress:** R11-check (gpt-6.1-sol), worktree `run011/R11-check`,
  testing the dispatch artifact.
- **Next:**
  1. Merge the R11-check evidence; run fix rounds if it reports FAIL.
  2. Write REPORT.md, update USAGE.md and the README.
  3. Tag `v0.4.0`, push the tag and confirm the release assets.
  4. Remove the R11 worktrees and branches.