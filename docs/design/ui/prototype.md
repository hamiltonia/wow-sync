# UI design: prototype brief for R3-proto

Part of the [UI design](README.md). This is the complete list of what R3-proto
builds: a **clickable GPUI prototype on fake data** that the owner reviews at
the phase-D barrier (A-42, PLAN §3). Anything not listed here is out of scope
for the prototype. The prose in [README.md](README.md) and
[screens.md](screens.md) is the authoritative layout; the wireframes follow it.
Where this brief and another UI file disagree, report it; do not guess.

> **Status (R3-fix, RV-9): frozen.** `crates/wow-sync-proto` is kept as the
> D-21-approved reference. Only D-21 copy fixes go to it (for example the
> per-reason decision wording, tested over every `DecisionReason`). It is to be
> retired or merged into a shared UI crate in a later run. RC-3 (R3-check) is
> fixed in the real app only; the prototype keeps its fake scenarios. Open UI
> questions are in [questions.md](questions.md).
## Ground rules

1. **Fake data only.** The prototype never reads WoW folders, the provider
   folder, Dropbox, or the runtime's state directory, and never starts the sync
   thread. It never starts, inspects or stops WoW or Battle.net.
2. **Clearly labelled.** A permanent strip at the bottom of the window reads
   "Prototype: fake data, nothing is synced" (ACCEPTANCE human-acceptance
   layer: a mocked prototype must be labelled). The tray tooltip starts with
   "Prototype:".
3. **Core types where they exist.** Fake scenarios build real
   `wow_sync_core::status` values (`StatusView`, `PackageStatus`,
   `MemberStatus`, `Explanation`, `PlanBlock`, `DecisionReason`) so the mapping
   in [screens.md](screens.md) is exercised. Data for gaps (UG-n) lives in
   prototype-only structs named `Fake…`, each with a comment naming its UG-n.
   No sync policy is written in the prototype.
4. **Names.** Fictional names only (Desk PC, Laptop, Den PC; add-ons such as
   Deadly Boss Mods, WeakAuras, Details!, Bagnon, Questie). No real account,
   realm, character or user names.
