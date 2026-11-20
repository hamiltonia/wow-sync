# R4-mac-kit evidence

Packet: [R4-mac-kit](../packets/R4-mac-kit.md). Branch `run004/R4-mac-kit`, from `main` at `1a9129d`.
Intent: D-23 (Mac available), ops.md §3, §4, §7 rows T-27-11 and T-28-13, ACCEPTANCE A-29.

## What was built

A macOS kit the owner runs, plus the owner instructions. No crate, lock file, CI or Windows tool changed.

| File | Purpose |
|---|---|
| `tools/mac/lib.sh` | Shared helpers: neutral lab root with a guard, synthetic lab creation, own-PID stop, process tree, footprint measurement, environment report |
| `tools/mac/build.sh` | Prerequisites (Xcode CLT, rustup, pinned rustc), `cargo xtask verify --tier offline`, `cargo build --release -p wow-sync-app --bins` |
| `tools/mac/run.sh` | `enroll` / `start [--hidden]` / `clean` on one synthetic lab, using devkit and `--dev` as the Windows harness does |
| `tools/mac/mem.sh` | A-29 scenarios a, b, c of `tools/mem/run.ps1`; aggregate private memory; sync start; results file with environment |
| `tools/mac/notify-check.sh` | T-27-11: one notification through `OsNotifier`, owner confirms the name |
| `tools/mac/update-check.sh` | T-28-13: the existing manual live-fetch test, one anonymous GET |
| `tools/mac/bundle.sh` | Builds an unsigned (ad-hoc signed) `WoW Sync.app` (`CFBundleIdentifier=com.wowsync.app`, `CFBundleName` and `CFBundleDisplayName` "WoW Sync", `CFBundleExecutable`, `LSUIElement`, `.icns` from `assets/brand/wow_addon_sync.png` via `sips`/`iconutil`) around the release binary and installs it to `~/Applications`; `--dock`, `--dir`, `--uninstall` |
| `tools/mac/.gitattributes` | Keeps the scripts LF on every checkout |
| `docs/runs/run-004/MAC-OWNER.md` | Non-developer steps: install, commands, what to look at, what to send, clean-up |

## What runs on macOS (from reading the code)

- App `wow-sync` and `devkit` build for macOS: `crates/wow-sync-app/Cargo.toml` has no Windows-only
  runtime dependency (`embed-resource` is `cfg(windows)` build-only). `devkit make-install` and
  `add-addon` are portable (`crates/wow-sync-app/src/bin/devkit.rs`); `fake-client` is Windows only
  (line 108), unused by the kit.
- Developer mode (`--dev --install --provider --state --name`) runs the runtime sync thread
  (`crates/wow-sync-app/src/main.rs` lines 156 to 200) with the GPUI window and tray
  (`crates/wow-sync-app/src/app.rs`, `tray-icon`).
- Enrolment is required before sync runs (`backend.rs`, `status().enrolled`), and there is no UI
  automation on macOS, so the owner clicks through the setup guide once (`run.sh enroll`). The
  memory kit then reuses that lab and makes a fresh synthetic addon change before each run so
  the first sync has something to publish and its start is measurable.
- Notifications: `OsNotifier` (`crates/wow-sync-adapters/src/notify.rs` lines 58 to 72) uses
  `notify-rust` on macOS.
- Update check: `GithubReleases` and the ignored `manual_live` test are platform neutral
  (`crates/wow-sync-adapters/src/update_http.rs` line 272).

## Memory method (A-29, macOS)

Private resident memory = macOS **physical footprint**, read with `vmmap --summary <pid>`
("Physical footprint:" line, the number Activity Monitor shows as "Memory"), falling back to
`footprint -p <pid>`. It is summed over the launched root and every live descendant
(`ps -axo pid,ppid`) about once per second. RSS from `ps` is recorded as context only. The verdict
is the Windows one: whole-run maximum of the aggregate at most 100 MiB. Sync start is launch to
the first changed file in the lab's `state` or `provider` folders, polled every 0.2 s.

Divergences from `tools/mac` vs `tools/mem/run.ps1`, all forced by the platform:

- No UIA: enrolment is manual (once); scenario b scroll/click and window close, and scenario c
  show/hide cycles, are owner-driven with prompts. Scenario a is unattended.
- Scenario c cannot use a second launch to show the window (see handover H-1); the owner uses the
  menu-bar icon instead, and reports the cycle count.
- The "window hidden after close" assertions of the Windows harness are not made.

## How each script was checked (Windows host, no macOS available)

