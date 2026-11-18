# Shared contracts: authority records, journals, identities and target composition

Status: **run-004 design, packet
[R4-design-fix](../runs/run-004/packets/R4-design-fix.md).** It fixes
R4D-1 and R4D-10 of the
[R4-design-review](../runs/run-004/evidence/R4-design-review.md), and the
shared parts of R4D-2 to R4D-14. **Not implemented.** Choices marked
**PENDING OWNER** use the conservative default in
[questions.md](questions.md). They are not owner decisions.

**This file is the only definition of:**
- the authority record family;
- file naming, hashing and references;
- issuer validity and historical membership;
- mirroring and repair;
- capabilities and downgrade;
- the apply journal and recovery point schemas;
- stable addon and settings identities;
- target composition, including the code-choice family.

[removal.md](removal.md), [settings.md](settings.md),
[rollback.md](rollback.md) and [ops.md](ops.md) define only kind-specific
bodies and behaviour on top of it. Where one of them appears to define any
of the above differently, this file wins and the other text is a defect.

Product authority: SPEC R-02 to R-14. Acceptance: A-04, A-08 to A-10,
A-16 to A-27, A-33, A-45. Builds on [publication.md](publication.md),
[apply.md](apply.md), [domain.md](domain.md) and the slice code in
`crates/wow-sync-core` (`records.rs`, `ids.rs`, `hash.rs`, `target.rs`,
`exec.rs`).

---

## 1. C-AUTH: the authority record family

### 1.1 Kinds

One dispatched family. Every kind has the same envelope (§1.4), the same
identity (§1.3), the same validation (§1.11) and the same learning,
mirroring and repair (§1.12). Size limit: **64 KiB per record**, every kind.

| `kind` | Schema | Body defined in | Meaning |
|---|---|---|---|
| `removal` | `wowsync.removal/1` | [removal.md](removal.md) §2.3 | the tombstone: these folders were deliberately removed |
| `readd` | `wowsync.readd/1` | [removal.md](removal.md) §2.3 | an explicit add-back of named folders |
| `ack` | `wowsync.removal-ack/1` | [removal.md](removal.md) §2.3 | this enrollment applied (or never had) these removal folders |
| `code-choice` | `wowsync.code-choice/1` | this file §5 | keep one copy, or resume automatic selection |
| `rejection` | `wowsync.rejection/1` | [rollback.md](rollback.md) §2.1 | this exact release content is rejected for the group |
| `lift` | `wowsync.rejection-lift/1` | [rollback.md](rollback.md) §2.2 | allow a rejected release again |
| `settings-policy` | `wowsync.settings-policy/1` | [settings.md](settings.md) §3.2 | settings master and per-addon exceptions |
| `settings-revision` | `wowsync.settings-revision/1` | [settings.md](settings.md) §3.3 | an intentional settings change, push or restore for one unit |
| `retirement` | `wowsync.retirement/1` | this file §1.10 and [ops.md](ops.md) §2 | this enrollment is retired |

There is no other authority kind. The run-003 "decision" objects, the
removal decision kinds and any settings-checkpoint rejection are **retired**
designs and are not part of the family.

### 1.2 Location and file name

```text
groups\<group_id>\members\<computer_id>\
  member.json
  state\<enrollment_id>-<seq:010>.json        code-state/1 (+ authority_index, features)
  authority\<sha256 hex>.json                 records: own and verbatim mirrors
  authority-index\<sha256 hex>.json           wowsync.authority-index/1 chunks (§1.6)
```

- Single writer per member directory (publication.md §1).
- A record file is written **write-if-absent**: temp file, flush, rename,
  then read back and hash-check. Only then may a state reference it.
- Records are never edited or deleted while the group exists (Q-44). The
  one exception is the repair of a bad copy in the own directory (§1.12).

### 1.3 Identity and hashing

- A record's identity is `hash::sha256(exact stored bytes)`, lowercase hex.
  The file name is that hash plus `.json`.
- There is **no** `id` field, no `{id, hash}` reference, no random object
  id, no domain-separation prefix and no second "compact" hash. The
  run-003/run-004-draft forms (`"id": "<32 hex>"`, `{ "id", "hash" }`,
  `"wowsync-authority-v1\n"`) are retired.
- Writers produce bytes with `records::encode` (deterministic). Mirrors copy
  bytes verbatim, so every copy has the same name.
- Two records with different bytes are two records. Identical bytes are one
  record. "Same id, different content" cannot be expressed.

### 1.4 Envelope and decoding

Common fields of every kind:

```json
{
  "schema": "wowsync.<kind schema>/1",
  "group_id": "<group id>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 39 },
  "written_unix_ms": 1767225600000
}
```

- `issued_at_seq` is the seq of the first own state whose authority index
  includes the record. The issuer reserves it (`LocalEnrollment.next_seq`)
  before writing the record.
- `written_unix_ms` is display only. Nothing orders by it.
- Records carry **no free text** except two untrusted display strings: the
  `title` of removal and re-add records and the `version_label` of
  rejection records (each ≤ 128 bytes). They are rendered as plain text
  with colour codes stripped (`detail::strip_colour_codes`), never decide
  anything except the rejection look-alike hold (rollback.md §3.3), and are
  never exported (§7).

