# R3-fix (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first. Authority includes **D-21** in
`docs/OWNER-DECISIONS.md`, `docs/design/ui/screens.md` and the findings in
`docs/runs/run-003/evidence/R3-review.md` (RV-1 to RV-9, RV-12) and
`docs/runs/run-003/evidence/R3-check2.md` (if merged; its findings are listed
under "Check2 findings" below).

File writes go through the edit/create tools or `cargo fmt` only. Never use
PowerShell `Set-Content`, `Out-File`, `>` or similar to write source or docs
(incident I-1).

**Owned paths:**
- `crates/wow-sync-app/**`, `crates/wow-sync-runtime/**`,
  `crates/wow-sync-adapters/**` (new ports only, for RV-7).
- `crates/wow-sync-core/src/view.rs` (only if a carried field needs a public
  accessor).
- `crates/wow-sync-proto/src/{model.rs,scenario.rs}` (D-21 copy only).
- `docs/ARCHITECTURE.md`, `docs/design/crates.md`, `docs/REPO-MAP.md`,
  `docs/design/core-api.md`, `docs/design/ui/{screens.md,prototype.md,questions.md}`,
  `docs/design/ui/wireframes/{dashboard-dark.svg,decision.svg}`,
  `docs/design/removal.md` (line about `DifferentClientBuild` only).
- `tools/mem/**` (only for RV-12's lab root).
- `.gitignore` (only the `assets/local/` line, for goal 12).
- `tests/required-cases.toml` (entries for cases you add).
- `docs/runs/run-003/evidence/R3-fix.md` and `evidence/R3-fix/**`.

Do not change product behaviour beyond the findings. Do not touch removal,
settings or rollback.

## Goals

1. **RV-1 single authority.** Carry `PackageRow.next` and `.awaiting` into the
   app world model and render core's `NextAction` sentence. Derive "why" text
   from core's `Explanation`. Delete the app's duplicated sentences.
   - `Interrupted` and `InterruptedByGame` must read differently.
   - No "every computer has the same files" claim unless core states it.
   - Recovery text must not say "no earlier copies are needed" when history
     exists; if the data isn't available, say nothing.
   - Grouping text must not claim a reason core doesn't give.
   - Tests: UI text follows core for `Interrupted` vs `InterruptedByGame`, and
     for at least one other next action.
2. **RV-2 D-21 copy.** Give each `DecisionReason` its own wording per
   screens.md, in the app and the prototype. No decision text mentions game
   builds, game versions or Interface support as a reason to hold. Add a test
   over all `DecisionReason` variants.
3. **RV-3 docs.** Update `core-api.md`, `screens.md`, `prototype.md`,
   `removal.md` and the two wireframes to the D-21 reason set (or mark the
   wireframes superseded in a visible note).
4. **RV-4 no false continuation.** Remove the hard-coded `PerPackage` scope and
   the "Only X is affected" / "{title} only" sentences until core exposes a
   typed scope (UG-25), as screens.md says. Remove the stale comment.
5. **RV-5 headline.** Use core's `Headline` as is. If you believe a promotion
   is needed, don't implement it; write it as an open question in
   `docs/design/ui/questions.md`.
6. **RV-6 confirmed count.** This computer's `confirmed` counts only applied
   or in-sync packages. Add a test.
7. **RV-7 adapter boundary.** Move the runtime's direct file and environment
   I/O (`control.rs` pause file, `discover.rs` directory reads, Dropbox
   `info.json` read, env reads) behind adapter ports. Bound the `info.json`
   read (64 KiB). An unparsable pause file is an error state shown in the
   UI, not "paused since 1970". Add tests for both.
8. **RV-8 docs match code.** Update `ARCHITECTURE.md` (drop "pending
   independent review"; describe `core::view`, the runtime snapshot, pause,
   quit, activity and discovery APIs, and the proto crate), `crates.md`
   (module table, crate graph) and `REPO-MAP.md` (add `wow-sync-proto`).
9. **RV-9 duplication.** Do not extract a shared crate in this packet.
   Record in `crates.md` and `prototype.md` that the prototype is frozen as
   the D-21-approved reference: only D-21 copy fixes go to it, and it is
   retired or merged into a shared UI crate in a later run. Note RC-3 is
   fixed in the app only.
10. **RV-12 evidence paths.** Make `tools/mem` and the UI evidence runs use a
    neutral lab root (for example `D:\WoWSyncLab`) via a parameter, and use it
    for this packet's shots. Do not rewrite old evidence.
11. **Check2 findings.** Fix any listed below that are in your owned paths.
12. **Optional owner icon, local only (lowest priority; skip if budget is
    tight).** The owner provided app art outside the repo. It contains
    third-party trademarks (the WoW crest and the Dropbox logo), and the repo
    is public, so **never commit, copy into tracked paths or embed that art**.
    - If it needs no new dependency, let the app load its window and tray
      icon at runtime from an optional local file. Use the env var
      `WOW_SYNC_ICON` or the gitignored path `assets/local/app-icon.png`, and
      add that path to `.gitignore`.
    - If the file is missing or invalid, fall back silently to the bundled
      icon.
    - Embedding the exe icon needs a resource-compiler dependency, so do not
      do it; note that in the evidence.
    - Test the fallback with a synthetic image only.

## Check2 findings

From `docs/runs/run-003/evidence/R3-check2.md`:

- **RC2-1 (medium).**
  - `crates/wow-sync-app/src/pages.rs` about line 202 prints a hard-coded
    "next in 4 min". Show the real next check from the runtime scheduler, or
    "unavailable".
  - Check the literal "for 2 add-ons" at about line 209 and derive it from
    data.
- **RC2-2 (low).** The close-option switch exposes the UIA TogglePattern,
  but invoking it does nothing. Make the toggle change and persist the
  setting just as Enter does.
- **RC2-3 (low).** Per Q-39, a tray left click must always open (show and
  focus) the window. Today it hides the window when it is already visible
  (`app.rs` about 1016-1021). The single-instance show request must also
  always show the window.
- **RC2-4 (low).** In the devkit, `fake-client` must not leave its stand-in
  stranded when its output is captured through a pipe. Make the helper
  close or redirect stderr for the child, and print the PID first. Show a
  piped capture returning within 5 s.

## Done

- `cargo xtask verify --tier offline` and `--tier native` both pass.
- A-38 negative control: break the RV-1 mapping (render a hard-coded sentence
  for `InterruptedByGame`) and show the new test fails, then restore.
- Updated real-app screenshots (with UIA text) for: dashboard, a decision,
  an interrupted addon, a damaged pause file.
- A-29 re-run with `tools/mem` stays under 100 MiB.
- Evidence lists each finding with file, change and test, plus any finding
  you could not close and why.
