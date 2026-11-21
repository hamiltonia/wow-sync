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
