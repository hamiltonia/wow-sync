# R9-check: independent Windows verification

## Verdict

**Artifact `7e4a43d`: FAIL — G1 (major), command completion latency.**
The window gives immediate feedback, but Decide and diagnostics Preview
still perform a multi-second scan inside the command. This is not a claim
that the window freezes.

**Artifact `7101088`: FAIL — additional major findings G2 and G3.**
The specifically requested conflict/startup/panic/diagnostics/memory rechecks
pass at their stated evidence levels, but a saved startup loses the hash
cache and interruption controls, and Quit can hide its confirmation above
the viewport. Neither artifact is approved for release by this evidence.

Authority: [PLAN](../PLAN.md), [packet](../packets/R9-check.md), D-31;
SPEC R-05/R-06/R-11/R-12/R-13/R-14/R-15/R-16; ACCEPTANCE
A-01/A-10/A-28/A-29/A-37/A-39/A-42. No product code or assertions changed.

## Environment and identity

- Worktree `D:\GitHub\wow-sync.worktrees\R9-check`, branch `run009/R9-check`.
  Tested source: `7e4a43dabc245c69d786dc2ac3f5f23e631a63aa`, version 0.3.2.
- Worker: independent verification, runtime `gpt-6.1-sol` (confirmed by the
  running-agent registry). No delegation or model override.
- Windows 11 Pro x64, build 26300; Intel i7-13700HX, 24 logical processors,
  68,375,879,680 bytes physical RAM. Interactive desktop, dark system theme,
  150% scaling.
- GUI tests below used the **downloaded release executable**, not a debug
  build. UIA Invoke/Value patterns, native folder picker, PID-scoped window
  scrolling and a foreground-PID-checked Quit shortcut drove it.
- Lab: `D:\WoWSyncLab\R9-check`. Each app had an explicit isolated `--state`.
  Enrolled providers were local lab folders only. Names, groups, choices
  and settings identities were synthetic.

### Artifact identity

Commands from the worktree root:

```powershell
gh run watch 45467533289 --exit-status
gh run download 45467533289 --dir D:\WoWSyncLab\R9-check\artifact
```

Dispatch completed successfully, head SHA matched the tested source.
ZIP hash matched its supplied `.sha256`:

```text
wow-sync-0.3.2-windows-x64.zip
ac06ba2e293c14520c9afe89356c08328eb8cea6ed4f3001ba97f21a2b274a98
wow-sync.exe
035d288b3b4706ae0452457ca620fcf09539adda8fb64d46ed00b09ad0c259b8
```

The saved GUI diagnostics independently reported
`build_commit: 7e4a43dabc245c69d786dc2ac3f5f23e631a63aa`.

## Results: artifact 7e4a43d

