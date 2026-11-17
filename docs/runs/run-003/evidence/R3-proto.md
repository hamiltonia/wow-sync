# R3-proto evidence (run-003)

Worktree `D:\GitHub\wow-sync.worktrees\R3-proto`, branch `run003/R3-proto`.
Environment: Windows (local desktop session), release build of `wow-sync-proto`,
GPUI pin unchanged (Zed v1.28.1). Fake in-memory data only; nothing is read from
or written to WoW, Dropbox or settings files.

## Built vs brief
- Pages P1-P14 and scenarios S1-S16 are built (see `crates/wow-sync-proto/src`).
- Switcher, light/dark/system theme, simulated zoom (`--zoom 1.5`), keyboard
  navigation and focus, accessible names, tray icon/menu/tooltip, notification
  preview (Activity page, nothing is posted to the OS), first-close sheet.
- Launcher `D:\WoWSyncDemo\Start-Prototype.cmd`; exe in `D:\WoWSyncDemo\bin\`.

## Commands and results
| Check | Command | Result |
|---|---|---|
| Unit tests | `cargo test -p wow-sync-proto --offline` | PASS, 10 tests |
| Negative control (A-38) | mutated `"Paused"` headline in `model.rs` | FAIL as expected: `every_scenario_builds_with_expected_headline` (S10), exit 101; restored, PASS |
| Tab traversal S7 | `walk/drive.ps1` posts Tab x1/2/4/7 | PASS: focus moves through Overview need buttons into the Activity list |
| Sheet focus trap | `--sheet first-close`, 5 Tabs | PASS: focus stays inside the sheet |
| Roving list S14 | Tab then Down/End (`walk/rove-S14.uia.txt`) | PARTIAL: Right on a filter moved focus to "Needs you" (earlier run); list Down/End not re-verified, focus ended on "Sync now" |
| UIA tree | `walk/uia-S7-*.uia.txt` | PASS: names/roles exposed (list, navigation, buttons with state in the name) |
| Screenshots | `capture.ps1`, `shots/` (about 0.88 MB) | PASS, own window only |
| Memory | working set | 62-66 MiB (info only) |
| Ctrl shortcuts, Shift+Tab | posted messages cannot send modifiers | NOT RUN |
| `cargo xtask verify --tier offline` | worktree root | see final report |

## Feasibility
- Q-30 notification preview: in-app preview built; OS toast not posted. Recorded as preview only.
- Q-31 text input: single-line name field works with typing/backspace; no IME, no selection (limitation).
- Q-32 list/roving focus: S14 (300 add-ons) scrolls; roving works with arrows (see above); one row exposes list-item names.
- Q-33 sheets: modal sheet with focus trap and Esc works.
- Q-35 live theme: GPUI at the pin handles WM_SETTINGCHANGE "ImmersiveColorSet" (`gpui_windows/src/events.rs:1240`) and the app logs "appearance changed: system=Dark" at startup. A live toggle at runtime was NOT RUN.
- Q-39 tray click: tray icon/menu in scope; click-to-restore and hide/show are logged. Windows shutdown/logoff behaviour NOT RUN.

## AccessKit at the pin
role, label (name), level, toggled, expanded, position-in-set are available on
`Stateful<Div>` only (needs `.id()`). No `aria_disabled`, live region or busy
state, so state is written into the accessible name.

## Owner checklist (self-checked, 27 items)
Self-check was by screenshots and UIA only, not by an owner. Items needing Ctrl or
Shift keys, live OS theme change, OS DPI scaling, IME and toast delivery are
NOT RUN; the rest were observed to behave as the brief describes.

## Design disagreements and limits
- Fake consent rule instead of core `consent_gap`.
- S12 headline unspecified; chose "Confirm the game version to continue syncing".
- S6: Unavailable takes precedence over Unstable in the headline.
- Computers "42 of 42" example vs derived counts (S3 Laptop shows 40 of 42).
- First-close sheet: Esc picks Hide (safe option), not Quit.
- Zoom is simulated, not OS DPI. Rail shows initial letters, not icons.
- Single-instance guard skipped. Shortcuts use Ctrl on Windows.

## Process note
One Set-Content slip occurred while editing `pages.rs`/`setup.rs`; the result was
corrected with the edit tool and verified by build and tests.
