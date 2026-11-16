# R2-design-review: independent design review

## Verdict: not ready

Three blockers and eight major findings require a design retry before
R2-core/R2-adapters/R2-slice implementation. These are design counterexamples,
not claims that unimplemented product tests failed.

Reviewer: R2-design-review, independent verification role. Assigned model:
`gpt-6.1-sol`; actual runtime selection/usage evidence must be retained by the
orchestrator under D-04/E-01, not inferred from this assignment.

Checkout: `D:\GitHub\wow-sync.worktrees\R2-design-review`,
branch `run002/R2-design-review`. Reviewed source:
`1741cc07db42b428c2ce60299f3ef0839a060985`.
Authority: SPEC/ACCEPTANCE/ENGINEERING, D-17, run-002 PLAN and this worker's
packet. Only this evidence file is changed. No sub-agents, game/provider data
access, push or merge.

## Findings

### B-01 — blocker: detected game activity still authorizes rollback writes

**Location:** `docs/design/apply.md` §2.3a–c and §3; corresponding
`docs/design/questions.md` Q-4; `docs/design/tests.md` §3 A-21:core.

After detecting WoW running between folders, the algorithm deliberately
renames live/recovery folders immediately. This is a known-running write,
not merely the unavoidable check-to-rename residual window allowed
provisionally by PLAN §6.7. Also, one activity check precedes *two* commit
renames (live-to-recovery and staging-to-live), despite the overview promising
a check before every rename. R-11/A-21 prohibit game-file changes while
running, including restoration.

**Fix:** gate every live mutation, including rollback renames. If activity
becomes Running/Unknown after any mutation, retain the journal and both
alternatives, report waiting/needs-help, and resume recovery only after a
fresh NotRunning observation. Replace the mandatory
`launch_mid_package_rolls_back_package_and_stops` oracle with one asserting
no further live mutations while running and eventual recovery after exit.
Any stronger exception needs a specific owner decision; listing Q-4 does
not authorize it.

### B-02 — blocker: stale scans permit overwriting a concurrent updater

**Location:** `docs/design/adapters-api.md` §3 scan rules;
`docs/design/apply.md` §1.3–7 and §2.2–3;
`docs/design/core-api.md` §6 `ApplyPlan`/`FolderApply` and §7.

Per-file before/after `(len, mtime)` checks do not establish a stable
*collection*: a writer can change an already-hashed file, or add/remove a
file/folder after enumeration. The inventory is reused across a potentially
long publish/stage operation. Before replacement, only process activity is
rechecked; the live tree is not compared with the planned prior tree and
quiescence is not revalidated. A cooperating app-instance lock does not
exclude CurseForge/another writer. A newer local update made during staging
can therefore be replaced using an obsolete target. S-proc §2c explicitly
demonstrates why metadata-only stability is insufficient; omitting a
disturbing exclusive probe does not solve this.

**Fix:** define complete-tree stability observations (including membership),
source revalidation before publishing, and expected live-tree/absence
preconditions carried in the plan/journal. Revalidate destination content
and writer/quiescence observations immediately before consequential changes;
changed or uncertain observations return to core for replanning, not
replacement. Preserve the residual external-writer race honestly. Add
real-filesystem tests for an updater changing an already-scanned file,
adding/removing files, and updating the destination during staging
(R-05/R-11; A-08/A-21/A-22).

### B-03 — blocker: unresolved or unverified recovery can be finalized/pruned

**Location:** `docs/design/apply.md` §2.4–6 and §4 `committed`;
`docs/design/publication.md` §2 apply-journal/recovery-point schemas;
`docs/design/tests.md` §3 A-22:core.

The post-check can yield NeedsHelp for launch/hash mismatch, but step 6
still instructs finalization, removal of the journal and pruning to two
recovery points without a success-only condition. On restart, phase
`committed` goes directly to finalization without the live-tree/activity
post-check. A crash just before that check can convert an unverified or
externally changed apply into a finalized state and prune recovery choices.
There is no durable needs-help/post-check-pending phase protecting this
state. This contradicts R-10/R-11 and A-17/A-22.

**Fix:** make successful validation a prerequisite for finalization,
confirmation and normal pruning. Persist unresolved state and explicit
retention pins; committed recovery must revalidate actual content/activity
before deciding its outcome. Add crash cases immediately before/after
post-check, with game activity, mismatching live bytes, restoration failure
and more than two necessary recovery copies.

