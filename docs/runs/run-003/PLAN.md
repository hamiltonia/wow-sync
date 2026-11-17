# Run 003: UI direction, verification debt, removal design

Status: **approved** (D-20). Active.

- Ceiling: 10,000 AIU counted from run-003 start.
  - Stop launching new work at ≥9,500.
  - Settling up to 11,000 is acceptable.
- Drafted 2026-11-16 by the Opus 5.5 orchestrator from
  [../run-002/REPORT.md](../run-002/REPORT.md) and D-18/D-19.
- Owner checks: [OWNER-CHECKS.md](OWNER-CHECKS.md).

## 1. Goals

1. **UI (R-12, R-13, A-26, A-42):** information architecture, screens and
   states, and a clickable GPUI prototype on synthetic data.
   - Style: a roomier dashboard in the spirit of Syncthing, following the
     system light/dark theme.
   - The **owner approves the prototype** before any build-out.
2. **Verification debt:**
   - The memory gate (A-29) re-measured over ≥3 runs with a
     creation-time-validated harness that stops only processes it launched.
   - The xtask `native` tier made executable (Q-17).
   - The flaky Windows CI adapters test found and fixed.
   - Dependabot alerts triaged and fixed.
3. **Q-21 rule (D-18):** a candidate from a different client build installs
   automatically when its `## Interface:` number is supported by the
   receiving client. Otherwise it needs a decision. Implemented in core,
   with tests.
4. **Addon uninstall/removal design (R-07, R-10):** how removals propagate,
   how recovery preserves and restores code and settings, and how this works
   with three-state history. Design only, reviewed.

## 2. Non-goals

- No UI build-out beyond the prototype. Build-out is a conditional phase F
  (below) that runs only after the owner approves the prototype and if the
  budget allows.
- No settings sync, rollback or real-data enrollment. The Q-24 marker gate
  stays.
- No unicode name support (Q-14 deferred).
- No macOS or ARM64 runtime claims.
- Real Dropbox provider tests run only after the owner completes Check 2.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R3-ui-design | Design / opus-5.5 | Screen inventory (status dashboard, computers, addons, details/why, setup and reconfirm, needs-help, settings placeholders, tray menu), state matrix, keyboard/focus, light/dark tokens, wireframes (SVG/markdown) in `docs/design/ui/` |
| A | R3-infra | Implementation / sonnet-5.5 | `native` tier in xtask; flaky-test root cause; Dependabot triage and fix; CI kept green |
| A | R3-compat | Implementation / sonnet-5.5 | Q-21 interface rule: design note, core and tests, A-38 controls |
| A | R3-removal-design | Design / opus-5.5 | Uninstall/removal design in `docs/design/` |
| B | R3-design-review | Verification / gpt-6.1-sol | Review of the UI design, the removal design and the Q-21 change |
| C | R3-proto | Implementation / sonnet-5.5 | Clickable GPUI prototype on synthetic/fake state covering every designed screen and state; screenshots in light and dark |
| C | R3-mem | Verification / sonnet-5.5 | Safe memory harness, ≥3 runs on the current app |
| D | — | **Owner barrier** | Owner reviews the prototype (demo launcher plus screenshots) and approves or requests changes |
| E | R3-check | Verification / gpt-6.1-sol | Independent checks: drive the prototype by keyboard and UIA, negative controls on R3-compat, CI evidence |
| E | R3-review | Integration / opus-5.5 | E-06/E-08 review; the orchestrator writes the report |
| F (conditional) | R3-ui-build | Implementation / sonnet-5.5 | Replace the slice shell with the approved design wired to the runtime; needs prototype approval and budget headroom |

Planned workers: 10 (+ retries, + phase F) under the D-05 limits: 5
concurrent, 20 total per run, no nesting, 2 retries per task.

## 4. Budget estimate

| Item | AIU |
|---|---|
| UI design + review | 900 |
| Removal design + review | 700 |
| Infra (native tier, flaky test, Dependabot) | 600 |
| Q-21 compat | 400 |
| Prototype (+ one revision round) | 1,500 |
| Memory gate | 300 |
| Verification + integration review | 900 |
| Orchestrator | 2,000 |
| Retry headroom | 700 |
| **Subtotal** | **8,000** |
| Phase F (only if approved and the budget allows) | ≈1,500 within the ceiling |

## 5. Stop conditions

- Stop at ≥9,500 AIU, at an owner barrier, or on any unapproved scope,
  credential or user-data need.
- **Stop all work** if a worker breaks a COMMON rule: gc/reset/rewrite, or
  stopping a process it did not launch. Report the incident.
- If a prototype limitation stems from GPUI, report the options; do not
  switch toolkit (D-18 option A).
- Phase F does not start without a recorded owner approval of the prototype.

## 6. Packet rules

Run-002 [COMMON.md](../run-002/packets/COMMON.md) applies unchanged. Packets
for run-003 are written after approval.
