# Run 008 report

Approval: D-30. Result: **`v0.3.1` published**. All [PLAN.md](PLAN.md) gates
pass.

## Problem and root cause

On RYZENFISH, `v0.3.0` setup appeared to hang while checking `_forever_`, and
tray Quit did nothing.

**Most likely cause:** the setup check hashes every add-on file. The check ran
on the backend command thread with no progress shown, and Quit queued behind
it. RYZENFISH has 3.2 GB in 31k add-on files. R8-check's synthetic tree of the
same size took 123.5 s to check from a cold cache. The RYZENFISH log ends 24 s
after the pick, so the owner was most likely watching a slow first check, not a
deadlock.

**Not proven:** this was never measured on RYZENFISH itself (RV8-08). The dev PC
checks its warm `_forever_` in about 6 s.

## Delivered

- **Setup check:** runs on its own cancellable thread, showing "Checking
  add-ons… N files" with **Cancel check**. Back, choosing another install, or
  Quit cancels it.
- **Quit:** read-only scans in setup, enroll, sync and status are cancelled at
  the next file. Only a write already in progress finishes first, and then the
  app exits. During setup, Quit goes straight to quit-when-safe.
- **Fewer scans:** a sync pass now scans about 3 times, down from about 12.
- **Results:** each check result lands only on its own row. The enroll digest
  must match the folder being enrolled.
- **Default selection:** a usable install is selected first, both from
  discovery and from the folder picker.
- **Log:** new lines for `inspect start/done/cancelled` and `tray: <item>`.
- **Other fixes:** RV8-02..07, RV8-09, RV8-11, RV8-12, G1 and G2.

## Evidence

- **Fix (R8-fix):** [evidence/R8-fix.md](evidence/R8-fix.md), worked over 4
  rounds.
- **Review (R8-review):** [evidence/R8-review.md](evidence/R8-review.md). No
  critical or major findings remain open. RV8-08 is a note, and RV8-13/14 are
  accepted notes.
- **Independent check (R8-check):** [evidence/R8-check.md](evidence/R8-check.md).
  All checks pass after recheck:
  - read-only Quit in 164–217 ms;
  - Quit during a setup write exits 2.6 s after the write finishes, with all
    6,001 files hash-identical;
  - the real `_forever_` check takes 5.6–5.9 s;
  - the artifact (release run 45293621382, at `f1fe1e9`) passed its smoke test,
    with Quit in 179 ms.
- **Builds:** both verify tiers pass on each merge; CI 45291126328 is green.

## Remaining risks

- The fix has not been confirmed on RYZENFISH. The owner should retry there.
  A cold first check of 3.2 GB of add-ons may still take about 2 min, but it now
  shows progress and can be cancelled.
- A full add-on hash on every check is inherently slow on large add-on sets. A
  faster check (for example, metadata-based change detection) would be a future
  product decision.
- One process incident: R8-review's first mutation run briefly edited the main
  checkout. It was reverted at once, nothing was committed, and the checkout
  was confirmed clean.