| Check | Command (environment) | Result |
|---|---|---|
| Bash syntax | `bash -n` and `bash --posix -n` on all six scripts (Git for Windows bash) | PASS |
| Strict POSIX syntax | `dash -n` on LF copies of all six scripts (Git for Windows dash) | PASS (first attempt failed on CRLF working-copy line endings; fixed with `.gitattributes`, copies exported with `git archive`) |
| Footprint parser | `footprint_bytes` with stubbed `vmmap`/`footprint` output `45.2M`, `512K`, `1.5G`, `980B`, `123`, and `Footprint: 45 MB` | PASS (47395635, 524288, 1610612736, 980, 123, 47185920) |
| Process-tree walk | `tree_pids 10` with stubbed `ps` | PASS (10 11 12, sibling excluded) |
| Median and seconds helpers | inline awk on sample input | PASS (median 2.5; scale 0.1 of 120 s gives 12) |
| Lab-root guard | `check_lab_root` with Dropbox, World of Warcraft, Library and relative paths | PASS (all refused) |
| Review against the Windows harness | read-through | done, divergences listed above |
| Follow-up (bundle): syntax | `bash -n` and `dash -n` on all seven scripts incl. `bundle.sh` (LF copies via `git archive`) | PASS |
| Follow-up (bundle): `pick_app` | dash with a fake bundle executable: bundle chosen when present, raw with `WOWSYNC_NO_BUNDLE=1` | PASS |

Not executed, because they need macOS: everything that calls `sw_vers`, `sysctl`, `vmmap`,
`footprint`, `xcode-select`, `stat -f`, the app, notifications, the network check, or a build.

## Rows the owner will produce

| Row | Environment | Result |
|---|---|---|
| T-27-11 macOS (notification shows "WoW Sync") | owner's MacBook Pro, Apple Silicon | NOT RUN |
| T-28-13 macOS (one live GET) | owner's MacBook Pro | NOT RUN |
| A-29 macOS (at most 100 MiB aggregate; sync starts within 10 s) | owner's MacBook Pro | NOT RUN |
| macOS build and `cargo xtask verify --tier offline` through `build.sh` | owner's MacBook Pro | NOT RUN |
| Enrolment and tray on macOS (`run.sh enroll`) | owner's MacBook Pro | NOT RUN |
| `bundle.sh` result (valid plist, icon, ad-hoc signature, "WoW Sync" sender) | owner's MacBook Pro | NOT RUN |

Unverified assumptions (each fails visibly on the Mac, none is claimed): BSD `stat -f %Fm` gives
fractional mtimes; `vmmap --summary` works for own processes without sudo; the app exits on SIGTERM.

## Handover (gaps outside this packet; nothing was edited)

- **H-1 Second instance / show request is a no-op on macOS.** `crates/wow-sync-app/src/win.rs`
  lines 108 to 113: `single_instance` always returns `Some(0)` and `show_requested` is false, so a second
  launch starts a second full app instead of raising the first, and the memory harness
  scenario c cannot reuse the Windows method. Also affects the A-29 "aggregate" count if a user
  starts twice.
- **H-2 Window hide and show do nothing on macOS.** `win.rs` line 107 (`set_visible` empty) and
  `crates/wow-sync-app/src/app.rs` lines 251 to 254 (`hwnd_of` returns 0 off Windows), used at app.rs
  lines 782 and 788. Only `show: false` at start hides the window; closing may not hide it to the tray
  as the Windows behaviour does. The owner is asked what happens on close.
- **H-3 The app never attaches the OS notifier or update source.** `Service::attach_os_services`
  (`crates/wow-sync-runtime/src/service.rs` line 444) has no caller in the tree at this commit, so the
  running app shows no notification and makes no update request on any platform. The kit calls
  `OsNotifier` directly; if R4-app wires `attach_os_services`, a runtime-path trigger for T-27-11 could
  replace the scratch program.
- **H-4 macOS notification identity (addressed in `f2597cf`, kit updated).** `notify.rs` now calls
  `set_application("com.wowsync.app")` on macOS. The name "WoW Sync" appears only if a bundle with
  that identifier exists, so `tools/mac/bundle.sh` builds and installs one, and `run.sh`, `mem.sh`
  and `notify-check.sh` use it. Still NOT RUN: whether macOS then shows "WoW Sync" is the owner's
  T-27-11 result. The bundle is ad-hoc signed only (not notarized); the owner may have to right-click
  and Open once, and macOS may ask to allow notifications for "WoW Sync".
- **H-5 The existing manual toast test cannot run on macOS.** `notify.rs` test
  `manual_toast_shows_the_app_name` unwraps `register_app_user_model_id`, which returns Err off Windows
  (`winproc.rs` unsupported-platform stubs). The kit avoids it with a scratch program.
- **H-6 Windows-only defaults.** `main.rs` line 102 (`COMPUTERNAME`) and line 109 (`LOCALAPPDATA`) have no
  macOS counterpart: non-dev runs get "This computer" and a temp-directory state folder. The kit
  uses dev mode with explicit `--name` and `--state`, so it is not affected; a normal macOS start is.
- **H-7 `local_offset_minutes` returns 0 on macOS** (`win.rs` line 114), so times shown are UTC.
- **H-8 Bundle limits.** The bundle is a copy of the release binary with `LSUIElement` (no Dock icon) and
  an ad-hoc signature; it is not notarized and has no auto-update of the copy (rerun `bundle.sh` after
  a rebuild; the scripts warn when the build is newer). Window and menu-bar behaviour inside a
  bundle is unverified.

## Verify

`cargo xtask verify --tier offline` from the worktree on the final tree (Windows 11 x64): PASS for
fmt, clippy, test, required-cases, deny and markdown-links; overall exit 0. The `native` tier was
not required (no crate changed).
