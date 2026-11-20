# R4-integrate: phase C step 2b (wire step-2 behaviour into the running service)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-integrate`, on branch
  `run004/R4-integrate`, created from `main` at `c0d6a93` (C1 plus all three
  step-2 packets merged).
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** none. It runs alone; step 3 (R4-app, R4-notify-diag,
  R4-mac-kit) starts after it merges.

## Why this packet exists

contracts.md §9.1 gave R4-core-contracts the `service.rs` cycle skeleton
"calling the hooks in order", every `control.rs` command and query variant,
and `target::compose` per-folder constraint composition (§4.4). All three
step-2 workers report that their hooks are implemented and unit-tested but
never called: `service.rs`, `control.rs` and `target::compose` don't reach
removal, settings, decisions or retirement. So the behaviour can't be seen
through the running service, and R4-app has nothing to drive. This packet
closes that C1 obligation and the cross-file follow-ups that step-2 workers
were not allowed to make. It adds no product scope.

## Authority

- [contracts.md](../../../design/contracts.md) §4.1–§4.4 (`compose`,
  constraint composition), §9.1 rows for `target.rs`, `ledger.rs`,
  `service.rs`, `control.rs`, `lib.rs`, §9.4 (hook traits; signatures are
  frozen, so don't change them), §10.
- [removal.md](../../../design/removal.md) §1.6 (cycle order, steps 3a/3b/6).
- [settings.md](../../../design/settings.md) §3.3 (revision parents may mix
  capture kinds, e.g. a push followed by auto) and
  [rollback.md](../../../design/rollback.md).
- [ops.md](../../../design/ops.md) §1–§2.
- D-21 (Q-49/Q-50 exactly), D-23 (UQ-1, UQ-2), D-25 (Q-77 has no approved
  value: real-data settings sync stays disabled; keep the test-only cap).
- Step-2 evidence, read in full, especially each "deferred" list:
  [R4-core-removal](../evidence/R4-core-removal.md),
  [R4-core-settings](../evidence/R4-core-settings.md),
  [R4-core-ops](../evidence/R4-core-ops.md).

## Files you own

- Runtime: `service.rs`, `control.rs`, `activity.rs`, `lib.rs`,
  `Cargo.toml`, plus the step-2 runtime files `removal_cycle.rs`,
  `detail.rs`, `settings_cycle.rs`, `rollback_flow.rs`, `decisions.rs`.
- Core: `target.rs`, `ledger.rs`, `explain.rs`, `status.rs`, plus the step-2
  core files `removal*.rs`, `settings*.rs`, `reject*.rs`, `choice*.rs`,
  `decide*.rs`, `retire*.rs`.
- Workspace `Cargo.lock` (only for adding `serde_json` to the runtime crate,
  already a workspace dependency; no new third-party crate).
- A new section of `tests\required-cases.toml`: add
  `# >>> section: R4-integrate` / `# <<< section: R4-integrate` immediately
  after the R4-core-ops section's closing marker. Don't change any other
  section or marker.
- Evidence: `docs\runs\run-004\evidence\R4-integrate.md`.

Don't touch the app crate, adapters, proto, xtask, notice/update/diagnostics
modules or golden fixtures. If you need one, stop and report it. Don't change
a frozen §9.4 hook signature; if one is wrong, stop and report it.

## Work (in priority order)

1. **Cycle wiring.** `service.rs` calls the hooks in removal.md §1.6 order
   (including removal steps 3a/3b/6 and the settings plan), honouring the
   existing admission barrier, WoW-running block and retirement barrier.
   `target::compose` applies per-folder constraint composition (§4.4) using
   `ConstraintHooks`. Add the removal variants to `PackageTarget`,
   `Explanation` and `PackageStatus` that the removal evidence lists.
2. **Control routing.** Every §9.1 command/query variant in `control.rs`
   routes to `removal_cycle`, `settings_cycle`, `rollback_flow` and
   `decisions`, including decide, keep/resume, retire, rollback and
   allow-again, settings master/exception/push, and `SettingsHistory`.
   Remove `#![allow(dead_code)]` from `decisions.rs` once it is used.
3. **Ledger bug:** `ledger.rs` must accept mixed-kind revision parents per
   settings.md §3.3. Add a test that fails on the old behaviour, and the
   missing ledger-level tests for `policy_heads` and `graph_from_ledger`.
4. **Persistence:** persist the ops kept queue and the settings
   epoch/seq/capture-kind state across restarts (use `serde_json`).
5. **Rollback flow:** build `RollbackInput` from live scans, wire the
   allow-again lift flow and the offline-resurrection guard.
6. **Cross-packet items:** the removal record for the smaller-copy keep;
   settings-cleanup op planning for removals; T-R4D21-readd-over-rollback;
   the writer's `TooLarge` split into parts; the Ambiguous,
   PartialCandidate and ChoicesDiffer item builders.
7. **A-38 controls:** run the 12 settings controls that R4-core-settings
   did not run, plus one each for the new wiring (a removal not applied
   through the service cycle, an unrouted decide command, a retired computer
   publishing through the cycle).

Each item needs a runtime-level (service or control) test that exercises it
end to end on synthetic data, including the negative case. If an item can't
be finished, mark it DEFERRED with the exact file and change it needs. Don't
skip silently.

## Rules specific to this packet

- Commit after each work item compiles and passes tests. Apply and revert
  mutations only with the edit tool on committed code, and confirm
  `git diff --exit-code` afterwards. Never use `git checkout --`,
  `git restore`, `git reset --hard` or `git stash` on uncommitted work
  (a step-2 worker lost its work this way).
- Keep existing tests passing. Don't weaken assertions.

## Gate and evidence

- **Verification:** offline and native verify pass on the final tree;
  `git diff --name-only main..HEAD` lists only your files and evidence; the
  required-cases section check passes.
- **Evidence:** `docs\runs\run-004\evidence\R4-integrate.md`, covering:
  - each work item: done or DEFERRED;
  - each test ID with its result;
  - the A-38 controls table;
  - test counts per crate, kept consistent throughout the file;
  - the handovers to R4-app (which control variants the GUI must drive).
- **Commits:** prefix `run004(R4-integrate):`, with the Copilot trailer.
  Don't push.
