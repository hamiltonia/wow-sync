# R3-ui-build (Implementation, sonnet-5.5): phase F

Read [COMMON.md](COMMON.md) first. Authority also includes **D-21**, which
approves the prototype as the basis for the real UI and sets the UI
defaults.

**Inputs:**
- `docs/design/ui/**`: the approved design, data gaps UG-1 to UG-26, and
  the interaction and style docs.
- The prototype `crates/wow-sync-proto/**`. It is approved, and its layout,
  copy and behaviour are the reference.
- Evidence `R3-proto.md` and `R3-mem.md` (the memory harness in
  `tools/mem/`).
- The current app `crates/wow-sync-app/**` and the runtime
  `crates/wow-sync-runtime/**`.

**Owned paths:**
- `crates/wow-sync-app/**`.
- `crates/wow-sync-runtime/src/**` (any file, including new modules).
- In core, only new modules plus `explain.rs` and `lib.rs`. Use them for
  typed view data such as UG-12 `next_action` and the UG-22 sentence fixes.
  Do not change the reconciliation, target, inventory or admission rules.
- `docs/design/ui/**`: mark gaps resolved or deferred, and record the D-21
  changes.
- `tests/required-cases.toml`, limited to the entries for your tests.
- The demo launchers in `D:\WoWSyncDemo\*.cmd` and the binaries in
  `D:\WoWSyncDemo\bin\`.
- `docs/runs/run-003/evidence/R3-ui-build.md` and `evidence/R3-ui-build/`.

Do not change `crates/wow-sync-proto` (it stays as the fake-data demo),
`Cargo.toml` dependencies or `deny.toml`. **No new third-party crates.** If
one is unavoidable, stop and report it.

## Goal

Replace the thin-slice shell with the approved design, **wired to the real
runtime**. The real app must never show fake data. Where data is not
available yet, show the designed "not available yet" or empty state, never
an invented value.

### Must have, in priority order

1. Window layout, sidebar and navigation, light/dark/system theme (Q-34),
   keyboard focus and accessible names, as in the prototype.
2. **Overview, Add-ons** (list plus detail with the "why" explanation),
   **Computers, Activity** and **Settings** (app settings), from runtime
   data. Close these data gaps in the runtime/core:
   - UG-1 typed status;
   - UG-3 title and versions;
   - UG-4 awaiting computers;
   - UG-5 and UG-6 member summary;
   - UG-7 activity reader;
   - UG-10 game activity;
   - UG-11 provider problem;
   - UG-12 next action;
   - UG-14 last/next cycle;
   - UG-22 sentences without IDs.
3. **Pause/resume (UG-2)**: computer-wide, persisted, lasts until resumed
   (Q-28). **Quit when safe (UG-17)**: a sheet with "Quit when safe" and
   "Keep running" (Q-29).
4. **Tray**: on Windows, left click opens the window and right click opens
   the menu (Q-39). Close hides to the tray. The first close shows the
   one-time explanation sheet. App settings has a **"Closing the window
   quits WoW Sync"** option, off by default (Q-26 per D-21).
5. **Setup and reconfirm screens** as designed, using the existing
   enrollment and consent APIs. The setup flow may read-only detect the WoW
   and Dropbox locations (UG-24) from known install locations and the
   registry.
   - It must never write outside the paths the user confirms.
   - Tests use synthetic installs only.
   - Do not implement renaming (Q-31 deferred); use the OS computer name.
6. Decisions and needs-help stay **read-only** (Q-25). Show the options;
   don't act on them yet (UG-8 and UG-9 are deferred).
7. **D-21 alignment:** no compatibility-hold or Interface-based UI states.
   The variant mismatch still shows.
   - You may also remove the now-dead
     `DecisionReason::DifferentClientBuild` and
     `Explanation::DifferentClientBuild` from core `target.rs` and
     `explain.rs`.
   - If you do, update the one fake scenario in `crates/wow-sync-proto` that
     uses them. That is the only proto edit allowed.
8. **File writes:** edit/create tools only. **No** `Set-Content`,
   `Out-File` or PowerShell string splicing into files (incident I-1).

### Deferred (show the designed placeholder; record in evidence)

- UG-8 and UG-9 actions.
- UG-13 recovery restore. Listing recovery points read-only is fine if
  cheap.
- UG-15 multiple groups.
- UG-16 reconfirm cause detail.
- UG-18 update check. Network access is not in scope this run.
- UG-19 OS notifications. Q-30 needs crate approval; keep the in-app
  preview off in the real app.
- UG-20 and UG-21.
- UG-23 start at sign-in. Do not write Run keys on the dev PC.

## Verification you must run

- **Unit tests:** cover the new runtime and core data APIs, including
  negative cases.
  - Pause persists across a restart.
  - Quit-when-safe waits for the safe point.
  - Sentences contain no computer IDs.
  - Do one A-38 negative control per new behaviour area, up to 3.
- **Real app on the synthetic demo:** `D:\WoWSyncDemo\Start-A.cmd` and
  `Start-B.cmd`, with `devkit fake-client`.
  - Rebuild the release binaries into `D:\WoWSyncDemo\bin\`.
  - Capture screenshots of Overview, Add-ons, an add-on detail, Computers,
    Activity, Settings, Paused, the WoW-running block, and the first-close
    sheet, in light and dark, under `evidence/R3-ui-build/`.
  - Total size under 1 MB.
- **UI Automation:** dump the accessibility tree for Overview and Add-ons
  to show accessible names and focus order.
- **A-29 memory:** re-run the `tools/mem` harness on the new app, at least
  3 runs. It must stay ≤100 MiB. Report the numbers. A FAIL is a stop
  condition; report it, don't work around it.
- `cargo xtask verify --tier offline` and `--tier native` both pass.

## Done

- Commit the work.
- The evidence lists:
  - every data gap closed or deferred;
  - each screen's status;
  - the commands run;
  - the memory results;
  - the known limitations.
