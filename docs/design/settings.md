# Addon settings sync

Status: **design for a later run** (run-004 packet
[R4-settings-design](../runs/run-004/packets/R4-settings-design.md)). Not
yet implemented and not yet independently reviewed. Nothing here changes the
running slice, which does not read or write anything under `WTF` today.

Product authority: SPEC R-07, R-08, R-09, R-10, R-11 and R-12
([SPEC.md](../../SPEC.md)). Acceptance: A-11 to A-17, A-19, A-20 to A-26 and
A-45 ([ACCEPTANCE.md](../../ACCEPTANCE.md)). Rollback and rejection are in
[rollback.md](rollback.md). This document builds on
[domain.md](domain.md), [publication.md](publication.md),
[apply.md](apply.md) and [core-api.md](core-api.md). It implements the
settings interface that [removal.md](removal.md) requires, in that
document's "settings for removed addons" section. Choices that approved
intent does not settle are marked **(Q-n)** and listed in
[questions.md](questions.md) from Q-54 on. The computer-retirement design
(`docs/design/ops.md`, run-004 R4-ops-design, written in parallel) uses the
same terms **settings master** and **reassign**.

## 0. Vocabulary

| Term | Meaning |
|---|---|
| identity | an account or character scope inside one variant. Account: `account:<ACCOUNT>`. Character: `char:<ACCOUNT>/<Realm>/<Character>`. Names are the exact WTF folder names (§1.4) |
| settings file | one saved-variable file that WoW writes for an addon folder: `<Folder>.lua` in an identity's `SavedVariables` folder (§1.1) |
| settings unit | `(package_key, identity)`: all settings files of one package for one identity. History, sharing, decisions and pause all work on units. The GUI groups units by addon ("per addon") and shows the identity as the scope |
| checkpoint | the exact bytes of every settings file of one unit at one moment, held as a content-addressed manifest (§3.1). Bytes are opaque |
| capture | a local checkpoint taken at a completed play-session boundary (§2) |
| revision | a group-visible, immutable record that says "the shared settings of unit U are now checkpoint C", with its kind and its parents (§3.2) |
| head | a revision of a unit that no other valid revision names as a parent. One head means the unit has one shared state. Two or more heads with different checkpoints means competing intentional choices (§4.4) |
| basis | the revision whose checkpoint is in this computer's live files for a unit, as recorded locally when it last applied or published (§2.3) |
| settings master | the computer whose completed-session captures publish automatically. One default master per group, plus per-addon exceptions (§5). Assigned and reassigned only by an explicit user action |
| routine change | a capture on a computer that is not the authority for that unit. It stays local and is never published automatically (R-08) |
| push | a deliberate user action on any computer that publishes its current checkpoint of one or more units as the new shared state ("Share my settings") |
| yield | replacing a computer's routine local settings with a newer shared revision, after moving the replaced files into recovery (§4.2) |
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

`<Folder>` is the name of an addon folder of an installed package
([domain.md](domain.md) §5), compared case-insensitively on Windows and
exactly otherwise, as domain.md does for folders. A file belongs to the
package that contains that folder **and** whose selected `.toc` declares
`## SavedVariables` (account file) or `## SavedVariablesPerCharacter`
(character file) **(Q-54)**. The declaration is `.toc` metadata, read by the
existing `.toc` rule; it is never Lua.

Everything else under `WTF` is out of scope and is never read for content,
written, moved or deleted by settings sync **(Q-55)**:

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
| Two installed packages declare the same saved-variable name in their `.toc` | `SharedVariable { name, packages }` | Units of every listed package are not synced in either direction; their live files are not touched |
| A file `<Folder>.lua` exists but no installed package folder declares saved variables for that shape | `Unowned { file }` | Not synced, not moved, not deleted. Listed in Details only, as information |
| A folder declares saved variables but the `.toc` selection for the folder is ambiguous ([domain.md](domain.md) `.toc` rule) | `AmbiguousToc { folder }` | Unit not synced |
| A path component is a reparse point, link, junction, or names differ only in case on a case-sensitive volume | `UnsafePath { path }` | Unit not synced; nothing under that path is written |
| A file exceeds the size limit (Q-64) | `TooLarge { file, size }` | Unit not synced |
| An identity folder name fails the safe-name rules of [domain.md](domain.md) (control characters, reserved names, trailing dot or space) | `UnsafeIdentity { name }` | Identity not synced |

