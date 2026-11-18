# Evidence: R4-design-fix3 (retry 2, final)

- **Packet:** [R4-design-fix](../packets/R4-design-fix.md). This is retry 2,
  after the failed [R4-design-recheck2](R4-design-recheck2.md).
- **Previous attempts:** [R4-design-fix](R4-design-fix.md) (`c939b30`) and
  [R4-design-fix2](R4-design-fix2.md) (`fda2233`).
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-design-fix3`
- **Branch:** `run004/R4-design-fix3`, created from `main` at `e1fe458`.
- **Environment:** Windows x64 owner workstation.

This change is design only. No product code, test code, build file or CI
file changed. Edits are limited to `docs\design\{contracts, removal,
settings, rollback, ops, questions}.md` and this file.

- **Owner decisions:** the owner was unavailable. Nothing here is recorded
  as an owner decision.
- **Q-49 and Q-50:** their owner-accepted D-21 semantics are unchanged.
- **PENDING OWNER labels:** every one is kept.
- **Status of these claims:** they are the author's own. They still need
  an independent recheck.
- **Named tests:** these are design obligations. None has been
  implemented or run.
- **Wire format:** the only addition is the `scope` field on code-choice
  `exclude` entries (contracts.md §5.2), which R4D-23 strictly needs.
  Nothing else changes on the wire.

## Finding → change → section

| Finding | Change | Sections |
|---|---|---|
| R4D-23 (major) | Matching to recognise the same release is now separate from per-folder routing and liveness. `same_release` is three-valued (`Same` / `Different` / `Unknown`). It compares complete recorded content over the union of both scopes; a listed scope folder that is not present counts as `Absent`. Overlap on common keys never proves equality, and unknown facts make a source wait. Sources are complete installed copies and never a local package grouping. A users table defines how rejection, exclusion, `considered`, the recovered chain and look-alike treat each result. `exclude` entries carry their own `scope`. The writer formula and V-CHOICE are updated. `NewVersionAfterDecision`, `NewerAfterRollback` and resume use `same_release`. The recovered chain advances only if every actual copy of the recovered release is rejected | contracts.md §4.1 (levels 2/6/8), **§4.2**, §4.4, §5.1–§5.4; rollback.md §2.1, §3, §3.1, §3.2, §10; removal.md §4; ops.md §1.5, §6, §7; questions.md Q-50, Q-78 |
| R4D-24 (major) | Ordinary selection (level 8) uses the whole addon: domain §6 `compare_package`, the completeness predicate, and no fallback to a lesser candidate. It ends in `Chosen(c)` or an addon-level decision before any per-folder step. Only then is c projected per folder. Per-folder liveness never mixes sources. Partial overrides come only from explicit keep or rollback records, with recovery. Hooks are now whole-source: `excluded` and `rejected` take `(&Package, &Source) -> ScopeMatch` | contracts.md §4.1, **§4.3**, §4.4, §9.4, §10; removal.md §4 |
| R4D-25 (major) | A retirement's status is three-valued (Effective / Void / Unresolved). It is a pure function of the ledger and depends only on the transitive status of the retirements it references. A missing, unreadable or Unresolved dependency, or a dependency still `not_a_member`, makes the dependent Unresolved, not "not effective". Monotonicity: a status moves only Unresolved → Effective or Unresolved → Void. `Retired(E)` ∈ {Yes, No, Unresolved}. HoldCut holds E's later records as `pending(retirement_unresolved)` without advancing the watermark. A held E is still awaited, is not shown as retired, and gets no master prompt. A held own computer pauses (Q-60 default). V-RET gains the pending row | contracts.md **§1.10** rules 1–7 and trace, §1.11, §6, §9.1, §10; ops.md §2.1, §2.2, §2.4, §6, §7 (T-45-5, T-R4D25); questions.md Q-60, Q-61 |
| R4D-26 (minor) | The foundation recipe is complete. R4-core-contracts owns and registers every module of core, adapters and runtime in their `lib.rs` files, with stubs for every step-2 and step-3 module (including test modules). Every decision DTO and every hook-signature type is in the new step-1 `decision.rs`, and `decide.rs` holds only functions. `Cargo.lock` is handed over serially (step 1 → R4-notify-diag). `tests\required-cases.toml` is a checked shared-append exception with `# >>> section` / `# <<< section` markers and a gate check. No later packet edits a `lib.rs` | contracts.md §9.1, §9.2, §9.3, §9.5, T-R4D17-file-ownership; ops.md §1.1, §6; settings.md §10.1 |
| R4D-27 (minor) | The rollback outcome's `settings` field and summary are computed at planning time from the target side (the S0 paired selection for R0) and whether it is fully restored. They are never copied from the source recovery point's `paired`, which describes R1/S1 | rollback.md §4, §10 (T-R08, T-R09) |
| Note: policy-head witness | `policy` is a bounded witness of 1–8 entries: all heads if there are ≤ 8, otherwise the 8 smallest hashes. The writer must be authority under every listed head. Each reader also checks every current head it knows; any disagreement gives `PolicyConflict`, which holds the revision without dropping or applying it. No wire change | settings.md §3.3, §11 (T-R4D4-nine-policy-heads plus its A-38 control) |
| Note: isolated epoch control | There are two controls. Ignoring the dirty-epoch hold must fail the experiment scenario. An isolated fixture with `epoch_clean = true` and an equal seq separately guards strict `>` (mutating it to `>=` fails) | settings.md §11, T-R4D19 row |
| Note: root spelling | removal.md now uses `SettingsRoot { unit, state: RootState }` | removal.md §3 |

