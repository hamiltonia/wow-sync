# Run 008 handoff

Status: **R8-fix running** (2026-11-23). Approval: D-30. Plan:
[PLAN.md](PLAN.md).

| ID | Status | Branch / SHA |
|---|---|---|
| R8-fix | running | `run008/R8-fix` |
| R8-review | waiting for R8-fix | |
| R8-check | waiting for R8-fix and the dispatch build | |

A fresh orchestrator resumes from this table. Read the evidence files, check
each branch with `git log main..run008/<ID>`, and merge with `--no-ff` after
both verify tiers pass.
