# Evidence: R4-design-recheck

## Result and scope

**FAIL — do not open the phase C design gate.** Centralizing the contracts
repairs important parts of the original review, but it does not make the
contracts mutually executable or preserve every recorded owner decision.
The author's closure table is not accepted as verification.

Active root findings: **3 blockers** (R4D-1, R4D-10, R4D-17),
**8 majors** (R4D-4, R4D-5, R4D-15, R4D-16, R4D-18–R4D-21) and
**1 minor** (R4D-22). Original R4D-8/R4D-9 remain partially closed,
with their residuals tracked under R4D-18/R4D-21 rather than double-counted.

| Document | Verdict | Principal remaining reasons |
|---|---|---|
| [contracts.md](../../../design/contracts.md) | **FAIL** | R4D-1, R4D-4, R4D-10, R4D-15–R4D-18, R4D-21 |
| [removal.md](../../../design/removal.md) | **FAIL** | mixed-operation journal contradiction; unwriteable recovery authority; cleanup/absent-ack ordering; retirement rules |
| [settings.md](../../../design/settings.md) | **FAIL** | R4D-4, R4D-18, R4D-19; shared recovery shapes and foundation dependencies |
| [rollback.md](../../../design/rollback.md) | **FAIL** | R4D-20, R4D-21; shared journal/recovery contract remains incomplete |
| [ops.md](../../../design/ops.md) | **FAIL** | keep/resume behavior contradicts Q-49/Q-50; retirement authority; shared ownership/order |
| [questions.md](../../../design/questions.md) | **FAIL** | Q-49/Q-50/Q-52 do not describe the implemented-by-design semantics; Q-61 cannot declare those product changes owner-free; Q-69 claims unexecuted proof |

The original R3D-13 **same-id/cross-package conflict remains eliminated**:
different stored bytes have different identities and contribute separate
removals. The removal set algebra passes the repeated permutation probe.
That is not an end-to-end pass: the bounded-history dead end and the new
retirement/constraint rules still provide counterexamples.

### Checkout, authority and constraints

- Reviewed checkout: `D:\GitHub\wow-sync.worktrees\R4-design-recheck`.
- Branch: `run004/R4-design-recheck`.
- Reviewed baseline: `c939b30cf870c299918d71c84747f75416425096`.
- Environment: Windows x64, Windows NT `10.0.26300.0`; rustc
  `1.100.0 (0ef1bc386 2026-11-09)`; cargo
  `1.100.0 (3acc79c0c 2026-11-05)`.
- Read the [fix packet](../packets/R4-design-fix.md), its
  [author evidence](R4-design-fix.md), the new common contract and revised
  designs/questions against [SPEC](../../../../SPEC.md),
  [ACCEPTANCE](../../../../ACCEPTANCE.md), D-21–D-24 in
  [OWNER-DECISIONS](../../../OWNER-DECISIONS.md), COMMON and PLAN.
  The authority documents, plan and crates are unchanged from the
  [original review](R4-design-review.md); a scoped `git diff --exit-code`
  confirmed that, rather than assuming it.
- Only this evidence and the small owned
  [design models](R4-design-recheck/design_probes.py) are added. No design,
  production, product-test, dependency, build or CI change. No subagents,
  provider requests, game data, process manipulation, pushes or rewrites.
- Runtime-model selection remains launcher-controlled, not independently
  introspected by this worker.

## Disposition of every original finding

“Closed” below closes the **reported counterexample at design level**,
not product acceptance or an unexecuted named test. New defects in a repair
are recorded separately instead of silently treating the repair as proof.

