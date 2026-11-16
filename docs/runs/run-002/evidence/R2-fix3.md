# R2-fix3 evidence (run-002, D-17)

Worker: R2-fix3 (implementer). Branch `run002/R2-fix3` from main `569a8b6`,
worktree `D:\GitHub\wow-sync.worktrees\R2-fix3`. Scope: follow-ups #9 and #6 and
findings mi-1 and mi-2 from [R2-review.md](R2-review.md).

## What changed

### Follow-up #9: durable apply report (report-after-recovery gap)

- `crates/wow-sync-core/src/report.rs` (new). Pure functions:
  - `finalized_result`: a journal produces an `applied` report only when it is
    `Verified`. Rolled-back or unfinished journals never claim applied.
  - `merge_report`: for the same package, the newer result replaces the older one.
  - `record_pending`: builds the pending store and discards a record that belongs
    to another group or enrollment.
  - `outstanding`: the report still to publish, minus results already carried by
    this enrollment's newest published state. This makes publication idempotent.
- `crates/wow-sync-core/src/records.rs`: added the `PendingReport` record
  (`wowsync.pending-report/1`).
- `crates/wow-sync-runtime/src/effects.rs`:
  - `pending-report.json` lives in the runtime state dir. `load_pending` treats an
    unreadable or corrupt file as absent.
  - `PendingSink` and `RealEffects::recording_into` added.
  - `remove_journal` records the finalized result *before* it removes the journal.
    If that write fails, the journal stays, so recovery re-derives the result.
- `crates/wow-sync-runtime/src/service.rs`, `sync_once`:
  - Reconciles the store at start, so outstanding results from a previous run are
    published on the next start or recovery.
  - Merges the apply outcome into the store before publishing.
  - `publish_step` clears the store's report only after `Published`.
- Status rebuilds "applied here" after a restart from the own latest state's report
  and the pending report.

### Follow-up #6 / mi-1 / mi-2: policy and rendering in core

- `crates/wow-sync-core/src/variant.rs`: added `ConsentAction`, `ConsentGap` and
  `consent_gap`. This is the core consent predicate for enroll and reconfirm,
  covering identity/scope consent and the guided-identity requirement.
  - Runtime: `Service::check_consent`.
  - App: `crates/wow-sync-app/src/ui.rs` (`enroll`, `reconfirm`). Its English
    notes are unchanged.
- `crates/wow-sync-core/src/status.rs`: added `headline_sentence`, `render_block`,
  `render_result`, `journal_status`, `remember_applied` and `applied_here`. They
  replace the runtime's `block_text` and `package_result_text`, and the core
  versions produce byte-identical text.
- `crates/wow-sync-core/src/plan.rs`: added `own_publish_slot`, which returns the
  own newest state and the next sequence number. A number is never reused.
- `crates/wow-sync-core/src/lib.rs`: module wiring.

### Behaviour changes (bug fixes)

- **mi-1:** the service refused an unfamiliar-client enroll without scope consent,
  but accepted a *recognized*-client enroll without it. The service now refuses it,
  matching the UI.
- The service now refuses a reconfirm without identity consent, matching the UI.
- The consent check runs after the existing variant confirmation, so the existing
  refusal reasons are unchanged.

### Tests, required cases and docs

- `crates/wow-sync-runtime/src/e2e.rs`: new tests and helpers.
- `tests/required-cases.toml`: added `R2-fix3:core` and `R2-fix3:slice`.
- `docs/design/publication.md`:
  - added the `wowsync.pending-report/1` row;
  - added "Apply report durability" to §4;
  - corrected the `inventory_complete` definition (mi-6).
- `docs/design/tests.md`:
  - provider tests panic with `BLOCKED: …` (mi-9);
  - added the `R2-fix3:slice` row and a core-tests note.

## Tests added

| Crate | Test | Covers |
|---|---|---|
| core | `report::tests::only_a_verified_journal_claims_applied` | never claim applied for rolled-back or unfinished applies |
| core | `report::tests::newer_result_replaces_older_for_same_package` | no contradictory results |
| core | `report::tests::record_discards_other_enrollment` | group/enrollment boundary |
| core | `report::tests::outstanding_drops_what_the_newest_state_already_carries` | no duplicate publication |
| core | `variant::tests::consent_predicate_requires_scope_and_guided_identity` | consent predicate |
| core | `plan::tests::own_publish_slot_never_reuses_a_number` | seq never reused |
| core | `status::tests::applied_here_is_rebuilt_from_own_report_after_restart` | status after restart |
| core | `status::tests::needs_help_journal_is_not_reported_as_interrupted` | journal split |
| runtime | `e2e::tests::apply_report_survives_crash_before_publish` | crash window, restart, published exactly once |
| runtime | `e2e::tests::recognized_enroll_without_scope_consent_is_refused` | mi-1 through the service |

### How `apply_report_survives_crash_before_publish` works

1. The test sweeps the crash point k over every effect of B's sync.
2. It requires that the window case is hit: apply finished, report not yet
   published, and `pending-report.json` present at crash time.
3. It restarts and syncs.
4. It asserts that exactly one `applied` alpha report is published, with the
   correct package hash, and that the last sync did not publish again.
