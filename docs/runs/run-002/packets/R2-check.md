# Work packet R2-check: independent verification of the slice

Role: verification. Model: gpt-6.1-sol. Rules: [COMMON.md](COMMON.md).

## Intent

Independently verify the merged slice (E-06, A-37, A-38): the author's
evidence is not proof. Scope per [crates.md](../../../design/crates.md) §7
and [tests.md](../../../design/tests.md) §5b, §6.

## Owned writes

`docs/runs/run-002/evidence/R2-check.md` and `evidence/R2-check/`
(synthetic-data screenshots/notes only). No product edits; negative controls
are applied on a scratch branch and reverted, never committed.

## Work

1. Clean clone of `main` into a temp directory; `cargo xtask verify --tier
   offline`, `cargo test --workspace`; `--tier native` and `--tier provider`
   (record BLOCKED honestly).
2. Negative controls from tests.md §6: apply each mutation, record which
   tests fail; any mutation with no failing test is a finding.
3. Two synthetic installs end to end through the app's developer mode:
   publish, apply, running-game wait (FakeClient), ambiguity.
4. GUI checks G1–G9 with retained notes/screenshots, PASS/FAIL/BLOCKED each.
5. Memory per PLAN §6.6 for the app (idle, hidden; 3 runs).
6. Spot-check that runtime/app hold no ordering, grouping, target, plan or
   status logic (E-08).

Findings: severity, location, reproduction. Verdict.
