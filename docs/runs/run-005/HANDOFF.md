# Run 005 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-21, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-005 is **closed**: [REPORT.md](REPORT.md). Plan [PLAN.md](PLAN.md)
  (D-27), ledger [USAGE.md](USAGE.md), packets in [packets/](packets/).
- `v0.1.0` is tagged on `8ecb621` and published as a GitHub pre-release
  (tag release run 44957612025). The next run needs a new owner-approved
  plan and ceiling.
- Worker worktrees: `D:\GitHub\wow-sync.worktrees\R5-<id>`, branches
  `run005/R5-<id>`.

## Progress

| Packet | State |
|---|---|
| R5-remove | merged `1c543ce` (branch `50d7ee2`) |
| R5-decide | merged `1c543ce` (branch `02bda94`); follow-up `220c9b0` (update-check "no release yet" result, as-built notes) merged `7a4df23` |
| R5-app | stage A merged `1fb0234` (`90ecfa4`); A2, Q-77 banner and no-release text merged `559cb4d` (`9261037`) |
| R5-release | merged `4ec8104` (`22eb997`, LICENSE `74a0c38`); first dispatch run 44942657034 failed (`-Maxdepth`); fix `81ee2a3` merged `4675f08`; dispatch run 44945165878 |
| R5-check | evidence merged `861f2ff` (`eb87de0`). Checks 1, 2, 4, 5, 6 PASS; 3 and 7 FAIL on F1 (major: originating code survived a removal after a V-02 group restore) and F2 (minor: stale 404 text in the release notes); 8 BLOCKED by the packet's `gh` limit. The orchestrator supplies check 8: CI 44945258236 on `1324b45` passed all 6 jobs. GUI items BLOCKED: Q-77 over-32-MiB window, native tray tooltip, GUI 404 state (dev build has no update source). The artifact smoke test passed on dispatch run 44945165878. Fix round: F2 notes fixed `d94d173` → `103a02f`; F1 fixed `1c50d9b` → `bd39010` (root cause: the apply plan reinstalled the folder in the cycle that recorded its removal). R5-remove then found that Restore from Removed cannot bring code back after a real uninstall, because the removal.md §7.4 code source and restore operation were never built. This is within goal 1, so R5-remove implemented §7.4 (`9d3a144`) and restore progress (`debc58f`), and R5-app added the confirmation, result and progress GUI (`4da4477`, `f478fca`). An R5-check GUI recheck is running from `f478fca` |
| R5-review | evidence merged `0df1a8e` (`b85d9eb`): no critical or major findings; §7.4 restore safety holds. RV5-01 (moderate), RV5-02, -04, -05, -06 and -07 fixed (`5b9a337`, `d24310d`); RV5-03 (stale README/REPO-MAP) fixed by the orchestrator |
| R5-check recheck | evidence merged `1a76cf8` (`edb4eac`): F1 sequence, other-peer restore and WoW-running wait PASS, with restored hashes matching. F3 (Removed hidden with zero active add-ons) fixed `2030912`; F4 (refusals not persisted) fixed `5b9a337`; both merged `54656e8` |
| R5-check final recheck | evidence merged `fc8f734` (`4ed9f08`) on candidate `8ecb621`: F3, F4, RV5-01, artifact smoke test (checksum, contents, notes, About 0.1.0, `build_commit` `8ecb621`) and CI 44955214721 PASS; no new findings |
| Tag | `v0.1.0` → `8ecb621` pushed; release run 44957612025 passed and published the pre-release with the zip and `.sha256` |

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
- Run the published `v0.1.0` exe on synthetic data (owner); see
  [REPORT.md](REPORT.md) §6.
- Next run scope and ceiling (owner).