| Check | Result | Actual evidence and limits |
|---|---|---|
| 1. Conflicting choices through GUI | **PASS** | Two synthetic computers, same issuer and sequence 2, empty `known`, different keeps excluding each other. Both release copies plus local-only were offered. A GUI choice converged both files and removed both decision badges; see reproduction below. |
| 2. Rapid/stale second choice | **PASS** | Retained both UIA option patterns; invoked A, B, A, A in **5 ms**, covering competing and repeated same-copy clicks. Four click acts, **one** accepted record, no second keep. Both computers converged. The runtime stale-answer test also passed in the full tier. This burst is not an OS mouse double-click-speed/a11y matrix. |
| 3. Commands during a large pass | **FAIL** | Decide feedback **24 ms**, completed **2,585 ms** (dispatch 2,484 ms). Preview feedback **39 ms**, completed **2,374 ms** (dispatch 2,235 ms). Pause during a pass **149 ms**; four queued Pause invokes still made one pause, not alternating toggles. G1. |
| 4. Missing tree/headline/other work | **PASS** | Live pending decision outranked missing data. After resolving it, headline named `Lab-b` and `sourceonlymissing`, never Dropbox downloading. `SafeAddon` applied while the missing add-on did not. Genuine packaged-app timer escalated at 30 minutes; observed by UIA after **1,801.18 s**, without a clock override. |
| 5. Action logs/privacy | **PASS** | Actual accepted Decide with record prefix; Pause/Resume; Preview and Save success lines. No profile path or synthetic computer name in those result lines. The refused/stale-result formatter was executed by the full suite, not produced by the click burst (the UI suppressed its repeated dispatches). |
| 6. Hash cache/full check | **PASS** | Read-only real AddOns scan **6,235 → 149 ms**, **41.85×**; 34,359 misses then all hits. Synthetic 3.35 GB **4,550 → <1 ms**; same-size edit gave exactly one miss, full check all 33 misses. Every benchmark invocation was a new process. GUI Check all files requested a full read; memory in check 14. |
| 7. Version skew | **PASS** | Rendered peer page: “Lab-b runs WoW Sync 0.3.1; this computer runs 0.3.2. Update every computer to the same version so they behave alike.” Viewed actual window capture; this paragraph is not exposed as UIA text. |
| 8. Regression/setup/Quit | **PASS** | Both verification tiers passed. Normal discovery found C/D and Battle.net's registered D-drive install; real read-only inspection completed in **6,205 ms**, 113 eligible/0 blocked/34,359 files. Native picker accepted synthetic `_forever_`; Cancel check showed Check again within **107 ms**, Next stayed unavailable. Explicit Quit exited setup and enrolled labs. |
| 9. Dispatch artifact smoke | **PASS** | Successful dispatch, verified ZIP, extracted executable, normal setup, real read-only discovery, enrollment-state startup, GUI choices, diagnostics and Quit all executed from this package. |
| 10. RYZENFISH panic regression | **PASS** | Same-issuer mutual-exclusion record shape was read from the permitted real records, then recreated with entirely synthetic identities/content. Baseline 0.3.1 reproduced the exact `choice.rs:374`, len 1/index 1 panic with an additional unpublished local edit. Identical panic-inducing bytes on 0.3.2 completed, offered alternatives and converged both computers without a remaining decision. See exact distinction below. |
| 11. Saved enrollment/re-enroll | **PASS** | Actual enrolled app Quit/restart: first observed UI at **122 ms** was Overview, “Checking add-ons…”, with normal navigation, no Welcome/setup. Second enrollment refusal passed the backend test; no second setup route was forced through the packaged UI. |
| 12. Panic resilience | **PASS — unit/code only** | Executed injected-pass-panic backend test, observed panic then passing fault/retry/quit assertions. Release package has no pass-panic injection hook. Code confirms catch-unwind around pass/command/status and 300-second normal retry. Did **not** inject a panic into the packaged GUI. |
| 13. Diagnostics versions/privacy | **PASS** | Actual Preview→Save produced two computer rows with `app_version=0.3.2` and `0.3.1`, positional aliases only. No `Lab-a`, `Lab-b`, group name or drive/profile path in export. Eleven diagnostics tests, including planted private strings, passed independently. |
| 14. Background private resident memory | **PASS** | Copied **only AddOns**, read-only from real `_forever_`, into a synthetic install; no WTF/accounts copied. Enrolled only a lab group. After completed passes and opening/closing UI, 20 hidden-background samples over **21.8 s**, one owned process: **47.73–82.01 MiB**, mean **53.96 MiB**. Separate GPU shared **40.06 MiB**, dedicated 0. Private bytes 111.69–147.13 MiB are not private resident memory. Short-window result, not a long-duration soak. |

## Conflict and panic reproductions

The real group's two permitted authority records were read only to establish
shape. Nothing from the real group was enrolled, modified or executed.

The independent lab helper linked the tested `wow-sync-runtime`, core and
adapters from this checkout; its sources/binaries lived only in the lab.
It created Forever installs using `SyntheticInstall`, enrolled a/b into a
local provider and published different synthetic payloads with the same
`HideAnything` TOC version 1.10.0. It used `build_keep` over an empty ledger
twice, **the same a issuer, `issued_at_seq=2`, neither knowing the other**,
with each copy excluding its rival. Each computer initially held one copy.
The author's different-issuer fixture alone was not substituted for this case.

