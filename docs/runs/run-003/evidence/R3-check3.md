# R3-check3 — independent fix verification

## Scope and environment

- Checkout: `D:\GitHub\wow-sync.worktrees\R3-check3`,
  branch `run003/R3-check3`.
- Product source: `413596c93fb77f1c5bac8bc94a8012e70d849ff3`.
- Independent Verification worker; assigned runtime `gpt-6.1-sol`. The worker
  did not launch agents. The packet label alone is not independent runtime
  selection evidence; the orchestrator retains responsibility for its registry.
- 2026-11-17; Windows 11 Pro x64 `10.0.26300`, i7-13700HX, 64 GiB RAM,
  Rust `1.100.0`, cargo-deny `0.20.2`, native MSVC. This is a development/
  Insider environment, not stable-Windows support certification.
- Built `cargo build --release -p wow-sync-app`, exit 0.
  App SHA-256:
  `56DEE3FAC57A162C1D5F09370E7335877D0D606A2B6C53E71437BADB22763311`.
  [Environment and both binary hashes](R3-check3/environment.json).
  [GPU/driver context](R3-check3/gpu-environment.json).
- Synthetic data only under `D:\WoWSyncLab\R3-check3`; ordinary local-folder
  provider, not Dropbox. No real WoW, Battle.net, Dropbox, demo app, imported
  Lua, account or character data was used or executed.
- Authority: [packet](../packets/R3-check3.md),
  [common packet](../packets/COMMON.md), [D-21](../../../OWNER-DECISIONS.md),
  [screens](../../../design/ui/screens.md),
  [SPEC](../../../../SPEC.md) R-11/R-12/R-13/R-14 and
  [ACCEPTANCE](../../../../ACCEPTANCE.md) A-21/A-24/A-25/A-26/A-29/A-38/A-39/A-42/A-43.

**Overall: FAIL.** Most requested fixes work, but truthful interruption
explanations and next steps remain incomplete. Physical tray interaction is
BLOCKED, not a pass. These are development results, not release acceptance.

## Packet checks

| Check | Verdict | Independently observed result |
|---|---|---|
| 1. Finding closure and own RV-1/RV-2 negative controls | **FAIL** | Both controls failed the named tests with cargo exit 101; exact restoration passed 8/8 app model tests. Most findings close, but the original interruption-truthfulness concern remains open as RC3-1. Detailed closure table below. |
| 2. Four real-app states, screenshot + UIA, no invented text | **FAIL** | Dashboard, decision, interrupted add-on with the stand-in running, and damaged pause were captured from the release app. RC3-1 to RC3-3 prevent a blanket truthful-text claim. Decision text is build-free. The dashboard still displays the real synthetic game build; see the literal packet limitation below. |
| 3. RC2-2 UIA TogglePattern changes and persists close option | **PASS** | Off → On through TogglePattern; disk `closing_quits=1`; stopped/restarted the app and UIA still reported On. Toggled back through UIA before hide/show tests. No keyboard fallback was used. |
| 4. RC2-3 tray left click and second instance always show | **BLOCKED** | Physical click blocked because the own icon's rectangle mapped to the overflow container, not a uniquely identifiable icon. No mouse click sent. Second instance: PASS for hidden → shown twice and shown → shown twice; each secondary exited 0. Source now unconditionally calls `show()`. |
| 5. RC2-4 piped devkit capture returns within 5 s | **PASS** | `$out = & $dev fake-client <variant> 2>&1` returned PID 31284 in **108 ms**. Stopped only that captured stand-in PID. |
| 6. Offline and native tiers | **PASS** | Both exit 0 without `--allow-blocked`; offline 48 listed cases / 330 test results, native 4 cases / 330 results. |
| 7. A-29 memory | **PASS (synthetic harness)** | Scale 1.0, three runs of each of a/b/c; 1,980 samples, max one app-owned process, 60/60 reopen cycles, zero harness findings. Whole-run aggregate private-resident peak **37.34 MiB**, below 100 MiB. |

### Finding closure

`CLOSED` means the cited defect was checked, not that every surrounding feature
or every platform is accepted. Documentation-only fixes were read directly.
Tests named below were actually executed, not inferred from source.