`authority::decode_authority(bytes, name_hash) -> Result<AuthorityRecord, AuthorityError>`:

1. `bytes.len() > 64 KiB` gives `TooLarge`.
2. `sha256(bytes) != name_hash` gives `BadCopy`. A bad copy is not a record
   and never a conflict. Another copy with the same name may still be good.
3. JSON object with a `schema` string, then dispatch on the schema name to
   the kind's `records::decode`. Unknown schema name or major gives
   `UnknownSchema` (ledger status `pending(unknown_schema)`).
4. Kind validation (well-formedness only; ledger-dependent rules are §1.11).

Unknown fields are ignored within a major (`records::decode` behaviour,
Q-13).

### 1.5 References

- **State to record:** `AuthorityRef { hash, kind, folders }`. `folders` is
  a routing hint. It holds the folder keys the record names: for an ack the
  union of its items' folders, for a settings revision its unit's folder
  key, and empty for policy and retirement. It scopes a deferral while the
  record is unreadable. It is never trusted over the record: once the
  record is read, a ref whose `kind` or `folders` differs is a
  **mismatched ref**, ignored with a diagnostic.
- **Record to record:** a plain hash. The field fixes the expected kind
  (for example `removals` must name `removal` records). A wrong kind gives
  `invalid(wrong_reference)`.
- **Record to tree or blob:** a plain hash of a provider tree record
  (publication.md §3).
- **Settings checkpoint:** `CheckpointRef = sha256(exact manifest bytes)`,
  the checkpoint manifest's file name (§3.4).

### 1.6 The authority index (R4D-4)

Refs no longer sit inline in the state. Each enrollment publishes a chain
of immutable index chunks:

```json
{
  "schema": "wowsync.authority-index/1",
  "group_id": "…",
  "enrollment_id": "…",
  "prev": "<chunk hash>|null",
  "refs": [ { "hash": "<sha256>", "kind": "removal", "folders": ["suite-core"] } ]
}
```

- Chunk limits: **≤ 1,024 refs and ≤ 256 KiB**. File name =
  `sha256(bytes)`. Written write-if-absent like records.
- `code-state/1` gains two additive fields (old readers ignore them):
  `authority_index: { "head": "<chunk hash>", "total": <ref count> }` and
  `features: [<capability>]` (§1.13).
- Grow-only per enrollment. To add refs, the writer either appends a new
  chunk whose `prev` is the old head, or, when the head chunk is not full,
  writes a **replacement** chunk with the same `prev` holding the old head's
  refs plus the new ones. The replaced chunk stays on disk, unreferenced.
- Readers walk from `head` along `prev` until they reach a chunk they have
  already processed completely, then stop. The sum of refs over the chain
  must equal `total`; otherwise, or when a chunk is missing or bad, the
  member's index is `unreadable` and the refs already learned are kept.
- A ref is listed once per enrollment. The index of an enrollment lists
  every **valid** ledger record of that computer, own and mirrored (§1.12).
- Size: 10,000 refs is about ten chunks of about 150 KiB. The state itself
  stays at its slice size.

### 1.7 The ledger (local)