Synthetic keep hashes:

```text
19234950c4772446241b7d66e03f540e915b5c6254d57ca1c75471f967f54a74
8d84c518f3a91d41353daa824627d40a49da6a83eafe135d784682384a42cbf0
```

0.3.2, PID 39468, actual decision controls:

```text
Keep this: Keep version 1.10.0. Held by This computer. Installs hideanything.
Keep this: Keep version 1.10.0. Held by Lab-b. Installs hideanything.
Keep this: Keep it on this computer only
```

Four rapid invokes took 5 ms; the log contained four `act Group Decide`
lines but one result:

```text
2026-11-24T19:37:22Z pid=39468 INFO Decide: accepted, wrote record 734195f97256 (37 ms)
```

There were three distinct authority hashes afterward (two original plus
one new; replicated copies are not additional choices). Both computers
showed “This computer is up to date”, no decision count, and their active
payload SHA-256 values matched:

```text
16687ca13a61413b940d95f2d5cb916250b2bd69cbdf7deb25b254c2693a585c
```

### Old-version negative controls

Downloaded the GitHub **release** `v0.3.1` ZIP, not the owner's installation.
Its ZIP SHA-256 matched:
`d6291b30b66482e3df4701506f5502206afa96ad463a363df3a9a8a28a710b09`.
Started only isolated lab processes.

1. With the original two held copies, 0.3.1 reproduced the no-options page:
   “Hide Anything: choose which copy to keep” followed only by Decide later.
   **It did not panic on that unchanged two-holder fixture.**
2. With the same keeps/provider/state but an unpublished third local payload
   on a, 0.3.1 reproduced the owner's exact panic signature:

   ```text
   2026-11-24T19:34:17Z pid=39356 ERROR PANIC panicked at crates\wow-sync-core\src\choice.rs:374:46: index out of bounds: the len is 1 but the index is 1
   ```

   It also misleadingly exposed Welcome/setup. This additional local edit
   is disclosed; it is not proof of the real computer's unseen bytes.
3. Restored that panic-inducing lab snapshot and ran the candidate (PID
   37016). A pass completed; the newer-local-choice page offered a retained
   choice and the local version. GUI “Use this” wrote `de25a429f906…`.
   Its `known.extra` explicitly contained **both** original hashes above.
   a and b both reached up to date with no decision. Matching active
   payload SHA-256:
   `30008f822a3341c961c1c90326e3b7595c98f5878558e9461e72d1d9b913153c`.

## G1 — major: group commands still re-scan inside dispatch

**Affected artifact:** 7e4a43d / dispatch 45467533289.
**Requirement:** PLAN goal 3 and release gate: commands handled within
about one second during a pass, not just immediate “Working on it…” feedback.

Reproduction:

1. On a synthetic enrolled instance with a pending newer-local Hide Anything
   choice, install a synthetic BigLab package: 32 × 100 MiB files and a TOC
   (3,355,443,200 payload bytes). Let an ordinary pass finish.
2. App settings → Check all files, immediately return to the pending choice.
   Confirm Syncing progress is present. Choose the local version.
3. Working feedback appears in 24 ms. Accepted result appears in 2,585 ms.
4. Open diagnostics options with Preview visible. Change only the 32
   synthetic files' mtimes to another settled time, starting a routine
   cache-miss pass with Sync now. While Syncing is present, invoke Preview.
   Feedback appears in 39 ms, but preview result takes 2,374 ms.

Actual result lines:

```text
2026-11-24T19:42:28Z pid=37016 INFO sync pass paused after 314 ms to answer a command; it starts again right after
2026-11-24T19:42:30Z pid=37016 INFO Decide: accepted, wrote record f18381fd1fab (2484 ms)
2026-11-24T19:43:46Z pid=37016 INFO sync pass paused after 168 ms to answer a command; it starts again right after
2026-11-24T19:43:48Z pid=37016 INFO ExportDiagnostics: ok, 39 line(s) of text (2235 ms)
```

