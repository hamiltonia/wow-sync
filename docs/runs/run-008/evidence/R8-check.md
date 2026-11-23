# R8-check: independent Windows verification

## Verdict

**FAIL: G2 is major and remains open; G1 is minor.** Read-only setup
inspection, cancellation, progress, logging, both verification tiers and the
dispatch artifact smoke test pass. Quit during an enrolled write from the setup
wizard does **not** exit after the write finishes.

Authority: [PLAN.md](../PLAN.md), [packet](../packets/R8-check.md), D-30;
SPEC R-01/R-03/R-13, ACCEPTANCE A-01/A-37/A-39/A-42/A-47. No product code or
test assertions were changed; implementation/reviewer claims were not used as
test evidence.

## Environment and build

- Windows 11 x64 dev PC, interactive desktop, system dark theme, 150% display
  scaling. UI driven using `System.Windows.Automation` Invoke/Toggle/Value
  patterns; native folder selection used the dialog's `Folder:` edit and
  confirmation button. Tray icons were right-clicked at their UIA rectangles;
  the resulting menu's PID was checked before invoking any item.
- Worktree `D:\GitHub\wow-sync.worktrees\R8-check`, branch `run008/R8-check`;
  tested source `1d94a78838278bf285bdc9517f0b92b3280f8938`, version 0.3.1.
  Assigned/runtime worker model: `gpt-6.1-sol`.
- Build: set `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"`, then
  `cargo build --release -p wow-sync-app`. PASS, 127 s. Four existing
  release-only unused-import/dead-code warnings in `update_http.rs`; no build
  error.
- Every GUI process had a distinct fresh `--state
  D:\WoWSyncLab\R8-check\<n>` and `WOWSYNC_LOG` pointing to a lab log.
  Synthetic instances also explicitly used a local lab `--provider`; no
  default developer provider/state directory was used.

## Results