`wowsync.authority-ledger-segment/1`: this enrollment's durable, grow-only
set of learned records, in append-only segment files under the app data
`authority\ledger\` directory.

| Field per entry | Meaning |
|---|---|
| `hash`, `kind`, `folders` | identity, kind and folder hint |
| `issuer` | from the record, or from the ref while pending |
| `learned_from` | `[computer_id]`, every member directory or index that named it |
| `mirrored_seq` | the own seq at which this computer first listed it, or `null` |

- Segments are at most **4 MiB** each. A new segment starts when the
  current one would exceed that. The 16 MiB local-record limit therefore
  never applies to the ledger as a whole (R4D-4).
- Record bytes are stored verbatim under `authority\objects\<hash>.json`.
- **Validity is not stored.** It is re-derived at load and whenever a
  member, retirement or referenced record changes (§1.11).
- `dismissed: [hash]` (the local "Continue without it" choices, removal.md
  §4.4) lives in a separate small local file. It is never published.
- Entries are never deleted. Provider absence is never a reason to forget a
  record.
- **Memory:** an in-memory entry is about 150 bytes plus its decoded body,
  which is loaded on demand. 100,000 entries are about 15 MiB, within the
  100 MiB background gate (R-14). Above 100,000 entries the app shows a
  diagnostic "This group has a very long history" (a technical cap, §8). It
  never drops entries.

### 1.8 Causal summary `Known`

Records that end or override earlier records (re-add, code choice, lift,
settings policy and revision) carry a causal summary instead of an
unbounded list of hashes:

```json
"known": {
  "seen": [ { "enrollment_id": "…", "seq": 57 } ],
  "extra": [ "<sha256>" ]
}
```

- Record r is **covered** by `known` iff `seen[r.issuer.enrollment_id] ≥
  r.issued_at_seq`, or `r.hash ∈ extra`.
- A writer may claim `(E, s)` only if it has read E's index chain up to an
  admitted state of seq ≥ s **and** every record E issued with
  `issued_at_seq ≤ s` is in its ledger as `valid` or `invalid` (none
  `pending`). Because a record's `issued_at_seq` is the first own state
  listing it, reading E's index at seq s yields every E-issued record up to
  s.
- `extra` (≤ 128 hashes) covers individual records learned only as mirrors,
  before their issuer's chain was readable.
- `seen` holds at most **512** enrollments (about 40 KiB). A group with a
  longer history cannot write such records and gets
  `NeedsHelp(GroupHistoryTooLarge)`; this is a technical cap (§8).
- `seen` and `extra` are sorted and unique.
- Never compare seqs of different enrollments. `Known` only compares a
  record's own issuer seq with the summary's entry for that issuer.

Supersession in every family uses coverage: "X supersedes Y" means X is
valid, X and Y concern overlapping scope (defined per kind), and X's
`known` covers Y. A record written concurrently with Y (X's writer had not
learned Y) does not cover it. This replaces the unbounded `supersedes`,
`knows` and `considered`-as-supersession lists of the drafts (R4D-4: 1,024
superseded removals needed 68,624 bytes).

### 1.9 Bounded record sizes

Every record must fit in 64 KiB. Writers split, never truncate:

| Kind | Bound | Split rule |
|---|---|---|
| removal, readd | ≤ 256 folders | split into several records with the same evidence and issued seq. Merging is a union per folder, so the split is equivalent (removal.md §3.2) |
| ack | ≤ 256 `(removal, folders)` items | several acks; obligations are per item (removal.md §3.4) |
| code-choice | ≤ 64 `considered` entries | one entry per distinct candidate copy; more gives `NeedsHelp(TooManyCandidates)` |
| rejection | ≤ 64 `considered` entries | as code-choice |
| retirement, policy, revision | fixed size | — |

`Known` is bounded by §1.8. A test writes a re-add after 1,024 live
removals of one folder and checks the record is below 64 KiB (T-C08).

### 1.10 Historical membership and the retirement barrier (R4D-11)

**Historical membership.** Core keeps a durable, grow-only local set of
`(computer_id, enrollment_id)` pairs ever proved to be members of the
group:

- the current `member.json` of each member directory;
- `member.json.previous_enrollments` (new additive field, written at
  re-enrollment, grow-only);
- the enrollment ids in state file names of that member directory;
- the `retired` subject of valid retirement records.

Issuer validity (V-ISSUER) uses this set, never only the current
`member.json`. A re-enrolled computer's old records therefore stay valid.

**Retirement record body** (`wowsync.retirement/1`, schema owned here,
behaviour in [ops.md](ops.md) §2):

```json
{
  "schema": "wowsync.retirement/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 61 },
  "retired": { "computer_id": "…", "enrollment_id": "…" },
  "cutoff_seq": 47,
  "was_settings_master": true,
  "written_unix_ms": 1767225600000
}
```

**The barrier.** For enrollment E:

1. **Retired(E)** iff the ledger holds any retirement record of E that
   passes V-BYTES, V-GROUP and V-ISSUER. Retirement records are **exempt**
   from V-RET: a retirement is never invalidated by the retirement of its
   issuer. Retired(E) is therefore monotone: once true on a computer it
   stays true, whatever arrives later.
2. **Cutoff(E)** = the **maximum** `cutoff_seq` over those records. More
   retirements can only raise it, so an already-valid record of E never
   becomes invalid because another retirement arrives later.
3. Every other record of E is valid only if `issued_at_seq ≤ Cutoff(E)`
   (V-RET). E's states with `seq > Cutoff(E)` are not admitted (§6).
4. A retired enrollment only becomes active again through explicit
   re-enrollment, which is a new enrollment id (A-45, R-03).

There is no `authority_known` list and no grounded labelling. Mutual or
transitive retirement retires every named enrollment. The GUI says, for
example, "Laptop and Den PC retired each other. Join again on the computer
you still use." The R4D-11 trace (A retires B; C later retires A with a
lower cutoff) leaves B retired in both delivery orders (T-C10).

### 1.11 Validation

A ledger entry is `valid` only if all rules hold. Kind-specific rules add
to these and are named `V-<KIND>-n` in the owning design.

| Rule | Check | Otherwise |
|---|---|---|
| V-BYTES | A good copy decodes (§1.4) | no good copy: `pending(unreadable)`; unknown schema/major: `pending(unknown_schema)` |
| V-GROUP | `group_id` is this group | `invalid(other_group)` |
| V-ISSUER | the issuer pair is in historical membership (§1.10) | `invalid(not_a_member)` |
| V-RET | not a retirement, or issuer not retired, or `issued_at_seq ≤ Cutoff(issuer)` | `invalid(after_retirement)` |
| V-REF | every referenced hash is in the ledger with the kind the field requires | absent: `pending(missing_reference)`; wrong kind: `invalid(wrong_reference)` |
| V-KIND | the owning design's kind rules | `invalid(<kind reason>)` |

| Status | Effect |
|---|---|
| `valid` | counts in its family |
| `pending(reason)` | counts as **not known**. A pending removal additionally defers installs of the folders in its hint (removal.md §4.4). It resolves automatically |
| `invalid(reason)` | ignored, diagnostic only, never blocks |

Validity is a pure function of the record, the ledger and historical
membership, so every computer with the same ledger derives the same result
in any order of arrival (A-20).

### 1.12 Learning, mirroring and repair

Cycle step 2a, for every member directory including retired members and
members whose newest state is unusable:

1. Read new index chunks (§1.6) and the `authority\` listing.
2. For each unknown hash, read a good copy: first the referencing member's
   `authority\`, then any member's. If none is readable the entry is
   `pending(unreadable)` with the ref's kind and folder hint.
3. Store the bytes and the ledger entry locally (temp + flush + rename)
   **before** any target is computed from them. Then derive validity.
4. Mirror every newly valid record verbatim into the own `authority\` and
   list it in the next own index. A record therefore survives its issuer's
   directory becoming unusable (Q-51).

**Repair before publishing.** Every record the next own index will list is
checked in the own `authority\` (stat; hash on size mismatch or once per
start):

| Own copy | Action |
|---|---|
| present, bytes hash to the name | nothing. **A good file is never overwritten** |
| missing | write-if-absent from the local verbatim bytes |
| present, bytes do not hash to the name (bad copy) | **repair:** write the local verbatim bytes to a temp file in the same directory, flush, read back and verify the hash, then rename over the bad file. If any step fails the bad file stays, the record stays listed, and the diagnostic says "A damaged copy could not be repaired" |

Only the directory's single writer repairs. A computer never writes into
another member's directory. Nothing is ever deleted by repair. A test
corrupts an own copy and a peer's copy and checks that only the own copy is
replaced and the peer copy is read past (T-C04).

### 1.13 Capabilities and downgrade

`code-state/1.features` lists what this app applies:

| Capability | Means |
|---|---|
| `authority/1` | reads, validates and mirrors the family and index (§1) |
| `removal/1` | applies removals and writes acks |
| `code-choice/1` | honours code choices |
| `rejection/1` | honours rejections and lifts |
| `settings/1` | settings sync |
| `retirement/1` | honours the retirement barrier |
| `journal/2` | writes and recovers `apply-journal/2` and `recovery-point/2` |

- A member without a capability is shown as "runs an older WoW Sync that
  does not apply <feature>". It never counts as confirmed for that feature,
  and group actions that need it are blocked with that explanation (for
  example group rollback, Q-84 PENDING OWNER).
- **Downgrade.** The slice app ignores `authority_index` and `features`,
  never reads `authority\`, and treats `/2` journals and recovery points as
  undecodable (§2.4). It therefore stops at `NeedsHelp` instead of acting
  on data it cannot understand. A downgraded member publishes states
  without the capabilities, which peers then show as above.
- A newer major of any kind is `pending(unknown_schema)`, never ignored as
  absent.

---

## 2. C-JRN: the apply journal and recovery point

### 2.1 Versioning against `record!`

`records::record!` hard-codes `MAJOR = 1`, and `decode` rejects any other
major. Phase C changes the macro:

- `const MAJORS: &[u32]` (accepted on decode) and `fn write_major(&self) ->
  u32` (chosen per value on encode).
- `fn validate_major(&self, major: u32)`: a value decoded as `/1` must not
  use any `/2`-only content.
- Every existing record keeps `MAJORS = [1]`, so its bytes are unchanged.

### 2.2 `wowsync.apply-journal`

**Code-only applies stay `/1`, byte-identical** to the slice (golden
fixture T-C05). Any operation with remove, restore, rollback or settings
content is written as `/2`:

| Field | `/1` | `/2` |
|---|---|---|
| `op`, `group_id`, `started_unix_ms`, `package`, `target_hash`, `phase`, `from_phase`, `reason`, `halted_by_game` | as slice | as slice |
| `folders: [JournalFolder]` | **non-empty** (slice validator) | may be empty only when `settings_ops` is not |
| `kind` | absent (means `apply`) | `apply \| remove \| restore \| rollback \| settings \| settings_cleanup` |
| `removals: [hash]` | absent | removal records this op applies (`remove`) |
| `pending_records: [{hash, bytes}]` | absent | ack, re-add or revision bytes fixed **before** commit, written after verify; retried byte-identically after a crash |
| `settings_ops: [SettingsFileOp]` | absent | settings file moves, below |
| `FolderAction` | slice values | adds `remove` (live folder moved to recovery, nothing installed) |

```rust
pub struct SettingsFileOp {
    pub unit: SettingsUnit,            // §3.2
    pub live_rel: SettingsPath,        // WTF-relative, validated
    pub expected_live: ExpectedLive,   // hash or absent, re-checked before the rename
    pub staged: Option<Sha256>,        // content to install, or None for move-out only
    pub recovery_slot: RecoverySlot,   // where the live file goes
}
```

`/2` validation: `folders` or `settings_ops` non-empty; `kind = remove`
needs non-empty `removals` and only `remove` folder actions; `kind =
settings | settings_cleanup` needs empty `folders`; `kind = apply` is
always written as `/1`.

### 2.3 `wowsync.recovery-point`

Code-only points stay `/1`, byte-identical. Points with settings content or
a removal are `/2` (limit raised to **256 KiB**):

| Field | `/2` addition |
|---|---|
| `kind` | `apply \| remove \| restore \| rollback \| settings \| settings_cleanup` |
| `removals: [hash]` | the removal records that moved the code (shown as "removed by …") |
| `settings: [SettingsRoot { unit, checkpoint: CheckpointRef, location: local \| provider }]` | settings files moved or replaced by this op |
| `paired` | `paired` (code and settings from the same op), `partial` (some settings units could not be captured, named in `settings` as missing), `code_only` (no settings captured), `not_applicable` |
| `code_package_hash` | the package hash of the code part, for paired selection (settings.md §4.6) |

These three recovery shapes are the only ones:

| Shape | `replaced`/`installed` | `settings` | `paired` |
|---|---|---|---|
| settings-only | empty | non-empty | `not_applicable` |
| mixed code and settings (update, remove, rollback with settings) | non-empty | non-empty | `paired` or `partial` |
| code-only fallback (settings paused, absent or uncapturable) | non-empty | empty | `code_only` |

Retention: current plus two previous per package (code) and per settings
unit, plus visible pinned exceptions (R-10). The settings roots of a `/2`
point are GC roots for local and provider checkpoints (settings.md §7).

### 2.4 Mixed-version safety

| Situation | Result |
|---|---|
| Slice app finds a `/2` journal | `exec::recover_interrupted` cannot decode it: `NeedsHelp(UnknownInterruptedState)`, journal kept, applies blocked (`journals_remaining`). Nothing is guessed |
| Slice app lists `/2` recovery points | `exec::decode_points` skips them, so the slice pruner never prunes them |
| Runtime decoders (`service.rs`, `effects.rs`) | same: undecodable means kept and shown, never deleted |
| New app crashes during a settings-only op, then is downgraded | as the first row (T-C06) |
| `PendingReport` | the local pending report and `code-state/1.apply_report` keep only slice `ReportKind` values. Removal and settings outcomes are reported only through acks and settings states, so an old peer never fails to decode a new state. A local pending report needing new kinds is written as `wowsync.pending-report/2`, which the slice treats as unreadable and replaces with a fresh report (T-C07) |

---

## 3. C-IDS: stable identities (R4D-6)

### 3.1 Addon identity

- The unit of code removal, rejection and choice is the **folder key**
  (`paths::FolderKey`).
- `package::Package.key` is `members[0]` of a local grouping. It is
  **display only** and never stored in a shared record, a policy or a
  settings unit. Regrouping (an addon gains or loses a folder) therefore
  changes no shared meaning.

### 3.2 Settings unit

`SettingsUnit { folder: FolderKey, identity: Identity }`:

- `folder` is the addon folder whose `.toc` declares the SavedVariables
  (settings.md §1.1). It is stable across regrouping.
- `identity` is the scope: `account(<account>)` or
  `character(<account>, <realm>, <character>)`, from the WTF path.
- Policies, exceptions, holds, revisions, checkpoints and recovery roots
  are keyed by `SettingsUnit` or by `folder`, **never by package key**.
  The GUI groups units by package and shows "mixed" when the units of one
  package differ.
- Regrouping never grants sharing. A folder that joins a package whose
  units are shared stays at its own folder-keyed policy (default: follows
  the master only if the master's policy names that folder or the group
  default covers it).

### 3.3 Identity normalization and collisions

- Identity text is compared after **Unicode NFC** normalization, exactly
  (case-sensitive).
- Two local paths whose NFC forms are equal but whose bytes differ:
  `IdentityNormalizationCollision`. Neither identity is synced on this
  computer; the GUI names both paths.
- Two computers whose identities differ only by case:
  `IdentityCaseMismatch`. They are not matched (Q-69, R-02); the GUI
  explains that the names differ only by letter case.
- A normalizing key never hides two distinct local identities. Tests run on
  each native filesystem: NTFS (case-insensitive) and APFS (case-insensitive,
  normalization-insensitive) (T-C11).

### 3.4 Checkpoint reference

A settings checkpoint manifest is a canonical record
(`wowsync.settings-checkpoint/1`, settings.md §3.1). Its reference is
`sha256(exact manifest bytes)`, also its file name. No domain prefix.

---

## 4. C-TGT: target composition

### 4.1 Precedence per folder

`target::compose` evaluates, highest first. The first level that applies
decides the folder's target on this computer:

| # | Level | Owner | Effect |
|---|---|---|---|
| 1 | Unresolved journal | slice | `AllApplies` hold (ops §1.6) |
| 2 | Rejection | rollback.md | rejected exact content is never a candidate, never kept as a target |
| 3 | Local hold (`Held(RemovalObserved \| RemovalUnclear \| ReinstallObserved)`, settings hold) | removal.md, settings.md | nothing installed, replaced or removed |
| 4 | Merged removal | removal.md §3 | folder removed into recovery |
| 5 | Stale copy (owed removal) | removal.md §3.4 | that copy is never a candidate |
| 6 | Constraint: live code choice or recovered rollback target | §5, rollback.md §3 | install exactly the constrained content |
| 7 | Pending removal information | removal.md §4.4 | installs of hinted folders deferred |
| 8 | Ordinary selection | domain.md §6 | newest demonstrable version |

The settings master is **not** code authority. No settings record affects
levels 2 to 8.

### 4.2 Constraint composition (R4D-9)

For a folder set F, the live constraints are the valid code choices and
rollback recovered targets for F that no other valid constraint for an
overlapping folder covers (§1.8):

- **None:** ordinary selection.
- **One:** install exactly its content, unless that content is rejected
  (level 2). Then follow the recovered chain (rollback.md §3.1) to the
  first non-rejected recovered target, else ordinary selection among
  non-rejected candidates.
- **Several, all the same content:** that content.
- **Several, different content** (concurrent choices or concurrent
  rollbacks on different computers): `NeedsDecision(ChoicesDiffer)` or
  `NeedsDecision(RollbackTargetsDiffer)`, with one option per distinct
  constrained content. The current copy stays installed meanwhile. Choosing
  writes one code choice whose `known` covers all of them.
- A valid re-add of a folder ends every constraint on that folder that its
  `known` covers.

Constraints never order by clock. Tests: T-R4D9-concurrent-rollbacks and
T-R4D9-choice-vs-rollback in rollback.md §10.

---

## 5. The code-choice family (R4D-10)

### 5.1 What is a decision, and what is not

D-21: WoW Sync syncs, it does not police addons. The `.toc` `## Interface`
number is therefore **advisory display text only**. It never blocks an
install and never creates a decision. There is no "Use it anyway" button.

