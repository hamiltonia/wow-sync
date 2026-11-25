# R10-check — independent verification of candidate 374a1a3

## Verdict

**DO NOT RELEASE this candidate.** G2, G3 and G4 are major visibility/history
defects on the original candidate. The tray-progress gate remains BLOCKED.
The authorized follow-up below closes the headless speed gate at **3.19×**.
Passing developer tests did not establish the other GUI requirements.

| Check | Result | Independent result |
|---|---|---|
| 1. History | **FAIL** | Update, addition, removal and restart persistence worked. Peer choices/rollback have the wrong reason; settings history lacks versions; addon recent-history rows omit sources. G1–G3. |
| 2. Live progress | **BLOCKED** | Headline, incoming summary and cleared decision text PASS during a 1 GiB apply. Actual tray tooltip observation was blocked on this desktop. |
| 3. Logs | **FAIL** | Receiving-side phase/package logs and their privacy PASS. A real publishing pass was reported as an idle check with zero changes, without package lines. G4. |
| 4. Speed/cache | **PASS** | Authorized fair headless follow-up: v0.3.2 median **927 ms**, candidate **291 ms**, **3.19×** faster on the same lab copy. Both read zero files. Cache-seeding test and artifact mutation detection PASS; quiet artifact counter limitation retained below. |
| 5. Memory | **PASS** | Sampled peak aggregate private working set during 1 GiB GUI apply: **37.85 MiB**; entire 90 s observation: 40.27 MiB. Sampling limitations below. |
| 6. Artifact smoke | **PASS** | Dispatch provenance/checksum, packaged version, two GUI enrollments, transfer, restart, hide/reopen and explicit GUI quit executed. |
| 7. GUI idle overhead, report only | **PASS** | Measurements and log-timestamp comparison reported below, including contention and censored baseline results. This is not a passing 3× performance gate or a proven diagnosis of the owner's live instance. |

## Identity, environment and safety

- Worktree: `D:\GitHub\wow-sync.worktrees\R10-check`; branch
  `run010/R10-check`; source `374a1a3dd87d4820c58217920d26a6573f3a0654`.
  Product code was not changed.
- Authority read: AGENTS, README, SPEC, ACCEPTANCE, ENGINEERING,
  OWNER-DECISIONS (D-32), run-010 PLAN/COMMON/packet and predecessor common
  packets. Author/reviewer evidence was context, not an oracle.
- Worker: independent verification, approved role/model assignment
  `gpt-6.1-sol`. The runtime does not expose a model-routing receipt to this
  worker; actual routing must be corroborated by the orchestrator's launch
  record rather than inferred from this assignment.
- Windows 11 Pro x64, build 26300; i7-13700HX; local NVMe lab filesystem.
  This Insider development environment is not stable-release Windows or
  macOS execution evidence. Dark theme, `--zoom 0.75`; UI Automation and
  PrintWindow were available, but direct shell tray-point observation was not.
- `$LAB` below means the approved R10-check lab; `$WT` means this worktree.
  Every executable launch used explicit isolated `--state` and synthetic
  provider paths. Synthetic computers were **LabA** and **LabB**.
- Only the approved real AddOns tree was read/copied. Benchmark copies used
  synthetic `wowf` build metadata. No service/enrollment was run against the
  real installation: `speed_lab` explicitly warns that it may write/lock its
  install. No real settings, real provider group, owner logs/state, demo, or
  owner process were touched. WoW was not started or stopped. Lua/data files
  were never executed. No data was uploaded.
- Captured app PIDs stopped: 38676, 22604, 21828, 1352, 25732, 37528.
  The final smoke instance, 36740, exited through **Quit WoW Sync**.
  Restart/handoff instances 35016, 27252 and 5156 exited themselves.
  No surviving processes were found among these captured PIDs.
- Raw logs, fixtures, screenshots and measurement scripts are retained only
  under `$LAB`, not committed with private/synthetic identity names.

## Artifact provenance and executed verification

`gh run view 45533540848 --repo hamiltonia/wow-sync` and `gh run watch`
were executed. Dispatch completed **successfully** on the exact candidate
SHA. Downloaded with `gh run download ... -D "$LAB\artifact"`.

