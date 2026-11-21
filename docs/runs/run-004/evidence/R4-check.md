# R4-check independent verification

Source: `511c7041fa3138af15ed99dac8b9ae1e0c9772b3`, branch
`run004/R4-check`, checkout `D:\GitHub\wow-sync.worktrees\R4-check`.
Verifier: R4-check, assigned independent-verification model `gpt-6.1-sol`;
no alternate model or nested agent used. The prompt supplies this selection;
no independent model-introspection facility was available to the worker.
Windows 11 Pro `10.0.26300`, x64; Rust `1.100.0 (0ef1bc386 2026-11-09)`.
Intel i7-13700HX, 63.7 GiB RAM, Intel UHD and RTX 2000 Ada laptop GPUs.
Manual lab and final-test synthetic data are beneath `D:\WoWSyncLab\R4-check`;
the initial framework-temp exception is disclosed below.
No real game/provider data or other checkouts deliberately used.
No `gh`, GitHub API, or live release-fetch command was run. Initial verification
scope mistakes are disclosed below; the initial audit's network isolation
cannot be claimed.

Authority read: AGENTS, COMMON, R4-check packet, README, SPEC, ACCEPTANCE,
ENGINEERING, OWNER-DECISIONS D-23/D-24/D-25, PLAN, USAGE, MAC-OWNER,
and implementation evidence (app, integrate, removal, settings, ops,
notify-diag, mac-kit). Tests/GUI evidence below are independent executions,
not adoption of author claims.

## Commands and environment

Every Cargo command starts in the checkout above with:

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\R4-check'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Initial `git status --short` was empty, branch and HEAD matched the assignment.
No nested agents. Raw evidence and the reusable UI Automation driver are
in [R4-check](R4-check/).

## Verdicts

| Packet check | Verdict | Evidence |
|---|---|---|
| 1 Tiers and A-38 controls | PASS | Both tiers rerun; six author controls plus three verifier controls each fail their named test with exit 101. Every mutation reverted. |
| 2 Windows GUI and notification | FAIL | Working actions and visible blockers listed below; missing screens, inaccurate settings banner, hidden refusal feedback and accessibility gaps. Notification and some window states BLOCKED. |
| 3 Recovery without terminal | FAIL | Removed-code restore and peer-held code rollback complete through GUI alone. Local recovery copies lack an actionable recovery entry; settings/paired recovery BLOCKED by Q-77. |
| 4 Memory and sync-start | PASS | Windows a/b/c at full scale: maximum 37.26 MiB; 20/20 show cycles. Separate actual new-addon transfer completes within 2.28 s of publisher launch. macOS NOT RUN. |
| 5 Latest main CI, including macOS | BLOCKED | Assignment explicitly forbids live GitHub calls; `gh run list/view` were not run. No fresh run URL or green claim can be supplied. |
| 6 Records and macOS accounting | FAIL | Evidence exists for sampled ledger rows, but the final USAGE phase-C barrier sentence is stale. Latest CI claims cannot be independently confirmed. All Mac runtime rows remain NOT RUN. |

## 1. Tiers, build and controls (A-38/A-39/A-43)

Commands:

```powershell
cargo xtask verify --tier offline
cargo xtask verify --tier native
cargo build --release -p wow-sync-app
```

Initial tiers: six offline checks PASS and native PASS, exit 0.
Release build PASS, exit 0, optimized profile, 2m 15s. Four existing
adapter warnings in `update_http.rs` (unused test-only imports/type).
No warning was suppressed and no product file was changed permanently.

Release artifact identities:

| Artifact | SHA-256 |
|---|---|
| `target\release\wow-sync.exe` | `01365854864E142C58F88CA59E53CDA522AD9D7AAC474852F0D2F6BFDFCA8980` |
| `target\release\devkit.exe` | `E5D04F710D4D2D9AA39D20822A4DEB67FBC5E1183EAE09607712377CC14A77E3` |

### Verification-scope incidents and scoped rerun

The initial tier command did **not** override TEMP/TMP, so framework-created
synthetic fixtures used the system temp directory rather than the assigned
lab. Also, `xtask` invokes `cargo deny check` without `--offline`; the initial
invocation did not enforce the assignment's network restriction, and its
traffic was not captured. These are verifier procedural mistakes, not product
passes or permission to access real data. No real game/provider data was
deliberately used; no live release test or GitHub CLI/API call was made.

All control runs and the final tiers use neutral TEMP/TMP. The final tier
rerun also uses `CARGO_NET_OFFLINE=true` and an evidence-only cargo proxy
that adds **only** `--offline` to `cargo deny check`. It forwards every other
argument unchanged to the installed cargo; tests/assertions/cases are unchanged.
The proxy is compiled into an ignored build-artifact directory, not installed.

