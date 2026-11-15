# S-proc evidence: WoW/updater activity and file stability

Run 001, Phase 2. Packet: [S-proc](../packets/S-proc.md). Branch `run001/S-proc`.
Spike code: `spikes/proc/` (Rust, windows-sys 0.59, own workspace).
Raw output of the final run: [suite-output.txt](S-proc/suite-output.txt).

Environment: Windows 11 Pro (Insider 26300) x64, i7-13700HX, non-elevated
(medium integrity) user, Rust 1.100.0 release build. Not official support
evidence. macOS: BLOCKED (no Mac; not attempted).

## SPEC §7 question and answer

Question: can the app reliably tell that the game client is running (or an
updater is active) and that game files are stable, without killing, injecting
or automating anything, and what remains of the launch-during-operation race?

Answer: **feasible on Windows, with a residual race that cannot be closed from
outside the game.** Confidence: medium-high for detection, high for the race
conclusion. Process detection by image path, Restart Manager holder lookup and
a quiescence-plus-probe stability rule all work on synthetic stand-ins. No
user-space mechanism can stop a client starting between the last check and the
write; it can only be made small and made harmless by staging and recovery.
Real-client behaviour is **NOT RUN** (see section 4). It would change if the
real client holds handles or spawns helper processes unlike the stand-ins.

## 1. Install layout (read-only, names only)

`D:\Games\World of Warcraft` top level: `_forever_`, `.battle.net`, `Data`,
`World of Warcraft Launcher.exe`. Executables found (depth 3):

- `_forever_\Wow.exe` (client for this "Forever" install; the root `.flavor.info`
  and the `_forever_` folder are the variant identity, verify per SPEC)
- `_forever_\BlizzardError.exe` (crash reporter)
- `_forever_\UTILS\BlizzardBrowser.exe`, `_forever_\UTILS\WowVoiceProxy.exe`
  (helpers)
- `World of Warcraft Launcher.exe` (root launcher)

Only the Forever variant is installed here. Names for other variants
(`WowClassic.exe` for Classic Era, `WowT.exe`/`WowB.exe` for PTR/Beta and
similar) and Battle.net `Battle.net.exe`/`Agent.exe` come from general
knowledge, not from this machine, and are **unverified**. The spike therefore
keys on image path under an install root, with the variant taken from the first
path component and the name list only used to classify client vs launcher vs
other. Battle.net/Agent were not located or started.

At the time of the runs no process had its image under the real install root
(**PASS**, enumeration only).

## 2a Process detection by image path: PASS (synthetic stand-ins)

Method: the spike exe copied to temp as `Wow.exe`, `WowClassic.exe`, a launcher,
`tool.exe`, a prefix-sibling `installA_evil\...\Wow.exe` and a second install
`installB\...\Wow.exe`; enumerated with Toolhelp snapshot, then
`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` and `QueryFullProcessImageNameW`.

- Name-only matching flags 4 processes, 2 of them decoys. Path matching under
  `installA` flags exactly the 2 true clients, assigns variant folders
  (`_forever_`, `_classic_era_`), classifies the launcher and `tool.exe`
  separately, rejects the `installA_evil` prefix, and is case-insensitive.
- A non-elevated process could not open about 167 of 360 processes (system and
  protected). Those cannot be tied to a root, which is acceptable for game
  clients running as the same user, but an elevated or other-session client
  would be invisible: **unknown**, needs the owner check in section 4.
- Cost: see section 3.

## 2b Restart Manager: PASS (synthetic, same-user stand-ins)

`RmStartSession`/`RmRegisterResources`/`RmGetList` on files held by a separate
stand-in process opened with share none, read, read+write and read+write+delete:
in all four cases RM returned the holder pid and image name; after the holder
exited it returned nothing; unheld and missing files returned nothing. The
exclusive probe agreed (held means probe fails). RM also reports non-target
holders such as the spike's own read handle (any process, so AV or indexers
would appear too). Latency was 100-175 ms per call (one file). RM was run
non-elevated and only same-user processes were tested; cross-user/service
holders **NOT RUN**.

## 2c File stability heuristic: PASS with a significant caveat

Rule tested: declare "stable" only when the size+mtime has been unchanged for
500 ms AND an exclusive-open probe (`CreateFileW`, `GENERIC_READ`, share 0,
closed immediately) succeeds; polled every 100 ms. Three synthetic writer
patterns, three detectors run side by side:

| pattern | meta-only | probe-only | meta+probe |
| --- | --- | --- | --- |
| hold handle, append every 200 ms | +360 ms after close | +59 ms | +360 ms |
| open-write-close bursts every 300 ms | +257 ms | **-1652 ms, false idle** | +258 ms |
| handle held silently, no writes | **-1247 ms, false idle** | +60 ms | +59 ms |

Neither signal alone is safe; the combination was correct in all three. Typical
detection latency is the quiescence window plus one poll (about 0.3 s here;
configurable). Quiescence longer than a real updater's write gaps is needed;
real gap length is not known (section 4).

**Disturbance (FAIL for the exclusive probe as a continuous check).** A writer
looping open-append-close with share read for 3 s, while a prober ran:

| prober | writer open attempts | failed opens |
| --- | --- | --- |
| none | 47,244 | 0 |
| exclusive probe hot loop | 269,278 | 268,944 |
| exclusive probe every 100 ms | 74,865 | 38,473 |
| Restart Manager every 5 ms (10 calls) | 75,959 | 9,994 |

An exclusive probe can make a concurrent writer's open fail with a sharing
violation, and with a 100 ms cadence the failures were far more than the
probe's own microsecond hold time explains (hypothesis, not tested: filter
drivers such as Defender extend the open/close; cause **unknown**). RM
calls also disturbed this writer. An updater that treats a sharing violation as
fatal could therefore be harmed. These counts are for a retry-less hot loop,
not a real updater, so real harm is unquantified.