There is **no fallback** to copying `WTF`, a whole account folder or a whole
`SavedVariables` folder. Settings sync never splits one file into parts and
never deletes a file it does not own. A removal of an addon whose ownership
is diagnosed is code-only for those units (removal design, owned-only rule;
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
  (`<ACCOUNT>`, `<Realm>`, `<Character>`) after Unicode NFC normalization.
  Names that differ only in case are not the same identity; on Windows
  such a pair on two computers is an `IdentityCaseMismatch` diagnosis for
  that identity **(Q-56)**.
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
real-client checks were deferred by D-23. Until a variant's layout is
verified on a real client, settings sync for that variant runs only on
synthetic test data (the Q-24 gate) **(Q-55)**.

## 2. Capture at completed play-session boundaries (R-07, A-11)

### 2.1 When a capture happens

A capture is taken per unit, never while WoW for that variant runs:

1. **Session end.** The activity monitor (existing `Activity` from
   [apply.md](apply.md) §5) sees the variant's WoW process stop. The
   capture waits for the settle window: two consecutive stable scans of
   the owned files, at least the Q-15 quiescence apart, with the same
   settle wait as removal detection (Q-41) **(Q-58)**.
2. **Startup / missed session.** At app start, or when the app has not seen
   a session end (it was not running, the computer slept), any unit whose
   files differ from the last capture or applied checkpoint is captured
   once, as kind `found_at_start`. Several missed sessions become one
   capture **(Q-59)**.
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
unit; otherwise the unit's capture is **suspect** **(Q-58)**:

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
  "unit": { "package_key": "details", "identity": "char:ERIC/Argent Dawn/Thrall" },
  "checkpoint": "<checkpoint hash>",
  "kind": "session_end|found_at_start|before_change",
  "complete": true,
  "basis": { "id": "<revision id>", "hash": "<object hash>" },
  "changed_in_session": true,
  "code_package_hash": "<sha256 of the installed package at capture>",
  "local_seq": 412
}
```

- `basis` is the revision applied or published by this computer for the
  unit before the session, or `null` if none.
- `changed_in_session` is true when the checkpoint differs from the
  previous capture or applied checkpoint of the unit **on this computer**.
  Only changed units can publish (§4.1). A difference that already existed
  before the session is not re-published by a later unrelated session.
- `code_package_hash` pairs the settings with the code that wrote them.
  Rollback uses it ([rollback.md](rollback.md), paired restore).
- `local_seq` is a local counter. Order never comes from timestamps.

## 3. Records and on-disk formats

### 3.1 Checkpoint manifest and blobs

A checkpoint is the canonical encoding of:

```json
{
  "schema": "wowsync.settings-checkpoint/1",
  "unit": { "package_key": "details", "identity": "account:ERIC" },
  "files": [
    { "folder_key": "details", "name": "Details.lua", "size": 183402, "blob": "<sha256>" }
  ]
}
```

- `files` is sorted by `folder_key`. An empty list is valid and means "this
  unit has no settings files" (for example after the user reset an addon).
- **Checkpoint hash** = SHA-256 over `"wowsync-settings-checkpoint-v1\n"` +
  canonical manifest. Same bytes, same unit, same hash on every computer.
- Blobs are the existing content-addressed provider blobs
  ([publication.md](publication.md) §1, §3). A manifest is stored in the
  provider as `trees\<checkpoint hash>.json` only when a revision
  references it (Q-63). Local captures stay local.
- Local store: `<variant>\.wowsync\settings\blobs\<sha256>` and
  `<variant>\.wowsync\settings\manifests\<checkpoint hash>.json`, on the
  same volume as `WTF`.

### 3.2 Group authority objects

Settings policy and revisions are **immutable authority objects** of the
same family as removal intents (removal design, "immutable authority
objects" and "authority ledger"): one file per object in
`groups\<g>\members\<computer>\authority\<id>.json`, write-if-absent,
temp + flush + rename, read back and hash-checked before any state that
references it is published, mirrored verbatim by every computer that learns
it, and learned into the durable grow-only local authority ledger. Common
fields: `schema`, `id`, `issuer {computer_id, enrollment_id,
issued_at_seq}`, `written_unix_ms` (display only). The object hash rule is
removal's: SHA-256 over `"wowsync-authority-v1\n"` + canonical encoding of
every field. References are variant references `{id, hash}`.

**Settings policy** (`wowsync.settings-policy/1`), a full snapshot each
time:

```json
{
  "schema": "wowsync.settings-policy/1",
  "id": "<32 hex>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 51 },
  "master": { "computer_id": "<den>", "enrollment_id": "<den enr>" },
  "addons": {
    "details": { "rule": "master", "computer_id": "<laptop>", "enrollment_id": "<laptop enr>" },
    "weakauras": { "rule": "not_shared" }
  },
  "supersedes": [{ "id": "…", "hash": "…" }],
  "written_unix_ms": 1767225600000
}
```

- `master` may be `null` (settings sync is set up but nobody publishes
  automatically; pushes still work).
- `addons` holds only exceptions. Absent means "follow the default master".
  `rule: "master"` names a different authority for that addon.
  `rule: "not_shared"` means the addon's settings are local only on every
  computer.
- `supersedes` lists every policy head the issuer had in its ledger.

**Settings revision** (`wowsync.settings-revision/1`):

```json
{
  "schema": "wowsync.settings-revision/1",
  "id": "<32 hex>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 52 },
  "unit": { "package_key": "details", "identity": "account:ERIC" },
  "kind": "auto|push|choice|restore",
  "checkpoint": "<checkpoint hash>",
  "parents": [{ "id": "…", "hash": "…" }],
  "policy": { "id": "…", "hash": "…" },
  "code_package_hash": "<sha256>|null",
  "rejects": [],
  "written_unix_ms": 1767225600000
}
```

- `kind`: `auto` = completed-session capture of the authority; `push` =
  explicit "Share my settings"; `choice` = resolution of competing heads
  (§4.4); `restore` = user restored an earlier copy, or a paired rollback
  ([rollback.md](rollback.md)).
- `parents` = the issuer's basis for an `auto`/`push`/`restore`, or every
  head being resolved for a `choice`. Empty only for the first revision of
  a unit.
- `policy` = the policy head under which the issuer acted. An `auto`
  revision is valid only if the issuer is the unit's authority under that
  policy (§5.1).
- `rejects` is used only by settings rejection ([rollback.md](rollback.md)).

**Settings state** (`wowsync.settings-state/1`), a new per-member stream in
`groups\<g>\members\<computer>\settings\<enrollment_id>-<seq:010>.json`,
written last, as the code state is:

```json
{
  "schema": "wowsync.settings-state/1",
  "computer_id": "…", "enrollment_id": "…", "seq": 88,
  "knows": [{ "id": "…", "hash": "…" }],
  "applied": [{ "unit": { "…": "…" }, "revision": { "id": "…", "hash": "…" },
                "checkpoint": "<hash>", "result": "applied|kept_local|paused|blocked|diagnosed" }],
  "paused": { "all": false, "packages": ["weakauras"] },
  "included_accounts": ["ERIC"],
  "capabilities": ["settings/1", "rejection/1"],
  "diagnoses": [{ "unit": { "…": "…" }, "kind": "SharedVariable" }]
}
```

`knows` lists every policy and revision object the computer has in its
ledger for the group. `applied` is the settings apply report
([publication.md](publication.md) §6): confirmation of a revision on a
computer requires an `applied` entry with the revision's checkpoint hash.
`kept_local`, `paused`, `blocked` and `diagnosed` are reported so the group
view never shows such a computer as in sync (R-12, A-15, A-24).
`capabilities` lists the object kinds this app version understands;
[rollback.md](rollback.md) §2.3 uses it before a group-wide rejection.

### 3.3 Local records (never uploaded)

| Record | Content |
|---|---|
| `wowsync.settings-local/1` (local state) | per unit: captures (newest first, §2.3), basis, last applied checkpoint, pinned flags; pause set; included accounts; per-computer consent and first-adoption choice (§5.4) |
| `wowsync.apply-journal/2` `settings_ops` | as introduced by the removal design; one op per settings file move (§4.3) |
| `wowsync.recovery-point/2` with `kind: "settings"` | yielded, restored-over or removed settings (§7) |

### 3.4 Compatibility

- All formats follow [publication.md](publication.md) §2 and Q-13: first
  field `schema`, unknown fields ignored within a major, unknown major
  refused and shown as "written by a newer version", size limits checked
  before parsing.
- Settings are new **streams and object kinds**, not a change to
  `wowsync.code-state`. A reader without settings support ignores
  `settings\` and does not list settings objects, so it neither applies
  nor removes settings. It does see unknown schema names in `authority\`;
  it must skip those (the removal design already requires per-object
  schema checks). An implementer must verify that the shipped reader
  skips, and does not refuse the whole folder for, an unknown authority
  schema; that is a test (T-S30).
- Admission of the settings stream uses the same rules as code states (the
  newest state only, no fallback to an older one, `Unusable`/`Regressed`
  handling), with a separate watermark in `wowsync.admission/1` as a new
  optional field.
- The removal tombstone's `settings` field changes from
  `{kind: "none_in_this_version"}` to `{kind: "checkpoints", roots: [{identity,
  checkpoint, location}]}` as a new removal-intent major, as the removal
  design states. `location` is `provider` when a revision references the
  checkpoint and `local` otherwise.

## 4. Sharing, receiving and applying

### 4.1 Who publishes what

For each unit, after each capture:

| This computer is… | Capture | Result |
|---|---|---|
| the unit's authority (§5.1), not paused, capture complete, `changed_in_session` | any | an `auto` revision, parent = basis |
| the authority, but the capture is unchanged, suspect, or the unit is paused | — | nothing published |
| not the authority | any | stays local as a routine change. **Never** published automatically (R-08, A-12) |
| any computer, user chooses "Share my settings" for the unit or addon | current files (captured first, kind `before_change`) | a `push` revision, parent = basis |
| any computer, unit diagnosed, not included, or addon `not_shared` | — | nothing published |

A push is offered only when WoW for the variant is not running (the files
must be a complete capture). A push for a whole addon issues one revision
per included identity that has files or a basis.

### 4.2 Receiving: the target per unit

The settings target is computed per unit from the valid revisions in the
ledger:

1. Drop invalid revisions: issuer not a valid member at `issued_at_seq`
   (retirement rules from `ops.md`), `auto` from a non-authority, rejected
   checkpoints ([rollback.md](rollback.md)), schema or size failures.
2. Compute heads. Heads whose checkpoint hashes are equal are one state
   (the representative is the smallest object hash).
3. One head state H:
   - live checkpoint == H's checkpoint: `InSync`; record basis = H.
   - basis is an ancestor of H (H is newer than what this computer had):
     **yield**. Capture the current files (`before_change`); if they
     differ from the basis checkpoint, keep that capture as a recovery
     state (it was a routine change or a stale copy); then apply H. The
     master and every other computer do this, so an accepted push reaches
     the master too (A-12, A-13).
   - basis == H and live differs: `LocalDifferent` ("Changed here, not
     shared"). Nothing is applied. The routine change stays until a newer
     head arrives or the user acts.
   - no basis (first adoption): §5.4.
4. Two or more head states: `NeedsDecision(SettingsChoice)` (§4.4).

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
3. **Journal.** `apply-journal/2` with `settings_ops`: for each file
   `{live_rel, expected_live: {size, sha256}|absent, staged, recovery_slot,
   target: {sha256}|absent}`.
4. **Commit.** Per file: re-check that the live file still matches
   `expected_live` (a different file means WoW or the user wrote it: stop,
   capture, re-plan), `G`, rename live → recovery slot, `G`, rename staged →
   live. A target `absent` only moves live → recovery (the addon has no
   settings file in the chosen copy).
5. **Verify** the live hashes, record the capture of the new state with
   basis = the applied revision, append the audit entry, finalize the
   journal, report `applied` in the next settings state.

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
- Resolving issues one `choice` revision with `parents` = all heads and the
  chosen checkpoint. Two computers resolving at once produce two `choice`
  revisions: if they chose the same checkpoint they are one state (§4.2
  step 2); otherwise it is a new decision that lists both earlier choices.
- A competing master `auto` revision is "intentional" because the master's
  completed-session changes are the group's default authority (R-09
  names "unpublished master changes" explicitly).

### 4.5 Stale master copies (A-13)

The master must not undo an accepted push with its old copy:

- Apply happens only when WoW is not running. If the master was playing
  when the push arrived, its session-end capture is taken **before** the
  push is applied, with basis = the old revision. If the session changed
  nothing (`changed_in_session` false, or the content equals the basis
  checkpoint), nothing is published and the push is then applied (yield).
- If the master did change the addon in that session, its `auto` revision
  has the old parent, so it competes with the push: a decision (§4.4), not
  a silent overwrite either way.
- After applying the push, the master's basis is the push revision, so its
  next `auto` revision is a child of the push.

## 5. Settings master and exceptions (R-07, A-11, A-45)

### 5.1 Authority for a unit

For package P: if the policy head has an `addons[P]` exception, use it;
otherwise use `master`. `not_shared` means no authority and no sync.
`null` master means no automatic publication; pushes, choices and restores
still work.

### 5.2 Assigning and reassigning

- Settings sync is **off** in a new group: there is no policy object and
  nothing under `WTF` is read for sharing (Q-57).
- "Choose a settings master" / "Change master…" (Settings sync page,
  Computers page) issues a new `settings-policy` with `supersedes` = all
  current policy heads. The same object carries exception changes
  ("Follow master", "Use <computer> for this addon", "Don't share this
  addon").
- **Never re-elected.** Nothing in this design writes a policy object
  except an explicit user action. In particular, master retirement,
  long absence, an unusable master state or a missing master folder never
  changes the master.
- **Concurrent policy changes.** Two policy heads that differ (Q-60): every
  addon whose effective authority differs between the heads has no
  automatic publication and no automatic apply until the user picks one
  policy ("Settings master was changed on two computers"). Addons whose
  authority is the same in every head continue. Choosing issues a policy
  that supersedes both.
- **Becoming master** does not publish existing local differences. Only
  captures from sessions that start after this computer learned the policy
  publish automatically. The confirmation for "Make this computer the
  master" lists addons whose settings differ here and offers "Share these
  now" (a push) as a separate checkbox, off by default (Q-62).

### 5.3 Master absent: retirement or offline

| Situation | Behaviour | GUI |
|---|---|---|
| The master (or an addon's exception computer) is **retired** (`ops.md`) | Its revisions issued before retirement stay valid. Later objects from it are invalid. No automatic publication for its units until the user reassigns. Pushes, choices and restores still work. The policy is not changed | Settings sync page and Needs attention: "The settings master Den PC was retired. Choose a new master; WoW Sync does not pick one for you." Button "Change master…" (A-45) |
| The master is **offline** or has not published for a long time | Nothing changes. Others keep its last revision; their routine changes stay local; pushes are accepted without the master (Q-61). When it returns, its captures with an old basis compete with pushes made meanwhile only if it changed those addons (§4.5) | "Settings master Den PC last shared settings 12 days ago." with "Change master…" as an option, not a prompt |
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
  settings here?"), with the same account list (Q-57). Until then that
  computer publishes nothing and applies nothing for settings.
- **First adoption** of a unit with no basis on that computer: the local
  files are captured, kept as a recovery state, and the head is applied.
  The summary names how many addons were replaced and where the recovery
  copies are.

## 6. Pause (R-08, A-15)

- **Pause settings for an addon** (Add-on detail) pauses all its units on
  this computer. **Pause all settings** (Settings sync page) pauses every
  unit of the variant on this computer, including addons installed later,
  and does not pause code (Q-70). Pause lasts until resumed (Q-28).
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
  pinned state to make room (A-17).
- Provider data: revision objects are immutable. Blobs and manifests of
  revisions older than each unit's newest three are not needed for
  recovery and become garbage-collection candidates under the future
  provider retention design (Q-7, Q-63).

## 8. Removal, re-add and identity boundaries

This section implements the removal design's "settings interface":

| Removal needs | Here |
|---|---|
| `settings_scope(variant, package, identities)` → owned items + `unknown_or_shared` | §1.1–§1.2. For a package that is already gone from `Interface\AddOns`, ownership uses the folder names and `.toc` declarations recorded in the presence baseline |
| `settings_checkpoint(scope)` → content-addressed ref scoped by `(GroupId, product, identity)` | §3.1, one checkpoint per unit |
| a removal plan of single-file moves to recovery with `expected_live` and `G` before each move | §4.3 with target `absent` |
| `settings_restore` to the same identity only | §4.3 applying a recovery checkpoint to the same unit; any other identity is refused |
| owned-only; shared or unknown ownership gives `removed_code_only` | §1.2 |
| paused settings are kept on removal (Q-47) | the pause wins: paused units are not moved; the recovery point is `code_only` for them |
| `paired` / `code_only` / `settings_not_applicable` flag | `settings_not_applicable` when the package declares no saved variables |
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
| `Waiting` | newer head, blocked by WoW running or by another journal | "New settings from Den PC will be used when WoW closes" |
| `LocalDifferent` | live differs from head; this computer is not the authority | "Changed here, not shared" with "Share my settings" |
| `Paused` | §6 | "Settings paused on this computer" |
| `NeedsChoice` | §4.4 | "Choose which settings to keep" |
| `PolicyConflict` | §5.2 | "Settings master was changed on two computers" |
| `MasterAbsent` | §5.3 retired | "The settings master Den PC was retired…" |
| `Diagnosed(kind)` | §1.2 | "WoW Sync can't tell which addon owns these settings" |
| `Suspect` | §2.2 | "Settings may be incomplete after WoW stopped" |
| `NotShared` | group exception | "Not shared (your choice)" |

The group roll-up for settings never says "in sync" while any included unit
on any non-retired computer is `Waiting`, `Paused`, `LocalDifferent`,
`NeedsChoice`, `Diagnosed`, `Suspect` or unconfirmed (A-15, A-24).

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

### 10.1 `wow-sync-core` (pure; the `arch` test stays as is)

| Module | Additions |
|---|---|
| `toc` | `TocInfo.saved_variables: Vec<String>`, `saved_variables_per_character: Vec<String>`: names from the `## SavedVariables` and `## SavedVariablesPerCharacter` lines, validated as identifiers, metadata only |
| `ids` | `Identity { Account(AccountName), Character(AccountName, RealmName, CharName) }`, `SettingsUnit { package: PackageKey, identity: Identity }`, `RevisionId`, `PolicyId`, `CheckpointHash` |
| `paths` | `wtf_saved_variables_rel(identity, folder) -> Result<RelPath, PathError>`: builds only the two §1.1 shapes; `classify_wtf_rel(&RelPath) -> WtfPathKind` |
| **new `settings`** | `SettingsScope { items, diagnoses }`, `ScopeDiagnosis`, `settings_scope(&VariantRoot, &Inventory, &WtfListing, &IncludedAccounts) -> SettingsScope`; `Checkpoint`, `checkpoint_of(unit, &[FileFact]) -> Checkpoint`; `Capture`, `classify_capture(prev, settle_scans, activity) -> CaptureClass {Complete, Suspect, Unchanged}`; `PolicyView`, `policy_heads(&Ledger) -> PolicyView`, `authority_for(&PolicyView, &PackageKey) -> Authority`; `RevisionView`, `unit_heads(&Ledger, &SettingsUnit, &Validity) -> Heads`; `UnitTarget { Apply(Revision), Keep, LocalDifferent, NeedsChoice(Vec<Revision>), Blocked(Reason) }`, `compute_settings_target(...) -> Vec<(SettingsUnit, UnitTarget)>`; `publish_decision(capture, authority, paused) -> PublishAction`; `resume_preview(...) -> ResumeCase`; `KEEP_SETTINGS_STATES = 3` re-using `plan::KEEP_OWN_STATES` |
| `records` | via `record!`: `SettingsPolicyObject`, `SettingsRevisionObject`, `SettingsCheckpointManifest`, `SettingsState`, `SettingsLocal`; `ApplyJournal.settings_ops: Option<Vec<SettingsFileOp>>`; `RecoveryPoint` v2 fields `kind`, `settings`, `paired` (shared with the removal design) |
| `plan` | `plan_settings_apply(&UnitTarget, &LiveFacts, &ApplyGate) -> Result<SettingsPlan, PlanBlock>`; new `PlanBlock::SettingsPaused`, `SettingsDiagnosed` |
| `exec` | `SettingsEffects` port trait (`stage_blob`, `live_fact`, `rename_live_to_recovery`, `rename_staged_to_live`, `gate`), `run_settings_apply`; `recover_interrupted` also settles `settings_ops` with `classify_slots` |
| `status` | `SettingsStatus` per unit (§9.1), group roll-up in `StatusView`; `Headline` counts settings attention items |
| `explain` | `Explanation` variants for every row of §9.1 and every Activity entry of §9.2, via `render_named` |
| `view` | `NextAction::{ChooseSettings, ChooseMaster, ReviewResume, ConfirmSettingsHere, FixSettingsScope}` |
| `testkit` | synthetic WTF trees, identities, revision ledgers |

