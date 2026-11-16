# Work packet R2-review: integration review

Role: integration review. Model: claude-opus-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

E-06/E-08 review of run-002 as merged on `main`: design conformance, no
policy outside core, evidence matches claims, safety rules in AGENTS.md
respected, owner questions and limitations visible.

## Owned writes

`docs/runs/run-002/evidence/R2-review.md` only.

## Work

1. Read PLAN, all run-002 evidence (incl. R2-check), the design and the code
   in `crates/`.
2. Check each claim in evidence against code/tests; flag overclaims.
3. Check easy-to-get-wrong rules (AGENTS.md) in code paths of the slice.
4. List required follow-ups for run-003, and owner decisions pending.

Findings: severity, location, why, fix. Verdict: accept / accept with
follow-ups / reject.
