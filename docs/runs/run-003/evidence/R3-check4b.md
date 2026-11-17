# R3-check4b — independent RC4-1 re-verification

## Scope and environment

- Same-task follow-up assigned by the orchestrator; verification only, no
  product fixes or agents launched.
- Checkout `D:\GitHub\wow-sync.worktrees\R3-check4b`, branch
  `run003/R3-check4b`; product source
  `a513fc8606de878a1ed9ebd04de4f4b3f2e2d0af`.
- Windows 11 Pro x64 `10.0.26300`, i7-13700HX, 64 GiB class; Rust
  `1.100.0`, cargo-deny `0.20.2`, native MSVC, 2026-11-17.
  [Environment and binary SHA-256](R3-check4b/environment.json).
- Built the actual release app and devkit with
  `cargo build --locked --release -p wow-sync-app`, exit 0.
  [Build output](R3-check4b/build.txt).
- Synthetic data only under `D:\WoWSyncLab\R3-check4b`; ordinary local-folder
  provider, **not Dropbox**. Successful-run lab:
  `D:\WoWSyncLab\R3-check4b\gui-retry`.
  No real game, launcher, Dropbox, demo apps, imported Lua or private data used.
- Authority: [COMMON](../packets/COMMON.md), [approved plan](../PLAN.md),
  [R3-fix2b evidence](R3-fix2b.md), [original RC4-1 evidence](R3-check4.md),
  [SPEC](../../../../SPEC.md) R-11/R-12 and
  [ACCEPTANCE](../../../../ACCEPTANCE.md) A-25/A-38/A-39/A-43.
  Requested worker role/model remains `gpt-6.1-sol`; parent retains runtime
  selection and cumulative-budget records, not inferred from a packet label.

## Verdicts

| Follow-up check | Verdict | Result |
|---|---|---|
| 1. RC4-1 real-app recovery, neutral cause, close-WoW step, eventual completion | **PASS** | After Resume with the stand-in running: unchanged cause-less journal, neutral wording and close-WoW step. After its captured PID stops and GUI Sync now: journal removed, Beta 5.0.0 fully matches B and app reports updated here. |
| 2. Original RC3-1 real-app regression once | **PASS** | Neutral crash wording with game running/stopped; close-WoW action only while running; next-sync action when stopped. Resume completes Beta 3.0.0. |
| 3. Own negative control on the new regression test | **PASS** | Independent Committed-path mutation makes the exact new test fail with cargo exit 101; exact restoration passes all 182 core tests. |
| 4. Offline tier | **PASS** | Exit 0 without `--allow-blocked`; 48 required offline cases verified against 337 test results. |

**Overall: PASS for this narrow follow-up. RC4-1 is closed. No RC4b product
findings.** Harness invocation failures are disclosed below, not hidden or
reported as passing invocations. These results are not release acceptance.

## Real-app procedure and oracle

The [retained harness adaptation](R3-check4b/gui-check.ps1) reuses the R3-check4
UIA, screenshot and captured-PID helpers and the same crash-after-3/4 sequence.
It omits the unrelated Alpha decision/pause-hint checks and adds the requested
post-game-exit completion oracle. The original regression completed once,
then the RC4-1 recovery fixture was created in the same fresh lab. The
continuation mode resumes that retained committing fixture without rerunning
the original regression; see the harness-error disclosure below.

GPUI's current UIA does not expose the body paragraphs containing the cause
and next action. Those paragraphs require independent visual inspection of
the actual screenshots; UIA/control output alone is not claimed as that proof.
[Independent visual observations](R3-check4b/visual-review.json).

### Original RC3-1 regression

1. Enroll A with Beta 1.0.0, join B, then stop A.
2. Publish stable Beta 3.0.0 from B and stop B.
3. Launch A with `--crash-after 3`, with no stand-in running. Keep the actual
   staging journal and stop the captured app PID.
4. Only afterwards, launch the stand-in and restart A. Inspect the neutral
   interruption wording and close-WoW next step.
5. Stop the captured app and stand-in PIDs; restart A and inspect neutral
   wording with a next-sync recovery action, not a close-WoW action.
6. Resume through the GUI; require Beta 3.0.0 to match B's fixture bytes.