## Counterexample walk-throughs

### R4D-23 added folder (T-R4D23-added-folder-rollback)

- **Setup:** the rejection has scope {core, extra}. It rejects R1 =
  `{core:U, extra:BAD}` and recovers R0 = `{core:U}`, which is
  `{core:U, extra:Absent}` over that scope.
- **R0 against R1:** extra is `Absent` on one side and BAD on the other,
  so the result is `Different`. R0 is not rejected; it is installed and
  stays a source.
- **R1 copy:** a source holding exactly `{core:U, extra:BAD}` is `Same` as
  R1, so it is rejected.
- **Overlap control:** matching on common keys only would compare {core},
  find U = U, and reject R0. The A-38 control makes this test fail.

### R4D-23 dropped folder (T-R4D23-dropped-folder-rollback)

- **Setup:** scope {core, old}. Release 2.1 dropped `old`, so the rejected
  R1 is `{core:U}` and the recovered R0 is `{core:U, old:O}`.
- **R0:** it is `Different` (O ≠ Absent), so it is installed with `old`.
- **Source without `old`:** `{core:U}` is `Same` as R1, so it is rejected.
- **Source `{core:U, old:O2}`:** it is `Different` from R1, so it can only
  be a look-alike.

### Q-50 keep and resume with a shared component (T-R4D23-keep-resume-shared-component)

- **Setup:** keep V2 `{a:2, b:U}`, declining V3 `{a:3, b:U}`. The exclude
  entry is V3 with `scope` {a, b}.
- **Resume without allow-again:** the entry is carried verbatim.
- **V2's source:** a differs, so it is `Different` from the entry and stays
  eligible. It is the target.
- **V3's source:** it is `Same` over {a, b} and is never selected.
- **Grouping:** the result is identical with a and b grouped or
  ungrouped. It is also identical when folder c joins between keep and
  resume: the entry keeps scope {a, b}, while the resume's `folders` is
  {a, b, c}.
- **Q-49/Q-50:** the semantics are intact. A source `Different` from every
  `considered` entry raises `NewVersionAfterDecision`, and the kept version
  stays.

### R4D-24 neutral hooks (T-R4D24-no-hybrid)

- **Setup:** X = `{a:3.0, b:1.0}` and Y = `{a:2.0, b:2.0}`.
- **Comparison:** `compare_package` gives `Ambiguous(MembersDisagree)`,
  which leads to an addon-level `NeedsDecision`.
- **Result:** `{a:3.0, b:2.0}` is never planned and nothing is installed.
  Grouping is the union over all observations, so every computer reaches
  the same result.

### R4D-25 R/S/T/U (T-R4D25-partial-retirement-dag)

The records are:

| Record | Retirement | Cutoff | References |
|---|---|---|---|
| R | A → B | 10 | S |
| S | C → A | 20 | T |
| T | D → C | 30 | U |
| U | E → D | 40 | none |

- **Complete-first:**
  - Statuses: U Effective, T Void, S Effective, R Void.
  - Retired: B No; A Yes (cutoff 20); C No; D Yes (cutoff 40).