## 2d Launch-during-operation race: PASS (detectable); race remains

Setup: an applier thread performs an 8-file staged write (write stage file,
25 ms, then rename into place), and a client stand-in is started at offsets
5-195 ms. Commits are classified by the spawn timestamps: safe (rename finished
before CreateProcess), violating (rename began after CreateProcess returned),
ambiguous (overlapping). 20 trials per mode:

| mode | safe | ambiguous | violating | trials with any non-safe commit |
| --- | --- | --- | --- | --- |
| no check | 66 | 8 | 86 | 20 |
| 1 s poll sets an abort flag | 66 | 2 | 84 | 19 |
| re-check before every rename | 45 | 0 | 1 | 1 |

- Re-checking at every commit point eliminated nearly all violations (all 20
  trials aborted when the client appeared) but not all: 1 commit landed after
  the client existed. The window between the check returning and the rename
  completing was up to 28 ms in the worst trial (mean far lower; typical scan
  cost 10 ms, see section 3).
- A poll-only guard (1 s) is nearly useless against a short multi-file write.
- The window cannot be closed: nothing prevents the client starting after the
  check; the client can also start between files and read a mixed old/new set.

Options for design (not decided here): (1) stage everything and commit with
re-check immediately before each rename, abort and roll back if a client
appears (rollback of already-renamed files is then needed); (2) keep the commit
phase short (fewer, smaller renames) and apply the set as one unit (for example
swap a directory or write a manifest last); (3) after the commit, re-check and
if a client appeared during it, mark the apply "unconfirmed" and keep the
recovery copy; (4) treat a launch during an operation as a first-class
outcome in the UI, not an error; (5) prefer a cheaper "is a client running"
check at the commit point than a full process scan.

## 3 Notification approach: polling measured, WMI/ETW partly

Polling (names-only path resolution, 10 trials each, latency from
CreateProcess return to detection):

| interval | min / mean / max latency | CPU (this process, one core) |
| --- | --- | --- |
| 1000 ms | 54 / 446 / 943 ms | 0.93% (0.281 s CPU in 30 s) |
| 250 ms | 10 / 133 / 259 ms | not measured |
| 100 ms | 6 / 27 / 69 ms | 7.3% (0.734 s in 10 s) |

A scan costs about 10 ms (snapshot dominates; names-only 10.1 ms vs resolving
every process 16 ms). Full-scan polling at 1 s measured 1.18% of one core.
Those are CPU time numbers for a release build on a 24-thread laptop CPU, not a
power measurement; memory was not measured here (the 100 MiB background gate
belongs to the app, not this spike).

- WMI `Win32_ProcessStartTrace`: **BLOCKED** (access denied; needs elevation,
  which a background app should not require).
- WMI `__InstanceCreationEvent WITHIN 1` via `Register-CimIndicationEvent`:
  **PASS** non-elevated, 4 runs, 840/297/282/268 ms including the cost of
  `Start-Process` (first run includes warm-up). Not better than 250 ms
  polling, adds the WMI service dependency and an in-process PowerShell was
  used, so not representative of Rust. An earlier `Register-WmiEvent` attempt
  with a name filter was inconclusive (no events within 5 s, cause unknown).
  The CPU cost of WMI was **NOT RUN**.
- ETW kernel process provider: **BLOCKED** (needs admin/kernel session); not
  attempted.

Recommendation: poll while sync is active or idle-watching, at 1 s with a
names-only scan (about 1% of one core, up to about 1 s detection latency),
tighten to 100-250 ms during a commit phase, and always re-check at each
commit. Revisit only if the owner accepts an elevated helper for ETW.

## 4. Cannot be verified without a real running client (BLOCKED, owner-run)

None of the following was run. WoW, Battle.net and any launcher were never
started, stopped or touched.

1. With the real client running (Forever and any other installed variant),
   confirm the image path is under the install root and under which variant
   folder, and whether any helper processes (BlizzardBrowser, WowVoiceProxy)
   appear and should count as "client running".
2. Which files in `_forever_\WTF` and `_forever_\Interface\AddOns` the client
   holds open while at the login screen, in-world, and after logout, using the
   spike's RM check on a copy (not the live files unless approved).
3. When the client exits, how long until saved variables are written and handles
   are released (this sets the quiescence window).
4. Battle.net/Agent: process names and image paths, whether an idle launcher
   holds any addon/WTF handles, and what an update looks like (file churn
   pattern, gap lengths, any exclusive opens).
5. Whether an elevated or other-session client is visible to a non-elevated
   scan (here 167 of 360 processes could not be opened).
6. Whether the exclusive probe or RM disturbs the real updater, and the cause of
   the high failure counts in 2c (Defender or other filter drivers).
7. Launch-during-apply behaviour with a real client (does it fail, wait, or read
   a mixed set).

## Limits and risks

- Stand-ins are the same binary under a client-like name; they do not model
  real handle or helper behaviour, or process tree structure.
- Timing numbers are from one machine, one run (race: 20 trials/mode with fixed
  offsets), under normal desktop load. They show feasibility, not guarantees.
- Names-only scanning misses a renamed executable; full scan costs about 6 ms
  more per poll.
- The 2c disturbance test is a worst-case retry-less loop.

## Rerun

```
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd D:\GitHub\wow-sync.worktrees\S-proc\spikes\proc
cargo build --release
.\target\release\wowsync-proc-spike.exe suite   # or t2a t2b t2c t2d t3
```

Temp data lives under `%TEMP%\wowsync-proc\` and was removed after the runs.
The WMI checks were ad hoc PowerShell one-liners using the spike exe as
`Wow.exe` under that temp folder (not in the repository).
