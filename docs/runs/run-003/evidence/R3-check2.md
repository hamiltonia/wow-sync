# R3-check2 — independent phase-F verification

## Identity, scope and verdict

- Verifier: R3-check2, independent Verification role. The running-agent registry
  reported **gpt-6.1-sol** for this worker (not merely the packet label).
- Checkout: `D:\GitHub\wow-sync.worktrees\R3-check2`, branch `run003/R3-check2`.
- Product source: `ce15e241d4058fca347ce0acfe9d917e4e2d33e9`.
- Date/environment: 2026-11-17, Windows 11 Pro x64 `10.0.26300`, native MSVC;
  development/Insider environment, not stable-Windows support certification.
- Rust `1.100.0 (0ef1bc386 2026-11-09)`; cargo-deny `0.20.2`.
- Authority: [packet](../packets/R3-check2.md),
  [D-21](../../../OWNER-DECISIONS.md), [SPEC](../../../../SPEC.md)
  R-11/R-12/R-13/R-14 and [ACCEPTANCE](../../../../ACCEPTANCE.md)
  A-21/A-24/A-25/A-26/A-29/A-38/A-39.
- **Overall: FAIL**, not approval of phase F. The real runtime is connected,
  but Overview still invents its next-check time. Physical tray interaction
  and complete visual inspection remain unverified; see limitations.

Only evidence is committed. Two production mutations were temporary and
restored byte-for-byte before subsequent positive verification.

## Check-by-check results

| Packet check | Verdict | Independently observed result |
|---|---|---|
| 1: grep for scenarios/fake fixtures | PASS | No prototype scenario selector, scenario models or fixture import in the real renderer. Matches were the separate `devkit` helper and a stale prototype/fake comment in `model.rs:512`. |
| 1: real screen values against disk | **FAIL** | Counts, versions, enrolled names, peer confirmations, pause and game activity matched the isolated runtime/provider state. Overview's next-check time is hard-coded; RC2-1. Add-on detail opened, but its prose is not exposed by the UIA dump, so a complete visual comparison is NOT RUN. |
| 2: pause persists across restart, Q-28 | PASS | Paused using the GUI, observed `paused.txt`, stopped only the recorded app PID, restarted the same state directory. Header and banner remained paused; Sync now was unavailable; Resume removed the pause. |
| 2: close defaults/first sheet, Q-26 | PASS | New state reported close-quits Off. First-close sheet appeared; Hide window left the app alive with its own window hidden. Same-state second instance reopened it and exited 0. |
| 2: close-quits option | PASS (keyboard) | Focus + Enter changed Off to On; `app-settings.txt` persisted `closing_quits=1`; WM_CLOSE then exited without forced termination. UIA TogglePattern did not work; RC2-2. |
| 2: theme override, Q-34 | PASS (selection) | UIA SelectionItem selected Dark, Light, Follow system and each corresponding accessible radio name changed to `selected`. Pixel-level appearance and persistence across restart are NOT RUN. |
| 2: tray left/right click, Q-39 | **BLOCKED** | Two safe attempts could resolve the launched process's icon HWND/ID with Shell_NotifyIconGetRect, but UIA at that point returned the overflow container rather than a uniquely identifiable icon. No mouse click was sent. Source discrepancy: RC2-3. |
| 2: sheet Esc/focus, RC-3 | PASS | Set focus to “Explain close-to-tray again next time”, opened first-close sheet, Tabbed inside it, sent Esc to the recorded app window. The same opener regained keyboard focus; window stayed visible. |
| 2: fake-client apply block | PASS | Stand-in client in A's exact synthetic variant; B published version 2.0.0. A showed Game Running, one waiting update and the explicit no-game-file-changes banner; its version stayed 1.0.0. After stopping the stand-in by its captured launch PID, A applied 2.0.0. |
| 2: fresh setup wizard | PASS | Real UIA flow created A's group; a separate fresh B install/state joined that group. Both enrollment files and provider member records existed, and B received the two addons. No pre-enrolled fixture was substituted. |
| 2: no computer IDs in sentences, UG-22 | PASS (UIA-exposed text) | Each dump compared names with actual A/B computer IDs from their own `computer.json`; zero matches, including the peer-confirmation and running-game screens. Inaccessible body prose is outside this observation. |
| 3: two own negative controls, A-38 | PASS | Both exact named tests failed with cargo exit 101 under distinct mutations; restored control suite passed 6/6. Details below. |
| 4: A-29 memory | PASS (synthetic workload) | One full-duration release run per scenario, Scale 1.0. Peak private resident memory 37.75 MiB; 660 samples; one app-owned process throughout; 20/20 reopen cycles; zero harness findings. |
| 5: offline tier | PASS | Exit 0: fmt, clippy, workspace tests, required cases, deny and Markdown links. 47 listed offline cases against 320 test results. |
| 5: native tier | PASS | Exit 0: 4 native cases against 320 test results, no allow-blocked flag. |

