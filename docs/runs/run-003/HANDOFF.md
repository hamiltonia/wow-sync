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
- Prototype screens P1 to P14 and scenarios S1 to S16 are implemented on
  fake data. The interaction logs cover only part of the scenario
  transitions (R3-check RC-2); the evidence is screenshots plus UIA dumps.
- It added no new dependencies, and both verify tiers pass.
- Launcher: `D:\WoWSyncDemo\Start-Prototype.cmd`.
- Unverified: Ctrl shortcuts, OS DPI, live theme toggle, OS toasts, IME, shutdown. See `evidence/R3-proto.md`.
- The worker once used the forbidden `Set-Content` on source files; it disclosed this and the files were rebuilt. |
| D | Owner barrier | **Done (D-21, 2026-11-17).** Prototype approved. The owner answered Q-21, the UI defaults and the removal defaults, and asked for a Dropbox API spike |
| E | R3-check (sol), R3-review (opus), R3-d21 (sonnet), R3-dropbox-api (opus) | R3-d21 merged: gating removed, spikes deleted, negative control recorded. R3-dropbox-api merged: recommends keeping the desktop folder for v1 and a possible read-only API hybrid later; owner questions DA-1 to DA-4 in `docs/design/dropbox-api.md`. R3-check merged:
- **PASS:** D-21 (own negative controls), memory re-run (peak 34.9 MiB on Windows), CI (latest `main` green, including native and macOS).
- **Prototype FAIL** on RC-1 and RC-3; **records FAIL** on RC-2. All three are moderate or minor; see below. |
| F | R3-ui-build | **Merged `17a360a`** (retry 1).
- The real UI runs on runtime data, with no fake data.
- Data gaps closed: UG-1 to UG-7, UG-10, UG-11, UG-14, UG-17, UG-22, UG-24; UG-12 partly. The rest are deferred, with placeholders.
- A-29 re-run passes: max 37.7 MiB, sync start ≤0.26 s.
- RC-3 is fixed in the app and verified by UIA.
- It also made disclosed changes to `tools/mem`.
- Independent check R3-check2 and integration review R3-review are running. |

## Incidents

- **I-1 (minor, COMMON rule):** R3-proto and R3-d21 each wrote source text
  with PowerShell string operations instead of the edit tool. Both workers
  disclosed it.
  - The orchestrator checked the index line endings (LF), BOM and NUL
    bytes, and re-ran both verify tiers: all clean.
  - It is not a PLAN §5 stop condition (that covers gc/reset/rewrite and
    foreign process stops).
  - Later packets restate the rule.

## Owner confirmation needed

- None open. D-21 replaced interim rule A: no Interface or build gating. R3-d21
  removes the gating code. R3D-3 is moot.
- Dependabot: per GitHub, alerts 1 to 4 are fixed (spikes deleted by
  R3-d21) and 5 and 6 (glib, Linux-only) are dismissed per D-21.

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
- R3-check findings:
  - **RC-1:** the prototype's S7 still had a fake build hold. Fixed in
    R3-ui-build `09be75e`, which removes the dead `DifferentClientBuild`
    and changes the scenario.
  - **RC-3:** Esc on a sheet restores focus to the window rather than to
    the control that opened the sheet. The prototype is not fixed; the
    requirement was passed to R3-ui-build for the real app.
  - **RC-2:** stale records. Corrected here. `R3-proto.md`'s "see final
    report" verify row is superseded by R3-check's independent offline and
    native PASS.

## Next actions

1. Merge each worker after the orchestrator re-runs offline verify.
2. Push and record usage.
3. Stop launching new work at ≥19,013 AIU cumulative.
