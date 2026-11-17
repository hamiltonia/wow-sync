# R3-check2 (Verification, gpt-6.1-sol): independent check of phase F

Read [COMMON.md](COMMON.md) first. Also read D-21 and
[R3-ui-build.md](R3-ui-build.md), the packet being verified.

**Owned paths:**
- `docs/runs/run-003/evidence/R3-check2.md`.
- `docs/runs/run-003/evidence/R3-check2/`, under 1 MB.

Do not change product code. Temporary mutations are allowed but must never
be committed.

## Checks

Do not trust `evidence/R3-ui-build.md`; re-run the checks yourself.

1. **No fake data in the real app.**
   - Grep `crates/wow-sync-app` for scenario or fake fixtures.
   - Launch the real app on synthetic data:
     - use a temp copy of the `D:\WoWSyncDemo` layout, or `Start-A.cmd`
       and `Start-B.cmd`;
     - use `devkit` for a second computer and `fake-client` for the
       WoW-running state.
   - Confirm that each screen's values match the runtime and provider
     state on disk.
2. **Behaviours, by UI Automation or keyboard:**
   - Pause persists across an app restart (Q-28).
   - Close hides to the tray; the first close shows the sheet. The
     "Closing the window quits WoW Sync" option works and is off by
     default (Q-26).
   - Theme override (Q-34).
   - Tray left click versus right click (Q-39).
   - Esc on a sheet restores focus to the control that opened it (RC-3).
   - The WoW-running block (fake-client) blocks apply and is shown.
   - The setup wizard on a fresh state directory enrols successfully.
   - No computer IDs appear in visible sentences (UG-22).
3. **Negative controls:** run at least 2 of your own on the new runtime
   APIs, for example the pause persistence and quit-when-safe gate. Each
   must make a named test fail with exit 101.
4. **A-29:** re-run the `tools/mem` harness as documented, at least 1 run
   per scenario, on the release build.
5. Run `cargo xtask verify --tier offline` and `--tier native`.

**Process safety:** stop only processes you launched, by the PID returned
at launch. Other WoW Sync demo processes may already be running and must
not be touched. Use a separate temp state and provider directory so you
don't disturb them.

## Output

- A PASS, FAIL, BLOCKED or NOT RUN verdict per check.
- **Findings:** list each with ID RC2-n and its severity.
- Commit only your evidence.
