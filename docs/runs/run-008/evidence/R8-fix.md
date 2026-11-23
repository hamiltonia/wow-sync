# R8-fix evidence

## Root cause
`Cmd::Inspect` ran `SliceService::inspect()` inline on the single backend
command thread (backend.rs). It SHA-256 hashes every add-on file, with two tree
snapshots per folder. `Cmd::QuitWhenSafe` sits on the same queue and the app calls
`cx.quit()` only after the backend publishes `quit_ready`. While a check runs,
Quit is therefore never read and nothing reports progress or allows cancel. The
RYZENFISH log shows `_forever_` accepted at 21:23:57, no `inspected` line, and
tray clicks at :05/:08/:10 doing nothing. The log ends at 21:24:21, so about 25 s.
Also, `merge_preview` applied a result to the selected row rather than its target,
and the auto-selected first row was the refused `_classic_beta_`.

## Reproduction (honest)
Isolated state D:\WoWSyncLab\R8-fix, read-only on the dev PC's `_forever_`
(33,141 files, 102 add-ons) via a temporary release-build harness calling
`inspect()`: 6.1-7.1 s warm (Recognized, eligible=102). The multi-minute RYZENFISH
stall was NOT reproduced; the dev PC is fast and warm. No deadlock or quadratic
behaviour was found. The root cause is a structural block of the command queue
for the full duration of the hash, so any slower or cold disk, or antivirus scanning,
makes Quit unresponsive for that long. The GUI itself was not driven by
automation; the release app was not run through a Quit-mid-check sequence.

## Before / after
- Before: Quit queued behind the whole inspect (6-7 s here, unbounded on a slow disk).
- After: the check runs on its own thread, and `QuitWhenSafe` cancels it first. Test
  `quit_does_not_wait_behind_a_running_read_only_inspection` holds an inspection and asserts
  quit is ready promptly (well under 5 s). Timings from the real GUI not measured.

## Changes
Cancellable scan (`ScanCtl`, per-file cancel and file counter), `Inspector` thread with a
250 ms progress ticker, `Cmd::CancelInspect`, sequence-guarded publish, preview
matched to (root, folder), usable-first row ordering, "Checking add-ons... N files"
plus Cancel check, log lines `inspect start`, `inspect done ... in N ms`,
`inspect cancelled`, `tray: <item>`. A sync write step still finishes first (inline
step; `QuitGate` tests in control.rs). Version 0.3.1, docs/release/v0.3.1.md.

## A-38 controls
Nothing was written to `_forever_` or Dropbox; no enrollment; no WoW Sync or
Battle.net process touched; D:\WoWSyncDemo untouched.

## Verification
`cargo xtask verify --tier offline`: PASS. `--tier native`: PASS.
New required cases: R8-fix:setup-inspection.