### 10.2 `wow-sync-adapters`

- `fs_scan`: `scan_wtf(&VariantRoot, &IncludedAccounts) -> WtfListing`:
  lists `WTF\Account\*\SavedVariables\*.lua` and
  `WTF\Account\*\*\*\SavedVariables\*.lua` only, with size and SHA-256; no
  other file content is read.
- `fs_safe`: a `WtfScope` root with the reparse policy; write operations
  restricted to the paths of an approved `SettingsPlan`.
- `fs_install`: settings staging, journal `settings_ops`, recovery slots,
  local blob/manifest store.
- `provider`: write and list settings objects and the settings stream
  (`write_atomic`, `put_object`, write-if-absent).
- `localstate`: `SettingsLocal`.
- `winproc`: unchanged; session end is a transition of the existing
  activity signal.

### 10.3 `wow-sync-runtime`

Cycle (removal design numbering, new steps marked): 1 recover (also
`settings_ops`), 2 admit (also settings stream), 2a learn authority (also
policy and revisions), 3 scan, **3s scan WTF and capture (§2)**, 3a observe
removals, 4 publish (code state, then settings objects and settings state),
5 target, **5s settings target (§4.2)**, 6 gate/plan, 7 apply code,
**7s apply settings (§4.3)**, 8 status. Settings steps never delay the code
steps for other addons. `activity.rs` emits the session-end event;
`control.rs` gains commands `SetPolicy`, `ShareSettings`, `PauseSettings`,
`ResumeSettings(ResumeChoice)`, `ChooseSettings`, `RestoreSettings`,
`ConfirmSettingsHere`; `detail.rs` gains the settings history view.

