# R2-slice evidence

Worker R2-slice, branch `run002/R2-slice`, Windows x64 (local). Synthetic installs under
`%TEMP%` only (all carry `.wowsync-synthetic`); provider = a temp folder. No real WoW or
Dropbox data touched. All started processes were stopped by PID.

## Results summary

| Check | Result |
|---|---|
| `cargo xtask verify --tier offline` (fmt, clippy -D warnings, test, required-cases, deny, markdown-links) | PASS |
| `cargo test --workspace` | PASS (runtime 16, others unchanged) |
| G1, G2, G3, G4, G5, G6, G7, G8, G9 | PASS (details below) |
| Memory (PLAN 6.6), idle, window hidden | 12.4 MiB max private working set (gate 100) |
| ARM64 build, macOS build/run, Linux | NOT RUN |

## Slice cases (runtime e2e, `cargo test -p wow-sync-runtime`, real adapters, temp dirs)

Registered in `tests/required-cases.toml` (tier offline unless noted).

| Case | Test(s) |
|---|---|
| A-06:slice | `two_installs_publish_and_apply_through_temp_provider`, `independent_updates_converge` |
| A-07:slice | `peer_confirmation_only_after_apply` |
| A-08:slice | `ambiguous_addon_stops_only_that_addon` |
| A-09:slice | `install_without_marker_refused`, `join_refused_for_other_product`, `unfamiliar_variant_blocked_until_confirmed` |
| A-21:slice | `game_running_on_target_blocks_apply_then_resumes` |
| A-21:slice-native (tier native, Windows) | `fake_client_blocks_real_apply` |
| A-46:slice | `two_package_crash_at_every_effect_recovers_with_real_adapters`, `crash_mid_apply_recovers_on_restart`, `crash_during_recovery_with_real_adapters_is_reentrant` |
| A-22:slice | `destination_updated_during_staging_is_not_replaced`, `publish_aborts_when_source_changes` |
| A-20:slice | `partial_suite_on_target_is_not_mixed` |
| A-24:slice | `empty_peer_does_not_receive_partial_suite` |

All 16 PASS. The crash-at-every-effect loop reached k>=6 effects; the recovery-crash loop
reached j>=1. A-21:slice-native runs only on Windows (stand-in client is a copy of PING.EXE
named `Wow.exe`).

## G1-G9 (GUI, UIA-driven on Windows x64, release build)

Harness: `evidence/R2-slice/gkit.ps1` (UIA + screenshots). Screenshots `G*.png` alongside (G6 screenshot removed: it showed a real user-profile path).

| G | Result | Observed |
|---|---|---|
| G1 | PASS | Recognized Forever: identity line, folder, scope text (add-on code only; settings excluded). Provider had no `WoWSync` folder before consent; "Create" without the agreement box was refused with a note and wrote nothing. After consent, group created and published. Identity wording is core's ("Forever - product wowf in folder _forever_"), not "World of Warcraft: Forever (wowf)". |
| G2 | PASS | `_test_`/`wow_test`: "Unfamiliar game version"; two checkboxes; scope alone refused ("Agreeing to the scope alone is not enough"); nothing written; both boxes then enrolled. |
| G3 | PASS | Computer B joined and applied: "alpha: updated and verified on this computer", "Recovery: 1 recovery point(s) kept". File present on disk. |
| G4 | PASS | Stand-in `Wow.exe` running: headline "Waiting", "World of Warcraft is running; nothing was changed", file unchanged (a1). After stopping it, applied a2. |
| G5 | PASS | Same label, different content: "cannot tell which version is newer: alpha (3.0 vs 3.0: same label, different content)" on both; both files unchanged. |
| G6 | PASS | Provider path replaced by a file: "Something needs attention" with "shared folder problem: NotPlainFile at ...: not a directory". Cosmetic gap: the group/computer line renders empty (" on ()") when the provider is unreachable. |
| G7 | PASS | A after publish: "Published ... ago", add-on line "up to date", no confirmation. After B applied: "alpha: applied and reported by computer(s) Desk-B". |
| G8 | PASS | `--crash-after 2` on B: headline "Waiting", "WoW started while alpha was being updated; it will be restored ...", "1 interrupted update(s)". "Sync now" recovered: "rolled back", 2 recovery points, 0 interrupted. |
| G9 | PASS | Initial focus on first control; Tab / Shift+Tab move between control-typed UIA elements with a visible yellow focus ring (`G9-focus.png`); Space toggles the checkbox, Enter activates; close (WM_CLOSE) hides to tray, process stays alive; second launch signals the first and re-shows the window. |

