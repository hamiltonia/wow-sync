# R2-review: run-002 integration review (E-06 / E-08)

- Worker: R2-review, role integration review, model claude-opus-5.5 (D-04 / D-17).
- Checkout: `wow-sync.worktrees/R2-review`, branch `run002/R2-review`.
- Reviewed: `main` at `de6ffc9` (Merge run002/R2-fix).
- Environment: Windows 11 x64, Rust 1.100.0, cargo-deny 0.20.2.
- Scope: I only read the code and evidence. No app, GUI or memory process was
  launched. The S-forever worktree was not read.
- R2-recheck (independent re-check of R2-fix) was running at the same time.
  **Its results are pending and are not reflected here.**

## Commands run

| Command | Result |
| --- | --- |
| `cargo xtask verify --tier offline` at `de6ffc9` | **PASS**: fmt, clippy, test, required-cases, deny and markdown-links all PASS; exit 0 |
| `cargo xtask verify --tier offline` with this file staged | **PASS**: exit 0 (re-run before commit) |
| GUI, memory and native tier | **NOT RUN** (outside this packet) |

## Verdict

**Accept with follow-ups.** This accepts the run-002 synthetic-only vertical
slice as an engineering checkpoint only. It does **not** accept the slice for
any real WoW data, and it is **not** a release claim.

The acceptance depends on the following gating follow-ups:

- MJ-1 is fixed and has a test.
- The memory gate (MJ-2) stays BLOCKED until the owner has reviewed incident V1
  and the harness has been fixed.
- R2-recheck confirms the R2-fix GUI behaviour.
- The A-38 negative controls for M1 to M5 are actually demonstrated (MJ-3).

If R2-recheck reports a FAIL, this verdict becomes **reject** until that FAIL is
fixed.

## Findings

No blockers were found in the code that was reviewed.

### Major

**MJ-1: reconfirmation checkboxes can appear already ticked**

- Location: `crates/wow-sync-app/src/ui.rs`.
  - The fields `identity_ok` and `scope_ok` are declared at lines 30-31 and
    initialised at lines 68-69.
  - They are only ever toggled, at lines 297, 310, 374 and 385.
  - They are never reset after `enroll()` or `reconfirm()` (line 96) sends its
    command.
- Why this matters:
  - Suppose a user enrolls in a session, and later in the same session the
    enrollment confirmation becomes stale.
  - The reconfirm panel then shows both checkboxes already ticked.
  - A single click on "Confirm and continue" reconfirms, without the user giving
    fresh consent tied to the new identity evidence.
  - This weakens the guided-identity / safe-scope rule (AGENTS.md, A-46, R-03).
  - There is no e2e test of `SliceService::reconfirm` (`service.rs:526`) or of
    the path from `NeedsReconfirmation` to `status.reconfirm`. R2-fix says the
    GUI was NOT RUN.
- Fix:
  - Clear both flags whenever a command is sent.
  - Clear both flags when the reconfirm panel first appears for a new
    confirmation request.
  - Add a runtime e2e test of reconfirm.
  - Have the GUI checked independently.

**MJ-2 (carried over, BLOCKED): memory gate A-29 is unresolved**

- Location: R2-check V1; `spikes/gpui-pin/measure.ps1`; PLAN section 6, item 6.
- Why this matters:
  - R2-check's measurement run may have killed up to 8 unrelated processes.
  - A reading of 763 MiB in that run cannot be attributed to a cause.
  - R2-slice's 12.4 MiB figure comes from a single run. It used the same
    ParentProcessId-traversal method that has the flaw.
  - R2-check's corrected repeat gave about 16 MiB. That is promising, but it is
    not gate evidence.
- Fix:
  - The owner reviews incident V1.
  - Validate process creation time in the harness.
  - Re-measure in at least 3 runs.

**MJ-3: the R2-fix fixes are not shown to fail without the fix (A-38)**

- Location: `docs/runs/run-002/evidence/R2-fix.md`.
- Why this matters:
  - Item 7 of the R2-fix packet required showing that each new test fails when
    its fix is removed. R2-fix honestly reports this as NOT RUN.
  - Without that evidence, the tests for M1 to M5 could pass vacuously.
- Fix: in R2-recheck or run-003, revert each fix locally and record the test
  failing.

### Minor

**mi-1: consent for recognized clients is enforced only in the GUI**

- Location:
  - `crates/wow-sync-core/src/variant.rs:415` (`confirm_variant`). Around
    line 450 it accepts a Recognized+Supported client with `scope=false`.
  - The consent gate exists only in `ui.rs:126`.
- Why this matters: this is policy living outside core (E-08 / A-43).
  `SliceService::enroll` will enroll a recognized client without consent.
