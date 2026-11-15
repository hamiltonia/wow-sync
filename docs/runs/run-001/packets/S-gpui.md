# Work packet S-gpui: GPUI window and tray lifecycle

Role: implementation. Model: claude-sonnet-5.5 (D-04). Common rules:
[S-common.md](S-common.md). ID `S-gpui`; spike dir `spikes/gpui/`.

## Intent

SPEC §7 "GPUI/tray lifecycle, reopening ... all four architectures"; informs
R-01, R-13, A-01 (no product claim). Does a GPUI app with a system-tray icon
support: start with window, hide window to tray (process stays), reopen from
tray, explicit quit from tray/menu, single instance, no discrete GPU required?

## Work

1. Minimal GPUI app (crates.io `gpui` or a pinned git rev of zed's gpui; pick
   the most reproducible, record version/rev and why) plus a tray-icon
   approach (e.g. `tray-icon` crate, or GPUI's own if it exists). Record how the
   two event loops coexist.
2. Lifecycle exercised programmatically: add a scripted self-test mode (env var
   or flag) that opens the window, hides it, confirms the process and tray
   remain, reopens, and quits, logging each transition with timestamps. Also
   verify from outside the process (PowerShell: window visibility via
   `Get-Process` / UI Automation / `user32` calls) that the window really hid
   and reappeared. Manual tray clicking is NOT RUN unless you can automate it;
   say so and list the manual check for the owner.
3. Close-button behavior: can the window close be intercepted to hide instead
   of exit? Document.
4. Renderer: which GPU/backend is used on this PC (RTX 2000 Ada + Intel UHD);
   can it run on the integrated GPU / WARP (no discrete GPU)? Try forcing it if
   feasible; otherwise NOT RUN with reason.
5. Builds: `x86_64-pc-windows-msvc` (run), `aarch64-pc-windows-msvc`
   (attempt local cross-build; if ARM64 MSVC libs are missing, BLOCKED with the
   exact error; do not install large toolchains beyond the rustup target).
   macOS targets cannot be built here: add `.github/workflows/spike-gpui.yml`
   (this file is owned by you) with `workflow_dispatch` and push triggers
   limited to `spikes/gpui/**`, `permissions: contents: read`, no secrets,
   actions pinned to full SHAs exactly as in `.github/workflows/ci.yml`
   (copy its pins), building `spikes/gpui` for the four targets (Windows
   x64/ARM64 on `windows-latest`; `aarch64-apple-darwin` native and
   `x86_64-apple-darwin` cross on `macos-latest`). Label jobs build-only. Do not
   push; the orchestrator pushes and records the run.
6. Record cold build time, binary size, and any blocking issues.

## Owned writes

`spikes/gpui/`, `.github/workflows/spike-gpui.yml`, evidence files (S-common).

## Done when

Evidence answers each lifecycle item with PASS/FAIL/BLOCKED/NOT RUN on Win x64,
build status per target, and a recommendation (proceed with GPUI+tray approach
X / risks / alternatives to evaluate) without switching frameworks.
