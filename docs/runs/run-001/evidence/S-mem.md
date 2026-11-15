# S-mem evidence: idle memory of a GPUI app (R-14 / A-29)

Run 001, Phase 2, worker S-mem. Environment: Windows 11 x64 Insider build 26300
dev PC (not official support evidence), Rust 1.100.0, release build,
`gpui = "=0.2.2"` (crates.io), D3D/blade renderer as shipped. macOS runtime:
**BLOCKED** (no Mac).

## SPEC §7 question and answer

Question: can a GPUI-based background app hold aggregate private resident
memory at or below 100 MiB at idle, including after UI open/close cycles?

**Answer: feasible on Windows x64 (high confidence for this synthetic UI;
medium for the product).** Measured steady-state private working set was
13 to 27 MiB in every scenario, i.e. roughly 4x headroom. Confidence is
limited because the UI is a 200-row text list, not the real product UI,
and macOS is unmeasured. What would change it: real UI content (fonts, images,
SVG, text-shaping caches), a tray/updater/sync engine in the same process,
or macOS behaviour. Re-measure with the real app.

## Definition of "private resident"

Counter used: `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate`
(perf counter "Working Set - Private") = physical pages currently in the
process working set that are not shareable with other processes (excludes
shared DLL/image/section pages). Chosen because the requirement says
*private resident*, not commit. Also recorded for context: `PrivateBytes`
(commit, includes paged-out/untouched pages), `WorkingSet` (includes shared
pages), and per-PID GPU memory
(`Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory`,
`DedicatedUsage` and `SharedUsage`, reported **separately**, not added to the
CPU-side number). Only the single app PID is sampled: the GPUI app spawns no
child processes here (aggregate = one process). The measured private WS
excludes shared system DLL pages, which a stricter "RSS" reading would include
(see the Working Set column below, 44 to 67 MiB, also under 100 MiB).

## Method

- App: `spikes/mem/src/main.rs` (`gpui_mem.exe`): window with 3 buttons and a
  scrollable list of 200 text rows; modes `hidden`, `show`,
  `show-hide SECS`, `cycles N` (1.5 s shown/1.5 s closed).
- Reference: `spikes/mem/src/bin/plain_idle.rs` (sleeps forever).
- Harness: `spikes/mem/measure.ps1` (1 Hz sampling via CIM, CSV per run);
  `spikes/mem/run-all.ps1` runs everything; `spikes/mem/summarize.ps1`
  produces `summary.txt`. Last column of each CSV records how many
  cargo/rustc processes ran on the PC at that second (other workers were
  building). Runs 3 of each scenario ran with zero concurrent builds
  (buildSamples=0); builds did not change steady-state values (runs agree to
  < 1 MiB) but inflate only the startup phase.
