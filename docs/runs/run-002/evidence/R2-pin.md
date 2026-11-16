# Evidence R2-pin: GPUI re-check on a pinned Zed release tag

Packet: [R2-pin](../packets/R2-pin.md) plus [COMMON](../packets/COMMON.md).
Worker R2-pin (claude-sonnet-5.5), branch `run002/R2-pin`. Authority: A-01,
A-29 method (PLAN §6.6), R-13, [PLAN](../PLAN.md) §6.2, run-001
[S-gpui](../../run-001/evidence/S-gpui.md), [S-mem](../../run-001/evidence/S-mem.md),
[S-a11y](../../run-001/evidence/S-a11y.md),
[S-a11y-design](../../run-001/evidence/S-a11y-design.md), P3-review M5.
Nothing here decides D-12 or edits root `deny.toml`.

## Environment

Windows 11 Pro 10.0.26300 (Insider) x64, i7-13700HX, 63.7 GiB, GPU
`NVIDIA RTX 2000 Ada Generation Laptop GPU` (reported by `gpu_specs`, not
software-emulated). Rust 1.100.0, cargo-deny 0.20.2. Release builds
(`opt-level 3`, thin LTO). Run from a fresh shell with
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
Spike: `spikes/gpui-pin/` (own `[workspace]`, own `Cargo.lock`, own `.gitignore`).
macOS runtime: BLOCKED (no Mac). Not a support claim (dev PC).

## Summary

| Item | Result |
|---|---|
| Pin chosen | **PASS**: tag `v1.28.1`, commit `e8bc16acee5ae1937600d571d441c4b5d7b0d288` |
| Lifecycle (tray, hide/show, close→hide, single instance, quit, keep-alive) | **PASS** all 14 observed items (quit via posted tray-menu command, see limits) |
| UIA exposure of heading / button / toggle / 10-item list | **PASS** (heading level not a UIA control type; see notes) |
| Memory, scenarios a, b, c vs 100 MiB (summed over all app processes, no trimming) | **PASS**, 11.7 to 19.3 MiB steady, max 22.6 MiB |
| Win x64 release build | **PASS** |
| Win ARM64 | compile **PASS** (`cargo check`); link **BLOCKED** (no ARM64 MSVC tools on this PC); CI to confirm |
| macOS targets | local **NOT RUN / BLOCKED** (Metal shader tooling absent on Windows); CI to confirm |
| cargo-deny candidate policy | **PASS** with the proposal below |
| `cargo xtask verify --tier offline` | **PASS** |

## 1. Pin selection

- `git ls-remote --tags https://github.com/zed-industries/zed` (2026-11-15):
  newest non-`-pre` tag with a release is `v1.28.1` = `e8bc16a…`
  (`v1.29.0-pre`, `v1.29.1-pre` are pre-releases, not used). Same tag named in
  S-a11y-design (marked "Immutable release").
- Source check in the cargo checkout (`crates/gpui_windows/src/window.rs`):
  `accesskit_windows::Adapter::new`, `A11yActivationHandler`,
  `A11yActionHandler`; `events.rs` forwards `WM_GETOBJECT` to AccessKit. So
  AccessKit is present at this tag, and it is the same commit that was
  built and exercised below.
- Pinned by `rev` (full SHA), crates `gpui` and `gpui_platform`, both from
  `https://github.com/zed-industries/zed`. `Cargo.toml` also states
  `version = "0.2.2"` / `"0.1.0"` (what the tag declares) so cargo-deny's
  wildcard ban passes.
- Lockfile: 631 packages; git sources from 6 repositories
  (`zed-industries/zed`, `gfx-rs/wgpu`, `zed-industries/{font-kit,oo7,scap,wasm_thread}`).
  `ring` is **not** in the lockfile (the run-001 ARM64 blocker with 0.2.2).
  Cold git fetch reuses the existing 413 MB Zed clone on this PC; a clean
  checkout needs GitHub reachable (S-a11y-design risk, unchanged).
- Keep-alive finding (source, `crates/gpui/src/app.rs`): the pin adds
  `Application::with_quit_mode(QuitMode::Explicit)`. Default is still
  `LastWindowClosed` on Windows (the 0.2.2 behaviour). With `Explicit` no
  anchor window is needed (least-cost keep-alive; verified below, K2/K3/H1).
  This removes the run-001 1x1 anchor-window workaround.

## 2. Spike app

`spikes/gpui-pin/src/main.rs`, binary `spike-gpui-pin`, built with
`#![windows_subsystem = "windows"]` (no console; no conhost child).
It logs to a file named by `WOWSYNC_LOG`.