These are scoped development results, not full A-26/A-29/A-34/A-35 release
acceptance. macOS, real Dropbox, screen readers and representative real addon
data were NOT RUN.

## Findings

### RC2-1 — Medium — fabricated next-check time in the real Overview

`crates/wow-sync-app/src/pages.rs:202` renders
`format!("{}; next in 4 min", rel(s))` whenever a last-check time exists.
The rebuilt real app repeatedly exposed:

```text
ControlType.Group: Last check, just now; next in 4 min
```

Both computers actually ran with `--interval-ms 1000`; the shipping default
in `main.rs` is 5000 ms, not four minutes. This violates UG-14 and R-12 and
the packet's prohibition on invented values. Derive the next due time from
the scheduler or honestly show it as unavailable. Source also contains a
literal “for 2 add-ons” in recovery-count copy (`pages.rs:209`), but that
conditional state was not reached; do not treat it as an executed finding.

### RC2-2 — Low — the advertised UIA close-option toggle is a no-op

Reproduced after explicitly focusing the switch:

```text
INITIAL=On
BEFORE UIA TOGGLE=Off
OFFSCREEN=False; FOCUSED=True
AFTER UIA TOGGLE=Off
KEYBOARD ENTER=On
PERSISTED=closing_quits=1
first_close_seen=1
KEYBOARD ENABLED CLOSE QUITS=True
```

TogglePattern is exposed and returns successfully but does not change this
Switch-role control. Enter works and persists the preference. The earlier
full GUI script consequently reported `CLOSING QUITS EXITED=False`, with
`closing_quits=0` still on disk: that was not evidence that the enabled
keyboard option fails. See [close-check.ps1](R3-check2/close-check.ps1).

### RC2-3 — Low — tray handler toggles rather than always opens

Source-confirmed discrepancy, **physical click NOT RUN**:
`app.rs:1016-1021` calls `hide()` on a left click when the window is already
visible. Q-39 says left click opens the window. The same event is also used
for the single-instance show request (`app.rs:363-364`). The hidden-to-shown
second-instance path was exercised, but this does not prove native mouse/menu
behavior. Correct the contract discrepancy and re-run actual tray interaction
in an environment where the own icon can be unambiguously targeted.

### RC2-4 — Low — devkit pipe capture can strand its stand-in

The initial `gui3` attempt used:
`$fakeId = [int](& $dev fake-client <synthetic-variant>)`.
The helper exited but PowerShell never completed that capture. Source gives
the child null stdin/stdout but inherits stderr; holding the native stderr
pipe open is consistent with the observed hang.

The successful `gui4` attempt instead used ProcessStartInfo with redirected
stdout/stderr, read the first stdout line immediately, and captured stand-in
PID **8272**. Only that returned PID was stopped; the pending update then
applied. The retained GUI script uses this working approach.

