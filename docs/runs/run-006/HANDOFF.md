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
| R6-gate | merged `37d489b` (both tiers green) |
| R6-app | stage A merged `34ef8e5`; stage A2 (test-mode GUI) in progress |
| R6-release | `url` 2.5.9 merged `d4fad8f`; 0.2.0 bump merged `58151ba`; notes fixes in progress |
| R6-check | not started (phase B, after phase A merges) |
| R6-review | not started (phase C) |

Run-006 usage: about 1,070 AIU of 10,000 (combined total 38,687.9 against a baseline of 37,617.3).

## Incidents

None yet.

## Owner barriers

- Optional: Q-3 in-client confirmation; correction of the "test opt-out"
  reading (D-28).
- macOS run of [../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md) (owner).
