# R4-app: phase D (the GUI for run-004 features)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-app`, on branch
  `run004/R4-app`, created from `main` after the R4-notify-diag merge
  (`4dd9d7b`).
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** R4-integrate is still running in its own worktree. It
  adds the remaining settings commands and, later, wires R4-notify-diag's
  functions into `service.rs`/`control.rs`. You don't own any runtime file.
  Merge `main` into your branch whenever the orchestrator tells you to; on
  conflicts in an app file, keep both sides' intent and report.

## Authority

- [ui/screens.md](../../../design/ui/screens.md),
  [ui/interaction.md](../../../design/ui/interaction.md),
  [ui/style.md](../../../design/ui/style.md),
  [ui/README.md](../../../design/ui/README.md): screens, wording, keyboard
  and style. Where these and a feature design disagree on behaviour, the
  feature design wins; record the wording difference.
- Feature GUI sections:
  - [rollback.md](../../../design/rollback.md) §8 (GUI recovery actions);
  - [settings.md](../../../design/settings.md) §9 (status, explanations,
    master, exceptions, push, pause, restore);
  - [removal.md](../../../design/removal.md) §9 (Removed section, restore,
    allow again, waiting states);
  - [ops.md](../../../design/ops.md) "Built by step 3, R4-app" (§6, about
    line 799): `ReviewNeeded` and retired-self headline and tray, the
    Decision page from `DecisionOption`, the Computers "Retired" group and
    sheet, About update states, notification toggles, **Export
    diagnostics…** with both options; plus T-26-10..14, T-45-9, T-33-10 and
    the UQ-1/UQ-2 text.
- [contracts.md](../../../design/contracts.md) §9.3, the R4-app row (your
  file set), and §9.1 for the control command and query API.
- Handover tables you must honour:
  - [evidence/R4-integrate.md](../evidence/R4-integrate.md) "Handovers to
    R4-app" (new variants, their display, the command table);
  - [evidence/R4-notify-diag.md](../evidence/R4-notify-diag.md) (About
    strings from `UpdateResult::text`; the export sheet uses
    `diagnostics::{NEVER_INCLUDED, SHARED_FOLDER_WARNING, REFUSED_TEXT}` and
    `Replaced::summary`);
  - GUI-only items handed over in `evidence/R4-core-removal.md` (T32,
    keyboard flows, Removed section commands), `evidence/R4-core-settings.md`
    (T-S32, T-R18) and `evidence/R4-core-ops.md`.
- D-25 defaults. Supported recovery must work through the GUI without a
  terminal (AGENTS.md).

## Files you own (exactly contracts.md §9.3)

- Every file under `crates\wow-sync-app` (including `Cargo.toml`, `build.rs`,
  assets and `src\bin\devkit.rs`).
- Your section of `tests\required-cases.toml`, between
  `# >>> section: R4-app` and `# <<< section: R4-app`.
- Evidence: `docs\runs\run-004\evidence\R4-app.md`.

Don't touch anything else: no runtime, core, adapters or proto file, no
`Cargo.lock` change beyond what an app `Cargo.toml` edit requires (avoid new
dependencies; if one is needed, stop and ask). If a screen needs a command
or query the control API does not have, don't invent one: show the action
disabled with an honest reason, list it under **Handover to R4-integrate**
with the exact variant and fields you need, and continue.

## Work (commit per item, in order)

1. **Read the current control API** (`crates\wow-sync-runtime\src\control.rs`)
   and the app's `backend.rs`. Map every command and query the GUI needs to
   an existing variant. Record the map in your evidence first.
2. **Headline and tray:** `ReviewNeeded`, retired-self, `Removing`
   ("Removing") and `Held` ("Waiting") states, UQ-1 wording.
3. **Decisions:** the Decision page from `DecisionOption` (choose, keep,
   resume), with confirmation where the design requires it.
4. **Computers:** the "Retired" group and sheet, retire and re-enroll /
   allow again, with guided confirmation; a retired computer cannot publish.
5. **Rollback:** the §8 rollback action and its result states, including
   `NotAvailable`.
6. **Settings:** master selection, per-addon exceptions, explicit push,
   pause/resume, use-shared, restore and confirm, per settings.md §9.
7. **Removed section and recovery:** restore of removed addons (code and
   settings), three-state history view, temporary exceptions visible.
8. **About and diagnostics:** update states, manual check, notification
   toggles, **Export diagnostics…** with both options and the warnings.
9. **Tests:** model/world/page tests for each item, with at least one
   negative case per item (a disabled action, a refused command, a
   retired computer's publish blocked). One A-38 mutation control per item.
   GUI checks that can only be done by driving the window are listed as
   manual and left to R4-check, with exact steps.

## Gate and evidence

- Offline and native verify pass on the final tree.
- `git diff --name-only main..HEAD` lists only your files and evidence.
- The section check passes; no Windows-only test in an offline case.
- **Evidence:** the control map, each item with its tests and result
  (passed, failed, blocked or not run), the A-38 controls, the T-rows above
  with results, the manual steps for R4-check, and the handover to
  R4-integrate.
- **Commits:** prefix `run004(R4-app):`, with the Copilot trailer. Commit
  per item. Don't push. Edit and create files with the edit/create tools
  only. Mutate and revert only with the edit tool, then check
  `git diff --exit-code`. Never use
  `git checkout --`/`restore`/`reset --hard`/`stash` on uncommitted work.
- Stop cleanly at an item boundary if you run short and report what
  remains.