The slice's `DecisionReason::NoCompatibleToc` actually means **unusable
metadata** (`target::folder_problem`: a non-`Ok` `FolderStatus`, or a
`TocKind` other than `Selected`/`Unvalidated`). Phase C renames it:

| Reason | Cause | Options |
|---|---|---|
| `Ambiguous` | copies cannot be ordered but differ (domain.md §6) | one **Keep this copy** per distinct copy; **Decide later** |
| `PartialCandidate` | one copy of a version lacks folders another has | **Keep the complete copy**; **Keep the smaller copy**; **Decide later** |
| `ChoicesDiffer`, `RollbackTargetsDiffer` | §4.2 | one option per constrained content; **Decide later** |
| `RecoveredUnavailable` | a live recovered rollback target has no source left (rollback.md §3.1) | one **Use <version>** per available non-rejected candidate; **Decide later** |
| `LookalikeOfRejected` | a candidate has the version label of an active rejection but different content (rollback.md §3.3) | **Use this copy**; **Block this copy too** (a rejection, not a choice); **Decide later** |
| `NewerAfterRollback` (notice, not blocking) | a non-rejected candidate not in the rejection's `considered` is demonstrably newer than the recovered target (rollback.md §3.1) | **Use <version>** (a keep covering the rejection); **Stay on <recovered>** (dismiss) |
| `UnusableMetadata(kind)` | `FolderStatus` `Unreadable`, `Unsafe`, `Unsupported`, `Unstable`, or `TocKind` `None`, `Ambiguous`, `Error` | **no override.** "This copy can't be used: <reason>. Reinstall or fix it on <computer>." Waits for a usable copy |
| `DifferentTocMapping`, `UnvalidatedTocMapping` | safe-scope validation (R-02) | **no override.** "Waiting for a WoW Sync update that knows this game version" |

