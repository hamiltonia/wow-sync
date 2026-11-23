# Run 007 handoff

Status: **phase A in progress** (2026-11-23). Approval: D-29. Plan:
[PLAN.md](PLAN.md).

| ID | Phase | Status | Branch / SHA |
|---|---|---|---|
| R7-sync | A | merged | `c0139d2` → main `b82eb0d` |
| R7-setup | A | merged `2e996e8` → main `b287d6b`; follow-up running (macOS test portability, release-notes TODO, log lines for blocked add-ons) | `run007/R7-setup` |
| R7-check | B | waiting on the R7-setup follow-up | |
| R7-review | C | running on `b82eb0d` | `run007/R7-review` |

CI on `b287d6b` failed on macOS, in two R7-setup tests; the follow-up fixes
them.

A fresh orchestrator resumes from this table. Read the evidence files, check
the branches with `git log main..run007/<ID>`, and merge with `--no-ff` after
both verify tiers pass.
