# R2-design-recheck: bounded independent re-verification

## Verdict: not ready

The retry fixes substantial parts of the review, but three original major
counterexamples remain and the revised recovery/cloud-access contracts
introduce one blocker and two majors. Safe defaults are acceptable when
they block uncertain transfers; labeling an unsafe automatic choice an
owner question is not approval.

Reviewer: R2-design-review, verification role; assigned `gpt-6.1-sol`
(orchestrator retains actual runtime selection/usage evidence under D-04).
Checkout: `D:\GitHub\wow-sync.worktrees\R2-design-recheck`,
branch `run002/R2-design-recheck`.
Reviewed source: `b80c6853a03cc8e9867b48ab749cf0eb49d0b9da`,
including design fix `6d3f881` and the draft core/adapters packets.

Scope: independently compare B-01..B-03/M-01..M-08 with the actual amended
documents, not just the designer's disposition table. Only new
blocker/major consequences of these fixes were considered; no new minor
audit. Authority documents retained from the first review were confirmed
unchanged by `git diff 22ac1eb HEAD` for README, SPEC, ACCEPTANCE,
ENGINEERING, OWNER-DECISIONS, run-002 PLAN and COMMON.

## Original finding disposition

“Resolved” below means resolved in the design, not executed product proof.

| Finding | Result | Independent reason / location |
|---|---|---|
| B-01: running-game rollback writes | **resolved** | `apply.md` §2.2.3b/c, §2.3, §3 gate each live rename and stop rather than roll back while Running/Unknown. Revised Q-4 now respects R-11, including restoration. The residual check-to-next-rename window is honestly bounded by PLAN §6.7. `tests.md` §3 replaces the unsafe mid-launch oracle and covers rollback gates. |
| B-02: stale scan overwrites updater | **resolved** | `adapters-api.md` §3 now snapshots complete membership/metadata around hashing and the top-level listing; `apply.md` §2.1–2.2 carries expected tree/fingerprint, revalidates destination content after staging, checks fingerprint before replacement and verifies old recovery content. `publication.md` §4.5 revalidates source fingerprints before state publication. The remaining identical-metadata/external-writer race is explicitly disclosed, not a guarantee of writer exclusion. This addresses the original normal updater counterexamples within the provisional slice. |
| B-03: unverified finalization/pruning | **resolved** | `apply.md` §2.2.5–6/§2.4/§4 now separates committed, verified and needs_help, re-verifies committed recovery, finalizes only verified packages and retains pinned/journal-referenced points. Those exact unsafe-finalization/pruning rules are corrected. The newly specified recovery/cleanup lifecycle still has N-01/N-03 below; this is not an overall crash-recovery pass. |
| M-01: hydrated Dropbox reparse rejection | **partially** | `adapters-api.md` §2/§5 correctly allows the cloud tag family for provider roots/ancestors, while rejecting links/unknown tags. Q-23's refusal of unavailable data and honest owner-prepared provider-test blocker are legitimate defaults. However, the newly defined type-dependent recall guard can permit remote hydration (N-02), so the full safe provider-access claim is not resolved. |
| M-02: unfamiliar confirmation without validation | **resolved** | `domain.md` §2.1, `core-api.md` §2 and `tests.md` §3 add layout assessment, duplicate/enrollment checks, evidence-bound identity/scope confirmations and rejection of stale consent. Q-20 blocks unfamiliar suffixed-TOC folders and excludes unvalidated settings explicitly. This is a conservative code-only guided scope, not a claim that arbitrary settings/client conventions were proved. |
| M-03: labels treated as release/compatibility proof | **partially** | `domain.md` §3/§4/§6 now gates Selected TOCs, matching present builds/mappings, separates channels and refuses divergent describe branches. Q-21 blocks uncertain cross-build transfers legitimately. Q-22 still explicitly makes a manually modified, label-bumped tree win automatically, and Q-3's guessed Forever mapping is treated as known loading evidence. Neither deferral has a safe blocking default; see residual R-M03. |
| M-04: partial package mixing | **partially** | `domain.md` §5–§6 correctly groups known name families and blocks the original A-full/B-new-Core-only replacement on A. But completeness is checked against only the destination's local set, not the known inseparable package membership. The same partial winner can be installed on an empty third peer (R-M04). Q-2's safe stated default is not fully implemented by the predicate. |
| M-05: stale fallback/unverified existing objects | **partially** | `publication.md` §6 and core `admit` now select newest-or-unusable, specify watermarks and refuse regression rather than falling back. Tree decoding/hash validation and object repair are added. `check_object` nevertheless still trusts cached size/mtime as proof of bytes, leaving the corrupt-existing-object counterexample open (R-M05). |
| M-06: inaccessible possible client fails open | **resolved** | `adapters-api.md` §6 now returns Unknown for uninspectable `wow*.exe` or a variant executable name; synthetic denied-query tests are assigned. Revised Q-16 blocks the ordinary elevated/other-session client case while excluding unrelated system PIDs. Its copied/renamed client outside the enrolled variant is an explicitly unclaimed detection boundary, not authority to ignore a normal possible client. Real-client evidence remains required before support claims. |
| M-07: incomplete lifecycle ports/core admission | **partially** | `core-api.md` §5/§7 now exposes admission, journal removal and package-addressed recovery slots, with mappings in `adapters-api.md` §9. Core/adapters packets exist and reference the amended design/shared-file procedure. A future R2-slice packet is an explicit orchestrator prerequisite, legitimately outside design-owned writes. However, per-package recovery/cleanup cannot safely use the still op-wide cleanup ports/rules (N-01/N-03); lifecycle implementability is not complete. |
| M-08: GUI verifier nonexistent | **resolved** | `tests.md` §5b and `crates.md` §7 assign G1–G9 to R2-slice developer execution, R2-check independent repetition and R2-review evidence review. Headless results cannot substitute. The orchestrator must carry these into the future slice/check packets; no nonexistent GUI worker remains. |

