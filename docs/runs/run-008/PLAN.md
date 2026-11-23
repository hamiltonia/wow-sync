# Run 008 plan

Approval: D-30 (2026-11-23). There is no AIU ceiling; usage is tracked in
[USAGE.md](USAGE.md). Target: the `v0.3.1` Windows pre-release.

## Problem

On RYZENFISH (WoW at `G:\Games\World of Warcraft`, fast NVMe drive, 31k
add-on files / 3.2 GB, no cloud or reparse entries), `v0.3.0` setup found
the install. Choosing `_forever_` left setup on "checking" indefinitely, and
tray Quit did nothing. The owner ended the process in Task Manager.

- **Log:** `D:\Dropbox\Gaming Sync\wow-sync\logs\wowsync-RYZENFISH-20261123-132100.log`
  shows the game folder accepted at 21:23:57, no `inspected _forever_` line
  afterwards, and tray clicks that did not quit.
- **Probe:** `...\logs\inspect-RYZENFISH-20261123-133055.txt`.
- **Code facts:**
  - `Cmd::Inspect` runs inline on the backend thread (`backend.rs` ~365).
  - Quit is `Cmd::QuitWhenSafe` on the same queue (~504), and the UI exits
    only after the backend publishes `quit_ready` (`app.rs` ~562).
- **Not explained yet:** near-identical data inspected quickly on the dev PC
  during R7-check. The cause is unknown; candidates include a hang or deadlock
  in the inspect path (locks, provider/Dropbox access, the state dir), work
  that grows with WTF size or account count, or a UI preview-matching bug.

## Goals

1. **Root cause:** find the inspection hang or slowness, prove it with a
   reproduction, and fix it.
2. **Quit always exits** within a few seconds, whatever the backend is doing.
   A read-only inspection is abandoned. A write step in progress still
   finishes safely (existing quit-when-safe semantics).
3. **Long checks are visible and cancellable:** setup shows progress during a
   read-only check, and Back or choosing another install cancels it.
4. **Default selection:** setup selects a usable install (not refused) by
   default; a refused one stays listed with its reason.
5. **Log:** record when each inspection starts (target folder), when it ends
   (result and duration) or is cancelled, and which tray item was clicked
   (open, quit, …).
6. Publish `v0.3.1`, with release notes.

## Packets and order

| ID | Role / model | Depends on |
|---|---|---|
| R8-fix | implementation / claude-sonnet-5.5 | — |
| R8-review | review / claude-opus-5.5 | R8-fix merged |
| R8-check | verification / gpt-6.1-sol | R8-fix merged, dispatch artifact |

The orchestrator gates, merges with `--no-ff`, runs both verify tiers, pushes
and watches CI.

## Gates for `v0.3.1`

- Both verify tiers pass; CI is green, including macOS `verify-offline`.
- The release dispatch build succeeds.
- R8-check reports PASS for:
  - the reproduction of the hang (or the closest faithful reproduction) now
    finishing;
  - Quit during a check exiting within 5 s;
  - progress and cancel;
  - default selection;
  - the log lines;
  - real-data read-only inspection of `_forever_` on the dev PC, using the
    release build without `--dev`;
  - an artifact smoke test.
- No open critical or major review findings.

## Stop conditions

- The root cause needs a product-intent change (SPEC) → ask the owner.
- The fix would need new access to real data beyond read-only inspection on
  the dev PC → ask the owner.
- Any irreversible action on user data → ask the owner.
