# R10-app evidence

Branch run010/R10-app, merged with main a6d898a (R10-history). Version 0.3.3.

## Implemented
Activity grouped by pass with per-add-on rows and capped file lists; add-on
"Recent changes"; throttled (500 ms) progress headline/tray with an incoming
summary and a 20 s "Updated N add-ons" note; log lines for pass start, phases,
each package (with ms) and the pass summary. `source: None` reads "from several
computers" (never "this computer"; a single package omits it).
`decision_satisfied` is matched on display names only. The backend's own Idle
event is skipped when the service already ended the pass with Idle.
Round-2 fix found in the live run: clicking a file-list row also folded its
group (click bubbled to the group); the row now stops propagation.

## Results
- cargo test -p wow-sync-app: passed (offline tier includes it), Windows, local.
- cargo xtask verify --tier offline / --tier native: both PASS (Windows, local).

## Live synthetic run (Windows, debug build)
Two instances from this worktree, `--dev`, state/provider under
`D:\WoWSyncLab\R10-app\` (A-demo, B-demo, synthetic installs, random-byte
files). A published 4 add-ons; B joined and installed them. A then added two
large add-ons and updated one (192 MB), removed one, and later added 240 MB
more; B applied each. Observed on B:
- Activity: "3 add-ons updated from A-demo (+192.0 MB)" expands to one row per
  add-on (version change, +/~ file counts, size, reason); a row expands to its
  relative file paths. Screenshot `R10-app/activity-expanded.png`.
- Add-on page "Recent changes": `R10-app/addon-recent-changes.png`.
- Headline during a 240 MB apply, with the incoming line under it:
  `R10-app/progress-installing.png`. UIA text sequence: "Checking files" ->
  "Installing 3 add-ons from A-demo: 0 B of 240.0 MB" -> "... 120.0 of 240.0 MB
  (MapPack)" -> "... 240.0 of 240.0 MB (VoiceOver)" -> "Updated 3 add-ons from
  A-demo — see Activity".
- Log (B) shows pass start with trigger, phases, one line per package with
  version/files/bytes/ms, and the pass summary; no paths or file contents.
Screenshots were taken with PrintWindow because the desktop was locked.

## Round 3 (main 1dfe772)
- `Phase::Removing`: "Removing Atlas" / "Removing 1 add-on" / "Removing N add-ons:
  i of N (name)"; never "Installing" for a removal. Mixed passes follow the
  service's per-package phase. Live: A removed 6 add-ons; B showed
  "Removing 2 add-ons: 1 of 2 (ZoneArt)" then "Removing 4 add-ons: 1 of 4
  (SpokenAudio)" (`R10-app/progress-removing.png`). That screenshot predates
  hiding the "Incoming: ... 0 B" line for a removal (now hidden; unit-tested,
  not re-captured).
- Audit lines "published seq N (0 new objects)" are dropped from Activity and
  the Overview list; "(K new objects)" reads "Shared K new items from this
  computer" (objects are not add-on counts, so the wording says items). Verified
  live: the earlier "Shared this computer's add-on changes" lines are gone.
- Header: fixed 76 px height replaced by a minimum height with vertical padding,
  so progress and incoming lines grow downward (see screenshots; first line no
  longer touches the title bar). Fixed at 0.7 zoom only.

## Not run / limits
- A multi-source ("several computers") apply was not produced live; covered by
  unit tests only.
- The owner's instance (LENOVOFISH) and real data were not touched; only my
  captured PIDs were stopped.
- macOS CI not run locally; the new tests use no Windows-only paths.

## Open issues
- A mixed update-and-remove pass was not produced live; the removal and install
  passes were run separately.
## Round 4 (RV10-02, RV10-07, release notes)

- RV10-02: `model::progress_headline` lets a routine phase (scanning, deciding, publishing) take the headline only when no decision is pending; otherwise the progress shows as the quiet line under it and the tray stays on the hourglass. Staging, installing and removing still take over. The pump no longer clears the "Updated N add-ons" note when the next pass starts; it clears it on real work or after NOTE_MS (20 s). Test: `model::tests::a_routine_check_never_replaces_a_waiting_decision`, `progress_ui::tests::the_note_survives_idle_passes_for_its_full_lifetime`.
- RV10-07: an idle pass is silent (a "slow check" line if 2 s or more) and is not shown before 1.5 s; a pass that works, or was asked for or follows a change, logs in full. Tests: `a_quick_idle_pass_is_silent_and_never_flashes_the_headline`, `a_slow_idle_pass_is_one_line_and_is_shown_once_it_lasts`, `real_work_replaces_the_note_and_a_manual_pass_is_logged_in_full`.
- Release notes rewritten to claim only this behaviour; RV10-01/03/04/05/06/08 are listed as limits because the R10-history round-4 fixes were not on main when written.
- Not re-run live; verified by unit tests only. No new screenshots.

## Round 5

- Release notes: the limits fixed by R10-history round 4 (RV10-01/03/04/05/06/08) are gone; what remains is source wording that can change mid-install (RV10-09), display-only clock skew in ordering, and the "unknown" old version for crash-recovered installs.
- An empty `ContentId("")` renders as "unknown" (`history_ui::tests::a_recovered_install_with_no_old_content_id_says_unknown`).
- The "tray updated" log line was written on every pass with the same tooltip (about 2 lines per pass while a decision waited); it now logs only when the tooltip changes.
- Live lab (synthetic, `D:\WoWSyncLab\R10-app`, computers A-demo and B-demo, PIDs 18264 and 5152, both stopped): A gained six 60 MB add-ons; B's headline read "Installing 6 add-ons from A-demo: 0 B of 360.0 MB", then named ShotAddon1 to ShotAddon6 in turn, ending "Updated 6 add-ons from A-demo — see Activity". Before the apply, B showed "Waiting for your decision on 2 add-ons" through every idle pass (about 25 s, 1.5 s interval) and the log had no lines for those passes. Screenshots: `progress-installing.png` (new, taken with PrintWindow) and `waiting-decision-stays.png`.
- The two waiting decisions came from my own test setup (add-ons written to A after being added), not from a product fault.
## Round 7 - macOS backend test flake

- Root cause 1 (test setup, predates run-010): `install_files` wrote add-on files and the test at once computed the enroll digest with `recent_ms: 0`. `fs_scan` marks a folder Unstable when its newest mtime >= now truncated to whole ms; macOS has fine-grained mtimes, so the add-on was skipped, the digest differed, and enroll failed with StaleConfirmation. Windows' coarse timestamps hide it. Fix: `install_files` now lets the clock pass the write time (25 ms) before returning.
- Root cause 2 (mine, round 4): `progress_ui::TRIGGER` is process-wide, so parallel backend tests' sync threads set it and a Pump in `the_note_survives_idle_passes_for_its_full_lifetime` read it. Test builds now never read the global (`Pump::next_trigger`).
- xtask offline test failures now print each failing test's panic message (`failure_section`; test `each_failed_test_prints_its_panic_message`). Backend `wait_for` timeouts report the service state.
- CI (draft PR #3, not merged): 45544671050 pass (before fix, flaky), 45546673082 fail (diagnostic run exposing StaleConfirmation), 45548002676 fail (trigger global), then three consecutive macOS passes: 45549066747, 45550393887, 45551929420.
- Local: `cargo xtask verify --tier offline` and `--tier native` PASS.
