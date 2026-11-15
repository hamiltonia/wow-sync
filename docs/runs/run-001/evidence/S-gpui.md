# Evidence S-gpui: GPUI window and tray lifecycle

Run 001, Phase 2. Worker S-gpui, branch `run001/S-gpui`. Informs R-01, R-13, A-01
(no product claim). Packet: [S-gpui](../packets/S-gpui.md).

## SPEC section 7 question and answer

Question: can a GPUI app with a system-tray icon start with a window, hide it to
the tray (process stays), reopen from the tray, quit explicitly, run as a single
instance, and not require a discrete GPU, on all four architectures?

**Answer: feasible on Windows x64 (tested); unknown on Windows ARM64 and both macOS
targets.** Confidence: high for Windows x64 lifecycle (observed from outside the
process); low/none for the other three (not built or not runnable here). What would
change it: a real tray-shell click test by a person, ARM64 and macOS CI build/runtime
results, and a WARP/no-GPU machine test.

## Environment

- Windows 11 x64 dev PC (Insider build 26300, not official support evidence),
  NVIDIA RTX 2000 Ada Laptop GPU plus Intel UHD Graphics. Rust 1.100.0 (root pin).
- `gpui = "=0.2.2"` from crates.io, `tray-icon = "=0.26.1"`, `raw-window-handle 0.6`.
  Chosen over a zed git rev because a registry version plus committed `Cargo.lock` is
  the most reproducible and needs no git access; 0.2.2 is the newest published version
  at the time (`cargo search`). Cost: it lags zed's main branch.
- Spike code: `spikes/gpui/` (own `[workspace]`, `Cargo.lock` committed).

## How the two event loops coexist

GPUI on Windows runs the main-thread Win32 message loop. `tray-icon` creates a hidden
window (class `tray_icon_app`) on the calling thread, so creating the tray inside
`Application::run` makes GPUI's loop also dispatch tray messages. No second loop or
thread is needed. `tray-icon` hands events to global channels
(`TrayIconEvent::receiver`, `MenuEvent::receiver`); a GPUI `cx.spawn` task polls them
every 50 ms and acts via GPUI/Win32. **PASS** (works, Windows x64). Poll latency is
negligible for this purpose, but an event-driven handoff would be cleaner (option below).

## Lifecycle results (Windows x64, release build, real window)

Method: `spikes/gpui/drive.ps1` launches the exe and observes it from outside via
user32 `EnumWindows`/`IsWindowVisible` by PID (raw: `evidence/S-gpui/drive-result.txt`).
The in-process self-test (`WOWSYNC_SPIKE_SELFTEST=1`) logs timestamped transitions
(raw: `evidence/S-gpui/selftest-result.txt`).

| Item | Result | Notes |
|---|---|---|
| Starts with window | PASS | External: `Zed::Window` visible 4 s after launch. |
| Hide to tray, process stays | PASS | Self-test and external: window hidden, process alive, tray hidden-window still present. |
| Reopen from tray | PASS (synthetic) | Posted the tray callback message (6002, WM_LBUTTONUP) to the `tray_icon_app` window; the app received the Click event and reshowed. Not a real shell click. |
| Tray click toggles hide again | PASS (synthetic) | Same method. |
| Close button intercepted | PASS | `Window::on_window_should_close` returning `false`; external `WM_CLOSE` hid the window and the process stayed. Hiding itself is a Win32 `ShowWindow(SW_HIDE)` call. |
| Explicit quit | PASS (in-process) | Self-test calls `cx.quit()` (the same function the Quit menu item handler calls); `run()` returned, exit code 0. |
| Quit via real tray menu click | NOT RUN | Needs a person to right-click the tray icon (below). |
| Real tray icon visible and clickable in the shell | NOT RUN | Not automated; owner manual check (below). |
| Single instance | PASS | Named mutex `Local\WowSyncSpikeGpuiSingleInstance`; second launch signalled a named event and exited; first window (hidden) reappeared; process count stayed 1. |
| Memory | informational | Working set 59-62 MiB for a trivial window (not the A-37 [correction by P3-fix: A-29] private-resident gate; not measured properly). |

Finding: GPUI 0.2.2 has no public hide/show window API (only minimize, remove,
activate). Hide/show therefore uses the raw HWND (via `HasWindowHandle`) and
`user32`. This is workable on Windows but needs per-platform code; macOS
(`NSWindow orderOut`/Dock activation policy) is a different mechanism and is untested.
GPUI also does not report the hidden state, so the app must track it itself.

## Renderer and GPU

GPUI on Windows uses Direct3D 11 and picks the first DXGI adapter that supports it.
`Window::gpu_specs()` reports the adapter.

