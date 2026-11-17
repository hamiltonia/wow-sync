# R3-check — independent phase-E verification

## Identity and scope

- Checkout: `D:\GitHub\wow-sync.worktrees\R3-check`.
- Branch: `run003/R3-check`; source baseline
  `1bf43fc13355e913fa60dd6183be231ccdea0cdc` (also remote `main`
  when verification began, 2026-11-17).
- Assigned independent Verification role/model: `gpt-6.1-sol`.
  Runtime selection is launcher-controlled, not independently introspected
  by a repository command. No delegation, push, merge or history rewrite.
- Windows 11 Pro x64, build 26300; Rust
  `1.100.0 (0ef1bc386 2026-11-09)`, Cargo
  `1.100.0 (3acc79c0c 2026-11-05)`; pinned GPUI unchanged.
- Read AGENTS, COMMON, packet, README, SPEC, ACCEPTANCE, ENGINEERING,
  PLAN, D-21, HANDOFF and the three named evidence files before testing.
  Additional phase evidence was read for the records spot-check.
- Links: SPEC R-02/R-05/R-12/R-13/R-14, ACCEPTANCE
  A-02/A-06/A-08/A-26/A-29/A-30/A-38/A-39/A-40/A-42.
  This is synthetic-data verification, not real-provider, macOS GUI,
  ARM64 runtime, or release acceptance.

All commands below ran from this checkout. Cargo shells first used:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Verdicts

| Packet check | Verdict | Summary |
|---|---|---|
| 1. D-21 behavior, boundaries, negative controls, spike cleanup | PASS | 20 core target tests and 5 runtime cross-build cases pass; both independently injected defects fail named tests with exit 101; mutations restored |
| 2. Prototype keyboard and UI Automation | FAIL | Shift+Tab, Down/End, sheet containment and names pass; Esc dismisses but does not restore prior sheet focus (RC-3) |
| 3. Memory reproduction | PASS, Windows reproduction only | One full-duration run per scenario; worst sampled private working set 34.89 MiB; macOS remains BLOCKED |
| 4. Latest main CI | PASS | Both workflows green for baseline main; Windows native and macOS offline independently checked |
| 5. HANDOFF phase claims | FAIL (minor records issues) | Phase evidence exists; stale/incomplete records detailed below |

## 1. D-21 — PASS

Read `crates/wow-sync-core/src/target.rs` directly. `eligible_holders`
does not inspect build versions or Interface declarations. It still checks
selected/validated TOC mapping, complete packages and the existing ordering
rules. `compute_target` rejects a peer state unless group, product and
computer match. These layout/integrity/order checks were not mistaken for
the removed build/Interface gate.

A scoped search of core/runtime found Interface retained for parsing/display,
build retained as identity metadata, and the obsolete decision/explanation
types, but no production build comparison or Interface eligibility test.
The obsolete types are still used by the fake prototype (finding RC-1).

Executed independently:

```text
cargo test -p wow-sync-core target::tests
  20 passed, 0 failed, 0 ignored
cargo test -p wow-sync-runtime different_build_
  5 passed, 0 failed, 0 ignored
```

The five runtime cases are
`different_build_{same_interface,different_interface,malformed_interface,missing_interface,new_addon}_applies`.
They exercise actual synthetic application, not only parsing. Core also
covers unknown builds and multi-folder packages across differing declarations.

### A-38 controls

Only the edit tool was used to mutate/restore `target.rs`.

1. Inserted an `else if client.build_version !=
   input.local_client.build_version` hold in `eligible_holders`, returning
   `DifferentClientBuild`.

   ```text
   cargo test -p wow-sync-core target::tests::different_build_syncs_whatever_the_interface_says -- --exact
   running 1 test
   test target::tests::different_build_syncs_whatever_the_interface_says ... FAILED
   assertion message: same interface
   test result: FAILED. 0 passed; 1 failed
   NEGATIVE_BUILD_EXIT=101
   ```

