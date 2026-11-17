# Run 003 report: real UI on runtime data

Status: **closed** (2026-11-17) after the planned phases A to F and the fix
chain.

- Usage: 17,029.2 AIU cumulative; 7,515.8 for run-003.
- Workers: 18 of 20 used.
- The final independent check, R3-check4b, passed.
- CI is green on Windows and macOS.

- Orchestrator: claude-opus-5.5, session `961f3baa`.
- Plan: [PLAN.md](PLAN.md) (D-20). Owner barrier: D-21.
- Usage: [USAGE.md](USAGE.md). State: [HANDOFF.md](HANDOFF.md).

## 1. Outcome

The Windows app now has a **real GPUI interface on runtime data**:

- setup wizard;
- overview;
- add-on list and add-on detail;
- decisions (read-only);
- computers;
- activity;
- settings;
- pause, resume and quit-when-safe;
- tray and single instance.

It still uses **synthetic data only** (the Q-24 marker gate) and the folder
provider. It is not a release.

The owner approved the clickable prototype at the phase D barrier. The
owner's answers there (D-21) removed all Interface/build gating: only the
variant and account/character boundaries limit sync. They also settled the
removal rules ("removed means removed everywhere").

## 2. What was built

| Area | Result | Evidence |
|---|---|---|
| UI design | Screens, wireframes, interaction rules, UI gaps UG-1 to UG-26; independently reviewed (UI PASS on recheck) | [R3-ui-design](evidence/R3-ui-design.md), [R3-design-review](evidence/R3-design-review.md) |
| Prototype | P1 to P14 and S1 to S16 on fake data; owner-approved (D-21). Frozen as the reference (RV-9) | [R3-proto](evidence/R3-proto.md) |
| Infrastructure | Windows native verify tier in CI | [R3-infra](evidence/R3-infra.md) |
| Compatibility | Interface/build gating built, then removed per D-21 (`R3-d21`); variant never-match kept | [R3-compat](evidence/R3-compat.md), [R3-d21](evidence/R3-d21.md) |
| Removal design | Design written; **R3D-13 open** and must be re-reviewed against D-21 before implementation | [R3-removal-design](evidence/R3-removal-design.md) |
| Memory (A-29) | Windows x64 PASS; app peak about 37 MiB, sync start ≤0.26 s; macOS not run | [R3-mem](evidence/R3-mem.md), [R3-check3](evidence/R3-check3.md) |
| Real UI | Runtime snapshot, pause, quit, activity and discovery APIs; `core::view` next actions and explanations rendered by the app | [R3-ui-build](evidence/R3-ui-build.md), [R3-fix](evidence/R3-fix.md) |
| Dropbox API spike | Keep the desktop folder for v1; a read-only API hybrid is a later option; DA-1 to DA-4 for the owner | [R3-dropbox-api](evidence/R3-dropbox-api.md), [dropbox-api.md](../../design/dropbox-api.md) |
| Verification | R3-check, R3-review (FAIL, 12 findings), R3-check2, R3-check3, R3-check4, then R3-check4b (PASS). Findings were fixed by R3-fix, R3-macfix, R3-fix2 and R3-fix2b. Interruption wording now follows the recorded cause only | [R3-check4b](evidence/R3-check4b.md) |

## 3. Not done or blocked

- **Removal (A-10):** design only, blocked on R3D-13.
- **Not started:** settings sync (A-11 to A-15), rollback (A-18, A-19),
  retire (A-45), notifications (A-27), update notice (A-28), telemetry and
  diagnostics (A-32, A-33), installers (A-35).
- **Real Dropbox (A-34):** not run; it needs the owner's prepared files
  (Q-23).
- **macOS and ARM64 runtime:** NOT RUN. CI builds and tests them.
- **Tray mouse click:** BLOCKED. No icon could be targeted unambiguously.
  Code is fixed and the second-instance path passes.
- **Decisions:** read-only in the GUI; there is no resolve action yet.

## 4. Incidents

- **I-1:** two workers (R3-proto, R3-d21) wrote files with PowerShell
  despite the rule. Line endings were checked and found OK, and the rule
  is restated in every packet.
- **macOS CI was red from `ce15e24` to `0e86e74`:** a test used a Windows
  path separator. Neither R3-review nor R3-check2 caught it; the
  orchestrator found it while watching CI. Lesson: the merge routine must
  watch CI on every `main` push, not only at the end.
- **RV-10:** an orchestrator commit (`f92a61a`) deleted the D-20 heading;
  restored in `ee244ef`.
- A synthetic stand-in process left by R3-check2 was stopped by the
  orchestrator. It was identified by its exact worktree path and command
  line and stopped by PID.

## 5. Owner decisions pending

- **OQ-ICON:** the provided app art includes third-party logos, and the
  repo is public. See HANDOFF.
- **DA-1 to DA-4:** the Dropbox API questions.
- **UQ-1 and UQ-2:** in `docs/design/ui/questions.md`.
- **Q-3 and Q-23:** real-PC checks ([OWNER-CHECKS.md](OWNER-CHECKS.md)).
- **R3D-13:** the removal re-review approach.
- **A-31:** numeric budgets.

## 6. Remaining work

See the remaining-work table in [R3-review](evidence/R3-review.md). In order:

1. Removal, after the R3D-13 re-review.
2. Settings sync and rollback.
3. Real Dropbox.
4. macOS runtime and memory.
5. Installers with platform smoke tests.
6. Notifications, diagnostics and telemetry.
