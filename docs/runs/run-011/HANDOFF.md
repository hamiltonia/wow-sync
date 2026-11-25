# Run 011 handoff

State: **in progress** (D-33). Checkout `D:\GitHub\wow-sync`, branch `main`.
Run-010 is closed and `v0.3.3` is published.

- **Plan:** [PLAN.md](PLAN.md). **Packets:** [packets/](packets/).
- **Owner review list:** [DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md)
  (OD-01 to OD-51). The owner is away, so every judgment call is logged
  there. OD-41 records a checker incident: two lines were written to the
  owner's log, and nothing else was touched.
- **Done and merged to main:**
  - R11-runtime: settings statuses, local copies, explicit share and share
    when ready, separate retention, Suspect files, settle wait, streaming
    (OD-30), provider cleanup (OD-34), lazy code publish (OD-39).
    - Fix rounds: pause all/resume all, choice metadata, recovery
      metadata, G9/G10/G11 fixes.
    - Memory rounds (OD-47, 49, 50): hidden after UI use went from about
      98-100 MiB to 80.6-81.2 MiB on the same lab.
  - R11-app: guided offer, consent, status wording, GUI recovery restore,
    Activity rows, version 0.4.0, and G3/G4/G5/G7/G9/G10 GUI fixes.
  - R11-review: no open critical or major finding (`a73837f`).
  - R11-check rounds 1 (`af62309`) and 2 (`20255ea`) both FAILED. Their
    findings G1-G12 have been fixed since.
- **CI:** green on `902e720` (45688966090). Release dispatch 45692122867
  succeeded on `902e720`.
- **In progress:** R11-check re-verification 3 of all gates on the
  `902e720` artifact (worktree `run011/R11-check`).
- **Next:**
  1. Merge the R11-check evidence; run fix rounds if it reports FAIL.
  2. Write REPORT.md, update USAGE.md and the README.
  3. Tag `v0.4.0`, push the tag and confirm the release assets.
  4. Remove the R11 worktrees and branches.
