# R4-design-review — independent phase-B review

## Verdict and identity

**Phase C: BLOCKED.** The original R3D-13 id/variant deadlock is eliminated
inside the rewritten removal algebra, but the complete design does not yet
meet the D-23 review gate. Counterexamples below include offline
resurrection, an unwriteable re-add, incompatible shared formats and an
undefined decision protocol. Author completion statements were not accepted
as evidence.

| Document | Verdict | Blocking/major findings |
|---|---|---|
| `docs/design/removal.md` | **FAIL** | R4D-1 through R4D-6; R4D-11 interaction |
| `docs/design/settings.md` | **FAIL** | R4D-1, R4D-5 through R4D-8; R4D-11 interaction |
| `docs/design/rollback.md` | **FAIL** | R4D-1, R4D-8, R4D-9; R4D-10/R4D-11 interactions |
| `docs/design/ops.md` | **FAIL** | R4D-1, R4D-10 through R4D-13 |
| `docs/design/questions.md`, Q-54–Q-84 | **FAIL** | R4D-4, R4D-10, R4D-13 and owner barriers below |

Totals: **2 blockers, 11 majors, 1 minor**.

- Checkout: `D:\GitHub\wow-sync.worktrees\R4-design-review`.
- Branch: `run004/R4-design-review`; reviewed source:
  `9489780292133f72cce4661677d28939cc0bee2e`.
- Environment: Windows x64, Windows NT `10.0.26300.0`; rustc
  `1.100.0 (0ef1bc386 2026-11-09)`; cargo
  `1.100.0 (3acc79c0c 2026-11-05)`.
- Independent verification packet: R4-design-review, assigned model
  `gpt-6.1-sol`. Actual model selection is launcher-controlled; this worker
  has no independent runtime-model introspection. Assignment is not offered
  as proof of the effective model.
- Read AGENTS, COMMON, packet, approved PLAN, D-21–D-24, SPEC and ACCEPTANCE
  in the requested order; also README, ENGINEERING, all four canonical
  designs, Q-54–Q-84, both R3 rechecks, the three authors' evidence files,
  and relevant current core definitions.
- No design/product/test file edits, subagents, game data, provider calls,
  process stopping, pushes or history rewrites.

## Method and six-check outcome

The retained [design probes](R4-design-review/design_probes.py) are small,
independent transcriptions of specified rules, **not executions of a removal,
settings or retirement implementation**. Their successful assertions confirm
that a specified bad outcome can be constructed; they do not pass acceptance.
The ordinary Rust suite validates the existing slice only.

| Packet check | Result and independent reasoning |
|---|---|
| 1. R3D-13 closure | Exact-byte content identity eliminates the original foreign-package id conflict. Set merging is order-independent for a fixed valid ledger: all 24 permutations of four records, with duplicate replay, give the same M. However R4D-2/R4D-3 break end-to-end anti-resurrection, and R4D-4 gives an unresolvable reachable state |
| 2. Settings/rollback | Explicit master reassignment, routine yield with recovery, whole-checkpoint choices, separate three-state histories, same-identity writes, opaque Lua bytes and per-rename WoW gates are stated correctly. They are not behavioral proof. Settings ancestry, package identity, source-side removal and concurrent rollback are incomplete: R4D-5–R4D-9 |
| 3. Ops | UQ-1 reconfirmation promotion and UQ-2 typed all-apply/package holds are plausible against current `status`/`plan` types. Decision persistence still relies on removed mechanisms. Retirement can reverse itself; redaction admits untrusted strings; notification fallback requires an owner: R4D-10–R4D-13 |
| 4. Cross-document consistency | **FAIL**: two authority identities/hash algorithms, different reference types, contradictory journal/recovery majors, obsolete decisions and missing shared ownership. See R4D-1/R4D-10 |
| 5. Implementability/test obligations | Existing `records`, `target`, `plan`, `exec`, `admit`, `toc`, `package`, `status`, `view`, `paths` and `layout` are plausible extension points. Proposed modules are not falsely claimed to exist. Parallel packets are not independent yet; ownership/interfaces must be frozen below. Named tests cover ordinary cases but omit the counterexamples in this review |
| 6. Questions | All Q-54–Q-84 reviewed. Existing intent supports many conservative defaults, but “follow defaults” cannot approve numeric gates or substitute in-app status for required native notifications. Specific owner requests follow |

