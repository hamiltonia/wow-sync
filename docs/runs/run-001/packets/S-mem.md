# Work packet S-mem: idle memory of a GPUI app

Role: implementation. Model: claude-sonnet-5.5 (D-04); independently checked
later by gpt-6.1-sol (S-check). Common rules: [S-common.md](S-common.md).
ID `S-mem`; spike dir `spikes/mem/`.

## Intent

SPEC §7 "idle cost"; R-14 / A-29 feasibility: background aggregate **private
resident memory ≤ 100 MiB** at idle, including after UI open/close cycles.
GPU memory is reported separately. Do not change or reinterpret the threshold.

## Work

1. Build a minimal GPUI app in `spikes/mem/` (independent of S-gpui, which runs
   in parallel; same crate/version choice is fine, record it) with a
   representative-ish window: a scrollable list of ~200 rows of text, a few
   buttons. It must be able to close its window while the process stays alive
   (hidden/background state), and reopen it. Provide a scripted cycle mode.
2. Measurement harness (PowerShell or a small Rust tool in `spikes/mem/`):
   sample once per second **Private Working Set** (private resident; e.g.
   `Get-Counter '\Process(<name>)\Working Set - Private'` or
   `QueryWorkingSetEx`/PSAPI), plus Private Bytes (commit) and full Working Set
   for context, and GPU process memory (e.g. `\GPU Process Memory(*)\Dedicated
   Usage` / `Shared Usage` counters filtered by PID) reported separately.
   Define "private resident" precisely in the evidence and why that counter.
3. Scenarios on release builds, each ≥ 3 runs:
   a. Started hidden (never showed window), idle 2 min.
   b. Window shown, idle 1 min, then hidden, idle 2 min.
   c. 20 show/hide cycles, then hidden idle 5 min (growth check).
   d. A plain non-GPUI Rust process baseline (tray-less, idle) for reference.
   Record min/median/max and the post-hide steady state; note if memory is
   returned to the OS after hide (and whether forcing trim, e.g.
   `EmptyWorkingSet`, is involved — if you use it, report both with and without
   and flag it as a design question, since trimming may be judged as gaming
   the metric).
4. Save raw CSVs under `docs/runs/run-001/evidence/S-mem/` (small). Report
   measured numbers vs 100 MiB honestly: PASS/FAIL per scenario on Win x64,
   macOS BLOCKED.
5. Options if over budget (e.g. destroy window/renderer on hide, separate
   lightweight background process + on-demand UI process — note SPEC forbids
   hidden helper offload; aggregate counts all app-owned processes).

## Owned writes

`spikes/mem/`, evidence files (S-common).

## Done when

Raw data, method, per-scenario numbers and a feasibility answer are committed.
