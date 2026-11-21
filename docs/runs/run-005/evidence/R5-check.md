# R5-check independent verification

Source: `4675f08ee7125c84c3e4c18b869bb3f92ad4fbb0`.
Checkout: `D:\GitHub\wow-sync.worktrees\R5-check`, branch `run005/R5-check`.
Verifier: R5-check; the agent registry reports actual model `gpt-6.1-sol`.
Windows 11 x64 development PC (D-07), Rust 1.100.0; local synthetic shared
folder, not real Dropbox. No WoW/addon Lua was executed. No public publishing.
Actual platform query: Windows 11 Pro build 26300, i7-13700HX, 63.68 GiB RAM;
[platform and artifact metadata](R5-check/environment-artifact.log).

Authority: [PLAN](../PLAN.md), [packet](../packets/R5-check.md),
[COMMON](../packets/COMMON.md), [SPEC](../../../../SPEC.md),
[ACCEPTANCE](../../../../ACCEPTANCE.md), D-26/D-27.

## Verdicts

| Check | Verdict | Actual evidence / limits |
|---|---|---|
| 1. Tiers and flake | PASS | Offline PASS, 127 required cases against 739 test results; native PASS three consecutive times, no reproduced flake. |
| 2. A-38 and cap boundary | PASS | Five author controls and two independent controls fail named tests with exit 101; every mutation reverted. Actual 33,554,432-byte and 33,554,433-byte files exercised, plus unmarked-install refusal. |
| 3. Windows GUI | FAIL | Originating code remained installed during an uninstall; see F1. Individual successes and blocked observations are below. |
| 4. GUI-only recovery | PASS | Removed recovery restores the peer's absent code and settings; settings restore returns the chosen bytes; code rollback 4.0.0 → 3.0.0 converges and records group rejection. No terminal recovery commands. |
| 5. Memory / readiness | PASS | All unscaled scenarios pass, whole-run peak 37.94 MiB aggregate private resident memory; 20/20 show cycles, no harness findings. Useful peer application independently completed in 1.937 s. |
| 6. Release artifact | PASS | Assigned dispatch succeeded, zip and checksum match, four required files included, actual executable opens; About OCR and diagnostic metadata show 0.1.0; diagnostic build commit matches dispatch. Stopped by captured PID. |
| 7. Release-note accuracy | FAIL | Current update-error description is stale; see F2. Removal claim has the counterexample F1. Other limitations are stated. |
| 8. Latest main CI | BLOCKED | Packet explicitly permits `gh` only for check 6. Did not bypass that restriction to query main CI; no claim of green CI. Orchestrator must supply authorization/evidence. |

This is not a green release gate: failures and unobserved GUI obligations remain.

## Execution and scope

Every shell prepended `$env:USERPROFILE\.cargo\bin` to PATH. Executed tests
use `TEMP` and `TMP` = `D:\WoWSyncLab\R5-check\temp`. GUI installs, state,
provider, diagnostic destinations and extracted artifact are under
`D:\WoWSyncLab\R5-check`. GUI processes receive an explicit `--dev`,
`--install`, `--provider`, `--state`, `--name`, `--interval-ms 1000` and
`--recent-ms 100`; `USERPROFILE` is overridden to a lab home for exports.
Developer mode disables the app's live GitHub source.

One initial tier invocation lacked the neutral TEMP/TMP settings. It was
cancelled during workspace compilation and is not credited as a tier run or
as isolation evidence. The cancellation tool left its cargo/xtask processes
alive; their captured, previously observed PIDs were stopped explicitly.
All completed test executions below used the scoped environment.

Only edit/create tools changed source fixtures and evidence scripts. Logs
were redirected to absolute owned evidence paths. Product-generated lab
files and standard build/test outputs are not hand-authored repository
edits. Main was not edited. No `.NET [IO.File]` call, `Set-Content`, or
`Out-File` was executed. The memory harness's prohibited writers are
replaced in memory with console sinks, not executed.

About's “Copy version info” was an unsuccessful observation method: the
clipboard remained unrelated/stale despite the “Copied version info”
message. That value was removed immediately from retained evidence; no
referenced location was accessed. Subsequent version observation used
in-memory OCR on a PID-validated foreground app window, never the clipboard.
No screenshot file was written or retained.

## 1. Tiers

From the stated checkout:

```powershell
cargo xtask verify --tier offline
cargo xtask verify --tier native   # three independent consecutive runs
```

Full emitted output and detailed generated reports:
[offline](R5-check/offline.log), [native 1](R5-check/native-1.log),
[native 2](R5-check/native-2.log), [native 3](R5-check/native-3.log).
`xtask` captures successful child-command output; its own full output is
therefore short. Reports identify the source commit and commands. Dirty
status is the verifier's untracked evidence, not a remaining product edit.
Native runs completed at 20:25:44, 20:26:30 and 20:27:16 UTC on 2026-11-21.
Final complete offline rerun also PASS:
[output and report](R5-check/offline-final.log).

Release build:
`WOWSYNC_BUILD_COMMIT=4675f08ee7125c84c3e4c18b869bb3f92ad4fbb0 cargo build --release --locked -p wow-sync-app`.
[Initial build](R5-check/build.log) was superseded by the
[clean rebuild](R5-check/build-clean.log) after all controls were reverted.
Only the clean rebuilt binary was used for local GUI work.

## 2. Negative controls (A-38)

Each mutation was applied with the edit tool; tests ran in a separate call;
then edits were reverted and `git diff --exit-code -- <mutated files>`
returned zero before further work. No test or implementation change remains.