### Prior findings: independently checked disposition

| Earlier finding | Disposition against the new source |
|---|---|
| R3D-1, R3D-2 | Historical Interface-validation findings; no Interface-based gate exists in the removal rewrite. The current D-21 rule supersedes the need for that gate, not evidence that every earlier implementation remains unchanged |
| R3D-3 | Superseded by D-21's express no-Interface-gating policy. Ops' new Interface-based choice is inconsistent with that policy (R4D-10) |
| R3D-4, R3D-5 | No new Forever/Era substitution found. Removal E3 uses confirmed variant/mapping and explicitly does not equate the changing inventory assessment digest |
| R3D-6 | New-removal planning correctly refuses any outstanding journal (§5.1); no journal continuation promise is accepted |
| R3D-7 | Separate durable baseline, persistence before publication, insertion at 3a and local hold preserve the original pruning fix |
| R3D-8 | Durable local copies, mirroring on learning and restart reload preserve the lost-carrier fix in principle. Repair of an already-existing bad provider file needs a specified safe replacement operation: write-if-absent alone cannot repair it |
| R3D-9 | Exact manifests and hash-checked records fix the reported compact-manifest/identity defect within removal. The other designs have not adopted that identity (R4D-1) |
| R3D-10 | Removal no longer chooses versions; its specific override mechanism is gone. Ops still needs a replacement keep/resume protocol, not a reference to the deleted one (R4D-10) |
| R3D-11, R3D-12 | Documents state design/GUI obligations rather than claiming implemented acceptance. Earlier bounded UI/layout closures are not reopened; no GUI acceptance was executed here |
| R3D-13 | **Original defect closed locally**: different bytes are separate removals; neither same-id variants nor a cross-package resolving decision exists. **D-23 end-to-end gate not passed**: R4D-2–R4D-4 |
| R3D-14 | Per-package baseline snapshot references preserve the fix; refreshing Q does not erase P's rename evidence |
| R3D-15 | Permanent metadata and live-removal payload roots are distinguished; superseded payload no longer roots itself forever. Metadata still hits finite protocol limits (R4D-4) |
| R3D-16 | No removal declined-set/resume formula remains. Ops' attempted reuse is undefined rather than a demonstrated preserved fix (R4D-10) |

## Findings

### R4D-1 — blocker — there is no common authority or recovery wire contract

**Location:** removal §§2.1–2.6, 10–11; settings §§3.2–3.4, 10;
rollback §2; ops §§1.4, 2.1–2.2, 6.

Removal uses `authority\<64-hex SHA256(exact bytes)>.json`, no independent
id, scalar hash references, `group_id` and `{hash, kind, folders}` state
refs. Settings explicitly prescribes `authority\<32-hex id>.json`,
domain-prefixed canonical hashing and `{id, hash}` refs; rollback follows
that older family. Retirement also has id variants and ignores conflicted
ids, a state the removal rewrite intentionally removed. Removal V4 accepts
only removal/re-add/ack dependencies, not policies/revisions/retirement.
These are incompatible bytes and validation rules, not harmless renames.

Removal extends `apply-journal/1` and `recovery-point/1`; settings requires
`apply-journal/2`/`recovery-point/2` “as introduced by removal”. It also
claims a removal-intent major change and `none_in_this_version`, neither of
which exists in the rewrite. Current `records::record!` fixes major 1;
the existing journal validator rejects empty `folders`, so a settings-only
journal needs an explicit validation/schema change.

