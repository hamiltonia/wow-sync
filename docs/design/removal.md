# Addon removal and uninstall across the group

Status: **design for a later run** (run-003 packet
[R3-removal-design](../runs/run-003/packets/R3-removal-design.md)). Not yet
implemented and not yet reviewed (R3-design-review). Nothing here changes
the running slice. The slice's rule "absence never removes" stays the
safety base. This document adds the **only** evidence that may remove addon
code and saved settings from a computer.

Product authority: SPEC R-03, R-04, R-06, R-07, R-09, R-10, R-11 and R-12.
Acceptance: A-04, A-09, A-10, A-16, A-17, A-18, A-19, A-20, A-21, A-22,
A-23, A-25, A-26 and A-45. Builds on [domain.md](domain.md) §5–§6,
[publication.md](publication.md) and [apply.md](apply.md). Choices that
approved intent does not settle are **(Q-n)** in
[questions.md](questions.md), Q-40 onwards.

Vocabulary used here:

| Term | Meaning |
|---|---|
| package | the addon unit of [domain.md](domain.md) §5: one or more folders grouped together, replaced and removed together |
| removal intent | the group-visible claim "computer X deliberately uninstalled package P, as these exact folder versions". Carried by a **tombstone** (§2) |
| covered | a folder version that the tombstone's originator knew about when it formed the intent (§2.3). Only covered versions may be removed without a decision |
| acknowledgment | a member's published statement that it has applied a tombstone (removed or never had the package). Used for "Confirmed" and for settling (§2.5) |
| re-add | a deliberate install of a package after its tombstone was acknowledged on the same computer. Supersedes the tombstone (§2.4) |
| folder drop | an update in which the author's new release no longer contains one folder of the package (§1.5) |

## 1. Intent: what counts as a deliberate removal (R-03, R-04)

### 1.1 Principle

