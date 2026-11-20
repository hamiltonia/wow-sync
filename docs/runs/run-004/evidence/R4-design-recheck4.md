# R4-design-recheck4: bounded independent fix4 recheck

## Result and boundary

**PASS at the design level. No remaining blocker or major in this
bounded scope.** R4D-28–31 are closed. One new minor, R4D-32, records
local mixed-copy handling guidance; it does not require another design
round, new shared type or wire change.

- Checkout: `D:\GitHub\wow-sync.worktrees\R4-design-recheck4`.
- Branch: `run004/R4-design-recheck4`.
- Reviewed HEAD: `832492d9079f87fd765e8b6b1d6d3e905d7e01b1`.
- Exact amendment scope: `git diff 9e98862 832492d`.
- Authority for this additional check: the owner's “continue,” explicitly
  assigned by the orchestrator as option 1 at the R4D-28 barrier: one
  bounded correction and one independent check. It is not approval of
  additional retries, product defaults or increased credit limits.
- Author claims read, not treated as proof:
  [R4-design-fix4.md](R4-design-fix4.md).
- Previous independent findings:
  [R4-design-recheck3.md](R4-design-recheck3.md).
- Only this report and the updated, synthetic
  [rule probes](R4-design-recheck4/design_probes.py) are added. Earlier
  evidence is preserved; designs, implementation, dependencies and CI
  were not edited.

AGENTS, COMMON and the relevant authoritative requirements/approval
boundaries were followed. The diff changes only contracts, ops, rollback,
settings and the author's evidence. Questions, OWNER-DECISIONS, SPEC,
ACCEPTANCE, ENGINEERING, PLAN and all crates are unchanged by fix4.
Review was limited to that amendment and its direct consumers, not a
fresh whole-design audit.

Actual environment: Windows x64; Rust/Cargo **1.100.0**; Python
**3.15.2**. No real addon/settings/provider data, network requests or game
processes were used. Runtime model selection remains launcher-controlled
and was not independently introspected.

## Finding status

| Finding | Status | Independent conclusion |
|---|---|---|
| **R4D-28**, permanent Unknown from source scope | **CLOSED** | Source/entry uses the entry's scope only, with complete source facts; entry/entry retains the union. All requested exact-source, regrouping, absence and unstable cases pass the updated model |
| **R4D-29**, empty recovered-copy set | **CLOSED** | Empty set explicitly gives `RecoveredUnavailable` without advancing/dropping the constraint; nonempty rejected, waiting and usable branches are distinguished. Mixed-copy summary precision is the minor R4D-32 |
| **R4D-30**, historical author rechecked against current master | **CLOSED** | Historical listed-head authorization, current publication authority and conflict between current heads are separate. Settled reassignment does not invalidate/hold old r; non-master explicit revisions need no issuer-authority proof |
| **R4D-31**, held self treated as proved retired | **CLOSED** | Uncertain headline, distinct local hold cause, unwritten queue retention, explicit cancellation, automatic Void resumption and Yes cancellation are specified separately; Join again remains available |
| **R4D-32**, mixed-copy summary precision | **OPEN, minor** | Use detailed rollback step 4 when a proven usable copy coexists with an Unknown copy; retain that local implementation fixture |

R4D-26's orchestration handoff is outside this bounded recheck. The current
PLAN already contains C1 foundation → C2 disjoint packets and the
sectioned-manifest/serial-lockfile rules; the author's old PLAN to-do is
not evidence that those changes are missing. Actual packet/merge-gate
execution remains the orchestrator's responsibility.

## Per-document verdicts