2. Restored that hold, then removed the group/product predicates from
   `compute_target`, retaining the computer predicate.

   ```text
   cargo test -p wow-sync-core target::tests::other_variant_state_ignored -- --exact
   running 1 test
   test target::tests::other_variant_state_ignored ... FAILED
   left: Chosen { ... version: Some("3.0") ... }
   right: InSync
   test result: FAILED. 0 passed; 1 failed
   NEGATIVE_VARIANT_EXIT=101
   ```

   The foreign-product peer's version 3.0 was wrongly chosen. The foreign-group
   peer and foreign-product peer are separate inputs in this existing test.

3. Restored the original predicates. **Before creating evidence files**,
   `git status --short` produced no entries and `git diff --exit-code`
   exited 0. Re-ran the 20 core and five runtime cases above: all pass.
   No mutation is committed.

### Deleted spikes

`Test-Path` returned **False** for each of `spikes\gpui`, `spikes\mem`,
`spikes\a11y`. A boundary-aware `git grep` of `.github`, `Cargo.toml`,
`xtask`, `tools` and `crates` found no active reference to those deleted
directories. The retained `spikes/gpui-pin` is a different directory.
The same search outside historical run records and the explicit
owner-decision/repository-map deletion notes also found no references.

Literal “nothing references them” is not true repository-wide: historical
evidence, packets and deletion notes deliberately retain their old paths.
D-21 explicitly retains spike results in docs; these historical references
are not stale executable dependencies.

GitHub read-only alert inspection independently found alerts 1–4 **fixed**
at `2026-11-17T16:47:22Z`, including the deleted spike's glib alert 1;
alerts 5–6 are **dismissed / not_used**. HANDOFF's wording that alert 1 was
dismissed is not the final GitHub state.

## 2. Prototype

The launcher is a fake-data-only `start` wrapper around
`D:\WoWSyncDemo\bin\wow-sync-proto.exe`. The initial checks launched that
exact target with `Start-Process -PassThru` to obtain its own PID and
creation time, rather than discovering a process by name/parent traversal.
Every forced stop used only that returned PID after creation-time validation.

Also executed the **actual launcher**, through `cmd /c`, with
`--scenario S3 --sheet first-close`. The wrapper PID returned at launch
was 31264; the child self-reported PID 3252 in the launch-specific
[launcher.log](R3-check/launcher.log). Its executable path and creation time
were checked before driving its UI. UIA focused `Quit WoW Sync` and Enter
invoked normal GUI quit: `LAUNCHER_NORMAL_QUIT=True`, command exit 0.
**No Stop-Process was used on this child** and no parent traversal was used.

Demo prototype SHA-256:
`FD0C8F18206DE7DC77B81ADF58D97A1315289C19B4CF5F06814DE69A70BCB7F0`.

Independently built the current app, devkit and prototype:

```text
cargo build --release -p wow-sync-app -p wow-sync-proto
Finished release profile in 2m 47s; exit 0
```

Own release app SHA-256:
`BC53275E81501D4E354366E7CA7A48817DAC809DDF41933A9C173BAA7140222C`.
Own prototype SHA-256:
`A2D5B744628E945E4E0E1AF307A71B64578F5987F8AB854352041FF3EF3ECFB5`.
Embedded build paths can change these binary hashes; source is the baseline
above. The prototype source has not changed since `4002f6e`.

Reproducible driver:
[prototype-check.ps1](R3-check/prototype-check.ps1).
It observes the real UI Automation tree, uses UIA `SetFocus`, and sends
actual `System.Windows.Forms.SendKeys` keyboard chords only after validating
the foreground window belongs to its launched PID. These are not mocked
view-model tests or unobserved posted key messages.

```powershell
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode keyboard
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode pages
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode supplemental
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode launcher
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode focusrestore -Exe "$PWD\target\release\wow-sync-proto.exe"
powershell -NoProfile -File docs\runs\run-003\evidence\R3-check\prototype-check.ps1 -Mode keyboard -Exe "$PWD\target\release\wow-sync-proto.exe"
```

### Keyboard observations