| Artifact | SHA-256 |
|---|---|
| Candidate zip, matched supplied `.sha256` | `44c39ce50d60cef611883d56693d26704ffdeba3d24cb96afb819fa657db88ea` |
| Extracted candidate `wow-sync.exe` | `20c050e3866bb60cb4e9c8d96ffe9198c2863a252fdc511ab57b4e38132630b4` |
| Published v0.3.2 zip, matched supplied `.sha256` | `bcc1844418fe1d816ad98847c1b12b0e966bbc6b1e6904648706c2db0bfb1c44` |

Baseline came from `gh release download v0.3.2`, not an owner/Dropbox copy.
All GUI behavior below used the extracted dispatch executable, except the
explicit baseline benchmark.

Executed in `$WT`, with the prescribed cargo PATH:

- `cargo xtask verify --tier offline`: **PASS**, including required cases.
- `cargo xtask verify --tier native`: first execution **FAIL**; default
  repeat **PASS**. Both results are retained, not replaced by the repeat.
- `cargo test --locked -p wow-sync-runtime change_history`: **15 passed**.
  The separately reported zero-test `public_api` target was filtered, not
  counted as history evidence.
- `cargo test --locked -p wow-sync-adapters
  an_installed_folder_is_not_read_again_by_the_next_scan_unless_it_changed`:
  **1 passed**, with cache hits, changed-file and full-read controls.
- Existing release examples were built with
  `cargo build --locked --release -p wow-sync-runtime --examples`.
  Initial GUI verification used the published v0.3.2 executable. The later,
  explicitly authorized headless baseline port/build is documented below;
  it was not substituted for the published GUI artifact.

### G5 — minor — intermittent native transport tests

The first native tier had 85 passes and two failures:

- `an_unreachable_endpoint_is_offline`: `Err(Tls)` rather than `Err(Offline)`.
- `redirect_to_http_to_another_host_and_a_fourth_redirect_fail_without_fetching`:
  HTTP counter **1**, expected **0**.

Reproduction command: `cargo xtask verify --tier native` with its default
parallel test execution. A targeted serial
`cargo test --locked -p wow-sync-adapters update_http::tls_tests --
--test-threads=1` passed all six tests; the unchanged default native tier
then passed. Root cause was **not established**; this is intermittent
verification evidence, not a confirmed transport vulnerability or a fixed bug.
Logs: `verify-native.log`, `tls-serial.log`, `verify-native-repeat.log`.

## Check 1: executed synthetic history flows

Launch form:

```text
<artifact exe> --dev --install <lab install A or B>
  --state <isolated A-state or B-state> --provider <lab shared>
  --name LabA|LabB --zoom 0.75 --interval-ms 5000
```

Both installations used `_forever_` and valid synthetic metadata. A created
a group through the setup guide; B joined it through the guide.

| Change on A, received on B | Observed B fields |
|---|---|
| HistoryAddon update | LabA, 1.0 → 2.0, +1/~2/−0 files, +19 B, newer release: **PASS** |
| NewAddon addition | LabA, new → 1.0, +2 files, +82 B: **PASS** |
| RemoveAddon removal | LabA, 1.0 → removed, −2 files, −85 B, removal: **PASS** |
| Keep A's ambiguous ChoiceAddon copy | LabA, 1.0 → 1.0, ~1 file, 0 B; reason **newer release**, not the owner's choice: **FAIL**, G3 |
| A-authored RollbackAddon rollback | LabA, 2.0 → 1.0, ~2 files, −9 B; reason **newer release**, not rollback: **FAIL**, G3 |
| Settings change | A recorded `settings-published` with empty old/new content IDs and zero size delta. A→B settings convergence/receipt history was not established: **FAIL** for recorded metadata, G2; receiving settings flow **BLOCKED**, not claimed passed. |

For rollback preparation, B first published RollbackAddon 1.0 and then 2.0
to A. A's addon recovery page then offered its old 1.0 copy; **Restore this
version → Restore** issued the group rollback. B subsequently received 1.0.

Settings were enabled separately on both computers for one synthetic matching
identity. LabA was selected as master. The addon-named saved file was changed
and **Share my settings** was invoked. The accepted command and the defective
publication record were observed; B's saved-file hash still differed during
the probe. This report does not silently turn that into a passing transfer.

Activity grouping, expansion and capped relative paths worked. The large
addon row had 50 stored paths and `paths_more = 16`. After restart, the
receiving Activity page still expanded the same records.

