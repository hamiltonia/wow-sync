# Addon removal and uninstall across the group

Status: **design for a later run** (run-003 packet
[R3-removal-design](../runs/run-003/packets/R3-removal-design.md)). Not yet
implemented. Revised in retry 1 (R3-removal-fix) for review findings
R3D-7 (presence baseline, §1.6), R3D-8 (authority ledger, §2.4–§2.6),
R3D-9 (immutable intent and refs, §2.2–§2.3) and R3D-10 (selection
constraint, §3.3), and in retry 2 (R3-removal-fix2) for R3D-13
(resolving inconsistent authority, §2.5), R3D-14 (per-baseline top-level
evidence, §1.6), R3D-15 (bounded payload retention, §2.7) and R3D-16
(per-choice `declined` formulas, §3.3); not yet re-reviewed. Nothing here changes
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
| removal intent | the group-visible claim "computer X deliberately uninstalled package P, as these exact folder versions". An immutable, hashed authority object, also called the **tombstone** (§2.2) |
| authority object | an immutable, content-hashed provider file holding an intent, decision, re-add supersession or confirming acknowledgment (§2.2). States carry only **refs** to them (§2.3) |
| authority ledger | this computer's durable, grow-only, validated set of learned authority objects (§2.4). Removal results are computed from it, not from any member's latest inventory |
| presence baseline | this computer's durable local record of the last complete scan in which package P was fully present (§1.6). The prior-presence evidence for E1 |
| covered | a folder version that the intent's issuer knew about when it formed the intent (§2.7). Only covered versions may be removed without a decision |
| acknowledgment | a member's statement that it has applied an intent (removed, or never had the package). Used for "Confirmed" and for settling (§2.7) |
| re-add | a deliberate install of a package after its intent was acknowledged on the same computer. Supersedes the intent (§2.6) |
| selection constraint | the effect of a "keep version V" decision: the group target for the package is exactly V until later authority replaces it (§3.3) |
| folder drop | an update in which the author's new release no longer contains one folder of the package (§1.5) |

## 1. Intent: what counts as a deliberate removal (R-03, R-04)

### 1.1 Principle