- Fix: add a consent predicate in core and an e2e test.

**mi-2: some rendering and policy code is still in runtime**

- Location: `crates/wow-sync-runtime/src/service.rs`. It still contains:
  - English wording in `block_text` and `package_result_text` (lines 1230 and
    1245).
  - The literal headline "Something needs your help" (line 931).
  - The decision between "interrupted" and "needs help" based on journal phase
    (lines 836-845).
  - The choice of its own newest state and `seq` (`max_by_key`, line 1153).
  - The `applied_here` state, which is kept only in memory and is lost on
    restart.
- Why this matters: R2-fix's statement "Policy lives in wow-sync-core" is
  overstated (A-43).
- Fix: move these into `wow-sync-core::status` and `plan`.

**mi-3: a changed product or folder is offered a reconfirm that cannot succeed**

- Location: `variant.rs:474` (`confirm_enrolled`, the check at line 493) and
  `variant.rs:509` (`reconfirm_variant`).
- Why this matters:
  - When the product or folder has changed, the status becomes
    `NeedsReconfirmation`, so the GUI offers a reconfirm.
  - `reconfirm_variant` then always refuses with `VariantChanged`.
  - Section 2 of domain.md says this case is `NeedsHelp(VariantChanged)`.
- Fix: map this case to NeedsHelp and add a test.

**mi-4: `NotSynced` does not affect the headline**

- Location: `crates/wow-sync-core/src/status.rs:211` and `target.rs`
  (NotSynced).
- Why this matters:
  - The headline says "Everything is up to date" while a folder is blocked.
  - Section 2.1 of domain.md models a blocked folder as a decision scoped to
    the addon.
- Fix: decide the intended behaviour (owner question O-N1), then align the code
  and the design.

**mi-5: details of a blocked folder are still published**

- Location: `crates/wow-sync-core/src/plan.rs:143-144`.
- What is published: the blocked folder's name, tree hash, file count and
  bytes. File contents are not uploaded, and peers get NeedsDecision.
- Why this matters: it publishes a fingerprint of a folder the user did not
  approve. publication.md allows this, but it may not be what the owner
  intends.
- Fix: owner question O-N1.

**mi-6: `inventory_complete` has changed meaning**

- Location: `crates/wow-sync-core/src/plan.rs:143`.
- Why this matters:
  - The value is now false whenever any folder is blocked.
  - publication.md defines it as "could not be read".
- Fix: update publication.md, or use a separate field.

**mi-7: a build-version change does not trigger reconfirmation**

- Location: `variant.rs:474` (`confirm_enrolled` ignores `build_version`).
- Why this matters: this is acceptable under Q-21, but it was the
  implementer's choice rather than the owner's.
- Fix: owner question O-N2.

**mi-8: running-game detection is broader than needed**

- Location: `wow-sync-adapters` winproc `is_possible_client`.
- Why this matters:
  - Any `wow*.exe` process that cannot be inspected blocks as Unknown. For
    example, an elevated updater/manager would block.
  - This is conservative under Q-16, but it is in tension with the rule that an
    idle updater's presence alone must not block.
- Fix: narrow the match to known client executables, and add a test.

**mi-9: project records are out of date**

- Location: HANDOFF.md, USAGE.md and `docs/design/tests.md:148`.
- What is wrong:
  - The HANDOFF phase table still shows E as pending, without the R2-check FAIL
    or the R2-fix merge.
  - USAGE.md has no rows for R2-check or R2-fix.
  - tests.md says provider tests "print BLOCKED and return", but they now
    panic.
- Fix: the orchestrator updates these records, and the design worker updates
  tests.md.

**mi-10: some recovery points are not shown in status**

- Location: `service.rs` `snapshot()`.
- Why this matters: recovery points are listed only for packages that are in
  the current target. Points for other packages are not shown, which conflicts
  with "visible temporary exceptions".
- Fix: list all recovery points.

**mi-11: two smaller gaps**

- Location: `crates/wow-sync-app/src/backend.rs` and the adapter's
  `NotPlainFile` error.
- What is wrong:
  - The m4 de-duplication in `backend.rs` has no test.
  - The raw adapter wording for `NotPlainFile` can reach the user.
- Fix: add a unit test and plain-language wording.

## Overclaims in the evidence

- **R2-slice:** marked G1 to G9 all PASS. R2-check later found G1 to G5 FAIL,
  and the author's own notes for G1, G4 and G6 admitted deviations.
- **R2-slice:** the memory figure (12.4 MiB) is one run with the flawed method
  (see MJ-2).
- **R2-fix:** "Policy lives in wow-sync-core; runtime/app only call it" is
  partly overstated (see mi-1 and mi-2).