| ID / original severity | Status | Independent recheck |
|---|---|---|
| R4D-1 / blocker | **PARTIALLY** | Exact-byte identity, plain authority hashes, common envelope, major-aware decode proposal, own-copy repair and index/ledger definitions now agree. C-JRN still cannot represent its consumers' mixed remove/update operation or all promised recovery outcomes. Details below. |
| R4D-2 / major | **CLOSED** | Obligations and ack items are per `(enrollment, removal, folder)`; a data-only ack does not release core. A subset re-add changes only that folder's boundary. Both arrival orders retain the other obligation. |
| R4D-3 / major | **CLOSED, original trace** | Removal §3.4's `state.seq >= ack.issuer.issued_at_seq` prevents a seq-11 ack from rehabilitating the already-admitted seq-10 inventory. Actual confirmation additionally requires absent folders (§5.6). The author's stronger “first absent state after ack.state_seq” claim is not the written candidate rule; there is no separate `state_seq` field in the new ack envelope. Do not treat that claim as additional proven behavior. |
| R4D-4 / major | **OPEN** | Segmenting the ledger fixes its aggregate 16 MiB ceiling. `Known` moves the write dead end into 512 historical enrollments/128 mirror-only records, and full policies can exceed the common byte limit even with empty `Known`. The new 8-issuer test narrows the original general counterexample. |
| R4D-5 / major | **PARTIALLY** | The baseline now preserves declarations and an origin settings-cleanup op exists. But the step-3b absent-ack predicate does not exclude pending cleanup; it precedes the planned cleanup at step 7. The universal cleanup-first promise and that executable schedule disagree. |
| R4D-6 / major | **CLOSED** | `SettingsUnit {folder, identity}` and folder-keyed exceptions/pause survive package regrouping. Package keys are explicitly local/display-only. Exact identity matching and collision diagnoses are stated. Native identity behavior is still unexecuted, not “proved.” |
| R4D-7 / major | **CLOSED** | Equal checkpoint heads retain the whole frontier. A child parenting both a and b leaves only that child as a head, including replay. Frontier-size exhaustion is a separate R4D-4-style limit, not a representative-collapse repair. |
| R4D-8 / major | **PARTIALLY** | Nodes, missing-parent waiting, invalid-parent ancestry, restore ancestry and zero-node behavior are now defined. Restricting the basis to nodes erases the information required by the retired-basis outcome; equality/vacuity also make target rows overlap. Residual behavior is R4D-18. |
| R4D-9 / major | **PARTIALLY** | The original two concurrent, different recovered targets for the same folder set now lead to `RollbackTargetsDiffer`, not two simultaneous installs. Coverage composition still drops constraints for untouched folders on partial overlap: R4D-21. |
| R4D-10 / blocker | **PARTIALLY** | A real code-choice family replaces deleted removal decisions; Interface is advisory and unusable metadata has no unsafe override. Keep/resume still changes D-21-approved Q-49/Q-50 behavior, and resume has no satisfiable literal V-CHOICE rule. Details below. |
| R4D-11 / major | **CLOSED, original trace** | The explicit monotone retirement barrier leaves B retired after A retires B and C later retires A, in both orders. That fixes reactivation of B. New post-retirement authority defects are R4D-15/R4D-16; closing the original membership trace does not endorse them. |
| R4D-12 / major | **CLOSED, original free-text leak** | Diagnostics no longer copy TOC version/title/notes or raw error text into a supposedly safe wrapper. The remaining name source is an explicitly previewed/toggleable folder-name field. Q-64 remains pending; pattern matching is not anonymization. See deviation decision below. |
| R4D-13 / major | **CLOSED, specified barriers** | Native macOS failure is not waived; file-size approval still gates real settings data; notice suppression/toggles/update cadence are explicitly pending. Defensive caps are separated from product gates. This does not bless the different Q-49/Q-50/Q-52 reinterpretations or the unapproved rollback limitation. |
| R4D-14 / minor | **CLOSED** | Ops §4.3 constructs the link from the exact validated tag, not a normalized version; bare and `v`-prefixed tags retain their distinct paths. |

Original disposition: **8 closed, 5 partially closed, 1 open**. R4D-8 and
R4D-9's residual problems are described under the new R4D-18/R4D-21 roots,
not asserted to be extra independent counterexamples.

## Remaining original blockers and majors

### R4D-1 — blocker — C-JRN still disagrees with its required operations

**Locations:** contracts §§2.2–2.3; removal §§1.5, 5.1–5.2, 6.2–6.3;
settings §8.

1. Removal §5.1 requires one operation mixing `remove`, `replace` and
   `install` for a dropped-folder update. Its journal is kind `remove`.
   C-JRN explicitly validates `kind=remove` only with remove folder
   actions; kind `apply` is always `/1`, which cannot express remove.
   The algorithm then assumes “no staging” and every action remove.
   The required mixed update has no defined valid journal/algorithm.