| Document | Verdict | Bounded conclusion |
|---|---|---|
| [contracts.md](../../../design/contracts.md) | **PASS** | R4D-28 repaired in the definition, users, exclusions, hooks and foundation fixture obligations; R4D-32 is minor guidance |
| [removal.md](../../../design/removal.md) | **PASS** | Unchanged by fix4; its previously failing dependency on source/entry identity is repaired |
| [settings.md](../../../design/settings.md) | **PASS** | R4D-30 corrected without a new record field or silent master reassignment |
| [rollback.md](../../../design/rollback.md) | **PASS** | Correct scoped source identity and explicit non-vacuous chain branches; apply R4D-32's mixed-copy guidance |
| [ops.md](../../../design/ops.md) | **PASS** | Correct keep/resume comparisons and held-self behavior; new discriminator is local status only |
| [questions.md](../../../design/questions.md) | **PASS** | Unchanged; Q-49/Q-50 preserved and no pending default promoted to owner-decided |

These are not product acceptance passes, a fresh verification of
unchanged subsystems, permission to skip remaining owner/data gates, or
authority to launch workers outside the current plan and credit ceiling.

## R4D-28: updated-rule counterexamples

The new model implements the two written forms directly. `Source` has
no scope member; its admitted inventory answers any requested key with a
tree, known absence or unreadable/unstable Unknown. Only a recorded entry
supplies the comparison scope when the other operand is a source.
Unadmitted source facts remain Unknown. A known difference still wins
over an unknown key; `Different` does not bypass the ordinary safety and
metadata gates.

**Executed:** 32 release cases in both argument orders. They include:

- **Exact rejected source**, entry scope `{core,extra}`, content
  `{core:U,extra:BAD}`: Same, with unrelated z installed or absent.
- **Exact recovered source**, same scope, content `{core:U}` and known
  absent extra: Same, with z installed or absent.
- The same sources with an unreadable unrelated z: still Same. An
  unrelated source problem is not an in-scope release difference.
- Cross-checking the rejected and recovered sources: Different because
  extra is a tree versus known absence.
- **Carried exclusion after regrouping:** entry V3 retains scope `{a,b}`;
  current sources include c. `{a:3,b:U,c:C}` is Same and declined;
  `{a:2,b:U,c:C}` is Different and eligible. V4 with a changed a is also
  Different, not newly declined. An unreadable c does not alter the
  exclusion comparison; ordinary whole-addon usability is still a
  separate gate.
- **Added/dropped folders:** same-scope recorded absence differs from a
  tree; the original unchanged-common-folder false equality does not
  return. Exact smaller and exact larger sources match their own entries.
- **Unstable in-scope extra:** Unknown until stable; then Same.
  If core already differs, the result is Different despite unknown extra.
- **Entry/entry union retained:** equal entries are Same; known
  tree/absence differences are Different; equal overlap with different
  scopes is Unknown, even when the wider entry records absence. Disjoint
  scopes are Unknown. A known difference still gives Different when
  another key lies outside one recorded scope.

Two **model-only** negative controls were caught: restoring the old
global-source-union rule and comparing only common tree keys. No product
code was mutated, so this is not A-38 execution.

The change is consistent across contracts §4.2/§5.2/§5.3/§9.4,
rollback §3–§3.1 and ops §1.5. It does not reintroduce per-folder ordinary
version selection or alter the carried `exclude.scope` wire field.

## R4D-29 and R4D-30 checks

For recovered entry `{core:U}` over `{core}` versus rejected entry
`{core:U,extra:BAD}` over `{core,extra}`, entry/entry remains Unknown.
Actual source checks then yield:

| Actual copies of recovered | Required result |
|---|---|
| Empty | `RecoveredUnavailable`; no advance, no ordinary fallback |
| All proven rejected | Advance to the rejecting records' recovered releases |
| Only unknown, or rejected plus unknown | Wait; no invented rejection proof |
| A proven usable copy | Retain the constraint; use only usable copies |
| Usable plus rejected and/or unknown copies | Retain constraint using the proven usable copy (detailed rollback step 4) |

The model exercises all those sets and a source with a different core
that is not a copy of the recovered entry. It does not prove a real
provider/IO implementation.

