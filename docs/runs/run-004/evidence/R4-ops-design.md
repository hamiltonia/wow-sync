# Evidence: R4-ops-design

- **Packet:** [R4-ops-design](../packets/R4-ops-design.md)
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-ops-design`
- **Branch:** `run004/R4-ops-design`, from `main` at `573615b`
- **Environment:** Windows x64 owner workstation, rustc 1.100.0, cargo-deny
  0.20.2

## What was built

This packet is design only. No product code changed.

- **[docs/design/ops.md](../../../design/ops.md)** (new) covers:
  - decision resolution, with UQ-1 and UQ-2;
  - retiring a computer;
  - notifications;
  - the update notice;
  - diagnostics export;
  - the implementation map, test list and GUI touchpoints.
- **[docs/design/ui/questions.md](../../../design/ui/questions.md):** UQ-1 and
  UQ-2 are marked decided yes by D-23, with links to the design.
- **[docs/design/questions.md](../../../design/questions.md):** a new section
  appends Q-54 to Q-61.

## Requirement-to-section map

| Requirement (packet goal) | SPEC / ACCEPTANCE | ops.md section |
|---|---|---|
| Decision actions per kind, preconditions | R-06, R-13 / A-26 | §1.1, §1.2, §1.3 |
| What core records, how peers see it | R-08 / A-26 | §1.4 |
| Undo and recovery | R-10 / A-26 | §1.5 |
| UQ-2 typed scope | D-23 | §1.6 |
| UQ-1 "Review needed", pause never hides NeedsHelp/Error | D-23 | §1.7 |
| Retire flow keeps history | R-03 / A-45 | §2.1, §2.3 (2), §2.5 |
| Currentness no longer waits for the retired computer | R-07, R-12 / A-45 | §2.3 (4) |
| Stale/offline publications ignored | R-03 / A-45 | §2.2, §2.3 (1) |
| Explicit re-enrollment | R-03 / A-45 | §2.3 (6), §2.4 |
| Settings master: reassign guidance, never auto-switch | R-07 / A-45 | §2.6 |
| Which events notify, dedup, rate limits, quiet while gaming, opt-out | R-13 / A-27 | §3.1, §3.2 |
| Notification crate (P4-2) with E-06 justification | D-24 / E-06 | §3.3 |
| Anonymous daily GET, silent offline, no ids, opt-out | R-15, R-16 / A-28, D-24 P4-1 | §4.1, §4.2, §4.3 |
| TLS approach passing cargo deny on four targets | E-06 | §4.4 |
| Diagnostics requested, previewed, redacted, bounded, format, never included | R-15 / A-33 | §5.1 to §5.4 |
| Types and functions mapped to modules | E-01 | §6 |
| Test list mapped to A-IDs with negative cases (planted strings) | E-03, E-04 / A-38 | §7 (T-33-1, T-33-2, T-33-4 plant private strings) |
| GUI screens | R-13 / A-26 | §8 |

## Commands and results

### Dependency research

The scratch crate was `target\r4-ops-scratch`, which is gitignored. It held a
copy of the repository's `deny.toml`. It was deleted after use.

| # | Command (scratch dir) | Dependencies | Result |
|---|---|---|---|
| 1 | `cargo deny check` | `ureq 3` (`default-features=false`, `native-tls`) + `notify-rust =4.19.0` | **FAIL** (licenses): `webpki-root-certs` is CDLA-Permissive-2.0, not allowed |
| 2 | `cargo deny check` | `ureq 2.12.1` (`default-features=false`, `native-tls`) + `native-tls 0.2` + `notify-rust =4.19.0` | **PASS**: `advisories ok, bans ok, licenses ok, sources ok`. Warnings only: a duplicate `syn` and unused/unmatched allow entries. Rerun before writing: same result |
| 3 | `cargo tree --target <t>` for the four deny targets | as row 2 | Windows uses schannel, windows 0.62.2 and tauri-winrt-notification 0.8.1. macOS uses security-framework 3.7.0 and mac-notification-sys 0.6.15. `openssl` appears only for non-target Unix, `zbus` only for Linux |
| 4 | `cargo check` (host x86_64-pc-windows-msvc) | as row 2 | **PASS** |
| 5 | `cargo check --target aarch64-pc-windows-msvc` | as row 2 | **PASS** |
| 6 | `cargo check -p ureq --target aarch64-apple-darwin` and `--target x86_64-apple-darwin` | ureq + native-tls | **PASS** (cross-checked from Windows) |
| 7 | `cargo check --target aarch64-apple-darwin` | full set | **BLOCKED** on a Windows host: the `mac-notification-sys` build script needs an Apple `cc` for Objective-C. Expected: CI builds macOS targets on `macos-latest` (`.github/workflows/ci.yml` build-targets). Native macOS build **NOT RUN** in this packet |
| 8 | Comparison of the scratch `Cargo.lock` with the workspace `Cargo.lock` (name + version) | as row 2 | New: `ureq 2.12.1`, `native-tls 0.2.18`, `schannel 0.1.30`, `security-framework 3.7.0`, `security-framework-sys 2.17.0`, `uuid 1.29.3` (patch bump), and the non-target Unix entries `openssl*`, `foreign-types*` and `vcpkg`. `notify-rust 4.19.0`, `tauri-winrt-notification 0.8.1` and `mac-notification-sys 0.6.15` are already locked through `gpui_linux` |

No network call other than crates.io index and download traffic was made.
There was no GitHub API call and no Dropbox call.

### Repository verification

| Command (worktree root) | Result |
|---|---|
| `cargo xtask verify --tier offline` | **PASS**: fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0` |

## E-06 dependency justifications

These dependencies are proposed for the implementing packets. None was added
to the workspace by this packet.

| Crate | Licence | Why needed | Maintenance | deny |
|---|---|---|---|---|
| `notify-rust =4.19.0` | MIT OR Apache-2.0 | OS notifications on Windows (WinRT toast) and macOS behind one API (P4-2) | Active 4.x line; already a GPUI Linux dependency | PASS |
| `ureq =2.12.1` (no default features, `native-tls`) | MIT OR Apache-2.0 | One small blocking HTTPS GET per day (P4-1) without an async runtime | Maintenance mode upstream (3.x current). Q-61 | PASS |
| `native-tls =0.2.18` | MIT OR Apache-2.0 | OS TLS (SChannel, Security.framework); no C toolchain, ring, aws-lc or bundled roots | Mature, widely used | PASS |

## Limits and open issues

- **Native verification required** (Q-57, Q-58) for:
  - Windows toast delivery with an HKCU AUMID;
  - the macOS `NSUserNotificationCenter` delivery;
  - click-through behaviour.
- **macOS build.** A full build with `mac-notification-sys` cannot be checked
  from a Windows host. It needs the macOS CI runners or the owner's Mac.
- **Parallel design dependencies.**
  - The settings master interface (`MasterState`) assumes settings.md
    terminology from R4-settings-design.
  - The decision kinds extend removal.md §3.3, which R4-removal-design is
    revising. The orchestrator should reconcile names on merge.
- **Design index.** `docs/design/README.md` is not an owned path, so it does
  not link `ops.md` yet. The orchestrator may add the link on merge.
- **No A-38 negative controls.** No behaviour code changed in this packet.
  The controls are planned per test in ops.md §7.
- **Open questions:** Q-54 to Q-61 in
  [docs/design/questions.md](../../../design/questions.md).
