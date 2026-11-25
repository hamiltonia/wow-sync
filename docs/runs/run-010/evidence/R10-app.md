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
  `R10-app/progress-installing-1.png`. UIA text sequence: "Checking files" ->
  "Installing 3 add-ons from A-demo: 0 B of 240.0 MB" -> "... 120.0 of 240.0 MB
  (MapPack)" -> "... 240.0 of 240.0 MB (VoiceOver)" -> "Updated 3 add-ons from
  A-demo — see Activity".
- Log (B) shows pass start with trigger, phases, one line per package with
  version/files/bytes/ms, and the pass summary; no paths or file contents.
Screenshots were taken with PrintWindow because the desktop was locked.

## Not run / limits
- A multi-source ("several computers") apply was not produced live; covered by
  unit tests only.
- The owner's instance (LENOVOFISH) and real data were not touched; only my
  captured PIDs were stopped.
- macOS CI not run locally; the new tests use no Windows-only paths.

## Open issues
- A removal pass shows "Installing 1 add-on" in the headline: the progress
  stream does not say whether a package is being removed.
- With three header lines (progress + incoming) the headline sits tight against
  the title bar at 0.7 zoom.
- "Shared this computer's add-on changes" legacy lines still appear between
  grouped entries.