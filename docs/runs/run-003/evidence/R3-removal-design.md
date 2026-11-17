# R3-removal-design evidence

- Packet: [R3-removal-design](../packets/R3-removal-design.md) with
  [COMMON](../packets/COMMON.md).
- Role and model: Design worker, Opus 5.5 (as assigned by the orchestrator;
  runtime selection not independently verifiable from inside the worker).
- Worktree: `D:\GitHub\wow-sync.worktrees\R3-removal-design`, branch
  `run003/R3-removal-design` from `main` (`b420717`).
- Date: 2026-11-16. Environment: Windows 11 x64 dev PC.

## What was built

Design only. No code, tests or required cases changed.

| File | Change |
|---|---|
| [docs/design/removal.md](../../../design/removal.md) | New. Removal and uninstall design: intent evidence, tombstone records, conflicts, apply/crash table, settings interface, three-state history, GUI needs, rollback/retire interaction, test obligations, and the changes other design files need when implemented |
| [docs/design/questions.md](../../../design/questions.md) | Appended a new section "Removal design questions" with Q-40 to Q-48 |

Not edited, as the packet requires: `docs/design/domain.md` and
`docs/design/README.md`. The needed `domain.md` changes are stated in
removal.md §10, including "supersedes domain.md §6 Q-2 when implemented"
(removal.md §1.5). The orchestrator adds the README row.

## Coverage of the packet's "Must cover" list

| # | Item | Where in removal.md | SPEC / ACCEPTANCE |
|---|---|---|---|
| 1 | Intent vs never-had, failed/partial scan, unreadable folder, non-ASCII name (Q-14), offline/stale provider; required evidence; never from absence alone | §1.1–§1.4 (evidence E1–E8, case table), §1.5 folder drop | R-03, R-04; A-04, A-05, A-20, A-23 |
| 2 | Tombstone record, which states carry it, lifetime | §2.1–§2.6 (`code-state/2`, copy-on-ack, grow-only `removals`/`superseded`, held/active/settled/superseded) | R-04, R-06, R-12; A-20 |
| 3 | Removal vs update conflicts: when a decision is needed, safe default | §3.1 (coverage-based table), §3.2 (frozen package), §3.3 (decision records, new information) | R-06; A-10, A-20 |
| 4 | Apply on receivers: WoW gate, staging/journal, crash idempotency extending apply.md §4.1, multi-folder unit, dependencies | §4.1–§4.5 (remove op, slot table, crash rows, mixed update op, dependency rules) | R-11; A-09, A-21, A-22 |
| 5 | Settings for removed addons, preserved and restorable with code; interface for the future settings design | §5.1 (operations), §5.2 (rules: owned only, paused, pairing, retention, no merge, identity, WoW-on-exit, master role) | R-07, R-08, R-09, R-10; A-09, A-15, A-19 |
| 6 | Three-state history and recovery; visible temporary exceptions | §6.1–§6.3 | R-10; A-16, A-17, A-19 |
| 7 | GUI needs and undo, screen design left to `ui/` | §7 | R-12, R-13; A-24, A-25, A-26, A-27 |
| 8 | Rollback/rejection and retired computers | §8.1, §8.2, plus joining §8.3 | R-03, R-10; A-18, A-45, A-04 |
| 9 | Test obligations mapped to acceptance IDs | §9 (T1–T33, A-38 control list) | A-04 to A-45 |

AGENTS.md product rules covered: "Additions/removals propagate" (§1–§4);
"Uninstall removes active code and settings; recovery must preserve and
restore both" (§4, §5, §6.3, with an explicit code-only state until the
settings design exists); "Three states" (§6); "Rollback rejects… offline
peers cannot resurrect" (§2.4, §3.1, §8.1); "Retired computers" (§8.2).

## Commands run

