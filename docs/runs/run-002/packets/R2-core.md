# Work packet R2-core: platform-free domain core

Role: implementation. Model: claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

Implement `crates/wow-sync-core` exactly as specified by the slice design
([../../../design/README.md](../../../design/README.md)) as amended by any
design-review fixes merged on `main`. E-08, A-39, A-43; slice R-02..R-05,
R-11, R-12 policy.

## Owned writes

Per [crates.md](../../../design/crates.md) §4: `crates/wow-sync-core/**`;
root `Cargo.toml` (exact §3 text); `Cargo.lock` (own dependencies);
`tests/required-cases.toml` (R2-core block at the top, §5.2); evidence.

## Work

1. Modules and APIs from crates.md §2, core-api.md, domain.md, publication.md
   (records), apply.md (plan + executor over port traits, recovery).
2. Required tests from tests.md §1 for R2-core, as unit tests in each module,
   each listed in `tests/required-cases.toml`. Include every negative/failure
   case listed; use `testkit::FakeEffects` with failure injection.
3. The `arch` check (crates.md §6).
4. Apply the implementer fixes F-01 (enforce the synthetic marker in core
   validation) and F-02 (verify restored live content before terminal
   rollback, including after crashes) from
   [R2-design-recheck2](../evidence/R2-design-recheck2.md), each with a
   required test.
5. Where the design is ambiguous or contradictory, choose the safer reading,
   note it in evidence under "design gaps"; do not change design docs.

## Done when

`cargo xtask verify --tier offline` passes; evidence lists each required case
with PASS, design gaps, and anything not implemented (labelled NOT RUN/BLOCKED).
