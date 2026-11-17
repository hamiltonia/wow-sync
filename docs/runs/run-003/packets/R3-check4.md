# R3-check4 (Verification, gpt-6.1-sol)

Read [COMMON.md](COMMON.md) first. You verify; you do not fix product code.
Your scope is narrow: the R3-fix2 changes only.

**Inputs:**
- `evidence/R3-fix2.md`.
- `evidence/R3-check3.md`: RC3-1 to RC3-3, plus its scripts in
  `evidence/R3-check3/`, which you may reuse.

**Owned paths:**
- `docs/runs/run-003/evidence/R3-check4.md`.
- `docs/runs/run-003/evidence/R3-check4/**`.

Rules:
- Use only synthetic data, under a neutral lab root.
- Stop only processes you launched, and only by their captured PID.
- Write only your own evidence.

## Checks

1. **RC3-1 in the real app.** Repeat R3-check3's sequence: an injected
   crash, then a stand-in game started afterwards. Check that:
   - the wording is neutral, not "WoW started while…";
   - with the game running, the next step is "close WoW";
   - with the game stopped, there is no "close WoW" step.

   Review the source and tests for the recorded-cause path. Run your own
   negative control.
2. **RC3-2.** An unstable scan shows "not available", not a blank.
3. **RC3-3.** The paused hint doesn't promise "0" while an update is
   waiting.
4. **Journal compatibility.** A journal written before R3-fix2 (without
   the new field) still loads.
5. **Tiers.** Run `cargo xtask verify --tier offline` and `--tier native`.

## Done

- The evidence gives a PASS, FAIL, BLOCKED or NOT RUN verdict per check.
- New findings are numbered RC4-n.
- Commit an evidence-only change on your branch. Do not push.
