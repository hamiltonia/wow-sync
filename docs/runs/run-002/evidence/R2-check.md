# R2-check: independent slice verification

## Verdict: FAIL

Five major product findings prevent accepting the slice. Verification
incident V1 below also blocks a trustworthy memory-gate conclusion.
Successful tests are not evidence that the GUI/design obligations all passed.
This is Windows development-PC evidence, not a release/support claim.

## Scope and reproducibility

- Worker R2-check, independent verification. Actual runtime model verified
  with `list_agents`: `gpt-6.1-sol`, agent
  `1ca392cb-c1d6-4e10-8286-dc10ba82f90d` (D-04/D-17).
- Owned checkout: `D:\GitHub\wow-sync.worktrees\R2-check`,
  branch `run002/R2-check`. Product under test: clean local clone of
  `D:\GitHub\wow-sync`, `main` at
  `2cff174` (`run002: restore records reverted by R2-slice rewrite; incidents; forbid gc`).
- Windows 11 x64 development PC; Rust/Cargo 1.100.0. No new dependencies
  installed. Cargo reused existing downloaded dependencies: this is a clean
  source clone, not a cold-cache/offline-network reproducibility claim.
  Actual versions: rustc `1.100.0 (0ef1bc386 2026-11-09)`,
  cargo `1.100.0 (3acc79c0c 2026-11-05)`, cargo-deny `0.20.2`.
- All scratch work was inside the owned evidence directory, not the OS temp
  directory: `R2-check\clean` and `R2-check\lab`. This implements the packet's
  disposable/temp-clone intent while respecting the session's prohibition on
  writing outside the owned checkout. Both scratch trees were removed before
  the evidence commit. No real WoW data, Dropbox, private profiles,
  S-forever worktree, credentials, or addon execution was used.
