# UI design: screens and states

Part of the [UI design](README.md). Each screen lists its purpose, the data it
shows mapped to `wow_sync_core::status` (`StatusView`, `PackageStatus`,
`MemberStatus`, `Explanation`) or to the runtime (`StatusSnapshot`,
`EnrollPreview`, `SyncReport`), its actions, and what it shows in every state.
Missing data is a gap **UG-n** ([Data gaps](#data-gaps)) with a suggested core
or runtime field. The UI adds no policy: where a screen needs a decision rule
it names the gap instead.

Sample names in this document (Desk PC, Laptop, Den PC, Deadly Boss Mods,
WeakAuras, Details!) are fictional placeholders for the prototype. Never use
real account, realm, character or user names.

## Shared vocabulary

### States

Every screen section has a state table with these rows. "Paused" is added
because pause changes every screen (R-13).

| State | Meaning in this UI | Main source |
|---|---|---|
| **Empty** | Nothing to show yet (no add-ons, no other computers, no history) | empty lists in `StatusView` |
| **Working** | A sync cycle is publishing or applying now | `Headline::Working`, `Shared.busy` |
| **Waiting** | Something is ready but a safe condition is not met (data not downloaded, files still changing, interrupted update waiting) | `Headline::Waiting`, `PlanBlock`, `PackageStatus::{Available, Unavailable, Interrupted, Blocked}` |
| **Success** | This computer matches the group's chosen add-ons | `Headline::Ok` |
| **Needs help** | The user must act: a decision for one add-on, an interrupted update that needs help, or a reconfirmation. What else continues depends on the kind (see [Continuation scope](#continuation-scope)) | `PackageStatus::{NeedsDecision, NeedsHelp}`, `StatusSnapshot.reconfirm`; headline per [Headline](#headline) |
| **Error** | The shared folder or local state cannot be read; nothing can sync | `Headline::Error`, `StatusSnapshot.provider_error` |
| **Offline/stale peer** | Another computer has not reported for a while, or its report is unusable | `MemberStatus.{state, seen_locally_at}` |
| **WoW running** | The game blocks every game-file change on this computer | `PlanBlock::GameRunning`, `StatusSnapshot.activity` (UG-10) |
| **Paused** | The user paused syncing on this computer | none today (UG-2) |

### Headline

The headline colour and icon always follow core's `Headline` value; the UI
never promotes or demotes it. The sentence may add counts from
`StatusView.packages`.

| Core `Headline` | Header and tray sentence | Colour / icon ([style.md](style.md)) |
|---|---|---|
| `Ok` | "This computer is up to date" (core says "Everything is up to date", which reads as group-wide; UG-22) | `ok` / check |
| `Working` | "Syncing: applying 2 of 5 updates" (the count needs UG-14; else "Syncing") | `working` / circular arrows |
| `Waiting` + `GameRunning` | "Waiting: WoW is running" (`render_headline`) | `waiting` / hourglass |
| `Waiting` with `NeedsDecision` packages and no block | "Waiting for your decision on 2 add-ons" | `waiting` / hourglass |
| `Waiting` (other) | "Waiting for Dropbox to finish downloading", "Waiting for add-on files to stop changing" (first block's reason, `render_block`) | `waiting` / hourglass |
| `NeedsHelp` | "An update needs your help" (core: "Something needs your help"); a journal `NeedsHelp` is the only core source today | `help` / exclamation in triangle |
| `Error` | "Can't read the shared folder" (category needs UG-11; else "Something needs attention") | `error` / cross in circle |
| (paused, UG-2) | "Paused" | `paused` / two bars |

**Decision headline gap (UG-26).** Core `derive_status` gives
`Headline::Waiting` when the only non-current packages are `NeedsDecision`
(asserted by core test `needs_decision_shows_evidence`), although a decision
needs the user. This design renders that value as is (waiting colour and
icon, the sentence above, the Needs attention badge). Whether decisions should
raise the headline to `NeedsHelp` is a core policy question recorded as UG-26;
the UI does not change it.

### Continuation scope

What keeps working while something needs the user is core policy (R-06, Q-25),
not a UI choice. Two cases are designed and must never be merged into one
sentence:

| Case | Core facts | Continuation sentence |
|---|---|---|
| **Add-on decision** (`NeedsDecision`, no apply journal on this computer) | `plan_apply` skips only that package; other packages are planned and applied | "Only WeakAuras is affected. Other add-ons keep syncing." |
| **Interrupted update needs help** (an apply journal in `NeedsHelp`) | `plan_apply` returns `UnresolvedJournal` before considering any package (Q-25, apply.md §1); publishing continues | "No updates are applied on this computer until this is resolved. Sharing this computer's add-ons continues." |

Both may be present at once; the journal sentence then wins everywhere. Today
the runtime does not export which case applies: it skips planning while
journals remain and pushes no block (UG-25). The prototype fakes the typed
field (`FakeApplyScope`). Until UG-25 exists, the build-out shows **neither**
continuation sentence rather than guess.

### Identity labels

Installation identity text comes from core, never from UI constants (R-02).
For the owner's client the canonical identity is
**World of Warcraft: Forever (wowf)**, folder `_forever_` (core
`display_label()`; OWNER-DECISIONS D-09). Forever is never called, mapped to
or shown with a Classic Era folder, product code or build. Build numbers are
shown exactly as core reports them; fake data uses obviously fake builds
(`fake-build-A`, `fake-build-B`), not real-looking numbers. Unfamiliar clients
in fake data use the fictional folder `_example_` and product `wowx`.

### Add-on status

How each `PackageStatus` appears in lists and on the detail page. The **short
label** is the list chip text; the **sentence** is the detail-page line and
comes from `render(Explanation)` once UG-22 is fixed.

| `PackageStatus` | Short label | Detail sentence (example) | Style |
|---|---|---|---|
| `InSync` | Up to date | "Same as the group's chosen version." | `ok` |
| `AppliedHere` | Updated here | "Updated from 11.0.3 to 11.0.4, copied from Laptop. No other computer has confirmed it yet." | `ok` + "not confirmed yet" in `unknown` |
| `ConfirmedBy(ids)` | Up to date | "Same as the group's version. Confirmed by Laptop (report received 5 min ago)." | `ok` |
| `Available` | Update ready | "Version 11.0.4 from Laptop will be applied on the next sync." | `working` |
| `Blocked(GameRunning)` | Waiting for WoW to close | "WoW is running; nothing was changed." | `waiting` |
| `Blocked(other)` | Waiting | `render_block` text, e.g. "Add-on files are still changing; waiting" | `waiting` |
| `Unavailable` | Waiting for Dropbox | "Laptop's copy has not finished downloading through Dropbox yet." | `waiting` |
| `Interrupted` | Update interrupted | "The update was interrupted because WoW started. It will finish after WoW closes." | `waiting` |
| `NeedsDecision(reason)` | Needs your decision | per reason, see [Decision](#decision) | `help` |
| `NeedsHelp(reason)` | Needs help | "An update stopped in an unexpected state. The old and new files are both kept." | `help` |
| `NotSynced` | Not synced here | "This folder uses a game-version-specific setup that is not supported yet, so it is never copied." | `unknown` |

### Other computers

| `MemberSummary` | Label | Style |
|---|---|---|
| `NoState` | "Has not shared anything yet" | `unknown` |
| `Usable` | "Report received" plus what it confirmed | `ok` if it confirms every current add-on (UG-4), else `unknown` |
| `Unusable` | "Its latest report could not be used" | `waiting` |
| `Regressed` | "Its report went backwards and was ignored" | `help` |
| `Invalid` | "Its enrollment record is not valid" | `error` |
| (retired, future UG-20) | "Retired on 3 Nov; history kept" | `unknown` |

**Times.** Every time is this computer's clock, shown relative ("5 min ago",
"12 days ago") with the absolute local date and time in the accessible name and
in Details. A peer's own clock (`reported_at_peer_clock`) appears only in
Details, labelled "by Laptop's own clock" (Q-37, `render_member`).

---

## Overview

**Purpose.** At a glance: is this computer OK, what needs me, and what did the
other computers last confirm. The Syncthing-style home page.

Wireframes: [light](wireframes/dashboard-light.svg),
[dark](wireframes/dashboard-dark.svg).

**Layout (authoritative; the wireframes follow it).** Header as in the
[README](README.md#window-layout): status icon, headline, context line
(`<identity label> · "<group>" group · this computer: <name>`), **Sync now**
and **Pause syncing**, on every page and in every state. Content: two columns
at ≥ 820 px content width, else one column in the order listed.

- Left column:
  - **Needs attention** panel, only when there is something (up to 3 items,
    then "See all N").
  - **This computer** panel: key/value rows (Add-ons, Game, Shared folder,
    Last shared, Last check, Recovery copies when > 0).
  - **Recent changes** panel: the last 5 activity batches.
- Right column:
  - **Other computers** panel: one compact card per computer.

**Data.**

| Element | Source | Gap |
|---|---|---|
| Headline, icon | `StatusView.headline`, `blocks` (`render_headline`) | UG-1 (runtime passes a string), UG-2, UG-22 |
| Context line (game version, group, this computer) | `StatusSnapshot.{identity_line, group_name, computer_name}` (`who_line`) | — |
| "Add-ons": "41 up to date, 1 update ready, 1 needs your decision, 2 not synced here" | counts over `StatusView.packages` by status | UG-1 |
| "Game": "Not running" / "Running" / "Can't tell" | `StatusSnapshot.activity` (string) | UG-10 |
| "Shared folder": path and state | `ServiceConfig.shared_root`, `provider_error` | UG-11 |
| "Last shared": "12 min ago (42 add-ons)" | `StatusView.last_publish` | — |
| "Last check": "1 min ago; next in 4 min" | none | UG-14 |
| "Recovery copies": "3 extra copies kept for 2 add-ons" (only when > 0) | `StatusView.retained_extra_recovery` | UG-13 for the per-add-on list |
| Other computers cards | `StatusView.members` | UG-5, UG-6, UG-4 |
| Recent changes | `Shared.report_lines` (last cycle only) | UG-7 |

**Actions.** Sync now; Pause/Resume (header); each Needs attention row opens
its item; "This computer" Add-ons row opens Add-ons filtered to "Not up to
date"; a computer card opens Computer detail; a Recent change opens Activity
with that entry selected.

**States.**

| State | What shows |
|---|---|
| Empty | No add-ons: "No add-ons found in this installation's AddOns folder yet. Add-ons you install with CurseForge or another updater appear here after the next check." No other computers: Other computers panel reads "No other computers have joined this group yet. Set up WoW Sync on another computer and choose **Join** to add it." Recent changes: "Nothing has changed yet." |
| Working | Header "Syncing…" with an indeterminate progress bar under the headline; **Sync now** reads "Syncing…" and is disabled. Panels keep the previous values (no flashing). |
| Waiting | Header in `waiting`; the first block's reason; the This computer Add-ons row shows "2 updates waiting" with the reason. |
| Success | Header "This computer is up to date"; This computer shows "Last check 1 min ago". No Needs attention panel. The Other computers panel never adds a group-wide "all up to date"; each card shows what that computer last confirmed. |
| Needs help | Needs attention panel first, with each item's one-line reason and a button ("Choose a copy", "See what happened", "Confirm game version"). Under the panel, the continuation sentence for the case shown ([Continuation scope](#continuation-scope)): decisions only → "Other add-ons keep syncing."; an interrupted update needing help → "No updates are applied on this computer until this is resolved. Sharing this computer's add-ons continues." Decisions alone keep the core `Waiting` headline (UG-26). |
| Error | Header in `error`: "Can't read the shared folder"; a banner with the cause and the next step (UG-11), for example "Dropbox is not running. Start Dropbox, then choose **Try again**." **Try again** = Sync now. Add-on counts show "as of 3 hours ago" (UG-14). |
| Offline/stale peer | That computer's card shows "Not heard from for 12 days" in `unknown` with a clock icon and "Last confirmed 40 of 42 add-ons, 12 days ago. What it has now is not known." (A-24, Q-27). |
| WoW running | Banner: "WoW is running. WoW Sync never changes game files while the game runs. 2 updates will be applied after you close WoW." Header "Waiting: WoW is running". Sharing this computer's add-ons continues (Q-5). |
| Paused | Banner in `paused` with **Resume syncing**; header "Paused"; This computer shows "3 updates will be applied when you resume" (counts of `Available`). Sync now is disabled with the tooltip "Resume syncing first". |

---

## Computers

**Purpose.** What each other computer **last confirmed**, when this computer
received that report, and what is still unconfirmed (A-24, R-12). It
never states a computer's present state.

Wireframe: [computers](wireframes/computers.svg).

**List page (authoritative layout).** One column, full content width. One row
per member, this computer first (labelled "This computer").
Columns: Name · Last report received here · Last confirmed · Status.
Activating a row opens **Computer detail**, which replaces the list (README
Detail pages); the list and the detail are never shown side by side.

| Element | Source | Gap |
|---|---|---|
| Name | `MemberStatus.display_name` | — |
| Last report received here | `MemberStatus.seen_locally_at` (local clock) | UG-5 (units, "never") |
| Last confirmed | "40 of 42 current add-ons" from `PackageStatus::ConfirmedBy` lists | UG-4, UG-6 |
| Status | `MemberStatus.state` (table above) | UG-20 for retired |
| Game version | none | UG-6 |

**Computer detail.**

- Header: name, status label, "Report received here 5 min ago".
- **"What we know"** panel: "Last confirmed: 40 of 42 current add-ons" and
  "Not confirmed yet: Deadly Boss Mods 11.0.4, WeakAuras 5.18.1"; each
  opens its add-on. A fixed line: "This is what Laptop last reported. What it has
  right now is not known until it reports again."
- **Details** (expandable): computer ID, enrollment ID, last report number
  (`last_seq`), "reported at … by Laptop's own clock", app version.
- **Retire this computer…** (future, A-45): shown disabled with "Coming
  later". When built: explains that history stays, it is no longer waited for,
  it cannot publish again without re-enrollment, the add-ons stay on the other
  computers, and if it was the settings master a new master must be chosen
  explicitly (UG-20).

**Actions.** Open detail; open an unconfirmed add-on; (future) retire.

**States.**

| State | What shows |
|---|---|
| Empty | "No other computers have joined this group yet." with a short "How to add a computer" text. This computer's row is still shown. |
| Working | Unchanged rows; this computer's row shows "Sharing now…" while publishing. |
| Waiting | A computer whose shared data is still downloading: "Report seen; some files are still downloading through Dropbox" (`Unavailable` with `NotAvailableYet`). |
| Success | Each computer: "Confirmed all 42 current add-ons, as of 5 min ago." The page header says "Based on each computer's last report", never "All computers are up to date". |
| Needs help | `Regressed`: "Laptop's report went backwards (older than one we already had) and was ignored. This can happen when a computer is restored from a backup." Next step: "Open WoW Sync on Laptop." |
| Error | `Invalid`: "Den PC's enrollment record is not valid, so its reports are ignored." Shared folder unreadable: the list shows the last known values with "as of" times and the page banner explains. |
| Offline/stale peer | Row emphasised: clock icon, "Not heard from for 12 days" (threshold Q-27); "Last confirmed 40 of 42 add-ons, 12 days ago. What it has now is not known." `NoState`: "Joined, but has not shared anything yet." |
| WoW running | No change to this page; peers are not affected by this computer's game. Header shows the waiting headline. |
| Paused | This computer's row: "Paused: not sharing or applying". Other rows still update from received reports. |

---

## Add-ons

**Purpose.** Every add-on of this group on this computer with its state, so
the user can find what changed or what waits (R-12).

Wireframe: [add-ons list](wireframes/addons.svg).

**List (authoritative layout).** Fixed order, no sort control: needs your
decision / needs help first, then waiting, then the rest by name. Columns:
Name · Version here · Status · Confirmed by. Filter buttons above the list
(toggle buttons, one active), with counts: **All (44)**, **Not up to date
(3)**, **Needs you (3)**, **Not synced here (2)** (S7 values).

| Element | Source | Gap |
|---|---|---|
| Name | `FolderKey` today (a folder key, e.g. `dbm-core`) | UG-3 (display title from the `.toc` `Title`, without colour codes) |
| Version here | none (`Explanation::NewerVersion.from/to` only when changing) | UG-3 |
| Status | `PackageStatus` (table above) | — |
| Confirmed by | `ConfirmedBy(ids)` → display names | UG-4 (who is still expected) |

**Actions.** Open add-on detail; change filter. No bulk actions.

**States.**

| State | What shows |
|---|---|
| Empty | "No add-ons found in this installation yet." Filter buttons hidden. |
| Working | Rows being applied show "Updating…" in `working`; others unchanged. |
| Waiting | Rows with `Available`/`Unavailable`/`Blocked`/`Interrupted` show their label and reason. |
| Success | All rows "Up to date"; Confirmed by lists names, or "Not confirmed yet" in `unknown`. |
| Needs help | Decision rows first, `help` style, with "Choose…" link text. |
| Error | Last known rows with an "as of" note in the page header. |
| Offline/stale peer | "Confirmed by" never includes a computer because it is silent; a stale computer appears only if its last report confirmed this version, with its age in the accessible name. |
| WoW running | Update-ready rows read "Waiting for WoW to close". |
| Paused | Update-ready rows read "Waiting: syncing is paused". |

## Add-on detail ("Why this version")

**Purpose.** Answer A-25 for one add-on: what changed, which copy was chosen,
why, what it affects, and the next action, without internal IDs.

Wireframe: [add-on detail](wireframes/addon-detail.svg).

**Sections, top to bottom.**

1. **Title row**: add-on name, status chip, Back link "← Add-ons".
2. **Versions**: "On this computer: 11.0.3 · Group's chosen version: 11.0.4"
   (UG-3).
3. **What happened**: the explanation sentence, e.g. "Updated from 11.0.3 to
   11.0.4, copied from Laptop." (`render(Explanation)`; `AppliedFrom`,
   `NewerVersion`, `Installed`).
4. **Why this version**: the evidence in plain words.
   - Newer version: "Laptop has 11.0.4. This computer had 11.0.3. 11.0.4 is
     newer by its version number. File dates and computer clocks are never used
     to decide." (R-05)
   - Same content everywhere: "Every computer that reported has the same files."
   - Decision: the candidates (see [Decision](#decision)).
5. **Next step**: one sentence and at most one button. Examples: "Nothing to
   do.", "Close WoW to let the update finish.", "Choose which copy to keep."
   **Choose a copy**. The next action comes from core (UG-12).
6. **On each computer**: This computer: "Updated here 4 min ago"; Laptop:
   "Confirmed, report received 5 min ago"; Den PC: "Not confirmed yet; not
   heard from for 12 days" (UG-4).
7. **Folders**: the folders of this package ("DBM-Core, DBM-StatusBarTimers,
   DBM-GUI") and, if grouped, why ("grouped because they share a name prefix",
   `GroupedBy`).
8. **Recovery copies**: "2 earlier copies kept" with dates and versions
   (UG-13). A visible exception when more than the normal number are kept
   ("1 extra copy kept until the decision is made", R-10). **Restore…** is
   shown disabled with "Coming later".
9. **Details** (collapsed): folder keys, package hash, tree hashes, the source
   computer ID, report numbers, peer-clock times.

**States.**

| State | What shows |
|---|---|
| Empty | Not reachable without an add-on. If the add-on disappears from the target while open: "This add-on is no longer in the group's list." with Back. |
| Working | Status chip "Updating…"; Next step "Wait a moment." |
| Waiting | Reason sentence and the next step, e.g. Unavailable: "Wait for Dropbox to finish downloading Laptop's copy. If it stays like this, make the WoW Sync folder available offline in Dropbox." |
| Success | "Up to date"; On each computer lists confirmations with ages; Next step "Nothing to do." |
| Needs help | Decision or needs-help summary with **Choose a copy** / **See what happened** to the Decision or Needs-help page, and the continuation sentence for its case ([Continuation scope](#continuation-scope)). |
| Error | Last known data with an "as of" note; Next step names the shared-folder fix. |
| Offline/stale peer | That computer's line: "Not confirmed yet; not heard from for 12 days. What it has now is not known." |
| WoW running | "Waiting for WoW to close. Nothing was changed." For `Interrupted` while running: "The update was interrupted because WoW started. It finishes after WoW closes." |
| Paused | "Waiting: syncing is paused on this computer." with **Resume syncing**. |

---

## Setup and reconfirm

**Purpose.** First-run enrollment and later identity reconfirmation through
guided identity and safe-scope confirmation (R-02, R-03, A-04, A-46). Steps
are listed in the [README](README.md#first-run-onboarding).

Wireframe: [setup, confirm step](wireframes/setup.svg).

**Data (step 4, the confirmation page).**

| Element | Source | Gap |
|---|---|---|
| Identity sentence | `EnrollPreview.identity_line` | — |
| Recognised / unfamiliar | `EnrollPreview.kind`, `needs_guided_confirmation` | — |
| Scope sentence | `EnrollPreview.scope_line` | — |
| Folders that will sync | `EnrollPreview.eligible` | — |
| Folders that will not sync, with reasons | `EnrollPreview.blocked` | — |
| What setting up does | `EnrollPreview.disclosures` | — |
| Problem that blocks setup | `EnrollPreview.problem`, `PreviewKind::Refused` | — |
| Groups to join | `EnrollPreview.groups` (`name`, `product`) | — |
| Consent digest | `EnrollPreview.digest` (never shown except in Details) | — |
| Dropbox folder, installations found | none (paths come from command-line options today) | UG-24 |
| What changed (reconfirm) | none | UG-16 |

**Actions.** Back; Continue; the two checkboxes; Create a new group; Join a
group; Set up this computer; Choose folder… (native picker); Try again
(re-inspect). Reconfirm: **Confirm and continue**, **Not now**.

Consent rules come from core (`consent_gap`): ticking scope alone is not enough
for an unfamiliar client; a changed preview (new digest) clears the ticks and
says so: "The installation changed while you were reading. Please check again."

**States.**

| State | What shows |
|---|---|
| Empty | No Dropbox found: "Dropbox was not found. WoW Sync needs the Dropbox desktop app. Install or start Dropbox, then choose **Try again**, or choose a folder yourself." No installations: "No World of Warcraft installation was found. Choose the folder that contains your game." Empty AddOns: "This installation has no add-ons yet. You can still set it up." |
| Working | "Checking this installation…" with progress; Continue disabled. During **Set up**: "Setting up…" and all controls disabled. |
| Waiting | Shared folder found but files online-only: "Some files in this folder are online-only in Dropbox. Make the folder available offline in Dropbox. WoW Sync will wait until they are downloaded." Continue allowed; joining shows the groups found so far. |
| Success | Finish page: "Setup complete. Syncing starts now." |
| Needs help | Unfamiliar client: identity checkbox appears with "WoW Sync does not recognise this game version yet. Check the details below and confirm them, or cancel." Ticking only scope shows the note "Confirm the game version too. Agreeing to the add-on list alone is not enough. Nothing has been changed." |
| Error | Refused layout: "This installation can't be synced safely: <problem>. Nothing has been changed." Only Back and Try again. Write failure: the runtime error sentence plus "Nothing was set up. Try again." |
| Offline/stale peer | Joining a group whose computers have not reported for a long time: each group shows "Last activity 3 weeks ago" (UG-24) so the user does not assume it is live. |
| WoW running | Setup may proceed (it writes no game files). The finish page says "WoW is running, so add-ons will be updated after you close it." |
| Paused | Not applicable before setup. Reconfirm while paused: confirming does not resume; the banner says "Syncing stays paused until you resume it." |

**Reconfirm** (`StatusSnapshot.reconfirm` is set, runtime `NeedsReconfirmation`):
the group's syncing is stopped until confirmed. It appears as a banner
("Confirm the game version to continue syncing") and as a Needs attention item.
The page reuses step 4 with the heading "Confirm the game version", a "What
changed" panel (UG-16; until then "The game installation changed since it was
set up."), and **Confirm and continue** / **Not now**. **Not now** leaves
syncing stopped and says so.

When reconfirmation is offered is core's decision (`confirm_enrolled`), not
the UI's:

| Change since setup | Core result | What the UI shows |
|---|---|---|
| The add-on file rule changed (`mapping_id`, e.g. `validated-plain/1` → `profile:wowf/1` once a verified Forever profile ships), so a different set of folders would sync | `StaleConfirmation` → reconfirm offered | Reconfirm page; "What changed: WoW Sync now uses a different rule for which add-on files this game version loads. 5 more folders would sync." with the new eligible/blocked lists |
| The recognition class changed (a recognised client is now unfamiliar, or the reverse) | `StaleConfirmation` → reconfirm offered | Reconfirm page with the identity checkbox |
| Only the build version changed (game patch) | Not a reconfirm (D-18 O-N2); syncing continues | Nothing to confirm. The new build appears in This computer → Game and in the computer's Details; compatibility of individual add-ons is core's Q-21 rule |
| New add-on folders appeared under the same rule | Not a reconfirm (scope covers the rule, not a frozen list) | They appear in Add-ons as usual |
| Product or folder changed (a different client) | `VariantChanged`; reconfirm is **not** offered (runtime test `changed_product_needs_help_without_a_doomed_reconfirm_offer`) | Error state: "This is not the game installation that was set up. Nothing is synced for it." No Confirm button |

---

## Needs attention

**Purpose.** One place for everything that needs the user, each scoped to what
it affects (R-06, R-12). What continues meanwhile follows
[Continuation scope](#continuation-scope).

Wireframe: [needs attention and decision](wireframes/decision.svg).

**List items** (most urgent first: error, then reconfirm, then needs help,
then decisions, then peer problems):

| Kind | Source | Title example | Button |
|---|---|---|---|
| Shared folder problem | `provider_error` | "Can't read the shared folder" | **See what to do** |
| Confirm game version | `reconfirm` | "Confirm the game version to keep syncing" | **Confirm…** |
| Interrupted update needs help | `PackageStatus::NeedsHelp` | "Deadly Boss Mods: an update stopped unexpectedly" | **See what happened** |
| Add-on decision | `PackageStatus::NeedsDecision` | "WeakAuras: can't tell which copy is newer" | **Choose a copy** |
| Another computer's report | `MemberSummary::{Unusable, Regressed, Invalid}` | "Laptop's latest report could not be used" | **See details** |
| Extra recovery copies (future) | `retained_extra_recovery`, UG-13 | "3 extra recovery copies are kept" | **See why** |

Each row: icon, title, one-line reason, "Affects: …", button. "Affects" is
"WeakAuras only" for a decision and "All updates on this computer" for an
interrupted update that needs help (Q-25).

**States.**

| State | What shows |
|---|---|
| Empty | "Nothing needs your attention." with a check icon in `ok`. |
| Working | List unchanged; resolved items disappear after the next cycle. |
| Waiting | Waiting conditions are not listed here (they need no user action); a link "See what is waiting" opens Add-ons filtered "Not up to date". |
| Success | Same as Empty. |
| Needs help | The list. Under it, the continuation sentence for the case ([Continuation scope](#continuation-scope)); if any interrupted update needs help, only the journal sentence is shown. |
| Error | The error item first; other items still listed with their last known reason. |
| Offline/stale peer | Not listed: silence is not a problem the user must fix. The Computers page shows it. |
| WoW running | Items stay. Decisions can be made while WoW runs; the result is applied after WoW closes, and the confirmation says so. |
| Paused | Items stay; making a decision while paused records it; it applies after resume. |

## Decision

**Purpose.** Choose for one add-on when core cannot (R-05, R-06, R-09, A-08).
Never a whole-profile choice.

**Content.**

- Title: "WeakAuras: choose which copy to keep".
- Why: a reason-specific sentence:
  - `Ambiguous`: "WoW Sync can't tell which copy is newer. Their version labels
    can't be compared safely (5.18.1 and 5.18.1-beta)."
  - Same label, different files: "Both copies say version 5.18.1 but their files
    differ, for example after a manual edit." (Q-22)
  - `PartialCandidate`: "Laptop's copy is missing folders that other copies
    have: WeakAurasOptions."
  - `DifferentClientBuild` / `NoCompatibleToc`: "This copy came from a computer
    with a different game build and does not say it supports this one." (the
    Q-21 rule is core's)
  - `UnvalidatedTocMapping` / `DifferentTocMapping`: "This add-on has
    game-version-specific files that WoW Sync does not handle yet."
- **Copies** (one card per candidate, from `Explanation::AmbiguousVersions.
  candidates` and `evidence`): version label, which computers hold it ("This
  computer and Laptop"), folder differences, per-folder evidence outcome.
- **What happens when you choose** (from core, UG-8): "The chosen copy is
  installed on every computer in this group. Other copies are kept as recovery
  copies."
- Buttons per card: **Keep this copy**. Page: **Decide later**.
- Scope line: "Only WeakAuras is affected. Other add-ons keep syncing." Shown
  only when no interrupted update needs help on this computer; otherwise "Your
  choice is kept, but no updates are applied on this computer until the
  interrupted update is resolved." ([Continuation scope](#continuation-scope),
  UG-25).
- Details: hashes, folder keys, computer IDs.

In the current core, decisions are read-only (Q-25, UG-8). The prototype shows
the flow with fake data; the build-out shows **Keep this copy** disabled with
"Choosing is coming later; nothing has been changed" until UG-8 exists.

**States.**

| State | What shows |
|---|---|
| Empty | The decision was resolved elsewhere: "This decision is no longer needed." Back. |
| Working | After **Keep this copy**: "Saving your choice…", buttons disabled. |
| Waiting | Choice recorded but not applied: "Your choice is saved. It will be applied on the next sync." |
| Success | "Laptop's copy of WeakAuras will be used on every computer." with a link to the add-on. |
| Needs help | The default state of this page. |
| Error | Choice could not be saved: the error sentence plus "Nothing has been changed. Try again." |
| Offline/stale peer | A candidate held only by a silent computer: "Only on Den PC, not heard from for 12 days. Its files may not be downloadable now." |
| WoW running | Choosing is allowed: "Your choice is saved and will be applied after you close WoW." |
| Paused | "Your choice is saved and will be applied when you resume syncing." |

## Needs-help detail

**Purpose.** An interrupted update stopped in a state core will not resolve
alone (`JournalPhase::NeedsHelp`, apply.md §4, Q-25).

- What happened: `render(Explanation::UnknownInterruptedState)` and the journal
  reason, in plain words: "An update of Deadly Boss Mods stopped in an unexpected
  state. Both the previous files and the new files are kept."
- What it affects: "No updates are applied on this computer until this is
  resolved. Sharing continues." (Q-25: any journal blocks new applies here.)
- Choices: from core (UG-9), e.g. **Try again**, **Keep the previous
  version**. Read-only until UG-9; the prototype fakes them.
- Details: operation ID, journal phase, staged/recovery paths.

States follow the Decision table, with Needs help as the default.

---

## Activity

**Purpose.** Review what happened: automatic version choices, applied
updates, sharing, removals (future), rejections (future), recoveries,
interruptions and decisions (R-12, A-25).

Wireframe: [activity](wireframes/activity.svg).

**List.** Newest first, grouped by day. One entry per sync batch ("Updated 3
add-ons from Laptop"), expandable to its items; single events (setup, pause,
decision, interruption, recovery) are their own entries. Filter buttons:
**All**, **Changes**, **Problems**. Each item links to its add-on.

| Element | Source | Gap |
|---|---|---|
| Last cycle's lines | `SyncReport.{recovered, applied, apply_note, published, publish_note}` (`Shared.report_lines`) | — |
| History beyond the last cycle | local audit log `wowsync.audit/1` | UG-7 (reader API, structured kinds, batch IDs) |
| Item sentences | `render_result`, `render(Explanation)` | UG-22 |

**States.**

| State | What shows |
|---|---|
| Empty | "Nothing has happened yet. Changes appear here after the first sync." |
| Working | A top entry "Syncing now…" that becomes the batch entry when done. |
| Waiting | Entry "Waiting: WoW is running (2 updates)", deduplicated: one entry per unchanged condition, with "since 21:02". |
| Success | Batch entries with ✓. |
| Needs help | Problem entries in `help` with a link to the item. |
| Error | Entry "Could not read the shared folder" once per condition, with a "since" time. |
| Offline/stale peer | No entries for silence. An entry when a long-silent computer reports again: "Den PC reported again after 12 days." |
| WoW running | "Interrupted because WoW started" entries say so (`InterruptedByGame`); interruptions while the game is not running do not blame it (`Interrupted`). |
| Paused | Entries "Syncing paused" and "Syncing resumed" with times. |

---

## Settings sync (coming later)

**Purpose.** Show the planned settings controls so the owner can judge the
structure. Marked **Coming later** everywhere; nothing on this page works yet.
The slice syncs add-on code only and never touches `WTF`.

Wireframe: [settings sync placeholder](wireframes/settings-sync.svg).

Page banner: "Settings sync is not available yet. Only add-on files are synced
now; your add-on settings stay as they are on each computer."

Planned panels (all controls disabled, each with "Coming later"):

1. **Settings master** (R-07): "Desk PC shares its add-on settings with the
   group automatically. Choosing a master shares settings for every add-on,
   including add-ons found later." Control: **Change master…**. Absent master
   (A-45): "The settings master Den PC was retired. Choose a new master; WoW
   Sync does not pick one for you."
2. **Exceptions** (R-07): per-add-on "Shared from a different computer" list.
3. **Share my settings** (R-08 explicit push): "Send this computer's settings
   for selected add-ons to every computer, including the master." Control:
   **Share settings…**.
4. **Pause settings** (R-08): per add-on and **Pause all settings on this
   computer**, with "Protected settings are not shared or replaced. When you
   resume, you'll see any differences first."
5. **Settings choices** (R-09): "When two computers deliberately changed the
   same add-on's settings, you choose which to keep. Settings are never
   merged."

**States.** Only the placeholder state exists now. When built: Empty (no master
chosen), Working (sharing settings), Waiting (session not finished: "Waiting
for your play session to end"), Success, Needs help (competing choice), Error,
Offline/stale peer (master not heard from), WoW running (settings captured
after the session), Paused (settings paused: "Not shared, not replaced", never
"in sync", A-15). Data: entirely future (UG-21).

---

## App settings

**Purpose.** Control the app itself (R-13, R-15).

Wireframe: [app settings](wireframes/app-settings.svg).

| Panel | Controls | Source / gap |
|---|---|---|
| Startup | **Start WoW Sync when I sign in** (toggle, A-01); **Start hidden in the tray** (toggle) | UG-23 |
| Syncing | **Pause syncing** / **Resume syncing** button with the current state; **Sync now** | UG-2 |
| Appearance | **Theme**: Follow system (default) · Light · Dark (three toggle buttons acting as a group, Q-34) | GPUI `window.appearance()`, Q-35 |
| Notifications | **Show a summary after each sync** (toggle, on); **Tell me when something needs me** (toggle, on); fixed text: "Notifications wait while WoW is running, except problems that risk your data." | Q-30, [interaction.md](interaction.md#notifications) |
| Window | **Explain close-to-tray again next time** (button) | Q-26 |
| Privacy | **Send anonymous crash reports** (toggle, future, R-15) with **What is sent?**; shown disabled "Coming later" until telemetry exists | future |
| Data | Shared folder path with **Open**; local data folder with **Open**; **Export diagnostics…** (future, A-33, disabled) | `ServiceConfig` |
| Quit | **Quit WoW Sync** | — |

**States.** Empty: n/a. Working: toggles that write OS settings show a spinner
until written. Waiting: n/a. Success: a short "Saved" next to the control.
Needs help/Error: "Could not turn on start at sign-in: <reason>"; the toggle
returns to its previous position. Offline/stale peer: n/a. WoW running: no
change. Paused: the Syncing panel shows "Paused since 21:04" with Resume.

## About and updates

**Purpose.** Version, honest unsigned-build status, and manual update guidance
(A-28, R-16).

Wireframe: [about](wireframes/about.svg).

| Element | Source / gap |
|---|---|
| "WoW Sync 0.1.0 (build e8bc16a)" | `ServiceConfig.app_version`; commit needs a build-time value (UG-18) |
| Unsigned notice: "This build is not signed by a publisher. Your system may warn you when installing it. Do not turn off system protections; follow the install guide." | static text |
| Updates: "You have the newest version (checked 2 hours ago)" / "Version 0.2.0 is available" with **Open download page** and **What's new** | UG-18, Q-36 |
| **Check for updates automatically** (toggle) with the disclosure "Checking contacts GitHub releases, which sees your IP address." | Q-36 |
| "WoW Sync never replaces itself. To update, download the new version and install it; your sync data and recovery copies are kept." | static text (A-28, R-16) |
| Licence: MIT, with third-party notices | static/generated |
| Developer-build banner (slice): "Developer build: synthetic installs only." | `Root::banner_text` |

**Actions.** Check for updates; Open download page (system browser); Copy
version info.

**States.**

| State | What shows |
|---|---|
| Empty | Never checked: "Not checked yet." **Check for updates**. |
| Working | "Checking for updates…" |
| Waiting | n/a |
| Success | "You have the newest version." or "Version 0.2.0 is available." with manual steps. |
| Needs help | A newer version is required by shared data (R-16): "Another computer uses a newer WoW Sync. Update this computer to keep syncing." with the download link. |
| Error | "Could not check for updates: no connection." Nothing else changes. |
| Offline/stale peer | n/a |
| WoW running | No change; update notifications are deferred like other nonurgent ones. |
| Paused | No change. |

---

## Tray states

The tray is not a screen, but it has states. Shapes are in
[style.md](style.md#status-icons).

| State | Icon | Tooltip / menu headline |
|---|---|---|
| Empty (not set up) | base icon with dot | "WoW Sync: Set up this computer" |
| Working | arrows | "WoW Sync: Syncing" |
| Waiting | hourglass | "WoW Sync: Waiting, WoW is running" (or other reason); decisions only: "WoW Sync: Waiting for your decision on 2 add-ons" (core `Waiting`, UG-26) |
| Success | check | "WoW Sync: This computer is up to date" |
| Needs help | exclamation | "WoW Sync: An update needs your help" |
| Error | cross | "WoW Sync: Can't read the shared folder" |
| Offline/stale peer | unchanged (success if this computer is fine) | headline unchanged; peers never change the tray icon |
| WoW running | hourglass | "WoW Sync: Waiting, WoW is running" |
| Paused | two bars | "WoW Sync: Paused" |

---

## Data gaps

Data a screen needs that core or the runtime does not expose. Each suggested
field is a suggestion for the owning packet; none is UI policy.

| ID | Gap | Needed by | Suggested field / API |
|---|---|---|---|
| UG-1 | The runtime passes the status as strings (`StatusSnapshot.headline`, `packages[].text`, `members: Vec<String>`); the UI cannot pick colours, counts or details without parsing text | all | Add `view: StatusView` (or a UI-neutral struct with `Headline`, `PackageStatus`, `Explanation`, `MemberStatus`) to `StatusSnapshot`; keep the strings for the CLI/logs |
| UG-2 | No paused state | header, tray, Overview | `StatusSnapshot.paused: Option<PausedSince>` and a runtime `pause()/resume()` that persists; headline override "Paused" decided in core (`Headline::Paused` or `render_headline` input) |
| UG-3 | Add-on display title and version here / target | Add-ons, detail | `PackageRow { title: String /* toc Title, colour codes stripped */, version_here: Option<String>, version_target: Option<String>, source: Option<ComputerId>, folders: Vec<String> }` |
| UG-4 | Who is still expected to confirm a package | Computers, add-on detail, Overview | `awaiting: Vec<ComputerId>` per package = enrolled non-retired members without a confirming report; computed in core |
| UG-5 | Time units and absence | Computers | Document `seen_locally_at` as Unix ms local clock; `None` rendered "never" |
| UG-6 | Per-member summary: confirmed count, unconfirmed packages, app version, game build, enrolled date | Computers | `MemberStatus { confirmed: u32, of: u32, unconfirmed: Vec<FolderKey>, app_version, client_build, enrolled_unix_ms }` |
| UG-7 | No reader for the local audit log, no batch grouping | Activity | `runtime::activity(limit, before) -> Vec<ActivityEntry { at_unix_ms, batch: Option<BatchId>, kind: ActivityKind, package: Option<FolderKey>, why: Explanation }>` |
| UG-8 | No decision API (Q-25: read-only in the slice) | Decision | `decision_options(package) -> Vec<DecisionOption { id, label, holders, consequence: Explanation }>` and `decide(package, option_id)` |
| UG-9 | No needs-help resolution API | Needs-help detail | `needs_help_options(op) -> Vec<HelpOption>` and `resolve(op, option)` |
| UG-10 | Game activity is a string | Overview, banners | `activity: Activity { NotRunning, Running { since }, Unknown(reason) }` |
| UG-11 | Provider error is a string; no category or next step | Overview, Needs attention | `provider_problem: Option<ProviderProblem { kind: NotFound / Offline / OnlineOnly(count) / PermissionDenied / Moved / OtherWriter, next_step }>` |
| UG-12 | No "next action" per item | add-on detail, Needs attention | `fn next_action(&PackageStatus, &Explanation) -> NextAction` in core (A-25) |
| UG-13 | Recovery copies are only a count | add-on detail | `recovery: Vec<RecoveryPointView { created_unix_ms, version_label, reason, pinned, exception: Option<Explanation> }>` per package |
| UG-14 | No last/next check time, no cycle progress | header, Overview | `last_cycle_unix_ms`, `next_cycle_unix_ms`, `progress: Option<(done, total)>` |
| UG-15 | One group per app instance | sidebar, Overview | `groups: Vec<GroupSummary { id, name, product, headline }>` |
| UG-16 | Reconfirm does not say what changed or why it is needed | Reconfirm | `ReconfirmCause { MappingChanged { from, to }, RecognitionChanged { from, to } }` plus the eligible/blocked difference; build-only changes are not a cause (D-18 O-N2) |
| UG-17 | No safe-point signal for pause/quit | Pause, Quit | runtime `request_stop() -> StopState { Now, AfterCurrentStep(package) }` and a "stopped" event |
| UG-18 | No update information or build commit | About | `UpdateInfo { current, latest, page_url, checked_unix_ms }`; build commit via build script |
| UG-19 | No batch summary events for notifications | notifications | `SyncBatchSummary { applied: Vec<(FolderKey, from, to, source)>, shared: bool, problems: Vec<ProblemKey> }` per completed cycle |
| UG-20 | No retirement or master-absence data (A-45, future) | Computers, Settings sync | `MemberStatus.retired: Option<unix_ms>`, `retire(computer)`, `master_absent: bool` |
| UG-21 | Settings sync does not exist (future) | Settings sync | settings-sync design (R-07..R-09) |
| UG-22 | Core sentences show computer IDs ("added from computer 3f9a…") and an ambiguous Ok headline ("Everything is up to date") | all sentences | `render` takes a name resolver `Fn(&ComputerId) -> String`; Ok headline "This computer is up to date" |
| UG-23 | No start-at-sign-in API | App settings | runtime `startup::{get, set}(enabled, hidden)` (Run key / LaunchAgent) |
| UG-24 | Setup cannot discover Dropbox or WoW installations; paths come from command-line options | Setup | runtime `discover() -> { dropbox: Option<Path>, installs: Vec<InstallCandidate> }` and group last-activity in `GroupInfo` |
| UG-25 | No typed continuation scope: the runtime skips apply planning while any journal remains and exports no block, so the UI cannot tell "only this add-on" from "all applies on this computer stopped" (Q-25) | Overview, Needs attention, Add-on detail, Decision | `StatusView.apply_scope: ApplyScope { AllAppliesBlocked { by: PlanBlock::UnresolvedJournal, packages: Vec<FolderKey> } \| PerPackage }`, or the runtime always reporting `PlanBlock::UnresolvedJournal` in `blocks` |
| UG-26 | Decision headline policy: core derives `Headline::Waiting` when only `NeedsDecision` packages are non-current (`needs_decision_shows_evidence`), although the user must act. The UI renders core's value and does not decide this | header, tray | core decision (owner if product meaning changes): keep `Waiting`, or derive `NeedsHelp` for decisions |