**Operational cleanup barrier:** the earlier stand-in's PID was not delivered
to its caller, so it was deliberately NOT stopped by inferred identity. A
read-only, worktree-path-scoped diagnostic observed PID **35908** at the
synthetic `target\rc2-scratch\gui3\A\install\_forever_\Wow.exe`. This is a
copied ping stand-in, not WoW. Its original app roots (29572 and 19964) were
stopped using their recorded launch PIDs. The hung command host was stopped
alone using the tool's registered launch PID 36228, never a process tree;
the stand-in was still present afterwards. Leave its lab directory intact
pending owner-authorized cleanup. **Historical PIDs are not future stop
authority; never blindly kill these numbers.** No unrelated app/demo, game
or launcher was stopped, signalled or automated.

## Build and reproducible commands

All commands ran from the checkout/branch stated above. For cargo calls:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = "$PWD\target\rc2-scratch"
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
cargo build --release -p wow-sync-app
cargo test -p wow-sync-runtime control::tests -- --nocapture
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

Release build: exit 0, optimized build, 2m50s. Independently built app SHA-256:
`63459CC617837E2FF4F5F78F430CC6E5C18EFC52075F9B5DCC91AFD0B37468A7`.
No demo binaries outside the worktree were overwritten.

Search:
`rg -n "scenario|fixture|fake|sample|demo" crates\wow-sync-app -g "*.rs"`.
Read the canonical runtime mapping (`world.rs`, `detail.rs`, `control.rs`) and
the referenced page/backend/tray handlers rather than relying on grep alone.

GUI procedure:

```powershell
& .\docs\runs\run-003\evidence\R3-check2\gui-check.ps1 -LabName gui4
& .\docs\runs\run-003\evidence\R3-check2\close-check.ps1
& .\docs\runs\run-003\evidence\R3-check2\tray-check.ps1
```

The GUI script was developed through isolated `gui`, `gui2`, `gui3`, `gui4`
labs. Early harness failures were an unsupported radio InvokePattern, an
already-exited second-instance PID lookup, and the native fake-client capture.
They are not hidden product passes. The final script uses SelectionItem for
radio buttons and handles immediate secondary-process exit. `gui4` ran to
completion, exit 0; its printed booleans are observations, not blanket
assertions of success (notably RC2-2).

[gui-continue.ps1](R3-check2/gui-continue.ps1) was the bounded continuation of
the recorded `gui3` app PIDs, which independently showed A blocked at 1.0.0
while B was 2.0.0, then cleaned up those two app roots. Its initial switch
InvokePattern was unsupported; the later focused keyboard test above
establishes the option's actual working path.

Synthetic data lived only under this worktree's `target\rc2-scratch`; provider
mode was an ordinary local directory. No real install, Dropbox data,
Battle.net, imported Lua or saved-variable files were used or executed.

## On-disk/GUI oracle

- A started with exactly two synthetic addon folders, both version 1.0.0.
  GUI Add-ons listed exactly those two titles/versions.
- Provider `group.json`, two `member.json` files and per-member state records
  agreed with local `enrollment.json`/`computer.json`. The UI changed from one
  computer to two, naming RC2-A and RC2-B, and reported B's actual 2/2 receipt.
- The activity UI showed enrollment, publication, pause and resume entries
  corresponding to the local audit log; raw “enrolled group …” and publication
  sequence jargon were rewritten instead of exposing computer IDs.
- B's version-2.0.0 publication appeared as one waiting update on running A;
  A's local metadata remained 1.0.0 until the captured stand-in stopped.
  Eight seconds later its local metadata was 2.0.0 and the game state was
  Not running.
- App settings accurately exposed deferred features as unavailable. The close
  preference was independently checked against its durable text record.
- Add-on detail navigation and its headings/peer link were real, but UIA did
  not expose the paragraph values. No complete body-text or screenshot proof
  is claimed. Needs attention/reconfirmation were not induced in this check.

## A-38 negative controls

No mutation or changed assertion was committed.