5. It asserts that A shows "applied and reported by".
6. It also simulates a lost local clear: it writes back the uncleared pending
   file, restarts, syncs twice, and asserts there is no republish and still
   exactly one report.

## Negative controls (A-38)

Each control was mutated in place, the exact test was run with
`cargo test -q -p <crate> --lib -- --exact <test>`, and the code was restored.
`git diff` was checked after each restore. No mutation was committed.

| # | Mutation | Test | Result |
|---|---|---|---|
| NC1 | `effects.rs` `remove_journal`: removed the `self.record_finalized(op)?` call | `e2e::tests::apply_report_survives_crash_before_publish` | EXIT=101, panicked `e2e.rs:724` "exactly one report, got []" |
| NC2 | `report.rs` `outstanding`: carried-results filter disabled | `report::tests::outstanding_drops_what_the_newest_state_already_carries` | EXIT=101, panicked `report.rs:229` |
| NC2 | same | `e2e::tests::apply_report_survives_crash_before_publish` (with the lost-clear step) | EXIT=101, panicked `e2e.rs:746` (republish) |
| NC3 | `finalized_result`: `Verified \|\| true` | `report::tests::only_a_verified_journal_claims_applied` | EXIT=101, panicked `report.rs:156` |
| NC4 | `merge_report`: keep the older result | `report::tests::newer_result_replaces_older_for_same_package` | EXIT=101, panicked `report.rs:175` |
| NC5 | `record_pending`: `&&` changed to `\|\|` in the group/enrollment filter | `report::tests::record_discards_other_enrollment` | EXIT=101, panicked `report.rs:200` |
| NC6 | `consent_gap`: scope gap only when guided | `variant::tests::consent_predicate_requires_scope_and_guided_identity` | EXIT=101, panicked `variant.rs:1214` |
| NC6 | same | `e2e::tests::recognized_enroll_without_scope_consent_is_refused` | EXIT=101, panicked `e2e.rs:791` |
| NC7 | `service.rs` enroll: `check_consent` call commented out | `e2e::tests::recognized_enroll_without_scope_consent_is_refused` | EXIT=101, panicked `e2e.rs:791` "enrolled without scope consent" |
| NC8 | `own_publish_slot`: `stored_next_seq` ignored | `plan::tests::own_publish_slot_never_reuses_a_number` | EXIT=101, panicked `plan.rs:606` |
| NC9 | `applied_here`: package-hash match bypassed | `status::tests::applied_here_is_rebuilt_from_own_report_after_restart` | EXIT=101, panicked `status.rs:726` |
| NC10 | `journal_status`: NeedsHelp sent to interrupted | `status::tests::needs_help_journal_is_not_reported_as_interrupted` | EXIT=101, panicked `status.rs:749` |

Every failure was a test panic, not a build error. Before the lost-clear step was
added to the e2e, it passed under NC2. `Faulty` cannot inject a crash between the
state write and the local clear, because `LocalState` writes are not counted
effects. The e2e therefore simulates that crash by restoring the file bytes.

## Commands and results

All commands were run in the worktree after `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo xtask verify --tier offline` | exit 0. PASS: fmt, clippy, test, required-cases, deny, markdown-links. `overall: PASS (allow-blocked: false)` |
| `cargo test -q --workspace` | exit 0 |
| `cargo test --workspace` (per-crate counts) | see the table below |

| Crate (binary) | Passed | Failed | Ignored |
|---|---|---|---|
| wow-sync-adapters (lib) | 45 | 0 | 3 |
| wow-sync-app (`wow_sync` main) | 2 | 0 | 0 |
| wow-sync-app (`devkit`) | 0 | 0 | 0 |
| wow-sync-core (lib) | 171 | 0 | 0 |
| wow-sync-runtime (lib) | 25 | 0 | 0 |
| xtask | 24 | 0 | 0 |
| doc-tests (adapters, core, runtime) | 0 | 0 | 0 |

## Skipped and not run

- **Digest matching for recognized clients.** docs/design does not specify a
  preview-digest requirement for recognized-client enroll or reconfirm, so the
  predicate checks only consent and guidance. Not invented.
- **Other runtime-rendered notes.** "not published: …", "published seq …", the
  recovery note, and "Published N s ago" stay in runtime. They were not named in
  mi-2.
- **"Applied here" detail after a restart.** It uses the generic
  `Explanation::AppliedHere`. Rich plan explanations are not persisted; that would
  need a design decision.
- **Basis of recovery-finalized results.** Results recorded at finalize or recovery
  have an empty basis, because the journal does not carry one.
- **apply.md §2.5.** No cross-reference was added, because the file is not in my
  owned paths.
- **O-N3 is still pending.** The owner has not yet confirmed that recognized-client
  scope consent belongs in core. The behaviour fix (mi-1) is implemented and
  listed here for review.
- **mi-9 in HANDOFF.md / USAGE.md.** Not changed, because those files are not
  owned paths.
- **GUI: NOT RUN.** The app was not driven interactively.
- **Memory measurement.** Not performed, as required by COMMON.md.

## Environment

- Windows 11 x64 (Insider build 26300)
- cargo 1.100.0 (3acc79c0c 2026-11-05) and rustc 1.100.0 (0ef1bc386 2026-11-09),
  pinned
- cargo-deny 0.20.2
- No user-specific paths, names or SIDs are recorded.
