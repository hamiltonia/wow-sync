# R3-design-recheck — independent recheck 1

## Verdicts and scope

| Area | Verdict | Reason |
|---|---|---|
| UI | **PASS** | R3D-4, R3D-5, R3D-6, R3D-11 and R3D-12 closed for their bounded design findings |
| Q-21 | **FAIL** | R3D-1 and R3D-2 closed; R3D-3 remains major and not closed |
| Removal | **FAIL** | The specific mechanisms behind R3D-7 through R3D-10 are replaced, but the replacement design introduces four major findings, R3D-13 through R3D-16 |

This is a recheck of the [original findings](R3-design-review.md), not owner
approval of a prototype, an implementation of removal, or full GUI acceptance.
The author's retry evidence was read as a claim to check, not as independent
test execution. No design or production change is delivered here.

- Assigned checkout: `D:\GitHub\wow-sync.worktrees\R3-design-recheck`.
- Branch: `run003/R3-design-recheck`.
- Part 1 reviewed UI/compatibility at
  `3cf482c4d0aff93a2c8a50a2c4444423e7a645f9`, including compatibility merge
  `76c491b6830644711f926764ea64d60b905c5f07`.
- Before part 2, `git merge --no-edit main` fast-forwarded this branch to
  `c046f252848b153848f8d47440b7dafd57a55c42`, including removal merge
  `fc37d9f7241b13b2222f938e6de5eab02cbc0412`. It changed removal/questions,
  removal evidence and handoff, not the reviewed UI or compatibility code.
- Windows x64, Windows NT `10.0.26300.0`; Rust
  `1.100.0 (0ef1bc386 2026-11-09)`; Cargo
  `1.100.0 (3acc79c0c 2026-11-05)`.
- Assigned verification role: R3-design-review, `gpt-6.1-sol` packet.
  Actual runtime selection is launcher-controlled, not independently
  introspected by a repository command. No subagents launched.

Authority remains SPEC, ACCEPTANCE, D-18 through D-20 and the approved run
plan. In particular, the handoff explicitly calls rule A **interim, pending
owner confirmation**. Q-41 through Q-50/Q-52 owner proposals are not treated
as approvals or as changes to SPEC.

## Original findings: dispositions

