# Work packet S-dropbox: local Dropbox client behavior

Role: implementation. Model: claude-sonnet-5.5 (D-04); checked later by
gpt-6.1-sol (S-check). Common rules: [S-common.md](S-common.md). ID
`S-dropbox`; spike dir `spikes/dropbox/`.

## Intent

SPEC §7 "Real Dropbox cloud-file states and filesystem differences"; R-08
provider behavior. Observe what a sync client sees on Windows through the
existing Dropbox desktop client (D-08). **Scope: only
`D:\Dropbox\Gaming Sync\WoW Sync Test`** (create if absent), synthetic data
only. Do not list, read or touch anything else under `D:\Dropbox`. No Dropbox
API or credentials. Do not change Dropbox client settings.

## Work

1. Record the Dropbox client version (from its exe file version resource or
   install dir metadata) and whether the folder is a Windows Cloud Files
   (cfapi) sync root.
2. Observations, each repeated ≥ 2 times with timestamps, in a work subfolder
   `WoW Sync Test\s-dropbox-<date>\`:
   a. File attributes/placeholder states for local vs online-only files
      (`FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`, `OFFLINE`, `PINNED`/`UNPINNED`,
      reparse tags). Try `attrib +U -P` / `attrib +P -U` on synthetic files to
      toggle online-only; record whether reading an online-only file hydrates
      it and how long it takes, and whether metadata reads (size/mtime/
      enumeration) trigger hydration.
   b. Upload visibility: write a large synthetic file (e.g. 200 MB random) and
      observe intermediate states (temp names, sizes, attributes, sync status
      if exposed via shell properties/cfapi), and how a reader could tell
      "fully uploaded" — or that it cannot be told locally.
   c. Atomic replace patterns: write-temp-then-rename vs in-place overwrite;
      rename and directory rename visibility; delete behavior (local).
   d. Conflicted copies: a true conflict requires a second client; try only
      locally safe methods (e.g. edit while the file is mid-upload). If no
      conflicted copy is produced, BLOCKED with the reason (needs second
      machine, A-34). Record Dropbox's naming pattern from public docs with
      citation.
   e. Case-only renames and characters invalid on macOS/Windows: note only
      the Windows side; macOS BLOCKED.
3. Clean up: delete your synthetic files at the end (leave the
   `WoW Sync Test` folder itself). Record the cleanup.
4. Implications for the sync design: "publication is not proof of apply",
   partial-file detection, hydration cost, and recommended safe patterns.

## Owned writes

`spikes/dropbox/` (scripts/tools), evidence files (S-common), and the
`WoW Sync Test` subfolder named above.
