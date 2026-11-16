# R2-design-recheck2: final bounded design review

## Verdict

**Ready with fixes — for implementation of the synthetic-only slice.**

The structural fixes resolve the original rename/journal crash prefixes and
cross-package cleanup hazard. Two mandatory implementation corrections remain
(F-01 and F-02 below). They use the existing observations, effect ports, hashes
and failure states; neither requires a new format, public function signature,
product decision or design document change. R2-core's packet permits the safer
reading of contradictory sketches, with the gap recorded in its evidence.
This verdict does not waive those corrections or their negative tests.

No new blocker. One new **major**, N-04, is the restart verification gap
described under F-02. Until implemented and independently verified, M-03,
M-07 and N-01 are only partially resolved.

This is not release readiness, real-data authorization, or a product
acceptance PASS.

## Scope and provenance

- Checkout: `D:\GitHub\wow-sync.worktrees\R2-design-recheck2`.
- Branch: `run002/R2-design-recheck2`.
- Reviewed source: `f1149123453c0610fb20bc156a94ff09f1e3d0d5`.
- Inputs: [previous re-check](R2-design-recheck.md), the "Retry 2" disposition
  in [designer evidence](R2-design-fix.md), and the actual architecture,
  domain, core/adapter APIs, publication, apply, questions and test contracts.
- Bounded to M-01, M-03, M-04, M-05, M-07, N-01, N-02 and N-03, plus
  newly introduced blocker/major consequences. No general minor/style review.
- SPEC, ACCEPTANCE, ENGINEERING, OWNER-DECISIONS, PLAN and COMMON are unchanged
  from the previous reviewed source. The orchestrator expressly permits
  explicit, safe synthetic-only scope narrowing for this re-check.
- Verification role assigned `gpt-6.1-sol` under D-04/D-17. Assignment is not
  independent telemetry proof of runtime model selection; the orchestrator
  retains that obligation.

## Disposition

| Finding | Result | Reason and checked contract |
|---|---|---|
| M-01 — hydrated provider reparse entries | **Resolved at design level** | [adapters-api.md](../../../design/adapters-api.md) §2/§5 separates provider `AllowCloudFiles` from install `RejectAll`, accepts the cloud-tag family only without unavailable attributes, and rejects links/unknown tags. N-02's corrections apply to every entry type. Real Dropbox recall behavior is still unverified under Q-23, not claimed to pass. |
| M-03 — unproved release/TOC choices | **Partially resolved; F-01 required** | [domain.md](../../../design/domain.md) §2.1/§3/§4 and Q-3/Q-20/Q-22/Q-24 explicitly restrict enrollment to synthetic installs, use plain-only selection for recognized and unfamiliar clients, and make the Forever suffix profile unreachable. Synthetic releases by construction legitimately defer real provenance/loading evidence; a bumped label is not newly declared real-release proof. The API sketch still needs the marker check wired into an existing core validation entry point, as detailed below. |
| M-04 — partial suite on an empty peer | **Resolved at design level** | Domain §6 now requires the candidate folder set to equal group-wide `Package.members`, not merely contain the destination's folders. Full-old A, partial-new B and empty C therefore yield `PartialCandidate` on C too; no older fallback. Q-2 explicitly defers legitimate folder retirement/removal. Core and three-install end-to-end cases cover this counterexample. |
| M-05 — same-size/mtime corrupt objects | **Resolved at design level** | [publication.md](../../../design/publication.md) §4 and Q-7 remove the verified metadata cache. Every referenced existing object is hashed before publication; corrupt bytes are replaced before the state record. Adapter `check_object` returns the observed hash. Required tests include same-size/mtime corruption and a negative control that trusts metadata. |
| M-07 — missing lifecycle/core admission wiring | **Partially resolved; F-02 required** | [core-api.md](../../../design/core-api.md) §5/§7 and adapter §9 supply admission, scoped staging/recovery/journal ports and per-package OpIds. [R2-slice](../packets/R2-slice.md) now owns runtime glue and real-adapter crash sweeps; core remains policy authority. The lifecycle is implementable with these ports, but the terminal rollback postcondition must survive a restart (N-04). |
| N-01 — rename/journal recovery not re-entrant | **Partially resolved; F-02 required** | [apply.md](../../../design/apply.md) §3/§4.1 replaces remembered per-folder progress with slot observations. Normal commit and rollback prefixes follow `Untouched ↔ Half ↔ Done`; a crash between rename and journal update no longer loses the inverse move. Journal-first staging and terminal, journal-last cleanup cover interrupted cleanup. However a crash after `R→L` can still skip the required restored-content verification, as N-04 demonstrates. |
| N-02 — recall guard permits hydration | **Resolved at design level** | Adapter §2 refuses either recall flag and OFFLINE for both files and directories, using parent-enumeration attributes before opening/enumerating an entry. It explicitly distinguishes enumeration RECALL_ON_OPEN from handle-info EA. Provider staging uses checked `open_read`, with OPEN_NO_RECALL defense in depth. Attribute-combination and fake-lister tests are required. Q-23 honestly leaves real dehydrate/open races and online-only validation unproved. |
| N-03 — operation-wide sibling cleanup | **Resolved at design level** | Apply §2.2/§2.5 assigns one package its own operation namespace and limits cleanup to that op. Committing rollback discards only absent or target-matching staging; unresolved ops retain all evidence. Q-25 blocks subsequent apply planning while journals remain. The three-op test specifically protects a NeedsHelp sibling while other ops settle. |

