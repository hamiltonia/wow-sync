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

## Round 2 (RV8-01..09)
- RV8-01: Quit during setup-after scans. Cause: enroll/status/sync ran inline on the backend thread and
  each hashed the tree many times (~12 scans per loop pass). Fix: `Shared.abort_reads` (set by the UI in
  `begin_quit`, cleared by "keep running") feeds `SliceService.read_ctl`; read-only scans return
  `ServiceError::Cancelled` at the next file and change nothing. Writes never consult the flag; the
  post-apply observe is uncancellable once files changed, so the cycle publishes. Scans per pass are now
  ~4 (status 1, sync 3, enroll 1) and the sync result's status snapshot is reused instead of rescanned.
  Tests: `e2e::quit_abandons_a_slow_sync_status_and_enroll_scan_within_a_bound` (slowed scan, Quit stops
  within 500 ms) and `e2e::a_write_in_progress_finishes_whole_even_when_quit_is_requested`.
- Quit timing during a sync scan: before ~12 scans x 6-7 s (real 33k-file tree) = ~1.5 min (reviewer's
  probe, 0.45 s/scan at 6k files); after: stops within one file's hash time (test bound 500 ms,
  slowed 25 ms/file). Measured on a synthetic tree only.
- RV8-02 release notes rewritten. RV8-03: seq guard is the sole stale-publish defence and is tested via
  `_late_` seam. RV8-04: `setup::fold_preview` + `digest_for` tests. RV8-05: `Install::start_checking`
  on every re-check, Next disabled until done. RV8-06: panic caught, ticker drop-guard, spawn failure
  ends as an error. RV8-07: setup_panel digest uses `digest_for`. RV8-09: redaction test for the new lines.
- Not reproduced: the RYZENFISH stall on real hardware; the GUI was not driven in either round.
- Verify: offline PASS, native PASS. New cases appended to R8-fix:setup-inspection.

## Round 3 (RV8-11, RV8-12)
- RV8-11: a cancelled enroll now publishes 'Setup was stopped before it finished...' (the page leaves 'Setting up' and Back returns); CancelQuit clears the abort flag and forces a fresh status read. Test: backend::keep_running_after_quit_leaves_a_stopped_setup_actionable_and_clears_the_flag (real enroll cancelled by the flag; asserts message, no enrolled_now, flag cleared, quit_ready false). The fresh-status read itself is not asserted by a test.
- RV8-12: a failed last look in sync_once now sets the Error headline and provider_error on the returned status in the same pass; no new test.
- Verify: offline PASS, native PASS.
