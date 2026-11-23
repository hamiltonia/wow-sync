# Run 006 report

Closed 2026-11-22. Authority: D-28 and the approved [PLAN.md](PLAN.md).
Ceiling 10,000 AIU; used **3,779.4** ([USAGE.md](USAGE.md)).

## Outcome

`v0.2.0` Windows pre-release published from tag `v0.2.0` at `5a18370`
(release run 45094412098):
<https://github.com/hamiltonia/wow-sync/releases/tag/v0.2.0>, with
`wow-sync-0.2.0-windows-x64.zip` and its `.sha256`. CI 45092557330 on
`5a18370` passed; both local tiers passed on the tagged commit.

## What changed (D-28)

- **No opt-in.** Q-24's synthetic-marker gate is off by default
  (`MarkerPolicy::NotRequired`). Real installs enroll through the GUI.
- **Test mode.** A Settings toggle, off by default. While on, only folders
  with the synthetic test marker sync; unmarked installs stop with a plain
  explanation, a "Paused by test mode" header and a "Turn off test mode"
  offer. This is the orchestrator's reading of the owner's "test opt-out".
- **Forever toc rule** (`profile:wowf/2`, Q-3 working answer from
  warcraft.wiki.gg): `_Forever`/`_Camelot`, then `_Mainline`, then plain;
  disagreement inside a tier is ambiguous. Enrollments made under
  `validated-plain/1` reconfirm once and lose nothing.
- **First run loses nothing.** No first-sync delete or replace without a
  recovery point; a joiner's copies of group-removed add-ons are kept;
  same-name, same-version, different-content copies are left untouched.
- **Plain refusal text** for every refusal/layout/block reason; "Try again"
  only when an outside fix can work.
- `url` 2.5.9 (advisory fix); version 0.2.0.

## Verification

| Packet | Result | Evidence |
|---|---|---|
| R6-gate, R6-app, R6-release | merged, both tiers green | [R6-gate](evidence/R6-gate.md), [R6-app](evidence/R6-app.md), [R6-release](evidence/R6-release.md) |
| R6-review (claude-opus-5.5) | 0 critical/major; RV6-01..09 fixed | [R6-review](evidence/R6-review.md) |
| R6-check (gpt-6.1-sol), live on the dev PC's real `_forever_` | G1 minor, G2 major, G3 tooling; real data restored, exact manifest equality | [R6-check](evidence/R6-check.md) |
| R6-recheck (gpt-6.1-sol), live | G1, G2, H1 fixed and passed; H2 passed on lab copies; real data restored, exact equality | [R6-recheck](evidence/R6-recheck.md) |
| R6-settings | H2 fix: uninstall removes owned settings without settings-sync consent | [R6-settings](evidence/R6-settings.md) |

Live findings and resolution:

- **G1 (minor):** Finish setup returned to Welcome. Fixed (`078fe45`).
- **G2 (major):** an unrelated add-on with an unreadable toc (QuestieDB's
  oversized tocs) froze rollback convergence for healthy add-ons. Fixed
  (`57dbe1f`): an incomplete inventory keeps known folders' identity; absence
  stays Unknown, never evidence of removal.
- **G3:** verifier tooling picked a 1×1 IPC window; not a product defect.
- **H1 (major):** a newer local copy after a group rollback was set aside with
  no `NewerAfterRollback` notice. Fixed (`6265778` merge of `dcdce7b`): the
  copy is held unpublished and Needs attention offers Use or Stay.
- **H2 (major):** uninstall left settings when settings sync had no consent.
  Fixed (`21c05dd`). A second live failure was an invalid fixture
  (`<Variable>.lua`); WoW writes `<AddOnFolder>.lua` (31/31 real files), as
  settings.md §1.1 states. The lab recheck with the real shape passed.

Not run: WoW-running blocking on real data (no stand-in launched); the
test-mode write-failure path against an unwritable state folder.

## Incidents

- I-6: a leftover run-003 stand-in `Wow.exe` had run since 2026-11-17; stopped
  by PID before live checks. Its directory
  `D:\GitHub\wow-sync.worktrees\R3-ui-build` is not a registered worktree and
  was left on disk for the owner to delete.

## Open for the owner

- Optional: confirm the Forever toc order in the client (Q-3).
- Confirm or correct the "test opt-out" reading (test-mode toggle).
- Uninstall now removes attributable settings in accounts not opted into
  settings sync (with recovery). Say if this should be narrower.
- macOS run ([../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md)).
- Proposed next spike: the Dropbox public API (owner request; D-21).