Absence is never removal authority, neither locally nor from the provider
(R-04: "A missing Dropbox object, incomplete join or temporary local/provider
failure is not deletion authority"). Removal is derived only from a
**positively observed local transition** on the originating computer:
"this computer had package P, completely and readably, and now has none of
it, and nothing else explains that". The originating computer then
publishes an explicit, immutable intent object (the tombstone, §2.2).
Receivers act only on a validated intent in their authority ledger (§2.4),
never on a peer's state that merely lacks the package.

Detection is local and passive: the user uninstalls with CurseForge, another
updater or Explorer, as today. The app does not require a WoW Sync button to
uninstall (R-05: the app is not a substitute updater). The GUI also offers
"Remove from the group" for convenience (§7). That action produces the same
evidence by moving the folders to recovery itself.

### 1.2 Required evidence (all must hold)

`core::removal::observe_removals(baseline, inventory, journals, ledger,
observations) -> (Baseline, Vec<RemovalObservation>)` is pure. It runs at
cycle step 3a (§1.6). A package P becomes a **removal intent** of this
computer only if every row holds:

| # | Evidence | Rules out |
|---|---|---|
| E1 | **Prior presence.** This enrollment has a durable presence-baseline entry for P (§1.6), written from a stable, complete scan of the same variant and AddOns directory in which every folder of P was present, `ok` and eligible. Published or admitted own states are **not** read: the absent state that the same cycle publishes, and pruning of old own states, cannot erase the evidence | "never had it"; a new enrollment's missing addons (R-03, A-04) |
| E2 | **Whole package absent.** No folder of P, by `FolderKey`, appears in the listing in **any** status (ok, unreadable, unsafe, unsupported, unstable). Partial absence is not a removal of P (§1.5) | unreadable folder; failed or partial scan of one folder |
| E3 | **Listing is trustworthy.** The AddOns top-level listing was stable and complete in the scan. The variant verdict is the enrolled identity (no `VariantChanged`) and the layout assessment is `Supported` with the enrolled mapping (domain.md §2.1). The assessment digest is **not** compared: it hashes the eligible and blocked folder lists, which change with every install and removal. The AddOns directory has the same **directory identity** as recorded in the baseline (Windows: volume serial + file ID; macOS: device + inode) | moved or re-pointed install root; a recreated empty AddOns; variant change (`NeedsHelp(VariantChanged)` already stops the cycle) |
| E4 | **Persistence.** E1–E3 hold in two consecutive complete scans at least `REMOVAL_HOLD` apart (Q-41, default 30 s and two sync cycles). Any reappearance of any folder of P resets the hold | an updater's delete-then-extract during an update; a transient listing glitch |
| E5 | **Not our own move.** No journal (any phase) of this computer names P or any folder of P, and no journal remained after `recover_interrupted` in either scan's cycle | our own apply or recovery temporarily moving folders out of `AddOns` |
| E6 | **No unexplained newcomer.** Compared with the top-level snapshot referenced by **P's own** baseline entry (§1.6), never a snapshot from a later scan that refreshed another package, no **new** non-`ok` top-level entry appeared (unsupported, unreadable, unsafe or unstable), and no new entry whose name differs from a folder of P only by Unicode, case or normalization, or that `paths` rejects (Q-14) | a rename to a non-ASCII or otherwise unsupported name; a folder that became unreadable (it is listed with that status, so E2 fails anyway); a copy that the app cannot read |
| E7 | **Not a mass disappearance.** Fewer than `MASS_REMOVAL` packages satisfy E1–E6 in the same window, and AddOns is not empty. Otherwise the intents are held as `PendingLocalConfirmation` and the GUI asks "Did you mean to uninstall these N addons from all computers?" (Q-42, default: ≥ 3 packages or ≥ 25 % of synced packages, or an empty AddOns) | a wiped or replaced AddOns; a mistaken bulk delete; an updater reset |
| E8 | **P was synced.** P was transfer-eligible here (no `Blocked(UnvalidatedTocMapping)` folder, domain.md §2.1) and is not in a `NeedsDecision` with an unresolved removal kind (§3) | removing something the group never shared |

E2 is evaluated over the baseline's folder keys for P, so a grouping change
cannot shrink the set that must be absent. Holding is persisted locally as
`wowsync.removal-observation/1` (§2.8). The hold spans ordinary cycles and
publications; while it lasts, P is `Held` and is not reinstalled (§1.6). A
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
| Never had it (new enrollment, or P was added elsewhere after this computer's last state) | E1 fails: no presence baseline for P. P is an addition and is installed if eligible (unchanged slice behaviour) |
| Failed or partial scan | Listing unstable or unreadable: the cycle publishes nothing (apply.md §1 step 3), so E3 fails. A folder that could not be read is listed with its status, so E2 fails |
| Unreadable folder | Listed as `unreadable`, so E2 fails. Shown as a folder problem, never as a removal |
| Non-ASCII name (Q-14) | Folders with such names are `unsupported`, never synced, so they are never subjects of E1. An ASCII folder that becomes absent while a new `unsupported` entry appears fails E6. Shown as "Can't tell whether *X* was removed or renamed". The user may confirm removal in the GUI (§7), which is then recorded with `evidence: user_confirmed` |
| Provider offline or stale here | The intent is formed from local evidence (the baseline, §1.6) and kept locally until it can be published. Its coverage basis (§2.7) is whatever was admitted. Staleness can only **add** later decisions, never cover a version the issuer did not see |
| Provider offline or stale on a receiver | A receiver that has not learned the intent changes nothing for it. An intent it **has** learned stays in its durable ledger (§2.4) and stays authority even if the issuer's newest state later becomes `Unusable` or `Regressed`, or the provider copy becomes unreadable. Inventory is still never taken from an unusable state (publication.md §6 no-fallback rule); only immutable authority objects are read from any member directory (§2.1) |
| Updater mid-update (delete then extract) | E4 resets on reappearance. If the folder reappears with new content, it is an ordinary update |
| User renames a folder (ASCII to ASCII) | The old package satisfies E1–E8 and is removed; the new name is a new addition. This is the honest observed meaning. The GUI shows both together in the grouped summary |
| Our apply or recovery moved folders out | E5 fails |

### 1.4 Explicit local removal

"Remove from the group" in the GUI (§7) on a package that is present and
`InSync` creates a local removal op (§4) on this computer first, with
`evidence: user_action`. After it is verified, E1 to E3 hold by
construction, so the hold of E4 is skipped and E7 does not apply (one
explicitly chosen package). The intent is then published as in §2. This
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

### 1.6 Presence baseline and cycle insertion point (R3D-7)

Prior presence (E1) is evidence that this computer **had** P. It must not
depend on the newest published own state, because the ordinary cycle
publishes the absent inventory in the same cycle that first sees P missing
(apply.md §1 step 4), and own state files older than the newest three are
pruned (publication.md §4 step 7). It is therefore a separate, local,
durable record: `wowsync.presence-baseline/1` (§2.8).

**Baseline rules.**

- **Written** for package P from any scan in which the AddOns top-level
  listing was stable and complete, the variant and layout digests equal the
  enrolled ones, no journal names P (E5), and every folder of P is present,
  `ok`, not `Unstable`, and transfer-eligible (E8). The entry records the
  folder keys, names and trees, the package hash, the local scan counter and
  the newest own published seq that listed P, if any (`published_seq`).
  The entry references the **top-level snapshot** of that scan (every
  top-level name with its status) by the scan counter. Snapshots are stored
  in the baseline record and kept while at least one package entry
  references them; a snapshot no entry references is dropped. Refreshing
  package Q therefore never replaces the snapshot that a still-absent
  package P depends on (R3D-14). Normally every present package refreshes
  each scan, so only the newest snapshot and one per held or absent
  baseline remain. The record-level `addons_dir_id` and enrolled variant
  identity and mapping are those of the scans; a change discards the whole
  record (below).
- **Persisted** with temp + flush + rename **before** the cycle proceeds to
  publication. A restart reloads it. It holds its own content (keys and
  trees), so pruning of own state files or of the provider never removes it.
- **Refreshed** when P is again fully present (reappearance or an update
  installed by anyone). A refresh replaces the folder list; it never merges
  in absent folders.
- **Cleared** for P when its own removal intent is published and this
  computer's own `removed` ack object is written (§2.2), when a folder drop
  is recorded for those folders (§1.5), when the user chooses "keep it" for
  a held observation and P is re-installed (§7), or when P is removed here
  by a received intent. The **whole** baseline is discarded on a new
  enrollment, a variant change or an `addons_dir_id` change: E1 then fails
  for every package until a new complete scan writes it again.
- **Not** written from a partial or unstable listing, from a scan during
  which a journal existed, or from a package with any non-`ok` folder.

**Cycle insertion.** Removal observation is a new step **3a** of the sync
cycle (apply.md §1), after the scan and `build_inventory` (step 3) and
before publication (step 4). A matching step **2a** after admission
(step 2) does authority learning (§2.4):

```text
1  recover_interrupted
2  admit member states; persist watermarks
2a learn authority objects into the ledger (§2.4); persist; derive validity (§2.5)
3  scan AddOns → build_inventory   (listing unstable → stop, as today)
3a observe_removals(baseline, inventory, journals, ledger, observations):
     update/persist baselines; update/persist observations (E1–E8);
     for each observation that became ready: write the intent and own ack
     objects (§2.2), record ledger entries, mark the observation
     `published` once step 4 lands
4  publish (inventory, authority_refs, progress) if anything changed
5  compute_target (rejections → held/pending → ledger authority →
   selection constraint → domain.md §6; §3.1)
6  activity gate → plan_apply (installs, replaces, removes)
7  run_apply; rescan; steps 3a and 4 again
8  derive_status
```

The absent inventory is published at step 4 as usual, in the first cycle
that sees P missing. That is harmless: absence is never authority (§1.1),
and E1 no longer reads published state. The hold (E4) spans as many cycles
as needed. Each cycle's step 3a advances it from the persisted observation
and the persisted baseline.

**Held packages.** From the first scan where E1–E3 hold for P until the
observation ends, the local target for P is `Held(RemovalObserved)`, and
while an observation is `pending_local_confirmation` it is
`Held(RemovalConfirmation)`. Nothing for P is installed, replaced or
removed here. Without this rule, the slice's addition rule would reinstall
P from peers during the hold and undo the user's uninstall. The
observation ends by:

- reappearance of any folder of P (hold reset; baseline refreshed);
- publication of the intent, after which P's target comes from the ledger
  (§3.1: the issuer's own intent covers its baseline trees, so P stays
  absent here and is acknowledged `removed`);
- the user choosing "keep it" (§7): the observation is cancelled with
  `reason: kept`, and P is an ordinary addition again, installed from the
  group; or
- a baseline discard (enrollment, variant or directory change), which also
  discards observations.

## 2. Records: authority objects, references and the ledger

Retry 1 (R3D-8, R3D-9) replaces the earlier "tombstone carried inside the
state" design. Removal **authority** (intents, decisions, re-add
supersessions and confirming acknowledgments) is kept separate from
**inventory** (the code state). Authority is immutable, content-hashed and
learned into a durable local ledger. Inventory stays the newest admitted
state with no fallback (publication.md §6).

### 2.1 Where it lives

```text
groups\<group_id>\members\<computer_id>\
  member.json                          unchanged
  state\<enrollment_id>-<seq:010>.json wowsync.code-state/2 (refs only, §2.3)
  authority\<id>.json                  immutable authority objects (§2.2), written by
                                       this computer: its own objects and verbatim
                                       mirrors of objects it learned (§2.4)
```

- Single writer per member directory is unchanged (publication.md §1). An
  authority file is written **write-if-absent**, temp + flush + rename, and
  read back and hash-checked before any state that references it is
  published. It is never rewritten or deleted while the group exists (Q-44).
- `<id>` is the object's random 32-hex id. Ids are never reused, so an
  issuer's own objects and the mirrors it holds share one directory without
  collisions.
- A reader lists `authority\` of **every** member directory of the group,
  including members whose newest state is `Unusable` or `Regressed`, and
  retired members. This is not an inventory fallback: authority objects are
  immutable facts whose validity is checked per object (§2.5), not "the
  last good inventory". Candidates for installing or keeping code still
  come only from admitted inventory.
- The state schema becomes `wowsync.code-state/2` (Q-40), a **major**
  change because it changes what absence means to a reader. `/1` readers
  refuse it as `UnsupportedMajor`, shown as "written by a newer version"
  (Q-13). They then neither remove nor resurrect anything from it.

### 2.2 Immutable authority objects

Every object has `schema`, `id`, `package_key`, `issuer`
`{computer_id, enrollment_id, issued_at_seq}` and `written_unix_ms`
(display only). `issued_at_seq` is the seq of the first own state that
references the object: the issuer reserves it before writing the object and
publishes that state next.

The **object hash** is SHA-256 over `"wowsync-authority-v1\n"` + the
canonical encoding of **every** field, including `written_unix_ms`. Nothing
is excluded and nothing is ever recomputed or rewritten. Mirrors copy the
bytes verbatim, so every copy of an object has the same hash.

**Removal intent** (`wowsync.removal-intent/1`, the tombstone):

```json
{
  "schema": "wowsync.removal-intent/1",
  "id": "<32 hex>",
  "package_key": "suite-core",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 39 },
  "evidence": "observed|user_action|user_confirmed|decision",
  "baseline": { "scan": 1207, "present_seq": 38 },
  "removed": [{ "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" },
              { "key": "suite_data", "name": "Suite_Data", "tree": "<sha256>" }],
  "removed_hash": "<package hash of `removed`>",
  "covers": [{ "key": "suite-core", "trees": ["<sha256>", "…"] },
             { "key": "suite_data", "trees": ["<sha256>"] }],
  "basis": [{ "computer_id": "…", "enrollment_id": "…", "seq": 17,
              "package_hash": "<sha256>|null" }],
  "knows": [{ "id": "<32 hex>", "hash": "<object hash>" }],
  "dependents_at_origin": ["<package keys that hard-depended on a removed folder>"],
  "settings": { "kind": "none_in_this_version" },
  "written_unix_ms": 1767225600000
}
```

- `removed` is the **exact restorable manifest**: every folder key, its
  name and its tree, taken from the presence baseline (§1.6). With the
  provider tree records and blobs it is sufficient to restore the package
  on any computer, including one that never had it (§6.3).
- `baseline.present_seq` is the newest own published state that listed the
  package, or `null` if the presence was never published (for example the
  provider was offline). It is provenance only. Coverage and restore do not
  depend on it.
- `settings` is the settings root. While settings are not synchronized it
  is `{ "kind": "none_in_this_version" }`. The settings design (§5) replaces
  it with `{ "kind": "checkpoints", "roots": [{ "identity": "…",
  "checkpoint": "<content hash>", "location": "local|provider" }] }`. A new
  settings form is a new object schema major, never an edit of an existing
  object.

Every reference from one authority object to another (`knows`,
`considered`, `supersedes`, `ack`, `intent`) is a **variant reference**
`{ "id", "hash" }`, never a bare id, so a record can name exactly which
copy of a conflicted id it means (§2.5). `knows` lists every variant of
every intent, decision and supersession for the package that the issuer
had in its ledger.

**Removal decision** (`wowsync.removal-decision/1`): §3.3.

**Re-add supersession** (`wowsync.removal-supersession/1`):
`{ id, package_key, issuer, supersedes: [<intent ref>], ack: <ack ref>,
present: [{key, name, tree}], written_unix_ms }` (§2.6).

**Confirming acknowledgment** (`wowsync.removal-ack/1`):
`{ id, package_key, issuer, intent: <intent ref>, result: "removed|absent",
written_unix_ms }`. Only these two results are authority. `pending` and
`decision` are progress and stay in the state (§2.3). The issuer writes
its own `removed` ack together with its intent.

### 2.3 References in the state (the compact form)

`wowsync.code-state/2` adds to every `/1` field (all required; empty arrays
allowed):

```json
{
  "schema": "wowsync.code-state/2",
  "…": "all code-state/1 fields unchanged",
  "addons_dir_id": "<opaque hash of the directory identity, E3>",
  "folders": [ { "…": "as in /1", "dropped_by_update": false } ],
  "packages": [
    { "key": "suite-core", "hash": "<sha256>",
      "dropped": [{ "key": "suite_old", "name": "Suite_Old", "trees": ["<sha256>"] }] }
  ],
  "authority_refs": [
    { "ref": 1, "id": "<32 hex>", "kind": "intent|decision|supersession|ack",
      "package_key": "suite-core", "hash": "<object hash>" }
  ],
  "progress": [ { "intent": "<32 hex>", "result": "pending|decision", "seq": 41 } ]
}
```

- A ref (`ref: 1` is the ref format version) is the **only** compact form.
  It never carries authority content, so there is no second hash to keep
  consistent with the object.
- **Equivalence:** a ref and an object are the same record iff `id`,
  `kind`, `package_key` and `hash` all match the object's fields and its
  recomputed hash. A ref whose `kind` or `package_key` differs from the
  object, or two objects or refs with the same `id` and different hashes,
  make that id **conflicted** (§2.5).
- `authority_refs` lists every **valid** entry of the member's ledger
  (§2.4), not only its own objects. It is grow-only for an enrollment. A
  reader receiving a state with fewer refs than an older admitted state of
  the same enrollment ignores the omission (refs are union-merged into the
  ledger, never subtracted).
- Unknown `ref` versions make the referencing state `Unusable(unknown ref
  format)` for inventory and contribute no refs. Unknown object schemas are
  `pending(unknown_schema)` in the ledger and put the package in
  `Unavailable(AuthorityPending)` (shown "written by a newer version").
- "Settled" (§2.7) is a derived status. No record is compacted, rewritten
  or shortened when an intent settles.

### 2.4 The authority ledger (local, durable)

`wowsync.authority-ledger/1` is a local, durable, grow-only set of
authority objects known to this enrollment:

| Field per entry | Meaning |
|---|---|
| `id`, `kind`, `package_key`, `hash` | as in the ref; an entry is one **variant** `(id, hash)`, so a conflicted id has several entries |
| `bytes_path` | local verbatim copy under the app data `authority\` directory |
| `issuer` | from the object |
| `learned_from` | `[{computer_id, path}]`, every place the object was read |
| `validity` | `valid` \| `pending(reason)` \| `conflicted` \| `invalid(reason)`, re-derived (§2.5) |
| `mirrored_seq` | the own seq at which this computer first referenced it, or `null` |

Learning, in cycle step 2a (§1.6):

1. For every admitted member state, every `authority_refs` entry not yet in
   the ledger is looked up: first the referencing member's `authority\`,
   then the issuer's, then any member's. An entry whose object cannot be
   read (missing, cloud-only, malformed, oversized) is recorded as
   `pending(object_unreadable)` with its ref data.
2. Every member's `authority\` listing is read. Each object not yet known
   is read and hash-checked.
3. Each new object is copied locally (temp + flush + rename) and the ledger
   entry is persisted **before** any target is computed from it. Then
   validity is derived (§2.5).
4. Each newly valid object is mirrored verbatim into this computer's own
   provider `authority\` and listed in the next own state's
   `authority_refs`. Mirroring happens on **learning**, not on
   acknowledgment (Q-51). The intent therefore survives its issuer's directory,
   newest state or enrollment becoming unusable before anyone acted on it.

Entries are never deleted. A restart reloads the ledger before the first
target computation, so an intent learned before a crash or restart is still
authority afterwards, even if every provider copy has since become
unreadable. Provider absence of an object is never a reason to forget it.

### 2.5 Validation

An entry is `valid` only if all hold:

| # | Rule | Otherwise |
|---|---|---|
| V1 | The object parses, its schema major is known, its size ≤ 64 KiB, and its id has exactly **one** known variant: every ref and every copy seen for that `id` has the same hash, kind and package key | two or more variants of one id → **every** variant is `conflicted` (below); unknown major → `pending(unknown_schema)`; unreadable bytes → `pending(object_unreadable)` |
| V2 | `issuer.computer_id`/`enrollment_id` is a member enrollment of this group (`member.json`), current or retired | `invalid(not_a_member)`; diagnostic only |
| V3 | If the issuer enrollment is retired: `issued_at_seq` ≤ the retirement cutoff, or the id is listed in the retirement record's `authority_known` (§8.2) | `invalid(after_retirement)` |
| V4 | Every variant reference in `knows`, `considered`, `supersedes`, `ack` and `intent` names a variant that is in the ledger, in **any** status (`valid`, `conflicted` or `invalid`). In addition `chose.remove.intent` must be `valid`, and V5 requires a `valid` ack. A reference never requires a conflicted variant to become valid | `pending(missing_reference)` |
| V5 | Supersession: `ack` is a `removed`/`absent` ack by the **same enrollment** for every superseded intent, with a lower `issued_at_seq` than the supersession | `invalid(readd_without_ack)` |
| V6 | Decision: §3.3 well-formedness (chosen alternative among `considered`, manifests hash to the declared package hashes, `declined` derived correctly) | `invalid(malformed_decision)` |
| V7 | `package_key` of every referenced object equals the entry's `package_key` | `invalid(wrong_package)`; diagnostic only |

Validity is re-derived whenever a member or retirement record changes, and
whenever a referenced entry arrives. `invalid(after_retirement)` can
therefore appear after the entry was `valid`. The effect of that is in §8.2.

Statuses are distinct and handled differently:

| Status | Meaning | Effect for package P |
|---|---|---|
| `valid` | authority | §2.6 |
| `pending(reason)` | **missing** authority: referenced or listed but unreadable, unknown schema, or a missing reference | `Unavailable(AuthorityPending)` on this computer: no install, replace or removal for P until it resolves. Other packages proceed. A computer that knows an authority object exists but cannot read it does not act on its older knowledge |
| `conflicted` | **known-inconsistent** evidence: the id has two or more variants | never authority (no Active(P), coverage, ack or supersession effect); `NeedsDecision(InconsistentRemovalRecord)` until resolved (below) |
| `invalid(reason)` | known and well-formed but not authority (not a member, after retirement, re-add without ack, malformed decision, wrong package) | ignored for P; diagnostic only; never blocks |

**Resolving a conflicted id (R3D-13).** The GUI decision for
`InconsistentRemovalRecord` is a normal decision object (§3.3) with
`kind: inconsistent_removal_record` that lists **every** known variant of
the conflicted id as variant references in `considered.intents` (or
`considered.decisions` for a conflicted decision id). Because V4 accepts
references to conflicted variants, the decision is `valid` while the
variants stay `conflicted` forever in the grow-only ledger. The id is then
**resolved**: it no longer causes a decision, and the decision's choice
(keep removed, or keep version V) is the authority for P. Late replay of a
listed variant, from any member or after a restart, changes nothing. A
variant of that id that the decision did not list (a third copy, or one
learned only afterwards) is new information: it reopens
`InconsistentRemovalRecord`, and the next decision must know the earlier
one and list the new variant. Every computer derives the same resolved
set from the same ledger, so the result converges.

An authority object that is not visible anywhere yet is unknown, not
pending. The result is only a delay, the same as an intent that has not
been published yet. As soon as it is learned, covered copies that spread
meanwhile are removed (§3.1). A computer that installed P during that
window has no ack for the intent, so its copy is never treated as a re-add
(§2.6).

### 2.6 Active authority per package

For package P, all from **valid** ledger entries (`conflicted` variants
are never members; references match on `(id, hash)`):

- **Intents(P)** = removal intents for P, minus those named in a valid
  supersession's `supersedes`, minus those in a valid decision's
  `considered.intents` (a decision replaces everything it considered).
- **Decisions(P)** = valid decisions for P not named in the `knows` of
  another valid decision or intent for P.
- **Active(P)** = Intents(P) ∪ Decisions(P). It depends only on the ledger,
  which only grows, so the result is independent of arrival order,
  duplicates, late delivery and restarts (R-06, A-20). It does not depend
  on whether the issuer's newest state is admitted.

**Re-add.** A member M that has a valid `removed`/`absent` ack for intent
T, and whose package P later becomes present by a deliberate install (two
stable complete scans, E5: not our own apply), writes a supersession for T
(with `ack` = a reference to its ack and `present` = the re-added manifest), then
publishes P present. The re-added version propagates as an ordinary
addition. A member without an ack for T is never re-adding. Its copy is
covered (removed when safe) or uncovered (decision). This is the line
between "re-add" and "offline peer resurrects" (R-06, R-10).

Uninstalling P again after a re-add creates a **new** intent whose `knows`
includes the supersession.

### 2.7 Coverage, lifetime and retention

**Coverage.** `covers` lists, per folder key, every tree that the issuer
**observed** when the intent was formed: its baseline tree (§1.6) plus the
trees of every candidate it had admitted for P, including `Unavailable`
candidates whose folder trees it knew. `basis` records where each came
from. A folder version on a receiver is **covered** iff its tree is in
`covers` for that key. A receiver package is covered iff every local folder
is covered and its folder set is a subset of the `removed` key set.

- **Conservative direction:** a version the issuer did not see is
  uncovered. An offline update always becomes a decision (§3), never a
  silent deletion.
- **Accepted imprecision (Q-43):** a member that freshly installs exactly a
  covered release concurrently with the removal, without having seen the
  intent, is treated as holding the removed release. Its copy is removed
  into a recovery point (restorable, §6) and named in the grouped summary.

**Lifetime** (derived status; the objects never change):

| Status | Condition |
|---|---|
| held | local observation only (§1.6); nothing published |
| active | in Active(P); some current non-retired member has no valid `removed`/`absent` ack |
| settled | in Active(P); every current non-retired member has a valid ack |
| superseded | removed from Active(P) by a supersession or decision; kept in the ledger as history |

A settled intent remains authority for coverage: a late or re-enrolled
member holding a covered version still has it removed when safe (§8).
Authority **metadata** (the objects: ids, hashes, manifests as lists of
tree hashes, covers) is kept while the group exists, so a removed addon
cannot be resurrected (Q-44). This metadata is small (under 64 KiB per
object, typically about 1 KiB) and holds no addon or settings content.

**Payload retention (R3D-15).** Authority metadata never roots payload by
itself. Provider **payload** (tree records, blobs and provider-held
settings checkpoints) is a retention root for removal only in these cases,
each tied to the package's **current** state or a visible temporary
exception, never to superseded history:

| Payload root | Kept while | Visible as |
|---|---|---|
| `removed` manifest and settings roots of an intent in Active(P) | the intent is in Active(P), i.e. the removal is P's current state (Q-53) | history state "Removed — restorable on any computer" with its size in storage use |
| chosen manifest of a keep decision in Active(P) | the decision is in Active(P) (it is P's current target) | the current version |
| every alternative of a frozen package (§3.2) | the decision is recorded and applied | temporary exception (§6.2) |

When an intent or keep decision leaves Active(P) (re-add, later decision,
or a later removal that knows it), its payload stops being a removal root
at once. It stays only if something else roots it (a current inventory, or
another root above). Earlier states of P are then restorable only from
local recovery points, which follow the three-state rule of §6.1. The
provider never keeps superseded removal payload as history. The garbage
collector (Q-7) applies its own grace period. A restore that finds payload
already released is `Unavailable` with the reason, never partial.

The GUI advertises a provider-sourced restore (§6.3) **only** for an
intent in Active(P). Other restores are offered only from local recovery
points that exist.

### 2.8 Local records (never uploaded)

| Schema | Fields | Limit |
|---|---|---|
| `wowsync.presence-baseline/1` | `group_id`, `enrollment_id`, `variant_identity`, `mapping`, `addons_dir_id`, `snapshots: [{scan, top_level: [{name, status}]}]` (each kept while referenced, for E6), `packages: [{package_key, folders: [{key, name, tree}], package_hash, scan (references a snapshot), published_seq?}]` (§1.6) | 1 MiB; exceeding it → `NeedsHelp(StorageLow)`, never dropping a referenced snapshot |
| `wowsync.removal-observation/1` | `group_id`, `enrollment_id`, `package_key`, `baseline_scan`, `first_absent_scan` (local monotonic scan counter, not a clock), `scans_absent`, `addons_dir_id`, `state` ∈ `holding \| pending_local_confirmation \| ready \| published`, `intent?`, `reason?` | 64 KiB |
| `wowsync.authority-ledger/1` | §2.4; object copies stored beside it, 64 KiB each | 16 MiB |
| `wowsync.apply-journal/2` | `/1` plus `kind` ∈ `apply \| remove \| restore`, `intent?`, `ack_id?` (fixed before commit so a retried finalize writes the same ack), folder `action` gains `remove` (§4.2), `settings_ops?` (§5) | 1 MiB |
| `wowsync.recovery-point/2` | `/1` plus `kind` ∈ `replaced \| removed \| restore_undo`, `intent?`, `settings?` (settings checkpoint reference, §5), `paired` ∈ `paired \| code_only \| settings_not_applicable` | 64 KiB |

## 3. Conflicts: removal against update and other intents (R-06)

### 3.1 Evaluation

Removal is evaluated per package in `core::target` at cycle step 5
(§1.6), after admission and authority learning. Inputs: the valid ledger
entries for P and Active(P) (§2.6), every admitted candidate for P (local
and members, inventory only, no fallback), group rejections (§8.1), local
observations (§1.6) and the local journals. Rejected versions are treated
as covered: the group already refuses them, so they cannot keep P alive.

**Precedence** per package, first match wins:

1. group rejections (§8.1) exclude versions in every level below;
2. local `Held` (§1.6);
3. an unresolved `conflicted` id for P: `NeedsDecision(InconsistentRemovalRecord)`
   (§2.5). It precedes level 4 because it is actionable: the user can
   resolve it. If `pending` entries also exist, the status says that more
   information is still arriving; nothing is applied either way;
4. `Unavailable(AuthorityPending)` for `pending` entries (§2.5);
5. ledger authority: Active(P) intents and decisions, including competing
   records (table below);
6. the selection constraint of an active keep decision (§3.3);
7. ordinary domain.md §6 ordering, minus the `declined` versions of an
   active decision (§3.3).

A later level never overrides an earlier one. In particular, ordinary
newest ordering (level 7) can never override a decision (levels 5–6).

| Situation (per package P) | Result | Effect |
|---|---|---|
| Observation holding or pending confirmation here | `Held(RemovalObserved \| RemovalConfirmation)` **on this computer only** | nothing for P installed, replaced or removed here (§1.6) |
| A ledger entry for P is `pending` (unreadable, unknown schema, missing reference) | `Unavailable(AuthorityPending)` **on this computer only** | nothing for P applied here until it resolves |
| No active intent and no active decision | unchanged domain.md §6 (minus `declined`) | — |
| Active intent(s); every candidate everywhere is covered or rejected; or there are no candidates | `Remove { intents }` | members holding P remove it when safe (§4); members without P acknowledge `absent` |
| Active intent; some candidate uncovered, not rejected and not `declined` | `NeedsDecision(RemovalVsUpdate)` | **frozen** (§3.2) on every computer |
| Two active intents, neither `knows` the other | union of `covers`; result as above | independent removals of the same package agree; both are acknowledged |
| An active intent and a valid supersession or keep decision for P, neither `knows` the other; or two keep decisions with different chosen versions, neither `knows` the other | `NeedsDecision(CompetingRemovalDecisions)` | frozen |
| Active keep decision (selection constraint, §3.3) | target exactly V | holders of other non-rejected versions replace with V |
| Active keep decision; a candidate W not considered, not declined and not dominated by V | `NeedsDecision(NewVersionAfterDecision)` | frozen; the constraint stays |
| An id for P has two or more variants (different hashes, or a ref that does not match its object, §2.3), not all listed by a valid resolving decision | `NeedsDecision(InconsistentRemovalRecord)` | frozen; diagnostics show every variant and source |
| An active automatic decision; every eligible candidate is `declined` | `NeedsDecision(OnlyDeclinedVersions)` | frozen; holders keep their copies; choices: allow a version again, keep removed |
| Receiver's local package has folders not in `removed` (grouping differs here) | `NeedsDecision(RemovalScopeDiffers)` **on this computer only** | frozen here; others proceed |
| Removing P would break a hard dependency of a remaining local package Q (§4.4) that was not in `dependents_at_origin` | `NeedsDecision(RemovalBreaksDependency)` **on this computer only** | frozen here; others proceed |
| Folder drop where the local tree of the dropped folder is neither listed nor dominated (§1.5) | `NeedsDecision(DroppedFolderDiffers)` | package frozen here |

Group-wide results are computed from the ledger and admitted inventory.
Ledgers converge because they are grow-only unions of the same immutable
objects, so every computer that has learned the same authority reaches the
same group-wide result (R-06 determinism). A computer that knows of an
object it cannot read waits (`AuthorityPending`) instead of acting on less.
Per
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
- The provider objects of every alternative are retention roots (§2.7).
- Other packages proceed (R-06, A-07).
- Status: `NeedsDecision`, shown on every computer that can show it, with
  the alternatives (§7).

### 3.3 Resolving, and the selection constraint (R3D-10)

The user chooses on any member through the GUI (A-26). The choice is an
immutable authority object (§2.2), learned and validated like an intent:

```json
{
  "schema": "wowsync.removal-decision/1",
  "id": "<32 hex>",
  "package_key": "suite-core",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 52 },
  "kind": "removal_vs_update|competing_removal_decisions|inconsistent_removal_record|new_version_after_decision|resume_automatic",
  "considered": {
    "intents": [{ "id": "<32 hex>", "hash": "<object hash>" }],
    "decisions": [{ "id": "<32 hex>", "hash": "<object hash>" }],
    "versions": [{ "package_hash": "<sha256>",
                   "manifest": [{ "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" }] }]
  },
  "chose": { "remove": { "intent": { "id": "<32 hex>", "hash": "<object hash>" } } },
  "declined": ["<package hashes>"],
  "knows": [{ "id": "<32 hex>", "hash": "<object hash>" }],
  "written_unix_ms": 1767225600000
}
```

`chose` is exactly one of `{ "remove": { "intent": <intent ref> } }`,
`{ "keep": { "package_hash": "<sha256>" } }` or
`{ "automatic": { "allow_again": ["<package hashes>"] } }`.

Well-formedness (V6): every `manifest` hashes to its `package_hash`;
`knows` ⊇ `considered.intents` ∪ `considered.decisions`; and `declined` is
exactly the value of the formula for its choice (R3D-16). Let
`inherited` = the union of the `declined` sets of the decisions in
`considered.decisions`, and `seen` = the package hashes of
`considered.versions`:

| Choice | `declined` | Notes |
|---|---|---|
| `keep W` | (`inherited` ∪ `seen`) − {W} | W ∈ `seen`. Choosing a previously declined W removes it from `declined` |
| `automatic` | `inherited` − `allow_again` | `allow_again` ⊆ `inherited`. **Nothing in `seen` is added**: the kept version and any new version are not declined by resuming |
| `remove` | ∅ | a removal ends version preferences; a later re-add is ordinary new intent (§2.6) |

Each formula is total and deterministic, so every computer validates the
same record the same way.

- **Keep removed:** the issuer first writes a new intent with
  `evidence: decision`, `covers` = the union of the old `covers` and the
  manifests of every considered version, `removed` = the issuer's own
  baseline manifest if it held P, otherwise the manifest of a considered
  version (so the intent is restorable, §6.3), and `knows` ⊇ the
  considered variant references (including conflicted ones). Then it writes the decision with `chose.remove.intent`
  naming it. Members holding a considered version remove it into a
  recovery point.
- **Keep version V:** the decision creates a **selection constraint** for P
  (below). It replaces every considered intent (§2.6). Removed members
  install V, and their pinned removal recovery points become ordinary
  history.
- **Resume automatic updates:** ends the constraint. Ordinary domain.md §6
  ordering applies again, except that versions **previously** declined stay
  excluded unless allowed again (Q-50). The version that was kept, and any
  version considered for the first time in this choice, stay eligible. The
  GUI lists the still-declined versions in the consequence text with a
  per-version "allow again" choice, recorded in `chose.automatic.allow_again`.
  Example: D1 keeps V2 and declines V3. Resuming without allowing V3 gives
  `declined = {V3}`: the target is V2 by ordinary ordering, and holders of
  V3 are moved to V2. If V4 (`Greater` than V3) is also present, the target
  is V4. A late replay of V3 stays excluded. If only declined versions are
  eligible, the result is `NeedsDecision(OnlyDeclinedVersions)` (§3.1).

Decision provenance (who, which alternatives, which choice) is the object
itself and is also in the audit log. It is retained with the ledger.

#### Selection constraint

While a valid keep decision D for P is in Active(P) (§2.6) and V is not
group-rejected (§8.1), the group target for P is **exactly V**: the folder
keys and trees of D's chosen manifest. It is not "V or newer".

| Local or candidate version W of P | Result |
|---|---|
| W = V | `InSync`; W is a source candidate for V |
| W ∈ `declined` (including a higher version the user saw and did not choose) | never selected; holders replace W with V (a downgrade replace, W into a recovery point, apply.md §2.2); never triggers a new decision |
| W not considered and `compare_package(V, W)` ∈ {`Greater`, `Same`} (V dominates W) | replaced with V, ordinary |
| W not considered and not dominated by V (W is `Greater` than V, or `Ambiguous`) | **later new information**: `NeedsDecision(NewVersionAfterDecision)` for P, frozen as §3.2. The constraint stays in force; nothing installs W and nothing removes V |
| W is group-rejected | excluded by the rejection; never selected (§8.1) |

Sources for V are admitted candidates whose trees equal V's manifest, or,
if no admitted member currently offers V, D's manifest with the provider
tree records and blobs (retention roots, §2.7), staged and verified like
any apply. V's eligibility predicates (domain.md §6) still apply on each
computer. If V is not eligible here, the result is the usual
`NeedsDecision(reason)` here. There is never a fallback to another
version.

`NewVersionAfterDecision` (Q-49) offers exactly three choices, each a new decision
whose `knows` and `considered.decisions` include D:

- **Keep V** (`chose.keep = V`): W joins `declined`. Ordinary ordering never
  reconsiders W.
- **Take W** (`chose.keep = W`): V joins `declined`. The constraint now
  selects W.
- **Resume automatic updates** (`chose.automatic`).

**Interactions.**

- **Rejection of V** (§8.1) ends the constraint: a rejection is a later,
  explicit statement that V is bad for the group. The rollback design's
  recovered version is then the target. `declined` versions stay excluded
  until a decision allows them. Rejection of a declined version changes
  nothing.
- **Supersession by later authority:** a decision or intent whose `knows`
  includes D replaces D (§2.6). An intent for P that does **not** know D,
  or a second keep decision that does not know D and chose a different
  hash, gives `NeedsDecision(CompetingRemovalDecisions)`. Two decisions
  that do not know each other but chose the same hash agree: the target is
  that hash, and `declined` is the union.
- **Re-add** cannot occur under a constraint, because P is installed (V) on
  every computer where eligible. A user uninstall of V while D is active is
  a new intent. It knows D when the observing computer has learned D;
  otherwise it competes with D.

Settings choices for a re-added or kept package follow the settings design
(R-09). Removal never merges or chooses between settings copies (§5).

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

A member without any folder of P plans nothing. It writes an `absent` ack
object for T (§2.2) and references it.

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
   history state, §6). Prune per §6. Write the `removed` ack object for T
   (write-if-absent, idempotent) and record the report result `removed` in
   the pending report (publication.md §4 durability rules, unchanged).
   `remove_journal`.

Report results gain `removed`. Confirmation of a removal on peer X means:
this computer's ledger has a valid `removed` or `absent` ack object from
X's current enrollment for the intent, **and** X's newest admitted state
lists no folder of the package. Publication of an intent is not
confirmation (A-24).

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
† J:verified † RP(unpinned) † prune † ack object + pending report † remove J
```

