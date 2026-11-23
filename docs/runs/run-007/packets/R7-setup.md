# R7-setup: find WoW anywhere, folder picker, diagnostic log, `v0.3.0`

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R7-setup`, branch `run007/R7-setup`.

## Why

On the owner's second PC, WoW is at `G:\Games\World of Warcraft`
(`_forever_`); Battle.net's `product.db` records exactly that path.
`runtime/src/discover.rs` `wow_candidates()` probes only `%ProgramFiles%` and
drives C–F. Setup step 3 (`app/src/setup.rs` ~131) lists `d.installs` with no
empty state and no way to choose a folder.

The only log is `app.rs` `log()`, which writes only when `WOWSYNC_LOG` is set
before launch. It records nothing about discovery. The owner's log from that
PC was 8 lines of window events.

## Goals

1. **Discovery (PLAN goal 1, A-47).** In `discover.rs`:
   - Every local fixed drive A–Z, via `GetLogicalDrives` and `GetDriveTypeW`
     with `DRIVE_FIXED` only, or the existing `win`/`probe` helpers. Never
     touch network, removable or CD drives.
   - Each drive gets the four locations in PLAN goal 1.
   - Paths from `%ProgramData%\Battle.net\Agent\product.db`, read
     bounded (≤ 4 MiB) as untrusted bytes. Extract absolute paths containing
     `World of Warcraft` (both slash styles), normalize them, and keep them
     only if they pass the existing `find_wow` check (a folder with `_x_`
     variant subfolders).
   - Deduplicate case-insensitively.
   - Keep `find_wow`'s read-only guarantee. Unit-test path extraction with
     synthetic bytes, including garbage, a missing file and a huge file.
   - Return the probe list and results so the app can log them.
2. **Folder picker (PLAN goal 1, A-47).**
   - Step 3 always shows "Choose folder…".
   - When nothing was found, it shows a plain message: "World of Warcraft was
     not found automatically. Choose the folder where it is installed (it
     contains folders like `_retail_` or `_forever_`)."
   - Use GPUI's directory prompt (`cx.prompt_for_paths` with directories only)
     if the pinned GPUI has it. If not, stop and report before adding a
     dependency.
   - Accept either an install root or a `_variant_` folder (use its parent).
     If neither has a variant, show the message again with the chosen path.
   - Chosen installs join the list and go through the unchanged identity and
     scope confirmation.
   - Do the same for step 2 when Dropbox is not found, with "Choose your
     Dropbox folder…". The shared folder stays `<Dropbox>\WoW Sync`.
   - Add app tests for the empty-state text and the root/variant
     normalization.
3. **Diagnostic log (PLAN goal 4).**
   - The default is `<state dir>\logs\wow-sync.log`, rotated to
     `wow-sync.log.1` at 1 MiB. `WOWSYNC_LOG` overrides the path. A
     `--state` instance logs under its own state dir.
   - Each line has a UTC timestamp, the pid and a level.
   - Log:
     - start, version and dev mode;
     - each discovery probe with its result (found / missing / no variants);
     - the `product.db` paths found;
     - the Dropbox result;
     - setup step transitions and picker choices;
     - service errors;
     - each blocked or unrecognised add-on with its reason (folder name and
       reason only).
   - Write `~` instead of the user-profile prefix.
   - Never log add-on or settings contents, or account, realm or character
     names.
   - The log must not break the 100 MiB memory budget or the startup time.
     Write synchronously with a small buffer, or use a bounded channel.
   - Make "Open data folder" (`pages.rs` ~2322) active: it opens the state
     dir in Explorer. Leave "Open shared folder" as is unless it is trivial.
   - Update the `main.rs` header comment.
4. **Release (PLAN goal 5).**
   - Bump the workspace version to 0.3.0 and update `Cargo.lock` with
     `cargo update -w`.
   - Write `docs/release/v0.3.0.md` in the style of v0.2.0, covering:
     - finding WoW on any drive, and the picker;
     - QuestieDB-style large `.toc` files;
     - unrecognised add-ons syncing, using R7-sync's user text (ask the
       orchestrator for it if it hasn't merged yet; leave a marked TODO
       until then);
     - where the log is;
     - remaining gaps.
   - Update `README.md` status and download lines.

## Files you own

- `crates/wow-sync-runtime/src/discover.rs` and its re-exports in
  `runtime/src/lib.rs`.
- `crates/wow-sync-app/src/{setup.rs,main.rs,app.rs,pages.rs,win.rs,world.rs,backend.rs}`.
- `crates/wow-sync-adapters/src/probe.rs`, if you need a drive helper there.
- `Cargo.toml` and `Cargo.lock` (version only), `docs/release/v0.3.0.md`,
  `README.md`, `docs/REPO-MAP.md` if paths change.
- Your required-cases section.

R7-sync owns core and runtime sync, plus `app/src/{features.rs,model.rs,removed_ui.rs}`.
If you need anything there, stop and ask.

## Done

Both tiers pass. Evidence at `docs/runs/run-007/evidence/R7-setup.md`
includes:
- screenshots of step 3 empty and after picking, made with the existing
  `--setup-step` evidence flags against a synthetic install under your lab;
- a sample log, redacted.

Reply with the SHA. Budget about 700 AIU; stop and report at 900.