All named commands have the form
`cargo test --locked -p <package> <test-name>`.

| Control | Temporary break | Named test / package | Observed failure |
|---|---|---|---|
| Author RV-04 | Add `|| true` to `RemovalSettingsCycle::restore_ops` guard | runtime / `an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both` | Expected captured settings, got `None`; exit 101. [Log](R5-check/author-restore.log) |
| Author Q-77 | Cap 33,554,432 → 33,554,431 | core / `the_approved_cap_is_32_mib_and_a_file_at_the_cap_syncs_but_one_byte_more_does_not` | Actual cap differs from approved value; exit 101. [Log](R5-check/author-cap.log) |
| Author Q-88 | Publish `w.inventory.clone()` instead of `inventory_without` | runtime / `a_newer_local_only_version_after_a_keep_stays_installed_unshared_and_is_asked_every_cycle` | Unshared newer tree was published; exit 101. [Log](R5-check/author-local-only.log) |
| Author DataAtRisk | Drop `local_state_risk` through `.filter(\|_\| false)` | runtime / `unreadable_local_state_is_a_data_at_risk_condition_in_the_snapshot_and_a_notice` | Snapshot has 0 risks instead of 1; exit 101. [Log](R5-check/author-risk.log) |
| Author Keep reason | Map every pending kind to `Ambiguous` | app / `keep_and_resume_send_the_pending_decisions_own_reason` | Expected `PartialCandidate`, got `Ambiguous`; exit 101. [Log](R5-check/author-keep-reason.log) |
| Independent uninstall | Suppress captured-unit cleanup in `removal.rs::cleanup_ops` | runtime / uninstall/restore test above | Settings remain after code removal, instead of `None`; exit 101. [Log](R5-check/own-left-settings.log) |
| Independent exact-size boundary | Change `f.size > cap` to `f.size >= cap` | runtime / `a_file_exactly_at_the_cap_is_captured_by_an_uninstall`, temporarily expanded to an actual 33,554,432-byte file | Exact-size settings wrongly remain installed; exit 101. [Log](R5-check/own-exclusive-boundary.log) |

For the independent boundary checks, only the two existing e2e fixture
setups were temporarily expanded (not new permanent tests): actual files
of 33,554,432 and 33,554,433 bytes, both with cap 33,554,432. Exact size
was captured/removed; one byte over stayed untouched. Both named tests
executed one real test and passed, not merely the secondary binary's
zero-test result:
[at cap](R5-check/boundary-at.log), [over cap](R5-check/boundary-over.log).
The subsequent exclusive-boundary mutation failed the actual exact-size case.
The fixtures and boundary mutation were then reverted.

`cargo test --locked -p wow-sync-runtime install_without_marker_refused`:
[PASS, one test](R5-check/marker.log). Thus approving the cap does not
remove the install marker gate. No permanent tests were added and the
R5-check required-cases section was left untouched.

Two failure logs omit only the synthetic saved-variable literal, preserving
the failed assertion, `Some`/`None`, line, test name and exit status.
The reverted implementation also passes the directly rerun HTTP-404 runtime
and neutral-app-message tests:
[runtime](R5-check/no-release-runtime.log), [app](R5-check/no-release-app.log).

## 3. Actual GUI drive (A-10, A-25, A-26, A-27, A-33)

[UIA/keyboard driver](R5-check/ui.ps1) restricts all app actions to captured
PIDs, refuses disabled/offscreen controls, focuses controls and posts Enter.
[OCR helper](R5-check/ocr.ps1) observes otherwise unnamed rendered text in
memory; it refuses another foreground PID. It does not write an image.

Setup: devkit creates A and B installs with SynAlpha and SynBeta, two
computers sharing one synthetic provider. SynAlpha initially has differing
same-version code and declared settings on matching synthetic accounts.
A creates the group through the real wizard; B joins it.
[A enrollment](R5-check/gui-enroll-A.log),
[ambiguous chooser](R5-check/gui-pending.log).

