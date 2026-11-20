# Evidence: R4-core-contracts (run-004, phase C, step 1)

Branch `run004/R4-core-contracts` in `D:\GitHub\wow-sync.worktrees\R4-core-contracts`, from main 313b5fb.
Status: **PARTIAL.** Foundation types, validation, ledger/barrier, same-release and composition, neutral
stubs and the tests listed below are done. The items under "Deferred" are NOT done.

## Verification (actual, Windows)

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) |
| `cargo xtask verify --tier native` | PASS |
| `cargo test --workspace` | core 207 passed (183 pre-existing, unmodified), adapters 50 passed (3 ignored, pre-existing), runtime 46, others 23/11/26; 0 failed |

Environment: Windows, Rust 1.100.0. Native NTFS/APFS identity collision behaviour (T-C11): **NOT RUN** (R4D-22).

## Tests added

- `target::contract_tests`: T-R4D23-same-release-table, T-R4D23-unknown-waits, T-R4D24-no-hybrid (observable
  result only; see A-38), T-R4D28-exact-source-unrelated-addon, T-R4D28-regrouped-exclusion, T-C14 (every
  adjacent level pair with neutral hooks), slice target unchanged under neutral hooks.
- `authority::tests`: T-R4D15-vret-predicate, T-R4D16-later-cutoff-no-reauth,
  T-R4D16-retired-issuer-retirement-void (both orders), T-R4D25-partial-retirement-dag (four orders plus a
  missing node), T-C10 (retirement barrier, both orders).
- `records::contract_tests`: T-R4D1-jrn-kind-actions, `/1` with `remove` rejected, T-R4D1 mixed-update shape,
  T-C05/T-C06 decode behaviour (slice decoder returns `UnsupportedMajor(2)`), T-R4D1-paired-derived.
- `ledger::tests`: T-C01 golden (removal, retirement, ack), T-C02 (wrong_reference), T-C03, T-C13,
  T-C09 (170 entries, over 16 MiB, in 4 MiB segments; also fixed a real segment-overflow bug found by this test).
- adapters `authority_store::tests`: T-C04 own-copy repair.

Golden fixtures are in `crates\wow-sync-core\fixtures\contracts\` (synthetic only):
`apply-journal-1.json`, `apply-journal-2-mixed.json`, `apply-journal-2-settings.json`,
`recovery-point-2-paired.json`, `removal-1.json`, `retirement-1.json`, `removal-ack-1.json`.
The helper `testkit::golden` fails on a missing file unless `WOWSYNC_BLESS=1`.

## A-38 negative controls (mutate, observe exit 101, restore; nothing mutated is committed)

| Control | Mutation | Observed |
|---|---|---|
| T-R4D23 common keys only | `same_release` skips keys the source lacks | `same_release_table` FAILED, exit 101 |
| T-R4D23 union of keys | `same_release_entries` uses only one entry's scope | `same_release_regrouped_exclusion` and `same_release_unknown_waits` FAILED, exit 101 |
| T-R4D25 representative | DAG never marks a node void for an effective dependency | `partial_retirement_dag_is_order_and_restart_stable` and `retirement_by_a_retired_issuer_is_void` FAILED, exit 101 |
| T-R4D24 per-folder selection | **NOT DEMONSTRATED**: the slice selects per package, so no per-folder selection exists to mutate; the no-hybrid test asserts the observable result only | not run |

## Design decisions and ambiguities

- **Additive types (owner amendment 1):** `/2` journal, point and report are separate types (`ApplyJournalV2`,
  `RecoveryPointV2`, `PendingReportV2`) with `decode_journal`/`decode_point` falling through on
  `UnsupportedMajor(2)`. Slice `/1` bytes are unchanged. Old `DecisionReason` stays; new `DecisionKind` is
  the full list. `view.rs`/`explain.rs` gained only the standalone `hold_sentence`, no enum variants.
- `arch.rs`: only `"unicode-normalization"` added to the pinned dependency list (approved).
- `Cargo.lock` hand-edited to add `unicode-normalization` (and `tinyvec`) only.
- `FolderAction::Remove` was added; its only consumers are `==` comparisons in owned code.
- `PendingReportV2`, `SettingsLocal`, `SettingsState` and `CleanupClass` shapes are my own reading of sparse text.
- `Record::MAJORS` default (empty) means only `MAJOR` is accepted.
- `InvalidReason::Kind` strings are the first word of the validator message.
- `compose` records removed and deferred sets and does not mutate `effective`.
- `MAX_SETTINGS_FILE_BYTES` is `None` pending Q-77; tests pass a test-only value.
- V-JRN ledger-level checks (each remove folder named by a listed removal; each listed removal valid or
  pending) are not in the journal validator.
- Process note: one test block was appended to `authority.rs` with a .NET `AppendAllText` call instead of the
  edit tool. Content is plain test code and visible in the diff.

## Deferred / not done (hand over to the owners)

- Executor: `/2` journal creation, the `remove` action, `pending_records` reserve/finalize and
  `recover_interrupted` for `/2` (T-C06 beyond decode behaviour, T-C06c crash slots, T-R4D1-mixed-update-drop
  recovery). Today a `/2` journal is `NeedsHelp`, kept, and blocks applies (slice behaviour).
- T-C06b (runtime `UnknownJournal` listing), T-C07 (pending report `/2` vs slice decoder).
- T-C08, T-R4D4-1024-issuers-readd, T-R4D4-mirror-only-readd, T-R4D4-full-policy, T-C12 (index chain).
- T-C11 offline fixtures for NFC collisions (the NFC helpers are implemented; the named diagnoses are not tested here).
- Golden files for the remaining kinds (readd, code-choice, rejection, lift, policy, revision, part, index chunk).
- `admit.rs` gained `barrier_gate`/`apply_barrier`; no end-to-end admission test yet.
- Runtime `control.rs`, `service.rs`, `effects.rs`, `activity.rs`: not changed (would not be needed for compile).
- Adapter `fs_install.rs`, `fs_safe.rs`, `provider.rs`, `testkit.rs`: not changed.

## Stubs and owners

Core: `removal`, `settings`, `reject`, `choice`, `decide`, `retire` (Behaviour structs delegating to `Neutral`
hooks), `notice`, `update`, `diagnostics`. Adapters: `wtf`, `settings_store`, `notify`, `update_http`.
Runtime: `removal_cycle`, `settings_cycle`, `rollback_flow`, `decisions`, `notices`, `updates`, `diag`.
Frozen hook signatures are in `crates\wow-sync-core\src\hooks.rs` (contracts.md §9.4).

## Commands run

`cargo check --workspace --tests`; `cargo test -p wow-sync-core <filter>`; `cargo test --workspace`;
`cargo fmt --all`; `cargo xtask verify --tier offline`; `cargo xtask verify --tier native`.