| Task | Result | Observed UIA focus/state |
|---|---|---|
| Shift+Tab | PASS | S14 `All, 300 add-ons, selected` → Tab → `Auction Lens 008…` → Shift+Tab → the same All filter |
| S14 Down | PASS | First row `Auction Lens 008…` → Down → `Bag Sorter Lite 093…` |
| S14 End | PASS | End → `Zone Info 291, up to date, version 1.2.3, confirmed by Laptop` (last displayed order, not numeric suffix order) |
| Sheet Tab trap | PASS | Quit sheet: six Tabs alternate only `Keep running` / `Quit when safe`; four Shift+Tabs stay between the same two |
| First-close containment | PASS | Four Tabs and two Shift+Tabs alternate only Hide/Quit. UIA `SetFocus` on background `Sync now` is redirected to `Hide window` |
| Esc, detail | PASS | Enter on S14 last row → `Back to Add-ons`; Esc restores the same `Zone Info 291…` list-row focus |
| Esc, quit sheet | PASS | Sheet disappears, zero dialogs observed; app remains alive |
| Esc, first-close | PASS | App stays alive and its main-window handle becomes zero (hidden to tray, not quit) |
| Esc, restore prior sheet focus | FAIL | Keyboard-focus `Pause syncing`, Ctrl+Q, Esc: focus lands on the window, not Pause syncing (RC-3) |

Main keyboard launches: PIDs 7980, 5140, 19848; all explicitly stopped by
their launch records. An earlier UIA probe PID 34004 was also stopped.
The same keyboard suite also passed against the **independently rebuilt**
prototype (PIDs 6868, 21848, 26096), with each observed row/control
`offscreen=False`, including the S14 End target.

Supplemental checks exited 0 (observation driver), including the finish-page
heading `This computer is set up` and named button `Finish and open WoW Sync`.
The separate focus-restoration regression check contains a failing assertion
and exited **1**, not a success-shaped observation log.

### RC-3: sheet focus restoration — moderate

Reproduced first through UIA focus on the demo, then using only keyboard
navigation on the independent current-source release:

```text
LAUNCH pid=13684 args=--scenario S4 --page Add-ons
initial FOCUS=ControlType.Button|All, 44 add-ons, selected
Shift+Tab 1 FOCUS=ControlType.Button|Overview
Shift+Tab 2 FOCUS=ControlType.Button|Pause syncing
open quit sheet FOCUS=ControlType.Button|Quit when safe
Esc restoration FOCUS=ControlType.Window|Add-ons · WoW Sync
FAIL_FOCUS_RESTORE expected='Pause syncing' actual='Add-ons · WoW Sync'
STOP tracked pid=13684
FOCUS_RESTORE_EXIT=1
```

The sheet trap/dismissal works, but the
[interaction contract](../../../design/ui/interaction.md) says dismissal
returns focus to where it was. In `app.rs`, `request_quit` records
`self.last_key`, updated by activation/clicks rather than Tab focus; then
`close_sheet` restores that possibly empty/stale key. Capture the actual
focused handle at sheet entry instead. No product repair was made by this
verification worker.

### Accessible names

Both `pages` and `keyboard` exited 0. Real UIA enumeration found **zero
unnamed keyboard-focusable controls** in every following window:

| Page/state | UIA nodes | Focusable | Unnamed |
|---|---:|---:|---:|
| P1/P2 shell and Overview | 29 | 17 | 0 |
| P3 Computers | 22 | 16 | 0 |
| P3 Computer detail | 21 | 14 | 0 |
| P4 S14 Add-ons | 322 | 315 | 0 |
| P5 Add-on detail | 30 | 17 | 0 |
| P6 Reconfirm | 23 | 17 | 0 |
| P7 Needs attention | 20 | 14 | 0 |
| P7 Decision | 22 | 17 | 0 |
| P7 Needs-help detail | 23 | 15 | 0 |
| P8 Activity | 30 | 19 | 0 |
| P9 Settings sync | 27 | 17 | 0 |
| P10 App settings | 41 | 27 | 0 |
| P11 About | 26 | 16 | 0 |
| P13 First-close sheet | 32 | 19 | 0 |
| P13 Quit sheet | 33 | 19 | 0 |
| P14 Scenario switcher | 33 | 27 | 0 |
| P6 Setup step 1 | 5 | 2 | 0 |
| P6 Setup step 2 | 7 | 4 | 0 |
| P6 Setup step 3 | 7 | 4 | 0 |
| P6 Setup step 4 | 8 | 5 | 0 |
| P6 Setup step 5 | 9 | 6 | 0 |
| P6 Setup step 6 | 6 | 3 | 0 |

