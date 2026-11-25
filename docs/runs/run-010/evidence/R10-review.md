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

## Recheck (main 63ec305)

Branch `run010/R10-review` fast-forwarded to main `63ec305`, which includes R10-history round 4
(068b6cb) and R10-app rounds 4–5 (07c1b16, 7c87ff0). Each fix was checked by reading
`git diff 5697f85 63ec305` and the new tests. Further temporary probes (listed below) were run
and then reverted. Test runs in this worktree:
- `cargo test -p wow-sync-runtime -p wow-sync-app --lib`: runtime 246 passed, 0 failed.
- `cargo test -p wow-sync-app`: 181 passed, 0 failed.

### Status per finding

| ID | Status | Evidence |
|---|---|---|
| RV10-01 | **partially fixed** | `tracking.rs:246-266`: a package's first `staging` event is now emitted unthrottled, so the next add-on is named at once. Bytes are now counted per copied file (`effects.rs` `copied`), and `complete` adds any remainder. Test `the_next_package_is_named_at_once_and_bytes_follow_the_copied_files`. Remaining: new finding RV10-12 (bytes go backwards in multi-folder packages). |
| RV10-02 | **fixed** | `model.rs:228` `progress_headline`: Scanning, Deciding and Publishing never replace Waiting while an unsatisfied decision is pending; they show as the sub-line. Routine checks under `QUIET_MS` (1.5 s) are not shown, so idle passes no longer flicker the tray. The note is cleared only by real work or a non-scheduled pass. Tests `a_routine_check_never_replaces_a_waiting_decision`, `a_quick_idle_pass_is_silent_and_never_flashes_the_headline`, `the_note_survives_idle_passes_for_its_full_lifetime`. Residual design note RV10-14. |
| RV10-03 | **fixed** (applied lines) | `complete` now stamps `unix_ms` when the package lands. In `history.rs` `covered()`, an `applied` line is hidden by a record of the same package made after that package's previous `applied` line and no later than 60 s after this line, whatever the apply length. Test `a_long_apply_does_not_list_each_package_twice` uses 450 s and 300 s applies. Residual: RV10-15. |
| RV10-04 | **fixed** | `changes.rs` `file_diff` compares the published file list with the previous tree read from the provider. If that tree cannot be read, it falls back to net counts (documented in a comment). e2e `a_published_update_counts_only_the_files_that_differ` gives (+1, ~1, −1) with exact paths. |
| RV10-05 | **fixed** | `published()` records a ThisComputer/Removal record for a previously published eligible folder that is absent from the current inventory. Peer removals applied in the same pass are skipped (`applied_keys` now includes `q.removes`). e2e tests: `an_uninstall_leaves_a_removed_record_on_the_computer_that_made_it` (recorded once over two passes) and `a_removal_applied_from_a_peer_is_not_also_recorded_as_a_local_removal`. Edge case: RV10-13. |
| RV10-06 | **fixed** | `service.rs` resets `read_ctl.files` to 0 at pass start, and the hint is taken after the first look. e2e `the_checking_files_counter_starts_again_each_pass`. |
| RV10-07 | **fixed** | `progress_ui.rs`: a scheduled pass logs only once it does real work (install, removal, a record, or an incoming summary). Otherwise it logs nothing, or one "slow check" line if it took 2 s or more. Manual and after-change passes are logged in full. `app.rs` logs the tray tooltip only when it changes. |
| RV10-08 | **fixed** | `service.rs` writes `changes::recovered` (ThisComputer/Recovery, old version `ContentId("")`, shown as "unknown" by `history_ui.rs`) for each op that crash recovery finished as Applied. e2e `a_crash_recovery_that_finishes_an_install_leaves_a_record`. Side effect: RV10-13. |
| RV10-09 | note, now documented | Known limits in the release notes cover the mixed-source wording and the writer clock. |
| RV10-10 | not changed (note) | The comment at `service.rs` ("files just written are too recent for the cache to trust and are re-read") is still outdated. Mtime-trust note unchanged. |
| RV10-11 | **fixed** | The release notes no longer quote "Checking files: X of Y", and now describe the decision, note, quiet-check and log behaviour. See the release-notes check below. |

### New findings

#### RV10-12 — minor — progress bytes go backwards at each folder of a multi-folder package

- `tracking.rs:254`: `staging` treats every call after the first as a retry. It runs
  `std::mem::take(&mut item.counted)` and subtracts that from `bytes_done`.