**Suggested fix:** publish one common schema/hash/ref/ledger/issuer/
capability contract and compatible journal/recovery specification before
splitting implementation. Choose downgrade behavior that cannot ignore
pending settings operations. Make cross-kind decode, reference validation,
old/new readers and settings-only journal recovery explicit tests.
Links: R-11/R-16, A-20/A-22/A-36, E-05/E-08.

### R4D-2 — major — acknowledging part of a removal releases untouched stale folders

**Location:** removal §§2.3 (ack), 2.6 V6/V7, 3.4, 4.1 levels 3–4, 5.6.

Let r remove `{core,data}`, with offline C in its audience holding newer
copies. A explicitly re-adds only `core`, superseding r for that folder.
C learns both together: M is `{data}`. Level 3 removes only C's `data`;
finalize acknowledges r. Ack validity requires no per-folder coverage.
C's untouched old `core` now owes nothing because the ack lists r.
It is a candidate and can defeat A's re-added version as newer. Delivering
r first instead removes both folders, so this is also order-dependent
application even though M is deterministic. The retained probe reproduces
the release of the untouched folder.

**Suggested fix:** track obligations and acknowledgments per
`(enrollment, removal, folder)`, or complete every owed folder before an ack
can discharge r; distinguish partial re-adds similarly. Test both delivery
orders and the actual final candidates/recovery files, not only M.
Links: R-06/R-10, A-10/A-20.

### R4D-3 — major — an ack can rehabilitate an older published stale inventory

**Location:** removal §§3.4, 4.1 observation stripping, 5.2 finalize, 5.6;
publication's independent file delivery/admission model.

C's admitted seq 10 contains offline v2. After removing it, C issues an
ack at seq 11. Deliver that immutable ack to B before C's absent seq-11
state. If the removal has been superseded by a re-add, B sees M empty and
C no longer owing, so its still-admitted seq-10 v2 becomes eligible again.
There is no rule requiring a candidate state to be at/after the ack or
causally after the re-add. Confirmation's absent-state check does not
protect target selection. Probe reproduces this eligibility.

**Suggested fix:** bind obligation discharge to the candidate's state
generation/causal evidence, rejecting older inventories even after the
enrollment ack arrives. Test metadata-before-state delivery, restart,
duplicates and a full v2 reinstall attempt.
Links: R-06/R-10/R-12, A-10/A-20/A-24.

### R4D-4 — major — finite record limits make a required re-add unwriteable

**Location:** removal §§2.3 `supersedes`, 2.4–2.6, 3.5, 11, 13.1;
questions Q-55/Q-56.

Accumulate 1,024 valid live removals of one folder, for example concurrent
uninstalls across enrollments before the records reach one another. Each
record is small and valid. Restore must list **every** live overlapping
removal in its re-add. The compact JSON `supersedes` field alone is
68,624 bytes in the probe, exceeding V1's 65,536-byte record limit before
other required fields or acks. The writer cannot emit the promised valid
restore. The state/ledger are also grow-only but capped at 16 MiB, with
no rollover/compaction path; repeated normal operations eventually stop
publication. “Every reachable state resolves” is false.

**Suggested fix:** define bounded causal summaries, chunked supersession
or another protocol that remains writable without forgetting removal
authority. Specify behavior before ledger/state bounds, including an
actionable GUI remedy. Test exact serialized limits and many settled cycles.
Any new user-visible/resource gate needs the specified approval.
Links: R-06/R-10/R-13/R-14, A-17/A-20/A-26.

### R4D-5 — major — passive uninstall does not remove its origin's saved settings

**Location:** removal §§1.2, 1.6 step 3a, 2.3 issuer ack, 4.1, 5–6;
settings §8.