For policy reassignment, the model keeps old `auto` r valid under its
listed P0 author A after a covering settled P1 chooses B. A is not
permitted to issue a new auto under P1; B is. A single current authority B
is not `PolicyConflict`, and explicit non-master push/choice/restore
does not fail an issuer-authority test. A genuine disagreement among
current heads still holds automatic apply; “explicit” is not a bypass of
that conflict or of ancestry/consent/scope gates.

The ninth/tenth-head regression remains covered: eight smallest agreeing
heads are a bounded historical witness; all current heads, including a
conflicting head outside that witness, are checked for a policy conflict.
No new field, reinterpretation of issuer sequence or automatic settings
master election was introduced.

## R4D-31: local-only cause, necessity and transitions

**The new `AllHoldCause::RetirementUnresolved` is a cross-crate local
status discriminator, not a between-computers wire variant.** Its complete
new use is in ops §1.6's `HoldScope::AllApplies`, the held-self view in
§2.4 and its named test. Contracts §1.10 adds queue retention/message
behavior, not a field in the retirement record.

Boundary trace checked against the actual slice and amended designs:

1. Core `status.rs` derives local views from admitted records, targets
   and runtime inputs. Actual `StatusView`/status enums have no serde
   derives or `Record` implementation. Runtime `service.rs`'s
   `StatusSnapshot` is a local Debug/Clone/Default value for the app, not
   a shared record.
2. The actual publication boundary uses `PublishContext`/`CodeState`
   and `records::encode`, not `StatusView` or `StatusSnapshot`.
   `CodeState`/`MemberRecord` do not embed `AllHoldCause`. Fix4 changes
   no crate or shared DTO.
3. The authority family/envelope, code-state additions
   (`authority_index`, `features`), retirement fields, settings-state
   results, and journal/recovery-point shapes are unchanged. None gains
   a typed `HoldScope` or this new cause. Existing shared report results
   retain slice `ReportKind` values (contracts §2.4).
4. `pending(retirement_unresolved)` is an already-defined derived ledger
   validity status, not this new view enum encoded into a retirement.
   Mirrored authority bytes remain verbatim. Own-held status stops all
   provider publication; kept choices remain **local and unwritten**.

Thus the amended contract does **not** write the cause into shared
protocol data and needs no schema/capability bump for it. Diagnostics
requested by a user and local Activity text are not a new group protocol.
This is a static design/current-code boundary check, **not** execution
of a future serializer: the new Rust enum is not yet implemented.
Foundation must preserve that separation, rather than serializing
`StatusView` into a shared state wholesale.

**Is it necessary?** Distinguishing held from proven-retired self is
necessary for truthful typed status and different automatic-resume/queue
semantics. Existing `Retired` asserts a settled fact, `Paused` asserts a
user pause, `ActivityUnknown` concerns game activity, and
`UnresolvedJournal` names a different recovery obstruction. Reusing them
would misdescribe the condition. A separate local flag could encode the
same distinction, but this enum variant is the minimal extension of the
already-approved typed hold design, not new product scope.

The model checks queued fixed bytes/seq during a hold, reconstruction of
the local queue after restart, release on Void without rewriting those
bytes, cancellation with an audit count on Yes, and explicit local
cancellation. New text correctly keeps the uncertain headline without
date/by facts, permits queued decisions while preventing publication,
and preserves **Join again…** as the missing-history escape.
Those GUI paths and real durable queue recovery remain implementation
tests, not results produced by the model.

## New finding

### R4D-32 — minor — recovered-chain summary overstates when Unknown blocks

**Location:** contracts §4.2 recovered-chain users row versus rollback
§3.1 steps 3–4, both changed by fix4.

The short common-contract row ends “a copy that waits makes the chain
wait,” without the detailed algorithm's qualifier **none is proven not
rejected**. With recovered scope `{core}`, rejection scope
`{core,extra}`, a safe copy `{core:U}` and another copy
`{core:U,extra:Unstable}`, the detailed step 4 can use the safe copy; the
unqualified summary suggests waiting for the other computer.

