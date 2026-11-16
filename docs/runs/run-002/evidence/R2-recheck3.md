# R2-recheck3: bounded independent re-check

## Verdict: FAIL

One new **major** finding: a failed pending-report clear can cause duplicate
publication after an inventory-only state. No new blocker found. Passing
existing tests does not close this finding.

Scope: independently check [R2-fix2](R2-fix2.md) and
[R2-fix3](R2-fix3.md), particularly A-38 controls, consent (R-02/R-03/A-46),
and durable apply reports (R-12/A-24 and publication design §4). Evidence-only
assignment; no product fixes or permanent test changes.

## Environment and commands

- Checkout: `D:\GitHub\wow-sync.worktrees\R2-recheck3`;
  branch `run002/R2-recheck3`.
- Tested source: `7da6d0f59004ef71aea50a384df5c77e6d363707`.
- Windows 11 x64, build 26300; pinned Rust/Cargo 1.100.0,
  cargo-deny 0.20.2. Existing dependency cache, not a cold-cache claim.
- Fresh-shell prerequisite: prepend `$env:USERPROFILE\.cargo\bin` to PATH.
  Commands below run from the exact checkout above.
- Initial worktree clean. Initial offline report independently checked:
  named source SHA above, `git_dirty=false`,
  `generated_at=2026-11-16T07:11:00Z`; xtask built in this checkout's own
  target directory, not reused from another checkout.

| Command/check | Actual result |
|---|---|
| `cargo xtask verify --tier offline` | PASS, exit 0: fmt, clippy, test, required-cases, deny, Markdown links |
| `cargo test -q --workspace` | PASS, exit 0: adapters 45, app 2, core 171, runtime 25, xtask 24; **267 passed, 3 ignored** provider cases |
| Four negative controls below | PASS as controls: all caused named assertion failures, exit 101; no build failures or zero-test passes |
| Additional independent runtime probes | 1 passed, 1 failed, exit 101; details below |
| Restored-source `cargo xtask verify --tier offline` | PASS, exit 0, all six checks including staged evidence links |
| Restored-source `cargo test -q --workspace` | PASS, exit 0, again 267 passed and 3 ignored |

Final offline report: same source SHA, `git_dirty=true` for the staged evidence,
`generated_at=2026-11-16T07:16:14Z`. `git diff --exit-code -- crates`
confirmed every temporary source/test edit restored. Disposable probe fixture
directory removed; only this evidence document is staged. Restored suite
success does not resolve R3-M1: the additional failing probe is retained as
repro source below, not installed into the product suite.

## A-38: independently executed negative controls

Each mutation was applied separately, tested, then reversed. The exact test
commands used `cargo test -q -p <crate> <target> -- --exact <name>`.
App target was `--bin wow-sync`; core/runtime target was `--lib`.
Each command executed exactly one test.

| Control | Temporary production mutation | Crate: exact failing test | Observed failure |
|---|---|---|---|
| NC1 | `consent.rs`: reset guard `self.panel != panel` changed to `false && self.panel != panel` | app: `consent::tests::boxes_reset_when_the_panel_is_reshown_or_changes` | `consent.rs:41`, “changed preview needs fresh consent”; 0 passed, 1 failed, exit 101 |
| NC2 | runtime `effects.rs`: guarded `self.record_finalized(op)?` with `if false` | runtime: `e2e::tests::apply_report_survives_crash_before_publish` | `e2e.rs:727`, `k=6: exactly one report, got []`; exit 101 |
| NC3 | core `report.rs`: retained carried results instead of dropping them (`c.results.contains(r) || true`) | core: `report::tests::outstanding_drops_what_the_newest_state_already_carries` | `report.rs:228`, retained alpha plus beta instead of beta only; exit 101 |
| NC3 | same mutation, runtime test too | runtime: `e2e::tests::apply_report_survives_crash_before_publish` | `e2e.rs:746`, `k=10: republished`, seq 3, 0 new objects; exit 101 |
| NC4 | core `variant.rs`: scope gap only for guided clients (`guided && !c.scope`) | core: `variant::tests::consent_predicate_requires_scope_and_guided_identity` | `variant.rs:1213`, `None` instead of `Some(Scope)`; exit 101 |
| NC4 | same mutation, service-boundary test too | runtime: `e2e::tests::recognized_enroll_without_scope_consent_is_refused` | `e2e.rs:791`, “enrolled without scope consent”; exit 101 |