Uninstall P with an updater on A. D-21 says WoW leaves its saved variables.
At observation maturity, A writes the removal and its own `removed` ack,
then clears P's presence baseline. There is no live code folder left, so
level 3's live-folder intersection cannot schedule the remove operation
whose settings moves are defined in §5. A can claim removal while its
owned active settings remain forever. Also, settings §8 expects the
baseline to retain `.toc` ownership declarations; removal §1.6 specifies
names/trees/package hash/snapshot, not those declarations or a durable
readable metadata source after the folder disappears.

**Suggested fix:** journal settings-only cleanup on the observing computer
before its removal ack, preserving the ownership evidence and paired
checkpoint before baseline clearing. A missing code folder must not skip
applicable settings. Add origin-side updater uninstall, folder drop and
crash-before-ack filesystem tests; preserve paused/unowned exceptions.
Links: R-04/R-07/R-10, A-09/A-10/A-22.

### R4D-6 — major — local package keys cannot safely identify shared settings policy

**Location:** removal vocabulary/§4.1 stripping; settings §§0, 3.1–3.2,
5.1, 8; rollback §§2.1, 6; current `package.rs::group_packages`.

Removal expressly makes packages local groupings, permits different
folder sets, and regroups after stripping removed/owed folders. Settings
units, checkpoints, per-addon masters and `not_shared` exceptions are
nevertheless keyed by that local package key. Current grouping chooses
`members[0]` as the key. A suite `{a,b}` is unit `a`; after folder `a` is
dropped, surviving `b` is unit `b`. Its policy exception, pause,
checkpoint history and code/settings associations no longer match.
For example `not_shared(a)` can become default-master sharing of b.
Different peers can disagree on these keys even before a removal.

**Suggested fix:** specify stable logical addon/settings ownership and
explicit split/merge/drop migration rules, or use stable folder-scoped
units consistently. Never let regrouping silently grant sharing authority,
lose pause protection or mispair recovery. Give the shared identity type
one owner and test unequal grouping plus removal/re-add.
Links: R-02/R-07/R-08/R-10, A-09/A-11/A-15/A-18/A-20.

### R4D-7 — major — equal-head collapse invents a settings conflict on the next change

**Location:** settings §§0, 3.2 `parents`, 4.1–4.2, 4.4.

Concurrent a and b choose the same checkpoint. §4.2 treats them as one
state with the smallest-hash representative a and records basis a.
The master's next legitimate change c parents only a. b is still a head,
now with a different checkpoint from c, so the protocol asks for a choice
against the already-accepted old state. The probe produces heads `{b,c}`.
The design's same-choice-convergence test T-S11 stops too early.

**Suggested fix:** preserve the entire accepted causal frontier when
equivalent heads coalesce; parent all equivalent heads or define a
deterministic equivalence-aware ancestry rule. Extend T-S11 through the
next changed session, push, replay and two subsequent resolutions.
Links: R-06/R-08/R-09, A-13/A-14/A-20.

### R4D-8 — major — settings target has no complete ancestry/dependency semantics

**Location:** settings §§3.2, 3.4, 4.2; rollback §5.

For a single incoming head, §4.2 defines equality, ancestor, same-basis and
no-basis, but no case where a non-null basis is not an ancestor. This is
reachable when a child arrives before its parent, when a basis revision
becomes invalid after retirement, and when §5 of rollback invalidates a
rejected checkpoint that is still the parent of the corrective restore.
The designs do not specify parent/policy dependency admission, fetching,
zero-head results or whether rejected nodes remain structural ancestry.
Malformed/late data can therefore produce an undefined target or a false
root/conflict instead of an actionable wait.

**Suggested fix:** define the graph's structural nodes separately from
selectable checkpoints; validate units/policies/parents and handle missing,
invalidated and rejected ancestors without forgetting causality. Enumerate
zero-head and non-ancestor-basis outcomes and GUI recovery. Add
child-before-parent, retired-basis and undo-then-next-session tests.
Links: R-06/R-09/R-10/R-16, A-14/A-18/A-20/A-23/A-45.

### R4D-9 — major — concurrent rollbacks can impose incompatible target constraints

