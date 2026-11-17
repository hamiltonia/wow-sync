# R3-check4 — independent verification of R3-fix2

## Scope and environment

- Checkout `D:\GitHub\wow-sync.worktrees\R3-check4`, branch
  `run003/R3-check4`; product source
  `976ce45d6710efaf13ea131ca2b1e61c5fdf3637`.
- Independent verification worker, requested role/model `gpt-6.1-sol`.
  No agents launched. The parent owns runtime-selection and cumulative-budget
  records; the packet's model label is not independent runtime proof.
- Windows 11 Pro x64 `10.0.26300`, i7-13700HX, 64 GiB class; Rust
  `1.100.0`, cargo-deny `0.20.2`, native MSVC, 2026-11-17.
  [Environment and release hashes](R3-check4/environment.json).
- Built the actual app and devkit with
  `cargo build --locked --release -p wow-sync-app`, exit 0.
  [Build output](R3-check4/build.txt). No prototype or fabricated app state.
- Synthetic data exclusively under `D:\WoWSyncLab\R3-check4`; ordinary
  local-folder provider, **not Dropbox**. Two synthetic peers, LabAlpha
  and LabBeta; no real WoW, launcher, Dropbox, demo apps or private data used.
  Synthetic Lua text was written as fixture data, never executed.
- Authority: [packet](../packets/R3-check4.md),
  [common packet](../packets/COMMON.md), [fix evidence](R3-fix2.md),
  [previous verification](R3-check3.md), [SPEC](../../../../SPEC.md)
  R-11/R-12 and [ACCEPTANCE](../../../../ACCEPTANCE.md)
  A-24/A-25/A-38/A-39/A-43.

**Overall: FAIL.** The exact original crash-after-3 screen sequence is fixed,
but a recovery retry can still convert an earlier crash into a false claim
that WoW caused the interruption (RC4-1). This is development verification,
not release or cross-platform acceptance.

## Verdict per packet check

| Check | Verdict | Independently observed result |
|---|---|---|
| 1. RC3-1 real app, recorded-cause path, own negative control | **FAIL** | Original staging-crash sequence: PASS for neutral wording, close-WoW action only while running, and next-sync recovery action when stopped. Recorded-cause recovery path: FAIL, RC4-1 below. Own deliberately broken cause/action mapping fails the two named tests with exit 101. |
| 2. RC3-2 unstable candidate version | **PASS** | Actual transient decision screen says `(1.0.0 and not available)` and `Version not available`, not a blank. Raw-label negative control fails the relevant app test. |
| 3. RC3-3 paused hint | **PASS** | Actual dashboard says `1 add-on waiting will be tried again when you resume`, alongside `1 waiting`, with no zero-update promise. Ignoring waiting causes the relevant app test to fail. |
| 4. Journal compatibility | **PASS** | Fresh release core successfully decodes the actual journal retained in pre-fix2 R3-check3 evidence; missing field defaults to false, is recognized as interrupted rather than needs-help, and roundtrips. Explicit true also roundtrips. |
| 5. Offline and native tiers | **PASS** | Both exit 0 without `--allow-blocked`; offline 48 listed cases / 336 test results, native 4 cases / 336 results. Production code was exactly restored first. |

## Real-app observations

The [retained harness](R3-check4/gui-check.ps1), adapted from R3-check3, drove
only the launched app windows through UIA. Screenshots are client-area captures
of those windows. The verifier **visually inspected** their actual rendered
text: GPUI does not expose these body paragraphs through its current UIA tree.
Consequently [automated results](R3-check4/gui-results.json) honestly mark
body-text assertions NOT RUN; that is not the final visual verdict above.
The accompanying `*.uia.txt` files document the actual controls and status.
[Visual-review record](R3-check4/visual-review.json).

1. Enrolled A with LabAlpha/LabBeta 1.0.0, joined B, changed B's Alpha bytes
   while keeping its 1.0.0 label, and synced B then A. The legitimate unstable
   candidate renders **not available** in both places:
   [decision screenshot](R3-check4/unstable-decision.png).
2. With no stand-in running, published stable Beta 3.0.0 from B and started A
   with `--crash-after 3`. Its actual
   [staging journal](R3-check4/crash-journal.json) has no `halted_by_game`
   property. The app automatically paused after the injected crash.
