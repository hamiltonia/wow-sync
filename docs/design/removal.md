# Addon removal and uninstall across the group

Status: **run-004 rewrite** (packet
[R4-removal-design](../runs/run-004/packets/R4-removal-design.md)) for owner
decisions D-21 ("removed means removed everywhere") and D-23 ("any removal
wins; conflicting removal records merge into one removal") in
[OWNER-DECISIONS.md](../OWNER-DECISIONS.md). It replaces the run-003 design
in full. **Not implemented.** It must pass the independent run-004 design
review (R4-design-review) before phase C implements it. Nothing here changes
the running slice, whose rule "absence never removes" stays the safety base.

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
| Random object `id` plus content hash; a `conflicted` status when one id had two hashes; `InconsistentRemovalRecord` decision (R3D-13) | **Removed.** A record's identity **is** the SHA-256 of its exact bytes, which is also its file name (§2.2). Two different contents are two records, both counted. "Same id, different content" cannot be expressed |
| Records scoped by one `package_key`, validation V7 "same package" | **Replaced.** Records name **folder keys**. Merging and superseding are evaluated per folder key (§3), so records with different package sets or spanning packages merge without any package-scope rule. New V7 checks only listing well-formedness (§2.6) |
| Coverage (`covers`, `basis`), "uncovered" copies, `NeedsDecision(RemovalVsUpdate)` | **Removed** (D-21). Every copy is removed; the copy goes to recovery and is named (§5) |
| Removal decisions, `chose`/`declined`, selection constraint, `NewVersionAfterDecision`, `OnlyDeclinedVersions`, `CompetingRemovalDecisions` (R3D-10, R3D-16) | **Removed.** Removal never chooses a version. Pinning or rejecting a version is rollback (`rollback.md`, written by R4-settings-design) |
| `RemovalScopeDiffers`, `RemovalBreaksDependency`, `DroppedFolderDiffers` per-computer decisions | **Removed.** Named folders are removed; unnamed folders stay; broken hard dependencies are a warning (§5.4, Q-54, Q-58) |
| E7 mass-removal confirmation (Q-42) | **Removed** (D-21 rejected Q-42) |
| `Unavailable(AuthorityPending)` for a whole package | **Narrowed.** Unreadable referenced information only defers **installs** of the folders it names, and the GUI can dismiss it (§4.4) |
| `wowsync.code-state/2` new major (Q-40) | **Replaced** by additive `serde(default)` fields in `code-state/1` (§11, Q-56) |

Kept, with their run-003 review fixes: the presence baseline with
per-package snapshots (R3D-7, R3D-14), the settle wait (Q-41), the durable
grow-only ledger with mirroring on learning (R3D-8, Q-51), immutable hashed
records (R3D-9), bounded payload retention (R3D-15, Q-53), the remove
operation with its crash table, code **and** settings removal, the
three-state history, GUI restore, the WoW gate and the retirement cutoff.

Vocabulary:

| Term | Meaning |
|---|---|
| package | the addon unit of [domain.md](domain.md) §5: folders grouped together. Packages are a **local** grouping; records do not depend on it |
| folder key | `paths::FolderKey`, the case-folded folder name. The unit of removal |
| removal record | an immutable, content-addressed group record: "this computer deliberately removed these folders, as these trees" (§2.3). The tombstone |
| re-add record | an immutable record: "this computer explicitly brought these folders back", superseding named removal records (§2.3, §3.5) |
| ack | an immutable record: "this enrollment applied these removals" (`removed`) or "had none of their folders" (`absent`) (§2.3) |
| ledger | this computer's durable, grow-only, validated set of learned records (§2.5) |
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
| Provider offline or stale on a receiver | a receiver that has not learned the record changes nothing for it; a record it has learned stays in its ledger (§2.5) |
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
operation (§5.1). If the author later ships X again, the reinstall is
observed as a re-add (§1.7).

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
  package hash, the local scan counter, and a reference to that scan's
  **top-level snapshot** (every top-level name with its status).
- **Snapshots** are kept while at least one package entry references them.
  Refreshing package Q never replaces the snapshot that a still-absent
  package P references (R3D-14).
- **Persisted** with temp + flush + rename before the cycle publishes.
- **Refreshed** when P is again fully present. A refresh replaces the folder
  list; it never merges absent folders in.
- **Cleared** for P when P's removal record and own ack are written, when
  a folder drop is recorded (for those folders), when "Keep it" is chosen
  for an unclear observation and P is present again, or when P is removed
  here by the merged removal. The **whole** baseline is discarded on a new
  enrollment, a variant change or a directory-identity change.
- **Not** written from a partial or unstable listing, from a scan during
  which a journal existed, or for a package with any non-`ok` folder.

**Cycle.** New steps 2a, 3a and 3b extend apply.md §1:

```text
1  recover_interrupted
2  admit member states; persist watermarks
2a learn records into the ledger (§2.5); persist; validate (§2.6);
   mirror newly valid records into own authority\; repair own authority\
3  scan AddOns → build_inventory   (listing unstable → stop, as today)
3a observe: update/persist baselines and observations (E1–E8, §1.7);
   a matured observation writes its record (+ own ack) and ledger entry
3b acknowledge: write `absent` acks that are due (§5.6)
4  publish (inventory, authority_refs, features) if anything changed;
   every record a state references is written and read back first
5  merge_removals (§3) → compute_target (precedence §4.1)
6  activity gate → plan_apply (removes, installs, replaces)
7  run_apply (a remove finalize writes its `removed` ack); rescan; 3a, 3b, 4 again
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

### 1.7 Reinstall observation: an explicit re-add by the user

After this computer applied a removal (it holds a valid own `removed` or
`absent` ack for every live removal naming the folders), the merged removal
would remove any copy that appears again. A user who reinstalls the addon
with an updater or Explorer is therefore observed, not fought:

- If any folder in the merged removal becomes present here and E3 and E5
  hold, the local target for its package is `Held(ReinstallObserved)`.
  Nothing is removed during the hold.
- After the same settle rule as E4 (two complete stable scans at least 30 s
  apart; any disappearance resets), this computer writes a re-add record
  with `evidence: observed_reinstall` (§2.3) that supersedes the live
  removals of those folders, and publishes the package present.
- If this computer does **not** hold own acks for those removals (it never
  applied them, for example an offline peer), a present copy is not a
  re-add. It is removed into recovery (§3.4). This is the line between an
  explicit re-add and an offline peer resurrecting an addon (R-06, R-10).

Whether an observed reinstall counts as explicit is Q-55 (default: yes,
otherwise the app would remove the addon the user just reinstalled, in a
loop with the updater). The GUI restore (§7.4) is the other explicit re-add.

## 2. Records and the ledger

### 2.1 Where they live

```text
groups\<group_id>\members\<computer_id>\
  member.json                          unchanged
  state\<enrollment_id>-<seq:010>.json wowsync.code-state/1 (+ authority_refs, §2.4)
  authority\<sha256 hex>.json          immutable records written by this computer:
                                       its own and verbatim mirrors of learned ones
```

- Single writer per member directory, unchanged (publication.md §1). A
  record file is written **write-if-absent** with temp + flush + rename, then
  read back and hash-checked before any state that references it is
  published. It is never rewritten or deleted while the group exists (Q-44).
- A reader lists `authority\` of **every** member directory of the group,
  including members whose newest state is `Unusable` or `Regressed`, and
  retired members. Records are immutable facts validated one by one (§2.6),
  not "the last good inventory". Inventory (code candidates) still comes
  only from admitted states (publication.md §6 no-fallback rule).

### 2.2 Identity: the content hash

A record's identity is `SHA-256(bytes)` over the **exact stored bytes**.
The file name is that hash in lowercase hex plus `.json`. Readers decode only
a file whose bytes hash to its name, like tree records (`records::decode_tree`).
A file whose bytes do not match its name is a **bad copy**: it is ignored
with a diagnostic, and another copy with the same name may still be good. It
is never a record and never a conflict.

Writers produce bytes with `records::encode` (deterministic). Mirrors copy
bytes verbatim, so every copy has the same name and hash. Two records with
different content are simply two records. Two records with identical bytes
are one record. There is no separate `id`, no compact form with its own
hash, and nothing is ever recomputed or rewritten (R3D-9).

### 2.3 Record kinds

Common fields: `schema`, `group_id`, `issuer {computer_id, enrollment_id,
issued_at_seq}` and `written_unix_ms` (display only, never ordering).
`issued_at_seq` is the seq of the first own state that references the
record; the issuer reserves it before writing.

**Removal record** (`wowsync.removal/1`, the tombstone):

```json
{
  "schema": "wowsync.removal/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 39 },
  "evidence": "observed|user_action|user_confirmed|folder_drop",
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
  lists only the dropped folders.
- `title` is display text from the selected toc, untrusted: rendered as plain
  text with colour codes stripped (`detail::strip_colour_codes`).
- `audience` is the set of current, non-retired member enrollments the
  issuer had admitted, plus itself (§3.4).
- `settings` is `{ "kind": "none" }` (settings not handled by this record)
  or `{ "kind": "checkpoints", "roots": [{ "identity": "…", "checkpoint":
  "<content hash>", "location": "local|provider" }] }` from the settings
  design (§6).

**Re-add record** (`wowsync.readd/1`):

```json
{
  "schema": "wowsync.readd/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 57 },
  "evidence": "user_restore|observed_reinstall",
  "title": "Suite",
  "folders": [ { "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" } ],
  "present_hash": "<package hash of folders>",
  "supersedes": [ "<removal record hash>" ],
  "acks": [ "<own ack hash>" ],
  "written_unix_ms": 1767225600000
}
```

`supersedes` lists every live removal (§3.1) naming any of `folders` in the
issuer's ledger. `acks` is required for `observed_reinstall` (V5) and empty
for `user_restore`.

**Ack** (`wowsync.removal-ack/1`):

```json
{
  "schema": "wowsync.removal-ack/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 41 },
  "removals": [ "<removal record hash>" ],
  "result": "removed|absent",
  "removed": [ { "key": "suite-core", "name": "Suite-Core", "tree": "<sha256>" } ],
  "settings": "paired|code_only|kept_paused|not_applicable",
  "written_unix_ms": 1767225600000
}
```

`removed` is the manifest this computer moved to recovery (empty for
`absent`). It names, for example, an offline peer's updated copy, so the
summary can say "Laptop's copy (v2.1) was kept in recovery on Laptop"
(A-10). The issuer of a removal writes its own `removed` ack with its record.

Schemas are open for extension: unknown fields are ignored within a major
(Q-13); an unknown major is `pending(unknown_schema)` (§2.6).

### 2.4 References in the state

`wowsync.code-state/1` gains two additive fields (§11):

```json
{
  "authority_refs": [
    { "hash": "<sha256>", "kind": "removal|readd|ack",
      "folders": ["suite-core", "suite_data"] }
  ],
  "features": ["removal/1"]
}
```

- `authority_refs` lists every **valid** ledger record of this computer,
  its own and mirrored. It is grow-only per enrollment. A reader
  union-merges refs into its ledger and ignores omissions.
- `folders` is a **routing hint**: the folder keys the record names (for an
  ack, the union of its removals' folders). It is used only to scope a
  deferral while the record is unreadable (§4.4). When the record is read,
  a ref whose `kind` or `folders` differs from the record is a **mismatched
  ref**: it is ignored with a diagnostic. The record stands on its own.
  There is no second hash to keep consistent.
- `features` tells peers this app applies removals. A member without
  `removal/1` is shown as "runs an older WoW Sync that does not apply
  removals" (§9) and never counts as confirmed (§5.6).
- Size: a ref is about 150 bytes; 10,000 records add about 1.5 MB to a
  state, within the 16 MiB state limit. Reviewed with provider GC (Q-7).

### 2.5 The ledger (local, durable)

`wowsync.authority-ledger/1` is this enrollment's durable, grow-only set of
learned records:

| Field per entry | Meaning |
|---|---|
| `hash`, `kind`, `folders` | identity, kind and named folder keys |
| `bytes_path` | local verbatim copy under the app data `authority\` directory |
| `issuer` | from the record |
| `learned_from` | `[{computer_id}]`, every member directory or ref it came from |
| `validity` | `valid` \| `pending(reason)` \| `invalid(reason)`, re-derived (§2.6) |
| `mirrored_seq` | the own seq at which this computer first referenced it, or `null` |

The ledger file also holds `dismissed: [hash]`, the local "Continue without
it" choices (§4.4). They are never published.

Learning, at cycle step 2a:

1. Every `authority_refs` entry of every admitted state that is not in the
   ledger is looked up in the referencing member's `authority\`, then in any
   member's. If no good copy is readable (missing, cloud-only, bad copy,
   oversized), the entry is recorded as `pending(unreadable)` with the ref's
   kind and folder hint.
2. Every member's `authority\` listing is read, and each unknown name is
   read and hash-checked.
3. Each new record is copied locally (temp + flush + rename) and the ledger
   entry is persisted **before** any target is computed from it. Then
   validity is derived.
4. Each newly valid record is mirrored verbatim into this computer's own
   `authority\` and listed in the next own state (Q-51). A removal therefore
   survives its issuer's directory, newest state or enrollment becoming
   unusable.
5. **Repair:** before publishing, every record that the new state will
   reference is checked in the own `authority\` (stat, then hash on
   mismatch) and rewritten from the local copy if missing or bad
   (write-if-absent, same bytes). A deleted provider copy therefore heals
   while any member that references the record is active.

Entries are never deleted. A restart reloads the ledger before the first
target computation. Provider absence of a record is never a reason to forget
it.

### 2.6 Validation

An entry is `valid` only if all hold:

| # | Rule | Otherwise |
|---|---|---|
| V1 | **Bytes.** A good copy is readable: its bytes hash to its name, it is at most 64 KiB, it parses, and its schema major is known | no good copy → `pending(unreadable)`; unknown major → `pending(unknown_schema)` |
| V2 | **Issuer.** `group_id` is this group, and `issuer.computer_id`/`enrollment_id` is a member enrollment of the group (`member.json`), current or retired | `invalid(not_a_member)` |
| V3 | **Retirement.** If the issuer enrollment is retired, `issued_at_seq` ≤ the retirement cutoff, or the hash is in the retirement record's `authority_known` (§8.2) | `invalid(after_retirement)` |
| V4 | **References resolve.** Every hash in `supersedes`, `acks` and `removals` is in the ledger (any validity) and has the expected kind (`removal` for `supersedes`/`removals`, `ack` for `acks`) | not yet in the ledger → `pending(missing_reference)`; wrong kind → `invalid(wrong_reference)` |
| V5 | **Re-add evidence.** For `observed_reinstall`: every listed ack is valid, from the **same enrollment**, `result` `removed` or `absent`, `issued_at_seq` lower than the re-add's, and the acks' `removals` together include every hash in `supersedes`. For `user_restore`: no ack is required (the GUI action is the explicit choice) | `invalid(readd_without_ack)` |
| V6 | **Ack consistency.** `removals` is non-empty. `result: absent` has an empty `removed`; `result: removed` has a non-empty `removed` | `invalid(malformed_ack)` |
| V7 | **Listing.** `folders` (removal, re-add) is non-empty, sorted by key, without duplicates, and every `name` validates to its `key` (`paths::validate_folder_name`). Every hash list is sorted and unique. Each superseded removal shares at least one folder key with the re-add. Every `removed` key of an ack is named by one of its removals. No rule compares package keys | `invalid(malformed_listing)` |

There is **no** `conflicted` status. Validity is re-derived when a member or
retirement record changes and when a referenced entry arrives, so
`invalid(after_retirement)` can follow `valid` (§8.2).

| Status | Effect |
|---|---|
| `valid` | counts in the merge (§3) |
| `pending(reason)` | counts as **not known** in the merge. A pending removal additionally defers installs and replacements of the folders in its hint on this computer (§4.4). Resolves automatically when it becomes valid or invalid |
| `invalid(reason)` | ignored; diagnostic only; never blocks |

V7 and the merge are consistent by construction: `knows`-style "list every
related record" rules do not exist, `supersedes` is required to overlap the
re-add's folders (V7) and is written from exactly the live removals that
overlap them (§2.3). A writer can therefore always produce a record that
passes V7, whatever packages or folder sets the related records span (§13).

## 3. The merged removal (D-23)

### 3.1 Definitions

All sets are taken over **valid** ledger entries only. For folder key x:

- `Rem(x)` = removal records whose `folders` name x.
- `Bnd(x)` = the **re-add boundary** of x: the union of `supersedes` over
  every re-add record whose `folders` name x.
- `Live(x)` = `Rem(x) − Bnd(x)`.
- x is **removed** iff `Live(x)` is non-empty.

The **merged removal** is `M = { x : Live(x) ≠ ∅ }`, with provenance
`Live(x)` per folder. For display, M is split into **removal units**: the
connected components of records in `⋃ Live(x)` linked by shared folder keys,
each shown with the titles of its records.

"Newest re-add boundary" is defined causally, never by a clock: a re-add can
only supersede removals that its issuer had already learned, so the boundary
of x is exactly the set of removals that some explicit re-add came after.
Re-adds never shrink it. A removal written after a re-add, or concurrently
with it (its issuer had not learned the re-add, and the re-add's issuer had
not learned it), is not in the boundary and wins (D-21: any removal wins).

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
| 4 | r1 {core}; later C: a1 user_restore {core} supersedes r1 | {} | explicit re-add; propagates as an addition (§3.5) |
| 5 | r1 {core}; a1 supersedes r1; later B (after learning a1): r2 {core} | {core} | a removal after a re-add wins |
| 6 | r1 {core}; a1 supersedes r1; concurrently D (not knowing a1): r3 {core} | {core} | a concurrent removal wins; the re-adder sees "removed again by D" and may restore again (a2 supersedes r3) |
| 7 | r1 {core, data}; a1 {core} supersedes r1 | {data} | per-folder boundary |
| 8 | r1 {core}; a1 observed_reinstall by C without an own ack | {core} | a1 invalid (V5); C's copy is removed into recovery (§3.4) |

### 3.4 Stale copies and the audience

A copy that existed before a removal and was never removed by it must not
come back through a later re-add. Otherwise the result would depend on
delivery order: a peer that learns the removal before the re-add removes its
copy, but a peer that learns both at once would keep it and could win as
"newest".

Member m **owes** removal r for folder x iff x ∈ `r.folders`, m's enrollment
is in `r.audience`, and the ledger holds neither a valid ack from m's
enrollment listing r nor a valid re-add from m's enrollment superseding r.
Then:

- m's copy of x is **never a candidate** for any computer (§4.1), even when
  x is no longer in M;
- on m itself, a live copy of x that m owes for is removed into recovery
  (§4.1 level 4), and m then acks; afterwards the re-added version is an
  ordinary addition there.

Example: C (in r1's audience) was offline with an updated v2 of `core`.
Whether C learns r1 first and a1 later, or both together, C ends with v2 in
recovery (named in the summary) and A's re-added version installed. The
result is the same either way.

A computer that joined after r was written is not in `r.audience` and owes
nothing for it: its local copy is an ordinary existing addon (A-04) once r is
superseded. While r is live, the copy is removed anyway (removal wins, Q-48).

A member in the audience that never had x writes an `absent` ack at step 3b
**before** it may install x (§5.6), so it never owes for a folder it holds.

### 3.5 Re-add rules

- A re-add is **explicit**: a GUI restore or add-back (`user_restore`,
  §7.4) or an observed reinstall after this computer applied the removal
  (`observed_reinstall`, §1.7, Q-55).
- The re-add supersedes exactly the live removals naming its folders in its
  issuer's ledger. Folders it does not name stay removed (example 7).
- After the re-add, x is not in M, and the re-added folders propagate as an
  ordinary addition among non-owing holders ([domain.md](domain.md) §6).
  Later genuine updates by non-owing holders propagate normally.
- Uninstalling again writes a new removal, which is not superseded by any
  earlier re-add (example 5).
- An offline peer cannot re-add by being present: without its own ack its
  presence is a stale copy (§3.4).

## 4. Target per package

### 4.1 Precedence

Removal is evaluated in `core::target` at cycle step 5, after authority
learning and admission. Inputs: the merged removal M with provenance, the
owing relation, the local holds and observations, the pending hints, the
local journals, group rejections (rollback.md) and every admitted candidate.

Before grouping ([domain.md](domain.md) §5) and candidate selection (§6):

- every folder in M is removed from every observation, including the local
  one, so group-wide membership and completeness exclude it (§1.5);
- every folder a member owes for is removed from that member's observation.

The **remove** decision is made on the unstripped local grouping, so the
removed folders of one local package form one operation.

Per local package P, first match wins:

1. **Rejections** (rollback.md) exclude rejected versions from every level
   below.
2. **Local hold:** `Held(RemovalObserved | RemovalUnclear |
   ReinstallObserved)` for P. Nothing for P is applied here.
3. **Merged removal:** some live local folder of P is in M →
   `Remove { folders: live(P) ∩ M, removals: ⋃ Live(x), reason: Removed }`.
4. **Stale copy:** some live local folder x of P is not in M but this
   computer owes for x → `Remove { folders, removals, reason: StaleCopy }`.
5. **Pending removal information:** a folder of P that would be installed or
   replaced is in the hint of a pending removal that is not dismissed →
   `Unavailable(WaitingForRemovalInfo)` for P on this computer (§4.4).
   Removals at levels 3–4 are never deferred by this level.
6. **Ordinary** [domain.md](domain.md) §6 selection over the stripped
   observations.

When levels 3 or 4 match, folders of P outside M stay and are evaluated by
level 6 in the next cycle as their own (stripped) package.

### 4.2 Results

| Situation (local package P) | Result | Effect |
|---|---|---|
| Observation holding or unclear here | `Held(RemovalObserved \| RemovalUnclear)` here only | nothing for P applied here (§1.6) |
| Reinstall observed after own ack | `Held(ReinstallObserved)` here only | not removed during the hold (§1.7) |
| Live local folders of P in M | `Remove { reason: Removed }` | removed into recovery when safe (§5); `removed` ack |
| P not held here, some folders of P in M | nothing to remove | `absent` ack (§5.6) |
| Local copy owed after the removal was superseded | `Remove { reason: StaleCopy }` | removed into recovery; ack; then ordinary addition |
| Install/replace blocked by a pending removal hint | `Unavailable(WaitingForRemovalInfo)` here only | wait, or "Continue without it" (§4.4) |
| Folders of P remain after removal and hard-depend on a removed folder | ordinary result plus `DependencyRemoved` warning | named in the summary (§5.4, Q-54) |
| None of the above | ordinary [domain.md](domain.md) §6 | — |

No removal situation produces `NeedsDecision`. `DecisionReason` gains no
values.

### 4.3 What happened to the run-003 removal decisions

| Run-003 result | Run-004 behaviour |
|---|---|
| `RemovalVsUpdate` | removal wins; the update goes to recovery and is named (D-21) |
| `CompetingRemovalDecisions` | no decisions exist; removal versus re-add is settled by `supersedes` (§3.1) |
| `InconsistentRemovalRecord` | cannot occur: identity is the content hash (§2.2, §13) |
| `NewVersionAfterDecision`, `OnlyDeclinedVersions`, selection constraint | no version choice in removal; version pinning and rejection are rollback.md |
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
replacements of the folders in its hint on this computer (level 5). It never
defers removals and never affects other folders.

It resolves by itself when a good copy is learned (any member's mirror) or
when the referencing member repairs its copy (§2.5 step 5). If it does not,
the GUI shows "Waiting for removal information from *Laptop* about *X*"
with **Continue without it** (§9). That adds the hash to the local
`dismissed` set: the record is then treated as unknown, exactly like a
record not published yet. If it becomes readable later, it is learned and
counts normally. "Continue without it" is local, never published, and
needs no terminal (Q-57).

## 5. Apply on receivers (R-11)

### 5.1 Plan

`plan_apply` gains a remove operation: `PackageApply` with `kind:
Remove`, `removals: Vec<Sha256>`, `reason`, folders with `FolderAction::Remove
{ expected_live: Tree{tree, fingerprint} }`, and `settings:
SettingsRemovalPlan` (§6). It is planned only when:

- the target is `Remove` for P;
- every live folder to remove is stable and readable (`ok`; a non-`ok`
  folder waits and is shown);
- no journal exists (Q-25, unchanged; R3D-6: the GUI never promises a
  removal while a journal blocks);
- `Activity::NotRunning` (R-11);
- the settings plan (§6) is derivable, or settings are not applicable.

When the same cycle also plans an update of P's remaining folders (for
example a folder drop, §1.5), the planner merges both into one operation
whose journal mixes `replace`, `install` and `remove` folders. Each folder
follows its own slot table, so the one-op crash argument holds per folder.

### 5.2 Algorithm

One package per operation, as in apply.md §2.2. There is no staging for
code. Live folders move to recovery by same-volume rename. Nothing is
deleted.

1. **Journal `staging`** (`kind: remove`, every folder `action: remove`,
   `target_tree` = the tree being removed). The ack hash is not fixed yet;
   the ack bytes are fixed in step 7a.
2. **Destination revalidation** (apply.md §2.2 step 2). Any difference →
   `Replanned(DestinationChanged)`; journal → `rolled_back`; nothing touched.
3. **Recovery point** `kind: removed`, `pinned: true`, `removals`, the
   folders as `replaced`, `installed: []`, settings reference (§6) →
   **journal `committing`**.
4. **Commit**, per folder in name order: `G` (WoW gate); fingerprint
   unchanged; `move(L→R)`. Then the settings steps (§6), each after `G`.
5. **Journal `committed`.**
6. **Verify:** `G`; every folder `Done` (`L` absent, `content(R)` equals the
   expected tree). Settings verified (§6). → **journal `verified`**, else
   `needs_help`.
7. **Finalize** (each step "skip if done"):
   a. encode the ack (`result: removed`, `removed` manifest, settings result)
      and store its bytes and hash in the journal (so a retry writes the same
      bytes);
   b. write the ack record (write-if-absent) and its ledger entry;
   c. recovery point `pinned: false` (an ordinary history state, §7);
      prune per §7;
   d. record report result `removed` in the pending report;
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
J:staging(remove) † [revalidate] † RP(removed, pinned) † J:committing
† f1: L→R † f2: L→R † … † [settings steps] † J:committed † [verify]
† J:verified † J(ack bytes) † ack record † ledger † RP(unpinned) † prune
† pending report † remove J
```

| Crash after | Observed on restart | Recovery |
|---|---|---|
| `J:staging` up to the RP write | phase `staging`, all `Untouched` | `rolled_back(from staging)`; cleanup removes the record-only recovery dir; re-planned next cycle |
| `J:committing`, any rename boundary | each folder `Untouched` or `Done` | `G`, then rollback moves each `Done` back. Each step is chosen from observation, so a second crash during rollback never repeats a move. Retried next cycle |
| a settings step | per the settings slot table (§6) | settings rollback first, then code rollback |
| `J:committed` | all `Done` | verify only → `verified` or `needs_help` |
| `J:verified`, any finalize step | — | finalize steps are idempotent; the ack bytes are in the journal; journal removal is last |
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

- **`removed` ack:** written in finalize (§5.2 step 7) for every live and
  owed removal of the folders it removed.
- **`absent` ack:** at step 3b, for every valid removal r whose audience
  includes this enrollment, which names no folder present here, and which
  this enrollment has not acked or superseded. Written before any install of
  r's folders is planned. Acks are batched: one ack per removal unit per
  cycle.
- **Confirmation** of a removal on peer X: this computer's ledger has a
  valid `removed` or `absent` ack from X's current enrollment listing the
  removal, **and** X's newest admitted state lists none of its live folders,
  **and** X advertises `removal/1`. Publication of a removal is not
  confirmation (A-24).
- **Settled** (derived, display only): every current, non-retired
  enrollment in the audience has acked or superseded the removal. No record
  is rewritten when a removal settles. A settled removal stays authority: a
  late or re-enrolled member's copy is still removed (§8).

## 6. Settings for removed addons

SPEC R-04 and A-10: an uninstall removes the active code **and** its
applicable saved settings; recovery keeps and restores both. The settings
design is written in run-004 (`settings.md`, packet R4-settings-design).
Removal relies on the contract below; the settings design provides it.

**Q-45 (D-21):** WoW keeps an uninstalled addon's saved variables. So WoW
Sync moves the owned saved-settings items itself, and WoW does not recreate
them while the addon is not installed.

### 6.1 Contract (core, pure; adapters follow apply.md's port-trait pattern)

| Operation | Contract |
|---|---|
| `settings::owned_scope(variant, folders, identities) -> SettingsScope` | Pure. For the removed folders, the saved-settings items owned by them per matched account/character identity, plus `unknown_or_shared: Vec<Diagnosis>`. Ownership comes from the folders' own declarations (for example `SavedVariables`), parsed without executing Lua (R-11). WoW's own UI/cloud files are never items (R-07) |
| `settings::checkpoint(scope) -> CheckpointRef` | Content-addressed copy of the owned items **before** removal, scoped by `(GroupId, product, identity)`. Retained with its recovery point |
| `SettingsRemovalPlan` | Per item: `expected_live` (hash + fingerprint), same-volume move to `<variant>\.wowsync\recovery\…\settings\…`, `G` before each move. Never an in-place edit; never a split of a shared file. Slot configurations `Untouched / Done / Unknown` as in §5.3 |
| `settings::restore(checkpoint, identities)` | Moves items back only for the **same** identity they came from (R-02, R-10); verifies hashes |
| `settings::is_removed(folder)` | The settings sync never publishes, pushes or applies settings for a folder in M. A settings master holding them moves them to recovery like every other member |

### 6.2 Rules

1. **Owned only.** Unknown or shared ownership: the code is removed and
   those items stay untouched; ack `settings: code_only` with a diagnosis
   (R-07, Q-46).
2. **Paused settings** (R-08): the code is removed; paused items stay in
   place; ack `settings: kept_paused`; shown "settings kept (paused)" (Q-47).
3. **Pairing.** A `removed` recovery point records `paired` only if every
   owned item was checkpointed and verified; otherwise `code_only` with the
   reason.
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
settings_not_applicable`, and the GUI says "Settings for this addon were not
changed by WoW Sync." It never claims a paired removal or restore (R-10,
A-19). A-10 is not complete until the settings half passes.

## 7. History and recovery (R-10, A-16, A-17)

### 7.1 Three states with removal

History is per package, separately for code and settings (R-10). A removal
is a history state whose content is "not installed":

| Position | Code | Settings |
|---|---|---|
| current | removed (nothing live) | removed (or unchanged with `code_only`) |
| previous 1 | the removed version (recovery point `kind: removed`) | checkpoint taken at removal |
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

1. **Source.** The local recovery point of kind `removed`. If there is none
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
3. **Operation** `kind: restore`: install or replace with journal, `G` and
   verification as apply.md §2.2. The live state before restore (normally
   absent) becomes a recovery point of kind `restore_undo`.
4. **Settings:** the paired checkpoint is restored for matching identities
   if present. Otherwise code-only restore with the warning "Settings were
   not restored: no saved copy from before the removal" (R-10, A-19).
   Partial and paired restores are reported distinctly.
5. **Re-add:** after verification, write a `user_restore` re-add record
   superseding every live removal that names a restored folder, then publish.
   The folders propagate as an addition (§3.5).
6. A group-rejected version (rollback.md) is not offered without first
   lifting the rejection, which is a separate explicit choice.

"Keep it" for a held local observation (§1.6) and **Add back to the
group** on a computer that still holds a copy while a removal is pending
there (for example WoW is running) use the same re-add record without
moving files: the live copy is the re-added version.

## 8. Rollback, retirement and joining

### 8.1 Rollback and rejection (R-10, A-18)

Owned by `rollback.md` (packet R4-settings-design, written concurrently;
this document does not link it until it exists). Removal requires:

- A removal is not a release and cannot be rejected. Undoing it is a
  restore (§7.4).
- Rejection records and removal records are independent grow-only sets.
  Neither supersedes the other. A rejected version stays excluded after a
  re-add.
- Rollback of a currently removed package is a restore, offering only
  non-rejected states.
- The ledger (§2.5) is generic over record kinds; rollback.md may add a
  rejection kind with the same identity, mirroring and validation rules.

### 8.2 Retirement (R-03, A-45)

Owned by the run-004 retirement design (R4-ops-design). Removal requires:

- Records issued by a computer later retired **stay valid** (V3); others
  have mirrored them. Retirement does not un-delete anything.
- The retirement record carries the retired enrollment's last admitted seq
  as known to the retiring computer (the cutoff) and `authority_known`, the
  record hashes of that enrollment in the retiring computer's ledger. A
  record of that enrollment is valid only if `issued_at_seq` ≤ cutoff or its
  hash is in `authority_known`.
- A record that becomes `invalid(after_retirement)` after it was acted on
  leaves the merge. Members that removed P keep their recovery points; P
  then follows ordinary rules, including stale-copy rules computed over valid
  removals only (Q-52).
- Settling waits only for current, non-retired members. A retired member's
  states are not candidates, so its copy can neither keep nor bring back a
  removed addon.
- A re-enrolled computer is a new enrollment: it is not in older audiences,
  and its old acks do not carry over.

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
| Needs attention | "Was *X* removed, or renamed to *Y*?" (E6); "Waiting for removal information from *Laptop* about *X*" (§4.4); interrupted removal | Confirm removal / Keep it; Continue without it; open needs-help detail |
| Decision | no removal decisions exist; nothing added | — |
| Needs-help detail | interrupted remove or restore journal, `Unknown` slot | the existing journal resolution flow |
| Activity | grouped events: "3 addons removed on this computer (from *Laptop*)"; "*X* restored and reinstalled on all computers"; "*Laptop*'s updated copy of *X* was kept in recovery" | open detail |
| Settings sync | settings of removed addons shown as removed (or kept: paused, unknown ownership) | — |
| Tray and OS notifications (R4-notify-diag) | grouped removal notification (R-13, A-27) | open Activity |

Every action works with the keyboard and without a terminal (A-26).

## 10. Implementation map

Core stays pure; one implementation is shared by runtime and tests (A-43).

| Module | Add or change |
|---|---|
| `core::authority` (new) | `RecordKind { Removal, Readd, Ack }`; `RemovalRecord`, `ReaddRecord`, `RemovalAck`, `Issuer`, `ManifestEntry {key, name, tree}`, `AudienceEntry`, `SettingsRoots` (all `serde`, `Record` impls); `AuthorityRef {hash, kind, folders}`; `decode_authority(bytes, name_hash) -> Result<AnyRecord, RecordError>` (hash check first, then size, then schema); `LedgerEntry`, `Validity { Valid, Pending(PendingReason), Invalid(InvalidReason) }`, `AuthorityLedger { entries, dismissed }` (`wowsync.authority-ledger/1`); `learn(&mut ledger, refs, listings, read) -> LearnDelta`; `validate(&mut ledger, members, retirements)`; `mirror_due(ledger, own_listing) -> Vec<Sha256>` |
| `core::removal` (new) | `MergedRemoval { live: BTreeMap<FolderKey, BTreeSet<Sha256>>, units: Vec<RemovalUnit> }`; `merge_removals(&ledger) -> MergedRemoval`; `owes(&ledger, enrollment, folder) -> BTreeSet<Sha256>`; `PresenceBaseline`, `Snapshot`, `BaselineEntry` (`wowsync.presence-baseline/1`); `RemovalObservation { kind: Removal \| Reinstall, state: Holding \| Unclear \| Ready \| Published, … }` (`wowsync.removal-observation/1`); `observe(...) -> ObserveOutcome { baseline, observations, new_records }`; `acks_due(&ledger, &merged, enrollment, local) -> Vec<RemovalAck>`; `readd_for_restore(...)`; `pending_hints(&ledger) -> BTreeSet<FolderKey>` |
| `core::records` | `CodeState` gains `#[serde(default, skip_serializing_if = "Vec::is_empty")] authority_refs: Vec<AuthorityRef>` and `features: Vec<String>`. `ApplyJournal` gains `kind: OpKind { Apply, Remove, Restore }` (default `Apply`), `removals: Vec<Sha256>`, `ack: Option<StoredAck>` (all `serde(default)`). `RecoveryPoint` gains `kind: PointKind { Replaced, Removed, RestoreUndo }` (default `Replaced`), `removals`, `settings: Option<CheckpointRef>`, `paired: Pairing` (default `NotApplicable`). `ReportKind` gains `Removed` |
| `core::plan` | `FolderAction::Remove`; `PackageApply` gains `kind`, `removals`, `settings`; `plan_apply` takes `&MergedRemoval` through `GroupTarget`; merging of remove and update folders of one package (§5.1) |
| `core::target` | `TargetInput` gains `merged: &MergedRemoval`, `owed: &BTreeMap<ComputerId, BTreeSet<FolderKey>>`, `holds`, `pending_hints`. `PackageTarget` gains `Remove { folders, removals, reason: RemoveReason { Removed, StaleCopy } }` and `Held { reason: HoldReason { RemovalObserved, RemovalUnclear, ReinstallObserved } }`. Observation stripping before `group_packages` (§4.1). `DecisionReason` unchanged |
| `core::package` | no change to `group_packages`/`compare_package`; called on stripped observations |
| `core::exec` | remove slot table and rollback (§5.3); finalize writes the ack via a new effect; `recover_interrupted` handles `kind: remove \| restore`; `ApplyEffects` gains `write_authority(hash, bytes)`; `PublishEffects` gains `list_authority(member)`, `read_authority(member, hash)`, `write_authority`, `stat_authority` (repair) |
| `core::status` | `PackageStatus` gains `RemovalHeld`, `RemovalUnclear`, `ReinstallHeld`, `RemovedHere`, `Removed { confirmed_by, pending_on, older_version }`, `WaitingForRemovalInfo`; headline mapping per §9; `confirmations` extended per §5.6 |
| `core::explain` | `Explanation` variants for removal provenance, stale copy, dependency warning, kept-in-recovery copies, pending information |
| `core::layout` | `authority_dir(g, c)`, `authority_record(g, c, hash)`; local names for ledger, baseline and observations |
| `core::testkit` | removal scenario helpers: records, ledgers, three-replica delivery permutations, fake authority effects |
| runtime `service.rs` | `sync_once` steps 2a, 3a, 3b and repair (§1.6); `remove_from_group`, `restore`, `add_back`, `confirm_unclear`, `keep_unclear`, `continue_without(hash)` service calls; directory identity for E3 |
| runtime `effects.rs` | provider `authority\` IO (write-if-absent, read-back, listing, cloud-only refusal per Q-6/Q-23); local ledger and record copies |
| runtime `detail.rs` | rows for the Removed section and removal provenance |
| app `pages.rs`, `model.rs` | the §9 screens and actions; no new `Page` |

Settings functions (§6.1) live in the settings module owned by
R4-core-settings; removal calls them through the contract.

## 11. On-disk formats and compatibility

| Record | Change | Older app reading it | This app reading older data |
|---|---|---|---|
| `wowsync.code-state/1` | additive `authority_refs`, `features` (`serde(default)`, omitted when empty) | unknown fields ignored (no `deny_unknown_fields`); it never removes anything (slice rule) and is shown to others as "older version" (§2.4) | missing fields = no refs, no `removal/1`: absence stays absence, never removal |
| `wowsync.removal/1`, `wowsync.readd/1`, `wowsync.removal-ack/1` | new provider records under `authority\` | not read | unknown fields ignored; unknown major → `pending(unknown_schema)`, shown "written by a newer version" |
| `wowsync.authority-ledger/1`, `wowsync.presence-baseline/1`, `wowsync.removal-observation/1` | new local records; limits 16 MiB, 1 MiB (never dropping a referenced snapshot; overflow → `NeedsHelp(StorageLow)`), 64 KiB | — | absent = empty |
| `wowsync.apply-journal/1` | additive `kind`, `removals`, `ack`; `FolderAction` gains `remove` | a journal with `remove` fails to decode: an app downgraded while a removal journal exists reports a needs-help journal and changes nothing | missing `kind` = `apply` |
| `wowsync.recovery-point/1` | additive `kind`, `removals`, `settings`, `paired` | unknown fields ignored | missing `kind` = `replaced` |
| `wowsync.pending-report/1` | `ReportKind::Removed` | an older app reading `removed` fails to decode the report; same downgrade note | — |

Q-56 records the choice of additive fields over a `code-state/2` major
(revising the Q-40 default). No record schema major changes. Records are
written with `records::encode`; the `record!` macro keeps major 1.

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
| T7 | `observed_reinstall` re-add without an own ack, and with an ack issued after the re-add | `invalid(readd_without_ack)`; P stays removed; the copy is removed as stale | A-10, A-20 |
| T8 | C in r1's audience holds v2, unacked; learns r1 and a1 (a) in one cycle, (b) in two cycles, (c) a1 first | all orders end with v2 in C's recovery and A's re-added version installed | A-20 |
| T9 | Joiner with a copy: (a) removal live; (b) removal superseded before the join | (a) listed in consent, removed after consent; (b) ordinary existing addon, newest wins | A-04 |
| T10 | Ref to an unreadable removal on one replica | installs of hinted folders deferred; other packages and removals proceed; record mirrored elsewhere → learned and applied; own copy deleted → repaired next publish; "Continue without it" → acts as unknown; record readable later → applied | A-20, A-26 |
| T11 | Ref whose `kind` or `folders` differ from the record; a file whose bytes do not hash to its name, with a good copy elsewhere | mismatched ref ignored with a diagnostic, record stands; bad copy ignored, good copy used; never a conflict | A-20, A-23 |
| T12 | Malformed records: unsorted or duplicate folders, a name that fails `validate_folder_name` (traversal, separators, non-ASCII), oversize, unknown major, wrong reference kind, non-member issuer, wrong `group_id` | `invalid` or `pending` as §2.6; nothing removed; no path escape; nothing executed; valid unrelated records still apply | A-23 |
| T13 | Retired issuer: record above the cutoff not in `authority_known`; one within the cutoff; one learned late | above-cutoff `invalid(after_retirement)`, also if learned before the retirement record; within-cutoff valid after the issuer's states become unusable; settling ignores the retired member | A-45, A-20 |
| T14 | Runtime cycle loop: P present and published; deleted; ≥ 4 cycles with more than three own states published and pruned before the hold matures; restart mid-hold | baseline survives; record and own ack published after the settle wait; P never reinstalled here during the hold; receivers remove into recovery | A-10 |
| T15 | P absent in one scan (absent state published), then back with the same or new content; also across a restart | no record; hold reset; new content is an ordinary update | A-10, A-21 |
| T16 | Unreadable folder; unstable listing; AddOns directory identity changed with all folders gone | no record | A-05, A-20 |
| T17 | ASCII folder absent while a new unsupported entry appears; ≥ 4 cycles where other packages update and refresh their baselines; restart | no record; `Held(RemovalUnclear)`; "Confirm removal" writes `user_confirmed`; "Keep it" cancels | A-23, A-26 |
| T18 | Every folder in AddOns deleted (same directory identity), ≥ 3 packages | removals after the settle wait, no confirmation (D-21) | A-10 |
| T19 | Our own interrupted apply leaves `L` absent | no record (E5); recovery restores | A-22 |
| T20 | Update drops folder X; receivers with the same and a different X tree, with and without the update planned in the same cycle | `folder_drop` record; X removed into recovery in the update op or alone; new version complete | A-09 |
| T21 | After own ack, the user reinstalls P with an updater | `Held(ReinstallObserved)`; not removed during the hold; `observed_reinstall` re-add; propagates | A-10 |
| T22 | Crash after every effect of the remove op and of its recovery (including a second crash during recovery); fakes and real temp filesystem | every restart reaches `Untouched` or `Done`; no lost folder; the ack bytes are identical on retry; accurate status | A-22 |
| T23 | Folder recreated in `L` after `L→R` | `Unknown` → `needs_help`; nothing deleted | A-22 |
| T24 | WoW starts before, between and after removal renames, and during a restore | gate stops; no further change; rollback after exit; no process stopped | A-21 |
| T25 | More than three history states with a removal among them; prune; low storage with an exception | current + two previous; `removed` counts; exceptions visible; `NeedsHelp(StorageLow)` | A-16, A-17 |
| T26 | Several remove → restore → update → remove cycles; inspect provider roots after each | only live removals' manifests and settings roots are removal roots; superseded payload unrooted; provider restore offered only for a live removal | A-16, A-17 |
| T27 | Restore with a local point; without one (from the provider, on a computer that never had P, after pruning); with released payload | restored and propagated; staged from the provider with hash = `removed_hash`; `Unavailable` | A-10, A-16 |
| T28 | Settings: owned, unknown/shared, paused; paired removal and restore; restore with a missing checkpoint (settings phase) | paired / code-only with diagnosis / kept with notice; code-only restore keeps current settings with a distinct warning; WoW's own files untouched | A-09, A-10, A-15, A-19 |
| T29 | Rejected release and removal | restore does not offer the rejected version; neither record set supersedes the other | A-18 |
| T30 | Confirmation and wording: a peer without `removal/1`; a peer with an ack but still listing P; a peer acked and absent | "older version"; not confirmed; confirmed. No "removed everywhere" before all current members confirm | A-24, A-25 |
| T31 | Compatibility: run-003 `code-state/1` without new fields; new state read by the run-003 decoder; old journals and recovery points | decode OK both ways; absence never removal; defaults apply | A-36, A-20 |
| T32 | GUI: remove from all computers, see per-computer progress, confirm an unclear removal, continue without pending information, restore, add back, with keyboard only | flows complete without a terminal; wording rules hold | A-25, A-26 |
| T33 | Installed app, real Dropbox, two computers (provider tier, after owner Check 2; Q-23) | removal applied and confirmed; provenance kept | A-34 |

**A-38 negative controls** (temporarily mutate the production rule, observe
the named test fail with exit 101, restore) for at least: T3 (reintroduce an
id-variant conflict), T4 (make the merge depend on arrival order, e.g. take
the latest-learned record), T5 (let an updated copy win), T7 (drop V5), T8
(ignore owing), T10 (forget a pending hint, or block removals on it), T11
(trust the ref over the record), T14 (read E1 from the newest published
state, or omit `Held`), T17 (compare E6 with a shared latest snapshot), T22
(regenerate the ack on retry), T26 (root superseded payload), and T30
(confirm on publication alone).

## 13. Review findings: status after this rewrite

### 13.1 R3D-13 closure

R3D-13 ([recheck 1](../runs/run-003/evidence/R3-design-recheck.md),
[recheck 2](../runs/run-003/evidence/R3-design-recheck2.md)) was: a record id
with two variants is permanently inconsistent, the decision meant to resolve
it had to list every variant, and V7 rejected the decision when a variant
named another package. **It is closed by removing the state, not by
resolving it.**

1. **No id variants.** Identity is the hash of the exact bytes and the file
   name (§2.2). Different content is a different record. A copy that does
   not match its name is not a record. The run-003 trace (variants `(T, hP)`
   and `(T, hQ)`) becomes two valid removals, of P's folders and of Q's
   folders (example 3, T3).
2. **Nothing needs resolving.** Any valid removal wins (D-21, D-23), and
   removals merge by union of folder keys (§3.1). There is no
   `conflicted` status, no `InconsistentRemovalRecord`, and no removal
   decision of any kind (§4.3).
3. **V7 holds.** No rule requires a record to list related records. The new
   V7 (§2.6) checks only that a record's own listing is well-formed and that
   a re-add overlaps what it supersedes. Writers derive `supersedes` from
   exactly those overlapping live removals, so every record the app writes
   passes V7, whatever packages the related records span.
4. **Every reachable state resolves.** For every folder key, the result is a
   total function of the ledger (§3.2). The only local states that wait
   either clear by themselves or have a GUI action: the settle hold (matures
   or resets), the unclear rename (confirm, keep, or reappearance), the
   reinstall hold (matures or resets), pending information (learned,
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
| R3D-8 | Removal: authority lost with an unusable newest state | **Kept, strengthened.** Durable ledger, mirroring on learning, plus own-copy repair (§2.5). Pending information no longer blocks a whole package (§4.4) |
| R3D-9 | Removal: compact tombstones and lost restore data | **Kept, simplified.** Identity = hash of exact bytes = file name; refs carry only that hash plus a routing hint that is never trusted over the record; exact `folders` manifest kept (§2.2–§2.4) |
| R3D-10 | Removal: keep-version decision overridden | **Mechanism removed.** Removal makes no version choice; version pinning and rejection belong to rollback.md. The defect cannot recur in removal |
| R3D-11 | UI: coverage claims from placeholders | **Closed; kept.** §9 lists needs; it claims no acceptance from placeholders |
| R3D-12 | UI: layout and headline rules | **Closed; kept.** ui/screens.md is authoritative; removal adds no headline state (§9) |
| R3D-13 | Removal: inconsistent authority unresolvable | **Closed by D-23** (§13.1) |
| R3D-14 | Removal: one global snapshot | **Kept unchanged.** Per-entry snapshot references (§1.6) |
| R3D-15 | Removal: superseded payload kept forever | **Kept, simplified.** Only live removals root payload (§7.3) |
| R3D-16 | Removal: resume formula declines the kept version | **Mechanism removed.** No `declined` sets or resume choice exist in removal |

## 14. Changes to other design documents when implemented

Recorded here because this packet does not own those files.

| Document | Change |
|---|---|
| [domain.md](domain.md) §6 | Selection runs below the removal precedence of §4.1: rejections, local holds, merged removal, stale copies, pending hints, then ordinary ordering over observations stripped of removed and owed folders. The Q-2 slice limitation is replaced by the folder-drop rule (§1.5). "There is no removal record in the slice" becomes: "Absence never removes; only a live removal record in the ledger (removal.md §3) removes." New target states `Remove` and `Held`; no new `DecisionReason` |
| [domain.md](domain.md) §1 step 5 | The consent text also states that addons deliberately uninstalled on one computer are removed on the others and kept for recovery, with their settings (§6), and lists the joiner's copies of removed addons (§8.3) |
| [publication.md](publication.md) §1 | layout gains `members\<computer_id>\authority\<sha256>.json` (single writer, write-if-absent, never deleted while the group exists). Readers list it for every member including unusable and retired ones |
| [publication.md](publication.md) §2 | additive `authority_refs` and `features` in `code-state/1`; record schemas of §2.3; local schemas of §11; report result `removed`; confirmation rule of §5.6 |
| [publication.md](publication.md) §4 | publish writes, reads back and repairs referenced records before the state; step 7 pruning never touches `authority\` |
| [publication.md](publication.md) §6 | admission unchanged; learning and validation (§2.5–§2.6) are a separate step whose output is the ledger |
| [apply.md](apply.md) §1 | steps 2a, 3a and 3b (§1.6); step 5 uses §4.1 |
| [apply.md](apply.md) §2.2, §3.1, §4.1 | remove and restore operations, the remove slot table and crash rows (§5.3); mixed update-and-remove operations (§5.1) |
| [apply.md](apply.md) §2.4 | pruning counts history states including `removed` (§7.1) |
| [apply.md](apply.md) §6 | statuses of §10 `core::status` |
| [core-api.md](core-api.md) | the §10 module map |
| [questions.md](questions.md) | Q-40's default is revised by Q-56; Q-42 stays rejected; Q-49 and Q-50 no longer apply to removal (no removal decisions) |