| Command | Environment | Result |
|---|---|---|
| `cargo xtask verify --tier offline` (PATH prefixed with `%USERPROFILE%\.cargo\bin`), from the worktree root, after writing removal.md and questions.md | Windows 11 x64 dev PC, Rust 1.100.0 | **PASS**: fmt, clippy, test, required-cases, deny, markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0` |
| same, re-run after adding this evidence file | same | **PASS**: all six checks PASS; `overall: PASS (allow-blocked: false) -> exit 0` |

Not run: no tests were added (design only), so no A-38 negative controls
apply to this packet. macOS CI: NOT RUN by this worker (no push).

## Limits

- Not reviewed yet. R3-design-review (phase B) is the independent check.
- The design relies on two later designs: settings (R-07 to R-09) and
  rejection/retirement. It states only the interfaces and interactions
  removal needs from them (§5, §8).
- The client behaviour for saved variables of uninstalled addons is unknown
  (Q-45). Until verified, removal is code-only and the GUI says so.
- Directory identity (E3) assumes the adapters can read a stable directory
  identity (Windows file ID, macOS inode). That has not been measured.
- Provider blob retention for tombstones is an input to the unresolved
  garbage-collection design (Q-7).

## Open questions raised

In [questions.md](../../../design/questions.md): Q-40 (state schema major),
Q-41 (removal hold), Q-42 (mass-removal confirmation), Q-43 (concurrent fresh
install of the removed release), Q-44 (settled tombstone lifetime), Q-45
(client behaviour for saved variables of uninstalled addons), Q-46 (unknown
or shared settings ownership on removal), Q-47 (paused settings on removal),
Q-48 (joining computer with a removed addon).

## Retry 1 (R3-removal-fix)

Worktree `D:\GitHub\wow-sync.worktrees\R3-removal-fix`, branch
`run003/R3-removal-fix`, based on main `917bc05`. The input was the FAIL
review [R3-design-review.md](R3-design-review.md) findings R3D-7 to R3D-10.
Files changed: `docs/design/removal.md`, the removal section of
`docs/design/questions.md` (Q-40, Q-43 and Q-44 revised; Q-49 to Q-52 added) and
this file. domain.md, apply.md, publication.md and the design README were
not edited; the changes they need are listed in removal.md §10.

| Finding | Disposition | Where |
|---|---|---|
| R3D-7 (prior presence lost by the normal publish loop and own-state pruning) | Fixed in the design. E1 now reads a local, durable `wowsync.presence-baseline/1`, written from stable complete scans and persisted before publication. It holds its own keys and trees, so the absent-state publication and pruning to three own states cannot erase it. E3 and E6 compare against the baseline. Cycle insertion: new step 3a after the scan and before publish (and step 2a for authority learning). While an observation is held or pending confirmation, P is `Held` and is not reinstalled from peers. T2 and T3 now run through the real cycle loop over several cycles, with pruning and a restart; T37 covers a long E7 confirmation | removal.md §1.2, §1.6, §2.8, §9 |
| R3D-8 (authority recomputed from latest admitted states) | Fixed in the design. Intents, decisions, re-add supersessions and confirming acks are immutable provider objects in `members\<cid>\authority\`, written before the state that references them. Learners mirror them on learning. Each computer keeps a durable, grow-only `wowsync.authority-ledger/1` with per-object validation V1–V7: membership, retirement cutoff or `authority_known`, hash, references, and a re-add requiring the issuer's own earlier ack. Active(P) is computed from the ledger only. Candidates still come only from admitted inventory (no stale-inventory fallback). A referenced but unreadable object gives `Unavailable(AuthorityPending)`. New T9b (learn, lose the carrier, restart, late reordered covered copy); T13 and T28 extended; T36 added | removal.md §1.3, §2.1–§2.6, §3.1, §8.2, §9 |
| R3D-9 (compact form hash mismatch; manifest dropped) | Fixed in the design. The intent object is immutable and its hash covers every field. The only compact form is a versioned ref `{ref, id, kind, package_key, hash}`, equivalent iff all of these match the object. Nothing is compacted on settling ("settled" is derived). The intent keeps the exact `removed` manifest and settings roots, which are retention roots. Restore on a computer that never had P uses the manifest, tree records and blobs, and checks `removed_hash`. Q-44 revised. New T34 (mixed ref/object/mirror arrivals) and T24b (never-present restore after all recovery points are pruned); T35 added | removal.md §2.2, §2.3, §2.7, §6.3, §9; questions.md Q-44 |
| R3D-10 (keep-version decision overridden by ordinary newest ordering) | Fixed in the design. A keep decision creates an enforced selection constraint: the target is exactly V (the chosen manifest), declined versions are never selected and are downgraded to V, and dominated versions are replaced. An unconsidered, non-dominated version is defined as later new information and gives `NeedsDecision(NewVersionAfterDecision)` with everything frozen and V kept. A fixed precedence (rejection, held/pending, ledger authority, constraint, ordinary ordering minus declined) prevents level 5 from overriding a decision. Interactions with rejection, supersession, competing decisions and resume are defined. New T11b (keep a lower V2, then a higher V4 arrives), and T13 extended; Q-49 and Q-50 added | removal.md §3.1, §3.3, §7, §8.1, §9, §10 |

Not changed: the removal findings outside R3D-7 to R3D-10, and every
non-removal finding (they belong to other packets).

### Commands and results (retry 1)

| Command (worktree root) | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, test, required-cases, deny, markdown-links; `overall: PASS (allow-blocked: false) -> exit 0` (Windows, this worktree, after all retry-1 edits) |

Design-only change: no code or tests were added, so there are no A-38
negative controls to run. The extended §9 obligations list the controls the
implementation must show. macOS CI: NOT RUN by this worker (no push).

### Limits (retry 1)

- Not yet re-reviewed. The fixes are design text; nothing is implemented.
- The retirement cutoff and `authority_known` fields are requirements on the
  later retirement design (§8.2, Q-52).
- Authority objects grow with the number of removals and decisions (about
  1 KiB each, per member that mirrors them). Their lifetime and the
  retention of their manifests are inputs to provider GC (Q-7, Q-44).