**Location:** rollback §§2.1–2.3, 3.1, 9; settings/ops decision interfaces.

A rolls bad R3 back to R2 while offline B rolls the same R3 back to R1.
Both rejection objects are valid and active; each lists all known
candidates. §3.1 says the target is its recovered release. It specifies no
causal winner, merge, decision or satisfiability rule when both constrain
the same addon. `selection_constraint(...) -> Option<Constraint>` cannot
express that competition. Iterating objects to select one would violate
R-06; asking the user is not defined by the advertised decision protocol.

**Suggested fix:** specify composition/supersession for multiple active
rollback selections separately from persistent rejected hashes. Surface
incompatible choices as a typed GUI decision, preserving both recovery
options. Test same/different recovered versions, late delivery, unavailable
targets, concurrent lifts and rollback versus removal.
Links: R-06/R-09/R-10, A-18/A-20/A-25/A-26.

### R4D-10 — blocker — ops depends on decisions the removal rewrite deleted

**Location:** ops §§1.2–1.5, 6–7; removal §§0, 4.3, 13; rollback §6;
questions Q-49/Q-50/Q-78/Q-81/Q-84 interactions.

Ops records decisions through removal §3.3, `chose.keep`, `declined`,
`knows`, the former V1–V7 and removed decision kinds. New removal §3.3 is
only worked examples and supplies no decision format/validity/selection
algorithm. Thus even ordinary ambiguous-code selection, changing a choice,
resume and allow-again have no implementable wire contract. Rollback
assumes a keep constraint exists but does not define it either.

Ops also describes `NoCompatibleToc` as insufficient Interface support and
requires “Use it anyway”, contrary to D-21's no-Interface-withholding rule.
Current `target.rs::folder_problem` uses that enum for unusable folder/TOC
evidence, not simply Interface support: a generic override cannot safely
turn unreadable or unvalidated content into an installable copy.

**Suggested fix:** design a distinct code-choice authority family, including
total keep/resume/allow formulas, concurrent choices and rejection/removal
precedence. Remove obsolete removal choices. Distinguish advisory
Interface text from actual unsafe/missing metadata and guided layout
validation; do not reinstate addon policing or bypass R-02. Re-run the
R3D-10/R3D-16 traces against the new choice protocol.
Links: D-21/D-23, R-02/R-05/R-06/R-13, A-08/A-20/A-25/A-26.

### R4D-11 — major — a late retirement can silently un-retire another computer

**Location:** ops §§2.2 R3, 2.3–2.4; removal §8.2; settings §5.3.

A's seq-20 object retires B and is learned by D. Later C's retirement of A
arrives with cutoff 10 and without A's object in `authority_known`.
R3 now invalidates A's retirement of B. With no remaining valid retirement,
B's stale states and authority are accepted again and it can resume
publishing without the explicit re-enrollment the GUI promised. The probe
demonstrates the validity flip. Grounded cycle labeling does not solve this
ordinary acyclic example.

Q-52 expressly permits invalidating a late *removal*; it does not explicitly
authorize silent reversal of an already-observed retirement.

**Suggested fix:** define a stable retirement/re-enrollment barrier,
including invalidated issuers, multiple cutoffs and transitive retirement.
If the intended product permits reinstatement without re-enrollment, that
requires an owner decision amending A-45/R-03, not a “fails safe” claim.
Test the three-issuer trace in both orders with master and stale decisions.
Links: R-03/R-07/R-12, A-45/A-20.

### R4D-12 — major — diagnostics' typed fields still carry arbitrary private source text

**Location:** ops §§5.3–5.4, T-33-1–T-33-5; Q-64.

The allow-list includes version labels and optional package names.
Version labels come from arbitrary `.toc` text; wrapping them in a typed
field does not sanitize them. A label containing a planted secret such as
`PLANTED_SECRET_DO_NOT_EXPORT` is neither a known account/path/name from
`RedactionContext` nor any listed pattern, so it leaves the process in an
export. This contradicts “never copies free text” and “credentials never
included”. Untrusted titles can do the same when names are on. Known-string
refusal is useful but cannot find secrets it has never been given.
Truncated package hashes also have no demonstrated “public” classification
for locally modified/private addon contents.