## Mandatory implementer fixes

### F-01 — consume marker evidence before issuing a usable verdict

**Residual M-03 (major).** Locations: domain §2 validation table and §2.1
marker check; core API §2 `VariantObservation`, `validate_variant`,
`VariantIdentity`, `assess_layout`; Q-24.

The marker is available only in `VariantObservation.synthetic_marker`.
Neither `VariantIdentity`, `OtherVariants` nor `assess_layout`'s arguments
carry it. The validation table currently reaches `Recognized`/`Unfamiliar`
without consuming it, so a literal implementation cannot perform the marker
check attributed to layout assessment. A GUI-only gate is insufficient:
`ConfirmedVariant` would no longer establish the slice's authorization
boundary for core planning.

**Implementation correction:** after metadata validation and before returning
either usable verdict, `validate_variant` must require the marker's first
line to equal `wowsync-synthetic/1`. Missing, unreadable or wrong-marker
observations must return a refusal carrying the not-synthetic reason. This
uses the existing input and `Refused(RefuseReason)` result; it does not add an
API parameter or authorize real data. Preserve the metadata error behavior
for invalid metadata. Revalidation each cycle must also refuse a removed
marker.

The existing required `install_without_synthetic_marker_refused` and
`install_without_marker_refused` tests must exercise core validation and
service enrollment, not just a GUI predicate. Include recognized and
unfamiliar clients, a wrong/unreadable marker, and marker removal after
enrollment. Record this safer placement as a design gap in R2-core evidence.
The Q-24 limitation remains authoritative; marker possession is not a claim
of real-release provenance.

### F-02 / N-04 — recheck rollback's restored-content postcondition

**New major.** Locations: apply §3.1 `Untouched`, §3.2 terminal transition,
and §4.1 case 3; tests §3 restoration mismatch and recovery crash sweeps.
Requirement: R-11; A-22 truthful restart reconciliation.

Counterexample for a replacement with expected old tree `O` and target `N`:

1. Journal is `committing`; slots are `L=N`, `S` absent, `R=O'`, where the
   recovery tree has become corrupt (`O' != O`).
2. Recovery recognizes `Done`, verifies `L=N`, gates and moves `L→S`,
   then gates and moves `R→L`.
3. Crash immediately after that second rename, before `content(L)==O`.
4. Restart sees `L=O'`, `S=N`, `R` absent: the table calls this `Untouched`.
   Its action is "none". Once all folders are Untouched, the algorithm
   writes `rolled_back`, discards staging and removes the journal.

Without the second crash, the required post-restore check reports
`RestoreMismatch`. With it, presence-only reconciliation skips that check
and clears the unresolved operation with corrupt restored data. Classifiable
rename prefixes are necessary but do not themselves establish verified
restoration.

**Implementation correction:** before writing the terminal
`rolled_back/from_phase: committing` journal, re-read every planned live
slot and require its content to equal the journal's expected old tree, or
absence for an originally absent folder. On mismatch/unreadable content,
persist NeedsHelp and keep journal/staging/recovery; do not perform terminal
cleanup. This is the conservative interpretation of the already required
restoration postcondition, using existing `tree_of`, `expected_live` and
NeedsHelp ports/states. It needs no progress field, new format, public API,
product decision or extra live rename. An externally changed Untouched
folder may consequently remain NeedsHelp instead of being declared restored:
preserve it rather than overwrite it, consistent with Q-25.

Extend the existing restoration-mismatch/re-entrant recovery tests to combine
corrupt recovery content with a crash immediately after `R→L`, including a
real synthetic filesystem case. Assert NeedsHelp, retained journal and
available alternatives, no terminal cleanup and no false restored/success
claim. Crash sweeps using only unmodified old/new trees cannot expose this
counterexample.

## Scope-narrowing assessment

Q-24 is an explicit slice limitation, not an approval to fabricate Forever
loading or real release-pair evidence. With F-01, real installs remain
unenrollable and unknown suffix mapping remains blocked.

Q-25's retained, visible NeedsHelp and apply-wide waiting are safe for this
bounded prototype. They defer the recovery-resolution GUI and unrelated-work
liveness, not the obligation to preserve unresolved data. They do not satisfy
full R-06/R-10/R-13 acceptance or permit terminal-only recovery as a released
workflow. Q-2 similarly defers suite retirement rather than guessing deletion.
None of these deferrals is substituted for an acceptance PASS.

## Verification

Environment: Windows 11 Pro `10.0.26300`, x64; Rust `1.100.0`
(`0ef1bc386059596dada8fb65b50425aab2ea637a`),
host `x86_64-pc-windows-msvc`; cargo-deny `0.20.2`.

Command, from the assigned checkout:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo xtask verify --tier offline
```

**PASS**: fmt, clippy, workspace test, required-cases, deny and Markdown links;
overall PASS, `allow-blocked: false`, exit 0. Repeated after staging this
evidence so the tracked-file Markdown check covers it.

The current runner checks **one** listed offline case against **24** libtest
results. These are existing scaffold/verification-tool tests, not execution
of the proposed product contracts or the two corrections above.

**NOT RUN**: product crash/restore/marker cases, native/provider tiers, real
Dropbox validation, GUI interaction, memory and cross-platform runtime.
No addon Lua/CMD execution, real WoW access, private provider/settings data
access, implementation edits, nested agents, push or merge occurred.
