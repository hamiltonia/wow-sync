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
