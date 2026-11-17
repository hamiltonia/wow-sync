# UI design: interaction rules

Part of the [UI design](README.md). Covers the keyboard map, focus order per
screen, AccessKit names and roles, and notifications (R-13, A-26, A-27).
"Mod" means Ctrl on Windows and Cmd on macOS.

## Keyboard map

Every action has a keyboard path; shortcuts are accelerators, not the only
path (R-13).

### Global

| Keys | Action |
|---|---|
| Tab / Shift+Tab | Next / previous focus stop |
| F6 / Shift+F6 | Next / previous region: sidebar → header → banner (if any) → content |
| Mod+1 … Mod+6 | Overview, Computers, Add-ons, Needs attention, Activity, Settings sync |
| Mod+, | App settings |
| F5 or Mod+R | Sync now (ignored while paused, during sync or before setup; the button's disabled reason is announced) |
| Mod+Shift+P | Pause syncing / Resume syncing |
| Mod+W | Close (hide) the window, as the X button |
| Mod+Q | Quit WoW Sync (sheet if an update is being applied) |
| Esc | Dismiss an open sheet (its non-default button); otherwise on a detail page go Back; otherwise nothing |
| Alt+Left (Windows) / Mod+[ (macOS) | Back from a detail page |

Shortcuts are GPUI `actions!` bound with `cx.bind_keys`, like the slice's
`FocusNextStop`/`FocusPrevStop`. Mod+number and F6 must not fire while a
sheet is open.

### Inside controls

| Control | Keys |
|---|---|
| Button, link, list row | Enter or Space activates |
| Checkbox, switch | Space toggles (Enter also toggles) |
| Sidebar | Up/Down move between items, Home/End first/last, Enter or Space opens the page |
| List (Add-ons, Computers, Activity, Needs attention) | One tab stop. Up/Down move the focused row, Home/End, PageUp/PageDown by a visible page, Enter opens the detail; Right/Left expand/collapse an Activity batch |
| Filter buttons, theme choice | One tab stop. Left/Right move, Space/Enter selects |
| Details disclosure | Enter/Space expands or collapses |
| Sheet | Focus starts on the default button; Tab cycles inside the sheet only; Esc chooses the non-default button |
| Setup wizard | Enter on the last control of a step activates Continue when enabled; Alt+Left goes Back |

"One tab stop" lists use a roving focus: only the focused row has a tab index.
GPUI has no stock roving list, so the prototype implements it with focus
handles and Up/Down actions. If this proves impractical, every row becomes a
tab stop (the R2-pin spike did this) and the deviation is recorded (Q-32).

### Focus rules

- Opening a page moves focus to that page's first focus stop listed below and
  sets the window title to "<Page> · WoW Sync" (`window.set_window_title`), so
  screen readers announce the page change.
- Back returns focus to the row that opened the detail.
- A list keeps its focused row when its data refreshes; if that row is gone,
  focus moves to the nearest following row.
- Data refreshes never steal focus.
- The window opened from the tray restores the last page and its focus.
- A sheet traps focus until dismissed, then returns it to where it was.

## Focus order per screen

Header and sidebar are shared: sidebar items (one stop), then header **Sync
now**, **Pause/Resume**, then any banner button, then the content. F6 jumps
between those regions. Below is the content order; "→ list" is one roving stop.

| Screen | Initial focus | Content focus order |
|---|---|---|
| Overview | first Needs attention button, else **Add-ons** row of This computer | Needs attention item buttons → "See all" → This computer rows that are links (Add-ons, Shared folder) → Recent changes → list → "See all activity" → Other computers → list |
| Computers | the list | → list (rows) |
| Computer detail | Back | Back → unconfirmed add-on links → Details disclosure → (future) Retire |
| Add-ons | the filter group | filter group → list |
| Add-on detail | Back | Back → Next-step button (if any) → computer lines that are links → Recovery copies (Restore, disabled but focusable with its reason) → Details disclosure |
| Needs attention | the list | → list (each row's button is the row action) |
| Decision | first **Keep this copy** | candidate cards in order (each: Keep this copy, then its Details) → Decide later → Back |
| Needs-help detail | first choice button | choices → Details → Back |
| Activity | the filter group | filter group → list |
| Settings sync | page banner text (read-only) | disabled controls remain focusable so their "Coming later" reason is reachable |
| App settings | Start at sign-in | Start at sign-in → Start hidden → Pause/Resume → Sync now → Theme group → Summary toggle → Needs-me toggle → Explain close again → Crash reports (disabled) → What is sent → Open shared folder → Open data folder → Export diagnostics (disabled) → Quit |
| About | Check for updates | Check for updates → Open download page (when shown) → What's new → Copy version info → Install guide link |
| Setup step | the first unticked checkbox, else Continue | step controls top to bottom → Back → Continue / Set up |
| Reconfirm | identity checkbox | identity checkbox → scope checkbox → Confirm and continue → Not now → Details |
| Sheet | default button | default button → other button |

Disabled controls stay focusable when their reason matters (Settings sync,
Restore, Export diagnostics) and announce it in their accessible name: "Restore,
coming later".

## Accessible names and roles

AccessKit is exposed through GPUI's `.role()`, `.aria_label()`,
`.aria_level()`, `.aria_toggled()`, `.aria_position_in_set()` and
`.aria_size_of_set()`, all used by the slice or the R2-pin spike. R2-pin
verified through UIA: buttons (Invoke), checkboxes (Toggle), lists and list
items, focus following Tab; heading levels are not a UIA control type and list
items exposed no selection pattern. Attributes **not** yet demonstrated at the
pin (selected, expanded, live region, description, disabled) are encoded into
the accessible name when GPUI lacks them; the prototype records which ones
exist. Narrator support is a nice-to-have (D-18); unverified items become
listed limitations.

| Element | Role | Accessible name pattern |
|---|---|---|
| Main window | Window | "<Page> · WoW Sync" |
| Header | Banner (fallback Group) | "Status" |
| Headline | Status (fallback Paragraph) | "<headline sentence>. <context line>" e.g. "Waiting, WoW is running. World of Warcraft: Forever (wowf), Main add-ons group, this computer Desk PC" |
| Sync now / Pause | Button | "Sync now"; "Sync now, unavailable while paused"; "Pause syncing" / "Resume syncing" |
| Sidebar | Navigation (fallback List) | "Sections" |
| Sidebar item | Tab in a TabList if `selected` exists, else Button | "Add-ons"; "Needs attention, 2 items"; "Settings sync, coming later"; current page adds ", current page" |
| Page title | Heading, level 1 | page name |
| Panel title | Heading, level 2 | "This computer", "Other computers" |
| Banner | Alert for error/needs-help, Status otherwise | full banner sentence |
| Key/value row | Group with a Label child | "Last shared, 12 minutes ago, 21:02 on 16 November" |
| List | List | "Add-ons, 45 items, filter Not up to date" |
| List row | ListItem (+ position/size in set) | "<name>, <status label>, version <v>, confirmed by <names or not confirmed yet>" e.g. "WeakAuras, needs your decision, version 5.18.1, not confirmed yet" |
| Computer row | ListItem | "Laptop, report received 5 minutes ago, confirmed 42 of 42 current add-ons" / "Den PC, not heard from for 12 days, last confirmed 40 of 42 add-ons; what it has now is not known" |
| Status chip | part of the row name, not a separate stop | — |
| Filter buttons | Button with `aria_toggled` (RadioButton if mapped) | "Not up to date, 3 add-ons, selected" |
| Theme choice | RadioGroup / RadioButton (fallback toggle buttons) | "Theme: Follow system, selected" |
| Toggle settings | Switch (fallback CheckBox) with `aria_toggled` | "Start WoW Sync when I sign in" |
| Consent checkboxes | CheckBox with `aria_toggled` | the full sentence, as in the slice |
| Details disclosure | Button | "Details, collapsed" / "Details, expanded" |
| Progress bar | ProgressIndicator | "Syncing" (indeterminate) or "Applying 2 of 5" |
| Sheet | Dialog (fallback Group) | its message |
| Icons | not exposed (decorative); meaning is in the name | — |

Rules: no accessible name contains an internal ID, hash or folder key, except
inside an expanded Details area; times use words ("5 minutes ago") and include
the absolute local time; every name for a status includes the status word, not
only an icon.

## Notifications

GPUI does not post OS notifications; the platform mechanism (Windows toast,
macOS user notifications) needs a dependency decision (Q-30). Until then the
prototype shows a **notification preview** list inside the app (in a developer
area of the Activity page) with what would be sent and when. The in-app status
(header, tray icon, tooltip, Needs attention) always updates immediately,
whatever happens to notifications (A-27: actionable status remains available).

### Kinds

| Kind | When | Example text | Urgency |
|---|---|---|---|
| Batch summary | A completed sync batch applied at least one change here (UG-19) | "3 add-ons updated from Laptop: Deadly Boss Mods, WeakAuras and 1 more." | nonurgent |
| Action needed | A new item appears in Needs attention, or its evidence changes | "WeakAuras needs your decision." / "2 add-ons need your decision." | nonurgent |
| Data at risk | Local state unreadable, or (future) low storage that threatens recovery copies (R-10) | "WoW Sync can't keep recovery copies: the disk is almost full." | urgent |
| App update | A newer app version is found (A-28) | "WoW Sync 0.2.0 is available. Open WoW Sync to download it." | nonurgent |

No notification for: sharing this computer's state, a batch with no change
here, waiting conditions, another computer being silent, pause/resume (the user
did it).

### Grouping

- **One summary per completed batch** (R-13). A batch is one sync cycle that
  applied changes; its items are listed by name, at most three, then "and N
  more".
- **Actionable problems are separate** from summaries (R-13). Several new
  action items found in the same batch become one notification with a count.
- Clicking a summary opens Activity at that batch; clicking an action item
  opens its page (or Needs attention for a group).

### Deferral while gaming

- While the game is running, or its state is unknown (`PlanBlock::ActivityUnknown`;
  treat "can't tell" as "maybe playing"), **nonurgent notifications are held**
  (R-13). Urgent ones are sent once.
- When the game is observed not running, held notifications are released as at
  most **two**: one combined summary ("While you played: 5 add-ons updated from
  Laptop and Den PC") and one action notification for items still open ("1
  add-on needs your decision"). Items resolved meanwhile are dropped.
- The held set is bounded (counts plus at most five names per kind), so a long
  session cannot grow memory (R-14).
- OS do-not-disturb and focus modes are respected by the OS; the app does not
  bypass them.

### No repeats for unchanged conditions

- Each condition has a key: kind + scope (add-on, computer, "shared folder",
  "update <version>").
- A key notifies when it first appears or when its content changes (for
  example a new candidate copy for the same decision). An unchanged condition is
  never notified again while it persists.
- After a condition clears, a recurrence of the same key notifies again at most
  once every 6 hours, so a flapping Dropbox connection cannot spam.
- Keys are kept in memory and in local state across restarts, bounded to the
  current open conditions plus the last 100 cleared keys.

### Settings

Two toggles in App settings: **Show a summary after each sync** and **Tell me
when something needs me**. The urgent kind cannot be turned off in the app;
the OS can. Gaming deferral is not optional because R-13 requires it.
