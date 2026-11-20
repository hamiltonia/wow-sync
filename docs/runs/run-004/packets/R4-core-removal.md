# R4-core-removal: phase C step 2

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-core-removal`, on branch
  `run004/R4-core-removal`. It is created from `main` after
  R4-core-contracts merges.
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** runs alongside R4-core-settings and R4-core-ops. The
  file sets are disjoint.

## Authority

- [removal.md](../../../design/removal.md): the whole design, including
  §12 Tests.
- [contracts.md](../../../design/contracts.md): §9.2 (your row and the
  shared-append rule), §9.4 `RemovalHooks`, §4 and §10 (rows owned by
  R4-core-removal).
- D-21 (removal wins; conflicting removals merge), D-23 R3D-13 simplify,
  and D-25 (Q-55 explicit add-back, Q-57 local "Continue without it",
  Q-58 named folders only).
- [R4-core-contracts evidence](../evidence/R4-core-contracts.md): the
  frozen signatures and the stub handover.

## Files you own (exactly contracts.md §9.2)

- `crates\wow-sync-core\src\removal.rs`
- `crates\wow-sync-core\src\removal_tests.rs`
- `crates\wow-sync-runtime\src\removal_cycle.rs`
- `crates\wow-sync-runtime\src\detail.rs`
- Your section of `tests\required-cases.toml`, only between
  `# >>> section: R4-core-removal` and `# <<< section: R4-core-removal`.

Don't touch anything else. That includes every `lib.rs`, `Cargo.toml` and
`Cargo.lock`, and every step-1 file such as `kinds.rs`, `hooks.rs`,
`target.rs` and `decision.rs`. If you need a step-1 change, stop and
report it as a contract change.

## Work

- Implement the removal behaviour behind `RemovalHooks`:
  - observation, settle, merge and stale copies;
  - acks and re-add (`user_add_back`, including `ReinstallObserved`);
  - the removal cycle steps (removal.md §1.6);
  - absent-ack and cleanup planning (R4D-5);
  - unreadable-reference waiting, with the local "Continue without it"
    choice.
- Wire the runtime removal cycle, and the detail queries the GUI needs.
- Implement the tests:
  - removal.md §12 tests at core or runtime level;
  - contracts §10 rows owned by R4-core-removal, including
    T-R4D21-readd-over-rollback and the T-C14 removal rows.

  List them in your section. GUI-only checks go to R4-app: list them in
  your evidence as handed over.
- Record A-38 negative controls for the key behaviours: removal wins,
  conflicting removals merge, no resurrection by an offline peer, and
  explicit add-back.

## Gate and evidence

- **Verification:**
  - offline and native verify pass;
  - `git diff --name-only main..HEAD` lists only your files plus evidence;
  - the required-cases section check passes.
- **Evidence** goes in `docs\runs\run-004\evidence\R4-core-removal.md`.
  Include each test ID with its result, the A-38 controls, the
  ACCEPTANCE/SPEC links, any deferred or BLOCKED items, and the GUI
  handovers.
- **Commit** with the prefix `run004(R4-core-removal):` and the Copilot
  trailer. Don't push.