| ID | Disposition | Independently checked evidence |
|---|---|---|
| R3D-1 | **CLOSED** | `toc.rs` retains Interface validity, including malformed/overflow/empty fragments and disagreeing repeated headers. `inventory.rs` carries it through `TocSummary`; absent old-wire validity defaults false. `target.rs::interface_set` requires valid, selected, nonempty evidence. Fixed core and runtime suites pass |
| R3D-2 | **CLOSED** | `same_known_build` requires both build strings present and equal. Otherwise `eligible_holders` requires Interface evidence. Two `None` builds no longer bypass that gate; the core regressions cover missing builds |
| R3D-3 | **NOT CLOSED — major** | Rule A compares the candidate with the same installed package/folder's declarations, not validated receiving-client support. The executed counterexamples below still return `Chosen`. The owner has not approved the proxy |
| R3D-4 | **CLOSED** | Both dashboard SVGs and the scenario identity rules use `World of Warcraft: Forever (wowf)` / `_forever_`; synthetic builds and the unfamiliar synthetic client are explicitly separate |
| R3D-5 | **CLOSED** | S12 now changes a consent-relevant mapping (`validated-plain/1` to fake `profile:wowf/1`); S15 separately changes only the build and does not reconfirm. This matches the predicates in `variant.rs::confirm_enrolled` and D-18 O-N2 |
| R3D-6 | **CLOSED** | `screens.md` continuation scope and UG-25 distinguish per-addon decisions from an unresolved journal that blocks all local applies. S7 has decisions without a journal and unrelated Details! progress; S16 separately holds updates behind a needs-help journal. The journal sentence takes precedence. Build-out shows neither continuation promise until typed evidence exists |
| R3D-7 | **CLOSED for the reported publication/pruning defect** | Removal §1.6 persists a separate presence baseline before publication; E1 no longer reads the newest own state. The insertion at step 3a, local `Held`, restart behavior and T2/T3 address the original lost-baseline loop. A distinct snapshot-association problem is R3D-14 below |
| R3D-8 | **CLOSED for the reported lost-carrier defect** | Removal §2.4 persists learned immutable objects before computing a target, mirrors on learning rather than ack, reloads across restart, and retains them despite issuer inventory becoming unusable. §2.5 stops on missing referenced authority; no stale inventory fallback was added. T9b/T13/T28 name the original delivery/restart case. New validation deadlock: R3D-13 |
| R3D-9 | **CLOSED for the reported hash/manifest defect** | §2.2 hashes every immutable field; §2.3 refs have explicit equivalence checks and never replace the object. Settling changes no bytes. The exact `removed` manifest, names and `removed_hash` remain available to §6.3, unlike the former compact tombstone. Retention is now overbroad: R3D-15 |
| R3D-10 | **CLOSED for the reported keep-version override** | §3.3 constrains the target to the chosen manifest, downgrades declined higher alternatives, freezes on unconsidered non-dominated information, and forbids ordinary ordering from overriding the choice. Rejection, competing choices and supersession are addressed. T11b explicitly chooses lower V2 over V3, then introduces V4. The new resume formula has a separate defect: R3D-16 |
| R3D-11 | **CLOSED** | Prototype coverage and deferred obligations are explicitly distinguished; retirement, settings, actual removal/rollback/recovery and diagnostics are not claimed satisfied by placeholders. This closes the coverage-claim defect, not their acceptance rows |
| R3D-12 | **CLOSED** | README/screens are declared authoritative; header/context, full-width Computers list/replacing detail and fixed Add-ons columns/order agree with the SVGs. `style.md` now accurately describes pinned GPUI `focus_visible` as focus plus `last_input_was_keyboard()`, with native behavior left as a prototype check |

## Q-21: rule A counterexamples and conservatism

Read the actual `toc.rs`, `inventory.rs`, `target.rs` and runtime e2e changes,
not only Q-21/domain prose. Local evidence remains scoped to the selected
variant and requires `Ok`/selected/valid folder evidence. Rule A fixes the
unrelated-addon union and subset loopholes, and is narrower than its
predecessor. **Narrower than the old implementation does not prove at least
as conservative as D-18.** Equality with another declaration does not imply
that a receiving client supports that declaration.

Two temporary target tests used the existing `Scenario`, `addon_interface`,
`suite_iface`, `client` and `pkg` helpers:

1. `r3_recheck_stale_same_addon_is_not_client_support`:
   local Suite v1.0, Interface `99999`, tree `a`; peer Suite v2.0, Interface
   `99999`, tree `b`, different build `2.0.0.1`.
2. `r3_recheck_equal_multi_client_sets_are_not_client_support`:
   the same scenario, but **both** declarations are `11508,99999`.

Each asserted `PackageTarget::NeedsDecision` with
`DecisionReason::DifferentClientBuild`: no independent receiver-support
evidence had been established. `cargo test -q -p wow-sync-core r3_recheck_`
returned **exit 101: 2 failed, 185 filtered out**. Both assertion messages
showed actual `PackageTarget::Chosen`, with the peer as source.

These are executable counterexamples to the claim that unsupported/unverified
declarations cannot authorize the target. They are **not** tests against a
real WoW client, and do not establish which actual client accepts or rejects
the synthetic number `99999`. No imported Lua or game data was executed.
Both temporary tests were removed; `git diff --exit-code --
crates\wow-sync-core\src\target.rs` then returned zero. They are not committed.

Required closure remains the original R3D-3 fix: validated client-support
evidence, or `NeedsDecision` when that support is unverifiable; alternatively,
an explicit owner decision accepting this weaker installed-copy proxy.
An orchestrator's pending choice cannot replace that approval.

## New removal findings

