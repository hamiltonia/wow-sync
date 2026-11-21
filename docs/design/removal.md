# Addon removal and uninstall across the group

Status: **run-004 rewrite** (packet
[R4-removal-design](../runs/run-004/packets/R4-removal-design.md)) for owner
decisions D-21 ("removed means removed everywhere") and D-23 ("any removal
wins; conflicting removal records merge into one removal") in
[OWNER-DECISIONS.md](../OWNER-DECISIONS.md). It replaces the run-003 design
in full. **Not implemented.** It must pass the independent run-004 design
review (R4-design-review) before phase C implements it. Nothing here changes
the running slice, whose rule "absence never removes" stays the safety base.

**Contract.** Revised by packet
[R4-design-fix](../runs/run-004/packets/R4-design-fix.md) for the
[R4-design-review](../runs/run-004/evidence/R4-design-review.md) findings.
Record identity, file naming, hashing, references, the authority index,
the ledger, `Known` coverage, validation (V-BYTES to V-KIND), historical
membership, the retirement barrier, mirroring, repair, capabilities,
journals, recovery points, identities and target precedence are defined
**only** in [contracts.md](contracts.md). This document defines the removal,
re-add and ack bodies, their kind rules (V-REMOVAL), the merge, observation
and the remove operation. Choices marked PENDING OWNER use the conservative
default in [questions.md](questions.md); they are not owner decisions.

Product authority: SPEC R-03, R-04, R-05, R-06, R-07, R-09, R-10, R-11,
R-12 and R-13. Acceptance: A-04, A-09, A-10, A-16, A-17, A-18, A-19, A-20,
A-21, A-22, A-23, A-24, A-25, A-26 and A-45. Builds on
[domain.md](domain.md) §5–§6, [publication.md](publication.md),
[apply.md](apply.md) and [core-api.md](core-api.md). Open choices are
**(Q-n)** in [questions.md](questions.md); this rewrite adds Q-54 to Q-58.

## 0. What changed from the run-003 design, and why

D-21 makes a deliberate removal win over a concurrent offline update. D-23
extends that to removal records themselves: any valid removal wins, and
several removal records for the same addon are one removal. The run-003
design needed group decisions because removal could lose. Under D-21/D-23 it
cannot lose, so those decisions and everything that served them go away.

| Run-003 mechanism | Run-004 |
|---|---|
| Random object `id` plus content hash; a `conflicted` status when one id had two hashes; `InconsistentRemovalRecord` decision (R3D-13) | **Removed.** A record's identity **is** the SHA-256 of its exact bytes, which is also its file name (§2.1, [contracts.md](contracts.md) §1.3). Two different contents are two records, both counted. "Same id, different content" cannot be expressed |
| Records scoped by one `package_key`, validation V7 "same package" | **Replaced.** Records name **folder keys**. Merging and superseding are evaluated per folder key (§3), so records with different package sets or spanning packages merge without any package-scope rule. The removal kind rules (V-REMOVAL-1 to V-REMOVAL-4) check only listing well-formedness (§2.6) |
| Coverage (`covers`, `basis`), "uncovered" copies, `NeedsDecision(RemovalVsUpdate)` | **Removed** (D-21). Every copy is removed; the copy goes to recovery and is named (§5) |
| Removal decisions, `chose`/`declined`, selection constraint, `NewVersionAfterDecision`, `OnlyDeclinedVersions`, `CompetingRemovalDecisions` (R3D-10, R3D-16) | **Removed.** Removal never chooses a version. Rejecting a version is [rollback.md](rollback.md); keeping or resuming a version is the code-choice family ([contracts.md](contracts.md) §5) |
| `RemovalScopeDiffers`, `RemovalBreaksDependency`, `DroppedFolderDiffers` per-computer decisions | **Removed.** Named folders are removed; unnamed folders stay (Q-58 PENDING OWNER); broken hard dependencies are a warning (§5.4, Q-54) |
| E7 mass-removal confirmation (Q-42) | **Removed** (D-21 rejected Q-42) |
| `Unavailable(AuthorityPending)` for a whole package | **Narrowed.** Unreadable referenced information only defers **installs** of the folders it names, and the GUI can dismiss it (§4.4) |
| `wowsync.code-state/2` new major (Q-40) | **Replaced** by additive `authority_index` and `features` fields in `code-state/1` (contracts.md §1.6, §11, Q-56) |

Kept, with their run-003 review fixes: the presence baseline with
per-package snapshots (R3D-7, R3D-14), the settle wait (Q-41), the durable
grow-only ledger with mirroring on learning (R3D-8, Q-51; now
contracts.md §1.7, §1.12), immutable hashed records (R3D-9), bounded
payload retention (R3D-15, Q-53), the remove operation with its crash
table, code **and** settings removal, the three-state history, GUI restore,
the WoW gate and the retirement barrier (contracts.md §1.10).

Changed for the R4-design-review: re-adds and other overriding records use
`Known` coverage instead of unbounded `supersedes` lists (R4D-4); acks and
obligations are per `(enrollment, removal, folder)` (R4D-2); a discharged
obligation releases only states at or after the ack (R4D-3); the observing
computer removes its own settings (R4D-5); a reappearance only offers an
explicit add-back (Q-55 PENDING OWNER).

Vocabulary:

| Term | Meaning |
|---|---|
| package | the addon unit of [domain.md](domain.md) §5: folders grouped together. Packages are a **local** grouping; records do not depend on it |
| folder key | `paths::FolderKey`, the case-folded folder name. The unit of removal |
| removal record | an immutable, content-addressed group record: "this computer deliberately removed these folders, as these trees" (§2.3). The tombstone |
| re-add record | an immutable record: "this computer explicitly brought these folders back", superseding the removals of those folders that its `known` covers (§2.3, §3.5) |
| ack | an immutable record: per `(removal, folders)` item, "this enrollment removed these folders" (`removed`) or "never had them" (`absent`) (§2.3) |
| ledger | this computer's durable, grow-only, validated set of learned records ([contracts.md](contracts.md) §1.7) |
| merged removal | the set of folder keys that are removed for the group, computed from the ledger only (§3.1) |
| audience | the member enrollments the issuer had admitted when it wrote a removal record. Used only to decide which leftover copies are stale (§3.4) |
| presence baseline | this computer's durable record of the last complete scan in which a package was fully present (§1.6) |
| hold | a local, temporary state in which nothing is installed, replaced or removed for a package on this computer (§1.6, §1.7) |

## 1. Observation: what counts as a deliberate removal (R-03, R-04)

### 1.1 Principle

Absence is never removal authority, locally or from the provider (R-04).
Removal comes only from a **positively observed local transition** on the
originating computer ("this computer had package P, completely and
readably, and now has none of it, and nothing else explains that"), from an
explicit GUI action (§1.4) or from a folder drop within an update (§1.5).
The originating computer then writes an immutable removal record (§2.3).
Receivers act only on valid records in their ledger, never on a peer state
that merely lacks the package.

Detection is passive: the user uninstalls with CurseForge, another updater
or Explorer (R-05: the app is not a substitute updater). The GUI also offers
"Remove from all computers" (§1.4).

### 1.2 Required evidence

`core::removal::observe(baseline, inventory, journals, observations, scan)
-> ObserveOutcome` is pure and runs at cycle step 3a (§1.6). Package P
becomes a removal of this computer only if every row holds:

| # | Evidence | Rules out |
|---|---|---|
| E1 | **Prior presence.** A durable presence-baseline entry for P exists (§1.6), written from a stable, complete scan of the same variant and AddOns directory in which every folder of P was present, `ok` and transfer-eligible. Published or admitted own states are **not** read | "never had it"; a new enrollment's missing addons (R-03, A-04) |
| E2 | **Whole package absent.** No folder of P, by `FolderKey` from P's baseline entry, appears in the listing in **any** status (ok, unreadable, unsafe, unsupported, unstable) | unreadable folder; partial scan; partial absence (§1.5) |
| E3 | **Listing is trustworthy.** The AddOns top-level listing was stable and complete. The variant verdict is the enrolled identity (no `VariantChanged`) and the layout assessment is `Supported` with the enrolled mapping ([domain.md](domain.md) §2.1). The assessment digest is not compared (it changes with every install and removal). The AddOns directory has the same **directory identity** as in the baseline (Windows: volume serial + file ID; macOS: device + inode) | re-pointed install root; a recreated empty AddOns; variant change |
| E4 | **Persistence (Q-41, D-21).** E1–E3 hold in two consecutive complete scans at least 30 s apart. Any reappearance of any folder of P resets the hold. A restart does not shorten it | updater delete-then-extract; transient listing glitch |
| E5 | **Not our own move.** No journal (any phase) names P or a folder of P, and no journal remained after `recover_interrupted` in either scan's cycle | our own apply, removal or restore |
| E6 | **No unexplained newcomer.** Compared with the top-level snapshot referenced by **P's own** baseline entry (never a later snapshot refreshed by another package, R3D-14), no new non-`ok` top-level entry appeared, and no new entry whose name differs from a folder of P only by Unicode, case or normalization, or that `paths` rejects (Q-14) | a rename to a non-ASCII or unsupported name |
| E7 | *Withdrawn.* D-21 rejected Q-42: no confirmation when many addons vanish at once. A wiped AddOns with an unchanged directory identity is a removal of every package after the settle wait; every copy goes to recovery (§5) | — |
| E8 | **P was synced.** No folder of P was `Blocked(UnvalidatedTocMapping)` ([domain.md](domain.md) §2.1) | removing something the group never shared |

If E1–E5 and E8 hold but E6 fails, P is **unclear**, not removed: the local
target is `Held(RemovalUnclear)` and the GUI asks "Was *X* removed, or
renamed to *Y*?" (§9). Confirming writes a removal with `evidence:
user_confirmed`; "Keep it" cancels the observation and P is an ordinary
addition again; reappearance resets it.

**Provider state is never an input to E1–E8.** An offline or stale provider
can only delay publication of a removal, never create one. Reading and
holding are allowed while WoW runs (Q-5).

Not evidence: a missing or cloud-only peer state, a peer state without P,
`inventory_complete: false`, a peer's `Unusable`/`Regressed` admission,
record timestamps, file times or display names.

### 1.3 Cases

| Situation | Outcome |
|---|---|
| Never had it (new enrollment, or P was added elsewhere after this computer's last complete scan) | E1 fails. P is an addition (unchanged slice behaviour) |
| Failed or partial scan | E3 fails; the cycle publishes nothing (apply.md §1 step 3) |
| Unreadable folder | listed as `unreadable`; E2 fails; shown as a folder problem |
| Non-ASCII rename (Q-14) | E6 fails: `Held(RemovalUnclear)` and a GUI question |
| Provider offline or stale here | the record is formed from local evidence, kept locally, published when possible |
| Provider offline or stale on a receiver | a receiver that has not learned the record changes nothing for it; a record it has learned stays in its ledger ([contracts.md](contracts.md) §1.7) |
| Updater mid-update (delete then extract) | E4 resets on reappearance; new content is an ordinary update |
| User renames ASCII to ASCII | the old package is removed; the new name is a new addition; the GUI groups both in the summary |
| Our apply or recovery moved folders | E5 fails |
| Many addons vanish at once | each is a removal after the settle wait (E7 withdrawn, D-21) |

### 1.4 Explicit removal from the GUI

"Remove from all computers" on a present package runs a local remove
operation (§5) first. After it is verified, E1–E3 hold by construction, so
E4 is skipped. The removal record carries `evidence: user_action`, and this
computer's own `removed` ack is written with it.

### 1.5 Folder drop within an update

A suite whose new release no longer ships folder X is a removal of X only,
carried by the update. On the computer where the updater applied the new
release, in one E4 window:

1. E1 held for the old version of P (all folders present, `ok`);
2. at least one remaining folder of P changed tree, and the new version is
   `Greater` than the old one under `compare_package` on the common
   folders;
3. the dropped folders satisfy E2, E3, E5, E6 and E8 individually.

The computer then writes a removal record with `evidence: folder_drop`
naming only the dropped folders. It merges like any other removal (§3). A
receiver removes X into recovery whatever its local tree of X is, and when
the update of P is planned in the same cycle the planner puts both into one
`apply-journal/2` operation with `kind: apply` (contracts.md §2.2, §5.1). If the author later ships X again while X is removed, the
reappearance is held and offers an explicit add-back (§1.7).

The computer that observed the drop also moves X's owned settings in its
own settings-cleanup operation before its ack (§6.3, R4D-5).

Because X is in the merged removal, grouping and completeness exclude it
(§4.1). The new version of P is therefore complete, which replaces the
slice limitation of [domain.md](domain.md) §6 (Q-2). Partial absence
without a dominant update is neither a drop nor a removal; the package looks
partial as in the slice.

### 1.6 Presence baseline and cycle insertion point (R3D-7, R3D-14)

E1 must not depend on the newest published own state: the cycle publishes
the absent inventory in the first cycle that sees P missing, and own state
files older than the newest three are pruned (publication.md §4 step 7). It
is a separate, local, durable record, `wowsync.presence-baseline/1` (§11).

- **Written** for package P from any scan in which the listing was stable
  and complete, the variant and mapping are the enrolled ones, no journal
  names P, and every folder of P is present, `ok`, not unstable and
  transfer-eligible. The entry holds P's folder keys, names and trees, the
  package hash, the local scan counter, a reference to that scan's
  **top-level snapshot** (every top-level name with its status), and each
  folder's **settings declarations**: the `SavedVariables` and
  `SavedVariablesPerCharacter` names parsed (never executed) from its
  selected `.toc`. These are the ownership evidence for the settings
  cleanup after the folder is gone (§6.3, R4D-5).
- **Snapshots** are kept while at least one package entry references them.
  Refreshing package Q never replaces the snapshot that a still-absent
  package P references (R3D-14).
- **Persisted** with temp + flush + rename before the cycle publishes.
- **Refreshed** when P is again fully present. A refresh replaces the folder
  list; it never merges absent folders in.
- **Cleared** for P when P's removal record is written **and** the
  settings cleanup for P is verified (or not applicable) **and** the own ack
  is written (§6.3); when a folder drop is recorded and cleaned up (for
  those folders); when "Keep it" is chosen
  for an unclear observation and P is present again, or when P is removed
  here by the merged removal. The **whole** baseline is discarded on a new
  enrollment, a variant change or a directory-identity change.
- **Not** written from a partial or unstable listing, from a scan during
  which a journal existed, or for a package with any non-`ok` folder.

**Cycle.** New steps 2a, 3a and 3b extend apply.md §1:

```text
1  recover_interrupted (apply-journal /1 and /2, contracts.md §2)
2  read member states; learn records and index chunks into the ledger,
   validate, mirror, repair own authority\ (contracts.md §1.12)
2a admit member states in contracts.md §6 order (retirement barrier first);
   persist watermarks
3  scan AddOns → build_inventory   (listing unstable → stop, as today)
3a observe: update/persist baselines and observations (E1–E8, §1.7);
   a matured observation writes its removal record and ledger entry.
   Then classify settings for every due (r, x) with x not present here,
   origins included (§5.6): cleanup_required, or no_cleanup(reason)
3b acknowledge: write `absent` ack items only for due (r, x) classified
   no_cleanup (§5.6); for cleanup_required items nothing is written here
4  publish (inventory, authority_index, features) if anything changed;
   every record and index chunk a state references is written and read
   back first
5  merge_removals (§3) → target::compose (contracts.md §4)
6  activity gate → plan_apply (removes, settings_cleanup ops for every
   cleanup_required (r, x), installs, replaces); WoW running → nothing
   planned, cleanup_required items stay unacked
7  run_apply (a remove or settings_cleanup finalize writes its ack from
   pending_records); rescan; 3a, 3b, 4 again
8  derive_status
```

**Held packages.** While an observation for P is `holding` (E1–E3 held at
least once, E4 not yet matured) the local target is `Held(RemovalObserved)`;
while it is `unclear`, `Held(RemovalUnclear)`. Nothing for P is installed,
replaced or removed here, so the addition rule cannot reinstall P during the
hold and undo the user's uninstall. The observation ends by reappearance
(hold reset, baseline refreshed), by maturing (the record is written and P's
target then comes from the merged removal, §4), by "Keep it" (§9), or by a
baseline discard.

### 1.7 Reappearance after a removal: offer an explicit add-back (Q-55 PENDING OWNER)

After this computer applied a removal (it holds valid own ack items for
every live removal naming the folders), the merged removal would remove any
copy that appears again. A user who reinstalls the addon with an updater or
Explorer is therefore held, not fought, and **not** silently re-added:

- If any folder in the merged removal becomes present here and E3 and E5
  hold, the local target for its package is `Held(ReinstallObserved)`.
  Nothing is removed and nothing is published as a re-add during the hold.
- Needs attention shows "*X* was installed again on this computer, but it
  was removed from all computers" with two actions:
  - **Add back to all computers**: writes a `user_add_back` re-add (§2.3)
    for the present folders, after the same settle rule as E4 (two complete
    stable scans at least 30 s apart). The live copy is the re-added
    version.
  - **Remove again**: the hold ends; the copy is removed into recovery like
    any receiver's copy (§5).
- The hold persists across restarts. It is never converted into a re-add
  automatically, because a reappearance may come from an updater rather
  than a deliberate user action (Q-55 conservative default). The run-004
  draft's `observed_reinstall` evidence is removed.
- If this computer does **not** hold own ack items for those removals (it
  never applied them, for example an offline peer), a present copy is not
  a reappearance. It is a stale copy and is removed into recovery (§3.4).
  This is the line between an explicit re-add and an offline peer
  resurrecting an addon (R-06, R-10).

The GUI restore (§7.4) is the other explicit re-add.

## 2. Records and the ledger

### 2.1 Where they live, identity, refs and the ledger

All of this is the shared contract, not removal-specific:

| Topic | Defined in |
|---|---|
| Location under `members\<computer_id>\authority\`, write-if-absent, single writer | [contracts.md](contracts.md) §1.2 |
| Identity = SHA-256 of the exact bytes = file name; bad copies | [contracts.md](contracts.md) §1.3, §1.4 |
| Envelope (`schema`, `group_id`, `issuer`, `written_unix_ms`) | [contracts.md](contracts.md) §1.4 |
| `AuthorityRef { hash, kind, folders }` and the routing hint | [contracts.md](contracts.md) §1.5 |
| Authority index chunks referenced from `code-state/1` | [contracts.md](contracts.md) §1.6 |
| Ledger segments, `dismissed`, memory | [contracts.md](contracts.md) §1.7 |
| `Known` coverage | [contracts.md](contracts.md) §1.8 |
| Size bounds and splitting | [contracts.md](contracts.md) §1.9 |
| Validation V-BYTES to V-KIND | [contracts.md](contracts.md) §1.11 |
| Learning, mirroring and repair | [contracts.md](contracts.md) §1.12 |
| Capability `removal/1` | [contracts.md](contracts.md) §1.13 |

Readers list `authority\` of **every** member directory, including members
whose newest state is `Unusable` or `Regressed` and retired members.
Inventory (code candidates) still comes only from admitted states
(publication.md §6 no-fallback rule).

### 2.2 (merged into §2.1)

### 2.3 Record kinds

The envelope fields (`schema`, `group_id`, `issuer {computer_id,
enrollment_id, issued_at_seq}`, `written_unix_ms`) are
[contracts.md](contracts.md) §1.4. The bodies below are removal's.

**Removal record** (`wowsync.removal/1`, the tombstone):

```json
{
  "schema": "wowsync.removal/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 39 },
  "evidence": "observed|user_action|user_confirmed|folder_drop|rollback",
  "rejection": "<rejection hash, only for evidence rollback>",
  "title": "Suite",
  "folders": [
    { "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" },
    { "key": "suite_data", "name": "Suite_Data", "tree": "<sha256>" }
  ],
  "removed_hash": "<package hash of folders>",
  "audience": [ { "computer_id": "…", "enrollment_id": "…" } ],
  "settings": { "kind": "none" },
  "written_unix_ms": 1767225600000
}
```

- `folders` is the **exact restorable manifest** from the presence baseline
  (or the live package for §1.4). With the provider tree records and blobs
  it can restore the folders on any computer (§7.4). For `folder_drop` it
  lists only the dropped folders. At most 256 folders; a larger removal is
  split into several records (contracts.md §1.9).
- `title` is display text from the selected toc, untrusted: rendered as plain
  text with colour codes stripped (`detail::strip_colour_codes`), never
  exported (contracts.md §7).
- `audience` is the set of current, non-retired member enrollments the
  issuer had admitted, plus itself (§3.4).
- `settings` is `{ "kind": "none" }` (settings not handled by this record)
  or `{ "kind": "checkpoints", "roots": [SettingsRoot] }`, where
  `SettingsRoot { unit, state: RootState }` is
  [contracts.md](contracts.md) §2.3 and the unit is the folder-keyed
  `SettingsUnit` of contracts.md §3.2 (§6).

**Re-add record** (`wowsync.readd/1`):

```json
{
  "schema": "wowsync.readd/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 57 },
  "evidence": "user_restore|user_add_back|rollback",
  "rejection": "<rejection hash, only for evidence rollback>",
  "title": "Suite",
  "folders": [ { "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" } ],
  "present_hash": "<package hash of folders>",
  "known": { "seen": [ { "enrollment_id": "…", "seq": 39 } ], "extra": [], "parts": [] },
  "written_unix_ms": 1767225600000
}
```

`known` is the issuer's causal summary (contracts.md §1.8); entries that
do not fit inline go into parts (contracts.md §1.9), so a re-add is always
writable. The re-add supersedes, for each of its folders, exactly the
removals naming that folder that `known` covers. There is no `supersedes`
or `acks` list (R4D-4).
`user_restore` is the GUI restore (§7.4); `user_add_back` is "Add back to
all computers" (§1.7, §7.4).

**Evidence `rollback`** (R4D-20): a rollback across a folder change writes,
in the same act and the same journal's `pending_records`, a removal of the
folders the rejected release added and a re-add of the folders it dropped
([rollback.md](rollback.md) §3.1). Both carry `rejection`, the hash of
that act's rejection record (V-REF, kind `rejection`), and are valid only
if it is valid; a later lift does not end them (allowing the release again
writes its own re-add, rollback.md §2.2). They merge like any other removal
and re-add.

**Ack** (`wowsync.removal-ack/1`):

```json
{
  "schema": "wowsync.removal-ack/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 41 },
  "items": [
    { "removal": "<removal record hash>", "folders": ["suite-core", "suite_data"] }
  ],
  "result": "removed|absent",
  "removed": [ { "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" } ],
  "settings": "paired|partial|code_only|kept_paused|not_applicable",
  "written_unix_ms": 1767225600000
}
```

- Each item discharges this enrollment's obligation for exactly the listed
  `(removal, folder)` pairs, never for other folders of that removal
  (R4D-2, §3.4). At most 256 items per ack; more are split
  (contracts.md §1.9).
- `removed` is the manifest this computer moved to recovery (empty for
  `absent`). It names, for example, an offline peer's updated copy, so the
  summary can say "Laptop's copy (v2.1) was kept in recovery on Laptop"
  (A-10).
- The issuer of a removal writes its own ack after its settings cleanup
  (§6.3).

Unknown fields are ignored within a major; an unknown major is
`pending(unknown_schema)` (contracts.md §1.4, §1.11).

### 2.4 References in the state

Refs are published through the enrollment's authority index chain
([contracts.md](contracts.md) §1.6), not inline. For removal kinds the
routing hint is: removal and re-add, their `folders` keys; ack, the union
of its items' folders. A member without `removal/1` in `features` is shown
as "runs an older WoW Sync that does not apply removals" (§9) and never
counts as confirmed (§5.6).

### 2.5 The ledger

[contracts.md](contracts.md) §1.7 and §1.12. Removal adds nothing to the
ledger format. `dismissed` holds the local "Continue without it" choices
(§4.4).

### 2.6 Validation: removal kind rules

Every entry first passes the shared rules V-BYTES, V-GROUP, V-ISSUER,
V-RET and V-REF ([contracts.md](contracts.md) §1.11). V-RET uses the
retirement barrier (contracts.md §1.10), not a per-retirement
`authority_known` list. The removal kinds add (V-KIND):

| # | Rule | Otherwise |
|---|---|---|
| V-REMOVAL-1 | **Listing.** `folders` (removal, re-add) is non-empty, at most 256, sorted by key, without duplicates, and every `name` validates to its `key` (`paths::validate_folder_name`). `audience` is sorted and unique. No rule compares package keys | `invalid(malformed_listing)` |
| V-REMOVAL-2 | **Ack shape.** `items` is non-empty, at most 256, sorted by removal hash, unique; each item's `folders` is non-empty, sorted, unique. `result: absent` has an empty `removed`; `result: removed` has a non-empty `removed` whose keys are all named in `items` | `invalid(malformed_ack)` |
| V-REMOVAL-3 | **Ack items match.** Each item's `removal` resolves to a removal record (V-REF) and every folder of the item is named by that removal | `invalid(malformed_ack)` |
| V-REMOVAL-4 | **Evidence.** Re-add `evidence` is `user_restore`, `user_add_back` or `rollback`; removal `evidence` is `observed`, `user_action`, `user_confirmed`, `folder_drop` or `rollback`. `rejection` is present iff evidence is `rollback`, and its rejection is valid and issued by the same issuer at the same `issued_at_seq`. `known` is well-formed (contracts.md §1.8). Any other evidence, including the withdrawn `observed_reinstall`, is invalid | `invalid(readd_evidence)` / `invalid(removal_evidence)` |

There is **no** `conflicted` status. Validity is re-derived as contracts.md
§1.11 says.

Because no removal record lists related records, a writer can always
produce a valid re-add or ack whatever packages or folder sets the related
records span, and however many there are (R4D-4, §13).

## 3. The merged removal (D-23)

### 3.1 Definitions

All sets are taken over **valid** ledger entries only. For folder key x:

- `Rem(x)` = removal records whose `folders` name x.
- `Bnd(x)` = the **re-add boundary** of x: the removals in `Rem(x)` that
  are covered ([contracts.md](contracts.md) §1.8) by the `known` of some
  valid re-add record whose `folders` name x.
- `Live(x)` = `Rem(x) − Bnd(x)`.
- x is **removed** iff `Live(x)` is non-empty.

The **merged removal** is `M = { x : Live(x) ≠ ∅ }`, with provenance
`Live(x)` per folder. For display, M is split into **removal units**: the
connected components of records in `⋃ Live(x)` linked by shared folder keys,
each shown with the titles of its records.

"Newest re-add boundary" is defined causally, never by a clock: a re-add
covers only removals that its issuer had already learned, so the boundary
of x is exactly the set of removals that some explicit re-add came after.
Re-adds never shrink it. A removal written after a re-add, or concurrently
with it (the re-add's issuer had not learned it), is not covered and wins
(D-21: any removal wins).

### 3.2 Properties

- **Deterministic and order-independent.** `Rem`, `Bnd` and `Live` are
  unions and differences of sets of immutable records. The ledger is a
  grow-only union of the same records on every computer, and validity is a
  function of the record, the ledger and the member/retirement records. So
  every computer with the same ledger computes the same M, whatever the
  order of arrival, duplicates, late delivery or restarts (R-06, A-20).
- **Merging is union.** Two removals of the same addon, removals with
  different folder sets (two computers that grouped the suite differently),
  and records that span packages all contribute their folders to M. No
  record has to name another, so none can be "inconsistent" with another.
- **Removal wins over installs.** No folder in M is ever installed or kept
  as a candidate anywhere (§4.1).
- **Monotonic until an explicit re-add.** Learning a removal can only add to
  M. Only a valid re-add can take folders out of M, and only for removals it
  supersedes.
- **Per-folder.** A re-add of a subset brings back only that subset. A
  removal of a subset of a package removes only that subset.

### 3.3 Worked examples

| # | Records (all valid) | M | Notes |
|---|---|---|---|
| 1 | A: r1 {core, data}; B: r2 {core, data} (independent uninstalls) | {core, data} | one merged removal; both issuers ack themselves; others ack once for both (§5.6) |
| 2 | A: r1 {core, data}; B: r2 {core, data, extra} (B grouped `extra` in) | {core, data, extra} | union of folder sets. C with {core, data, extra, options} removes three; `options` stays (Q-58) |
| 3 | r_P {p} and r_Q {q} written with otherwise identical content by one issuer at one seq (the run-003 R3D-13 counterexample) | {p, q} | two hashes, two records; both removals win. Nothing to resolve (§13) |
| 4 | r1 {core}; later C: a1 user_restore {core}, `known` covers r1 | {} | explicit re-add; propagates as an addition (§3.5) |
| 5 | r1 {core}; a1 covers r1; later B (after learning a1): r2 {core} | {core} | a removal after a re-add wins |
| 6 | r1 {core}; a1 covers r1; concurrently D (not knowing a1): r3 {core} | {core} | a concurrent removal wins; the re-adder sees "removed again by D" and may restore again (a2 covers r3) |
| 7 | r1 {core, data}; a1 {core} covers r1 | {data} | per-folder boundary |
| 8 | r1 {core}; C (holding an old copy, never acked r1) reinstalls nothing but its copy is present | {core} | not a re-add; C's copy is a stale copy and is removed into recovery (§3.4). Only an explicit GUI add-back re-adds (§1.7) |
| 9 | 1,024 concurrent removals of {core}; one restore | {} | the restore's `known` covers all of them in a record below 64 KiB (R4D-4, T-R4D4-1024-removals-readd-bound) |

### 3.4 Stale copies and the audience

A copy that existed before a removal and was never removed by it must not
come back through a later re-add. Otherwise the result would depend on
delivery order: a peer that learns the removal before the re-add removes its
copy, but a peer that learns both at once would keep it and could win as
"newest".

Obligations are per **`(enrollment, removal, folder)`** (R4D-2). Member m
**owes** removal r for folder x iff x ∈ `r.folders`, m's enrollment is in
`r.audience`, and the ledger holds neither a valid ack from m's enrollment
with an item `(r, folders ∋ x)` nor a valid re-add from m's enrollment
naming x whose `known` covers r. An ack for r that names only other folders
of r discharges nothing for x. Then:

- m's copy of x is **never a candidate** for any computer (contracts.md
  §4.1 level 5), even when x is no longer in M;
- on m itself, a live copy of x that m owes for is removed into recovery
  (stale copy), and m then acks that `(r, x)` item; afterwards the
  re-added version is an ordinary addition there.

**Discharge binds to state generation (R4D-3).** When m's obligation for
`(r, x)` is discharged by an ack item, m's copy of x becomes a candidate
again only from an admitted state of m with `seq ≥ ack.issuer.issued_at_seq`.
An older admitted state of m (for example seq 10 still listing an offline
v2, delivered before m's seq-11 state that follows the ack) stays
ineligible for x even after the ack arrives. Because the ack's
`issued_at_seq` is the first own state listing it, and that state is
published after the remove operation verified, the first eligible state
already shows the post-removal inventory. The same rule applies to a
discharge by m's own re-add: only states at or after the re-add's
`issued_at_seq`.

Example (R4D-2): r removes {core, data}; offline C in r's audience holds
newer copies of both; A re-adds only `core`. If C learns both at once, M is
{data}, but C still owes `(r, core)` and `(r, data)`. Level 4 removes
`data`, level 5 removes C's stale `core`, and C acks both items. If C
learns r first, it removes both in one operation and acks both items. In
both orders C ends with both old copies in recovery and A's re-added `core`
installed (T-R4D2-partial-readd-both-orders).

Example (R4D-3): C's admitted seq 10 lists offline v2 of x. C removes it and
writes an ack at seq 11. B receives the ack before C's seq-11 state, and a
re-add has emptied M. C's seq-10 state is still not a candidate source for
x, because 10 < 11 (T-R4D3-ack-before-state).

Example: C (in r1's audience) was offline with an updated v2 of `core`.
Whether C learns r1 first and a1 later, or both together, C ends with v2 in
recovery (named in the summary) and A's re-added version installed. The
result is the same either way.

A computer that joined after r was written is not in `r.audience` and owes
nothing for it: its local copy is an ordinary existing addon (A-04) once r is
superseded. While r is live, the copy is removed anyway (removal wins, Q-48).

A member in the audience that never had x writes an `absent` ack item
`(r, x)` at step 3b **before** it may install x (§5.6), so it never owes
for a folder it holds.

### 3.5 Re-add rules

- A re-add is **explicit**: the GUI restore (`user_restore`, §7.4) or "Add
  back to all computers" (`user_add_back`, §1.7, §7.4). A reappearance
  alone never re-adds (Q-55 PENDING OWNER).
- For each of its folders, the re-add supersedes exactly the removals
  naming that folder that its `known` covers. Folders it does not name stay
  removed (example 7).
- After the re-add, x is not in M, and the re-added folders propagate as an
  ordinary addition among non-owing holders ([domain.md](domain.md) §6).
  Later genuine updates by non-owing holders propagate normally.
- Uninstalling again writes a new removal, which no earlier re-add covers
  (example 5).
- An offline peer cannot re-add by being present: without its own ack items
  its presence is a stale copy (§3.4).

## 4. Target per package

### 4.1 Precedence

Removal is evaluated in `core::target` at cycle step 5, after authority
learning and admission. Inputs: the merged removal M with provenance, the
owing relation, the local holds and observations, the pending hints, the
local journals, group rejections (rollback.md) and every admitted candidate.

Before grouping ([domain.md](domain.md) §5) and candidate selection (§6):

- every folder in M is removed from every observation, including the local
  one, so group-wide membership and completeness exclude it (§1.5);
- every folder a member owes for is removed from that member's observation,
  and every folder whose obligation was discharged is removed from that
  member's admitted states older than the discharging record (§3.4, R4D-3).

The **remove** decision is made on the unstripped local grouping, so the
removed folders of one local package form one operation.

Per local package P, the levels of the shared precedence
([contracts.md](contracts.md) §4.1) apply, first match wins. Removal owns
levels 3 (its holds), 4, 5 and 7:

1. **Unresolved journal** (contracts level 1).
2. **Rejections** (rollback.md, contracts level 2) exclude every source
   whose complete content is a rejected release (contracts.md §4.2
   `same_release`) from every level below.
3. **Local hold:** `Held(RemovalObserved | RemovalUnclear |
   ReinstallObserved)` for P. Nothing for P is applied here.
4. **Merged removal:** some live local folder of P is in M →
   `Remove { folders: live(P) ∩ M, removals: ⋃ Live(x), reason: Removed }`.
5. **Stale copy:** some live local folder x of P is not in M but this
   computer owes `(r, x)` for some r (§3.4) →
   `Remove { folders, removals, reason: StaleCopy }`.
6. **Constraints** (code choice, rollback recovered target; contracts.md
   §4.4, §5) for the remaining folders.
7. **Pending removal information:** a folder of P that would be installed or
   replaced is in the hint of a pending removal that is not dismissed →
   `Unavailable(WaitingForRemovalInfo)` for P on this computer (§4.4).
   Removals at levels 4–5 are never deferred by this level.
8. **Ordinary** [domain.md](domain.md) §6 selection over the stripped
   observations, whole-addon, projected per folder only after selection
   (contracts.md §4.3).

When levels 4 or 5 match, folders of P outside M stay and are evaluated by
levels 6–8 in the next cycle as their own (stripped) package.

### 4.2 Results

| Situation (local package P) | Result | Effect |
|---|---|---|
| Observation holding or unclear here | `Held(RemovalObserved \| RemovalUnclear)` here only | nothing for P applied here (§1.6) |
| Reinstall observed after own ack | `Held(ReinstallObserved)` here only | not removed; offers Add back to all computers / Remove again (§1.7) |
| Live local folders of P in M | `Remove { reason: Removed }` | removed into recovery when safe (§5); `removed` ack items |
| P not held here, some folders of P in M | nothing to remove | `absent` ack items (§5.6) |
| Local copy owed after the removal was superseded | `Remove { reason: StaleCopy }` | removed into recovery; ack items; then ordinary addition |
| Install/replace blocked by a pending removal hint | `Unavailable(WaitingForRemovalInfo)` here only | wait, or "Continue without it" (§4.4) |
| Folders of P remain after removal and hard-depend on a removed folder | ordinary result plus `DependencyRemoved` warning | named in the summary (§5.4, Q-54) |
| None of the above | ordinary [domain.md](domain.md) §6 | — |

No removal situation produces `NeedsDecision`. Removal adds no
`DecisionReason`; the code-choice reasons are contracts.md §5.

### 4.3 What happened to the run-003 removal decisions

| Run-003 result | Run-004 behaviour |
|---|---|
| `RemovalVsUpdate` | removal wins; the update goes to recovery and is named (D-21) |
| `CompetingRemovalDecisions` | no decisions exist; removal versus re-add is settled by `known` coverage (§3.1) |
| `InconsistentRemovalRecord` | cannot occur: identity is the content hash (contracts.md §1.3, §13) |
| `NewVersionAfterDecision`, `OnlyDeclinedVersions`, selection constraint | no version choice in removal; keeping a copy is the code-choice family (contracts.md §5) and rejecting a release is rollback.md |
| `RemovalScopeDiffers` | named folders are removed; other folders stay (Q-58) |
| `RemovalBreaksDependency` | removal proceeds with a warning (Q-54) |
| `DroppedFolderDiffers` | the dropped folder goes to recovery whatever its tree (§1.5) |
| `Unavailable(AuthorityPending)` for the whole package | install deferral for hinted folders only, dismissible (§4.4) |

### 4.4 Pending information

A ref whose record cannot be read, a record with an unknown schema major, or
a record with a missing reference is **not known** for the merge. Acting on
less knowledge is safe here: an unknown removal can only arrive later and
remove a copy into recovery; an unknown re-add or ack can only delay an
install. To avoid needless churn and to honour R-06 ("uncertainty is not
evidence that nothing happened"), a **pending removal** defers installs and
replacements of the folders in its hint on this computer (level 7). It never
defers removals and never affects other folders.

It resolves by itself when a good copy is learned (any member's mirror) or
when the referencing member repairs its copy (contracts.md §1.12). If it
does not,
the GUI shows "Waiting for removal information from *Laptop* about *X*"
with **Continue without it** (§9). That adds the hash to the local
`dismissed` set: the record is then treated as unknown, exactly like a
record not published yet. If it becomes readable later, it is learned and
counts normally. "Continue without it" is local, never published, and
needs no terminal (Q-57 PENDING OWNER). The confirmation explains the risk:
"If this information is a removal, *X* may be installed here now and
removed again later. The removed copy is kept for recovery."

## 5. Apply on receivers (R-11)

### 5.1 Plan

`plan_apply` gains a remove operation: `PackageApply` with `kind:
Remove`, `removals: Vec<Sha256>`, `reason`, folders with `FolderAction::Remove
{ expected_live: Tree{tree, fingerprint} }`, and `settings_ops:
Vec<SettingsFileOp>` (§6). Its journal is `wowsync.apply-journal/2` and its
recovery point `wowsync.recovery-point/2` ([contracts.md](contracts.md)
§2), so a downgraded slice app stops at `NeedsHelp` instead of misreading
it. It is planned only when:

- the target is `Remove` for P;
- every live folder to remove is stable and readable (`ok`; a non-`ok`
  folder waits and is shown);
- no journal exists (Q-25, unchanged; R3D-6: the GUI never promises a
  removal while a journal blocks);
- `Activity::NotRunning` (R-11);
- the settings plan (§6) is derivable, or settings are not applicable.

When the same cycle also plans an update of P's remaining folders (for
example a folder drop, §1.5), the planner merges both into one operation
with `kind: apply`, whose `/2` journal mixes `replace`, `install` and
`remove` folders and lists the drop's removal in `removals`
([contracts.md](contracts.md) §2.2). Each folder follows its own slot
table (apply.md for install/replace, §5.3 for remove), so the one-op crash
argument holds per folder.

### 5.2 Algorithm

One package per operation, as in apply.md §2.2. There is no staging for
code. Live folders move to recovery by same-volume rename. Nothing is
deleted.

1. **Journal `staging`** (`/2`, `kind: remove`, every folder `action:
   remove`, `target_tree` = the tree being removed, `settings_ops` from §6).
   The ack is encoded now (`result: removed`, one item per `(removal,
   folders)` to remove, the `removed` manifest, the settings result the
   plan will produce, `issued_at_seq` = the reserved next seq) and its
   bytes and hash are stored in `pending_records`. They never change
   afterwards (contracts.md §2.2).
2. **Destination revalidation** (apply.md §2.2 step 2). Any difference →
   `Replanned(DestinationChanged)`; journal → `rolled_back`; nothing touched.
3. **Recovery point** (`/2`, `kind: remove`), `pinned: true`, `removals`,
   the folders as `replaced`, `installed: []`, `settings` roots and
   `paired` (§6) → **journal `committing`**.
4. **Commit**, per folder in name order: `G` (WoW gate); fingerprint
   unchanged; `move(L→R)`. Then the settings steps (§6), each after `G`.
5. **Journal `committed`.**
6. **Verify:** `G`; every folder `Done` (`L` absent, `content(R)` equals the
   expected tree). Settings verified (§6). → **journal `verified`**, else
   `needs_help`.
7. **Finalize** (each step "skip if done"):
   a. (nothing to encode: the ack bytes were fixed in step 1);
   b. write the reserved ack bytes from `pending_records` (write-if-absent,
      idempotent) and its ledger entry;
   c. recovery point `pinned: false` (an ordinary history state, §7);
      prune per §7;
   d. record the outcome locally (the pending report keeps only slice
      `ReportKind` values; the removal outcome is published by the ack,
      contracts.md §2.4);
   e. `remove_journal`.

### 5.3 Slot configurations and crash idempotency (extends apply.md §3–§4.1)

Remove (`O` is the expected tree):

| Config | `L` | `R` | Meaning |
|---|---|---|---|
| `Untouched` | present | absent | not moved |
| `Done` | absent | present | moved to recovery |
| `Unknown` | present | present | something recreated `L` after the move |
| `Unknown` | absent | absent | folder vanished externally |

Rollback (phase `committing`), reverse name order:

| Config | Action |
|---|---|
| `Untouched` | none |
| `Done` | `G`; `move(R→L)`; check `content(L) == O`, else `needs_help(RestoreMismatch)` |
| `Unknown` | journal → `needs_help(UnknownInterruptedState)`; nothing moved |

Write order with crash points `†`:

```text
J:staging(remove, ack bytes) † [revalidate] † RP(remove, pinned) † J:committing
† f1: L→R † f2: L→R † … † [settings steps] † J:committed † [verify]
† J:verified † ack record † ledger † RP(unpinned) † prune
† local outcome † remove J
```

| Crash after | Observed on restart | Recovery |
|---|---|---|
| `J:staging` up to the RP write | phase `staging`, all `Untouched` | `rolled_back(from staging)`; the reserved ack bytes are discarded, never written; cleanup removes the record-only recovery dir; re-planned next cycle |
| `J:committing`, any rename boundary | each folder `Untouched` or `Done` | `G`, then rollback moves each `Done` back. Each step is chosen from observation, so a second crash during rollback never repeats a move. Retried next cycle |
| a settings step | per the settings slot table (§6) | settings rollback first, then code rollback |
| `J:committed` | all `Done` | verify only → `verified` or `needs_help` |
| `J:verified`, any finalize step | — | finalize steps are idempotent; the reserved ack bytes are in the journal since step 1; journal removal is last. Nothing of this computer is published while the journal exists (contracts.md §2.2) |
| `needs_help` | — | nothing touched; recovery point stays pinned |

WoW starting mid-commit behaves as apply.md §2.3 (Q-4): stop with no further
game-file change, keep the journal, restore after WoW exits. Status: "WoW
started while *X* was being removed; it will be put back after WoW exits,
and the removal will be retried."

### 5.4 Dependencies

- **Removing a dependency:** before planning, core computes remaining local
  packages whose selected toc `hard_deps` name a removed folder. The removal
  proceeds (D-21: a sync tool, not addon police); the result and the summary
  carry `DependencyRemoved { dependents }`: "*Y* needs *X*, which was
  removed from all computers" (Q-54).
- **Removing a dependent** never removes its dependencies. Removal is never
  inferred transitively.
- Folders joined by `Dependency`, `NamePrefix` or `Family` edges are one
  package already ([domain.md](domain.md) §5), so suite parts usually leave
  together.
- Optional dependencies are not read and have no effect.

### 5.5 WoW and other writers

The WoW gate `G` precedes every live rename, including rollback and restore
moves (R-11, A-21). An idle updater does not block. An updater writing into
P's folder after planning causes a fingerprint mismatch: `Replanned` before
commit, or a stop in commit. A recreated folder found after the move is
`Unknown` → `needs_help`; it is never deleted. No process is ever stopped.

### 5.6 Acknowledgments, confirmation and settling

- **`removed` ack items:** written in finalize (§5.2 step 7) for every live
  and owed `(removal, folder)` pair whose folder it removed.
- **Due items.** `(r, x)` is **due** here iff r is valid, r's audience
  includes this enrollment, x is not present here, and this enrollment has
  neither acked `(r, x)` nor re-added x with `known` covering r. This
  includes the **origin** of a passive uninstall or folder drop.
- **Classification (step 3a, R4D-5).** Every due `(r, x)` is classified
  before any ack is written:
  - `cleanup_required` iff `owned_scope` (§6.1) of x here, from the
    baseline declarations or the removal's manifest `.toc`, has at least
    one owned, unpaused item;
  - otherwise `no_cleanup(reason)` with reason `nothing_owned`
    (`settings: not_applicable`), `all_paused` (`kept_paused`) or
    `owner_unknown` (`code_only`, with the diagnosis).
- **`absent` ack items at step 3b** are written **only** for
  `no_cleanup` items, in one batched ack per cycle (split at 256 items),
  before any install of x is planned. This is the no-op path: no journal
  and no recovery point (nothing is taken, contracts.md §2.3).
- **`cleanup_required` items** get no ack at 3b. Step 6 plans a
  `settings_cleanup` operation for them (§6.3), whose journal reserves the
  `absent` ack bytes in `pending_records`. The ack is written only by its
  finalize, after the settings moved and verified. While WoW runs, the
  operation is not planned and the items stay unacked and shown as
  "waiting for WoW to close" (R-11).
- **Origin.** The same rule applies to the origin: its own ack for a
  passive uninstall or folder drop is an `absent` ack, written at 3b only
  if nothing needs cleanup, otherwise by the cleanup finalize. Its
  baseline entry is cleared after that ack (§1.6).
- **Confirmation** of a removal on peer X: for **every** folder of the
  removal, this computer's ledger has a valid ack item from X's current
  enrollment for that `(removal, folder)`, **and** X's admitted state at or
  after the ack's `issued_at_seq` lists none of its live folders, **and** X
  advertises `removal/1`. Publication of a removal is not confirmation
  (A-24).
- **Settled** (derived, display only): every current, non-retired
  enrollment in the audience has acked or re-added every folder of the
  removal. No record is rewritten when a removal settles. A settled removal
  stays authority: a late or re-enrolled member's copy is still removed
  (§8).

## 6. Settings for removed addons

SPEC R-04 and A-10: an uninstall removes the active code **and** its
applicable saved settings; recovery keeps and restores both. The settings
design is [settings.md](settings.md); the shared units and journal ops
are [contracts.md](contracts.md) §2–§3.
Removal relies on the contract below; the settings design provides it.

**Q-45 (D-21):** WoW keeps an uninstalled addon's saved variables. So WoW
Sync moves the owned saved-settings items itself, and WoW does not recreate
them while the addon is not installed.

### 6.1 Contract (core, pure; adapters follow apply.md's port-trait pattern)

| Operation | Contract |
|---|---|
| `settings::owned_scope(variant, declarations, identities) -> SettingsScope` | Pure. For the removed folders, the saved-settings items owned by them as `SettingsUnit`s (folder key plus identity, [contracts.md](contracts.md) §3.2), plus `unknown_or_shared: Vec<Diagnosis>`. Ownership comes from the folders' own `SavedVariables` declarations: from the live `.toc` when the folder is present, otherwise from the presence-baseline entry (§1.6), parsed without executing Lua (R-11). A missing code folder never skips applicable settings (R4D-5). WoW's own UI/cloud files are never items (R-07) |
| `settings::checkpoint(scope) -> CheckpointRef` | Content-addressed copy of the owned items **before** removal (contracts.md §3.4), scoped by `(GroupId, product, unit)`. Retained with its recovery point |
| `SettingsFileOp` list | Per item: `expected_live` (hash + fingerprint), same-volume move to `<variant>\.wowsync\recovery\…\settings\…`, `G` before each move ([contracts.md](contracts.md) §2.2). Never an in-place edit; never a split of a shared file. Slot configurations `Untouched / Done / Unknown` as in §5.3 |
| `settings::restore(checkpoint, units)` | Moves items back only for the **same** unit they came from (R-02, R-10); verifies hashes |
| `settings::is_removed(folder)` | The settings sync never publishes, pushes or applies settings for a folder in M. A settings master holding them moves them to recovery like every other member |

### 6.2 Rules

1. **Owned only.** Unknown or shared ownership: the code is removed and
   those items stay untouched; ack `settings: code_only` with a diagnosis
   (R-07, Q-46).
2. **Paused settings** (R-08): the code is removed; paused items stay in
   place; ack `settings: kept_paused`; shown "settings kept (paused)" (Q-47).
3. **Pairing.** A `/2` remove recovery point records `paired` only if
   every owned item was checkpointed and verified, `partial` if some were
   not (named), otherwise `code_only` with the reason
   ([contracts.md](contracts.md) §2.3).
4. **Retention.** A settings checkpoint referenced by a retained recovery
   point is never pruned before that point (R-10).
5. **No merge.** Restoring settings never merges with other copies (R-09).
   After a restore, the restored settings are this computer's settings, and
   the settings design's master/push rules apply from there.
6. **Identity boundaries.** Character items follow matching identities only
   (R-02, A-03). An identity absent here means nothing to remove for it.
7. **WoW gate.** Settings moves are gated by `G` like code.
8. **Master role.** Removal is a collection act from any member. It does not
   need the settings master.

**Integration fallback.** If removal is integrated before the settings
contract exists, the removal record carries `settings: {kind: none}`, acks
carry `settings: not_applicable`, recovery points `paired:
not_applicable`, and the GUI says "Settings for this addon were not
changed by WoW Sync." It never claims a paired removal or restore (R-10,
A-19). A-10 is not complete until the settings half passes.

### 6.3 Settings cleanup on the observing computer (R4D-5)

A passive uninstall (§1.2) or a folder drop (§1.5) leaves no live code
folder on the observing computer, so no remove operation exists there to
carry the settings moves. The observing computer therefore runs its own
**settings-cleanup** operation:

1. When the observation matures, the removal record is written and its
   ledger entry stored. The presence-baseline entry is **kept**: it holds
   the settings declarations (§1.6).
2. Step 3a classifies the origin's due items (§5.6). `no_cleanup` items
   get their `absent` ack at step 3b with `settings: not_applicable`,
   `kept_paused` or `code_only`; go to step 5.
3. Otherwise step 6 plans a `wowsync.apply-journal/2` with `kind:
   settings_cleanup`, empty `folders`, the `settings_ops` of every owned
   item and the `absent` ack bytes in `pending_records` (fixed at journal
   creation), and a `/2` recovery point with `paired: settings_only`
   (contracts.md §2.3). Crash recovery and the WoW gate are the settings
   slot table's (settings.md §5.2). While WoW runs nothing is planned.
4. After verify, finalize writes the reserved ack (`settings: paired`, or
   `partial` with the uncaptured units named).
5. Only then is the baseline entry cleared (§1.6).

A crash before the ack restarts from the journal; no ack for a
`cleanup_required` item is ever written before its settings moved
(T-R4D5-absent-ack-waits-cleanup). Unowned, shared and paused items stay
in place with their notice (§6.2). The GUI "Remove from all computers"
(§1.4) moves code and settings in one remove operation and needs no
separate cleanup.

**Receivers** that never held the code but hold owned settings of a
removed folder (for example a character's saved variables on a computer
where the addon was already removed by hand) follow the same §5.6
classification. Their ownership evidence is the `.toc` named by the
removal record's `folders` manifest, read from the provider tree and blob
and parsed without executing it. If that is unreadable, the item is
`no_cleanup(owner_unknown)`: the settings stay in place and the ack says
`code_only` with the diagnosis.

## 7. History and recovery (R-10, A-16, A-17)

### 7.1 Three states with removal

History is per package, separately for code and settings (R-10). A removal
is a history state whose content is "not installed":

| Position | Code | Settings |
|---|---|---|
| current | removed (nothing live) | removed (or unchanged with `code_only`) |
| previous 1 | the removed version (recovery point `/2`, `kind: remove`) | checkpoint taken at removal |
| previous 2 | the version before that, if retained | its checkpoint |

Pruning in finalize (apply.md §2.4) counts states, not recovery points. A
`removed` state holds no bytes but occupies a position. Re-adding P makes
`removed` a previous state.

### 7.2 Temporary exceptions (A-17)

Kept beyond three, each shown as "kept for recovery: N extra copies
(reason)", and returned to three when the reason clears:

| Reason | Kept until |
|---|---|
| removal operation unresolved (journal in any phase, `needs_help`) | journal cleared or resolved in the GUI |
| restore in progress (§7.4) | restore verified |
| settings checkpoint referenced by a retained code point | that point pruned |

Low storage while an exception exists → `NeedsHelp(StorageLow)` with the
consumption shown. Exceptions are never pruned to make room (R-10). The
run-003 exceptions for frozen decision alternatives no longer exist.

### 7.3 Provider payload retention (R3D-15, Q-53)

Record **metadata** is kept while the group exists (Q-44); it holds tree
hashes and names, no addon or settings content. Metadata never roots
payload by itself. Provider **payload** (tree records, blobs and
provider-held settings checkpoints) is a retention root for removal only
while it backs P's **current** state:

| Payload root | Kept while | Visible as |
|---|---|---|
| `folders` manifest and settings roots of a removal in `Live(x)` for some x | that removal is live (the removal is the current state, Q-53) | history state "Removed — restorable on any computer", with its size in storage use |
| a restore in progress (§7.4) | the restore is verified | temporary exception |

When a removal leaves `Live` (superseded by a re-add), its payload stops
being a root at once. Earlier states are then restorable only from local
recovery points (current plus two previous). Ack manifests (another
computer's removed copy) are never provider roots; that copy is restorable
on the computer that holds it. The provider has no garbage collector yet
(Q-7); these rules bind the collector when it is designed. A restore that
finds payload already released is `Unavailable` with the reason, never
partial.

### 7.4 Restore ("undo removal", explicit re-add)

From the package's history or the Removed list (§9), on this computer:

1. **Source.** The local recovery point of kind `remove`. If there is none
   (this computer never had P, or its points were pruned) **and** a live
   removal names the folders, the source is that record's `folders` manifest:
   each tree record from the provider, then its blobs, staged and verified
   like an apply (apply.md §2.2); the staged package hash must equal
   `removed_hash`. Unreadable tree or blob → `Unavailable` with the reason.
   A recovery point of an owed or updated copy (§3.4) is a source only on
   the computer that holds it.
2. **Confirm.** The GUI states before confirming: "This reinstalls *X* on
   all computers in this group" (R-04: one collection; a per-computer
   exception is not a product feature).
3. **Operation** `kind: restore` (`apply-journal/2`): install or replace
   with journal, `G` and verification as apply.md §2.2. The live state
   before restore (normally absent) becomes a `/2` recovery point of kind
   `restore`.
4. **Settings:** the paired checkpoint is restored for matching units
   (contracts.md §3.2) if present. Otherwise code-only restore with the
   warning "Settings were not restored: no saved copy from before the
   removal" (R-10, A-19). Partial and paired restores are reported
   distinctly.
5. **Re-add:** after verification, write a `user_restore` re-add record
   whose `known` is this computer's current causal summary (it therefore
   covers every removal of the restored folders this computer has learned),
   then publish. The folders propagate as an addition (§3.5).
6. A group-rejected version (rollback.md) is not offered without first
   lifting the rejection, which is a separate explicit choice.

*As built (run-005 R5-remove).* `Restore` on a computer that no longer has
the folders validates the plan at once (an unreadable file list or blob, a
rejected version or a hash mismatch is refused with the reason and nothing is
queued) and queues a local request. The next cycle that may change game files
(Active, WoW not running) plans one `restore` operation (core `plan_restore`,
runtime `restore_flow`). The source is read per folder: the provider tree
record, else a local recovery point holding exactly that tree (any kind, not
only `remove`); blobs likewise fall back to that point. The `user_restore`
re-add is a pending record of the operation, so it is written only after
verification. Settings come back in the following cycle through the capture
book (the removal no longer stands); with no captured copy the cycle's note is
the R-10 warning. The result is queued ("Recorded"), not "Written". A computer
that still holds the folders only writes the re-add, as before.

"Keep it" for a held local observation (§1.6) does not write a re-add (no
removal was published). **Add back to all computers** for a held
reappearance (§1.7), or on a computer that still holds a copy while a
removal is pending there (for example WoW is running), writes a
`user_add_back` re-add without moving files: the live copy is the re-added
version.

## 8. Rollback, retirement and joining

### 8.1 Rollback and rejection (R-10, A-18)

Owned by [rollback.md](rollback.md). Removal requires:

- A removal is not a release and cannot be rejected. Undoing it is a
  restore (§7.4).
- Rejection records and removal records are independent grow-only sets.
  Neither supersedes the other. A rejected version stays excluded after a
  re-add.
- Rollback of a currently removed package is a restore, offering only
  non-rejected states.
- Both are kinds of the one authority family
  ([contracts.md](contracts.md) §1.1), with the same identity, mirroring
  and validation; precedence is contracts.md §4.1.

### 8.2 Retirement (R-03, A-45)

Owned by [ops.md](ops.md) §2; the barrier is
[contracts.md](contracts.md) §1.10. Removal requires:

- Records issued by a computer later retired **stay valid** when
  `issued_at_seq ≤ Cutoff(E)`, the **minimum** cutoff over the effective
  retirements of E. Others have mirrored them. Retirement does not
  un-delete anything. There is no `authority_known` list.
- A record above the cutoff is `invalid(after_retirement)`. A later
  retirement can only lower the cutoff, so a record never becomes valid
  again (R4D-16). If it was acted on before the retirement arrived, it
  leaves the merge. Members that removed P keep their recovery points; P
  then follows ordinary rules, including stale-copy rules computed over
  valid removals only (Q-52).
- Rollback-evidence removals and re-adds (§2.3) are ordinary records for
  this purpose.
- Retirement is monotone: a retired enrollment never becomes active again
  without re-enrollment, whatever arrives later (R4D-11).
- Settling waits only for current, non-retired members. A retired member's
  states are not candidates, so its copy can neither keep nor bring back a
  removed addon.
- A re-enrolled computer is a new enrollment: it is not in older audiences,
  and its old acks do not carry over. Its old records stay valid through
  historical membership (contracts.md §1.10).

### 8.3 Joining with removed addons (R-03, A-04)

- A joiner's local copy of a folder in M is listed in the join consent:
  "These addons were removed by the group and will be removed here (kept for
  recovery)". After consent it is removed like any receiver (Q-48). There is
  no decision.
- A joiner's copy of a folder whose removals are all superseded is an
  ordinary existing addon (the joiner is not in those audiences, §3.4).
- A joiner's locally **absent** group addons are installed, never treated as
  removals (E1 fails).

## 9. GUI touchpoints

Screen design is owned by [ui/screens.md](ui/screens.md) (authoritative,
R3D-12); this lists what removal needs and which screen shows it. Core
provides every field as a typed status or explanation (A-25, A-26). No
record hashes are shown outside an expandable details area. Wording rule:
"removed" means removed **here**; "Confirmed removed on *Desktop*" needs that
member's ack (§5.6); the UI never says "removed everywhere" before every
current member confirmed (A-24).

| Screen (ui/screens.md) | Removal content | Actions |
|---|---|---|
| Overview | headline per the existing rules: a removal in progress is `Working`, one waiting for WoW is `Waiting`, an unclear observation or a pending-information wait counts as an attention item | open Needs attention |
| Computers | per computer: removal confirmations outstanding; "runs an older WoW Sync that does not apply removals" when `removal/1` is missing | — |
| Add-ons | a **Removed** section or filter: each removal unit with title, origin computer, evidence kind, "removed here at" (local clock), per-computer state (removed / never had it / pending / not heard from / older version) | open detail; Restore |
| Add-on detail ("Why this version") | provenance: which computer removed it and how (uninstalled, removed in WoW Sync, confirmed, dropped by an update); other computers' copies kept in recovery (from ack manifests), e.g. "Laptop's copy (v2.1) was kept in recovery on Laptop"; dependency warning; history with three states including "removed", paired or code-only | **Remove from all computers** (consequence text: removed on all computers, kept for recovery, settings effect); **Restore** (§7.4); **Add back to the group** (§7.4) |
| Needs attention | "Was *X* removed, or renamed to *Y*?" (E6); "*X* was installed again on this computer, but it was removed from all computers" (§1.7); "Waiting for removal information from *Laptop* about *X*" (§4.4); interrupted removal or settings cleanup | Confirm removal / Keep it; Add back to all computers / Remove again; Continue without it; open needs-help detail |
| Decision | no removal decisions exist; nothing added | — |
| Needs-help detail | interrupted remove or restore journal, `Unknown` slot | the existing journal resolution flow |
| Activity | grouped events: "3 addons removed on this computer (from *Laptop*)"; "*X* restored and reinstalled on all computers"; "*Laptop*'s updated copy of *X* was kept in recovery" | open detail |
| Settings sync | settings of removed addons shown as removed (or kept: paused, unknown ownership) | — |
| Tray and OS notifications (R4-notify-diag) | grouped removal notification (R-13, A-27) | open Activity |

Every action works with the keyboard and without a terminal (A-26).

## 10. Implementation map

Core stays pure; one implementation is shared by runtime and tests (A-43).
File ownership and build order are [contracts.md](contracts.md) §9 only;
this table says which packet builds each removal part.

**Built by step 1, R4-core-contracts** (removal defines, the foundation
implements): the removal, re-add and ack DTOs with V-REMOVAL-1…4
(`kinds.rs`); `ApplyJournal/2` and `RecoveryPoint/2` (`records.rs`);
`FolderAction::Remove`, `PackageApply { kind, removals, settings_ops,
pending_records }` (`plan.rs`); the remove slot table, mixed journals,
`pending_records` reservation and finalize, `recover_interrupted` for
`remove`, `apply/2` and `settings_cleanup` (`exec.rs`); `PackageTarget::
{Remove, Held}`, `RemoveReason`, `HoldReason` and the compose levels 3–5
and 7 calling `RemovalHooks` (`target.rs`); every removal `PackageStatus`
and `Explanation` variant (`status.rs`, `explain.rs`); authority layout
and IO (`layout.rs`, adapters `provider.rs`, `authority_store.rs`); the
runtime cycle skeleton with steps 2, 2a, 3a, 3b, 6 and 7 calling the hooks
(`service.rs`) and the removal control commands (`control.rs`).

**Built by step 2, R4-core-removal** (only these files):

| File | Contents |
|---|---|
| core `removal.rs`, `removal_tests.rs` | `RemovalHooks` implementation; `MergedRemoval { live, units }`; `merge_removals`; `owes` per `(enrollment, removal, folder)`; `eligible_from` (§3.4); `PresenceBaseline`, `Snapshot`, `BaselineEntry` with settings declarations (`wowsync.presence-baseline/1`); `RemovalObservation` (`wowsync.removal-observation/1`); `observe(...) -> ObserveOutcome`; `classify_due(...) -> Vec<(AckItem, CleanupClass)>` (§5.6); `ack_items_due`; `plan_removals` including update-with-drop merging (§5.1) and `settings_cleanup` ops (§6.3, via `SettingsHooks::roots_for`); `readd_for_restore`; `pending_hints`; T1–T32 and the T-R4D2/3/4/5 core cases |
| runtime `removal_cycle.rs` | the removal hook bodies the cycle skeleton calls, and `remove_from_group`, `restore`, `add_back`, `remove_again`, `confirm_unclear`, `keep_unclear`, `continue_without(hash)`; directory identity for E3; T-R4D5-absent-ack-waits-cleanup |
| runtime `detail.rs` | rows for the Removed section and removal provenance |

GUI work (§9) is R4-app's. Settings functions (§6.1) are R4-core-settings'
`SettingsHooks`; removal calls them through the hook.

## 11. On-disk formats and compatibility

| Record | Change | Older app reading it | This app reading older data |
|---|---|---|---|
| `wowsync.code-state/1` | additive `authority_index`, `features` (`serde(default)`, omitted when empty); `apply_report` keeps slice `ReportKind` values only | unknown fields ignored (no `deny_unknown_fields`); it never removes anything (slice rule) and is shown to others as "older version" (§2.4) | missing fields = no refs, no `removal/1`: absence stays absence, never removal |
| `wowsync.authority-index/1` | new provider chunks ([contracts.md](contracts.md) §1.6) | not read | — |
| `wowsync.removal/1`, `wowsync.readd/1`, `wowsync.removal-ack/1` | new provider records under `authority\` | not read | unknown fields ignored; unknown major → `pending(unknown_schema)`, shown "written by a newer version" |
| `wowsync.authority-ledger-segment/1`, `wowsync.presence-baseline/1`, `wowsync.removal-observation/1` | new local records; limits 4 MiB per segment (contracts.md §1.7), 1 MiB (never dropping a referenced snapshot; overflow → `NeedsHelp(StorageLow)`), 64 KiB | — | absent = empty |
| `wowsync.apply-journal` | remove, update-with-drop, restore, rollback and settings ops are `/2` ([contracts.md](contracts.md) §2.2); pure code applies stay `/1`, byte-identical | `/2` fails to decode: the slice keeps it, blocks applies and publication and changes nothing (its GUI journal list omits it, contracts.md §2.4) | `/1` = `kind: apply` |
| `wowsync.recovery-point` | remove, restore and settings points are `/2` (§2.3) | `/2` skipped by `decode_points`, so never pruned by the slice | `/1` = code-only `apply` point |
| `wowsync.pending-report` | `/2` only if a new kind is needed (contracts.md §2.4) | replaced, not misread | — |

Q-56 records the choice of additive fields in `code-state/1`. Journals and
recovery points with new content take a new major so the slice refuses
them safely; the `record!` change that permits this is contracts.md §2.1.

## 12. Tests

Every case is behavioural and must fail for a plausible broken
implementation (A-38). Core cases use `core::testkit` (tier `offline`).
Runtime cases use synthetic installs under a temp directory and three
replicas with injected delivery order through the temp provider (tier
`offline`). Native rename/in-use cases are tier `native`. Paths in tests are
built with `Path::join`. The implementing packet lists required cases in
`tests/required-cases.toml`.

| # | Case | Expected | Acceptance |
|---|---|---|---|
| T1 | Two members independently uninstall P | one merged removal; one remove op on each holder; each issuer's own ack plus others' acks; no decision | A-10, A-20 |
| T2 | r1 {core, data} and r2 {core, data, extra}; receiver holds {core, data, extra, options} where `options` hard-depends on `core` | core, data, extra removed in one op; `options` stays; `DependencyRemoved` warning names it | A-09, A-10, A-20 |
| T3 | **R3D-13 replay:** two records identical except folders {p} and {q}, same issuer and seq; then restart and replay of both from two other members | two valid records; p and q removed; no decision or blocked state exists; replay changes nothing | A-20, A-23 |
| T4 | Property test over permutations on 3 replicas: removals, re-adds, acks, refs before records, records without refs, duplicates, a bad copy, restarts between deliveries | identical final M, inventories and ack sets for every order | A-20 |
| T5 | Offline C updates P to v2; A uninstalls P; C reconnects | v2 removed into recovery on C; C's ack names v2; summary on every computer names "C's copy (v2) kept in recovery"; v2 never a candidate; restorable through the GUI on C | A-10, A-20, A-25 |
| T6 | Explicit restore on C supersedes r1; concurrently D writes r3 unaware of it | after a1, P propagates; after r3 arrives, P removed again everywhere ("removed again by D"); a second restore supersedes r3 | A-10, A-20 |
| T7 | P reappears on C after C's own ack (updater reinstall, manual copy), with and without a restart, and C never chooses | `Held(ReinstallObserved)` indefinitely; no re-add is written and nothing propagates; "Add back to all computers" writes `user_add_back` and propagates; "Remove again" removes into recovery (Q-55 PENDING OWNER) | A-10, A-20 |
| T8 | C in r1's audience holds v2, unacked; learns r1 and a1 (a) in one cycle, (b) in two cycles, (c) a1 first | all orders end with v2 in C's recovery and A's re-added version installed | A-20 |
| T9 | Joiner with a copy: (a) removal live; (b) removal superseded before the join | (a) listed in consent, removed after consent; (b) ordinary existing addon, newest wins | A-04 |
| T10 | Ref to an unreadable removal on one replica | installs of hinted folders deferred; other packages and removals proceed; record mirrored elsewhere → learned and applied; own copy deleted → repaired next publish; "Continue without it" → acts as unknown; record readable later → applied | A-20, A-26 |
| T11 | Ref whose `kind` or `folders` differ from the record; a file whose bytes do not hash to its name, with a good copy elsewhere | mismatched ref ignored with a diagnostic, record stands; bad copy ignored, good copy used; never a conflict | A-20, A-23 |
| T12 | Malformed records: unsorted or duplicate folders, a name that fails `validate_folder_name` (traversal, separators, non-ASCII), oversize, unknown major, wrong reference kind, non-member issuer, wrong `group_id` | `invalid` or `pending` as §2.6; nothing removed; no path escape; nothing executed; valid unrelated records still apply | A-23 |
| T13 | Retired issuer: a record above the cutoff; one within it; one learned late; a second retirement of the same issuer with a **lower** cutoff arriving later; a third with a **higher** cutoff | above-cutoff `invalid(after_retirement)`, also if learned before the retirement record; within-cutoff valid after the issuer's states become unusable; the lower cutoff invalidates the records between the two cutoffs (their recovery points stay); the higher cutoff re-validates nothing; settling ignores the retired member (contracts.md T-C10, T-R4D16-later-cutoff-no-reauth) | A-45, A-20 |
| T14 | Runtime cycle loop: P present and published; deleted; ≥ 4 cycles with more than three own states published and pruned before the hold matures; restart mid-hold | baseline survives; record and own ack published after the settle wait; P never reinstalled here during the hold; receivers remove into recovery | A-10 |
| T15 | P absent in one scan (absent state published), then back with the same or new content; also across a restart | no record; hold reset; new content is an ordinary update | A-10, A-21 |
| T16 | Unreadable folder; unstable listing; AddOns directory identity changed with all folders gone | no record | A-05, A-20 |
| T17 | ASCII folder absent while a new unsupported entry appears; ≥ 4 cycles where other packages update and refresh their baselines; restart | no record; `Held(RemovalUnclear)`; "Confirm removal" writes `user_confirmed`; "Keep it" cancels | A-23, A-26 |
| T18 | Every folder in AddOns deleted (same directory identity), ≥ 3 packages | removals after the settle wait, no confirmation (D-21) | A-10 |
| T19 | Our own interrupted apply leaves `L` absent | no record (E5); recovery restores | A-22 |
| T20 | Update drops folder X; receivers with the same and a different X tree, with and without the update planned in the same cycle | `folder_drop` record; X removed into recovery in the update op or alone; new version complete | A-09 |
| T21 | After own ack, the user reinstalls P with an updater; then chooses "Add back to all computers" on C while D concurrently removes P again | `Held(ReinstallObserved)`; not removed and not propagated during the hold; after the choice a `user_add_back` re-add propagates; D's concurrent removal (not covered by C's `known`) wins (example 6) | A-10, A-20 |
| T22 | Crash after every effect of the remove op and of its recovery (including a second crash during recovery); fakes and real temp filesystem | every restart reaches `Untouched` or `Done`; no lost folder; the ack bytes are identical on retry; accurate status | A-22 |
| T23 | Folder recreated in `L` after `L→R` | `Unknown` → `needs_help`; nothing deleted | A-22 |
| T24 | WoW starts before, between and after removal renames, and during a restore | gate stops; no further change; rollback after exit; no process stopped | A-21 |
| T25 | More than three history states with a removal among them; prune; low storage with an exception | current + two previous; `removed` counts; exceptions visible; `NeedsHelp(StorageLow)` | A-16, A-17 |
| T26 | Several remove → restore → update → remove cycles; inspect provider roots after each | only live removals' manifests and settings roots are removal roots; superseded payload unrooted; provider restore offered only for a live removal | A-16, A-17 |
| T27 | Restore with a local point; without one (from the provider, on a computer that never had P, after pruning); with released payload | restored and propagated; staged from the provider with hash = `removed_hash`; `Unavailable` | A-10, A-16 |
| T28 | Settings: owned, unknown/shared, paused; paired removal and restore; restore with a missing checkpoint (settings phase) | paired / code-only with diagnosis / kept with notice; code-only restore keeps current settings with a distinct warning; WoW's own files untouched | A-09, A-10, A-15, A-19 |
| T29 | Rejected release and removal | restore does not offer the rejected version; neither record set supersedes the other | A-18 |
| T30 | Confirmation and wording: a peer without `removal/1`; a peer with an ack but still listing P; a peer acked and absent | "older version"; not confirmed; confirmed. No "removed everywhere" before all current members confirm | A-24, A-25 |
| T31 | Compatibility: run-003 `code-state/1` without new fields; new state (with `authority_index`, `features`) read by the run-003 decoder; `/1` journals and points read by the new app; `/2` remove journal and `/2` point read by the run-003 `exec` | state decodes both ways; absence never removal; `/1` defaults apply; the run-003 app reports `NeedsHelp(UnknownInterruptedState)`, keeps the `/2` journal, changes nothing and never prunes `/2` points (contracts.md T-C06) | A-36, A-20, A-22 |
| T32 | GUI: remove from all computers, see per-computer progress, confirm an unclear removal, continue without pending information, restore, add back, with keyboard only | flows complete without a terminal; wording rules hold | A-25, A-26 |
| T33 | Installed app, real Dropbox, two computers (provider tier, after owner Check 2; Q-23) | removal applied and confirmed; provenance kept | A-34 |
| T-R4D2-partial-readd-both-orders | Removals r1 {p, q} and r2 {q} live; C writes a re-add of {p} only covering both; receivers learn (a) re-add then removals, (b) removals then re-add; C holds q, D holds p and q, E holds nothing | p re-added; q stays removed by r1 and r2; identical inventories in both orders (D: p installed, q in recovery; C: q removed into recovery); obligations per `(enrollment, removal, folder)`: q's obligations to r1 and r2 survive; recovery points name r1, r2 for q | A-10, A-20 |
| T-R4D3-ack-before-state | C acks r for P with ack `issued_at_seq` 7, but its state seq 8 (still listing P) is learned before the ack; then the ack; then restart; duplicate ack copies; then C publishes seq 9 with v2 of P (a reinstall after acking) | seq 8 is not a discharge; discharge only from the first admitted state with seq ≥ the ack's `issued_at_seq` that is absent for P; duplicates change nothing; v2 at seq 9 is `Held(ReinstallObserved)` on C, never a candidate elsewhere | A-20, A-25 |
| T-R4D4-1024-removals-readd-bound | 1,024 live removals of one folder from 8 issuers; one restore re-adds it | the re-add is ≤ 64 KiB (`known` only); covers all 1,024; the folder becomes Added | A-20 |
| T-R4D4-1024-issuers-readd | 1,024 removals of one folder by 1,024 different enrollments (contracts.md §10) | the re-add carries `known.seen` in parts, each file ≤ 64 KiB; it stays pending until every part arrives, then covers all 1,024 | A-20 |
| T-R4D4-ledger-overflow-16MiB | Synthetic ledger above 16 MiB across segments, plus 100 remove → restore cycles of one folder | segments roll at 4 MiB; nothing is dropped or `NeedsHelp`; the merged result equals the unsegmented reference; memory within the C-AUTH note (contracts.md §1.7) | A-20, A-41 |
| T-R4D5-passive-uninstall-settings | C uninstalls P by hand (P owns account and character SavedVariables); also a folder drop that removes an owning folder; crash after cleanup before the ack; crash before cleanup | C's settings for P are checkpointed then moved by a `settings_cleanup` op (§6.3); C's `absent` ack (`settings: paired`) is written only by its finalize, from the bytes reserved at journal creation; after either crash the restart finishes or rolls back, the ack is not written early, and settings are never lost (`/2` point, `paired: settings_only`) | A-09, A-10, A-15, A-22 |
| T-R4D5-absent-ack-waits-cleanup | Runtime `sync_once` (real step order 2…8): the passive-uninstall removal matures on C **while WoW runs**; C also receives a peer removal of folder Q whose owned settings it holds; several cycles; then WoW exits | while WoW runs: no ack item for P or Q is written at 3b or anywhere (both `cleanup_required`), nothing is published as acked, status "waiting for WoW to close"; after exit: one cleanup op each, acks written by finalize only; a third folder with nothing owned is acked at 3b in the first cycle (`not_applicable`) | A-10, A-15, A-21, A-24 |
| T-R4D1-ack-bytes-fixed | Remove op: crash between verify and the ack write; restart with a different clock and after learning new records | the written ack is byte-identical to the bytes reserved in `pending_records` at journal creation (same hash, `issued_at_seq`, `known`) | A-22 |

**A-38 negative controls** (temporarily mutate the production rule, observe
the named test fail with exit 101, restore) for at least: T3 (reintroduce an
id-variant conflict), T4 (make the merge depend on arrival order, e.g. take
the latest-learned record), T5 (let an updated copy win), T7 (re-add without an explicit choice), T8
(ignore owing), T10 (forget a pending hint, or block removals on it), T11
(trust the ref over the record), T14 (read E1 from the newest published
state, or omit `Held`), T17 (compare E6 with a shared latest snapshot), T22
(regenerate the ack on retry), T26 (root superseded payload), T30
(confirm on publication alone), T-R4D2 (key obligations per package or
per removal only), T-R4D3 (discharge on any later-learned state), T-R4D4
(store covered removals in the re-add), T-R4D5 (ack before settings
cleanup) and T-R4D5-absent-ack-waits-cleanup (drop the
`cleanup_required` exclusion from step 3b). T7 mutates the hold into an
automatic re-add.

## 13. Review findings: status after this rewrite

### 13.1 R3D-13 closure

R3D-13 ([recheck 1](../runs/run-003/evidence/R3-design-recheck.md),
[recheck 2](../runs/run-003/evidence/R3-design-recheck2.md)) was: a record id
with two variants is permanently inconsistent, the decision meant to resolve
it had to list every variant, and V7 rejected the decision when a variant
named another package. **It is closed by removing the state, not by
resolving it.**

1. **No id variants.** Identity is the hash of the exact bytes and the file
   name (§2.1). Different content is a different record. A copy that does
   not match its name is not a record. The run-003 trace (variants `(T, hP)`
   and `(T, hQ)`) becomes two valid removals, of P's folders and of Q's
   folders (example 3, T3).
2. **Nothing needs resolving.** Any valid removal wins (D-21, D-23), and
   removals merge by union of folder keys (§3.1). There is no
   `conflicted` status, no `InconsistentRemovalRecord`, and no removal
   decision of any kind (§4.3).
3. **The run-003 V7 problem is gone.** No rule requires a record to list related records. The
   rules (§2.6) check only that a record's own listing is well-formed. A
   re-add supersedes the removals its `known` covers, per folder, so every
   record the app writes passes validation, whatever packages the related
   records span.
4. **Every reachable state resolves.** For every folder key, the result is a
   total function of the ledger (§3.2). The only local states that wait
   either clear by themselves or have a GUI action: the settle hold (matures
   or resets), the unclear rename (confirm, keep, or reappearance), the
   reinstall hold (add back or remove again), pending information (learned,
   repaired, or "Continue without it"), WoW running (exits), and a
   needs-help journal (the existing GUI resolution). None needs a terminal or
   a record that cannot be written.
5. **Order independence.** A removal concurrent with a re-add wins
   (example 6); stale copies are decided by the audience and acks recorded
   in immutable records, not by arrival order (§3.4, T4, T8).

### 13.2 All run-003 design findings

| Finding | Area | Status in this rewrite |
|---|---|---|
| R3D-1 | Q-21: malformed Interface entries | **Closed in run-003; unaffected.** Removal never reads `## Interface:` |
| R3D-2 | Q-21: unknown builds bypass the check | **Closed in run-003; unaffected.** Removal has no build gate |
| R3D-3 | Q-21: installed declarations as proof of support | **Superseded by owner decision D-21** (no Interface gating; SPEC R-05 amended). This rewrite makes no compatibility claim |
| R3D-4 | UI: Forever labelled as Classic Era | **Closed; kept.** Removal text uses the identity labels of ui/screens.md |
| R3D-5 | UI: reconfirm scenario | **Closed; kept.** A variant or mapping change discards the baseline (E3, §1.6); a build change does not (O-N2) |
| R3D-6 | UI: continuation promises versus journals | **Closed; kept.** Any journal blocks new removals (Q-25, §5.1); the GUI never promises a removal while a journal blocks |
| R3D-7 | Removal: publication destroys the baseline | **Kept unchanged.** Separate durable presence baseline, step 3a, `Held` (§1.6) |
| R3D-8 | Removal: authority lost with an unusable newest state | **Kept, strengthened.** Durable ledger, mirroring on learning, plus own-copy repair ([contracts.md](contracts.md) §1.12). Pending information no longer blocks a whole package (§4.4) |
| R3D-9 | Removal: compact tombstones and lost restore data | **Kept, simplified.** Identity = hash of exact bytes = file name; refs carry only that hash plus a routing hint that is never trusted over the record; exact `folders` manifest kept (§2.1–§2.4; [contracts.md](contracts.md) §1.3–§1.5) |
| R3D-10 | Removal: keep-version decision overridden | **Mechanism removed from removal.** Removal makes no version choice. The code-choice family that replaces it is [contracts.md](contracts.md) §5, which retraces R3D-10 there |
| R3D-11 | UI: coverage claims from placeholders | **Closed; kept.** §9 lists needs; it claims no acceptance from placeholders |
| R3D-12 | UI: layout and headline rules | **Closed; kept.** ui/screens.md is authoritative; removal adds no headline state (§9) |
| R3D-13 | Removal: inconsistent authority unresolvable | **Closed by D-23** (§13.1) |
| R3D-14 | Removal: one global snapshot | **Kept unchanged.** Per-entry snapshot references (§1.6) |
| R3D-15 | Removal: superseded payload kept forever | **Kept, simplified.** Only live removals root payload (§7.3) |
| R3D-16 | Removal: resume formula declines the kept version | **Mechanism removed from removal.** Code-choice resume keeps earlier exclusions and never declines the kept version (Q-50); retraced in [contracts.md](contracts.md) §5.4 |

## 14. Changes to other design documents when implemented

Recorded here because this packet does not own those files.

| Document | Change |
|---|---|
| [domain.md](domain.md) §6 | Selection runs below the removal precedence of §4.1: rejections, local holds, merged removal, stale copies, pending hints, then ordinary ordering over observations stripped of removed and owed folders. The Q-2 slice limitation is replaced by the folder-drop rule (§1.5). "There is no removal record in the slice" becomes: "Absence never removes; only a live removal record in the ledger (removal.md §3) removes." New target states `Remove` and `Held`; no new `DecisionReason` |
| [domain.md](domain.md) §1 step 5 | The consent text also states that addons deliberately uninstalled on one computer are removed on the others and kept for recovery, with their settings (§6), and lists the joiner's copies of removed addons (§8.3) |
| [publication.md](publication.md) §1 | layout gains `members\<computer_id>\authority\<sha256>.json` (single writer, write-if-absent, never deleted while the group exists). Readers list it for every member including unusable and retired ones |
| [publication.md](publication.md) §2 | additive `authority_index` and `features` in `code-state/1` ([contracts.md](contracts.md) §1.6); record schemas of §2.3; local schemas of §11; ack item results; confirmation rule of §5.6 |
| [publication.md](publication.md) §4 | publish writes, reads back and repairs referenced records and index chunks before the state; step 7 pruning never touches `authority\` |
| [publication.md](publication.md) §6 | admission order of [contracts.md](contracts.md) §6; learning and validation are a separate step whose output is the ledger |
| [apply.md](apply.md) §1 | steps 2a, 3a and 3b (§1.6); step 5 uses §4.1 |
| [apply.md](apply.md) §2.2, §3.1, §4.1 | remove and restore operations, the remove slot table and crash rows (§5.3); mixed update-and-remove operations (§5.1) |
| [apply.md](apply.md) §2.4 | pruning counts history states including `removed` (§7.1) |
| [apply.md](apply.md) §6 | statuses of §10 `core::status` |
| [core-api.md](core-api.md) | the §10 module map |
| [questions.md](questions.md) | Q-40's default is revised by Q-56; Q-42 stays rejected; Q-49 and Q-50 no longer apply to removal (no removal decisions) |
