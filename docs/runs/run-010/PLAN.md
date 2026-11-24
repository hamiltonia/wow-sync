# Run 010 plan

Approval: D-32 (2026-11-24). There is no AIU ceiling; usage is tracked in
[USAGE.md](USAGE.md). Target: the `v0.3.3` Windows pre-release.

## Problem

`v0.3.2` is live on RYZENFISH and LENOVOFISH (dev PC), one Dropbox group,
`_forever_`.

- **Invisible work.** After the owner picked RYZENFISH's Hide Anything copy,
  LENOVOFISH staged and installed about 1.8 GB / 21k files of Spoken add-ons
  plus Hide Anything for about 7.5 min (15:28–15:35 local). Meanwhile:
  - the tray and headline still said "Waiting for your decision on 1 add-on";
  - the log had no line between `23:28:23Z act Nav` and the pass summary at
    `23:35:09Z`;
  - nothing said what was arriving, from which computer, or how far along it
    was. The owner thought it was stuck.
- **Thin history.** `audit.jsonl` / the Activity page hold only lines such as
  `applied hideanything applied` and `published seq 9 (0 new objects)`
  (`wow-sync-runtime` `log_activity`, `ActivityEntry { unix_ms, kind,
  package, text }`). There is no source computer, old or new version, file or
  size change, or reason.
- **Slow idle passes.** The pass right after the apply reported `108365 file(s)
  reused from the hash cache, 43 read` and took 19.4 s. RYZENFISH logs show
  21–110 s with 0 files read. About 36k files are installed, so each file
  seems to be scanned about three times per pass. The apply pass itself
  (453 s) then read 28,958 files, apparently re-hashing what it had just
  installed.
- **Memory during apply.** Private bytes were 136 MiB during the install.
  Private working set was 55.8 MiB once idle. The 100 MiB limit is on private
  resident memory, but it has not been measured during a large apply.

## Goals

1. **Change history (D-32 item 1).**
   - Each applied, removed, rolled-back, kept or published add-on change, and
     each saved-settings change, records:
     - the add-on (display name and folders);
     - the source computer (friendly name), or "this computer";
     - the old and new version: the TOC `## Version` when present, else a short
       content id; "new" or "removed" where it applies;
     - the files added, changed and removed, and the size change;
     - the reason: newer release, your choice (or a choice made on <computer>),
       the settings master, a peer push, removal, rollback, or recovery
       restore.
   - The history is persisted, bounded, survives a restart and reads older
     `audit.jsonl` lines.
   - The Activity page groups entries by pass ("3:35 PM — 15 add-ons updated
     from RYZENFISH, +1.8 GB"), and each group expands to per-add-on rows. An
     add-on's own page shows its recent history.
   - An optional per-add-on detail lists the changed file paths, relative to
     the add-on folder, capped with "and N more".
2. **Live progress (D-32 item 2).**
   - During a pass the headline and tray show the phase and progress, for
     example:
     - "Checking files: 12,000 of 36,000";
     - "Installing 15 add-ons from RYZENFISH: 1.2 of 1.8 GB
       (SpokenQuestsAudioShared)";
     - "Publishing changes: 3 of 7".
   - Before a large apply, show an incoming summary: add-ons, source and size.
   - When a decision has been satisfied (a newer record covers it, or it is
     being applied), its "Waiting for your decision" text is replaced at once
     by the progress text.
   - Progress updates no more than about 2 per second and do not slow the pass.
3. **Logs (D-32 item 3).** Log a line at pass start (trigger), each phase
   start, each package applied or removed (name, source computer, version
   change, file counts, bytes, ms) and the pass summary. No profile paths,
   account or character names, or file contents.
4. **Idle pass speed (D-32 item 4).**
   - Profile the 19 s all-cache-hit pass and find why about 108k files are
     visited for about 36k installed.
   - Scan each install root at most once per pass, reusing the result across
     phases.
   - After installing files, seed the cache from the hashes already known
     from the staged blobs, so the next pass does not re-read them.
   - Gate: an unchanged pass on the real `_forever_` (read-only, isolated
     state) is at least 3× faster than `v0.3.2` measured on the same machine.
   - Change detection is unchanged: a changed size or mtime still forces a
     re-hash, and mtime is never ordering evidence.
5. **Memory (D-32 item 5).** Measure peak private resident memory
   (private working set) while applying about 2 GB of synthetic add-ons. It
   must be ≤ 100 MiB. Fix it if not.
6. **Release `v0.3.3`** with release notes.

## Packets and order

| ID | Role / model | Owns | Depends on |
|---|---|---|---|
| R10-history | implementation / claude-sonnet-5.5 | goal 1 data and goal 2 events: `wow-sync-runtime` (activity, apply/report paths, progress events), `wow-sync-core` where needed | — |
| R10-speed | implementation / claude-sonnet-5.5 | goals 4–5: `wow-sync-adapters` (`fs_scan`, `hash_cache`), scan call sites in runtime | — |
| R10-app | implementation / claude-sonnet-5.5 | goals 1–3 UI and logs: `wow-sync-app`, version 0.3.3, release notes, user docs | R10-history interface commit |
| R10-review | review / claude-opus-5.5 | all three merges | R10-* merged |
| R10-check | verification / gpt-6.1-sol | the `v0.3.3` candidate | merged, dispatch artifact |

- **Interface first.** R10-history first commits only the public types and
  signatures: the change record, the progress event and the subscription.
  It reports the SHA. The orchestrator merges that commit to `main`, and
  R10-app rebases onto it. R10-history then fills in the implementation.
- **`service.rs` sharing.** R10-history owns it. R10-speed edits only the
  scan call sites there, keeps those edits small, and reports them. The
  orchestrator merges R10-speed first; R10-history rebases.
- The orchestrator gates, merges with `--no-ff`, runs both verify tiers,
  pushes and watches CI.

## Gates for `v0.3.3`

- Both verify tiers pass; CI is green, including macOS `verify-offline`.
- The release dispatch build succeeds.
- R10-check reports PASS for:
  - change history on two synthetic computers: an update, a new add-on, a
    removal, a choice, a rollback and a settings change, each with its source,
    versions, counts, size and reason, surviving a restart;
  - live progress during a ≥1 GB synthetic apply: the headline and tray
    change within about 2 s, no stale decision text, and the incoming summary
    is shown;
  - log lines, with privacy;
  - the idle pass ≥3× faster than `v0.3.2` on the real `_forever_`
    (read-only, isolated state), no re-read after an install, and a changed
    file is still detected;
  - memory ≤ 100 MiB private working set during the large apply;
  - the artifact smoke test.
- No open critical or major review findings.

## Stop conditions

- A fix needs a product-intent change beyond D-32 → ask the owner.
- Any write to the real Dropbox group, or to the owner's running instances
  → forbidden; ask the owner.
- Any irreversible action on user data → ask the owner.