```powershell
$env:TEMP='D:\WoWSyncLab\R4-check\temp'
$env:TMP=$env:TEMP
$env:CARGO_NET_OFFLINE='true'
New-Item -ItemType Directory -Force $env:TEMP,target\verify-tools | Out-Null
rustc docs\runs\run-004\evidence\R4-check\offline-cargo.rs -o target\verify-tools\cargo.exe
$env:PATH = "$(Get-Location)\target\verify-tools;$env:PATH"
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

Scoped rerun: every offline check PASS, native PASS, exit 0. Native report:
source `511c7041fa3138af15ed99dac8b9ae1e0c9772b3`,
`generated_at=2026-11-20T23:11:16Z`,
6 native cases / 676 test results. `git_dirty=true` is verifier evidence only.
This full restored-code run covers all nine named tests below.

### A-38 controls

All edits and reversions used `apply_patch`. After **each** reversion:
`git diff --exit-code` returned 0. `git status --short` showed only owned
untracked evidence. No mutation was committed.

Run a row with:

```powershell
& docs\runs\run-004\evidence\R4-check\control.ps1 -Package <package> -Test <exact-test>
```

The driver executes `cargo test -p <package> <exact-test> --lib -- --exact`
(app uses `--bin wow-sync`). It sets PATH and neutral TEMP/TMP.

| # | Origin / boundary | Temporary production mutation | Package / exact test | Actual result |
|---|---|---|---|---|
| 1 | author / removal | `removal.rs::candidate_is_stale`: insert `return false;` | core / `removal_tests::an_offline_peer_that_still_owes_cannot_resurrect_a_removed_package` | 1 test FAILED; exit 101 |
| 2 | author / settings | `settings.rs`: authority sequence `<= since` becomes `< since` | core / `settings_tests::the_authority_epoch_holds_an_experiment_and_needs_a_strictly_later_capture` | 1 test FAILED; exit 101 |
| 3 | author / rollback | `reject.rs`: rejected-copy `all` becomes `any` | core / `reject_tests::the_recovered_chain_decides_from_sources_in_four_branches` | 1 test FAILED; exit 101 |
| 4 | author / decisions | `decide.rs`: freshness condition prefixed with `false &&` | core / `decide_tests::a_changed_item_refuses_and_writes_nothing` | 1 test FAILED; exit 101 |
| 5 | author / notify-diag | `diagnostics.rs::find_private`: insert `return None;` | core / `diagnostics::tests::a_string_forced_into_a_typed_field_is_refused_whole` | 1 test FAILED; exit 101 |
| 6 | author / app | `decisions_ui.rs`: warning confirmation condition prefixed with `false &&` | app / `decisions_ui::tests::a_warning_must_be_confirmed_before_the_choice_is_built` | 1 test FAILED; exit 101 |
| 7 | verifier / variant | `target.rs`: product equality wrapped in `true ||` | core / `target::tests::other_variant_state_ignored` | 1 test FAILED; exit 101 |
| 8 | verifier / retired publishing | `service.rs::publish_step`: standing guard prefixed with `false &&` | runtime / `e2e::tests::a_retired_computer_cannot_publish_until_it_re_enrolls` | 1 test FAILED; exit 101 |
| 9 | verifier / offline resurrection | `reject.rs::active`: rejection retention condition suffixed with `&& false` | runtime / `e2e::tests::a_rollback_installs_the_recovered_release_and_an_offline_peer_cannot_bring_the_rejected_one_back` | 1 test FAILED; exit 101 |

Package shorthand: `wow-sync-core`, `wow-sync-runtime`, `wow-sync-app`.
The first driver attempt exited 1 with an argument-parsing error; it did
**not** count as a caught control. The evidence driver was corrected to
array-splat `--` and `--exact`, then row 1 failed behaviorally as listed.

## 2. Actual Windows GUI drive

This is the release **real runtime/window** in `--dev` synthetic-install mode,
not the prototype or mock world. That mode does not change the production
settings cap, and explicitly disables OS services.

Lab: `D:\WoWSyncLab\R4-check\gui`, with independent `a\install`, `a\state`,
`b\install`, `b\state` and one shared `provider` folder. No Dropbox service.
Each install is built with `devkit make-install`; SynAlpha starts with two
different synthetic contents at version 1.0.0; SynBeta starts identical.
Never executed Lua. `home\Documents` is a neutral diagnostic destination.

```powershell
target\release\devkit.exe make-install <lab>\<pc>\install
target\release\devkit.exe add-addon <lab>\<pc>\install _forever_ SynAlpha alpha-<pc> 1.0.0
target\release\devkit.exe add-addon <lab>\<pc>\install _forever_ SynBeta beta-shared 1.0.0
target\release\wow-sync.exe --dev --install <lab>\<pc>\install --provider <lab>\provider --state <lab>\<pc>\state --name Check-<pc> --interval-ms 1000 --recent-ms 200
```

Use the persisted [UIA driver](R4-check/ui.ps1): `-AppPid <captured-id>`
prints the actual tree; `-Action invoke -Name '<control>'` invokes/toggles;
`-Action focus` followed by `-Action key -Key 13` uses Enter.
`tools\mem\MemLib.ps1` `Send-Wheel` scrolls only the captured window.
Offscreen invoke attempts were not credited as actions; controls were
scrolled into view or activated by focused Enter and their effects checked.

A created the group through Next → Use this folder → Next → Next →
identity tick → safe-scope tick → Next → Create a new group → Next →
Set up this computer → Finish and open. B used **Join My add-ons, wowf**
at the group step. Both windows and independent state directories were real.

| Item / acceptance | Verdict | Independent observation |
|---|---|---|
| Two-computer enrolment / A-04 | PASS | Both windows reach live status; one shared provider, separate local state. |
| Decision headline / A-25/A-26 | PASS | Both show `Waiting for your decision on 1 add-on`; SynAlpha needs a choice while SynBeta continues. |
| Literal Review needed promotion | BLOCKED | Two synthetic B identity-change attempts did not yield an inspectable UIA tree. Metadata was restored. A wrongly foregrounded screenshot was discarded, not used as proof. The pure headline test passed in the tiers, but does not prove this window state. |
| Choose / A-26 | PASS | `Choose a copy: SynAlpha…` → first `Keep this`, focused Enter. A and B then have matching code hash `3C62CE1C056A56B71CD9B2FF16FCD5F5586C4CD7D23A333DD3EFE3C76E0922DE`. |
| Warning option stays disabled until tick (R4-app manual 1) | BLOCKED | This real decision has no warning option; no GUI tick gate was observed. The independent broken-warning control is caught by the named app test, not counted as GUI proof. |
| Keep / Resume screens | FAIL | After the choice, no Change decision / Resume automatic updates screen or controls exist. Known author gap, not a disabled control with a reason. |
| Retire (manual 2, A-45) | PASS | Panel names Check-B, preserves history, explains that the settings master must be reassigned. Before tick: `Retire, Tick the box to confirm.` has no InvokePattern. After tick, Retire works; B shows `This computer was retired`. |
| Cancel default focus | FAIL | On opening the inline retire panel, focus stays on `Retire this computer…`; Cancel has `HasKeyboardFocus=False`. |
| Retired publishing controls (manual 3) | PASS | Master controls state `This computer was retired. Join the group again to make choices for it.`; they have no action. B's headline is retired. Independent runtime publication control also catches bypass. |
| Join again / A-45 | PASS | B → Computers → `Join again…` returns to up-to-date status. Old retirement history remains. No automatic settings-master reassignment was observed. |
| Settings-master absence message | FAIL | A and retired B still say `Check-B shares its add-on settings with the group. Settings are never merged.` even though that enrollment was retired, and also when Q-77 blocks all sharing. |
| Rollback code (manual 4, A-18) | PASS | Pause A; publish SynBeta 2.0.0 from B; B → addon → Recovery copies → Go back to this version… → Go back. Both code hashes become `57F17C0FCAC90BB6251345052E79E4C251949898AF9313058A33DD24B60A9622` (the v1 copy). |
| Rollback not-available refusal | FAIL (feedback); PASS (safety) | With the confirmation open, remove only the old synthetic Beta blob, after copying it to a lab backup. Go back leaves B at 2.0.0 and writes no replacement, but displays **no refusal text**. Restore the blob; the same GUI action succeeds. |
| Allow again / manual 4 | PASS | After rollback, Computers exposes `Allow again…`; invoking it removes that block/action. |
| Local-only recovery entry / A-26 | FAIL | SynAlpha has local recovery data after replacement, but its addon page offers only `Restore…, coming later` when no peer advertises the old copy. There is no Go back action for that local recovery. |
| Held-release date | FAIL | Recovery row says `Version 1.0.0, held by Check-A`, with no date. Known author gap. |
| Settings master and exception (manual 5) | PASS (policy only) | Use Check-B as master changes the banner; Use this computer for SynAlpha produces an exception with `Remove this exception`. This is **not** a claim that settings transferred. |
| Turn on settings sync / account chooser | FAIL | No account list or Turn on settings sync control on the real Settings sync page. Known gap; metadata-only fixture below does not satisfy GUI consent. |
| Q-77 Disabled unit / A-11..A-15 | PASS | Seed only known-unit metadata, no saved settings or cap: the real window says `Settings sharing is off until a size limit is approved`; Share/Choose/Restore/Use shared are not offered for this unit. |
| Settings push, choose, use-shared, restore, effective pause/resume | BLOCKED (Q-77) | Production cap is `None`; developer mode has no public test-cap switch. `set_settings_cap_for_tests` is private and test-only. No cap was invented or injected into production code. Pause button exists but no effective settings-transfer/recovery claim is made. |
| Removed section / A-10 | PASS (code) | Delete synthetic B's SynAlpha folder as uninstall stimulus. It eventually appears under Removed on A and is absent on both. A's `Restore` restores code on **both** computers; Removed clears. Maturation was not instant; B restart preceded the observed removal. No unsupported timing claim. |
| Paired removed/settings recovery | BLOCKED (Q-77) | No approved-cap settings workload can be driven through this app. |
| About empty update state / A-28 | PASS | `No update check has finished yet.` plus unsigned-build/manual-install warning. |
| About offline-unavailable check | FAIL | Check for updates is offered, but after invoking it the empty result text remains; no unavailable-source/failure explanation appears. No network source is attached in developer mode. |
| About checking/newest/available states in real window | BLOCKED | No supported CLI/UI fake-source injection. Unit/runtime fake-source tests ran, but cannot substitute for these real window states. No GitHub request made. |
| Startup notification with app name / A-27, T-27-11 | BLOCKED | `--dev` disables notifier/update attachment. Non-dev startup autodetects fixed real-game locations and attaches a live update source, outside this assignment's data/network scope. Adapter acceptance or an author claim is not visual sender proof. |
| Diagnostics preview/options/save / A-33 | PASS | Explicit export → Preview → Save writes a 1030-byte default export to neutral Documents. Toggling options removes Save until a new preview. Both-on and both-off exports reflect their flags; both-off omits synthetic addon names. |
| Diagnostics shared-save guard / A-33 | PASS | Relaunch A with USERPROFILE set to the **synthetic provider** directory, making the existing default destination shared. Preview → Save says `That place is not allowed… Nothing was saved.` File does not exist. This is test fixture setup, not a product file picker. |
| Diagnostics completeness | FAIL | Saved export has `os: windows unknown`, `process_memory: unknown`, `build_commit: dev`, and no journals. Include fingerprints changes its flag but no fingerprints are populated for these known addon trees. |
| Diagnostics file picker | FAIL | None; the real window specifies a fixed `wow-sync-diagnostics.txt` destination. Known gap. |
| Keyboard/focus | PASS (sample) | Choose and Save activated by focused Enter; five Tabs from Pause settings traverse Sync now → Pause syncing → current nav item → settings status → Change master. Visible focus ring retained in diagnostic refusal screenshot. |
| Screen-reader names | FAIL | Three decision actions all expose `Keep this`; release/holder/consequence paragraphs are absent from the UIA tree. Actual NVDA/other screen-reader session NOT RUN. |

The known settings unit is seeded by copying [disabled-unit.json](R4-check/disabled-unit.json)
to A's `settings-local-<synthetic-group>.json` while A is stopped. This contains
only a synthetic placeholder identity and empty bookkeeping; **no settings
contents, checkpoint, history, production cap or private identity**.
It proves Disabled display, not consent or settings sync. See
[settings-disabled.png](R4-check/settings-disabled.png).

### Exact unavailable-rollback stimulus

After B installs Beta 2.0.0 and opens the v1 Go back confirmation, copy the
following **synthetic** blob to `gui\fault-backups`, then remove it:

```text
D:\WoWSyncLab\R4-check\gui\provider\WoWSync\groups\2e6d3bd5e2d947a3ec5512f21dbad7b0\blobs\57\57f17c0fcac90bb6251345052e79e4c251949898af9313058a33dd24b60a9622
```

B has no local Beta recovery copy at this point. Invoke Go back, observe its
2.0.0 TOC unchanged and no refusal message. Copy the blob back from the lab
backup, invoke Go back again, and observe v1 code on both computers.
Never changed provider data outside this synthetic lab.

## 3. Recovery result

GUI-only **actions** restored removed code and rolled back to a peer-held
release. The terminal was used solely to prepare/fault the synthetic fixture
and verify bytes, not to execute those recoveries.
Full recovery acceptance is not green: local recovery lacks a reachable entry,
and settings/paired restoration remain BLOCKED by Q-77 and absent consent UI.

## 4. Memory and useful start (A-29/A-30)

Exact full-scale command, with neutral TEMP/TMP and cargo PATH first:

```powershell
& tools\mem\run.ps1 -Runs 1 -LabRoot D:\WoWSyncLab\R4-check\mem -OutDir docs\runs\run-004\evidence\R4-check\mem
```

Exit 0, `Scale=1`. Raw CSVs and summary are in [mem](R4-check/mem/).

| Scenario | Steady median private resident MiB | Whole-run maximum MiB | 100 MiB verdict |
|---|---:|---:|---|
| a hidden idle | 29.01 | 29.43 | PASS |
| b use UI then hide | 35.54 | 37.26 | PASS |
| c 20 show/hide cycles then idle | 35.05 | 36.26 | PASS |

20/20 cycles visible; zero harness findings. One run per scenario, not a
multi-run representative-platform claim. Concurrent verifier tests/GUI work
occurred during this run; the harness measured its launched app's aggregate,
not unrelated verification processes. GPU memory is separate in the raw
summary (shared medians 12.99 / 39.86 / 21.09 MiB).

The harness's 0.19 / 0.21 / 0.26 s signals are admission/observation **state
writes**, not proof of useful transfer. They are reported as proxies only.
Supplementary actual-work check:

1. Stop the captured B app.
2. `devkit add-addon <lab>\b\install _forever_ SynDelta delta-v1 1.0.0`.
3. Confirm Delta is absent on A; compute B's code hash.
4. Start B release app with the exact lab arguments above and a stopwatch.
5. Poll A's Delta file every 50 ms until its hash matches B's.

Actual A application completes **2.28 s after B launch**, hash
`65ACF995AB0FA678D5E268730CF599EA5D4BE744EA6D0C42EDA8D9D558E8A6FD`.
That is an upper bound on useful sync start and is below ten seconds; it
is one synthetic ready-work Windows trial, not macOS/real-provider proof.

## 5. CI

BLOCKED by the stricter assignment-level no-live-GitHub rule. Neither
`gh run list` nor `gh run view` was invoked. No latest-run URL or fresh
all-jobs-green claim. Earlier author ledger claims are historical, not
verification of `511c704`.

## 6. Record spot-check and findings

USAGE rows 7/8/9 link to real removal/settings/ops evidence; sampled A-38
claims independently reproduced above. Row 10's staged integration commits
and snapshot paths have evidence, including later pass-13 additions.
Rows 11/12 have real test/build evidence but do not establish today's GUI or
latest CI outcome. Row 13 honestly leaves all macOS runtime checks NOT RUN.
PLAN's phase table is a work/scope table, **not** a completed-phase checklist;
its GUI/Mac rows cannot be marked delivered from build/unit tests alone.
USAGE's closing `Phase C remains blocked only on the PENDING OWNER product
questions` contradicts D-25 and the document's own merged phase-C rows.

| ID | Severity | Finding and exact reproduction |
|---|---|---|
| V-01 | major | Kept local update is silently put back to the old version without new-version review. On the initial two-computer SynAlpha ambiguous fixture, choose B's v1 copy; run `devkit add-addon <lab>\b\install _forever_ SynAlpha alpha-v2 2.0.0` while B is active; wait five seconds. B's TOC is back at 1.0.0; both headlines say up to date, with no new-version decision. This does not assert that recovery bytes were destroyed. |
| V-02 | major | Missing Keep/Resume and local-recovery entry. Resolve SynAlpha, reopen its addon page: only `Restore…, coming later` for local recovery; no Change decision/Resume screen. Core/runtime command existence is not GUI completion (A-26). |
| V-03 | major | False settings sharing/master-absence claim. GUI-select Check-B as master, retire B, inspect A and B Settings sync: banner still says B shares. The same claim appears with the Q-77 Disabled unit. No automatic master replacement was observed; the displayed claim is wrong (A-11/A-45). |
| V-04 | major | Rollback/other-page action failures are invisible. Use the unavailable-blob reproduction above; no refusal text on addon page. About → Check for updates in offline developer mode likewise leaves the empty state. `app.rs` stores the result in `self.notice`; only App settings renders it, and navigation clears it (A-25/A-26/A-28). |
| V-05 | major | Settings consent cannot be given in GUI. Open Settings sync: no Turn on settings sync/account list. Independently blocks supported settings flow even when a cap is later approved; current settings actions additionally remain BLOCKED by Q-77 (A-26). |
| V-06 | major | Diagnostic export lacks recorded journals, OS version and memory; no destination picker. Export/preview/save on the known synthetic lab: unknown fields and fixed path as recorded above. Known author gap (A-33). |
| V-07 | major | Decision screen-reader ambiguity. On unresolved SynAlpha, UIA enumerates three identically named `Keep this` actions without the release/holder/consequence text. Keyboard Enter works, but accessible names do not identify the choice (A-26). |
| V-08 | minor | Retire Cancel is not default-focused. Open peer → Retire; UIA focus remains Retire this computer, Cancel false. Known inline-panel deviation. |
| V-09 | minor | Held-release dates absent; rollback confirmation duplicates full version-summary sentences. Pause A, publish B Beta 2.0, open its v1 rollback confirmation; see [rollback-confirm.png](R4-check/rollback-confirm.png). |
| V-10 | minor | Settings sync navigation still says Coming later despite live policy controls. Observe nav in every retained window screenshot. |
| V-11 | minor | Stale USAGE phase-C barrier sentence. Compare its last line against D-25 and its own merged C1/C2/C3 entries. |
| V-12 | major (verification procedure) | Initial TEMP/TMP and advisory-network isolation were not enforced. Reproduction is the unqualified initial tier invocation. Scoped final reruns pass, but do not retroactively prove the first run stayed inside the requested temp/network scope. |

The diagnostic omission is also independently confirmed in
`crates\wow-sync-runtime\src\diag.rs::input_from`: fingerprint, OS version
and process memory are assigned `None`, journals `Vec::new()`.
No active-journal GUI fault was injected; that runtime row is NOT RUN.

No product fixes were authorized or made. Required follow-up: assigned
implementation workers address the FAIL items; an approved cap and usable
settings-consent/test-mode seam are needed for settings GUI recovery; an
offline app service seam is needed for full About/notification window checks;
network permission is needed for fresh CI confirmation.

## Cleanup and persistence

Captured GUI PIDs: A `34140`, B `29724`, A2 `39228`, B2 `20676`.
First A/B were stopped by those exact PIDs before relaunch. Memory harness
stopped only its captured roots; zero child-survivor findings.
Final cleanup stops A2/B2 by captured PID; no name-based or discovered-child
termination. Both A2/B2 were confirmed exited. The old A PID `34140` was
seen running again later; it was **not** stopped or assumed to be our app
after reuse. Synthetic lab retained under the assigned lab root for reproduction.
No push. Final mutation diff is empty; only these owned evidence paths are
committed. Evidence is kept below 1 MB.
Final cleanup/gate size before these small notes: 887,647 bytes.
Offline commit gate PASS, exit 0; recorded at `2026-11-20T23:30:45Z`:
fmt, clippy, tests, required cases, deny and Markdown links all PASS.
Owned evidence was staged before this gate. Privacy-path scan found no
personal profile path; staged names are confined to the two allowed paths.
Metered worker usage and runtime-model introspection were unavailable in
this worker context; the orchestrator must attach its actual meter/model
record rather than infer them from the packet label.

## macOS owner prerequisites

All macOS runtime rows remain **NOT RUN**, not Windows passes:
T-27-11 native notification sender; T-28-13 live release GET; A-29 private
aggregate memory; A-30 useful sync start; native build/offline verification
through `build.sh`; synthetic enrolment/tray; bundle plist/icon/ad-hoc
signature/sender; second-launch raise; close/hide/menu-bar Show; host name,
Application Support state path and local-time offset.
Owner procedure: [MAC-OWNER.md](../MAC-OWNER.md).
Local CI definition has two offline jobs (Windows/macOS) and four build-only
targets (Windows x64/ARM64, macOS Intel/Apple Silicon); source inspection
does not establish their latest outcomes.

## Recheck — 2026-11-20 (Windows)

Narrow assignment: V-02..V-10 only, checkout
`D:\GitHub\wow-sync.worktrees\R4-recheck`, branch `run004/R4-recheck`,
source `af1cd3fa89968ecb2379b492533ffe034c500073`.
The named app/integration fixes are in this source. This is an independent
release-window execution, not adoption of their test results.
**V-01 was NOT RUN**: Q-88 remains pending. No product fix or mutation made.
Same Windows/Rust environment as above. No nested agents.
Assignment allowance about 500 AIU; actual metering remains unavailable
here and must come from the orchestrator. Scope was kept to these findings.

### Isolation, build and execution

Every build, verification and GUI-driving shell explicitly sets neutral
TEMP/TMP and offline Cargo from the start:

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\R4-recheck'
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP='D:\WoWSyncLab\R4-recheck\temp'
$env:TMP=$env:TEMP
$env:CARGO_NET_OFFLINE='true'
cargo build --offline --release -p wow-sync-app
rustc docs\runs\run-004\evidence\R4-check\offline-cargo.rs -o target\verify-tools\cargo.exe
$env:PATH="$(Get-Location)\target\verify-tools;$env:PATH"
cargo xtask verify --tier offline
```

