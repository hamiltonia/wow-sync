# Run 004: removal, settings sync, rollback, retire, decisions, notices, diagnostics

Status: **approved** (D-24). Active. Scope and ceiling are from D-23.

- Ceiling: 20,000 AIU, counted from the start of run-004.
  - Stop launching new work at ≥19,000.
  - Settling up to 22,000 is acceptable.
- Drafted 2026-11-17 by the Opus 5.5 orchestrator from
  [../run-003/REPORT.md](../run-003/REPORT.md) and D-21 to D-23.

## 1. Goals

1. **Removal (R-07, R-10, A-10):**
   - Rewrite `docs/design/removal.md` for D-21 using the D-23
     simplification: any removal wins, and conflicting removal records
     merge into one removal. This closes R3D-13.
   - Independent re-review, then implement.
   - Removal deletes active code and settings. Recovery preserves and
     restores both.
2. **Settings sync (A-11 to A-15):**
   - a settings master;
   - per-addon exceptions;
   - explicit peer pushes;
   - routine writes from non-master computers yield to master changes,
     with a recovery copy kept;
   - competing intentional choices are never merged;
   - three states per addon (current plus two previous).
3. **Rollback (A-18, A-19):** reject a bad release for the whole group.
   Offline peers cannot bring it back.
4. **Resolve decisions in the GUI (A-26):** decisions become actionable,
   not read-only.
   - UQ-1: "Review needed" headline promotion.
   - UQ-2: core exposes the typed scope of a pending decision.
5. **Retire a computer (A-45):** through the GUI. History is kept.
   Re-enrollment must be explicit. The settings master is never silently
   replaced.
6. **Notifications (A-27) and update notice (A-28):** OS notifications for
   events that need the user. The update notice depends on owner question
   P4-1.
7. **Diagnostics export (A-33):** produced only on request, shown for
   preview, redacted and bounded.
8. **Mac runtime (owner-assisted):**
   - a worker prepares a script plus instructions;
   - the owner runs the app and the A-29 memory check on the Mac;
   - the evidence is recorded as owner-run.

## 2. Non-goals

- No Windows installer (A-35). No real-data enrollment: the Q-24 marker
  gate stays.
- No Dropbox API (DA-2 to DA-4 deferred, D-23).
- No A-31 numeric budgets (deferred).
- No default telemetry (A-32). Diagnostics are a user-requested export
  only.
- Q-3 and Q-23 are not prerequisites. If they come back during the run,
  the orchestrator records them.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R4-removal-design | Design / opus-5.5 | Rewrite `removal.md` for D-21/D-23 and close R3D-13 |
| A | R4-settings-design | Design / opus-5.5 | New `docs/design/settings.md` and `rollback.md`: master, exceptions, peer push, yield and recovery, three states, rollback tombstones, offline resurrection guard |
| A | R4-ops-design | Design / opus-5.5 | Retire, decision resolution actions (UQ-1/UQ-2), notifications, update notice and diagnostics export (redaction rules, bounds, preview) |
| B | R4-design-review | Verification / gpt-6.1-sol | Independent review of all three designs. One fix round per design (retries) |
| B | — | **Owner barrier, only if needed** | Only if the designs raise new product questions. Otherwise phase C starts after the review passes |
| C1 | R4-core-contracts | Implementation / sonnet-5.5 | Foundation, merged alone. Shared kinds, DTOs (incl. `decision.rs`), journal, authority, hooks, fixtures; registers every later module in core/adapters/runtime `lib.rs` with typed stubs; sectioned `tests\required-cases.toml`. File set: [contracts.md §9.1](../../design/contracts.md) |
| C2 | R4-core-removal | Implementation / sonnet-5.5 | Core and runtime removal plus recovery, with tests. Files: contracts.md §9.2 |
| C2 | R4-core-settings | Implementation / sonnet-5.5 | Core and runtime settings sync plus rollback, with tests. Files: contracts.md §9.2 |
| C2 | R4-core-ops | Implementation / sonnet-5.5 | Core retire, decision actions, typed scope (UQ-2). Files: contracts.md §9.2 |
| D | R4-app | Implementation / sonnet-5.5 | GUI: resolve decisions, UQ-1 headline, retire, rollback, settings master/exceptions/push, removal and recovery screens. Files: contracts.md §9.3 |
| D | R4-notify-diag | Implementation / sonnet-5.5 | OS notifications, update notice (per P4-1), diagnostics export. Files: contracts.md §9.3; receives `Cargo.lock` after C1 |
| D | R4-mac-kit | Implementation / sonnet-5.5 | Mac build, run and memory script plus owner instructions |
| E | R4-check | Verification / gpt-6.1-sol | Independent drive through the GUI, negative and failure cases, CI evidence, memory re-run |
| E | R4-review | Integration / opus-5.5 | E-06/E-08 review; the orchestrator writes the report |

Phase C is staged (R4D-17/R4D-26). C1 merges alone and creates every
shared type, stub and module registration. The C2 packets then run in
parallel on pairwise-disjoint files. No step-2 or step-3 packet creates a
new file or edits a `lib.rs`. The only cross-owner file is
`tests\required-cases.toml`. Each packet appends only inside its own
section markers, and the merge gate checks that: outside the packet's
markers the file equals the base, the markers are unchanged, and every
added case id is new. Step 2 adds no dependency; `Cargo.lock` passes
serially from C1 to R4-notify-diag. Remaining integration conflicts are
resolved by the orchestrator when it merges, or by a retry.

Planned workers: 14 (C1 added after the design review). Up to 6 more for fixes and rechecks under the D-05
limits: 5 concurrent, 20 total, no nesting, 2 retries per task.

## 4. Budget estimate

| Item | AIU |
|---|---|
| Three designs + review + fix round | 3,000 |
| Core removal | 1,800 |
| Core settings + rollback | 3,000 |
| Core ops | 1,200 |
| App GUI | 2,500 |
| Notifications, update notice, diagnostics | 1,300 |
| Mac kit | 400 |
| Verification + integration review | 1,500 |
| Orchestrator | 2,500 |
| Fix chain and retry headroom | 2,000 |
| **Total** | **19,200** |

## 5. Stop conditions

- Stop at ≥19,000 AIU, at an owner barrier, or on any unapproved scope,
  credential or user-data need.
- **Stop all work** if a worker breaks a COMMON rule: gc/reset/rewrite, or
  stopping a process it did not launch. Report the incident.
- Phase C does not start until the design review passes.
- Every `main` push is followed by a CI watch (run-003 lesson).

## 6. Owner questions for this plan

| # | Question | Default if unanswered |
|---|---|---|
| P4-1 | Update notice (A-28): may the app check GitHub Releases on this public repo (an anonymous HTTPS GET, no credentials)? | No network check. Show only the installed version and a "check releases" link |
| P4-2 | May the notification work add a small, free, widely used OS notification crate (justified under E-06, `cargo deny` clean)? | Yes, if it is justified in the evidence |

## 7. Packet rules

Run-002 [COMMON.md](../run-002/packets/COMMON.md) applies unchanged, plus
the run-003 additions:
- no PowerShell file writes (I-1);
- stop processes only by a captured PID;
- synthetic data in `D:\WoWSyncLab\`.

Packets are written after approval.
