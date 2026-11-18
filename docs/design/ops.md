# Operations: decisions, retirement, notifications, updates and diagnostics

Part of the [slice design](README.md). This document was written by run-004
packet R4-ops-design. It covers five operational features of the GUI app:

| § | Feature | Requirement | Acceptance |
|---|---|---|---|
| 1 | Resolving decisions in the GUI, with the headline promotion (UQ-1) and the typed hold scope (UQ-2) | R-06, R-13 | A-26 |
| 2 | Retiring a computer | R-03, R-07, R-12 | A-45 |
| 3 | OS notifications | R-13 | A-27 |
| 4 | The app update notice | R-13, R-15, R-16 | A-28 |
| 5 | Diagnostics export | R-15 | A-33 |
| 6 | Implementation map (types and functions per module) | E-01, E-06 | — |
| 7 | Test list | E-03, E-04 | A-26, A-27, A-28, A-33, A-38, A-45 |
| 8 | GUI touchpoints | R-13 | A-26 |

**Authority.**
- Product intent comes from [SPEC.md](../../SPEC.md).
- Approvals:
  - D-21 is the run-003 owner review.
  - D-23 answers UQ-1 and UQ-2 with "yes".
  - D-24 approves P4-1 (the GitHub Releases check) and P4-2 (an OS
    notification crate).
- This document does not change product meaning. Choices the approved intent
  does not settle are listed as Q-59 to Q-66 in [questions.md](questions.md),
  each with a safe default.

**Inputs this design builds on.**
- Removal records, decisions and the selection constraint come from
  [removal.md](removal.md), in particular:
  - §2.2: immutable authority objects;
  - §2.5: validation V1 to V7;
  - §3.3: decisions;
  - §8.2: retirement interactions.
