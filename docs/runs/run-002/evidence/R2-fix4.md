# R2-fix4 evidence (run-002, D-17)

Worker: R2-fix4 (implementer). Branch `run002/R2-fix4`, created from main
`b73b60e`, in worktree `D:\GitHub\wow-sync.worktrees\R2-fix4`. This fixes **R3-M1**
from [R2-recheck3.md](R2-recheck3.md): the same Alpha `applied` result was
published twice. It happened when clearing the pending report failed and an
inventory-only state was published next.

## Design chosen

`report::outstanding` deduplicates only against this computer's **newest** own
state. The fix keeps that rule and adds an invariant: a newer own state may
replace the state carrying a report only after the saved pending store no longer
holds any result that state carries.

Before every publication, `publish_step` loads the saved store. If core
`report::store_needs_reconcile` returns true, it durably writes the store reduced
to its outstanding results. If that write fails, it publishes nothing in that
cycle ("not published: the saved apply report could not be updated; will retry")
and tries again on the next sync. A failed clear therefore delays publication;
it never duplicates a report.

Alternatives not chosen:

- **Carry the report in every later state.** Peers would see the same report
  again in every state.
- **Deduplicate against all retained own states.** Own states are pruned, so the
  evidence would eventually disappear.

The design is recorded in `docs/design/publication.md` §4 under "Apply report
durability".

## Changes

- `crates/wow-sync-core/src/report.rs`: new pure function
  `store_needs_reconcile`, plus a unit test.
- `crates/wow-sync-runtime/src/service.rs`: `publish_step` applies the
  reconcile-before-publish gate. Two comments now explain why the remaining
  ignored writes are safe.
- `crates/wow-sync-runtime/src/e2e.rs`: new permanent test
  `pending_clear_failure_does_not_duplicate_report` (Windows only). It is the
  re-check's reproduction, extended so that after the handle is released,
  publication resumes, Beta is published, and there is still exactly one Alpha
  result.
- `tests/required-cases.toml`: added `R2-fix4:core` and `R2-fix4:slice`.
- `docs/design/publication.md` and `docs/design/tests.md`: record the behaviour
  and the test.

## Repro against unfixed code

At the point of this run, the core function and the e2e test were present, but
`service.rs` was unchanged from `b73b60e`.

Command:
`cargo test -q -p wow-sync-runtime --lib -- --exact e2e::tests::pending_clear_failure_does_not_duplicate_report`

Result: **EXIT=101**, panicked at `e2e.rs:837`. The assertion failed with
"duplicate Alpha applied report after inventory-only publication". The second
sync returned `published seq 3 (0 new objects)`, which matches the re-check.

## A-38 negative controls

Each mutation was made in place, the exact test was run, and the code was
restored. `git diff` showed no remaining `MUTATION` markers, and nothing was
committed with a mutation.

| # | Mutation | Test | Result |
|---|---|---|---|
| NC-A | `service.rs` gate disabled (`if false && store_needs_reconcile(..)`) | `e2e::tests::pending_clear_failure_does_not_duplicate_report` | EXIT=101, panicked `e2e.rs:837` |
| NC-B | `report.rs` `store_needs_reconcile` always false (`.. && false`) | `report::tests::store_holding_a_carried_result_must_be_reconciled_first` | EXIT=101, panicked `report.rs:275` |
| NC-B | same | `e2e::tests::pending_clear_failure_does_not_duplicate_report` | EXIT=101, panicked `e2e.rs:837` |

Every failure was a test panic, not a build error.

## Commands and results

All commands were run in the worktree with the pinned toolchain on PATH.

| Command | Result |
|---|---|
| `cargo fmt --all` | applied; then verify fmt PASS |
| `cargo xtask verify --tier offline` | exit 0. PASS: fmt, clippy, test, required-cases, deny, markdown-links |
| `cargo test --workspace` | exit 0 |

| Crate | Passed | Failed | Ignored |
|---|---|---|---|
| wow-sync-adapters | 45 | 0 | 3 |
| wow-sync-app (`wow_sync`, `devkit`) | 2 + 0 | 0 | 0 |
| wow-sync-core | 172 | 0 | 0 |
| wow-sync-runtime | 26 | 0 | 0 |
| xtask | 24 | 0 | 0 |

## Not run and limitations

- **GUI:** NOT RUN.
- **Memory:** NOT RUN, as required by COMMON.md.
- **Non-Windows platforms:** the e2e test is `#[cfg(windows)]`, because the
  sharing violation is Windows-specific. The core predicate test runs on all
  platforms.
- **Liveness:** while the store stays unwritable, this computer publishes nothing,
  including new inventory. The publish note shows this. The cost is accepted so
  that no result is published twice.

## Environment

- Windows 11 x64 (Insider build 26300)
- Rust 1.100.0 (pinned)
- cargo-deny 0.20.2
- Synthetic fixtures only. No user paths, names or SIDs are recorded.