Names convey status and future/unavailable behavior, for example:

```text
ControlType.Button|Settings sync, coming later, current page
ControlType.Button|Syncing…, unavailable while syncing
ControlType.CheckBox|This is still the right game: World of Warcraft: Forever (wowf)
ControlType.Edit|Computer name, Desk PC
ControlType.Button|Restore…, coming later
ControlType.ListItem|Zone Info 291, up to date, version 1.2.3, confirmed by Laptop
```

This proves name exposure, not Narrator speech or quality of every label.
Candidate buttons repeat `Keep this copy` / `Details, collapsed`; this
check does not claim unique names or screen-reader workflow acceptance.
The tray menu (P12), OS DPI, IME, live OS theme, OS notifications and
shutdown are outside this page-name sample and **NOT RUN**.

The S7 UIA tree independently exposes the D-21 contradiction:

```text
ControlType.ListItem|Questie: the copies are for different game builds, This copy came from a computer with a different game build and does not say it supports this one., affects Questie only, Choose a copy
```

This is fake prototype state, not evidence that the actual core still gates.

## 3. Memory

**PASS for the scoped Windows numeric reproduction**, not a new whole-platform
A-29/A-30 acceptance. Safety self-test independently executed:

```text
powershell -NoProfile -File tools\mem\selftest.ps1
13 PASS checks; SELFTEST PASS; exit 0
```

Ran the unchanged harness, default `Scale=1`, **one run per scenario**:

```powershell
$env:TEMP = "$PWD\docs\runs\run-003\evidence\R3-check\runtime"
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
powershell -NoProfile -File tools\mem\run.ps1 -Scenarios a,b,c -Runs 1 -OutDir docs\runs\run-003\evidence\R3-check\memory
```

Only the scratch location and evidence output path were redirected into this
worktree to obey the worker's no-temp-directory boundary; scenario behavior,
durations, dataset, counters and executable selection were unchanged.
The harness used **this worktree's built app/devkit**, not the mutable demo
app being rebuilt by R3-ui-build. Synthetic three-addon install only.
The lab was deleted by the harness; its empty parent was then removed.
The counter is summed `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate`
over the creation-time-validated owned set, not Private Bytes; GPU counters
are separate. The library re-snapshots membership at each sample and never
stops descendants.

| Scenario | Author steady / sampled peak (MiB) | Independent steady / sampled peak (MiB) | Samples | Result |
|---|---|---|---:|---|
| a: hidden, 120 s idle | 11.61–11.83 / 14.68 | 16.61 / 16.96 | 120 | PASS |
| b: shown 60 s, hidden 120 s | 15.72–16.10 / 19.44 | 32.51 / 34.89 | 180 | PASS |
| c: 20 show/hide cycles, idle 300 s | 15.73–15.87 / 19.63 | 32.75 / 33.60 | 360 | PASS |

All samples had **one owned process**. Scenario c confirmed **20/20**
visible show cycles. Harness findings **0**, exit **0**. Independent
recalculation from the CSVs matched the summary's medians and peaks.
Scenario c's first/last 30 idle samples averaged **32.777 / 32.817 MiB**
(+0.041 MiB), no material idle growth in this run.

The reported low numeric range was **not reproduced**, although every
sample stayed below the unchanged 100 MiB gate. No tolerance was invented
to call the old and new numbers equivalent. During a/b, independent Cargo
verification and another worker's work could contend for machine resources;
this is not isolated-idle-machine evidence.

