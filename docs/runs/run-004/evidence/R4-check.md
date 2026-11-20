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
