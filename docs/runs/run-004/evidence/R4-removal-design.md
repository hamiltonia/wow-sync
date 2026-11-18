# Evidence: R4-removal-design

Packet: [R4-removal-design](../packets/R4-removal-design.md). Branch
`run004/R4-removal-design`, worktree
`D:\GitHub\wow-sync.worktrees\R4-removal-design`, based on `main` at
`573615b`. Design only; no code changed.

## What was produced

- [docs/design/removal.md](../../../../docs/design/removal.md): full
  rewrite for owner decisions D-21 and D-23. Any valid removal wins. Removal
  records are content-addressed (identity = SHA-256 of the exact bytes = file
  name), so they cannot have id variants. Records name folder keys, and
  removals merge by union per folder key, bounded only by explicit re-add
  records (`supersedes`). The result is deterministic and independent of
  order. Stale copies are handled with an audience/ack "owing" rule.
  Code and settings removal (settings contract with an integration fallback),
  the three-state history, GUI restore and add-back, the presence baseline,
  the settle wait, the WoW gate and the crash table are kept. The document
  also gives an implementation map onto the existing core, runtime and app
  modules, the on-disk compatibility rules (additive `code-state/1` fields),
  a test list T1–T33 mapped to A-IDs with negative cases and A-38 controls,
  and the GUI touchpoints per ui/screens.md.
- [docs/design/questions.md](../../../../docs/design/questions.md): Q-54 to
  Q-58 appended, each with a safe default.
- This evidence file.

## Commands and results

| Command | Environment | Result |
|---|---|---|
| `cargo xtask verify --tier offline` (in the worktree, after `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`) | Windows NT 10.0.26300, rustc 1.100.0, PowerShell 7 | **PASS**: fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0` |

No tests were added: this packet is a design document only. The design's
tests (removal.md §12) are obligations for the implementing packet.

## R3D-13 closure

**Closed by D-23** (removal.md §13.1):

1. Record identity is the content hash and the file name, so "one id, two
   variants" cannot be expressed. The run-003 counterexample becomes two
   valid removals.
2. Any removal wins, and removals merge by union of folder keys. There is
   no conflicted status, no `InconsistentRemovalRecord`, and no removal
   decision.
3. The new V7 checks only a record's own listing, plus overlap between a
   re-add and what it supersedes. Writers derive `supersedes` from exactly
   the overlapping live removals, so V7 is always satisfiable.
4. Every waiting state either clears by itself or has a GUI action.

## R3D-1 to R3D-16 status

| Finding | Status |
|---|---|
| R3D-1, R3D-2 | Closed in run-003; unaffected (removal reads no Interface data) |
| R3D-3 | Superseded by D-21 (no Interface gating); not reintroduced |
| R3D-4, R3D-5, R3D-6, R3D-11, R3D-12 | Closed in run-003; kept (removal follows ui/screens.md; journals block removals with no continuation promise) |
| R3D-7 | Kept (durable presence baseline, step 3a, `Held`) |
| R3D-8 | Kept and strengthened (ledger, mirroring on learning, own-copy repair) |
| R3D-9 | Kept and simplified (hash of exact bytes is the identity; ref hints are never trusted over the record) |
| R3D-10, R3D-16 | Mechanism removed (no removal decisions, declined sets or selection constraint) |
| R3D-13 | Closed by D-23 (above) |
| R3D-14 | Kept (per-entry snapshot references) |
| R3D-15 | Kept and simplified (only live removals root payload) |

These are the author's claims. Independent confirmation is the job of
R4-design-review (D-23 requires review before implementation).

## Open questions

Q-54 (dependency of a remaining addon: proceed with a warning), Q-55
(observed reinstall after an applied removal is an explicit re-add), Q-56
(additive `code-state/1` fields instead of `code-state/2`), Q-57 (unreadable
referenced removal defers installs of its folders; GUI "Continue without
it"; no timer), and Q-58 (only named folders are removed; dropped folders go
to recovery whatever their content).

**Collision risk:** R4-settings-design and R4-ops-design may also append
questions from Q-54 onward. The integrator must renumber at merge; a note
in questions.md says so.

## Not done or out of scope

- `rollback.md` and `settings.md` do not exist on this base. removal.md
  names them without links, and its §6 settings contract and §8.1 rollback
  requirements must be reconciled with them at integration.
- Changes to domain.md, publication.md, apply.md and core-api.md are listed
  in removal.md §14, not made (not owned by this packet).

## Links

SPEC R-03, R-04, R-05, R-06, R-07, R-09, R-10, R-11, R-12 and R-13;
ACCEPTANCE A-04, A-09, A-10, A-16 to A-26, A-38 and A-45; decisions D-21,
D-23 and D-24 in [OWNER-DECISIONS.md](../../../OWNER-DECISIONS.md).
