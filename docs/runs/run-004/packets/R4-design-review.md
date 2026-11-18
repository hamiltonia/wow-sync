# R4-design-review (Verification, gpt-6.1-sol)

Read [COMMON.md](COMMON.md) first. This is an independent review: the
authors' claims are not evidence.

**Owned paths:**
- your evidence file `docs/runs/run-004/evidence/R4-design-review.md`;
- `evidence/R4-design-review/**`.

Do not edit any design file.

## Scope

Review these documents against SPEC.md, ACCEPTANCE.md, ENGINEERING.md and
D-21 to D-24:
- `docs/design/removal.md`;
- `docs/design/settings.md`;
- `docs/design/rollback.md`;
- `docs/design/ops.md`;
- the new questions Q-54 to Q-84 in `docs/design/questions.md`.

## Checks

1. **R3D-13 closure (D-23 gate).**
   - Try to construct a removal state that cannot be resolved, a
     merge that depends on arrival order, or an offline resurrection.
   - Re-check the R3D-1 to R3D-16 status claims in
     `evidence/R4-removal-design.md` against the earlier findings in
     `docs/runs/run-003/evidence/R3-design-recheck*.md`.
2. **Settings and rollback.**
   - **Master:** the settings master is never silently re-elected.
   - **Yield:** non-master writes yield with a recovery copy kept.
   - **No merging:** competing intentional choices are not merged.
   - **Three states:** current plus two previous, with visible
     exceptions.
   - **Boundaries:** the variant and account/character boundaries.
   - **No execution:** saved-variable files are never parsed as Lua.
   - **WoW running:** the block on game-file changes applies.
   - **Rollback:** the rejection survives offline peers (A-18/A-19).
   - **Rollback with removal:** removal and re-add interact consistently.
3. **Ops.**
   - **Decisions:** decision actions work as designed, including UQ-1/UQ-2
     per D-23.
   - **Retire (A-45):**
     - no automatic master switch;
     - history kept;
     - stale publications ignored;
     - re-enrollment is explicit.
   - **Notifications:** check the scope and the opt-out.
   - **Update check (D-24 P4-1):**
     - anonymous, at most daily, opt-out, fails silently;
     - the TLS and licence approach is sound on all four targets.
   - **Diagnostics redaction (A-33):** completeness, including planted
     strings.
4. **Consistency across documents.**
   - Shared terms ("settings master", "reassign", recovery points,
     history states) mean the same thing everywhere.
   - Interactions between removal, rollback, settings and retire have no
     contradictions or gaps.
   - The on-disk formats are compatible with each other and with the
     existing `code-state/1` and the journal.
5. **Implementability.**
   - The types and module mappings exist or are plausible in the current
     `crates/wow-sync-core`.
   - The test lists cover every A-ID claimed, with negative cases.
   - Each document can be implemented independently by the three planned
     core packets (removal, settings plus rollback, ops). Flag shared
     types that need one owner.
6. **Questions.** Flag any Q-54 to Q-84 default that is unsafe, or that
   needs an owner decision before implementation. Numeric limits need
   owner approval.

## Output

- Findings named `R4D-N`, each with severity (blocker, major, minor), the
  location, the problem and a suggested fix.
- A verdict for each document: PASS, or FAIL with blockers or majors.
- A list of questions that need the owner before phase C.
- `cargo xtask verify --tier offline` passes. Commit on your branch.
