# S-dropbox evidence: local Dropbox client behavior

Run 001, Phase 2, packet [S-dropbox](../packets/S-dropbox.md) (common rules:
[S-common](../packets/S-common.md)). Worktree `D:\GitHub\wow-sync.worktrees\S-dropbox`,
branch `run001/S-dropbox`. No sub-agents. Dropbox access was limited to
`D:\Dropbox\Gaming Sync\WoW Sync Test` (created by me; it did not exist) with
synthetic random bytes. No Dropbox API/credentials, no settings changes, no
WoW access. Environment for every result below: Windows 11 x64 dev PC (Insider
26300, not official support evidence), Dropbox desktop client **279.4.5723**
(`Dropbox.exe` file version, `C:\Program Files (x86)\Dropbox\Client`), one
computer only.

## Question and answer (SPEC §7, R-08)

Question: what does a sync client see through the Dropbox desktop client for
cloud-file states and filesystem differences?

- **Local Windows behavior of ordinary writes/renames/deletes: feasible (PASS)**,
  confidence medium-high for the tested patterns.
- **Online-only (dehydrated) states and hydration cost: unknown (BLOCKED)**.
  I could not make a file online-only without Dropbox's own UI, so hydration was
  never exercised. Confidence in any claim here is low.
- **"Fully uploaded" detection from local data: unknown, probably not possible
  from attributes alone.** The only local signal seen was a reparse-tag change
  1 to 3 s after write, long before a 200 MB upload plausibly finished.
- **Conflicted copies: BLOCKED** (needs a second client, A-34).

What would change it: a manual per-file "Make online-only" in Explorer (a UI
action by the owner, not scripted) would unblock hydration; a second machine
unblocks conflicts; shell sync-status properties / cfapi calls (not tried)
might expose upload state.

## Commands and results

Rerun: `powershell -NoProfile -File spikes\dropbox\observe.ps1` (about 5.5
minutes; it creates and leaves a synthetic `s-dropbox-<date>` folder; delete it
afterwards). Raw timestamped log of the final clean run:
[observations.txt](S-dropbox/observations.txt). An earlier run was invalid
(accidental concurrent script instances interleaving) and was discarded; its
files were removed.

### Item 1: client version and sync root: PASS

- Version above. `HKLM\...\Explorer\SyncRootManager` has exactly one entry named
  `Dropbox!<SID>!<account id>` (redacted), so the Dropbox folder **is a Windows
  Cloud Files (cfapi) sync root**. `HKCU` has none.
- The test folder and subfolders carry `REPARSE_POINT|PINNED` (0x80430) with reparse tag
  `0x9000601a` (a cloud-files tag). Files created locally start as plain
  `ARCHIVE|PINNED` (0x80020, no reparse tag) and are converted by Dropbox to
  placeholders.

### 2a Attributes and online-only toggle: PARTIAL (attributes PASS; toggle BLOCKED)

- PASS (2 reps): new file 0x80020 with no reparse tag; within 8 s
  0x80420 (`REPARSE|PINNED|ARCHIVE`) with tag `0x9000601a`. The 200 MB files
  went from no tag, to tag `0x9000201a` at ~1 s, to `0x9000601a` at ~3 s
  (2 reps). `OFFLINE`, `RECALL_ON_DATA_ACCESS` and `UNPINNED` were never seen.
- BLOCKED: `attrib +U -P`, `attrib +U`, `attrib -P` and the reverse return exit
  code 0 but change nothing (attribute stays 0x80420, `P` stays set) after 5 to
  10 s, in 2 reps plus a manual probe. Two failed approaches, so stopped. Dropbox
  apparently owns pin state (also the folder default is pinned/local). Therefore
  "does reading hydrate" and "do metadata reads hydrate" were **NOT tested**;
  reads of local (pinned) files took 1 to 8 ms for 4 MB and metadata reads
  (`Get-Item`, `Get-ChildItem`, `LastWriteTime`) changed no attribute. Those
  are local-file results only.

### 2b Upload visibility (200 MB, 2 reps): PASS for observed states; completion detection NOT possible