The complete B `changes.jsonl` was byte-identical before and after restart
and the final smoke session:
`afeb9998d63a37c24792e8d0d025e7d4878719f67d886dca95849c3355176345`.
One immediate post-termination launch exited as a duplicate; a subsequent
real restart succeeded. No new setup or rewritten history was substituted.

### G1 — minor — addon recent-history rows omit each change's source

Reproduce: A publishes HistoryAddon 1.0, then updates it to 2.0; B applies
both. On B open **Add-ons → HistoryAddon**, scroll to **Recent changes**.
The two rows show versions, counts, sizes and reason, but **neither includes
LabA**. The separate latest “What happened” paragraph does show LabA; it
does not supply provenance for each older row. Activity's group source works.
Screenshot: `B-addon-recent.png`.

### G2 — major — saved-settings publication cannot identify either version

Reproduce the scoped master/settings flow above; inspect A's structured
history and Activity. The actual record was:

```text
kind=settings-published, name=HistoryAddon settings, source=ThisComputer
old_version=ContentId(""), new_version=ContentId("")
files=+0/~1/−0, byte_delta=0, reason=SettingsMaster
```

Empty IDs are not short content identities. Thus the UI can only say
“unknown”, not show what settings version changed. Nonempty old/new identities
and an actual size comparison are required; the present row is not evidence
that the receiving settings workflow passed.

### G3 — major — peer intentional choice and rollback are recorded as “newer release”

Copy-choice reproduction:

1. Establish the shared ChoiceAddon 1.0 baseline.
2. Pause both lab computers; write different inert text to each copy without
   changing its 1.0 version; resume. Both show one copy decision.
3. Pause B. On A choose **Keep version 1.0 held by This computer**.
4. Resume B after publication; inspect its received record/Activity.

B records `reason=NewerRelease`. Its expanded row says
`ChoiceAddon · 1.0 · ~1 files · newer release`.

Rollback reproduction: prepare B→A 1.0 then 2.0, restore 1.0 on A through
the GUI, let B apply. B again records `reason=NewerRelease`, although its
version decreased 2.0 → 1.0. A's own rollback record has `Rollback`, but its
new version is a short content ID despite the recoverable TOC saying 1.0.
Correct bytes do not make the displayed intent correct.
Screenshots/logs: `B-history-restart.png`, A/B logs and `changes.jsonl`.

## Checks 2, 3 and 5: 1 GiB apply

Generated four synthetic addons, each with 64 random 4 MiB blobs plus two
small addon files. B also had the outstanding ChoiceAddon decision above.
A published its choice and the four addons while B was paused; B resumed
and received all five in one pass.

- Receiving hashes matched **all 264 large-addon files / 1,073,742,164 bytes**.
- Apply started at approximately **03:06:10Z**. First UI sample at
  **03:06:11.345Z** showed “Installing 5 add-ons from LabA: 0 B of 1.0 GB”.
  It subsequently showed 256 MB, 492 MB, 768 MB and 1.0 GB, naming the
  current LargeAddon. No stale decision text appeared in these active samples.
- `large-progress.png` shows the incoming summary in the real GUI:
  “Incoming: 5 add-ons from LabA, 1.0 GB”.
- Actual shell tooltip was **BLOCKED**: the icon rectangle was resolved
  from B's owned HWND/ID, but UI Automation at that point did not expose
  the WoW Sync icon. No owner icon was clicked. Source tests/log absence
  were not substituted for an observed tray result; progress tray logging
  is intentionally suppressed.
- Receiving log phases: checking/deciding/installing at 03:06:10Z;
  each package completed at 10, 12, 13, 14 and 15Z; publishing at 17Z;
  final summary at 18Z, **7,713 ms**, **399 reused / 0 read**.
  Phase/package lines contained addon/computer names and numerical metadata,
  not absolute paths, settings contents or identity names. Developer startup
  and setup lines still contain lab paths; this is not a claim that the
  entire existing diagnostic log is path-free.
- Memory sampler: 69 samples over 90 s; one app-owned process, no helpers.
  Sampling intervals 905–2,219 ms; six samples overlapped the installing
  headline. Sampled apply private working set peak **37.85 MiB**, private
  bytes peak **102.37 MiB**. Private bytes are not the 100 MiB gate.
  Whole-observation private working set peak **40.27 MiB**.
  Subsecond spikes and a separate 2 GiB/many-small-files workload were
  **NOT RUN**; the executed workload meets this packet's ≥1 GiB minimum.