**Observed PASS:** [running-game wording](R3-check4b/original-running.png)
is `Updating labbeta was interrupted; it will be finished or restored on the next sync.`
The [next step while running](R3-check4b/original-running-lower.png)
is `Close WoW to let the update finish.`
[When stopped](R3-check4b/original-stopped.png), wording remains neutral and
the next step becomes
`Nothing to do. It will be finished or restored on the next sync.`
The [actual crash journal](R3-check4b/original-crash-journal.json) is staging,
with no game-cause field. The
[original-run metadata oracles](R3-check4b/attempt2-gui-results.json) also
confirm full Beta 3.0.0 fixture equality after Resume.

### RC4-1 recovery sequence

1. With A stopped, publish Beta 5.0.0 from B and stop B.
2. Launch A with `--crash-after 4`, with no stand-in running. Retain the
   actual `committing` journal, without a recorded game cause, and stop A.
3. Only afterwards, launch the stand-in, restart A, and choose **Resume syncing**
   while the stand-in is already running.
4. Require the journal to remain byte-identical with no `halted_by_game`
   property. Require the live Beta 3.0.0 fixture to remain unchanged.
5. Inspect neutral interruption wording and the close-WoW next step.
6. Stop only the captured stand-in PID, leave A running, and choose
   **Sync now** in the GUI. Require journal removal, Beta 5.0.0 metadata,
   and full synthetic add-on fixture equality with B. Inspect the finished
   app screen. This explicitly exercises a GUI retry, not automatic readiness
   latency or a claim that waiting for the scheduler was tested.

**Observed PASS:** the [before](R3-check4b/recovery-before-journal.json) and
[after-Resume](R3-check4b/recovery-after-journal.json) journals have no cause
property and identical SHA-256
`F7E24342C055C8C831E84B540FDB4AC1B689A3B18111DCD03B978341745589C6`.
The [actual post-Resume screen](R3-check4b/recovery-running-after-resume.png)
says `Updating labbeta was interrupted; it will be finished or restored on the next sync.`
Its next step is `Close WoW to let the update finish.`
No false “WoW started while…” history remains.

After stopping only captured stand-in PID 32064 and clicking Sync now in the
still-running app, the [completion oracle](R3-check4b/recovery-oracle.json)
shows zero remaining journals, version 5.0.0, and full fixture digest
`7813A26B1BBA1BF44E3D90A10C40B00FFCDE68CF3E146137C91C1837FC3AB715`
on both peers. Live-file digests before/while the game runs are also identical.
[Finished screen](R3-check4b/recovery-finished.png): `This computer is up to date`,
Beta `Updated here`, both displayed versions 5.0.0, source Lab-B, verification
on this computer, and `Nothing to do` next step.

[Continuation log](R3-check4b/gui.txt),
[metadata checks](R3-check4b/gui-results.json),
[actual exit 0](R3-check4b/gui-exit.json).

The independent fixture digest hashes sorted relative file names, byte
lengths and per-file SHA-256 values, then hashes that list. It covers both
fixture files, not only a version label or one payload. No add-on contents
are committed; the evidence retains only journal metadata and hashes.

## A-38: independent negative control and restoration

The new test is
`exec::tests::recovery_blocked_by_a_running_game_does_not_record_a_cause`.
It covers both Committing and Committed crash journals.

Unlike the author's Committing-path control, this verifier reintroduced
`record_game_halt(fx, j, &h)` inside `verify_and_finish`'s running-game gate,
which breaks **Committed** recovery. No test or assertion was changed.
[Exact zero-context mutation](R3-check4b/negative-mutation.diff).

```powershell
cargo test --locked -p wow-sync-core --lib exec::tests::recovery_blocked_by_a_running_game_does_not_record_a_cause -- --exact
```

**PASS negative-control obligation:** the named test actually **FAILED**,
with `Committed: cause stays unknown`, cargo **exit 101**,
one failed test / 181 filtered:
[output](R3-check4b/negative.txt), [captured exit](R3-check4b/negative-exit.json).

Restored the production file exactly:
[before/after SHA-256](R3-check4b/mutation-hashes.json), `ExactMatch=true`;
`git diff --exit-code -- crates` returned 0 before final verification.

