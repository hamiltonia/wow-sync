# Operations: decisions, retirement, notifications, updates and diagnostics

Part of the [slice design](README.md). Written by run-004 packet
R4-ops-design and **revised by packet
[R4-design-fix](../runs/run-004/packets/R4-design-fix.md)**, which fixes
R4D-10 to R4D-14 and the ops parts of R4D-1 of the
[R4-design-review](../runs/run-004/evidence/R4-design-review.md). **Not
implemented.** Choices marked **PENDING OWNER** use the conservative
default in [questions.md](questions.md). They are not owner decisions.

| § | Feature | Requirement | Acceptance |
|---|---|---|---|
| 1 | Resolving decisions in the GUI, with the headline promotion (UQ-1) and the typed hold scope (UQ-2) | R-02, R-06, R-13 | A-26 |
| 2 | Retiring a computer | R-03, R-07, R-12 | A-45 |
| 3 | OS notifications | R-13 | A-27 |
| 4 | The app update notice | R-13, R-15, R-16 | A-28 |
| 5 | Diagnostics export | R-15 | A-33 |
| 6 | Implementation map | E-01, E-06 | — |
| 7 | Test list | E-03, E-04 | A-26, A-27, A-28, A-33, A-38, A-45 |
| 8 | GUI touchpoints | R-13 | A-26 |

**Contract.** [contracts.md](contracts.md) alone defines the authority
record family, identity, references, `Known` coverage, validity, mirroring
and capabilities (§1), historical membership and the retirement barrier
(§1.10), target precedence (§4), the code-choice family with its decision
reasons and `decide` freshness (§5), the admission order (§6), the
diagnostics boundary (§7) and the split between technical caps and product
gates (§8). This file defines behaviour on top of them. Where it seems to
differ, contracts.md wins. Phase C: shared types and the retirement
barrier are built by **R4-core-contracts**; the decision, code-choice and
retirement-command behaviour by **R4-core-ops**; notifications, update
notice and diagnostics by **R4-notify-diag** (contracts.md §9, this file
§6). Revised by R4-design-fix2 for R4D-10 (code-choice semantics), R4D-15
and R4D-16 (retirement).

**Approvals used.** D-21 (run-003 owner review: WoW Sync syncs, it does not
police addons), D-23 (UQ-1 and UQ-2: yes), D-24 (P4-1, the GitHub Releases
check; P4-2, an OS notification crate). Open choices are Q-59 to Q-66 and
Q-85 to Q-87 in [questions.md](questions.md).