The following are source-level counterexamples to a **future design**, not
claims that unimplemented removal tests or real three-replica experiments ran.
Sources are [removal.md](../../../design/removal.md) and
[questions.md](../../../design/questions.md).

### R3D-13 — major — inconsistent authority cannot be resolved by its decision

**Locations:** removal §2.5 V1/V4 and the paragraph after V7
(lines 412–428); §3.1 precedence; §3.3 decision well-formedness; §7.

V1 makes a mismatched object id permanently `invalid(inconsistent)`.
The designed GUI offers a decision to resolve that record. But V4 requires
**every** authority id in `knows` and `considered` to be `valid`. A decision
that names the inconsistent record it resolves is therefore
`pending(missing_reference)`, not valid; an intent produced by a
"keep removed" decision has the same dependency problem.

For example, learn two different hashes for intent T; choose a valid
restorable version V in the advertised GUI; record D considering/knowing T.
T cannot become valid while both immutable copies remain in the grow-only
ledger, so D never becomes valid either. Omitting T instead fails the
decision's required provenance/considered rules. Additionally, an admitted
ref to invalid T satisfies the `AuthorityPending` condition ahead of the
actionable inconsistency result; these two status rules conflict.

**Required fix:** distinguish missing authority from known-inconsistent
evidence and define how a resolving decision safely names and supersedes
the exact conflicting variants without requiring those variants to become
valid authority. Resolve status precedence. Require a test from conflicting
copies through a GUI decision, restart and late replay to convergence,
not merely T31/T34's initial freeze. Links: R-06/R-12/R-13,
A-20/A-23/A-25/A-26.

### R3D-14 — major — one global top-level snapshot cannot preserve each baseline

**Locations:** removal §1.2 E6; §1.6 baseline refresh and step 3a;
§2.8 `wowsync.presence-baseline/1` (line 506).

Baseline package entries retain different historical scan counters, but
there is only **one record-level `top_level` snapshot**. §1.6 refreshes a
baseline when that package is updated or reappears, and says that the
record-level snapshot is from that scan.

Start with stable ASCII P and at least four other eligible packages, avoiding
E7's mass threshold. Rename P to an unrelated unsupported non-ASCII name.
P's old snapshot correctly identifies a new unsupported entry. Now an
independent update to still-present Q refreshes Q's baseline, replacing the
shared `top_level` with a snapshot that **already contains that entry**.
P's old folder manifest/scan remains, but its corresponding top-level
snapshot is no longer represented. E6 can now treat the entry as no longer
"new", losing its required rename barrier. A restart cannot reconstruct the
missing association. This is a loss of required safety evidence, not an
executed claim that all other removal gates passed.

Freezing the one global snapshot would conversely contradict refresh for
other package entries; the record needs to represent both scan histories.
The unchanged E3 wording also needs reconciliation with domain §2.1's
inventory-sensitive assessment digest before implementation: literal
equality with the original enrollment digest can block ordinary removals.
That pre-existing issue is not counted as a new finding or silently waived
to claim an end-to-end removal counterexample.

**Required fix:** retain the top-level evidence associated with each
package's baseline (or durable scan snapshots referenced by those entries)
until that baseline is discarded. E6 must never compare against another
package's refreshed scan. Extend T6 to multiple ordinary cycles with another
package updating its baseline and a restart; the unexplained newcomer must continue
to block intent formation. Links: R-04/R-11, A-05/A-10/A-20/A-23.

### R3D-15 — major — superseded history bytes become permanent retention roots

**Locations:** removal §2.7 retention roots (lines 495–499), §6.1–§6.2;
questions Q-44.

The ledger retains valid historical objects forever, including superseded
intents and keep decisions. The new retention rule roots the trees/blobs
and settings for **every valid intent and every valid keep decision**,
not only active, unresolved or still-promised history. Q-44 keeps those
manifests for the life of the group.

