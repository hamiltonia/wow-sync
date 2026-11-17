# R3-check (Verification, gpt-6.1-sol): phase E

Read [COMMON.md](COMMON.md) first. Also read **D-21**.

You are an independent verifier. Do not trust the authors' completion
claims. Re-run the checks yourself.

**Owned paths:**
- `docs/runs/run-003/evidence/R3-check.md`.
- `docs/runs/run-003/evidence/R3-check/`, under 1 MB.

Do not change product code, tests or docs. Temporary mutations for
negative controls are allowed but must never be committed. Restore them and
show `git status` clean.

## Checks

1. **D-21 (Q-21 removal), from evidence `R3-d21.md`:**
   - Confirm in the code and tests that a different game build or
     Interface declaration never withholds a candidate.
   - Confirm that the variant boundary still holds.
   - Run your own negative controls: re-introduce a build-based hold, and
     separately remove the variant check. Each must make a named test fail
     with exit 101.
   - Confirm that `spikes/gpui`, `spikes/mem` and `spikes/a11y` are gone
     and nothing references them.
2. **Prototype, from evidence `R3-proto.md`:** launch
   `D:\WoWSyncDemo\Start-Prototype.cmd` (the fake-data demo) and drive it
   by keyboard and UI Automation. Verify these items, which the author
   could not:
   - Shift+Tab;
   - Down/End roving in the S14 list;
   - the sheet focus trap;
   - Esc;
   - accessible names on each page.

   Record PASS, FAIL or NOT RUN for each. Stop only the process you
   launched.
3. **Memory, from evidence `R3-mem.md`:** re-run the harness exactly as
   documented, at least 1 run per scenario. Compare with the reported
   numbers.
4. **CI:** use `gh run list` and `gh run view` to confirm that the latest
   `main` commit's CI is green on all jobs, including native and macOS.
   Link the run.
5. **Records:** spot-check that every claim in the HANDOFF phase table has
   evidence.

## Done

- Commit the evidence with a PASS, FAIL or BLOCKED verdict per check.
- List the findings with a severity for each.
