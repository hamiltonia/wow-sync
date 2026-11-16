# Run 002: Design and first vertical slice

Status: **active under D-17** (owner: continue until the credit budget is
used, then review remaining work). Budget: the remaining D-16 allowance,
shared with run-001 (stop launching at ≥9,500 AIU cumulative across run-001
and run-002; settle up to 11,000). Prerequisite decisions in §6 are
**provisional orchestrator defaults** pending owner review (D-17).
Drafted 2026-11-15 by the Opus 5.5 orchestrator from
[../run-001/REPORT.md](../run-001/REPORT.md) §4–5.
State: [HANDOFF.md](HANDOFF.md). Ledger: [../run-001/USAGE.md](../run-001/USAGE.md)
(continues for run-002).

## 1. Goal

Turn run-001 feasibility evidence into a core design and one thin,
independently verified Windows vertical slice on synthetic data: enroll a
computer, identify its variant, inventory addons, publish an addon-code state
to the test Dropbox folder and apply it to a second synthetic install.

Targets (initial evidence only, no release claims): R-02, R-03, R-04, R-05,
R-11, R-12; A-39/A-43 for the new domain code; A-29 re-check on the chosen
GPUI pin.

## 2. Non-goals

- No settings sync, conflicts UI, rollback/rejection, telemetry or packaging.
- No real gaming data beyond D-14; Dropbox only `WoW Sync Test` (D-08).
  Slice tests use synthetic installs under a temp/test root, not the dev
  PC's `_forever_` folder, unless a packet says otherwise and snapshots first.
- No macOS runtime claims until the Mac is available.
- Reversible defaults only; nothing in §6 marked "held" is done.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R2-pin | Implementation / sonnet-5.5 | Re-run lifecycle + memory (aggregate incl. child processes) + 4-target CI build on the §6.2 GPUI pin; propose `deny.toml` changes per §6.4 |
| A | R2-design | Design / opus-5.5 | Domain model, identities, ordering, `.toc` rule, publication format, apply staging (R-11), module boundaries |
| B | R2-design-review | Verification / gpt-6.1-sol | Independent review of R2-design against SPEC/ACCEPTANCE |
| C | R2-core | Implementation / sonnet-5.5 | Platform-free domain core with behavioral tests and required-case entries |
| C | R2-adapters | Implementation / sonnet-5.5 | Windows filesystem/process adapters; Dropbox-folder publication (staging, atomic rename, re-check) |
| D | R2-slice | Implementation / sonnet-5.5 | Minimal GPUI shell wiring the slice (enroll, status, apply) without terminal use |
| E | R2-check | Verification / gpt-6.1-sol | Clean-clone verification, negative controls, two synthetic installs end-to-end, memory re-check |
| E | R2-review | Integration / opus-5.5 | E-06/E-08 review; then orchestrator report |

Planned workers: 8 (+ retries) under D-05 values (5 concurrent, 20 total per
run, no nesting, 2 retries per task). Phase order may compress if budget
requires; unfinished phases are reported, not claimed.

## 4. Budget

| Item | Estimate (AIU) |
|---|---|
| R2-pin | 250 |
| Design + review | 600 |
| Core, adapters, slice | 1,500 |
| Verification | 500 |
| Orchestrator | 1,200 |
| Retry headroom | 450 |
| **Estimate** | **4,500** (cap: remaining D-16 allowance) |

## 5. Stop conditions

As run-001 PLAN §7, plus: stop slice work if the chosen GPUI pin fails the
memory or lifecycle re-check (R2-pin) and report options.

## 6. Prerequisite decisions (provisional under D-17)

| # | Decision | Applied default | Status |
|---|---|---|---|
| 1 | Commit S-forever report | — | **Held** for owner review |
| 2 | GPUI source | Pin a Zed release tag with AccessKit | Provisional |
| 3 | D-12 screen-reader policy | Windows Narrator goal; unverified items and VoiceOver listed as limitations | Provisional; D-12 stays owner-open |
| 4 | `deny.toml` policy | Allow BSD-2-Clause, Zlib, MPL-2.0, CC0-1.0, NCSA; warn on unmaintained; allow pinned zed-industries git deps | Provisional |
| 5 | Dependabot alerts (spikes only) | — | **Held** (left open) |
| 6 | Memory gate method | Sum "Working Set - Private" over all app-owned processes; trimming never passes | Provisional |
| 7 | R-11 residual launch race | Design minimizes (staging, atomic rename, re-check, recovery); residual window reported, not hidden | Provisional |
| 8 | `git gc --prune=now` | — | **Held** |
| 9 | Run-002 plan | This plan | Active under D-17 |