**Suggested fix:** export structural status and constrained normalized
version classes by default; omit arbitrary source strings/hashes unless
separately, specifically consented and accurately described. Make matching
case/normalization-aware, and test secrets in every permitted free-string
field, not only names already present in the redaction context. Keep exact
preview/no-upload/path refusal.
Links: R-15, A-23/A-33, E-06.

### R4D-13 — major — recommended defaults cross unapproved acceptance/limit barriers

**Location:** ops §§3.2–3.3, 4.1–4.2, 5.2, 9; questions Q-63/Q-77 and
“Implementers follow the recommended default”.

Q-63 treats in-app/tray-only macOS behavior as an acceptable fallback if
native notifications fail. A-27 requires OS notification behavior; the
owner approved a notification dependency and Mac testing, not removal of
that platform obligation. A visible limitation is not its approval.

Q-77 correctly says its 64 MiB limit needs owner approval, but the questions
instruction otherwise directs implementation of defaults. Ops introduces
additional numeric policies without a decision: six-hour recurrence
suppression, notice ledger/history bounds, request/response/time bounds,
ready delay/manual debounce and diagnostic size/row/line caps. The packet
requires numeric limits to be approved; these must not quietly become new
acceptance gates or indefinitely suppress important new recurrences.

**Suggested fix:** record the precise required owner answers before phase C
or leave the affected behavior blocked. Distinguish technical defensive
caps from owner-approved product/performance gates. A failed native backend
must remain a failed/blocked A-27 obligation unless an explicit scope
exception is approved.
Links: D-23/D-24, R-13/R-14, A-27/A-31/A-44, E-02/E-07.

### R4D-14 — minor — accepted unprefixed release tags link to a different tag

**Location:** ops §4.3 parsing and download URL.

The parser accepts both `0.2.0` and `v0.2.0`, but the download URL always
uses `v0.2.0`; a valid unprefixed release can therefore produce a broken
manual-update link.

**Suggested fix:** preserve the validated exact tag or only accept the
prefix policy actually published. Test both allowed forms. Link: A-28.

## Cross-packet contracts and verification additions

Phase C cannot safely treat the following as disjoint edits. Assign one
implementation owner for each and make dependents consume it, rather than
independently extending the same records/runtime methods:

| Shared surface | Required ownership/interface decision |
|---|---|
| `authority`, hash/ref types, issuer/validity, durable historical membership, mirroring/repair, capabilities | One common owner; removal/settings/rollback/retire/decide extend a single dispatched record family. Define safe repair of an existing hash-mismatched file |
| `ids`, stable addon/settings-unit identity, checkpoint ref, `SettingsRoots` | One owner; freeze grouping migration and identity normalization/collision diagnoses |
| `records::{ApplyJournal,RecoveryPoint}`, `plan`, `exec` and adapter journal I/O | One owner for schema and all-or-nothing code/settings recovery; distinguish settings-only, mixed update/remove and code-only fallback |
| `target`, code-choice/rejection/removal precedence | One owner for composition; settings master is not code authority |
| `admit`, retirement cutoffs and settings watermarks | Retire validates before any currentness/publication/target use; preserve historical enrollment evidence when `member.json` changes |
| `status`, `view`, `explain`, runtime `service`/`control`/`effects` | Assign integration edits explicitly; agree typed settings identity/hold scope and action freshness. `decide(item, option, shown)` must validate against fresh current evidence, not merely compare two stale UI values |

Required additions to the proposed tests, beyond the finding-specific
counterexamples:

- Independent full inventory/application outcomes for the removal
  permutation tests; equal M alone is insufficient.