Absence is never removal authority, neither locally nor from the provider
(R-04: "A missing Dropbox object, incomplete join or temporary local/provider
failure is not deletion authority"). Removal is derived only from a
**positively observed local transition** on the originating computer:
"this computer had package P, completely and readably, and now has none of
it, and nothing else explains that". The originating computer then
publishes an explicit tombstone. Receivers act only on an admitted tombstone,
never on a peer's state that merely lacks the package.

Detection is local and passive: the user uninstalls with CurseForge, another
updater or Explorer, as today. The app does not require a WoW Sync button to
uninstall (R-05: the app is not a substitute updater). The GUI also offers
"Remove from the group" for convenience (§7). That action produces the same
evidence by moving the folders to recovery itself.

### 1.2 Required evidence (all must hold)

`core::removal::observe_removals(prev, scans, local) -> Vec<RemovalObservation>`
is pure. A package P becomes a **removal intent** of this computer only if
every row holds:

| # | Evidence | Rules out |
|---|---|---|
| E1 | **Prior presence.** This enrollment's newest *published and admitted* own state (seq `s0`) lists every folder of P with `status: ok` and a tree. A local inventory alone is not enough | "never had it"; a new enrollment's missing addons (R-03, A-04) |
| E2 | **Whole package absent.** No folder of P, by `FolderKey`, appears in the listing in **any** status (ok, unreadable, unsafe, unsupported, unstable). Partial absence is not a removal of P (§1.5) | unreadable folder; failed or partial scan of one folder |
| E3 | **Listing is trustworthy.** The AddOns top-level listing was stable and complete in the scan. The variant verdict and layout-assessment digest equal the enrolled ones (domain.md §2.1). The AddOns directory has the same **directory identity** as at `s0` (Windows: volume serial + file ID; macOS: device + inode; recorded locally with every published state) | moved or re-pointed install root; a recreated empty AddOns; variant change (`NeedsHelp(VariantChanged)` already stops the cycle) |
| E4 | **Persistence.** E1–E3 hold in two consecutive complete scans at least `REMOVAL_HOLD` apart (Q-41, default 30 s and two sync cycles). Any reappearance of any folder of P resets the hold | an updater's delete-then-extract during an update; a transient listing glitch |
| E5 | **Not our own move.** No journal (any phase) of this computer names P or any folder of P, and no journal remained after `recover_interrupted` in either scan's cycle | our own apply or recovery temporarily moving folders out of `AddOns` |
| E6 | **No unexplained newcomer.** Between `s0` and now, no **new** non-`ok` top-level entry appeared (unsupported, unreadable, unsafe or unstable), and no new entry whose name differs from a folder of P only by Unicode, case or normalization, or that `paths` rejects (Q-14) | a rename to a non-ASCII or otherwise unsupported name; a folder that became unreadable (it is listed with that status, so E2 fails anyway); a copy that the app cannot read |
| E7 | **Not a mass disappearance.** Fewer than `MASS_REMOVAL` packages satisfy E1–E6 in the same window, and AddOns is not empty. Otherwise the intents are held as `PendingLocalConfirmation` and the GUI asks "Did you mean to uninstall these N addons from all computers?" (Q-42, default: ≥ 3 packages or ≥ 25 % of synced packages, or an empty AddOns) | a wiped or replaced AddOns; a mistaken bulk delete; an updater reset |
| E8 | **P was synced.** P was transfer-eligible here (no `Blocked(UnvalidatedTocMapping)` folder, domain.md §2.1) and is not in a `NeedsDecision` with an unresolved removal kind (§3) | removing something the group never shared |

Holding is persisted locally as `wowsync.removal-observation/1` (§2.6). A
restart does not shorten the hold: after a restart, a fresh scan is needed
again for E4. Reading and holding are allowed while WoW runs (Q-5). Removal
evidence is local only. **Provider state is never an input to E1–E8**, so an
offline or stale provider can only delay publication of an intent, never
create one.

What is **not** evidence: a missing or cloud-only peer state, a peer state
without P, `inventory_complete: false`, a peer's `Unusable`/`Regressed`
admission, record timestamps, file times, or the user's display names.

### 1.3 The cases the packet names

| Situation | Outcome |
|---|---|
| Never had it (new enrollment, or P was added elsewhere after this computer's last state) | E1 fails: no prior present own state. P is an addition and is installed if eligible (unchanged slice behaviour) |
| Failed or partial scan | Listing unstable or unreadable: the cycle publishes nothing (apply.md §1 step 3), so E3 fails. A folder that could not be read is listed with its status, so E2 fails |
| Unreadable folder | Listed as `unreadable`, so E2 fails. Shown as a folder problem, never as a removal |
| Non-ASCII name (Q-14) | Folders with such names are `unsupported`, never synced, so they are never subjects of E1. An ASCII folder that becomes absent while a new `unsupported` entry appears fails E6. Shown as "Can't tell whether *X* was removed or renamed". The user may confirm removal in the GUI (§7), which is then recorded with `evidence: user_confirmed` |
| Provider offline or stale here | The intent is formed from local evidence and kept locally until it can be published. Its coverage basis (§2.3) is whatever was admitted. Staleness can only **add** later decisions, never cover a version the originator did not see |
| Provider offline or stale on a receiver | The receiver has no admitted tombstone, so it changes nothing. A tombstone carried only by an `Unusable` or `Regressed` member state is not used (publication.md §6 no-fallback rule) |
| Updater mid-update (delete then extract) | E4 resets on reappearance. If the folder reappears with new content, it is an ordinary update |
| User renames a folder (ASCII to ASCII) | The old package satisfies E1–E8 and is removed; the new name is a new addition. This is the honest observed meaning. The GUI shows both together in the grouped summary |
| Our apply or recovery moved folders out | E5 fails |

### 1.4 Explicit local removal

"Remove from the group" in the GUI (§7) on a package that is present and
`InSync` creates a local removal op (§4) on this computer first, with
`evidence: user_action`. After it is verified, E1 to E3 hold by
construction, so the hold of E4 is skipped and E7 does not apply (one
explicitly chosen package). The tombstone is then published as in §2. This
path also serves as the confirmation path for E6 and E7.

### 1.5 Folder drop within an update (supersedes domain.md §6 Q-2 when implemented)

A suite whose new release no longer ships a folder is not a package removal.
It is a **membership change carried by the update**. On the computer where
the updater applied the new release, the new package version records
`dropped` folders when, in the same E4 window:

1. E1 held for the old version (all folders present, ok);
2. at least one remaining folder of P changed tree, and the new version is
   `Greater` than the old one under `compare_package` on the common folders;
3. the dropped folders satisfy E2, E3 and E5 to E7 individually.

The published package version then carries
`dropped: [{key, name, trees: [<tree last seen here>]}]`. Folder drop
**supersedes the Q-2 completeness rule of domain.md §6** as follows:

> A candidate `c` from source member S is **complete** if its folder set
> equals the group-wide membership **minus** the folders that `c` lists as
> `dropped`. A receiver applying `c` removes a dropped folder only if its
> local tree for that folder is one of the `dropped.trees` **or** is the tree
> of a candidate that `c` dominates. Otherwise the package is
> `NeedsDecision(DroppedFolderDiffers)`.

Grouping (domain.md §5) still uses the union of observations, so a dropped
folder stays a package member until it is absent on every member or its
drop has been acknowledged everywhere. The apply op for `c` removes it in
the same journal as the replaced folders (§4.2, action `remove`).

Partial absence without an accompanying dominant update (rule 2 fails) is
not a drop and not a removal. The package simply looks partial, as in the
slice (`PartialCandidate` for others; `InSync` with "incomplete" note here).

## 2. Records: the tombstone

### 2.1 Where it lives

Removal intents travel in the member's own code-state record. Each member
directory has a single writer, so there is no new shared mutable file and no
provider conflict (publication.md §1). The state schema becomes
`wowsync.code-state/2` (Q-40). This is a **major** change because it changes
what absence means to a reader. A `/1` reader must not treat a `/2` state as
an ordinary inventory. Old apps refuse it as `UnsupportedMajor`, shown as
"written by a newer version" (Q-13). They then neither remove nor resurrect
anything from it.

New fields of `wowsync.code-state/2` (all required; empty arrays allowed):

```json
{
  "schema": "wowsync.code-state/2",
  "…": "all code-state/1 fields unchanged",
  "addons_dir_id": "<opaque hash of the directory identity, E3>",
  "folders": [
    { "…": "as in /1", "dropped_by_update": false }
  ],
  "packages": [
    { "key": "suite-core", "hash": "<sha256>",
      "dropped": [{ "key": "suite_old", "name": "Suite_Old", "trees": ["<sha256>"] }] }
  ],
  "removals": [ { "…": "tombstone, §2.2" } ],
  "superseded": [ { "intent": "<32 hex>", "by": "readd|decision", "at_seq": 41 } ],
  "acks": [ { "intent": "<32 hex>", "result": "removed|absent|pending|decision", "seq": 41 } ],
  "decisions": [ { "…": "decision record, §3.3" } ]
}
```

- `removals` is a **grow-only set** per member: once an intent appears in a
  member's state it appears in every later state of that enrollment until
  that member supersedes it. It may be compacted (§2.5).
- `superseded` is also grow-only. An intent listed there is inactive for
  this member's view, whatever any other record says.
- Every member that admits a tombstone and acknowledges it **copies** it
  verbatim into its own `removals`. The intent therefore survives its
  originator going offline, being retired (§8) or having an unusable newest
  state.
- `acks` is per intent. Only `removed` and `absent` count as confirmation.
  The originator acknowledges its own intent with `removed` in the same
  state that first publishes it.
- `decisions` is grow-only, like `removals` (§3.3).

### 2.2 Tombstone content

```json
{
  "intent": "<32 hex, random, never reused>",
  "package_key": "suite-core",
  "origin": { "computer_id": "…", "enrollment_id": "…",
              "present_seq": 38, "absent_seq": 39 },
  "evidence": "observed|user_action|user_confirmed|decision",
  "removed": [{ "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" },
              { "key": "suite_data", "name": "Suite_Data", "tree": "<sha256>" }],
  "removed_hash": "<package hash of `removed`>",
  "covers": [{ "key": "suite-core", "trees": ["<sha256>", "…"] },
             { "key": "suite_data", "trees": ["<sha256>"] }],
  "basis": [{ "computer_id": "…", "enrollment_id": "…", "seq": 17,
              "package_hash": "<sha256>|null" }],
  "knows": ["<intent ids of earlier intents/supersessions for this package>"],
  "dependents_at_origin": ["<package keys that hard-depended on a removed folder>"],
  "settings": { "kind": "none_in_this_version" },
  "written_unix_ms": 1767225600000
}
```

The tombstone hash is SHA-256 over
`"wowsync-tombstone-v1\n"` + the canonical encoding of all fields except
`written_unix_ms`. Two records with the same `intent` and different hashes
make that intent **invalid** for the package. The package then goes to
`NeedsDecision(InconsistentRemovalRecord)`, and nothing is removed.
Timestamps are display only (publication.md §2).

`settings` is a placeholder reference filled by the settings design (§5).
It is `none_in_this_version` while settings are not synchronized.

### 2.3 Coverage

`covers` lists, per folder key of the package, every tree that the
originator **observed** when the intent was formed. That is its own last
present tree plus the trees of every candidate it had admitted for that
package, including `Unavailable` candidates whose hash and folder trees it
knew. `basis` records which member state each came from.

A folder version on a receiver is **covered** iff its tree is in `covers`
for that key. A receiver package is covered iff every local folder of the
package is covered and its folder set is a subset of the `removed` key set.

Coverage is decided by content, not by time or by who holds it. This means
two things.

- **Conservative direction:** any version the originator did not see is
  uncovered. An offline update therefore always becomes a decision (§3),
  never a silent deletion.
- **Accepted imprecision (Q-43):** a member that freshly installs exactly a
  covered release concurrently with the removal, without having seen the
  tombstone, is treated as holding the removed release. Its copy is removed
  and goes into a recovery point (restorable, §6), with the grouped summary
  naming it. The alternative, per-folder "present since seq" tracking, is
  listed in Q-43.

### 2.4 Supersession and re-add

A tombstone T stops being active for package P when either:

- **Re-add:** a member M whose own state has `acks` for T with `removed`
  or `absent` later publishes P present. Before that, P was absent in M's
  acknowledging state, and the presence satisfies E4-style persistence:
  two scans, not our own apply. M lists `superseded: {T, by: readd}`. The
  re-added version then propagates as an ordinary addition. This is a
  deliberate install after the user saw the removal, so it is new intent
  (R-04).
- **Decision:** a user resolves a removal decision (§3) with "keep version
  V". The deciding member supersedes the intents named in the decision.

A member that has **not** acknowledged T (it was offline) and holds P is
never treated as re-adding. Its copy is covered (removed when safe) or
uncovered (decision). This is the line between "re-add" and "offline peer
resurrects" (R-06, R-10).

The **active** tombstones for package P are those present in any admitted
member's `removals` and not present in any admitted member's `superseded`.
Both sets only grow, so the result does not depend on arrival order,
duplicates or late delivery (R-06, A-20). Admission is unchanged: only the
newest admitted state of each current enrollment is read, with no fallback.

Uninstalling P again after a re-add creates a **new** intent id. Its `knows`
includes the superseded one.

### 2.5 Lifetime

| Stage | Condition | Carried as |
|---|---|---|
| held | E1–E3 hold, E4 hold not yet complete, or E7 confirmation pending | local `wowsync.removal-observation/1` only; nothing published |
| active | published by the originator; at least one non-retired member has not acknowledged with `removed`/`absent` | full tombstone in originator's and every acknowledging member's `removals` |
| settled | every current non-retired member's newest admitted state acknowledges with `removed`/`absent` | compact tombstone (Q-44): `intent`, `package_key`, `origin`, `covers`, `removed_hash`, `knows`; `basis` and `dependents_at_origin` dropped |
| superseded | listed in any admitted `superseded` | removed from `removals` of the superseding member; other members drop it from `removals` once they admit the supersession, keeping only the `superseded` entry |

A settled tombstone remains active for **coverage**. A late or re-enrolled
member holding a covered version still has it removed when safe (§8). The
default lifetime of a settled tombstone is "until superseded" (Q-44). It
costs a few hundred bytes. Unbounded growth is reviewed together with
provider garbage collection (Q-7) before release.

The provider blobs and tree records referenced by the `removed` trees of an
active or settled tombstone are **retention roots** for the future blob
garbage collector. Without them a group-wide restore from a computer
without a local recovery point is impossible (§6.3). This is an input to Q-7.

### 2.6 Local records (never uploaded)

| Schema | Fields | Limit |
|---|---|---|
| `wowsync.removal-observation/1` | `group_id`, `enrollment_id`, `package_key`, `present_seq`, `first_absent_scan` (local monotonic counter, not a clock), `scans_absent`, `addons_dir_id`, `state` ∈ `holding \| pending_local_confirmation \| ready`, `reason?` | 64 KiB |
| `wowsync.apply-journal/2` | `/1` plus `kind` ∈ `apply \| remove \| restore`, `intent?`, folder `action` gains `remove` (§4.2), `settings_ops?` (§5) | 1 MiB |
| `wowsync.recovery-point/2` | `/1` plus `kind` ∈ `replaced \| removed \| restore_undo`, `intent?`, `settings?` (settings checkpoint reference, §5), `paired` ∈ `paired \| code_only \| settings_not_applicable` | 64 KiB |
| `wowsync.decision/1` | see §3.3 | 64 KiB |

## 3. Conflicts: removal against update and other intents (R-06)

### 3.1 Evaluation

Removal is evaluated per package in `core::target`, after admission and
before the version choice of domain.md §6. Inputs: active tombstones for P,
every admitted candidate for P (local and members), group rejections (§8.1),
and decisions (§3.3). Rejected versions are treated as covered: the group
already refuses them, so they cannot keep P alive.

| Situation (per package P) | Result | Effect |
|---|---|---|
| No active tombstone | unchanged domain.md §6 | — |
| Active tombstone(s); every candidate everywhere is covered or rejected; or there are no candidates | `Remove { intents }` | members holding P remove it when safe (§4); members without P acknowledge `absent` |
| Active tombstone; some candidate V uncovered and not rejected | `NeedsDecision(RemovalVsUpdate)` | **frozen** (§3.2) on every computer |
| Two active tombstones, neither `knows` the other | union of `covers`; result as above | independent removals of the same package agree; both are acknowledged |
| A tombstone and a supersession for P, neither `knows` the other (concurrent "remove" and "keep" decisions or re-add) | `NeedsDecision(CompetingRemovalDecisions)` | frozen |
| Same intent id with different content | `NeedsDecision(InconsistentRemovalRecord)` | frozen; diagnostics show both sources |
| Receiver's local package has folders not in `removed` (grouping differs here) | `NeedsDecision(RemovalScopeDiffers)` **on this computer only** | frozen here; others proceed |
| Removing P would break a hard dependency of a remaining local package Q (§4.4) that was not in `dependents_at_origin` | `NeedsDecision(RemovalBreaksDependency)` **on this computer only** | frozen here; others proceed |
| Folder drop where the local tree of the dropped folder is neither listed nor dominated (§1.5) | `NeedsDecision(DroppedFolderDiffers)` | package frozen here |

Group-wide results are computed from the same inputs on every computer, so
every computer reaches the same group-wide result (R-06 determinism). Per
computer results are explained per computer, like `DifferentClientBuild`.

### 3.2 Safe default while undecided

"Neither resurrection nor deletion is silently preferred" (R-06). For a
frozen package:

- No folder of P is installed, replaced or removed on any computer. That
  includes the uncovered version V spreading to members that still have an
  older covered copy. Spreading V would advance one side of the conflict.
- Members that already removed P stay without it. Their recovery points
  stay **pinned**: they are an alternative (§6.2).
- Members that hold P keep it live. Their copy is the other alternative.
  Their inventory still publishes it.
- The provider objects of every alternative are retention roots (§2.5).
- Other packages proceed (R-06, A-07).
- Status: `NeedsDecision`, shown on every computer that can show it, with
  the alternatives (§7).

### 3.3 Resolving

The user chooses on any member through the GUI (A-26). The choice is
recorded as `wowsync.decision/1`, carried in the deciding member's state
under `decisions` (a grow-only set, like `removals`):

```json
{ "decision": "<32 hex>", "package_key": "suite-core",
  "kind": "removal_vs_update|competing_removal_decisions|inconsistent_removal_record",
  "considered": { "intents": ["…"], "versions": ["<package hash>", "…"] },
  "chose": { "remove": true } ,
  "by": { "computer_id": "…", "enrollment_id": "…", "seq": 52 },
  "knows": ["…"] }
```

- **Keep removed:** a new tombstone with `evidence: decision`. Its `covers`
  is the union of the old `covers` and every considered version, and its
  `knows` lists the considered intents. Members holding V then remove it,
  each into a recovery point.
- **Keep version V:** `superseded` for every considered intent with
  `by: decision`. V then propagates as an ordinary candidate under
  domain.md §6, including its eligibility predicates. If V is not eligible
  on some computer, that computer gets the usual `NeedsDecision(reason)`.
  Removed members install V, and their pinned removal recovery points become
  ordinary history.
- New information (a version neither side `considered`) after a decision
  creates a **new** decision. It does not reverse the recorded one (R-06).
- Decision provenance (who, which alternatives, which choice) stays in the
  audit log and in the decision record. The decision record is retained
  with the tombstone it produced.

Settings choices for a re-added package follow the settings design (R-09).
Removal never merges or chooses between settings copies (§5).

## 4. Apply on receivers (R-11)

### 4.1 Plan

`plan_apply` gains `PackageRemove { op, package, intent, folders:
[FolderRemove { name, key, expected_live: Tree{tree, fingerprint} }],
settings: SettingsRemovalPlan }`. It is planned only when:

- target is `Remove` for P;
- every live folder of P is stable, `ok` and covered;
- no journal exists (Q-25, unchanged);
- `Activity::NotRunning`;
- the dependency check (§4.4) passes;
- the settings plan (§5) is derivable, or settings are not applicable in
  this version.

A member without any folder of P plans nothing. It publishes
`acks: {T, absent}`.

### 4.2 Algorithm

One package per op, as in apply.md §2.2. There is no staging content. Live
folders move to recovery by same-volume rename. Nothing is deleted.

1. **Journal `staging`** (`kind: remove`, every folder `action: remove`,
   `target_tree` = the tree being removed, `expected_live` = Tree). No
   staging directory is created for code.
2. **Destination revalidation** (apply.md §2.2 step 2). Any difference →
   `Replanned(DestinationChanged)`; journal → `rolled_back`
   (`from_phase: staging`); nothing touched. If the user updated P locally
   meanwhile, the next cycle sees an uncovered version → decision.
3. **Recovery point** `kind: removed`, `pinned: true`, `intent`, the
   folders as `replaced`, `installed: []`, settings reference (§5) → then
   **journal `committing`**.
4. **Commit**, per folder in name order: `G`; fingerprint unchanged; `move(L→R)`.
   Then the settings steps (§5.2), each preceded by `G`.
5. **Journal `committed`.**
6. **Verify:** `G`; every folder `Done` (`L` absent, `content(R) == O`).
   Settings verified per §5.2. → **journal `verified`**, else `needs_help`.
7. **Finalize:** recovery point `pinned: false` (it is now an ordinary
   history state, §6). Prune per §6. `remove_journal`. Record
   `acks: {T, removed}` and the report result `removed` in the pending
   report (publication.md §4 durability rules, unchanged).

Report results gain `removed`. Confirmation of a removal on peer X means:
X's newest admitted state has `acks` for the intent with `removed` or
`absent`, **and** its inventory lists no folder of the package. Publication
of a tombstone is not confirmation (A-24).

### 4.3 Slot configurations and crash idempotency (extends apply.md §3–§4.1)

Remove (`O` is a tree, `N` = absent, `S` unused):

| Config | `L` | `R` | Meaning |
|---|---|---|---|
| `Untouched` | present | absent | not moved |
| `Done` | absent | present | moved to recovery |
| `Unknown` | present | present | something recreated `L` after the move (updater or user) |
| `Unknown` | absent | absent | folder vanished externally |

Rollback (phase `committing`), reverse name order:

| Config | Action |
|---|---|
| `Untouched` | none |
| `Done` | `G`; `move(R→L)`; check `content(L) == O`, else `needs_help(RestoreMismatch)` |
| `Unknown` | journal → `needs_help(UnknownInterruptedState)`; nothing moved |

Write order with crash points `†`:

```text
J:staging(remove) † [revalidate, reads] † RP(kind removed, pinned) † J:committing
† f1: L→R † f2: L→R † … † [settings steps, §5.2] † J:committed † [verify, reads]
† J:verified † RP(unpinned) † prune † ack recorded in pending report † remove J
```

Rows added to the apply.md §4.1 argument:

| Crash after | Observed on restart | Recovery |
|---|---|---|
| `J:staging` up to RP write | phase `staging`, all `Untouched` | `rolled_back(from staging)`; cleanup removes the record-only recovery dir; P is re-planned next cycle |
| `J:committing`, any rename boundary | each folder `Untouched` or `Done` | `G`, then rollback §4.3 moves each `Done` back. Re-running after a crash during rollback never repeats a move, because each step is chosen from observation. The removal is retried next cycle (accepted cost, same as apply.md §4.1 item 4) |
| a settings step | per §5.2 table | settings steps have the same observe-then-act form; code rollback runs after settings rollback |
| `J:committed` | all `Done` | verify only, no moves → `verified` or `needs_help` |
| `J:verified`, any finalize step | — | finalize steps are each "skip if done"; the ack is idempotent (the newer result replaces the older one in the pending report); journal removal last |
| `needs_help` | — | nothing touched; recovery point stays pinned |

WoW starting mid-commit behaves as in apply.md §2.3 (Q-4): stop with no
further game-file change, keep the journal, restore after WoW exits. Status
says "WoW started while *X* was being removed; it will be put back after WoW
exits, and removal will be retried."

A removal can also be part of an **update op** (folder drop, §1.5). The
journal then mixes `replace`, `install` and `remove` folders. Each folder
follows its own configuration table, so the one-op chain argument holds
per folder.

### 4.4 Dependency effects

- **Removing a dependency:** before planning, core computes the remaining
  local packages whose selected toc `hard_deps` name a folder of P
  (domain.md §3). If any such package Q exists and Q is not listed in
  `dependents_at_origin`, the result is `NeedsDecision
  (RemovalBreaksDependency)` here. The originator did not accept that
  breakage, so this computer must decide. If Q **is** listed, the
  originator ran with the same broken dependency and the removal proceeds,
  with a warning naming Q.
- **Removing a dependent:** never removes its dependencies. A library P
  used by a removed Q stays installed. Removal is never inferred
  transitively.
- Packages joined by `Dependency`, `NamePrefix` or `Family` edges are one
  package already (domain.md §5), so suite parts leave together.
- Optional dependencies (`OptionalDeps`) are not read (domain.md §3) and
  have no effect.

### 4.5 WoW and other writers

The WoW gate (`G`) precedes every live rename, including rollback and
restore moves (SPEC R-11, A-21). An idle updater does not block (unchanged).
An updater that writes into P's folder after planning causes a fingerprint
mismatch, which gives `Replanned` before commit or a stop in commit. Any
recreated folder found after the move is `Unknown` → `needs_help`. It is
never deleted.

## 5. Settings for removed addons: the interface the settings design must satisfy

The settings design does not exist yet (SPEC R-07 to R-09 are out of the
slice). Until it exists, removal is **code only**. The tombstone carries
`settings: none_in_this_version`, recovery points carry
`paired: settings_not_applicable`, and the GUI says "Settings for this
addon were not changed by WoW Sync." It never claims a paired removal or
restore (R-10, A-19 honesty).

When settings exist, removal and recovery must satisfy this interface
(core-side; adapters follow apply.md's port-trait pattern).

### 5.1 Required operations

| Operation | Contract |
|---|---|
| `settings_scope(variant, package, identities) -> SettingsScope` | Pure. Returns the saved-settings items **owned** by P for each matched account/character identity, plus `unknown_or_shared: Vec<Diagnosis>`. Ownership evidence comes from P's own data (e.g. `SavedVariables` declarations), parsed without executing Lua (R-11). WoW's own UI/cloud files are never items (R-07) |
| `settings_checkpoint(scope) -> CheckpointRef` | Captures the owned items **before** removal. Content-addressed, verified by hash, scoped by `(GroupId, product, identity)`. A checkpoint is a recovery object, retained with its recovery point (§6) |
| `SettingsRemovalPlan` steps | Each item: `expected_live` (hash + fingerprint), same-volume move to `<variant>\.wowsync\recovery\…\settings\…`, `G` before each move. Never in-place edits. Never splits a shared file |
| `settings_restore(checkpoint, identities)` | Moves or writes items back only for the **same** identity they came from (R-02, R-10). Verifies hashes after the move |
| classification for recovery | Same `Untouched / Done / Unknown` configurations as code (§4.3), so the crash argument extends unchanged |

### 5.2 Rules the settings design must keep

1. **Owned only.** Unknown or shared ownership → the code is removed, and
   the unknown/shared items stay untouched. The removal result is
   `removed_code_only(SettingsOwnershipUnknown)` with a diagnosis for that
   scope (R-07: never "delete unowned state to satisfy an addon removal").
   Whether the code removal should instead wait is Q-46.
2. **Paused settings** (R-08) on this computer: the code is removed. The
   paused items stay in place, are recorded as `settings_kept(paused)`, and
   are shown (Q-47). An experiment is neither published nor replaced.
3. **Pairing.** A recovery point of kind `removed` records `paired: paired`
   only if every owned item was checkpointed and verified. Otherwise it
   records `code_only` with the reason. Restoring a `code_only` point
   follows the R-10 fallback: restore code, preserve current settings, warn
   afterwards (A-19).
4. **Retention.** A settings checkpoint referenced by a retained recovery
   point is never pruned before that point (R-10).
5. **No merge.** Restoring settings after a re-add or a decision never
   merges with other copies (R-09). If competing intentional settings exist,
   that is a settings decision under the settings design.
6. **Identity boundaries.** Character-specific items follow matching
   identities only (R-02, A-03). An identity absent on a receiver means
   nothing to remove there for that identity. It is not an error.
7. **WoW writes on exit.** Removal and restore are gated by `G` like code.
   Whether the client rewrites or discards saved variables of an addon that
   is no longer installed must be verified on a real client (Q-45) before
   the settings design claims a removal is complete.
8. **Master role.** Removal is a collection (code) act from any member. It
   does not need the settings master. The master's copy of the removed
   addon's settings is moved to recovery like every other member's.

## 6. History and recovery (R-10, A-16, A-17)

### 6.1 Three states with removal

History is per package, separately for code and settings (R-10). A removal
is a **history state** whose content is "not installed". After removal:

| Position | Code | Settings |
|---|---|---|
| current | removed (nothing live) | removed (nothing live, or `code_only`: settings unchanged) |
| previous 1 | the removed version (recovery point `kind: removed`) | checkpoint taken at removal |
| previous 2 | the version before that, if retained | its checkpoint |

Pruning in finalize (apply.md §2.4) counts states, not recovery points.
`removed` states hold no bytes but occupy a position. Normal pruning keeps
current plus the two previous states. Re-adding P makes `removed` a previous
state. Its byte-holding neighbour is retained while it is within the window.

### 6.2 Temporary exceptions (A-17)

Kept beyond three, each shown as "kept for recovery: N extra copies
(reason)", and returned to three when the reason clears:

| Reason | Kept until |
|---|---|
| removal op unresolved (journal any phase, `needs_help`) | journal cleared or GUI resolution |
| package frozen in a removal decision (§3.2) | decision recorded and applied |
| covered copy removed while another member's copy is uncovered (alternative of a pending decision) | as above |
| restore in progress (§6.3) | restore verified |
| settings checkpoint referenced by a retained code point | that point pruned |

Low storage while an exception exists → `NeedsHelp(StorageLow)` with the
consumption shown. Exceptions are never pruned to make room (R-10).

### 6.3 Restore ("undo removal")

From the package's history (GUI, §7), the user restores a removed package
on this computer:

1. Source: the local recovery point of kind `removed`. If it is absent (this
   computer never had P, or the point was pruned), the provider trees and
   blobs of the tombstone's `removed` version are used, staged and verified
   exactly like an apply (apply.md §2.2). If neither is available:
   `Unavailable`, with the reason.
2. Op `kind: restore`: install or replace with journal, gate and
   verification as apply.md §2.2. The restored folders' live state before
   restore (normally absent) becomes a recovery point of kind
   `restore_undo`.
3. Settings: the paired checkpoint is restored for matching identities if
   present. Otherwise code-only restore with the post-operation warning
   "Settings were not restored: no saved copy from before the removal"
   (R-10, A-19). Partial and paired restores are reported distinctly.
4. After verification, the presence satisfies the re-add rule (§2.4): this
   computer acknowledged T, so its state supersedes T and the package
   propagates to the group. The GUI states this before the user confirms:
   "This reinstalls *X* on all computers in this group" (R-04: the group
   follows one collection; a per-computer exception is not a product
   feature).
5. A version that is group-rejected (§8.1) is not offered for restore
   without first lifting the rejection, which is an explicit, separate
   choice.

## 7. GUI needs (for `docs/design/ui/`)

Screen design is owned by `ui/`. Removal needs these elements and data.
Each is a status/explanation field to be provided by core (A-25, A-26).

| Need | Data from core | Actions |
|---|---|---|
| Removed addons visible in the addon list (filter or section) | package display name, intent origin computer (display name), evidence kind, "seen here at" (local clock), per-member ack state: removed / never had it / pending / decision / unknown (offline) | open detail; restore (§6.3) |
| Local hold notice | `RemovalObservation` state and remaining hold | "Keep it" (opens restore, which also cancels the hold) |
| Mass-removal confirmation (E7) | list of packages, counts, reason the hold triggered | confirm all / confirm selected / keep all |
| Removed-or-renamed (E6) notice | the absent package and the new unsupported entry | confirm removal / keep waiting |
| Explicit "Remove from the group" | consequence text: removed on all computers, kept for recovery here, settings effect (§5) | confirm / cancel |
| Removal vs update decision | per alternative: version label, holders (computers), whether removed where, recovery availability, consequence of each choice | keep removed / keep version V |
| Competing removal decisions; inconsistent record | the conflicting records' origins and choices | choose one |
| Scope differs; breaks dependency (this computer only) | local folder lists vs tombstone lists; the dependent packages | remove anyway (records a decision with `chose: remove` for this computer's alternative) / keep it (records a decision with `chose: keep`, which supersedes the removal for the whole group; stated before confirming) |
| History per package | three states incl. "removed", paired vs code-only flag, exceptions | restore a state |
| Waiting | "WoW is running", interrupted removal, storage low | — |
| Grouped notification | "3 addons removed on this computer (from Laptop)" (R-13 grouping, A-27) | open details |

No internal intent or decision ids are shown outside an expandable details
area. Wording rule: "removed" means removed **here**. "Confirmed removed on
Desktop" requires that member's ack (§4.2). The UI never says "removed
everywhere" before every non-retired member acknowledged (A-24).

## 8. Rollback, rejection and retirement

### 8.1 Rejection and rollback (R-10, A-18)

The rejection design is a later run. Removal requires these interactions:

- A **rejected** release counts as covered for removal (§3.1). A late peer
  holding a rejected release can neither keep a removed package alive nor
  trigger a removal decision.
- Rollback of a package that is currently removed is a restore (§6.3).
  The restore offers only non-rejected states.
- A removal is not a release. It cannot be "rejected". Undoing it is a
  re-add (restore) with its own provenance.
- Rejection records and tombstones are independent grow-only sets keyed by
  package. Neither expires the other.

### 8.2 Retirement (R-03, A-45)

The retirement design is a later run. Removal requires these interactions:

- Tombstones originated by a computer later retired **stay valid**: its
  history is preserved and others have copied the tombstone (§2.1).
  Retirement is not deletion of the collection. It also does not un-delete
  anything.
- A retirement record carries the retired enrollment's last admitted seq as
  known to the retiring computer. States of that enrollment with a higher
  seq are ignored entirely, including any `removals`, `superseded`,
  `decisions` and `acks` they carry. Delayed delivery of pre-retirement
  history is not proof of renewed membership.
- Settling (§2.5) waits only for current non-retired members. A retired
  member's missing ack never blocks settling. Its candidates no longer count
  as alternatives, so they can neither block nor cause a removal decision.
- A re-enrolled (formerly retired) computer is a joining member (§8.3). Its
  old acks do not carry over to the new enrollment.
- If the retired computer was the settings master, master absence handling
  belongs to the retirement and settings designs. Removal does not depend
  on the master (§5.2 rule 8).

### 8.3 Joining with removed addons (R-03, A-04)

A joining computer's locally installed package P that matches an active or
settled tombstone:

- **Covered:** listed in the join consent: "These addons were removed by
  the group and will be removed here (kept for recovery)". After consent
  they are removed like any receiver (Q-48).
- **Uncovered:** `NeedsDecision(RemovalVsUpdate)` for P. The joiner's copy
  stays live.

A joiner's locally **absent** group addons are installed, never treated as
removals (E1 fails).

## 9. Test obligations for the implementation

Every case is behavioural and must fail for a plausible broken
implementation (A-38). Core cases use `core::testkit` (tier `offline`).
Runtime cases use synthetic installs and ≥ 3 replicas with injected delivery
order (tier `offline`). Native rename/in-use cases are tier `native`.
Settings cases become required only when the settings design lands.

| # | Case | Expected | Acceptance |
|---|---|---|---|
| T1 | New enrollment lacks a group addon | installed; no tombstone; no removal ack | A-04 |
| T2 | Package present in published state, deleted locally, two scans past hold | tombstone published with `covers`, `basis`; receivers remove into recovery | A-10 |
| T3 | Single scan absent, then reappears (updater pattern) | no intent; hold reset | A-10, A-21 |
| T4 | Folder becomes unreadable / scan unstable / listing unstable | no intent (E2/E3) | A-05, A-20 |
| T5 | AddOns directory identity changed (re-pointed root) with all folders gone | no intent; needs help | A-05, A-36 |
| T6 | ASCII folder absent while a new unsupported (non-ASCII) entry appears | no intent; "removed or renamed" notice; user confirmation creates `user_confirmed` intent | A-23 |
| T7 | ≥ `MASS_REMOVAL` packages vanish | held as pending confirmation; nothing published until confirmed | A-10, A-20 |
| T8 | Our own interrupted apply leaves `L` absent (`Half`) | no intent (E5); recovery restores | A-22 |
| T9 | Peer state lacks P, peer newest state unusable or regressed, provider object missing | nothing removed anywhere | A-20 |
| T10 | Offline peer updated P (uncovered) then reconnects | `NeedsDecision(RemovalVsUpdate)` on every computer; package frozen; both alternatives preserved and listed; other packages proceed | A-10, A-17, A-07 |
| T11 | Decide "keep removed" then "keep V" (separate runs) | respective propagation; decision provenance in audit and record | A-10, A-25 |
| T12 | Concurrent "keep removed" and "keep V" decisions on two members | `CompetingRemovalDecisions`; no silent winner | A-20 |
| T13 | Late / duplicate / reordered delivery of tombstone, supersession and decision (property test over permutations, 3 replicas) | identical result independent of order; no reversal of a recorded decision | A-20 |
| T14 | Two members independently remove P | union; both acked; no decision | A-20 |
| T15 | Re-add after ack on the same member | supersedes T; propagates as addition | A-10 |
| T16 | Offline member that never acked holds a covered version | removed when safe, not treated as re-add | A-10, A-18 |
| T17 | WoW starts before / between / after removal renames | gate stops; no further change; rollback after exit; no process killed | A-21 |
| T18 | Crash after every effect of the remove op and of its recovery (incl. a second crash during recovery); fakes and real temp filesystem | every restart reaches `Untouched` or `Done`; no lost folder; accurate status | A-22 |
| T19 | Folder recreated in `L` after `L→R` | `Unknown` → `needs_help`; nothing deleted | A-22 |
| T20 | Mixed update op with a folder drop; local dropped tree listed / not listed | removed with the update / `DroppedFolderDiffers` | A-09 |
| T21 | Receiver grouping has an extra folder not in `removed` | `RemovalScopeDiffers` on that computer only | A-09 |
| T22 | Remaining package hard-depends on removed folder, not in / in `dependents_at_origin` | decision here / proceeds with warning | A-09 |
| T23 | More than three history states with a removal among them; prune | current + two previous retained; `removed` state counts; exceptions visible; low storage → needs help | A-16, A-17 |
| T24 | Restore removed package with local point; without local point (from provider); with pruned provider objects | restored and propagated / staged from provider / `Unavailable` | A-10, A-16 |
| T25 | Restore when settings checkpoint missing | code-only restore, current settings preserved, distinct warning (settings phase) | A-19 |
| T26 | Remove with settings owned / unknown-shared / paused (settings phase) | paired / code-only with diagnosis / kept with notice; WTF native files untouched | A-09, A-10, A-15 |
| T27 | Rejected release on a late peer for a removed package | covered; no decision; not reinstalled | A-18 |
| T28 | Retired member's post-retirement state carries a tombstone or supersession | ignored; settling does not wait for retired member | A-45 |
| T29 | Joining computer with a covered / uncovered copy of a removed addon | listed in consent and removed / decision | A-04 |
| T30 | `code-state/2` read by a `/1` reader, and `/1` state read by `/2` reader | `/1` reader shows "written by a newer version" and does nothing; `/2` reader treats `/1` absence as absence (never removal) | A-36, A-20 |
| T31 | Same intent id with different content | `InconsistentRemovalRecord`; nothing removed | A-23 |
| T32 | GUI: remove, see pending/confirmed per computer, resolve a removal decision, restore — without a terminal, keyboard only | flows complete; wording does not claim "removed everywhere" early | A-25, A-26, A-24 |
| T33 | Installed-app removal through real Dropbox on two computers (provider tier, after owner Check 2) | removal applied and confirmed; provenance retained | A-34 |

A-38 negative controls for at least T2, T3, T9, T10, T13, T16 and T18 must
show a named test failing (exit 101) when the corresponding rule is
removed, for example by treating absence as removal or by dropping the
coverage check.

## 10. Changes to existing design documents when implemented

Recorded here because this packet does not own those files.

| Document | Change |
|---|---|
| [domain.md](domain.md) §6 | **Supersedes domain.md §6 Q-2 when implemented:** completeness is "group-wide membership minus the candidate's evidenced `dropped` folders" (§1.5). The final sentence "There is no removal record in the slice" is replaced by: "Absence never removes; only an admitted active tombstone (removal.md §2–§3) yields `Remove`, and an uncovered candidate under a tombstone yields `NeedsDecision(RemovalVsUpdate)`." New `DecisionReason` values: `RemovalVsUpdate`, `CompetingRemovalDecisions`, `InconsistentRemovalRecord`, `RemovalScopeDiffers`, `RemovalBreaksDependency`, `DroppedFolderDiffers` |
| [domain.md](domain.md) §1 step 5 | Consent text changes from "Addons missing here are installed, never treated as removed" to also state that addons deliberately uninstalled on one computer are removed on the others and kept for recovery, and that settings effects follow §5 |
| [publication.md](publication.md) §2 | `wowsync.code-state/2` (§2.1), local schemas of §2.6; report result `removed`; confirmation rule of §4.2 |
| [apply.md](apply.md) §2.2, §3.1, §4.1 | remove op, remove slot table, crash rows (§4.3); "Folders of the package present locally but absent from the target cannot occur in a plan" is replaced by the folder-drop rule |
| [apply.md](apply.md) §2.4 | pruning counts history states including `removed` (§6.1) |
| [apply.md](apply.md) §6 | statuses `RemovalHeld`, `RemovalPendingHere`, `RemovedHere`, `RemovalConfirmedBy(ids)` |
| `core::target`, `core::plan`, `core::exec`, `core::status` | new pure `core::removal` module (observe, activity, coverage, settle); one implementation shared by runtime and tests (A-43) |
