# R10-review evidence — independent review of run-010 merges

- Worker: R10-review (review role). Worktree `D:\GitHub\wow-sync.worktrees\R10-review`,
  branch `run010/R10-review`, based on main `5697f85`.
- Scope: `git diff e87a023 5697f85` — R10-history (33261fe, a6d898a, 1dfe772), R10-speed
  (407cd06), R10-app (5697f85), and `docs/release/v0.3.3.md`.
- Method: reading the diff, plus temporary probe tests (listed under each finding). The probes
  were run with `cargo test` in this worktree on synthetic lab data only and were **reverted**
  (`git checkout -- crates`). No product code is changed by this commit. The owner's instance,
  Dropbox folder, WoW install and demo folder were not touched.

## Verdict

**Open major findings: RV10-01, RV10-02, RV10-03, RV10-04.** No critical findings. Sync safety
(scan reuse, tree cache, hash-cache seeding) looks correct. The defects are in the new
progress and history display, which is the main user-visible feature of v0.3.3, and some of them
contradict the release notes.

## Findings

### RV10-01 — major — the next package's staging event is almost always dropped, so the headline names the add-on that already finished

- `crates/wow-sync-runtime/src/tracking.rs:240-250`: `ApplyProgress::staging` uses
  `hub.emit_throttled`, which drops events within 500 ms of the last emit
  (`history.rs:400`).
- `tracking.rs:254-274`: `complete` uses the unthrottled `emit` and updates the last-emit time.
  It is called from `effects.rs:204` (`record_finalized`) straight after a package finalizes. The
  next package's `stage_folder` then calls `staging` within milliseconds, inside the throttle
  window, so that event is dropped.
- Effect: during a long install the headline and tray keep showing the **previous** add-on and
  its byte count until the next completion. For a single 1 GB folder (SpokenQuestsAudioShared,
  the case the release notes cite) that is minutes. R10-app's UIA capture "120.0 of 240.0 MB
  (MapPack)" is consistent with this: it was a completed event, not a staging one.
- Also, `bytes_done` grows by the whole folder size when staging *starts*, not as bytes are
  copied. Progress jumps ahead and then stalls for the whole copy.
- Probe (in `tracking.rs` tests): `announce` → `staging(a)` → `complete(a)` →
  `staging(b, 1 GB)`, all in under 500 ms. Only two events arrive: the announce event
  (`current = None`) and a's completed event. **Both staging events were dropped.**
- Fix direction: emit package/folder-start events unthrottled (throttle only the in-package
  updates), and report bytes as they are copied.

### RV10-02 — major — a still-pending decision is hidden during every pass

- `crates/wow-sync-app/src/model.rs:172-176`: `headline()` checks for progress text before the
  core `Headline::Waiting` match (`model.rs:195`). It does not check whether a pending decision
  is satisfied (`decision_satisfied`, `model.rs:281`, is used only in the summary count at
  `:303`).
- `PassGuard::start` (`tracking.rs:47`) emits Scanning on every pass, and the pump shows the
  first event at once. Passes run every 5 s by default.
- Effect: while a decision is unanswered, the headline "Waiting for your decision on 1 add-on",
  the tray Hourglass icon and the tooltip switch to "Checking files" / Arrows on every pass,
  and then back. In v0.3.2 that did not happen. PLAN.md and the release notes say only a
  decision whose add-on *starts applying* is replaced.
- The same precedence hides the OK/Check state on every idle pass, so the tray flickers every
  5 s.
- Related: `progress_ui.rs:231` sends `Note(None)` when the next pass becomes active, so the
  "Updated N add-ons — see Activity" note (`NOTE_MS` = 20 s, `progress_ui.rs:14`) lasts about
  5 s. The release notes say "briefly"; the code intends 20 s.
- Probe (in `model.rs` tests): a World with a pending NeedsDecision, Waiting headline, and
  progress text "Checking files". **The headline was "Checking files"**, tray shape Arrows,
  tooltip "WoW Sync: Checking files". Expected "Waiting for your decision on 1 add-on" /
  Hourglass.
- Fix direction: show routine Scanning/Publishing progress only when there is no unsatisfied
  decision or error headline. Suppress the progress headline for quiet passes; one option is to
  show it only after a delay or during Installing/Removing.

### RV10-03 — major — after applies longer than 60 s, every package is listed twice in Activity, and record times are the apply start time

- `crates/wow-sync-runtime/src/history.rs:259-263`: legacy `applied` lines are hidden only if a
  structured record is within 60 s.