Independent median GPU dedicated/shared memory (reported separately):
a **0 / 2.85 MiB**, b **0 / 29.76 MiB**, c **0 / 16.71 MiB**.
These differ substantially from the author's dedicated-memory counters.
The PC exposes NVIDIA RTX 2000 Ada and Intel UHD adapters, but the active
adapter was **not independently identified**; no causal explanation is
claimed. Private Bytes a/b/c: **43.85 / 82.81 / 82.54 MiB**;
working set: **40.89 / 74.15 / 67.27 MiB**.

Launch-to-first-admission-file-change: **0.31, 0.19, 0.20 s**;
min/median/max **0.19 / 0.20 / 0.31 s**, no timeouts. This compares with the
author's **0.18 / 0.22 / 0.26 s** over nine launches. It is the same
documented *proxy*, not proof of useful ready-work application or the
blocker/resumption requirements of complete A-30.

Raw data and original harness summary:
[a](R3-check/memory/a-run1.csv),
[b](R3-check/memory/b-run1.csv),
[c](R3-check/memory/c-run1.csv),
[summary](R3-check/memory/summary-abc-090508.txt).
1 Hz sampling does not prove a continuous/start-up peak; no trimming was
used. macOS is **BLOCKED**; real installations and long-run soak are
**NOT RUN**, as in the author's limitations.

The dummy child survives its parent stop and is reported, not killed.
Only explicitly launched roots are stopped; no process found through
parent traversal is stopped.

Also investigated, then **rejected**, a possible capture-window finding:
could a launched process exit and its PID be recycled before `Get-Record`?
A synthetic process-table substitution can manufacture that input, but it
does not model the live launch handle correctly. In the exact harness host,
Windows PowerShell **5.1.26100.9818**, a separate launched dummy process had:

```text
haveProcessHandle=True
m_processHandle=Microsoft.Win32.SafeHandles.SafeProcessHandle
HANDLE_INVALID=False HANDLE_CLOSED=False
disposed=False
NATURAL_EXIT=True
```