`git diff` confirmed all four production files restored before the additional
probes; only the two temporary probe tests remained in `e2e.rs`. Those were
also removed before final verification. No existing assertion was weakened.

## Major R3-M1: duplicate report after a failed clear and inventory publication

**Locations:** `crates/wow-sync-runtime/src/service.rs:1069-1077`
(ignored reconciliation-write failure), `:1297-1306` (ignored clear failure),
and `crates/wow-sync-core/src/report.rs:81-85` (deduplicates against only the
latest own state).

**Reproduction on unmutated production code:**

1. Synthetic A publishes Alpha; B joins, applies and publishes its report
   in seq 1. Assert exactly one published Alpha applied result.
2. Restore that same valid report to B's local `pending-report.json`,
   simulating the already-supported crash-after-publication/before-clear
   window. Restart B.
3. Keep the pending file readable but deny its replacement with a Windows
   read handle using `share_mode(1)` (`FILE_SHARE_READ`, no delete/write
   sharing). Independently assert `LocalState::write_atomic` fails for this
   file. Enrollment and provider files remain writable.
4. Add synthetic Beta locally at B. Sync: seq 2 publishes its inventory
   without a report; reconciliation's failed clear was ignored.
5. Sync again: latest seq 2 has no report, so the stale pending Alpha result
   is treated as outstanding and published again in seq 3.

**Actual:** temporary independent test
`e2e::tests::recheck_pending_clear_failure_does_not_duplicate_report`
failed with **2** Alpha applied results versus expected **1**.
The second sync returned `published seq 3 (0 new objects)`, no apply or
recovery operations, and headline “Everything is up to date”.
This violates publication design §4's “a report must not be published twice”.
It demonstrates duplicate reporting, **not** a false apply or addon-data loss.

Command:
`cargo test -q -p wow-sync-runtime --lib recheck_ -- --nocapture`.
Two temporary probes ran: the consent probe passed; this probe failed
(exit 101). Only the probes, not production behavior, were added for this
reproduction. Source of the failing probe is retained below so the finding is
repeatable without retaining changes outside the evidence assignment.

**Needed correction:** do not advance to a report-free inventory state after
failing to durably reconcile a carried report, or retain durable deduplication
evidence across later states. Preserve retryability; add a regression covering
this ordinary local sharing violation. No correction attempted here.

## Pending-report review

- **Ordinary crash/restart and lost clear:** existing
  `apply_report_survives_crash_before_publish` passed unmodified, including its
  assertion that a post-apply/pre-publish window was exercised and its simulated
  lost-clear case. NC2 and NC3 demonstrate sensitivity to persistence/dedup
  regressions. Its lost-clear case has no intervening inventory change or
  persistent clear failure, which is why R3-M1 is not caught.
- **Contradictory results:** `report.rs:27-50` replaces the older result for
  each package; the executed `newer_result_replaces_older_for_same_package`
  covers replacing Applied with Interrupted. This does not justify a claim
  that every compound I/O failure was exercised.
- **Unfinished/rolled-back apply:** `report.rs:13-23` produces Applied only
  for `Verified`. The executed `only_a_verified_journal_claims_applied` checks
  Staging, Committing, Committed, NeedsHelp and RolledBack return no report.
  `exec.rs:712-718` transitions to Verified after verification;
  `:748-784` finalizes before returning Applied. Runtime records before journal
  removal (`effects.rs:133-152,269-272`). No new false-Applied defect established
  in this bounded review.
- **Failed publication:** `service.rs:1309-1313` leaves a successfully stored
  pending report intact on Aborted/Failed; only Published requests clearing.
  A failed finalized-result write propagates before journal deletion, leaving
  recovery possible. Non-crash provider failure was reviewed, not separately
  injected in this check. Unreadable/corrupt stores count as absent by the
  explicit design; no stronger durability claim is made for those cases.

## Consent changes: intent, coverage and wiring

