# Run 003 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-17, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Plan approved (D-20). Prototype approved and design answers given (D-21).
  See [PLAN.md](PLAN.md).
- Ledger: [USAGE.md](USAGE.md).
- Owner checks pending: [OWNER-CHECKS.md](OWNER-CHECKS.md).

| Phase | Workers | Status | Details |
|---|---|---|---|
| A | R3-ui-design, R3-infra, R3-compat, R3-removal-design | Done | All merged |
| B | R3-design-review (+ 2 rechecks) | Done, removal open | B-1 |
| C | R3-proto, R3-mem | Done | C-1, C-2 |
| D | Owner barrier | Done | D-21 (2026-11-17) |
| E | R3-d21, R3-dropbox-api, R3-check, R3-review, R3-check2 | Review FAIL, fixes pending | E-1 to E-4 |
| F | R3-ui-build (+ retry 1) | Merged `17a360a` | F-1 |

- **B-1:** review FAIL `917bc05`; recheck `09b2673`, UI PASS.
  - Q-21 R3D-3 is moot under D-21.
  - Removal: R3D-13 is still open after recheck2 `07fe195`, and retries
    are exhausted. It blocks any run that implements removal.
- **C-1, R3-mem:** A-29 PASS on Windows x64 (worst 19.6 MiB of 100; sync
  start ≤0.26 s; macOS BLOCKED). Merged `8e82673`.
- **C-2, R3-proto:** screens P1 to P14 and scenarios S1 to S16 are
  implemented on fake data.
  - The interaction logs cover only part of the scenario transitions
    (RC-2).
  - No new dependencies.
  - Launcher: `D:\WoWSyncDemo\Start-Prototype.cmd`.
  - Unverified: Ctrl shortcuts, OS DPI, live theme toggle, OS toasts, IME
    and shutdown.
- **E-1, R3-d21:** gating removed, spikes deleted, negative control
  recorded.
- **E-2, R3-dropbox-api:** keep the desktop folder for v1; a read-only API
  hybrid is an option later. Owner questions DA-1 to DA-4 are in
  `docs/design/dropbox-api.md`.
- **E-3, R3-check:** D-21, memory (34.9 MiB) and CI PASS. Findings RC-1 to
  RC-3, below.
- **E-4, R3-review:** FAIL with 7 major and 5 minor findings, no blockers.
  - The orchestrator fixed RV-10 (restored the D-20 heading) and RV-11
    (records).
  - RV-1 to RV-9 and RV-12 go to the fix packet R3-fix.
- **E-5, R3-check2:**
  - Negative controls, memory (peak 37.75 MiB) and both verify tiers PASS.
  - Real screen values FAIL on RC2-1: an invented "next in 4 min".
  - Tray clicks BLOCKED (no physical click was sent).
  - RC2-1 to RC2-4 go to R3-fix.
  - The orchestrator stopped the stranded synthetic stand-in (a renamed
    `ping -t 127.0.0.1` in the R3-check2 scratch lab). It was identified by
    its exact worktree executable path and command line and stopped by its
    literal PID. No other process was touched.
- **F-1, R3-ui-build:**
  - The real UI runs on runtime data.
  - Data gaps closed: UG-1 to UG-7, UG-10, UG-11, UG-14, UG-17, UG-22 and
    UG-24; UG-12 partly.
  - A-29 max 37.7 MiB.
  - RC-3 is fixed in the app.
  - It also made disclosed changes to `tools/mem`.

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
- Dependabot, per GitHub on 2026-11-17: alerts 1 to 4 are fixed (the spikes
  deleted by R3-d21 held them); alerts 5 and 6 (glib, Linux-only) are
  dismissed per D-21.

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

## Owner question: app icon (OQ-ICON, 2026-11-17)

- The owner provided app art in a local folder outside the repo: a 1254 px
  PNG and an ICO with sizes from 16 to 256 px.
- The art contains Blizzard's WoW "W" crest and the Dropbox logo. The repo is
  **public**. Committing the art would publish third-party trademarks, which
  needs owner approval under AGENTS.md.
- The owner was not available, so the orchestrator chose the reversible
  option: the art is **not committed**. R3-fix goal 12 optionally lets the app
  use it as a local-only runtime override (gitignored).
- Owner decision needed:
  - commit the art as is;
  - keep it local only;
  - or provide a version without the third-party logos, which could then be
    committed and embedded as the exe icon.

## Next actions

Run-003 is **closed** at 17,029.2 AIU cumulative (7,515.8 for run-003),
18 of 20 workers used. See [REPORT.md](REPORT.md).

- **Fix chain:** R3-fix (`042b44c`), R3-macfix (`0e86e74`, macOS CI red
  from `ce15e24` until then), R3-fix2 (`8afde5d`) and R3-fix2b (`a513fc8`).
- **Final independent check:** R3-check4b (`00a9837`) PASS. Every RV, RC2,
  RC3 and RC4 finding is closed, except the following:
  - the tray left click on the real icon is BLOCKED. It is an owner-manual
    check.
  - goal 12 (the local icon override) was not done; it waits for the
    OQ-ICON decision.
- **The next run needs owner input first:**
  - OQ-ICON, DA-1 to DA-4, UQ-1 and UQ-2;
  - the Q-3 and Q-23 real-PC checks;
  - how to re-review R3D-13 before removal;
  - an approved run-004 plan and credit ceiling.
