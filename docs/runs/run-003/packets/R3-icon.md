# R3-icon (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first. Authority: **D-22** in
`docs/OWNER-DECISIONS.md`. This packet supersedes goal 12 of R3-fix.

**Inputs:**
- `assets/brand/wow_addon_sync.ico`: 7 sizes from 16 to 256 px, each
  stored as PNG.
- `assets/brand/wow_addon_sync.png`: 1254 px.

Do not modify the art.

**Owned paths:**
- `crates/wow-sync-app/**`, including a `build.rs`.
- `Cargo.toml` and `Cargo.lock` (only if a dependency is needed).
- `deny.toml` (only if a new license must be allowed; justify it).
- `docs/design/crates.md` (a dependency note).
- `docs/runs/run-003/evidence/R3-icon.md` and `evidence/R3-icon/**`.

File writes go through edit/create or `cargo fmt` only (incident I-1). Use
only synthetic data. Stop processes only by the PID you captured at launch.

## Goals

1. **Tray icon.** Replace the generated 32x32 pixels in `app.rs`
   `tray_image` with the brand art, at the size the platform tray expects.
2. **Window icon** (the taskbar and Alt-Tab icon on Windows), if GPUI
   exposes a way to set it. Otherwise record that it is not possible with
   the pinned GPUI.
3. **`.exe` icon (Windows).** Embed the ICO as the executable's icon
   resource.
   - Only `wow-sync.exe` needs it. The devkit and proto binaries don't.
   - macOS bundling is out of scope; note it.
4. **Dependencies.**
   - Prefer what is already in `Cargo.lock`. For example, if the `image` or
     `png` crate is already in the tree through GPUI, use it with the
     same version.
   - A small, widely used, free build dependency (such as `winresource` or
     `embed-resource`) is acceptable for goal 3 if needed. Justify it in
     the evidence (license, maintenance, why it's needed) and make sure
     `cargo deny` passes. Do not add a paid or network-using dependency.
   - Decoding at runtime must stay within A-29: re-run `tools/mem` and stay
     under 100 MiB. Decoding once at start-up, or pre-decoding at build
     time, is fine.
5. **Tests.** Add a test that the bundled art decodes to the tray size with
   non-transparent pixels. Run a negative control: point the test at a
   corrupt byte slice and show it fails.

## Done

- `cargo xtask verify --tier offline` and `--tier native` both pass.
- The macOS build must still compile. Gate Windows-only code with
  `#[cfg(windows)]`, and note in the evidence that CI checks this.
- Screenshots: the tray icon area, if it can be captured safely; the
  window or taskbar; and the `.exe` file properties or Explorer icon. If a
  screenshot isn't possible, give a verdict of NOT RUN with the reason.
- A-29 re-run under 100 MiB.
- Evidence lists every change, dependency and result.
- Commit on your branch. Do not push.
