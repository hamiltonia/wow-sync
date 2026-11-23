# R8-check: independent verification of v0.3.1

Role: verification / gpt-6.1-sol. Rules: [COMMON.md](COMMON.md). Write only
your evidence and, if needed, your own required-cases section. Do not change
product code. Report findings G1, G2, … with a severity and reproduction
steps.

Build under test: the `main` SHA named by the orchestrator, plus the dispatch
artifact (check its sha256, unzip it, and run `wow-sync.exe`).

## Checks

1. **Original scenario.** Start the release build with a fresh `--state`
   lab dir and without `--dev`. Go through setup to step 3 and select
   `D:\Games\World of Warcraft\_forever_`, both from the list and through
   "Choose folder…" (UI Automation). The read-only check must finish, and the
   log must show its start, end and duration. Repeat R8-fix's reproduction or
   stand-in, as described in its evidence.
2. **Quit during a check.** Make the check slow, for example with a large
   synthetic stand-in. Choose tray Quit while it runs: the process must exit
   within 5 s, with no orphaned process. Also show that Quit during a write
   step still waits safely.
3. **Progress and cancel.** Progress is visible during a long check. Back, or
   choosing another install, cancels it, and no stale result appears for the
   other row.
4. **Default selection.** With a refused `_classic_beta_` stand-in and a
   usable `_forever_` stand-in, the usable one is selected first.
5. **Log.** The tray item names and the inspect lines appear; no profile
   paths and no names.
6. **Regression.** Both verify tiers pass. Smoke-test v0.3.0 behaviours: drive
   discovery and the folder picker.
7. **Artifact smoke test.**

Write `docs/runs/run-008/evidence/R8-check.md` with PASS / FAIL / BLOCKED /
NOT RUN for each check and the environment. Clean up the lab, commit, and
reply with the SHA, a table and the findings.