Rows added to the apply.md §4.1 argument:

| Crash after | Observed on restart | Recovery |
|---|---|---|
| `J:staging` up to RP write | phase `staging`, all `Untouched` | `rolled_back(from staging)`; cleanup removes the record-only recovery dir; P is re-planned next cycle |
| `J:committing`, any rename boundary | each folder `Untouched` or `Done` | `G`, then rollback §4.3 moves each `Done` back. Re-running after a crash during rollback never repeats a move, because each step is chosen from observation. The removal is retried next cycle (accepted cost, same as apply.md §4.1 item 4) |
| a settings step | per §5.2 table | settings steps have the same observe-then-act form; code rollback runs after settings rollback |
| `J:committed` | all `Done` | verify only, no moves → `verified` or `needs_help` |
| `J:verified`, any finalize step | — | finalize steps are each "skip if done"; the ack object is write-if-absent with a fixed id stored in the journal, so a retry writes the same bytes; journal removal last |
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

Provider payload follows §2.7: only the current removal of P (an intent in
Active(P)) and visible temporary exceptions are kept in the provider.
Superseded removal payload is released, so provider retention never
exceeds what the three-state window and its exceptions promise (R-10).

### 6.2 Temporary exceptions (A-17)

Kept beyond three, each shown as "kept for recovery: N extra copies
(reason)", and returned to three when the reason clears:

