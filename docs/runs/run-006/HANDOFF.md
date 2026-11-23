# Run 006 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-22, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-006 is **closed**; see [REPORT.md](REPORT.md). `v0.2.0` is published
  from `5a18370`. Plan [PLAN.md](PLAN.md) (D-28), ledger [USAGE.md](USAGE.md),
  packets in [packets/](packets/).
- No next run is approved. A new run needs an owner-approved plan and ceiling.

## Progress

| Packet | State |
|---|---|
| R6-gate | merged `37d489b` (both tiers green) |
| R6-app | stage A merged `34ef8e5`; stage A2 merged `6e87a35` |
| R6-release | `url` 2.5.9 merged `d4fad8f`; 0.2.0 bump merged `58151ba`; notes fixes merged `e4dadcd` |
| R6-check | merged `b00d86d`: G1 minor, G2 major, G3 tooling; real data restored and proven equal. Fixes G1 `078fe45`, G2 `57dbe1f` |
| R6-recheck | G1/G2/H1 pass on real data; H2 pass on lab copies (`5a18370`); real data restored and proven equal; providers deleted |
| R6-settings | H2 fix `21c05dd` |
| R6-review | merged `6336525`: 0 critical/major; RV6-01..09 fixed (`dbfc7bc`, `ca929fc`, `57dbe1f`) |

Run-006 usage: 3,779.4 AIU of 10,000. All run-006 worktrees and branches are removed.

## Incidents

- I-6: a leftover stand-in `Wow.exe` from run-003 (`wow-sync.worktrees\R3-ui-build`) had been running since 2026-11-17. The orchestrator stopped it by PID before R6-check. The directory `D:\GitHub\wow-sync.worktrees\R3-ui-build` is not a registered
  worktree; it was left on disk for the owner to delete.

## Owner barriers

- Optional: Q-3 in-client confirmation; correction of the "test opt-out"
  reading (D-28); the scope of settings removal on uninstall (REPORT).
- Proposed next: a Dropbox public API spike (owner request; D-21).
- macOS run of [../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md) (owner).