Release build PASS, exit 0, 2m29s; same four adapter warnings.
Offline tier PASS, all six checks, exit 0; rerun after final staging.
No native/full-memory/macOS rerun is claimed for this narrow evidence task.
Cargo deny uses the offline-only forwarding proxy, not an advisory refresh.
No GitHub/release request, non-dev startup, real game/provider path or
`D:\WoWSyncDemo` used.

Release SHA-256:
- App: `9AC16BC8459C4E85E201661CF140CD57DC2657CBD37B7FE063890768412149D0`
- Devkit: `6F57951797AF5F24A8C77AE6F44C99D58D0775E81D412B934A011576E380D148`

Fresh lab `D:\WoWSyncLab\R4-recheck`: independent `a\install`, `a\state`,
`b\install`, `b\state`, common `provider`; no Dropbox service.
Devkit commands are the original two-addon fixture commands above, with
this lab root and names `Recheck-a`/`Recheck-b`.
App command is the original `--dev --install ... --provider ... --state ...`
with `--interval-ms 1000 --recent-ms 200`.
For app launches USERPROFILE/HOME point to `lab\home`, APPDATA/LOCALAPPDATA
to its neutral subfolders. Diagnostic Documents is under that home.
B's final launch points USERPROFILE/HOME to **lab\provider** solely to
retest shared-save refusal. No settings cap was supplied or invented.

