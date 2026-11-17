# R3-mem evidence: A-29 memory re-measure of the current product app

Run 003, worker R3-mem. Branch `run003/R3-mem`. Links: SPEC R-14, ACCEPTANCE A-29,
OWNER-DECISIONS D-18. macOS: **BLOCKED** (no Mac; CI only).

Environment: Windows 11 x64 dev PC, release build `target\release\wow-sync.exe` built from this
branch (`cargo build --release -p wow-sync-app`, GPUI at the pinned Zed `v1.28.1`), launched with
`--dev` against a synthetic install and temp provider/state folders under `%TEMP%\wowsync-mem`.
No WoW/Battle.net/Dropbox involvement. Product code unchanged.

## What was built

- `tools/mem/MemLib.ps1`: sampling and process-identity library.
- `tools/mem/run.ps1`: scenario driver (a, b, c, sync start).
- `tools/mem/selftest.ps1`: safety self-test.
- Raw data: `docs/runs/run-003/evidence/R3-mem/` (`<scenario>-run<N>.csv`, `run-log.txt`, `summary-abc-*.txt`, `selftest.txt`).

## Counter

Private working set (`Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate`, i.e. "Working
Set - Private"): physical pages in the working set that are not shareable, summed over the
validated app-owned set, sampled at 1 Hz. Private Bytes (commit) and Working Set recorded for
context. GPU memory (`GPUProcessMemory` Dedicated/Shared Usage, filtered by `pid_<PID>_*`) is
recorded separately and not added. No `EmptyWorkingSet`/trimming anywhere.

## Harness safety (V1 must not recur)

- Root = the process the harness launched; its PID **and creation time** are recorded at launch.
- Owned set is recomputed from a fresh process snapshot every sample: root (same PID and same
  creation time) plus descendants whose ParentProcessId names an owned process **and** whose
  creation time is later than that parent's. A reused root PID yields an empty set (the run aborts).
- Only the root is ever stopped (`Stop-Launched`: re-validates PID + creation time, then
  `Stop-Process -Id <tracked PID>`). Children are never stopped; survivors would be reported as findings.
  No process is found by name. The app has no graceful-quit CLI (Quit is a tray menu item), so the
  root is stopped with `Stop-Process` after each run.
- Self-test (`selftest.txt`): **SELFTEST PASS** (13 checks). Synthetic tables: valid child and
  grandchild counted; an older process naming the root as parent not counted; a grandchild older
  than its parent not counted; an unrelated tree not counted; reused root PID (different creation
  time) and vanished root both give an empty set. Live dummy `cmd` parent with a real `ping` child:
  both counted; an independently launched `ping` not counted; a record with a wrong creation time owns
  nothing and `Stop-Launched` refuses to stop; the validated parent is stopped; the independent
  process is untouched. (The dummy's child survived its parent, as expected, and was reported, not stopped.)
- During the real runs: max owned processes = 1 in every sample of every run; findings = 0 (no
  surviving children, no window-state anomalies, 20/20 show cycles confirmed visible).

## Method and scenarios

Each run uses a fresh synthetic lab (3 synthetic add-ons) and is enrolled first through the GUI
(posted Space/Tab/Enter keys; UIA Invoke/Toggle did not change app state on this PC, posted keys
do), then the enrolled app is stopped and relaunched for measurement.

- **a.** `--start-hidden` (tray only, never shown), idle 120 s.
- **b.** Window shown 60 s with input (posted mouse-wheel up/down each second, Enter on the
  focused control every 5 s), then hidden (WM_CLOSE, the app's hide-to-tray) and idle 120 s.
- **c.** Started hidden; 20 show/hide cycles (show = second instance signals the first; hide =
  WM_CLOSE; 3 s per cycle), each confirmed visible, then idle 300 s.
- **d.** Sync start = seconds from launch (before `Start-Process`) to the first change in the
  state/provider files, polled every 50 ms.

Steady = median of the last 60 s of the final idle phase. Verdict uses the strict whole-run max
(includes shown-window peaks and start-up) against 100 MiB.

## Results (private working set, MiB, release, Win x64, 3 runs each)

| Scenario | Steady per run | Min / median / max steady | Whole-run max | Verdict (limit 100) |
|---|---|---|---|---|
| a. hidden, idle 120 s | 11.65, 11.83, 11.61 | 11.61 / 11.65 / 11.83 | 14.68 | **PASS** |
| b. shown 60 s, hidden, idle 120 s | 15.72, 16.10, 15.90 | 15.72 / 15.90 / 16.10 | 19.44 (shown peak) | **PASS** |
| c. 20 show/hide cycles, idle 300 s | 15.87, 15.73, 15.73 | 15.73 / 15.73 / 15.87 | 19.63 | **PASS** |
| d. sync start (9 launches) | n/a | 0.18 / 0.22 / 0.26 s | n/a | **PASS** (limit 10 s) |
| macOS | n/a | n/a | n/a | **BLOCKED** |

Context (median of runs): Private Bytes a 57.5, b 76.3, c 76.5 MiB; Working Set 44.6, 58.4, 59.1
MiB. GPU memory (separate): dedicated 7.8 (a), 17.5 (b, c) MiB; shared 0.7 (a), 1.1 (b), 0.25 (c) MiB.

Post-hide steady state: b 15.7 to 16.1 MiB, about 4 MiB above never-shown (a), so memory is only
partly returned after hide. Growth check (c): the first and last 30 s means of the 300 s idle
differ by at most 0.15 MiB in every run (15.92 to 15.90, 15.74 to 15.77, 15.73 to 15.79); after
the 20 cycles the cycle-phase mean falls from about 16.3 to 15.9 MiB. No growth observed.

## Limits and open issues

- Sync start is the first change to the app's state files after launch (`state\admission.json`,
  rewritten by each sync cycle; it is again rewritten at the 5 s interval). It shows the sync
  thread's first cycle began; the app has no explicit "sync started" log line.
- UI use in b relies on posted wheel/Enter messages; their visible effect was not checked, so the
  shown-window memory may be slightly lower than with heavy real interaction. Peaks stayed below 20 MiB.
- Synthetic install of three tiny add-ons; real installs, longer runs and macOS are not measured.
- One PC and GPU/driver; 1 Hz sampling; WMI counters.
- Tiers: no product tests changed, so there are no A-38 negative controls for this packet.

## Commands to re-run (R3-check)

From `D:\GitHub\wow-sync.worktrees\R3-mem` (or any checkout):

```
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build --release -p wow-sync-app
powershell -NoProfile -File tools\mem\selftest.ps1
powershell -NoProfile -File tools\mem\run.ps1 -Scenarios a,b,c -Runs 3 -OutDir <scratch dir>
```

Run time about 35 minutes; the PC must be idle, interactive and unlocked. Smoke test:
add `-Scale 0.1` (not for gated numbers).