- The writer finished in 1.2 to 1.6 s. The final name and full size (209715200)
  were visible immediately, from the first stat: no temp name appeared in
  directory listings (polled each second; only the file's own name), and size
  never showed a partial value to a reader.
- Attributes and tag changed as above within ~3 s, then stayed constant for the
  remaining 90 s watched. Nothing locally distinguishes "uploaded" from
  "placeholder converted"; the tag stays `0x9000601a` throughout. Whether
  `0x9000601a` means in-sync is **unverified**. Shell sync-status properties
  and cfapi sync-state queries were not tried.
- Opening the file with share mode `None` succeeded in all reps, so a share-lock
  is not a usable "upload in progress" signal.

### 2c Replace/rename/delete (2 reps): PASS (Windows local)

- Temp-then-rename over an existing file and in-place overwrite both succeeded
  in 1 to 30 ms. Single-file rename and directory rename succeeded with no
  error, and name listings reflected them immediately.
- Case-only rename (`renamed.txt` to `RENAMED.TXT`) worked locally in both reps.
- Windows invalid characters `:` and `*` are rejected by .NET before reaching
  Dropbox. Trailing dot/space names were silently normalized by .NET Win32 path
  handling (`trail.` became `trail`), so that test does not prove what Dropbox
  does with them. `con.txt` was created OK. This is a Windows-side note only;
  macOS-side behavior is **BLOCKED** (no Mac).
- Delete: the first directory delete (about 5 s after writes) silently failed
  in both reps (the script uses `-ErrorAction SilentlyContinue`; the dir still
  existed). A retry a few minutes later succeeded, and removal of the work
  folder failed once ("in use by another process") then succeeded 5 s later.
  Dropbox transiently holds handles. Cause not captured; observed twice.

### 2d Conflicted copies: BLOCKED

Appending to a 100 MB file 2 s after writing succeeded in both reps (no sharing
violation) and no conflicted copy appeared (waited 40 s each; only `conf1.bin`
and `conf2.bin`). A true conflict needs two clients (A-34), so none was
possible. Public documentation: a conflicted copy gets the editor's username,
"conflicted copy" and the save date appended to the file name, and the last
version saved appears as the conflicted copy
(https://help.dropbox.com/organize/conflicted-copy, fetched 2026-11-15; the
exact name format is not stated on that page).

### 2e Case-only rename and invalid characters

See 2c for Windows. macOS: **BLOCKED**.

### Cleanup: done

All files, subfolders and the `s-dropbox-20261115` folder were deleted. The
`WoW Sync Test` folder was left in place and is empty. Dropbox syncs the deletes
to the cloud (not verified); no other path under `D:\Dropbox` was listed or
touched.

## Method limits and risks

- Single machine, single Dropbox account state, small samples (2 reps), one
  client version; Dropbox's pin default for this folder is local, so cloud-only
  behavior is untested.
- Reparse-tag meaning is inferred from tag values, not documented Dropbox
  behavior.
- The first (discarded) run showed that concurrent writers produce confusing
  logs; the script was fixed only for PowerShell parsing, not guarded.

## Implications for sync design

- **Publication is not proof of apply:** a file becoming visible locally with the
  right name and size says nothing about upload or about another computer. Apply
  must be confirmed by the peer (a state/ack record), never by local file state.
- **Partial files:** never judged by name/size/lock. A writer must include
  a manifest or checksum and a "complete" marker written last (via temp+rename);
  readers verify content against the manifest and ignore anything unverifiable.
- **Atomic replace:** temp+rename works locally and is the recommended pattern;
  in-place overwrite also worked but is less detectable for readers.
- **Hydration cost:** unknown; design must assume reads can block or fail on
  online-only files and must handle that without treating it as a content
  change. Needs an owner-run follow-up.
- Be tolerant of transient delete/rename failures (retry with backoff) because
  Dropbox briefly holds handles.
- Do not rely on conflicted-copy detection locally; treat unexpected
  `conflicted copy` names as untrusted data requiring a decision.

## Options for the owner

1. Owner marks one synthetic file online-only in Explorer; rerun a read test
   to unblock hydration (small manual action).
2. Add a spike for cfapi/shell sync-status (`PKEY_StorageProviderState`-style
   properties) to see whether upload state is exposed.
3. Run item 2d on a second computer when available.

## Spike code

`spikes/dropbox/observe.ps1` (PowerShell only; no Cargo workspace needed).
Raw log: `docs/runs/run-001/evidence/S-dropbox/observations.txt`.
