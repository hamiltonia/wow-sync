# Run 006 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-22, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-006 is **active**: [PLAN.md](PLAN.md) (D-28), ledger [USAGE.md](USAGE.md),
  packets in [packets/](packets/).
- Worker worktrees: `D:\GitHub\wow-sync.worktrees\R6-<id>`, branches
  `run006/R6-<id>`.

## Progress

| Packet | State |
|---|---|
| R6-gate | launched (phase A) |
| R6-app | launched, stage A (phase A) |
| R6-release | launched (phase A) |
| R6-check | not started (phase B, after phase A merges) |
| R6-review | not started (phase C) |

## Incidents

None yet.

## Owner barriers

- Optional: Q-3 in-client confirmation; correction of the "test opt-out"
  reading (D-28).
- macOS run of [../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md) (owner).
