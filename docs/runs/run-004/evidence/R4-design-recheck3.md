# R4-design-recheck3: final independent design recheck

## Result and scope

**FAIL: one remaining major root, R4D-28; no blockers.** R4D-23 is
partially repaired but its replacement release-matching contract has a
permanent-Unknown case. R4D-24 and R4D-25 are closed at the design level.
There are four minor handoff/implementation items: R4D-26 and R4D-29–31.
No further design-fix retry is authorized by this report.

- Assigned checkout: `D:\GitHub\wow-sync.worktrees\R4-design-recheck3`.
- Branch: `run004/R4-design-recheck3`.
- Reviewed baseline: `e6a6790831e1ea02ffcb94e6b0c6a238ebc46ac3`.
- Prior review: [R4-design-recheck2.md](R4-design-recheck2.md).
- Author claims examined, not accepted as proof:
  [R4-design-fix3.md](R4-design-fix3.md).
- Only this report and its synthetic
  [written-rule probes](R4-design-recheck3/design_probes.py) are added.
  No design, implementation, dependency, product-test or CI file changed.

The assigned final recheck supersedes the earlier evidence-only output
path. COMMON, the approved plan, D-21–24, SPEC and ACCEPTANCE remain
authoritative. Their unchanged bytes, and those of the implementation,
were checked between `3751253` and the reviewed baseline. The canonical
revised contracts and affected removal/settings/rollback/ops/question
sections were read, not inferred from the author's walkthrough.

Environment: Windows x64, actual Rust and Cargo **1.100.0**, Python
**3.15.2**. No macOS
execution, real WoW/settings/provider data, network requests or game
processes were used. The runtime model selection is launcher-controlled,
not independently introspected by this worker.

## Per-document verdicts

These are design verdicts, not feature acceptance or permission to launch
phase C. A shared-contract dependency is identified explicitly.

| Document | Verdict | Reason |
|---|---|---|
| [contracts.md](../../../design/contracts.md) | **FAIL** | R4D-28 contradicts the intended source/entry identity results and is a foundation contract |
| [removal.md](../../../design/removal.md) | **FAIL** | Its §4 target composition consumes the same defective rejection/selection contract; no new removal-union or offline-update resurrection defect found |
| [settings.md](../../../design/settings.md) | **PASS** | No remaining shared-contract major in settings proper; apply the local policy-consumer guidance R4D-30 and retain all owner/data gates |
| [rollback.md](../../../design/rollback.md) | **FAIL** | Exact rejected/recovered source recognition depends on R4D-28; R4D-29 is locally resolvable |
| [ops.md](../../../design/ops.md) | **FAIL** | Keep/resume/exclusion identity depends on R4D-28; held-self presentation/queue handling is local guidance R4D-31 |
| [questions.md](../../../design/questions.md) | **PASS** | No false new owner decision; Q-49/Q-50 product intent is unchanged, although R4D-28 prevents its promised implementation |

## Finding disposition

| Prior item | Status | Independent recheck |
|---|---|---|
| **R4D-23**, complete-release identity | **PARTIAL** | `Absent` fixes the original equal-common-folder false equality. However source scope is global while recorded scope is finite, so exact source/entry equality can never settle: new major R4D-28 |
| **R4D-24**, per-folder ordinary hybrid | **CLOSED** | Contracts §4.3 now selects a whole addon with `compare_package` and completeness before projection; mixed member orders remain an addon decision, not a hybrid |
| **R4D-25**, partial retirement DAG reactivation | **CLOSED** | Dependency-closed Effective/Void/Unresolved evaluation fixes every R/S/T/U intermediate set; 24 delivery permutations, 120 prefix snapshots, duplicates and ledger reloads examined |
| **R4D-26**, mechanical foundation handoff | **PARTIAL, minor** | §9 now assigns shared DTOs, all three module registries, stubs, fixtures, manifest sections and serial lockfile ownership. Actual PLAN/packets still need the orchestrator's mechanical update before launch |
| **R4D-27**, source pairing copied into restore result | **CLOSED** | Rollback §4 and T-R08/T-R09 now derive outcome from restored S0, independently of captured S1; reserved bytes are published only after the operation succeeds |
| Bounded policy-witness implementation note | **CLOSED for construction** | Eight smallest hashes represent nine agreeing heads; a conflicting tenth head outside the witness is still checked. Do not turn ordinary later reassignment into a conflict: R4D-30 |
| Isolated epoch-control implementation note | **CLOSED** | The dirty-epoch scenario now has its own hold-removal control; clean-epoch/equal-seq independently detects `>` changed to `>=` |