- Default: NVIDIA RTX 2000 Ada (software_emulated=false). **PASS**
- Integrated GPU: with the per-app Windows preference `GpuPreference=1;` (power
  saving) set for the exe under `HKCU\Software\Microsoft\DirectX\UserGpuPreferences`,
  GPUI used Intel(R) UHD Graphics, window opened. **PASS** (raw: `gpu-result.txt`;
  script `gpu-pref.ps1` restores or removes the preference; the key did not exist
  before and was removed again).
- WARP / no GPU at all: **NOT RUN.** Forcing it would require disabling both adapters
  (a machine-wide change). Source reading says it would fall through to any adapter
  that supports D3D 11, including the Basic Render Driver, but this is untested.
  Needs a VM or a machine without a usable GPU.

## Builds

| Target | Result | Detail |
|---|---|---|
| x86_64-pc-windows-msvc | PASS (built and run) | Release. Cold build with registry sources already fetched: about 3 min 11 s (other workers were building concurrently). Binary 11.07 MB (11,067,392 bytes). Debug-info stripping not tried. |
| aarch64-pc-windows-msvc | BLOCKED (build-only attempt failed) | `rustup` target is installed, but `ring v0.17.14` (via gpui -> gpui_http_client -> zed-reqwest -> rustls) fails in its build script: `failed to find tool "clang"`; the VS 2022 Build Tools here only have x64/x86 libraries and tools (no ARM64 component). Not worked around: would require installing LLVM/clang and the VS ARM64 build tools. |
| x86_64-apple-darwin | NOT RUN locally (no Mac; cannot build here). CI workflow added | See below. |
| aarch64-apple-darwin | NOT RUN locally. CI workflow added | See below. |

macOS runtime is BLOCKED (no Mac). The workflow
`.github/workflows/spike-gpui.yml` (this packet owns it) is build-only,
`workflow_dispatch` plus push limited to `spikes/gpui/**` and the workflow itself,
read-only permissions, no secrets, action pinned as in `ci.yml`. **Not pushed;
the orchestrator must push and record the run result.** Risk: gpui's macOS build
compiles Metal shaders and may need the Metal toolchain on the runner; the workflow
prints diagnostics but does not install it. Windows ARM64 on `windows-latest` may or
may not have clang and ARM64 MSVC tools; the run will show it.

## Manual checks for the owner (NOT RUN)

1. Run `spikes\gpui\target\release\spike-gpui.exe` (build: `cargo build --release` in
   `spikes\gpui` with the cargo bin dir on PATH).
2. Confirm a tray icon appears (it may be in the overflow flyout).
3. Click the window close button: window should vanish, icon stays.
4. Left-click the tray icon: window reappears; click again: hides.
5. Right-click the tray icon: Show and Quit menu; Quit should end the process.
6. Start the exe again while running: no second window or process, the first reappears.

## Risks and recommendations

- Proceed with the GPUI plus `tray-icon` approach on Windows x64 as a viable base: the
  windowing, hide-to-tray, close interception, single instance and iGPU operation all
  worked. This does not switch frameworks.
- Hiding/showing needs OS-specific code outside GPUI's API; budget for it on macOS.
- gpui 0.2.2 drags a heavy dependency tree (reqwest/rustls/ring, blade, etc.) even for
  a tray-resident process. 11 MB binary and about 60 MiB working set for a trivial
  window mean the 100 MiB aggregate background-memory budget (R-13/A-37 [correction by P3-fix: memory case is A-29]) needs real
  measurement, and a design that closes the GPUI window (or runs the sync engine in a
  separate process) when idle may be needed. Not measured here.
- Windows ARM64 needs clang and ARM64 MSVC tooling in CI/dev (ring build script).
  Decision for the owner/orchestrator: accept that tooling, or evaluate alternatives.
- The 50 ms channel poll should become an event-driven wake (`tray-icon` event handler
  plus a GPUI-side notify) in any real design.
- Alternatives to evaluate only if the above fails: a split tray/background process
  (no GPUI) plus an on-demand GPUI settings process.

## Rerun

```
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd spikes\gpui
cargo build --release
$env:WOWSYNC_SPIKE_SELFTEST = "1"; .\target\release\spike-gpui.exe   # scripted lifecycle
powershell -NoProfile -File drive.ps1                                 # external observation
powershell -NoProfile -File gpu-pref.ps1                              # iGPU forcing, restores registry
```

Method limits: single machine, single run of each scripted pass; tray events were
synthesized by posting the library's callback message rather than real shell input;
no soak, no DPI/multi-monitor, no sleep/resume tests.

## CI build-only evidence (added by P3-fix)

`gh run view 43756896472 --json conclusion,headSha,jobs` showed: conclusion
`success`, headSha `88242f196a50e16b94ed944f9bc1b28d49a9689e` (commit 88242f1),
workflow spike-gpui.yml, four jobs all `success`: build-only
x86_64-pc-windows-msvc, aarch64-pc-windows-msvc, x86_64-apple-darwin and
aarch64-apple-darwin. This shows the spike compiles for all four targets; it is
build-only and says nothing about running, tray behavior or memory on any of them.
