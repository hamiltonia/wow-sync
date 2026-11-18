# Evidence: R4-design-fix2 (retry 1)

- **Packet:** [R4-design-fix](../packets/R4-design-fix.md), retry 1 after
  the failed [R4-design-recheck](R4-design-recheck.md)
- **Previous attempt:** [R4-design-fix](R4-design-fix.md), merged as
  `c939b30`
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-design-fix2`
- **Branch:** `run004/R4-design-fix2`, from `main` at `68f43ed`
- **Environment:** Windows x64 owner workstation

This retry is design only. No product code, test code, build or CI file
changed. The owner was unavailable, so nothing here is recorded as an
owner decision. Q-49 and Q-50 keep their owner-accepted (D-21) semantics.
Every open choice stays at its conservative default marked **PENDING
OWNER**. These are the author's claims; an independent recheck is still
required. The tests named below are **design obligations**: none has been
implemented or run.

## Files changed

| File | Change |
|---|---|
| [contracts.md](../../../design/contracts.md) | Known with parts and no caps (§1.8–§1.9); retirement with `retirements_known`, void retirements, MIN cutoff, V-RET predicate (§1.10–§1.11); C-JRN kinds × actions table, `pending_records`, crash table, derived `paired` with `SettingsRoot` states (§2.2–§2.3); runtime `UnknownJournal` row (§2.4); native NTFS/APFS NOT RUN (§3.3); per-folder constraint liveness and `matches` (§4); code-choice `exclude`/`allow_again`, `NewVersionAfterDecision`, per-action V-CHOICE (§5); phase C packet map, hook traits and build order (§9); new tests (§10) |
| [removal.md](../../../design/removal.md) | cycle classifies due acks before any absent ack (R4D-5); mixed update+drop in one `apply/2`; ack bytes fixed in `pending_records`; MIN cutoff; step-1/step-2 map; tests |
| [rollback.md](../../../design/rollback.md) | folder-changing rollback F = F0 ∪ F1 in one `rollback` journal with rejection, rollback removal and re-add in `pending_records` (R4D-20); union rule; per-folder liveness (R4D-21); derived `paired`; step-1/step-2 map; tests |
| [settings.md](../../../design/settings.md) | ordered, mutually exclusive target cases (R4D-18); activation capture and strict epoch (R4D-19); partable policy folders and revision parents (R4D-4); derived `paired`; step-1/step-2 map; tests |
| [ops.md](../../../design/ops.md) | resume keeps exclusions and `NewVersionAfterDecision` (R4D-10); effective/void retirements and MIN cutoff (R4D-15/16); step-1/2/3 map; tests |
| [questions.md](../../../design/questions.md) | Q-49, Q-50 (owner semantics kept), Q-52, Q-61 (now PENDING OWNER), Q-69 (NOT RUN), Q-84 |

## Finding → fix → named counterexample tests

| Finding | Fix | Tests |
|---|---|---|
| R4D-1 (blocker) C-JRN vs required ops | contracts §2.2: `/1` only for a pure install/replace apply; `/2` kinds × actions table; a remove needs a listed removal in the ledger or in `pending_records`; one `apply/2` for update+drop; `pending_records` reserved at creation and written after verify; crash table; §2.3 `paired` derived from `SettingsRoot` states | T-R4D1-jrn-kind-actions, T-R4D1-mixed-update-drop, T-R4D1-paired-derived, T-R4D1-ack-bytes-fixed, T-C05, T-C06b, T-C06c |
| R4D-4 write dead ends | `known` and every unbounded list are partable (`wowsync.authority-part/1`, depth ≤ 8, fan-out ≤ 16, parts ≤ 60 KiB); no `GroupHistoryTooLarge`, `TooManySettingsHeads` or 512-folder limit; mirror-only coverage through `extra` | T-R4D4-1024-issuers-readd, T-R4D4-mirror-only-readd, T-R4D4-full-policy, T-R4D4-policy-parts |
| R4D-5 absent ack bypasses cleanup | removal cycle 3a classifies every due item (`cleanup_required` / `no_cleanup`) before 3b; only `no_cleanup` is acked at once; `cleanup_required` is acked by the `settings_cleanup` journal's finalize | T-R4D5-absent-ack-waits-cleanup, T-R4D5-passive-uninstall-settings |
| R4D-8/R4D-18 settings target overlap | settings §4.2: recorded basis kept as recorded; seven ordered, mutually exclusive cases; automatic yield only when the head is strictly newer; an authority with a local change gets `ConcurrentPush`; an invalidated basis is `BasisNotShared`, never first adoption | T-R4D18-unchanged-head-local-change, T-R4D18-invalidated-basis, T-R4D8-retired-basis |
| R4D-9/R4D-21 wide constraint lost | contracts §4.1–§4.2: liveness and supersession per folder; a narrower later record covers only the folders it names; `matches` projection | T-R4D21-partial-choice-over-choice, T-R4D21-choice-over-rollback, T-R4D21-readd-over-rollback, T-R4D9-choice-vs-rollback, T-R4D9-concurrent-rollbacks |
| R4D-10 (blocker) changed decided intent | code choice keeps Q-49 (an unconsidered newer or unordered candidate gives `NewVersionAfterDecision`; the keep stays) and Q-50 (resume inherits `exclude` unless `allow_again`); V-CHOICE conditional per action; no Interface gating, no unsafe-metadata override | T-R4D10-new-version-after-keep, T-R4D10-resume-keeps-exclusions, T-R4D10-resume-allow-again, T-R4D10-vchoice-per-action, T-R4D10-unusable-metadata-no-override, T-R4D10-interface-advisory |
| R4D-15 V-RET exempts the wrong records | V-RET: a retirement (void → `invalid(void_retirement)`), or issuer not retired, or `issued_at_seq ≤ Cutoff`; evaluated after V-REF | T-R4D15-vret-predicate |
| R4D-16 reauthorized retired authority | Cutoff = MIN over effective retirements (valid → invalid only); a retirement written knowing its issuer was retired is void (`retirements_known`) | T-R4D16-later-cutoff-no-reauth, T-R4D16-retired-issuer-retirement-void, T-45-8, T-S26 |
| R4D-17 (blocker) ownership graph | contracts §9: step 1 R4-core-contracts owns every shared type, kind, validator, journal, fixture and hook trait; step 2 removal / settings(+rollback) / ops in parallel on disjoint files; step 3 notify-diag and app | T-R4D17-file-ownership (merge gate) |
| R4D-19 epoch equality | activation capture A with `local_seq > authority_since`; a difference at activation sets `epoch_clean = false` until the user shares or uses shared settings; publication requires `compared_with_local_seq > authority_since` (strict) | T-R4D19-epoch-equal-seq |
| R4D-20 folder-changing rollback | rollback.md §1: F = F0 ∪ F1; one `rollback` journal with replace, install and remove; rejection plus rollback removal of F1\F0 and re-add of F0\F1 in `pending_records`; "Allow again" lifts and re-adds; Q-84 also requires `removal/1` | T-R4D20-rollback-added-folder, T-R4D20-rollback-dropped-folder |
| R4D-22 platform proof claim | Q-69, contracts §3.3, T-C11 and T-S17: offline synthetic fixtures only; native NTFS/APFS NOT RUN, a future obligation | T-C11 (offline) |

## Phase C ownership (contracts.md §9)

| Step | Packet | Files |
|---|---|---|
| 1 | **R4-core-contracts** | core `Cargo.toml`, `Cargo.lock`, `records.rs`, `ids.rs`, `toc.rs`, `paths.rs`, new `authority.rs`, new `kinds.rs`, new `ledger.rs`, new `hooks.rs`, `layout.rs`, `admit.rs`, `target.rs`, `plan.rs`, `exec.rs`, `exec_tests.rs`, `status.rs`, `view.rs`, `explain.rs`, `lib.rs`, `testkit.rs`, stubs `removal.rs`, `settings.rs`, `reject.rs`, `choice.rs`, `decide.rs`, `retire.rs`, new `fixtures\contracts\`; adapters `provider.rs`, `fs_install.rs`, `fs_safe.rs`, `testkit.rs`, new `authority_store.rs`; runtime `service.rs`, `effects.rs`, `control.rs`, `activity.rs`, `lib.rs`, stubs `removal_cycle.rs`, `settings_cycle.rs`, `rollback_flow.rs`, `decisions.rs`; `tests\required-cases.toml` |
| 2 | **R4-core-removal** | core `removal.rs`, new `removal_tests.rs`; runtime `removal_cycle.rs`, `detail.rs` |
| 2 | **R4-core-settings** | core `settings.rs`, new `settings_tests.rs`, `reject.rs`, new `reject_tests.rs`; adapters new `wtf.rs`, new `settings_store.rs`; runtime `settings_cycle.rs`, `rollback_flow.rs` |
| 2 | **R4-core-ops** | core `choice.rs`, new `choice_tests.rs`, `decide.rs`, new `decide_tests.rs`, `retire.rs`, new `retire_tests.rs`; runtime `decisions.rs` |
| 3 | **R4-notify-diag** | core new `notice.rs`, `update.rs`, `diagnostics.rs` (+ their `lib.rs` lines); adapters new `notify.rs`, `update_http.rs`, `winproc.rs`, `localstate.rs`, adapters `Cargo.toml`, `Cargo.lock`; runtime new `notices.rs`, `updates.rs`, `diag.rs` |
| 3 | **R4-app** | every file of `crates\wow-sync-app` |

Each step-2 packet appends only under its own section of
`tests\required-cases.toml`. `docs\runs\run-004\PLAN.md` still lists the
old three phase C packets; adding R4-core-contracts there is the
orchestrator's change.

## PENDING OWNER (conservative defaults, not decided)

Q-55, Q-57, Q-58, Q-60, Q-61 (newly pending: concurrent unknowing mutual
retirement retires both), Q-63, Q-64, Q-77, Q-78, Q-81, Q-82, Q-84, Q-85,
Q-86, Q-87. Q-64 and Q-81 keep the deviations the recheck accepted
provisionally; they remain PENDING OWNER.

## Verification

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` (worktree above) | **PASS**: fmt, clippy, test, required-cases, deny, markdown-links; overall PASS, exit 0 |

The offline tier only proves the docs change did not break the repository
build and tests. It does not validate any design rule.