Driver: [recheck/ui.ps1](R4-check/recheck/ui.ps1), `-AppPid <captured-id>`,
`-Action invoke -Name '<UIA name>'` uses focused Enter; `dump` records names
and focus; `scroll -Key -2400` scrolls the actual captured window.
Screenshots assert the foreground PID, crop the app content, and redact
account buttons. One startup tree was not ready yet; waiting recovered it.
Current-page suffixes and an already-expanded rollback confirmation needed
corrected selectors. These driver misses are not product-test failures.

### Verdict table

| Finding | Verdict | Independently observed result |
|---|---|---|
| V-02 | **FAIL** | Keep/Resume controls and confirmations now work. The original local-only code recovery still cannot be reached; the page falsely says no earlier version is kept here. |
| V-03 | **FAIL** | Retired-master and known Disabled-unit banners PASS. Fresh consent with no known units still produces “shares” while Q-77 prevents sharing; see residual below. |
| V-04 | **FAIL** | About's offline error now appears on About. Unavailable rollback still gives no refusal after six seconds; it shows “Saved on this computer” and “Nothing to do.” |
| V-05 | **PASS** | Local account list, empty-selection guard, toggling and persisted consent work. All effective settings-transfer/recovery actions remain **BLOCKED (Q-77)**. |
| V-06 | **PASS (fixed fields)** | GUI-generated/saved export has OS version, nonzero memory and the seeded journal. **Save picker absent**, unchanged and outside the named fixes. |
| V-07 | **PASS** | Three real ambiguous-choice buttons expose distinct release/holder/consequence names; focused Enter selects the intended copy. Actual screen-reader session NOT RUN. |
| V-08 | **PASS** | Cancel has UIA keyboard focus on opening Retire. Enter cancels without retiring; acknowledgment still gates retirement. |
| V-09 | **FAIL** | Duplicated version-summary sentence is gone. Held dates still absent; confirmation falls back to “the installed version”, not the actual installed 2.0.0. |
| V-10 | **PASS** | Settings sync navigation consistently has no “Coming later” suffix. |