2. `paired=partial` must name uncaptured settings units as missing, but
   `SettingsRoot` requires a concrete `CheckpointRef` and has no missing
   variant/field. Settings §8 promises to list paused, uncaptured units
   too. No checkpoint exists to put in that required field.
3. The only code-only `/2` recovery shape has `paired=code_only`, but
   removal's not-applicable fallback requires a `/2` removal point with
   `paired=not_applicable`. The only table row admitting `not_applicable`
   is settings-only, with empty code and nonempty settings. A removal of
   code with no applicable settings cannot use that row.
4. C-JRN fixes `pending_records` before commit; removal §5.2 fixes its ack
   in finalize after verification. Both can be designed safely with
   appropriate durability, but they are not the same wire/state-machine
   contract and cannot both be mandatory as written.

**Suggested fix:** give mixed update/remove one valid kind, action
validation and staging/recovery algorithm; define missing-root metadata
and code-with-no-settings shapes; choose the durable point at which
pending record identities become immutable. Add concrete encoded examples
and crash-slot cases for each, not just a “mixed” test name.

Links: R-04/R-10/R-11/R-16; A-09/A-10/A-19/A-20/A-22/A-36.

### R4D-4 — major — causal compression still leaves reachable write dead ends

**Locations:** contracts §§1.8–1.9; removal §3.3 example 9 and
T-R4D4-1024-removals-readd-bound; settings §§3.2–3.3.

- With 1,024 concurrent removals from **1,024 historical enrollments**,
  one `Known` can cover at most 512 issuer prefixes plus 128 individual
  extras: at least 384 removals remain uncovered. The contract explicitly
  returns `NeedsHelp(GroupHistoryTooLarge)` for longer history. Re-enrolling
  does not remove historical enrollments or tombstones.
- Even with eight issuers, 1,024 removals learned only from mirrors cannot
  use `seen` while the issuers' index chains are unavailable. `extra`
  covers only 128. Mirroring was supposed to preserve authority after an
  issuer becomes unusable.
- The new removal test quietly limits its fixture to eight issuers.
  It passes the compression example, not the general “1,024 concurrent
  removals; one restore” promise or the original dead-end objection.
- “Retirement, policy, revision: fixed size” is false. A legal full policy
  with 512 ordinary short folder names, two valid member pairs, empty
  `Known`, and master exceptions is **93,143 bytes** in the independent
  pretty-JSON model, above 65,536. Splitting a *full snapshot* policy into
  folder fragments is not defined and would change absent-folder defaults.

The problem is not that every defensive cap needs owner approval. It is
that these caps change whether a legitimate removal can be undone or a
valid policy can be represented, with no specified supported GUI
resolution. Ledger segmentation does not solve record expressibility.

**Suggested fix:** define byte-aware, semantics-preserving partitioning or
compaction for each affected family, including causal coverage and policy
snapshots; specify offline/mirror-only recovery. For re-adds, multiple
coverage fragments could work with the per-folder union, but that writer
protocol is not specified here. Test the original arbitrary-issuer and
missing-issuer-index cases without narrowing them.

Links: R-06/R-10/R-12; A-10/A-17/A-20/A-25/A-26.

### R4D-5 — major — origin cleanup can be bypassed by the absent-ack schedule

**Locations:** removal §1.6 cycle steps 3a/3b/7, §5.6, §6.3 steps 2–5.

At 3a a passive uninstall matures, stores r and **plans** settings cleanup.
The origin is in r's audience, has no code, no ack and no re-add. Thus every
listed §5.6 predicate for an `absent` ack is already true at 3b, before
cleanup at 7. The special origin sentence only delays the `removed` ack.
An absent ack can settle the same obligation while active settings remain,
including while WoW prevents their movement.

The final §6.3 prose says no ack is early, but the actionable predicate
and cycle do not implement that invariant. The no-owned-items/paused branch
also jumps to baseline clearing without explicitly creating its own ack.
A reviewer cannot infer an extra guard/constructor and call it verified.