| ID | Status | Evidence / limit |
|---|---|---|
| RV-1 | **OPEN** | `world.rs` now carries runtime `sentence`, `next`, `awaiting`; `addon_story` consumes them, drops invented recovery/group-prefix sentences and uses status-specific local icons. Tests `interrupted_text_follows_core_and_differs_by_cause` and `next_step_text_is_core_s_for_other_actions` pass, including the verifier's broken-mapping control. However, the core currently infers interruption cause from current game activity and gives a non-running game a CloseGame next step: RC3-1. Architectural mapping fixed; user-visible truthfulness not fully closed. |
| RV-2 | CLOSED | App and proto have exhaustive, distinct per-reason D-21 copy; no generic different-build fallback. Both `every_decision_reason_*build_free_wording` tests pass. Own app mutation fails on `DifferentTocMapping`. The proto test includes S7; reconfirmation copy remains a separate identity flow. |
| RV-3 | CLOSED | Read `core-api.md`, `screens.md`, `removal.md`, `prototype.md` and the two cited wireframes. The obsolete decision/build-gating copy is removed or explicitly historical. No claim that the removal design itself is implemented or D-21-approved. |
| RV-4 | CLOSED | No hard-coded `PerPackage` continuation in the app; decision/needs-help `affects` strings empty. Decision screenshots show no invented “only this add-on” / “other add-ons keep syncing” scope. |
| RV-5 | CLOSED | `headline` no longer promotes reconfirmation. Core Error/NeedsHelp is not hidden by pause; `headline_follows_the_runtime_state` passes. `questions.md` UQ-1/UQ-2 record the remaining design questions. Damaged pause displays its explicit problem, not an epoch date. |
| RV-6 | CLOSED | `this_confirmed` counts only InSync/AppliedHere/ConfirmedBy. `this_computer_confirms_only_applied_or_in_sync_packages` passes (one executed test). |
| RV-7 | CLOSED | Discovery uses adapter `probe` calls and bounded 64 KiB reads; pause removal uses `LocalState::remove`; malformed pause remains held. `an_oversized_dropbox_info_file_is_not_read` and `a_damaged_pause_file_holds_sync_and_is_an_error_not_a_time` each pass. Real damaged-file GUI shows an error, disables Sync now and clears the file through Resume. |
| RV-8 | CLOSED | `ARCHITECTURE.md`, `design/crates.md`, `REPO-MAP.md` describe core view helpers, runtime detail/control/discovery and the proto. Read against current modules, not just searched for headings. |
| RV-9 | CLOSED (documented deferral) | `prototype.md`, architecture §2.2a and crate map explicitly freeze the duplicate prototype, permit only D-21 copy fixes and plan retirement/sharing in a later run. Duplication is not removed; closure is the review's permitted documented alternative. |
| RV-12 | CLOSED | GUI/memory labs use the neutral root. UIA/screenshots contain only neutral synthetic paths. `tools/mem/run.ps1` accepts explicit `-LabRoot`; this verification uses it. |
| RC2-1 | CLOSED | Scheduler-backed `next_check_at`; no product literal “next in 4 min” or “for 2 add-ons”. Test `last_check_text_uses_the_scheduler_or_says_unavailable` passes. Dashboard actually shows next check about 12:48 with a 60,000 ms scheduler. |
| RC2-2 | CLOSED | Actual TogglePattern Off → On persisted over restart. `app.rs` handles accessible Click directly. |
| RC2-3 | **NOT VERIFIED (physical tray)** | Handler and second-instance paths now call unconditional `show()`. Actual single-instance behavior passes; actual tray click is BLOCKED. |
| RC2-4 | CLOSED | Actual native piped capture completed in 108 ms and delivered its stand-in PID. |

RV-10/RV-11 belong to the orchestrator, not this packet.

## New findings

### RC3-1 — Medium — current game activity is mistaken for interruption cause

R-12/A-25; original RV-1's truthfulness concern remains partly open.

Observed sequence, retained in [GUI interruption log](R3-check3/gui-interrupted.txt):