`cargo test --locked -p wow-sync-core --lib` then **PASS**, exit 0,
182 passed / zero failed:
[restored suite](R3-check4b/positive-core.txt).
This includes the new test and genuine-launch recording tests.

Source reviewed: only the original apply path now records the game cause;
recover-one, verify, rollback and restore recovery gates no longer add it.
Current activity still determines whether the next action is close-WoW.
No product changes or temporary mutation are committed.

## Offline verification

[Verification driver](R3-check4b/verify-final.ps1),
[captured exit codes](R3-check4b/final-exits.json):

- `cargo test --locked -p wow-sync-core --lib`: **PASS**, exit 0, 182 tests.
- `cargo xtask verify --tier offline`: **PASS**, exit 0, no
  `--allow-blocked`; [output](R3-check4b/offline.txt),
  [report](R3-check4b/offline-report.json).
  Formatting, clippy with warnings denied, all workspace tests, all 48 required
  offline cases against 337 results, dependency policy and Markdown links pass.

Offline verification is repeated after staging the final evidence; retained
offline output/report reflect that final run. Reports name product source
`a513fc8` and `git_dirty=true` for evidence in progress, not a purported
clean-tree report for the later evidence-only commit.

## Commands, execution notes and limits

Every command ran in the checkout/branch above. Every cargo shell first used
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
Both `TEMP` and `TMP` were set to
`D:\WoWSyncLab\R3-check4b\scratch` for build, GUI and tests.

```powershell
cargo build --locked --release -p wow-sync-app
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4b\gui-check.ps1
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4b\gui-check.ps1 -Lab 'D:\WoWSyncLab\R3-check4b\gui-retry'
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4b\gui-check.ps1 -Lab 'D:\WoWSyncLab\R3-check4b\gui-retry' -ContinueRecovery
cargo test --locked -p wow-sync-core --lib exec::tests::recovery_blocked_by_a_running_game_does_not_record_a_cause -- --exact
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4b\verify-final.ps1
cargo xtask verify --tier offline
```

The first GUI attempt **FAILED in harness setup**, before either requested
scenario. The adaptation omitted TogglePattern handling for an enrollment
checkbox and got `Unsupported Pattern`. Restored the original helper's
TogglePattern branch and used a fresh lab for the retry:
[attempt log](R3-check4b/attempt1-gui.txt),
[exit 1](R3-check4b/attempt1-gui-exit.json),
[cleanup](R3-check4b/attempt1-cleanup.json).
Only captured app PID 6284 was launched/stopped in that failed attempt; no
stand-in was launched. It is not counted as a passing product check.

The second invocation completed the original RC3-1 regression and created
the committing recovery journal, then **FAILED in a different harness
targeting error**, before recovery Resume: redundant `Use 'Overview'`
on the already-current Overview page matched its non-invokable heading.
Removed that unnecessary action. The bounded continuation reused the
unchanged retained crash fixture rather than repeating the completed
regression or creating a third lab:
[second log](R3-check4b/attempt2-gui.txt),
[exit 1](R3-check4b/attempt2-gui-exit.json),
[all captured processes stopped](R3-check4b/attempt2-cleanup.json).
These harness invocation failures are disclosed separately from the actual
scenario results; neither is represented as an exit-0 run.

Continuation completed without another harness failure. All process stops
used only a launch-returned PID, revalidated against its captured creation
time, and `Stop-Process -Id`; no name match or tree/descendant stop was used.
[Final process ledger](R3-check4b/process-ledger.json),
[final cleanup](R3-check4b/cleanup.json), and both earlier cleanup records
show no surviving captured processes and no lost stand-in PID.
**Processes left running: none.**

GUI host output is captured through an external `pwsh -NoProfile` process,
merging all streams, avoiding the earlier R3-check4 missing-information-stream
log issue. Synthetic Lua text is fixture data only and was never executed.

**NOT RUN:** separate native-tier invocation (not requested here), macOS,
real Dropbox, real mid-commit game-start timing, broader UI/release acceptance,
automatic reaction latency. Offline execution still includes the existing
Windows tests; it is not represented as a separate native-tier run.