- `exec.rs:538-539` calls `stage_folder`, and so `staging`, once **per folder** of a package.
- Effect: for a multi-folder add-on (for example DBM-* or ElvUI with its options folders), the
  headline's "X of Y GB" drops back by the bytes already copied each time a new folder starts.
  It under-reports until `complete` adds the rest. The final total is correct. Display only.
- Probe `rv_probe_second_folder_of_a_package_resets_bytes`: package bytes 100; `staging`,
  `copied(60)`, `staging` (second folder). **`bytes_done` sequence `[0, 60, 0]`.**
- Fix direction: reset only on a real retry (track bytes per folder key), or do not subtract.

#### RV10-13 — minor — an install finished by crash recovery is also recorded as "published from this computer"

- The `recovered` keys are not in the publish skip set. `applied_keys` (`service.rs`, the publish
  block) is built only from `applied_plan`. So the publish later in the same pass records the
  recovered peer install as `Published` / ThisComputer / NewerRelease, in addition to the new
  `Applied` / Recovery record.
- A recovered peer removal would likewise get a false local `Removed` record. Its Recovery
  record also shows `new_version = New`, because the key is gone from the inventory.
- The same misattribution applies whenever the publish of an applied change is deferred to a
  later pass (for example after a provider write error), because the skip set covers only the
  current pass. That last part is by reasoning only.
- Probe `rv_probe_recovered_install_is_also_recorded_as_a_local_publish`, a copy of the authors'
  crash loop. At crash points 6–9, b's records were
  **`[(Published, ThisComputer, NewerRelease), (Applied, ThisComputer, Recovery)]`**, so the
  user sees a's add-on as published by b.
- Crash-only path and display only, so minor.
- Fix direction: add `rec.ops` keys to `applied_keys`. More robustly, skip folders whose
  published tree equals a tree this computer installed from a peer.

#### RV10-14 — note — any install or removal still replaces a waiting decision

- `progress_headline` lets Staging, Installing and Removing replace the Waiting headline even
  when the add-on being installed is not the one awaiting a decision.
- This lasts only for the apply. The release notes state it accurately ("The headline changes
  only when an install or removal actually starts"). Acceptable, but different from PLAN.md's
  "only satisfied decisions" wording. The orchestrator should confirm.

#### RV10-15 — note — other legacy lines still appear next to structured records

- `log_cycle` (`service.rs:4064`) still writes `published seq N (K new objects)` (K > 0) and
  `recovered …` lines.
- `covered()` hides only `applied` lines and 0-object publishes, so a local publish or a
  crash-recovered install shows both a structured entry and a plain line.
- Cosmetic.

### Release notes (docs/release/v0.3.3.md) against the code

- Accurate:
  - Progress examples: "Installing …", "Removing Atlas", "Publishing changes: 3 of 7".
  - Twice-a-second display rate (`MIN_GAP_MS` 500).
  - Routine checks under about 1.5 s are not shown (`QUIET_MS` 1 500).
  - A waiting decision stays the headline during routine checks.
  - The note lasts about 20 s (`NOTE_MS`).
  - "slow check" at two seconds or more (`SLOW_IDLE_MS` 2 000).
  - Crash-recovered installs show "unknown".
  - Known limits on mixed sources and clocks.
- Minor imprecision:
  - "Later routine checks do not clear it; starting real work does." A manual or after-a-change
    pass also clears the note at start, even when it does no work (`progress_ui.rs`
    `start_work` for a non-scheduled trigger).
  - The "1.2 of 1.8 GB" byte progress can step backwards for multi-folder add-ons (RV10-12).
  - Neither is worth blocking the release.

### Recheck probes (all reverted)

| Probe | File | Result |
|---|---|---|
| `rv_probe_second_folder_of_a_package_resets_bytes` | `crates/wow-sync-runtime/src/tracking.rs` (tests) | failed = bug confirmed (`[0, 60, 0]`) |
| `rv_probe_recovered_install_is_also_recorded_as_a_local_publish` | `crates/wow-sync-runtime/src/e2e.rs` (`change_history`) | failed = bug confirmed (Published/ThisComputer next to Recovery at crash points 6–9) |

### Recheck verdict

**No open critical or major findings.**
- RV10-02 through RV10-08 and RV10-11 are fixed. RV10-01 is fixed for its major part (the
  next add-on is named at once, and bytes follow the copy).
- New: RV10-12 and RV10-13 are minor; RV10-14 and RV10-15 are notes.
- Still open from the first review: RV10-09 is now documented, and RV10-10 is a note.