| Reason | Kept until |
|---|---|
| removal op unresolved (journal any phase, `needs_help`) | journal cleared or GUI resolution |
| package frozen in a removal decision (§3.2) | decision recorded and applied |
| covered copy removed while another member's copy is uncovered (alternative of a pending decision) | as above |
| restore in progress (§6.3) | restore verified |
| provider payload of every alternative of a frozen package (§2.7) | decision recorded and applied |
| settings checkpoint referenced by a retained code point | that point pruned |

Low storage while an exception exists → `NeedsHelp(StorageLow)` with the
consumption shown. Exceptions are never pruned to make room (R-10).

### 6.3 Restore ("undo removal")

From the package's history (GUI, §7), the user restores a removed package
on this computer:

1. Source: the local recovery point of kind `removed`. If it is absent (this
   computer never had P, or every local point was pruned) **and** the
   intent is in Active(P) (§2.7), the source is
   the intent object's exact `removed` manifest (§2.2) from the ledger: each
   folder's tree record from the provider `trees\`, then its blobs, staged
   and verified exactly like an apply (apply.md §2.2), and the staged
   package hash must equal `removed_hash`. These are retention roots
   (§2.7). If a tree or blob is unreadable: `Unavailable`, with the reason.
   Settings on such a computer are restored only if the intent's settings
   roots name provider-held checkpoints (settings design, §5); otherwise
   step 3's code-only path applies.