**Suggested fix:** classify settings applicability before *any* absent ack;
exclude every folder with pending required cleanup, including origins,
from step 3b. Define the no-op cleanup ack path. Make the named crash test
exercise the actual cycle order with cleanup blocked, not just call the
cleanup executor directly.

Links: R-04/R-10/R-11; A-10/A-15/A-22/A-24.

### R4D-10 — blocker — the replacement choice protocol changes decided intent

**Locations:** contracts §§5.2–5.4; ops §1 and
T-R4D10-resume-declines-nothing; questions Q-49/Q-50; D-21.

- Q-49 still accurately records the accepted default: an unconsidered
  newer W raises `NewVersionAfterDecision`; V stays until explicit choice.
  C-CHOICE §5.3 instead says W ends the keep and **is installed
  automatically**; §5.4 repeats that in the purported R3D-10 retrace.
  Renaming a removal choice to code-choice is not authority to invert it.
- Q-50's accepted rule preserves exclusions of previously declined
  versions after resume. The rewrite redefines “declined” to mean only
  separately rejected releases. For unordered alternatives, keeping one
  records no exclusion of the others, and resume reintroduces them as
  ambiguity. That changes the existing intentional-choice semantics
  without an owner decision.
- V-CHOICE requires `keep` iff action keep, but unconditionally requires
  `considered` to contain `keep.package_hash`. A valid `resume` has no keep,
  so that validation predicate is undefined. The rules must discriminate
  action variants rather than silently special-case one in implementation.

**Suggested fix:** retain Q-49/Q-50 semantics in the new family, with
explicit durable exclusion/allow-again behavior and conditional validation,
or obtain a new owner decision that supersedes them. Rerun the *original*
traces, not traces that first add a rejection the user never made.

Links: D-21; R-06/R-09/R-12; A-14/A-20/A-25/A-26.

## New findings introduced by the repair

### R4D-15 — major — V-RET's normative boolean exempts the wrong records

**Location:** contracts §1.11 V-RET versus §1.10 steps 1 and 3.

The table checks `not a retirement OR issuer not retired OR seq <= cutoff`.
For an ordinary removal/policy/revision from a retired issuer, its first
clause is true even above the cutoff. For a retirement from a retired
issuer above cutoff, the predicate is false, contrary to the prose's
retirement exemption. The model accepts ordinary seq 11 after cutoff 10.

This is a contradictory validation contract, not a claim that the
unimplemented product already executes that table.

**Suggested fix:** align the predicate and tests with the chosen approved
retirement rule; if retaining the stated exemption, its first clause
would be “is a retirement,” not “not a retirement.” Do not stop at that
text fix: R4D-16 challenges the exemption itself.

Links: R-03/R-11; A-20/A-45.

### R4D-16 — major — monotone retirement now admits post-retirement authority

**Locations:** contracts §1.10; removal §8.2; ops §2.1; questions Q-52/Q-61.

Two independent traces survive even if R4D-15's boolean is corrected:

1. A retires B at cutoff 10. B's seq-11 removal is invalid and excluded.
   Later an offline C's retirement of B with cutoff 12 arrives.
   `MAX(10,12)` makes that removal valid again. An offline/post-barrier
   change becomes group authority without re-enrolling B.
2. After B is retired, a new retirement issued by B above its cutoff
   still retires an active A, because **all** retirement records are
   exempt. The rule has no evidence distinguishing a delayed pre-barrier
   retirement from a new act of the retired enrollment.

Monotone *retired membership* fixes R4D-11, but does not require validity of
all later authority or an ever-widening authorization cutoff. D-21
explicitly kept Q-52 because post-retirement actions are not authority.
Q-61 cannot deem this broader permission a mere engineering detail.

**Suggested fix:** separate the monotone retirement fact from admissibility
of acts by retired issuers; define deterministic causal evidence/cutoffs
that do not reauthorize post-barrier actions. Keep retirement membership
monotone without silently changing D-21. Any intentional permission change
must go to the owner.

Links: D-21/Q-52; R-03; A-20/A-45.

### R4D-17 — blocker — the phase C ownership graph cannot follow its build order

**Locations:** contracts §9–§10; settings §10; rollback §9; removal §10.

