# Evidence S-a11y: screen-reader feasibility with GPUI

Packet: [S-a11y](../packets/S-a11y.md) (implementation part). Authority: SPEC §7
accessibility, R-13, A-26, E-07. Feeds deferred decision D-12. Worker S-a11y,
branch `run001/S-a11y`. Environment for every runtime result below: Windows 11
x64 dev PC (Insider build 26300; not official support evidence), Windows
PowerShell 5.1 `System.Windows.Automation` client running outside the target
process, Rust 1.100.0 (root `rust-toolchain.toml`), MSVC 17.14.

## Question and answer

SPEC §7 question: can the GPUI GUI be made usable with Windows UI Automation
(UIA) and screen readers?

- **Published GPUI (crates.io `gpui` 0.2.2): not feasible as is.** UIA exposes
  one empty top-level Window and nothing inside it. Confidence: high (measured).
- **GPUI from Zed `main` (git, AccessKit integration): feasible in principle,
  UIA side measured PASS.** Controls, names, states, focus tracking and
  Invoke/Toggle/Value patterns appear to a UIA client once the app annotates its
  elements. Confidence: medium-high for UIA; **unknown for actual Narrator
  speech** (NOT RUN; an agent cannot hear). Not yet released on crates.io, so
  it needs a git dependency pin, and upstream still has open accessibility bugs.
- What would change this: Narrator/NVDA manual results (script below), a
  published GPUI release containing AccessKit, or the open upstream bugs
  listed below being fixed or proving blocking.

## Spike code (rerun)

| Path | Purpose |
|------|---------|
| `spikes/a11y/` | Window on published `gpui = "=0.2.2"` (same pin as S-gpui). Heading, text field, Save/Cancel buttons, toggle, 10-item list. No a11y annotations are possible in this version. |
| `spikes/a11y-main/` | Same UI on Zed `main` commit `24d37721115bd410b8f73998f8da5c4598b64a03` (2026-11-15) with `role`, `aria_label`, `aria_value`, `aria_toggled`, `aria_level`, `aria_position_in_set` builders. Needs the `gpui_platform` crate for `application()`. |
| `spikes/a11y/uia-dump.ps1` | Launches a target, dumps the raw UIA tree, sends Tab 6 times and records `AutomationElement.FocusedElement`; screenshots per step. `-Target gpui\|gpui-main\|baseline`. |
| `spikes/a11y/uia-actions.ps1` | Exercises UIA SetFocus, Value, Invoke, Toggle on the `gpui-main` build. |
| `spikes/a11y/kbd-test.ps1` | Keyboard-only run of the 0.2.2 build with screenshots. |
| `spikes/a11y/baseline-form.ps1` | Baseline: standard Win32 controls via WinForms (a message box has no equivalent controls). |

Build and run, from `D:\GitHub\wow-sync.worktrees\S-a11y\spikes\a11y` (and
`..\a11y-main`): `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo build`,
then `powershell -NoProfile -File uia-dump.ps1 -Target gpui -OutDir <dir>`.
Scripts use `SendKeys`, so the desktop must be unlocked and the spike window
foreground; one run initially failed with "Access is denied" because the
interactive desktop was not available, and was repeated (data below is from the
successful repeats).

## UIA results

Raw dumps: [uia-gpui.txt](S-a11y/uia-gpui.txt),
[uia-gpui-main.txt](S-a11y/uia-gpui-main.txt),
[uia-baseline.txt](S-a11y/uia-baseline.txt),
[uia-actions-gpui-main.txt](S-a11y/uia-actions-gpui-main.txt).

### gpui 0.2.2 (`spikes/a11y`) - **FAIL** (runtime, Windows 11 x64)

- Tree: one element, `Window` named from the title bar (`A11y Spike`), class
  `Zed::Window`, patterns Window and Transform. Depth 0; **0 descendants**.
  Heading, field, buttons, toggle and list are invisible to UIA.
- `FocusedElement` stays the Window after every Tab even though the app visibly
  moves its focus ring; no focus events a screen reader could follow.
- Source check: the downloaded `gpui-0.2.2` source has no `accesskit`,
  `WM_GETOBJECT` or UIA provider references; Cargo.toml has no accessibility
  dependency. The title-bar name is the only text exposed (from the OS window
  caption).