### M-01 — major: blanket reparse rejection prevents using hydrated Dropbox

**Location:** `docs/design/adapters-api.md` §2 `Root`/`is_reparse`, §5
`ProviderRoot`; `docs/ARCHITECTURE.md` §2.3 path-safety boundary.

Every operation rejects every Windows `FILE_ATTRIBUTE_REPARSE_POINT`,
including components traversed to open a root. Run-001
`evidence/S-dropbox.md` items 1/2a records that the authorized test folder
and subfolders themselves carry cloud reparse tags, and locally readable
pinned files become reparse placeholders. They are not unsafe junctions.
The proposed provider cannot open the tested root or use ordinary hydrated
files after placeholder conversion. Plain temporary-directory tests miss
this functional failure (R-03; A-05).

**Fix:** distinguish approved cloud-file reparse tags from links/junctions
and unknown tags, without weakening containment or accidentally hydrating
online-only data. Define provider-specific observations and safe access
rules. Require native coverage of a hydrated tagged directory/file,
online-only refusal, and unsafe junction rejection; retain real-provider
limitations where owner-created cloud states are still unavailable.

### M-02 — major: unfamiliar-client confirmation is naming consent, not validation

**Location:** `docs/design/domain.md` §1.3 and §2 validation rows 9–11;
`docs/design/core-api.md` §2 `confirm_variant`;
`docs/design/tests.md` §3 variant tests and §5 A-46:slice.

Any syntactically valid unfamiliar product with agreeing BPSV rows and a
plain AddOns directory becomes ConfirmedVariant after a Boolean checkbox.
The GUI only shows product/folder/build. No guided safe-scope/layout
validation evidence is required or recorded, and an unfamiliar client is
assumed to use plain TOCs. A directory's existence and agreeing labels do
not establish that this client loads/manageably isolates that code layout.
R-02/A-46 explicitly distinguish consent from validated identity and safe
scope, even when settings are excluded.

**Fix:** model the guided validation result separately from display-name/
consent input and require it before constructing ConfirmedVariant.
Show/validate the actual authorized addon layout and applicable TOC
selection/identity evidence; unsupported layouts stay blocked. The slice
need not implement settings, but must disclose their exclusion rather than
claiming their layout validated. Test an unknown product with agreeing
metadata but an unvalidated layout remaining blocked after consent.

### M-03 — major: recognizable labels are treated as release/compatibility proof

**Location:** `docs/design/domain.md` §3–§6;
`docs/design/core-api.md` §3–§6;
`docs/design/publication.md` §2 group/code-state TOC fields.

Target selection has no compatibility predicate. Group equality is product
code only; build versions are display data, Interface evidence is ignored
by selection and absent from the transmitted TOC summary. Changed trees
with recognizable larger labels can win, including manual edits or
unverified prerelease/git-describe histories. In particular, commits-ahead
counts on different branches do not prove chronological release ordering.
Even a sole candidate with only a foreign-suffix TOC gets
`TocSelection::None` but can be Chosen and installed: the selection rule
records “no toc for this variant” without making compatibility a plan gate.
S-addon/S-check explicitly establish *recognition*, not correctness of real
release pairs or client compatibility, and Forever's loaded TOC remains
unverified. Thus newer *compatible* installed releases (R-05/A-08) are not
actually distinguished from merely larger strings.

**Fix:** carry release-identity, comparability/channel and target-client
compatibility evidence into core and require demonstrable predicates before
automatic choice. Unverified pairs, manual modifications and unvalidated
client TOC mappings need addon-scoped decisions. Synthetic slice pairs can
be explicitly validated test evidence; do not generalize their ordering to
arbitrary installed releases. Add incompatible-build, modified-content and
divergent-describe-branch counterexamples.

### M-04 — major: partial package candidates produce mixed effective packages

**Location:** `docs/design/domain.md` §5 comparison steps 3–6 and §6;
`docs/design/core-api.md` §6 paragraph after `plan_apply`;
`docs/design/questions.md` Q-1/Q-2.