Note: reading the UIA tree right after launch of a second window occasionally returned empty
once; a re-read after a short wait worked (harness timing, not a product failure).

## Memory (PLAN 6.6)

Method: `spikes/gpui-pin/measure.ps1`, sum of Working Set - Private over the PID and all
descendants, 1 Hz, 60 s, no trimming. Scenario: enrolled computer, window hidden
(`--start-hidden`), sync every 5 s, 1 add-on. One run (`idle-hidden-run1.csv`).

- Private working set sum: max 12.4 MiB, avg 11.7 MiB (gate 100 MiB).
- Total working set (not the gate): 45.3 MiB. 1 process (no children).
- Not measured: during an apply of large add-ons; multiple runs; ARM64.

## Launching developer mode

```
cargo build --release -p wow-sync-app
target\release\devkit.exe make-install <root> [--folder _forever_] [--product wowf]
target\release\devkit.exe add-addon <root> <variant-folder> <Name> <tag> [<version>] [--depends X]
target\release\wow-sync.exe --dev --install <root> --provider <existing dir> --state <dir> `
  [--name N] [--folder _forever_] [--start-hidden] [--interval-ms N] [--recent-ms N] `
  [--crash-after N] [--hide-after-secs N]
```

The install must carry the `.wowsync-synthetic` marker (made by `devkit make-install`).
Provider must be an existing directory. Provider/state default to temp dirs; there is no default
install. Without `--dev --install` the window only explains this. `WOWSYNC_LOG` names a log
file; `WOWSYNC_INSTANCE` allows parallel instances. `devkit fake-client <variant>` starts a
stand-in client but, when its output is captured by a parent, inherits handles and blocks the
parent; the harness uses `Start-Fake` in `gkit.ps1` instead.

## Gaps and candidates for core

- Product compare for joining a group (`rec.product != confirmed.product()`) lives in the
  runtime; candidate to move into core. The "publish needed" comparison (`same_folders`) is also
  in the runtime.
- If a crash happens between apply and publish, the apply report is lost; no report is published
  after a recovery.
- The guided digest is re-checked each cycle; adding an add-on to an unfamiliar enrolled variant
  makes the confirmation stale.
- Peers are rendered by id in core and mapped to display names in the runtime (`name_peers`);
  member lines show raw peer-clock epoch ms (core wording).
- No editable names (group name fixed "Add-on sync group"); no uninstall/rollback/retire GUI
  (outside this slice).
- Join screen has no way to pick among same-product groups beyond the listed buttons.
- Block wording is "World of Warcraft is running; nothing was changed" under headline "Waiting",
  not "Waiting - WoW is running".
- ARM64 link blocked locally (no ARM64 tools); macOS and Linux not run. The gpui git dependency
  is large (long first build, about 2 min release incremental); CI offline tier on macOS was not
  run here, so its practicality is unverified. No checks were weakened.
- deny.toml: adopted only the R2-pin proposal (BSD-2-Clause, Zlib, MPL-2.0, allow-git zed,
  `unmaintained = "workspace"`). Path deps carry `version = "0.0.0"` to satisfy the wildcard ban.
- No tray-menu Quit/Show or tray left-click driven here beyond what R2-pin covered; only
  window close/second-instance re-show were driven.