| Item | Verdict | Observed result |
|---|---|---|
| V-02 group fix (`2cc8858`) | PASS | B chooses its own ambiguous Alpha copy; A → Add-ons → SynAlpha → Recovery copies → Restore this version… → Restore. Both live code hashes become `e16193b65678ba144932eff4d39a55bbe98078a0a703ae565354091849ee1cea`, A's original copy; headline becomes “This computer is up to date.” [Confirmation](R5-check/gui-V02-confirm.log), [result](R5-check/gui-V02-result.log). |
| Keep and Resume real reason | PASS | On Beta's pending newer-version decision, Add-on-page Keep, then another newer local install and Resume, write `reason=new_version_after_decision`, not ambiguous. [Persisted action metadata](R5-check/gui-keep-reason.log). |
| Q-88 local-only | PASS | Fresh Beta ambiguity is kept; B installs 3.0.0. A retains the chosen 1.0.0 bytes while B retains newer bytes through repeated cycles. Needs attention says “Version 3.0.0 … is installed only on this computer and is not shared with the group.” Four choices are Keep the version you chose / Use version 3.0.0 / Follow newest again / Decide later. Decide later leaves the item present. [Attention](R5-check/gui-Q88-attention.log), [options](R5-check/gui-Q88-options.log). |
| RV-04 uninstall | FAIL | Alpha settings disappear on both peers and B's code disappears, but originating A retains code and says “SynAlpha, removing” after the settle wait; F1. [Removed state](R5-check/gui-removed.log). |
| Restore Removed | PASS | A → Add-ons → Removed → Restore (one activation, no second confirmation) restores B's absent code and both settings files. Both restored settings hashes are `32a6d4cb76fcc88ad5f967b0d06c07430d86a15fa6c5f6eb8485f0d0f90b29c2`, matching pre-uninstall state. [Fingerprints](R5-check/gui-removed-restore.log). |
| Q-77 banner | PASS | Enabled Settings sync says “Check-A shares its add-on settings with the group. Settings are never merged.” No “size limit pending” banner. [OCR](R5-check/gui-settings-banner-ocr.log). |
| Q-77 over-32-MiB real window | BLOCKED | Actual-file automated boundary checks ran; no retained oversized GUI fixture was constructed with the permitted edit/create-only writes within this packet. App refusal wording for that real-window state was not observed; no GUI pass inferred from unit tests. |
| DataAtRisk headline / attention | PASS | Turn both notification options off, corrupt only B's lab `pending-report.json`, wait cycles. Headline: “WoW Sync can't read its saved state.” Needs attention repeats this and “Your add-ons and settings may not be recoverable.” Separate read-only toggle inspection observed both zero. [UI](R5-check/gui-risk.log). |
| DataAtRisk native tray | BLOCKED | Direct taskbar and notification-overflow UIA attempts did not expose the own app tooltip. Code/test rendering is not credited as an observed tray state. |
| Diagnostics RV-01 / RV-09 | PASS | Preview with fingerprints while notifications are off; turn summaries on after preview; Save twice. Two distinct files, identical SHA-256 `a815aea1ea0cdad63f4ac090843864a26d51e3d9a705ee5784e20a66092972a1`, both retain preview's `notify_summaries: off`, real fingerprints and build commit. This exercises frozen preview after live state changes, not a fresh export at Save. [Results](R5-check/gui-diag-result.log). |
| Diagnostics forbidden location | PASS | Restart only own B with lab `USERPROFILE` set to the provider, so default export destination is shared. Preview → Save: “That place is not allowed … Nothing was saved.” Provider diagnostic file count zero. [OCR](R5-check/gui-diag-refused-ocr.log). |
| About version | PASS | Downloaded executable's real About rendering says WoW Sync 0.1.0 (OCR reads zero as `O`); diagnostic metadata independently gives `app_version: 0.1.0`. [OCR](R5-check/artifact-about-ocr.log), [metadata](R5-check/artifact-diag.log). |
| About fake/offline HTTP 404 | BLOCKED | Developer mode attaches no update source. Check for updates says “update checking is not available in this build. Nothing was changed.” Did not enable live networking or change product code to inject a source. The actual GUI NoRelease state remains unobserved. [OCR](R5-check/gui-about-offline-ocr.log). |

The first Alpha deletion attempt ran while A was active: it reinstated
files before the empty-directory removal. That setup race was abandoned.
The completed attempt explicitly paused A, deleted its two code files with
the edit tool, removed only the now-empty directory, then resumed A and
waited 36 seconds plus a further five seconds. It produced F1. This
historical/recovery fixture is not represented as an isolated fresh-addon
uninstall test.

## 4. Recovery without a terminal

Recovery actions themselves were exclusively real GUI controls:

- Removed restore: above; peer absent code and both saved-settings files return.
- Restore settings: account consent on both computers, A set as master;
  Share my settings establishes a checkpoint; change synthetic local settings,
  then Settings sync → Restore the copy from 21 November… → Restore.
  The chosen earlier bytes return, independently checked without executing Lua.
  [UI](R5-check/gui-restore-settings.log).
- Rollback: after Beta Resume reaches 4.0.0 on both computers, B → Add-ons →
  SynBeta → Recovery copies → Restore this version… → Restore returns 3.0.0
  on both, hash `34cb7dc67d3ca576dbda550b9df7ea88987846308e8b337a95e2e06ca0d9dac9`;
  both mirrors carry a rejection of 4.0.0.
  [Versions, hashes, rejection](R5-check/gui-rollback.log).

Fixture setup and read-only post-operation hash checks use a terminal.
Neither is a terminal recovery command.

## 5. Memory and useful readiness

[Console-sink adapter](R5-check/memory.ps1) executes the existing
`tools\mem\run.ps1 -Runs 1` body with lab root
`D:\WoWSyncLab\R5-check\mem`, unscaled default durations/scenarios a,b,c.
The only in-memory changes are absolute repository/library paths and replacing
CSV/summary file writers with full console output redirected to the owned
[harness log](R5-check/mem/harness.log). Measurement, sampling, GUI enrollment,
20 show/hide cycles, aggregate process identity tracking and thresholds are
unchanged. No `.NET [IO.File]` writer runs.

Harness completed successfully at 12:46 on 2026-11-21, one run per scenario:

| Scenario | Steady private resident median | Whole-run private resident maximum | GPU dedicated / shared median |
|---|---|---|---|
| a: hidden idle | 28.72 MiB | 29.60 MiB | 0 / 13.00 MiB |
| b: shown then hidden | 35.96 MiB | 37.94 MiB | 0 / 39.80 MiB |
| c: 20 show/hide cycles then idle | 34.90 MiB | 35.95 MiB | 0 / 20.97 MiB |

All are below the **100 MiB aggregate private resident** gate. Private bytes
are a different metric (c median 104.50 MiB), not a failed private-resident
gate. One app-owned process per sample, 20/20 successful show cycles, zero
timeouts and zero harness findings; 660 raw samples retained. First-state-write timings 0.27, 0.19,
0.27 s. Concurrent GUI lab instances model separate computers and are not
helpers/offloaded work; the harness aggregates its own root and descendants.
The remaining GUI instances were stopped before c's final idle tail.

