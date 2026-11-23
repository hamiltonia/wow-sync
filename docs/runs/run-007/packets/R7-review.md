# R7-review: safety review before `v0.3.0`

Role: integration review / claude-opus-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R7-review`, branch `run007/R7-review`.
Read-only for product code. Write only
`docs/runs/run-007/evidence/R7-review.md`.

## Scope

Review the run-007 diff on `main` (`git diff 8b0dee0..<SHA named by the
orchestrator>`) against:
- SPEC R-03, R-04, R-05 (as amended by D-29) and R-06;
- AGENTS.md product rules;
- A-47 and A-48.

Focus on:

1. **Unrecognised add-ons.**
   - Can any path choose a winner by timestamp, clock or machine preference?
   - Is "lineage" proven from records, or inferred from local state that
     could be stale or forged by a peer?
   - Can an offline peer resurrect a rejected or removed copy?
   - Does a decision block only that add-on?
   - Are recovery copies kept for every replacement?
2. **Header-only toc.**
   - Can a crafted `.toc` exhaust memory or time?
   - Does a truncated header ever change variant or settings ownership
     unsafely?
3. **Discovery and picker.**
   - Can a crafted `product.db` path, a junction or a picked folder escape
     the identity and scope confirmation, or cross variants?
   - Can a network or removable drive be probed?
4. **Log.**
   - Any private data (profile paths, names, settings contents)?
   - Is it bounded in size?
   - Does it affect the memory budget or startup time?

Report findings RV7-01, RV7-02, … with a severity (critical / major / minor),
the file and line, and a suggested fix. Reply with the SHA and the counts.
Budget about 300 AIU; stop at 400.