- Authority read: repository entry documents, COMMON/worker packet, PLAN
  §6 table item 6 (the packet's “§6.6”), VERIFY, design tests §5/§5b/§6,
  crates §7, and all four requested author evidence files.
- Exact command context for the clean-clone commands below was
  `D:\GitHub\wow-sync.worktrees\R2-check\docs\runs\run-002\evidence\R2-check\clean`;
  initial branch `main`, temporary branch `verification-negative-controls`,
  then restored `main`. Fresh shells used
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

## Executed commands

| Command | Independent result |
|---|---|
| `git clone D:\GitHub\wow-sync <owned-evidence>\clean`; `git checkout main` | PASS; SHA above |
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, tests, required-cases, deny, markdown-links |
| `cargo test --workspace` | PASS: adapters 45, core 152, runtime 16, xtask 24 = 237 passed; 3 ignored provider tests; app/devkit/doc targets have zero tests |
| `cargo xtask verify --tier native` | BLOCKED, exit 1: “no native cases defined yet (4 listed case(s) cannot be run)” |
| `cargo xtask verify --tier provider` | BLOCKED, exit 1: requires D-08 Dropbox test folder and enrolled peers |
| `cargo build --release -p wow-sync-app` | PASS; 1m56s, from clean source |
| Scratch-branch controls, then `git switch main`; `cargo test --workspace` | PASS after restoring every mutation; no product modifications retained |
| Owned-worktree `cargo xtask verify --tier offline` | PASS: all six checks; evidence paths staged so Markdown links were checked |

Native-looking Windows cases also execute in the ordinary workspace suite,
including the real FakeClient slice case; that does not make the **xtask**
native tier PASS. Provider tests were not forced with `--ignored`; no real
provider/second-device behavior is claimed. macOS, ARM64, stable Windows,
screen-reader speech and real Dropbox delivery: NOT RUN.

Raw command output: [offline](R2-check/clean-offline.log),
[workspace](R2-check/clean-workspace.log),
[native](R2-check/clean-native.log), [provider](R2-check/clean-provider.log),
[release build](R2-check/clean-release.log),
[restored workspace](R2-check/clean-restored-workspace.log).

## Negative controls (A-38)

All 24 representative production-code mutations were applied individually on
the scratch branch, tested with
`cargo test -p <crate> <exact-name> -- --exact --nocapture`, and restored in
`finally`. Every row below produced an actual failing named test, exit 101;
none is a build-error-as-detection or a zero-tests-as-detection result.
No tests, assertions, required-case inventory or thresholds were changed.

The exact substitutions are retained in [mutations.py](R2-check/mutations.py)
and machine-readable results in
[negative-controls.json](R2-check/negative-controls.json). Each numbered
mutation also has its own small `.log` file.

| # / mutation | Observed failing test |
|---|---|
| 01 cross-scheme winner | `version::tests::cross_scheme_is_ambiguous` |
| 02 release/prerelease ordering | `version::tests::prerelease_vs_release_is_ambiguous` |
| 03 omitted pre-live-rename activity gate | `exec::tests::activity_checked_before_every_live_rename` |
| 04 immediate live rollback after launch | `exec::tests::no_live_mutation_after_launch_mid_commit` |
| 05 committed finalized without verification | `exec::tests::crash_before_verify_reverifies_on_restart` |
| 06 skipped post-staging destination check | `exec::tests::destination_changed_during_staging_replans` |
| 07 pruning ignores pinned | `exec::tests::pinned_recovery_points_exceed_two_visibly` |
| 08 peer absence removes package from target | `target::tests::absent_package_is_not_removal` |
| 09 partial candidate accepted | `target::tests::partial_suite_candidate_needs_decision` |
| 10 peer timestamp chooses ambiguous winner | `target::tests::record_timestamps_cannot_influence_target` |
| 11 malformed newest skipped for older selection | `admit::tests::newest_malformed_no_fallback` |
| 12 existing corrupt blob trusted | `exec::tests::corrupt_existing_blob_replaced_before_state` |
| 13 suffixed toc selected | `toc::tests::suffixed_tocs_unvalidated_for_recognized_client` |
| 14 completeness limited to local folders | `target::tests::partial_candidate_not_installed_on_empty_peer` |
| 15 recovery assumes remembered Done slots | `exec::tests::crash_during_recovery_at_every_effect_is_reentrant` |
| 16 cleanup affects all operations | `exec::tests::settling_one_op_keeps_other_ops_evidence` |
| 17 existing local object accepted without rehash | `exec::tests::same_size_mtime_corrupt_object_rehashed_and_replaced` |
| 18 RECALL_ON_OPEN ignored | `fs_safe::tests::unavailable_attribute_combinations` |
| 19 synthetic marker gate disabled | `variant::tests::install_without_synthetic_marker_refused` |
| 20 consent accepts unsupported assessment | `variant::tests::consent_alone_does_not_confirm` |
| 21 mount point treated as cloud file | `fs_safe::tests::classify_tag_cloud_vs_link_vs_other` |
| 22 inaccessible possible client ignored | `winproc::tests::inaccessible_possible_client_is_unknown` |
| 23 member/state existence implies confirmation | `status::tests::published_is_not_confirmed` |
| 24 state record written before objects | `exec::tests::publish_failure_leaves_no_state_record` |

Scope of the representative controls: 08 removes the target/effective entry
when a peer lacks a local package; the slice has no removal-plan variant.
11 skips a malformed newest selection: the core observation port contains
only newest bytes, so this is not a full runtime older-payload fetch/admission
implementation. 15 assumes `Done` rather than observing slots; 17 accepts
local objects outright rather than implementing a particular metadata-cache
data structure. These controls demonstrate the named tests reject those
weakenings, not exhaustive mutation coverage.

## Two-install end to end and G1–G9 (A-06/A-08/A-21/A-24/A-25/A-26/A-46)

Independent release binary, UI Automation observations and physical keyboard
events; real adapters; one disposable local provider folder. Synthetic
`Check-A`/`Check-B` are display names, not real machine names. No headless test
is substituted for these observations. Harness:
[gui.ps1](R2-check/gui.ps1), [followup.ps1](R2-check/followup.ps1).

- A published Alpha 2.0; B joined with Alpha 1.0, applied, and **every Alpha
  file's SHA-256** equaled A's afterward; recovery count 1.
- With a copied `PING.EXE` named `Wow.exe` under B's synthetic variant,
  A published 3.0 while B remained on 2.0. Stopping that FakeClient **by PID**
  resumed apply to 3.0; recovery count 2.
- Equal 4.0 labels with different synthetic bodies on A/B produced an
  unresolved addon on both; bodies stayed `a4` and `b4`.
- Injecting a failure after two mutating effects left one interrupted journal
  and retained recovery copies. GUI “Sync now” recovered, applied 5.0 and
  showed zero interrupted updates.

The following are strict comparisons to design observable criteria, not the
author's all-PASS interpretation:

| GUI item | Result | Independent observation / retained evidence |
|---|---|---|
| G1 recognized enrollment | FAIL | No provider/local-state files before consent, and scope/settings exclusion shown; identity is “Forever - product wowf in folder _forever_ (build 1.0.0.1)”, not required “World of Warcraft: Forever (wowf)”. Required enrollment disclosures also incomplete (M4). [before](R2-check/gui/G1-before.txt), [refusal](R2-check/gui/G1-refusal.txt) |
| G2 unfamiliar enrollment | FAIL | Identity-only and scope-only both refused; blocked folder listed; both confirmations permit enrollment. However the folder explicitly excluded from the consent list is published afterward (M2). [preview](R2-check/gui/G2-preview.txt), [scope only](R2-check/gui/G2-scope-only.txt), [identity only](R2-check/gui/G2-identity-only.txt), [both](R2-check/gui/G2-both.txt), [published bytes](R2-check/gui/blocked-publication.txt) |
| G3 safe apply | FAIL | Real file hashes and recovery count PASS; post-apply line only says “alpha: updated and verified on this computer”, losing source/version (m3). Source/version were visible while waiting, not in this post-apply state. [G3](R2-check/gui/G3.txt) |
| G4 running wait/resume | FAIL | File safety and resume PASS; visible headline is “Waiting” plus separate “World of Warcraft is running; nothing was changed”, not specified “Waiting — WoW is running” (m2). [wait](R2-check/gui/G4-wait.txt), [resume](R2-check/gui/G4-resume.txt) |
| G5 ambiguity | FAIL | Labels/reason displayed, bytes unchanged; candidate-to-source evidence absent (M5). [A](R2-check/gui/G5-A.txt), [B](R2-check/gui/G5-B.txt) |
| G6 provider/error | PASS | Missing root shows “Something needs attention” and “Problem: shared folder problem: NotFound at <synthetic-lab>\missing-provider: not found”; missing root is not recreated. Obstruction by a file additionally shows “not a directory”. Both include a plain-English reason; duplicate raw errors and empty “ on ()” are deviations, not hidden (m4). [missing](R2-check/gui/G6-missing.txt), [obstructed](R2-check/gui/G6.txt) |
| G7 truthful confirmation | PASS | A first shows Published/up to date, no confirmation; only after B applies/reports does A show “applied and reported by computer(s) Check-B”. [before](R2-check/gui/G7-before.txt), [after](R2-check/gui/G7-after.txt) |
| G8 interrupted/recovery | PASS | Waiting, two retained points and one interrupted operation visible; Sync now recovers. False “WoW started” cause despite NotRunning is m5. [interrupted](R2-check/gui/G8-interrupted.txt), [recovered](R2-check/gui/G8-recovered.txt) |
| G9 keyboard/focus | PASS | Tab/Shift-Tab, visible yellow focus, Space on both consent controls, Enter on Create/Join/Sync, all action types reachable. Close hides without exit; second invocation exits and re-shows first. [keyboard](R2-check/gui/G9-keyboard.txt), [reverse tab](R2-check/gui/G9-shift-tab.txt), [join](R2-check/gui/G9-keyboard-join.txt), [sync](R2-check/gui/G9-sync-enter.txt), [lifecycle](R2-check/gui/G9-close-reopen.txt), [cropped focus screenshot](R2-check/gui/G9-focus.png) |

Additional observed failure: adding a plain-toc addon to the enrolled
unfamiliar client permanently blocks its ordinary sync cycle; see M1 and
[retained note](R2-check/gui/unfamiliar-addition.txt).

## Findings

### M1 — MAJOR: ordinary additions invalidate unfamiliar enrollment permanently

- Location: `crates/wow-sync-runtime/src/service.rs:615–624`;
  `crates/wow-sync-core/src/variant.rs:431–432`.
- Reproduce: enroll synthetic `_test_`/`wow_test` with both confirmations;
  add a new `Extra\Extra.toc` addon using devkit; wait one cycle.
- Observed: `refused: StaleConfirmation`, “Something needs attention”, no
  addon list, and only Sync now. Restart/Sync now does not offer guided
  reconfirmation; [restart and retry were also observed](R2-check/gui/unfamiliar-addition-restart.txt).
  The unchanged product/layout is unusable.
- Violates design domain §2.1: consent covers the mapping rule, not a frozen
  folder list; later folders must be evaluated by that rule. A-46/R-02 and
  addition propagation are affected. Author's caveat was independently
  reproduced, not accepted as proof.

### M2 — MAJOR: “blocked/not synced” code is published outside displayed consent scope

- Location: `crates/wow-sync-core/src/plan.rs:105–129`;
  `inventory.rs:155–183`; app `ui.rs:241–254,273–280`.
- Reproduce: unfamiliar client with eligible `Good.toc` and only
  `Blocked_Unknown.toc` in another folder; agree to the displayed Good-only
  list and enroll.
- Observed: blocked code's `main.lua` SHA-256 object exists in the provider,
  and its published state says `status=ok`, `toc=unvalidated`, `files=2`.
  The GUI changes from “Blocked folders (not synced)” to
  “blocked: up to date”, concealing the exclusion.
- Cause: publication filters only `FolderStatus::Ok`, not transfer eligibility.
  No blocked-folder **installation** was observed; the demonstrated boundary
  failure is unauthorized-by-displayed-list publication (R-03/A-46).

### M3 — MAJOR: sync policy remains outside the shared core (E-08/A-43)

- Location: `crates/wow-sync-runtime/src/service.rs:530–536` and
  `1048–1090,1147–1155`.
- Reproduce/read: `enroll` itself decides whether the group's product permits
  joining; `publish_step`/`same_folders` decide whether publication is needed
  by comparing folder counts, keys, status, trees and completeness.
- These are domain/plan decisions, not OS observation or rendering. They
  contradict design crates §6 and the platform-free shared-core requirement.
  App source, runtime activity/effects/service, and their core call sites were
  read directly. Ordering/grouping/target calculation and live execution do
  otherwise call core; no second version-ordering engine was found in app.

### M4 — MAJOR: enrollment omits approved consent and guided-scope evidence

- Location: `crates/wow-sync-app/src/ui.rs:225–234,273–280`;
  runtime `service.rs:446–468`.
- Reproduce: open either fresh recognized A or unfamiliar D, before consent.
- Observed: only code/settings exclusion and relative `_folder_/Interface/AddOns`
  are shown. No chosen install-root/full AddOns path, `validated-plain/1`
  mapping, explicit automatic newer-addon installation/replacement warning,
  missing-addon-not-removal rule, or recovery location/preservation disclosure.
- Design domain §1 step 5 and §2.1 explicitly require these disclosures.
  A friendly code/product/folder label is not sufficient guided safe-scope
  evidence (R-03/A-46). No unauthorized real-data changes were attempted.

### M5 — MAJOR: ambiguity evidence omits candidate sources

- Location: `crates/wow-sync-core/src/target.rs:291–310`,
  `explain.rs` rendering of `AmbiguousVersions`; runtime
  `service.rs:831–837`; app displays only the rendered package line.
- Reproduce: A/B each publish version 4.0 with different bodies.
- Observed: “alpha (4.0 vs 4.0: same label, different content)” without which
  computer holds either candidate. The separate member list cannot establish
  that association. `NeedsDecision.candidates` exists but is discarded from
  the visible status. G5 explicitly requires labels **and sources** (A-08/A-25).

### Minor observable deviations

| ID | Location / reproduction | Observed deviation |
|---|---|---|
| m1 | runtime `service.rs:446–456`; fresh A | G1 recognized identity is not the specified canonical product label |
| m2 | runtime `service.rs:1166–1180`; FakeClient wait | G4 required combined running-game wording is split/reworded; safety still passes |
| m3 | core `status.rs` AppliedHere / `explain.rs`; B after apply | G3 post-apply status drops source and from/to versions |
| m4 | runtime `service.rs:858–882`, app `ui.rs:318–339`; obstruct provider | identity renders “ on ()”; raw filesystem error appears twice, once with `NotPlainFile` |
| m5 | core `status.rs:160–166`; injected non-game failure | G8 says “WoW started ... after WoW exits” while Game activity is NotRunning |

## Memory (PLAN §6 item 6; A-29)

Measured independently with [memory.ps1](R2-check/memory.ps1). Release app,
enrolled A with one addon, 5-second normal sync interval. Each run first opens
the real window, then hides it after 3 seconds; sampling begins after
8 seconds and verifies no visible app-owned top-level window. This is not
the lower-cost never-created-window case. A preliminary `--start-hidden`
probe was stopped by exact child/parent PIDs and excluded.

For each fresh launch, took 61 samples, summing Windows
`WorkingSetPrivate` over the root PID **and all descendants**, recording
private commit/total working set separately. No trimming, working-set API,
or limit change was used. Actual elapsed sample times are retained; CIM
queries can make the nominal 1 Hz cadence uneven.

Initial hidden-window results:

| Run | App-attributed processes | Maximum / average private MiB | Result |
|---|---|---|---|
| 1 | 1 | 16.602 / 15.872 | PASS for measured process |
| 2 | 1 | 16.645 / 16.388 | PASS for measured process |
| 3, original | 9 | 763.305 / 763.121 | Raw threshold FAIL; process ownership unverified, see V1 |
| 3, corrected repeat | 1 | 16.207 / 15.948 | PASS for creation-time-validated process tree; does not resolve historical anomaly |

Original raw data and summary are retained under
[memory/](R2-check/memory/), including `run3-anomalous.csv` and
`summary-original.json`. The anomalous run took approximately five seconds
per sample instead of one; it is not misrepresented as a 60-second run.

### V1 — BLOCKER: memory process attribution and cleanup incident

The initial measurement harness followed the existing author's
ParentProcessId-only descendant traversal, then stopped the inferred
descendants by explicit PID. On its third launch it attributed nine processes
and 763 MiB to an application whose reviewed source starts no subprocesses.
Windows ParentProcessId records can outlive a parent and point to a recycled
PID. Creation-time causality was **not** checked in that initial harness.

The cleanup ran before the anomaly was recognized. Consequently eight
possibly unrelated processes may have been targeted for termination; their
identities and actual termination outcomes were not retained, and collateral
termination cannot be ruled out. No arbitrary user files were targeted, and
no attempt was made to restart unknown processes. This is a verification
safety incident, not proof the app consumed 763 MiB, and not a justified
reason to silently discard a high measurement.

The retained harness now requires each child creation time to be at least
its currently live parent's creation time, records the actual attributed
PIDs, and terminates **only the app PID it directly launched**, not inferred
descendants. The corrected third launch had 61 samples over approximately
61 seconds and one attributed app process; its maximum was 16.207 MiB.
The exact original harness is retained as disabled text
`memory/memory-initial.ps1.txt`, alongside original/repeated raw measurements.
The
overall memory gate remains **BLOCKED / attribution unresolved**, not PASS:
the anomalous historical ownership cannot now be proven either way.

## Final validation and handoff

- Owned-worktree offline verification: **PASS**, all six checks.
  [Command output](R2-check/worktree-offline.log). Its report names the
  pre-evidence source SHA with `git_dirty=true` because evidence was staged;
  it is not misrepresented as a clean post-commit report.
- The clean clone was back on `main`, at the full SHA retained in
  [environment.json](R2-check/environment.json), with empty `git status
  --porcelain`, before deletion. Workspace tests after every control was
  reverted passed again (237 passed, 3 ignored).
- Final process check found **zero** app/FakeClient executables under the
  owned scratch root; clone, build products within it, and synthetic lab
  were removed. All retained artifacts are small evidence only.
- Handoff: resolve M1–M5, the minor GUI deviations, and V1's process-ownership
  incident before claiming run-002 acceptance. Native/provider remain BLOCKED;
  do not infer cross-machine delivery or other-platform support.

## Method limitations and cleanup

- Initial harness runs encountered transient empty UIA enumerations and a
  stale synthetic lab on restart. Bounded readiness retries/fresh fixtures
  were used; these are not claimed as product test failures.
- Capturing `devkit fake-client` stdout inherited a live child handle and
  stalled the harness, as the author's instructions warned. Its exact PID
  was stopped; the completed GUI run used `Start-Process` on the copied
  stand-in directly. Directly launched app/FakeClient processes were tracked
  and stopped by `Stop-Process -Id`; V1 documents the additional unsafe
  inferred-descendant cleanup. Naturally completed build/test helpers were
  allowed to exit. No name-based termination, git maintenance/history rewrite,
  push, merge, or nested agent was used.
- Notes replace the owned lab's path with `<synthetic-lab>`. The sole screenshot
  was inspected and cropped inside the app frame; no real user/machine names
  or profile paths are visible. Synthetic IDs/display names are not private
  identities.
- Evidence-only worker: no fixes were made to product/tests/docs owned by
  other packets. Credit/budget accounting remains the orchestrator's shared
  ledger responsibility; this worker did not reset counters or spawn work.