- Records get `unix_ms` from `cx.now_ms` (`service.rs:3407`), which is set when the apply is
  *prepared*. `complete()` updates only the duration.
- The legacy line is written by `log_cycle` (`service.rs:4033`) at the end of the pass.
- Effect: in applies over 60 s (the owner's real 453 s apply is exactly the case run-010 exists
  for), each add-on gets a structured record **and** a plain "applied <key> applied" line. The
  group's timestamp shows the start of the apply, not when the change landed.
- Probe (in `history.rs` tests): a record stamped 120 s before a legacy "applied" line.
  `history()` returned **both** entries: `["legacy applied", "change SpokenAudio"]`.
- Fix direction: stamp records at finalize time. Stop writing the legacy per-package line when a
  structured record is written; do not rely on a time window.

### RV10-04 — major — `published()` file counts are wrong: unchanged files count as "changed"

- `crates/wow-sync-runtime/src/changes.rs:491-520`: for a published update,
  `files.changed = min(new_count, old_count)` and added/removed are the net difference in file
  counts. No per-file comparison is made.
- Effect: editing 1 file in a 100-file add-on is recorded as "~100". Adding one file and removing
  another is "~N, +0, −0". The release notes promise "files added (+), changed (~) and removed
  (−)".
- The apply side (`prepare`, `changes.rs:221`) uses a real per-file diff. This is the
  publishing computer's view only. That computer is the one whose user made the change, so it
  is the record most likely to be read.
- Fix direction: diff the published tree against the previous tree, as `prepare` does.

### RV10-05 — minor — a local uninstall leaves no Activity entry on the computer that made it

- `changes.rs:499`: `published()` iterates only `now.folders`, so a folder removed locally
  produces no Published/Removed record.
- The plain "published seq N (0 new objects)" line that a removal produces is now hidden
  (`history.rs` covered() and `world.rs` `plain_text`; test at `e2e.rs`
  `a_removal_streams_removing_and_a_publish_of_nothing_new_is_not_listed`).
- Effect: on the computer where the user uninstalled an add-on, Activity shows nothing for that
  change. Peers do show "removed on X".

### RV10-06 — minor — the "Checking files: X of Y" counter is cumulative across passes

- `crates/wow-sync-adapters/src/fs_scan.rs:341,366` increment `ScanCtl.files`. Nothing resets
  it: the service keeps one `read_ctl` (`service.rs:3040`), and the app sets it once
  (`backend.rs:798`).
- `service.rs:3047-3052` stores the cumulative value as `scan_total_hint`.
  `tracking.rs:83-87` reports `items_done = cumulative` and `items_total = max(hint, done)`.
- Effect: from the second pass on, the headline reads "Checking files: N of N", and N grows by
  the add-on file count every pass (and more, because re-observe and last-look scans also
  count). The total is never meaningful. Example from the release notes: "Checking files: 12,000
  of 36,000".
- Probe (in `e2e.rs` `change_history` tests): two add-ons, a shared `ScanCtl`, four
  `sync_once` calls. **Counter after each pass: `[4, 8, 12, 16]`.**
- Fix direction: reset the counter (or record a baseline) at pass start, and take the hint from
  the first scan only.

### RV10-07 — minor — idle passes write about five log lines each

- Every pass now logs "sync pass starting", phases checking, deciding and publishing, and a pass
  summary. A Publishing phase is emitted even when nothing is published (`tracking.rs:62`).
- At the 5 s interval with 1 MiB rotation, this cuts the log history to a few hours. That makes
  it harder to diagnose the kind of multi-hour sync issue run-010 was planned for.
- Suggest logging an idle pass as one line, or not at all when nothing changed.

### RV10-08 — minor — an interrupted apply finished by crash recovery writes no change record

- `recover_interrupted` completes the journal without a reporting context, so the add-on changes
  but Activity shows no structured entry for it (only the legacy recovery line).

### RV10-09 — note — mixed sources and source switching in the install headline

- With items from several computers, `announce` uses "from several computers". This includes a
  peer mixed with ThisComputer (for example, a restore plus a peer update).
- Later per-item events carry each item's own source, so the header "Installing 15 add-ons from
  A" can switch source text mid-install.
- `newest()` picks records by the issuer's wall-clock `written_unix_ms`. Clock skew only affects
  display.

### RV10-10 — note — the hash-cache seeding comment is outdated, and there is a small mtime-trust risk