No option ever makes unvalidated, unsafe or unreadable content installable
(R-02).

### 5.2 Record

```json
{
  "schema": "wowsync.code-choice/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 72 },
  "reason": "ambiguous|partial_candidate|choices_differ|rollback_targets_differ|newer_after_rollback|recovered_unavailable|lookalike_of_rejected",
  "action": "keep|resume",
  "folders": ["weakauras", "weakauras_options"],
  "keep": { "package_hash": "<sha256>",
            "folders": [ { "key": "weakauras", "name": "WeakAuras", "tree": "<sha256>" } ] },
  "considered": [ { "package_hash": "<sha256>", "holders": ["<enrollment id>"] } ],
  "known": { "seen": [], "extra": [] },
  "written_unix_ms": 1767225600000
}
```

V-CHOICE rules: `folders` non-empty, sorted, unique; `keep` present iff
`action = keep`, its folders ⊆ `folders` and its hash is the package hash
of its folders; `considered` ≤ 64 entries, sorted, unique, containing
`keep.package_hash`; every referenced tree is a plain hash.

### 5.3 Semantics

- **Keep:** a constraint (level 6) on `folders`: install exactly `keep`.
  Candidates in `considered` never end it, whatever their version.
- **New information:** a candidate that is **not** in `considered` and is
  demonstrably newer than the kept copy (domain.md §6) ends the keep. It is
  an ordinary update and is installed; the Activity log says "A newer
  version of WeakAuras replaced your earlier choice." A candidate not in
  `considered` that is not ordered against the kept copy raises a new
  `Ambiguous` item; the keep holds meanwhile.