### G4 — major — a publishing pass is logged as “nothing changed”

Reproduce: on an enrolled, otherwise unchanged A, add the four large addons
and let the scheduled pass publish them.

At **03:05:42.355Z**, A persisted four `published` change records, each
with +66 files and +268,435,541 bytes. At **03:05:44Z**, its diagnostic log said:

```text
sync pass took 7071 ms: 9 file(s) reused from the hash cache, 264 read
slow check, pass summary: nothing changed on this computer (0 change(s), 7024 ms)
```

There were no phase/per-package publication lines for that pass. B then
received the published bytes. This is real work being reported as an
unchanged idle pass, not a failed publication or an invented expected count.
Source log: `A-state\logs\wow-sync.log`.

## Checks 4 and 7: speed, cache and GUI overhead

The initial comparison and its blocker are retained here as historical
evidence. **Check 4 is superseded by the fair follow-up below**; check 7's
GUI observations are unchanged.

Dataset: copies of the approved real AddOns tree, **36,142 files /
3,772,226,174 bytes / 113 folders**, with identical synthetic build metadata.
Only copies were enrolled. Warm results, not initial full hashing/publication:

| Mode | Three warm durations | Median | Interpretation |
|---|---|---|---|
| Candidate release headless, scripted activity | 298, 306, 402 ms | **306 ms** | 36,142 reused, 0 read; one scan |
| Candidate release headless, real activity | 314, 312, 314 ms | **314 ms** | Real process observation does not explain seconds of overhead here |
| Artifact GUI, Add-ons page, manual unchanged passes | 339, 434, 506 ms | **434 ms** | Actual GUI log summaries, 0 changes |
| Published v0.3.2 GUI, Add-ons page, contended | 3,929, 4,956, 3,330 ms | **3,929 ms** | Each log line: 108,426 reused, 0 read; **not a fair gate baseline** |

Candidate GUI used the completed candidate headless lab enrollment/provider.
After that process exited, its verified warm hash-cache file was copied to
the GUI's install/variant cache scope. No entries/hashes were fabricated.
Baseline GUI warmed its own cache through GUI inspection and publication.

### Where the observed extra time occurred

- The long baseline unchanged GUI pass summaries were timestamped
  **02:52:24Z**, **02:52:34Z**, **02:52:42Z**.
- The separate candidate headless cold publication was creating provider
  objects from **02:51:02.964Z through 02:53:22.701Z**. Thus those three
  slow baseline samples overlapped a multi-GB lab publication.
- After that publication ended, baseline passes were generally not logged
  because v0.3.2 omits warm zero-read passes ≤2,000 ms. A later logged
  sample at **02:54:51Z** was 2,012 ms. An exact, uncensored median of three
  uncontended baseline passes could not be recovered from this log.
- Candidate GUI manual logs at **02:58:27–28Z**, **02:58:33–34Z** and
  **02:58:39–40Z** give the exact durations above. Checking/deciding/publishing
  fit inside those subsecond passes. GUI overhead here is about 120 ms
  versus the real-activity headless median, not a reproduced 2–5 s gap.
- Receiving apply logs also expose a separate post-finalize interval:
  the last large package completed at 03:06:15Z, publishing began at 17Z
  and the summary followed at 18Z. That is an **apply** interval, not an
  unchanged idle-pass measurement.

The long baseline passes correlate with concurrent publication/disk work;
this is not a proven GUI-internal bottleneck or a diagnosis of the owner's
instance. Neither a contended 9× comparison nor censored baseline samples
were used to declare the approved **3× gate passed**. A strict service
benchmark directly against the read-only real install was **NOT RUN**,
because enrollment/locking it is forbidden.

### Change-detection negative control

After the large install and hash comparison, append inert bytes to one B
installed blob, changing its size. Next artifact pass:

```text
03:07:11Z sync pass took 1074 ms:
272 file(s) reused from the hash cache, 1 read
```

The GUI then raised a decision for that addon. Detection was not lost to
cache seeding. The focused existing seeded-cache test independently
executed its next-scan **zero-read** and changed-file controls. The artifact's
post-install re-observation reported zero reads; an exact counter line for
its next quiet scheduled pass was unavailable due log suppression and is
**not separately claimed proven**.