1. With **no stand-in running**, B published stable LabBeta 3.0.0.
2. Started A with `--crash-after 3`; a genuine production-format
   `staging` journal was created for `labbeta`. The backend automatically
   paused after its injected crash. Stopped that captured A PID.
3. Only **afterwards**, started the stand-in, captured PID 31284, then restarted A.
4. [Actual add-on screenshot](R3-check3/interrupted-addon.png) says:
   “WoW started while labbeta was being updated; it will be restored to its
   previous version after WoW exits.” The stand-in did not cause this interruption.
5. Stopped only the captured app/stand-in PIDs. With the same journal still
   present and **no game running**, [the next screenshot](R3-check3/interrupted-no-game.png)
   correctly says “Updating labbeta was interrupted; it will be finished or
   restored on the next sync”, but its next step still says
   **“Close WoW to let the update finish.”**

Source: `core/src/status.rs::derive_status` chooses `InterruptedByGame` solely
from current `input.game_running` when any interrupted journal is present.
`core/src/view.rs::next_action` maps *all* Interrupted statuses to CloseGame and
ignores `_why`. The app now faithfully renders these core values; its mapping
tests therefore pass while the real explanation/next step is false.

Use recorded cause evidence or neutral historical wording, and an appropriate
next action when the game is already stopped. This verifier made no fix.
This injected-crash control is not proof of an actual mid-commit game-start
race; that precise causal scenario was **NOT RUN**.

### RC3-2 — Low — an unavailable candidate label is rendered as empty text

R-12/A-25. During B's legitimate unstable scan, the real app showed:

> Their version labels can't be compared safely (1.0.0 and ).

The second card is simply **“Version”**, with no value or uncertainty marker.
See [decision screenshot](R3-check3/pending-decision.png) and
[attention UIA](R3-check3/pending-attention.uia.txt).
After stable publication, [the stable screenshot](R3-check3/pending-decision-stable.png)
shows both actual 1.0.0 labels; no data was invented to make that state.

Source: runtime `CandidateRow.version` is optional; app `world.rs` converts
None to `""`, `model.rs::decision_reason` prints it, and `pages.rs::decision_view`
prints `Version {value}`. Preserve/display “not available” instead of a blank.

### RC3-3 — Low — paused update count omits an interrupted update

R-12/A-24. [Damaged-pause screenshot](R3-check3/damaged-pause.png) simultaneously
shows **“1 waiting”** and **“0 updates will be applied when you resume”**.
LabBeta was locally 1.0.0, with the retained journal and group target 3.0.0.
After Resume, the file was removed and Beta actually became 3.0.0
([metadata-only oracle](R3-check3/provider-oracle.csv), A sequence 2).

Source: `model.rs::paused_hint` counts only `counts(w).ready`, not the waiting
interrupted update. When exact future recovery/apply counts are not known,
omit the promise rather than report zero.

## Real-app procedure and evidence

All commands below ran in the checkout/branch at the top. `gui-check.ps1`
drives the actual GPUI app through UIA and uses devkit-generated installs,
not the prototype or fabricated snapshots. Source of truth was synthetic disk
metadata, provider state records and the durable apply journal.

```powershell
& .\docs\runs\run-003\evidence\R3-check3\gui-check.ps1
& .\docs\runs\run-003\evidence\R3-check3\gui-check.ps1 -Resume
& .\docs\runs\run-003\evidence\R3-check3\gui-check.ps1 -InterruptOnly
& .\docs\runs\run-003\evidence\R3-check3\tray-check.ps1
& .\docs\runs\run-003\evidence\R3-check3\gui-check.ps1 -FinalOnly
```

The retained script is the final harness. Development attempts are not
silently treated as passes:

- Initial [GUI log](R3-check3/gui.txt) verified dashboard/toggle/show behavior,
  then failed to find a separate “Choose a copy” UIA element. This view exposes
  its combined ListItem action instead; the harness was corrected.
- The first `-Resume` invocation had a harness parser typo (“The Try statement
  is missing its Catch or Finally block”, line 80) and launched nothing. That
  failed before its Tee-Object pipeline created a log; it was corrected before
  the actual resumed run.