The harness's first-state-write timing alone is bookkeeping evidence, not proof
of useful synchronization. An additional real release-app check created
SynLatency 1.0.0 on both enrolled peers, completed a synthetic B update to
2.0.0, then measured until A's actual live code matched B's new hash.
Complete useful peer application took **1.937 seconds**, stronger than the
ten-second start gate for this one ready, unblocked synthetic change.
[Raw result](R5-check/latency.log). Not a representative throughput/soak claim.

## 6. Assigned release artifact

Read-only `gh` was used only here:

```powershell
gh run watch 44945165878 --repo hamiltonia/wow-sync --exit-status
gh run download 44945165878 --repo hamiltonia/wow-sync -D D:\WoWSyncLab\R5-check\artifact
gh run view 44945165878 --repo hamiltonia/wow-sync --json headSha,event,conclusion,jobs,url
```

[Dispatch](https://github.com/hamiltonia/wow-sync/actions/runs/44945165878),
[watch](R5-check/release-watch.log), [run metadata](R5-check/artifact-run.json):
build succeeds, publishing job skipped, `workflow_dispatch`, source commit
matches the checkout.

Zip: `wow-sync-0.1.0-windows-x64.zip`, 7,695,885 bytes.
SHA-256 `8a457ea121164b157a78fbc566e41d34b968dc6c5ee3fe920c9195fd48a69a5b`
matches the accompanying `.sha256`. Extracted contents are `wow-sync.exe`,
`LICENSE`, `README.md`, `RELEASE-NOTES.md`; actual signature status is
`NotSigned`, exe size 19,428,352 bytes. Extracted app PID 38824 opens on
the lab, About version is observed with in-memory OCR, diagnostic export's
`build_commit` is `4675f08ee7125c84c3e4c18b869bb3f92ad4fbb0`, then that
captured PID and creation time were checked and stopped.

Workflow review: every action reference is a full 40-character SHA;
top-level permission is `contents: read`, only publish grants
`contents: write`; publish requires tag push and tag prefix; its guard
compares tag to `v` plus workspace version before release creation.
Dispatch never satisfies that publish condition; observed skipped job agrees.
Publishing/tag mismatch and unused download-action execution were not run.
Node 20 deprecation annotation is a workflow warning, not a failing step.

## 7. Release-note claim review

`docs\release\v0.1.0.md`:

- Single executable zip, license, README and notes: artifact verified.
- Unsigned, no installer, no macOS binary/runtime, Q-24 synthetic gate:
  limitations explicitly stated; no other-architecture runtime claim credited.
- Setup/tray/status/history, actionable choices, settings consent/master/
  exceptions/pause/recovery and retirement: offline/native named suites plus
  the actual subset of GUI flows above; not all existing feature flows repeated.
- Code sharing and three-state retention, rollback/offline rejection:
  behavioral suites; actual sharing and GUI group rollback above.
- Removal including settings: automated round trip passes, but F1 is a
  counterexample to unconditional working removal.
- Notifications/update notice: deterministic tests; no live notification or
  published-release check claimed here. 404 description is false (F2).
- Previewed/redacted diagnostics: actual frozen two-file export and forbidden
  destination above.
- Historical run-004 ~37 MiB / ~2.3 s statement matches its cited report;
  current memory result is separate.
- Peer-only release-date gap, fixed Documents destination/no picker,
  no telemetry, no real-data enrollment, no macOS runtime and other budgets:
  stated without pretending they were completed.

## Findings and exact reproduction

### F1 — major: originating code survives the completed removal observation

Source `4675f08`, local release, two enrolled lab peers.
Follow the V-02 fixture above: differing same-version Alpha code, B chooses
its own copy, A restores its displaced version for the group. Grant matching
synthetic-account settings consent, select A as master, establish a shared
settings checkpoint. Pause A through GUI. Delete only A's Alpha `.toc`
and `main.lua` with the edit tool; remove the empty Alpha directory; Resume
syncing. Wait 36 seconds, then another five seconds (30-second observation
gap is unchanged). No game process or unavailable provider blocker.

For an exact reproduction of this observed lab history, also include the
preceding abandoned attempt: while A is active, delete the two code files,
allow its one-second cycles to recreate them before removing the directory,
then use the paused deletion sequence above. The failure is not asserted
for a fresh fixture without this history.

Expected: code and owned settings absent on both peers.
Observed: A's settings absent, but A's `SynAlpha\main.lua` and `.toc` exist;
B's code and settings absent. A's list says
“SynAlpha, removing, version 1.0.0, not confirmed yet” and headline “Waiting.”
Removed offers Restore. Restoring through GUI makes both settings return and
B's code return. This is a coupled group-restoration/removal counterexample,
not a claim that every isolated uninstall fails. [UI](R5-check/gui-removed.log).

### F2 — minor: packaged release notes describe obsolete HTTP-404 behavior

Open `docs\release\v0.1.0.md` (also packaged as `RELEASE-NOTES.md`), Known gaps,
Update notice. It promises “Could not check for updates.” and a “server
refused” activity entry while only pre-releases exist.

Reproduction: run
`cargo test --locked -p wow-sync-runtime a_404_means_no_release_yet_and_is_neither_a_failure_nor_a_notice`.
The current implementation produces `NoRelease`, text
“No release has been published yet.”, without the claimed failure activity.
This is the intended run-005 fix, so the notes, not that behavior, are wrong.
Actual GUI 404 remains BLOCKED as stated above.

## Final hygiene

All source/test mutations are reverted; no product, test, tool or required-case
file is committed. Final offline PASS and three native PASS runs are retained.
The harness completed and stopped its own captured roots with no survivors.
Manually launched GUI PIDs 19392, 38452, 38824, 31676 and 37828 were stopped
only after checking their captured creation times. No unrelated process was
stopped. Data labs remain only under the assigned lab root.

Owned evidence directory is 336,222 bytes across 49 files, below 1 MB.
Evidence was screened
for user-profile paths, unrelated Dropbox paths, settings literals and SIDs;
none remain. At 20:47:32 UTC on 2026-11-21,
`git -C D:\GitHub\wow-sync status --short` was empty and
`git diff --exit-code -- crates tests tools .github Cargo.toml Cargo.lock`
returned zero. Main cleanliness is checked again immediately before commit.
No workers were launched and no
budget counters were reset; billing/remaining run budget is maintained by
the orchestrator, not inferred from token or tool counts here.

## Recheck

Recheck source: `f478fca0c9f0ab41e39855c1d8a3347c7956db36`, after
`git merge main` in `D:\GitHub\wow-sync.worktrees\R5-check`, branch
`run005/R5-check`. Executed on 2026-11-21 in a **fresh** synthetic lab:
`D:\WoWSyncLab\R5-check\recheck`. No previous installation, provider or
recovery data was reused. This section supersedes the initial restore
interpretation: its PASS relied on F1 leaving originating code installed.
It did **not** prove that a genuine uninstall could restore originating
code. The new checks below require both peers' code and settings to be
absent before every SynPlain restore.

The orchestrator authorized read-only main-CI queries for this recheck.
The release app was rebuilt with
`WOWSYNC_BUILD_COMMIT=f478fca0c9f0ab41e39855c1d8a3347c7956db36`;
[build](R5-check/recheck/build.log). Every shell prepended Cargo's bin
directory to PATH. Test TEMP/TMP were scoped to the fresh lab's `temp`.
Apps used `--dev`, explicit A/B lab installation, provider and state paths,
`--name Recheck-A|Recheck-B --interval-ms 1000 --recent-ms 100`, and lab
home/log overrides. No live update source, real game or real Dropbox was used.

### Recheck verdicts

| Requested check | Verdict | Observation |
|---|---|---|
| 1. F1 sequence | PASS | V-02 group restore, settings consent/master, paused deletion and resume: Alpha code/settings absent on both peers; Removed row present. |
| 2. Plain uninstall and origin restore | FAIL | Genuine uninstall removes both code and settings. Removing the last addon hides all Removed/Restore controls (F3). After adding unrelated SynAnchor, origin restore succeeds and all six pre-uninstall hashes match. |
| 3. Other-peer restore | PASS | B has no recoverable local SynPlain code bytes; provider-based GUI restore returns both peers' code/settings with all six hashes matching. |
| 4. Restore while WoW runs | PASS | Benign devkit fake-client blocks restore, visible Waiting text appears, neither peer receives partial files, and stopping the captured stand-in permits complete matching restoration. |
| 5. Unavailable / retry / dismiss | FAIL | Missing bytes are safely refused. Queued Unavailable and Dismiss work, but immediate refusal has no Needs attention item; Try again loses a persisted issue when it fails again (F4). |
| 6. Two additional native runs | PASS | Both native runs exit 0; seven native cases verified against 757 results each. No reproduced flake. |

### Removal and full restoration evidence

- F1 fresh sequence: [setup](R5-check/recheck/F1-setup.log),
  [result](R5-check/recheck/F1-result.log). The original F1 is closed on
  this tested source; no abandoned active-deletion attempt was needed.
- SynPlain baseline records A/B code, TOC and settings SHA-256 values:
  [baseline](R5-check/recheck/baseline.log). The two local settings copies
  differ, so matching each peer's own before/after hash is required.
- Plain uninstall: [both peers absent](R5-check/recheck/plain-absent.log).
  With no active addons, [UIA](R5-check/recheck/last-addon-removed.log) and
  [OCR](R5-check/recheck/last-addon-removed-ocr.log) show only the empty state.
  An unrelated, newly created SynAnchor exposes Removed; no removed
  code/settings were manually put back.
- Origin A confirmation, observed through the actual rendered window:
  “This reinstalls synplain on all computers in this group. If World of
  Warcraft is running, it waits until the game closes.”
  [OCR](R5-check/recheck/plain-confirm-ocr.log).
  Confirming in the GUI restores all six matching hashes:
  [result](R5-check/recheck/plain-restored.log).
- Other peer B: [absent](R5-check/recheck/peer-absent.log),
  [local code payload removed](R5-check/recheck/peer-no-code-copy.log),
  [restored hashes](R5-check/recheck/peer-restored.log).
  Its synthetic recovery metadata remained, but the recoverable local
  SynPlain `.toc` and `main.lua` were deleted before requesting Restore on B.
  This proves provider-only code restoration, not absence of all metadata.
- Simulated WoW: [absent before restore](R5-check/recheck/game-absent.log),
  [Waiting UIA](R5-check/recheck/game-waiting.log),
  [Waiting OCR](R5-check/recheck/game-waiting-ocr.log),
  [complete matching restoration](R5-check/recheck/game-restored.log).
  Observed: “Restoring synplain: waiting for World of Warcraft to close.”
  `target\release\devkit.exe fake-client <lab-variant-dir>` launches a
  benign Windows ping stand-in at the lab's `Wow.exe` path; no addon Lua
  or real game ran. Captured PID 39248, creation
  `2026-11-21T21:48:25.4898011Z`, was identity-checked and stopped.
  Restore completed in the subsequent five-second observation.

Every uninstall was performed by GUI pause, synthetic code-file deletion,
removal of only the empty directory, GUI resume and a 36-second observation.
The 30-second removal gap was not changed. File inspection/hash checks
establish outcomes; all recovery actions themselves used the real GUI.

### Unavailable, Try again and Dismiss

1. Uninstall SynPlain again, verify both peers absent, delete its synthetic
   provider main-code blob `ab000ca8...35f79a5`, then confirm Restore on A,
   which has no matching local code fallback.
   [Absent](R5-check/recheck/unavailable-absent.log),
   [refusal](R5-check/recheck/preflight-refusal.log).
   The GUI says the file is not in the shared folder or a local copy and
   “Nothing was changed.” However, Needs attention is empty:
   [UIA](R5-check/recheck/unavailable-needs.log),
   [OCR](R5-check/recheck/unavailable-needs-ocr.log).
   The Removed row has Restore, not the promised persistent Unavailable
   state and Try again.
2. To exercise the journaled path independently, pause A, queue Alpha's
   Restore while its provider blob is intact, then delete that blob
   `a5bebc47...319b6c` before resuming.
   [Paused Waiting](R5-check/recheck/alpha-queued-ocr.log).
   Now Needs attention correctly reports one Unavailable item:
   [UIA](R5-check/recheck/queued-unavailable.log),
   [OCR](R5-check/recheck/queued-unavailable-ocr.log).
   Removed shows Try again and Dismiss:
   [controls](R5-check/recheck/queued-unavailable-controls.log).
   Neither peer has Alpha code/settings:
   [absence](R5-check/recheck/queued-unavailable-absent.log).
3. Activate Try again without repairing the missing blob. It refuses again,
   but the persisted Unavailable item disappears; Needs attention loses
   its count and the row loses Try again/Dismiss:
   [result](R5-check/recheck/queued-unavailable-retry.log).
   Both peers remain absent:
   [absence](R5-check/recheck/queued-unavailable-retry-absent.log).
4. Independently test Dismiss with an additional, disposable SynZed fixture.
   It has code only and first propagates to B. Uninstall it, queue its
   Restore while paused and delete its provider code blob
   `0883b849...b2b3034` before resuming.
   [Queued](R5-check/recheck/dismiss-queued.log),
   [Unavailable](R5-check/recheck/dismiss-unavailable.log).
   Activate the Removed row's Dismiss: the Needs attention item disappears
   without installing code/settings on either peer:
   [result](R5-check/recheck/dismiss-result.log),
   [absence](R5-check/recheck/dismiss-absent.log).
   An initial automation attempt selected nonexistent Dismiss index 1 and
   took no action; selecting the actual sole Dismiss control completed
   this observation. No missing provider blobs were repaired by hand.

### New findings and exact reproduction

#### F3 — major: final uninstall hides the supported GUI recovery entry

On `f478fca`, enroll A/B and complete settings-enabled Alpha and Plain
uninstalls in that order, leaving zero active addons. Open Add-ons.
Expected: Removed entries and Restore remain available.
Observed: “No add-ons found in this installation yet.” with no Removed
section or Restore controls, confirmed by UIA and OCR linked above.
Creating unrelated SynAnchor makes both Removed rows appear again.
The early return at `crates\wow-sync-app\src\pages.rs:727-730` precedes
Removed rendering at line 825. This blocks RV-04 / GUI-only recovery
precisely when the last addon is removed; the anchor is a test workaround,
not a supported recovery path or a product fix.

#### F4 — major: failed restore preflight loses the actionable issue

On `f478fca`, with at least one unrelated active addon, uninstall an addon
whose originating computer has no local code fallback.
Delete one required provider blob and confirm Restore. Expected:
Unavailable remains in Needs attention with Try again/Dismiss until
successful recovery or deliberate Dismiss. Observed: only transient result
text; no persistent item or retry controls.

For the stronger retry reproduction, queue a valid restore while paused,
delete a required provider blob, and resume. Unavailable appears correctly.
Press Try again while the blob remains missing. It fails, yet clears the
existing issue and removes the actionable Needs attention item without any
successful recovery or Dismiss. No partial code/settings is installed.
`crates\wow-sync-runtime\src\service.rs:926` clears the issue before
`plan_requested`; its error mapping returns immediately without persisting
the refusal. The linked immediate-refusal and queued-retry logs demonstrate
both paths independently.

### Tiers, release notes, CI and hygiene

Additional offline PASS:
[log and report](R5-check/recheck/tier-1.log), generated 21:42:39 UTC.
After writing the recheck report, the
[final offline tier](R5-check/recheck/offline-final.log) also passed all
six checks, including Markdown links.
Two additional native PASS:
[first](R5-check/recheck/tier-2.log),
[second](R5-check/recheck/tier-3.log).
Reports identify `f478fca`; dirty state comprises only owned evidence.
F2's known-gap wording is corrected to “No release has been published yet.”
in `docs\release\v0.1.0.md`; no new GUI fake-404 claim is made.

Assigned-source main CI
[44952684441](https://github.com/hamiltonia/wow-sync/actions/runs/44952684441)
was **CANCELLED**, with all six jobs cancelled:
[metadata](R5-check/recheck/main-ci.json).
Subsequent main CI
[44952756402](https://github.com/hamiltonia/wow-sync/actions/runs/44952756402),
source `8f17099820ec4f7152d37b84dd374d963e39e9b6`, also completed
**CANCELLED**:
[metadata](R5-check/recheck/latest-main-ci.json).
Latest main CI at the final query,
[44954027850](https://github.com/hamiltonia/wow-sync/actions/runs/44954027850),
source `0df1a8e17e5a02646de7032b990adcfcb0f65250`, remained **IN PROGRESS**:
[current metadata](R5-check/recheck/current-main-ci.json).
The tested source was not silently changed to that newer main.
CI verification therefore remains BLOCKED; recheck does not claim a release
gate PASS. Memory, release artifact and earlier blocked GUI observations
were not repeated by this bounded recheck.

Captured GUI roots A1700 (`2026-11-21T21:41:31.0872240Z`) and B33692
(`2026-11-21T21:41:53.6590710Z`) were creation-time checked and stopped:
[cleanup](R5-check/recheck/cleanup.log).
No unrelated process was stopped. Before final report edits, evidence
directory size was 383,231 bytes across 89 files, below 1 MB.
Logs contain labelled fingerprints/absence results, not settings contents;
synthetic account labels are redacted from retained GUI logs.
The read-only observation helper names only its scoped synthetic fixture.
Main status was empty during final inspection and is checked again before commit.
Only owned evidence is staged. No product/test edits, push, tag, nested
workers or budget-counter resets were made.

## Final recheck

Source/tag candidate: `8ecb62160ed070a456f1cea5abf2126ef082ce86`, after
`git merge main` in `D:\GitHub\wow-sync.worktrees\R5-check`, branch
`run005/R5-check`. Windows 11 x64, 2026-11-21. Fresh lab:
`D:\WoWSyncLab\R5-check\final`, with one enrolled synthetic computer and
one settings-enabled addon. This bounded final check does not repeat the
earlier two-peer restoration, memory or other blocked GUI observations.
All new recovery requests are made through the real GUI; the terminal is
used only for fixture/fault setup and read-only outcome checks.

| Final item | Verdict |
|---|---|
| 1. F3 last-addon GUI recovery | PASS |
| 2. F4 refusal, failed retry and repaired retry | PASS |
| 3. RV5-01 corrupt-blob refusal | PASS |
| 4. Assigned release artifact smoke | PASS |
| 5. Assigned candidate CI jobs | PASS |

### Final F3 — PASS

Rebuilt the release app and devkit binaries with
`WOWSYNC_BUILD_COMMIT=8ecb62160ed070a456f1cea5abf2126ef082ce86` and
`cargo build --release --locked -p wow-sync-app`:
[build](R5-check/final/build.log).
An initial invocation wrongly treated the `devkit` binary as a separate
package and failed before building; the corrected successful build above
supersedes it.

Created and enrolled only a scoped, marked synthetic installation:
[enrollment](R5-check/final/enroll.log). Enabled its synthetic-account
settings scope and explicitly selected this computer as settings master.
The rendered page confirmed settings syncing. An exploratory assumption
that settings use the code-blob filename layout was wrong; this is not
credited as a product failure or as publication evidence.

Captured app PID 31228, creation `2026-11-21T22:11:15.0829990Z`.
Launch used `--dev`, explicit final-lab installation/provider/state paths,
`--name Final-A --interval-ms 1000 --recent-ms 100`, and scoped lab home/log
overrides. Cargo bin was prepended to PATH in every shell; tests use the
final lab's `temp` directory as TEMP/TMP.

GUI pause, deletion of only the two synthetic addon code files, removal of
the empty directory, GUI resume and a 36-second observation genuinely
uninstall the last active addon including its owned settings:
[absent](R5-check/final/F3-absent.log).
Despite zero active addons, Removed and Restore remain actionable:
[UIA](R5-check/final/F3-empty.log).
GUI confirmation restores code, TOC and settings, with all three SHA-256
values matching the [baseline](R5-check/final/baseline.log):
[restored](R5-check/final/F3-restored.log).
No unrelated anchor and no hand-installed replacement code/settings were
needed. F3 is closed on this source.

### Final F4 — PASS

Repeat the genuine uninstall; the originating computer has no recoverable
local code bytes. Delete the required provider code blob whose expected
SHA-256 is `d80d1c876a39615b81e045874f4111bdf3c0f014daff6359cc9c45f3da01119b`.

- Immediate GUI Restore refusal persists as Unavailable on Removed, with
  Try again and Dismiss:
  [row](R5-check/final/F4-immediate-removed.log).
  Needs attention still contains the actionable issue after another
  five seconds/multiple normal cycles:
  [item](R5-check/final/F4-immediate-needs.log).
  No live code/settings is installed:
  [absence](R5-check/final/F4-immediate-absent.log).
- Restore the provider bytes from the retained synthetic lab copy, verify
  its SHA-256, deliberately Dismiss the initial issue, pause syncing and
  queue a valid GUI Restore. The actual window shows Waiting for syncing:
  [OCR](R5-check/final/F4-queued-ocr.log).
  The queued request file is independently checked before fault injection.
  Delete the same provider blob, then resume:
  [Unavailable](R5-check/final/F4-queued-unavailable.log).
- Try again with the blob still missing. The persistent issue and row's
  Try again/Dismiss survive the failed retry:
  [Removed](R5-check/final/F4-retry-removed.log),
  [Needs attention](R5-check/final/F4-retry-needs.log).
  Neither code nor settings is partially installed:
  [absence](R5-check/final/F4-retry-absent.log).
- Put back only the provider blob from the retained lab copy; never put
  live addon/settings files back manually.
  [Verified identical copy](R5-check/final/F4-copy-match.log).
  Activate Try again: all three baseline hashes match:
  [complete restore](R5-check/final/F4-restored.log).
  Needs attention clears:
  [cleared](R5-check/final/F4-cleared.log).

F4 is closed on the tested source. Provider repairs are fault-fixture setup
explicitly requested for this check, not a terminal product recovery path.

### Final RV5-01 — PASS

Uninstall again:
[absence](R5-check/final/RV5-before-absent.log).
Replace the provider code blob with incorrect content while leaving it
present:
[fault condition](R5-check/final/RV5-corrupt.log).
No recoverable local code bytes exist. GUI Restore is accepted, but download
verification fails and becomes Unavailable, not an endlessly queued restore:
[UIA](R5-check/final/RV5-result.log),
[rendered text](R5-check/final/RV5-result-ocr.log).
Observed: “the downloaded copy of SynFinal did not verify; nothing was
changed.” No “Settings are restored” note or “next sync” line appears.
After another seven seconds/multiple cycles the issue remains actionable:
[lasting item](R5-check/final/RV5-lasting.log),
[persisted issue](R5-check/final/RV5-issue.json).
No live code/settings exists:
[absence](R5-check/final/RV5-absent.log).
The misleading settings-success/endless-waiting defect is not reproduced.

### Final local tiers

Both required post-merge tiers passed, with scoped TEMP/TMP:
`cargo xtask verify --tier offline`:
[log](R5-check/final/offline.log);
`cargo xtask verify --tier native`:
[log](R5-check/final/native.log),
[source-identifying report](R5-check/final/native-report.json).
Seven native cases were verified against 762 results.
The post-report [final offline log](R5-check/final/offline-final.log) and
[report](R5-check/final/offline-report.json) also pass all six checks.
No product code, tests or required-case assertions were changed.

### Final assigned CI — PASS

[Run 44955214721](https://github.com/hamiltonia/wow-sync/actions/runs/44955214721)
is on `8ecb62160ed070a456f1cea5abf2126ef082ce86`.
All six jobs completed successfully:
[job/step metadata](R5-check/final/ci.json).

| Job | Result |
|---|---|
| verify-offline (windows-latest), including its native tier | PASS |
| verify-offline (macos-latest) | PASS |
| build-only x86_64-pc-windows-msvc | PASS |
| build-only aarch64-pc-windows-msvc | PASS |
| build-only x86_64-apple-darwin | PASS |
| build-only aarch64-apple-darwin | PASS |

Build-only cross-target jobs are not runtime verification for those targets.

### Final assigned artifact — PASS

Waited for
[dispatch 44955214908](https://github.com/hamiltonia/wow-sync/actions/runs/44955214908)
using `gh run watch 44955214908 --repo hamiltonia/wow-sync --exit-status`,
then downloaded it to `D:\WoWSyncLab\R5-check\artifact-final`.
[Watch output](R5-check/final/release-watch.log);
[completed source/job metadata](R5-check/final/release.json).
The source is exactly `8ecb62160ed070a456f1cea5abf2126ef082ce86`;
build succeeded and publish was skipped. No release/tag was published.

Archive verification:
[metadata](R5-check/final/artifact-metadata.log).

- Name: `wow-sync-0.1.0-windows-x64.zip`, 7,723,704 bytes.
- SHA-256:
  `f3224865f4a5ece98a661dd730a9690367e7fafda1526388dc8cd1f88de3f1aa`,
  matching the downloaded `.sha256`.
- Exactly four files: `wow-sync.exe`, `LICENSE`, `README.md`,
  `RELEASE-NOTES.md`.
- Packaged notes match `docs\release\v0.1.0.md` byte-for-byte, not merely
  after line-ending normalization.

Stopped the development app after validating its captured PID/creation time
before launching the actual extracted executable against the scoped final
synthetic lab. Artifact PID 3748, creation
`2026-11-21T22:33:03.2328920Z`, opened the real window.
About shows version 0.1.0:
[OCR](R5-check/final/artifact-about-ocr.log)
(Windows OCR represents the leading numeric zero as “O”).
Through App settings → Export diagnostics → Preview → Save, the artifact
generated a scoped diagnostic text export. Its actual `build_commit` was
read and compared with the full candidate SHA:
[metadata](R5-check/final/artifact-diagnostics.log).
An initial lookup incorrectly assumed a `.json` filename; locating the
actual scoped `.txt` export resolved the observation without accessing any
real user folder or clipboard.
The captured artifact PID/creation time was checked and stopped:
[cleanup](R5-check/final/cleanup.log).

### Final scope and hygiene

No new findings in these final items. F3/F4 are closed on the candidate and
RV5-01 is not reproduced. This is a PASS for the assigned final checks,
not a claim that earlier blocked observations or the owner Q-24 real-data
gate were completed.

Both manually launched roots are stopped; no real game/client, unrelated
process, real settings or real provider data was touched. The fresh lab
remains only under the approved synthetic root. No addon Lua was executed.
At final inspection the owned evidence folder was 638,776 bytes across
129 files, below 1 MB. Retained logs contain scoped synthetic metadata,
labelled fingerprints and redacted account labels, not addon/settings
payloads, user-profile paths or secrets.
Only owned evidence is staged; main cleanliness, product-file diff and
evidence whitespace are checked before committing. No push, tag, nested
worker or counter reset was performed.