- Tray (`tray-icon =0.26.1`): left-click toggles show/hide, menu Show/Quit.
- Window modes: `normal` (close → hide), `destroy` (close really destroys, tray
  Show reopens), `hidden` (no window at start), `show-hide SECS`, `cycles N`.
- Single instance: named mutex + event; the second instance signals the first
  and exits. `WOWSYNC_INSTANCE` only suffixes the names so the harness can run
  copies side by side.
- Controls with AccessKit roles/labels: heading (level 1), two buttons, a
  toggle (`Role::CheckBox` + `aria_toggled`), a 10-item list with
  `aria_position_in_set`/`size_of_set`; Tab/Shift+Tab bound explicitly.

Build: `cargo build --release --locked` in `spikes\gpui-pin`
(first cold build including all dependencies ≈ 1 min 50 s of compile before
the first source error; incremental rebuild 18 s). Result: **PASS**,
`target\release\spike-gpui-pin.exe` 10.9 MB.

## 3. Lifecycle, external observation (A-01)

Command: `powershell -NoProfile -File spikes\gpui-pin\drive.ps1`
(user32 window enumeration of the app's PID; messages posted to the app
windows; processes stopped by PID). Raw: [drive-result.txt](R2-pin/drive-result.txt)
(includes the app log).

| ID | Observation | Result |
|---|---|---|
| L1 | start: `Zed::Window` visible, tray window `tray_icon_app` exists | PASS |
| L2 | `WM_CLOSE` → window hidden, process alive | PASS |
| L3 | tray left-click (hidden) → shown | PASS |
| L4 | tray left-click (shown) → hidden | PASS |
| L5 | second instance exits (code 0), first shows its window, first alive | PASS |
| L6 | exactly 1 process | PASS |
| L7 | quit via tray menu → process exits (`run() returned`) | PASS (see limits) |
| K1 | `destroy` mode starts with a visible window | PASS |
| K2 | `WM_CLOSE` destroys the window; **no `Zed::Window` left, process and tray still alive after 3 s** (`QuitMode::Explicit`) | PASS |
| K3 | tray click with no window → new window opened and shown | PASS |
| K4 | closing that window again → gone, process alive | PASS |
| K5 | second instance while no window → first opens a window | PASS |
| H1 | `hidden` mode: no window ever, process + tray alive | PASS |
| H2 | tray click in `hidden` mode → window created and shown | PASS |

Limits: the tray click and the menu commands were **posted window messages**
to the tray window, not real mouse input on the notification area. Quit was
driven by posting `WM_COMMAND` with muda's internal item ids (1000 = Show,
1001 = Quit); the app log confirms `tray menu: Quit` → `cx.quit()`. A real
right-click menu was not exercised. The real X-button was not clicked
(`WM_CLOSE` is what the button sends). Run on one PC only.

## 4. UIA exposure (R-13 / A-26 method)

Command: `powershell -NoProfile -File spikes\gpui-pin\uia.ps1`
(.NET UIAutomationClient; raw: [uia-result.txt](R2-pin/uia-result.txt)).
Tree: `Window` "WoW Sync Pin Spike" with 15 descendants.

| Control | UIA | Result |
|---|---|---|
| Heading "WoW Sync Pin Heading" | type `Text`, named (level is not a UIA control type) | PASS (named); heading semantics NOT verified via UIA properties |
| Buttons "Sync now (0)", "Settings (0)" | `Button`, focusable, InvokePattern | PASS |
| Toggle "Enable sync" | `CheckBox`, TogglePattern | PASS |
| List "Items" + 10 `List item N` | `List` + 10 `ListItem`, all focusable | PASS (10/10) |
| Invoke "Sync now" | label changed to "Sync now (1)" | PASS |
| Toggle() | Off → On | PASS |
| SetFocus "List item 5", then Tab ×4 | UIA focus followed: 5 → 6, 7, 8, 9 | PASS |

Not done: Narrator speech (NOT RUN, owner check per D-12), macOS VoiceOver
(BLOCKED), `SelectionItem`/position-in-set patterns not exposed on list items
(`patterns=[]`), known upstream AccessKit issues from S-a11y-design not
retested. Result matches S-a11y on Zed `main`.

## 5. Memory (A-29 method, PLAN §6.6)