- [Resumed run](R3-check3/gui-resume2.txt) captured the pending decision but
  the immediate post-write B scan was unstable, so no apply journal existed.
  The bounded correction waited for stable publication and synced twice.
- [Interruption run](R3-check3/gui-interrupted.txt) created the journal and
  captured the stand-in state; its final damaged-pause capture was attempted
  before UIA readiness and failed. All captured roots were stopped in `finally`.
- [Final run](R3-check3/gui-final.txt) waited for an actual Resume control,
  captured the damaged pause and no-game interruption, cleared the damaged
  file through the GUI, observed recovery, and captured the stable decision.
- [Tray attempt](R3-check3/tray.txt) found an own-PID icon rectangle, but UIA
  returned “System tray overflow window.” No click was sent.

| State | Actual image | Actual UIA |
|---|---|---|
| Dashboard | [PNG](R3-check3/dashboard.png) | [text](R3-check3/dashboard.uia.txt) |
| Pending decision, stable metadata | [PNG](R3-check3/pending-decision-stable.png) | [text](R3-check3/pending-decision-stable.uia.txt) |
| Interrupted add-on while stand-in runs | [PNG](R3-check3/interrupted-addon.png) | [text](R3-check3/interrupted-addon.uia.txt) |
| Damaged pause file | [PNG](R3-check3/damaged-pause.png) | [text](R3-check3/damaged-pause.uia.txt) |

UIA does **not** expose all ordinary body prose (especially detail/decision
paragraphs); screenshots were inspected as well. This is not a Narrator test
or a claim of complete accessible text.

Screenshots are real app client pixels. Six initial outer-window captures
were cropped to the client rectangle (11,45,1560,1080) to remove transparent
desktop shadows/titlebar; [crop script](R3-check3/crop-shadows.ps1). No pixels
were redrawn or substituted. Later captures use GetClientRect/ClientToScreen
directly. No full-desktop screenshot is retained.

**Literal packet limitation:** “none mentions builds or game versions” cannot
be marked PASS for the entire dashboard: its Game row includes the actual
devkit build `1.0.0.1`, which agrees with `.build.info` and with screens.md's
identity-label design. Decision/interruption reasons contain no game-build or
game-version compatibility gating. This is a visible oracle/design mismatch,
not permission to reinterpret the packet or remove an expected identity field.

## Tests and own negative controls

For **every cargo command**, prepended:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = 'D:\WoWSyncLab\R3-check3\test-scratch'
$env:TMP = $env:TEMP
```

No mutation or weakened assertion is committed.

1. Baseline `cargo test -p wow-sync-app model::tests`: 8/8 PASS.
2. RV-1 mutation: replaced production `world.rs` `why: p.sentence.clone()`
   with `why: "An update was interrupted.".into()`.
   `cargo test -p wow-sync-app --bin wow-sync model::tests::interrupted_text_follows_core_and_differs_by_cause -- --exact`:
   one test FAILED, **cargo exit 101**, expected the core-rendered Demo
   explanation rather than the replacement literal.
   [Raw output](R3-check3/negative-rv1.txt).
3. Restored RV-1. RV-2 mutation: changed production
   `decision_title(DifferentTocMapping)` to
   “the copies are for different game builds”.
   `cargo test -p wow-sync-app --bin wow-sync model::tests::every_decision_reason_has_its_own_build_free_wording -- --exact`:
   one test FAILED, **cargo exit 101**, “DifferentTocMapping mentions build”.
   [Raw output](R3-check3/negative-rv2.txt).
4. Restored RV-2. `git diff --exit-code -- crates`: exit 0.
   [Before](R3-check3/source-hashes-before.json) and
   [restored](R3-check3/source-hashes-restored.json) SHA-256 values match.
   `cargo test -p wow-sync-app --bin wow-sync model::tests`: 8/8 PASS,
   cargo exit 0 ([output](R3-check3/restored-app-tests.txt)).
5. Each of these ran **one named test**, PASS/exit 0
   ([full output](R3-check3/cited-tests.txt)):

```powershell
cargo test -p wow-sync-runtime this_computer_confirms_only_applied_or_in_sync_packages
cargo test -p wow-sync-runtime an_oversized_dropbox_info_file_is_not_read
cargo test -p wow-sync-runtime a_damaged_pause_file_holds_sync_and_is_an_error_not_a_time
cargo test -p wow-sync-proto every_decision_reason_has_distinct_build_free_wording
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

