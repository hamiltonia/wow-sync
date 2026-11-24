# R9-check: independent Windows verification

## Verdict

**Latest: `104d238` requested targeted rechecks PASS.** Its unrelated
Decide completed in 129 ms during the exact invalidated-BigLab case that
failed on f95def1. Initial-upload clicks and G3 restart smoke also pass.
The broader f95def1 matrix below remains historical evidence; it was not
silently treated as a fresh full 1–14/memory matrix on 104d238.

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

## Final recheck f95def1

Requested by the orchestrator after the preceding evidence was committed.
Same worker, branch, machine, display scaling and protected-data rules.
No product changes, nested agents, real enrollment or owner-process actions.
The previously removed lab was recreated for this independent recheck.

### Package and environment

Waited for release dispatch **45485437850** to complete successfully before
downloading/running it. Its Windows build job passed; the pre-release
publishing job was **skipped**, not run by this worker.

```text
head/build_commit: f95def1363cd5d3407e6ac1bc6adb60d42d4314b
ZIP SHA-256, matched supplied .sha256:
e9165d5b13d07accd7eeb2c5c749258087085544ed18134c52b01c556f481c33
packaged wow-sync.exe SHA-256:
e5140b3129c2c2ea3dd4efd3de5a48b26ac2f1350e589ee8735e5df9df796da9
```

Commands used:

```powershell
gh run watch 45485437850 --exit-status
gh run download 45485437850 --dir D:\WoWSyncLab\R9-check\artifact
```

All GUI checks ran the extracted package with explicit lab `--state`.
Synthetic conflict computers and saved-state restart checks used normal
non-developer mode. First-enrollment/initial-upload fixtures used `--dev`
solely to preselect an explicit **synthetic** install/provider and avoid
selecting real discovery results; the actual setup, enrollment, transfer,
GUI and command paths were exercised. Runtime hooks were not used to fake
latency or completion.

For the deterministic idle/post-Sync/routine matrix, the lab's automatic
interval was 60,000 ms; explicit Sync now initiated the measured passes.
The final never-used first-enrollment fixture used the normal 5,000 ms
interval. Hash stability used the normal 2,000 ms window, with settled
synthetic mtimes.

### Results