“Closed” means the written contract now answers the counterexample. None
of these new phase-C behaviors has been implemented or accepted by the
existing baseline suite.

## Remaining major: owner barrier

### R4D-28 — major — globally scoped sources make exact matches permanently Unknown

**Locations:** contracts §4.2, especially `scope(X) ∪ scope(Y)` and “a
source's scope is every key”; §4.2 users/counterexamples; §5.2–§5.3;
rollback §3–§3.1; ops §1.5.

The repair correctly distinguishes known `Absent` from a tree. It then
gives a recorded entry E content only inside its finite scope, but gives
an installed source m **every key** as its scope. The comparison domain
therefore includes keys E can never describe. An exact match on E's
scope is `Unknown`, not `Same`, permanently. This is not an unstable
folder or a missing download: E's immutable bytes cannot acquire that
information.

Concrete finite counterexample, weaker than the stated universal scope:

1. Rejection entry E has scope `{core, extra}` and content
   `{core:U, extra:BAD}`.
2. A complete admitted source has exactly that content plus an unrelated
   installed addon `{z:Z}`. Over E's scope everything agrees.
3. The union comparison also examines z. `content(E,z)` is unknown,
   while `content(m,z)=Z` is known. Nothing known differs, so the result
   is **Unknown**. Even if z is absent, source absence is known and E's
   content remains unknown.
4. Waiting, restart, more copies and regrouping cannot make this Same.
   The same defect prevents matching a real available source to a
   `recovered` entry, and prevents positive considered/exclusion
   recognition.

The new named keep/resume example contradicts the formula directly:
carried exclusion scope `{a,b}`, resume scope `{a,b,c}`, and an exact
declined source `{a:3,b:U,c:C}`. The table promises Same **over `{a,b}`**;
the normative union includes c and returns Unknown. The kept source
`{a:2,b:U,c:C}` is Different, so the original false-equality counterexample
is fixed, but a declined source now waits forever. The no-stale-fallback
rule can consequently prevent transferring the available kept copy
after resume, and known recovered copies can be treated as unavailable.

This is not proof of offline resurrection: Unknown remains ineligible.
It is a shared contract that cannot deliver its promised correction and
decision behavior, with no pending information capable of resolving it.
Foundation owns `same_release`; settings/rollback and ops independently
consume it. Choosing different comparison domains in those workers would
reintroduce divergent authority, so it must not be left to local guesses.

**Exact minimal required change:** define source-versus-recorded-entry
comparison over the **recorded entry's scope only**. The source remains a
complete admitted inventory used to answer every key of that scope,
including known absence and unreadable-folder Unknown. Do not union the
source's global knowledge domain into the release identity. Preserve the
stated union rule for two recorded entries, and the explicit actual-copy
resolution for incomparable entries. Align the helper contract, hooks and
users table with that distinction before foundation freeze.

Required focused fixtures: exact rejected and recovered source matches
with an unrelated addon present **and absent**; carried `{a,b}`
exclusion after `{a,b,c}` regrouping; existing added/dropped-folder and
unstable-in-scope cases. The probe demonstrates that this specific
recorded-scope repair resolves the source examples; it does not change
the design files or claim implementation proof.

**Owner options now that retries are exhausted:**

1. Authorize this bounded shared-contract correction and one independently
   checked amendment, with an explicit run/budget authorization; no
   additional swarm or retry is assumed here.
2. Keep phase C stopped and revise/defer the dependent scope explicitly.
   “Wait forever” cannot be called a passing implementation of the
   existing approved goals. Proceeding without the correction would need
   an explicit scope/acceptance exception, not an engineering assumption.

No new product behavior is proposed by option 1: it makes the helper
match the already-written scope-specific examples and preserves
Q-49/Q-50. The owner barrier is required by the exhausted retry/run gate.

Links: SPEC R-05/R-06/R-10/R-12; A-18/A-20/A-25/A-26/A-44.

