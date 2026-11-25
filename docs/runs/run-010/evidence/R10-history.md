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

## Open issues

- Settings records name the folder only; no account/character paths are stored.
- Not rebased onto R10-speed's scan changes yet.
