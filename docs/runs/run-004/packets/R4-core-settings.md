# R4-core-settings: phase C step 2 (settings sync and rollback)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-core-settings`, on branch
  `run004/R4-core-settings`. It is created from `main` after
  R4-core-contracts merges.
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** runs alongside R4-core-removal and R4-core-ops. The
  file sets are disjoint.

## Authority

- [settings.md](../../../design/settings.md), including §11 Test
  obligations.
- [rollback.md](../../../design/rollback.md), including §10 Test
  obligations.
- [contracts.md](../../../design/contracts.md): §9.2 (your row and the
  shared-append rule), §9.4 `SettingsHooks` and `ConstraintHooks`
  (`rejected`), §4.2 `same_release` (both forms), and §10 rows owned by
  R4-core-settings.
- D-25:
  - Q-77: no size value is approved, so real-data settings sync stays
    disabled by the step-1 gate;
  - Q-78: a rollback blocks exact content;
  - Q-81: only an explicit lift or choice ends a rollback;
  - Q-82: no settings-checkpoint rejection;
  - Q-84: block group rollback while any peer lacks support.
- [R4-core-contracts evidence](../evidence/R4-core-contracts.md): the
  frozen signatures and the stub handover.

## Files you own (exactly contracts.md §9.2)

- `crates\wow-sync-core\src\settings.rs`, `settings_tests.rs`, `reject.rs`,
  `reject_tests.rs`
- `crates\wow-sync-adapters\src\wtf.rs`, `settings_store.rs`
- `crates\wow-sync-runtime\src\settings_cycle.rs`, `rollback_flow.rs`
- Your section of `tests\required-cases.toml`, only between
  `# >>> section: R4-core-settings` and `# <<< section: R4-core-settings`.

Don't touch anything else, including any `lib.rs`, `Cargo.toml`,
`Cargo.lock` or step-1 file. You add no dependencies. If you need a
step-1 change, stop and report it.

## Work

- **Settings** (settings.md):
  - WTF scan and capture;
  - the local blob, manifest and `SettingsLocal` store;
  - the target (§4.2), epoch, revisions and policy (master, per-addon
    exceptions, explicit peer push);
  - routine non-master writes yield to master changes, with recovery
    preserved;
  - three states per unit;
  - the settings history query.
- **Rollback** (rollback.md):
  - rejection and lift;
  - the recovered chain, using the §3.1 four-branch algorithm;
  - R4D-32: a proven usable copy is never held back by an unknown
    redundant copy;
  - folder-changing rollback (R4D-20);
  - the outcome comes from the actual R0/S0 restore (R4D-27);
  - the offline-resurrection guard.
- **Tests:**
  - settings.md §11 and rollback.md §10 at core, adapter or runtime level,
    including:
    - T-R4D23-added-folder-rollback and T-R4D23-dropped-folder-rollback;
    - T-R4D21-choice-over-rollback;
    - T-R08 and T-R09;
    - T-R4D29-empty-recovered-copies and T-R4D30-reassigned-author;
    - T-R4D4-nine-policy-heads and the isolated T-R4D19 epoch controls;
    - a mixed safe-plus-unknown recovered-copy fixture (R4D-32).
  - GUI-only checks go to R4-app: list them in your evidence as handed
    over.
- **Synthetic data only.** Never read the real WTF folder.
- Record A-38 negative controls for: master yield, no merge of competing
  choices, rejection resurrection, exact-content match and the
  three-state limit.

## Gate and evidence

- **Verification:**
  - offline and native verify pass;
  - `git diff --name-only main..HEAD` lists only your files plus evidence;
  - the required-cases section check passes.
- **Evidence** goes in `docs\runs\run-004\evidence\R4-core-settings.md`.
  Include each test ID with its result, the A-38 controls, the
  ACCEPTANCE/SPEC links, any deferred or BLOCKED items (for example,
  real-data settings sync blocked by Q-77), and the GUI handovers.
- **Commit** with the prefix `run004(R4-core-settings):` and the Copilot
  trailer. Don't push.