- Screen layout comes from [ui/screens.md](ui/screens.md).
- Notification rules come from
  [ui/interaction.md](ui/interaction.md#notifications).
- The settings master, per-addon exceptions and pushes are defined by
  `settings.md`, which R4-settings-design writes in parallel. This document
  only uses its terms **settings master** and **reassign**, and the
  interface named in §2.6.
- If the parallel R4-removal-design rewrite renames a removal schema or
  section, the merged names win. The meaning here does not change.

---

## 1. Resolving decisions in the GUI (A-26, R-06, R-13)

### 1.1 One option model for every decision kind

Every item in **Needs attention** that asks the user to choose is shown on the
Decision page through one generic interface (UG-8). The GUI never builds
options itself. It only renders what core returns and passes the chosen
option back.

```rust
// wow-sync-core::decide
pub struct DecisionItem {
    pub package: PackageKey,          // or SettingsScope for settings kinds (settings.md)
    pub kind: DecisionKind,
    pub reach: DecisionReach,         // Group | ThisComputer (§1.6)
    pub options: Vec<DecisionOption>,
    pub evidence: EvidenceToken,      // hash of everything the options were built from
}
pub struct DecisionOption {
    pub id: OptionId,                 // stable within one EvidenceToken
    pub label: OptionLabel,           // typed, rendered by the app (no free text)
    pub holders: Vec<MemberRef>,      // computers that hold this copy now
    pub consequence: Consequence,     // typed: installs, removals, recoveries, declined versions
    pub warning: Option<OptionWarning>,
}
pub fn decision_item(view: &StatusView, ledger: &LedgerView, package: &PackageKey) -> Option<DecisionItem>;
pub fn decide(item: &DecisionItem, option: &OptionId, shown: &EvidenceToken, issuer: Issuer)
    -> Result<DecisionEffect, DecideError>;
```

- `decide` is pure. It returns either:
  - `DecisionEffect::Group(AuthorityObject)`, the immutable object to write;
    or
  - `DecisionEffect::Local(LocalResolution)`, a journal action for this
    computer only.
- `DecideError` has these variants:
  - **`Changed`:** the evidence changed between showing the options and
    choosing.
  - **`NotAMember`:** this computer is retired or not enrolled (§2.5).
  - **`UnknownOption`.**
  - **`NotOffered`:** for example, an option the kind does not allow.
- "Something changed while you were choosing" is the GUI text for `Changed`.
  The page re-renders the new options and never applies a choice made
  against old evidence.

### 1.2 Actions per decision kind

`DecisionReason` (core `target.rs`) and the removal kinds of removal.md §3.1
map to these actions. Every group action is a decision authority object under
removal.md §3.3. The new code kinds below are added to its `kind` set. They
reuse `chose.keep` (a selection constraint) and `chose.automatic` unchanged.

| Kind | Shown when | Options (labels as rendered) | Records | Reach |
|---|---|---|---|---|
| `Ambiguous` | Two or more admitted copies share a version label (or cannot be ordered) but differ in content (domain.md §6) | One **Keep this copy** option per distinct copy. Each is labelled with its version text and the computers that hold it, for example "Keep the copy from Laptop and Den PC". Then **Decide later** | `kind: ambiguous_version`, `chose.keep {package_hash}`; the other copies go into `declined` (§3.3 formula) | Group |
| `PartialCandidate` | One copy lacks folders that another copy of the same version has | **Keep the complete copy**. **Keep the smaller copy (drops <N> folders)**: the dropped folders are removed into recovery as in removal.md §1.5. **Decide later** | `kind: partial_candidate`, `chose.keep` | Group |
| `NoCompatibleToc` | No admitted copy declares an interface version for this game variant | **Use it anyway**. Warning: "This add-on does not say it supports this game version. It may not load. WoW Sync does not judge add-ons; you decide." (D-21: WoW Sync is not an addon police). **Decide later** | `kind: no_compatible_toc`, `chose.keep {package_hash}`, `acknowledged: ["no_compatible_toc"]` | Group |
| `DifferentTocMapping`, `UnvalidatedTocMapping` | The variant's TOC mapping is not validated (R-02 safe scope) | **No choice.** Choosing would bypass safe-scope validation. The item is informational: "Waiting for a WoW Sync update that knows this game version." It never notifies (§3.2) and never promotes the headline | nothing | — |
| Removal kinds (`RemovalVsUpdate`, `CompetingRemovalDecisions`, `InconsistentRemovalRecord`, `NewVersionAfterDecision`, `OnlyDeclinedVersions`) | removal.md §3.1 | As listed in removal.md §7: **Keep it removed**, **Keep version V**, **Resume automatic updates** (with per-version **Allow again**) | removal.md §3.3 | Group |
| Settings conflicts | settings.md | Defined by settings.md. Competing intentional choices are not merged (R-09). Each option is one whole choice | settings.md | Group or ThisComputer, as settings.md states |
| `NeedsHelp` journal (UG-9) | An interrupted apply could not be finished or rolled back automatically (`PlanBlock::UnresolvedJournal`) | **Try again** (re-run journal recovery). **Put back the previous version** (restore the journal's recovery point). **Keep the new version**: offered only if the staged tree re-hashes to its manifest | local journal resolution entry | ThisComputer |

**Decide later** is always offered and records nothing. The item stays in
Needs attention, and it does not notify again while unchanged (§3.4).

### 1.3 Preconditions

| Condition | Group decision | Local journal action |
|---|---|---|
| This computer is retired or not enrolled | Not offered (`NotAMember`). Text: "This computer was retired. Join the group again to make choices for it." | Allowed. It is the user's own disk |
| WoW is running, or its state is unknown | **Allowed.** It is recorded and shared, because no game file changes. The apply waits: "Saved. WoW Sync applies it after you close WoW." | Only "Put back the previous version" and "Keep the new version" when they touch no game files. Otherwise the action is disabled with "Close WoW first" |
| Syncing is paused | **Allowed.** It is recorded in the local ledger as `unshared`. It is written to the shared folder on resume, and the text says so: "Saved. It is shared when you resume syncing." | Allowed |
| The shared folder is unavailable or offline | Allowed. The authority file is written when the folder is writable again. Until then the item shows "Saved here; not shared yet" | Allowed |
| The evidence changed since it was shown | `Changed`. Re-render the options | `Changed` |
| An unresolved journal exists for another package | Allowed. Applying is held by the journal (UQ-2 scope `AllApplies`, §1.6) | — |

### 1.4 What core records, and how peers see it

1. Core `decide` builds the decision object, including `considered`,
   `declined` and `knows`. It is validated by the same V1 to V7 rules every
   peer uses (removal.md §2.5). A computer never writes an object it would
   itself reject.
2. Runtime writes the object to `members\<self>\authority\<id>.json`
   (write-if-absent, read-back hash check). It then publishes the next own
   state, which references the object (removal.md §2.1, §2.3).
3. Runtime appends to the local audit log and to the activity log, for
   example "You chose to keep WeakAuras 5.12 from Laptop for the group."
4. **Peers** learn the object through the shared folder and mirror it on
   learning (Q-51). They then:
   - apply the selection constraint at their next safe apply;
   - drop the item from their Needs attention;
   - show an Activity entry with the issuer's display name, for example
     "Laptop chose to keep WeakAuras 5.12."
   - Its per-package story names the decision.
   - Dropbox publication alone is not shown as "applied" anywhere (R-08).
     The usual confirmations still come from peers' published states.
5. Local journal actions are recorded in the local journal and the activity
   log only. Peers see the result through this computer's next published
   state: the package is no longer `NeedsHelp` or `Interrupted`.

### 1.5 Undo and recovery

- **Group decisions are never edited.** "Change decision" on the package page
  opens the Decision page again with the current options, plus **Resume
  automatic updates**. The new choice is a new decision whose `knows`
  includes the old one (removal.md §3.3 formulas). Declined versions stay
  declined unless they are allowed again individually (Q-50).
- Every replacement or removal caused by a decision goes through the normal
  apply path. That keeps a recovery point: current plus two previous per
  package (R-10). **Restore** on the Recovery page works as before.
- A local journal action is undone by restoring the matching recovery point.

### 1.6 UQ-2: the typed hold scope (D-23: yes)

Core states how much is held, so the app never guesses.

```rust
// wow-sync-core::status
pub enum HoldScope {
    /// Nothing is held.
    None,
    /// Only these packages wait. Everything else keeps syncing.
    Packages(Vec<PackageKey>),
    /// No game-file change is applied on this computer until the cause clears.
    AllApplies(AllHoldCause),
}
pub enum AllHoldCause { UnresolvedJournal { packages: Vec<PackageKey> }, GameRunning, ActivityUnknown, Paused }
pub enum DecisionReach { Group, ThisComputer }
pub struct StatusView { /* existing fields */ pub hold: HoldScope }
impl PackageStatus { pub fn reach(&self) -> Option<DecisionReach> }
```

- `derive_status` computes `hold` from data it already has:
  - `PlanBlock::UnresolvedJournal`, `GameRunning` and `ActivityUnknown` give
    `AllApplies`. Pause is an input from runtime.
  - Otherwise, the set of packages in `NeedsDecision`, `NeedsHelp` or
    `Blocked` gives `Packages`.
- `AllApplies` takes precedence over `Packages`.
- App text:
  - **`Packages([p])`:** "Only WeakAuras is waiting. Everything else keeps
    syncing."
  - **`Packages(n)`:** "Only these 3 add-ons are waiting…"
  - **`AllApplies(UnresolvedJournal)`:** "Nothing is updated on this computer
    until this is resolved."
  - **`Group` reach:** the Decision page says "Your choice applies to every
    computer in the group."
  - **`ThisComputer` reach:** "This only affects this computer."
- No scope sentence is shown when `hold` is `None`. This replaces the
  "until core states it" rule in ui/screens.md (UG-25).

### 1.7 UQ-1: "Review needed" headline (D-23: yes)

- Add `Headline::ReviewNeeded` to core, and `StatusInput.reconfirm_pending:
  bool`. Runtime sets it from the guided reconfirmation state (the app's
  `ReconfState` source).
- **Precedence, highest first:**
  1. `NeedsHelp`;
  2. `Error`;
  3. `ReviewNeeded`;
  4. Paused (the app overlay);
  5. `Working`;
  6. `Ok` / `Waiting`.
- **Pause never hides `NeedsHelp`, `Error` or `ReviewNeeded`.** The first two
  are D-23. ReviewNeeded is also kept, because pausing does not answer the
  review.
- **Text:** "Review needed". Subline: "Confirm this computer's WoW folders so
  syncing can continue." The action button opens the reconfirmation.
- **Tray:** attention tone, the same shape as `NeedsHelp`, with the tooltip
  "WoW Sync: review needed".
- Pending decisions (`NeedsDecision`) do **not** promote the headline. They
  keep core's headline and the Needs attention count (Q-59, safe default).

---

## 2. Retiring a computer (A-45, R-03, R-07, R-12)

### 2.1 Retirement record

A retirement is an immutable, **group-scoped** authority object. The issuer
writes it in its own member directory, so the single-writer rule holds
(publication.md §1, removal.md §2.1):

```text
groups\<group_id>\members\<issuer computer_id>\authority\<id>.json
```

```json
{
  "schema": "wowsync.retirement/1",
  "id": "<32 hex>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 61 },
  "retired": { "computer_id": "…", "enrollment_id": "…" },
  "cutoff_seq": 47,
  "authority_known": [{ "id": "<32 hex>", "hash": "<object hash>" }],
  "was_settings_master": true,
  "written_unix_ms": 1767225600000
}
```

The record follows these rules:

- **No free text.** The display name is the retired computer's own
  `member.json` value, read at display time.
- **`cutoff_seq`** is the issuer's admitted watermark for the retired
  enrollment: its last admitted seq (core `admit` watermarks).
- **Self-retirement.** When a computer retires itself, `cutoff_seq =
  issued_at_seq − 1`.
- **`authority_known`** lists the variant references of that enrollment's
  authority objects in the issuer's ledger (removal.md §8.2, V3).
- **Hashing, writing and mirroring** follow removal.md §2.2 and §2.4. The
  object is SHA-256 hashed, written write-if-absent and mirrored by every
  computer on learning (Q-51). Retirement therefore survives even if the
  issuer's directory is unusable later.
- Readers find retirement objects by listing `authority\` of every member
  directory, as for removal objects.

### 2.2 Validity

The R rules mirror removal V1, V2 and V3.

| # | Rule | Otherwise |
|---|---|---|
| R1 | Parses, schema major is known, size ≤ 64 KiB, and the id has exactly one variant | conflicted → **ignored**; the GUI shows "Two different retirement records exist for Den PC" in Computers, with **Retire again** (writes a fresh record) |
| R2 | The issuer enrollment and the retired enrollment are both enrollments in this group | `invalid(not_a_member)`, diagnostic only |
| R3 | If the **issuer** enrollment is itself retired by a valid record with `cutoff_seq < issued_at_seq` (and the id is not in that record's `authority_known`), this record is invalid | `invalid(after_retirement)` |

**R3 can be circular**, for example when A and B retire each other
concurrently. Validity is computed as a grounded labelling:
- A record is `valid` if every record that could invalidate it is `invalid`.
- A record is `invalid` if some `valid` record invalidates it.
- Records left undecided in a cycle are **treated as valid**, so every
  enrollment in the cycle is retired.

Retirement only removes authority, so this choice fails safe. A computer
someone meant to retire cannot keep publishing because of a race. The GUI
says: "Laptop and Den PC retired each other at the same time. Join again on
the computer you still use." (Q-61.) Every computer derives the same result
from the same set of records.

Several valid records may retire the same enrollment. The effective cutoff is
the **minimum** `cutoff_seq`, and the union of the `authority_known` lists
applies.

### 2.3 Effects for every reader

Once a valid retirement of enrollment E exists:

1. **Stale and offline publications are ignored.** States of E with
   `seq > cutoff` are not admitted. They are neither candidates nor
   confirmations, and they cannot reactivate E. This holds whenever they are
   delivered, including a delayed Dropbox sync from the retired computer.
2. **History remains.**
   - E's admitted states ≤ cutoff, `member.json`, authority objects valid
     under V3, and the provider objects they reference are not deleted.
   - The retirement deletes no shared collection (R-03).
   - E's member directory stays in the shared folder.
3. **E contributes no candidates.** E no longer offers versions for target
   selection. Its earlier removal authority stays valid only as removal.md
   §8.2 says.
4. **Currentness no longer waits for E.** E is excluded from:
   - `awaiting` and confirmation counts (`status::confirmations`);
   - "Waiting for <computer>" text;
   - removal settling (removal.md §2.7);
   - the 72-hour "not heard from" emphasis (Q-27).
5. **Display.** Computers lists E under **Retired** with "Retired on 12 Jan
   by Laptop. History kept." E has no status dot and is not counted in "N
   computers". Its old activity remains.
6. **Re-enrollment.** A new enrollment of the same `computer_id` is a new
   EnrollmentId and therefore a joining member (removal.md §8.3). The old
   retirement never applies to it.

### 2.4 On the retired computer itself

When a computer admits a valid retirement of its own current enrollment:

- It **stops publishing** immediately. No state, no authority objects and no
  mirrors are written, and nothing is uploaded on its behalf.
- It **stops applying** group changes (Q-60).
- Local addons, settings, recovery points and logs stay **untouched**.
  Nothing is uninstalled.
- The headline becomes `NeedsHelp`-tone **"This computer was retired"**, with
  "Retired on 12 Jan by Laptop. Your add-ons and settings here are kept.
  Join the group again to sync." The only action is **Join again…**, which
  runs the explicit join flow (A-04, guided identity and safe-scope
  validation) with a new EnrollmentId.
- A pending decision or push on this computer is discarded with an Activity
  entry. It had not been shared yet (§1.3).

### 2.5 GUI flow (A-45, A-26)

The flow lives at Computers > select a computer > **Retire this computer…**.
The same command appears on this computer's own detail page as **Retire this
computer** (self-retirement).

1. **The sheet title** is "Retire Den PC?"
2. **The body lists the facts:**
   - "Den PC will stop sharing changes. Other computers stop waiting for it."
   - "Its history and recovery copies are kept. Nothing is deleted from the
     shared folder."
   - "To use it again, join the group again from that computer."
3. **Recent report warning.** If the computer reported within the last 72
   hours (Q-27), the sheet adds: "Den PC reported 3 hours ago. Retire only
   computers you no longer use, or that are lost." This is a warning, not a
   block (Q-65).
4. **Settings master paragraph** (§2.6), if the computer is the settings
   master.
5. **A confirmation checkbox:** "I understand Den PC stops syncing." The
   **Retire** button stays disabled until it is checked. **Cancel** is the
   default focus.
6. **Result.**
   - Runtime calls core `build_retirement(...)`, writes the record and
     publishes the next own state.
   - The Activity entry reads "You retired Den PC."
   - Computers moves Den PC to **Retired**.
   - Not available while paused or retired: "Resume syncing to retire a
     computer" or "This computer is retired". Retirement must be shared to
     have any effect.
7. **Undo** is re-enrollment of that computer. A retirement is never deleted
   or edited.

### 2.6 Retiring the settings master

This section uses the settings.md terms.

- The interface is `settings_master(view) -> MasterState`, with:
  - `Present(EnrollmentRef)`;
  - `Absent { was: EnrollmentRef, why: MasterAbsence::Retired { at, by } }`;
  - `NotChosen`.
- **On the sheet**, before confirming:
  - "Den PC is the settings master. After retiring it, settings are not
    shared automatically until you reassign the settings master. Nothing
    changes on the other computers' settings."
  - The sheet does **not** offer to pick a new master inline. Choosing is a
    separate explicit action. That keeps "retire" from ever implying a
    switch.
- **After retirement, on every computer:**
  - `MasterState::Absent`;
  - master auto-publication stops;
  - non-master computers keep their current settings;
  - per-addon exceptions and **explicit peer pushes** still work;
  - the master's published settings history is not deleted.
- **Banner on Dashboard and Settings:** "Settings master retired. Den PC was
  the settings master. Reassign the settings master to share settings
  automatically again." It has the button **Reassign settings master…**,
  which opens the settings.md flow.
- **Never auto-switch.** No computer becomes settings master by:
  - age;
  - being the last one left;
  - being the issuer of the retirement;
  - a timestamp.

  This holds even when only one active computer remains. Tests assert it
  (T-45-4).

---

## 3. Notifications (A-27, R-13)

### 3.1 Behaviour

This follows ui/interaction.md, Notifications, which this design makes
implementable.

**Kinds.** There are four kinds of notification:
- **batch summary**;
- **action needed**;
- **data at risk**, which is urgent;
- **app update**.

**Events that never notify:**
- sharing this computer's state;
- a batch with no change here;
- waiting conditions;
- a silent peer;
- pause or resume;
- informational TOC-mapping items (§1.2);
- retirement of *another* computer (it shows in Activity).

**Retirement of this computer notifies once**, as action needed.

### 3.2 Pure planner in core

```rust
// wow-sync-core::notice
pub enum NoticeKind { Summary, ActionNeeded, DataAtRisk, AppUpdate }
pub struct NoticeKey { pub kind: NoticeKind, pub scope: NoticeScope }   // Package(key) | Computer(id) | SharedFolder | Update(version) | Batch
pub struct NoticeEvent { pub key: NoticeKey, pub fingerprint: [u8; 32], pub text: NoticeText }
pub struct NoticeLedger { open: BTreeMap<NoticeKey, Fingerprint>, cleared: VecDeque<(NoticeKey, i64)>, held: HeldSet }
pub struct NoticePrefs { pub summaries: bool, pub needs_me: bool }
pub fn plan_notices(ledger: &NoticeLedger, events: &[NoticeEvent], open_now: &BTreeSet<NoticeKey>,
                    game: GameActivity, prefs: NoticePrefs, now_unix_ms: i64) -> (Vec<Notice>, NoticeLedger);
```

The planner is pure: time and game state are inputs. Rules:

1. **First appearance or changed content.** A key notifies when it first
   opens or when its fingerprint changes, for example when a new candidate
   appears for the same decision.
2. **No repeats while unchanged.** An unchanged open key never notifies
   again (A-27).
3. **Recurrence.** A recurrence after the key cleared notifies only if at
   least **6 hours** have passed since that key last notified. This stops
   flapping.
4. **One summary per completed batch** that changed something here. It lists
   at most three names, then "and N more". Several new action items from one
   batch become **one** action notification with a count.
5. **Gaming deferral.**
   - **When it applies:** whenever `GameActivity` is `Running` or
     `Unknown`. `Unknown` maps from `PlanBlock::ActivityUnknown`; "can't
     tell" is treated as "maybe playing".
   - **What is held:** nonurgent notices go into `held`.
   - **Urgent notices:** `DataAtRisk` is sent once, even while gaming.
   - **Release:** when activity becomes `NotRunning`, the held notices are
     released as **at most two**:
     - one combined summary ("While you played: …");
     - one action notification for items **still open**.
   - Items resolved meanwhile are dropped.
6. **Bounds.**
   - `held` keeps counts plus at most five names per kind.
   - `cleared` keeps the last 100 keys.
   - `open` is bounded by the open conditions themselves (R-14).
7. **Preferences.**
   - `summaries = false` suppresses `Summary`.
   - `needs_me = false` suppresses `ActionNeeded` and `AppUpdate`.
   - `DataAtRisk` cannot be turned off in the app; the OS can turn it off.
   - Suppressed keys are still recorded as notified. That stops a burst when
     the user turns the toggle on again.

**Persistence and display.**
- Runtime persists `NoticeLedger` in local state (`notices.json`, at most
  64 KiB) so restarts do not repeat notices.
- The in-app status (headline, tray icon and tooltip, Needs attention)
  updates immediately whatever the planner decides (A-27: actionable status
  remains available).

### 3.3 Delivery and the crate choice (D-24 P4-2)

**Chosen: `notify-rust =4.19.0`** (MIT OR Apache-2.0), behind an adapters
trait:

```rust
// wow-sync-adapters::notify
pub trait Notifier { fn show(&self, n: &RenderedNotice) -> Result<(), NotifyError>; }
pub struct OsNotifier { app_id: &'static str }   // notify-rust
pub struct RecordingNotifier { pub shown: Mutex<Vec<RenderedNotice>> } // testkit
```

**Why notify-rust (E-06):**
- **Widely used and maintained.** It is the long-standing general Rust
  desktop notification crate, actively released in the 4.x line, and GPUI
  (Zed) already depends on it for Linux.
- **One API for both platforms:**
  - Windows toast through `tauri-winrt-notification 0.8.1`, which uses
    `windows 0.62`;
  - macOS through `mac-notification-sys 0.6.15`;
  - Linux D-Bus through `zbus`. This is not on our targets.
- **Small footprint.** It and both platform backends are **already in
  `Cargo.lock`**, through GPUI's Linux platform crate. Adding it adds **no new
  crate versions** for Windows or macOS (checked against the scratch lock;
  see the evidence).
- **Licences.** MIT/Apache-2.0 throughout.
- **cargo deny.** `cargo deny check` passes with this repository's
  `deny.toml` for all four targets.
- **Free.** There is no service and no network use.

**Rejected alternatives:**
- **Hand-written WinRT toast and macOS UserNotifications FFI.** This needs
  `unsafe` code on two platforms and more maintenance, for the same result.
- **`tauri-plugin-notification`.** It is tied to the Tauri runtime.
- **`win-toast-notify` and `winrt-toast`.** They are less used, Windows only,
  and still need a macOS crate.

**Platform caveats (native verification required, Q-62 and Q-63):**
- **Windows identity (AUMID).** An unpackaged app needs an AppUserModelID,
  or Windows attributes toasts to PowerShell (the backend's default) or drops
  them.
  - At startup, runtime registers
    `HKCU\Software\Classes\AppUserModelId\Hamiltonia.WoWSync` with
    `DisplayName` = "WoW Sync" and `IconUri` = the installed icon. It then
    calls `SetCurrentProcessExplicitAppUserModelID` (adapters `winproc`,
    `windows-sys`, adding the `Win32_UI_Shell` feature).
  - The user's uninstall guide removes the key.
  - Fallback: a Start menu shortcut carrying the AUMID, if a later installer
    exists.
- **Click-through.** notify-rust does not expose activation callbacks on
  Windows or macOS.
  - Clicking a notification brings Windows' default handling.
  - The in-app status is always current, and each notification's text says
    where to look ("Open WoW Sync to choose").
  - Opening a specific page on click needs the backend's activation hook
    (`tauri-winrt-notification` `on_activated`). That is a later, optional
    step behind the same `Notifier` trait.
- **macOS.** `mac-notification-sys` uses `NSUserNotificationCenter`, which is
  deprecated but still delivered.
  - It is verified on the owner's Mac (D-23).
  - If delivery fails, the macOS fallback is in-app plus the tray badge only,
    with a "Notifications are not available on this Mac" line in App
    settings.
  - `mac-notification-sys` compiles Objective-C (`cc`), so macOS targets
    build on macOS hosts. CI already does this (`ci.yml` build-targets on
    `macos-latest`). It cannot be cross-checked from Windows (see the
    evidence).
- **Error handling.** A delivery error is logged once per kind per session
  and never retried in a loop. It never changes sync behaviour.

---

## 4. App update notice (A-28, R-13, R-15, R-16; D-24 P4-1)

### 4.1 The request

| Item | Value |
|---|---|
| URL | `https://api.github.com/repos/hamiltonia/wow-sync/releases/latest`. This is a compile-time constant and the only URL |
| Method | `GET`, HTTPS only. Redirects are followed only to `https` (at most 3) |
| Headers | `User-Agent: WoW-Sync` (GitHub requires one; **no version and no id**), `Accept: application/vnd.github+json` |
| Not sent | No cookies (no cookie store), credentials, tokens, query strings, install id, computer id, group id, OS user name or proxy credentials. Proxy settings from the environment are not read |
| Timeouts | connect 10 s; whole request 20 s |
| Response bound | Read at most **256 KiB**. Larger counts as a failure. The response is parsed only as JSON data, never executed or rendered as HTML |
| Rate | Automatic: at most **once per 24 hours** (§4.2). GitHub's unauthenticated limit is 60 per hour per IP, so a 403 or 429 is an ordinary failure |

**What GitHub sees** is the user's IP address and the request above, nothing
else. The About page discloses this (Q-36): "Checking contacts GitHub
releases, which sees your IP address." This is the unavoidable transport
metadata R-15 asks us to document. GitHub's handling is described by its own
privacy statement. No other WoW Sync network call exists (D-23).

### 4.2 Schedule and opt-out

- **Setting.** `AppSettings.update_check: bool` defaults to **on** (Q-36,
  D-21). The toggle is **Check for updates automatically**, on About.
- **When off,** no automatic request is ever made, including at startup.
  Tests assert zero transport calls.
- **Persisted state.** Runtime stores `update_check.json` in local state
  (≤ 4 KiB) with:
  - `last_attempt_unix_ms`;
  - `last_result`: `Newest { checked }`,
    `Available { version, tag, checked }`, or `Failed { at }`;
  - `notified_version`.
- **When an automatic check runs** (core `update::due`): when enabled, **and**
  the app has been ready for at least 60 seconds (so it never competes with
  the ten-second sync start), **and**:
  - no attempt has been recorded; or
  - the last attempt is ≥ 24 hours ago; or
  - the last attempt is more than 24 hours in the future (clock change).

  Failures count as attempts, so offline machines also try at most daily.
- **Check now.** **Check for updates** on About is an explicit request, so
  it is allowed even when automatic checks are off. It is debounced to once
  a minute. It shows "Checking for updates…" and then the result.
- **The check runs on a background thread.** It never blocks sync, the UI or
  quit, and quit does not wait for it.

### 4.3 Results and failure

- **Parsing.**
  - Core `update::parse_latest(bytes) -> Result<ReleaseInfo, UpdateParseError>`
    reads only `tag_name`, `draft` and `prerelease` and ignores everything
    else. `name`, `body` and `html_url` are **not** used.
  - The tag must match `v?MAJOR.MINOR.PATCH` (digits only, each ≤ 9 digits).
    Drafts, prereleases and anything else are `Failed`.
- **Comparison.** `update::compare(current, latest)` uses numeric semver
  triples. The app version comes from `CARGO_PKG_VERSION`.
- **Opening the page.** **Open download page** opens
  `https://github.com/hamiltonia/wow-sync/releases/tag/v<MAJOR.MINOR.PATCH>`,
  built from the parsed numbers in the system browser. A tampered response
  therefore cannot point the user elsewhere.
- **Failure is silent.** Covered failures:
  - offline;
  - DNS, TLS, timeout or HTTP errors;
  - oversize responses;
  - parse failures.

  Each produces no notification, dialog, banner or log noise beyond one
  activity-log diagnostic line ("Update check failed: offline"). About shows
  "Could not check for updates." with the time. Nothing else changes.
- **Available.**
  - About shows "Version 0.2.0 is available", with **Open download page**
    and manual steps.
  - One `AppUpdate` notice per version (`NoticeKey::Update(version)`, §3).
    It is deferred while gaming like any nonurgent notice.
- **Never self-updating.** WoW Sync never downloads, stages or replaces the
  app (R-16, A-28). About states this. It also states the unsigned status:
  "This build is not signed by a publisher…" (ui/screens.md, About).
- **Distinct from the R-16 "newer version required" state.** That comes from
  shared data schema majors (`UnsupportedMajor`), not from this check.

### 4.4 TLS approach

**Chosen: `ureq =2.12.1`** (`default-features = false`, features
`["native-tls"]`) plus **`native-tls =0.2.18`**. The connector is passed with
`AgentBuilder::tls_connector`.
- **Windows** (x64 and ARM64) uses SChannel through `schannel 0.1.30`.
- **macOS** (x64 and ARM64) uses Security.framework through
  `security-framework 3.7.0`.
- The OS trust store and OS TLS updates apply.
- No C toolchain, ring, aws-lc, OpenSSL or bundled root store is used on our
  targets. `openssl` appears in the lock only for non-target Unix and is
  never built.

**E-06 justification:**
- **ureq** (MIT OR Apache-2.0) is a small blocking HTTP client with no async
  runtime. That fits one daily GET from a background thread.
- **native-tls** (MIT OR Apache-2.0) is the standard OS-TLS facade.
- **New crate versions in `Cargo.lock`:**
  - `ureq 2.12.1`, `native-tls 0.2.18`, `schannel 0.1.30`,
    `security-framework 3.7.0` and `security-framework-sys 2.17.0`;
  - Unix-only lock entries that are not built for our targets: `openssl*`,
    `foreign-types*` and `vcpkg`;
  - `uuid 1.29.3`, a newer patch than the workspace lock that the scratch
    resolution picked; in the workspace, `--locked` resolution keeps the
    existing pin where compatible.
- Everything else is already locked.
- **`cargo deny check`** passes: advisories ok, bans ok, licenses ok, sources
  ok. The evidence has the commands.

**Rejected alternatives:**
- **`ureq 3` with `native-tls`.** It pulls `webpki-root-certs`
  (CDLA-Permissive-2.0, not on the deny allow list). deny **fails**.
  Allowing that licence is an owner decision (Q-66).
- **`rustls` with `ring` or `aws-lc-rs`.** `ring` needs a C toolchain and
  clang for ARM64 Windows, and `aws-lc-rs` needs cmake and NASM. Root stores
  (`webpki-roots`) are CDLA-licensed too.
- **`reqwest`.** It is a heavy async stack (tokio, hyper) for one GET.
- **Direct WinHTTP and NSURLSession FFI.** This adds zero crates but needs
  `unsafe` code on two platforms. It is kept as the fallback if ureq 2 stops
  receiving security fixes.

**Maintenance caveat.** ureq 2.x is in maintenance mode upstream; 3.x is
current. That is acceptable for one pinned, deny-checked GET. Revisit before
release (Q-66).

**Architecture guard.** Only `wow-sync-adapters/src/update_http.rs` may name
`ureq` or `native_tls`. A test in the adapters crate scans the workspace
sources (like core `arch.rs`) and fails on any other use. It also fails on any
`https://` literal other than the two constants above (A-38 control).

---

## 5. Diagnostics export (A-33, R-15)

### 5.1 Flow

1. **Request.** The user selects App settings > Data > **Export
   diagnostics…**. It is never automatic, and there is no upload or send
   button.
2. **Build.** Runtime gathers a typed `DiagnosticsInput`. Core
   `diagnostics::build(input, &RedactionContext, options) -> Result<Export,
   ExportRefused>` produces the exact text.
3. **Preview.** A sheet shows **the exact bytes that will be saved**, in a
   scrollable, selectable monospace view. It also shows:
   - the size;
   - a summary of what was removed ("Removed: 4 folder paths, 3 computer
     names, 12 character names");
   - the never-included list (§5.4);
   - the warning "Anyone you send this to can read it. Do not save it in your
     shared Dropbox folder: everyone in the group can read that folder."
4. **Option.** **Include add-on names** is a checkbox, on by default (Q-64).
   Toggling it rebuilds the preview.
5. **Save.** **Save…** opens the OS save dialog (GPUI `prompt_for_new_path`).
   The default name is `wowsync-diagnostics-<yyyymmdd-hhmm>.txt` in Documents.
   - A path inside the shared folder, or any install root, is refused with
     the text above.
   - The file is written temp + rename.
   - The text is never copied to the clipboard automatically. A **Copy**
     button is available.
6. **Cancel** discards everything. Nothing is cached.

### 5.2 Format and bound

- **Format.**
  - UTF-8 text.
  - The first line is `wowsync-diagnostics/1`, then `key: value` sections in
    a fixed order: About, Environment, Settings, Variants, Computers,
    Packages, Holds, Recent activity, Errors.
  - It is human-readable for support and stable for tests.
- **Bound.**
  - At most **256 KiB**.
  - Activity is limited to the last 200 entries and Packages to 500 rows,
    each line ≤ 300 bytes.
  - When a limit is hit, the section ends with `(… N more omitted)`.
  - Building stops at the bound. It never builds a larger string and cuts
    it.

### 5.3 Contents and redaction

The export is **built from an allow-list of typed fields**. It never copies
free text from files, providers, addons or OS errors.

| Section | Included | Redacted form |
|---|---|---|
| About | App version, build commit, settings toggles (on/off), schema versions | — |
| Environment | OS family and version, CPU architecture, memory of the app process, WoW running (yes/no/unknown) | — |
| Variants | Variant kind (Retail, Classic, Classic Era, …), verified identity status, counts of addons | install roots → `<install-1>`; never the path |
| Computers | One row per member: alias, enrollment state, retired, last report age, member summary (`Usable`/`Regressed`/…), watermark seq | display names and computer ids → `Computer A`, `Computer B`, … (this computer = `This computer`) |
| Packages | Package key and name (if the option is on), status, candidate count, version labels, decision kind, hold scope | names omitted when off → `Add-on 1…` |
| Holds | `PlanBlock` kinds, journal states | — |
| Recent activity | The activity entry *kind*, time, alias names | rendered from typed entries with aliases, never stored strings |
| Errors | Error kind and OS error code (for example `Io(PermissionDenied, 5)`) | no OS message text and no paths |

**Second defence: the output scan.** After rendering, `diagnostics::build`
scans the whole text for every string in `RedactionContext` and for these
patterns:

| Pattern | Matches |
|---|---|
| SIDs | `S-1-5-21-…` |
| Windows profile paths | `[A-Za-z]:\\Users\\…` |
| macOS profile paths | `/Users/…` |
| Home | `~/…` |
| UNC paths | `\\server\…` |
| E-mail-like strings | |
| WoW account folder forms | `WTF\Account\…`, `#\d+` account suffixes |

`RedactionContext` holds:
- home directory;
- OS user name;
- computer names and display names;
- install, shared, Dropbox and local-state roots;
- the WTF account, realm and character folder names found by the scanner.

**Any hit is a refusal (fail closed).** The result is
`ExportRefused::PrivateDataFound(kind)`, with no partial file. The GUI says:
"WoW Sync could not make a safe export. Nothing was saved." It also writes an
activity entry with the kind only.

### 5.4 Never included

The export never includes:
- saved-variable (WTF) contents;
- addon files or Lua;
- `.toc` text;
- account, realm or character names;
- OS user names, SIDs or user-profile paths;
- full or partial filesystem paths;
- display names and computer, enrollment and group ids;
- credentials or Dropbox tokens (none exist);
- memory dumps;
- arbitrary error text;
- hashes of private files.

**Exception:** the hashes of public package versions are included, truncated
to 12 hex characters.

Telemetry does not exist yet. The App settings > Privacy toggle stays "Coming
later".

---

## 6. Implementation map

New public names are listed per crate. Existing names are those in the R3/R4
code.

| Crate / module | Adds | Notes |
|---|---|---|
| `wow-sync-core::status` | `Headline::ReviewNeeded`; `StatusInput.reconfirm_pending`; `HoldScope`, `AllHoldCause`, `DecisionReach`; `StatusView.hold`; `PackageStatus::reach`; `MemberStatus.retired: Option<RetiredInfo>`; `derive_status` excludes retired members from `awaiting`/confirmations | precedence §1.7 |
| `wow-sync-core::decide` (new) | `DecisionItem`, `DecisionOption`, `OptionId`, `OptionLabel`, `Consequence`, `EvidenceToken`, `DecisionEffect`, `LocalResolution`, `DecideError`; `decision_item`, `decide` | builds objects via the removal §3.3 builder; pure |
| `wow-sync-core::retire` (new) | `Retirement` (schema `wowsync.retirement/1`), `RetirementStatus`, `build_retirement`, `validate_retirements` (grounded labelling, §2.2), `effective_cutoffs` | used by `admit` for cutoffs and by `status` |
| `wow-sync-core::admit` | applies `effective_cutoffs`: `seq > cutoff` → not admitted (`MemberSummary::Retired`) | existing watermarks unchanged |
| `wow-sync-core::notice` (new) | §3.2 types; `plan_notices` | pure; time and game state are inputs |
| `wow-sync-core::update` (new) | `ReleaseInfo`, `SemVer`, `parse_latest`, `compare`, `due` | pure; uses `serde_json` (already a workspace dependency) |
| `wow-sync-core::diagnostics` (new) | `DiagnosticsInput`, `RedactionContext`, `ExportOptions`, `Export {text, removed_counts}`, `ExportRefused`, `build` | pure; fail closed |
| `wow-sync-core::view` | `NextAction::ReviewSetup`, `NextAction::JoinAgain`, `NextAction::ReassignMaster` | |
| `wow-sync-adapters::notify` (new) | `Notifier`, `OsNotifier` (notify-rust), `RecordingNotifier` (testkit) | |
| `wow-sync-adapters::update_http` (new) | `ReleaseSource` trait, `GithubReleases` (ureq + native-tls), `FakeReleases` (testkit) | the only network code; arch guard §4.4 |
| `wow-sync-adapters::winproc` | `register_app_user_model_id`, `set_process_app_id` (Windows) | `windows-sys` `Win32_UI_Shell`, `Win32_System_Registry` |
| `wow-sync-adapters::localstate` | files `notices.json`, `update_check.json` | bounded reads as today (`SMALL`) |
| `wow-sync-runtime::service` | `SliceService::decision`, `decide`, `retire`, `export_diagnostics`, `check_updates`; `StatusSnapshot.hold`, `.retirements`, `.update` | writes authority files via the existing publication path |
| `wow-sync-runtime::control` | `AppSettings { update_check, notify_summaries, notify_needs_me }` | defaults on |
| `wow-sync-runtime::notices` (new) | drives `plan_notices` after each cycle and on game-state change; persists the ledger | |
| `wow-sync-runtime::updates` (new) | schedules `due`, runs `ReleaseSource` on a worker thread, stores the result | |
| `wow-sync-app::model` | `headline` handles `ReviewNeeded` and retired-self; `need_items` uses `DecisionItem`; scope sentences from `HoldScope`; `tray_shape` attention tone for `ReviewNeeded` | |
| `wow-sync-app::pages` | Decision page renders `DecisionOption`s; Computers "Retired" group and **Retire this computer…**; About update states; App settings notification toggles and **Export diagnostics…** | |
| `wow-sync-app::app` | `Sheet::Retire`, `Sheet::DiagnosticsPreview`; `Act::Decide`, `Act::Retire`, `Act::Export`, `Act::CheckUpdates` | |
| `wow-sync-app::world` | `UpdateInfo { current, latest: Option<String>, checked, failed }` | extends the existing struct |

**Dependencies to add** (exact pins, E-06; §3.3, §4.4):
- `notify-rust = "=4.19.0"`, in adapters;
- `ureq = { version = "=2.12.1", default-features = false, features = ["native-tls"] }`,
  in adapters;
- `native-tls = "=0.2.18"`, in adapters;
- the `windows-sys` features `Win32_UI_Shell` and `Win32_System_Registry`.

No new dependency goes into core.

---

## 7. Test list

Every new behaviour test gets an A-38 negative control: mutate the named
production function, observe the named test fail (exit 101), then restore.
"Neg" marks a negative case.

### A-26: decisions (core `decide`, app model)

| ID | Test | Kind |
|---|---|---|
| T-26-1 | `Ambiguous` with two copies: one Keep option per copy plus Decide later; holders correct | positive |
| T-26-2 | Keep copy X → object validates V1–V7; `declined` = the other copies; peers' target is exactly X | positive |
| T-26-3 | `decide` with a stale `EvidenceToken` (a new copy appeared) → `Changed`; nothing written | Neg |
| T-26-4 | `DifferentTocMapping`/`UnvalidatedTocMapping` → no options, no notice, no headline promotion | Neg |
| T-26-5 | `NoCompatibleToc` → option carries the warning; the decision has `acknowledged` | positive |
| T-26-6 | `PartialCandidate` keep smaller → the dropped folders go to recovery (removal §1.5) | positive |
| T-26-7 | Decision while paused → recorded `unshared`, not written until resume | positive |
| T-26-8 | Decision while WoW is running → written and shared; no game file is touched until the game exits | Neg (no apply) |
| T-26-9 | Retired computer → `NotAMember`; no file written | Neg |
| T-26-10 | Change decision → a new object `knows` the old one; previously declined stays declined (Q-50) | positive |
| T-26-11 | Journal **Keep the new version** is not offered when the staged tree fails re-hash | Neg |
| T-26-12 | UQ-2: an unresolved journal → `HoldScope::AllApplies(UnresolvedJournal)`; one NeedsDecision → `Packages([p])`; the app sentence matches | positive |
| T-26-13 | UQ-1: reconfirm pending + Paused → headline `ReviewNeeded`; reconfirm pending + NeedsHelp → `NeedsHelp`; pause + Error → Error | positive / Neg |
| T-26-14 | NeedsDecision alone does not produce `ReviewNeeded` (Q-59 default) | Neg |
| T-26-15 | GUI: Decision page keyboard flow (Tab to options, Space to select, Enter to confirm); screenshots with interaction evidence | manual A-26 |

### A-45: retirement (core `retire`, `admit`, `status`; runtime e2e)

| ID | Test | Kind |
|---|---|---|
| T-45-1 | Retire B (cutoff 10): B's state seq 11 delivered later is not admitted; B is not in `awaiting`; the headline does not wait for B | Neg (stale publication) |
| T-45-2 | B's history ≤ 10, its member dir and blobs are untouched after retirement (file listing before = after) | Neg (no deletion) |
| T-45-3 | B's authority object with `issued_at_seq` 12 not in `authority_known` → `invalid(after_retirement)` | Neg |
| T-45-4 | Retire the settings master with one active computer left → `MasterState::Absent`; no computer becomes master; the banner says reassign | Neg (no auto switch) |
| T-45-5 | Mutual retirement A↔B, both above cutoff → both retired on every reader; same result regardless of read order (property test over permutations) | positive |
| T-45-6 | B re-enrolls (new EnrollmentId) → treated as joining; the old retirement does not apply; removal §8.3 rules hold | positive |
| T-45-7 | Retired-self: after admitting its own retirement, no file is written to its member dir in later cycles; local addons unchanged | Neg |
| T-45-8 | Conflicted retirement id (two variants) → ignored and reported; no reader retires B from it | Neg |
| T-45-9 | GUI: retire sheet disabled until the checkbox; 72-hour warning shown; Cancel default focus | manual A-45 |

### A-27: notifications (core `notice`; runtime with `RecordingNotifier`)

| ID | Test | Kind |
|---|---|---|
| T-27-1 | Batch with 5 applied items → one summary, three names + "and 2 more" | positive |
| T-27-2 | A conflict appears while gaming → no notice; status shows it immediately; on game exit → exactly one action notice | positive |
| T-27-3 | Unchanged persistent condition over 100 cycles → notified once | Neg (spam) |
| T-27-4 | Condition clears and recurs after 1 h → no notice; after 6 h → one | Neg |
| T-27-5 | Many summaries while gaming → released as ≤ 2 notices; items resolved meanwhile are dropped | positive |
| T-27-6 | `ActivityUnknown` is treated as gaming | Neg |
| T-27-7 | `DataAtRisk` while gaming → sent once | positive |
| T-27-8 | Toggles off → no Summary/ActionNeeded; DataAtRisk still sent; toggling back on does not burst | Neg |
| T-27-9 | Ledger persisted: restart with an open condition → no repeat | Neg |
| T-27-10 | Ledger and held set stay bounded after 10,000 synthetic keys | Neg |
| T-27-11 | Native: a toast/notification appears with the WoW Sync name on Windows x64 (owner machine) and macOS (owner's Mac) | manual |

### A-28: update notice (core `update`; runtime with `FakeReleases`)

| ID | Test | Kind |
|---|---|---|
| T-28-1 | `parse_latest` accepts `v0.2.0`; rejects drafts, prereleases, `0.2`, `v0.2.0-rc1` and oversize input | positive / Neg |
| T-28-2 | `due`: off → never; < 24 h → no; ≥ 24 h → yes; future timestamp → yes; < 60 s after ready → no | positive / Neg |
| T-28-3 | Opt-out: 7 simulated days → `FakeReleases` call count 0 | Neg |
| T-28-4 | Offline (`FetchError::Offline`) → no notice, no banner; About shows "Could not check"; the next attempt is 24 h later | Neg (silent) |
| T-28-5 | Newer version → one AppUpdate notice per version, deferred while gaming; same version next day → no notice | Neg (spam) |
| T-28-6 | The request built by `GithubReleases` has exactly the documented headers; no cookie, no auth, no query (the request builder is unit-tested without the network) | Neg (identifiers) |
| T-28-7 | Arch guard: `ureq`/`native_tls`/`https://` outside `update_http.rs` fails the test | Neg (A-38 control by planting a use) |
| T-28-8 | The download URL is built from parsed numbers; an `html_url` pointing elsewhere is ignored | Neg |
| T-28-9 | No code path writes into the app's install directory (static scan + e2e file listing) | Neg (no self-update) |
| T-28-10 | Live: one real GET against GitHub, run manually by the owning implementation packet only (D-24); result recorded as evidence | manual |

### A-33: diagnostics (core `diagnostics`)

| ID | Test | Kind |
|---|---|---|
| T-33-1 | Synthetic install with **planted private strings** (user name `zz_user_PLANT`, home `C:\Users\zz_user_PLANT`, computer name `PLANT-PC`, account folder `PLANTACCT#1`, realm `PlantRealm`, character `Plantchar`, SID `S-1-5-21-1111-2222-3333-1001`, Dropbox root, an OS error text containing a path) → none appear in the export, as a substring or case-insensitively | Neg |
| T-33-2 | The same with the planted string *forced* into a typed field (test-only injection) → `ExportRefused::PrivateDataFound`, nothing saved | Neg (fail closed) |
| T-33-3 | 10,000 activity entries / 5,000 packages → export ≤ 256 KiB with "more omitted" markers | Neg (bound) |
| T-33-4 | Never-included: plant WTF/Lua/.toc contents with unique markers → absent | Neg |
| T-33-5 | Add-on names off → no package names; on → names present | positive |
| T-33-6 | Saving under the shared folder or an install root → refused | Neg |
| T-33-7 | No export is produced without the user action (runtime has no scheduled call to `export_diagnostics`) | Neg |
| T-33-8 | GUI: preview shows exactly the saved bytes (byte compare) | positive |

---

## 8. GUI touchpoints

| Screen (ui/screens.md) | Change |
|---|---|
| Dashboard headline | `ReviewNeeded` (§1.7); "This computer was retired" (§2.4); scope sentence from `HoldScope` (§1.6) |
| Needs attention | Items from `DecisionItem`; informational TOC items without actions; reach label |
| Decision | Option cards from `DecisionOption` (label, holders, consequence, warning), **Decide later**, "Something changed…" re-render, saved/unshared notes (§1.3) |
| Add-on detail | **Change decision**; decision provenance line ("Kept by your choice on Laptop, 12 Jan") |
| Computers | **Retired** group; **Retire this computer…** sheet (§2.5); retired-self banner with **Join again…**; conflicted retirement line |
| Settings (settings.md) | "Settings master retired" banner with **Reassign settings master…** (§2.6) |
| Activity | Decision, retirement, update-check-failed and export-refused entries |
| App settings | Notifications toggles (existing); "Notifications are not available on this Mac" line when the backend fails; **Export diagnostics…** enabled (§5) |
| About | Update states as in ui/screens.md; **Check for updates automatically**; **Check for updates**; **Open download page** (§4) |
| Tray | Attention tone for `ReviewNeeded` and retired-self |

## 9. Open questions

Q-59 to Q-66 in [questions.md](questions.md). Implementers follow the safe
defaults until a decision is recorded.
