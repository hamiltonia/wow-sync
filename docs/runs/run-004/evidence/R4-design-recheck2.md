# Evidence: R4-design-recheck2

## Result

**FAIL: three major shared-behavior defects remain. No blocker is retained
merely for a locally resolvable implementation detail.** The previous
foundation dependency cycle is substantially repaired. Most original
counterexamples are now closed at design level.

| Document | Verdict | Reason |
|---|---|---|
| [contracts.md](../../../design/contracts.md) | **FAIL** | New overlap matching, projected ordinary selection and incomplete retirement-dependency evaluation: R4D-23–R4D-25 |
| [removal.md](../../../design/removal.md) | **FAIL, shared dependency** | Cleanup ordering is repaired; ordinary selection/restore still consumes the unsafe shared matching/selection rules |
| [settings.md](../../../design/settings.md) | **PASS, design only** | Original target/epoch/large-list traces repaired; foundation handoff and bounded policy-witness guidance below are implementation details, not a retained blocker |
| [rollback.md](../../../design/rollback.md) | **FAIL** | The new folder-changing flow rejects its own recovered release in R4D-23; R4D-27 is local outcome-reporting guidance |
| [ops.md](../../../design/ops.md) | **FAIL** | Q-50 filtering and retirement consume R4D-23/R4D-25 |
| [questions.md](../../../design/questions.md) | **PASS, approval audit** | Owner-accepted Q-49/Q-50 text restored; Q-61 now honestly pending; native proof no longer claimed |

These are component design verdicts, not implementation acceptance. The
common-contract FAIL still prevents opening the phase C design gate.
The three major roots must be settled before the foundation freezes its
shared semantics. The two minors do not require another product/design
round: apply their local guidance in the responsible implementation/
orchestration handoff.

## Environment, authority and scope

- Checkout: `D:\GitHub\wow-sync.worktrees\R4-design-recheck2`.
- Branch: `run004/R4-design-recheck2`.
- Reviewed baseline: `37512533b976cbf018ddb6ae4b96786c6ac6f8a3`.
- Windows x64; pinned rustc `1.100.0 (0ef1bc386 2026-11-09)` and cargo
  `1.100.0 (3acc79c0c 2026-11-05)`.
- Read AGENTS, COMMON, [fix2 evidence](R4-design-fix2.md), the revised
  common contract and affected removal/settings/rollback/ops/question
  sections against the [previous recheck](R4-design-recheck.md),
  [SPEC](../../../../SPEC.md), [ACCEPTANCE](../../../../ACCEPTANCE.md),
  PLAN and [OWNER-DECISIONS](../../../OWNER-DECISIONS.md).
- A scoped diff confirms the requirements, owner register, approved plan,
  COMMON and actual crates are unchanged from `c939b30`. Earlier source
  inspection therefore remains applicable; the real package comparator
  was additionally read and its relevant existing test executed.
- Writes are only this evidence and the small owned
  [written-rule models](R4-design-recheck2/design_probes.py). No design,
  production, product-test, dependency, build or CI edits. No subagents,
  provider requests, game data, process manipulation, pushes or rewrites.
- Actual runtime-model choice is launcher-controlled, not independently
  introspected by this worker.

## Every requested finding rechecked

“Closed” means the reported counterexample is repaired in the written
design, not that future tests or acceptance have run. New root defects are
identified separately to avoid carrying an old blocker label onto a
different minor omission.

