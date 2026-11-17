# R3-icon evidence

Packet: [R3-icon](../packets/R3-icon.md). Authority: D-22 in
[OWNER-DECISIONS](../../../OWNER-DECISIONS.md). Branch `run003/R3-icon` from
`main` 8601907. Environment: Windows 11 x64, Rust 1.100.0, GPUI pin
`e8bc16a` (Zed v1.28.1, unchanged). The art files are unmodified.

## Changes

| File | Change |
|---|---|
| `crates/wow-sync-app/src/brand.rs` (new) | `include_bytes!` of `assets/brand/wow_addon_sync.ico`; parses the ICO directory, decodes the 32 px PNG entry with `png`, caches it in a `OnceLock` (decoded once on first use). Two tests. |
| `crates/wow-sync-app/src/model.rs` | `tray_rgba` now starts from the brand art instead of the generated blue square and "W". The generated state badge (check, arrows, hourglass, exclamation, cross, bars, setup dot) in the lower-right is kept, so state is still carried by shape, not colour (existing test `tray_shape_differs_by_state_not_only_colour` still passes). `tray_image` in `app.rs` is unchanged: it calls `tray_rgba`. |
| `crates/wow-sync-app/src/main.rs` | `mod brand;` |
| `crates/wow-sync-app/build.rs` (new), `resources/wow-sync.rc` (new) | Windows only (`#[cfg(windows)]`): `embed_resource::compile_for(..., ["wow-sync"], ...)` embeds the ICO as icon resource id 1 into `wow-sync.exe` only. |
| `Cargo.toml`, `crates/wow-sync-app/Cargo.toml`, `Cargo.lock` | `png =0.18.1` (dependency), `embed-resource =3.0.12` (`[target.'cfg(windows)'.build-dependencies]`). |
| `docs/design/crates.md` | Dependency note. |

## Goals

1. **Tray icon: PASS.** The tray overflow flyout shows the brand art with the
   state badge ([tray-overflow-crop.png](R3-icon/tray-overflow-crop.png),
   first icon; [tray-area.png](R3-icon/tray-area.png) shows the notification
   area). The 32 px ICO entry is the size `tray-icon` was already given.
2. **Window/taskbar icon: PASS, and possible.** The pinned GPUI Windows
   backend (`gpui_windows/src/platform.rs` `load_icon`) loads icon resource
   **id 1** of the running module for its window class; `WindowOptions` has
   an `icon` field but that is X11 only. So resource id 1 in the exe also
   sets the window, taskbar and Alt-Tab icon; no Rust code needed.
   Evidence: [window-titlebar-icon.png](R3-icon/window-titlebar-icon.png),
   [taskbar-button-icon.png](R3-icon/taskbar-button-icon.png),
   [window-icon-from-hwnd.png](R3-icon/window-icon-from-hwnd.png) (class
   `HICON` read back from the live window handle).
3. **`.exe` icon: PASS.** `ExtractAssociatedIcon` on
   `target\release\wow-sync.exe` gives the brand art
   ([exe-icon-wow-sync.png](R3-icon/exe-icon-wow-sync.png)); `devkit.exe`
   gives the generic icon ([exe-icon-devkit.png](R3-icon/exe-icon-devkit.png)),
   so only `wow-sync.exe` carries it. **Explorer file-properties screenshot:
   NOT RUN** (the API extraction above is the substitute). macOS bundling
   (`.icns`, `Info.plist`) is out of scope.
4. **Tests: PASS** (see below).

## Dependencies

- `png 0.18.1`: MIT OR Apache-2.0, maintained by image-rs. Already in
  `Cargo.lock` (via `tray-icon`, `image`), so no new version enters the tree;
  needed to decode the ICO's PNG entries at run time. No network use.
- `embed-resource 3.0.12`: MIT, widely used. Already in `Cargo.lock` (GPUI's
  build). Windows build dependency only; needed to embed the `.rc`/ICO
  resource into one binary. `compile_for` limits it to `wow-sync`.
- `deny.toml` unchanged; `cargo deny` passes (offline tier).
- Only two `Cargo.lock` lines were added (the two edges on `wow-sync-app`).

## Tests and results

```
cargo test -p wow-sync-app brand
brand::tests::bundled_art_decodes_to_tray_size_with_visible_pixels ... ok
brand::tests::corrupt_art_is_rejected ... ok
test result: ok. 2 passed
```

Negative controls (A-38; mutations were temporary and restored, nothing
committed):

- `decode_png_rgba` returned all `0xff` bytes without decoding:
  `corrupt_art_is_rejected` FAILED (`assertion failed:
  decode_ico_entry(&mangled, TRAY_SIZE).is_err()`), exit 101.
- `decode_png_rgba` returned all-zero (transparent) bytes: both
  `bundled_art_decodes_to_tray_size_with_visible_pixels` and
  `corrupt_art_is_rejected` FAILED, exit 101.

The corrupt-bytes cases: empty input, zero bytes, the 32 px entry truncated,
and the 32 px entry overwritten.

```
cargo xtask verify --tier offline
[PASS] offline/fmt  clippy  test  required-cases  deny  markdown-links
overall: PASS -> exit 0
cargo xtask verify --tier native
[PASS] native/tier
overall: PASS -> exit 0
```

(One earlier offline run failed clippy `chunks_exact_to_as_chunks`; fixed with
`as_chunks` and re-run, PASS.)

## A-29 memory (tools/mem, release build, 3 scenarios x 3 runs, scale 1)

Raw CSVs in [R3-icon/mem](R3-icon/mem). Private working set, limit 100 MiB:

| Scenario | Steady median per run (MiB) | Whole-run max | Verdict |
|---|---|---|---|
| a | 28.86 / 28.52 / 28.59 | 30.1 | PASS |
| b | 35.75 / 35.15 / 35.38 | 37.57 | PASS |
| c | 34.98 / 35.20 / 35.35 | 36.52 | PASS |

Sync start: median 0.23 s, max 0.28 s (n=9, no timeouts). Findings: 0. Lab
data under `D:\WoWSyncLab\R3-icon` (synthetic).

## Limits and notes

- macOS: `build.rs` Windows code and `embed-resource` are `cfg(windows)`;
  `png` and `brand.rs` are portable. CI must confirm the macOS build; not run
  locally (NOT RUN).
- The new tests are not in `tests/required-cases.toml` (not an owned path);
  they run in the offline tier through `cargo test`. The orchestrator may
  list `brand::tests::*`.
- The badge covers the lower-right quarter of the 32 px art. Owner may prefer
  an unbadged icon; that would lose the shape-based state signal (SPEC
  accessibility), so it was kept.
- Screenshots are cropped to the app window, taskbar button and notification
  area; the rest of the screen was not saved.
- Trademark caution from D-22 stands: revisit before public release.