### Exact repros, evidence and residuals

**V-02, continuing major.** Enrol A, join B, choose B's ambiguous SynAlpha
1.0.0. Both live code hashes become
`3C62CE1C056A56B71CD9B2FF16FCD5F5586C4CD7D23A333DD3EFE3C76E0922DE`.
A has an actual displaced `main.lua` under
`.wowsync\recovery\synalpha\0a2ee3f3c112ed85f71cb13f566b3491\SynAlpha`,
hash `DF9EC583F6F2B725B6CE8ED7F4FF20280896391D6EBDFE3B80657DE75B5EEDB1`,
plus recovery metadata and TOC. Open A → Add-ons → SynAlpha and scroll down:
“No earlier version of this add-on is kept here. Older versions appear here
when this computer or another holds one.” No restore/go-back action exists
for those local bytes. [Local recovery](R4-check/recheck/local-recovery.jpg).
Resume on active A confirms and reports “Saved and shared with the group.”
For Keep, pause A, publish Beta 2.0.0 from B, open A's Beta: Keep 2.0.0
confirmation Cancel works; confirming Keep and then Resume each reports
“Saved. It is shared when you resume syncing.” These are honest **queued
local intents**, not proof of their eventual publication while paused.
Keep is not offered on the converged Alpha page; only Resume appears.