| Finding | Status | Independent disposition |
|---|---|---|
| R4D-1 | **CLOSED** | `apply/2` now permits mixed install/replace/remove; the remove authorization can name a journal-local pending removal; missing roots are representable; `paired` covers no-applicable/settings-only outcomes; pending bytes are fixed at creation. The original four incompatible requirements have a viable common representation. Follow the authoritative `RootState` shape where removal's illustrative old `{unit,checkpoint,location}` spelling remains. Interpret `/1` point preservation as the pure legacy-apply case, not a prohibition on explicit `/2` rollback points. |
| R4D-4 | **CLOSED, original traces** | `Known` parts cover 1,024 issuers and mirror-only hashes without the old caps; full-policy parts preserve the snapshot. The model reconstructs 5,000 exceptions from 22 bounded parts. No artificial `GroupHistoryTooLarge` dead end remains for those cases. A bounded policy-reference witness needs the construction note below; a new write dead end was not established from the remaining `policy <= 8` shape alone. |
| R4D-5 | **CLOSED** | Every due item, origins included, is classified before 3b. `cleanup_required` cannot take the absent/no-op ack path, and the finalizer owns its ack. The no-cleanup and paused branches now explicitly ack before baseline clearing. |
| R4D-8 | **CLOSED** | The recorded basis is no longer shrunk; invalidated-basis handling precedes first adoption; missing parents wait; restore ancestry and zero-node behavior remain specified. |
| R4D-9 | **CLOSED, original composition traces** | Same-scope different recovered trees lead to a decision; chain following remains explicit. Per-folder liveness retains uncovered scope. The new matching/ordinary-selection defects below are not claimed as a pass of complete rollback behavior. |
| R4D-10 | **PARTIAL; residual tracked by R4D-23** | Q-49 pin-until-choice and Q-50 inherited exclusions/explicit allow-again are restored; V-CHOICE is action-discriminated. Original single-folder keep/resume/replay/new-version traces work without inventing a rejection. Exact multi-folder semantics are not preserved by the new overlap-matching rule. |
| R4D-15 | **CLOSED** | The retirement/non-retirement boolean now exempts the intended record family, and void retirement handling is explicit. |
| R4D-16 | **PARTIAL; residual tracked by R4D-25** | MIN prevents the original higher-cutoff reauthorization. A retirement explicitly referencing a known effective retirement of its issuer is void when that dependency is resolved. However effectiveness can change when a referenced retirement's own dependencies arrive, reactivating a member. |
| R4D-17 | **PARTIAL, minor handoff only** | The original type/fixture/validator cycle is removed: a new foundation owns all kinds, shared DTOs, journal execution, hooks and fixtures before behavior workers. Its gate can compile typed neutral stubs without implementing feature behavior. The remaining registration/type-placement/manifest exceptions are R4D-26, not the former blocker. |
| R4D-18 | **CLOSED** | Ordered cases distinguish no basis, invalid basis, unchanged frontier and strict progress. The original unchanged-head and wholly-invalidated-basis probes now keep local files. |
| R4D-19 | **CLOSED** | Activation is unpublished; an existing difference dirties the epoch until an explicit choice; automatic publication also requires the strict comparison boundary. The original seq-5 startup experiment cannot auto-publish. |
| R4D-20 | **CLOSED, scope restriction** | The identical-folder-only prohibition is removed. One rollback journal carries the union scope and the removal/re-add/ack records needed for changed folders. The restored flow still needs the R4D-23 matching correction to work for an unchanged common folder. |
| R4D-21 | **CLOSED, liveness defect** | A later a-only record ends X for a, not for b. The original wide-constraint counterexample is repaired; it does not justify selecting ordinary releases independently per folder. |
| R4D-22 | **CLOSED** | Native NTFS/APFS identity behavior is explicitly NOT RUN; fixtures and named tests are obligations, not native execution evidence. |

### Q-49 and Q-50: precise answer

- **Q-49's intentional-choice rule is restored:** an unconsidered newer or
  unordered copy raises `NewVersionAfterDecision`; the kept copy stays.
  No rejection must be fabricated first.
- **Q-50's intended exclusion state is restored:** resume inherits declines;
  explicit `allow_again` clears selected inherited exclusions; neither the
  kept nor an unseen version is newly declined.
- **Cannot certify exact end-to-end preservation:** C-TGT's new
  nonempty-intersection matching lets an exclusion match the unchanged
  component of the kept version. The declared Q-50 state is right, but its
  application is wrong in R4D-23. This is a shared matching error, not a
  reason to ask the owner to answer Q-50 again.

## New major findings

### R4D-23 — major — overlap equality treats different releases as the same

**Locations:** contracts §4.2 “Projection and matching,” §5.3 exclusion
filtering; rollback §§2.1, 3 and 3.1.

The shared predicate is:

`matches(C,E) = nonempty(keys(C) intersect keys(E)) AND equal trees on every common key`.

It does not require either complete recorded scope or equal release
content. A direct, small counterexample needs no corrupt metadata:

| Release | Folder trees |
|---|---|
| R1, rejected | `{core: U, extra: BAD}` |
| R0, recovered | `{core: U}` |