The table improves accountability but its claimed independent foundation
cannot land as specified:

| Foundation obligation at step 1 | Dependency assigned to a later step |
|---|---|
| C-JRN's `SettingsFileOp`, `/2` recovery roots, settings-only/mixed golden fixtures | `SettingsUnit`, `Identity`, `CheckpointRef`, `SettingsRoot` owned by settings at step 3 |
| Family-wide validation V-ISSUER/V-RET | historical membership and retirement barrier owned by ops at step 2 |
| T-C01 “one golden file per kind,” T-C01–T-C07 all required before anything else merges | code-choice/retirement kinds at step 2; policy/revision/rejection/lift and settings ops at step 3 |

Settings and ops in turn consume C-AUTH/C-JRN from the foundation. The
documents provide no earlier shared-types slice, versioned extension API,
or contract-owner delivery of those later-owned definitions to break the
cycle. A “compose skeleton” does not solve the required typed journal and
golden fixtures.

The integration maps also still assign later workers changes to
`recover_interrupted`, `plan`/`exec`, runtime `control` and authority-kind
dispatch, while §9 says consumers do **not** extend the same records or
runtime methods. Same-file sequential work could be safe, but the actual
handoff/API/method ownership is not stated; labeling a row “consumes” does
not make those edits disappear.

**Suggested fix:** approve a topologically ordered packet map. Land all
shared DTOs and wire variants with one foundation owner first, separate
family body/rule ownership from dispatch integration, and assign concrete
executor/runtime extension ports or serial integration handoffs. Move
consumer-dependent tests to the stage that can actually build them,
without claiming unfinished placeholders are a passing foundation.

Links: E-05/E-08; A-37/A-39/A-43/A-44.

### R4D-18 — major — settings target rows overwrite local differences

**Location:** settings §4.2, basis definition and outcomes table.

For one head h and basis `{h}`, ancestry explicitly includes equality.
With a changed local file and no new shared revision, the “B ancestor of
every head” **yield** row matches. The following `B == heads` /
`LocalDifferent` row also matches but is later. A literal first-match
implementation overwrites a routine laptop experiment with the unchanged
shared checkpoint; a non-ordered interpretation still has two conflicting
targets.

For `B={}`, the universal ancestry predicate is vacuously true, so the
first-adoption row also overlaps yield. Worse, “restrict B to nodes” turns
a wholly invalidated retired basis into `{}`, losing the distinction from
first adoption. The claimed `BasisNotShared` row requires B nonempty and
cannot handle that named test.

**Suggested fix:** preserve original basis provenance/invalid status;
define mutually exclusive cases for no basis, invalid basis, equal
frontier and strictly newer descendant. Require strict progress before
automatic yield, and put local differences/consent ahead of it. Add the
unchanged-head and wholly-invalidated-basis negative cases.

Links: R-08/R-09; A-12/A-14/A-20/A-45.

### R4D-19 — major — authority epoch equality can publish a preexisting experiment

**Location:** settings §§2.1–2.3, 4.1; T-R4D-startup-after-reassign.

The last complete local capture is seq 5. The app closes; the laptop's
settings become an unshared experiment. Elsewhere the laptop is made
master. On startup it learns that policy and records
`authority_since=current local_seq=5`. Its complete `found_at_start`
capture differs from capture 5, so `changed_in_session=true` and
`compared_with_local_seq=5`. The guard `5 >= 5` permits automatic
publication, although the compared checkpoint was taken **before** the
authority epoch. The stated “never publishes that experiment” conclusion
does not follow from the specified counter.

**Suggested fix:** establish a capture/baseline boundary after authority
activation, with preexisting differences explicitly unshared. Record enough
ordering to prove a comparison capture belongs to the epoch; do not treat
the current pre-epoch sequence as proof of that. Preserve explicit push
and the first genuine post-activation session.

Links: R-07/R-08; A-11/A-12/A-20.

### R4D-20 — major — ordinary folder-changing releases cannot be rolled back

**Location:** rollback §1 step 1 and T-R20.

The rewrite disables group rollback unless R0 and R1 have **identical
folder keys**. A normal update adding or dropping an addon-suite folder
therefore makes its retained earlier release unrecoverable through the
promised “Go back” action. The newly specified test enforces the
restriction instead of verifying recovery across the release change.
Neither R-10/A-18 nor D-21–D-24 grants that limitation.