5. **Game identity.** The enrolled client is always
   "World of Warcraft: Forever (wowf)", folder `_forever_`
   ([identity labels](screens.md#identity-labels)). Never a Classic Era folder,
   product code or build for Forever. Builds are `fake-build-A` /
   `fake-build-B`. The unfamiliar client in S2 is the fictional `_example_` /
   `wowx`.
6. **Headline and continuation from core.** The headline is core's
   `derive_status` result for the fake inputs (decisions alone give
   `Waiting`, UG-26). The continuation sentence comes from the fake typed
   scope `FakeApplyScope` (UG-25), never from a UI rule.
7. **Dependencies.** Only what the workspace already uses (GPUI at the pinned
   tag, `tray-icon`). Icons are SVG files authored in the repository
   ([style.md](style.md#status-icons)). Any other dependency needs orchestrator
   approval first.
8. **Process model.** One process, tray plus window, close hides, as the slice
   (R2-pin lifecycle).
9. **Launch.** A separate binary (suggested name `wow-sync-proto`; the packet
   sets the exact path) with `--scenario <id>` and `--theme system|light|dark`.
   Defaults: scenario `S3`, theme `system`. A scenario switcher is reachable
   inside the app (see below), so the owner never needs a terminal after
   launch; a double-clickable launcher script or shortcut is provided.

## Screens to build

| # | Screen | Spec | Wireframe | Must show |
|---|---|---|---|---|
| P1 | Window shell: header, sidebar, content, banners, sheets | [README](README.md#window-layout) | [dashboard light](wireframes/dashboard-light.svg) | collapse to icon rail below 900 px |
| P2 | Overview | [screens](screens.md#overview) | [light](wireframes/dashboard-light.svg), [dark](wireframes/dashboard-dark.svg) | two columns ≥ 820 px content width |
| P3 | Computers list; computer detail replaces it | [screens](screens.md#computers) | [computers](wireframes/computers.svg) (list page) | last confirmed vs unknown now (A-24) |
| P4 | Add-ons list | [screens](screens.md#add-ons) | [add-ons](wireframes/addons.svg) | filters, fixed order, roving focus |
| P5 | Add-on detail ("Why this version") | [screens](screens.md#add-on-detail-why-this-version) | [detail](wireframes/addon-detail.svg) | all 9 sections, Details collapsed |
| P6 | Setup wizard (6 steps + finish) and Reconfirm | [screens](screens.md#setup-and-reconfirm) | [setup](wireframes/setup.svg) | consent rules incl. "scope alone is not enough" |
| P7 | Needs attention, Decision, Needs-help detail | [screens](screens.md#needs-attention) | [decision](wireframes/decision.svg) | decision scoped to one add-on (S7); journal needs-help stops all applies here (S16) |
| P8 | Activity (+ notification preview) | [screens](screens.md#activity) | [activity](wireframes/activity.svg) | batches expand; preview shows deferral |
| P9 | Settings sync placeholder | [screens](screens.md#settings-sync-coming-later) | [settings sync](wireframes/settings-sync.svg) | everything "Coming later", focusable |
| P10 | App settings | [screens](screens.md#app-settings) | [app settings](wireframes/app-settings.svg) | theme switch live; startup toggles faked |
| P11 | About and updates | [screens](screens.md#about-and-updates) | [about](wireframes/about.svg) | manual update guidance, unsigned notice |
| P12 | Tray icon, tooltip and menu | [README](README.md#tray) | [tray and quit](wireframes/tray-and-quit.svg) | icon overlay per state; menu items work |
| P13 | Close, pause, quit sheets | [README](README.md#close-pause-and-quit-r-13) | [tray and quit](wireframes/tray-and-quit.svg) | first-close sheet; quit-while-working sheet |
| P14 | Scenario switcher (prototype only) | below | — | reachable by keyboard |

Style tokens, spacing, type and focus ring from [style.md](style.md); keyboard
map, focus order, accessible names and notification rules from
[interaction.md](interaction.md).

**Scenario switcher.** An extra sidebar item **Prototype scenarios** (below
About, visually separated, labelled "Prototype") lists the scenarios as
buttons with their one-line description; activating one reloads the fake state
and returns to the Overview. Mod+Shift+N moves to the next scenario.

## Fake data scenarios

Each scenario is a named, deterministic fake state. Fake transitions use
timers (seconds, not real sync). "Group" is always "Main add-ons" on
"World of Warcraft: Forever (wowf)", folder `_forever_`, unless stated.

| ID | Scenario | States it demonstrates | Contents |
|---|---|---|---|
| S1 | First run, recognised client | Setup: Empty → Working → Success | Not enrolled; Dropbox found; one installation (World of Warcraft: Forever (wowf), `_forever_`, `fake-build-A`, recognised); 38 eligible folders, 2 blocked (suffixed toc); one existing group to join; **Set up** fakes 2 s of "Setting up…" then the finish page, then S3 |
| S2 | First run, unfamiliar client and refused layout | Setup: Needs help, Error | Installation list: one unfamiliar client (fictional `_example_`, product `wowx`, "Not recognised yet") needing the identity checkbox, and one refused installation with a problem sentence; Dropbox folder has online-only files (Waiting variant of step 2) |
| S3 | All good | Success | 42 add-ons: 40 `ConfirmedBy[Laptop]`, 1 `AppliedHere` (not confirmed yet), 1 `InSync`; 2 `NotSynced`; Laptop `Usable`, report 5 min ago; recent changes: 3 batches |
| S4 | Syncing | Working | S3 with 5 `Available`; header "Syncing: applying 2 of 5 updates"; after 6 s they become `AppliedHere` and one batch summary is queued |
| S5 | WoW running | WoW running, Waiting | `PlanBlock::GameRunning`; 2 `Blocked(GameRunning)`; 1 `Interrupted` with `InterruptedByGame`; banner; notification preview shows 2 held notifications; button **Simulate: WoW closed** releases them as one summary + one action |
| S6 | Waiting for Dropbox | Waiting | 2 `Unavailable` (`NotAvailableYet`), 1 `Blocked(Unstable)`; header "Waiting for Dropbox to finish downloading" |
| S7 | Add-on decisions; other add-ons keep syncing | Waiting (core headline, UG-26), Needs attention | No apply journal (`FakeApplyScope::PerPackage`). WeakAuras `NeedsDecision(Ambiguous)` with 2 candidates (5.18.1 here and Laptop, 5.18.1-beta on Den PC); Bagnon `NeedsDecision(PartialCandidate)`; Questie `NeedsDecision(DifferentTocMapping)`; 39 others fine, including Details! `Available` from Laptop, which becomes `AppliedHere` after 4 s while the decisions stay open (shows unrelated work proceeding). Headline "Waiting for your decision on 3 add-ons". **Keep this copy** fakes Working → Waiting ("applied on the next sync") → Success |
| S8 | Shared folder error | Error | `provider_error` "Dropbox is not running"; last known add-on data shown with "as of 3 hours ago"; **Try again** toggles to S3 after 2 s |
| S9 | Silent and late computers | Offline/stale peer | Laptop current (`Usable`, 5 min, confirms all); Den PC `Usable` but last report 12 days ago, confirmed 40 of 42; Spare PC `NoState`; Old PC `Regressed`; button **Simulate: Den PC reports** delivers a late acknowledgement (Den PC then confirms 42 of 42, "reported again after 12 days" activity entry) (A-24) |
| S10 | Paused | Paused | S4's 5 updates, paused since 21:04; banner; Sync now disabled; Resume moves to S4 |
| S11 | Empty group | Empty | Enrolled; 0 add-ons; no other computers; no activity |
| S12 | Reconfirm needed: add-on file rule changed | Needs help (setup) | `reconfirm` set because the stored `mapping_id` `validated-plain/1` no longer matches the current `profile:wowf/1` (fake; a consent-relevant layout/mapping change, core `StaleConfirmation`); banner "Confirm the game version to continue syncing"; Needs attention item; Reconfirm page with "What changed: WoW Sync now uses a different rule for which add-on files this game version loads. 5 more folders would sync." and the new folder lists (UG-16); both checkboxes required; syncing stays stopped until confirmed |
| S13 | Update available | About states | About shows "Version 0.2.0 is available"; one app-update notification in the preview; **Check for updates** toggles Working → Success |
| S14 | Large collection | list performance | 300 add-ons (mixed statuses) and 4 computers; used for scrolling, keyboard and focus checks (Q-32) |
| S15 | Game patched: build-only change | Success (no reconfirm) | Starts as S3 with `fake-build-A`; button **Simulate: game patched** changes only the build to `fake-build-B` (same product, folder, mapping and recognition class). Result: no reconfirm banner, no Needs attention item, headline stays "This computer is up to date", This computer → Game shows `fake-build-B` (D-18 O-N2) |
| S16 | Interrupted update needs help; all applies here stopped | Needs help | Deadly Boss Mods journal `NeedsHelp` (`FakeApplyScope::AllAppliesBlocked`); 3 other add-ons `Available` from Laptop that stay unapplied ("Not applied: an earlier update needs help"); headline "An update needs your help" (core `NeedsHelp`); continuation "No updates are applied on this computer until this is resolved. Sharing this computer's add-ons continues."; Needs-help detail with fake **Try again** / **Keep the previous version** (UG-9) |

All scenarios support: theme switch, window close (hide) and reopen from the
tray, Pause/Resume, Sync now (fake), Quit (with the working sheet in S4).

## Evidence R3-proto must produce

- Screenshots of every screen in the scenarios listed in the checklist below,
  in light and dark, at 100 % scale, plus the Overview and Add-on detail at
  150 %. Saved under the packet's evidence folder, each named
  `<screen>-<scenario>-<theme>-<scale>.png`.
- A keyboard walk-through log: for each screen, the Tab/arrow sequence used and
  where focus landed.
- A UIA tree dump (as R2-pin `uia.ps1`) of the Overview and Add-ons in S7,
  showing roles and names.
- A table of which AccessKit attributes exist at the pin (selected, expanded,
  live region, disabled) and which fallbacks were used.
- Whether Windows live theme change was followed (Q-35), whether text scaling
  is honoured (style.md rule 6), and whether roving focus was kept (Q-32).
- The launcher and the exact launch command.

## Owner acceptance checklist (A-42)

The owner runs the prototype from the launcher and ticks each item. "Pass"
means the owner agrees it behaves and reads as described; each item names the
scenario to use.

**Overall look**

1. The window feels like a roomy dashboard in the spirit of Syncthing, not a
   tray popup (D-19). (S3)
2. It follows the system light/dark setting; switching the system setting
   switches the app; App settings can override it. (S3)
3. Text is readable in both themes; status is shown by icon and word, not only
   colour. (S7, both themes)

**This computer and other computers**

4. Within five seconds the owner can say whether this computer is up to date,
   busy, waiting, needs help, has an error or is paused. (S3, S4, S5, S7, S8,
   S10, S16)
5. The app never says every computer is up to date; each computer shows what
   it last confirmed and when that report arrived. (S3, S9)
6. A computer silent for 12 days is clearly shown as "not heard from", with
   its last confirmation, and "what it has now is not known". (S9)
7. After **Simulate: Den PC reports**, Den PC's confirmation updates and an
   activity entry appears. (S9)

**Add-ons and explanations**

8. For an updated add-on, the detail page answers what changed, which copy was
   chosen, why, on which computers, and what to do next, without IDs or
   hashes outside Details. (S3, S4)
9. A decision affects only that add-on; the page says other add-ons keep
   syncing, and Details! is updated while the decisions are still open; the
   copies show who has them. (S7)
10. An interrupted update that needs help clearly says that no updates are
    applied on this computer until it is resolved, the other waiting updates
    say why they are held, and sharing continues. (S16)
11. Choosing a copy shows saved → waiting → done, and says when it will be
    applied. (S7)
12. Waiting reasons (WoW running, Dropbox downloading, files changing) are
    understandable and say what to do. (S5, S6)

**Setup**

13. Setup can be completed without a terminal; nothing is written before
    **Set up this computer**. (S1)
14. For an unfamiliar game version, agreeing to the add-on list alone is not
    enough; the identity must be confirmed too; a refused installation explains
    why and offers no way to continue. (S2)
15. Joining a group explains that replaced add-ons are kept and missing ones
    are not treated as removed. (S1)
16. When the add-on file rule for this game version changes, Reconfirm
    explains what changed (which folders would now sync) and that syncing
    stays stopped until confirmed. (S12)
17. A game patch that only changes the build number does not ask for
    confirmation again; the new build is shown and syncing continues. (S15)

**Tray, close, pause, quit**

18. Closing the window keeps the app in the tray, and the first close explains
    this. (any)
19. Pause shows "Paused" everywhere (header, tray icon, tooltip) and never
    looks up to date; Resume continues. (S10)
20. Quit is in the tray menu and is explicit; quitting while an update is
    applied waits for a safe point. (S4)
21. The tray icon changes shape for working, waiting, needs help, error and
    paused. (S4, S5, S7, S8, S10, S16)

**Keyboard and accessibility**

22. Every screen and action in this checklist can be done with the keyboard
    only; focus is always visible. (all)
23. The app is usable at 150 % scaling, and at the minimum window size the
    sidebar collapses without losing anything. (S3, S14)
24. (Nice-to-have, D-18) Narrator reads the headline, sidebar items and list
    rows with their status. Failures are listed as limitations, not
    blockers.

**Notifications and future areas**

25. The notification preview shows one summary per batch, separate action
    items, held notifications while WoW runs released as at most two on exit,
    and no repeat for an unchanged condition. (S5, S7)
26. Settings sync is visibly "Coming later" and its structure (master,
    exceptions, share, pause) makes sense. (any)
27. About explains the unsigned build and manual update without automatic
    replacement. (S13)

## Deferred: not demonstrated and not claimed

The prototype does not demonstrate the following, and passing the checklist
above is **not** evidence for them. Each remains an open obligation for a
later design and build-out:

- retirement, re-enrollment and settings-master absence/reassignment (A-45,
  UG-20);
- restoring a recovery copy and rolling back a bad release (A-25 recovery
  parts, A-26 rollback);
- add-on removal and uninstall propagation (owned by the removal design);
- settings sync: master, per-add-on exceptions, explicit push, pause and
  competing-choice handling (A-15, A-26 settings parts); only a "Coming later"
  placeholder is shown (P9);
- diagnostic export (A-33).

The owner records approval or requested changes; build-out (phase F) starts
only after a recorded approval (PLAN §5).
