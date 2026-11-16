# R2-design-fix: design retry 1 evidence

Worker: R2-design (role design, claude-opus-5.5, D-04/D-17). Checkout
`D:\GitHub\wow-sync.worktrees\R2-design-fix`, branch `run002/R2-design-fix`
(from `main` at `c032d85`). Input: [R2-design-review](R2-design-review.md)
(verdict not ready). Changed only `docs/ARCHITECTURE.md`, `docs/design/**`
and this file. No code, no real WoW/Dropbox data, no sub-agents; the
S-forever worktree was not read.

## Disposition

| Finding | Disposition | Where |
|---|---|---|
| B-01 rollback writes while game active | **Fixed.** Activity gate `G` before every live rename in apply, rollback and recovery; launch mid-commit → no further live mutation (no rollback), journal and all slots kept, `Waiting`; recovery only after a fresh `NotRunning`. Residual window stated (PLAN §6.7). Q-4 revised to this safe default | [apply.md](../../../design/apply.md) §2.2–§2.3, §3, §4; [questions.md](../../../design/questions.md) Q-4; tests `activity_checked_before_every_live_rename`, `no_live_mutation_after_launch_mid_commit`, `interrupted_package_recovers_after_game_exit`, `rollback_gates_every_rename` |
| B-02 stale scans overwrite concurrent updater | **Fixed.** Complete-tree metadata fingerprints (files + dirs), listing and per-folder before/after snapshots; plan carries `expected_live`; destination revalidation (fingerprint + content + quiescence) after staging → `Replanned`; fingerprint re-check before each rename; recovery copy re-hashed at verify; publish source revalidation before the state record | [apply.md](../../../design/apply.md) §2.1–§2.2; [adapters-api.md](../../../design/adapters-api.md) §3; [publication.md](../../../design/publication.md) §4; [core-api.md](../../../design/core-api.md) §6–§7; tests in [tests.md](../../../design/tests.md) §3–§5 |
| B-03 unverified finalize / pruning | **Fixed.** Per-package phases `staging → committing → committed → verified → finalized` plus durable `needs_help`; finalize only from `verified`; `committed` recovery re-verifies; recovery points pinned until verified; pinned/journal-referenced points never pruned and shown as extra copies (A-17) | [apply.md](../../../design/apply.md) §2.2, §2.4, §4; [publication.md](../../../design/publication.md) §2; tests `crash_before_verify_reverifies_on_restart`, `pinned_recovery_points_exceed_two_visibly`, … |
| M-01 reparse rejection vs hydrated Dropbox | **Fixed** with safe default **Q-23**: `ReparsePolicy::AllowCloudFiles` only for the provider root; cloud tag family accepted when hydrated, online-only refused, links/junctions/unknown refused everywhere; provider-tier tests report BLOCKED without an owner-prepared online-only file | [adapters-api.md](../../../design/adapters-api.md) §2, §5; [tests.md](../../../design/tests.md) §4 |
| M-02 unfamiliar confirmation = naming consent | **Fixed** with safe default **Q-20**: `assess_layout` (AddOns layout, duplicate product folder, already enrolled, toc mapping), digest-bound identity + scope confirmation; folders with suffixed tocs blocked for unfamiliar clients | [domain.md](../../../design/domain.md) §2.1, §3; [core-api.md](../../../design/core-api.md) §2 |
| M-03 labels as release/compatibility proof | **Fixed** with safe defaults **Q-21**, **Q-22**: ordering only within scheme and channel; prerelease vs release ambiguous; describe only vs its exact tag; eligibility predicates (selected toc, same mapping, same client build); limitation of manual label-bump edits stated | [domain.md](../../../design/domain.md) §4, §6; [questions.md](../../../design/questions.md) |
| M-04 partial candidates mix packages | **Fixed.** `PartialCandidate` → `NeedsDecision` (no install, no fallback); `Family` edge (Questie/QuestieDB); set relation recorded; grouping change changes hashes; Q-1/Q-2 revised | [domain.md](../../../design/domain.md) §5–§6; e2e `partial_suite_on_target_is_not_mixed` |
| M-05 corrupt latest falls back to stale | **Fixed.** Core `admit` module: newest state or unusable, no fallback; persisted watermarks detect regression; publish verifies existing objects by hash (`check_object`, verified-objects cache) and replaces corrupt ones; readers recompute tree hashes | [publication.md](../../../design/publication.md) §2, §4, §6; [core-api.md](../../../design/core-api.md) §5, §7 |
| M-06 inaccessible possible clients safe | **Fixed** with revised **Q-16** (blocking default): uninspectable process named `wow*.exe` or matching an `.exe` in the variant folder → `Unknown`; pure `classify` tested offline | [adapters-api.md](../../../design/adapters-api.md) §6 |
| M-07 core/effect interface incomplete | **Fixed (design part).** Complete `ApplyEffects`/`PublishEffects` ports (`remove_journal`, slot-based `tree_of`/`observe` with package identity, `remove_recovery`, `stat/hash/put_object`, `observe_source`); per-package journal recovery traversal; core-owned admission; full adapter mapping; e2e two-package crash test over real adapters. **Deferred to orchestrator:** writing the R2-slice packet and keeping the shared-file merge procedure in all packets (not a design-owned path) | [core-api.md](../../../design/core-api.md) §5, §7; [adapters-api.md](../../../design/adapters-api.md) §9; [apply.md](../../../design/apply.md) §4; [crates.md](../../../design/crates.md) §5, §7 |
| M-08 GUI verification owner | **Fixed.** GUI checklist G1–G9 (consent/enrollment, unfamiliar client, safe apply, running-game wait/resume, ambiguity, provider/error, truthful confirmation, interrupted/needs-help, keyboard/focus); R2-slice developer checks, R2-check independent repetition with retained evidence, R2-review checks evidence; headless tests are not GUI evidence; no "R2-gui" | [tests.md](../../../design/tests.md) §5b; [crates.md](../../../design/crates.md) §7 |
| Minor: arch test excluded `testkit.rs` but fixtures live in `testkit/` | Fixed | [crates.md](../../../design/crates.md) §6 |
| Minor: mapping section reference §8 → §9 | Fixed | [crates.md](../../../design/crates.md) §1 |
| Minor: negative controls did not cover new safety rules | Fixed (12 new mutations) | [tests.md](../../../design/tests.md) §6 |

