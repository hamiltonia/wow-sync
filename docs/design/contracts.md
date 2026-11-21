# Shared contracts: authority records, journals, identities and target composition

Status: **run-004 design, packet
[R4-design-fix](../runs/run-004/packets/R4-design-fix.md), revised by
R4-design-fix2** after the failed
[R4-design-recheck](../runs/run-004/evidence/R4-design-recheck.md). It
fixes R4D-1 and R4D-10 of the
[R4-design-review](../runs/run-004/evidence/R4-design-review.md), the
shared parts of R4D-2 to R4D-14, and the recheck findings R4D-1, R4D-4,
R4D-10, R4D-15 to R4D-17 and R4D-21 to R4D-22. **Not implemented.** Choices
marked **PENDING OWNER** use the conservative default in
[questions.md](questions.md). They are not owner decisions.

**This file is the only definition of:**
- the authority record family;
- file naming, hashing and references;
- issuer validity and historical membership;
- mirroring and repair;
- capabilities and downgrade;
- the apply journal and recovery point schemas;
- stable addon and settings identities;
- target composition, including the code-choice family;
- the phase C packet map, file ownership and extension points (§9).

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
| `part` | `wowsync.authority-part/1` | this file §1.9 | continuation items of a long list field of another record; meaningless on its own |

There is no other authority kind. The run-003 "decision" objects, the
removal decision kinds and any settings-checkpoint rejection are **retired**
designs and are not part of the family. Every kind's body DTO, wire
encoding and well-formedness (V-KIND shape) rules are implemented by the
foundation packet R4-core-contracts (§9); the owning design defines them.

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
  key, and empty for policy, retirement and part. It scopes a deferral while the
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

Records that end or override earlier records (re-add, code choice,
rejection, lift, settings policy and revision) carry a causal summary
instead of an unbounded list of hashes:

```json
"known": {
  "seen": [ { "enrollment_id": "…", "seq": 57 } ],
  "extra": [ "<sha256>" ],
  "parts": [ "<part hash>" ]
}
```

- Record r is **covered** by `known` iff `seen[r.issuer.enrollment_id] ≥
  r.issued_at_seq`, or `r.hash ∈ extra`, where `seen` and `extra` are the
  inline entries **plus** the entries of every part reachable from
  `parts` (§1.9). For an enrollment named in several places, the largest
  `seq` counts.
- A writer may claim `(E, s)` only if it has read E's index chain up to an
  admitted state of seq ≥ s **and** every record E issued with
  `issued_at_seq ≤ s` is in its ledger as `valid` or `invalid` (none
  `pending`). Because a record's `issued_at_seq` is the first own state
  listing it, reading E's index at seq s yields every E-issued record up to
  s. A writer's own entry is at most its last **published** seq, so a
  record never covers records issued in the same act (same reserved seq).
- `extra` covers individual records learned only as mirrors, before (or
  without) their issuer's chain being readable (§1.12). A writer whose
  issuer indexes are unreadable puts every learned record of that issuer in
  `extra`.
- There is **no** limit on the number of enrollments or extras and no
  `GroupHistoryTooLarge` state. Entries that do not fit inline move into
  parts (§1.9). `seen` and `extra` are sorted and unique per container;
  the same enrollment or hash in two containers is allowed and harmless.
- Never compare seqs of different enrollments. `Known` only compares a
  record's own issuer seq with the summary's entry for that issuer.