## Local implementation guidance: minors, not another shared-contract blocker

### R4D-26 — minor, residual — synchronize actual launch packets

**Locations:** contracts §9; ops §6; current PLAN §3.

The design dependency graph is acyclic: foundation alone → three
behavior packets → notification/app. Step-2 code path sets are disjoint.
Every shared hook-signature DTO now has a foundation owner in
`decision.rs`; family shapes/validators and golden fixtures are foundation
work. Core/adapters/runtime module registration and future module/test
stubs precede their consumers. `Cargo.lock` transfers serially.

The required-cases manifest is correctly described as a **shared-file
exception**, not literally disjoint: unchanged outside the packet's own
markers, markers retained, new IDs globally unique. Recheck that condition
against the integration base after earlier packets merge.

PLAN still lists the old packet graph; the author explicitly leaves its
update to the orchestrator. Carry §9 into the actual bounded packets and
merge gates before launching. Build the whole workspace at foundation
merge so registrations, neutral hooks and existing consumers really
compile. Routine compatibility shims needed for that build are not a
reason to reopen the wire design. No missing shared DTO/fixture family
requiring a new authority contract was found.

### R4D-29 — minor — guard an empty recovered-source set before `all`

**Location:** rollback §3.1, Unknown comparison and unavailable target.

If entries have different scopes, no actual copy of X's recovered
release may remain. “Every such copy is rejected” is then vacuously
true. Advancing a rejection chain or dropping its constraint in that case
could select an unchosen release instead of the adjacent required
`RecoveredUnavailable` decision.

**Guidance:** first test whether the actual-copy set is empty and use
`RecoveredUnavailable`. Require a nonempty set before its universal
rejection test. An unresolved comparison is not proof of rejection.
Exercise missing sources with differently scoped rejection entries.
The existing decision variant and records suffice; `reject.rs` can
resolve this locally, so it is minor.

### R4D-30 — minor — a historical author need not remain the current master

**Locations:** settings §3.3 new witness-consumer paragraph versus
§5.1–§5.4.

The bounded witness itself works. But checking that an old revision's
issuer is authority under **all current** heads, and otherwise calling
the folder `PolicyConflict`, also catches a normal explicit reassignment:
P0 makes A master; A publishes r; P1 covers P0 and makes B master. There
is one current policy head, it unambiguously chooses B, and r is still a
historically authorized shared copy. A joining reader must not require
changing the master back to A to adopt r. The same issue can unnecessarily
hold an explicit push from a non-master if the paragraph is applied
outside `auto` revisions.

**Guidance:** check all current heads for the actual per-unit authority
conflict defined by §5.1–§5.2, without reauthorizing historical nodes
against a later settled policy. Keep historical witness validity,
current publication authority and policy-conflict apply holds distinct.
Test one covering reassignment with no subsequent session, first
adoption, and an explicit peer push, alongside the ninth/tenth-head
scenario. This is a local consumer predicate using the existing graph,
statuses and wire fields; no shared-record change is needed.

### R4D-31 — minor — distinguish held self from proven retired self

**Locations:** contracts §1.10 rule 6 versus ops §2.4.

The common contract correctly says Unresolved is **not shown as
retired**, with a “may have retired” waiting message. Ops §2.4 now covers
both Yes and Unresolved, but unconditionally says “This computer was
retired,” supplies a definite retirement date/by line, and discards
unwritten reserved actions. That is not established while history may
still make the retirement Void.

**Guidance:** branch the existing view/command handling on the
three-valued status. Held self gets the uncertain waiting explanation,
automatic resumption on Void and the explicit **Join again…** escape;
actual retirement gets the definite message. Do not cancel held
unpublished decisions as though retirement were proved; retain them
pending resolution or obtain an explicit cancellation. Local files and
recovery remain untouched. Existing state/command types suffice.

## Retirement recheck: every intermediate set and restart

The original records and cutoffs are unchanged: R A→B/10 knowing S;
S C→A/20 knowing T; T D→C/30 knowing U; U E→D/40 knowing nothing. All
issuers/subjects are historical members. The independent model tabulates
**all 16 delivery subsets**, rather than only the author's one order:

| Delivered set | R, S, T, U statuses (— absent) | Retired | Held |
|---|---|---|---|
| none | —, —, —, — | none | none |
| R | Unresolved, —, —, — | none | B |
| S | —, Unresolved, —, — | none | A |
| T | —, —, Unresolved, — | none | C |
| U | —, —, —, Effective | D | none |
| R,S | Unresolved, Unresolved, —, — | none | A,B |
| R,T | Unresolved, —, Unresolved, — | none | B,C |
| R,U | Unresolved, —, —, Effective | D | B |
| S,T | —, Unresolved, Unresolved, — | none | A,C |
| S,U | —, Unresolved, —, Effective | D | A |
| T,U | —, —, Void, Effective | D | none |
| R,S,T | Unresolved, Unresolved, Unresolved, — | none | A,B,C |
| R,S,U | Unresolved, Unresolved, —, Effective | D | A,B |
| R,T,U | Unresolved, —, Void, Effective | D | B |
| S,T,U | —, Effective, Void, Effective | A,D | none |
| R,S,T,U | Void, Effective, Void, Effective | A,D | none |

The 24 permutations visit 120 prefix snapshots including the empty
snapshot. At each prefix, the model checks the independent expected
status table, derived admission/hold classification, awaiting eligibility,
master-retired-prompt eligibility, own-publish pause, duplicate delivery
and reloading the serialized ledger. Settled statuses never flip;
Retired=Yes never reverts; B and C never become retired. Held A/B/C
remain awaited, with HoldCut 20/10/30 respectively; no retired-master
prompt is inferred from a hold. Final A/D cutoffs are 20/40, B/C held
states are released. A model queue retains held states without advancing
its watermark on their account. This is not execution of actual runtime
watermarks or crash recovery.

A referenced `invalid(not_a_member)` stays Unresolved until historical
membership arrives; a permanently invalid wrong-group dependency can
settle as not effective. Missing/unreadable facts do not become false
merely because their bytes have not arrived.

**Permanent missing-history escape: present in the written design.**
Contracts §1.10 explicitly permits re-enrollment or a fresh effective
retirement by another active member; ops §2.4 exposes **Join again…**
also for held self. Re-enrollment is guided, creates a new enrollment ID
and cannot be affected by a record naming the old enrollment. If every
old computer is held, that explicit join is still the specified escape;
it does not require an active old issuer or a terminal. No missing-data
timeout or silent master election is needed. Old enrollment history
and watermarks remain; currentness must use the current enrollment,
not count replaced historical enrollments as new active members.

This closes the original unsafe-authority finding, not the GUI
acceptance gate. R4D-31 is the local presentation/queue correction. Phase
C/GUI verification must actually complete the permanent-missing-history
join, including the all-held and settings-master cases, before A-26/A-45
can pass.

## `exclude.scope` versioning and backward-safety audit

The new field is inside the explicitly versioned **initial**
`wowsync.code-choice/1` schema; `code-choice/1` capability is unchanged.
V-CHOICE makes `scope` mandatory, nonempty/sorted/unique, bounds it and
requires entry keys to belong to it. Inheritance preserves it verbatim.
It is not a new field added to an already implemented choice reader.

Actual `records.rs` has no code-choice DTO, no authority-index or
capability fields in `CodeState`, and no authority-family implementation.
Its `record!` major is 1, `decode` rejects other majors, and unknown fields
within a major are ignored. The existing slice therefore ignores this
**whole not-yet-implemented family**, not just `exclude.scope`.
Contracts §1.13 correctly says it cannot count as confirmed for the new
feature; the group-action capability gate is necessary. A `/2` journal
still blocks its core applies, not proof that its old GUI understands it.

**Accepted against the real baseline, with a precise compatibility
boundary:** changing an unreleased initial schema before its first
implementation does not by itself require a `/2`. Foundation fixtures,
required-field decoding and capability advertisement must all start with
the scoped form; neutral stubs must not advertise behavior they do not
honor. Never default a missing carried scope to the new parent `folders`.

**Not backward-compatible with a hypothetical pre-scope draft reader.**
Such a reader could ignore `scope`, reconstruct it from the carrying
record and mishandle regrouping while advertising the same capability.
There is no such implemented reader in this checkout, so that hypothetical
is not promoted to a new major. If support for any out-of-tree pre-scope
writer/reader is required, use a new schema **and** distinguishable
capability, or an equivalently fail-closed migration, before deployment.
No mixed-binary scope-migration execution is claimed: the product types
do not yet exist.

