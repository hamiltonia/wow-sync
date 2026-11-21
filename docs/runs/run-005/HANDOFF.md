# Run 005 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-21, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-005 is **active**: [PLAN.md](PLAN.md) (D-27), ledger [USAGE.md](USAGE.md),
  packets in [packets/](packets/).
- Worker worktrees: `D:\GitHub\wow-sync.worktrees\R5-<id>`, branches
  `run005/R5-<id>`.

## Progress

| Packet | State |
|---|---|
| R5-remove | launched |
| R5-decide | launched |
| R5-app | launched (stage A) |
| R5-release | launched |
| R5-check | not started |
| R5-review | not started |

## Owner barriers

- macOS run of [../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md) (owner).
- The `v0.1.0` tag is pushed only after R5-check passes the artifact test.