[Offline report](R3-check3/offline-report.json) and
[native report](R3-check3/native-report.json) both identify source `413596c`.
They truthfully report `git_dirty=true` because evidence was being collected,
not because a product mutation remained. No `--allow-blocked` was used.
The first staged-evidence offline rerun failed only the Markdown-link check:
this report linked a parser-error log that had never been created. The broken
link was removed; [that honest intermediate failure](R3-check3/offline-final.txt)
is retained rather than counted as a product failure or silently discarded.
The [final staged-evidence offline run](R3-check3/offline-evidence.txt) then
passed all six checks, exit 0
([report](R3-check3/offline-evidence-report.json)).
`git diff --cached --check` warns about two captured UIA names' trailing
spaces and three raw logs' final blank lines; these original capture bytes
were preserved. There are no product-code whitespace changes.

## A-29 memory

Command:

```powershell
& .\tools\mem\run.ps1 `
  -LabRoot D:\WoWSyncLab\R3-check3\mem `
  -OutDir docs\runs\run-003\evidence\R3-check3\mem
```

**PASS (synthetic workload)**, full `Scale=1.0`, default `Runs=3`, scenarios
a/b/c, harness exit 0. Ran 12:53–13:30 local time. Each fresh lab held three
small synthetic add-ons (two files each), enrolled through the real GUI.

| Scenario | Steady private-resident medians per run (MiB) | Whole-run private-resident peak (MiB) |
|---|---|---|
| a: start hidden, idle 120 s | 28.82 / 28.77 / 28.39 | 30.05 |
| b: shown 60 s, hidden 120 s | 35.28 / 35.74 / 34.97 | 37.34 |
| c: 20 reopen cycles, then idle 300 s | 35.19 / 35.23 / 35.31 | 36.47 |

1,980 retained samples; maximum one app-owned process; all three c runs
confirmed 20/20 reopen cycles; zero harness findings. Aggregate peak private
resident **37.34 MiB** (gate 100 MiB); private bytes 105.89 MiB and total
working set 71.86 MiB are separate counters, not substituted for that gate.
GPU peaks reported separately: dedicated 0 MiB, shared 43.11 MiB.
[Raw nine CSVs and summary](R3-check3/mem/),
[summary](R3-check3/mem/summary-abc-133026.txt),
[independently aggregated peaks](R3-check3/memory-peaks.json),
[execution log](R3-check3/memory.txt).

Launch → first state/provider change: n=9, min/median/max
0.19/0.25/0.27 s, zero timeouts. This is the harness's sync-start proxy, not
proof of all A-30 readiness/actual-useful-work scenarios. These small fixtures
and short runs do not establish representative real-addon or long-soak release
acceptance on every supported platform.

The existing harness validates PID + creation time and stops only the launched
root, never a process tree.

## Limits and process safety

Only exact PIDs captured at launch were stopped, through creation-time-checked
`Stop-Launched`. The stand-in was PID 31284, captured from devkit stdout before
any stop. No PID inferred from a name, process tree or historical report was
used as stop authority.

GUI roots were all stopped, including on harness errors, and every logged
`STOP APP` result is True. This is not permission to stop those historical PIDs
in a later session. The memory harness completed normally with zero findings.
A final read-only executable-path-scoped process query found **zero survivors**
under this worktree or lab ([empty result](R3-check3/cleanup.json)).
**No app, stand-in, helper or service was left running by this worker.**

macOS, ARM64, real Dropbox, actual WoW/updaters, Narrator and full supported
release acceptance were **NOT RUN**. The requested precise game-caused
mid-apply race remains NOT RUN; the observed interrupted journal + running
stand-in is intentionally distinguished from proof of historical cause.
