# R3-design-review (Verification, gpt-6.1-sol)

Read [COMMON.md](COMMON.md) first.

**Owned paths:** `docs/runs/run-003/evidence/R3-design-review.md` only. Do
not edit any design or code file. Write down findings; the authors fix them.

## Scope

Independently review three merged phase-A outputs against SPEC.md,
ACCEPTANCE.md, AGENTS.md product rules, and OWNER-DECISIONS D-18 to D-20.

1. **UI design:** `docs/design/ui/**` and Q-26 to Q-39.
   - Every screen and state required by R-12, R-13, A-24, A-25, A-26, A-42,
     A-45 and A-46 is covered.
   - No screen invents product policy that is not in SPEC or core.
   - The data gaps (UG-*) are real: check them against
     `crates/wow-sync-core/src/status.rs` and the runtime.
   - Contrast claims: recompute at least 6 token pairs per theme.
   - Feasibility at GPUI `v1.28.1`: spot-check claims against the pinned
     source in the cargo git checkout or the run-002 R2-pin evidence.
   - `prototype.md` is buildable without guessing, and its owner checklist
     is observable.
2. **Removal design:** `docs/design/removal.md` and Q-40 to Q-48.
   - Removal is never inferred from absence alone.
   - Uninstall preserves and restores both code and settings.
   - Three-state history is respected.
   - Rollback rejection and retired computers cannot resurrect removed
     addons.
   - Crash/journal idempotency is complete.
   - Test obligations map to acceptance IDs.
   - Look for unsafe defaults and contradictions with `apply.md` and
     `publication.md`.
3. **Q-21 change** (merge `944141a`): the `interface_supported` rule in
   `crates/wow-sync-core/src/target.rs`, `domain.md` §6, and the tests.
   - Is it at least as conservative as D-18 requires?
   - Can a missing, malformed or multi-number Interface line cause an
     unsafe automatic install?
   - Does the local evidence wrongly include folders from another variant
     or an unusable folder?
   - Run `cargo test -q -p wow-sync-core` and the runtime e2e test, and
     record the results.

## Output

- **Findings:** list each with ID R3D-n, its severity (blocker / major /
  minor), the exact file and section, the evidence, and the required fix.
- **Verdict:** PASS, PASS-with-minors, or FAIL. FAIL if there is any blocker
  or major finding.
- Commit only your evidence file, with prefix `run003(R3-design-review):`
  and the Copilot trailer. Run `cargo xtask verify --tier offline` first.