### gpui from Zed main (`spikes/a11y-main`) - **PASS for UIA exposure**

Tree (16 descendants, depth 2):

| Element | UIA type | Name | Patterns | Focusable |
|---------|----------|------|----------|-----------|
| heading | Text | `A11y Spike Heading` | none (no level/heading type surfaced by this client) | no |
| text field | Edit | `Name` | Value | yes |
| Save / Cancel | Button | `Save (0)` / `Cancel (0)` | Invoke | yes |
| toggle | CheckBox | `Enable sync` | Toggle | yes |
| list | List | `Items` | none | no |
| 10 rows | ListItem | `List item N` | none | yes |

- Tab order and UIA focus (PASS): Edit `Name` -> `Save (0)` -> `Cancel (0)` ->
  CheckBox `Enable sync` -> ListItem 1 -> 2 -> 3, matching the visual focus ring.
- Patterns (PASS unless noted): `SetFocus` moves focus; Value read returns what
  was typed (`hello`); Invoke on Save incremented the click count (name changed
  to `Save (1)`); Toggle flipped Off to On. **Limitation:** `ValuePattern.SetValue`
  returned success but changed nothing (the spike did not implement the
  set-value action).   The list exposes no Selection/SelectionItem patterns, and the dump shows no position-in-set (not queried separately).
- These names exist only because the spike annotated each element. Un-annotated
  GPUI elements are still invisible; every control and every live status
  message would need explicit roles and labels (E-07 cost).
- Baseline run on an unannotated build was not made; the 0.2.2 run is the
  un-annotated control.

### Baseline Win32 controls (`baseline-form.ps1`) - partial

Names and Tab order are exposed (heading text, `Save`, `Cancel`, `Enable sync`;
Tab cycles Edit, Save, Cancel, CheckBox, ListBox and wraps). The harness reported
every child as control type `Pane` with no patterns and `IsKeyboardFocusable`
false, which is not what Narrator would see for real Win32/WinForms controls
(they normally report Edit/Button/CheckBox/List with patterns). Treat the
baseline's control types as inconclusive (method limit: Windows PowerShell 5.1
client on an Insider build); its use is only to show names and focus are
reported for native controls where GPUI 0.2.2 reports none. The first Tab
reading in that file shows the desktop (`#32769`) because focus had not yet
moved to the form.

## Keyboard-only operation and focus visibility (0.2.2 and main identical)

Evidence: [kbd screenshots](S-a11y/kbd-1-type.png) and `gpui*-tab*.png` in
`docs/runs/run-001/evidence/S-a11y/`. Runtime, Windows 11 x64, `SendKeys`.

- **Tab/Shift+Tab do nothing by default.** `Window::focus_next`/`focus_prev`
  exist but GPUI binds no key to them; the spike had to add a `tab` and
  `shift-tab` keybinding to an action (first attempt without it left focus on
  the field: PASS only after the binding). Product code must provide this.
- Once bound, Tab order follows `tab_index`, wraps, and Shift+Tab reverses.
  Focus visibility is **only** what the app draws: the spike draws a 2 px yellow
  border (visible in the screenshots); GPUI draws no default focus indicator.
- `on_click` handlers fire on Enter and Space for the focused element (a
  separate key handler doubled the count in a first run). So keyboard activation
  of buttons/toggle works without extra code. PASS.
- Text input: typing printable keys and Backspace works in a hand-written field
  (`key_char`); the spike's field is a plain `div` with no IME, selection,
  caret, clipboard or composition. A production text field must be built or
  taken from a component crate (not evaluated here).
- Not tested: arrow-key list navigation, menus/dialogs, tray menu, high-contrast
  themes, system text scaling, reduce-motion.

## Upstream status (public sources, fetched 2026-11-15)

- Published GPUI: crates.io `gpui` 0.2.2, created 2025-10-22 (latest at
  fetch). Source inspected locally: no AccessKit.
- Zed `main` HEAD `24d37721115bd410b8f73998f8da5c4598b64a03`
  (2026-11-15): `crates/gpui/Cargo.toml` and `crates/gpui_windows/Cargo.toml`
  depend on `accesskit` / `accesskit_windows`; element builders include `role`,
  `aria_label`, `aria_value`, `aria_toggled`, `aria_level`, `aria_selected`,
  `aria_expanded`, `accessibility_id`, `aria_position_in_set`, etc.
