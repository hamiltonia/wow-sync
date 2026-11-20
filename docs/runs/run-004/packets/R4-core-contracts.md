# R4-core-contracts: phase C step 1 (foundation)

- **Role / model:** implementation, claude-sonnet-5.5 (D-04, PLAN §3, C1).
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-core-contracts`, on branch
  `run004/R4-core-contracts`.
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Merge:** this packet merges alone, before any step-2 packet starts.

## Authority

Read these first. They are authoritative; this packet only points into them.

- The design review passed:
  [R4-design-recheck4](../evidence/R4-design-recheck4.md).
- The D-25 owner answers are recorded in
  [questions.md](../../../design/questions.md).
- [contracts.md](../../../design/contracts.md) as a whole, especially:
  - §9.1, the exact file set you own;
  - §9.4, the extension-point traits, whose signatures freeze at your merge;
  - §9.5, the gate;
  - §10, the contract tests.
- The feature designs, which define the DTOs and V-KIND rules you
  implement once:
  - [removal.md](../../../design/removal.md);
  - [settings.md](../../../design/settings.md);
  - [rollback.md](../../../design/rollback.md);
  - [ops.md](../../../design/ops.md).

## Goal

Implement every shared type, record kind, identity, journal shape, family
validation rule, admission rule and target-composition rule exactly once.
Create neutral, compiling stubs for every step-2 and step-3 module, so
those packets only fill in behaviour behind the §9.4 hooks.

Slice behaviour must not change. With the neutral hooks, the existing
test suite still passes unmodified.

## Files you own (exactly contracts.md §9.1)

You may change only these files, plus your evidence:

- **core** (`crates\wow-sync-core\`):
  - `Cargo.toml`;
  - `src\records.rs`, `ids.rs`, `toc.rs`, `paths.rs`, `layout.rs`,
    `admit.rs`, `target.rs`, `plan.rs`;
  - `src\exec.rs`, `exec_tests.rs`, `status.rs`, `view.rs`, `explain.rs`,
    `lib.rs`, `testkit.rs`;
  - new `src\authority.rs`, `kinds.rs`, `ledger.rs`, `hooks.rs`,
    `decision.rs`;
  - new `fixtures\contracts\`.
- **core stubs** (neutral; handed over at merge):
  - `src\removal.rs`, `removal_tests.rs`;
  - `src\settings.rs`, `settings_tests.rs`;
  - `src\reject.rs`, `reject_tests.rs`;
  - `src\choice.rs`, `choice_tests.rs`;
  - `src\decide.rs`, `decide_tests.rs`;
  - `src\retire.rs`, `retire_tests.rs`;
  - `src\notice.rs`, `update.rs`, `diagnostics.rs`.
- **adapters** (`crates\wow-sync-adapters\src\`):
  - `provider.rs`, `fs_install.rs`, `fs_safe.rs`, `testkit.rs`, `lib.rs`;
  - new `authority_store.rs`;
  - stubs `wtf.rs`, `settings_store.rs`, `notify.rs`, `update_http.rs`.
- **runtime** (`crates\wow-sync-runtime\src\`):
  - `service.rs`, `effects.rs`, `control.rs`, `activity.rs`, `lib.rs`;
  - stubs `removal_cycle.rs`, `settings_cycle.rs`, `rollback_flow.rs`,
    `decisions.rs`, `notices.rs`, `updates.rs`, `diag.rs`.
- **Root:**
  - `Cargo.lock`, only for the NFC dependency (contracts §3.3);
  - `tests\required-cases.toml`.

If you need any other file, stop and report. That is a contract change.

**Amendment 1 (orchestrator).** `crates\wow-sync-core\src\arch.rs` is
added for one change only: add `"unicode-normalization"` to the pinned
core dependency list in `arch::tests::core_sources_use_no_os_io_or_clock`.
That is the dependency contracts §3.3 calls for. The rest of the guard
stays unchanged.

## Required details

1. **Modules and types.**
   - Each `lib.rs` (core, adapters, runtime) declares **every** step-1,
     step-2 and step-3 module listed in §9.1–§9.3. No later packet edits
     a `lib.rs`.
   - Every type named in a §9.4 signature, or exchanged for decisions,
     lives in `decision.rs` (§9.1 row).
   - `AllHoldCause::RetirementUnresolved` (R4D-31) goes in `status.rs`. It
     is local only and never written to any shared or wire file.
2. **Stubs.**
   - Stubs implement the §9.4 traits with neutral behaviour: no feature
     effect, and slice behaviour preserved.
   - Stub test modules are empty but compile.
   - Adapter stubs add no dependency.
3. **`tests\required-cases.toml`.**
   - Keep your own block.
   - Add one empty section per later packet, delimited by
     `# >>> section: <packet>` and `# <<< section: <packet>` lines.
   - The later packets are R4-core-removal, R4-core-settings, R4-core-ops,
     R4-notify-diag, R4-app and R4-mac-kit.
   - Confirm that `cargo xtask verify` still parses the file.
4. **Tests you implement** (§9.1 and §10, rows not marked with another
   owner):
   - T-C01 to T-C14, plus T-C06b and T-C06c;
   - T-R4D1-* and T-R4D4-*;
   - T-R4D15-vret-predicate (extended);
   - T-R4D16-*;
   - T-R4D23-same-release-table and T-R4D23-unknown-waits;
   - T-R4D24-no-hybrid;
   - T-R4D25-partial-retirement-dag;
   - T-R4D28-exact-source-unrelated-addon and T-R4D28-regrouped-exclusion.

   T-C11 is offline-fixture only; native NTFS/APFS is NOT RUN (R4D-22).
   List every test in your section of `tests\required-cases.toml`. Record
   A-38 negative controls for the controls §10 names (T-R4D23 compares
   common keys only or unions source keys; T-R4D24 uses per-folder
   selection) and for a representative T-R4D25 control.
5. **`same_release`** has two forms (§4.2, R4D-28):
   - source against a recorded entry compares over the entry's scope only;
   - entry against entry compares over the union of their scopes.

   An `Unknown` result never counts as equal.
6. **Recovered chain summary (R4D-32).** If the foundation code touches
   it, follow rollback §3.1's four-branch algorithm. A proven usable copy
   is never held back by an unknown redundant copy.
7. **D-25 product gates.**
   - Q-77 has no approved value. Define `MAX_SETTINGS_FILE_BYTES` as a
     named gate that keeps real-data settings sync disabled. Tests may
     use a test-only value.
   - Implement no product behaviour beyond the contracts.
8. **Golden fixtures.** Commit a golden fixture for every kind, part,
   index chunk, journal and point (`/1` and `/2`). Fixture contents must
   be synthetic only.

## Gate (§9.5) and evidence

- **Verification:**
  - `cargo xtask verify --tier offline` passes;
  - `cargo xtask verify --tier native` passes;
  - every stub compiles;
  - the existing slice tests pass unmodified.
- **Scope check:** `git diff --name-only main..HEAD` lists only the files
  above plus your evidence.
- **Evidence** goes in `docs\runs\run-004\evidence\R4-core-contracts.md`.
  Include:
  - a table mapping each §9.1 row to the files and types delivered;
  - the frozen §9.4 signatures;
  - each test ID with its result;
  - the A-38 controls;
  - the handover list (stub, step-2/3 owner);
  - anything deferred or BLOCKED.
- **Commit** with the prefix `run004(R4-core-contracts):` and the Copilot
  trailer. Don't push.