Harness: `spikes\gpui-pin\measure.ps1` via `run-all.ps1`; summary
`summarize.ps1` → [summary.txt](R2-pin/summary.txt); per-run CSV and app log
in `docs/runs/run-002/evidence/R2-pin/`.
Per second, sums `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate`
("Working Set - Private") over the launched PID **and all descendants**; GPU
dedicated/shared (per-PID `GPUProcessMemory`) recorded separately. No
`EmptyWorkingSet` or other trimming. Max descendant count observed per sample:
**1 process** in every run (console-less, no child). Scenarios as S-mem:
a = started hidden, no window ever, idle 120 s; b = window shown 60 s then
hidden, idle (180 s run); c = 20 show/hide cycles (1.5 s/1.5 s) then hidden
idle (360 s run). 3 runs each. Two hide variants: **destroy** (window really
closed, process kept alive by `QuitMode::Explicit`) and **sw** (`SW_HIDE`,
window kept: the close→hide design). Scenario a is one variant (no window).
The destroy and sw sets ran **concurrently** (two instances at once) to save
time; an earlier attempt was discarded because relaunching within milliseconds
hit the single-instance mutex (harness bug, fixed by waiting for process exit).

Steady = last 60 s. MiB, summed private working set:

| Scenario | Steady per run (1/2/3) | Whole-run max (1/2/3) | vs 100 MiB |
|---|---|---|---|
| a. never a window | 11.7 / 12.1 / 12.1 | 14.4 / 15.0 / 15.0 | **PASS** |
| b. shown 60 s, destroyed | 17.2 / 17.3 / 17.1 | 19.8 / 20.1 / 20.5 | **PASS** |
| b. shown 60 s, SW_HIDE | 17.6 / 17.6 / 17.0 | 20.0 / 20.4 / 19.7 | **PASS** |
| c. 20 cycles, destroy | 19.3 / 19.1 / 19.1 | 22.0 / 22.6 / 21.6 | **PASS** |
| c. 20 cycles, SW_HIDE | 17.6 / 17.7 / 17.7 | 20.2 / 20.0 / 20.1 | **PASS** |

Context (steady): Private Bytes (commit) 56 to 57 MiB (a), 73 to 74 (b), 92 to
93 (c destroy), 73 (c sw); Working Set 44 to 66 MiB. GPU, reported separately,
last sample: dedicated 7.8 MiB (a), 13.1 to 14.3 (b), 22.3 (c destroy),
15.4 (c sw); shared 0.2 to 1.1 MiB. Compared with run-001 0.2.2 (13 to 27 MiB
with anchor window), the pin is the same order or lower; steady values were
flat across runs (≤ 0.6 MiB spread).

Observation: after 20 destroy/recreate cycles, private bytes (92 MiB) and
GPU dedicated memory (22.3 MiB) are higher than after the single b cycle
(74 MiB, 14 MiB), while private working set stays ~2 MiB higher. Possible slow
growth in recreation was **not** characterised (one end value per run, 20
cycles). The design that hides with `SW_HIDE` showed no such growth. Limits:
synthetic UI, one PC and GPU, 100 MiB is the only numeric gate evaluated;
real UI (fonts, images), the sync engine and the updater are not in this
process. Aggregate over a product's other processes is not measured here.

## 6. Build targets

- **Win x64 release**: PASS (above).
- **Win ARM64**: `cargo build --release --locked --target aarch64-pc-windows-msvc`
  (2026-11-15, 1 min 19 s): all crates **including gpui, gpui_windows,
  accesskit_windows, tray-icon and the spike compiled**; the final step
  failed: `linker link.exe not found`. Cause: this PC's VS 2022 Build Tools
  have only `Hostx64\x64|x86` (no `Hostx64\arm64`; ARM64 C++ tools component
  missing). `cargo check --release --locked --target aarch64-pc-windows-msvc`:
  **PASS** ([arm64-check.txt](R2-pin/arm64-check.txt), full build log
  [arm64-build.txt](R2-pin/arm64-build.txt)). Link/runtime: **BLOCKED locally**; no
  system software was installed. Absence of `ring` removes the run-001 clang
  blocker. CI result to be recorded by the orchestrator.
- **macOS** (`cargo check --target aarch64-apple-darwin` on Windows): fails in
  `gpui_apple` because the build script cannot produce `shaders.metallib`
  without Apple Metal tooling. This is expected off-macOS and **not
  evidence about macOS**; **NOT RUN** locally. CI (`macos-latest`) is the
  evidence; the workflow now includes this spike.

## 7. Workflow