- Merged PR "gpui: Accesskit support"
  <https://github.com/zed-industries/zed/pull/56065> (merged 2026-05-27, scoped
  to GPUI only; "first step to addressing" issue
  <https://github.com/zed-industries/zed/issues/41138>, still open: "Windows:
  Screen reader accessibility missing completely", opened 2025-10-24).
- Follow-ups merged: <https://github.com/zed-industries/zed/pull/61926>
  (accessibility identifiers, 2026-07),
  <https://github.com/zed-industries/zed/pull/60397> (landmarks and menu
  improvements), <https://github.com/zed-industries/zed/pull/57954>
  (`Application::inaccessible()`).
- Open problems (relevant to a long-lived Windows app): issue 66690 "views
  drawn from the cache drop their accessibility nodes, and focus falls back to
  the window" <https://github.com/zed-industries/zed/issues/66690>; 66730
  "elements clipped out of view get accessibility nodes reported as visible"
  <https://github.com/zed-industries/zed/issues/66730>; 65623 "Window::transact
  does not roll back a11y nodes ... Duplicate a11y node id" (List autoscroll)
  <https://github.com/zed-industries/zed/issues/65623>; PR 66631 "gpui_windows:
  Report window focus to AccessKit when activated without WM_ACTIVATE" (open)
  <https://github.com/zed-industries/zed/pull/66631>; discussion 66758 notes no
  `disabled`/`read_only` builders yet
  <https://github.com/zed-industries/zed/discussions/66758> (PR 64214 for
  `aria_current` was closed over an unsigned CLA). macOS has its own open
  focus bug 66691 (macOS runtime BLOCKED here).
- No published release or timeline for AccessKit in crates.io GPUI was found.

## Narrator manual check (owner; NOT RUN by an agent)

Agents cannot hear speech; these steps record what a human must verify on the
`spikes/a11y-main` build (and, for contrast, `spikes/a11y`). Windows 11 x64.

1. Build: `cargo build` in `spikes\a11y-main` and `spikes\a11y`.
2. Press Win+Ctrl+Enter to start Narrator; wait for it to be ready.
3. Run `spikes\a11y-main\target\debug\spike-a11y-main.exe`. Expected on window
   activation: window name `A11y Spike`, and the focused control, e.g.
   "Name, edit". Record the exact words.
4. Press Tab repeatedly. Expected: "Save (0), button", "Cancel (0), button",
   "Enable sync, check box, not checked", "List item 1" ... each with role.
5. On a button press Enter or Space: expected activation and, ideally, an
   announcement of the changed name ("Save (1)"). Press Space on the check box:
   expected "checked". Record whether state changes are spoken.
6. Type text in the field: expected character echo and, after leaving it, the
   value. Record what is spoken (a custom field may be silent).
7. Use Narrator scan mode (Caps Lock+Space off/on) and Caps+Left/Right: expected
   to reach heading, edit, buttons, check box, list and 10 items; record gaps.
8. Repeat with `spikes\a11y\target\debug\spike-a11y.exe`: expected silence
   beyond the window title (nothing inside).
9. Optional: repeat with NVDA and Windows Magnifier; record versions.
10. Record Pass/Fail per step, Narrator version, Windows build, and any
    interaction that required the mouse.

## Risks, limits, options for design (S-a11y-design / D-12)

- UIA exposure on Zed `main` is real but depends on an unreleased git pin and
  open correctness bugs (cached views, clipped nodes, list autoscroll), which
  fit a status-heavy sync GUI. Cost: annotate every control and every changing
  status message; build or adopt accessible text input; draw focus indicators;
  bind Tab/Shift+Tab; implement set-value and selection actions as needed.
- Method limits: one machine, one Windows build, Windows PowerShell 5.1 UIA
  client (not Accessibility Insights or Narrator itself); baseline control
  types were inconclusive; no Narrator/NVDA speech, no high-contrast, no text
  scaling, no IME; main-branch behaviour may change daily; macOS BLOCKED.
- Options (not decided here): pin Zed main at a reviewed commit and annotate
  everything; contribute upstream fixes; a parallel native shell for key flows;
  a documented limitation per D-12. Owner/design decision required.
- Tools installed: none beyond Cargo git dependencies (zed repository clone
  into the Cargo git cache).