## Owner-decision audit and final owner queue

**No new false owner-decided attribution found.** D-21–24 and product
authority files are unchanged. Q-49 still pins V until an explicit
choice after unseen newer/unordered W; Q-50 still carries prior declines
without newly declining the kept or unseen copy. Adding a carried scope
does not authorize changing either product rule; R4D-28 prevents the
claimed scope-specific outcomes rather than authorizing new semantics.
Q-52 and the minimum effective cutoff are unchanged.

Q-60 now explicitly marks held-self pausing as part of its **pending**
default. Q-61 still marks concurrent unknowing retirement and the
Unresolved extension **PENDING OWNER**. Neither is silently owner-approved.
The earlier Q-64 and Q-81 deviations remain provisionally acceptable
defaults, not owner decisions: plain-pattern folder names are still
potentially private, and first-seen time is not proof of release creation.

Before phase C, the orchestrator must take the remaining **R4D-28**
stop/correction/scope choice above to the owner, with remaining run budget.
The outstanding product queue is unchanged:

- **Q-55:** stable reinstall after own removal acknowledgment.
- **Q-57:** local continuation while removal information stays unreadable.
- **Q-58:** exact named-folder removal scope under differing grouping.
- **Q-60:** retired and unresolved-self stop/apply behavior.
- **Q-61:** concurrent retirement and unresolved-history semantics.
- **Q-63:** macOS native notification backend/failure handling.
- **Q-64:** diagnostics folder-name inclusion and privacy consent.
- **Q-77:** saved-variable file cap; real settings sync remains disabled.
- **Q-78:** exact release rejection and look-alike handling.
- **Q-81:** only explicit choice/lift ends recovered-version constraints.
- **Q-82:** no settings-checkpoint rejection.
- **Q-84:** capability gating of group rollback.
- **Q-85:** repeat-notice suppression interval.
- **Q-86:** notification toggle behavior for DataAtRisk.
- **Q-87:** automatic update-check interval.

These questions may retain their documented synthetic implementation
defaults only within a subsequently authorized packet. They are not all
new preimplementation blockers: dependent acceptance remains **BLOCKED**
until the relevant owner answer; Q-77 additionally disables real-data
settings sync. No settings cap, native-notification fallback or numeric
acceptance result is invented here.

## Executed verification and limits

Commands were run from the exact assigned checkout above, with cargo's
directory prepended to PATH in each fresh shell.

| Command/check | Result |
|---|---|
| `git diff --exit-code 3751253 e6a6790 -- AGENTS.md README.md SPEC.md ACCEPTANCE.md ENGINEERING.md docs\OWNER-DECISIONS.md docs\runs\run-004\PLAN.md docs\runs\run-004\packets\COMMON.md crates` | **PASS**, exit 0: authority/plan/common and implementation unchanged |
| `rustc --version`; `cargo --version` | **PASS**, both 1.100.0 |
| `python docs\runs\run-004\evidence\R4-design-recheck3\design_probes.py` | **PASS as a probe runner**, exit 0; reports the expected **DESIGN FAIL R4D-28**, original distinction closure, all 120 retirement snapshots and local-guidance cases |
| `cargo test -p wow-sync-core package::tests::mixed_member_orders_are_ambiguous -- --exact` | **PASS**, 1 passed, 181 filtered out; actual existing comparator, not a new feature test |
| `cargo xtask verify --tier offline` | **PASS**, exit 0, allow-blocked false: fmt, clippy, test, required-cases, deny and markdown-links all passed |
| Staged ownership/whitespace checks | **PASS**: `git diff --cached --check`; exactly the two assigned evidence paths staged; design, implementation and authority diff against baseline empty |

The Python script independently transcribes written rules and a proposed
repair. It does not execute product authority, retirement, rollback,
policy, GUI or recovery code. Its epoch mutations are **model controls**,
not A-38 production negative controls. The baseline repository suite
cannot turn these not-yet-implemented designs into accepted behavior.
Native four-target runs, new-feature crash/IO injection, GUI recovery,
cross-binary migration, performance and real Dropbox acceptance are
**NOT RUN** by this evidence-only packet.