| Requested item | Result | Actual evidence |
|---|---|---|
| G1: idle | **PASS** | With 3.35 GB BigLab installed and cached: Decide **93 ms**, Preview **110 ms**. |
| G1: right after Sync now | **PASS** | Decide **112 ms**; Preview **152 ms**. Preview had Syncing visible at invocation. The cached pass had already finished at the Decide invocation, so that measurement is not called a during-pass case. |
| G1: routine BigLab pass | **FAIL — G1 remains** | Both had Syncing visible. Preview **144 ms**; unrelated HideAnything Decide **2,209 ms**, despite **13 ms** UIA feedback. Its dispatch took **2,093 ms**. See the controlled invalidation reproduction below. |
| G1: FIRST pass / initial upload after enroll | **PASS** | Never-used install/provider/state: Preview **196 ms**, Decide **131 ms**, Syncing visible at both clicks, **zero own published code-state files** before either command. Decide followed an explicit Sync now to resume the initial upload after Preview yielded it; this is initial-upload evidence, not a claim that both ran inside one uninterrupted `sync_once`. |
| G2: bottom-scrolled busy Quit | **PASS** | Quit confirmation reached in **119 ms**. Quit when safe UIA rect **1617,419,188,53**, wholly inside window **489,196,1582,1136**. No scrolling up needed. Confirming exited within **232 ms** of the original Quit invocation. |
| G3: saved enrollment/cache/full check/Quit | **PASS** | Restart PID 38508 immediately showed Overview/Checking add-ons, not setup. Persisted scoped `.txt` cache existed. Check all files worked, including a new request on the restarted process; no “nothing is set up” warning. Normal GUI Quit **234 ms**; busy real-copy Quit **437 ms** including its visible confirmation. |
| RV9-16: close setup-complete screen | **PASS** | PID 32904 on “This computer is set up”: Close hid its Win32 window while the process remained alive. Launching the same explicit-state executable signalled/reopened that instance; Finish and open WoW Sync reached the main view. No invisible first-close sheet. Physical tray-icon clicking was not separately automated. |
| RV9-15: interrupted full check survives restart | **PASS** | Requested full check, quit while Syncing, and found the one-byte scoped `.full` marker after exit. Restart re-read **all 35 files** in **2,228 ms**; marker disappeared only after that completed full scan. `.txt` cache remained. |
| 10. Same-issuer panic regression | **PASS** | Same-issuer, sequence-2, empty-known, mutually excluding keeps offered both copies. Rapid A/B/A/A burst in **2 ms** wrote exactly one accepted keep and both computers converged. Additional unpublished third local payload also completed and converged, without a panic. Detail below. |
| 11. Saved startup / duplicate enrollment | **PASS** | Actual package startup above showed the main view. Exact-source restart and duplicate-enrollment refusal tests also passed. No second enrollment was forced through GUI. |
| 12. Panic resilience | **PASS — unit/code only** | Exact f95def1 injected-pass-panic, reopen-from-disk and restored-cache tests passed. No release-GUI panic injection hook; no packaged panic was injected. |
| 13. Diagnostics versions/privacy | **PASS** | Actual GUI Preview→Save, correct full build commit, all three computer rows have `app_version=0.3.2`, aliases instead of names, no drive/profile paths. Actual file save **174 ms**, new Preview **230 ms**. All saves stayed inside redirected lab Documents. |
| 14. Background memory after real-AddOns pass | **PASS** | Read-only copy of only the real 34,359-file AddOns tree, no WTF/accounts. Local synthetic group, isolated writable copy. Completed initial pass **206,859 ms**, 34,359 reads; hidden sampling then measured **54.01–88.87 MiB** aggregate private resident memory. |
| Memory during a BigLab pass | **PASS** | Hidden extended BigLab full-read pass: **131 files / 13.42 GB**, pass **11,792 ms**, 10 samples overlapping its full-read interval. Entire 20-sample window **37.21–37.37 MiB**, one owned process. Extension/measurement details below. |

### G1 — major, still open on f95def1

The idle, warm-cache post-Sync and initial-upload observations are
substantially better than the earlier candidates. **A routine cache miss in unrelated BigLab still
delays a HideAnything decision beyond the requested approximately-one-second
click-to-result gate.**

Bounded reproduction, normal packaged app, enrolled lab computer:

1. Install BigLab with 32 distinct 100 MiB `.bin` payloads and a small TOC:
   **3,355,443,259 bytes / 33 files**. Complete a pass so its hashes exist.
2. Resolve a HideAnything keep, make a settled synthetic local HideAnything
   edit, Sync now, and open its “newer version is not shared” decision.
3. Change **only BigLab's mtimes**, not bytes, to a different settled past
   time. This legitimately invalidates its `(path,size,mtime)` hashes.
4. Click **Sync now**, verify the Syncing progress indicator, then click
   HideAnything's **Use this** option.

Measured UIA feedback **13 ms**, fresh accepted result **2,209 ms**:

```text
2026-11-24T21:23:15Z pid=37128 INFO Decide: accepted, wrote record 99cbc1f23e0d (2093 ms)
```

This was **not** Check all files, an artificial pause, a provider delay or
a stale-result measurement. Other commands in the same invalidation
condition remained quick:

```text
2026-11-24T21:19:08Z pid=37128 INFO ExportDiagnostics: ok, 32 line(s) of text (17 ms)
```

That Preview completed in 144 ms with Syncing visible. The final source's
`command` uses `observe_for_command(false)` for Decide, so uncached changed
files in unrelated packages are still read inside its fresh observation.
Immediate feedback is not substituted for command completion.

Measurement method: UIA Invoke of an in-window control, monotonic stopwatch,
fresh result-log count for the captured PID **and** the corresponding UIA
result message. The short feedback timings are UIA-observed messages, not
an assertion that an offscreen notice banner was visible at every scroll
position. End-to-end result times, not those messages alone, determine G1.

