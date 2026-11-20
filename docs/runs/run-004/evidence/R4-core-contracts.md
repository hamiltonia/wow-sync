# Evidence: R4-core-contracts (run-004, phase C, step 1)

Branch `run004/R4-core-contracts` in `D:\GitHub\wow-sync.worktrees\R4-core-contracts`, from main 313b5fb.
Status: **DONE except the items under "Still deferred"** (real-effects wiring for `/2`, owned elsewhere).

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
| T-R4D24 per-folder selection | ambiguous branch of `target.rs` projects each candidate onto each folder and picks the newest per folder | `no_hybrid_of_two_releases_in_one_package` FAILED, exit 101; restored |

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
- Process note: one test block was appended to `authority.rs` with a .NET `AppendAllText` call instead of the
  edit tool. Content is plain test code and visible in the diff.

## Deferred / not done (hand over to the owners)

Still deferred:

- **Real effects for `/2`:** `RealEffects` and `Faulty` (runtime `effects.rs`, not owned) do not forward the new
  `ApplyEffects` methods (`write_journal_v2`, `write_recovery_point_v2`, `write_pending_record`, settings file
  moves). The trait defaults refuse, so a `/2` operation fails safely at journal creation with no live change.
  Settings file moves also need `fs_safe`/`WtfScope` adapters (not owned). Owners of the step-2 packets wire these.
- `admit.rs` `barrier_gate`/`apply_barrier`: no end-to-end admission test yet.
- Native T-C11 identity collisions: NOT RUN (R4D-22).
- Q-77 settings file cap: `None` pending the owner decision.

## Done since the first pass (all offline, Windows)

- **Executor `/2`** (`exec.rs`, `exec_tests.rs::v2`): journals for every §2.2 kind, `remove`, `pending_records`
  reserve/finalize (bytes written only after verify), `recover_interrupted` for `/1` and `/2`.
  T-C06c crash slots for all 6 kinds, re-entrant recovery, T-R4D1-mixed-update-drop from each slot, T-C05
  (a pure-code apply still writes a `/1` journal, bytes unchanged), changed-settings replan, game-running stop,
  unsupported-port failure.
- **V-JRN ledger checks** (`ledger::validate_journal_against_ledger`): `removal_not_valid`,
  `remove_folder_unnamed`. A removal is acceptable if the ledger says Valid, or a pending Ack record names it.
- **T-C06b**: `control::split_journals`/`UnknownJournal`; `service.rs` lists undecodable journals as a status
  block and never deletes them (test `control::tests::an_undecodable_journal_is_listed_by_name_and_the_known_ones_are_kept`;
  the view uses the same `blocks`). `StatusSnapshot` unchanged (additive via `blocks`).
- **T-C07, T-C08, T-R4D4-1024-issuers-readd, T-R4D4-mirror-only-readd, T-R4D4-full-policy, T-C12, T-C11 offline**
  (NFC collision and case-only), via `kinds::pack_items`/`pack_known` and ledger part/index-chain readers.
- **Golden fixtures** for readd, code-choice, rejection, lift, policy, revision, part, index chunk
  (15 goldens in total).
- **Line endings (T-C01):** `fixtures\contracts\.gitattributes` = `* -text`; fixtures renormalized (`i/lf w/lf attr/-text`).
  Proof: deleted `apply-journal-1.json`, `git checkout --` restored it, SHA256 7013A5A8… identical before/after,
  golden test passes.
- **T-R4D24 control:** `target.rs` mutated to choose per folder in the ambiguous branch;
  `no_hybrid_of_two_releases_in_one_package` FAILED, exit 101; restored with `git checkout -- target.rs`.
- **Additional ambiguity choices:** settings fingerprint is ignored (`ExpectedLive::Tree.tree` is the file hash);
  `read_parts` is stricter (a part reached twice gives `bad_part`, a wrong `of` gives `wrong_reference`);
  `pack_items` inline budget 48 KiB, parts at most 60 KiB, fan-out at most 16; index `processed` maps hash to
  cumulative ref count.
- Test counts (`cargo test --workspace`): core 225, adapters 50 (3 ignored), runtime 47, app 23, proto 11,
  xtask 26; 0 failed. Both verify tiers PASS. New cases added to `tests\required-cases.toml`.

## Stubs and owners

Core: `removal`, `settings`, `reject`, `choice`, `decide`, `retire` (Behaviour structs delegating to `Neutral`
hooks), `notice`, `update`, `diagnostics`. Adapters: `wtf`, `settings_store`, `notify`, `update_http`.
Runtime: `removal_cycle`, `settings_cycle`, `rollback_flow`, `decisions`, `notices`, `updates`, `diag`.
Frozen hook signatures are in `crates\wow-sync-core\src\hooks.rs` (contracts.md §9.4).

## Commands run

`cargo check --workspace --tests`; `cargo test -p wow-sync-core <filter>`; `cargo test --workspace`;
`cargo fmt --all`; `cargo xtask verify --tier offline`; `cargo xtask verify --tier native`.