- Exact schema bytes/ref-kind/major golden fixtures and settings-only
  crash/downgrade recovery; parent-before/after policy and state delivery.
- Completed/missed sessions immediately after reassignment: reconcile
  Q-72's startup capture publication with Q-75's “only sessions after the
  change” rule. Startup must not publish the prior non-master experiment.
- Retention tests including paired/unresolved provider roots, 16 MiB ledger
  overflow and capture/storage failures, not only three local files.
- Negative scope tests for NFC collisions and differing case on each native
  filesystem; a normalizing key must not hide two local identities.
- Update transport tests for invalid/wrong-host certificates, downgrade
  redirects, proxy environment isolation, concurrent/manual versus scheduled
  checks, silent errors and persisted attempt-before-request. OS TLS with
  SChannel/Security.framework is a reasonable four-target approach; the
  author's scratch-build/deny claims were **not independently rerun** here
  and do not establish native TLS or notification runtime delivery.
- All four supported targets retain honest runtime status; Windows ARM64
  and macOS Intel builds alone do not satisfy native notifications/TLS/GUI.
  macOS compilation/runtime and real-provider acceptance remain required.

## Owner questions before implementation

“Owner” in a questions-table column is not an approval. Ordinary technical
repairs above are the designers' job; do not ask the owner to invent a
hashing algorithm or deem an unsafe identity mapping safe.

| Questions | Review disposition |
|---|---|
| Q-54 | Proceed-with-dependency-warning follows D-21's sync-not-police intent. No new dependency-protection barrier required |
| Q-55 | **Owner answer needed:** does a stable reappearance after an ack authorize group-wide re-add, including updater reappearance not demonstrated to be a deliberate user action? Until then offer explicit GUI add-back; repair R4D-2/R4D-3 regardless |
| Q-56 | Additive code-state fields are a technical choice, not a product override. Freeze the actual shared/downgrade contract first (R4D-1); “old readers ignore fields” is not proof of safe active mixed-version operation |
| Q-57 | **Owner answer needed:** may the user dismiss unreadable known removal information and risk an install followed by a later removal? Explain recovery and local-only effect; never infer that missing authority is absent |
| Q-58 | **Owner confirmation needed:** union removes named folders while unnamed suite parts remain. This is more specific than “removed everywhere”; confirm the resulting collection behavior, after fixing stable settings scope |
| Q-59 | No extra headline promotion is a reasonable bounded interpretation of UQ-1; decisions must still be visibly actionable, not a false “fully synced” roll-up |
| Q-60 | **Owner confirmation needed:** retirement stops following as well as publishing; preserve local data and permit only explicit rejoin |
| Q-61 | Grounded mutual-retirement handling is technical, but R4D-11 must be resolved. **Ask owner only if designers retain silent reinstatement without re-enrollment**, because that changes A-45 |
| Q-62 | Per-user notification identity is an ordinary native implementation detail. Verify HKCU-only effects and uninstall guidance; do not call it tested here |
| Q-63 | **Owner answer required for any macOS notification exception**. Prefer a working native backend; fallback cannot self-waive A-27 |
| Q-64 | **Owner confirmation needed** for default inclusion of addon names. Arbitrary version labels/secrets are not implicitly approved by that checkbox (R4D-12) |
| Q-65 | Warning rather than blocking retirement follows the already explicit action. Q-27's 72-hour emphasis is approved; it is not proof that a computer is safe/unsafe to retire |
| Q-66 | OS TLS/ureq 2 is technically plausible with maintenance risk. No new licence approval is needed for the proposed allow-listed path; switching to a disallowed licence does require the stated owner decision. Target-specific build/runtime evidence is still outstanding |
| Q-67, Q-68 | Conservative ownership, excluded WoW state and synthetic-only scope follow existing rules. **Technical feasibility/evidence prerequisite:** establish correct saved-file ownership/layout; owner agreement is not substitute evidence |
| Q-69 | Conservative no-cross-identity matching follows R-02. Resolve NFC/case collision detection and prove native behavior; no arbitrary identity migration |
| Q-70 | Explicit sharing consent and recovery on adoption follow R-03/R-07. Make the master-for-another-computer consent truthful about which accounts that computer has included |
| Q-71 | Capture completeness is technical. Reused Q-41 timing is approved for removals, not automatically a new settings performance gate; propose any additional numerical policy explicitly |
| Q-72 | One missed-session checkpoint is reasonable; must not defeat reassignment/non-master experiment protection (Q-75) |
| Q-73, Q-74, Q-75 | Explicit conflict, offline push and no publishing pre-existing master differences follow R-08/R-09. Implement the causal fixes, not timestamp selection |
| Q-76 | Local recovery/provider sharing follows the existing recovery scope. Freeze bounded retention/GC roots before advertising provider recoverability; do not claim the future collector exists |
| Q-77 | **Owner approval required:** proposed 64 MiB saved-file cap and user outcome above it |
| Q-78, Q-79, Q-80 | Exact-content rejection, explicit lifts and paired/code-only recovery are consistent with R-10. Paired-checkpoint selection across several remote revisions needs a deterministic, truthful selection rule, not comparison of different computers' local sequence numbers |
| Q-81 | **Owner answer needed:** may an already-created but previously unseen offline release automatically end rollback's recovered-version constraint? “Newly seen” is not proof of a genuinely new post-rollback release |
| Q-82 | **Owner approval needed for the expanded feature:** persistent settings-checkpoint rejection, beyond code-release rejection. If retained, resolve ancestry/lift semantics and show its consequence distinctly from ordinary settings restore |
| Q-83 | Settings-only pause including future addons follows R-08; no permission to overwrite paused settings during removal or rollback |
| Q-84 | **Owner answer needed:** may unsupported-peer rollback be local-only with no group rejection? That cannot satisfy A-18 and may immediately be undone by ordinary sync. Safe default is block group rollback and explain the update prerequisite, unless a genuinely protected local-only mode is separately specified/approved |