Supersession in every family uses coverage: "X supersedes Y" means X is
valid, X and Y concern the same scope element (a **folder key** for code
records, §4.4; a settings unit for revisions; the group for policies), and
X's `known` covers Y. It is evaluated **per scope element**: X ends Y only
for the folders both name, never for Y's other folders (R4D-21). A record
written concurrently with Y (X's writer had not learned Y) does not cover
it. This replaces the unbounded `supersedes`, `knows` and
`considered`-as-supersession lists of the drafts (R4D-4: 1,024 superseded
removals needed 68,624 bytes).

### 1.9 Bounded record sizes and parts (R4D-4)

Every record file is at most 64 KiB. Writers never truncate and never
refuse because a list is long. Two semantics-preserving mechanisms exist:

**Splitting** into several records, for kinds whose meaning is a union of
independent items:

| Kind | Inline bound | Split rule |
|---|---|---|
| removal, readd | ≤ 256 folders | several records with the same evidence, issuer act and reserved seq. Merging is a union per folder, so the split is equivalent (removal.md §3.2) |
| ack | ≤ 256 `(removal, folders)` items | several acks; obligations are per item (removal.md §3.4) |
| lift | ≤ 64 `lifts` | several lifts; each ends exactly the rejections it names (rollback.md §2.2) |

**Parts** for list fields that belong to one indivisible record (a full
policy snapshot, a choice's candidate list, a causal summary). A
**partable** field `L` holds its first items inline and may name
continuation parts in a sibling field `L_parts: [part hash]` (for `known`,
the nested `known.parts`):

```json
{
  "schema": "wowsync.authority-part/1",
  "group_id": "…",
  "of": "known|considered|exclude|allow_again|policy_folders|revision_parents",
  "items": [ "…items of the parent field's item type…" ],
  "more": [ "<part hash>" ]
}
```

- A part has no issuer. It is content-addressed, stored, listed (kind
  `part`), mirrored and repaired exactly like a record (§1.2, §1.6, §1.12).
  Its validity is V-BYTES, V-GROUP and V-PART: `of` is a known field name,
  every item has that field's item type and passes that field's item rules,
  `items` is sorted and unique, `more` ≤ 16 hashes.
- The value of a partable field = inline items ∪ items of every part
  reachable through `L_parts` and `more`. The writer computes it; a reader
  resolves it before validating the parent. A part reachable twice, or a
  depth above 8, makes the parent `invalid(bad_part)`; a part of the wrong
  `of` gives `invalid(wrong_reference)`.
- **V-REF covers parts:** the parent stays `pending(missing_reference)`
  until every reachable part is in the ledger. A pending record counts as
  not known (§1.11), so a half-delivered record never acts on a partial
  list.
- Duplicates across containers: for set-valued fields the union is
  idempotent. For `policy_folders` the same folder key twice (inline or in
  any part) makes the policy `invalid(policy_body)` (settings.md §3.2).
- Writer rule: fill inline items until the parent would exceed 48 KiB,
  then put the remainder in parts of at most 60 KiB, fan-out ≤ 16 per
  level. Depth 8 with fan-out 16 holds far more than any group can reach,
  so no product case hits the depth bound; it exists only to stop
  malicious input (§8).

| Kind | Partable fields | Fixed part |
|---|---|---|
| readd, code-choice, rejection, lift, settings-policy, settings-revision | `known` | — |
| code-choice | `considered`, `exclude`, `allow_again` (§5.2) | envelope, `keep` |
| rejection | `considered` (same item type as code-choice, §5.2) | envelope, `rejected`, `recovered` |
| settings-policy | `folders` (as `policy_folders`) | envelope, `master` |
| settings-revision | `parents` (as `revision_parents`) | envelope, unit, checkpoint, `policy` (≤ 8) |
| retirement | — | about 5 KiB: envelope, subject, cutoff, `retirements_known` ≤ 64 (§1.10) |

There is no `TooManyCandidates`, `GroupHistoryTooLarge` or
`TooManySettingsHeads` state. Tests: T-C08, T-R4D4-1024-issuers-readd,
T-R4D4-mirror-only-readd, T-R4D4-full-policy (§10).

### 1.10 Historical membership and the retirement barrier (R4D-11)

**Historical membership.** Core keeps a durable, grow-only local set of
`(computer_id, enrollment_id)` pairs ever proved to be members of the
group:

- the current `member.json` of each member directory;
- `member.json.previous_enrollments` (new additive field, written at
  re-enrollment, grow-only);
- the enrollment ids in state file names of that member directory.

(A retirement's `retired` subject must already be in this set; a
retirement never adds a member.)

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
  "retirements_known": [ "<hash of a retirement of the issuer>" ],
  "written_unix_ms": 1767225600000
}
```

`retirements_known` lists every retirement **of the issuer's own
enrollment** that the issuer had learned when writing (sorted, unique,
≤ 64; V-REF with kind `retirement`; an entry whose `retired` is not the
issuer is `invalid(wrong_reference)`). A correct app never writes it
non-empty: it stops publishing once it learns its own retirement (Q-60
PENDING OWNER default). The field exists so that a retired enrollment that
keeps acting **knowingly** (an older or faulty app) cannot retire others.
Plain hashes can only name records that existed before, so these
references never form a cycle.

**The barrier.** For enrollment E:

1. **Status of a retirement record R** is three-valued and is a pure
   function of the ledger (no stored bit, no arrival order). Records R
   that fail V-BYTES are not known at all (their subject is unknown); an
   R that fails V-GROUP, V-ISSUER or a V-REF kind check is `invalid` and
   has no effect. For every other R, over D = `R.retirements_known`
   (each a retirement of R's issuer):

   | Status | Condition |
   |---|---|
   | **Void** | some d ∈ D is **Effective** (R's issuer wrote R knowing it was itself retired) |
   | **Effective** | every d ∈ D is **settled not effective**: Void, or `invalid` |
   | **Unresolved** | otherwise: no d is Effective and some d is missing (`pending(missing_reference)`), unreadable, or Unresolved |

   A dependency d that is `invalid(not_a_member)` counts as Unresolved,
   not as settled, because membership facts can still arrive (§1.10
   historical membership is grow-only). Plain hashes only name earlier
   records, so D is a finite acyclic DAG and the status is well defined
   by induction over it.
2. **Monotonicity (R4D-25).** Learning a record can only turn a missing
   or Unresolved dependency into a settled one, so a status only moves
   Unresolved → Effective or Unresolved → Void, never Effective ↔ Void.
   By induction: a settled status depends only on settled statuses of
   its dependencies, which never change.
3. **Retired(E)** ∈ {Yes, No, Unresolved}: **Yes** iff some retirement of
   E is Effective; otherwise **Unresolved** iff some retirement of E is
   Unresolved; otherwise **No**. Yes is final: an Effective retirement
   stays Effective (rule 2), so nothing later un-retires E (R4D-11). An
   Unresolved E is **never shown or treated as retired** and never as
   free of retirement; it is *held* (rule 6).
4. **Cutoff(E)** = the **minimum** `cutoff_seq` over the Effective
   retirements of E. Learning another retirement can only lower it. Every
   record of E is therefore only ever valid → invalid, never invalid →
   valid: a record above any effective cutoff is never authority, however
   many later retirements with higher cutoffs arrive (R4D-16, D-21 kept
   Q-52: post-retirement actions are not authority).
5. Every record of E other than a retirement is valid only if
   `issued_at_seq ≤ Cutoff(E)` (V-RET, §1.11). A retirement record of E
   is governed by rule 1 instead, so a retirement E wrote before learning
   of its own retirement stays effective. E's states with
   `seq > Cutoff(E)` are not admitted (§6).
6. **Held (Unresolved).** Let HoldCut(E) = the minimum `cutoff_seq` over
   E's Effective **and** Unresolved retirements. Records and states of E
   with seq > HoldCut(E) are `pending(retirement_unresolved)`: not
   authority, not candidates, not confirmations, and the admission
   watermark does not advance past HoldCut(E) for them, so they can still
   be admitted if every Unresolved retirement turns Void. Everything else
   about E stays as if it were a member: currentness still **awaits** E
   (it counts in `awaiting`, removal settling and confirmation counts, so
   holding only delays), E is not listed under Retired, and no decision
   that depends on E being retired is made (no master reassignment
   prompt, no dropping of E from awaited sets). Content already applied
   from E's earlier admitted states stays installed with its recovery
   points, exactly as for Q-52; holding never removes anything. A
   computer whose own enrollment is held stops publishing and applying
   group changes until it is resolved, like a retired one (ops.md §2.4,
   Q-60 PENDING OWNER default), but it is not shown as retired and keeps
   its unwritten reserved decisions pending (R4D-31). It shows "Another computer may have
   retired this one. Waiting for the rest of the group history."
7. A retired enrollment only becomes active again through explicit
   re-enrollment, which is a new enrollment id (A-45, R-03).

**R4D-25 trace** (T-R4D25-partial-retirement-dag). All issuers and
subjects are historical members; each reference correctly names a
retirement of its issuer:

| Record | Issuer → subject | `cutoff_seq` | `retirements_known` |
|---|---|---|---|
| R | A → B | 10 | S |
| S | C → A | 20 | T |
| T | D → C | 30 | U |
| U | E → D | 40 | ∅ |

| Step (delivery) | Statuses | Retired / held | Currentness and publishing |
|---|---|---|---|
| Complete-first: R, S, T, U together | U Effective; T Void; S Effective; R Void | B **No**; A **Yes**, cutoff 20; C No; D **Yes**, cutoff 40 | B fully admitted and awaited; A's records > 20 invalid, A not awaited, A stops publishing when it learns S; D likewise above 40 |
| Partial 1: R and S only | S Unresolved (T missing); R Unresolved (S Unresolved) | B **Unresolved**, HoldCut 10; A **Unresolved**, HoldCut 20 | B's states > 10 and A's > 20 held, both still awaited; neither shown retired; no master reassignment; B and A themselves pause publishing; nothing is uninstalled |
| Partial 2: + T | T Unresolved (U missing); S, R Unresolved | C **Unresolved**, HoldCut 30; A, B as before | as partial 1, plus C's records > 30 held |
| Partial 3: + U | U Effective → T Void → S Effective → R Void | B **No** (its held states are admitted); A **Yes**, cutoff 20 (its held records become invalid); C **No** (released); D **Yes**, cutoff 40 | identical to complete-first |
| Restart after partial 1 | recomputed from the ledger: same as partial 1 | same | same; the held seqs are still admissible because the watermark did not pass them |
| Other order: T, U first, then R, S | U Effective, T Void at once; then S Effective, R Void on arrival | B never Unresolved or Yes; A Yes | identical final result |

At no step is B Retired = Yes, and no status flips between Effective and
Void; R moves only Unresolved → Void and S only Unresolved → Effective.
A missing dependency never settles by itself: if T never arrives, B and A
stay held and Details explains "Waiting for a retirement record from
<computer>". The user can still act: re-enrollment, or a new retirement
of A or B from another computer (empty `retirements_known`, so it is
Effective at once).

**Consequences, all deterministic:**

| Trace | Result |
|---|---|
| R4D-11: A retires B (cutoff 10) at A-seq 20; C, unaware, retires A with cutoff 15; both orders | A's retirement of B is effective (A did not know of its own retirement), so B stays retired; A is retired with cutoff 15. A's other records above 15 are invalid |
| R4D-16 (1): A retires B at cutoff 10; B's seq-11 removal is invalid; C's retirement of B with cutoff 12 arrives later | Cutoff(B) = min(10, 12) = 10. The removal stays invalid (T-R4D16-later-cutoff-no-reauth) |
| R4D-16 (2): B is retired; B, **knowing** that (an older app that keeps publishing), writes a retirement of A | void: its `retirements_known` names B's effective retirement. A is not retired (T-R4D16-retired-issuer-retirement-void) |
| B, offline and **not knowing** it was retired, concurrently retires A | effective: concurrent mutual retirement retires both (**Q-61 PENDING OWNER** default). No rule can distinguish this from the R4D-11 trace without trusting clocks |
| A lower second cutoff for B arrives after others acted on B's records in between | those records become invalid; members keep their recovery points and the GUI explains (Q-52, owner-accepted) |

There is no `authority_known` list and no grounded labelling. The GUI says,
for mutual retirement, "Laptop and Den PC retired each other. Join again
on the computer you still use."

### 1.11 Validation

A ledger entry is `valid` only if all rules hold. Kind-specific rules add
to these and are named `V-<KIND>-n` in the owning design.

| Rule | Check | Otherwise |
|---|---|---|
| V-BYTES | A good copy decodes (§1.4) | no good copy: `pending(unreadable)`; unknown schema/major: `pending(unknown_schema)` |
| V-GROUP | `group_id` is this group | `invalid(other_group)` |
| V-ISSUER | the issuer pair (and for a retirement also the `retired` pair) is in historical membership (§1.10) | `invalid(not_a_member)` |
| V-REF | every referenced hash (including parts, §1.9) is in the ledger with the kind the field requires | absent: `pending(missing_reference)`; wrong kind: `invalid(wrong_reference)` |
| V-RET | the record **is a retirement** (governed by §1.10 rule 1: void retirements are `invalid(void_retirement)`, unresolved ones `pending(retirement_unresolved)`), **or** Retired(issuer) = No, **or** `issued_at_seq ≤` Cutoff(issuer) when Retired = Yes, **or** `issued_at_seq ≤` HoldCut(issuer) when Retired = Unresolved | Retired = Yes: `invalid(after_retirement)`; Retired = Unresolved: `pending(retirement_unresolved)` (§1.10 rule 6) |
| V-KIND | the owning design's kind rules | `invalid(<kind reason>)` |

V-RET is evaluated after V-REF because voidness reads `retirements_known`.
The predicate is the exact one the implementation tests
(T-R4D15-vret-predicate): an ordinary record of a retired issuer above the
cutoff is invalid; the same record at or below the cutoff is valid; a
retirement of a retired issuer is valid unless void; an ordinary record
of an issuer whose retirement is Unresolved is pending above HoldCut and
valid at or below it.

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
2. For each unknown hash, from a ref or referenced by a learned record (a
   part, a `retirements_known` entry, an ack's removal, a revision parent),
   read a good copy: first the referencing member's
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

**Only a pure code apply stays `/1`, byte-identical** to the slice (golden
fixture T-C05). A `/1` journal is written iff `kind = apply`, every folder
action is `install` or `replace`, and there are no removals, settings ops or
pending records. Everything else, including an ordinary update that also
drops a folder (removal.md §5.1), is written as `/2`:

| Field | `/1` | `/2` |
|---|---|---|
| `op`, `group_id`, `started_unix_ms`, `package`, `target_hash`, `phase`, `from_phase`, `reason`, `halted_by_game` | as slice | as slice |
| `folders: [JournalFolder]` | **non-empty** (slice validator) | may be empty only when `settings_ops` is not |
| `kind` | absent (means `apply`) | `apply \| remove \| restore \| rollback \| settings \| settings_cleanup` |
| `removals: [hash]` | absent | removal records whose folders this op removes |
| `pending_records: [{hash, bytes}]` | absent | ack, re-add, removal, rejection or revision bytes, fixed at journal creation, written after verify |
| `settings_ops: [SettingsFileOp]` | absent | settings file moves, below |
| `FolderAction` | `install \| replace` | adds `remove` (live folder moved to recovery, nothing installed) |

**Kinds and allowed contents** (the `/2` validator, V-JRN):

| `kind` | Folder actions | `removals` | `settings_ops` | Notes |
|---|---|---|---|---|
| `apply` | install, replace, remove | required iff any `remove` | allowed | update, update-with-drop, add |
| `remove` | remove only, ≥ 1 | non-empty | allowed | removal (removal.md §5.2) |
| `restore` | install, replace | empty | allowed | GUI restore of a recovery point |
| `rollback` | install, replace, remove | required iff any `remove` | allowed | rollback.md §3, including folder-changing rollback (R4D-20) |
| `settings` | none | empty | non-empty | settings apply/restore (settings.md §5) |
| `settings_cleanup` | none | empty | non-empty | uninstall cleanup when code is already absent (removal.md §6.3) |

Further V-JRN rules: every `remove` action's folder key is a folder of some
listed removal hash, and each listed hash is either in the ledger as a
valid removal or among this journal's own `pending_records` (rollback
writes its own removal, rollback.md §3.1). `pending_records` hashes equal
`sha256` of their bytes, and each decodes as an authority kind. Violations
are `invalid` and make the journal undecodable (§2.4 row 1).

**Folder actions and slots.** `install` and `replace` use the slice slot
table (stage, swap, verify) unchanged. `remove` has no staged content: it
moves the live folder to its recovery slot and verifies absence (slot
table in removal.md §5.3). Settings ops use the slot table in settings.md
§5.2. One journal may mix all three; `exec::recover_interrupted` resumes
or rolls back each action by its own table and the journal phase.

```rust
pub struct SettingsFileOp {
    pub unit: SettingsUnit,            // §3.2
    pub live_rel: SettingsPath,        // WTF-relative, validated
    pub expected_live: ExpectedLive,   // hash or absent, re-checked before the rename
    pub staged: Option<Sha256>,        // content to install, or None for move-out only
    pub recovery_slot: RecoverySlot,   // where the live file goes
}
```

**`pending_records` timing (R4D-1).** The writer reserves `next_seq`,
builds every pending record (with that `issued_at_seq`, its `known` and
parts) and stores the bytes in the journal **at creation, before the first
live rename**. The bytes never change afterwards. At finalize, after
verify, they are written to `authority\` (idempotent: same hash, same
bytes) and indexed; then the journal is deleted. If the op rolls back,
they are discarded and never published. This is safe because nothing of
this computer is published while any journal exists
(`journals_remaining` blocks publication), and `recover_interrupted` runs
before any publication on start: a crash anywhere either finishes the op
and writes exactly the reserved bytes, or rolls it back and writes none.

**Example: an update that drops a folder** (`apply/2`):

```json
{
  "schema": "wowsync.apply-journal/2",
  "op": "8f1c…", "group_id": "…", "started_unix_ms": 1767225600000,
  "kind": "apply",
  "package": "Details", "target_hash": "c0de…",
  "phase": "staged", "from_phase": null, "reason": null, "halted_by_game": false,
  "folders": [
    { "key": "details", "action": "replace", "expected_live": "aa11…", "staged": "bb22…", "recovery_slot": "r/8f1c/details" },
    { "key": "details_streamer", "action": "remove", "expected_live": "cc33…", "staged": null, "recovery_slot": "r/8f1c/details_streamer" }
  ],
  "removals": [ "dd44… (folder_drop removal of details_streamer)" ],
  "pending_records": [],
  "settings_ops": []
}
```

**Example: uninstall cleanup with a pending ack** (`settings_cleanup/2`):

```json
{
  "schema": "wowsync.apply-journal/2",
  "op": "91aa…", "group_id": "…", "started_unix_ms": 1767225900000,
  "kind": "settings_cleanup",
  "package": "WeakAuras", "target_hash": null,
  "phase": "staged", "from_phase": null, "reason": null, "halted_by_game": false,
  "folders": [],
  "removals": [],
  "pending_records": [ { "hash": "ee55…", "bytes": "<canonical removal-ack/1 bytes: result absent, settings paired>" } ],
  "settings_ops": [
    { "unit": { "folder": "weakauras", "identity": { "account": "ACCOUNT1" } },
      "live_rel": "Account/ACCOUNT1/SavedVariables/WeakAuras.lua",
      "expected_live": { "hash": "ff66…" }, "staged": null,
      "recovery_slot": "r/91aa/s0" }
  ]
}
```

(`target_hash` and `staged` are `null` where nothing is installed; in `/2`
`target_hash` is optional. Values shown with "…" are abbreviated.)

**Crash cases** (each is a T-C06c case, run against every slot):

| Crash point | Recovery |
|---|---|
| after journal creation, before any rename | roll back: delete the journal; no record written |
| mid folder actions or settings ops | each action resumes or rolls back per its slot table, journal phase decides direction |
| after verify, before or during writing `pending_records` | finish: write the reserved bytes (idempotent), index them, delete the journal |
| after `pending_records` written, before journal delete | finish: rewrite is a no-op, delete the journal |

### 2.3 `wowsync.recovery-point`

Code-only points stay `/1`, byte-identical. Points with settings content or
a removal are `/2` (limit raised to **256 KiB**):

| Field | `/2` addition |
|---|---|
| `kind` | `apply \| remove \| restore \| rollback \| settings \| settings_cleanup` |
| `removals: [hash]` | the removal records that moved the code (shown as "removed by …") |
| `settings: [SettingsRoot]` | every settings unit this op applies to, captured or missing (below) |
| `paired` | derived from the code part and `settings` (table below) and validated, never chosen freely |
| `code_package_hash` | the package hash of the code part, for paired selection (settings.md §4.6) |

```rust
pub struct SettingsRoot { pub unit: SettingsUnit, pub state: RootState }
pub enum RootState {
    Captured { checkpoint: CheckpointRef, location: Location /* local | provider */ },
    Missing { reason: MissingReason },
}
pub enum MissingReason {
    Paused, UnknownOwner, SharedFile, Unreadable, TooLarge,
    IdentityCollision, NotIncluded,
}
```

`settings` lists **every** applicable unit of the op (settings.md §1.1),
so a reader can always tell "nothing to capture" from "could not capture".

**`paired` is a function** (V-POINT rejects any other value):

| Code part (`replaced`/`installed`/removed) | `settings` | `paired` |
|---|---|---|
| non-empty | empty (no applicable unit) | `not_applicable` |
| non-empty | all `Captured` | `paired` |
| non-empty | some `Captured`, some `Missing` | `partial` |
| non-empty | non-empty, all `Missing` | `code_only` |
| empty | ≥ 1 `Captured` | `settings_only` |
| empty | none `Captured` | **no point is written**: the op takes nothing, so there is nothing to recover (the no-op path, removal.md §1.6 step 3b) |

**Example: a code-only removal point** (`recovery-point/2`):

```json
{
  "schema": "wowsync.recovery-point/2",
  "op": "8f1c…", "group_id": "…", "kind": "remove",
  "package": "OldAddon", "code_package_hash": "a1b2…",
  "replaced": [ { "key": "oldaddon", "tree": "c3d4…", "slot": "r/8f1c/oldaddon" } ],
  "installed": [],
  "removals": [ "e5f6…" ],
  "settings": [],
  "paired": "not_applicable"
}
```

Retention: current plus two previous per package (code) and per settings
unit, plus visible pinned exceptions (R-10). The `Captured` roots of a `/2`
point are GC roots for local and provider checkpoints (settings.md §7).

### 2.4 Mixed-version safety

| Situation | Result |
|---|---|
| Slice app finds a `/2` journal | `exec::recover_interrupted` cannot decode it: `NeedsHelp(UnknownInterruptedState)`, journal kept, applies blocked (`journals_remaining`). Nothing is guessed |
| Slice app lists `/2` recovery points | `exec::decode_points` skips them, so the slice pruner never prunes them |
| Runtime decoders | **Correction (R4D-1):** the slice `service::journals` drops journals that fail to decode, so the slice GUI journal list does not show a `/2` journal, although core `journals_remaining` still keeps it and blocks applies and publication. The new runtime lists every undecodable journal or point as `UnknownJournal { file }` end to end (service, control, GUI) and never deletes it (T-C06b). The slice behaviour is accepted as is: it is safe, only less informative |
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
- A normalizing key never hides two distinct local identities. Phase C
  tests this with offline fixtures that simulate case-insensitive and
  normalization-insensitive lookup (T-C11). Behaviour on native NTFS and
  APFS volumes is a **future test obligation, NOT RUN** (R4D-22); no
  native-filesystem result is claimed.

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
| 2 | Rejection | rollback.md | a source whose content is the same release as an active rejection's `rejected` (§4.2 `same_release`) is never a candidate, never kept as a target |
| 3 | Local hold (`Held(RemovalObserved \| RemovalUnclear \| ReinstallObserved)`, settings hold) | removal.md, settings.md | nothing installed, replaced or removed |
| 4 | Merged removal | removal.md §3 | folder removed into recovery |
| 5 | Stale copy (owed removal) | removal.md §3.4 | that copy is never a candidate |
| 6 | Constraint: live code choice or recovered rollback target **for this folder** | §4.4, §5, rollback.md §3 | install exactly the constrained tree for this folder |
| 7 | Pending removal information | removal.md §4.4 | installs of hinted folders deferred |
| 8 | Ordinary selection | domain.md §6 | **whole-addon** selection (domain.md §6, `compare_package`, completeness) among sources not excluded (§5.3) and not rejected; the chosen whole result is projected per folder afterwards (§4.3) |

The settings master is **not** code authority. No settings record affects
levels 2 to 8.

### 4.2 Release identity and matching (R4D-23)

Two different questions use two different rules:

- **Which exact release is this?** (rejection, exclusion, `considered`,
  look-alike, recovered chain): **complete-content identity**, below.
  It never compares single folders.
- **Which record governs folder f?** (liveness, coverage, routing of a
  constraint's tree to f): **per folder**, §4.4.

**Content over a scope.** A recorded release entry E (a `keep`,
`considered`, `exclude`, `rejected` or `recovered` entry) belongs to a
record whose `folders` list is its **scope** S; an `exclude` entry
carries its own `scope` instead, because it is carried between records
(§5.2). The writer records E's
complete content over S: every key of S that E's copy had, with its
tree; a key of S that E does not list was **absent** from that copy
(V-CHOICE and V-REJECTION already require the entries' keys ⊆ `folders`;
E's `package_hash` is the package hash of exactly its listed folders). So
`content(E, k)` is a tree or `Absent` for every k ∈ S, and **unknown**
for k ∉ S.

A **source** m is one computer's complete installed copy: a member's
admitted state or the local inventory. `content(m, k)` is the folder's
tree, `Absent` if m does not have folder k, and **unknown** if m's folder
k is not `Ok` (unreadable, unstable, unsafe) or m's state is not
admitted. Sources are taken from complete states, never from a local
package grouping, so a computer that groups the folders differently
computes the same content.

A source has **no scope of its own**: it is a complete inventory that
answers any key asked of it (a tree, known `Absent`, or unknown for a
folder that is not `Ok`). Its other keys are never part of a release
identity (R4D-28).

**`same_release`** has two forms, distinguished only by the key set K:

- **Source against a recorded entry**, `same_release(m, E)`: K =
  scope(E) **only**. An unrelated installed or missing addon, or a
  folder outside E's scope, never affects the result.
- **Two recorded entries**, `same_release(X, Y)`: K = scope(X) ∪
  scope(Y). A key outside one entry's scope is unknown for that entry, so
  differently scoped entries that agree on their overlap are `Unknown`.
  Users never wait on such an `Unknown` from entries alone. They resolve
  it through actual sources compared against each entry by the first
  form (the recovered-chain row below).

| Result | Condition over K |
|---|---|
| `Same` | for **every** k ∈ K both contents are known and equal (`Absent` = `Absent`, tree = same tree) |
| `Different` | for **some** k ∈ K both contents are known and differ (`Absent` ≠ any tree) |
| `Unknown` | otherwise: some content is unknown and nothing known differs |

For a source against an entry, `Unknown` arises only from the source's
own not-`Ok` folders inside scope(E) or an unadmitted state. It
therefore resolves when the source's facts settle. Equal trees on common
keys never prove equality: a folder that one side has and the other
lacks is a difference. `Unknown` is never treated as `Same`; each user
states what it does with it. Every user except the recovered-chain step
compares a **source** with an entry:

| User | `Same` | `Different` | `Unknown` |
|---|---|---|---|
| Level 2 rejection (rollback.md §3) | source never a candidate or a target | not rejected by this record | source **waits**: not a candidate, not rejected, shown "Checking against a blocked version" |
| Exclusion (§5.3) | source never selected | not excluded | source waits, as above |
| `considered` (Q-49, `NewerAfterRollback`) | already considered: raises nothing | not considered | no decision or notice yet; waits |
| Recovered chain (rollback.md §3.1), entry against entry | the recovered entry is rejected: follow the chain | not rejected | decided by the actual copies: the sources that are `Same` as the recovered entry. If that set is **empty**, the outcome is `RecoveredUnavailable` (R4D-29). Otherwise the chain advances only if every copy in the set is rejected at level 2 (`Same` as an active rejection's `rejected`); a copy that waits makes the chain wait |
| Look-alike (rollback.md §3.3) | — (that is level 2) | look-alike if the label also matches | waits |

A source that waits is held exactly like an ineligible candidate
(domain.md §6): it never falls back to a lesser candidate and installs
nothing. Recorded entries are never compared through a local package
hash, so changed local grouping cannot change any result.

**Counterexamples (named tests, §10):**

| Test | Facts | Result |
|---|---|---|
| T-R4D23-added-folder-rollback | rejection scope {core, extra}; R1 `{core:U, extra:BAD}` rejected, R0 `{core:U}` recovered | R0 = `{core:U, extra:Absent}` is `Different` from R1: R0 is installed and is a source; the chain does not visit R0 again; a source with exactly `{core:U, extra:BAD}` is rejected |
| T-R4D23-dropped-folder-rollback | scope {core, old}; R1 `{core:U}` rejected (2.1 dropped `old`), R0 `{core:U, old:O}` recovered | R0 is `Different` (old: O ≠ Absent) and is installed with `old`; a source with `{core:U}` and no `old` is rejected; a source `{core:U, old:O2}` is `Different` and can only be a look-alike |
| T-R4D23-keep-resume-shared-component | keep V2 `{a:2, b:U}` declining V3 `{a:3, b:U}`, then resume (no allow-again); run with a and b grouped, ungrouped, and regrouped between keep and resume (a new folder c joins the package, so the resume's `folders` = {a, b, c} while the carried V3 entry keeps `scope` {a, b}) | the source holding V2 is `Different` from the exclude entry V3 and stays eligible; the target is V2 (a:2 and b:U); the V3 source is `Same` over {a, b} and is never selected, with or without c; identical on every grouping |
| T-R4D23-unknown-waits | as the first row, but the candidate's `extra` folder is `Unstable` | `Unknown`: neither installed nor rejected; it waits; once stable it resolves |
| T-R4D28-exact-source-unrelated-addon | rejection scope {core, extra}: `rejected` `{core:U, extra:BAD}`, `recovered` `{core:U}`. Source m1 = `{core:U, extra:BAD}`; source m0 = `{core:U}` (extra known `Absent`). Run each once with an unrelated addon `{z:Z}` installed on the source and once with z absent | m1 is `Same` as `rejected` (rejected at level 2); m0 is `Same` as `recovered` (a real copy of the recovered release, installed); z never changes either result; A-38 control: union the source's keys into K (every result becomes `Unknown`, test fails) |
| T-R4D28-regrouped-exclusion | carried exclude entry V3 with `scope` {a, b}, `{a:3, b:U}`; after regrouping the resume's `folders` = {a, b, c}; declined source `{a:3, b:U, c:C}`, kept source `{a:2, b:U, c:C}` | declined source `Same` over {a, b} (never selected); kept source `Different` (eligible, target); c ignored for the exclusion |

### 4.3 Ordinary selection is whole-addon (R4D-24)

Ordinary selection (level 8) is exactly domain.md §6 on the **whole
package**: candidates are the sources' complete package versions over the
package's full membership, minus sources that level 2 rejects, level 5
marks stale or §5.3 excludes (or that wait, §4.2), compared by
`compare_package` with the completeness predicate. Its result is decided
**before** any per-folder step:

- `Chosen(c)`: the whole release c is the ordinary target. Only then is c
  **projected** per folder: f's ordinary tree is `content(c, f)`.
- `NeedsDecision(Ambiguous(MembersDisagree))`, `PartialCandidate` or any
  other reason: an **addon-level** decision (§5.1). No folder of the
  package gets an ordinary tree; installed copies stay.

Per-folder liveness never picks folders from different sources. A
package can mix trees from different releases only through an explicit
record that names exactly those folders (a keep or a rollback whose scope
is a subset of the package, R4D-21): each such folder takes the
constraint's tree at level 6, the other folders take the projection of
the whole ordinary result. Such a deliberate partial override goes
through the normal apply with a recovery point and is undone by its own
"Change decision" (ops.md §1.5) or "Allow again…" (rollback.md §2.2).

Counterexample T-R4D24-no-hybrid (neutral hooks, no records): sources X
`{a:3.0, b:1.0}` and Y `{a:2.0, b:2.0}`, every folder `Ok`, all trees
different. `compare_package(X, Y)` = `Ambiguous(MembersDisagree)` →
`NeedsDecision(Ambiguous)` for the addon, with the two whole copies as
options. `{a:3.0, b:2.0}` is never planned; nothing is installed. Every
computer reaches this result, because packages come from grouping over
the union of all observations (domain.md §5), not from one computer's
local view.

### 4.4 Constraint composition, per folder (R4D-9, R4D-21)

Liveness and routing are evaluated **per folder key f**, never per
package or per record. Which release a source holds is never decided per
folder (§4.2).

**Constraint records** are valid code choices with `action = keep` and
active rejections (their `recovered` target). For each, `trees(X, f)` is
the tree X names for folder f (`keep.folders` or `recovered.folders`), or
absent.

**Liveness.** X is **live for f** iff X is valid and active, f ∈
`folders(X)`, and **no** valid code choice, rejection or re-add that also
names f has a `known` covering X (§1.8). A record that covers X for some
folders does not end X for the folders it does not name (R4D-21).

**The target of f:**

- **No live constraint for f:** f takes the projection of the whole
  ordinary result (§4.3); if that result is a decision, f is unchanged.
- **One distinct tree:** install exactly that tree for f, unless the
  constraint's release is rejected (§4.2 `Same`). Then follow the
  recovered chain (rollback.md §3.1) to the first non-rejected recovered
  release, else the projection of the whole ordinary result (§4.3).
- **A live constraint names f without a tree** (a keep or recovered target
  that lacked f, for example after a folder change): f has no constrained
  content; if it is installed it stays, and if a candidate supplies it the
  decision is `PartialCandidate`.
- **Several different trees** (concurrent choices or concurrent rollbacks
  on different computers): `NeedsDecision(ChoicesDiffer)` or
  `NeedsDecision(RollbackTargetsDiffer)`, one option per distinct tree
  set over the affected folders. The current copy stays installed
  meanwhile. Choosing writes one keep whose `known` covers all of them.

**Sources for a constrained tree.** A source supplies f's constrained
tree only if it is not rejected or waiting at level 2 as a whole (§4.2);
a rejected installed copy is never a source, even of an unchanged folder.

Constraints never order by clock. Tests: T-R4D9-concurrent-rollbacks and
T-R4D9-choice-vs-rollback (rollback.md §10); T-R4D21-partial-choice-over-choice,
T-R4D21-choice-over-rollback and T-R4D21-readd-over-rollback (§10), each
run with two different local groupings of the same folders.

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
| `ChoicesDiffer`, `RollbackTargetsDiffer` | §4.4 | one option per constrained content; **Decide later** |
| `RecoveredUnavailable` | a live recovered rollback target has no source left (rollback.md §3.1) | one **Use <version>** per available non-rejected candidate; **Decide later** |
| `LookalikeOfRejected` | a candidate has the version label of an active rejection but different content (rollback.md §3.3) | **Use this copy**; **Block this copy too** (a rejection, not a choice); **Decide later** |
| `NewerAfterRollback` (notice, not blocking) | a non-rejected source that is `Different` (§4.2) from every entry of the rejection's `considered` and is demonstrably newer than the recovered target (rollback.md §3.1) | **Use <version>** (a keep covering the rejection); **Stay on <recovered>** (dismiss) |
| `NewVersionAfterDecision` | a source that is `Different` (§4.2) from every `considered` entry of a live keep and is demonstrably newer than, or unordered against, the kept copy (§5.3, Q-49) | **Keep <kept>** (a new keep that lists the new copy in `considered`); **Use <new>** (a keep of the new copy); **Follow newest again** (resume); **Decide later**. The kept copy stays installed meanwhile |
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
  "reason": "ambiguous|partial_candidate|choices_differ|rollback_targets_differ|newer_after_rollback|new_version_after_decision|recovered_unavailable|lookalike_of_rejected",
  "action": "keep|resume",
  "folders": ["weakauras", "weakauras_options"],
  "keep": { "package_hash": "<sha256>",
            "folders": [ { "key": "weakauras", "name": "WeakAuras", "tree": "<sha256>" } ] },
  "considered": [ { "package_hash": "<sha256>",
                    "folders": [ { "key": "weakauras", "tree": "<sha256>" } ],
                    "holders": ["<enrollment id>"] } ],
  "exclude": [ { "package_hash": "<sha256>",
                 "scope": ["weakauras", "weakauras_options"],
                 "folders": [ { "key": "weakauras", "tree": "<sha256>" } ] } ],
  "allow_again": [ "<package_hash of an inherited exclude entry>" ],
  "known": { "seen": [], "extra": [], "parts": [] },
  "considered_parts": [], "exclude_parts": [], "allow_again_parts": [],
  "written_unix_ms": 1767225600000
}
```

`considered`, `exclude`, `allow_again` and `known` are partable (§1.9).

**Exclude scope (R4D-23).** An exclusion is carried from record to record
(Q-50), possibly into a record whose `folders` differ because grouping
changed. Each `exclude` entry therefore keeps the `scope` it was first
written with (the `folders` of that record), and §4.2 compares a source
against it over that scope only (the source-against-entry form): a key of
`scope` it does not list was `Absent`, a key outside `scope` is never
consulted, whatever the source has there (R4D-28). This is the only wire addition of
R4-design-fix3; `considered`, `keep`, `rejected` and `recovered` entries
are always written fresh over their own record's `folders`.

**The writer computes `exclude`** (Q-50: declined copies stay declined):

- *inherited* = the union of `exclude` entries over the choices this
  record's `known` covers whose `scope` intersects `folders`, carried
  verbatim with their `scope`;
- **keep:** `exclude` = inherited ∪ (`considered` − `keep`, each with
  `scope` = `folders`), minus every entry that the kept source is `Same`
  as (§4.2, over that entry's scope). Every copy the user saw and did not
  keep is declined, and the kept copy is never declined;
- **resume:** `exclude` = inherited − `allow_again`, with `allow_again` ⊆
  inherited package hashes. Resume keeps previous exclusions unless the
  user explicitly ticks "Allow <version> again".

**V-CHOICE** (conditional per action, R4D-10):

| Rule | `keep` | `resume` |
|---|---|---|
| `folders` non-empty, sorted, unique | ✓ | ✓ |
| `keep` | present; `keep.package_hash` = package hash of `keep.folders`; keys ⊆ `folders` | **absent** |
| `considered` | sorted, unique, **contains** the kept entry | sorted, unique, may be empty |
| `exclude` | sorted, unique by (`scope`, `package_hash`); `scope` non-empty, sorted, unique, ≤ 256; entry keys ⊆ its `scope`; no entry with `scope` = `folders` equal to `keep` | sorted, unique by (`scope`, `package_hash`); same `scope` rules |
| `allow_again` | **absent** | sorted, unique; disjoint from `exclude` hashes |
| every tree a plain hash, every `considered` folder key ∈ `folders`, every `exclude` `scope` intersects `folders` | ✓ | ✓ |

Violations are `invalid(choice_shape)` (T-R4D10-vchoice-per-action). The
validator cannot recompute *inherited* without the ledger, so `exclude`
is trusted as written; it can only narrow candidates, never authorize
unsafe content (§5.1).

### 5.3 Semantics

For each folder f, the choices **live for f** are those no later choice,
rejection or re-add naming f covers (§4.4).

- **Live keep:** a constraint (level 6) on f: install exactly the kept
  tree. Sources whose content over the choice's scope is the same release
  as a `considered` entry (§4.2 `same_release` = `Same`) never end it,
  whatever their version, and older candidates are ignored.
- **New information (Q-49):** a source that is `Different` from **every**
  `considered` entry and is demonstrably newer than, or unordered
  against, the kept copy (`compare_package` over the choice's scope)
  raises `NeedsDecision(NewVersionAfterDecision)`. A source that is
  `Unknown` against some entry raises nothing yet and waits (§4.2).
  The keep **stays**: the kept copy remains the target until the user
  chooses. Nothing is installed automatically and no record is written
  until then.
- **Live resume:** imposes no target. Whole-addon ordinary selection
  (§4.3) applies, **filtered by its `exclude`**: a source that is `Same`
  as an exclude entry over that entry's `scope` is never selected (Q-50), whatever its version,
  until a later choice that covers this resume drops it via
  `allow_again`. Folders of the source outside the entry's `scope` (for
  example a folder c that joined the package later) are not compared
  (T-R4D28-regrouped-exclusion). A source that only shares some folders' trees with an
  exclude entry is `Different` and stays eligible (the kept V2 of
  T-R4D23-keep-resume-shared-component).
- A live keep also filters by its `exclude` (relevant only if its kept
  tree becomes unavailable and §4.4 falls back to the ordinary result).
- **Rejection is separate:** exclusions are per-choice declines, ended by
  a later covering choice. Rejection (rollback.md) blocks exact content
  for the group until an explicit lift. "Allow again" on a rejection is
  the lift (rollback.md §2.2); "Allow again" on an exclusion is a resume
  with `allow_again`.
- **Supersession:** per folder (§4.4): X ends Y for f iff both name f and
  X's `known` covers Y.
- **Concurrent choices:** §4.4.
- **Precedence:** rejection > removal > choice > ordinary (§4.1). A keep of
  rejected content is inert; a removal removes the folder whatever the
  choice; a valid re-add ends the choices it covers for its folders.

**As-built (run-005, Q-88 option (a), D-26).** When the kept version is held
by no source (for example the owner replaced it locally with a different
one), the kept copy is synthesized from the keep record, so the order of any
unconsidered copy against it is unknown. Any such copy, of any order, raises
`NewVersionAfterDecision`. A newer copy held only by this computer stays
installed and is not published, and the decision stays open every cycle until
answered.

### 5.4 Retracing R3D-10, R3D-16 and Q-49/Q-50

No trace adds a rejection; every exclusion below is a code-choice
`exclude`.

| # | Trace | Result |
|---|---|---|
| 1 | Keep V2 while V3 is a candidate (V3 in `considered`) | target V2; V3 is in `exclude` |
| 2 | Same, then an unconsidered V4 appears, newer than V2 | `NeedsDecision(NewVersionAfterDecision)`; V2 stays installed (T-R4D10-new-version-after-keep) |
| 3 | Keep V2 declining V3, then resume (no allow-again) | the resume's `exclude` inherits V3; target V2, the newest non-excluded candidate (T-R4D10-resume-keeps-exclusions) |
| 4 | Trace 3, plus a new V4 | target V4 (ordinary, not excluded) |
| 5 | Trace 3, plus a replay of V3's exact content | that source is `Same` as the exclude entry: never selected |
| 6 | Trace 3, but resume with `allow_again = {V3}` | V3 is eligible again; target V3 (T-R4D10-resume-allow-again) |
| 7 | R3D-10: user wants V2 although V3 is demonstrably newer | that is a rollback (rejection of V3, recovered V2), or a keep listing V3 in `considered`; both hold until an explicit choice |
| 8 | A later unconsidered V4 after a **rollback** | V4 does not end the recovered constraint by itself (Q-81 PENDING OWNER default); `NewerAfterRollback` notice offers "Use 2.2" |
| 9 | Trace 3 where V2 `{a:2, b:U}` and V3 `{a:3, b:U}` share b (R4D-23) | V2's source is `Different` from V3 (a differs): eligible, target V2; the shared b:U never excludes V2 (T-R4D23-keep-resume-shared-component) |

Tests T-R4D10-new-version-after-keep, T-R4D10-resume-keeps-exclusions,
T-R4D10-resume-allow-again, T-R4D10-vchoice-per-action,
T-R4D10-unusable-metadata-no-override and T-R4D10-interface-advisory are
in ops.md §7.

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
2. Retirement barrier: Retired(E) ∈ {Yes, No, Unresolved}, Cutoff(E) and
   HoldCut(E) from the ledger (§1.10).
3. Code-state admission (slice watermarks), skipping seqs > Cutoff(E)
   and holding seqs > HoldCut(E) without advancing the watermark past
   them.
4. Settings-state admission with its own watermark, the same cutoff and
   hold.
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
| Part depth ≤ 8, part fan-out (`more`, `L_parts`) ≤ 16, inline fill 48 KiB, part ≤ 60 KiB; folders per removal/readd ≤ 256, ack items ≤ 256, lifts ≤ 64, `retirements_known` ≤ 64 (split or parts, never refuse, §1.9) | technical cap | engineering default |
| Ledger large-history diagnostic at 100,000 entries | technical cap | engineering default |
| Update check: connect/read timeouts, 256 KiB response limit, redirect limit | technical cap | engineering default (ops §4) |
| Diagnostics export: size and row limits, history list lengths | technical cap | engineering default (ops §5) |
| `notices.json` 64 KiB, notification history length | technical cap | engineering default (ops §3) |
| 100 MiB background memory; sync within 10 s | product gate | approved (AGENTS, R-14) |
| Removal settle 2 scans / 30 s | product gate | approved (Q-41) |
| 72 h "not heard from" emphasis | product gate | approved (Q-27) |
| Settings saved-file size cap | product gate | **32 MiB (33,554,432 bytes)**, owner decision D-26 (Q-77), as built; a missing value keeps settings sync disabled |
| 6 h repeat suppression of an unchanged notice | product gate | **PENDING OWNER** (Q-85); never applies to `DataAtRisk` |
| `DataAtRisk` and the notification toggle | product gate | **PENDING OWNER** (Q-86): respects the toggle; always shown in the app |
| 24 h automatic update-check interval | product gate | **PENDING OWNER** (Q-87) |

---

## 9. Phase C packets, file ownership and build order (R4D-17)

Every shared type, record kind, identity type, journal shape and family
validation rule is implemented **once**, in step 1. Step-2 packets only
fill behaviour behind the step-1 extension points and touch disjoint
files. A packet never edits a file owned by another packet; a needed
change to a step-1 file is a contract change: it stops the packet and goes
back to the orchestrator.

Paths are relative to the repository root. "new" marks a file the owner
creates.

### 9.1 Step 1: R4-core-contracts (foundation, merges alone)

| Crate | Files owned | Contents |
|---|---|---|
| core | `crates\wow-sync-core\Cargo.toml`, `Cargo.lock` | the NFC dependency (§3.3) |
| core | `src\records.rs` | `record!` majors (§2.1), `ApplyJournal/2`, `RecoveryPoint/2`, `PendingReport/2`, `SettingsFileOp`, `SettingsRoot`, `RootState`, `MissingReason`, V-JRN, V-POINT, derived `paired` (§2); the settings records `SettingsCheckpointManifest`, `SettingsState`, `SettingsLocal` (settings.md §3) |
| core | `src\ids.rs` | `SettingsUnit`, `Identity` (NFC, collision and case diagnoses), `CheckpointRef`, `SettingsRoot` re-export (§3) |
| core | `src\toc.rs`, `src\paths.rs` | `TocInfo.saved_variables{,_per_character}` (removal.md and settings.md §1); the two WTF path builders and `classify_wtf_rel` (settings.md §1.1) |
| core | new `src\authority.rs` | envelope, decode/dispatch, `AuthorityRef`, `Known` and coverage, parts and V-PART, V-BYTES…V-REF, historical membership, the three-valued retirement barrier (Effective / Void / Unresolved, `Retired`, Cutoff, HoldCut) and V-RET (§1.10, §1.11), learning, mirroring, repair plan (§1.12), capabilities (§1.13) |
| core | new `src\kinds.rs` | DTOs and **all** V-KIND rules of every kind, as the owning design defines them: V-REMOVAL-1…4, V-CHOICE (§5.2), V-REJECTION, V-LIFT, V-POLICY, V-REVISION, the checkpoint manifest rules, retirement shape, V-PART |
| core | new `src\ledger.rs` | ledger entries, states, segment encoding (§1.7) |
| core | new `src\hooks.rs` | extension-point traits (§9.4) and their neutral defaults |
| core | `src\layout.rs` | `authority\`, ledger, settings-local and recovery paths |
| core | `src\admit.rs` | admission order (§6), cutoff-skipping, the retirement hold (`pending(retirement_unresolved)`), settings watermark |
| core | `src\target.rs` | `compose` skeleton per folder (§4.1), level traits, `Source`, `Content`, `ScopeMatch` and `same_release` (§4.2), whole-addon ordinary selection and projection (§4.3), per-folder constraint composition (§4.4), all `DecisionReason` variants including the rename (§5.1) and `NewVersionAfterDecision` |
| core | `src\plan.rs` | `FolderAction::Remove`, `PackageApply { kind, removals, settings_ops, pending_records }`, planner hook calls |
| core | `src\exec.rs`, `src\exec_tests.rs` | full C-JRN execution: every slot table (install/replace, remove, settings ops), `pending_records` reservation and finalize, `recover_interrupted` for every kind (§2.2) |
| core | `src\status.rs`, `src\view.rs`, `src\explain.rs` | every new status, hold, decision and notice enum variant, with hook calls for feature text |
| core | new `src\decision.rs` | **every** type named in a §9.4 signature or exchanged between core, runtime and app for decisions: `DecisionId`, `DecisionScope`, `DecisionItem`, `DecisionOption`, `OptionId`, `OptionLabel`, `Consequence`, `OptionWarning`, `EvidenceToken`, `FreshEvidence`, `DecisionEffect`, `LocalResolution`, `DecideError`, `DecideOutcome`, `RetirePlan`, `Constraint`, `RemovalView`, `AckItem`, `CleanupClass`, `SettingsTarget`, `CycleCx` (ops.md §1.1) |
| core | `src\lib.rs`, `src\testkit.rs` | the complete module list: every step-1 module and **every step-2 and step-3 module and test module below**, so no later packet edits a `lib.rs`; shared fixture builders |
| core | new stubs `src\removal.rs`, `src\removal_tests.rs`, `src\settings.rs`, `src\settings_tests.rs`, `src\reject.rs`, `src\reject_tests.rs`, `src\choice.rs`, `src\choice_tests.rs`, `src\decide.rs`, `src\decide_tests.rs`, `src\retire.rs`, `src\retire_tests.rs` | neutral implementations of the §9.4 traits and empty test modules, **handed over** to their step-2 owner at merge |
| core | new stubs `src\notice.rs`, `src\update.rs`, `src\diagnostics.rs` | empty modules (types only where §9.4 or `control.rs` needs them), handed over to R4-notify-diag |
| core | new `fixtures\contracts\` | golden bytes of every kind, part, index chunk, journal and point (`/1` and `/2`) |
| adapters | `src\provider.rs`, `src\fs_install.rs`, `src\fs_safe.rs`, `src\testkit.rs`, new `src\authority_store.rs` | authority and index IO, settings stream and checkpoint upload, own-copy repair, ledger segment files, remove action, settings file moves, the `WtfScope` root |
| adapters | `src\lib.rs`; new stubs `src\wtf.rs`, `src\settings_store.rs` (to R4-core-settings), `src\notify.rs`, `src\update_http.rs` (to R4-notify-diag) | module declarations and re-exports for every adapters module; stubs compile with no behaviour and no new dependency |
| runtime | `src\service.rs`, `src\effects.rs`, `src\control.rs`, `src\activity.rs`, `src\lib.rs` | cycle skeleton calling the hooks in order, `UnknownJournal` listing (§2.4), every control command and query variant (including `SettingsHistory`, `ExportDiagnostics`, `CheckUpdates`, `AppSettings`), the session-end event; `lib.rs` declares every runtime module below |
| runtime | new stubs `src\removal_cycle.rs`, `src\settings_cycle.rs`, `src\rollback_flow.rs`, `src\decisions.rs` (to their step-2 owner), `src\notices.rs`, `src\updates.rs`, `src\diag.rs` (to R4-notify-diag) | neutral hook implementations, handed over at merge |
| repo | `tests\required-cases.toml` | its own block plus one section per later packet, delimited by `# >>> section: <packet>` and `# <<< section: <packet>` comment lines (the shared-append exception, §9.2) |

Tests: T-C01…T-C14 and T-C06b, T-C06c (§10), plus T-R4D1-*, T-R4D4-*,
T-R4D15-*, T-R4D16-*, T-R4D23-unknown-waits, T-R4D23-same-release-table,
T-R4D24-no-hybrid, T-R4D25-partial-retirement-dag,
T-R4D28-exact-source-unrelated-addon and T-R4D28-regrouped-exclusion.
T-C11 is
offline-fixture only.

**Handover.** At step-1 merge every stub file above passes to the packet
named in its row, and `Cargo.lock` passes to R4-notify-diag for step 3
(no step-2 packet adds a dependency). These are serial transfers: the
file never has two owners at once.

### 9.2 Step 2: three packets in parallel, after step 1 merges

| Packet | Files owned (all under `crates\`) | Behaviour |
|---|---|---|
| **R4-core-removal** | `wow-sync-core\src\removal.rs`, `wow-sync-core\src\removal_tests.rs`; `wow-sync-runtime\src\removal_cycle.rs`, `wow-sync-runtime\src\detail.rs` | removal.md: observation, settle, merge, stale copies, acks, re-add, removal cycle steps (removal.md §1.6), absent-ack/cleanup planning (R4D-5) |
| **R4-core-settings** (settings and rollback) | `wow-sync-core\src\settings.rs`, `settings_tests.rs`, `wow-sync-core\src\reject.rs`, `reject_tests.rs`; `wow-sync-adapters\src\wtf.rs`, `wow-sync-adapters\src\settings_store.rs`; `wow-sync-runtime\src\settings_cycle.rs`, `wow-sync-runtime\src\rollback_flow.rs` | settings.md and rollback.md: WTF scan, capture, local blob/manifest/`SettingsLocal` store, target (settings.md §4.2), epoch, revisions, policy, settings history query, rejection/lift behaviour, recovered chain, folder-changing rollback (R4D-20) |
| **R4-core-ops** | `wow-sync-core\src\choice.rs`, `choice_tests.rs`, `wow-sync-core\src\decide.rs`, `decide_tests.rs`, `wow-sync-core\src\retire.rs`, `retire_tests.rs`; `wow-sync-runtime\src\decisions.rs` | ops.md §1–§2: code-choice semantics (§5.3), `decide` freshness (T-C15), retirement command and master reassignment flow |

All these files exist as step-1 stubs and are already declared in their
crate's `lib.rs`; no step-2 packet creates a module or edits a `lib.rs`,
`Cargo.toml` or `Cargo.lock`. The three code file sets are disjoint.

**Shared-append exception: `tests\required-cases.toml`.** It is the one
file every later packet changes. Each packet may change only the lines
strictly between its own `# >>> section: <packet>` and
`# <<< section: <packet>` markers, which step 1 creates. The gate
(T-R4D17-file-ownership) checks it: outside the packet's own section the
file is byte-identical to the base, the markers are unchanged, and every
case id added inside the section is new in the file. Concurrent appends
in different sections merge without conflict.

### 9.3 Step 3: after all of step 2 merges

| Packet | Files owned |
|---|---|
| **R4-notify-diag** | core `src\notice.rs`, `src\update.rs`, `src\diagnostics.rs` (step-1 stubs, already declared); adapters `src\notify.rs`, `src\update_http.rs` (step-1 stubs), `src\winproc.rs`, `src\localstate.rs`, `wow-sync-adapters\Cargo.toml`, `Cargo.lock`; runtime `src\notices.rs`, `src\updates.rs`, `src\diag.rs` (step-1 stubs); its section of `tests\required-cases.toml`. No `lib.rs` |
| **R4-app** | every file of `crates\wow-sync-app`; its section of `tests\required-cases.toml` |

R4-notify-diag and R4-app may run in parallel if R4-app consumes only the
runtime control API; otherwise R4-app follows R4-notify-diag.

### 9.4 Extension points (owned by step 1)

```rust
// crates\wow-sync-core\src\hooks.rs
pub trait RemovalHooks {   // filled in removal.rs
    fn merged_removals(&self, l: &Ledger, f: &FolderKey) -> RemovalView;
    fn stale_copy(&self, l: &Ledger, f: &FolderKey, c: &Candidate) -> bool;
    fn classify_due(&self, cx: &CycleCx) -> Vec<(AckItem, CleanupClass)>;
    fn plan_removals(&self, cx: &CycleCx) -> Vec<PackageApply>;
}
pub trait ConstraintHooks { // choice.rs (keep/resume) and reject.rs (rejection)
    fn live_constraints(&self, l: &Ledger, f: &FolderKey) -> Vec<Constraint>;
    // whole-source checks by `same_release` (§4.2), source against each
    // recorded entry over that entry's scope only; never per folder
    fn excluded(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch;
    fn rejected(&self, l: &Ledger, p: &Package, s: &Source) -> ScopeMatch;
}
pub trait SettingsHooks {  // settings.rs
    fn settings_target(&self, cx: &CycleCx, u: &SettingsUnit) -> SettingsTarget;
    fn plan_settings(&self, cx: &CycleCx) -> Vec<PackageApply>;
    fn roots_for(&self, cx: &CycleCx, folders: &[FolderKey]) -> Vec<SettingsRoot>;
}
pub trait DecisionHooks {  // decide.rs
    fn decide(&self, item: &DecisionId, option: &OptionId,
              shown: &EvidenceToken, now: &FreshEvidence) -> DecideOutcome;
}
pub trait RetireHooks {    // retire.rs (command flow only; the barrier is in authority.rs)
    fn plan_retirement(&self, cx: &CycleCx, subject: &Enrollment) -> RetirePlan;
}
```

Runtime calls them from `service.rs` in cycle order (removal.md §1.6), and
`control.rs` routes commands to `removal_cycle`, `settings_cycle`,
`rollback_flow` and `decisions`. Signatures are frozen at step-1 merge.

### 9.5 Build order

1. **R4-core-contracts** alone. Gate: offline verify green, golden
   fixtures committed, every step-2 stub compiles and keeps slice
   behaviour.
2. **R4-core-removal**, **R4-core-settings**, **R4-core-ops** in parallel.
   Gate per packet: its tests green; no file outside its set touched,
   except its own section of `tests\required-cases.toml` (§9.2).
3. **R4-notify-diag**, then (or with) **R4-app**; cross-feature tests last.

---

## 10. Contract tests

Owned by R4-core-contracts unless noted. Each is a behavioural test with
fixed bytes where stated.

| Test | Checks |
|---|---|
| T-C01-golden-bytes | one golden file per kind (including part) and per index chunk: exact bytes, schema/major, file name = sha256; decode round-trip |
| T-C02-retired-forms-rejected | `id` fields, `{id,hash}` refs and a domain prefix are not produced; a ref of wrong kind is `invalid(wrong_reference)` |
| T-C03-bad-copy | a file whose bytes do not match its name is skipped and another good copy is used |
| T-C04-repair-own-only | own bad copy is replaced via temp + verify + rename; a good own file is never overwritten; a peer's bad copy is untouched |
| T-C05-journal-v1-golden | a pure code apply journal and code-only point encode byte-identical to slice fixtures; an update that drops a folder is **not** `/1` |
| T-C06-v2-under-slice | a `/2` journal (settings-only and mixed) fed to slice `recover_interrupted` gives `NeedsHelp`, journal kept, applies blocked; slice `decode_points` keeps `/2` points; same after a crash in a settings-only op |
| T-C06b-unknown-journal-listed | the new runtime lists an undecodable journal as `UnknownJournal` in service, control and view output; it is never deleted |
| T-C06c-jrn-crash-slots | for each kind in the §2.2 table, a crash at every slot (including between verify and `pending_records`) recovers to exactly-once records: the reserved bytes, or none after rollback |
| T-R4D1-jrn-kind-actions | every `kind` × action combination outside the §2.2 table is rejected by V-JRN; a `remove` action whose folder no listed removal names is rejected; a `/1` journal with a `remove` action is rejected |
| T-R4D1-mixed-update-drop | an update that replaces one folder and drops another uses one `apply/2` journal with a `folder_drop` removal; recovery from each slot |
| T-R4D1-paired-derived | each row of the §2.3 `paired` table, and every wrong `paired` value, is checked by V-POINT; no point is written when nothing is captured or replaced |
| T-C07-pending-report-v2 | an old state never contains new report kinds; a `/2` pending report under the slice decoder is replaced, not misread |
| T-C08-known-bound | re-add after 1,024 live removals of one folder fits in 64 KiB and supersedes all of them; a concurrent removal is not covered |
| T-R4D4-1024-issuers-readd | 1,024 removals by 1,024 different enrollments: the re-add moves `seen` into parts, every part ≤ 64 KiB, covers all of them, and stays pending until every part arrives |
| T-R4D4-mirror-only-readd | issuer indexes unreadable: every learned record goes into `extra` parts; coverage identical |
| T-R4D4-full-policy | a policy naming 5,000 folders is written in parts and read back identical; a duplicate folder in two parts is `invalid(policy_body)` |
| T-C09-ledger-segments | a ledger past 16 MiB in 4 MiB segments reloads, re-derives validity and keeps every entry; memory per entry measured |
| T-C10-retirement-barrier | three issuers: A retires B, C retires A with lower cutoff; both delivery orders; B stays retired, A retired, B's states above cutoff never admitted; master and stale decisions included |
| T-R4D15-vret-predicate | V-RET truth table: ordinary record of a retired issuer above/at cutoff; retirement by a retired issuer, void and not void; issuer not retired; issuer `Retired = Unresolved` above its HoldCut → `pending(retirement_unresolved)`, never invalid and never admitted |
| T-R4D16-later-cutoff-no-reauth | A retires B at cutoff 10; B's seq-11 record is invalid; C's retirement of B at cutoff 12 arrives: still invalid; a third retirement at cutoff 8 makes B's seq 9–10 records invalid |
| T-R4D16-retired-issuer-retirement-void | B, knowing its own retirement (non-empty `retirements_known`), retires A: void, A not retired, in both delivery orders |
| T-C11-identity-collisions | NFC collision and case-only difference in **offline fixtures** give the named diagnoses; neither identity is synced. Native NTFS/APFS: NOT RUN (R4D-22) |
| T-C12-index-chain | replacement of a partial head chunk; a missing middle chunk makes the member's index unreadable without forgetting learned refs; `total` mismatch detected |
| T-C13-historical-membership | a record from an enrollment no longer in `member.json` stays valid via `previous_enrollments` or state file names; a retirement naming a non-member is `invalid(not_a_member)` |
| T-C14-compose-precedence | each adjacent pair of levels in §4.1 with neutral hooks; feature rows are added by the step-2 owner of the hook |
| T-R4D21-partial-choice-over-choice | keep X on {a, b}; later keep Y on {a} covering X: a follows Y, b still follows X; same result with {a, b} grouped and ungrouped locally (owner: R4-core-ops) |
| T-R4D21-choice-over-rollback | rejection on {a, b}; keep on {a} covering it: a follows the keep, b keeps the recovered tree (owner: R4-core-settings) |
| T-R4D21-readd-over-rollback | rejection on {a, b}; re-add of {b} covering it: b leaves the constraint, a stays constrained (owner: R4-core-removal) |
| T-C15-decide-fresh | a record learned between showing and choosing gives `Changed`; no write happens (owner: R4-core-ops) |
| T-R4D17-file-ownership | merge gate run by the orchestrator for every phase C packet: `git diff --name-only <base>..<packet head>` lists only files of that packet's §9 row, plus `tests\required-cases.toml` when the section check passes (outside the packet's own `# >>> section` / `# <<< section` markers the file equals the base, markers unchanged, added case ids new); a step-2 or step-3 packet touching a step-1 file (including any `lib.rs`) or another packet's file fails the gate. Counterexamples: a removal packet editing `kinds.rs` is refused; a settings packet adding a case inside the R4-core-ops section is refused; R4-notify-diag editing `wow-sync-core\src\lib.rs` is refused |
| T-R4D23-same-release-table | `same_release` truth table (§4.2), both forms. Entry against entry (K = union): equal on all keys; a key `Absent` on one side and a tree on the other; disjoint scopes (`Unknown`); overlap-equal with differing scopes (`Unknown`). Source against entry (K = entry scope only): exact match `Same` with extra source keys present or absent; an in-scope folder not `Ok` with nothing different (`Unknown`); the same plus a known difference (`Different`). The same entries under two local groupings give identical results. A-38 controls: compare common keys only (T-R4D23-added-folder-rollback fails); union the source's keys (T-R4D28-exact-source-unrelated-addon fails) |
| T-R4D23-unknown-waits | §4.2 last row: a source with an `Unstable` folder in scope is neither a candidate nor rejected; ordinary selection does not fall back to a lesser candidate; it resolves once stable |
| T-R4D23-added-folder-rollback, T-R4D23-dropped-folder-rollback | §4.2 rows (owner: R4-core-settings, rollback.md §10) |
| T-R4D23-keep-resume-shared-component | §4.2 row (owner: R4-core-ops, ops.md §7) |
| T-R4D24-no-hybrid | §4.3 with neutral hooks: X `{a:3.0, b:1.0}` vs Y `{a:2.0, b:2.0}` gives `NeedsDecision(Ambiguous(MembersDisagree))` for the addon; no plan contains `{a:3.0, b:2.0}`; also with X complete and Y missing b: `PartialCandidate`, nothing installed; A-38 control: per-folder ordinary selection (hybrid planned, test fails) |
| T-R4D25-partial-retirement-dag | the §1.10 R/S/T/U trace in every listed delivery order and after a restart at each partial step: each record's status, `Retired`/Cutoff/HoldCut per enrollment, held vs admitted seqs, `awaiting`, master prompts and own-publish pause exactly as the tables; no status ever flips between Effective and Void; final state identical in every order (ops.md §7 repeats the user-facing checks) |