## Remaining original majors

### R-M03 — major: Q-22/Q-3 do not safely defer unproved release choices

**Locations:** `domain.md` §3 compatibility gate, §4 last rules, §6
eligibility; `questions.md` Q-3/Q-22.

Q-22 explicitly says a manual content edit with a larger version label wins
as newer. Same build, selected TOC and complete folders do not establish
that this is an unmodified installed release; they do not close the original
R-05/A-08 counterexample. Also, the Forever suffix order is still marked
unverified, yet a Selected result under `profile:wowf/1` is described as
evidence that the client loads that folder. Equal versions/dependencies/IDs
between TOCs do not establish equal loaded file lists or Interface
compatibility. Giving the guessed mapping a versioned name is not
validation.

**Fix:** block automatic use where the release/TOC evidence remains
unvalidated, or restrict this slice explicitly to positively validated
synthetic release/mapping fixtures. Preserve the real-client/real-release
validation gate rather than claiming general compatibility. An owner can
decide a proposed limitation separately, but D-17's PLAN defaults do not
approve Q-22's automatic preference. Add the bumped-label modification
counterexample and TOCs with equal summary fields but different loaded
content to the required decision/blocking cases.

### R-M04 — major: an empty peer can receive an incomplete suite

**Location:** `domain.md` §6 eligibility row `local set ⊆ c's set`,
choice rules and Suite example; `tests.md` §3/§5 partial-suite cases.

Extend the designer's example with C having no suite. A reports Core v1
and Data v1; B reports Core v2 only. Grouping knows the inseparable package
is `{Core, Data}`, but B dominates on the common Core comparison. On C,
the local set is empty, so the subset predicate passes and B is installed
without Data. This is not a proven complete package (R-05/A-09), even though
it avoids leaving old local Data beside new Core.

**Fix:** validate against established inseparable package membership,
not only locally present folders. A partial candidate may stay untouched
on its source but cannot automatically seed an incomplete installation.
If membership cannot be established, block that package with a decision.
Test the three-peer/full-old/partial-new/empty-destination case.

### R-M05 — major: metadata cache still accepts corrupt existing bytes

**Locations:** `publication.md` §4.2, verified-objects schema;
`core-api.md` §7 `check_object`;
`tests.md` §3 `cached_verified_object_not_rehashed`.

After verification, an object can be replaced/corrupted with bytes of the
same size and preserved mtime. The cache then returns VerifiedOk without
reading those bytes, and a new state can reference the corrupt object.
Provider replication is another writer; content addressing in the name is
not enforcement of immutability. This is the original “existence is not
complete verified content” defect with a stronger-looking metadata proxy
(R-11/A-20/A-23), not an integrity proof.

**Fix:** hash referenced existing objects for this bounded slice, or define
and validate a genuine content-generation/immutability guarantee including
provider replacements. Add a corrupt-byte test preserving cached size and
mtime. Do not retain an oracle requiring that such an object bypass hashing.

## New findings introduced by the revised contracts

### N-01 — blocker: new per-folder journal states are not crash-idempotent

**Locations:** `apply.md` §2.2.3, §3 recovery table, §4 final paragraph;
`publication.md` §2 journal `state` enumeration; `tests.md` §3/§5
crash-at-every-effect cases.

The recovery table accepts only pending/live-old, moved_old/live-absent/
recovery-old, and installed_new/live-new. There is no interpretation for
a crash between a rename and its journal update. If moved_old is written
after the rename (as §2.2.3b suggests), a crash leaves pending plus live
absent/recovery old, which the table rejects. If it is written before
the rename (as §4 requires), a crash leaves moved_old plus live old/
recovery absent, which is also rejected.

