# R4-removal-design (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `docs/design/removal.md`;
- `docs/design/questions.md` (append only);
- your evidence file.

Read-only inputs:
- `docs/design/{domain,publication,apply,core-api}.md`;
- `docs/runs/run-003/evidence/R3-removal-design.md`;
- `R3-design-recheck.md` and `R3-design-recheck2.md` (the R3D findings);
- D-21 and D-23;
- the current core code in `crates/wow-sync-core` (only to learn its names
  and structure).

## Goals

1. **Rewrite `removal.md` for D-21 ("removed means removed everywhere") and
   the D-23 simplification.**
   - Any valid removal record for an addon in a variant scope wins over
     install candidates. The exception is an explicit later re-add.
   - Conflicting removal records for the same addon merge into **one**
     removal. This includes records that span packages and records with
     different package sets.
   - Define the merge as deterministic and order-independent:
     - the union of affected folders;
     - the newest re-add boundary.
   - **Close R3D-13 explicitly.** Show that no state remains that cannot
     be resolved, and that V7 (listing) holds.
   - Re-check R3D-1 to R3D-12 and state, for each one, how the rewrite
     keeps or changes its fix.
2. **Preserve these behaviours:**
   - an uninstall removes the active code **and** the settings;
   - recovery keeps both, uses the three-state history, and restores both
     through the GUI;
   - offline peers cannot resurrect removed data;
   - a re-add must be explicit;
   - the WoW-running block on game-file changes still applies.
3. **Implementation-ready sections:**
   - the core types and functions to add or change, mapped to existing
     modules;
   - the on-disk record format and its compatibility (`serde(default)`,
     forward-compatible);
   - a test list mapped to A-10, A-20, A-23, A-25 and A-26, including the
     negative cases;
   - which GUI screens are touched.
4. Record any new product question as `Q-NN` in `docs/design/questions.md`,
   with a safe default. Do not block on it.

## Done

- The evidence lists every R3D finding with its status, the R3D-13
  closure argument, and any open questions.
- `cargo xtask verify --tier offline` passes.
- Committed on your branch.