### 10.4 `wow-sync-app`

`pages.rs`/`model.rs`/`backend.rs`: the screens of §9.3, driven only by
core `StatusView`, `Explanation` and `NextAction` (no logic in the GUI).

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
| T-S11 | two computers resolve the same choice identically: one state; differently: new decision | A-14, A-20 |
| T-S12 | pause one addon, then pause all: no revision published, no apply, status "paused" in group view | A-15 |
| T-S13 | each resume row of §6 gives the stated explanation and default; resume never publishes | A-15 |
| T-S14 | more than three captures per unit: three kept; code history independent | A-16 |
| T-S15 | pinned states (choice, paired, removal, suspect, needs_help) survive pruning with an exception, then prune back to three | A-17 |
| T-S16 Neg | low storage: no pinned state deleted; needs-help message | A-17 |
| T-S17 Neg | account identity settings never applied to another account; character never to another character or realm; case-only differences diagnosed | A-23 |
| T-S18 Neg | `config-cache.wtf`, `bindings-cache.wtf`, `macros-cache.txt`, `layout-local.txt`, `AddOns.txt`, `Blizzard_*.lua` never read, moved or written | A-23 |
| T-S19 Neg | shared declared variable: both packages' units diagnosed, live files untouched, other addons sync | A-23 |
| T-S20 Neg | file with Lua that would write a file or run a command if executed: bytes copied, nothing executed (no process spawn, no file created) | A-23 |
| T-S21 Neg | junction/symlink at `WTF`, account, realm, character or `SavedVariables`: no write through it | A-23 |
| T-S22 Neg | malformed or oversized policy/revision/state objects: refused per object, others still used | A-20, A-23 |
| T-S23 | WoW starts between file renames: stop before next rename, recovery settles later, no process killed (real temp filesystem) | A-21, A-22 |
| T-S24 | crash/disk-full/permission failure at every settings op transition: next start reports the actual state; prior files recoverable | A-22 |
| T-S25 | master retired: no automatic publication for its units, master-absent status and reassign action; no policy object written automatically | A-45 |
| T-S26 Neg | stale publication from the retired master after retirement: not valid, does not reactivate it | A-45 |
| T-S27 | concurrent policy changes: only addons whose authority differs are held | A-20 |
| T-S28 | three replicas with reordered, duplicated and deleted deliveries and a manifest without its blobs: no false `InSync`, no apply of a partial checkpoint | A-20 |
| T-S29 | removal of an addon with settings: owned files moved to recovery; "undo removal" restores both to the same identity; diagnosed ownership gives code-only with the GUI text | A-09, A-19 |
| T-S30 | a reader without settings support skips settings objects and streams without refusing the group | A-20 |
| T-S31 | group status never "in sync" with any paused, waiting, local-different or unconfirmed unit; confirmation needs an `applied` report | A-24 |
| T-S32 (GUI) | setup consent, change master, exception, share, pause/resume, settings choice and restore all complete without a terminal, with keyboard and screen evidence | A-25, A-26 |

A-38 controls to include, at minimum: publish non-master captures
automatically (T-S06 must fail); pick the newer timestamp for a choice
(T-S10); apply a revision to a different account (T-S17); skip `G` before a
settings rename (T-S23); auto-elect a master on retirement (T-S25).
