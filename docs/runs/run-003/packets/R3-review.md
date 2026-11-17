# R3-review (Integration, opus-5.5): phase E

Read [COMMON.md](COMMON.md) first.

**Owned paths:** `docs/runs/run-003/evidence/R3-review.md` only. Do not
change code or docs. Write down findings; the orchestrator assigns the
fixes.

## Scope

Review the merged state of `main` after run-003 against ENGINEERING
E-01 to E-09 (especially E-06 and E-08), ACCEPTANCE A-37 to A-44, and
D-21.

1. **E-06 (safety and supply chain):**
   - dependency or permission changes in run-003, and `deny.toml`;
   - no execution of imported content;
   - no private data, user paths or SIDs in committed evidence or
     screenshots (grep the run-003 evidence tree);
   - CI permissions;
   - the `tools/mem` harness: does it only stop processes it launched?
2. **E-08 (maintainability and architecture):**
   - Core is still the single policy authority. Does the app or runtime
     re-implement policy, for example status headline, next action or
     eligibility?
   - `ARCHITECTURE.md` and `crates.md` match the code, including the new
     `core::view` and the runtime snapshot/pause/quit APIs.
   - Dead code left by D-21.
   - The prototype versus app code split: is the duplication acceptable,
     and is there a plan?
3. **D-21 consistency:** no Interface or build gating anywhere (code, UI
   copy, docs). The removal design note is present. `questions.md` matches
   D-21.
4. **Records (E-05, A-40):** a fresh orchestrator could resume from
   `docs/runs/run-003/HANDOFF.md`, `USAGE.md`, the packets and the
   evidence alone.
5. **Remaining work:** list what is needed before a first release
   candidate. Use SPEC, ACCEPTANCE and the evidence, grouped by acceptance
   ID, and mark each item done, partial or not started.

## Output

- **Findings:** list each with ID RV-n, its severity (blocker / major /
  minor), the location, the evidence and the suggested fix.
- **Verdict:** PASS, PASS-with-minors, or FAIL.
- Run `cargo xtask verify --tier offline`, then commit only your evidence.