- **Partial 1 (R, S):**
  - Statuses: S is Unresolved because T is missing; R is Unresolved.
  - Held: B and A are held, with HoldCut 10 and 20.
  - Currentness: both are still awaited and neither is shown as retired.
  - No master prompt is shown.
  - A and B pause their own publishing.
  - Nothing is uninstalled.
- **Partial 2 (+T):**
  - T is Unresolved because U is missing.
  - C is also held, with HoldCut 30.
- **Partial 3 (+U):**
  - Statuses settle: U Effective → T Void → S Effective → R Void.
  - B: its held states are admitted.
  - A: retired at 20; its held records become invalid.
  - C: released.
  - D: retired at 40.
  - The result equals complete-first.
- **Restart after any partial step:** the statuses are recomputed and are
  identical. The held seqs are still admissible, because the watermark
  never passed them.
- **Order T, U then R, S:** B is never held or retired, and the final
  result is the same.
- **Invariants:**
  - B is never Retired = Yes.
  - No status flips between Effective and Void.
  - The Q-52 boundary (MIN cutoff, no re-authorization) and the Q-61
    PENDING OWNER default for unknowing mutual retirement are unchanged.

### R4D-27 (T-R08, T-R09)

- **S1 readable, S0 missing:** the source point's `paired` is true, but
  the outcome is `code_only`, with the A-19 warning in the summary.
- **No S1 files, S0 restored:** the outcome is `paired`.
- **A-38 control:** copying the source `paired` value fails T-R08.

## Orchestrator to-do (not edited here: PLAN.md and packet files)

1. **PLAN.md phase C:**
   - Step 1 is **R4-core-contracts**, alone.
   - Step 2 is R4-core-removal, R4-core-settings and R4-core-ops, in
     parallel.
   - Step 3 is R4-notify-diag, then (or with) R4-app.
   - Copy the file sets exactly from contracts.md §9.1–§9.3.
2. **R4-core-contracts packet:** include all of the following.
   - The new `decision.rs`, with every DTO listed in §9.1.
   - `lib.rs` of core, adapters and runtime declaring every later
     module.
   - Every stub listed in §9.1, including the `*_tests.rs` stubs, the
     adapters stubs (`wtf.rs`, `settings_store.rs`, `notify.rs`,
     `update_http.rs`) and the runtime stubs (`notices.rs`, `updates.rs`,
     `diag.rs`).
   - The sectioned `tests\required-cases.toml`.
   - The new tests: T-R4D23-same-release-table,
     T-R4D23-unknown-waits, T-R4D24-no-hybrid,
     T-R4D25-partial-retirement-dag and the extended
     T-R4D15-vret-predicate.
3. **Step-2 and step-3 packets:**
   - Remove every "new file" and `lib.rs` line from their scope.
   - R4-notify-diag no longer has the "pre-agreed `lib.rs` lines"
     exception.
4. **Merge gate (T-R4D17-file-ownership):**
   - Implement the `required-cases.toml` section check: outside the
     packet's own markers the file equals the base, the markers are
     unchanged, and every added case id is new.
   - Allow it as the only cross-owner file.
5. **`Cargo.lock`:** state the serial handover (step 1 → R4-notify-diag)
   in the packets. Step 2 adds no dependency.
6. **Test owners:**
   - R4-core-settings: T-R4D23-added and dropped folder rollback, T-R08
     and T-R09.
   - R4-core-ops: T-R4D23-keep-resume-shared-component and the
     user-facing T-R4D25 checks.
   - The settings packet: T-R4D4-nine-policy-heads and the isolated
     T-R4D19 control.

## Remaining owner queue (all PENDING OWNER, conservative defaults)

- Q-55, Q-57, Q-58, Q-60, Q-61, Q-63, Q-64, Q-77, Q-78, Q-81, Q-82, Q-84,
  Q-85, Q-86, Q-87.
- Q-64 and Q-81 keep the reviewer's provisional acceptance of their
  deviations; that is not an owner decision.
- Q-49, Q-50 and Q-52 stay owner-decided (D-21) and unchanged.

## Verification

`cargo xtask verify --tier offline` was run in this worktree before the
commit, on Windows x64. All six checks passed: offline/fmt, clippy, test,
required-cases, deny and markdown-links. The overall result was PASS
(allow-blocked: false, exit 0).

This run checks the repository and its links. It does not run any of the
design tests named above.