**V-03, major residual newly reachable through consent.** Before adding any
known-unit metadata, on fresh A: Settings sync → choose one account → Turn
on settings sync → Change master → Use Recheck-b. `units` is empty and the
production cap remains None, yet the banner says
“Recheck-b shares its add-on settings with the group. Settings are never merged.”
This is not truthful Q-77 disabling. The banner checks for a pending-size
unit rather than representing the global missing cap.
Then stop A and seed the original metadata-only Disabled unit, preserving
its actual selected account/consent; restart A. Banner correctly says
“Recheck-b is the settings master, but settings sharing is off until a size
limit is approved.” Retire B from A. Both A and B correctly say B is no longer
active and settings have no active master. No replacement is elected.
Explicit B Join again works. Post-re-enrollment master banner was NOT RUN.

**V-04, major residual/newly visible misleading feedback.** Pause A, update
B's Beta with `devkit add-addon <lab>\b\install _forever_ SynBeta beta-v2 2.0.0`.
On B → Beta → Go back to this version… (A holds 1.0.0), copy the old blob
to `lab\fault-backups`, then remove only
`provider\WoWSync\groups\fe8ec480368882ec5077d2279b3a630c\blobs\57\57f17c0fcac90bb6251345052e79e4c251949898af9313058a33dd24b60a9622`.
B has no local Beta recovery. Click Go back; leave the blob missing for
**six seconds**. B stays at 2.0.0, headline up to date, Next step “Nothing to do”,
notice “Saved on this computer.” No unavailable/refusal message.
[Observed feedback](R4-check/recheck/rollback-refusal.jpg).
Restore the blob and click Go back again: GUI recovery succeeds, hash
`57F17C0FCAC90BB6251345052E79E4C251949898AF9313058A33DD24B60A9622`.
The earlier short-hold trial also recovered after restoration but was not
credited as a sustained refusal check. No rollback timing claim.
About → Check for updates now says
“That didn't work: update checking is not available in this build. Nothing was changed.”
No live update source was attached.

**V-05/V-10.** Create two synthetic account directories under A's
`_forever_\WTF\Account`, another under B, and one under A's other-variant
directory. Only A's two local accounts appear on A. With neither selected:
“Choose at least one account”; activation does not enable consent.
Toggle first on; toggle second on/off; Turn on settings sync.
Persisted local state has `consent=true`, one included account, exactly the
selected one. The panel disappears and “Saved on this computer” appears.
No account names are committed. [Consent](R4-check/recheck/consent.jpg)
has its synthetic account button blacked out. Transfer/push/settings recovery
not exercised or credited: Q-77. No missing nav suffix on any inspected page.

**V-06.** While A is paused, use the edit tool to create synthetic
`.wowsync\journal\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json` containing only
`{"schema":"wowsync.apply-journal/9"}`. App settings → Export diagnostics… →
Preview → Save produces a real export with:

```text
os: windows 10.0.26300
process_memory: 103 MiB
journal: other
```

Subsequent both-on and both-off exports have memory 104 MiB and the same
OS/journal fields, with matching option flags. Option changes remove Save
until Preview is regenerated. No Lua/settings payload executed or exported.
The journal remains on disk. This proves **Other**, not Pending/NeedsHelp;
those real GUI journal rows are NOT RUN.
[Diagnostics](R4-check/recheck/diagnostics.jpg) shows the real preview/save
and OS version; memory/journal are also confirmed in the GUI-saved file.
The memory field is adapter private usage, **not** an A-29 aggregate private
resident-memory measurement; this is not a new memory-gate result.
No file dialog appears: Save uses a fixed neutral Documents path.
Relaunch B with the synthetic provider as USERPROFILE, Preview → Save:
“That place is not allowed… Nothing was saved”; target file absent.
Existing build-commit `dev` and absent populated fingerprints are not fixed
by these commits. No fresh unrelated acceptance claim.