### Check 4 follow-up — fair headless comparison, 2026-11-24

**PASS: 927 / 291 = 3.185567×**, exceeding the approved ≥3× gate. This
closes the previously blocked speed comparison on the same lab copy
explicitly authorized in the orchestrator's follow-up. It does not turn
the earlier contended GUI comparison into a valid benchmark.

#### Baseline provenance and faithful port

- Created the requested detached `$LAB\v032-src` worktree of `v0.3.2`,
  resolved to **78c4045580b36dbc9ffa8bbba70a07186a3ca0a0**.
- That tag has no `speed_lab` example. Ported candidate 374a1a3's example
  against **only the tag's 0.3.2 core/adapters/runtime crates**.
- No production file or dependency was changed. All configuration, activity
  selection, HashCache/ScanCtl setup, enrollment branch, counter resets,
  `sync_once()` call and `Instant` timing remained identical. The only
  functional source difference in the harness was removal of unavailable
  directory-listing/stat telemetry and its output. Existing hash-cache
  hit/miss counters remained in both harnesses. No candidate runtime or
  optimization was backported.
- Built with `cargo build --locked --release -p wow-sync-runtime
  --example speed_lab`, Rust 1.100.0/MSVC; build completed successfully.
  Candidate used the already built release example from 374a1a3; comparison
  of this worker checkout's product sources against that SHA was empty.
- Retained baseline port in `$LAB\fair032-speed-lab.rs`, SHA-256
  `0f01705c854231d1db06da44c5a05b22f4fb0a1b5e2309fd23cedef5877839f9`.
  Baseline executable `$LAB\fair032-benchmark.exe`, SHA-256
  `707351be2bbcec2372925f895f9c707b1691a8981e8775e65f16a0f1b2d1cda7`.
  Candidate example SHA-256:
  `d7ccbfe96939893c95d7799dd05ddaba578af374f01eb310ee53891fbfb99efa`.

#### Fair setup and execution

Both executions used the **same physical** `$LAB\bench033` installation
and `$LAB\headless033-provider`, with separate identical starting state
snapshots, `fair032-state` and `fair033-state`. The tree remained 36,142
files / 3,772,226,174 bytes. Existing verified cache bytes were copied
unchanged into the harness's `speed-lab` cache scope in both snapshots.
No invented cache entries, real-install enrollment or live-provider access
were needed.

Both builds used `ScriptedActivity(NotRunning)` and `recent_ms=2000`, just
like the original speed harness. Each process ran four passes: pass 0 was
the explicitly excluded warm-up; **passes 1–3 were all the measured warm
passes**, with no selection or retry after seeing the ratio.

Immediately before and after the serial pair, process enumeration found
**zero other lab/product benchmark processes**. No other lab publication
was active: the shared provider's latest write was **02:53:22.701Z**
before the pair and remained exactly that timestamp afterwards. The
R10-check app/sampler processes were stopped, its baseline build had
completed, and the two benchmarks ran synchronously, not concurrently.
Nothing belonging to the owner was stopped to obtain these conditions.

```text
<baseline speed_lab.exe> <same lab install> _forever_ <same provider> <fair032-state> 4
<candidate speed_lab.exe> <same lab install> _forever_ <same provider> <fair033-state> 4
```

The baseline process ran **03:19:39.169Z–03:19:46.412Z**; the candidate
followed immediately and exited at **03:19:47.784Z**. Startup/cache loading
was outside the timed `sync_once()` calls.

| Build | Excluded warm-up | Warm passes 1–3 | Median | Reused per warm pass | Read |
|---|---|---|---|---|---|
| v0.3.2 tag, faithful harness port | 969 ms | **948, 927, 866 ms** | **927 ms** | 108,426 | **0** |
| Candidate 374a1a3 | 417 ms | **295, 280, 291 ms** | **291 ms** | 36,142 | **0** |

Candidate warm directory listings were 1,426, listed entries 37,676 and
stat counter 775. These additional counters were not available in the
unmodified baseline crates and are **not** the gate: the gate is measured
wall time, whose ratio is **3.185567×**.

Raw outputs: `fair032.log`, `fair033.log`, `fair-pair.log` and
`fair032-build.log`, retained under the lab.

#### Cleanup and unchanged scope

