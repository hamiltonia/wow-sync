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
  "inventory_complete": true,
  "folders": [
    { "name": "DBM-Core", "key": "dbm-core", "status": "ok",
      "tree": "<sha256>", "files": 211, "bytes": 1048576,
      "toc": { "kind": "selected", "file": "DBM-Core_Vanilla.toc", "version": "12.1.23",
               "hard_deps": [], "ids": [{ "kind": "curse", "value": "3358" }] } }
  ],
  "apply_report": {
    "op": "…", "basis": [{ "computer_id": "…", "seq": 7 }],
    "results": [{ "package": "dbm-core", "result": "applied", "package_hash": "<sha256>" }]
  }
}
```

- `status` ∈ `ok | unreadable | unsafe | unsupported | unstable`; `tree` only
  for `ok`. `toc.kind` ∈ `selected | none | ambiguous | error`.
- `seq` strictly increases per `(computer, enrollment)`; the reader takes the
  highest valid `seq` file for the member's **current** enrollment id (from
  `member.json`). A state file whose body disagrees with its file name, group,
  computer, enrollment or product is invalid.
- `inventory_complete` is `false` if any folder could not be read; the record
  is still published (other computers can use the readable folders) but the
  absent/unreadable folders never imply removal.
- `apply_report` is optional; present after an apply on that computer. Results
  ∈ `applied | skipped | rolled_back | needs_help | failed`.

Local-only records (under `<variant>\.wowsync` or local state, never uploaded):

| Schema | Fields | Limit |
|---|---|---|
| `wowsync.apply-journal/1` | `op`, `group_id`, `started_unix_ms`, `packages: [{package, phase, folders: [{name, action, target_tree, had_live}]}]`; `phase` ∈ `staging \| committing \| committed \| finalized \| rolled_back` | 1 MiB |
| `wowsync.recovery-point/1` | `op`, `package`, `created_unix_ms`, `replaced: [{name, tree?}]`, `installed: [{name, tree}]`, `reason` | 64 KiB |
| `wowsync.local-computer/1` | `computer_id`, `display_name`, `created_unix_ms` | 64 KiB |
| `wowsync.local-enrollment/1` | `group_id`, `enrollment_id`, `shared_root`, `install_root`, `variant_folder`, `product`, `confirmed_unfamiliar`, `next_seq` | 64 KiB |
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

1. Refuse if the inventory is unstable, the variant is not confirmed, or the
   member record is missing/retired.
2. Upload every missing blob referenced by `ok` folders (write-if-absent with
   hash verification of the temp file before rename).
3. Write every missing tree record.
4. Write the state record `…/state/<enrollment>-<seq>.json` (temp + rename);
   increment and persist `next_seq` locally **before** writing (a crash may
   skip a number, never reuse one).
5. Remove own state files older than the newest three (own member dir only).

A state record is only written after everything it references exists in the
local copy of the provider folder. Peers may still see it before the blobs
arrive (provider replication order is unknown); readers therefore treat
missing or cloud-only trees/blobs as `NotAvailableYet`, never as absence.
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
  (blobs already present are skipped) and shows progress; the state record
  is written only at the end.
- Old blobs/trees are never deleted in the slice (no GC). This is unbounded
  growth with addon churn and must be resolved before release (Q-7, R-14).
- Synthetic test data stays small (≤ 50 MB per provider run, Q-8); the
  real-size dataset is used only for measurements in a later approved run.

## 6. Read and confirmation

- Members = `members\*\member.json` valid for the group. Others are shown as
  invalid with the reason.
- Latest state per member: highest-`seq` valid file of the current
  enrollment; a cloud-only or oversized file makes that member `Unavailable`
  (shown "waiting for files to arrive").
- **Confirmed** for package P on peer X means: X's latest state contains an
  `apply_report` whose `results` include P with `applied` and the target
  `package_hash`, **and** X's inventory in that same record shows the target
  trees for P's folders. Seeing our own state published, or a peer's member
  file, is not confirmation (A-24, A-25).