**V-07/V-08.** In the initial unresolved Alpha chooser, UIA names are:
“Keep this: Keep version 1.0.0. Held by This computer. Installs synalpha.”;
same with “Recheck-a”; and “Keep this: Keep it on this computer only”.
[Choice screen](R4-check/recheck/decision-names.jpg).
Selecting the first by focused Enter converges hashes above.
On A's B retirement panel, Cancel `HasKeyboardFocus=True`; plain Enter
closes it, leaving B active. Reopen, acknowledge, Retire: B becomes retired.
The unticked action still says “Tick the box to confirm”.

**V-09, minor residual.** Same B 2.0/A-held 1.0 rollback fixture:
row “Version 1.0.0, held by Recheck-a” has no date. Confirmation now reads
“Go back to SynBeta 1.0.0? Every computer in the group replaces the installed
version with 1.0.0.” It no longer duplicates the full summary, but does not
identify 2.0.0 in that confirmation. No claim that the wrong release was blocked.

### Redaction, cleanup and final gate

All five retained screenshots were individually visually reviewed: only app
content and synthetic identities/paths, no personal user path/name/SID or
desktop contents. Synthetic account button is blacked out. Text and saved
export scans check profile paths, SID patterns and configured user identity;
no matches. Screenshots are lossy crops, not edited product UI.
Only this appended section and `R4-check\recheck\` are committed.
The original evidence remains unchanged; combined owned evidence stays
below 1,000,000 bytes. No push.
Size before these final notes: 997,166 bytes. JPEG metadata inspection found
only 0x5090/0x5091 quantization tables; no personal metadata.

Captured PIDs: A 8520, B 22804, A2 37396, B2 11152.
A/B stopped by their captured IDs before relaunch; A2/B2 stopped and
confirmed exited at `2026-11-21T00:09:56Z`. No recycled-ID, name-based or
child-discovery termination. Synthetic lab retained for reproduction.
All macOS rows, V-01/Q-88, full memory/native tiers and live CI remain NOT RUN
in this narrow recheck. Final staged offline gate: all six checks PASS, exit 0.

## Recheck 2 — 2026-11-20 (Windows)

Source `99e2b01e48d52f90c999bb08f88c178a5bbaed7c`, branch
`run004/R4-recheck2`, checkout `D:\GitHub\wow-sync.worktrees\R4-recheck2`.
Assigned scope: four prior FAIL repros, V-07/V-08 regression spot-checks.
The named runtime/app fixes are in this source. No product changes,
mutations, nested agents, live GitHub calls or V-01/Q-88 recheck.
Allowance about 400 AIU; actual meter/model introspection remains unavailable
here and must be attached by the orchestrator. Same Windows/Rust environment.

### Isolation and executed commands

All execution shells set neutral TEMP/TMP and offline Cargo **from the start**:

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\R4-recheck2'
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP='D:\WoWSyncLab\R4-recheck2\temp'
$env:TMP=$env:TEMP
$env:CARGO_NET_OFFLINE='true'
cargo build --quiet --offline --release -p wow-sync-app
rustc docs\runs\run-004\evidence\R4-check\offline-cargo.rs -o target\verify-tools\cargo.exe
$env:PATH="$(Get-Location)\target\verify-tools;$env:PATH"
cargo xtask verify --tier offline
```

Release build PASS, exit 0, four unchanged adapter warnings. Offline gate
PASS, all six checks, exit 0; repeated after final evidence staging.
No native tier, full memory, CI or macOS rerun is claimed.
The deny forwarding proxy prevents advisory refresh; GUI is the real
release app in synthetic `--dev` mode, without notifier/live update services.
No real game/Dropbox data or `D:\WoWSyncDemo` accessed.

Release SHA-256:
- App: `1617CF5E381720FF659FB2EDD3B66EAF71120544A9392EB51BE5E9DAEB54B6D5`
- Devkit: `230D46B3F3666FDD0DE11544E2F13D2ACDF7D34761C85DEDEDCAA8B304AF0132`

Fresh lab `D:\WoWSyncLab\R4-recheck2`, independent `a\install`, `a\state`,
`b\install`, `b\state`, common `provider`. Devkit uses the original Alpha
different-content/same-version and identical Beta 1.0.0 fixture.
Launch arguments are the original `--dev --install ... --provider ...
--state ... --interval-ms 1000 --recent-ms 200`, with names
`Recheck2-a` / `Recheck2-b`. USERPROFILE/HOME and APPDATA/LOCALAPPDATA
point only to neutral `lab\home` paths. No cap or known-unit seed supplied.
One synthetic account directory on A supports the fresh-consent repro;
no account name/content is committed.

[Recheck 2 UI driver](R4-check/recheck2/ui.ps1) adapts the retained previous
driver **in memory**, changing execution/capture paths to this checkout.
It never executes the helper from another worktree. `invoke` uses focused
Enter; `dump` gives actual UIA names/focus; `scroll`/`shot` operate on the
captured PID, with foreground-PID assertion and account-button redaction.
B's initial Next control was not ready; a later dump showed Welcome and
the guided join then completed. Not credited as a product failure.

### Verdicts

| Check | Verdict | Observation |
|---|---|---|
| V-02 | **FAIL (group result); local restore PASS** | A's displaced copy is reachable and restored. The confirmation promises every computer replaces its version, but B keeps the displaced copy and both ask for another Alpha decision. |
| V-03 | **PASS** | Fresh consent, zero known units: master banner honestly states sharing is off until a size limit is approved. |
| V-04 | **PASS (requested refusal/outcome/success cases)** | Missing files refuse at command time without queuing; files lost after queuing produce visible outcome; restoring the blob and retrying restores Beta. Stale/cross-addon requested notice is a remaining minor below. |
| V-09 | **FAIL (peer-held date); version wording PASS** | Confirmation names installed 2.0.0 and target 1.0.0 correctly. Local recovery has a date; the original peer-held Beta row still has none. |
| V-07 spot-check | **PASS** | Three distinct release/holder/consequence UIA choice names; intended copy selected by Enter. No actual screen-reader session. |
| V-08 spot-check | **PASS** | Cancel is keyboard-focused by default; Enter cancels the retirement panel, without retiring B. |