Removed the sole added baseline example file, then ran
`git worktree remove "$LAB\v032-src"` without force. The worktree directory
was absent afterwards and no baseline worktree registration remained.
No benchmark process survived. Only this evidence file is committed.

The previous cache negative control remains valid and its quiet artifact
counter limitation remains explicit. This speed follow-up does not
re-verify G1/G3/G4 fixes, claim G2 was fixed, or authorize release.

## Artifact GUI lifecycle smoke and remaining limits

Two real artifact windows completed synthetic create/join setup, received
the verified large batch and persisted history. Logs identify version 0.3.3.
After restart, paused state and history remained.

The first close correctly presented its close-to-tray explanation, rather
than immediately hiding. Choosing **Hide window** produced an invisible
owned HWND. Launching the same isolated state again exited the duplicate
and reopened that same running PID. **App settings → Quit WoW Sync** then
terminated it. This was not an owner instance or a forced-termination claim
masquerading as GUI quit.

Unverified gates remain visible: actual tray progress, receiving settings
history/convergence, runtime
model receipt, non-Windows native execution, and the separately larger
2 GiB workload. No product fixes, relaxed assertions or public release were
performed by this worker.

## Recheck — candidate e3c1054, 2026-11-25

### Scope, artifact and environment

Merged `main` into `run010/R10-check` as instructed; the checkout was clean
and fast-forwarded to `e3c10542cb521b27bfdac6b3f386a9942fae75e0`.
Only this evidence file was edited in the repository. This section supersedes
the original candidate's findings **only for the explicitly rechecked items**.