The pass yields promptly, but dispatch still does expensive fresh observation
with pass interruption disarmed. The full-check case additionally retains
its full-read request until a complete scan. This is much better than the
old 45-second queue, but does not meet the approved one-second handling gate.
No new threshold or code fix was introduced by verification.

Pause control: during a full pass, four rapid invokes produced one
`pause: syncing paused`, visible Resume in 149 ms and a stable Paused state
after another four seconds. Resume was logged separately.

## Cache and unavailable-data evidence

Benchmark command shape (verification-only helper, release build):

```text
r9-probe scan <AddOns root> <isolated cache state> [full]
```

It invokes `scan_addons_ctl` with the production `HashCache` and prints its
actual hit/miss counters. Each row below is a fresh process, including
cache reopen; no scan enrolled or wrote the real install.

| Data | Mode | ms | Files | Hits | Misses |
|---|---|---:|---:|---:|---:|
| Real `_forever_`, 113 folders | Empty cache | 6235 | 34359 | 0 | 34359 |
| Same real folder | Restart/reopen | 149 | 34359 | 34359 | 0 |
| Same real folder | Force full | 6556 | 34359 | 0 | 34359 |
| Synthetic BigLab, 32 × 100 MiB + TOC | Empty cache | 4550 | 33 | 0 | 33 |
| Same synthetic folder | Restart/reopen | <1 | 33 | 33 | 0 |
| Same-size one-byte edit, changed settled mtime | Restart/reopen | 160 | 33 | 32 | 1 |
| Same synthetic folder | Force full | 4280 | 33 | 0 | 33 |

An earlier exploratory 32,100-file/3.27 GB synthetic scan was stopped after
roughly ten minutes without a completion result; it is **not** counted as
a passing benchmark. The bounded-file-count 3.35 GB dataset above is the
reported synthetic speed-up. Its size/mtime edit and force-full controls
did execute completely.

Missing-tree fixture: b publishes SourceOnlyMissing and SafeAddon beside
the conflicted package; remove **only the lab** SourceOnlyMissing tree
record before a's next pass:

```text
a applied=[AppliedLine { package: "safeaddon", text: "applied" }]
sourceonlymissing: waiting for 1 file(s) to arrive from the sync folder
```

The SafeAddon file existed locally; SourceOnlyMissing did not. Before the
choice, UI headline was “Waiting for your decision on 1 add-on”; afterward:
“Waiting for Lab-b to finish sharing sourceonlymissing”.

### Live 30-minute escalation

**PASS**, packaged 7e4a43d binary, no clock override. PID 15468 started at
`2026-11-24T19:47:38.5655500Z`. Initial UIA headline was “Waiting for Lab-b
to finish sharing sourceonlymissing”. The application logged the escalation
at `20:17:38Z`; a five-second UIA poll observed it after **1,801.182173 s**:

```text
Lab-b has not finished sharing sourceonlymissing. Open WoW Sync on it and let it sync.
```

The window retained normal enrolled navigation. Requested GUI Quit at
`20:17:40Z`, then independently verified process exit. This is actual elapsed
wall time, not the corresponding 1,800-second boundary unit test.

## Diagnostics, resilience and memory details

Actual GUI Preview→Save text excerpts:

```text
app_version: 0.3.2
build_commit: 7e4a43dabc245c69d786dc2ac3f5f23e631a63aa
This computer: enrollment=active retired=off report=current last_report_age=none watermark_seq=0 app_version=0.3.2
Computer A: enrollment=active retired=off report=current last_report_age=0h watermark_seq=0 app_version=0.3.1
```

No profile/drive path or lab computer/group name remained in the saved
report. Actual action logs:

```text
ExportDiagnostics: ok, 34 line(s) of text (24 ms)
SaveDiagnostics: ok, 34 line(s) of text (30 ms)
```

Save uses the app's automatic Documents destination, not a picker. The one
timestamp-identified report created by this lab was copied back into the
lab and removed from Documents; no other Documents file was read/changed.

