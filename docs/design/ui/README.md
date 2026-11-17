# UI design: information architecture

Status: **run-003 design** by R3-ui-design
(packet [R3-ui-design](../../runs/run-003/packets/R3-ui-design.md)). It is the
input for the R3-proto clickable prototype and, after owner approval of that
prototype (A-42), for the later UI build-out. Product meaning lives in
[SPEC.md](../../../SPEC.md) and [ACCEPTANCE.md](../../../ACCEPTANCE.md); this
design only says how the GUI presents it. Design questions are numbered
**Q-26 to Q-39** in [../questions.md](../questions.md) and marked **(Q-n)** at
the point of use. Data the UI needs but core/runtime does not expose yet is
marked **(UG-n)** and listed in [screens.md](screens.md#data-gaps).

Owner direction (D-19): a **roomier dashboard in the spirit of Syncthing**,
following the **system light/dark theme**, with a tray presence that has honest
pause and quit behaviour.

## Documents

| File | Contents |
|---|---|
| [README.md](README.md) | Window layout, navigation map, tray, close/pause/quit, first-run onboarding (this file) |
| [screens.md](screens.md) | One section per screen: purpose, data, actions, every state; data gaps UG-1..UG-24 |
| [style.md](style.md) | Colour tokens (light/dark, measured contrast), spacing, type, status colours and icons, focus ring, high-DPI |
| [interaction.md](interaction.md) | Keyboard map, focus order, AccessKit names and roles, notifications and gaming deferral |
| [prototype.md](prototype.md) | Exactly what R3-proto builds: screens, fake scenarios, owner acceptance checklist |
| [wireframes/](wireframes/) | One SVG per main screen; the dashboard in light and dark |

## Principles

1. **This computer first (R-12).** The first thing the user sees is the state
   of the computer in front of them: up to date, working, waiting, needs help,
   error or paused. Other computers come second and are always described by
   what they **last confirmed**, never by a guess about now (A-24).
2. **Never "all synced" without proof.** Published, available here, applied
   here and confirmed by another computer are four different words with four
   different visual treatments ([style.md](style.md#status-vocabulary)). The UI
   never says "every computer is up to date" (R-12, A-24).
3. **Drill down, do not dump.** Every summary row opens a detail that answers
   "what changed, which copy was chosen, why, what it affects, and what to do
   next" (A-25). Internal IDs (computer IDs, hashes, sequence numbers, folder
   keys) appear only inside an expandable **Details** area.
4. **The context is always visible.** The game version, sync group and this
   computer's name are on every screen (in the header), so the user sees the
   variant/group/identity context before any consequential action (R-02).
5. **The GUI shows, core decides.** No screen computes sync policy. Counts and
   groupings only present core/runtime data; anything core does not yet expose
   is a named gap (UG-n), not a UI-side rule.
6. **Plain English, short.** Sentences a WoW player understands without knowing
   Lua, hashes or Dropbox internals (R-09, R-12).

## Window layout

One resizable main window. Default size 1040 × 720 logical px, minimum
760 × 520. Below 900 px wide the sidebar collapses to a 64 px icon rail whose
items keep their full accessible names. The layout is the Syncthing pattern
adapted to a sidebar: a persistent header with the headline, a navigation
sidebar, and a content area of stacked panels.

```text
+--------------------------------------------------------------------------+
| HEADER  [status icon] Headline sentence                [Sync now] [Pause]|
|         Forever · "Main add-ons" group · this computer: Desk PC          |
+-------------+------------------------------------------------------------+
| SIDEBAR     | CONTENT (scrolls)                                          |
|  Overview   |  [banner: paused / WoW running / setup check, if any]      |
|  Computers  |  page title                                                |
|  Add-ons    |  +-----------------------------+ +------------------------+ |
|  Needs      |  | panel                       | | panel                  | |
|  attention 2|  +-----------------------------+ +------------------------+ |
|  Activity   |  +------------------------------------------------------+ |
|  Settings   |  | panel / list                                         | |
|  sync (soon)|  +------------------------------------------------------+ |
|  ---------  |                                                            |
|  App        |                                                            |
|  settings   |                                                            |
|  About      |                                                            |
+-------------+------------------------------------------------------------+
```

- **Header** (fixed, not scrolled): status icon and headline sentence, the
  context line, and two global buttons, **Sync now** and **Pause syncing** /
  **Resume syncing**. The headline is the same sentence as the tray tooltip.
- **Sidebar** (fixed, 220 px): navigation. **Needs attention** shows a count
  badge when non-zero. **Settings sync** carries a "Coming later" tag. App
  settings and About sit at the bottom, below a divider.
- **Content**: one page per navigation item, scrolled vertically. Panels are
  cards (surface colour, 8 px radius, 20 px padding). The Overview shows two
  panel columns when the content area is at least 820 px wide; other pages are
  one column, max width 880 px, left-aligned.
- **Banners** sit at the top of the content area for conditions that change
  what the page means: **Paused**, **WoW is running**, **Confirm the game
  version** (reconfirm), **Shared folder problem**. At most two banners show;
  the rest collapse into "and N more" linking to Needs attention.
- **Detail pages** (add-on detail, computer detail, decision) replace the list
  in the content area and show a **Back** link naming the list
  ("← Add-ons"). They are pages, not modal dialogs (Q-33).
- **Sheets** are the only overlay: a panel pinned to the top of the content
  area with a short message and two buttons, used for the first-close and
  quit-while-working messages. They take focus and Esc dismisses them (Q-33).
- **Group context**: with one enrolled group (the slice) the context line is
  plain text. With several groups (future, UG-15) a group selector appears at
  the top of the sidebar and the Overview summarises every group (Q-38).

Wireframes: [dashboard light](wireframes/dashboard-light.svg),
[dashboard dark](wireframes/dashboard-dark.svg).

## Navigation map

```text
Tray icon --click--> main window (last page shown, or Overview)
Tray menu --> Open | Sync now | Pause/Resume | Needs attention... | Quit

Main window
+-- Overview (default)
|   +-- "This computer" panel --> Add-ons (filter: not up to date)
|   +-- "Needs attention" panel --> Decision / Needs-help detail / Reconfirm
|   +-- "Other computers" panel --> Computer detail
|   +-- "Recent changes" panel --> Activity (entry selected)
+-- Computers --> Computer detail --> Add-ons (filter: waiting for that computer)
|                                 +-> Retire this computer... (future, A-45)
+-- Add-ons --> Add-on detail ("Why this version") --> Decision (if one is open)
|                                                  +-> Recovery copies (future restore)
+-- Needs attention --> Decision (choose a copy)
|                   +-> Needs-help detail (interrupted update)
|                   +-> Confirm the game version (reconfirm, A-46)
+-- Activity --> Activity entry detail --> Add-on detail
+-- Settings sync (future placeholder: master, exceptions, push, pause)
+-- App settings (startup, pause, theme, notifications, close behaviour, crash reports)
+-- About (version, updates, data locations, unsigned-build note, licences)

First run (not enrolled): the Setup wizard replaces the sidebar layout until
the computer is enrolled; then Overview.
```

Every detail page is reachable from the Overview in at most two activations.
Every list remembers its scroll position, filter and selected row when the
user comes back.

## Tray

The tray icon is present whenever the app runs (A-01). On Windows a left click
opens or focuses the main window and a right click opens the menu; on macOS a
click opens the menu, which is the platform convention (Q-39). The icon shape
and tooltip follow the headline ([style.md](style.md#status-icons)); colour is
never the only signal.

Tooltip: `WoW Sync: <headline sentence>`, for example
`WoW Sync: Waiting, WoW is running` or `WoW Sync: Paused`.

Tray menu, top to bottom. It is built with `tray-icon`/`muda` (already used by
the slice), which supports disabled items, separators and label changes at run
time:

| Item | Behaviour |
|---|---|
| *Headline sentence* (disabled, informational) | e.g. "Up to date", "2 add-ons need your decision", "Paused" |
| **Open WoW Sync** | Shows and focuses the main window |
| **Sync now** | Same as the header button; disabled while paused, while a sync runs and before setup |
| **Pause syncing** / **Resume syncing** | One item whose label flips; see [Pause](#pause) |
| **Needs attention (N)…** | Only when N > 0; opens the Needs attention page |
| *separator* | |
| **Quit WoW Sync** | See [Quit](#quit) |

Wireframe: [tray menu, close and quit](wireframes/tray-and-quit.svg).

## Close, pause and quit (R-13)

Three different things with three different words. None hides behind another.

### Close

Closing the window (title-bar X, Alt+F4, Ctrl+W / Cmd+W) **hides** it; the app
keeps syncing in the tray (Q-26). The window is hidden, not destroyed: R2-pin
measured flat memory for hide and possible growth over destroy/recreate
cycles. The slice already intercepts close with `on_window_should_close`.

- The **first** time the window is closed after setup, a sheet appears instead
  of hiding: "WoW Sync keeps running in the tray and keeps your add-ons in sync.
  To stop it, choose **Quit WoW Sync** from the tray menu." Buttons: **Hide
  window** (default) and **Quit WoW Sync**. It is shown once; App settings has
  **Explain this again next time**.
- Nothing about syncing changes on close. The tray icon stays.

### Pause

**Pause syncing** stops the app from starting new publish or apply work on
this computer. It applies to the whole computer and stays paused across an app
restart until resumed (Q-28).

- Work already committing a package finishes to a safe point first (core
  decides what a safe point is, UG-17). Until then the header reads
  "Pausing, finishing the current step".
- While paused, the headline is **Paused** everywhere (header, tray tooltip,
  tray icon), in the `paused` colour, with the banner "Syncing is paused on this
  computer. Updates from other computers are not applied here, and changes here
  are not shared. **Resume syncing**". Paused is never shown as up to date
  (R-12). Core has no paused state today (UG-2).
- The app keeps reading status while paused, so the user sees what is waiting
  ("3 updates will be applied when you resume").
- **Settings pause** (per add-on or all settings, R-08) is a different, future
  feature on the Settings sync page; it is never called "Pause syncing".

### Quit

**Quit WoW Sync** (tray menu, App settings, Ctrl+Q / Cmd+Q) ends the app.
Afterwards nothing syncs on this computer until WoW Sync starts again.

- If nothing is in progress, quit is immediate, with no confirmation: the
  wording is already explicit.
- If an update is being applied, a sheet appears: "WoW Sync is updating
  *Deadly Boss Mods*. Quitting waits for the current step to finish safely."
  Buttons: **Quit when safe** (default) and **Keep running**. The app never
  stops its own write half-way on purpose; crash recovery exists but is not a
  quit strategy (Q-29, UG-17).
- Quitting does not change **Start WoW Sync when I sign in**. When that is on,
  the quit sheet and the tray tooltip of the next start say so.

### Startup

App settings has **Start WoW Sync when I sign in** (A-01), on by default after
setup, and **Start hidden in the tray** (on by default). The OS registration
(Run key / LaunchAgent) is runtime work, not GPUI (UG-23).

## First-run onboarding

When the computer is not enrolled, the window shows a **Setup** wizard instead
of the sidebar layout: a linear sequence of pages with **Back**, **Continue**
and a step indicator ("Step 2 of 6"). Nothing is written until the final
**Set up this computer** button: the slice preview is read-only (domain.md
§1). Each step's states are in [screens.md](screens.md#setup-and-reconfirm).

1. **Welcome.** What WoW Sync does (keeps add-ons the same on your computers
   through Dropbox) and does not do (it does not download add-ons; CurseForge or
   your updater still does that; it never changes game files while WoW is
   running). The crash-report notice and opt-out are shown here, before any
   transmission (R-15; telemetry is future).
2. **Shared folder.** Dropbox is required (R-03). Shows the detected Dropbox
   folder or "Dropbox was not found"; **Choose folder…** opens the native folder
   picker. Explains "Make this folder available offline in Dropbox" without
   claiming that asking for offline availability means the files are present
   (R-03, Q-6).
3. **Game installation.** Lists the WoW installations found, each with a plain
   identity ("World of Warcraft Forever, build 1.15.7, folder _forever_"). The
   user picks one. An unfamiliar client is labelled "Not recognised yet: you
   will confirm it in the next step" (A-46).
4. **Confirm game version and add-ons.** The guided identity and scope
   confirmation (A-46): identity facts, the add-on folders that would sync, the
   folders that will not (with reasons), and what syncing does (disclosures).
   Checkboxes: **This is the game version shown above** (unfamiliar clients
   only) and **Sync the add-ons listed above for this installation**. A refused
   layout stops here with an explanation and no Continue.
5. **Sync group.** **Create a new sync group** or **Join "<name>"** for each
   group of the same game version found in the shared folder. Joining explains:
   "This computer will follow the group's add-ons. Add-ons replaced here are
   kept as recovery copies. Add-ons you do not have here are not treated as
   removed." (R-03, A-04).
6. **Name this computer.** The name other computers will see (default: the OS
   computer name; editing it needs a text field, Q-31). **Set up this
   computer** writes the enrollment. The finish page says "Setup complete.
   Syncing starts now.", shows the startup toggle and the close-to-tray
   explanation, then **Go to overview**.

Wireframe: [setup, confirm step](wireframes/setup.svg).
