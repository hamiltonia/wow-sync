# R10-history evidence

Branch `run010/R10-history`, built on the merged interface (main `33261fe`).

## What was built

- `history.rs`: `ChangeRecord` persistence in `changes.jsonl` (state dir; one synced JSON line per
  record, torn lines skipped, newest 2000 kept, trimmed past 4 MiB), queries (`recent_changes`,
  `changes_for_package`, `changes_by_pass`, `history` merging legacy `audit.jsonl` lines), and a
  working `ProgressHub` (`emit` always, `emit_throttled` at 500 ms).
- `changes.rs`: builds a record per package from the plan, pre-apply inventory, target and ledger
  (source computer, old/new version, file counts, byte delta, reason, capped relative paths).
  Also Published, SettingsPublished and Kept records.
- `tracking.rs`: `PassGuard` (Scanning at start, Idle on every exit), `ScanTicker`,
  `ApplyProgress` (incoming summary, `decision_satisfied`, per-package completion that writes the
  record and emits `completed`), `PublishProgress`.
- `service.rs` / `effects.rs`: Deciding, Installing, Publishing phases; hooks on stage, finalize
  and publish object checks.

## Results

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, links).
- `cargo xtask verify --tier native`: PASS.
- New tests (required-cases section `R10-history`): history unit tests (throttle, torn line, old
  record without `ms`), e2e `change_history` (install and update records with source/versions/
  counts/reason and restart persistence, removal author, ordered progress with incoming summary
  and completed records, legacy lines still listed).

## Round 2

- `IncomingSummary.source` is now `Option<ChangeSource>`; `None` means several computers sent the
  add-ons (no new enum variant). `ProgressEvent.source` is `None` in that case too.
- Tests added: mixed-source summary, `decision_satisfied` (display names only), rollback, restore
  (recovery) and master settings records. Settings records now use the display name; restore
  records carry an 8-hex content-id version label.

## Round 4 (review fixes)

- RV10-01: a package's first staging event is unthrottled; bytes now move as files are copied
  (`ApplyProgress::copied`, per file in `stage_folder`) and are topped up at completion.
- RV10-03: records are stamped at finalize time. `history()` hides a legacy `applied` line when
  a record of that package was made after the package's previous such line (no 60 s window).
- RV10-04: `published()` diffs file by file against the previous published tree (read from the
  provider only for changed folders); falls back to net counts if that tree is unreadable.
- RV10-05: `published()` records a `Removed` record (this computer, reason Removal) for a folder
  in the previous published state that is gone locally and was not removed by this pass's apply.
- RV10-06: the scan counter is reset at pass start.
- RV10-08: a crash-recovered install writes an `Applied` record with reason `Recovery` (old
  version unknown: an empty content id).
- No public API change.

## Round 5 (recheck fixes)

- RV10-12: byte progress is counted per folder; only a second try at the same folder resets
  that folder's count. Test: `tracking::tests::bytes_never_go_back_across_the_folders_of_one_add_on`.
- RV10-13: keys finished by crash recovery are excluded from publishing, so no local
  `Published`/`Removed` record is made for them. Test:
  `e2e::tests::change_history::a_recovered_peer_install_keeps_its_source_and_is_not_a_local_publish`.
- RV10-10: stale hash-cache comment in `service.rs` updated.
- No public API change (`staging`/`copied` are `pub(crate)`).

## Round 5b (recovery source)

- The apply saves each package's source to `apply-sources.json` in the state dir
  (`history::save_apply_sources`, no paths or contents) before it touches the game files;
  recovery reads it (`load_apply_sources`) and `changes::recovered` takes it. A missing file
  (journal from an older version) gives `Computer("another computer")`, never `ThisComputer`.
  No public API change; the UI already renders `Computer(n)`.
- New test `a_recovered_install_without_a_saved_source_is_not_called_this_computer`; the
  peer-install test now asserts `Computer("a")`.
- Fails-without-fix proof (fix temporarily reverted, then restored):
  - Folder bytes: `bytes_never_go_back_across_the_folders_of_one_add_on` failed with
    `[0, 40, 60, 0, 30, 40, 100]`.
  - No local publish: `a_recovered_peer_install_keeps_its_source_and_is_not_a_local_publish`
    failed at k=6 with a `Published`/`ThisComputer` record beside the Recovery record.
  - Source: with the saved source ignored, both source tests failed at k=6
    (`left: ThisComputer`, `right: Computer("a")` / not `ThisComputer`).

## Round 6 (R10-check findings)

- G3: ledger lookups now match on the record's own folders (Entry.folders is empty for learned decisions), so a peer's copy choice records `Choice(Some(computer))`. A peer rollback arriving as a plain apply is recorded `RolledBack` / `Rollback`. A's own rollback shows the live TOC version. Tests: `a_peers_copy_choice_...` (failed before: NewerRelease vs Choice), `a_peers_rollback_...`.
- G4: publishing, settings-published and crash-recovered records are now announced on the progress stream (`tracking::recorded`, `completed: Some(rec)`), so the pass summary counts them and phase/record log lines appear. Tests: `publishing_a_package_announces_its_record_on_the_progress_stream`, app `a_pass_that_publishes_is_logged_and_summarised_as_publishing`. App files touched: `progress_ui.rs` (test only), `history_ui.rs`, `pages.rs`.
- G1: add-on page Recent changes rows use `ChangeRow::line_with_source` ("from <computer>" / "from this computer"). Test `an_add_on_pages_rows_name_where_each_change_came_from`.
- G2 deferred (settings records untouched). G5: no fixed ports or obvious race found in the update_http TLS tests; root cause not established, no change.
- No public API break (ChangeRow gained a `from` field; app-internal).

## Open issues

- Settings records name the folder only; no account/character paths are stored.
- Not rebased onto R10-speed's scan changes yet.