| Check | Result | Observed evidence / timing |
|---|---|---|
| 1. Original scenario and faithful stand-in | **PASS** | Release, **without `--dev`**, list-selected real `_forever_`: 5,607 ms; native **Choose folder…** selecting the real variant directory: 5,958 ms. Both reached “can be synced”, enabled Next, and logged start/done/duration, 102 eligible add-ons, 33,141 files. Setup was abandoned before enrollment. Synthetic 100 add-ons / 32,200 scanned files / 3,276,800,000 payload bytes completed in **123,539 ms**, with 100 eligible / 0 blocked. This reproduces a genuinely long scan, not the exact unknown RYZENFISH cause. |
| 2. Quit during check and write | **FAIL** | Tray Quit during live synthetic read-only checks exited in **217 ms** and **214 ms**, with `inspect cancelled`; no surviving lab process. The synthetic write remained whole, but the first tray Quit during it did **not** exit: **60,092 ms timeout**, still alive after application completed. See G2. |
| 3. Progress / Back / replacement / Cancel | **PASS** | Window capture showed “Checking add-ons… **435 files**”, Cancel check and disabled Next; a later capture showed **29,973 files** during the long scan. Back returned to step 2 within a 353 ms observation window and logged cancellation. Returning with Next started a fresh check. Picking another root cancelled the old check, completed the new two-file check in **4 ms**, and kept the old row unchecked; unchanged after another 7 s. Explicit Cancel check also left Check again available and Next disabled. |
| 4. Usable-first default | **FAIL** | The automatically discovered real usable install was selected first. However, picking a synthetic root containing refused `_classic_beta_` and usable `_forever_` selected and inspected the refused beta instead. The usable row remained unchecked and Next disabled. See G1. |
| 5. Logs and privacy | **PASS** | Observed `inspect start`, `inspect done … in N ms`, `inspect cancelled`, `tray: open`, and `tray: quit`. Ten lab logs: no `\Users\` path. All 39 new inspect/tray lines had no WTF/account/profile path markers, account/realm/character names, file names or settings/add-on contents; only target roots/variants, aggregate counts, durations and tray item names. This is actual lab-log evidence, not an arbitrary-install privacy claim. |
| 6. Regression / verification tiers | **PASS** | `cargo xtask verify --tier offline`: fmt, clippy, test, required-cases, deny, markdown-links all PASS. `cargo xtask verify --tier native`: PASS. Normal discovery found fixed drives C/D and the Battle.net-registered D-drive game root; real variant and synthetic root picker selection both worked. No alternate-drive/network-disconnection matrix was run. |
| 7. Dispatch artifact | **PASS** | `gh run watch 45285544216`, then `gh run download 45285544216 -D D:\WoWSyncLab\R8-check\artifact`: successful dispatch, head SHA matches tested source. ZIP SHA-256 matched the supplied checksum. Extracted `wow-sync.exe` opened setup, discovered/inspected the real variant read-only (**5,647 ms**), handled tray Open, then exited on tray Quit in **224 ms**. |

The polling loop's initial 100 s stand-in wait expired while progress continued;
this was **not** treated as a hang or a passing completion. The subsequent
matching `inspect done … in 123539 ms` and ready row proved completion.

### Artifact identity

ZIP `wow-sync-0.3.1-windows-x64.zip`:
`cf4ea685961cd3bfeb1abac8c645add4b216b73e17338273cc60ebbbb5d697c3`.

Extracted executable:
`ed6eb296608f138b17a005ff4c78cf8cb63700572c5415714402cca064e1ca32`.

## Findings

### G1 — minor: root folder picker defaults to a refused variant

1. Create a synthetic root with `.build.info` entries for `wowf` and
   `wow_classic_beta`, `_forever_\Interface\AddOns`, and
   `_forever_\.flavor.info` identifying `wowf`.
2. Also create `_classic_beta_\Interface\AddOns`, but omit its `.flavor.info`.
3. Start a fresh normal release instance with isolated state, reach step 3,
   invoke **Choose folder…**, and select this root (not a variant).
4. The beta row is selected. Its missing-flavor refusal is shown; the usable
   Forever row is not checked, and Next stays disabled.

Observed with PID 1252. Log starts inspection of
`D:\WoWSyncLab\R8-check\mixed\_classic_beta_`, completes in 1 ms, then reports
the missing-flavor refusal. This contradicts PLAN goal 4 for the picker path.
`setup::usable_first` is called by startup discovery, but
`setup::add_found_install` selects the first matching root row without using it.
An earlier fixture with beta's flavor metadata present was accepted as
unfamiliar, so it was **not** used as refusal evidence.

### G2 — major: Quit during setup's first write creates an invisible modal

1. Use two synthetic installs and a purely local provider. In the source,
   create one add-on containing a TOC and 6,000 synthetic 16 KiB files.
2. Through the GUI create a group and let the source publish; then join the
   same group from an empty target through the GUI.
3. At step 6 invoke **Set up this computer**. Watch target
   `_forever_\.wowsync\staging` to establish that an apply write has started.
4. Right-click the target's captured tray icon, verify the menu PID, and invoke
   **Quit WoW Sync** while staging still contains only part of the package.
5. The process stays alive after the whole apply completes. No Quit/Keep running
   sheet is visible in setup; **Finish and open WoW Sync** is also inert.

PID 31488: first staged file detected 84 ms after Set up. Immediately before
Quit, staging held **25 files**, active `WriteLab` held **0**. The log records
`tray: quit` and `window shown` at 23:29:24Z, but the process did not exit within
the measured **60,092 ms**. At 23:30:04Z it reported up-to-date. Independently
hashed all **6,001** active files: relative paths and SHA-256 values exactly
equal the source; no journal or staging files remained. Thus the write was
safe, but Quit was not completed.

UIA after completion exposed only the finished-setup view, with no
**Quit when safe** or **Keep running** control. Invoking Finish did nothing.
Targeted Escape later logged `act SheetKeepRunning`; only then did Finish work,
followed by a normal explicit quit. A separate first attempt, PID 8208, also
completed all files but was ultimately stopped by its captured PID for cleanup.

Code corroboration: `app::request_quit` sets `Sheet::Quit` when busy rather than
calling `begin_quit`; setup renders `setup_view` instead of the content view
that renders the sheet. The existing modal action guard then suppresses setup
actions. This violates PLAN goal 2 / A-01: a write may finish safely, but must
not strand the user's quit request behind a confirmation they cannot see.

## Safety, controls and cleanup

- Real `_forever_` was inspected only; no consent/enrollment, group publication
  or game-file write on real data. The detected Dropbox root was not enrolled
  or published to. Actual groups were exclusively local lab providers.
- No `D:\WoWSyncDemo` access, WoW execution, Battle.net automation/termination,
  imported Lua execution, saved-variable execution, or default-state mutation.
- One initial tray-identification probe opened the owner's menu; its PID did
  not match the captured lab PID, so **no item was invoked** and Escape dismissed
  the menu. All subsequent tray commands were PID-guarded.
- Captured application PIDs: **16480, 16160, 7484, 29668, 22516, 1252, 26732,
  8208, 31488, 11824**. All confirmed absent afterward. Only PID 8208 required
  `Stop-Process -Id 8208`; others exited through app quit. A command-line-scoped
  process check also found no remaining `R8-check` app instances.
- The two partially timed GUI-write attempts were not credited as successful
  Quit tests. The second established G2 with a live PID-verified tray command.
  Source/target full-file hash comparisons confirmed complete writes in both.
- No new test/mutation was added because the packet forbids product-code edits.
  Local tiers independently executed existing required cases; A-38 mutations
  were **NOT RUN** here, rather than claimed from reviewer evidence.
- Lab installs, providers, states, logs, scripts, screenshots and extracted
  artifact were removed after recording these results. The ZIP hash is retained
  above, not private data. No subst drive was created. No push or merge.