Example: A has `Suite-Core` v1 plus `Suite_Data` v1; B has only
`Suite-Core` v2. Grouping joins the folders, but comparison orders only the
common Core folder and chooses B. Apply replaces Core while deliberately
keeping A's old Data. The effective package is neither candidate and cannot
match the selected package hash for confirmation. Similarly, known
name-related but ungrouped families such as Questie/QuestieDB are silently
treated as independent transferable units rather than diagnosed. This
violates R-05/A-09's no-mixed-package obligation. No-removal authority is
correct, but does not authorize mixing a partial winner with old parts.

**Fix:** establish package completeness/identity evidence and block only
the inseparable package when it is insufficient. Keeping extra folders must
not turn a partial candidate into a proven complete release; require a
decision where applying the winner would mix parts. Surface known unresolved
family relationships rather than quietly splitting them. Add partial-suite
and changing-group-membership tests verifying exact effective hashes.

### M-05 — major: incomplete/corrupt latest input can fall back to stale state

**Location:** `docs/design/publication.md` §2 sequence rule, §4.2–4 and §6;
`docs/design/domain.md` §6; `docs/design/core-api.md` §6 `plan_publish`
and §7 `PublishEffects::blob_exists`.

Selecting the highest *valid* sequence allows a malformed or incompatible
newest state to be discarded and an older valid state used automatically.
Cloud-only/oversized files explicitly block, but malformed JSON/unknown
schema/body mismatch have no equivalent no-fallback rule. A partial newest
delivery can therefore authorize an obsolete candidate. No persisted
highest-observed/accepted watermark prevents regression when state files
disappear. Additionally, existing blobs/trees are skipped based on
existence: `put_blob` checks an existing blob only if called, while the plan
and executor can avoid calling it. A corrupt preexisting object can be
referenced by a freshly published state despite the complete/verified
publication claim (R-06/R-11/R-16; A-20/A-23/A-36).

**Fix:** define core-owned admission/latest-state rules, distinguish
observed newer-but-invalid/unavailable data from absence, and prohibit
automatic stale fallback. Persist monotonic observation/acceptance state
with explicit handling of missing records. Validate existing referenced
objects or use trustworthy verified-content observations before publishing.
Test seq N+1 malformed/unsupported/partial while N is readable, deletion
after observing N+1, and corrupt blobs/trees already at expected paths.

### M-06 — major: inaccessible possible clients become a safe activity result

**Location:** `docs/design/adapters-api.md` §6 `observe_variant`;
`docs/design/apply.md` §5; `docs/design/questions.md` Q-16.

After failed process-image queries, the adapter can return NotRunning with
only a diagnostic count. S-proc §2a explicitly notes that an elevated or
other-session game client may be invisible. This is not evidence that the
game is stopped. Core then authorizes writes, despite the stated invariant
that Unknown blocks. D-17's residual launch-window default does not approve
ignoring an already-running uninspectable client (R-11/A-21).

**Fix:** separate known unrelated protected processes from unresolved
possible clients and return Unknown where safe exclusion cannot be
established. Define a supported detection boundary and targeted candidate/
additional observation strategy rather than globally blocking on every
system PID or silently allowing every failed lookup. Add denied-query
possible-client tests. Any unavoidable detection limitation that relaxes
R-11 needs an explicit owner decision before applicable real use.

### M-07 — major: the fixed core/effect interface cannot execute its lifecycle

**Location:** `docs/design/core-api.md` §7 `ApplyEffects`;
`docs/design/adapters-api.md` §4 and §9;
`docs/design/apply.md` §2.6 and §4; `docs/design/crates.md` §1/§4/§5.

The executor must remove journals, but ApplyEffects has no remove-journal
method (the adapter does). Its `exists(Area::Recovery, op, folder)` omits the
package needed by `Slot::Recovery { pkg, op, folder }`. The journal schema
stores phase per package while recovery instructions describe a journal
phase, without defining multi-package recovery traversal. Finally,
runtime must choose member latest states, but the fixed core API provides
no reducer for those raw observations: implementing §6 of publication in
runtime would place admission/freshness policy outside core. These gaps
break the promised mechanical mapping and one-pass implementation boundary.

Source-crate ownership is separate, but Cargo.toml/Cargo.lock/inventory
ownership is explicitly shared, not disjoint; §5's controlled merge plan
must remain part of the implementation packets. Those three implementation
packets do not exist in this reviewed checkout yet.