Also approve or revise ops' new numeric policies listed in R4D-13, and
confirm whether mandatory `DataAtRisk` notifications may bypass both app
notification toggles. Existing Q-27/Q-41 approvals and R-14's memory/reaction
gates are not reopened.

## Actual commands, results and limits

Commands ran from the assigned worktree only. Every Cargo shell first used
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Command/check | Result |
|---|---|
| `git status --short`; `git branch --show-current`; `git rev-parse HEAD` | **PASS**: clean starting worktree; expected branch and reviewed SHA above |
| `rustc --version`; `cargo --version`; `[System.Environment]::OSVersion.VersionString` | **PASS**: versions/environment recorded above |
| Direct canonical document/core reads and scoped heading/symbol searches | **PASS** as inspection; findings are design findings, not executed product failures |
| `python docs\runs\run-004\evidence\R4-design-review\design_probes.py` | Final **PASS**, exit 0: six model tests; reproduced partial ack, stale-state eligibility, false settings conflict, retirement reversal and the 68,624-byte required supersession field; 24 merge permutations plus duplicate replays were invariant |
| `git diff --check`; `git diff --cached --check`; `git diff --cached --stat` | **PASS**: no whitespace errors; only this evidence and its 5,176-byte model staged |
| `cargo xtask verify --tier offline` | **PASS**, exit 0: offline/fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0`. Both new evidence files were staged before the run so the link check included them |

No product changes or newly required product behavior tests were authored,
so no production A-38 mutation was performed. All future removal/settings/
rollback/ops tests, GUI interaction, native tier, invalid-certificate
transport exercises, macOS/four-target runtime and real Dropbox transfers
are **NOT RUN** by this packet. Do not interpret a green current-slice
offline tier or the model probes as A-10/A-11–A-19/A-26/A-27/A-28/A-33/A-45
acceptance. This report and its small model are the only authored artifacts.