Repeatedly remove/re-add/update one addon at distinct versions, settling
and resolving every operation. Supersession changes Active(P), not the
historical objects' validity, so every removed/chosen payload remains
rooted indefinitely. Local pruning to three points cannot prune those
provider history states. They also have no temporary reason that can
clear under §6.2. This directly contradicts R-10/A-16/A-17, rather than
merely costing a few KiB of authority metadata.

**Required fix:** separate permanent anti-resurrection metadata from
retention of historical payloads. Define payload roots, advertised restores
and eventual release after supersession/resolution within current plus two
previous, with necessary visible temporary exceptions. A pending owner
question cannot authorize indefinite full-payload retention. Extend T23 to
inspect provider blobs/settings after several settled remove/re-add/keep
cycles, not just local recovery-point counts.

### R3D-16 — major — resume-automatic formula newly declines the kept version

**Locations:** removal §3.3 well-formedness (lines 605–608), automatic
choice and selection-constraint interactions; questions Q-50.

The one V6 formula defines `declined` as all `considered.versions` minus
the **chosen hash**, plus inherited declined versions, minus `allow_again`.
A `chose.automatic` choice has **no chosen hash**. Applying the formula with
no chosen hash excludes nothing from the considered set; treating it as
undefined instead leaves the automatic record unvalidatable.

For example, D1 deliberately keeps V2 rather than V3, so only V3 is
declined. The user then chooses "Resume automatic updates", with D1 and
current V2 considered, and does not allow V3 again. The formula now makes
V2 declined as well. If a new V4 is also considered it is newly declined,
too. This is not the promised return to ordinary ordering while
**previously declined** releases remain excluded; it can leave no eligible
version after a routine resume. The GUI promises no such rejection of V2.

**Required fix:** give each choice an explicit, deterministic formula.
Resuming should preserve only the exclusions its stated consequence
authorizes, minus explicit `allow_again`; it must not silently decline the
currently kept release merely because automatic has no chosen hash.
Require keep-V2/decline-V3 → resume with no allow-again, with and without a
new V4, plus a replay of V3. Validate resulting target and record validity.
Links: R-06/R-12, A-08/A-20/A-25/A-26.

## Executed verification and limits

Commands ran in the assigned worktree. Cargo commands used
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` first.

| Command/check | Result |
|---|---|
| `cargo test -q -p wow-sync-core` | **PASS**: 185 passed; none failed or ignored; doc-tests zero |
| `cargo test -q -p wow-sync-runtime e2e` | **PASS**: 30 passed; none failed or ignored |
| `cargo test -q -p wow-sync-core r3_recheck_` | **FAIL**, exit 101: the two temporary conservative-oracle assertions above both returned `Chosen`; probes removed afterward |
| Parse/read all 12 updated SVGs | **PASS**: XML parsed; displayed identity, context, controls and status text checked against the revised prose |
| Recompute theme text contrasts in Python | **PASS**: 10 text tokens × 2 backgrounds per theme; all 40 ratios ≥ 4.5:1 and within 0.011 of published values |
| Compare reconfirm and continuation/headline promises with `variant.rs`, `plan.rs`, runtime and `status.rs` | **PASS** for the corrected design claims; typed gaps are explicit, not treated as implemented |
| `git diff --exit-code -- crates\wow-sync-core\src\target.rs` after probes | **PASS**: temporary tests fully restored |
| `cargo xtask verify --tier offline` after combined evidence | **PASS**, exit 0: fmt, clippy, test, required-cases, deny and markdown-links; `overall: PASS (allow-blocked: false)` |

Removal §9 T2/T3/T9b/T11b/T13/T24b/T34 and the new counterexamples above
are **NOT RUN as removal behavior**: no removal implementation exists.
Passing offline checks validate the existing slice and repository hygiene,
not that future ledger/baseline/decision algorithms work.

Actual GUI rendering/keyboard interaction, owner prototype approval,
Windows native tier, real WoW-client support, real Dropbox/provider tier,
macOS and screen-reader behavior: **NOT RUN** in this design recheck.
No processes were stopped, no user game files accessed, no push performed.
Only this evidence is a persistent authored change.
