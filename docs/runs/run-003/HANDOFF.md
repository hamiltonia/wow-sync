# Run 003 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-16, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Plan approved (D-20); see [PLAN.md](PLAN.md).
- Ledger: [USAGE.md](USAGE.md).
- Owner checks pending: [OWNER-CHECKS.md](OWNER-CHECKS.md).

| Phase | Workers | Status |
|---|---|---|
| A | R3-ui-design (opus), R3-infra (sonnet), R3-compat (sonnet), R3-removal-design (opus) | Done, all merged |
| B | R3-design-review (sol) | Review FAIL, merged `917bc05`. Recheck `09b2673`: **UI PASS**. Q-21: R3D-3 owner decision. Removal: R3D-7 to R3D-10 and R3D-14 to R3D-16 closed; **R3D-13 still open, retries exhausted** (recheck2 `07fe195`). It is a blocker for any run that implements removal |
| C | R3-proto (sonnet), R3-mem (sonnet) | R3-mem **A-29 PASS** on Windows x64 (worst 19.6 MiB of 100; sync start ≤0.26 s; macOS BLOCKED), merged `8e82673`. R3-proto merged.
- Screens P1 to P14 and scenarios S1 to S16 run on fake data.
- It added no new dependencies, and both verify tiers pass.
- Launcher: `D:\WoWSyncDemo\Start-Prototype.cmd`.
- Unverified: Ctrl shortcuts, OS DPI, live theme toggle, OS toasts, IME, shutdown. See `evidence/R3-proto.md`.
- The worker once used the forbidden `Set-Content` on source files; it disclosed this and the files were rebuilt. |
| D | Owner barrier | **Done (D-21, 2026-11-17).** Prototype approved. The owner answered Q-21, the UI defaults and the removal defaults, and asked for a Dropbox API spike |
| E | R3-check (sol), R3-review (opus), R3-d21 (sonnet), R3-dropbox-api (opus) | Pending |
| F | R3-ui-build | Approved by D-21. Must include the Q-26 "close quits" option and drop compatibility-hold UI states that D-21 removes |

## Incidents

None yet.

## Owner confirmation needed

- None open. D-21 replaced interim rule A: no Interface or build gating. R3-d21
  removes the gating code. R3D-3 is moot.
- glib alerts 1, 5 and 6 were dismissed per D-21. The grid alerts (2 to 4)
  close when R3-d21 deletes the spikes.

## Follow-ups found

- **R3D-13 (removal design, major, open).** Conflicting removal-record
  variants that span packages cannot be resolved:
  - listing every variant violates V7;
  - omitting one violates the resolution rule.

  The run-003 retries are exhausted. The run that implements removal must
  close it first. D-21 ("removal wins") may simplify it, so re-review it
  against D-21.
- R3-mem limitations:
  - UIA Invoke had no effect, so the UI-use phase used posted keys.
  - Shown-window memory with real interaction may be somewhat higher.
  - The app has no graceful quit command-line option, so the harness used
    `Stop-Process` on the validated root process.

- `fake_client_blocks_real_apply` (runtime) can hit the same process
  start/exit race fixed in the adapters by R3-infra. Assign to a runtime
  packet.
- Dependabot: none of the 6 alerts has a minimal fix. **Owner decision
  needed** (from R3-infra):
  - **glib** (3 alerts): comes from `tray-icon` through Linux-only gtk3-rs,
    which is capped at 0.18. It is not compiled on Windows or macOS.
  - **grid** (3 alerts): only in the obsolete crates.io GPUI spikes
    (`spikes/gpui`, `spikes/mem`, `spikes/a11y`). Deleting those spikes
    would close alerts 1–4.

## Next actions

1. Merge each worker after the orchestrator re-runs offline verify.
2. Push and record usage.
3. Stop launching new work at ≥19,013 AIU cumulative.