Release dispatch [45540521958](https://github.com/hamiltonia/wow-sync/actions/runs/45540521958)
was watched to successful completion; its reported head SHA matched exactly.
Downloaded the new dispatch artifact into `$LAB\recheck-artifact`, checked
the zip against its supplied `.sha256`, and extracted into
`$LAB\recheck-candidate`. Every GUI check below used that extracted executable,
not a locally built binary or the earlier dispatch.

| Artifact | SHA-256 |
|---|---|
| Candidate zip | `1bc3a2ee47f08a8e24b77b17ec75991552ab18088296efe73542633d93323425` |
| Extracted executable | `8d37bae4eb365120a83351aa8c5a9d36f9b5da72bf09767606879e34dcbc9d50` |

Same Windows 11 Pro x64 Insider build 26300, i7-13700HX, local NVMe,
dark GUI and `--zoom 0.75`. Reused the existing two synthetic installations,
isolated states and synthetic provider. No real installation, settings,
Dropbox group, owner process/state/log or demo was touched in this recheck.
No Lua was executed, no WoW process was started/stopped, and no nested worker
was launched. Actual model-routing receipt still requires the orchestrator's
launch evidence.

### Recheck result table

| Item | Result | Actual observation |
|---|---|---|
| G1: add-on Recent changes sources | **PASS** | B's HistoryAddon page visibly names LabA on the new 2.0 → 3.0 row and both older rows. |
| G3: receiving copy-choice reason | **PASS** | B receives A's chosen copy; record reason is `Choice("LabA")`. Expanded Activity says “a choice made on LabA”. |
| G3: receiving rollback reason/kind | **PASS** | B receives 2.0 → 1.0, `rolled-back` / `Rollback`, source LabA; add-on Recent changes visibly shows those values. Activity's separate source omission is G6 below. |
| G3: A's rollback version label | **PASS** | A's persisted record and settled add-on Recent changes show TOC 2.0 → 1.0, not a short content ID. |
| G4: publishing phase/package/summary logs | **PASS** | Both a 1 GiB initial publication and a subsequent scheduled four-add-on publication emit phases, per-package `published` lines and non-zero summaries. |
| Quick artifact smoke | **PASS** | Existing enrollment loads; GUI navigation, actual transfers, close explanation, hide, same-state duplicate reopening, clean GUI quit and history restart were executed. |
| Quick apply memory sample | **PASS** | 2.0 GiB incoming batch: sampled apply private working set peak **39.14 MiB**, whole observation **40.66 MiB**, one owned app process. Sampling limits below. |
| Native tier, run 1 | **PASS** | `cargo xtask verify --tier native`, exit 0, no allow-blocked flag. |
| Native tier, run 2 | **PASS** | Same command, consecutive default-concurrency run, exit 0. G5 did not recur in either run; root cause is still unknown and no fix is established. |
| Offline tier | **PASS** | fmt, clippy, workspace tests, required cases, deny and markdown links; exit 0, no allow-blocked flag. |
| G2: settings history/convergence | **NOT RUN** | Deferred to run-011 per the orchestrator's OD-12 instruction; not fixed or independently reverified here. |
| Actual tray progress | **BLOCKED** | Original direct-observation limitation remains; this round does not replace it with logs or headless evidence. |
| New minor observations G6/G7 | **FAIL** | Receiving rollback Activity omits its source; local Keep history omits available version labels. Details below. |

### Reproduction and retained evidence

**G1.** Updated A's existing HistoryAddon to TOC 3.0 with inert changed
content. B actually applied it. Opened **Add-ons → HistoryAddon**, scrolled
to Recent changes and inspected `recheck-B-history-source.png`. The new row
reads “from LabA · 2.0 → 3.0 · ~2 files · +12 B · newer release”; both older
rows also name LabA. The account-bearing recovery area in this raw lab
screenshot is not committed or reproduced in this evidence.

**G3 choice.** Used fresh RecheckChoice to avoid inherited pins: established
shared version 1.0, paused both, wrote different inert `main.lua` bytes with
the same TOC version, resumed both and observed the ambiguity. Paused B;
A chose **Keep this: Keep version 1.0. Held by This computer**. After
publication, resumed B and checked its installed bytes and new persisted
record. A/B selected file hashes matched. B's record has source LabA,
TOC 1.0 → 1.0, one changed file, zero byte delta and `Choice("LabA")`,
not `NewerRelease`. The expanded actual Activity row visibly says
“a choice made on LabA” (`recheck-B-choice-detail.png`).

**G3 rollback.** Used fresh RecheckRollback: B published 1.0, then 2.0;
A actually received both. Paused B. A selected the GUI recovery copy
labelled Version 1.0, then **Restore**; dismissed the remaining confirmation
only after the command was accepted. A's completed record is
`rolled-back`, `Rollback`, source this computer, TOC 2.0 → 1.0,
two changed files and +2 B. The settled GUI row shows the same TOC labels
(`recheck-A-rollback-label.png`).

Resumed B. Its completed record is also `rolled-back` / `Rollback`,
TOC 2.0 → 1.0, source LabA, two changed files and +2 B, not `NewerRelease`.
Both restored files hash-match A. B's add-on Recent changes visibly names
LabA and “rolled back for the group” (`recheck-B-rollback-source.png`).
The Activity reason/version recheck passes, but its missing source is
recorded separately as G6.

**G4.** Four fresh RecheckLarge add-ons contain 64 random 4 MiB blobs each,
plus TOC/main files. The initial publisher pass logged:

```text
03:54:54Z sync pass starting: start or after a change
03:54:54Z phase: checking files
03:54:57Z phase: deciding
03:54:58Z phase: publishing
03:55:00Z published RecheckLarge0 ... files +66 ~0 -0, 268435560 bytes
03:55:00Z published RecheckLarge1 ... files +66 ~0 -0, 268435560 bytes
03:55:00Z published RecheckLarge2 ... files +66 ~0 -0, 268435560 bytes
03:55:00Z published RecheckLarge3 ... files +66 ~0 -0, 268435560 bytes
03:55:00Z pass summary: 6 add-ons shared from this computer (6 change(s), 6367 ms)
```

The other two changes were HistoryAddon and the fresh choice baseline.
To cover the originally failing **scheduled** quiet-pass path rather than
only startup/manual logging, changed all four RecheckLarge TOCs to 2.0 and
their inert main files while A remained running. Did not request Sync now.
The scheduled pass at 03:59:50Z logs checking/deciding/publishing, four
per-package 1.0 → 2.0 lines and:

```text
pass summary: 4 add-ons shared from this computer (4 change(s), 2817 ms)
```

It is not reported as “nothing changed”. Package `ms` is still 0 for these
publication records; the actual non-zero pass duration is reported above.
Checked new phase/package/summary lines for absolute/profile/account paths;
none were present. Developer startup diagnostics retain the explicitly
supplied synthetic state path, as before; this is not a claim that every
developer-log line is path-free.

**Memory and apply.** B resumed into ten incoming add-ons, displayed as
2.0 GiB: four existing 256 MiB add-ons updated, four fresh 256 MiB add-ons,
HistoryAddon and RecheckChoice. Phase log: 03:55:54Z installing; final
installation completion: 03:56:07Z; pass summary: 03:56:12Z, 17,535 ms,
ten received changes plus B's one local publication. All **264 fresh
received files / 1,073,742,240 bytes** hash-matched A, checked independently
after apply. Updated old payloads are not claimed as another newly
downloaded GiB; they were part of the actual 2.0 GiB install workload.

The PID-scoped MemLib sampler retained 37 samples across about 45 seconds,
13 with an actual Installing headline, one owned process. Actual intervals
were 768–1,383 ms, not the nominal 200 ms sleep: counter/UI reads add overhead.
Installing-sample peak private working set: **39.14 MiB**; whole-window peak:
**40.66 MiB**. This is a sampled quick recheck, not a bound on unsampled
instantaneous peaks. First Installing headline was observed at
03:55:55.563Z and changed throughout the apply. No tray result is inferred
from that headline.

**Smoke and persistence.** Enabled the next-close explanation in B's
isolated App settings. Close displayed the actual explanation; **Hide
window** left its captured HWND invisible. A same-state duplicate exited
and reopened the original PID, which remained responsive. **Quit WoW Sync**
exited it. Restarted B from the new artifact: its `changes.jsonl` SHA-256
remained byte-identical,
`eb6a53d7c0a2b60d0904d6d20cc3af64ab3abf4b4c2ec9040709d9d37621a092`,
and the rollback history was visible again. Quit restarted B and A via the
GUI. Captured PIDs 21384, 30316, 12336 and 35728 all exited; no surviving
R10-check lab executable was found. No unrelated process was terminated.

Raw evidence retained under `$LAB`: `recheck-dispatch-watch.log`,
`recheck-native-1.log`, `recheck-native-2.log`, `recheck-offline.log`,
`recheck-evidence-offline.log` (the post-edit tier, including markdown links),
`recheck-samples.jsonl`, `recheck-start.json`, `recheck-A.log`,
`recheck-B.log`, `recheck-A-changes.jsonl`, `recheck-B-changes.jsonl`,
`recheck-pids.txt` and the named screenshots. Neither author completion
claims nor new test names were substituted for these GUI/receipt checks.

### Additional minor observations

**G6 — minor: receiving rollback Activity omits the source computer.**
In the completed peer rollback above, B's persisted source is LabA, and its
add-on Recent changes correctly says “from LabA”. However B's expanded
Activity group reads “1 add-on rolled back”; its row reads
“RecheckRollback · rolled back, 2.0 → 1.0 · ~2 files · +2 B · rolled back
for the group”, with no source. Reproduce by the G3 rollback flow, then open
B's Activity and expand that group. Screenshot:
`recheck-B-choice-top.png` (captured after the rollback arrived).
The packet requires source visibility in Activity as well as the add-on
page. This is a UI omission, not incorrect stored attribution or a failed
rollback; source remains discoverable on the add-on page.

**G7 — minor: the choosing computer's Keep history omits known versions.**
After the fresh TOC-1.0 choice above, A's local `kept` record has
`old_version: New`, `new_version: New`; expanded Activity reads
“RecheckChoice · kept · you chose this computer's copy” without version
labels. Reproduce by the G3 choice flow, then expand A's **Kept your choice**
group. Screenshot: `recheck-A-own-choice-label.png`.
The chosen addon is already installed/shared at 1.0, not a newly installed
addon. D-32 goal 1 requests available versions for kept changes too.
This is outside the specifically requested receiving-side G3 fix; no claim
is made that round 6 introduced it. B's receiving choice is correct.

### Release verdict

**Requested G1/G3/G4 fixes: PASS.** New artifact smoke, sampled memory and
both native runs PASS; G5 was not reproduced, not proven fixed. No new
critical or major defect was found in this recheck. G6/G7 are minor
history-display omissions, not silently passed requirements.

**Unconditional full run-010 release gate: still BLOCKED.** The original
actual-tray observation remains blocked, G6/G7 are not resolved, and this
round does not re-run the whole original acceptance matrix or speed gate
on e3c1054. G2 remains explicitly deferred per OD-12, not verified.
The orchestrator must retain those dispositions/limits and any required
owner acceptance; this worker does not grant a waiver or publish a release.