Initial-upload evidence used fresh PID 14872 / `fresh\d`, an entirely new
install root with no previous journals/provider objects. It had **zero**
own code-state files before Preview and before Decide:

```text
2026-11-24T21:27:14Z pid=14872 INFO ExportDiagnostics: ok, 29 line(s) of text (17 ms)
2026-11-24T21:27:15Z pid=14872 INFO Decide: accepted, wrote record 43d9a12f15d0 (35 ms)
```

Earlier attempts that missed the active first-upload timing window were
**discarded as first-pass latency evidence**, not counted as passes. They
did independently verify completion-screen close/reopen. The never-used
fixture above supplies the final timing result.

### Conflict regression detail

The verification-only helper linked exact archived f95def1 runtime/core/
adapters to create synthetic identities, enrollment and the two authority
records; it did not provide the GUI's decision/result implementation.
Both records use the **same a issuer**, sequence 2, empty known, and each
pins one copy while excluding the other.

Packaged PIDs 33212/12568 offered both 1.10.0 copies plus local-only.
Retained option handles A/B/A/A, invoked in 2 ms, produced only:

```text
2026-11-24T21:14:21Z pid=33212 INFO Decide: accepted, wrote record 4c0090b0f136 (21 ms)
```

Both showed up to date, no decision, and identical active payload SHA-256:

```text
6a3093e98b97c8b2f063086e0465f1f7c80574561f542d1e24453c1a943f8938
```

Restored pristine conflict, added a third unpublished local payload on a
(the shape needed to expose the old eager-index panic), and used packaged
PIDs 36392/34620. Choice accepted in 15 ms dispatch / 70 ms end-to-end.
Both converged, no decision and no PANIC, with identical active payload:

```text
669ad7c51c2944a4cf5806f4a44fa4ac8184acb686c3f3307df7191d262d05f1
```

The old-version panic control remains the actual 0.3.1 execution documented
earlier; 0.3.1 was not redownloaded/re-executed for this final recheck.

### Durable full-check and memory evidence

Actual G2/RV9-15 sequence, original PID 37128 → restart PID 38508:

```text
after busy Quit:
hashcache-86407542f053f74e.full: 1 byte
hashcache-86407542f053f74e.txt: 4098 bytes
2026-11-24T21:25:58Z pid=38508 INFO sync pass took 2228 ms: 70 file(s) reused from the hash cache, 35 read
after restarted full scan: zero .full markers, .txt cache retained
```

For the final **memory-only** phase, extended BigLab with 96 regular-file
hard links **inside the isolated lab**, aliasing the original payloads
under distinct names. This made a bounded **129-file / 13,421,772,859-byte**
BigLab workload while retaining the same 32 distinct payload contents.
Including HideAnything, the full scan read **131 files**. This extension
was **after** the G1 matrix; G1 measurements used the original 3.35 GB data.
It is a longer scanning-memory exercise, not a claim about 13 GB of unique
data publication or an upload-memory soak.

The existing approved MemLib PID+creation-time and descendant instrumentation
was used, unchanged. Both windows were explicitly hidden and Win32
visibility false before sampling. No name-based process selection/stopping.

| Hidden sample | Samples | Private resident bytes min / mean / max | Private bytes max | GPU shared max |
|---|---:|---|---:|---:|
| Completed real-copy pass, PID 31196 | 20, about 25.4 s | 56,631,296 / 63,503,769.6 / 93,188,096 | 160,968,704 | 41,234,432 |
| Extended BigLab full read, PID 38508 | 20, about 22.2 s | 39,022,592 / 39,071,334.4 / 39,182,336 | 106,409,984 | 41,881,600 |

One owned process throughout, GPU dedicated 0, private resident gate
104,857,600 bytes. Private bytes and GPU shared are reported separately,
not silently substituted for that gate. Short sampled windows, not a
long-duration or unsampled-peak guarantee.

```text
2026-11-24T21:28:13Z pid=38508 INFO check all files requested: the next pass re-reads every add-on file
2026-11-24T21:28:13Z pid=38508 INFO window hidden
2026-11-24T21:28:24Z pid=38508 INFO sync pass took 11792 ms: 262 file(s) reused from the hash cache, 131 read
```