### Repros and observed evidence

**V-02 / new major group-convergence finding (A-18/A-26).**
Enrol A, join B, open B's ambiguous SynAlpha chooser and keep B's copy.
B's chosen/live copy has hash
`3C62CE1C056A56B71CD9B2FF16FCD5F5586C4CD7D23A333DD3EFE3C76E0922DE`.
On A → Add-ons → Alpha → Recovery copies, the displaced version now has
“Version 1.0.0, set aside here on 20 November. apply” and
**Restore this version…**. Open it and confirm **Restore**:
[local Restore confirmation](R4-check/recheck2/local-restore.jpg).
No terminal command executes the recovery.
A's live hash becomes its earlier
`DF9EC583F6F2B725B6CE8ED7F4FF20280896391D6EBDFE3B80657DE75B5EEDB1`.
That fixes the missing local GUI entry and restores A's actual bytes.

However, B remains at the first hash and both active windows report
“Waiting for your decision on 1 add-on”; B's Alpha page offers Choose a copy.
This occurred **before** A was paused for the independent Beta fixture.
After completing Beta, A was explicitly resumed and allowed further cycles:
same mismatching hashes and decision headlines, both with Pause syncing
(not Resume) at final capture. The local Restore confirmation says
“Every computer in the group replaces version 1.0.0 with 1.0.0.”
Thus local-only success does not establish the promised group restoration.
No timing threshold or root-cause/“resurrection” assertion is inferred.
This was a GUI intentional restore, not the forbidden Q-88 newer-local-install repro.

**V-03 (A-11/A-26).** Fresh A → Settings sync → select its local account →
Turn on settings sync → Change master → Use Recheck2-b.
Persisted state: `consent=true`, `units=[]`; no metadata seed.
Banner: “Recheck2-b is the settings master, but settings sharing is off until
a size limit is approved.” [Fresh consent](R4-check/recheck2/fresh-consent.jpg).
Q-77 transfer/settings recovery remains BLOCKED; no settings bytes were used.

**V-04 (A-18/A-25/A-26).** Pause A while its Beta stays 1.0.0. On active B:
`devkit add-addon <lab>\b\install _forever_ SynBeta beta-v2 2.0.0`.
B → Beta → Go back to this version… (A holds 1.0.0).
Back up, then remove only this synthetic old-code blob:
`provider\WoWSync\groups\d640ef9aa9ce5ed04d8e56c018c3925d\blobs\57\57f17c0fcac90bb6251345052e79e4c251949898af9313058a33dd24b60a9622`.
Click Go back. Visible refusal:
“That release can't be installed again because its files are no longer
available. Nothing was changed.”
`rollback-request-d640ef9aa9ce5ed04d8e56c018c3925d.json` is absent.
B keeps 2.0.0/hash
`494B3062CA931F9F6198F6D7CF2DCDA4D9989AE2EE588323F628E8B1D2D7610E`.
[Command-time refusal](R4-check/recheck2/rollback-refusal.jpg).

Restore the blob; pause **B**; click Go back while files are available.
The request exists and the notice says:
“Requested. It will happen on the next sync, when WoW is closed. Nothing has changed yet.”
Remove the same blob, Resume syncing on B, wait six seconds.
The actual Beta page shows:
“The earlier version could not be read when WoW Sync tried to go back.
Nothing was changed; you can ask again later.”
[Queued outcome](R4-check/recheck2/rollback-outcome.jpg).
Restore the blob again and click Go back; Beta becomes 1.0.0/hash
`57F17C0FCAC90BB6251345052E79E4C251949898AF9313058A33DD24B60A9622`.
No code mutation, injected cap or live service used. The faulted blob is restored.

**Remaining minor notice regression.** After the successful Beta apply and
verified v1 bytes, the old “Requested… Nothing has changed yet” notice still
appears. Navigate B → Add-ons → Alpha: the same Beta request notice is shown
on Alpha's page. Actual UIA dumps confirm both observations. It should not
be treated as current pending state or a result for Alpha.

**V-09, continuing minor date gap.** The original B 2.0/A-held 1.0 confirmation
now reads “Go back to SynBeta 1.0.0? Every computer in the group replaces
version 2.0.0 with 1.0.0.” It correctly names the blocked version once.
But its history row is still only “Version 1.0.0, held by Recheck2-a”:
[date/version repro](R4-check/recheck2/rollback-versions.jpg).
There is no held date for that peer-only release. A's local recovery date
is present as recorded above. The keyed date is absent for peer-only copies;
the GUI must not invent a date, but the original missing-date repro is not fixed.

**V-07/V-08 spot-checks.** Initial Alpha chooser exposes
“Keep this: Keep version 1.0.0. Held by This computer. Installs synalpha”,
the corresponding Recheck2-a name, and “Keep this: Keep it on this computer only”.
On A → Computers → B → Retire, Cancel has `HasKeyboardFocus=True`;
plain Enter closes the panel, leaving B active. No retirement was performed.

### Redaction, evidence budget and cleanup

Dropped **only** the older, unlinked `R4-check\retire-before.png`
(118,182 bytes), as permitted, to make room. Its historical observations
remain in this report and the original commit. All other old images remain.
All five new JPEGs were individually visually inspected: genuine app content,
no personal user paths/names/SIDs or desktop pixels. Text scans check profile
paths, SIDs and configured user identity; JPEG metadata contains only
quantization tables. Account buttons are masked by the driver when present.
Combined owned evidence is verified below 1,000,000 bytes before commit.

Captured A PID **14712**, B **16296**: stopped only by those IDs and confirmed
exited at `2026-11-21T00:48:42Z`. No process discovery/traversal/name-based
termination. Lab retained under its approved synthetic root.
Only this appendix, `recheck2\` and the documented image removal are committed.
Final staged offline verification PASS, all six checks, exit 0. No push.
V-01/Q-88, other GUI flows, native/memory/CI/macOS remain NOT RUN here.
