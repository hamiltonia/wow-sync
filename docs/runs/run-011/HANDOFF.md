# Run 011 handoff

State: **in progress** (D-33). Checkout `D:\GitHub\wow-sync`, branch `main`.
Run-010 is closed and `v0.3.3` is published.

- **Plan:** [PLAN.md](PLAN.md). **Packets:** [packets/](packets/).
- **Owner review list:** [DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md).
  The owner is away, so every judgment call is logged there.
- **Merged to main:** R11-runtime interfaces and step 2 (`84b9beb`), and
  R11-app's guided offer, consent, statuses and version 0.4.0 (`05b4cd4`).
- **In progress:**
  - **R11-runtime follow-up:**
    - set the progress source on peer applies;
    - fix: a single missing character makes every add-on wait;
    - add tests for OwnedBySeveral, progress events and name cleaning;
    - fails-without-fix checks;
    - lab re-run with a memory measurement.
  - **R11-review** (claude-opus-5.5) on main `f3fc1a0`, worktree
    `run011/R11-review`.
- **Next:**
  1. Merge the R11-runtime follow-up.
  2. Run review fix rounds.
  3. Get CI green, then do a release dispatch.
  4. Run R11-check (gpt-6.1-sol) on the artifact.
  5. Tag `v0.4.0` and close the run.