1. In `control::set_paused`, inverted `if paused` to `if !paused`.
   Command:
   `cargo test -p wow-sync-runtime control::tests::pause_persists_across_a_restart_until_resumed -- --exact`.
   Actual: one test FAILED at the `expect("paused")`; **exit 101**.
2. Restored that change. In `QuitGate::request`, inverted `if self.in_step`
   to `if !self.in_step`.
   Command:
   `cargo test -p wow-sync-runtime control::tests::quit_waits_for_the_safe_point -- --exact`.
   Actual: one test FAILED, `left: Now`, `right: AfterCurrentStep`;
   **exit 101**.
3. Restored the second change. `git diff --exit-code --
   crates\wow-sync-runtime\src\control.rs` exited 0; tracked-file status was
   empty. `cargo test -p wow-sync-runtime control::tests -- --nocapture`:
   **6 passed, 0 failed, 0 ignored**, exit 0. Native tier subsequently passed.

## A-29 memory

Command (release artifacts from this worktree):

```powershell
$env:TEMP = "$PWD\target\rc2-scratch"
$env:TMP = $env:TEMP
& .\tools\mem\run.ps1 -Runs 1 -OutDir '.\docs\runs\run-003\evidence\R3-check2\mem'
```

One full-duration run per scenario, default Scale 1.0; no shortened smoke
substitution. The harness enrolls its own synthetic installs through the real
wizard and stops only its directly launched root PIDs after identity checks.
Concurrent verification/GUI activity is measurement context, not a threshold
adjustment. The counter is aggregate **private working set**, not Private
Bytes; GPU is reported separately. Raw CSV/summary are retained in
`R3-check2/mem/`.

| Scenario | Steady median MiB | Whole-run maximum MiB | Samples | Verdict |
|---|---:|---:|---:|---|
| a: start hidden, idle 120 s | 28.42 | 28.72 | 120 idle | PASS |
| b: input/shown 60 s, hidden 120 s | 35.97 | 37.75 | 60 shown + 120 hidden | PASS |
| c: 20 show/hide cycles, idle 300 s | 35.24 | 36.25 | 60 cycles + 300 idle | PASS |

Every sample had one app-owned process. Scenario c confirmed all 20 reopen
cycles. Harness exit 0, findings 0; limit unchanged at 100 MiB. Context medians
(not gated counters):

| Scenario | Private Bytes MiB | Working Set MiB | GPU dedicated/shared MiB |
|---|---:|---:|---:|
| a | 56.02 | 59.09 | 0 / 13.00 |
| b | 99.96 | 71.57 | 0 / 40.11 |
| c | 104.48 | 69.34 | 0 / 21.09 |

The harness also reported launch-to-first-state-change 0.45/0.21/0.23 s;
the signal in every case was `state\admission.json`. That is **not** proof
of useful synchronization starting, so this report does not claim A-30 from
that proxy.

Raw evidence:
[a-run1.csv](R3-check2/mem/a-run1.csv),
[b-run1.csv](R3-check2/mem/b-run1.csv),
[c-run1.csv](R3-check2/mem/c-run1.csv),
[harness summary](R3-check2/mem/summary-abc-113750.txt).

## Handoff

- Fix RC2-1 before calling the real UI free of invented data.
- Keyboard functionality does not waive RC2-2's advertised UIA action defect.
- Re-test real tray left/right clicks without risking another demo process;
  the two attempts here deliberately sent no mouse input.
- Owner-authorized cleanup is still needed for the unreturned-launch-PID
  stand-in described in RC2-4. Its own lab is retained; other throwaway GUI
  labs can be regenerated with devkit. Do not stop a historical PID blindly.
- The verification JSON reports identified the product source above with
  `git_dirty=true` because evidence was being added; no clean-commit report
  or clean release certification is claimed. Product source restoration
  was independently checked with `git diff --exit-code` and the unchanged
  release SHA-256.