- **Resume:** ends every choice it covers and imposes nothing. It **never
  excludes** any release. Exclusion exists only as rejection
  (rollback.md), which only an explicit lift ends.
- **Allow again** is the rejection lift (rollback.md §2.2). It is not a
  code-choice action.
- **Supersession:** choice X ends choice Y iff their `folders` overlap and
  X's `known` covers Y.
- **Concurrent choices:** §4.2.
- **Precedence:** rejection > removal > choice > ordinary (§4.1). A keep of
  rejected content is inert; a removal removes the folder whatever the
  choice; a valid re-add ends choices it covers.

### 5.4 Retracing R3D-10 and R3D-16

| Trace | Run-004 result |
|---|---|
| R3D-10: user keeps V2 although V3 is considered | Choosing a **lower ordered** version is not a code choice: the user rolls back V3 (rejection of V3's exact content, recovered target V2). Among unordered copies, keep V2 installs exactly V2 because V3 is in `considered` (§5.3), not by ordinary ordering |
| R3D-10: a later unconsidered higher V4 | Keep: V4 ends the keep and installs (ordinary update). Rollback: V4 does **not** end the recovered-target constraint by itself (Q-81 PENDING OWNER default). The GUI offers "Use 2.2", an explicit code choice whose `known` covers the rejection (§4.2); "Allow again" lifts the rejection (rollback.md §2.2) |
| R3D-16: keep V2 / reject V3, then resume with no allow-again | Resume ends the keep and declines nothing. V3 stays rejected by its rejection record. Target: V2 (newest non-rejected) |
| R3D-16: same, plus a new V4 | Target V4 (ordinary, non-rejected) |
| R3D-16: same, plus a replay of V3 | V3 is still rejected; never installed |

Tests T-R4D10-keep-considered, T-R4D10-resume-declines-nothing (with and
without V4, plus V3 replay), T-R4D10-unusable-metadata-no-override and
T-R4D10-interface-advisory are in ops.md §7.

### 5.5 `decide` freshness

`decide(item, option, shown, now: &FreshEvidence)` recomputes the decision
item from **current** evidence (ledger, admitted states, local inventory) at
write time and compares its `EvidenceToken` with `shown`. Any difference
gives `Changed`. Runtime holds the cycle lock from recomputation to the
record's read-back, so no learning step runs in between. Comparing only two
stale UI values is not enough (cross-packet table).

---

## 6. Admission order

`admit` runs in this order for every member enrollment:

1. Historical membership update (§1.10).
2. Retirement barrier: Retired(E) and Cutoff(E) from the ledger.
3. Code-state admission (slice watermarks), skipping seqs > Cutoff(E).
4. Settings-state admission with its own watermark, the same cutoff.
5. Currentness, confirmations, candidates and targets use only the result.

Retirement is therefore validated before any currentness, publication or
target use. Watermarks of a retired or re-enrolled enrollment are kept.

---

## 7. Diagnostics export boundary (R4D-12)

Shared rule for every design: an exported field is either a closed enum, a
number, a hash prefix the user can uncheck, or an addon folder name the
user can uncheck (Q-64 PENDING OWNER). **Never exported:** version labels,
`.toc` titles or notes, record `title`s, settings content, file paths
outside the app's own relative layout, account, realm or character names,
and any other source text. Version information is exported only as a
normalized class (`numeric(n parts)`, `date`, `text`, `none`). Details in
ops.md §5.

---

## 8. Technical caps and product gates

Technical caps are engineering defaults. They protect the process and may
be tuned by engineering without owner approval; they never suppress a
user-visible notice. Product gates change what the user experiences and
need owner approval; unapproved ones are **PENDING OWNER**.

| Value | Class | Status |
|---|---|---|
| Record 64 KiB, index chunk 256 KiB / 1,024 refs, ledger segment 4 MiB, recovery point `/2` 256 KiB | technical cap | engineering default |
| `Known.seen` ≤ 512, `extra` ≤ 128, `considered` ≤ 64, folders per record ≤ 256, ack items ≤ 256 | technical cap | engineering default |
| Ledger large-history diagnostic at 100,000 entries | technical cap | engineering default |
| Update check: connect/read timeouts, 256 KiB response limit, redirect limit | technical cap | engineering default (ops §4) |
| Diagnostics export: size and row limits, history list lengths | technical cap | engineering default (ops §5) |
| `notices.json` 64 KiB, notification history length | technical cap | engineering default (ops §3) |
| 100 MiB background memory; sync within 10 s | product gate | approved (AGENTS, R-14) |
| Removal settle 2 scans / 30 s | product gate | approved (Q-41) |
| 72 h "not heard from" emphasis | product gate | approved (Q-27) |
| Settings saved-file size cap | product gate | **PENDING OWNER** (Q-77); settings sync blocked until set |
| 6 h repeat suppression of an unchanged notice | product gate | **PENDING OWNER** (Q-85); never applies to `DataAtRisk` |
| `DataAtRisk` and the notification toggle | product gate | **PENDING OWNER** (Q-86): respects the toggle; always shown in the app |
| 24 h automatic update-check interval | product gate | **PENDING OWNER** (Q-87) |

---

## 9. Shared surfaces: phase C ownership and build order

Each shared surface has exactly one implementing owner. Dependents consume
its API; they do not extend the same records or runtime methods.

| Shared surface | Owner packet | Consumers |
|---|---|---|
| C-AUTH: `authority` module (family, decode, `AuthorityRef`, index chain, ledger segments, `Known`, validation, learning, mirroring, repair, capabilities) | **R4-core-removal** | settings, rollback, ops |
| `records` majors (`record!` change), `ApplyJournal/2`, `RecoveryPoint/2`, `PendingReport/2`; `plan`/`exec` journal dispatch; adapter and runtime `effects` journal and authority I/O | **R4-core-removal** | settings, rollback |
| `target::compose` (§4) and removal kinds | **R4-core-removal** | rollback, ops |
| Historical membership, retirement kind and barrier, `admit` order (§6) including the settings watermark | **R4-core-ops** | all |
| Code-choice kind (§5), `decide` freshness, `status`/`view`/`explain` integration, runtime `service`/`control` cycle, diagnostics boundary (§7) | **R4-core-ops** | all |
| `ids` additions: `SettingsUnit`, `Identity`, `CheckpointRef`, `SettingsRoot`; NFC/case diagnoses (§3) | **R4-core-settings** | removal (settings cleanup), ops |
| Settings module, settings exec ops, policy and revision kinds | **R4-core-settings** | ops |
| Rollback: `reject` module, rejection and lift kinds, recovered-target constraint | **R4-core-settings** | ops |

**Build order:**

1. **R4-core-removal foundation first:** C-AUTH, C-JRN and `target::compose`
   skeleton, with the golden fixtures T-C01 to T-C07. Nothing else merges
   before it.
2. Then in parallel: **R4-core-ops** (retirement barrier, admission, code
   choice, decide) and the rest of **R4-core-removal** (removal kinds,
   remove ops, acks).
3. Then **R4-core-settings** (ids, settings, then rollback), consuming the
   merged C-AUTH, C-JRN, admission and compose.
4. Then integration: status/view/explain and runtime wiring by R4-core-ops,
   and the cross-feature tests.

---

## 10. Contract tests

Owned by R4-core-removal unless noted. Each is a behavioural test with
fixed bytes where stated.

| Test | Checks |
|---|---|
| T-C01-golden-bytes | one golden file per kind and per index chunk: exact bytes, schema/major, file name = sha256; decode round-trip |
| T-C02-retired-forms-rejected | `id` fields, `{id,hash}` refs and a domain prefix are not produced; a ref of wrong kind is `invalid(wrong_reference)` |
| T-C03-bad-copy | a file whose bytes do not match its name is skipped and another good copy is used |
| T-C04-repair-own-only | own bad copy is replaced via temp + verify + rename; a good own file is never overwritten; a peer's bad copy is untouched |
| T-C05-journal-v1-golden | code-only journal and recovery point encode byte-identical to slice fixtures |
| T-C06-v2-under-slice | a `/2` journal (settings-only and mixed) fed to slice `recover_interrupted` gives `NeedsHelp`, journal kept, applies blocked; slice `decode_points` keeps `/2` points; same after a crash in a settings-only op |
| T-C07-pending-report-v2 | an old state never contains new report kinds; a `/2` pending report under the slice decoder is replaced, not misread |
| T-C08-known-bound | re-add after 1,024 live removals of one folder fits in 64 KiB and supersedes all of them; a concurrent removal is not covered |
| T-C09-ledger-segments | a ledger past 16 MiB in 4 MiB segments reloads, re-derives validity and keeps every entry; memory per entry measured |
| T-C10-retirement-barrier | three issuers: A retires B, C retires A with lower cutoff; both delivery orders; B stays retired, A retired, B's states above cutoff never admitted; master and stale decisions included (owner: R4-core-ops) |
| T-C11-identity-collisions | NFC collision and case-only difference on NTFS and APFS fixtures give the named diagnoses; neither identity is synced (owner: R4-core-settings) |
| T-C12-index-chain | replacement of a partial head chunk; a missing middle chunk makes the member's index unreadable without forgetting learned refs; `total` mismatch detected |
| T-C13-historical-membership | a record from an enrollment no longer in `member.json` stays valid via `previous_enrollments` or state file names (owner: R4-core-ops) |
| T-C14-compose-precedence | each adjacent pair of levels in §4.1 (owner: R4-core-removal; rollback and choice rows by their owners) |
| T-C15-decide-fresh | a record learned between showing and choosing gives `Changed`; no write happens (owner: R4-core-ops) |
