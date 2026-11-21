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
| R5-remove | merged `1c543ce` (branch `50d7ee2`) |
| R5-decide | merged `1c543ce` (branch `02bda94`); follow-up `220c9b0` (update-check "no release yet" result, as-built notes) merged `7a4df23` |
| R5-app | stage A merged `1fb0234` (`90ecfa4`); stage A2 + Q-77 banner built, awaiting commit and the About "no release yet" text |
| R5-release | merged `4ec8104` (`22eb997`, LICENSE `74a0c38`); dispatch build running |
| R5-check | not started |
| R5-review | not started |

## Incidents

- **I-5 (2026-11-21 11:41), file-write rule (COMMON, I-1).** Stray writes
  landed in the main checkout `D:\GitHub\wow-sync`: a duplicate of
  R5-decide's `diag.rs` `fingerprints_from` block with literal `\n` text,
  and a workspace version bump in `Cargo.toml`. Cause: a .NET
  `[IO.File]::WriteAllText` call with a relative path, which .NET resolves
  against the process start directory, not the PowerShell location.
  R5-decide disclosed such a call. R5-app also used a .NET write once, and
  R5-remove used `Set-Content` once, both inside their own worktrees.
  - Containment: the orchestrator saved the stray diff outside the
    repository, restored the two files, and confirmed the workers'
    worktree copies were intact and correct. The R5-app write was checked
    (no BOM, LF endings). Nothing stray was committed.
  - Decision: these breaches were non-destructive and contained, so the run
    continued instead of stopping all work under PLAN §5. All workers were told
    to use the edit/create tools with absolute paths only, and to confirm
    that `git -C D:\GitHub\wow-sync status --short` is empty before each commit.
- **Intermittent native tier (2026-11-21).** After the `7a4df23` merge,
  the first `cargo xtask verify --tier native` returned FAIL. Two immediate
  reruns on the same tree passed. The failing step's output was not
  captured, so the cause is unknown. Treat it as an open flake: capture the
  full log on any recurrence. R5-check should run the native tier
  repeatedly.

## Owner barriers

- macOS run of [../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md) (owner).
- The `v0.1.0` tag is pushed only after R5-check passes the artifact test.