3. Stopped the captured A PID, **then** launched the stand-in and restarted A.
   [Running-game screenshot](R3-check4/interrupted-running.png):
   `Updating labbeta was interrupted; it will be finished or restored on the next sync.`
   [Scrolled next step](R3-check4/interrupted-running-lower.png):
   `Close WoW to let the update finish.`
4. Stopped only the captured app and stand-in PIDs, restarted A with the same
   paused journal and no game. [Stopped-game screenshot](R3-check4/interrupted-stopped.png):
   neutral interruption wording plus
   `Nothing to do. It will be finished or restored on the next sync.`
   No close-WoW next step.
5. [Paused dashboard](R3-check4/paused-waiting.png) correctly counts the
   interrupted update as waiting, without promising zero future updates.

## RC4-1 — Medium — recovery overwrites an unknown crash cause with game blame

R-11/R-12, A-25. This is specifically the new R3-fix2 recorded-cause path,
not a request to expand product scope.

Reproduced in the actual release app, in the same harness:

1. Resumed A to finish the first case, published Beta 5.0.0 from B, and started
   A with `--crash-after 4`, **without any stand-in running**. The resulting
   [journal before recovery](R3-check4/recovery-before-journal.json) is
   `committing`, without a recorded game cause.
2. Stopped that captured app PID. Started the stand-in only afterwards,
   then restarted A. [Before Resume](R3-check4/recovery-before-resume.png)
   correctly shows neutral crash wording.
3. Chose **Resume syncing** while the stand-in was already running.
   No new game launch occurred during this retry.
4. The same journal now contains
   [`"halted_by_game": true`](R3-check4/recovery-after-journal.json).
   [Actual screen after Resume](R3-check4/recovery-after-resume.png):
   **“WoW started while labbeta was being updated; it will be restored to
   its previous version after WoW exits.”**

The program correctly waits instead of applying while the game runs, but it
incorrectly rewrites the historical explanation of the crash. The persisted
flag now represents “game blocks a recovery attempt,” not evidence that the
game started during the original update. GUI harness exits **1** because the
independent journal-cause oracle catches this.

Source reviewed:

- `core/src/exec.rs::recover_one`, `JournalPhase::Committing` branch:
  a currently-running game calls `record_game_halt` on the existing journal.
  The committed/rollback recovery paths also use this recorder.
- `record_game_halt` sets the new bool on any `InterruptedByGame` gate result;
  that gate reports current activity, not the original interruption cause.
- `status::game_halted` and `derive_status` then treat the persisted bool as
  historical evidence; `explain::render` emits the causal sentence.
- The new executor test verifies a genuine launch after a rename, but does
  not cover a game already running when recovery of a crash is retried.

Keep the original cause separate from a current recovery blocker, or use
wording that only states the observed blocker. No product fix made here.

## Source review and A-38 controls

Reviewed `records.rs`'s `serde(default)`/false omission, executor journal
creation/transition/recording and recovery paths, status selection, typed
next actions, runtime snapshot/detail propagation and app rendering.
The app's `world.rs` preserves the core sentence/action; `pages.rs` uses
`version_label` for candidate cards. `paused_hint` separates ready and waiting.

The verifier temporarily broke four production behaviors, without changing
tests or assertions:

- `game_halted` ignores the recorded flag.
- Interrupted next actions always request closing the game.
- `version_label` returns the raw, possibly blank value.
- `paused_hint` ignores waiting add-ons.

Exact [temporary mutation](R3-check4/negative-mutation.diff). Core command:
`cargo test --locked -p wow-sync-core --lib`, **exit 101**,
179 passed / 2 failed:

- `status::tests::only_journals_that_record_the_game_halt_blame_the_game`
- `view::tests::an_interruption_asks_to_close_wow_only_while_the_game_runs`

[Core control output](R3-check4/negative-core.txt).
App command:
`cargo test --locked -p wow-sync-app --bin wow-sync model::tests::`,
**exit 101**, 7 passed / 3 failed:

- `model::tests::a_missing_candidate_version_reads_not_available_never_blank`
- `model::tests::interrupted_text_follows_core_and_differs_by_cause`
- `model::tests::the_paused_hint_never_promises_zero_while_an_update_waits`

[App control output](R3-check4/negative-app.txt).
Both intentionally failing commands returned 101; the control driver returned
0 only after checking both exit codes. The extra app interruption failure
confirms that the broken core next action reaches the app.