- **Recognized enroll without scope: PASS.** SPEC R-03 requires setup consent,
  without a recognized-client exemption. Service `service.rs:623` calls the
  core predicate before opening/writing provider enrollment records. Existing
  `recognized_enroll_without_scope_consent_is_refused` passed: refuses, writes
  no shared `WoWSync` directory, remains unenrolled, then accepts scope consent.
  NC4 independently proves that boundary test detects weakening the predicate.
- **Reconfirm without identity: PASS.** Core
  `variant.rs:553-576,1231-1243` requires both confirmations for reconfirm,
  including recognized clients. This is consistent with the guided
  identity/scope safety intent of SPEC R-02/A-46 and the existing reconfirm UI.
  Service `service.rs:563` enforces it before writing enrollment.
  No dedicated existing service-boundary test for recognized reconfirm was
  found. Independently added temporary
  `recheck_recognized_reconfirm_requires_identity`: recognized A enrolled;
  scope=true/identity=false reconfirm refused, enrollment bytes unchanged;
  identity=true then accepted. **PASS** in the two-probe command above.
  This probe was removed afterward, not represented as a permanent regression.
- **Consent reset:** app helper test passed normally and failed under NC1.
  Reviewed `ui.rs:259-270`: every nonfatal render observes the panel identity,
  including `None`; enroll/reconfirm handlers clear consumed consent.
  GUI visuals, raw-error-line removal and visual VariantChanged rendering
  were **NOT RUN**; workspace covers their existing pure/runtime tests.
- R2-fix3's O-N3 owner-policy note remains owner-held. This review establishes
  consistency with the stated consent intent, not a new owner approval.

## Safety and limitations

- Synthetic fixtures only; no real WoW/Dropbox data, imported Lua execution,
  S-forever worktree, credentials or private settings accessed.
- **Memory: NOT RUN**; V1 owner review remains pending and is not closed here.
- **GUI: NOT RUN** (optional). No app/FakeClient launched; no process stopped,
  discovered or traversed. Cargo/test processes completed naturally.
- No subagents, Git maintenance, reset, history rewrite, push or merge.
- Findings refer to the tested source; this is not release acceptance or
  native/provider/platform evidence.

## Repro source

Insert temporarily inside the existing `e2e.rs` test module, run the two-probe
command above (or this exact test), then remove. Existing `Lab`, `Pc`,
`synced`, `computer_id` and `published_applied` helpers are used unchanged.

```rust
#[cfg(windows)]
#[test]
fn recheck_pending_clear_failure_does_not_duplicate_report() {
    use std::os::windows::fs::OpenOptionsExt;
    use wow_sync_core::records::{decode, encode, CodeState, PendingReport};
    let lab = Lab::new();
    let mut a = lab.pc("a");
    let mut b = lab.pc("b");
    a.addon("Alpha", "alpha-v1");
    let g = a.create();
    synced(&mut a);
    b.join(&g);
    synced(&mut b);
    assert_eq!(published_applied(&lab, &g, &b, "alpha").len(), 1);
    let states = lab.shared.join("WoWSync").join("groups").join(g.as_str())
        .join("members").join(computer_id(&b)).join("state");
    let st: CodeState = std::fs::read_dir(states).unwrap()
        .map(|e| decode::<CodeState>(&std::fs::read(e.unwrap().path()).unwrap(), 16 * 1024 * 1024).unwrap())
        .find(|st| st.apply_report.is_some()).unwrap();
    let pending = PendingReport {
        group_id: g.clone(),
        enrollment_id: st.enrollment_id,
        report: st.apply_report,
    };
    let bytes = encode(&pending);
    let path = b.cfg.state_dir.join("pending-report.json");
    std::fs::write(&path, &bytes).unwrap();
    b.restart();
    let _hold = std::fs::OpenOptions::new().read(true).share_mode(1).open(&path).unwrap();
    assert!(LocalState::open_at(&b.cfg.state_dir).unwrap()
        .write_atomic("pending-report.json", &bytes).is_err());
    b.addon("Beta", "beta-v1");
    let first = synced(&mut b);
    assert!(first.published.is_some(), "{first:?}");
    assert_eq!(published_applied(&lab, &g, &b, "alpha").len(), 1);
    let second = synced(&mut b);
    assert_eq!(
        published_applied(&lab, &g, &b, "alpha").len(), 1,
        "duplicate Alpha applied report after inventory-only publication: {second:?}"
    );
}
```