Removal already has per-folder removal/re-add authority; the rollback
design must compose with it rather than silently narrowing the product.
A safety refusal is preferable to a destructive attempt, but cannot count
as implementing the approved rollback requirement.

**Suggested fix:** specify and test rejection plus exact recovered-folder
set restoration, including necessary removals/re-adds and settings
pairing, in one recoverable flow. Otherwise obtain an explicit owner scope
deferral before implementing this as the supported product behavior.

Links: R-10/R-12; A-09/A-10/A-18/A-19/A-25/A-26.

### R4D-21 — major — partial overlap supersedes an entire wider constraint

**Locations:** contracts §§4.2, 5.3; rollback §3.2; C-IDS §3.1.

X constrains `{a,b}` to `{A1,B1}`. After regrouping, a valid Y explicitly
chooses only `{a}`, with `known` covering X. The stated test for
supersession is **overlap**, so X stops being live as a whole; Y says
nothing about b. Ordinary selection can now change b to B2, although no
choice/re-add/lift ended the constraint for b. Different local grouping is
explicitly allowed and cannot change durable meaning.

Saying composition is “per folder” in §4.1 is not enough: §4.2 defines
liveness of the record using any overlapping folder, and the manifest/hash
projection of a partially live constraint is never specified.

**Suggested fix:** define constraint liveness, supersession and target
projection per folder, preserving uncovered scope; or define atomic scope
rules that safely hold incompatible overlapping choices. Test partial
choice-over-choice, choice-over-rollback and re-add-over-rollback with
different local package groupings, not only identical folder sets.

Links: R-06/R-10/R-11; A-18/A-20/A-23.

### R4D-22 — minor — Q-69 claims platform proof that this work has not produced

**Locations:** questions Q-69; contracts §3.3/T-C11; settings T-S17.

Q-69 says identity matching/collision behavior is “Proved on NTFS and
APFS.” The new identity types/normalization logic and the named tests are
not implemented; the design calls for future native tests. Existing
offline slice tests cannot prove the rewritten identity algorithm, and
this worker ran no APFS environment.

**Suggested fix:** replace the proof assertion with an explicit future
test obligation/NOT RUN status until linked native evidence exists.

Links: E-08; A-23/A-34/A-37.

## Real crates/validators: what was and was not established

Direct source inspection of
[records.rs](../../../../crates/wow-sync-core/src/records.rs),
[plan.rs](../../../../crates/wow-sync-core/src/plan.rs),
[exec.rs](../../../../crates/wow-sync-core/src/exec.rs),
[runtime service](../../../../crates/wow-sync-runtime/src/service.rs) and
[runtime effects](../../../../crates/wow-sync-runtime/src/effects.rs)
supports these **existing slice** facts:

- `Record`/`record!` currently has one `MAJOR=1`, not accepted-majors or
  per-value write/validation hooks. Existing apply journals require code
  folders; `FolderAction` currently has only install/replace. The proposal
  recognizes these required migrations; none has happened yet.
- `decode` rejects an unknown major before normal record validation.
  `recover_interrupted` retains undecodable journals, emits
  `NeedsHelp(UnknownInterruptedState)` and reports journals remaining.
  Runtime skips new applies and publication while that flag remains.
  This supports the *core blocking* part of the downgrade design.
- `decode_points` skips undecodable recovery points; the slice's ordinary
  prune list is built from successfully decoded points. Thus `/2` points
  are not pruned by that path.
- `effects::load_pending` turns decode failures into no pending report.
  This supports the specifically documented pending-report replacement
  exception, not the blanket phrase “every undecodable record is shown.”
- `service::journals` filters failed decodes from its typed journal list.
  Therefore the common contract's “runtime decoders: kept and shown”
  cannot be proved merely from that decoder: the recovery report/status/
  GUI paths must be exercised end-to-end. No installed downgrade GUI
  recovery was tested here.
- The new `/2` shapes, cross-kind validators, new identity types and
  major-selection hooks do not exist in these crates. T-C01–T-C15 are
  obligations, not executed tests. The still-contradictory C-JRN contract
  cannot be declared validated by a green baseline suite.

