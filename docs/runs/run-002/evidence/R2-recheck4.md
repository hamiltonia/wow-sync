# R2-recheck4: final bounded R3-M1 re-check

## Verdict: PASS

**R3-M1 resolved within the checked scope.** No new finding established.
Independent execution below verifies [R2-fix4](R2-fix4.md), rather than treating
the author's results as proof. Intent: durable reporting, R-12/A-24 and
publication design §4; mutation sensitivity, A-38.

## Environment and commands

- Checkout: `D:\GitHub\wow-sync.worktrees\R2-recheck4`;
  branch `run002/R2-recheck4`.
- Tested source: `63e0f5494c31041d2723c43a56a5ff54a8661736`.
  Initial worktree clean; existing dependency cache, not a cold-cache claim.
- Windows 11 x64, build 26300; pinned Rust toolchain 1.100.0.
- All commands below ran from that checkout after prepending
  `$env:USERPROFILE\.cargo\bin` to PATH. For fixture isolation, TEMP/TMP pointed
  to this checkout's disposable `target\recheck-fixtures`, removed afterward.

| Command | Actual result |
|---|---|
| `cargo test -q -p wow-sync-core --lib -- --exact report::tests::store_holding_a_carried_result_must_be_reconciled_first` | PASS, exit 0; 1 passed |
| `cargo test -q -p wow-sync-runtime --lib -- --exact e2e::tests::pending_clear_failure_does_not_duplicate_report` | PASS, exit 0; 1 passed |
| Same exact runtime command with publication gate disabled | Expected FAIL, exit 101; 0 passed, 1 failed, assertion panic, not a build failure |
| After restoration: `cargo test -q -p wow-sync-core --lib report::tests::` | PASS, exit 0; 5 passed |
| After restoration: `cargo test -q -p wow-sync-runtime --lib report` | PASS, exit 0; 2 passed: locked-clear regression and `apply_report_survives_crash_before_publish` |
| `git diff --exit-code -- crates` after restoration | PASS, exit 0; no product/test changes retained |

### Independent mutation control

Changed only `crates/wow-sync-runtime/src/service.rs:1269` to
`if false && store_needs_reconcile(...)`, then ran the exact e2e above.
Observed `e2e.rs:841`: **2** Alpha applied results versus expected **1**;
second sync published seq 3 with 0 new objects and no apply/recovery operations.
This reproduces the original [R3-M1](R2-recheck3.md) when the gate is disabled.
Restored the original line and re-ran the report tests successfully.

## Brief loss/liveness review

- `crates/wow-sync-core/src/report.rs:94-113`: reconciliation is required only
  for same-group/enrollment results already carried by the newest own state.
  `outstanding` (`:73-87`) retains other results, including different hashes;
  reducing the store does not discard an as-yet-unpublished result.
- `crates/wow-sync-runtime/src/service.rs:1268-1285`: before a needed
  publication, reloads the durable store and persists its reduced form.
  Failure returns **before** sequence advancement or publication, retaining
  the newest state that supplies deduplication evidence. It leaves inventory
  unpublished rather than falsely recording it as published.
- `service.rs:1063-1078`: reconciliation is retried on subsequent syncs;
  publication is still requested when inventory differs. The Windows e2e
  (`e2e.rs:786-875`) releases the sharing-denial handle, syncs again, verifies
  Beta eventually appears in published inventory and Alpha still has exactly
  one applied result. Thus the checked lock-release case does not permanently
  stall inventory or duplicate a report.
- `service.rs:1331-1336`: Aborted/Failed publication does not clear pending
  results. A crash after a successful reduction leaves any outstanding results
  on disk; removed results were already published. Lost post-publication
  clears remain guarded by this same reconciliation gate on the next publish.
- **Known limitation, not a new finding:** a persistently unwritable pending
  store can delay all publication, including inventory, as documented. The
  existing policy treating unreadable/corrupt stores as absent remains unchanged;
  this is not a guarantee against loss under arbitrary state corruption or
  every compound I/O failure. No additional permanent stall or report loss
  introduced by this gate was established in this bounded read-only review.

## Limits and safety

- Full offline/workspace verification **NOT RUN**, explicitly waived for this
  tiny check. Orchestrator reported offline PASS; that is not claimed as my
  independent execution.
- GUI, native/provider validation and **memory: NOT RUN**. V1 owner review
  remains outside this assignment.
- Synthetic fixtures only; no real WoW/Dropbox data, imported Lua execution,
  private settings, credentials or user-specific paths recorded.
- No app/FakeClient launched or process stopped/discovered/traversed; Cargo
  and tests finished naturally. No subagents, Git maintenance, reset, rewrite,
  push or merge. Only this evidence document is committed.
