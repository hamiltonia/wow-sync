# Run 011 plan: add-on settings (WTF) sync for real use

Approval: D-33 (2026-11-24). The owner asked for WTF sync to start while
away; the orchestrator decides and records each judgment call in
[DECISIONS-FOR-REVIEW.md](DECISIONS-FOR-REVIEW.md). There is no AIU ceiling;
usage is tracked in [USAGE.md](USAGE.md). Target: the `v0.4.0` Windows
pre-release.

## Starting point

- Settings sync (SPEC R-07 to R-10, [design/settings.md](../../design/settings.md))
  was built in run-004 and later runs:
  - core `settings.rs`;
  - adapters `wtf.rs`, `settings_store.rs`;
  - runtime `settings_cycle.rs`;
  - app `settings_ui.rs` and the Settings sync page.
- The 32 MiB per-file cap (D-26) is active in production
  (`records::MAX_SETTINGS_FILE_BYTES`, `service.rs` ~587). Comments in
  `service.rs`, `control.rs`, `settings_cycle.rs`, `core/settings.rs` and
  `fs_install.rs` still say that real settings sync is off pending Q-77.
- The live group has no settings policy. Nobody has chosen a master, and
  the only place to do it is the Settings sync page. Setup never mentions
  settings.
- Matching is by exact account / realm / character folder name (Q-69).
  Capture waits for two stable scans after WoW exits (Q-71). The app never
  creates account, realm or character folders.
- Real-data shape (dev PC `_forever_\WTF`, read-only inspection): 171 files,
  80 SavedVariables (36 account-wide, 44 per-character), about 1.18 MiB, the
  largest 474 KB.
- Settings sync has never been exercised end to end on realistic data with
  two computers and the GUI.

## Goals

1. **Guided start (OD-04).**
   - Once a computer's add-ons are up to date and no decision is waiting,
     show a one-time, dismissible "Share add-on settings?" offer (headline
     and Settings sync page; not a modal on every start).
   - The offer uses the §5.4 consent text and account list, and an explicit
     master choice with this computer pre-selected. Nothing is shared until
     the user clicks.
   - On every other computer, a matching "<computer> is now the settings
     master. Use its add-on settings here?" offer with that computer's
     account list (§5.4).
   - "Not now" hides the offer until the user opens the Settings sync page;
     it never nags again on its own.
2. **Visibility.**
   - Each add-on shows its settings status in plain words (§9.1): up to
     date (from <computer>), waiting for WoW to close, waiting for
     <character> to log in on this computer (the folder does not exist
     here), too large, owned by several add-ons, paused, choice needed.
   - Settings captures, publishes, applies, yields and restores appear in
     the run-010 change history and Activity, with source computer, add-on,
     identity in plain words, reason, and file / size change (§9.2).
   - Live progress covers the settings phase ("Saving settings for 12
     add-ons", "Using settings from RYZENFISH: 8 add-ons").
   - Logs gain one line per settings phase and per unit changed, without
     account or character names (log privacy, as run-010).
3. **Realistic end-to-end proof.**
   - Two synthetic computers, each with a lab copy of the dev `_forever_`
     add-ons and `WTF`, sharing a synthetic provider root.
   - Cover A-11 to A-15, plus a character that exists on only one
     computer, a file over the cap, an add-on whose saved variables are
     declared by two add-ons, recovery through the GUI, and the guided
     offers above.
   - Fix every defect found. A defect that needs a product change goes to
     DECISIONS-FOR-REVIEW and waits for the owner.
4. **Clean-up.** Correct the stale Q-77 "real settings off" comments.
5. **Release `v0.4.0`** with release notes that tell the owner exactly how
   to turn settings sharing on and what to expect.

## Packets and order

| ID | Role / model | Owns | Depends on |
|---|---|---|---|
| R11-runtime | implementation / claude-sonnet-5.5 | goals 2 (data, events, logs), 3 (lab harness, runtime fixes), 4: `wow-sync-runtime`, `wow-sync-core`, `wow-sync-adapters` | — |
| R11-app | implementation / claude-sonnet-5.5 | goals 1 and 2 UI: `wow-sync-app`, version 0.4.0, release notes, user docs | R11-runtime interface commit, if one is needed |
| R11-review | review / claude-opus-5.5 | both merges; safety of settings writes on real data | R11-* merged |
| R11-check | verification / gpt-6.1-sol | the `v0.4.0` candidate, with the GUI artifact on lab data | merged, dispatch artifact |

- **Interface first.** If R11-app needs new runtime queries or events,
  R11-runtime commits only those types and signatures first and reports the
  SHA. The orchestrator merges it to `main` and R11-app merges `main`.
- R11-runtime owns `service.rs` and `settings_cycle.rs`. R11-app does not
  edit runtime files; it asks through the orchestrator.

## Gates for `v0.4.0`

- Both verify tiers pass; CI is green, including macOS `verify-offline`.
- The release dispatch build succeeds.
- R11-check reports PASS, using the dispatch artifact on two synthetic
  computers with lab copies of the dev data, for:
  - the guided offer on both computers, with consent and account choice;
  - A-11 to A-15 as observable in the GUI;
  - a character missing on one computer: shown as waiting, applied after
    the folder appears, never created by the app;
  - a file over the cap and a shared-ownership file: shown, never synced;
  - settings entries in Activity and history, with source, reason and
    size, surviving a restart;
  - recovery of replaced settings through the GUI;
  - log and history privacy: no account or character names in logs, no
    absolute paths, no file contents;
  - nothing written outside the lab `WTF` folders and the lab provider
    root;
  - background memory ≤ 100 MiB private working set;
  - the artifact smoke test.
- No open critical or major review findings.

## Stop conditions

- A fix needs a product-intent change (SPEC, ACCEPTANCE, an approved
  Q-answer) → record it in DECISIONS-FOR-REVIEW and leave the behaviour
  as approved.
- Any write to the real Dropbox group, the real `_forever_`, or the
  owner's running instances → forbidden.
- Turning settings on in the real group → forbidden; the owner does it.
- Any irreversible action on user data → stop and wait for the owner.