**Other inputs.** Removal holds and their actions: [removal.md](removal.md)
§1.2, §1.7. Rejections and lifts: [rollback.md](rollback.md). Settings
master and reassignment: [settings.md](settings.md) §5. Screens:
[ui/screens.md](ui/screens.md). Notification rules:
[ui/interaction.md](ui/interaction.md#notifications).

---

## 1. Resolving decisions in the GUI (A-26, R-06, R-13)

### 1.1 One option model for every decision kind

Every item in **Needs attention** that asks the user to choose is shown on
the Decision page through one generic interface (UG-8). The GUI never
builds options itself. It renders what core returns and passes the chosen
option back.

```rust
// wow-sync-core::decision (types, step 1: R4-core-contracts); the two
// functions are filled in wow-sync-core::decide (step 2: R4-core-ops)
pub enum DecisionScope { Folders(Vec<FolderKey>), Settings(SettingsUnit) }  // contracts.md §3
pub struct DecisionItem {
    pub scope: DecisionScope,         // the identity; the package name is display only
    pub reason: DecisionReason,       // contracts.md §5.1, or a settings kind (settings.md §4.4)
    pub reach: DecisionReach,         // Group | ThisComputer (§1.6)
    pub options: Vec<DecisionOption>,
    pub evidence: EvidenceToken,      // hash of everything the options were built from
}
pub struct DecisionOption {
    pub id: OptionId,                 // stable within one EvidenceToken
    pub label: OptionLabel,           // typed, rendered by the app (no free text)
    pub holders: Vec<MemberRef>,      // computers that hold this copy now
    pub consequence: Consequence,     // typed: installs, removals into recovery, settings moves
    pub warning: Option<OptionWarning>,
}
pub fn decision_items(now: &FreshEvidence) -> Vec<DecisionItem>;
pub fn decide(scope: &DecisionScope, option: &OptionId, shown: &EvidenceToken,
              now: &FreshEvidence, issuer: Issuer) -> Result<DecisionEffect, DecideError>;
```

- `decide` recomputes the item from **current** evidence and compares its
  token with `shown` (contracts.md §5.5). Runtime holds the cycle lock from
  that recomputation until the record has been written and read back.
- It returns `DecisionEffect::Group(Vec<AuthorityRecordBytes>)` (one or
  more records to write) or `DecisionEffect::Local(LocalResolution)` (a
  journal action for this computer only).
- `DecideError`: **`Changed`** (the evidence differs), **`NotAMember`**
  (this computer is retired or not enrolled, §2.4), **`UnknownOption`**,
  **`NotOffered`**.
- "Something changed while you were choosing" is the GUI text for
  `Changed`. The page re-renders the new options and never applies a
  choice made against old evidence.

### 1.2 Actions per decision kind

Every code decision follows contracts.md §5.1. The Interface number in a
`.toc` is **advisory text only** (D-21): it never creates a decision and
never blocks an install, and there is no "Use it anyway" option anywhere.
No option makes unreadable, unsafe or unvalidated content installable
(R-02).

| Kind | Options (labels as rendered) | Records written | Reach |
|---|---|---|---|
| `Ambiguous` | one **Keep this copy** per distinct copy, labelled with its version text and holders ("Keep the copy from Laptop and Den PC"); **Decide later** | one `code-choice` keep (contracts.md §5.2) whose `considered` lists every copy shown | Group |
| `PartialCandidate` | **Keep the complete copy**; **Keep the smaller copy (drops <N> folders)**; **Decide later** | a keep. For the smaller copy, also a `removal` of the dropped folders ([removal.md](removal.md) §1.4), shown in the consequence; the dropped folders go into recovery | Group |
| `ChoicesDiffer`, `RollbackTargetsDiffer`, `RecoveredUnavailable`, `LookalikeOfRejected`, `NewerAfterRollback` | contracts.md §5.1 and [rollback.md](rollback.md) §3 | a keep whose `known` covers the competing records; "Block this copy too" writes a rejection | Group |
| `UnusableMetadata(kind)` | **no options.** "This copy can't be used: <reason>. Reinstall or fix it on <computer>." | nothing | — |
| `DifferentTocMapping`, `UnvalidatedTocMapping` | **no options.** "Waiting for a WoW Sync update that knows this game version." Never notifies (§3) and never promotes the headline | nothing | — |
| Removal holds (`RemovalUnclear`, `ReinstallObserved`) | as [removal.md](removal.md) §1.2 and §1.7 (**Confirm removal** / **Keep it**; **Add back to all computers** / **Remove again**) | removal or re-add records (removal.md) | Group |
| Settings choices | [settings.md](settings.md) §4.4. Competing intentional choices are not merged (R-09); each option is one whole choice | settings revision | Group or ThisComputer, as settings.md states |
| `NeedsHelp` journal (UG-9) | **Try again** (re-run journal recovery); **Put back the previous version** (restore the journal's recovery point); **Keep the new version**, only if the staged tree re-hashes to its manifest | local journal resolution | ThisComputer |

**Decide later** is always offered where there are options and records
nothing. The item stays in Needs attention and does not notify again while
unchanged (§3.2).

The Add-on detail page shows the declared Interface number next to the
game's ("Lists Interface 110000; this game is 110002") as information only.

### 1.3 Preconditions

| Condition | Group decision | Local journal action |
|---|---|---|
| This computer is retired or not enrolled | not offered (`NotAMember`): "This computer was retired. Join the group again to make choices for it." | allowed; it is the user's own disk |
| This computer's own retirement is unresolved (held, §2.4) | **allowed**, kept locally like a paused record: "Saved. It is shared if this computer is still in the group." It is never shown as refused for retirement | allowed |
| WoW is running, or its state is unknown | **allowed.** The record is written and shared, because no game file changes. The apply waits: "Saved. WoW Sync applies it after you close WoW." | only actions that touch no game files; otherwise disabled with "Close WoW first" |
| Syncing is paused | **allowed.** The record bytes and their reserved seq are kept locally and written to the shared folder on resume: "Saved. It is shared when you resume syncing." | allowed |
| The shared folder is unavailable | allowed; written when the folder is writable again: "Saved here; not shared yet" | allowed |
| The evidence changed since it was shown | `Changed`; re-render | `Changed` |
| An unresolved journal exists for another addon | allowed; applying is held by the journal (`AllApplies`, §1.6) | — |

### 1.4 What core records, and how peers see it

1. `decide` builds the record bytes with `records::encode`, including
   `considered` and `known` (contracts.md §1.8, §5.2), and validates them
   with the same rules every peer uses (contracts.md §1.11, V-CHOICE). A
   computer never writes a record it would itself reject.
2. Runtime writes it as `members\<self>\authority\<sha256>.json`
   (write-if-absent, read-back hash check) and lists it in the next index
   chunk of the next own state (contracts.md §1.2, §1.6).
3. Runtime appends to the local audit and Activity logs, for example "You
   chose to keep WeakAuras 5.12 from Laptop for the group."
4. Peers learn and mirror it (contracts.md §1.12), apply the constraint at
   their next safe apply, drop the item from Needs attention, and show
   "Laptop chose to keep WeakAuras 5.12." Dropbox publication alone is
   never shown as "applied" (R-08); confirmations still come from peers'
   published states.
5. Local journal actions are recorded in the local journal and Activity
   only. Peers see the result through this computer's next state.

### 1.5 Changing a choice

- Records are never edited. **Change decision** on the Add-on detail page
  opens the Decision page with the current options plus **Resume automatic
  updates**. The new choice is a new record whose `known` covers the old
  one (contracts.md §5.3).
- **Resume automatic updates** writes a `resume` code choice (contracts.md
  §5.2–§5.3). It ends the keep it covers, imposes no new pin, and **keeps
  the exclusions** of the versions the user declined earlier: a resume's
  `exclude` carries every previously excluded package forward, so a
  declined version is never installed automatically after resume (Q-50,
  owner-accepted, D-21). The consequence text lists them. A declined
  version becomes eligible again only when the user ticks it under
  **Allow these again** in the same resume choice (`allow_again`). The
  kept version and any version the user has not yet seen are never
  excluded by a resume; a version that only shares some unchanged
  folders with a declined one is not the declined version (exclusions
  compare the source's complete content over each exclude entry's own
  `scope` only, contracts.md §4.2; folders that joined the addon later
  are not compared, so the declined version is still recognised after
  regrouping, T-R4D28-regrouped-exclusion).
- **A newer version after a keep (Q-49, owner-accepted).** While a keep is
  live it pins the kept version. A candidate that the keep did not
  consider (its source is `Different` from every `considered` entry, each
  compared over that entry's scope, contracts.md §4.2), whether demonstrably newer or
  unordered, raises `NeedsDecision(NewVersionAfterDecision)`; the kept
  version stays installed until the user chooses explicitly (contracts.md
  §5.1, §5.3). Nothing is installed automatically past a keep.
- Rejecting a release is a separate record ([rollback.md](rollback.md));
  **Allow again…** for a rejected release is its lift. A code-choice
  exclusion is not a rejection, and a lift never ends an exclusion.
- Only the `keep` and `resume` actions of the code-choice family exist;
  there is no "allow unsafe metadata" or Interface override (D-21,
  `UnusableMetadata` has no options).
- Every replacement or removal caused by a choice goes through the normal
  apply path with a recovery point: current plus two previous per addon
  (R-10). **Restore** on the Recovery page works as before.
- A local journal action is undone by restoring the matching recovery
  point.

**As-built (run-005, Q-88 option (a), D-26).** With the kept copy held
nowhere, an unconsidered copy of any order raises `NewVersionAfterDecision`;
a newer local-only copy stays installed, is not shared, and the decision is
asked again every cycle (see contracts §5.3).

### 1.6 UQ-2: the typed hold scope (D-23: yes)

Core states how much is held, so the app never guesses.

```rust
// wow-sync-core::status
pub enum HoldScope {
    /// Nothing is held.
    None,
    /// Only these addons wait. Everything else keeps syncing.
    Addons(Vec<DecisionScope>),
    /// No game-file change is applied on this computer until the cause clears.
    AllApplies(AllHoldCause),
}
pub enum AllHoldCause { UnresolvedJournal { folders: Vec<FolderKey> }, GameRunning, ActivityUnknown, Paused, Retired, RetirementUnresolved }
pub enum DecisionReach { Group, ThisComputer }
pub struct StatusView { /* existing fields */ pub hold: HoldScope }
impl PackageStatus { pub fn reach(&self) -> Option<DecisionReach> }
```

- `derive_status` computes `hold` from data it already has:
  `PlanBlock::UnresolvedJournal`, `GameRunning` and `ActivityUnknown` give
  `AllApplies`; pause, retired-self and held-self are runtime inputs. Otherwise the
  addons and settings units in `NeedsDecision`, `NeedsHelp` or held give
  `Addons`.
- `AllApplies` takes precedence over `Addons`.
- App text: **`Addons([p])`** "Only WeakAuras is waiting. Everything else
  keeps syncing."; **`Addons(n)`** "Only these 3 add-ons are waiting…";
  **`AllApplies(UnresolvedJournal)`** "Nothing is updated on this computer
  until this is resolved."; **`Group` reach** "Your choice applies to every
  computer in the group."; **`ThisComputer` reach** "This only affects this
  computer."
- No scope sentence is shown when `hold` is `None`. This replaces the
  "until core states it" rule in ui/screens.md (UG-25).

### 1.7 UQ-1: "Review needed" headline (D-23: yes)

- Add `Headline::ReviewNeeded` to core, and `StatusInput.reconfirm_pending:
  bool`. Runtime sets it from the guided reconfirmation state.
- **Precedence, highest first:** `NeedsHelp`; `Error`; `ReviewNeeded`;
  Paused (the app overlay); `Working`; `Ok` / `Waiting`.
- **Pause never hides `NeedsHelp`, `Error` or `ReviewNeeded`.**
- **Text:** "Review needed". Subline: "Confirm this computer's WoW folders
  so syncing can continue." The action opens the reconfirmation.
- **Tray:** attention tone, same shape as `NeedsHelp`, tooltip "WoW Sync:
  review needed".
- Pending decisions do **not** promote the headline. They keep core's
  headline, the Needs attention count and the tray tooltip count, so the
  dashboard never claims "fully synced" while a decision is open (Q-59).

---

## 2. Retiring a computer (A-45, R-03, R-07, R-12)

### 2.1 Record and barrier

The retirement record body, historical membership and the retirement
barrier are contracts.md §1.10. In short:

- The issuer writes `wowsync.retirement/1` in its own `authority\`
  directory; it is mirrored like every record.
- `cutoff_seq` is the issuer's admitted watermark for the retired
  enrollment (its last admitted seq). Self-retirement uses `cutoff_seq =
  issued_at_seq − 1`.
- **Retired(E)** is Yes once any **Effective** retirement of E exists: it
  passes V-BYTES, V-GROUP, V-ISSUER and V-REF, and every retirement named
  in its `retirements_known` is settled as not effective (Void or
  invalid). It is **Void** if one of them is Effective (the issuer wrote
  it knowing it was retired), and **Unresolved** while one of them is
  missing or itself Unresolved (contracts.md §1.10 rule 1). Statuses only
  move Unresolved → Effective or Unresolved → Void, so Retired(E) = Yes is
  final and partial delivery never retires and then un-retires anyone
  (R4D-25, T-R4D25-partial-retirement-dag). While E's only candidate
  retirements are Unresolved, E is **held**, not retired: its records
  and states above the lowest such cutoff are pending, it is still
  awaited, and nothing that assumes E is retired happens (contracts.md
  §1.10 rule 6). **Cutoff(E)** is the **minimum**
  cutoff over the Effective retirements of E: a further record can only
  lower it, so E's records only ever go valid → invalid, and no later
  retirement with a higher cutoff re-authorizes a post-retirement action
  (R4D-16, Q-52).
- The issuer fills `retirements_known` with every retirement of itself it
  has learned. A correct app never writes it non-empty, because it stops
  publishing when it learns its own retirement (§2.4).
- Mutual or transitive retirement by enrollments that did not know of
  their own retirement retires every named enrollment (**Q-61 PENDING
  OWNER** default). The GUI says: "Laptop and Den PC retired each other.
  Join again on the computer you still use." A retirement written by an
  enrollment that knew it was retired is void and retires nobody
  (T-R4D16-retired-issuer-retirement-void).
- Only explicit re-enrollment (a new enrollment id) makes the computer
  active again (A-45, R-03).

The run-004 draft's `authority_known` list and grounded labelling are
**retired** (R4D-11); `retirements_known` (only the issuer's own
retirements) replaces them. The R4D-11 trace (A retires B; C later, unaware,
retires A with a lower cutoff) leaves B retired in every delivery order
(T-R4D11-late-retirement-three-issuers).

### 2.2 Effects for every reader

`admit` applies the barrier before any currentness, publication or target
use (contracts.md §6). A held enrollment (Retired = Unresolved) gets none
of the effects below; only its records above the hold cutoff wait
(contracts.md §1.10 rule 6), and Details says "Waiting for a retirement
record from <computer>". Once Retired(E) = Yes:

1. **Stale and offline publications are ignored.** E's code and settings
   states with `seq > Cutoff(E)` are not admitted. They are neither
   candidates nor confirmations, whenever they are delivered.
2. **History remains.** E's admitted states, `member.json`, records valid
   under V-RET and the provider data they reference are not deleted. E's
   member directory stays (R-03).
3. **E contributes no candidates** and no new authority.
4. **Currentness no longer waits for E.** E is excluded from `awaiting`,
   confirmation counts, "Waiting for <computer>", removal settling
   ([removal.md](removal.md) §8.2) and the 72-hour "not heard from"
   emphasis (Q-27).
5. **Display.** Computers lists E under **Retired**: "Retired on 12 Jan by
   Laptop. History kept." No status dot; not counted in "N computers".
6. **Re-enrollment** of the same `computer_id` is a new enrollment and
   therefore a joining member ([removal.md](removal.md) §8.3). The old
   retirement never applies to it; the old enrollment stays retired.

### 2.3 Malformed and conflicting records

A retirement that fails V-BYTES is `pending(unreadable)` or
`pending(unknown_schema)` and has no effect until readable (contracts.md
§1.11). Two different effective retirement records of the same enrollment
are two records, not a conflict: both count, and the cutoff is their
**minimum** (contracts.md §1.10). A void retirement has no effect and is
listed under Details as "ignored: written after <computer> was retired".
There is no "conflicted retirement" state.

### 2.4 On the retired computer itself (Q-60)

**Q-60 PENDING OWNER default: stop publishing and stop applying.** It
applies in two cases that are shown and handled differently (R4D-31):
**retired**, when the computer admits Retired(own enrollment) = Yes; and
**held**, when its own enrollment is Unresolved (contracts.md §1.10 rule
6), so the retirement is not proven and may still turn out Void.

In both cases:

- It **stops publishing** immediately: no state, record, index chunk or
  mirror is written, and nothing is uploaded on its behalf.
- It **stops applying** group changes: `HoldScope::AllApplies(Retired)`
  when retired, `HoldScope::AllApplies(RetirementUnresolved)` when held.
- Local addons, settings, recovery points and logs stay **untouched**.
  Nothing is uninstalled.
- **Join again…** is available: the explicit join flow (A-04, guided
  identity and safe-scope validation) with a new enrollment id.

| | Retired (Yes) | Held (Unresolved) |
|---|---|---|
| Headline | `NeedsHelp`-tone **"This computer was retired"**: "Retired on 12 Jan by Laptop. Your add-ons and settings here are kept. Join the group again to sync." | **"Waiting for group history"**: "Another computer may have retired this one. Waiting for the rest of the group history. Your add-ons and settings here are kept." No date or "by" line is shown as fact; Details names the missing retirement record(s) and the computer that wrote the pending one |
| Group actions | only **Join again…** | **Join again…**; otherwise wait |
| Records reserved while paused and never written (§1.3) | discarded with an Activity entry (they had not been shared) | **kept** unwritten and pending; shown as "Waiting to share: N changes". They are not cancelled as if the retirement were proven. The user can explicitly cancel them ("Discard waiting changes…", with an Activity entry) |
| When the status settles | stays retired until re-enrollment | turns Void: publishing and applying resume by themselves, and the kept reserved records are written with their reserved bytes and seq, as on resume from pause (§1.3). Turns Yes: it becomes the retired case, and the kept records are then discarded with an Activity entry |

### 2.5 GUI flow (A-45, A-26)

The flow lives at Computers > select a computer > **Retire this
computer…**, and on this computer's own detail page as **Retire this
computer** (self-retirement).

1. **Title:** "Retire Den PC?"
2. **Facts:** "Den PC will stop sharing changes. Other computers stop
   waiting for it." "Its history and recovery copies are kept. Nothing is
   deleted from the shared folder." "To use it again, join the group again
   from that computer."
3. **Recent report warning.** If it reported within the last 72 hours
   (Q-27), add: "Den PC reported 3 hours ago. Retire only computers you no
   longer use, or that are lost." A warning, not a block (Q-65).
4. **Settings master paragraph** (§2.6), if it is the settings master or
   holds a per-addon exception.
5. **Checkbox:** "I understand Den PC stops syncing." **Retire** stays
   disabled until it is checked. **Cancel** has default focus.
6. **Result.** Runtime calls core `build_retirement(...)`, writes the record
   and publishes the next own state. Activity: "You retired Den PC."
   Computers moves Den PC to **Retired**. Not available while paused or
   retired ("Resume syncing to retire a computer" / "This computer is
   retired"): a retirement must be shared to have any effect.
7. **Undo** is re-enrollment of that computer. A retirement is never
   deleted or edited.

### 2.6 Retiring the settings master

Behaviour owned by [settings.md](settings.md) §5.3. Ops uses the
interface `master_state(view) -> MasterState` (`Present`, `Absent { was,
why: Retired { at, by } }`, `NotChosen`), owned by
[settings.md](settings.md) §10.1.

- **On the sheet:** "Den PC is the settings master. After retiring it,
  settings are not shared automatically until you choose a new master.
  Nothing changes on the other computers' settings." The sheet does not
  offer to pick a new master inline.
- **Afterwards:** settings.md §5.3 applies (no automatic publication,
  pushes and restores still work, history kept). The banner "Settings
  master retired… **Change master…**" opens the settings.md flow.
- **Never auto-switch.** No computer becomes settings master by age, by
  being the last one left, by issuing the retirement or by a timestamp,
  even with one active computer left (T-45-4).

---

## 3. Notifications (A-27, R-13)

### 3.1 Behaviour

This follows ui/interaction.md, Notifications.

**Kinds:** **batch summary**; **action needed**; **data at risk** (urgent);
**app update**.

**Events that never notify:** sharing this computer's state; a batch with
no change here; waiting conditions; a silent peer; pause or resume;
informational TOC-mapping items (§1.2); the retirement of *another*
computer (Activity only). **Retirement of this computer notifies once**,
as action needed.

### 3.2 Pure planner in core

```rust
// wow-sync-core::notice
pub enum NoticeKind { Summary, ActionNeeded, DataAtRisk, AppUpdate }
pub struct NoticeKey { pub kind: NoticeKind, pub scope: NoticeScope }   // Addon(scope) | Computer(id) | SharedFolder | Update(tag) | Batch
pub struct NoticeEvent { pub key: NoticeKey, pub fingerprint: [u8; 32], pub text: NoticeText }
pub struct NoticeLedger { open: BTreeMap<NoticeKey, Fingerprint>, cleared: VecDeque<(NoticeKey, i64)>, held: HeldSet }
pub struct NoticePrefs { pub summaries: bool, pub needs_me: bool }
pub struct NoticePolicy { pub recurrence_quiet_ms: Option<i64> }        // Q-85; None = no suppression
pub fn plan_notices(ledger: &NoticeLedger, events: &[NoticeEvent], open_now: &BTreeSet<NoticeKey>,
                    game: GameActivity, prefs: NoticePrefs, policy: NoticePolicy,
                    now_unix_ms: i64) -> (Vec<Notice>, NoticeLedger);
```

The planner is pure: time, game state and policy are inputs. Rules:

1. **First appearance or changed content.** A key notifies when it first
   opens or when its fingerprint changes (for example a new candidate for
   the same decision). A changed fingerprint always notifies; rule 3 never
   applies to it.
2. **No repeats while unchanged.** An unchanged open key never notifies
   again (A-27).
3. **Recurrence (Q-85 PENDING OWNER).** The same key with the same
   fingerprint recurring after it cleared notifies only if at least
   `recurrence_quiet_ms` (default **6 hours**) passed since it last
   notified. This damps flapping. It **never applies to `DataAtRisk`**,
   which notifies on every recurrence. If the key has been evicted from
   `cleared` (rule 6) the recurrence notifies, so the cap errs toward
   notifying.
4. **One summary per completed batch** that changed something here: at most
   three names, then "and N more". Several new action items from one batch
   become **one** action notification with a count.
5. **Gaming deferral.** Whenever `GameActivity` is `Running` or `Unknown`
   ("can't tell" is treated as "maybe playing"), nonurgent notices go into
   `held`. `DataAtRisk` is sent at once, even while gaming. When activity
   becomes `NotRunning`, held notices are released as **at most two**: one
   combined summary ("While you played: …") and one action notification
   for items **still open**. Items resolved meanwhile are dropped.
6. **Technical caps** (contracts.md §8): `held` keeps counts plus at most
   five names per kind; `cleared` keeps the last 100 keys; `open` is
   bounded by the open conditions themselves (R-14). These never suppress
   a first appearance, a changed fingerprint or `DataAtRisk`.
7. **Preferences (Q-86 PENDING OWNER).** `summaries = false` suppresses
   `Summary`. `needs_me = false` suppresses OS notifications of
   `ActionNeeded`, `AppUpdate` **and** `DataAtRisk`: the toggles are
   respected. `DataAtRisk` is always shown in the app (headline, tray,
   Needs attention) whatever the toggles. Suppressed keys are recorded as
   notified, so turning a toggle on again causes no burst.

**Persistence and display.** Runtime persists `NoticeLedger` in local state
(`notices.json`, at most 64 KiB, a technical cap) so restarts do not
repeat notices. The in-app status always updates immediately, whatever the
planner decides (A-27: actionable status remains available).

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
- Widely used and maintained; GPUI (Zed) already depends on it for Linux.
- One API for both platforms: Windows toasts through
  `tauri-winrt-notification 0.8.1` (`windows 0.62`); macOS through
  `mac-notification-sys 0.6.15`.
- It and both platform backends are already in `Cargo.lock` through GPUI's
  Linux platform crate, so it adds no new crate versions for Windows or
  macOS. This was checked by the R4-ops-design author on a scratch lock and
  has **not been independently rerun**; phase C reruns `cargo deny check`
  and the lock diff as evidence.
- MIT/Apache-2.0 throughout; no service and no network use.

**Rejected alternatives:** hand-written WinRT toast and macOS
UserNotifications FFI (`unsafe` on two platforms, same result);
`tauri-plugin-notification` (tied to the Tauri runtime);
`win-toast-notify` and `winrt-toast` (less used, Windows only).

**Platform obligations (native verification required):**
- **Windows identity (Q-62).** An unpackaged app needs an AppUserModelID,
  or Windows attributes toasts to PowerShell or drops them. At startup,
  runtime registers `HKCU\Software\Classes\AppUserModelId\Hamiltonia.WoWSync`
  (`DisplayName` "WoW Sync", `IconUri` the installed icon) and calls
  `SetCurrentProcessExplicitAppUserModelID` (adapters `winproc`,
  `windows-sys` with `Win32_UI_Shell`). HKCU only, no admin rights. The
  uninstall guide removes the key.
- **Click-through.** notify-rust exposes no activation callback on Windows
  or macOS. Each notification's text says where to look ("Open WoW Sync to
  choose"). Opening a specific page on click is a later, optional step
  behind the same trait.
- **macOS (Q-63 PENDING OWNER).** `mac-notification-sys` uses the
  deprecated `NSUserNotificationCenter`. Native delivery must be verified
  on the owner's Mac (D-23). **If delivery fails, A-27 is FAILED (or
  BLOCKED) for macOS** and is reported as such. The app then shows "Notifications are not
  available on this Mac" in App settings so the user is not misled, but
  that line is not an accepted fallback: only an explicit owner scope
  exception could accept it. The fix path is a `UNUserNotificationCenter`
  backend, which needs a bundled app identity. `mac-notification-sys`
  compiles Objective-C, so macOS targets build on macOS hosts (CI
  `macos-latest`).
- **Error handling.** A delivery error is logged once per kind per session
  and never retried in a loop. It never changes sync behaviour.

---

## 4. App update notice (A-28, R-13, R-15, R-16; D-24 P4-1)

### 4.1 The request

| Item | Value |
|---|---|
| URL | `https://api.github.com/repos/hamiltonia/wow-sync/releases/latest`. A compile-time constant and the only request URL |
| Method | `GET`, HTTPS only |
| Redirects | followed only to `https` URLs on `api.github.com` or `github.com`, at most 3. A redirect to `http`, to another host or past the limit is a failure; nothing is fetched from it |
| Headers | `User-Agent: WoW-Sync` (GitHub requires one; **no version and no id**), `Accept: application/vnd.github+json` |
| Not sent | no cookies (no cookie store), credentials, tokens, query strings, install id, computer id, group id or OS user name |
| Proxy | no proxy is configured and environment proxy variables (`HTTPS_PROXY`, `ALL_PROXY`, …) are **not** read (`AgentBuilder::try_proxy_from_env(false)`) |
| TLS | the OS verifier (§4.4). Invalid, expired, self-signed and wrong-host certificates fail; there is no "accept anyway" path |
| Timeouts | connect 10 s; whole request 20 s (technical caps) |
| Response bound | read at most **256 KiB** (technical cap). Larger is a failure. Parsed only as JSON data, never executed or rendered |

**What GitHub sees** is the user's IP address and the request above. About
discloses this (Q-36): "Checking contacts GitHub releases, which sees your
IP address." No other WoW Sync network call exists (D-23).

### 4.2 Schedule and opt-out

- **Setting.** `AppSettings.update_check: bool`, default **on** (Q-36,
  D-21). The toggle is **Check for updates automatically**, on About.
- **When off,** no automatic request is ever made, including at startup.
- **Persisted state.** `update_check.json` in local state (≤ 4 KiB, a
  technical cap): `last_attempt_unix_ms`; `last_result` (`Newest {
  checked }`, `Available { tag, checked }`, `Failed { at, kind }`);
  `notified_tag`.
- **Attempt before request.** The attempt time is written durably (temp +
  flush + rename) **before** the request is sent. A crash or kill during
  the request therefore never causes a retry loop.
- **When an automatic check runs** (core `update::due`): enabled, **and**
  the app has been ready for at least 60 seconds (a technical delay so it
  never competes with the ten-second sync start), **and** no attempt is
  recorded, or the last attempt is at least the interval ago, or more
  than the interval in the future (clock change). **Interval: 24 hours
  (Q-87 PENDING OWNER).** Failures count as attempts.
- **Check now.** **Check for updates** on About is explicit, so it is
  allowed even when automatic checks are off. It is debounced to once a
  minute (technical). It shows "Checking for updates…" and then the result.
- **Single flight.** At most one request runs at a time. A manual check
  while a scheduled one runs waits for and shows that result; a scheduled
  check while a manual one runs is skipped and recorded as that attempt.
- **Background thread.** The check never blocks sync, the UI or quit, and
  quit does not wait for it.

### 4.3 Results and failure (R4D-14)

- **Parsing.** Core `update::parse_latest(bytes) -> Result<ReleaseInfo,
  UpdateParseError>` reads only `tag_name`, `draft` and `prerelease`.
  `name`, `body` and `html_url` are ignored. The tag must match
  `^v?[0-9]{1,9}\.[0-9]{1,9}\.[0-9]{1,9}$`. Drafts, prereleases and
  anything else are `Failed`.
- `ReleaseInfo { tag: ValidatedTag, version: SemVer }` keeps the **exact
  validated tag string** ("v0.2.0" or "0.2.0") next to its numbers.
- **Comparison.** `update::compare(current, latest)` compares numeric
  triples. The current version comes from `CARGO_PKG_VERSION`.
- **Opening the page.** **Open download page** opens
  `https://github.com/hamiltonia/wow-sync/releases/tag/<exact tag>` in the
  system browser. The tag can contain only digits, dots and an optional
  leading `v`, so a tampered response cannot point elsewhere, and an
  unprefixed release links to its own tag.
- **Failure is silent.** Offline; DNS, TLS, certificate, redirect, timeout
  or HTTP errors (403 and 429 included: GitHub allows 60 unauthenticated
  requests per hour per IP); oversize responses; parse failures. Each
  produces no notification, dialog or banner, and one Activity diagnostic
  line with the failure kind only ("Update check failed: offline"). About
  shows "Could not check for updates." with the time.
- **Available.** About shows "Version 0.2.0 is available" with **Open
  download page** and manual steps. One `AppUpdate` notice per tag
  (`NoticeKey::Update(tag)`, §3), deferred while gaming like any nonurgent
  notice.
- **Never self-updating.** WoW Sync never downloads, stages or replaces the
  app (R-16, A-28). About says so, and states the unsigned status ("This
  build is not signed by a publisher…").
- **Distinct from R-16 "newer version required".** That comes from shared
  data schema majors (`UnsupportedMajor`), not from this check.

### 4.4 TLS approach

**Chosen: `ureq =2.12.1`** (`default-features = false`, features
`["native-tls"]`) plus **`native-tls =0.2.18`**, passed with
`AgentBuilder::tls_connector`. Windows (x64 and ARM64) uses SChannel
(`schannel 0.1.30`); macOS (x64 and ARM64) uses Security.framework
(`security-framework 3.7.0`). The OS trust store and OS TLS updates apply.
No C toolchain, ring, aws-lc, OpenSSL or bundled root store is used on our
targets.

**E-06 justification:** ureq is a small blocking client with no async
runtime, which fits one daily GET from a background thread; native-tls is
the standard OS-TLS facade. New lock entries: `ureq 2.12.1`, `native-tls
0.2.18`, `schannel 0.1.30`, `security-framework 3.7.0`,
`security-framework-sys 2.17.0`, plus Unix-only entries that are not built
for our targets. The R4-ops-design author's `cargo deny check` pass was on
a scratch lock and **has not been independently rerun**; phase C records
it as evidence.

**Rejected alternatives:** `ureq 3` (pulls `webpki-root-certs`,
CDLA-Permissive-2.0, not allowed by `deny.toml`; an owner licence decision,
Q-66); `rustls` with `ring` or `aws-lc-rs` (C toolchain, clang, cmake or
NASM; CDLA root stores); `reqwest` (heavy async stack); direct WinHTTP and
NSURLSession FFI (`unsafe` on two platforms; kept as the fallback if ureq 2
stops receiving security fixes). ureq 2.x is in maintenance mode; revisit
before release (Q-66).

**Architecture guard.** Only `wow-sync-adapters/src/update_http.rs` may name
`ureq` or `native_tls`. An adapters test scans the workspace sources and
fails on any other use, and on any `https://` literal other than the API
URL and the release-page prefix (A-38 control).

**Four-target honesty.** The supported targets are Windows x64, Windows
ARM64, macOS ARM64 and macOS x64. A build or a cross-check is **not**
evidence that TLS, redirects, certificate failures or notifications work at
runtime. Each target's runtime result is recorded as passed, failed,
blocked or not run, on a named native machine. Windows ARM64 and macOS
Intel builds alone satisfy nothing in A-27 or A-28.

---

## 5. Diagnostics export (A-33, R-15, R4D-12)

### 5.1 Flow

1. **Request.** App settings > Data > **Export diagnostics…**. Never
   automatic; no upload or send button.
2. **Build.** Runtime gathers a typed `DiagnosticsInput`. Core
   `diagnostics::build(input, &RedactionContext, options) -> Result<Export,
   ExportRefused>` produces the exact text.
3. **Preview.** A sheet shows **the exact bytes that will be saved**, in a
   scrollable, selectable monospace view, with the size, a summary of what
   was replaced ("Replaced: 4 folder paths, 3 computer names"), the
   never-included list (§5.4), and the warning "Anyone you send this to can
   read it. Do not save it in your shared Dropbox folder: everyone in the
   group can read that folder."
4. **Options.**
   - **Include add-on folder names** (Q-64 PENDING OWNER default: **on**):
     "Add-on folder names exactly as they appear in your AddOns folder.
     Uncheck to replace them with numbers." Only names that match
     `^[A-Za-z0-9_.\- ]{1,64}$` are ever included; any other name is
     replaced by `Add-on N` even when the option is on. Titles, notes and
     version text are never included.
   - **Include file fingerprints** (default **off**): "Short fingerprints of
     add-on files. They show whether two computers have the same files.
     Fingerprints of add-ons you changed yourself are fingerprints of
     private files." When off, no hash or hash prefix appears.
   - Toggling either option rebuilds the preview.
5. **Save.** **Save…** opens the OS save dialog (GPUI
   `prompt_for_new_path`); default name
   `wowsync-diagnostics-<yyyymmdd-hhmm>.txt` in Documents. A path inside the
   shared folder or any install root is refused with the warning text. The
   file is written temp + rename. Nothing is copied to the clipboard
   automatically; a **Copy** button is available.
6. **Cancel** discards everything. Nothing is cached.

### 5.2 Format and bound

- UTF-8 text. The first line is `wowsync-diagnostics/1`, then `key: value`
  sections in a fixed order: About, Environment, Settings, Variants,
  Computers, Addons, Holds, Recent activity, Errors.
- **Technical caps** (contracts.md §8): at most 256 KiB; Activity limited
  to the last 200 entries and Addons to 500 rows, each line ≤ 300 bytes.
  When a limit is hit the section ends with `(… N more omitted)`. Building
  stops at the bound; it never builds a larger string and cuts it.

### 5.3 Contents (contracts.md §7)

The export is built from an allow-list of typed fields. Every exported
value is a closed enum, a number, an alias, an opted-in folder name (§5.1)
or an opted-in hash prefix. **No source string is copied**: no version
label, `.toc` field, record `title`, provider or OS message text.

| Section | Included | Form |
|---|---|---|
| About | app version, build commit, settings toggles (on/off), schema majors | constants of this build |
| Environment | OS family and version, CPU architecture, process memory, WoW running (yes/no/unknown) | enums and numbers |
| Variants | variant kind, verified-identity status, addon counts | install roots → `<install-1>` |
| Computers | per member: alias, enrollment state, retired, last report age, member summary, watermark seq | `Computer A`, `Computer B`, … (`This computer`) |
| Addons | folder name or `Add-on N`; status; candidate count; **version class** (`numeric(n parts)`, `date`, `text`, `none`); decision reason; hold scope; optional 12-hex fingerprint | §5.1 options |
| Holds | `PlanBlock` kinds, journal states | enums |
| Recent activity | entry kind, time, aliases | rendered from typed entries |
| Errors | error kind and OS error code (`Io(PermissionDenied, 5)`) | no OS message text, no paths |

**Second defence: the output scan.** After rendering, `build` scans the
text, after **Unicode NFC normalization and case folding** of both sides,
for every string in `RedactionContext` and for these patterns: SIDs
(`S-1-5-21-…`); Windows profile paths (`[A-Za-z]:\\Users\\…`); macOS
profile paths (`/Users/…`); home (`~/…`); UNC paths (`\\server\…`);
e-mail-like strings; WoW account forms (`WTF\Account\…`, `#\d+` suffixes).
`RedactionContext` holds the home directory, OS user name, computer and
display names, install, shared, Dropbox and local-state roots, and the WTF
account, realm and character folder names the scanner found.

**Any hit is a refusal (fail closed):** `ExportRefused::PrivateDataFound
(kind)`, no partial file, GUI text "WoW Sync could not make a safe export.
Nothing was saved.", and an Activity entry with the kind only.

The scan cannot find a secret it was never told about. That is why no
free-text source field is exported at all; the scan is a second defence,
not the first.

### 5.4 Never included

Saved-variable (WTF) contents; addon files or Lua; `.toc` text, titles,
notes and version labels; account, realm or character names; OS user
names, SIDs or user-profile paths; full or partial filesystem paths;
display names and computer, enrollment and group ids; credentials or
Dropbox tokens (none exist); memory dumps; arbitrary error text; hashes
unless **Include file fingerprints** is on.

Telemetry does not exist. The App settings > Privacy toggle stays "Coming
later".

---

## 6. Implementation map

File ownership and build order are [contracts.md](contracts.md) §9 only;
this section says what each part contains.

**Built by step 1, R4-core-contracts:** historical membership, the
retirement DTO with `retirements_known`, the three-valued barrier
(Effective / Void / Unresolved, Retired Yes / No / Unresolved, Cutoff,
HoldCut), voidness and V-RET (`authority.rs`, `kinds.rs`); the
code-choice DTO with the exclude `scope` and V-CHOICE (`kinds.rs`); the
admission order with cutoff skipping and the hold (`admit.rs`); the
`DecisionReason` rename to `UnusableMetadata(kind)` and
`NewVersionAfterDecision`, `same_release` and whole-addon ordinary
selection (`target.rs`); **every decision DTO** of §1.1 —
`DecisionScope`, `DecisionItem`, `DecisionOption`, `OptionId`,
`OptionLabel`, `Consequence`, `OptionWarning`, `EvidenceToken`,
`FreshEvidence`, `DecisionEffect`, `LocalResolution`, `DecideError`,
`DecideOutcome`, `RetirePlan` (new `decision.rs`); every status, view and
explain variant below (`status.rs`, `view.rs`, `explain.rs`);
`DecisionHooks`, `RetireHooks` and the code-choice half of
`ConstraintHooks` (`hooks.rs`); the runtime cycle, the `decide`, `retire`
and `AppSettings` control variants (`service.rs`, `control.rs`).

**Built by step 2, R4-core-ops:**

| File | Contents | Notes |
|---|---|---|
| core `choice.rs`, `choice_tests.rs` | `ConstraintHooks::{live_constraints, excluded}` for code choices: live keeps per folder, `NewVersionAfterDecision` detection, resume exclusions by `same_release` over each entry's `scope`, the writer's `exclude` formula (contracts.md §5.2–§5.3) | consumed by `target::compose` levels 6 and 8 |
| core `decide.rs`, `decide_tests.rs` | the functions `decision_items` and `DecisionHooks::decide` over the step-1 `decision.rs` types; no type is defined here | pure; freshness contracts.md §5.5 |
| core `retire.rs`, `retire_tests.rs` | `RetireHooks::plan_retirement`: `build_retirement` with the cutoff and `retirements_known`, the master consequence (§2.6) | the barrier itself is step 1 |
| runtime `decisions.rs` | bodies of the decide and retire commands (under the cycle lock), record write through the shared authority path | |

**Step 1 status and view variants** used here: `Headline::ReviewNeeded`;
`StatusInput.reconfirm_pending`; `HoldScope`, `AllHoldCause`,
`DecisionReach`; `StatusView.hold`; `PackageStatus::reach`;
`MemberStatus.retired: Option<RetiredInfo>`; retired members excluded from
`awaiting`/confirmations (§1.6, §1.7); `NextAction::{ReviewSetup,
JoinAgain, ChangeMaster}`.

**Built by step 3, R4-notify-diag.** Every module below already exists as
a step-1 stub, declared in its crate's `lib.rs` by R4-core-contracts
(contracts.md §9.1); step 3 fills the stub files and never edits a
`lib.rs`.

| Crate / module | Adds | Notes |
|---|---|---|
| `wow-sync-core::notice` (new) | §3.2 types; `plan_notices` | pure |
| `wow-sync-core::update` (new) | `ReleaseInfo`, `ValidatedTag`, `SemVer`, `parse_latest`, `compare`, `due`, `release_page_url` | pure; `serde_json` |
| `wow-sync-core::diagnostics` (new) | `DiagnosticsInput`, `RedactionContext`, `ExportOptions`, `VersionClass`, `Export`, `ExportRefused`, `build` | pure; fail closed |
| `wow-sync-adapters::notify` (new) | `Notifier`, `OsNotifier` (notify-rust), `RecordingNotifier` (testkit) | |
| `wow-sync-adapters::update_http` (new) | `ReleaseSource`, `GithubReleases` (ureq + native-tls, no env proxy, redirect policy), `FakeReleases` (testkit) | the only network code; arch guard §4.4 |
| `wow-sync-adapters::winproc` | `register_app_user_model_id`, `set_process_app_id` (Windows) | `windows-sys` `Win32_UI_Shell`, `Win32_System_Registry` |
| `wow-sync-adapters::localstate` | `notices.json`, `update_check.json` | bounded reads (`SMALL`) |
| `wow-sync-runtime::notices` (new) | drives `plan_notices` after each cycle and on game-state change; persists the ledger | |
| `wow-sync-runtime::updates` (new) | single-flight scheduler for `due`; attempt persisted before the request | |
| `wow-sync-runtime::diag` (new) | `export_diagnostics`, `check_updates` bodies behind the step-1 control variants | |

`StatusSnapshot.hold`, `.retirements` and `.update` and the
`AppSettings { update_check, notify_summaries, notify_needs_me }` control
(defaults on) are step-1 `service.rs`/`control.rs` fields.

**Built by step 3, R4-app** (`model`, `pages`, `app`, `world`): headline
and tray for `ReviewNeeded` and retired-self; Decision page from
`DecisionOption`; Computers "Retired" group and sheet; About update states;
notification toggles; **Export diagnostics…** with both options.

**Dependencies to add** (exact pins, E-06): `notify-rust = "=4.19.0"`;
`ureq = { version = "=2.12.1", default-features = false, features =
["native-tls"] }`; `native-tls = "=0.2.18"` (all in adapters); the
`windows-sys` features `Win32_UI_Shell` and `Win32_System_Registry`. No new
dependency goes into core.

---

## 7. Test list

Every new behaviour test gets an A-38 negative control: mutate the named
production function, observe the named test fail, then restore. "Neg"
marks a negative case.

### A-26: decisions (core `decide`, `choice`, app model)

| ID | Test | Kind |
|---|---|---|
| T-26-1 | `Ambiguous` with two copies: one Keep option per copy plus Decide later; holders correct | positive |
| T-26-2 | Keep copy X → the record passes V-CHOICE; `considered` lists both copies; peers' target is exactly X | positive |
| T-26-3 | `decide` after a new copy is learned between showing and choosing → `Changed`; nothing written (with contracts T-C15) | Neg |
| T-26-4 | `DifferentTocMapping`/`UnvalidatedTocMapping` → no options, no notice, no headline promotion | Neg |
| T-26-5 | `PartialCandidate` keep smaller → a keep plus a removal of the dropped folders; they go into recovery | positive |
| T-26-6 | Decision while paused → kept locally with its reserved seq, written on resume | positive |
| T-26-7 | Decision while WoW is running → written and shared; no game file touched until the game exits | Neg |
| T-26-8 | Retired computer → `NotAMember`; no file written | Neg |
| T-26-9 | Change decision → the new record's `known` covers the old; the old keep no longer constrains | positive |
| T-26-10 | Journal **Keep the new version** not offered when the staged tree fails re-hash | Neg |
| T-26-11 | UQ-2: unresolved journal → `AllApplies(UnresolvedJournal)`; one NeedsDecision → `Addons([p])`; the app sentence matches | positive |
| T-26-12 | UQ-1: reconfirm pending + Paused → `ReviewNeeded`; + NeedsHelp → `NeedsHelp`; pause + Error → Error | positive / Neg |
| T-26-13 | NeedsDecision alone does not produce `ReviewNeeded`, and the headline is not "fully synced" (Q-59) | Neg |
| T-26-14 | GUI: Decision page keyboard flow (Tab, Space, Enter); screenshots with interaction evidence | manual A-26 |
| **T-R4D10-new-version-after-keep** | keep V2 among unordered V2/V3 (V3 in `considered`): V3 never ends it; an unconsidered demonstrably newer V4 → `NeedsDecision(NewVersionAfterDecision)`, V2 stays installed until the user chooses; an unconsidered unordered V4′ → the same; A-38 control: auto-install V4 makes it fail (Q-49) | Neg |
| **T-R4D10-resume-keeps-exclusions** | keep V2 declining V3, then resume without allow-again: the resume's `exclude` inherits V3; target V2; with a new V4: V4; a replay of V3's exact content: never selected (Q-50) | Neg |
| **T-R4D23-keep-resume-shared-component** | contracts.md §4.2: kept V2 `{a:2, b:U}`, declined V3 `{a:3, b:U}`, then resume; with a and b grouped, ungrouped, and regrouped (folder c joins) between keep and resume: V2's source stays eligible and is the target; the V3 source is never selected; the carried exclude entry keeps `scope` {a, b}; with c joined, the declined source `{a:3, b:U, c:C}` is `Same` (never selected) and the kept source `{a:2, b:U, c:C}` is `Different` (contracts.md T-R4D28-regrouped-exclusion); A-38 controls: compare on common folders only (V2 becomes excluded, test fails); union the source's keys (the declined source waits forever, test fails) | Neg |
| **T-R4D10-resume-allow-again** | same, but resume with `allow_again = {V3}`: V3 eligible again, target V3; `allow_again` naming a hash not inherited → the writer refuses to write it | positive / Neg |
| **T-R4D10-vchoice-per-action** | V-CHOICE per action (contracts.md §5.2): a keep without `keep` or with `allow_again`, a keep whose `considered` lacks the kept entry, `exclude` containing `keep`, a resume with `keep`, `allow_again` overlapping `exclude`, unsorted lists, folder keys outside `folders` → each `invalid(choice_shape)`; the valid forms of each action pass | Neg |
| **T-R4D21-partial-choice-over-choice** | contracts.md §10: keep X on {a, b}; a later keep Y on {a} covering X → a follows Y, b still follows X; same result whether a and b are grouped locally or not | positive / Neg |
| **T-R4D10-unusable-metadata-no-override** | each `FolderStatus` and `TocKind` cause of `UnusableMetadata` → no option, no record can make it installable; `decide` with any option id → `NotOffered` | Neg |
| **T-R4D10-interface-advisory** | a copy whose Interface number differs from the game's installs normally; no decision, no notice; the detail page shows the advisory line | Neg |

### A-45: retirement (core `retire`, `admit`, `status`; runtime e2e)

| ID | Test | Kind |
|---|---|---|
| T-45-1 | Retire B (cutoff 10): B's state seq 11 delivered later is not admitted; B is not in `awaiting`; the headline does not wait for B | Neg |
| T-45-2 | B's history ≤ 10, its member directory and blobs are untouched (listing before = after) | Neg |
| T-45-3 | B's record with `issued_at_seq` 12 → `invalid(after_retirement)`; one at 9 stays valid | Neg |
| T-45-4 | Retire the settings master with one active computer left → `MasterState::Absent`; no computer becomes master | Neg |
| T-45-5 | Mutual unknowing retirement A↔B (empty `retirements_known`) → both retired on every reader, in every read order (property test); with references whose targets arrive late, the intermediate states are held, never retired or un-retired (T-R4D25-partial-retirement-dag) | positive |
| T-45-6 | B re-enrolls (new enrollment id) → joining member; the old enrollment stays retired; its old valid records stay valid via historical membership | positive |
| T-45-7 | Retired-self: no file written to its member directory in later cycles; no apply; local addons unchanged (Q-60 default) | Neg |
| T-R4D31-held-self | own enrollment held (a retirement of it is Unresolved): headline "Waiting for group history" with no retirement date or "by" line, never "This computer was retired"; `AllApplies(RetirementUnresolved)`; nothing published or applied; a decision made meanwhile is kept unwritten, not discarded; **Join again…** offered. Then (a) the retirement turns Void: publishing and applying resume without user action and the kept record is written with its reserved bytes; (b) it turns Yes: the retired headline appears and the kept record is discarded with an Activity entry; A-38 control: treat held as retired (kept record discarded in (a), test fails) | Neg |
| **T-R4D11-late-retirement-three-issuers** | A retires B (cutoff 10) at A's seq 20; D learns it; then C retires A with cutoff 15 (< 20). Both delivery orders, and with a restart between. B stays retired on D; A is retired; B's states > 10 and B's records > 10 never admitted; a stale B settings revision and a stale B removal stay invalid; B never becomes master again. Same trace as contracts T-C10 | Neg |
| T-45-8 | Two different effective retirement records of B with cutoffs 8 and 12 → Cutoff(B) = 8 in either order (MIN); a record of B at 11 is invalid in both orders, and a third retirement with cutoff 20 re-validates nothing | Neg |
| **T-R4D16-later-cutoff-no-reauth** | contracts.md §10: A retires B at cutoff 10; B's seq-11 removal is invalid; C's retirement of B with cutoff 12 arrives later, in both orders and across a restart → the removal stays invalid; nothing B wrote above 10 is ever authority | Neg |
| **T-R4D16-retired-issuer-retirement-void** | contracts.md §10: B is retired; B (an old app) writes a retirement of A listing its own retirement in `retirements_known` → void; A is not retired; Details lists it as ignored | Neg |
| **T-R4D15-vret-predicate** | contracts.md §10: V-RET over retirement / non-retirement, retired / not-retired / held issuer, `issued_at_seq` below, at and above the cutoff → exactly the contracts.md §1.11 table | Neg |
| **T-R4D25-partial-retirement-dag** | contracts.md §1.10 R/S/T/U trace: complete-first; R, S then T then U; T, U then R, S; a restart after each partial step. Every intermediate step checks each record's status, Retired/HoldCut of A, B, C and D, admitted vs held seqs, `awaiting`, master prompts and whether A/B publish. B is never Retired = Yes; no status flips Effective ↔ Void; the final result is identical in every order; A-38 control: treat Unresolved dependencies as not effective (B retired at partial 1, test fails) | Neg |
| T-45-9 | GUI: retire sheet disabled until the checkbox; 72-hour warning; Cancel default focus | manual A-45 |

### A-27: notifications (core `notice`; runtime with `RecordingNotifier`)

| ID | Test | Kind |
|---|---|---|
| T-27-1 | Batch with 5 applied items → one summary, three names + "and 2 more" | positive |
| T-27-2 | A conflict appears while gaming → no notice; status shows it at once; on game exit exactly one action notice | positive |
| T-27-3 | Unchanged persistent condition over 100 cycles → notified once | Neg |
| T-27-4 | Q-85 default: same fingerprint clears and recurs after 1 h → no notice; after 6 h → one; with a changed fingerprint after 1 h → one; with `recurrence_quiet_ms = None` → one | Neg / positive |
| T-27-5 | Many summaries while gaming → released as ≤ 2 notices; items resolved meanwhile dropped | positive |
| T-27-6 | `ActivityUnknown` is treated as gaming | Neg |
| T-27-7 | `DataAtRisk` while gaming → sent at once; recurring after 1 h → sent again | positive |
| T-27-8 | Q-86 default: `needs_me` off → no OS notice for ActionNeeded, AppUpdate or DataAtRisk; DataAtRisk still in headline, tray and Needs attention; toggling back on does not burst | Neg |
| T-27-9 | Ledger persisted: restart with an open condition → no repeat | Neg |
| T-27-10 | Ledger and held set bounded after 10,000 keys; an evicted key's recurrence notifies | Neg |
| T-27-11 | Native: a notification with the WoW Sync name on each target's native machine; each target recorded passed, failed, blocked or not run. A macOS failure is A-27 FAILED, not a fallback pass (Q-63) | manual |

### A-28: update notice (core `update`; runtime with `FakeReleases`; adapters transport)

| ID | Test | Kind |
|---|---|---|
| T-28-1 | `parse_latest` accepts `v0.2.0` and `0.2.0`; rejects drafts, prereleases, `0.2`, `v0.2.0-rc1`, `V0.2.0`, 10-digit parts, oversize input | positive / Neg |
| **T-R4D14-tag-forms** | `v0.2.0` opens `…/releases/tag/v0.2.0`; `0.2.0` opens `…/releases/tag/0.2.0`; an `html_url` pointing elsewhere is ignored | Neg |
| T-28-2 | `due`: off → never; < interval → no; ≥ interval → yes; future timestamp → yes; < 60 s after ready → no | positive / Neg |
| T-28-3 | Opt-out: 7 simulated days → `FakeReleases` call count 0 | Neg |
| T-28-4 | Offline, 403, 429, timeout, oversize, parse failure → no notice or banner; one Activity line with the kind; About "Could not check"; next attempt one interval later | Neg |
| T-28-5 | Newer tag → one AppUpdate notice per tag, deferred while gaming; same tag next day → none | Neg |
| T-28-6 | The built request has exactly the documented headers; no cookie, auth or query | Neg |
| T-28-7 | Arch guard: `ureq`/`native_tls`/an unlisted `https://` outside `update_http.rs` fails the test (A-38 control by planting a use) | Neg |
| T-28-8 | No code path writes into the app's install directory (static scan + e2e listing) | Neg |
| T-28-9 | Transport, against a local test server through a test-only endpoint constructor: a self-signed certificate, an expired one and a wrong-host one each fail; a redirect to `http` and one to another host fail without fetching it; a fourth redirect fails | Neg |
| T-28-10 | Proxy isolation: with `HTTPS_PROXY`/`ALL_PROXY` set to a local listener, the listener receives no connection | Neg |
| T-28-11 | Single flight: a manual check during a scheduled one makes one request and both show its result | Neg |
| T-28-12 | Attempt persisted before request: the fake transport kills the worker mid-request; after restart `last_attempt_unix_ms` is set and no immediate retry happens | Neg |
| T-28-13 | Live: one real GET against GitHub per native target, run manually by the implementing packet (D-24); each target recorded with its actual result | manual |

### A-33: diagnostics (core `diagnostics`)

| ID | Test | Kind |
|---|---|---|
| **T-R4D12-planted-secret** | plant `PLANTED_SECRET_DO_NOT_EXPORT` (and a case/NFC variant) separately in **every** free-string source: version label, `.toc` title and notes, record `title`, rejection `version_label`, OS error text, provider error text, display name, member alias source, folder name. With both options off: absent from the export. With folder names on: present only when planted in a folder name that passes the name pattern, and nowhere else | Neg |
| T-33-1 | Synthetic install with planted private strings (user `zz_user_PLANT`, home `C:\Users\zz_user_PLANT`, computer `PLANT-PC`, account `PLANTACCT#1`, realm `PlantRealm`, character `Plantchar`, a SID, the Dropbox root, an OS error text with a path) → none appear, exactly, case-insensitively or NFC-normalized | Neg |
| T-33-2 | A planted string forced into a typed field (test-only injection) → `ExportRefused::PrivateDataFound`, nothing saved | Neg |
| T-33-3 | 10,000 activity entries / 5,000 addons → export ≤ 256 KiB with "more omitted" markers | Neg |
| T-33-4 | WTF/Lua/.toc contents planted with unique markers → absent | Neg |
| T-33-5 | Folder names off → `Add-on N` only; on → only names matching the pattern; a non-matching name stays `Add-on N` | positive / Neg |
| T-33-6 | Fingerprints off (default) → no hex run of 12 or more characters from any package hash; on → 12-hex prefixes present | Neg |
| T-33-7 | Version information only as `VersionClass` (`numeric(3)`, `date`, `text`, `none`) | Neg |
| T-33-8 | Saving under the shared folder or an install root → refused | Neg |
| T-33-9 | No export without the user action (no scheduled call to `export_diagnostics`) | Neg |
| T-33-10 | GUI: preview shows exactly the saved bytes (byte compare) | positive |

---

## 8. GUI touchpoints

| Screen (ui/screens.md) | Change |
|---|---|
| Dashboard headline | `ReviewNeeded` (§1.7); "This computer was retired" (§2.4); scope sentence from `HoldScope` (§1.6) |
| Needs attention | items from `DecisionItem`; unusable-metadata and TOC-mapping items without actions; reach label |
| Decision | option cards from `DecisionOption` (label, holders, consequence, warning), **Decide later**, "Something changed…" re-render, saved/unshared notes (§1.3) |
| Add-on detail | **Change decision**; provenance ("Kept by your choice on Laptop, 12 Jan"); advisory Interface line (§1.2) |
| Computers | **Retired** group; **Retire this computer…** sheet (§2.5); retired-self banner with **Join again…** |
| Settings ([settings.md](settings.md)) | "Settings master retired" banner with **Change master…** (§2.6) |
| Activity | decision, retirement, update-check-failed and export-refused entries |
| App settings | notification toggles; "Notifications are not available on this Mac" when the backend fails (an A-27 failure, §3.3); **Export diagnostics…** |
| About | update states; **Check for updates automatically**; **Check for updates**; **Open download page** (§4) |
| Tray | attention tone for `ReviewNeeded` and retired-self |

## 9. Open questions

Q-59 to Q-66 and Q-85 to Q-87 in [questions.md](questions.md). Items
marked PENDING OWNER are not decided. Their conservative defaults may be
implemented and tested, but any acceptance item that depends on them is
reported as blocked on the owner answer, not as passed.