Only the added folder changed the release. R0 and R1 have different valid
package hashes, so V-REJECTION admits them. But `matches(R0,R1)` is true
because their only common tree is U. Level 2 therefore rejects the very
R0 the rollback promises to recover. The recovered-chain rule also finds
R0 rejected by the same rejection, revisits it and exhausts the chain.
An issuer that directly installed R0 is subsequently labeled as holding
rejected content; peers cannot follow the promised correction.

There is a Q-50 counterexample even without rollback:

- Keep V2 `{a:2,b:U}`, declining V3 `{a:3,b:U}`, then resume.
- The exclusion is V3, not V2. Full vectors distinguish them.
- C-TGT selects ordinary candidates over projections. V2's projection
  `{b:U}` **matches the V3 exclusion**, so the kept copy's b component
  becomes ineligible. The same issue arises with separately grouped b.

This is not just a missing check an executor can add locally: matching is
the authoritative shared identity/equivalence rule used by both choices
and rejections.

**Must change before foundation freeze:** define complete-content
rejection/exclusion matching separately from per-folder routing/liveness.
Missing context must not prove equality. Preserve a whole candidate's
recorded release facts through matching, or explicitly wait for sufficient
scope facts. Do not equate exact releases by any equal overlap.

**Required counterexamples:** added-folder rollback with byte-identical
core, dropped-folder rollback with byte-identical common folders, and
keep/resume where kept/declined copies share an unchanged component,
including changed local grouping.

Links: R-05/R-06/R-10; A-08/A-18/A-19/A-20; Q-50/Q-78.

### R4D-24 — major — ordinary per-folder selection manufactures a release

**Location:** contracts §4.2, “No live constraint for f.”

The rewrite selects ordinary content over the **projection of each
candidate onto f**, independently for each folder. That changes the slice
and domain's whole-addon comparison:

| Installed candidate | a version | b version |
|---|---|---|
| X | 3.0 | 1.0 |
| Y | 2.0 | 2.0 |

All trees are usable and differ. Existing `compare_package(X,Y)` returns
`Ambiguous(MembersDisagree)`: neither installed release is demonstrably
newer. The new projected ordinary rule picks a from X and b from Y,
creating `{a:3.0,b:2.0}`, a release no computer has. This can mix
incompatible suite components without the required addon-level choice.
There need be no removal, rejection or code-choice record at all.

The real existing
`package::tests::mixed_member_orders_are_ambiguous` was executed: **PASS,
one test**. The design model shows the contrary projected result; this is
not a claim that the current unchanged product already synthesizes it.

**Must change before foundation freeze:** preserve whole-addon
candidate/completeness/comparison decisions for ordinary selection;
project the **chosen result** only after that selection. Per-folder
authority/liveness is not permission to invent a per-folder release
ordering. Deliberate partial overrides still need their explicit decision
and recovery rules.

**Required counterexample:** with neutral hooks/no authority records,
mixed member orders must remain an addon decision, not a hybrid install.

Links: SPEC R-05/R-06; domain §§5–6; A-08/A-20/A-23.

### R4D-25 — major — partial retirement dependencies can reactivate a member

**Locations:** contracts §1.10 rules 1–3, §1.11 V-REF; ops §2.1.

The new boolean voidness rule is not monotone while its dependency DAG is
partially delivered. V-REF checks presence and kind of a record's direct
references; it does not require a referenced retirement's effective/void
status to be resolved transitively.

All issuers/subjects below are already historical members, and each
`retirements_known` entry correctly names a retirement of its issuer:

| Record | Issuer → retired subject | `retirements_known` |
|---|---|---|
| R | A → B | S |
| S | C → A | T |
| T | D → C | U |
| U | E → D | empty |

1. Deliver **R and S**, without T/U. S is pending because T is missing,
   so S is not effective. R's direct reference S is present with the right
   kind; R names no *effective* retirement of A. Under the listed
   predicate R is effective, and **B becomes retired**.
2. Deliver T and U. U is effective; T is void; therefore S is effective;
   therefore R is void. **B is no longer retired**, with no re-enrollment.
3. A reader receiving the complete DAG first never retires B. Caching
   “ever retired” to mask step 2 would instead make the final membership
   depend on delivery order.

