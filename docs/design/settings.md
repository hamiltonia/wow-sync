# Addon settings sync

Status: **design for a later run** (run-004 packet
[R4-settings-design](../runs/run-004/packets/R4-settings-design.md),
revised by [R4-design-fix](../runs/run-004/packets/R4-design-fix.md) after
[R4-design-review](../runs/run-004/evidence/R4-design-review.md)). Not
yet implemented. Nothing here changes the running slice, which does not
read or write anything under `WTF` today.

**Contract status.** Authority records (envelope, identity, refs, `Known`,
validation, learning, mirroring), apply journals, recovery points, settings
units, identity normalization, checkpoint references and target precedence
are defined only in [contracts.md](contracts.md). This document defines
the settings-specific bodies (policy, revision, checkpoint manifest,
settings state) and behaviour. Where an earlier draft disagreed with
contracts.md, the draft text has been removed. Phase C: shared types are
built by R4-core-contracts, the settings behaviour by R4-core-settings
(contracts.md §9, this file §10). Revised again by R4-design-fix2 for
R4D-4 (parts), R4D-18 (ordered target cases) and R4D-19 (activation
capture).

Product authority: SPEC R-07, R-08, R-09, R-10, R-11 and R-12
([SPEC.md](../../SPEC.md)). Acceptance: A-11 to A-17, A-19, A-20 to A-26 and
A-45 ([ACCEPTANCE.md](../../ACCEPTANCE.md)). Rollback and rejection are in
[rollback.md](rollback.md). This document builds on
[domain.md](domain.md), [publication.md](publication.md),
[apply.md](apply.md) and [core-api.md](core-api.md). It implements the
settings interface that [removal.md](removal.md) §6 requires. Choices that
approved intent does not settle are marked **(Q-n)** and listed in
[questions.md](questions.md) from Q-67 on; **PENDING OWNER** marks a
conservative default the owner has not decided. Computer retirement is
[ops.md](ops.md) §2 with the barrier of contracts.md §1.10; it uses the same
terms **settings master** and **reassign**.

## 0. Vocabulary

| Term | Meaning |
|---|---|
| identity | an account or character scope inside one variant: `account(<account>)` or `character(<account>, <realm>, <character>)`, from the exact WTF folder names, compared after NFC ([contracts.md](contracts.md) §3.2–§3.3) |
| settings file | the saved-variable file that WoW writes for an addon folder: `<Folder>.lua` in an identity's `SavedVariables` folder (§1.1) |
| settings unit | `SettingsUnit { folder, identity }` ([contracts.md](contracts.md) §3.2): the settings file of one addon **folder** for one identity. History, sharing, policy, decisions and pause all work on units or folders, never on local package keys. The GUI groups units by addon ("per addon"), shows "mixed" when one addon's units differ, and shows the identity as the scope |
| checkpoint | the exact bytes of a unit's settings file (or its absence) at one moment, held as a content-addressed manifest (§3.1). Bytes are opaque |
| capture | a local checkpoint taken at a completed play-session boundary (§2) |
| revision | a group-visible, immutable authority record that says "the shared settings of unit U are now checkpoint C", with its kind and its parents (§3.3) |
| revision graph | per unit, the **structural** graph of valid revisions and their parent edges (§4.2). A revision stays in the graph as ancestry even when its checkpoint is no longer selectable |
| head | a valid revision of a unit that no other valid revision names as a parent. **Head states** group heads by checkpoint: one head state means one shared state, even if several heads carry it. Two or more head states means competing intentional choices (§4.4) |
| basis | the **set** of revisions whose common checkpoint is in this computer's live file for a unit, as recorded locally when it last applied or published: all heads of the head state it applied (§2.3, R4D-7) |
| settings master | the computer whose completed-session captures publish automatically. One default master per group, plus per-folder exceptions (§5). Assigned and reassigned only by an explicit user action |
| routine change | a capture on a computer that is not the authority for that unit. It stays local and is never published automatically (R-08) |
| push | a deliberate user action on any computer that publishes its current checkpoint of one or more units as the new shared state ("Share my settings") |
| yield | replacing a computer's routine local settings with a newer shared revision, after moving the replaced files into recovery (§4.3) |
| pause | a per-computer protection of one addon's units, or of all units ("Pause settings"), so local experiments are neither published nor replaced (§6) |

