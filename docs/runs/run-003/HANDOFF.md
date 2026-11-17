# Run 003 handoff

This is the resume point. The repository records are authoritative; prior
chat is not.

## State (2026-11-16, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Plan approved (D-20); see [PLAN.md](PLAN.md).
- Ledger: [USAGE.md](USAGE.md).
- Owner checks pending: [OWNER-CHECKS.md](OWNER-CHECKS.md).

| Phase | Workers | Status |
|---|---|---|
| A | R3-ui-design (opus), R3-infra (sonnet), R3-compat (sonnet), R3-removal-design (opus) | Done, all merged |
| B | R3-design-review (sol) | Review FAIL, merged `917bc05`. Recheck `09b2673`: **UI PASS**. Q-21 FAIL only on R3D-3 (owner proxy decision; see below). Removal FAIL (R3D-13 to R3D-16); retry 2 (final) running |
| C | R3-proto (sonnet), R3-mem (sonnet) | Both running |
| D | Owner prototype review | Pending |
| E | R3-check (sol), R3-review (opus) | Pending |
| F | R3-ui-build (conditional) | Pending |

## Incidents

None yet.

## Owner confirmation needed

- **Q-21 evidence rule (R3D-3).** The owner was unavailable when asked, so
  the orchestrator chose rule **A**, the most conservative option. It is
  recorded here as interim, pending owner confirmation.
  - A different-build or unknown-build candidate installs automatically only
    when the same addon's installed local copy declares exactly the same valid
    Interface set for every folder. Anything else becomes a decision.
  - Alternatives offered:
    - B: also allow new addons by majority local evidence.
    - C: keep the union of all local declarations.
  - The recheck (R3D-3, still open) notes that even rule A is a proxy:
    WoW's install files do not expose the interface numbers the client
    supports. Only the owner can accept a proxy. Until then, the code ships
    rule A, which is the most conservative.

## Follow-ups found

- `fake_client_blocks_real_apply` (runtime) can hit the same process
  start/exit race fixed in the adapters by R3-infra. Assign to a runtime
  packet.
- Dependabot: none of the 6 alerts has a minimal fix. **Owner decision
  needed** (from R3-infra):
  - **glib** (3 alerts): comes from `tray-icon` through Linux-only gtk3-rs,
    which is capped at 0.18. It is not compiled on Windows or macOS.
  - **grid** (3 alerts): only in the obsolete crates.io GPUI spikes
    (`spikes/gpui`, `spikes/mem`, `spikes/a11y`). Deleting those spikes
    would close alerts 1–4.

## Next actions

1. Merge each worker after the orchestrator re-runs offline verify.
2. Push and record usage.
3. Stop launching new work at ≥19,013 AIU cumulative.
