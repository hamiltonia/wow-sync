# R4-core-ops: phase C step 2 (decisions, code choice, retirement)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-core-ops`, on branch
  `run004/R4-core-ops`. It is created from `main` after R4-core-contracts
  merges.
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** runs alongside R4-core-removal and R4-core-settings.
  The file sets are disjoint.

## Authority

- [ops.md](../../../design/ops.md) §1–§2 and the core and runtime rows of
  §7.
- [contracts.md](../../../design/contracts.md):
  - §5, the code-choice family;
  - §1.10, retirement;
  - §9.2 (your row and the shared-append rule);
  - §9.4 `DecisionHooks`, `RetireHooks` and `ConstraintHooks`
    (`live_constraints` and `excluded` for keep/resume);
  - §10 rows owned by R4-core-ops.
- D-21: Q-49 (an unconsidered newer version raises
  `NewVersionAfterDecision`; the kept version stays) and Q-50 (resume
  keeps earlier exclusions). **Preserve both exactly.**
- D-23: UQ-1 ("Review needed" headline) and UQ-2 (typed scope).
- D-25:
  - Q-60: a retired computer stops publishing and applying, and rejoins
    only explicitly;
  - Q-61: mutual retirement retires both.
- [R4-core-contracts evidence](../evidence/R4-core-contracts.md): the
  frozen signatures and the stub handover.

## Files you own (exactly contracts.md §9.2)

- `crates\wow-sync-core\src\choice.rs`, `choice_tests.rs`, `decide.rs`,
  `decide_tests.rs`, `retire.rs`, `retire_tests.rs`
- `crates\wow-sync-runtime\src\decisions.rs`
- Your section of `tests\required-cases.toml`, only between
  `# >>> section: R4-core-ops` and `# <<< section: R4-core-ops`.

Don't touch anything else, including any `lib.rs`, `Cargo.toml`,
`Cargo.lock` or step-1 file. If you need a step-1 change, stop and report
it.

## Work

- **Code choice** (contracts §5.3): keep, resume and exclusions, using
  `same_release` source against the entry's scope.
- **`decide`:** check freshness (T-C15) and support the typed scope
  (UQ-2).
- **Retirement:** the retirement command flow and master reassignment.
  The barrier itself is step-1 `authority.rs`.
- **Held self (R4D-31):** while this computer's own retirement is
  Unresolved, show "Waiting for group history" and keep its decisions.
- **Runtime:** the decision routing in `decisions.rs`.
- **Tests:**
  - ops.md §7 core and runtime tests, including T-R4D31-held-self, the
    user-facing T-R4D25 checks, T-45-* and the decision tests;
  - T-R4D21-partial-choice-over-choice;
  - T-R4D23-keep-resume-shared-component;
  - T-C15-decide-fresh.

  GUI-only rows (for example parts of T-26-*) go to R4-app. Notification,
  update and diagnostics rows (T-27-*, T-28-*, T-33-*) go to
  R4-notify-diag. List those handovers in your evidence.
- Record A-38 negative controls for: Q-49 pinning, Q-50 inherited
  exclusions, stale-evidence `Changed`, and a retired computer not
  publishing.

## Gate and evidence

- **Verification:**
  - offline and native verify pass;
  - `git diff --name-only main..HEAD` lists only your files plus evidence;
  - the required-cases section check passes.
- **Evidence** goes in `docs\runs\run-004\evidence\R4-core-ops.md`.
  Include each test ID with its result, the A-38 controls, the
  ACCEPTANCE/SPEC links, any deferred or BLOCKED items, and the handovers.
- **Commit** with the prefix `run004(R4-core-ops):` and the Copilot
  trailer. Don't push.