**Implementation guidance:** use the detailed four-branch result:
nonempty all-rejected advances; at least one proven non-rejected copy
supplies the existing constraint; otherwise unknown copies wait. Do not
make an unusable redundant copy hold a proven usable recovered source.
Retain the mixed safe-plus-unknown fixture in the rollback packet. The
summary can be tightened to “waits if no usable copy is proven.”

This is a local selection/summary precision issue with a complete
detailed branch already specified. It needs no new shared record,
decision or owner policy and is not another major/design-gate failure.

## Owner audit and queue

**No false new owner decision found.** Fix4 does not edit questions or
OWNER-DECISIONS; all PENDING OWNER labels remain. The bounded “continue”
closes the R4D-28 correction/check authorization question only.

- **Q-49 unchanged:** a live keep pins V; unseen newer/unordered W raises
  `NewVersionAfterDecision`, not automatic replacement.
- **Q-50 unchanged:** resume carries prior declines unless explicitly
  allowed again; it does not newly decline the kept/unseen source.
  Comparing against each exclusion's own scope makes the regrouping
  example implement that rule instead of waiting forever.
- Q-52's retirement cutoff semantics are not changed by this amendment.
  The pending Q-60/Q-61 defaults are not newly owner-decided.
- No renewed R4D-28 owner barrier is required by this bounded result.

Outstanding product questions remain **Q-55, Q-57, Q-58, Q-60, Q-61,
Q-63, Q-64, Q-77, Q-78, Q-81, Q-82, Q-84, Q-85, Q-86, Q-87**.
Their authoritative meanings/defaults remain in
[questions.md](../../../design/questions.md). Dependent acceptance stays
**BLOCKED** pending the relevant answer; the documented conservative
synthetic defaults are not those answers. Q-77 still disables real-data
settings sync, and Q-64/Q-81 remain only provisionally accepted defaults.
No new product question is raised by R4D-32.

## Executed verification

Commands ran in the assigned checkout; cargo's directory was added to
PATH in each fresh shell.

| Command/check | Result |
|---|---|
| `git diff --stat 9e98862 832492d`; bounded per-file diffs | **PASS**, five design/evidence files only; inspected complete per-file patches |
| `git diff --exit-code 9e98862 832492d -- AGENTS.md README.md SPEC.md ACCEPTANCE.md ENGINEERING.md docs\OWNER-DECISIONS.md docs\runs\run-004\PLAN.md docs\design\questions.md crates` | **PASS**, exit 0; authority, questions, plan and implementation unchanged by fix4 |
| `rustc --version`; `cargo --version`; `python --version` | **PASS**, 1.100.0 / 1.100.0 / 3.15.2 |
| `python docs\runs\run-004\evidence\R4-design-recheck4\design_probes.py` | **PASS**, exit 0: 32 release cases in both directions, two expected model-control failures detected, recovered-copy branches, policy cases and held-queue model transitions |
| `cargo xtask verify --tier offline` | **PASS**, exit 0, allow-blocked false: fmt, clippy, test, required-cases, deny and markdown-links all passed |
| Pre-commit ownership and whitespace checks | **PASS**: `git diff --cached --check`; exactly the two assigned evidence paths staged; design, implementation and authority diff against baseline empty |

These are evidence-only written-rule models and the existing baseline
suite. No new product behavior, A-38 production mutation, native/GUI
flow, four-target migration, actual queue crash recovery or performance
acceptance was run. No dependency was installed or changed.

Requirements relevant to the bounded checks: SPEC R-05/R-06/R-07/R-10/
R-12; ACCEPTANCE A-18/A-20/A-24/A-25/A-26/A-45. Product implementation
evidence for those rows is still **NOT RUN** by this packet.