All three modified production files were restored byte-for-byte:
[before/after SHA-256](R3-check4/mutation-hashes.json), all `ExactMatch=true`;
`git diff --exit-code -- crates` returned 0 before final verification.
No temporary production mutation or changed assertion is committed.

Restored-positive commands passed: [181/181 core tests](R3-check4/positive-core.txt)
and [10/10 app model tests](R3-check4/positive-app.txt), both exit 0.
This includes the recorded genuine game-start executor test, cause selection,
old-journal default, context-sensitive next action, missing-version and paused
waiting-hint tests. The passing suite does **not** close the independently
observed recovery-cause gap.

## Final verification tiers

[Verification driver](R3-check4/verify-final.ps1) and
[captured exit codes](R3-check4/final-exits.json):

- `cargo xtask verify --tier offline`: **PASS**, exit 0;
  [output](R3-check4/offline.txt), [report](R3-check4/offline-report.json).
  Formatting, clippy with warnings denied, all workspace tests, all 48 required
  offline cases, cargo-deny and 141 Markdown files' links passed.
- `cargo xtask verify --tier native`: **PASS**, exit 0;
  [output](R3-check4/native.txt), [report](R3-check4/native-report.json).
  All 4 required native cases verified against 336 executed test results.

No ignored/missing mandatory case or `--allow-blocked` success is used.
The reports identify the product source SHA above and `git_dirty=true` because
this evidence was staged/in progress; production diffs were empty. They are
not represented as clean-commit reports for the later evidence-only commit.

An additional, non-packet hygiene check, `git diff --cached --check`, returned
2 for whitespace in retained patch context, a literal UIA name ending in a
space, and cargo logs' final blank lines. Those raw evidence values are
preserved rather than silently trimmed. This is not a passing hygiene check;
the production tree has no whitespace or other changes.

## Legacy journal probe

[Probe source](R3-check4/legacy-probe.rs) and
[driver](R3-check4/legacy-check.ps1) extract, without changing its fields,
the real `JOURNAL=` JSON from R3-check3's retained `gui-interrupted.txt`.
The [extracted journal](R3-check4/legacy-journal.json) predates R3-fix2.
The driver links directly to the **unmutated release core** built above.
[Actual execution](R3-check4/legacy.txt): **exit 0**; all assertions passed.

Two distinct harness preflight problems are disclosed, not counted as passes:
the first lookup assumed the older `target\release\deps` layout; this toolchain
stores rlibs under `target\release\build\...\out`. The next invocation located
the rlib but included shell quotes in a rustc response-file argument; rustc
rejected the quoted crate name before compiling. Correcting that quoting
produced the successful retained execution. Neither failed preflight modified
product code or ran the decoder.

## Exact execution and limits

All commands ran from the checkout/branch at the top. Each cargo shell first
used `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
Test/GUI scratch uses both `TEMP` and `TMP` set to
`D:\WoWSyncLab\R3-check4\scratch`.

```powershell
cargo build --locked --release -p wow-sync-app
& .\docs\runs\run-003\evidence\R3-check4\gui-check.ps1
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4\legacy-check.ps1
cargo test --locked -p wow-sync-core --lib
cargo test --locked -p wow-sync-app --bin wow-sync model::tests::
& pwsh -NoProfile -File .\docs\runs\run-003\evidence\R3-check4\verify-final.ps1
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

The final driver reruns the two test commands after restoration, then executes
the two tiers. Offline verification is repeated after finalizing/staging this
evidence; the retained offline log/report reflect that final run.

The GUI's `Write-Host` information stream was not captured by the invocation's
`2>&1 | Tee-Object`; the original screenshots, UIA snapshots, journal copies,
oracle results and cleanup result were written directly by the harness and
are retained. Do not infer a full GUI console log exists.
The [selected console excerpt](R3-check4/gui-console-excerpt.txt) is explicitly
transcribed from this execution, including the launch/stop PIDs and exit 1.

**Process safety:** only launch-returned PIDs, revalidated by creation time,
were stopped, individually with `Stop-Process -Id`. No name-based stop,
process-tree stop or descendant traversal was used. All launched app/stand-in
processes were stopped; [remaining processes](R3-check4/remaining-processes.json)
is `[]`. No process is knowingly left running.

**NOT RUN:** real mid-commit game-start timing race (covered by the existing
fake-effects executor test, not claimed as real-app evidence), macOS,
real-provider/Dropbox checks, release acceptance or broader UI review.
No new scope, paid service, credentials, publishing or nested agents.
