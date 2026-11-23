# R8-fix: setup inspection hang, Quit, progress and log

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goals 1–6 in [../PLAN.md](../PLAN.md).

## 1. Reproduce and find the root cause first

- Build the release binary from your worktree. Start it with a fresh
  `--state D:\WoWSyncLab\R8-fix\s1`, **without** `--dev`, so discovery and
  setup work as on RYZENFISH. Go through setup to step 3 and select
  `D:\Games\World of Warcraft\_forever_`, read-only. Record the timings from
  the log.
- If it does not reproduce, compare with the RYZENFISH log and probe, and
  build the closest synthetic stand-in. Points to compare:
  - a second variant (`_classic_beta_`) that has no `.flavor.info`;
  - the folder chosen by picker versus the discovered row;
  - WTF with several accounts and realm folders (495 files);
  - Dropbox at `D:\Dropbox` with no existing `WoW Sync` group;
  - not enrolled.
- Read the inspect path end to end: `SliceService::new` → `inspect()`, the
  provider and Dropbox access, locks on the state dir, and per-file work. Look
  for a deadlock, an unbounded loop or wait, or work that is quadratic in the
  file count.
- Record the root cause with evidence: a stack, timings or a minimal failing
  test. If you cannot find a cause, say so plainly, ship goals 2–5 anyway, and
  list what you ruled out.

## 2. Fix

- Fix the root cause, with a regression test that fails without the fix.
- **Quit:** Quit must exit within 5 s while a read-only inspection is running.
  - Run the inspection off the command loop: a separate thread with a cancel
    flag, or a cancellable call.
  - `QuitWhenSafe` must not wait behind a read-only inspection. It still waits
    for a write step, as today.
  - Test both cases.
- **Progress and cancel:** a long check shows progress on setup step 3 (for
  example "Checking add-ons… 12,000 files"). Back, another selection, or Quit
  cancels it. The result of a cancelled check never reaches the UI for a
  different selection; match previews to their target, not to the current
  index.
- **Default selection:** when one install row is usable and another is
  refused, the usable one is selected first. Test it.
- **Log:** record these lines:
  - `inspect start <folder>`;
  - `inspect done <folder> in <ms> ms: …`, or `inspect cancelled <folder>`;
  - `tray: open`, `tray: quit`, and so on, naming the item.
  Profile paths are written as `~`.
- Update the setup-related docs if behaviour text changes, write the release
  notes in `docs/release/v0.3.1.md`, and bump the version to 0.3.1.

## Done

- Both verify tiers pass; no Windows-only paths in tests (macOS CI).
- Add a required-cases section for your new tests.
- Write the evidence: root cause, reproduction, timings, before and after, the
  A-38 controls, and anything you could not reproduce.
- Commit, then reply with the SHA and a summary.