The launch wrapper retains that process object while acquiring the record.
An open process handle keeps the Windows process object/ID alive
([Microsoft explanation](https://devblogs.microsoft.com/oldnewthing/20110107-00/?p=11803)).
After capture, creation-time checks cover later reuse. Therefore the
manufactured pre-capture-reuse trace was **not a valid defect reproduction**
and is not reported as a finding. Dummies PIDs 10892 and 33776 exited
naturally; no foreign process was stopped.

## 4. CI

Remote main at start: `1bf43fc13355e913fa60dd6183be231ccdea0cdc`.

- [ci, run 44129725748](https://github.com/hamiltonia/wow-sync/actions/runs/44129725748)
- [spike-gpui, run 44129725922](https://github.com/hamiltonia/wow-sync/actions/runs/44129725922)

Checked with `gh run list --branch main --limit 8 --json
databaseId,headSha,status,conclusion,workflowName,url` and `gh run view
<id> --json headSha,status,conclusion,jobs,url`.

At `2026-11-17T08:59:50-08:00`, `ci` was **completed / success**, with all
six jobs successful:

- `verify-offline (windows-latest)`, including the **completed / success**
  step `Native verification tier (Windows only)`;
- `verify-offline (macos-latest)` on a real macOS CI runner;
- build-only `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`.

`spike-gpui` was also **completed / success**, all four gpui-pin build jobs.
macOS's Windows-only native step is deliberately **skipped**; this is not
macOS native-tier or GUI execution evidence. Cross-architecture jobs are
compile checks, not runtime support claims.

## 5. Records

Every phase has a discoverable evidence/authority record. Read the actual
records, checked artifact presence, and used `git merge-base --is-ancestor`
to verify every named integration SHA below exits 0 against the baseline:

| Phase | Independent spot-check |
|---|---|
| A | `R3-ui-design.md`, `R3-infra.md`, `R3-compat.md`, `R3-removal-design.md` exist and describe their bounded outputs. Prototype commit diff adds only the local crate entry to Cargo.lock, no new third-party dependencies |
| B | `R3-design-review.md` records FAIL; `R3-design-recheck.md` explicitly records UI PASS and outstanding Q-21/removal findings; `R3-design-recheck2.md` leaves R3D-13 major/open. SHAs `917bc05`, `09b2673`, `07fe195` are ancestors. No claim that removal is implemented or accepted |
| C | Merge `8e82673` includes nine memory CSVs, summary and self-test; read the actual summary, whose figures match R3-mem.md. Prototype merge `5c3fa38` includes UIA walk artifacts and 15 screenshots. Independent executions here replace author claims only for the tasks actually tested |
| D | D-21 explicitly approves the prototype and phases E/F. This is recorded owner authority, not inferred from a green test |
| E | D-21 and API merges `f06cbdf`, `804b39c` are ancestors. R3-d21.md records the gating deletion and controls; own reruns are above. R3-dropbox-api.md supplies a research recommendation, citations and unmeasured/live-trial limits, not API runtime evidence |
| F | HANDOFF labels build-out approved, not completed. No unimplemented phase-F feature was accepted here |

**RC-2, minor:** durable records are stale/incomplete:

- HANDOFF says alert 1 was dismissed and still contains a Dependabot
  “owner decision needed / deleting would close” section. GitHub now shows
  1–4 fixed and 5–6 dismissed; D-21 already supplies the decisions.
- `R3-proto.md` ends its offline-verification command row with “see final
  report”, not an actual final result/report link. Both tiers independently
  pass in this verification, but that does not make the older placeholder
  executed evidence.
- The phase-C “P1 to P14 / S1 to S16 run” sentence is broader than the
  retained explicit interaction logs. Screenshot/UIA presence is not proof
  of every scenario transition. This verification likewise does not claim
  that all 16 scenarios' complete workflows were driven.

The removal blocker is preserved and correctly visible; it is not silently
closed because D-21 changed removal policy. No removal behavior was tested.

## Findings

| ID | Severity | Finding |
|---|---|---|
| RC-1 | Moderate, prototype only | S7 still withholds Questie for a different build, contradicting D-21. Confirmed in live UIA and current prototype source; already disclosed by R3-d21. Remove this fake hold/copy before using it as current product-policy evidence |
| RC-2 | Minor, records | Stale Dependabot status, prototype verification placeholder and overbroad retained interaction-coverage claim |
| RC-3 | Moderate, prototype keyboard | Esc dismisses a keyboard-opened quit sheet but restores window focus rather than the previous control; reproduced on an independent current-source release, asserting driver exit 1 |

## Verification commands and limitations

Initial command corrections (not product failures):
`cargo build ... -p wow-sync-devkit` exited 101 because devkit is a binary
of `wow-sync-app`, not a package; corrected build passes.
`cargo test -p wow-sync-runtime --test e2e different_build_` found no test
target because e2e is an internal module; corrected filter ran five tests.
A preliminary `--setup-step 7` launch actually renders step 6 (clamped);
it was not counted as finish-page evidence.

An initial supplemental keyboard attempt refused to send keys because
foreground validation failed; PID 10416 was stopped by its launch record.
That is not a product test failure. A later supplemental run passed those
observations; the independent focus-restoration assertion genuinely failed
as recorded above.

No imported Lua executed; no WoW, Battle.net, real settings or Dropbox
data accessed. No author evidence file, product code or test is changed.
Only this evidence and its owned subdirectory will be committed.

Local repository tiers independently executed:

```text
cargo xtask verify --tier offline
  PASS fmt, clippy, test, required-cases, deny, markdown-links
  overall: PASS (allow-blocked: false) -> exit 0
cargo xtask verify --tier native
  PASS native/tier
  overall: PASS (allow-blocked: false) -> exit 0
```

Final reruns after the evidence was written: offline and native both exit 0;
`git diff --check` passes, and `git diff --exit-code HEAD -- crates tools
Cargo.toml Cargo.lock tests .github` exits 0. Owned raw/helper files total
less than 50 KiB, under the 1 MB packet ceiling.
Remote main was rechecked at `2026-11-17T09:06:57-08:00` and still matched
the tested baseline; both listed workflow runs remained successful.
