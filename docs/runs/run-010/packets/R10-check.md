# R10-check: independent verification of v0.3.3

Role: verification / gpt-6.1-sol. Rules: [COMMON.md](COMMON.md). Write only
your evidence and, if needed, your own required-cases section. Do not change
product code. Report findings G1, G2, … with a severity and reproduction
steps.

Build under test: the `main` SHA named by the orchestrator, plus the dispatch
artifact (check its sha256, unzip it, and run `wow-sync.exe`).

## Checks

1. **History.** Use two synthetic computers (isolated `--state` dirs, a
   synthetic or `WoW Sync Test` provider). Make each change on A and let B
   apply it:
   - an update;
   - a new add-on;
   - a removal;
   - a copy choice;
   - a rollback;
   - a settings change.
   On B, the Activity page and the add-on page show the right source computer,
   old → new version, file counts, size and reason. Restart B; the history is
   still there.
2. **Live progress.** Apply ≥ 1 GB of synthetic add-ons:
   - the headline and tray change within about 2 s of the apply starting and
     keep updating;
   - the incoming summary is shown;
   - with a pending decision that the apply satisfies, the decision text does
     not linger.
3. **Logs.** Phase and per-package lines appear while the pass runs. There
   are no absolute paths, profile names or character names.
4. **Speed.**
   - On the real `_forever_` (read-only, isolated state, synthetic provider),
     measure an unchanged pass, warm, median of 3, for `v0.3.2` and the
     candidate. The candidate must be ≥ 3× faster.
   - After an install, the next pass reads 0 installed files.
   - A file edited after the install is detected.
5. **Memory.** During the ≥ 1 GB apply, the peak private working set must be
   ≤ 100 MiB.
6. **Artifact smoke test.**

Write `docs/runs/run-010/evidence/R10-check.md` with PASS, FAIL or BLOCKED
per check and the actual environment. Commit, then reply with the SHA and a
verdict.