The GUI words come from [ui/screens.md](ui/screens.md): "Settings master",
"Change master…", "Share my settings", "Pause settings" (never "Pause
syncing"), "Settings choices", "recovery copy".

## 1. Scope: which files sync

### 1.1 Files

Per variant, the settings root is `<variant>\WTF\Account`. Only these two
shapes are settings files:

```text
WTF\Account\<ACCOUNT>\SavedVariables\<Folder>.lua                     account identity
WTF\Account\<ACCOUNT>\<Realm>\<Character>\SavedVariables\<Folder>.lua character identity
```

`<Folder>` is the name of an installed addon folder
([domain.md](domain.md) §5), compared case-insensitively on Windows and
exactly otherwise, as domain.md does for folders. The file is the settings
of unit `{ folder: FolderKey(<Folder>), identity }` when that folder's
selected `.toc` declares `## SavedVariables` (account file) or
`## SavedVariablesPerCharacter` (character file) **(Q-67)**. This is the
SavedVariables ownership rule that [contracts.md](contracts.md) §3.2 refers
to. Ownership depends only on the folder, not on how folders are grouped
into packages, so regrouping never moves a file to another unit (R4D-6).
The declaration is `.toc` metadata, read by the existing `.toc` rule; it is
never Lua.

Everything else under `WTF` is out of scope and is never read for content,
written, moved or deleted by settings sync **(Q-68)**:

| Excluded | Why |
|---|---|
| `WTF\Config.wtf`, `WTF\Account\<A>\config-cache.wtf`, `bindings-cache.wtf`, `macros-cache.txt`, `chat-cache.txt`, `layout-local.txt`, `AddOns.txt`, any other non-`SavedVariables` file | WoW's own UI, key bindings, macros, layout and cloud-sync files (R-07) |
| `SavedVariables\Blizzard_*.lua` and any file whose folder is not part of a package installed in `Interface\AddOns` | WoW's own UI state, or no known owner |
| `*.lua.bak`, `*.lua.old`, temporary files | WoW's own backup or partial files. Left untouched |
| `WTF\Account\SavedVariables` (account folder missing) and anything deeper than the two shapes above | Not one of the supported shapes. Reported as a scope diagnosis if it matches a package name |

### 1.2 Ownership diagnoses

The scope function returns owned items and diagnoses. A diagnosis blocks
settings sync **for the affected units only**; other units continue
(R-07, R-09):

| Case | Diagnosis | Effect |
|---|---|---|
| Two installed folders declare the same saved-variable name in their `.toc` | `SharedVariable { name, folders }` | Units of every listed folder are not synced in either direction; their live files are not touched |
| A file `<Folder>.lua` exists but no installed folder declares saved variables for that shape | `Unowned { file }` | Not synced, not moved, not deleted. Listed in Details only, as information |
| A folder declares saved variables but the `.toc` selection for the folder is ambiguous ([domain.md](domain.md) `.toc` rule) | `AmbiguousToc { folder }` | Unit not synced |
| A path component is a reparse point, link, junction, or names differ only in case on a case-sensitive volume | `UnsafePath { path }` | Unit not synced; nothing under that path is written |
| A file exceeds the saved-file size cap `MAX_SETTINGS_FILE_BYTES` (Q-77, D-26: 32 MiB) | `TooLarge { file, size }` | Unit not synced |
| An identity folder name fails the safe-name rules of [domain.md](domain.md) (control characters, reserved names, trailing dot or space) | `UnsafeIdentity { name }` | Identity not synced |
| Two local identity paths with equal NFC forms but different bytes | `IdentityNormalizationCollision { paths }` ([contracts.md](contracts.md) §3.3) | Neither identity synced on this computer |
| A peer's identity differs from a local one only by letter case | `IdentityCaseMismatch` (contracts.md §3.3) | Not matched; neither applied to the other |

**Q-77 (decided, D-26).** The size cap is a product gate (contracts.md
§8). The owner approved **32 MiB (33,554,432 bytes)**, as built in
`MAX_SETTINGS_FILE_BYTES = Some(33_554_432)`: a file at the cap syncs and one
byte more is `TooLarge`. Tests may still pass a smaller test-only value.

There is **no fallback** to copying `WTF`, a whole account folder or a whole
`SavedVariables` folder. Settings sync never splits one file into parts and
never deletes a file it does not own. A removal of an addon whose ownership
is diagnosed is code-only for those units ([removal.md](removal.md) §6.2;
Q-46).

### 1.3 Opaque bytes

Settings files are **never parsed, evaluated, loaded or executed** as Lua,
and never searched for text. The app uses only metadata: path, size,
SHA-256, and the WoW-not-running and quiescence checks. It hashes and
copies the bytes. A file that looks like code, contains `os.execute`, binary
data or anything else is treated exactly the same. This extends the
imported-content rule in [domain.md](domain.md) and the A-23 obligations.

### 1.4 Identities and matching across computers

- An identity is matched across computers by **exact** WTF folder names
  (`<ACCOUNT>`, `<Realm>`, `<Character>`) after Unicode NFC normalization,
  with the collision and case diagnoses of
  [contracts.md](contracts.md) §3.3 **(Q-69)**. A normalizing key never
  hides two distinct local identities; both are diagnosed instead.
- A computer shares only the accounts the user included when turning on
  settings sync on that computer (§5.4). A newly seen account folder is
  "not included" until the user includes it. Characters inside an included
  account are included automatically, because WoW creates them as you play.
- There is no automatic mapping of a renamed or transferred character, and
  no mapping between accounts. Settings for identity I are applied only to
  identity I (R-10, AGENTS.md "matching account/character boundaries").
- A variant is a group ([domain.md](domain.md)); every record below lives
  under one group, so settings never cross variants. "Forever" identity is
  whatever the variant validation decided; this design adds nothing to it.

### 1.5 Paths and write authority

Writes to `WTF` are a **new authorized write scope**, limited to:

- the exact live file paths of owned items in an approved plan (§4.3);
- nothing else under `WTF`. Directory creation is limited to a missing
  `SavedVariables` folder inside an existing identity folder. The app never
  creates account, realm or character folders.

All other settings data lives in the existing authorized scopes:
`<variant>\.wowsync\settings\…` (same volume as `WTF`, so moves are
renames), local state and the provider group folder. The WTF path builder
in core rejects any relative path that is not one of the two shapes in
§1.1. The adapter re-checks every component with the existing reparse
policy before each operation.

**Unverified assumption.** The two shapes are the documented WoW layout.
They have not been checked on a real client for every variant, and
real-client checks were deferred by D-23. Under D-28 real WTF settings sync by
default once the variant is validated; the synthetic marker is required only
in test mode **(Q-68)**. R6-check (run-006) observed the real WTF scope and
the saved-variable declarations validate; settings removal and restore on real
data have not been run yet, and a recheck is pending. No real-client
confirmation of every variant exists.

## 2. Capture at completed play-session boundaries (R-07, A-11)

### 2.1 When a capture happens

A capture is taken per unit, never while WoW for that variant runs:

1. **Session end.** The activity monitor (existing `Activity` from
   [apply.md](apply.md) §5) sees the variant's WoW process stop. The
   capture waits for the settle window: two consecutive stable scans of
   the owned files, at least the Q-15 quiescence apart, with the same
   settle wait as removal detection (Q-41) **(Q-71)**.
2. **Startup / missed session.** At app start, or when the app has not seen
   a session end (it was not running, the computer slept), any unit whose
   files differ from the last capture or applied checkpoint is captured
   once, as kind `found_at_start`. Several missed sessions become one
   the capture is compared with the activation capture or a later capture
   (§4.1), so it never publishes a difference that predates this computer
   becoming the authority (Q-75, R4D-19).
3. **Before a settings or code change by this app.** Before a yield,
   restore, removal or rollback moves a unit's files, the current files are
   captured (kind `before_change`) so that what is replaced is a recovery
   state.

Repeated writes during a session (reloads, logouts without exit) never
produce captures, because WoW is running. A capture whose content hash
equals the previous capture or the basis checkpoint is not stored again.
So history fills at play-session rate, not file-write rate (A-11).

### 2.2 Incomplete writes

A capture is **complete** only if all of these hold for every file of the
unit; otherwise the unit's capture is **suspect** **(Q-71)**:

- WoW for the variant is not running at the start and end of the capture;
- every file has the same size and hash across the two settle scans and
  during hashing;
- no file shrank to zero bytes, and no file present in the previous
  capture disappeared while a `.lua.bak` of it exists (WoW stopped between
  its own backup and write).

A suspect capture is kept locally as history, is never published and never
counts as an intentional change. The unit shows "Settings may be
incomplete after WoW stopped" with "Check again" and "Restore previous
copy". A later complete capture replaces the condition.

### 2.3 What a capture records

```json
{
  "unit": { "folder": "details", "identity": { "character": ["ERIC", "Argent Dawn", "Thrall"] } },
  "checkpoint": "<CheckpointRef>",
  "kind": "session_end|found_at_start|before_change|activation",
  "complete": true,
  "basis": ["<revision hash>"],
  "compared_with_local_seq": 405,
  "changed_in_session": true,
  "code_package_hash": "<sha256 of the installed package at capture>",
  "local_seq": 412
}
```

- `basis` is the set of revision hashes (contracts.md §1.5, plain hashes)
  applied or published by this computer for the unit before the session:
  all heads of the head state it last applied, or `[]` if none.
- `compared_with_local_seq` is the `local_seq` of the previous capture or
  applied checkpoint that `changed_in_session` compares with. The
  authority epoch rule (§4.1) uses it.
- `changed_in_session` is true when the checkpoint differs from the
  previous capture or applied checkpoint of the unit **on this computer**.
  Only changed units can publish (§4.1). A difference that already existed
  before the session is not re-published by a later unrelated session.
- `code_package_hash` pairs the settings with the code that wrote them.
  Paired selection (§4.6) uses it.
- `local_seq` is a local per-unit counter, strictly increasing over every
  capture and applied checkpoint of the unit. Order never comes from
  timestamps. The authority epoch (§4.1) compares only values of this
  counter.
- Kind `activation` is the capture taken when this computer becomes the
  unit's authority (§4.1). It is a recovery state and never publishes.

## 3. Records and on-disk formats

### 3.1 Checkpoint manifest and blobs

A checkpoint is the canonical encoding of:

```json
{
  "schema": "wowsync.settings-checkpoint/1",
  "unit": { "folder": "details", "identity": { "account": "ERIC" } },
  "files": [
    { "name": "Details.lua", "size": 183402, "blob": "<sha256>" }
  ]
}
```

- `files` holds zero or one entry (the unit's one file). An empty list is
  valid and means "this unit has no settings file" (for example after the
  user reset an addon).
- **Checkpoint reference** = `sha256(exact manifest bytes)`, written with
  `records::encode` ([contracts.md](contracts.md) §3.4). No domain prefix.
  Same unit and bytes give the same reference on every computer.
- Blobs are the existing content-addressed provider blobs
  ([publication.md](publication.md) §1, §3). A manifest is stored in the
  provider as `trees\<checkpoint ref>.json` only when a revision
  references it (Q-76). Local captures stay local.
- Local store: `<variant>\.wowsync\settings\blobs\<sha256>` and
  `<variant>\.wowsync\settings\manifests\<checkpoint ref>.json`, on the
  same volume as `WTF`.

### 3.2 Settings policy (`wowsync.settings-policy/1`)

Policy and revisions are kinds of the one authority family
([contracts.md](contracts.md) §1.1): same envelope (§1.4), identity
(`sha256` of the exact bytes, §1.3), location, mirroring, learning and
validation (§1.11–§1.12). Only the bodies are defined here.

A policy is a full snapshot each time:

```json
{
  "schema": "wowsync.settings-policy/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 51 },
  "master": { "computer_id": "<den>", "enrollment_id": "<den enr>" },
  "folders": [
    { "folder": "details", "rule": "master", "computer_id": "<laptop>", "enrollment_id": "<laptop enr>" },
    { "folder": "weakauras", "rule": "not_shared" }
  ],
  "known": { "seen": [], "extra": [], "parts": [] },
  "folders_parts": [],
  "written_unix_ms": 1767225600000
}
```

- `master` may be `null` (settings sync is set up but nobody publishes
  automatically; pushes still work).
- `folders` holds only exceptions, keyed by **folder key**, sorted and
  unique across inline items and parts (`policy_folders`, contracts.md
  §1.9). There is no count limit: a policy naming every folder of a large
  install is written with parts and never refused (R4D-4). A folder key
  twice, inline or in any part, is `invalid(policy_body)`. Absent means
  "follow the default master".
  `rule: "master"` names a different authority for that folder.
  `rule: "not_shared"` means the folder's settings are local only on every
  computer. The GUI sets one rule for every folder of an addon at once and
  shows "mixed" if they differ; a regrouping never changes any folder's rule
  (R4D-6).
- **Supersession:** policy X supersedes policy Y iff X's `known` covers Y
  (contracts.md §1.8). The **policy heads** are the valid policies no
  other valid policy supersedes.
- V-POLICY rules: named computers are in historical membership; folder
  keys pass `validate_folder_name`; otherwise `invalid(policy_body)`.

### 3.3 Settings revision (`wowsync.settings-revision/1`)

```json
{
  "schema": "wowsync.settings-revision/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 52 },
  "unit": { "folder": "details", "identity": { "account": "ERIC" } },
  "kind": "auto|push|choice|restore",
  "checkpoint": "<CheckpointRef>",
  "parents": ["<revision hash>"],
  "parents_parts": [],
  "policy": ["<policy hash>"],
  "code_package_hash": "<sha256>|null",
  "written_unix_ms": 1767225600000
}
```

- `kind`: `auto` = completed-session capture of the authority; `push` =
  explicit "Share my settings"; `choice` = resolution of competing head
  states (§4.4); `restore` = user restored an earlier copy, or a paired
  rollback ([rollback.md](rollback.md)).
- `parents` = plain hashes of revisions of the same unit: the issuer's
  whole basis set for `auto`/`push`/`restore`, or **every head** of every
  head state being resolved for a `choice`. Empty only for the first
  revision of a unit. Partable (`revision_parents`, contracts.md §1.9):
  any number of heads is written with parts, never refused, and there is
  no `TooManySettingsHeads` state (R4D-4).
- `policy` = a bounded **witness** of the policy heads under which the
  issuer acted (1–8 entries). The writer lists every current policy head
  when there are at most 8; with more, it lists the 8 with the smallest
  hashes (they all agree on this unit's authority, or the issuer would be
  in `PolicyConflict` and write no `auto`). An `auto` revision is valid
  only if the issuer is the unit's authority under **every** listed head.
  The witness proves only that the issuer **had** authority when it
  wrote the revision. Three checks stay distinct (R4D-30):
  1. **Historical validity** (V-REVISION): an `auto` revision is valid iff
     the issuer is the unit's authority under every listed head. A later
     settled policy never re-checks it. P0 makes A master, A publishes
     r, then P1 (covering P0) makes B master: r stays a valid, shared
     node and a joining reader adopts it by §4.2 and §5.4 without
     changing the master back to A.
  2. **Current publication authority** (§4.1): only the computer that is
     the unit's authority under the current heads writes new `auto`
     revisions. A former master stops.
  3. **Policy-conflict hold** (§5.1–§5.2): automatic apply waits only if
     the policy heads the reader currently knows **disagree among
     themselves** about the unit's authority. Agreement on a different
     authority than the revision's issuer is not a conflict. Under a
     conflict, a valid revision stays a node that is not applied
     automatically: it waits for the user's policy choice, and is never
     silently dropped or applied.

  These checks apply to `auto` revisions only; `push`, `choice` and
  `restore` are explicit and need no authority witness, so a peer push
  from a non-master is never held by check 3 unless the heads really
  conflict. A listed head the reader lacks is `pending(missing_reference)`
  as usual. No wire change: the field is the existing ≤ 8 list
  (T-R4D4-nine-policy-heads, T-R4D30-reassigned-author).
- There is no `rejects` field: settings-checkpoint rejection is not part
  of this design (Q-82 PENDING OWNER, [rollback.md](rollback.md) §5).
- V-REVISION rules (V-KIND): `parents` and `policy` sorted and unique;
  V-REF for `parents` (kind `settings-revision`, same unit) and `policy`
  (kind `settings-policy`); a parent of a different unit is
  `invalid(wrong_reference)`. A missing parent or policy is
  `pending(missing_reference)` (§4.2).
- The authority index ref of a revision carries its unit's folder key as
  the hint (contracts.md §1.5).

### 3.4 Settings state (`wowsync.settings-state/1`)

A new per-member stream in
`groups\<g>\members\<computer>\settings\<enrollment_id>-<seq:010>.json`,
written last, as the code state is:

```json
{
  "schema": "wowsync.settings-state/1",
  "computer_id": "…", "enrollment_id": "…", "seq": 88,
  "applied": [{ "unit": { "…": "…" }, "revisions": ["<revision hash>"],
                "checkpoint": "<CheckpointRef>",
                "result": "applied|kept_local|paused|blocked|diagnosed|waiting" }],
  "paused": { "all": false, "folders": ["weakauras"] },
  "included_accounts": ["ERIC"],
  "diagnoses": [{ "unit": { "…": "…" }, "kind": "SharedVariable" }]
}
```

- There is no `knows` list and no `capabilities` list. Which records a
  computer has is its authority index (contracts.md §1.6); what it
  understands is `code-state/1.features` (§1.13), including `settings/1`.
- `applied` is the settings apply report ([publication.md](publication.md)
  §6): confirmation of a revision on a computer requires an `applied`
  entry naming that revision (or a head of the same head state) with its
  checkpoint. `kept_local`, `paused`, `blocked`, `diagnosed` and `waiting`
  are reported so the group view never shows such a computer as in sync
  (R-12, A-15, A-24). Diagnoses carry closed kinds only, never paths or
  names (contracts.md §7).
- Admission: same rules as code states, with its own watermark and the
  retirement cutoff (contracts.md §6).

### 3.5 Local records (never uploaded)

| Record | Content |
|---|---|
| `wowsync.settings-local/1` (local state) | per unit: captures (newest first, §2.3), basis set, last applied checkpoint, pinned flags, `authority_since` (§4.1); pause set (folders); included accounts; per-computer consent and first-adoption choice (§5.4) |
| `wowsync.apply-journal/2` with `settings_ops` | [contracts.md](contracts.md) §2.2; kinds `settings` and `settings_cleanup` |
| `wowsync.recovery-point/2` | [contracts.md](contracts.md) §2.3; one of the three shapes (settings-only, mixed, code-only fallback) |

### 3.6 Compatibility

- All formats follow [publication.md](publication.md) §2 and Q-13: first
  field `schema`, unknown fields ignored within a major, unknown major
  `pending(unknown_schema)` and shown as "written by a newer version",
  size limits checked before parsing.
- Settings are a new stream and two new authority kinds, not a change to
  `wowsync.code-state`. The slice app never reads `authority\`,
  `authority-index\` or `settings\` (contracts.md §1.13), so it neither
  applies nor removes settings. A newer app without `settings/1` (not
  possible within run-004, but defined) treats the kinds as
  `pending(unknown_schema)` and never as absent (T-S30).
- Removal records carry the settings they took as `SettingsRoot`s in their
  `settings` field ([removal.md](removal.md) §2.3); receivers record theirs
  in `/2` recovery points (contracts.md §2.3) and in the ack's `settings`
  value. No removal schema major changes for settings.

## 4. Sharing, receiving and applying

### 4.1 Who publishes what

For each unit, after each capture:

| This computer is… | Capture | Result |
|---|---|---|
| the unit's authority (§5.1), not paused, capture complete, `changed_in_session`, and inside its **authority epoch** (below) | any | an `auto` revision, parents = basis set |
| the authority, but the capture is unchanged, suspect, outside the epoch, or the unit is paused | — | nothing published; a difference outside the epoch is `LocalDifferent` with "Share my settings" |
| not the authority | any | stays local as a routine change. **Never** published automatically (R-08, A-12) |
| any computer, user chooses "Share my settings" for the unit or addon | current file (captured first, kind `before_change`) | a `push` revision, parents = basis set |
| any computer, unit diagnosed, not included, or folder `not_shared` | — | nothing published |

**Authority epoch (Q-72 with Q-75, R4D-19).** `local_seq` is one counter
per unit on this computer; every capture and every applied checkpoint gets
a value strictly greater than all earlier ones (§2.3).

1. **Activation.** When this computer first learns a policy head under
   which it is the authority for a unit, it records `authority_since` =
   the counter's current value (§3.5). It then takes an **activation
   capture** A (kind `activation`) as soon as WoW for the variant is not
   running: immediately if it is not running, otherwise at the settled end
   of the current session. A gets `local_seq > authority_since`. A is never
   published automatically.
2. **Pre-existing difference.** If A's checkpoint differs from the
   checkpoint of the unit's basis (the shared state this computer last
   applied or published; for an empty basis, from the current head state),
   `epoch_clean = false`: the unit shows `LocalDifferent` ("Changed here,
   not shared") until the user chooses "Share my settings" (push) or "Use
   shared settings" (yield on request). Either sets `epoch_clean = true`.
   Otherwise `epoch_clean = true` at once.
3. **Automatic publication** of a later capture C requires `epoch_clean`,
   C complete, `changed_in_session`, and `C.compared_with_local_seq >
   authority_since` (strict): the capture C is compared with was itself
   taken inside the epoch (A or later).
4. Losing the authority clears `authority_since` and `epoch_clean`; a new
   activation starts again at step 1.

So:

- a `session_end` capture of a session that started after activation
  (compared with A or later) publishes (Q-75);
- a difference that already existed at activation, including an
  experiment made while the app was closed, never publishes
  automatically: it is A's difference and stays `LocalDifferent`
  (T-R4D19-epoch-equal-seq);
- a session in progress at activation is not published; its end is the
  activation capture, and the next session publishes. This is the
  conservative reading of Q-75.

A push is offered only when WoW for the variant is not running (the file
must be a complete capture). A push for a whole addon issues one revision
per folder and included identity that has a file or a basis.

### 4.2 Receiving: the target per unit (R4D-7, R4D-8)

The settings target is computed per unit from the ledger. The **structural
graph** and the **selectable heads** are separate:

1. **Nodes.** Every `valid` revision of the unit is a node. Validity is the
   contract's (contracts.md §1.11) plus §3.3 and §5.1: an `auto` revision
   from an issuer that is not the authority under its **listed** policy
   heads is `invalid(not_authority)` (never re-checked against later
   policies, §3.3 check 1); a revision above its
   issuer's retirement cutoff is `invalid(after_retirement)`. Invalid
   revisions are not nodes. There is no settings-checkpoint rejection
   (Q-82 PENDING OWNER), so no valid node is ever excluded from
   selection.
2. **Pending.** A revision whose parent or policy is not yet in the
   ledger is `pending(missing_reference)`. It is **not** a node yet. If any
   pending revision of the unit exists, the unit is `Waiting(MissingParent)`:
   nothing is applied or published automatically for it until the record
   arrives (it resolves by itself, contracts.md §1.11). The GUI says
   "Waiting for settings information from <computer>".
3. **Invalid parents.** A valid revision whose parent is `invalid` (for
   example issued above its issuer's retirement cutoff) stays a node; the
   invalid parent is treated as absent for ancestry. Its own validity does
   not depend on its parent's (V-REF needs only presence and kind), so a
   late retirement never deletes a later valid revision.
4. **Heads** = nodes that no node names as a parent. **Head states** group
   heads by checkpoint ref.
5. **Basis status (R4D-18).** `B` is this computer's recorded basis set
   (§2.3), kept locally **as recorded**, with its checkpoint `Cb`; it is
   never silently shrunk. `Bv` = the elements of B that are nodes. `L` is
   the live checkpoint and `cH` the checkpoint of a head state H. "X ≤ H"
   means every element of X is a head of H or an ancestor of one, over node
   edges only.

Outcomes: the cases are **mutually exclusive and evaluated in this
order**; the first that matches is the target, and no later case is
consulted.

| # | Situation | Target |
|---|---|---|
| 1 | any pending revision of the unit (step 2) | `Waiting(MissingParent)` |
| 2 | zero nodes | `Off` for this unit if there is no policy; otherwise `NoSharedCopy`: nothing applied, local file kept, first `auto`/`push` creates the root |
| 3 | two or more head states | `NeedsDecision(SettingsChoice)` (§4.4) |
| 4 | one head state H; B = ∅ (never applied or published here) | first adoption, §5.4 |
| 5 | one head state H; B ≠ ∅ and (`Bv` = ∅, or not `Bv ≤ H`): the recorded basis became wholly invalid, or is on a branch H does not descend from | `LocalDifferent(BasisNotShared)` (or `InSync` with basis = all heads of H if L == cH): nothing applied automatically; "Use shared settings" (yield on request, current file kept in recovery) or "Share my settings" (push). Never a silent overwrite, never first adoption |
| 6 | one head state H; `Bv` = heads of H (no newer shared revision) | L == cH: `InSync`. L ≠ cH: `LocalDifferent` ("Changed here, not shared"); nothing applied, whoever the authority is |
| 7 | one head state H; `Bv ≤ H` and `Bv` ≠ heads of H (H is **strictly newer** than the basis) | L == cH: `InSync`, basis = all heads of H. L == Cb (no local change): **yield**. L ≠ Cb and this computer is not the unit's authority: **yield**, with the current file captured (`before_change`) and kept as a recovery state with the notice "Your changes here were replaced by the shared settings; a recovery copy was kept" (R-08, A-12, A-13). L ≠ Cb and this computer is the authority: `LocalDifferent(ConcurrentPush)`, offering "Share my settings" (a push naming H's heads and its basis) or "Use shared settings"; nothing applied automatically |

A **yield** captures the current file (`before_change`), applies H (§4.3)
and sets basis = all heads of H. Automatic yield happens only in case 7,
so an unchanged head never overwrites a local experiment
(T-R4D18-unchanged-head-local-change), and an invalidated basis is never
mistaken for first adoption (T-R4D18-invalidated-basis).

**Equal heads (R4D-7).** Concurrent `choice` or `push` revisions with the
same checkpoint are several heads of **one** head state. They are not
collapsed to a representative: the basis records every head, and the next
`auto`/`push`/`restore` names **all** of them as parents. So the master's
next change after an equal-choice convergence has every equivalent head as
an ancestor and produces one head state, not a false conflict
(T-R4D7-equal-heads-frontier).

**Ancestry over the valid graph only (R4D-8).** Ancestry is never computed
through pending or invalid revisions, and a missing parent is never treated
as a root. The undo-then-next-session case (a `restore` revision followed
by the master's next session) is ordinary ancestry: the restore names the
basis set as parents and the next `auto` names the restore.

Paused units and units blocked by a diagnosis are skipped (§6, §1.2).

### 4.3 Applying a unit

Settings apply reuses the apply model of [apply.md](apply.md) §2–§4 with
single files instead of folders:

1. **Gate.** `ApplyGate` as for code: WoW for the variant not running, no
   existing apply journal, variant still valid. The gate check `G` runs
   before **every** live rename. If WoW starts, the op stops before the
   next rename and the journal is recovered later; no process is killed
   (R-11, A-21).
2. **Stage.** Copy each target blob to
   `<variant>\.wowsync\settings\staging\<op>\<n>`, flush, re-hash.
3. **Journal.** `apply-journal/2`, `kind: settings`, with one
   `SettingsFileOp` per file ([contracts.md](contracts.md) §2.2):
   `unit`, `live_rel`, `expected_live` (hash or absent), `staged` (or
   `None` to only move the live file out) and `recovery_slot`.
4. **Commit.** Per file: re-check that the live file still matches
   `expected_live` (a different file means WoW or the user wrote it: stop,
   capture, re-plan), `G`, rename live → recovery slot, `G`, rename staged →
   live. With `staged: None` it only moves live → recovery (the addon has no
   settings file in the chosen copy).
5. **Verify** the live hashes, record the capture of the new state with
   basis = all heads of the applied head state, write the settings-only
   `/2` recovery point, append the audit entry, finalize the journal,
   report `applied` in the next settings state.

Recovery after a crash uses the slot classification of apply.md §3–§4
(`classify_slots`: Untouched/Done/Unknown) per file. A unit is all-or-nothing
for its files: if any file op of the unit cannot be settled, the whole unit
is `needs_help` (Q-25) and its recovery slots are kept.

### 4.4 Competing intentional choices (R-09, A-14)

Two head states mean two intentional choices that neither knew about: for
example a laptop push and a master `auto` revision with the same parent,
two pushes, or a push and a restore.

- **No merge, no timestamp winner.** The app never combines files or picks
  by time, version or computer name.
- **Safe default while undecided:** no computer applies anything for that
  unit; each keeps its live files. Both checkpoints are pinned locally on
  every computer that has them and stay in the provider (they are
  referenced by revisions).
- The unit shows on the Decision page as an addon-level "Settings choice":
  addon, identity, the two (or more) sources (computer, kind "shared on
  purpose" or "master's automatic copy"), when each was made (local clock,
  Q-37), and the consequence: "The copy you don't pick is kept as a
  recovery copy." Choices: "Use <computer>'s settings" for each head, and
  "Decide later". Other addons and code continue (A-14).
- Resolving issues one `choice` revision with `parents` = **every head of
  every head state** shown and the chosen checkpoint. Two computers
  resolving at once produce two `choice` revisions: if they chose the same
  checkpoint they are two heads of one head state (§4.2, no new decision,
  and the next change parents both); otherwise it is a new decision whose
  resolution parents both earlier choices.
- `decide` re-derives the head states from current evidence before
  writing (contracts.md §5.5); if a head arrived meanwhile the choice is
  refused with `Changed` and shown again.
- A competing master `auto` revision is "intentional" because the master's
  completed-session changes are the group's default authority (R-09
  names "unpublished master changes" explicitly).

### 4.5 Stale master copies (A-13)

The master must not undo an accepted push with its old copy:

- Apply happens only when WoW is not running. If the master was playing
  when the push arrived, its session-end capture is taken **before** the
  push is applied, with the old basis set. If the session changed
  nothing (`changed_in_session` false, or the content equals the basis
  checkpoint), nothing is published and the push is then applied (yield).
- If the master did change the addon in that session, its `auto` revision
  has the old parents, so it competes with the push: a decision (§4.4), not
  a silent overwrite either way.
- After applying the push, the master's basis is the push's head state, so
  its next `auto` revision descends from the push.

### 4.6 Paired selection (Q-80)

When code version X of an addon is restored (removal restore, rollback),
the settings restored with it are chosen per unit of X's folders, in this
order, and **never** by comparing different computers' `local_seq` or
clocks:

1. The settings roots of the `/2` recovery point that took X out
   (contracts.md §2.3): exact pairing from the same operation.
2. Otherwise, this computer's own local history states of the unit whose
   `code_package_hash` is X's package hash: the one with the highest
   **local** `local_seq` (one computer's own counter).
3. Otherwise, valid revisions of the unit with `code_package_hash` = X:
   if exactly one of them is not an ancestor of another such revision, use
   it; if several are mutually unordered, do not pick: restore code only
   and list each as "Settings saved with this version on <computer>" in
   the unit's history, for an explicit restore.
4. Otherwise, code only, with the warning "Settings were not restored: no
   saved copy for this version" (A-19).

Paired, partial and code-only outcomes are reported distinctly
(`paired` field, contracts.md §2.3).

## 5. Settings master and exceptions (R-07, A-11, A-45)

### 5.1 Authority for a unit

For unit `{folder, identity}`: collect the authority each policy head
gives the folder: its `folders` exception if one names the folder,
otherwise its `master`. `not_shared` means no authority and no sync.
`null` master means no automatic publication; pushes, choices and restores
still work. If the policy heads disagree for the folder, the folder is in
`PolicyConflict` (§5.2). The local package key is never consulted, so a
folder keeps its rule when it joins or leaves a package (R4D-6,
T-R4D6-regroup-policy-stable). The settings master is not code authority
(contracts.md §4.1).

### 5.2 Assigning and reassigning

- Settings sync is **off** in a new group: there is no policy record and
  nothing under `WTF` is read for sharing (Q-70).
- "Choose a settings master" / "Change master…" (Settings sync page,
  Computers page) issues a new `settings-policy` whose `known` covers all
  current policy heads. The same record carries exception changes
  ("Follow master", "Use <computer> for this addon", "Don't share this
  addon"), written for every folder of the addon.
- **Never re-elected.** Nothing in this design writes a policy record
  except an explicit user action. In particular, master retirement,
  long absence, an unusable master state or a missing master folder never
  changes the master.
- **Concurrent policy changes.** Two policy heads that differ (Q-73): every
  addon whose effective authority differs between the heads has no
  automatic publication and no automatic apply until the user picks one
  policy ("Settings master was changed on two computers"). Addons whose
  authority is the same in every head continue. Choosing issues a policy
  whose `known` covers both.
- **Becoming master** does not publish existing local differences. Only
  differences that arise inside the authority epoch (§4.1) publish
  automatically, including after a restart. The confirmation for "Make
  this computer the master" lists addons whose settings differ here and
  offers "Share these now" (a push) as a separate checkbox, off by default
  (Q-75).
- **Consent is truthful about accounts (Q-70).** "Make <other computer>
  the master" states that only the accounts that computer has included are
  shared from it, and names them as reported in its settings state; it
  never implies that this computer's account list applies there.

### 5.3 Master absent: retirement or offline

| Situation | Behaviour | GUI |
|---|---|---|
| The master (or a folder's exception computer) is **retired** ([ops.md](ops.md) §2; barrier [contracts.md](contracts.md) §1.10) | Its revisions with `issued_at_seq ≤ Cutoff` stay valid; later ones are invalid and leave the graph (§4.2 step 3). No automatic publication for its units until the user reassigns. Pushes, choices and restores still work. The policy is not changed. Retirement is monotone: nothing later makes it the master again without re-enrollment and an explicit reassignment (Q-60 PENDING OWNER) | Settings sync page and Needs attention: "The settings master Den PC was retired. Choose a new master; WoW Sync does not pick one for you." Button "Change master…" (A-45) |
| The master is **offline** or has not published for a long time | Nothing changes. Others keep its last revision; their routine changes stay local; pushes are accepted without the master (Q-74). When it returns, its captures with an old basis compete with pushes made meanwhile only if it changed those addons (§4.5) | "Settings master Den PC last shared settings 12 days ago." with "Change master…" as an option, not a prompt |
| The master's state is `Unusable` or `Regressed` | As offline, plus the existing admission explanation | as offline, with the admission reason |
| The master **re-enrolls** after retirement | It is a new enrollment, not the master, until the user reassigns | "Den PC rejoined. It is not the settings master." |

### 5.4 Consent and first adoption on each computer

- Turning on settings sync (choosing the first master) shows, before
  anything is shared: "All addon settings on this computer are shared,
  including addons you install later. WoW's own settings, key bindings,
  macros and layout are not shared. Settings stay inside each account and
  character." plus the list of accounts found, each with a checkbox
  (included by default only on the computer making the choice) (R-07).
- On every other computer, settings sync starts only after the user
  confirms there once ("Den PC is now the settings master. Use its addon
  settings here?"), with the same account list (Q-70). Until then that
  computer publishes nothing and applies nothing for settings.
- **First adoption** of a unit with no basis on that computer: the local
  files are captured, kept as a recovery state, and the head is applied.
  The summary names how many addons were replaced and where the recovery
  copies are.

## 6. Pause (R-08, A-15)

- **Pause settings for an addon** (Add-on detail) pauses every unit of its
  folders on this computer (the pause set holds folder keys, so a
  regrouping never loses the pause). **Pause all settings** (Settings sync page) pauses every
  unit of the variant on this computer, including addons installed later,
  and does not pause code (Q-83). Pause lasts until resumed (Q-28).
- While paused: captures continue locally (history and recovery), but no
  revision is published and no revision is applied for those units.
  Incoming revisions are learned and listed as "waiting".
- The settings state reports the pause, so every computer shows "Settings
  paused on Laptop" for those addons and the group never shows them as
  in sync (A-15, A-24).
- **Resume** never publishes. It compares the current files, the basis and
  the current head, and explains before acting:

| Here | Shared meanwhile | Resume explains | Choices (default first) |
|---|---|---|---|
| unchanged | unchanged | "Nothing changed." | Resume |
| changed | unchanged | "Your changes stay on this computer and are not shared." | Keep mine here; Share mine; Go back to shared settings |
| unchanged | newer copy from <computer> | "Den PC's newer settings will be used here." | Use shared; Stay paused |
| changed | newer copy from <computer> | "Den PC's newer settings will replace your changes. Your changes are kept as a recovery copy." | Stay paused; Use shared (keep mine as recovery copy); Share mine instead |

"Share mine instead" is a push and is resolved as any push (§4.2, §4.4).
If this computer is the master, "Keep mine here" leaves the difference
unshared; the confirmation says that the next play session that changes
the addon will share all its settings.

## 7. Three states and exceptions (R-10, A-16, A-17)

- Per unit, settings history keeps the **current plus two previous**
  checkpoints, independently from code (`KEEP_OWN_STATES = 3`, as code in
  [apply.md](apply.md) §2.4). States are local captures, applied revisions
  and recovery copies (yielded, replaced, removed), in `local_seq` order.
- **Pinned beyond three** (shown as a temporary exception with its reason,
  as in the removal design's history section):
  - both sides of an unresolved settings choice (§4.4);
  - the checkpoint paired with a retained code recovery point (paired
    restore promise, [rollback.md](rollback.md));
  - settings in a removal recovery point;
  - a suspect capture until a complete one follows;
  - the files of a unit in `needs_help`.
- When the reason ends, the extra states are pruned back to three, oldest
  first, never the current one and never one that a retained recovery point
  names. A pinned checkpoint is pruned only with the recovery point that
  promises it.
- Pruning deletes only `.wowsync\settings\…` data, never a live file. If
  storage is low, the app stops capturing new states and shows "Not enough
  space to keep settings history" with the size needed; it never prunes a
  pinned state to make room (A-17). A capture that fails (read error,
  disk full while storing blobs) is not recorded as a state, leaves
  nothing half-written in the store (temp + rename), and the unit shows
  "Settings could not be saved" with the reason; the previous three states
  are untouched.
- **GC roots (Q-76).** A checkpoint (local manifest and blobs, or provider
  tree and blobs) is a root, and therefore kept, while any of these names
  it:
  - the unit's current plus two previous local states;
  - a pinned exception above;
  - a `SettingsRoot` of a retained `/2` recovery point
    ([contracts.md](contracts.md) §2.3), local or provider;
  - a `SettingsRoot` of a live removal record ([removal.md](removal.md)
    §7.3);
  - any head of the unit's revision graph, and every head of an
    unresolved settings choice;
  - a paired-selection candidate (§4.6) of a retained code recovery point.
- Provider data: revision records are immutable and never deleted.
  Provider blobs and manifests that no root names are **candidates** for
  the future provider collector (Q-7). This design does not claim the
  collector exists, and the GUI does not promise provider recovery of a
  checkpoint that no root names.

## 8. Removal, re-add and identity boundaries

This section implements [removal.md](removal.md) §6.1:

| Removal needs | Here |
|---|---|
| `settings_scope(variant, folders, identities)` → owned units + diagnoses | §1.1–§1.2, per folder. For a folder already gone from `Interface\AddOns`, ownership uses the `.toc` declarations recorded in the presence baseline ([removal.md](removal.md) §1.6) or, on a receiver, the removal record's manifest (§6.3 there) |
| `settings_checkpoint(unit)` → `CheckpointRef` | §3.1, one checkpoint per `SettingsUnit` |
| a plan of single-file moves to recovery with `expected_live` and `G` before each move | §4.3 with `SettingsFileOp.staged = None`, in a `/2` journal of kind `remove` or `settings_cleanup` |
| restore to the same unit only | §4.3 applying a recovery checkpoint to the same `SettingsUnit`; any other identity is refused |
| owned-only; shared or unknown ownership gives `code_only` | §1.2 |
| paused settings are kept on removal (Q-47) | the pause wins: paused units are not moved; the ack says `kept_paused` and the recovery point lists them as not captured |
| `paired` / `partial` / `code_only` / `settings_only` / `not_applicable` | derived from the point's code and settings roots (contracts.md §2.3), never stored as a free value; `not_applicable` when no removed folder declares saved variables |
| settings cleanup on the observing computer before its ack (R4D-5) | [removal.md](removal.md) §6.3 uses §4.3 and these units |
| no merge, no master needed | removal moves files; it never publishes a revision |

WoW keeps the saved variables of an uninstalled addon (Q-45, D-21).
So a removal must move the owned files itself, or the GUI must say that it
did not. Re-adding an addon does not restore settings automatically. The
"undo removal" restore restores code and paired settings together; a plain
re-install (a newer release appears) applies the current shared revision if
one exists, otherwise leaves the unit empty.

## 9. Status, explanations and GUI touchpoints

### 9.1 Unit status

| Status | Meaning | Shown as (example) |
|---|---|---|
| `Off` | no policy, or not confirmed on this computer, or account not included | "Settings not shared" |
| `InSync` | live == head, reported `applied` | "Settings up to date (from Den PC)" |
| `Waiting` | newer head, blocked by WoW running or by another journal; or `Waiting(MissingParent)` (§4.2) | "New settings from Den PC will be used when WoW closes"; "Waiting for settings information from Den PC" |
| `LocalDifferent` | live differs from head and is not published (non-authority, `EpochNotClean`, `BasisNotShared` or `ConcurrentPush`, §4.1–§4.2) | "Changed here, not shared" with "Share my settings" (and "Use shared settings" for `EpochNotClean`, `BasisNotShared` and `ConcurrentPush`) |
| `NoSharedCopy` | a policy exists but no valid revision of the unit yet | "Not shared yet" |
| `Paused` | §6 | "Settings paused on this computer" |
| `NeedsChoice` | §4.4 | "Choose which settings to keep" |
| `PolicyConflict` | §5.2 | "Settings master was changed on two computers" |
| `MasterAbsent` | §5.3 retired | "The settings master Den PC was retired…" |
| `Diagnosed(kind)` | §1.2 | "WoW Sync can't tell which addon owns these settings" |
| `Suspect` | §2.2 | "Settings may be incomplete after WoW stopped" |
| `CaptureFailed` | §7 | "Settings could not be saved" |
| `NotShared` | group exception | "Not shared (your choice)" |

The group roll-up for settings never says "in sync" while any included unit
on any non-retired computer is `Waiting`, `Paused`, `LocalDifferent`,
`NeedsChoice`, `Diagnosed`, `Suspect`, `CaptureFailed` or unconfirmed (A-15, A-24).

### 9.2 Explanations (R-12, A-25)

Each settings change appended to the audit and Activity names: addon,
identity in plain words ("account ERIC", "Thrall on Argent Dawn"), source
computer, kind ("Den PC's automatic copy after play", "Laptop shared on
purpose", "Your choice", "Restored"), what was replaced and where the
recovery copy is. No revision ids, hashes or Lua in the default view; they
are in Details.

### 9.3 Screens touched (screen design stays in `ui/`)

| Screen ([ui/screens.md](ui/screens.md)) | Change |
|---|---|
| Settings sync (now "coming later") | Settings master panel with "Change master…"; exceptions list; "Share my settings"; "Pause all settings"; accounts included; master-absent text; storage-pressure message |
| Add-on detail | settings status per identity; "Pause settings"/"Resume"; "Share my settings"; "Don't share this addon"/"Use <computer> for this addon"; settings history (three states, exceptions) with "Restore"; diagnosis text |
| Decision | "Settings choice" and "Settings master changed on two computers" (§4.4, §5.2) |
| Needs attention | master absent, suspect capture, `needs_help`, diagnoses, unconfirmed computer, storage pressure |
| Activity | yields, pushes, choices, restores, first adoption |
| Computers | which computer is master; retire flow states the master consequence (`ops.md`) |
| Setup / consent | §5.4 consent text and account list |

This closes data gap UG-21 (settings) and, together with `ops.md`, the
master-absence part of UG-20.

## 10. Implementation map

File ownership and build order are [contracts.md](contracts.md) §9 only;
this section says what each part contains.

### 10.1 Built by step 1, R4-core-contracts

| File | Settings contents |
|---|---|
| core `toc.rs` | `TocInfo.saved_variables: Vec<String>`, `saved_variables_per_character: Vec<String>`: names from the `## SavedVariables` and `## SavedVariablesPerCharacter` lines, validated as identifiers, metadata only |
| core `ids.rs` | `Identity { Account(AccountName), Character(AccountName, RealmName, CharName) }`, `SettingsUnit { folder: FolderKey, identity: Identity }`, `CheckpointRef`, `SettingsRoot`; NFC/case diagnoses (contracts.md §3). No revision or policy id types: records are named by their hash (contracts.md §1.3) |
| core `paths.rs` | `wtf_saved_variables_rel(identity, folder) -> Result<RelPath, PathError>`: builds only the two §1.1 shapes; `classify_wtf_rel(&RelPath) -> WtfPathKind` |
| core `kinds.rs` | the `settings-policy` and `settings-revision` DTOs with V-POLICY and V-REVISION (§3.2, §3.3), including `policy_folders` and `revision_parents` parts |
| core `records.rs` | `SettingsCheckpointManifest`, `SettingsState`, `SettingsLocal` (with `authority_since`, `epoch_clean`, the recorded basis and its checkpoint), `SettingsFileOp`, `ApplyJournal/2`, `RecoveryPoint/2` |
| core `plan.rs`, `exec.rs` | `PackageApply.settings_ops`; `PlanBlock::SettingsPaused`, `SettingsDiagnosed`; the `SettingsEffects` port (`stage_blob`, `live_fact`, `rename_live_to_recovery`, `rename_staged_to_live`, `gate`); settings slot execution and `recover_interrupted` for `settings_ops` |
| core `status.rs`, `explain.rs`, `view.rs` | `SettingsStatus` per unit (§9.1) including `LocalDifferent(BasisNotShared | ConcurrentPush | EpochNotClean)`, group roll-up, `Explanation` variants for §9.1 and §9.2, `NextAction::{ChooseSettings, ChooseMaster, ReviewResume, ConfirmSettingsHere, FixSettingsScope}` |
| core `hooks.rs` | `SettingsHooks` (contracts.md §9.4) |
| adapters `fs_safe.rs`, `fs_install.rs`, `provider.rs` | the `WtfScope` root with the reparse policy and writes restricted to an approved plan; settings staging and recovery slots; settings stream (`write_atomic`) and checkpoint manifest and blob upload (write-if-absent) |
| `lib.rs` of core, adapters and runtime; stubs | the module declarations and empty stubs of every §10.2 file (contracts.md §9.1); R4-core-settings fills them and edits no `lib.rs` |
| runtime `activity.rs`, `control.rs`, `service.rs` | the session-end event; the commands `SetPolicy`, `ShareSettings`, `PauseSettings`, `ResumeSettings(ResumeChoice)`, `ChooseSettings`, `RestoreSettings`, `ConfirmSettingsHere`, `UseSharedSettings` and the `SettingsHistory` query; the cycle steps below calling `SettingsHooks` |

### 10.2 Built by step 2, R4-core-settings

| File | Contents |
|---|---|
| core `settings.rs`, `settings_tests.rs` | the `SettingsHooks` implementation: `settings_scope(&VariantRoot, &Inventory, &WtfListing, &IncludedAccounts) -> SettingsScope`; `checkpoint_of`; `classify_capture(prev, settle_scans, activity) -> CaptureClass {Complete, Suspect, Unchanged}`; `policy_heads`, `authority_for(&PolicyView, &FolderKey)`, `master_state(&PolicyView, &Membership) -> MasterState { Present, Absent { was, why }, NotChosen }` (consumed by ops.md §2.6); the authority epoch (§4.1); `revision_graph(&Ledger, &SettingsUnit) -> RevisionGraph { nodes, pending, heads, head_states }`; the ordered target cases of §4.2; `publish_decision`; `paired_selection` (§4.6); `gc_roots` (§7); `resume_preview`; `KEEP_SETTINGS_STATES = 3` re-using `plan::KEEP_OWN_STATES` |
| adapters `wtf.rs` | `scan_wtf(&VariantRoot, &IncludedAccounts) -> WtfListing`: lists `WTF\Account\*\SavedVariables\*.lua` and `WTF\Account\*\*\*\SavedVariables\*.lua` only, with size and SHA-256; no other file content is read |
| adapters `settings_store.rs` | local blob and manifest store under `<variant>\.wowsync\settings\`; `SettingsLocal` persistence through the existing `localstate` API (no edit of `localstate.rs`) |
| runtime `settings_cycle.rs` | the bodies of steps 3s, 5s and 7s and of the settings commands; the `SettingsHistory` query result (the settings history view; `detail.rs` is not touched) |

Rollback's settings caller is rollback.md §9. Screens of §9.3 are R4-app's.

### 10.3 Cycle

Cycle (removal.md §1.6 numbering, new steps marked; the skeleton is
step 1's `service.rs`): 1 recover (also `settings_ops`), 2 admit (also
settings stream, contracts.md §6), 2a learn authority (also policy and
revisions), 3 scan, **3s scan WTF and capture (§2)**, 3a observe removals,
4 publish (code state, then settings records, index and settings state), 5
target, **5s settings target (§4.2)**, 6 gate/plan, 7 apply code, **7s
apply settings (§4.3)**, 8 status. Settings steps never delay the code
steps for other addons.

### 10.4 `wow-sync-app`

`pages.rs`/`model.rs`/`backend.rs` (R4-app, step 3): the screens of §9.3,
driven only by core `StatusView`, `Explanation` and `NextAction` (no logic
in the GUI).

## 11. Test obligations

All offline tier unless stated. "Neg" marks a negative case. Each test
belongs to `tests.md` when implemented and needs an A-38 control (a
deliberate break that makes it fail).

| ID | Test | A-ID |
|---|---|---|
| T-S01 | master + one exception: each addon's `auto` revision comes only from its authority; a newly installed addon on the master publishes after its first session | A-11 |
| T-S02 | 50 writes to one file while WoW "runs", then exit: exactly one capture; identical captures are not stored twice | A-11 |
| T-S03 Neg | file still changing at exit, or shrunk to zero with `.lua.bak` present: capture suspect, not published | A-11, A-21 |
| T-S04 | app not running during two sessions: one `found_at_start` capture | A-11 |
| T-S05 | laptop routine change, then master revision: laptop yields; replaced copy restorable from history; no laptop revision exists | A-12 |
| T-S06 Neg | laptop routine change with no new master revision: laptop files untouched, status `LocalDifferent`, nothing published | A-12 |
| T-S07 | laptop push: every computer including the master applies it; master's next unchanged session publishes nothing | A-13 |
| T-S08 Neg | master playing during push, no change in session: no stale `auto` revision after exit | A-13 |
| T-S09 | push vs unpublished master change on same parent: both heads, `NeedsChoice`, no apply anywhere, both checkpoints pinned; another addon updates meanwhile | A-14 |
| T-S10 Neg | no merged bytes ever; choosing by older/newer timestamp is impossible (fixture with reversed clocks gives the same decision) | A-14, A-20 |
| T-S11 | two computers resolve the same choice identically: two heads of one head state, no decision; then the master's next changed session, a push, a replay of both choices, and two later resolutions: never a false conflict (extends to T-R4D7); differently: new decision | A-14, A-20 |
| T-S12 | pause one addon, then pause all: no revision published, no apply, status "paused" in group view | A-15 |
| T-S13 | each resume row of §6 gives the stated explanation and default; resume never publishes | A-15 |
| T-S14 | more than three captures per unit: three kept; code history independent | A-16 |
| T-S15 | pinned states (choice, paired, removal, suspect, needs_help) survive pruning with an exception, then prune back to three | A-17 |
| T-S16 Neg | low storage: no pinned state deleted; needs-help message | A-17 |
| T-S17 Neg | account identity settings never applied to another account; character never to another character or realm; case-only differences and NFC collisions diagnosed on synthetic NTFS and APFS name fixtures (contracts.md T-C11, offline; native volumes NOT RUN) | A-23 |
| T-S18 Neg | `config-cache.wtf`, `bindings-cache.wtf`, `macros-cache.txt`, `layout-local.txt`, `AddOns.txt`, `Blizzard_*.lua` never read, moved or written | A-23 |
| T-S19 Neg | shared declared variable: both packages' units diagnosed, live files untouched, other addons sync | A-23 |
| T-S20 Neg | file with Lua that would write a file or run a command if executed: bytes copied, nothing executed (no process spawn, no file created) | A-23 |
| T-S21 Neg | junction/symlink at `WTF`, account, realm, character or `SavedVariables`: no write through it | A-23 |
| T-S22 Neg | malformed or oversized policy/revision/state records, a parent of another unit, a wrong-kind policy ref: refused per record (`invalid` or `pending`, contracts.md §1.11), others still used | A-20, A-23 |
| T-S23 | WoW starts between file renames: stop before next rename, recovery settles later, no process killed (real temp filesystem) | A-21, A-22 |
| T-S24 | crash/disk-full/permission failure at every settings op transition: next start reports the actual state; prior files recoverable | A-22 |
| T-S25 | master retired: no automatic publication for its units, master-absent status and reassign action; no policy record written automatically | A-45 |
| T-S26 Neg | stale publication from the retired master above its cutoff: not valid, not a graph node, does not reactivate it; a later retirement with a higher cutoff re-validates nothing, one with a lower cutoff invalidates the records above it (MIN cutoff, contracts.md §1.10) | A-45 |
| T-S27 | concurrent policy changes: only addons whose authority differs are held | A-20 |
| T-S28 | three replicas with reordered, duplicated and deleted deliveries and a manifest without its blobs: no false `InSync`, no apply of a partial checkpoint | A-20 |
| T-S29 | removal of an addon with settings: owned files moved to recovery; "undo removal" restores both to the same identity; diagnosed ownership gives code-only with the GUI text | A-09, A-19 |
| T-S30 | the slice app with a group holding settings records, index chunks and a settings stream: it ignores them and never applies or removes settings; a `/2` settings journal under the slice gives `NeedsHelp` (contracts.md T-C06) | A-20, A-22 |
| T-S31 | group status never "in sync" with any paused, waiting, local-different or unconfirmed unit; confirmation needs an `applied` report | A-24 |
| T-S32 (GUI) | setup consent, change master, exception, share, pause/resume, settings choice and restore all complete without a terminal, with keyboard and screen evidence | A-25, A-26 |
| T-R4D6-regroup-policy-stable | suite {a, b} with `not_shared` for a's folder and default master for b; pause on a; then folder a is removed (and later re-added) and peers group the suite differently | b's units keep the master rule, a's `not_shared` and pause stay on folder a; no unit changes key; no sharing is granted by regrouping; recovery stays paired with the right folder | A-11, A-15, A-18, A-20 |
| T-R4D7-equal-heads-frontier | concurrent `choice` revisions a and b with the same checkpoint; master's next change c; then a push, a replay of a and b, and two later resolutions | c's parents are {a, b}; one head state throughout; no `NeedsChoice` until a genuine competing change | A-13, A-14, A-20 |
| T-R4D8-child-before-parent | revision c arrives before its parent p, then p; with and without restarts | `Waiting(MissingParent)` with nothing applied; after p, ordinary yield; never a false root or conflict | A-20 |
| T-R4D8-retired-basis | this computer's basis revision is from a master later retired with a cutoff below it; a valid push descends from it | the basis is invalid; the push stays a node; target is `LocalDifferent(BasisNotShared)` with "Use shared settings", never a silent overwrite | A-20, A-45 |
| T-R4D8-undo-then-next-session | a `restore` revision, then the master's next session | the restore parents the basis set; the next `auto` parents the restore; one head state | A-18, A-20 |
| T-R4D8-zero-heads | policy set, no valid revision; and every revision invalid | `NoSharedCopy`; nothing applied or deleted; first `auto`/`push` creates the root | A-20 |
| T-R4D19-epoch-equal-seq | last complete capture has `local_seq` 5; app closed; the laptop's file changes (experiment); the laptop is made master elsewhere; app starts (`authority_since` = 5, `found_at_start` compared with 5); also: WoW running at activation; and a later genuine session | the experiment is not published: the activation capture differs from the basis, `epoch_clean = false`, `LocalDifferent(EpochNotClean)` with "Share my settings" / "Use shared settings"; the session running at activation is not published; after the user's choice, the next completed session (compared with a capture above 5) publishes; A-38 controls as listed below the table (ignoring the dirty-epoch hold fails this scenario; `>=` instead of `>` fails the isolated clean-epoch equal-seq fixture) | A-11, A-12, A-20 |
| T-R4D18-unchanged-head-local-change | one head h, basis {h}, laptop (non-authority) changes its file; no new revision; then the master publishes h2 descending from h | before h2: `LocalDifferent` (case 6), file untouched, nothing published; after h2: yield (case 7) with the laptop's file kept in recovery and the notice; with the laptop as authority instead: `LocalDifferent(ConcurrentPush)`, nothing applied | A-12, A-14, A-20 |
| T-R4D18-invalidated-basis | basis {r}, r later invalid (issuer retired below r); valid head h not descending from any valid basis element; live differs from h | case 5 `LocalDifferent(BasisNotShared)`; never first adoption (§5.4), never a silent overwrite; "Use shared settings" yields with recovery | A-20, A-45 |
| T-R4D4-policy-parts | a policy with 1 000 folder exceptions and a choice revision with 100 parents | written with `policy_folders` / `revision_parents` parts, valid on every peer, never refused; a folder repeated in two parts is `invalid(policy_body)` | A-11, A-14, A-20 |
| T-R4D4-nine-policy-heads | nine concurrent policy heads that all make the laptop master of folder a; the laptop's `auto` revision; then a tenth head that makes the den master of a, known only to one reader | the revision lists the 8 smallest head hashes and is valid everywhere; readers that know only the nine apply it; the reader that knows the tenth shows `PolicyConflict` for a and does not apply it, and it is neither invalid nor dropped; after a policy choice for the laptop it applies; A-38 control: validate only the listed heads (the tenth-head reader applies, test fails) | A-11, A-20 |
| T-R4D30-reassigned-author | P0 makes A master of folder a; A publishes `auto` r; P1, covering P0, makes B master; B has no later session. Readers: (1) a member following r; (2) a newly joining computer (first adoption, §5.4); (3) an explicit push from non-master C after P1 | (1), (2): one policy head, no `PolicyConflict`; r is valid and adopted by §4.2/§5.4 with no prompt to make A master again; A writes no further `auto` revision for a; (3) the push is applied by the ordinary target rules and is not held for policy; A-38 control: re-check r's issuer against the current heads (r held as `PolicyConflict`, test fails) | A-11, A-13, A-20 |
| T-S-paired-selection | restore code X with: an exact `/2` pairing; only local history with X's hash; two unordered remote revisions with X's hash; none | exact pairing; newest local; code-only with both offered explicitly; code-only with the A-19 warning. Never by comparing peers' `local_seq` or clocks | A-18, A-19 |
| T-S-retention-roots | paired recovery roots, unresolved-choice heads, live removal roots and provider-location roots across prune and restart; a 16 MiB+ ledger (contracts.md T-C09) | no root pruned; unrooted checkpoints become collector candidates only; nothing claims provider recovery for them | A-16, A-17 |
| T-S-capture-failures | read error, disk full while storing blobs, crash between blob and manifest | no partial state recorded; previous three states intact; `CaptureFailed` shown | A-17, A-22 |
| T-S-size-cap-pending | a file above the test-only cap | `TooLarge` diagnosis; the approved cap is 32 MiB (Q-77, D-26); with no cap value, settings sync stays disabled | A-23 |

A-38 controls to include, at minimum: publish non-master captures
automatically (T-S06 must fail); pick the newer timestamp for a choice
(T-S10); apply a revision to a different account (T-S17); skip `G` before a
settings rename (T-S23); auto-elect a master on retirement (T-S25);
collapse equal heads to one representative parent (T-R4D7); treat a
missing parent as a root (T-R4D8-child-before-parent); key policy by
package key (T-R4D6); for T-R4D19-epoch-equal-seq, two separate
controls: (1) ignore the dirty-epoch hold (publish while
`epoch_clean = false`) must fail the main scenario, which is what
directly guards the experiment; (2) an isolated fixture with
`epoch_clean = true`, an activation capture **equal** to the basis and a
later capture with the same `local_seq` as `authority_since` must not
publish, so mutating the strict `>` to `>=` fails it on its own; yield on
an unchanged head
(T-R4D18-unchanged-head-local-change); shrink an invalid basis to empty
(T-R4D18-invalidated-basis); validate only the listed policy witness
(T-R4D4-nine-policy-heads); re-check a historical author against a later
settled policy (T-R4D30-reassigned-author).
