# Run 011 report

Approval: D-33. Result: **`v0.4.0` pre-release**, adding add-on settings
(WTF `SavedVariables`) sync. The owner was away for the run. Every judgment
call is in [DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md) (OD-01 to OD-64)
for review. No critical or major findings remain open. The known limits are
below and in the [release notes](../../release/v0.4.0.md).

## Owner request and delivery

| Owner request | Delivered |
|---|---|
| "get started on the WTF sync … continue as long as you can" | Settings sync is ready for real use. Turning it on is guided, with explicit consent per account and a choice of master. Each add-on's status is shown in plain words, and local copies are kept before anything is replaced. Choices name the kind, source, time and size of each copy. Pause all and per-add-on pause are available, and recovery (restore and undo) works from the GUI. Settings changes appear in Activity and in each add-on's history. |
| "collect them all for review" | 64 decisions are logged with reasons, and each one is marked reversible or not. |

## Evidence

- **Implementation:**
  - [evidence/R11-runtime.md](evidence/R11-runtime.md): settings cycle,
    statuses, retention, the G9-G13 and G16 fixes, and the memory rounds.
  - [evidence/R11-app.md](evidence/R11-app.md): guided offer, GUI, choice
    labels, and the G18 bisection.
- **Review:** [evidence/R11-review.md](evidence/R11-review.md). Two review
  rounds plus focused re-checks of the G13 fix. RV11-17 (major: a master's
  in-game edit could be reverted) and RV11-20 (minor) were found and fixed.
  None is open.
- **Independent check:** [evidence/R11-check.md](evidence/R11-check.md). Six
  rounds were run on release-dispatch artifacts, never on local builds.
  - Rounds 1-5 found G1-G18. Of these, G1 was a checker launch incident
    (OD-41). All the product findings are fixed except G18, which was traced
    to the test harness (OD-62, OD-64).
  - The final product code is `7a65a11`, verified as dispatch 45749110502.
    Later commits change only a test and documentation.
  - PASS:
    - consent and master choice on both computers;
    - A-11 to A-15;
    - missing character;
    - over-cap and shared-ownership files;
    - history, restart and recovery;
    - G13/G16 convergence, 6 of 6 in about 7.6 seconds with no false "up to
      date";
    - G17 choice labels;
    - privacy and diagnostics export;
    - write audit;
    - all six v0.3.3 code-sync operations in both directions.
  - **Memory, hidden (A-29, 100 MiB):**
    - fresh processes: 89-93 MiB;
    - window never shown: 64-71 MiB;
    - 8 repeated UI cycles: 86-89 MiB with no growth trend.
  - The memory reduction rounds took this from about 104 MiB (OD-47, OD-49,
    OD-50).
- **Speed:** carried from R10 (3.19×; OD-54). The same lab showed no
  slowdown in this run.
- **Owner data:** after each check round I read the owner's log myself. Every
  line in each check window came from the owner's own v0.3.2 (PID 8816).
  The only exception is the two-line G1 incident in round 1 (OD-41).

## Highlights for owner review

- **OD-41:** the checker launched the candidate once without a separate state
  folder. It added two lines to your log and showed your window. Nothing else
  was changed.
- **OD-45, OD-53, OD-64:** how the memory gate was applied.
  - OD-45: the gate covers the window hidden after use; visible peaks are
    only reported.
  - OD-53: sampling had gaps of up to about 1 second.
  - OD-64: G18 was traced to the test harness, and a long-running screen
    reader is not yet measured.
- **OD-55, OD-56:** the settings master now applies its own choice or restore
  without republishing. A later in-game edit is offered as a choice, never
  silently replaced.
- **OD-59:** while WoW is running, the headline can still say up to date
  even when an add-on is waiting for the game.
- **Earlier highlights:** OD-23, OD-28, OD-30, OD-34, OD-38 and OD-39.

## Deferred or accepted

| Item | Disposition |
|---|---|
| Long-lived screen-reader memory | Not measured. Known limit (OD-64). Candidate for a future run. |
| Headline while WoW runs (OD-59) | Unchanged from earlier releases; the add-on's row says "waiting for the game". |
| "Not now" on the settings offer | Not saved between runs (release notes, known limits). |
| RV11-19 residual | If an apply fails without recording a fact, the status can still show Active for one pass. |
| RV11-21 | An edit made in game before a selection made during the same session is offered as a choice. |
| Earlier risks | G5 TLS flake; FAT32 and Unix paths untested; computers offline for more than 30 days and garbage collection. |

## Process notes

- Worker scripts must use absolute paths, because .NET IO resolves relative
  paths against the main checkout.
- Every candidate launch must pass `--dev --install --state --provider`. The
  log starts before the single-instance check, so a bare launch writes to
  the owner's log (OD-41).
- Usage: about 12,650 AIU ([USAGE.md](USAGE.md)).