- Raw data: `docs/runs/run-001/evidence/S-mem/*.csv`, `*.log`, `summary.txt`.
- Commands (PowerShell, from `spikes\mem`, after
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`):
  `cargo build --release` then `.\run-all.ps1` then `.\summarize.ps1`.

## Finding: GPUI 0.2.2 quits when its last window closes (Windows)

`platform/windows/platform.rs` `close_one_window` returns true when no windows
remain and the handler calls `PostQuitMessage(0)`. A first test (show-hide
with no other window) confirmed the process **exited** at the close. A
background-resident design therefore needs either an invisible 1x1 anchor
window (`show: false`; enabled here with env `WOWSYNC_ANCHOR=1`) or a
patched/forked GPUI. A process that never opened a window (scenario a) is
unaffected. This is a design constraint, and the anchor window keeps a
renderer alive, which the numbers below include.

## Results (private working set, MiB; 3 runs each, release, Win x64)

"Steady" = last 60 s of the run (hidden idle for a, b, c). min/median/max
over the 3 runs' steady medians; whole-run max includes transient show peaks.

| Scenario | Steady per run (MiB) | Min/Median/Max steady | Whole-run max | Verdict vs 100 MiB |
|---|---|---|---|---|
| a. started hidden, no window ever, idle 120 s | 13.1, 13.0, 13.1 | 13.0 / 13.1 / 13.1 | 15.0 | **PASS** |
| a'. started hidden with 1x1 anchor window, idle 120 s | 17.6, 17.7, 17.6 | 17.6 / 17.6 / 17.7 | 17.9 | **PASS** |
| b. shown 60 s, then hidden (anchor), idle 120 s | 22.3, 20.9, 21.6 | 20.9 / 21.6 / 22.3 | 24.4 | **PASS** (shown peak 24.4) |
| c. 20 show/hide cycles, then hidden idle 5 min | 26.8, 27.6, 27.1 | 26.8 / 27.1 / 27.6 | 30.8 | **PASS** |
| d. plain Rust process idle 120 s (reference) | 0.3 to 0.4 | 0.3 / 0.3 / 0.4 | 0.4 | reference |

Context (steady, averaged): Private Bytes (commit) 55 to 82 MiB (a: 55;
a': 61; b: 72 to 74; c: 79 to 82); Working Set 44 (a), 53 (a'), 60 to 62 (b),
67 (c) MiB. GPU memory, reported separately: dedicated 7.8 MiB (a, a'),
12.6 MiB (b), 13.6 MiB (c); shared 0.2 to 1.0 MiB.
macOS: **BLOCKED**.

Observations:

- Memory after hide is **partially** returned: b drops from the shown peak
  (~24 MiB) to ~21.6 MiB; it stays ~8 MiB above never-shown + anchor (17.6).
  GPUI/allocator caches are not fully released on hide.
- Growth check (c): steady value after 20 cycles (26.8 to 27.6) was flat from
  t=100 s to t=360 s (spread < 0.1 MiB, no upward trend), so **no leak was
  observed** in 5 min. Residual after cycles is ~5 MiB more than after one
  show/hide (b), i.e. a one-time step, not per-cycle linear growth in this
  data (only 20 cycles; 5 min; short for slow leaks).
- No `EmptyWorkingSet` was used for the headline
  numbers above.

### Forced trim (design question, flagged)

One run of scenario b with `EmptyWorkingSet` called every second
(`b-trim-run1.csv`, `-Trim`): private WS read **0.1 MiB** (steady, 0.1 to 1.2),
Working Set 0, while Private Bytes stayed 73 MiB. This shows OS-level trimming
can make the metric trivially small, since pages are only paged to the
standby list and return on touch. It is **not** used for any PASS claim; judging
it as gaming the metric is an owner/design decision. The untrimmed numbers
already pass.

## Method limits and risks

- Synthetic, simple UI; no tray icon, no sync engine, no file watching,
  no fonts beyond the system default, no images. Real app will add memory.
- One PC, one GPU/driver; 1 Hz sampling; the whole-run min value in
  `summary.txt` is the process start-up and is not meaningful.
- GPU counters are sampled per PID and only report the final value in the
  summary; the CSV has the full series.
- Concurrent builds by other workers overlapped runs 1 and 2 (see CSV last
  column and `buildSamples` in `summary.txt`); run 3 of each scenario was
  clean. Steady values agreed across runs.
- Only one process measured; adding a separate helper/service process would
  add to the aggregate, and SPEC forbids hidden helper offload.
- Plain-process baseline d is for scale only.

## Options / recommendations

1. Within budget as is: keep a single process; steady-state headroom is
   about 70 MiB. Budget the real UI with a re-measurement at the first
   product milestone and track against 100 MiB.
2. The quit-on-last-window behaviour needs a decision: invisible anchor
   window (costs ~4.5 MiB private WS here, a renderer stays alive), a GPUI
   patch/fork, or a git-pinned newer GPUI whose Windows platform supports a
   keep-alive mode (not investigated).
3. If memory grows with the real UI, options are destroying the renderer
   window on hide (already approximated here) and trimming; trimming is a
   design decision and is not needed now.
4. Verify macOS (BLOCKED) once a Mac is available; unified-memory GPU use
   may count differently.

## Spike code pointers

`spikes/mem/Cargo.toml`, `src/main.rs`, `src/bin/plain_idle.rs`,
`measure.ps1`, `run-all.ps1`, `summarize.ps1`, `Cargo.lock`.