The design explicitly supports void retirement records from older/faulty
writers; these are well-formed negative-case inputs, not executed hostile
code or forged membership. A deterministic final boolean on a complete
DAG does not prove safe intermediate authority or monotone retirement.

**Must change before foundation freeze:** distinguish unresolved
retirement knowledge from an established “not effective.” Require the
relevant transitive dependency status to be settled before a retirement
can become effective; specify how pending/invalid dependencies propagate.
Do not fix it by an arrival-order-dependent sticky membership bit.

**Required counterexample:** the four-record trace in both complete-first
and partial-DAG delivery, with restart, checking *every intermediate*
Retired/currentness/publish decision, not only the final set.

Links: R-03/R-06/R-11; A-20/A-45.

## Local implementation/orchestration guidance: minors

### R4D-26 — minor — complete the mechanical foundation handoff

**Locations:** contracts §9.1–§9.5; settings §10; ops §6.

The high-level dependency graph is now **acyclic**:
foundation → three behavior packets → notification/app/integration.
The three step-2 **code** path sets are pairwise disjoint, and the common
DTOs/kinds/fixtures are assigned before their consumers. The former
R4D-17 blocker is not retained.

The literal file/DTO inventory is not yet a complete merge recipe:

- New step-2 adapter files `wtf.rs`/`settings_store.rs` need module
  declarations/re-exports. `wow-sync-adapters\src\lib.rs` is not assigned
  to the foundation or those consumers. Similarly, step-3 adapter/runtime
  modules need registration; the only explicit module-list exception is
  for core's `lib.rs`.
- `hooks.rs` already mentions `OptionId`, `EvidenceToken`, `FreshEvidence`
  and `DecideOutcome`, while ops §6 still lists decision DTOs as step-2
  contents. Under the authoritative “all shared types at step 1” rule,
  define those DTOs in the foundation's typed stub/shared modules, not
  for the first time in step 2.
- All behavior packets intentionally append to **the same**
  `tests\required-cases.toml`; they are not strictly file-disjoint.
  The filename-only merge gate's step-2 path rows omit that exception,
  so it would reject their required test registrations.
- PLAN still names the old packets. The author correctly identifies that
  updating it belongs to the orchestrator, not this worker.

**Guidance:** foundation registers/creates the adapter/runtime stubs and
exports at its merge; make explicit serial ownership transfers for those
files and shared DTOs. Admit the per-packet manifest sections as a checked
exception, validating section ownership rather than pretending the file
sets are disjoint. Update actual packets/PLAN before launch. The
foundation build and typed-hook fixtures must compile all these seams.
No new wire format or product choice is required, so this is **minor**.

### R4D-27 — minor — do not copy source recovery pairing into rollback outcome

**Location:** rollback §4, paragraph after steps 1–4; contracts §2.3.

The recovery point retains the state **taken out**, R1 and its captured
S1 roots. The rollback outcome concerns the state **restored**, R0 and
whether S0 was available. Those are different facts.

If S1 is capturable but R0's S0 is missing, the rollback correctly keeps
current S1. Capturing S1 for R1's recovery can yield a paired recovery
point, but §4 says the rejection's outcome copies that `paired` value
as “every unit restored.” That would wrongly suppress the A-19 code-only
warning. Conversely, an incomplete source recovery does not itself prove
the target restore was incomplete.

**Guidance:** retain the common recovery-root contract; calculate the
existing rejection `settings` field and summary from actual target
selection/restoration results, not from source-point pairing. Exercise
T-R08/T-R09 with readable current S1 and missing old S0. This is a local
rollback-flow correction using existing fields, hence **minor**, not a
new shared-contract blocker.

### Other bounded implementation notes, not additional gate findings

- A revision's `policy` references remain bounded at eight. Before
  implementing the writer, specify a bounded authorization witness for
  multiple agreeing policy heads (while checking all current heads for
  conflict), or route a genuine representation problem to the foundation.
  V-REVISION tests every **listed** policy; it does not explicitly require
  an exhaustive current-head vector. Merely counting nine heads is
  therefore insufficient to prove another R4D-4 dead end. Test that case
  rather than silently dropping an authority-conflicting head.