This is a design recheck, not implementation acceptance. In particular it
does not claim A-38 negative controls, actual `/2` crash fixtures, native
NTFS/APFS identity behavior or aggregate-memory measurement.

## Repeated counterexamples and probes

The committed Python file contains **14 independent written-rule models**.
Successful assertions establish either that the old counterexample no
longer follows from the revised rule, or that a stated bad outcome remains
constructible. They do **not** execute future product code, qualify as
product tests, or replace its required-cases/A-38 evidence.

| Probe/trace | Result against revised text |
|---|---|
| Same immutable removal algebra, reordered/duplicated | PASS for fixed valid ledger: all 24 permutations and duplicate replays give the same removed set |
| Partial ack leaves untouched folder owed | PASS, original R4D-2 trace repaired |
| Seq-11 ack versus old seq-10 inventory | PASS, original R4D-3 trace repaired |
| Equal heads a/b followed by child parenting both | PASS, original R4D-7 trace repaired |
| A retires B, then C retires A; both orders | PASS for monotone retired-membership set, original R4D-11 trace repaired |
| 1,024 removals from eight issuers with readable complete indexes | PASS, author’s narrower compression example works |
| 1,024 historical issuers; unavailable issuer indexes; full policy body | FAIL design expressibility: Known cannot cover all; policy model 93,143 bytes |
| Absent-ack stage before pending settings cleanup | FAIL design schedule: listed absent predicates do not test cleanup |
| Literal V-RET above cutoff | FAIL design predicate: ordinary authority exempted |
| Later higher retirement cutoff | FAIL authority boundary: formerly invalid seq 11 becomes valid |
| Unchanged singleton settings head / empty restricted basis | FAIL targets: both hit yield before their protective cases |
| Startup reassignment with preexisting experiment | FAIL epoch predicate: compared seq 5 satisfies epoch boundary 5 |
| Narrow choice supersedes wider constraint | FAIL uncovered scope: b loses X despite Y naming only a |

The R3D-13 review therefore has three distinct answers:

1. **Unresolvable id-variant state:** original mechanism gone; fixed.
2. **Order-dependent merge:** fixed-ledger removal set remains invariant;
   no claim that all new validity/application transitions are proven.
3. **Offline resurrection/reachable dead end:** original partial-ack and
   older-inventory paths are repaired, but R4D-4/R4D-16 leave an
   unwriteable restore and reauthorized retired authority. The group gate
   cannot pass.

## Deliberate deviations

### Q-81 stricter ending rule: ACCEPT as a pending-owner conservative default

No timestamp or first observation proves a release was created after a
rollback. Keeping the recovered target until an **explicit** lift or
choice is safer than silently inferring that proof. The nonblocking
newer-release notice supplies an explicit path. This acceptance is of the
engineering fallback in the packet, **not** an owner decision and not
permission to mark dependent acceptance passed. Q-81 stays PENDING OWNER.

It does not excuse C-CHOICE's opposite handling of an ordinary keep under
the already-decided Q-49.

### Q-64 plain-pattern folder names: ACCEPT the narrowing, not an anonymity claim

Limiting the optional source to previewed folder names, excluding TOC
titles/version labels/free text, is a reasonable privacy-reducing narrowing
of the packet's names-with-checkbox default. Keep the checkbox and actual
preview, and keep Q-64 PENDING OWNER.

A matching folder name can itself be private or arbitrary untrusted text;
`PLANTED_SECRET_DO_NOT_EXPORT` also matches the pattern. It is exported
only under the explicitly disclosed name option, not because a regex
proved it nonsensitive. The author evidence's “only name source not
otherwise free text” rationale is not accepted. Turning names off must
remove that source entirely; the negative planted-label cases still
belong in the eventual product test.

## Owner-decision audit and phase C owner queue

**Cannot confirm the blanket claim that no undecided behavior is treated as
decided.** The owner register itself is unchanged, and the fourteen marked
PENDING OWNER rows are honestly pending. However:

- Q-49/Q-50 retain “Owner, D-21: accepted” while appending run-004
  interpretations that contradict or narrow those accepted choices.
