# R3-check3 (Verification, gpt-6.1-sol)

Read [COMMON.md](COMMON.md) first. You verify; you do not fix product code.

**Inputs:**
- `docs/runs/run-003/evidence/R3-fix.md`.
- The findings RV-1 to RV-9 and RV-12 in `evidence/R3-review.md`.
- The findings RC2-1 to RC2-4 in `evidence/R3-check2.md`.
- D-21 and `docs/design/ui/screens.md`.

**Owned paths:**
- `docs/runs/run-003/evidence/R3-check3.md`.
- `docs/runs/run-003/evidence/R3-check3/**`.

Rules:
- Use only synthetic data, under a neutral lab root (for example
  `D:\WoWSyncLab\R3-check3`), with no user profile paths.
- Stop only processes you launched, and only by their captured PID.
- No PowerShell writes to tracked files except your own evidence.

## Checks

1. **Finding closure.** For each ID, check the code and run the tests
   R3-fix cites. Mark it CLOSED, OPEN or NOT VERIFIED.
   - For RV-1 and RV-2, run your own negative control: break the mapping,
     show that a test fails, then restore it.
2. **Real app, observed.** Use the devkit and the synthetic provider to
   build real-app states. Capture a screenshot and the UIA text for:
   - the dashboard;
   - a pending decision;
   - an add-on interrupted by the game;
   - a damaged pause file.

   Confirm that no text is invented and none mentions builds or game
   versions. R3-fix did not produce these screenshots.
3. **RC2-2.** The close-option UIA TogglePattern changes the setting and
   the change persists.
4. **RC2-3.** Tray left click and the single-instance show request always
   show the window.
   - Use a physical click only if you can unambiguously target this app's
     own icon.
   - Otherwise mark the tray click BLOCKED and test the second-instance
     path.
5. **RC2-4.** A piped devkit capture returns within 5 s.
6. **Tiers.** Run `cargo xtask verify --tier offline` and `--tier native`.
7. **Memory.** Run the A-29 `tools/mem` check.

## Done

- The evidence gives a PASS, FAIL, BLOCKED or NOT RUN verdict per check,
  with commands, results and environment.
- New findings are numbered RC3-n, with a severity for each.
- Commit an evidence-only change on your branch. Do not push.