- The epoch negative control must isolate the behavior it claims to
  break. In the preexisting-experiment fixture, `epoch_clean=false`
  already prevents publication even if `>` is mutated to `>=`.
  Ignoring the dirty-epoch hold is a direct behavioral control; isolate
  the boundary predicate separately if retaining that particular control.
- Follow the authoritative new root shape and journal kind/action table
  where older illustrative spellings or section references remain.
  Routine naming/module/fixture corrections are not grounds for another
  blocking design round.

## Owner-decision audit and questions

**No newly false owner-decision attribution found in fix2.** The owner
register is unchanged; Q-49/Q-50 again state their existing D-21 decisions,
not the earlier inverted defaults. Q-52 retains the accepted
post-retirement boundary and MIN is its attempted implementation. Q-61
now labels concurrent unknowing retirement **PENDING OWNER**, rather than
claiming a new authority policy needs no owner. The three major defects
are failures to deliver those stated rules, not approvals to change them.

Q-64's plain-pattern name-source narrowing and Q-81's explicit-choice-only
rollback ending retain the previous **provisional** acceptance. They are
still pending owner choices, not decisions by this verifier.

Existing owner queue, all still pending:

| Questions | Answer/default needing owner confirmation |
|---|---|
| Q-55 | explicit add-back versus automatic re-add after reinstall |
| Q-57 | local “Continue without it” with risk/recovery disclosure |
| Q-58 | exact named-folder removal scope |
| Q-60 | retired self stops publishing and applying, rejoins explicitly |
| Q-61 | concurrent unknowing mutual retirement/default authority policy |
| Q-63 | required native macOS notifications; no implicit fallback waiver |
| Q-64 | previewed optional plain-pattern folder names, default included |
| Q-77 | settings-file size parameter; real settings sync remains disabled until approved |
| Q-78 | exact-content rejection and look-alike hold |
| Q-81 | explicit lift/choice ends recovered target; no inferred creation time |
| Q-82 | no settings-checkpoint rejection |
| Q-84 | required peer capabilities before group rollback |
| Q-85 | recurring-notice suppression; never `DataAtRisk` |
| Q-86 | OS `DataAtRisk` notification toggle behavior |
| Q-87 | automatic update-check cadence |

Do **not** ask the owner to re-decide Q-49/Q-50 or to approve a hybrid
release, false content equality or temporary re-enrollment. Correct those
engineering rules. The removed identical-folder restriction and repaired
original history caps no longer require the scope-deferral questions from
the previous recheck.

Pending defaults may still be implemented/tested synthetically where the
approved packet allows it, but dependent acceptance remains BLOCKED, never
passed by assumption. This review does not grant a retry-budget exception,
a new run approval or an implementation launch.

## Executed evidence and limitations

The [12 Python probes](R4-design-recheck2/design_probes.py) are independent
models of the written rules. PASS means an expected repair or
counterexample was reproduced. It is not a new-feature product test,
native result or A-38 control. Only the separately named existing Rust
test executes production comparison code.

| Command/check, in the assigned checkout | Result |
|---|---|
| `git rev-parse HEAD`; `git branch --show-current`; `git status --short` | PASS: assigned baseline/branch, clean before evidence |
| `git diff --exit-code c939b30 3751253 -- AGENTS.md README.md SPEC.md ACCEPTANCE.md ENGINEERING.md docs\OWNER-DECISIONS.md docs\runs\run-004\PLAN.md docs\runs\run-004\packets\COMMON.md crates` | PASS, exit 0: approvals, requirements and actual implementation unchanged |
| `rustc --version`; `cargo --version` | PASS: actual pinned versions recorded above |
| `python -B docs\runs\run-004\evidence\R4-design-recheck2\design_probes.py` | PASS, exit 0: final 12 model assertions; expected counterexamples reported explicitly |
| `cargo test -p wow-sync-core package::tests::mixed_member_orders_are_ambiguous -- --exact` | PASS, exit 0: 1 passed, 181 filtered out |
| `cargo xtask verify --tier offline` | PASS, exit 0: fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false)` |
| Native NTFS/APFS, installed GUI/downgrade recovery, real client/provider, aggregate memory, new-feature A-38 controls | NOT RUN; future implementation/platform obligations |

The design verdict is independent of baseline build success and the
author's completion claims.