- Q-52 still says post-retirement acts have no authority, while the
  normative barrier reauthorizes them; Q-61 calls that “not an owner
  question.”
- Identical-folder-only rollback is neither a recorded owner decision nor
  a clearly owner-gated scope deferral.
- Q-69's unsupported proof claim is an evidence error, not a new owner
  approval.

The engineering defects above should be repaired by designers, not handed
to the owner as algorithm questions. Before phase C uses **changed product
meaning**, either retain the existing intent or obtain these explicit
owner decisions:

| Issue | Specific owner question only if the deviation is retained |
|---|---|
| Q-49/Q-50 | May a previously kept choice be automatically replaced by new information, and may resuming forget earlier declined alternatives? Existing answer is no; restore it unless superseded explicitly. |
| Q-52/Q-61 | May later retirements widen an earlier authorization cutoff, or may a retired issuer create new retirement authority? D-21 says post-retirement acts are not authority. |
| Rollback scope, R4D-20 | Is recovery across a release's folder additions/drops explicitly deferred? Otherwise the design must support it; there is no approved identical-folder exception. |
| History limits, if treated as a supported-product limit | May an otherwise valid group permanently lose GUI restore/policy actions because of accumulated enrollments/causal heads? Otherwise specify recovery/partitioning, not just a diagnostic. Defensive byte limits alone are not this decision. |

Carry the following existing pending questions to the phase C barrier:

| Question | Pending answer / default under review |
|---|---|
| Q-55 | explicit add-back only; no automatic re-add on reappearance |
| Q-57 | local “Continue without it,” with risk/recovery disclosure |
| Q-58 | remove only the named folders |
| Q-60 | retired self stops publishing/applying, retains data, rejoins explicitly |
| Q-63 | native macOS notifications required; fallback is not a pass |
| Q-64 | previewed optional plain-pattern folder names included by default |
| Q-77 | choose settings-file size parameter; until approved, real-data settings sync disabled |
| Q-78 | exact-content release rejection, same-label/different-content hold |
| Q-81 | stricter explicit lift/choice ending rule, no inferred release-creation time |
| Q-82 | no settings-checkpoint rejection; ordinary restore only |
| Q-84 | block group rollback until non-retired peers support rejection |
| Q-85 | 6-hour repeat-notice suppression; never `DataAtRisk` |
| Q-86 | OS `DataAtRisk` obeys notification toggle; always visible in app |
| Q-87 | automatic update-check cadence, default at most once per 24 hours |

The fix packet permits synthetic implementation/testing of pending
defaults; it does not require inventing answers while the owner is absent.
Dependent acceptance remains BLOCKED, and explicitly disabled real-data
features remain disabled. Separately, this independent **design FAIL**
keeps phase C's design prerequisite unmet.

## Commands, execution and limitations

Every command below ran in the assigned worktree, not the primary checkout.
Fresh Cargo shells prepend the documented user Cargo-bin directory to PATH.

| Command/check | Result |
|---|---|
| `git rev-parse HEAD`; `git branch --show-current`; `git status --short` | PASS: assigned baseline/branch; clean before owned evidence was created |
| `git diff --exit-code 9489780 c939b30 -- AGENTS.md README.md SPEC.md ACCEPTANCE.md ENGINEERING.md docs\OWNER-DECISIONS.md docs\runs\run-004\PLAN.md docs\runs\run-004\packets\COMMON.md crates` | PASS, exit 0: approvals/requirements and actual implementation unchanged from original review |
| `rustc --version`; `cargo --version`; OS version query | PASS: actual environment recorded above |
| `python -B docs\runs\run-004\evidence\R4-design-recheck\design_probes.py` | PASS, exit 0: final 14 model assertions; expected counterexamples explicitly reported, not product acceptance |
| `cargo xtask verify --tier offline` | PASS, exit 0: fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false)` |
| Native/macOS/APFS, installed GUI/downgrade recovery, real client/provider, memory/start-time gates, new-feature A-38 controls | NOT RUN: no corresponding implementation/platform run in this packet |

No conclusion above relies on an author completion claim, a test name
without execution, or a passing baseline suite proving the future design.