- `fs_install.rs:349/362`: `note_installed` seeds cache entries after the Staging→Live rename,
  only for files whose size and mtime match a fresh snapshot.
- Staging content is checked by the write hash plus an on-disk re-read, so a partial or failed
  install cannot be marked good. A failed stage never renames. Rollback-restored files have
  different mtimes, so they miss the cache.
- Seeding bypasses the cache's racy-mtime rule (only store settled mtimes). That matters on
  filesystems with coarse mtimes (FAT/exFAT 2 s): a later same-size rewrite within the same tick
  would hit the cache with the old hash. WoW installs are normally NTFS, so this is a note.
- `hash_slot` (`fs_install.rs:277`) also lacks the `mtime != 0` guard that `fs_scan.rs` has.
- The comment at `service.rs:3518` ("files just written are too recent for the cache to trust and
  are re-read") is now false and should be updated.

### RV10-11 — note — release notes

- "Installing … 1.2 of 1.8 GB (SpokenQuestsAudioShared)": not what users will see while
  RV10-01 is open.
- "Checking files: 12,000 of 36,000": not what users will see while RV10-06 is open.
- "A 'Waiting for your decision' headline is replaced as soon as that add-on starts applying":
  true, but it is also replaced on every routine pass (RV10-02).
- The notes do not mention the idle-pass speedup measured by R10-speed (3.25x on a copy of the
  real group) or the memory result. Optional.

## Areas checked and found correct

- **Scan reuse (`observe_world_recent`, `service.rs:2317`; quiet cycle).** A settled scan under
  10 s old is reused only when there is no `applied_plan` and `rec.ops` is empty.
  - Apply, removal, rollback, restore and settings writes all go through `planned`, which sets
    `applied_plan`, so after any game-file write the re-observe is a real scan.
  - `removal_steps` and the settings publish touch only the ledger.
  - Reuse needs a stable scan with no Unstable folder. The provider is still re-read.
  - Residual risk: a user edit made between the pass's first scan and its publish is published
    one pass later (5 s). It is not lost. Acceptable.
- **`tree_cache`** (10 s, Ok-only, content-addressed). It cannot return wrong content. A tree
  that vanished at the provider fails later at the blob read, safely.
- **fs_scan quiet single snapshot (250 ms).** Any changed mtime misses the cache, which sets
  `read_any` and forces the second snapshot, so the stability check still holds.
- **Progress plumbing.**
  - The hub uses an unbounded channel and never blocks the pass.
  - `PassGuard` emits Idle in `Drop` (`tracking.rs:57`), so every exit sends Idle, including
    errors and panics. The backend also adds Idle in `end_pass` if it is missing.
  - The pump thread exits on disconnect.
  - No stuck headline was found on error paths.
- **Attribution.**
  - Applied/InstalledNew: source = the publishing computer, reason = chosen copy or newer
    release.
  - Rollback: the Rejection author. Removal: the Removal author with `VersionLabel::Removed`.
  - Settings: the revision issuer, reason PeerPush or SettingsMaster.
  - Restore: ThisComputer / Recovery.
  - e2e tests cover each of these.
- **Privacy.** `changes.jsonl` holds add-on names, clipped versions, friendly computer names and
  paths relative to the add-on folder. Settings records use the add-on display name with no paths
  or account/character names (e2e test asserts no "ACC"). Log lines from `record_log` contain no
  paths. Tray text is built from the same fields. No secrets or file contents were found.

## Probe tests (all reverted)

| Probe | File | Result |
|---|---|---|
| `rv_probe_next_package_staging_is_dropped_after_completion` | `crates/wow-sync-runtime/src/tracking.rs` (tests) | passed = bug confirmed (only announce + a's completed event delivered) |
| `rv_probe_unsatisfied_decision_hidden_during_idle_pass` | `crates/wow-sync-app/src/model.rs` (tests) | failed = bug confirmed (headline "Checking files", tray Arrows) |
| `rv_probe_long_apply_duplicates_legacy_line` | `crates/wow-sync-runtime/src/history.rs` (tests) | failed = bug confirmed (legacy + structured both listed) |
| `rv_probe_scan_counter_is_cumulative_across_passes` | `crates/wow-sync-runtime/src/e2e.rs` (`change_history`) | failed = bug confirmed (`[4, 8, 12, 16]`) |

Commands: `cargo test -p wow-sync-runtime --lib rv_probe -- --nocapture` and
`cargo test -p wow-sync-app --lib rv_probe`, run in this worktree. The full suites were not run
by this reviewer; the implementers' evidence files record those runs.
