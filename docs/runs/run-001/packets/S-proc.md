# Work packet S-proc: WoW/updater activity and file stability

Role: implementation. Model: claude-sonnet-5.5 (D-04). Common rules:
[S-common.md](S-common.md). ID `S-proc`; spike dir `spikes/proc/`.

## Intent

SPEC §7 "Session completion, updater stability and game-launch-during-operation
races"; R-11 (WoW running blocks game-file changes; idle updater presence alone
does not). Detect without killing, injecting or automating anything.

## Work

1. Read-only research of install layout: list executable/launcher names under
   `D:\Games\World of Warcraft` (names only) and document which process names
   identify a running client per variant and the Battle.net launcher/agent
   (`Battle.net.exe`, `Agent.exe`, etc.). Do not start them.
2. Spike in `spikes/proc/` (Rust, Windows):
   a. Process detection by image path (not only name) so a running client can
      be tied to an install root/variant. Test with **synthetic stand-ins**: a
      trivial exe you build, copied to a temp dir under a client-like name
      (e.g. `%TEMP%\wowsync-proc\_forever_\WowClassic.exe`). Never use the real
      WoW directory for stand-ins.
   b. Who-has-this-file-open: Windows Restart Manager
      (`RmStartSession`/`RmRegisterResources`/`RmGetList`) on synthetic files
      held open by a stand-in, with and without write sharing.
   c. File-stability heuristic for "session complete/updater idle": size/mtime
      quiescence window, exclusive-open probe (`CreateFile` with no sharing,
      immediately closed, read-only access) and whether probing itself can
      disturb a writer. Measure detection latency.
   d. Launch-during-operation race: simulate a stand-in client starting while a
      synthetic multi-file write is in progress; show what the spike can detect
      and the remaining TOCTOU window. Options (e.g. write staging + atomic
      rename, re-check before each commit point).
3. Notification approach: polling interval vs WMI process-start events vs
   ETW; measure CPU cost of polling at 1 s on this PC.
4. Record what cannot be verified without a real running client (BLOCKED;
   owner-run manual check list).

## Owned writes

`spikes/proc/`, evidence files (S-common). Temp files only under
`%TEMP%\wowsync-proc\`; clean up after.

## Done when

Evidence covers 2a–2d and 3 with real results, plus recommendations and race
notes.
