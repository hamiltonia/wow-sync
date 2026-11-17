# R3-mem (Verification, sonnet-5.5)

Read [COMMON.md](COMMON.md) first.

**Inputs:**
- SPEC R-14 and A-29.
- OWNER-DECISIONS D-18, which confirms the memory method and the V1 safety
  rule.
- The run-001 method:
  [S-mem.md](../../run-001/packets/S-mem.md) and
  [S-mem evidence](../../run-001/evidence/S-mem.md).
- The run-002 incident V1 in `docs/runs/run-002/HANDOFF.md`.

**Owned paths:**
- `tools/mem/**`, which is new: the harness script or a small Rust tool.
  If it is Rust, keep it out of the workspace members.
- `docs/runs/run-003/evidence/R3-mem.md`.
- Raw CSVs in `docs/runs/run-003/evidence/R3-mem/`, under 1 MB total.

Do not change product code.

## Goal

Re-measure A-29 on the **current product app**: a release build of
`wow-sync.exe` in `--dev` mode against a synthetic install and a temp provider
folder. Use a harness that is safe by construction.

## Harness safety (mandatory; V1 must not recur)

- Start the app yourself and keep the PID **and process creation time**
  returned at launch.
- A child process counts as app-owned only if all of these hold:
  - its ParentProcessId equals a tracked PID;
  - its creation time is **later** than that parent's creation time;
  - the parent is still the same process, i.e. the creation time matches.
- Re-validate on every sample. PID reuse must never cause a foreign process
  to be counted or stopped.
- Stop only the process you launched: first gracefully if the app supports
  it, then `Stop-Process -Id <literal or tracked validated PID>` on that
  process alone. Never stop children by traversal. If any app-owned child
  survives, report it as a finding instead of killing it.
- Include a self-test in the harness, run against a dummy parent/child you
  launch, showing that a PID-reused or older process is not counted.

## Measurement

- **Counter:** private working set (private resident), summed across the
  validated app-owned process set, sampled once per second. Also record
  Private Bytes and Working Set for context. Record GPU dedicated/shared
  memory separately, filtered by PID.
- **Scenarios** (release build; ≥3 runs each; Windows x64):
  - a. Started hidden or minimised to the tray if supported (otherwise the
    window is shown and then closed or hidden), idle for 2 min.
  - b. Window shown for 1 min with UI use (scroll and click any controls),
    then hidden, then idle for 2 min.
  - c. 20 show/hide cycles if the app supports it, then idle for 5 min, to
    check for growth. If show/hide is unsupported, record NOT RUN with the
    reason.
  - d. Sync start time: seconds from launch to the first published state or
    log line that shows readiness and sync start. SPEC requires ≤10 s.
- No forced trimming (`EmptyWorkingSet`) for the gated numbers.
- **Report:**
  - min, median and max values, plus the post-hide steady state, against
    100 MiB;
  - PASS/FAIL per scenario;
  - macOS BLOCKED.

## Done

Commit the harness, the evidence and the raw CSVs. Give the exact commands so
R3-check can re-run them.