Executed focused commands from this worktree with
`$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"`:

```powershell
cargo test --locked -p wow-sync-app --bin wow-sync -- a_restart_with_a_saved_enrollment setup_finishing_for_an_enrolled an_unavailable_add_on --nocapture
cargo test --locked -p wow-sync-app a_panic_in_a_pass_is_shown_keeps_commands_working_and_is_retried -- --exact backend::tests::a_panic_in_a_pass_is_shown_keeps_commands_working_and_is_retried --nocapture
cargo test --locked -p wow-sync-core --lib diagnostics::tests -- --nocapture
```

Results: 3/3, **1/1** (plus an explicitly zero-test devkit binary), 11/11
passed. The injected panic printed `backend.rs:1013: injected test panic`;
the sync thread recovered and the test completed in 2.37 s. The test
checks fault publication, retry and subsequent Quit; it does not exercise
every command while the fault is active. Source reading, not an installed
fault injection, establishes the separate command/status guards.

Memory method: existing `tools\mem\MemLib.ps1`, root PID **and creation
time** validated, owned descendants enumerated each sample;
`Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate` summed.
Real AddOns were copied with metadata into `real-copy`; only that synthetic
install was writable/enrolled. Initial helper publication completed
33,945 new objects in a 148,623 ms pass. The packaged app then completed
a real-copy scan in 7,215 ms, subsequently using the cache.

The GUI's first close opens its explanatory sheet; that visible phase was
**discarded**, not called background. After explicitly invoking **Hide
window**, Win32 window visibility was false and the valid 20 samples ran.

```text
sample seconds: 1.1 ... 21.8
owned processes: 1 throughout
private resident bytes min/mean/max: 50053120 / 56576819.2 / 85995520
private committed bytes max: 154279936
GPU dedicated bytes: 0
GPU shared bytes: 42008576
```

## Verification tiers

From `D:\GitHub\wow-sync.worktrees\R9-check`, branch `run009/R9-check`:

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

Both passed independently on source 7e4a43d:

```text
[PASS] offline/fmt
[PASS] offline/clippy
[PASS] offline/test
[PASS] offline/required-cases  (151 listed cases / 875 results)
[PASS] offline/deny
[PASS] offline/markdown-links
[PASS] native/tier            (7 listed cases / 875 results)
```

This is Windows execution, not a macOS/ARM claim. No assertions, limits,
test inventory or dependencies in the product were changed.

## Artifact 7101088

The orchestrator requested a separate conditional recheck. Dispatch
**45472986890** completed successfully while the original artifact's live
30-minute timer was running. No branch/source update was made in this
worktree.

```text
head: 7101088f7151d1b0d5a9fc619e857e1515cb4234
ZIP SHA-256 (matched supplied checksum):
bd15f2bb489054014819d6ca77f84ab19f0fbeafe1c12e1d9ccf799c8e236389
executable SHA-256:
e093c43325867cf3f844bd5d3904104f3b449b9b801c6380fe2dfcfe1440a8cb
```

Downloaded with `gh run download 45472986890 --dir
D:\WoWSyncLab\R9-check\artifact-7101088`, expanded and used that executable.
The actual saved GUI report independently identifies the same full commit.

