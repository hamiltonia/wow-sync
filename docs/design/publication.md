# Provider folder, records and publication

Part of the [slice design](README.md). The provider is a folder the user's
sync client (Dropbox in tests, D-08) replicates. Only the app's own files
inside `<shared>\WoWSync\` are written. Publication proves only that bytes
were written locally into that folder; it does **not** prove upload, download
or application elsewhere (R-12, A-20, A-24).

## 1. Layout

```text
<shared>\WoWSync\
  format.json                                   wowsync.root/1
  groups\<group_id>\
    group.json                                  wowsync.group/1        (creator writes once)
    members\<computer_id>\
      member.json                               wowsync.member/1       (that computer only)
      state\<enrollment_id>-<seq:010>.json      wowsync.code-state/1   (that computer only)
    trees\<hh>\<tree_sha256>.json               wowsync.folder-tree/1  (any member, write-if-absent)
    blobs\<hh>\<file_sha256>                    raw bytes              (any member, write-if-absent)
```

`<hh>` = first two hex chars of the hash. All names are lowercase ASCII.
Blobs and trees are **per group**, so data of one variant is never referenced
by another (R-02). Each member directory has exactly one writer (its own
computer), which avoids provider conflicts on mutable files; shared content
is immutable and content-addressed, so concurrent identical writes are
harmless. `group.json` is never rewritten in the slice.

Readers ignore `*.tmp` and `conflicted copy` entries (reported as
diagnostics) and any file whose name does not match the layout.

## 2. Record schemas

Every JSON record has `"schema": "<name>/<major>"` as its first field.
Decoding: missing `schema` → invalid; unknown name or different major →
refused (`UnsupportedMajor`; shown as "written by a newer/other version");
unknown extra fields within the same major are **ignored** (Q-13). Size
limits are checked before parsing. All integers are JSON numbers ≤ 2^53.
Timestamps are `*_unix_ms` from the writer's clock: **display only**, never
used for ordering or freshness decisions.

| Schema | Fields (all required unless `?`) | Limit |
|---|---|---|
| `wowsync.root/1` | `created_unix_ms` | 64 KiB |
| `wowsync.group/1` | `group_id`, `product`, `variant_label`, `display_name`, `created_by` (computer id), `created_unix_ms` | 64 KiB |
| `wowsync.member/1` | `group_id`, `computer_id`, `enrollment_id`, `display_name`, `product`, `variant_folder`, `enrolled_unix_ms`, `app_version`, `retired`? (reserved, must be absent or `false` in the slice) | 64 KiB |
| `wowsync.code-state/1` | see below | 16 MiB |
| `wowsync.folder-tree/1` | `tree` (hash), `files: [{path, size, sha256}]` sorted by path | 32 MiB |

`wowsync.code-state/1`:

```json
{
  "schema": "wowsync.code-state/1",
  "group_id": "…", "computer_id": "…", "enrollment_id": "…", "seq": 12,
  "product": "wowf", "app_version": "0.0.0", "written_unix_ms": 1767225600000,
  "client": { "build_version": "1.15.8.12345", "toc_mapping": "validated-plain/1" },
  "inventory_complete": true,
  "folders": [
    { "name": "DBM-Core", "key": "dbm-core", "status": "ok",
      "tree": "<sha256>", "files": 211, "bytes": 1048576,
      "toc": { "kind": "selected", "file": "DBM-Core_Forever.toc", "suffix": "Forever",
               "version": "12.1.23", "interface": [11507],
               "hard_deps": [], "ids": [{ "kind": "curse", "value": "3358" }] } }
  ],
  "apply_report": {
    "op": "…", "basis": [{ "computer_id": "…", "seq": 7 }],
    "results": [{ "package": "dbm-core", "result": "applied", "package_hash": "<sha256>" }]
  }
}
```

- `client.build_version` is the `Version` of the variant's active
  `.build.info` row (absent → `null`); `client.toc_mapping` names the `.toc`
  selection rule used: `profile:<product>/<n>` for a known profile, or
  `validated-plain/1` for a guided unfamiliar client ([domain.md](domain.md)
  §2–§3). Both are compatibility evidence for the target (M-03); they are
  not identity.
- `status` ∈ `ok | unreadable | unsafe | unsupported | unstable`; `tree` only
  for `ok`. `toc.kind` ∈ `selected | none | ambiguous | error`; `suffix` is
  `null` for a plain `.toc`.
- `seq` strictly increases per `(computer, enrollment)`. Which state is used
  is decided by core admission (§6), **not** "highest valid": an unusable
  newest file never falls back to an older one (M-05). A state file whose
  body disagrees with its file name, group, computer, enrollment or product
  is invalid.
- `inventory_complete` is `false` if any folder could not be read; the record
  is still published (other computers can use the readable folders) but the
  absent/unreadable folders never imply removal.
- `apply_report` is optional; present after an apply on that computer. Results
  ∈ `applied | skipped | replanned | interrupted | rolled_back | needs_help |
  failed`; only `applied` (= verified and finalized, [apply.md](apply.md)
  §2.2) counts toward confirmation.

Local-only records (under `<variant>\.wowsync` or local state, never uploaded):

| Schema | Fields | Limit |
|---|---|---|
| `wowsync.apply-journal/1` | `op`, `group_id`, `started_unix_ms`, `package`, `target_hash`, `phase` ∈ `staging \| committing \| committed \| verified \| needs_help \| rolled_back`, `from_phase?` (for `rolled_back`: `staging \| committing`), `reason?`, `folders: [{name, key, action, target_tree, expected_live}]`; `expected_live` = `{"kind":"absent"}` or `{"kind":"tree","tree","fingerprint"}`. One package per op; no per-folder progress field — recovery classifies observed slots ([apply.md](apply.md) §2–§4) | 1 MiB |
| `wowsync.recovery-point/1` | `op`, `package`, `created_unix_ms`, `replaced: [{name, tree}]`, `installed: [{name, tree}]`, `reason`, `pinned` (bool; true until the apply is verified, and while any journal or needs-help state references it) | 64 KiB |
| `wowsync.local-computer/1` | `computer_id`, `display_name`, `created_unix_ms` | 64 KiB |
| `wowsync.local-enrollment/1` | `group_id`, `enrollment_id`, `shared_root`, `install_root`, `variant_folder`, `product`, `validation` (the `VariantValidation` record of [domain.md](domain.md) §2, incl. `kind` known/guided and the evidence shown), `next_seq` | 64 KiB |
| `wowsync.admission/1` | `group_id`, `members: [{computer_id, enrollment_id, max_seq_seen, admitted_seq?}]` (watermarks, §6) | 1 MiB |
| `wowsync.audit/1` (one JSON object per line) | `at_unix_ms`, `event`, `package`?, `detail` | line ≤ 16 KiB |

## 3. Hashes

- File: SHA-256 of contents, lowercase hex.
- Folder tree: SHA-256 of `"wowsync-tree-v1\n"` followed, for every file
  sorted by `path` bytes, by `path + "\t" + size + "\t" + sha256 + "\n"`
  (`path` relative to the folder, `/`-separated, original case). Empty
  directories are not represented (limitation).
- Package: SHA-256 of `"wowsync-package-v1\n"` followed, for every member
  folder sorted by key, by `key + "\t" + tree + "\n"`.

Changing either format is a schema major change.

## 4. Publish (R-03, R-12, A-20)

`plan_publish` + `run_publish` order:

1. Refuse if the AddOns listing is unstable, the variant is not confirmed, or
   the member record is missing/retired. Folders that are `unstable` are
   published with that status and no tree (never as absent).
2. For every blob and tree the state will reference, obtain an
   `ObjectCheck` (core `exec::check_object`, built on
   `PublishEffects::stat_object`/`hash_object`): `VerifiedOk` only if it is
   `Local` and hashing its bytes **now** gives the expected hash; `Missing`;
   `Corrupt` (local, wrong hash/size); `Unverifiable` (cloud-only).
   Neither existence nor cached metadata is trusted (M-05, retry 2): every
   referenced existing object is re-hashed on every publish. Slice cost
   limitation: a full re-read of referenced provider objects per publish (Q-7).
3. Upload every blob not `VerifiedOk`: copy the **source file** to a temp
   file in the provider folder, hash the temp file, and only if it equals the
   expected hash rename it over the object path (`put_object`) (content-addressed, so
   replacing a corrupt or unverifiable object with verified bytes is always
   safe).
4. Same for tree records (encoded bytes are hashed in memory first).
5. **Source revalidation (B-02):** re-observe the metadata fingerprint of
   every published folder (`observe_source`); any change since the scan → abort, no state record,
   `Waiting(unstable)`, next cycle rescans. (Uploaded blobs stay; they are
   verified content.)
6. Write the state record `…/state/<enrollment>-<seq>.json` (temp + rename);
   increment and persist `next_seq` locally **before** writing (a crash may
   skip a number, never reuse one).
7. Remove own state files older than the newest three (own member dir only).

A state record is only written after every object it references was verified
in the local copy of the provider folder. Peers may still see it before the blobs
arrive (provider replication order is unknown); readers therefore treat
missing or cloud-only trees/blobs as `NotAvailableYet`, never as absence.
Readers recompute the hash of every tree record they use (must equal its
name and its `tree` field) and every staged blob; a mismatch is `Corrupt`
and makes only that package unavailable.
Publish runs when the inventory changed (different set of folder hashes) or
after an apply; otherwise nothing is written.

## 5. Size considerations

The dev PC's `_forever_` AddOns are 102 folders, ~3.3 GB (OWNER-DECISIONS,
D-14 context; S-addon). Consequences for the slice:

- Per-file content-addressed blobs: an unchanged file is uploaded once per
  group; an addon update uploads only changed files. No compression and no
  packing in the slice (Q-7).
- The state record carries per-folder summaries, not file lists; file lists
  live in tree records that are shared and write-if-absent. A state record
  for ~100 folders is a few tens of KiB.
- First publish of a large install is a long operation: it is resumable
  (objects already `VerifiedOk` are skipped) and shows progress; the state record
  is written only at the end.
- Old blobs/trees are never deleted in the slice (no GC). This is unbounded
  growth with addon churn and must be resolved before release (Q-7, R-14).
- Synthetic test data stays small (≤ 50 MB per provider run, Q-8); the
  real-size dataset is used only for measurements in a later approved run.

## 6. Read and confirmation

- Members = `members\*\member.json` valid for the group. Others are shown as
  invalid with the reason.
- **Admission (core `admit::admit_member_states`, M-05/M-07).** Runtime only
  supplies raw observations: for each member, the state-file names present
  (with cloud state and size) and, for the highest-numbered name of the
  current enrollment, its bytes if readable. Core decides:

  | Observation for member X (current enrollment) | Result |
  |---|---|
  | newest name `N` readable, decodes, validates for group/member/name | `Valid(N)`; watermark `max_seq_seen = max(old, N)`, `admitted_seq = N` |
  | newest `N` cloud-only, oversized, malformed, unknown major, body mismatch | `Unusable(N, reason)`; **no fallback** to `N-1` or older; X contributes no candidates (shown "waiting for usable data from X"). The target is computed from the other members, which can delay X's newer versions but never selects X's superseded data |
  | highest name present `< max_seq_seen` (files disappeared or replication regressed) | `Regressed(max_seq_seen)`; no fallback; shown as "data from X went backwards" |
  | no state files and `max_seq_seen` unset | `NoState` |
  | member's enrollment id changed | watermark for the old enrollment kept for audit; new enrollment starts unset |

  Watermarks are persisted in local state (`wowsync.admission/1`) by the
  runtime as returned by core. An `Unusable` or `Regressed` member is
  **not** absence: nothing it previously offered is treated as removed, and
  nothing it offered earlier is used either.
- **Confirmed** for package P on peer X means: X's latest state contains an
  `apply_report` whose `results` include P with `applied` and the target
  `package_hash`, **and** X's inventory in that same record shows the target
  trees for P's folders. Seeing our own state published, or a peer's member
  file, is not confirmation (A-24, A-25).