Rollback has an additional concrete gap: after moving new live content to
staging, but before restoring old recovery content, another crash leaves
installed_new with live absent, staging new and recovery old. No rollback
progress state or matching table row exists. Restart yields needs_help
instead of restoring a complete old/new live package. The retained bytes
are useful, but do not satisfy the mandatory crash oracle, and GUI resolution
is outside this slice. A safe default cannot manufacture the missing
restart transition.

**Fix:** specify durable rename intent plus observed-slot/hash reconciliation
for both sides of every commit/rollback effect, including interrupted
rollback. Make recovery itself idempotent under repeated crashes and game
activity pauses. Retain journals/copies on truly unknown content, but do
not classify these expected crash prefixes as unknown. Extend the
real-adapter crash sweep to crash during recovery as well as initial apply.

### N-02 — major: the new recall predicate can hydrate virtual entries

**Locations:** `adapters-api.md` §2 `entry_meta`/`is_cloud_only` and §5
provider reads/listings; `questions.md` Q-23.

`is_cloud_only` checks RECALL_ON_DATA_ACCESS/OFFLINE for files, but
RECALL_ON_OPEN only for directories. Windows documents both recall flags
as relevant to files **and directories**: a RECALL_ON_OPEN file may fetch
remote data when opened, and a RECALL_ON_DATA_ACCESS directory may fetch
remote contents when enumerated. The proposed per-type omission can
therefore permit the very hydration Q-6/Q-23 says must be refused.
Furthermore RECALL_ON_OPEN is documented as a directory-enumeration
observation, whereas `entry_meta` specifies only handle-based basic/tag/
standard information; the unavailable-state observation source needs
clarification rather than treating that bit as universally supplied.

**Fix:** conservatively reject either recall indication before data reads
or enumeration, for either entry type, and specify how enumeration flags
are obtained without a recall. Preserve the separate EA interpretation of
the overlapping attribute value for handle information. Add pure
attribute-combination tests and retain the owner-prepared native/provider
verification blocker; do not infer online-only coverage from a tagged,
pinned local file.

Reference inspected read-only:
[Microsoft file attribute constants](https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants),
RECALL_ON_OPEN/RECALL_ON_DATA_ACCESS descriptions (2026-11-16 UTC).
No real Dropbox folder or unavailable file was opened by this reviewer.

### N-03 — major: per-package cleanup deletes whole-operation evidence

**Locations:** `apply.md` §2.2.1–2/§2.2.6, §3 final paragraph, §4;
`core-api.md` §7 `discard_staging(op)`/`remove_journal(op)`;
`adapters-api.md` §4 `remove_staging(op)`.

The revised journal has independently recoverable packages, but staging
cleanup and journal deletion are still operation-wide. §4 says to discard
staging for an op when just one package is staging. That deletes staged
alternatives of another committing/waiting package. §3 and finalization
also remove the journal after one package, contradicting §4's rule that
it stays until *every* package is terminal. For example, recovery of B
can finish and remove the journal while A's needs_help must remain durable.
Pinning an individual recovery record does not preserve A's per-folder
state or prevent new apply once the journal disappears.

**Fix:** make cleanup package/folder-scoped, or retain op-wide staging until
all references are terminal. Remove the journal only once every package is
safely terminal and all unresolved state is preserved. Add mixed
needs_help/committing/staging package tests verifying that settling one
does not delete another's journal or alternatives. Carry the granular
cleanup contract into both effect and adapter interfaces.

## Executed verification and limits

**PASS:** independent baseline `cargo xtask verify --tier offline`, exit 0,
without `--allow-blocked`. Exact command:

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\R2-design-recheck'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo xtask verify --tier offline
```

All six checks passed: fmt, clippy, workspace tests, required cases, deny,
Markdown links. Baseline report `target\verify\report.json`:
source SHA above, `git_dirty: false`, generated
`2026-11-16T03:56:55Z`, 24 test results, one listed offline case,
66 tracked Markdown files.

Repeated after staging this record: **PASS**, all six checks, exit 0.
Report generated `2026-11-16T04:01:03Z`, same source SHA,
`git_dirty: true`, 24 test results, one required case and 67 tracked
Markdown files. `git diff --cached --check` passed. The dirty flag
correctly identifies the documentation-only precommit state.

Actual environment checked: Windows 11 Pro `10.0.26300`, 64-bit (Insider
development machine), rustc `1.100.0`
`0ef1bc386059596dada8fb65b50425aab2ea637a`, host
`x86_64-pc-windows-msvc`, cargo-deny `0.20.2`.

**NOT RUN:** proposed product tests, crash injections, GUI, performance,
real-client detection and real Dropbox behavior. Product implementation
still does not exist in this input. The passing offline scaffold is not
execution evidence for any proposed fix. Native/provider product gates and
other architectures' runtime proof are not waived. No sub-agents, private
game/provider data access, other-worktree reads, push or merge.
