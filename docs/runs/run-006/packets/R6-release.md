# R6-release: version 0.2.0, release notes, README, devkit

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R6-release`, branch `run006/R6-release`.

## Goals (PLAN goal 7)

0. **First, before anything else: unblock `cargo deny`.** A new advisory
   (2026-11-22) against `url` 2.5.8 (`Url::origin()` stack overflow) fails
   `offline/deny` on `main`; it is fixed in `url` 2.5.9. Run
   `cargo update -p url --precise 2.5.9` (or the smallest update that clears
   the advisory). Do not add ignores to `deny.toml`. Pass both tiers and
   commit this alone (`run006(R6-release): url 2.5.9 for <advisory id>`).
   Reply to the orchestrator with that SHA right away, then continue with
   the rest of the work.
1. **Version.** Workspace version `0.2.0`. Update whatever else the
   `release.yml` tag check and the About page read. Do not create a tag.
2. **Release notes** `docs/release/v0.2.0.md`, in the style of `v0.1.0.md`:
   - **Real installs work without opt-in.** "Back up your
     `Interface\AddOns` and `WTF` folders first" is advice, not a gate.
   - **First-run safety:** anything replaced or removed can be restored
     from the app. Word this only from R6-gate's evidence; until then, use
     the placeholder `TBD-R6-gate` and list it in your evidence.
   - **The Forever toc rule:** Forever and Camelot, then Mainline, then
     plain. Its source is public documentation; the owner's in-client check
     is pending.
   - **Test mode.**
   - **What is still missing:** no installer, signing or macOS binary; the
     SmartScreen steps from 0.1.0; the BLOCKED GUI items from run-005.
3. **`release.yml`.** Make sure it picks up the notes for the tag being
   built, not a hard-coded `v0.1.0.md`. Fix it if needed.
4. **README.** Change the status lines to describe run-006 (active) and the
   real-install change. Keep the run-005 report link. Do not link files that
   do not exist yet.
5. **devkit and test helpers.** `crates/wow-sync-app/src/bin/devkit.rs` and
   any scripts or docs that tell users or testers that the marker is
   required: update the wording. Keep the marker writer, because test mode
   uses it.

## Files you own

- `Cargo.toml` (version field only) and `Cargo.lock` (the version change
  and the `url` update).
- `docs/release/v0.2.0.md`, `.github/workflows/release.yml`, `README.md`,
  `crates/wow-sync-app/src/bin/devkit.rs`, `docs/USAGE*.md` or similar user
  docs if they exist.
- Your required-cases section.

## Done

Both tiers pass. Evidence at `docs/runs/run-006/evidence/R6-release.md`.
Reply with the SHA. Budget about 400 AIU; stop and report at 600.