Ten samples at elapsed 1.5–11.2 s overlap that active full-read interval.
The initial visible-window memory attempt missed the BigLab transfer and
is **not** used to establish the during-pass memory result.

### Diagnostics and focused tests

Actual saved GUI report:

```text
app_version: 0.3.2
build_commit: f95def1363cd5d3407e6ac1bc6adb60d42d4314b
This computer: enrollment=active retired=off report=current last_report_age=none watermark_seq=0 app_version=0.3.2
Computer A: enrollment=active retired=off report=current last_report_age=0h watermark_seq=0 app_version=0.3.2
Computer B: enrollment=active retired=off report=current last_report_age=0h watermark_seq=0 app_version=0.3.2
```

No synthetic/real computer or profile names or drive paths in its text;
all Documents saves used a process-local USERPROFILE redirected into the
lab. No file was saved to the owner's Documents on this recheck.

Exact source was a `git archive f95def1` extraction under the lab, not a
modified product checkout. Executed:

```powershell
cargo test --manifest-path D:\WoWSyncLab\R9-check\source\Cargo.toml --target-dir D:\GitHub\wow-sync.worktrees\R9-check\target --locked --offline -p wow-sync-app --bin wow-sync -- a_restart_with_a_saved_enrollment setup_finishing_for_an_enrolled a_panic_in_a_pass a_service_opened_from_a_saved_enrollment after_a_panic_the_service_is_reopened the_quit_sheet_is_outside --nocapture
cargo test --manifest-path D:\WoWSyncLab\R9-check\source\Cargo.toml --target-dir D:\GitHub\wow-sync.worktrees\R9-check\target --locked --offline -p wow-sync-adapters --lib a_check_all_files_request_survives_a_reopen_until_a_full_pass_completes -- --nocapture
```

App **6/6 PASS in 2.28 s**; adapters **1/1 PASS in 0.01 s**.
Observed injected panics at final `backend.rs:1103` followed by passing
recovery/reopen/cache/quit assertions. The native-layout test is supplementary
to the actual on-screen rectangle check, not a replacement for it.

Mandatory local tiers were rerun from this evidence branch (unchanged
7e4a43d product source plus the report), not misrepresented as a full local
f95def1 suite. Initial offline attempt **FAIL** before tests could run:

```text
error[E0004]: non-exhaustive patterns: `PublishOutcome::Yielded` not covered
crates\wow-sync-runtime\src\service.rs:3601:15
```

The shared worker target had been used for both the archived newer source
and this older checkout. Its cached newer core enum contained `Yielded`,
which the older checkout's on-disk enum does not contain. Cleared only this
worker's six local-package build caches with `cargo clean -p`, changed no
source/assertions, and retried: **offline PASS / native PASS**, both exit 0.
Initial required-case checking was **SKIPPED** because compilation failed;
it passed on retry. Native was first run after that successful retry.
f95def1 itself additionally has its successful dispatch verification tiers,
the exact-source targeted tests and the independent packaged checks above.

Lab was retained solely for the subsequently assigned 104d238 recheck,
then removed as recorded below.

## Artifact 104d238

Follow-up assigned after f95def1 observations were complete. Requested
dispatch **45492591074**, head
`104d238d8124fb278a48eadbfb17ad32df9c3a3d`.
The watch encountered an HTTP 502 near packaging; that was a **watch/API
error**, not a failed build. A fresh `gh run view` confirmed overall
**success**, matching head, Windows job **success**, publishing **skipped**.
Only then downloaded and ran the new package.

```text
ZIP SHA-256, matched supplied .sha256:
29ef679a8785b150b2b5ddd562fa01137b1f127ba5964a755bf63fdfa3f6ab2e
packaged wow-sync.exe SHA-256:
40713a9c3903c5f7f8ea6b5bdf5076bc8c2d7a2b949038bd3af8feec2eb2dd7a
actual GUI-saved report build_commit:
104d238d8124fb278a48eadbfb17ad32df9c3a3d
```