## New or changed owner questions

- Q-4 (changed): no rollback while WoW runs; restore/verify after exit.
- Q-16 (changed): uninspectable possible client blocks.
- Q-1, Q-2 (changed): Family edge; partial candidates need a decision.
- Q-17 (clarified): BLOCKED provider tests are never recorded as passed.
- Q-20 (new): unfamiliar clients — suffixed-toc folders blocked.
- Q-21 (new): different client build/toc mapping → decision, not install.
- Q-22 (new): manual edits with same/bumped label indistinguishable.
- Q-23 (new): cloud-file reparse tags accepted only under provider root.

## Verification

`cargo xtask verify --tier offline` from the worktree root on the dev PC
(Windows), with the changes staged: fmt, clippy, test, required-cases, deny,
markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0`.
This checks documents only (links); no product behavior exists or was run.
Native/provider tiers, GUI and product tests: NOT RUN (no code in this packet).

## Retry 2

Worktree `D:\GitHub\wow-sync.worktrees\R2-design-fix2`, branch
`run002/R2-design-fix2`, input
[R2-design-recheck](R2-design-recheck.md). Following the orchestrator's
instruction, hazards are removed by narrowing slice scope where possible; each
narrowed behaviour is a slice limitation with a Q-n
([questions.md](../../../design/questions.md), "Slice limitations").

| Finding | Disposition | Where |
|---|---|---|
| N-01 (blocker) journal recovery not crash-idempotent | **Fixed by simplification.** One package per operation (own OpId, journal, staging, recovery slots); no per-folder progress is stored — recovery **classifies observed slots** (`Untouched`/`Half`/`Done`/`Unknown`) from live/staging/recovery presence plus content checks; rollback runs the commit chain backwards so every crash or game-launch stop leaves a classifiable configuration; journal/record writes are temp + flush + rename. Explicit idempotency argument: every journal phase × crash point → recovery action, recovery re-entrant (8 cases + write-order diagram). Crash after last commit rename but before `committed` → rollback (accepted cost) | [apply.md](../../../design/apply.md) §2.2, §3.1–§3.3, §4, §4.1; [publication.md](../../../design/publication.md) journal schema; [core-api.md](../../../design/core-api.md) `classify_slots`, `RecoveryOutcome`; tests `classify_slots_table`, `crash_during_recovery_at_every_effect_is_reentrant`, e2e `crash_during_recovery_with_real_adapters_is_reentrant` |
| N-02 (major) recall guard can permit hydration | **Fixed.** Pure `is_unavailable(enum_attrs)` refuses `OFFLINE`, `RECALL_ON_OPEN` or `RECALL_ON_DATA_ACCESS` for files **and** directories; attributes come only from parent-directory enumeration (`FindFirstFileExW`), checked before any open or enumeration; handle-info EA bit (0x40000) never used for availability; opens add `FILE_FLAG_OPEN_NO_RECALL` as defense in depth; staging reads through `ProviderRoot::open_read` (`path_of` removed). Native/provider behaviour remains BLOCKED pending an owner-prepared online-only file (**Q-23** revised) | [adapters-api.md](../../../design/adapters-api.md) §2, §3, §5, §9; tests `unavailable_attribute_combinations`, `unavailable_directory_not_enumerated` |
| N-03 (major) cleanup deletes sibling packages' evidence | **Fixed by simplification.** Cleanup is per op only (one package), idempotent, journal removed last; `rolled_back` from `committing` discards staging only when each slot is absent or equals the target, else `needs_help(UnexpectedStaging)`; `needs_help` ops keep everything; while any journal exists no new apply is planned (**Q-25**, new) | [apply.md](../../../design/apply.md) §1, §2.5; tests `settling_one_op_keeps_other_ops_evidence`, `needs_help_op_blocks_new_apply_and_is_kept`, `any_journal_blocks_apply_plan` |
| R-M03 (M-03 partial) unproved release choices | **Fixed by narrowing.** Slice enrollment requires the synthetic marker `.wowsync-synthetic` (`Unsupported(NotSyntheticInstall)`, **Q-24** new); toc mapping is `validated-plain/1` for **every** client including `wowf` — any suffixed toc → blocked; the Forever profile rule is specified but unreachable (**Q-3**, **Q-20** revised); no provenance claim for same-label edits (**Q-22** revised) | [domain.md](../../../design/domain.md) §2.1, §3, §4; [core-api.md](../../../design/core-api.md) `synthetic_marker`; [adapters-api.md](../../../design/adapters-api.md) §3, §8; tests `install_without_synthetic_marker_refused`, `suffixed_tocs_unvalidated_for_recognized_client`, e2e `install_without_marker_refused` |
| R-M04 (M-04 partial) empty peer receives incomplete suite | **Fixed.** Completeness is against group-wide `Package.members`; an incomplete candidate is `PartialCandidate` on every computer, including empty peers (**Q-2** revised; dropped-folder releases stay a decision) | [domain.md](../../../design/domain.md) §6; tests `partial_candidate_not_installed_on_empty_peer`, `complete_candidate_installed_on_empty_peer`, e2e `empty_peer_does_not_receive_partial_suite` |
| R-M05 (M-05 partial) metadata cache accepts corrupt bytes | **Fixed by simplification.** Verified-objects cache removed; every referenced existing object is re-hashed on each publish and replaced when corrupt (**Q-7** revised: cost) | [publication.md](../../../design/publication.md) §4; [core-api.md](../../../design/core-api.md) `check_object`; test `same_size_mtime_corrupt_object_rehashed_and_replaced` |
| M-01 partial | Resolved via N-02 above | as N-02 |
| M-07 partial | Design part resolved via N-01/N-03 (per-op ports: `discard_staging`/`remove_recovery` scoped to one op, `classify_slots`, `plan_apply` allocates one `OpId` per package). The R2-slice packet already exists on main | [core-api.md](../../../design/core-api.md) §5, §7; [adapters-api.md](../../../design/adapters-api.md) §9 |
| Negative controls | 7 new mutations (suffixed-toc selection, local-only completeness, remembered per-folder state, cross-op cleanup, size/mtime trust, recall bit ignored, marker gate removed) | [tests.md](../../../design/tests.md) §6 |

### New or changed owner questions (retry 2)

- Q-24 (new): slice only on synthetic installs (marker gate).
- Q-25 (new): any journal blocks new applies; `needs_help` resolved later via GUI.
- Q-2 (changed): completeness against group-wide membership.
- Q-3 (changed): Forever profile rule disabled; plain-only in the slice.
- Q-7 (changed): full re-hash of referenced objects on every publish.
- Q-20 (changed): plain-only rule applies to all clients.
- Q-22 (changed): no provenance claim; scoped by Q-24.
- Q-23 (changed): recall/offline refused for files and directories, from enumeration attributes.

### Verification (retry 2)

`cargo xtask verify --tier offline` from the worktree root on the dev PC
(Windows), changes staged: fmt, clippy, test, required-cases, deny,
markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0`.
Documents only; no product behaviour exists or was run. Native/provider
tiers, GUI and product tests: NOT RUN (no code in this packet).