- **R2-fix:** the A-38 requirement that each test fails without its fix is not
  demonstrated. R2-fix honestly says NOT RUN (see MJ-3).
- **HANDOFF / USAGE:** both are out of date (see mi-9).
- **Accurately stated:**
  - R2-core's test counts.
  - R2-adapters' provider results, honestly labelled FAIL/BLOCKED.
  - R2-check's G6 PASS, with m4 noted.
  - R2-pin's results with its stated limits (ARM64 link and macOS BLOCKED).

## AGENTS.md easy-to-get-wrong rules checked against the slice code

| Rule | Result |
| --- | --- |
| Variant / account boundary | OK: the group product is checked by `check_join`; peers with a different product are filtered out in `compute_target` |
| "Forever" is not Classic Era | OK: the display label is "World of Warcraft: Forever (wowf)", with a test |
| Enrolling an unfamiliar client | Core is OK: two confirmations plus digest checks, and M1 is fixed. **Gaps:** the GUI pre-tick (MJ-1) and consent for recognized clients enforced only in the GUI (mi-1) |
| Retired computers | Not in the slice |
| Newer compatible release; ambiguity leads to a decision | OK: no timestamps are used (negative control 10) |
| Settings master / exceptions | Not in the slice |
| Additions / removals propagate | Additions: OK. Removals: not implemented (Q-2); follow-up |
| Rollback | Not in the slice |
| Three states | OK: own states are pruned to 3; recovery is 2 plus pinned points. Partial visibility gap (mi-10) |
| WoW running blocks game-file changes | OK: activity is checked before every live rename. Detection is broader than needed (mi-8) |
| Dropbox publication is not proof of apply | OK: confirmation requires the peer's Applied report plus a matching tree (G7) |
| Recovery through the GUI | Interrupted operations recover through "Sync now". There is no GUI for needs_help (Q-25); follow-up |
| Memory <= 100 MiB; sync starts within 10 s | Memory: BLOCKED (MJ-2). Start: OK, the backend forces an immediate cycle |

## Required follow-ups for run-003

1. Fix MJ-1 and add a runtime e2e test of reconfirm.
2. Fix mi-3 (map a changed product or folder to NeedsHelp).
3. Run the GUI re-check of the R2-fix changes, if R2-recheck has not already
   covered it.
4. Demonstrate the A-38 negative controls for M1 to M5.
5. Re-measure memory over at least 3 runs with a harness that validates process
   creation time, after the owner has reviewed V1.
6. Move the remaining runtime rendering and policy into core (mi-1, mi-2), and
   add a core consent predicate.
7. Make the xtask `native` tier executable (Q-17).
8. Resolve the provider tests that are BLOCKED (Q-23).
9. Close the report-after-recovery gap: a crash between apply and publish loses
   the apply report.
10. Implement the `s_addon` real-metadata table, which is still missing.
11. Design and implement, in later runs:
    - removal and uninstall;
    - a GUI for needs_help;
    - rollback and retire;
    - settings.
12. Run on macOS and ARM64 (CI).
13. Update the records: HANDOFF.md, USAGE.md, publication.md (mi-6) and
    tests.md (mi-9).

## Owner decisions pending (consolidated)

**From PLAN section 6:**

- Held:
  - #1: the S-forever report.
  - #5: the Dependabot alerts.
  - #8: git gc. Note that R2-slice already ran gc; that is an incident.
- Provisional, need confirmation:
  - #2: the GPUI pin v1.28.1.
  - #3: the D-12 screen-reader policy.
  - #4: `deny.toml`.
  - #6: the memory measurement method.
  - #7: the R-11 residual race.

**From the design questions (`docs/design/questions.md`):**

- Q-1, Q-2, Q-3, Q-4, Q-5, Q-6, Q-8, Q-9, Q-10, Q-12, Q-14, Q-16, Q-18, Q-20,
  Q-21, Q-22, Q-23, Q-24, Q-25.
- Q-3 needs the owner to run a client check.
- Q-23 needs a file the owner prepares that exists only online.

**Raised by this review:**

- **O-N1:** how blocked folders should be handled.
  - Should they be NotSynced or an addon decision?
  - Should they affect the headline?
  - Should their details be published?
  - (mi-4, mi-5)
- **O-N2:** should a build-version change require reconfirmation? (mi-7)
- **O-N3:** confirm that consent for recognized clients belongs in core.
  (mi-1)

**Incidents needing owner review:**

- V1: possible termination of unrelated processes during R2-check's memory run.
- R2-slice's gc and history rewrite.
- R2-slice's revert of `47cccb3`.

## Pending

- R2-recheck results: **pending**. They are not reflected in this review.