Executed from
`D:\WoWSyncLab\R9-check\artifact-104d238\expanded\wow-sync.exe`.
Captured process path independently matched that executable. Same machine,
UIA timing method and safety rules as above. Restored pristine synthetic
conflict at its original paths, linked only the **original 32-payload,
3.35 GB BigLab** into a, and completed its baseline publication. The 96
memory-only aliases from the previous artifact were not present.

| Requested 104d238 item | Result | Actual evidence |
|---|---|---|
| G1: exact unrelated-Decide / invalidated BigLab case | **PASS** | Settled HideAnything local edit and offered decision; changed only all BigLab mtimes to another settled past time, Sync now, Syncing visible. Decide feedback **52 ms**, fresh accepted result **129 ms**, dispatch **15 ms**. Same 33-file / 3,355,443,259-byte BigLab, no full-check request. f95def1's corresponding result was 2,209 ms. |
| G1: first-upload clicks | **PASS** | Never-used `fresh104\e` install/provider/state, real GUI enrollment. Syncing visible and **zero own published code-state files** before both clicks. Preview **163 ms** / 16 ms dispatch; Decide **81 ms** / 15 ms dispatch. Resumed the yielded initial upload with Sync now before Decide, exactly as the final f95def1 initial-upload check. |
| G3: quick saved-enrollment restart | **PASS** | Quit first instance in **197 ms**. Restarted same state: main Overview observed in **195 ms**, not setup; scoped cache survived (**4,098 bytes**). New Check all files accepted and completed, `.full` marker cleared, no missing-setup warning. Final Quit **201 ms**. |

Actual result lines, normal-mode PID 1516:

```text
2026-11-24T21:56:34Z pid=1516 INFO Decide: accepted, wrote record 3467a2d0b6fa (15 ms)
```

Restart PID 1620:

```text
2026-11-24T21:56:39Z pid=1620 INFO sync pass took 4164 ms: 79 file(s) reused from the hash cache, 40 read
```

That is **40 cumulative reads**, including initial/interrupted work around
the restart, not a claim that the 35-file install contains 40 distinct
files. The full-check request completed over the 35-file tree and its
persisted `.full` marker was gone afterward. Actual GUI Preview/Save on
this process additionally completed in 77/98 ms and independently confirmed
the new full build commit quoted above.

Fresh first-enrollment PID 1288:

```text
2026-11-24T21:56:46Z pid=1288 INFO ExportDiagnostics: ok, 29 line(s) of text (16 ms)
2026-11-24T21:56:46Z pid=1288 INFO Decide: accepted, wrote record e921faa377c5 (15 ms)
```

No product changes, extra GUI panic injection, or additional full matrix
were performed for this bounded follow-up. G1 idle/post-Sync, G2, RV9-15/16,
checks 10–14 and memory were **NOT RERUN on 104d238**; their f95def1 results
remain explicitly attached to that artifact. All three requested 104d238
rechecks pass; no new finding was observed in this scope.

### Final validation and cleanup

After staging both final artifact sections, reran the mandatory local
evidence-branch tiers: **offline PASS / native PASS**, both exit 0.
No tests/assertions, product code or required-case lists changed. Only this
report is staged. The subsequent edit records these outcomes and cleanup.

Verified zero remaining executables rooted in the verification lab after
the captured app PIDs exited. Removed the entire
`D:\WoWSyncLab\R9-check` directory and verified it no longer exists,
including copied AddOns, stress data, synthetic providers/states, archived
source, fixture-helper sources, reports, screenshots/CSVs and downloaded
packages. Ordinary ignored Cargo build outputs remain in the worker's
`target`; no lab process or monitor remains running.

Protected real-group/WoW data remained read-only; the owner's application,
its state, Battle.net and `D:\WoWSyncDemo` were not changed or stopped.
No imported Lua, saved-variable files or legacy CMD metadata executed.
No push, merge, tag or public publishing was performed by this worker.

**Final bounded conclusion:** f95def1 fails G1's dirty-unrelated-package
case; all its other requested checks pass at the stated evidence levels.
104d238 passes all three subsequently requested targeted rechecks.
