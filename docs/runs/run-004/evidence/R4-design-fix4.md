# Evidence: R4-design-fix4 (bounded R4D-28 amendment)

- **Authority:** the owner said "continue" at the R4D-28 barrier. The
  orchestrator treats this as option 1 of
  [R4-design-recheck3](R4-design-recheck3.md): a bounded shared-contract
  correction plus one independent check, beyond the normal retry limit.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-design-fix4`
- **Branch:** `run004/R4-design-fix4`, created from `main` at `9e98862`
- **Environment:** Windows x64 owner workstation

This change is design only. No code, test, build or CI file changed, and
nothing is recorded as an owner decision. Q-49 and Q-50 keep their D-21
semantics unchanged, and every PENDING OWNER label is kept. There is no
wire change. The only new type is the local, non-wire enum variant
`AllHoldCause::RetirementUnresolved`, needed for R4D-31. The named tests
are design obligations: none is implemented or run.

## Finding → change → section

| Finding | Status | Change | Sections |
|---|---|---|---|
| R4D-28 (major) | Fixed as specified | `same_release` has two forms. **Source against a recorded entry** compares over the entry's scope only. The source is a complete inventory that answers every key of that scope (tree, known `Absent`, or `Unknown` for a folder that is not `Ok`). Its other keys are never in K. **Two recorded entries** keep the union rule; an `Unknown` between them is resolved only through actual copies. The users table labels the recovered chain as the entry-against-entry user. The writer formula, the exclude-scope paragraph, the resume filter and the hook comment all say "over the entry's scope". The new fixtures are added, and the truth-table test now covers both forms | contracts.md §4.2, §5.2, §5.3, §9.1 (tests), §9.4 (hooks), §10; rollback.md §3, §3.1, §10; ops.md §1.5, §7 |
| R4D-29 (minor) | Fixed | The recovered chain uses four ordered steps. An empty copy set gives `RecoveredUnavailable`; the chain never advances on it and the constraint is never dropped. A non-empty set where every copy is rejected advances the chain. A copy that cannot yet be compared makes the chain wait. Otherwise the constraint stands. The constraint is dropped only when every release on the chain is proven rejected | rollback.md §3.1, §10 (T-R4D29-empty-recovered-copies); contracts.md §4.2 users row |
| R4D-30 (minor) | Fixed | Three checks are kept separate. (1) Historical validity: the issuer must be authority under the revision's listed heads, and later policies never re-check it. (2) Current publication authority: only the current authority writes new `auto` revisions. (3) Policy-conflict hold: only when the reader's current heads disagree among themselves. Only `auto` revisions need a witness; `push`, `choice` and `restore` do not. `invalid(not_authority)` now refers to the listed heads | settings.md §3.3, §4.2 step 1, §11 (T-R4D30-reassigned-author and its A-38 control) |
| R4D-31 (minor) | Fixed | A proven-retired self and a held self share the publish/apply pause and the **Join again…** escape. A held self gets the "Waiting for group history" headline with no retirement date or "by" line, and the new variant `AllApplies(RetirementUnresolved)`. Its reserved decisions are kept unwritten, not discarded, unless the user cancels them explicitly. It resumes by itself if its retirement turns Void; if it turns Yes, it becomes the retired case. Decisions are allowed while held (a new §1.3 row) | ops.md §1.3, §1.6, §2.4, §7 (T-R4D31-held-self); contracts.md §1.10 rule 6 |
| R4D-26 (minor, residual) | Orchestrator to-do | Not edited (PLAN.md and packet files are owned by the orchestrator); see below | — |

## R4D-28 walk-throughs (source against entry, K = entry scope)

1. **Exact rejected source, with and without an unrelated addon** (T-R4D28-exact-source-unrelated-addon).
   - The rejected entry is `{core:U, extra:BAD}` over {core, extra}.
   - Source m1 holds `{core:U, extra:BAD}`, with addon `{z:Z}` either installed or absent.
   - K = {core, extra}. Both keys are known and equal, so the result is `Same` and m1 is rejected.
   - z is never in K.
2. **Exact recovered source** (same test).
   - The recovered entry is `{core:U}` over {core, extra}, so extra is `Absent` in the entry.
   - Source m0 holds core U and no extra folder (known `Absent`).
   - The result is `Same`: m0 is a real copy of the recovered release and is installed.
   - The result is the same with z installed or absent.
3. **Carried exclusion after regrouping** (T-R4D28-regrouped-exclusion).
   - The exclude entry is V3 `{a:3, b:U}` with scope {a, b}; the resume's `folders` are {a, b, c}.
   - The declined source `{a:3, b:U, c:C}`: K = {a, b}, so it is `Same` and never selected.
   - The kept source `{a:2, b:U, c:C}`: a differs, so it is `Different`, eligible and the target.
4. **Existing cases still hold.**
   - Added folder: R0 `{core:U}` against R1 over {core, extra} is `Different` (extra is `Absent` in R0, BAD in R1).
   - Dropped folder: R0 `{core:U, old:O}` is `Different` from R1 `{core:U}`.
   - Unstable folder in scope: a source whose extra folder is `Unstable` is `Unknown` and waits, then resolves once the folder is stable.
5. **Two recorded entries still use the union rule.**
   - X.recovered `{core:U}` over {core} against Y.rejected `{core:U, extra:BAD}` over {core, extra} gives `Unknown`.
   - The actual copies of X.recovered decide (rollback.md §3.1 steps 1–4).
   - If no copy exists, the result is `RecoveredUnavailable` (R4D-29).

The reviewer's `design_probes.py` still runs with exit 0 and prints the
same lines as before: "DESIGN FAIL R4D-28" for the formula it hard-codes
(the old union over the source's keys), and "PASS proposed
recorded-scope comparison repairs these source cases". That second line
is the rule now written. The script models the rule and does not read
the design files, so this is not independent proof.

## Orchestrator to-do (R4D-26 residual; not edited here)

1. Update PLAN.md phase C from contracts.md §9:
   - step 1: R4-core-contracts alone;
   - step 2: R4-core-removal, R4-core-settings and R4-core-ops in parallel;
   - step 3: R4-notify-diag, then (or alongside) R4-app.
2. Write the packet files with the exact file sets from §9.1–§9.3. This includes `decision.rs`, every `lib.rs` registration, and the stubs (including the `*_tests.rs` stubs).
3. Remove the old "pre-agreed `lib.rs` lines" exception from R4-notify-diag.
4. Implement the T-R4D17 merge gate, including the `tests\required-cases.toml` section check: unchanged outside the packet's own markers, markers kept, and new case IDs globally unique. Re-check this against the integration base after earlier packets merge.
5. Hand over `Cargo.lock` serially: step 1, then R4-notify-diag.
6. At the foundation merge, build the whole workspace.
7. Add the new step-1 tests to the R4-core-contracts packet:
   - T-R4D28-exact-source-unrelated-addon
   - T-R4D28-regrouped-exclusion
   - the extended T-R4D23-same-release-table
8. Assign the remaining new tests:
   - R4-core-settings: T-R4D29-empty-recovered-copies and T-R4D30-reassigned-author.
   - R4-core-ops: T-R4D31-held-self.
   - The `AllHoldCause::RetirementUnresolved` variant goes in step-1 `status.rs`.

## Remaining owner queue (unchanged, all PENDING OWNER)

Q-55, Q-57, Q-58, Q-60, Q-61, Q-63, Q-64, Q-77, Q-78, Q-81, Q-82, Q-84,
Q-85, Q-86 and Q-87. The reviewer accepted the Q-64 and Q-81 deviations
provisionally. Q-49, Q-50 and Q-52 stay owner-decided (D-21).

## Verification

`cargo xtask verify --tier offline` was run in this worktree before the
commit on Windows x64. All six checks passed: offline/fmt, clippy, test,
required-cases, deny and markdown-links. The overall result was PASS
(allow-blocked: false, exit 0). It checks the repository and its links,
not the design tests named above.