**Fix:** complete the public lifecycle ports, include package identity in
recovery-slot lookup, specify per-package journal recovery, and expose
core-owned member/admission reduction. Provide the actual bounded packets
with shared-file integration ownership before launching workers. Require a
two-package crash/recovery test exercising the complete adapter mapping,
not only fakes that can access private storage directly (E-05/E-08/A-43).

### M-08 — major: GUI verification is assigned to a nonexistent worker

**Location:** `docs/design/tests.md` §5 final sentence;
`docs/runs/run-002/PLAN.md` §3 phases D/E.

The design says GUI checks belong to “R2-gui's (PLAN)”. PLAN contains no
R2-gui worker. Its slice tests are headless service calls; no actual
enrollment/status/apply interaction checklist, oracle or evidence owner is
given here. Consequently a green required-case inventory could accompany a
GPUI shell whose buttons do not complete the promised no-terminal slice.
Neither prior GPUI spike screenshots nor service tests establish this
behavior (R-12/R-13; E-03/E-04; A-25/A-26).

**Fix:** assign real slice GUI task verification to the existing R2-check
packet, with R2-slice developer checks: consent/enrollment, safe apply,
running-game wait/resume, addon ambiguity, provider/error state and truthful
confirmation, including keyboard/focus and retained interaction evidence.
Keep native/provider/manual outcomes separately reported; do not invent a
new worker or treat headless tests as GUI evidence.

## Checks and evidence boundaries

- **PASS, document review:** versioned/content-addressed records, immutable
  member states, state-last publication, no Lua execution, no WTF access,
  absence-is-not-deletion and publication-not-confirmation are explicitly
  designed. The defects above prevent treating these intentions as proved.
- **PASS, existing tier/source audit:** only the empty core skeleton and
  xtask exist in `crates/`/`xtask`; none of the proposed product modules or
  tests exist yet. `xtask/src/main.rs` actually runs workspace tests in
  offline (therefore Windows-native nonignored tests also run on Windows).
  Required-case enforcement applies only to offline;
  native/provider remain unconditionally BLOCKED. Unique exact libtest names
  are necessary because `xtask/src/inventory.rs` matches without crate names.
  The inventory currently contains only A-39. CI executes offline on
  Windows/macOS and four build-only targets, not four native runtime tiers.
  These design claims are accurate within those limitations.
- **PASS, evidence review only:** run-001 REPORT and all top-level evidence
  records were read, including independent P1-check/S-check/P3-review
  findings; relevant S-addon coverage and S-check identity observations were
  inspected. Prior reports are scoped evidence, not new executions. The
  uncommitted S-forever worktree/report was neither read nor modified.
- **NOT RUN:** proposed product behavior, seeded product negative controls,
  GUI, performance, native cloud/process/filesystem cases and real Dropbox.
  macOS/ARM64 product runtime proof remains unavailable here. This review
  neither waives those gates nor approves the owner questions in Q-1–Q-19.
  D-17 applies PLAN §6's recorded provisional defaults, not blanket approval
  for additional product/safety exceptions.

## Executed offline verification

Environment independently checked: Windows 11 Pro `10.0.26300`, 64-bit
(Insider development environment); rustc `1.100.0`
`0ef1bc386059596dada8fb65b50425aab2ea637a`, host
`x86_64-pc-windows-msvc`; cargo-deny `0.20.2`. Development-profile workspace
verification, no installed product artifact.

Exact command from the assigned worktree:

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\R2-design-review'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo xtask verify --tier offline
```

**PASS**, exit 0, without `--allow-blocked`: fmt, clippy, test,
required-cases, deny and markdown-links all PASS. The clean-source report at
`target\verify\report.json` recorded the source SHA above,
`git_dirty: false`, generated `2026-11-16T03:35:45Z`, one listed offline
case verified against 24 test results and 62 tracked Markdown files.
That is proof of the current scaffold's verification, not acceptance of
this design's unimplemented safety or product behaviors.

Repeated with this evidence staged: **PASS**, all six checks, exit 0;
report generated `2026-11-16T03:40:18Z`, same source SHA,
`git_dirty: true`, 24 test results, one required case and 63 tracked
Markdown files. `git diff --cached --check` also passed. The dirty flag
correctly identifies the precommit documentation change.