2. Op `kind: restore`: install or replace with journal, gate and
   verification as apply.md §2.2. The restored folders' live state before
   restore (normally absent) becomes a recovery point of kind
   `restore_undo`.
3. Settings: the paired checkpoint is restored for matching identities if
   present. Otherwise code-only restore with the post-operation warning
   "Settings were not restored: no saved copy from before the removal"
   (R-10, A-19). Partial and paired restores are reported distinctly.
4. After verification, the presence satisfies the re-add rule (§2.6): this
   computer acknowledged T (`removed`, or `absent` if it never had P), so it
   writes a supersession for T and the package propagates to the group. The GUI states this before the user confirms:
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
| Local hold while uninstall is observed | "Removing *X* from all computers in N s"; that it will not be reinstalled meanwhile (§1.6) | keep it (cancels; reinstalls from the group) |
| Removal vs update decision | per alternative: version label, holders (computers), whether removed where, recovery availability, consequence of each choice | keep removed / keep version V |
| New version after a decision (`NewVersionAfterDecision`) | the recorded decision (who, when, chosen V, declined versions), the new version W and its holders | keep V / take W / resume automatic updates (lists versions that stay declined, each with "allow again") |
| Only declined versions available (`OnlyDeclinedVersions`) | declined versions, who declined them and when | allow a version again / keep removed |
| Inconsistent record (`InconsistentRemovalRecord`) | every variant of the id with its source computers; whether more information is still pending | keep removed / keep version V (the decision lists every variant shown) |
| Kept version (selection constraint active) | "Kept at version V by decision on *computer*"; declined versions | resume automatic updates |
| Waiting for removal information (`AuthorityPending`) | which computer referenced it; that nothing changes for *X* until it is readable | — |
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
- Rejection records and removal authority are independent grow-only sets
  keyed by package. Neither expires the other.
