# Run 007 report

Closed 2026-11-23. Authority: D-29 and the approved [PLAN.md](PLAN.md).
Ceiling 4,000 AIU; used **3,084.4** ([USAGE.md](USAGE.md)).

## Outcome

The `v0.3.0` Windows pre-release was published from tag `v0.3.0` at `a09d233`
(release run 45257886538):
<https://github.com/hamiltonia/wow-sync/releases/tag/v0.3.0>, with
`wow-sync-0.3.0-windows-x64.zip` and its `.sha256`. CI 45257882100 on
`a09d233` passed. Both local tiers passed on the last code merge (`b537000`),
and only evidence was merged after it.

## What changed (D-29)

- **Install discovery (A-47).** Discovery searches every fixed drive from A to
  Z, plus the Battle.net launcher record (`product.db`, read-only and limited
  to 4 MiB). When no install is found, setup steps 2 and 3 offer
  "Choose folder…".
  - A picked folder is checked off the UI thread, and the page shows
    "Checking…" meanwhile.
  - A Dropbox folder outside Dropbox's own list gets a warning, but is still
    accepted.
  - An enrolled computer starts syncing before the full drive search runs.
- **Large `.toc` files.** A `.toc` file over 256 KiB is read from its header.
  It counts as recognised only if the header visibly ends inside the part that
  was read.
- **Unrecognised add-ons sync (A-48).** An add-on that cannot be recognised
  (no `.toc`, an ambiguous `.toc`, or one that cannot be read) is synced by
  its content and shown as "Unrecognised: synced by content — <reason>".
  - The order is decided by **published lineage**: `code-state/1` records
    carry an optional `lineage` field naming the copies each change replaced.
  - A copy wins only when the other copy is one of its ancestors. A computer
    that was offline cannot bring back an old copy.
  - Copies that changed independently, and reverts, raise one decision.
  - A copy that is still being edited waits, and no decision is raised for it.
  - Replaced copies are kept for recovery.
  - Only lineage from a publisher that holds that copy is trusted.
  - Settings of these add-ons are not synced, because they declare none.
- **Diagnostic log**, on by default, at `<state>\logs\wow-sync.log`. It
  rotates at 1 MiB to `.log.1`, writes the user profile as `~`, can be moved
  with `WOWSYNC_LOG`, and records why each add-on is blocked or unrecognised.
  - "Open data folder" and "Open shared folder" now work.
- Version 0.3.0.

## Verification

| Packet | Result | Evidence |
|---|---|---|
| R7-sync, R7-setup | merged; both tiers green at every merge | [R7-sync](evidence/R7-sync.md), [R7-setup](evidence/R7-setup.md) |
| R7-review (claude-opus-5.5), review plus 2 rechecks | RV7-01 critical, RV7-10 major and 12 minor findings. All fixed, except that RV7-05 (warning instead of refusal) is accepted as designed | [R7-review](evidence/R7-review.md) |
| R7-check (gpt-6.1-sol), live lab, real `_forever_` data and the release artifact, plus 2 rechecks | G1 major and G2 minor, both fixed and passed live. Real data: QuestieDB published, exact restore proven | [R7-check](evidence/R7-check.md) |

Key findings:

- **RV7-01 (critical):** the first lineage design was stored only on each
  computer, so a returning offline computer could put an old copy back on
  every computer. Lineage was redesigned to be published, and the review's
  exact probe now passes.
- **RV7-10 (major):** the real sync service never passed this computer's own
  last published record into the newer-copy decision. Fixed with a new
  `local_prev` input, and both new end-to-end tests fail without the fix.
- **G1 (major, live):** an unstable mid-edit publication dropped lineage. This
  caused a needless decision on a single edit, and a live revert was silently
  undone. Fixed by carrying lineage forward.
- **G2 (minor, live):** a live edit briefly offered "Choose a copy". Fixed by
  holding the decision while any copy is changing. All 174 GUI samples during
  the live recheck showed zero decisions.

**Blocked:** the A-47 setup screen for when no install is found could not be
shown in the GUI on the dev PC. Normal discovery finds the real D: install, and
there is no safe way to provide an empty install list. The not-found path is
covered by code-path tests only.

**Partial controls:**
- **G2 end-to-end test:** it passes without the fix; only the core test and the
  live recheck prove G2.
- **RV7-10 revert test:** its control run was stopped after about 4 minutes,
  so it has no proven failure without the fix. Live rechecks covered the
  revert instead.

## Open for the owner

- The D-29 defaults were taken from "approved, continue" after the form went
  unanswered: the fallback rule for unrecognised add-ons and the 4,000 AIU
  ceiling. Correct them if they are wrong.
- RV7-05: a picked Dropbox folder that Dropbox does not list gets a warning
  but is still accepted. Say if you want it refused.
- RYZENFISH: install `v0.3.0`. Discovery should find
  `G:\Games\World of Warcraft` from the launcher record. If it doesn't, use
  "Choose folder…". The log is at
  `%LOCALAPPDATA%\WoWSync\state\logs\wow-sync.log`.
- Carried over from run-006: the Q-3 Forever toc order, the test-mode reading,
  the macOS run, and the Dropbox API spike.