| Recheck | Result | Actual evidence |
|---|---|---|
| 1. Conflicting choices | **PASS** | Restored original same-issuer/seq-2/empty-known two-copy fixture. PIDs 7852/13308 offered both copies and local-only; GUI choice converged both, cleared decision badges and produced the same `16687c…` active payload hash as the first artifact. |
| 2. Repeat/competing choice | **PASS** | A/B/A/A option-pattern burst in **1 ms**; one accepted result `e63867fc1945…`, **32 ms** dispatch, no second choice. |
| 3. Full during-pass latency sequence | **NOT RUN** on this artifact | The original Decide/Preview/Pause sequence was not fully repeated. Additional idle Decide was measured at 2,592 ms, and saved-startup scan controls are absent (G3); neither G1's resolution nor responsive interrupted passes can be claimed. |
| 10. Former-panic fixture | **PASS** | Restored identical unpublished-local-edit fixture that panicked on 0.3.1. PIDs 18864/28896 completed passes, offered retained/local alternatives, accepted local choice `d92785c51528…` in **29 ms**, and both converged with no decision. Both payload hashes `30008f…`, matching the first candidate. No panic from either new process. |
| 11. Saved startup / duplicate enrollment | **PASS** | Actual saved-enrollment startup (PID 16148) exposed Overview/Checking add-ons, not setup. Restart and duplicate-enrollment backend tests executed from exact archived 7101088 source passed. G3 is a distinct, newly observed startup regression. |
| 12. Panic resilience | **PASS — unit/code only** | Exact archived-source injected-pass-panic test passed, including service recreation/retry. All three targeted backend tests passed in **2.32 s**. No release-GUI pass-panic hook exists; no installed-app injection claimed. |
| 13. Diagnostics | **PASS** | Actual GUI Preview→Save, PID 6420: full build commit 7101088, both computer rows `app_version=0.3.2`, aliases instead of names, no drive/profile paths. For this launch USERPROFILE pointed to a lab-only profile so its automatic Documents save stayed in the lab. |
| 14. Background memory | **PASS** | Same isolated real-AddOns copy and lab group; actual new-artifact pass completed. Win32 visibility false, **20 samples / 23.34 s**, one owned process. Private resident **38,211,584–64,802,816 bytes** (36.44–61.80 MiB), mean 46,074,265.6 bytes; private bytes max 131,932,160, separately reported. GPU shared 41,267,200 bytes; dedicated 0. G3 explains why this restarted instance has no in-memory hash cache; this memory result does not waive G3. |

Targeted source test command (a `git archive 7101088` extraction in the lab,
not an edited implementation/worktree):

```powershell
cargo test --manifest-path D:\WoWSyncLab\R9-check\source-7101088\Cargo.toml --target-dir D:\GitHub\wow-sync.worktrees\R9-check\target --locked --offline -p wow-sync-app --bin wow-sync -- a_restart_with_a_saved_enrollment setup_finishing_for_an_enrolled a_panic_in_a_pass --nocapture
```

The new source's injected panic occurred at `backend.rs:1100`; 3 passed,
151 filtered. Source inspection confirms recreation uses the new `open`
closure. It also exposed G3; passing the recovery test does not prove that
the recreated service retained scan controls.

Actual new GUI report:

```text
build_commit: 7101088f7151d1b0d5a9fc619e857e1515cb4234
This computer: enrollment=active retired=off report=current last_report_age=none watermark_seq=0 app_version=0.3.2
Computer A: enrollment=active retired=off report=current last_report_age=0h watermark_seq=0 app_version=0.3.2
```

One initial attempted new launch signalled an old lab process still using
that state. That attempted conflict recheck was **discarded**. Both writers
were explicitly quit, process exit verified, and the fixture restored before
the valid 7852/13308 run cited above. This affected only synthetic lab data.

### G2 — major: Quit confirmation is offscreen on scrolled App settings

**Confirmed artifact:** 7101088; PID 16148, isolated real-AddOns copy.
**Requirement:** SPEC R-13 / A-01: an explicit normal GUI Quit must present
its consequential confirmation and remain understandable.

1. Open App settings, scroll to the bottom to reach Quit WoW Sync.
2. Invoke Quit while a pass has the Syncing indicator.
3. The app logs `act Quit` and prepends its confirmation to the scrolled
   page **without bringing it into view**. Window stayed alive past the
   helper's ten-second timeout and for approximately 49 seconds overall.
4. UIA reports the sheet and buttons above the viewport:

   ```text
   Window: WoW Sync is applying changes right now. Quitting waits for the current step to finish safely.
   Quit when safe: rectangle 1617,-1021,188,54
   Keep running: rectangle 1829,-1021,174,54
   ```