- Rejection of a version chosen by a keep decision ends that selection
  constraint; rejection of a declined version changes nothing (§3.3).

### 8.2 Retirement (R-03, A-45)

The retirement design is a later run. Removal requires these interactions:

- Intents and other authority objects issued by a computer later retired
  **stay valid** (V3, §2.5): its history is preserved and others have
  mirrored the objects (§2.4).
  Retirement is not deletion of the collection. It also does not un-delete
  anything.
- A retirement record carries the retired enrollment's last admitted seq as
  known to the retiring computer (the cutoff) and `authority_known`, the ids
  of that enrollment's authority objects in the retiring computer's ledger.
  States of that enrollment with a higher seq are ignored entirely. An
  authority object of that enrollment is valid only if `issued_at_seq` ≤
  cutoff or its id is in `authority_known` (V3). Delayed delivery of
  pre-retirement history is not proof of renewed membership.
- An object that becomes `invalid(after_retirement)` after it was acted on
  leaves Active(P). Members that removed P keep their recovery points; P
  then follows ordinary rules (an addition if any admitted member holds it).
  The GUI explains this per package (Q-52).
- Settling (§2.7) waits only for current non-retired members. A retired
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
| T2 | Runtime, real cycle loop: P present and published; deleted locally; the loop runs ≥ 4 cycles (the absent inventory is published in the first) and more than three later own states are published and pruned before the hold matures; then restart mid-hold and continue | baseline survives; hold matures; intent object (with `removed` manifest, `covers`, `basis`) and own ack published; P never reinstalled here during the hold (`Held`); receivers remove into recovery | A-10 |
| T3 | Runtime, real cycle loop: P absent in one scan (absent state published), then reappears with the same or new content; and the same across a restart | no intent; hold reset; baseline refreshed; new content is an ordinary update | A-10, A-21 |
| T4 | Folder becomes unreadable / scan unstable / listing unstable | no intent (E2/E3) | A-05, A-20 |
| T5 | AddOns directory identity changed (re-pointed root) with all folders gone | no intent; needs help | A-05, A-36 |
| T6 | ASCII folder absent while a new unsupported (non-ASCII) entry appears; continued over ≥ 4 ordinary cycles in which another package Q (≥ 4 other packages, below E7) is updated and refreshes its baseline, then a restart, then more cycles | no intent; "removed or renamed" notice; user confirmation creates `user_confirmed` intent | A-23 |
| T7 | ≥ `MASS_REMOVAL` packages vanish | held as pending confirmation; nothing published until confirmed | A-10, A-20 |
| T8 | Our own interrupted apply leaves `L` absent (`Half`) | no intent (E5); recovery restores | A-22 |
| T9 | Peer state lacks P, peer newest state unusable or regressed, provider object missing; also an admitted ref whose object is unreadable | nothing removed anywhere; the unreadable ref gives `AuthorityPending` and nothing is installed for P either | A-20 |
| T9b | Receiver learns intent T (ledger), has not acked; then T's issuer's newest state becomes `Unusable`, its provider `authority\` copy is deleted, the receiver restarts, and a late covered copy of P arrives from a third member (reordered deliveries) | T stays active from the ledger; the covered copy is removed when safe; no stale inventory of the issuer is used; the receiver's mirror lets a fourth member learn T | A-20, A-22 |
| T10 | Offline peer updated P (uncovered) then reconnects | `NeedsDecision(RemovalVsUpdate)` on every computer; package frozen; both alternatives preserved and listed; other packages proceed | A-10, A-17, A-07 |
| T11 | Decide "keep removed" then "keep V" (separate runs) | respective propagation; decision provenance in audit and record | A-10, A-25 |
| T11c | D1 keeps V2 and declines V3; then "resume automatic updates" without allow-again, (a) with no new version, (b) with a new V4 `Greater` than V3, (c) then a late replay of V3; also (d) only V3 eligible anywhere | resume record valid with `declined = {V3}` (V2 and V4 not declined); target (a) V2, V3 holders moved to V2, (b) V4, (c) V3 still excluded; (d) `OnlyDeclinedVersions`, nothing changed | A-08, A-20, A-25, A-26 |
| T11b | Alternatives V2 and V3 (V3 `Greater`); user keeps the **lower** V2; then a new V4 (`Greater` than V3) appears after the decision on one member; separately a V1 dominated by V2 appears | V3 holders are downgraded to V2 (V3 into recovery); ordinary ordering never selects V3; V4 gives `NewVersionAfterDecision` with everything frozen and V2 kept; V1 is replaced by V2 without a decision; "keep V2" declines V4; "resume automatic" keeps V3 and V4 excluded unless allowed again | A-10, A-20 |
| T12 | Concurrent "keep removed" and "keep V" decisions on two members | `CompetingRemovalDecisions`; no silent winner | A-20 |
| T13 | Late / duplicate / reordered delivery of intent, ack, supersession and decision objects and of the states referencing them, including refs before objects, objects without refs, restarts between deliveries, and a higher version delivered after a keep decision (property test over permutations, 3 replicas) | identical final result independent of order; `AuthorityPending` only while a referenced object is unreadable; no reversal of a recorded decision; never a higher version overriding a selection constraint | A-20 |
| T14 | Two members independently remove P | union; both acked; no decision | A-20 |
| T15 | Re-add after ack on the same member | supersedes T; propagates as addition | A-10 |
| T16 | Offline member that never acked holds a covered version | removed when safe, not treated as re-add | A-10, A-18 |
| T17 | WoW starts before / between / after removal renames | gate stops; no further change; rollback after exit; no process killed | A-21 |
| T18 | Crash after every effect of the remove op and of its recovery (incl. a second crash during recovery); fakes and real temp filesystem | every restart reaches `Untouched` or `Done`; no lost folder; accurate status | A-22 |
| T19 | Folder recreated in `L` after `L→R` | `Unknown` → `needs_help`; nothing deleted | A-22 |
| T20 | Mixed update op with a folder drop; local dropped tree listed / not listed | removed with the update / `DroppedFolderDiffers` | A-09 |
| T21 | Receiver grouping has an extra folder not in `removed` | `RemovalScopeDiffers` on that computer only | A-09 |
| T22 | Remaining package hard-depends on removed folder, not in / in `dependents_at_origin` | decision here / proceeds with warning | A-09 |
| T23 | More than three history states with a removal among them; prune. Then several settled remove → re-add → update → keep-decision cycles of one package at distinct versions, inspecting the **provider** root set (trees, blobs, settings checkpoints) after each | local: current + two previous retained; `removed` state counts; exceptions visible; low storage → needs help. Provider: removal roots are only the Active(P) intent's manifest and settings, the Active keep decision's manifest and frozen alternatives; every superseded payload is no longer rooted; provider restore is advertised only for an active intent | A-16, A-17 |
| T24 | Restore removed package with local point; without local point (from provider); with pruned provider objects | restored and propagated / staged from provider / `Unavailable` | A-10, A-16 |
| T24b | Restore on a computer that never had P, after every member's local recovery points for P were pruned and the issuer's state files were pruned | restored from the intent manifest, tree records and blobs; package hash equals `removed_hash`; code-only settings warning; supersession written; propagates | A-10, A-16, A-19 |
| T25 | Restore when settings checkpoint missing | code-only restore, current settings preserved, distinct warning (settings phase) | A-19 |
| T26 | Remove with settings owned / unknown-shared / paused (settings phase) | paired / code-only with diagnosis / kept with notice; WTF native files untouched | A-09, A-10, A-15 |
| T27 | Rejected release on a late peer for a removed package | covered; no decision; not reinstalled | A-18 |
| T28 | Retired member: object with `issued_at_seq` above the cutoff and not in `authority_known`; one within the cutoff learned before retirement; one learned after retirement arrives late | above-cutoff object `invalid(after_retirement)` and ignored (also when learned before the retirement record arrived); within-cutoff objects stay valid after the issuer's states become unusable; settling does not wait for the retired member | A-45 |
| T29 | Joining computer with a covered / uncovered copy of a removed addon | listed in consent and removed / decision | A-04 |
| T30 | `code-state/2` read by a `/1` reader, and `/1` state read by `/2` reader | `/1` reader shows "written by a newer version" and does nothing; `/2` reader treats `/1` absence as absence (never removal) | A-36, A-20 |
| T31 | Same intent id with different content | `InconsistentRemovalRecord`; nothing removed | A-23 |
| T31b | Two variants of intent T on 3 replicas → GUI decision (each of: keep removed, keep version V) listing both variants → restart → late replay of both variants from other members → then a third variant appears | decision is `valid` while the variants stay `conflicted`; status precedence shows the decision, not `AuthorityPending`; after the decision all replicas converge on the chosen result and stay there across restart and replay; the third variant reopens `InconsistentRemovalRecord` | A-20, A-23, A-25, A-26 |
| T34 | Mixed arrivals for one intent: ref first then object; object from a mirror; ref with matching id but a different hash, kind or package key; the same object from three members | equal-hash copies are one record; any mismatch → `InconsistentRemovalRecord`; no false inconsistency from verbatim mirrors | A-20, A-23 |
| T35 | Settling: all current members acked | status "settled"; objects unchanged byte for byte; a late member with a covered copy still has it removed | A-10, A-20 |
| T36 | Re-add supersession without an earlier own ack, or with an ack issued later than the supersession | `invalid(readd_without_ack)`; intent stays active | A-10, A-20 |
| T37 | E7 mass removal pending confirmation for a long time while peers keep publishing P | P stays `Held(RemovalConfirmation)`; never reinstalled until "keep" is chosen | A-10 |
| T32 | GUI: remove, see pending/confirmed per computer, resolve a removal decision, restore — without a terminal, keyboard only | flows complete; wording does not claim "removed everywhere" early | A-25, A-26, A-24 |
| T33 | Installed-app removal through real Dropbox on two computers (provider tier, after owner Check 2) | removal applied and confirmed; provenance retained | A-34 |

A-38 negative controls for at least T2, T3, T6, T9, T9b, T10, T11b, T11c,
T13, T16, T18, T23, T24b, T31b and T34 must show a named test failing (exit 101) when the
corresponding rule is removed, for example by treating absence as removal,
dropping the coverage check, reading E1 from the newest published state,
omitting `Held`, recomputing active intents from latest admitted states
instead of the ledger, hashing a compact form, dropping the `removed`
manifest, letting ordinary newest ordering override a keep decision,
comparing E6 with a shared latest snapshot, requiring referenced variants
to be valid, rooting superseded payload, or deriving `declined` for
`automatic` from the considered versions.

## 10. Changes to existing design documents when implemented

Recorded here because this packet does not own those files.

| Document | Change |
|---|---|
| [domain.md](domain.md) §6 | Selection runs below the removal precedence of §3.1: rejections, `Held`/`AuthorityPending`, ledger authority, selection constraint, then ordinary ordering minus `declined` versions. **Supersedes domain.md §6 Q-2 when implemented:** completeness is "group-wide membership minus the candidate's evidenced `dropped` folders" (§1.5). The final sentence "There is no removal record in the slice" is replaced by: "Absence never removes; only an active intent in the authority ledger (removal.md §2–§3) yields `Remove`, and an uncovered candidate under an intent yields `NeedsDecision(RemovalVsUpdate)`." New `DecisionReason` values: `RemovalVsUpdate`, `CompetingRemovalDecisions`, `InconsistentRemovalRecord`, `NewVersionAfterDecision`, `OnlyDeclinedVersions`, `RemovalScopeDiffers`, `RemovalBreaksDependency`, `DroppedFolderDiffers`. New target states `Held(RemovalObserved \| RemovalConfirmation)` and `Unavailable(AuthorityPending)` |
| [domain.md](domain.md) §1 step 5 | Consent text changes from "Addons missing here are installed, never treated as removed" to also state that addons deliberately uninstalled on one computer are removed on the others and kept for recovery, and that settings effects follow §5 |
| [publication.md](publication.md) §1 | layout gains `members\<computer_id>\authority\<id>.json` (single writer, write-if-absent, never deleted while the group exists; §2.1). Readers list it for every member including unusable and retired ones; that is authority, not an inventory fallback |
| [publication.md](publication.md) §2 | `wowsync.code-state/2` with `authority_refs` and `progress` (§2.3); object schemas `wowsync.removal-intent/1`, `removal-decision/1`, `removal-supersession/1`, `removal-ack/1` (§2.2); local schemas of §2.8 (`presence-baseline/1`, `authority-ledger/1`); report result `removed`; confirmation rule of §4.2 |
| [publication.md](publication.md) §4 | publish writes and reads back new authority objects before the state that references them; step 7 pruning never touches `authority\` |
| [publication.md](publication.md) §6 | admission unchanged; add the authority learning and validation of §2.4–§2.5 as a separate step whose output is the ledger |
| [apply.md](apply.md) §1 | new cycle steps 2a (authority learning) and 3a (presence baseline and removal observation) as in §1.6; step 5 uses the precedence of §3.1 |
| [apply.md](apply.md) §2.2, §3.1, §4.1 | remove op, remove slot table, crash rows (§4.3); "Folders of the package present locally but absent from the target cannot occur in a plan" is replaced by the folder-drop rule |
| [apply.md](apply.md) §2.4 | pruning counts history states including `removed` (§6.1) |
| [apply.md](apply.md) §6 | statuses `RemovalHeld`, `RemovalPendingHere`, `RemovedHere`, `RemovalConfirmedBy(ids)`, `AuthorityPending`, `KeptByDecision` |
| `core::target`, `core::plan`, `core::exec`, `core::status` | new pure `core::removal` module (observe, activity, coverage, settle); one implementation shared by runtime and tests (A-43) |