`.github/workflows/spike-gpui.yml`: matrix now `spike: [gpui, gpui-pin]` ×
4 targets (os assigned by `include` per target; `fail-fast: false`),
`working-directory: spikes/${{ matrix.spike }}`, push paths include
`spikes/gpui-pin/**`, added `concurrency: spike-gpui-${{ github.ref }}`
(cancel in progress). Action pins and `contents: read` unchanged. YAML not
executed locally (**NOT RUN**); the orchestrator's push provides the run.

## 8. cargo-deny: product policy proposal

Commands (in `spikes\gpui-pin`): `cargo deny --locked --config deny.toml check`
with the root policy (copied) as baseline
([deny-baseline.txt](R2-pin/deny-baseline.txt)) and the candidate
`spikes/gpui-pin/deny.toml` ([deny-candidate.txt](R2-pin/deny-candidate.txt)).

Baseline failures (root policy, 4 graph targets): licenses **4 rejected**
(BSD-2-Clause: `arrayref`; MPL-2.0: `cbindgen`; Zlib: `foldhash`, `slotmap`),
sources **16 `git` sources not allowed**, advisories **2 unmaintained**
(RUSTSEC-2026-0206 `rustybuzz`, RUSTSEC-2026-0192 `ttf-parser`, both
transitive through GPUI/usvg), bans **wildcard** (git dependencies without
`version`; the spike's own crate unlicensed is a spike artefact).

What a product `deny.toml` needs for this pin:

1. `[licenses] allow` additions: **BSD-2-Clause, Zlib, MPL-2.0**
   (all three were needed and encountered; CC0-1.0 and NCSA from the PLAN
   default were **not** needed for the 4-target graph: `dunce` is
   `CC0-1.0 OR MIT OR Apache-2.0`). The root `ISC` allowance was not
   encountered by this dependency set (it may be needed by other product
   crates). For the product crate itself set `[licenses] private = { ignore = true }`
   or give it a license; the spike uses the former.
2. `[sources] allow-git = ["https://github.com/zed-industries/zed"]`. The five
   other git repositories in `Cargo.lock` (wgpu, font-kit, oo7, scap,
   wasm_thread) are only used on non-target platforms; with the 4 targets
   in `[graph]`, adding them produced "allowed source not encountered"
   warnings, so they are not needed. Adding Linux/web targets would require
   them. Keep `unknown-git = "deny"`. `required-git-spec = "rev"` was not
   enabled or tested.
3. `[advisories] unmaintained = "workspace"`: unmaintained advisories on
   transitive crates are not actionable by the product (both flagged crates
   come from GPUI). cargo-deny 0.20 has no "warn" level for these (the
   PLAN's "warn on unmaintained" is expressed this way); direct dependencies
   remain checked. The alternative is two `ignore` entries with reasons,
   which must be re-reviewed per update. Vulnerability advisories stay deny
   (none found).
4. `[bans]`: keep `wildcards = "deny"` and put `version = "…"` on the git
   dependencies (as `spikes/gpui-pin/Cargo.toml` does). `multiple-versions =
   "warn"` yields 15 duplicate warnings (bitflags, getrandom, hashbrown,
   objc2*, png, syn, toml*, …); none block.

Result with the candidate: **advisories ok, bans ok, licenses ok, sources ok**
(exit 0). The root `deny.toml` was not edited.

## 9. Verification and reproduction

- `cargo xtask verify --tier offline` from the worktree root: **PASS**
  (fmt, clippy, test, required-cases, deny, markdown-links).
- Rebuild: `cd spikes\gpui-pin; cargo build --release --locked`.
  Lifecycle: `powershell -NoProfile -File drive.ps1`. UIA:
  `powershell -NoProfile -File uia.ps1`. Memory:
  `run-all.ps1 -OutDir <dir> -Set destroy` and `-Set sw`, then
  `summarize.ps1 -Dir <dir>` (~35 min per set).
- All test processes were stopped by PID; no stray `spike-gpui-pin` remained.
  WoW, Battle.net and Dropbox were not touched.

## 10. Open issues

1. ARM64 link and runtime unverified locally (needs VS ARM64 build tools or CI).
2. macOS builds and runtime unverified here (CI only; no Mac).
3. Real tray-menu click and real X-button click were not automated; message
   posting only.
4. Narrator speech and heading semantics not verified; list items expose no
   selection/position patterns.
5. Possible growth over many window destroy/recreate cycles (GPU dedicated
   22 MiB after 20 cycles) is uncharacterised; prefer hide-not-destroy or
   re-measure with the real UI.
6. Git-pin supply risk (413 MB clone, upstream reachability) unchanged.
7. `docs/REPO-MAP.md` does not yet list `spikes/gpui-pin` (outside this
   packet's owned writes).