The actual captured window showed the bottom Settings sections, not this
confirmation. Scrolling **up** to the top moved Quit when safe to y=419;
invoking it then exited normally:

```text
2026-11-24T20:08:53Z pid=16148 INFO act Quit
2026-11-24T20:08:53Z pid=16148 INFO window shown
2026-11-24T20:09:42Z pid=16148 INFO act SheetQuitWhenSafe
```

This is recoverable entirely through GUI scrolling, unlike the run-008
setup-only invisible modal, but the normal bottom-of-settings Quit flow
still leaves the user without a visible next step.

### G3 — major: saved startup/reopen drops hash cache and scan interruption

**Confirmed artifact/source:** 7101088.
**Requirements:** D-31/PLAN goals 3 and 6; optional Check all files must work
on an enrolled computer, and routine scans must reuse the persisted cache.

Reproduction:

1. Start an already enrolled lab state through the packaged app, without
   going through setup again. This is the ordinary upgrade/restart path.
2. Add the synthetic BigLab package used in G1 and let passes run.
3. App settings → Check all files while still enrolled.
4. Actual result:

   ```text
   2026-11-24T20:10:17Z pid=18864 WARN check all files: nothing is set up to check
   ```

No hash-cache file was created for this restarted conflict instance and
there were no cache-counter/pass-timing log lines. Repeated passes remained
full scans. The normal enrolled pages and decisions still worked, so this
is not an unenrolled-state refusal.

Exact source evidence in the archived `backend.rs`:

- Lines 721–730 define `cache` and `read_ctl`; that helper both stores the
  app's cache reference and creates the cancel/interrupt-bearing `ScanCtl`.
- The new `open` closure, approximately lines 733–739, calls `open_service`
  and `inject_crash_after`, **but never `set_read_ctl(read_ctl(t))`**.
- Saved-target startup calls `open(t)` (approximately line 746); fault
  recreation also calls it.
- Only the separate enrollment path calls `set_read_ctl` (line 848).

Thus a normal saved startup and a recovered service lose **both** the
hash cache and the application's cancellation/command-interruption handles.
The focused startup test checks which page appears, not this attachment.

An additional new-artifact Decide measurement, **with no Syncing progress
at the moment of invocation**, still took 2,592 ms end-to-end / 2,481 ms
inside dispatch, despite 38 ms feedback:

```text
2026-11-24T20:11:04Z pid=18864 INFO Decide: accepted, wrote record c747212ec818 (2481 ms)
```

This is not misrepresented as a repeat of G1's during-pass experiment.
It establishes that expensive command observation also remains on this
artifact. The full 1–9 matrix was not repeated for 7101088; these extra
failures were encountered while executing the authorized conditional checks.

## Final validation and cleanup

Re-ran both original-source verification tiers after staging this report's
new-artifact draft: offline **PASS**, native **PASS**, both exit 0.
The only subsequent edits record the completed live timer and cleanup.
No tests, assertions, required-case lists or implementation files changed.

Verified the final captured application PIDs 15468/18864 had exited and
enumerated zero executable processes rooted in this lab. Removed the entire
`D:\WoWSyncLab\R9-check` directory, including copied AddOns, stress data,
providers, states, reports, downloaded executables and helper sources/builds;
verified the directory no longer exists. No helper session remains running.
The measurements and relevant log excerpts are persisted in this report;
the transient screenshots/CSVs were deliberately removed with the lab.

All protected owner locations remained read-only: no real-group enrollment,
no WoW/Dropbox lock/write/rename/delete, no owner-process stop/state access,
no `D:\WoWSyncDemo` changes. No Lua/saved-variable/CMD content executed.
The initially auto-saved Documents report was removed as disclosed above.

**Summary:** original artifact **13 PASS / 1 FAIL** (G1); conditional new
artifact **7 requested rechecks PASS**, full additional latency sequence
**NOT RUN**, and new major findings **G2/G3**. Fix/reverify the findings
before declaring this candidate release-ready; passing unit suites does not
supersede these packaged-app observations.
