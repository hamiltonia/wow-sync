# Work packet R2-design-review: independent design review

Role: verification. Model: gpt-6.1-sol. Rules: [COMMON.md](COMMON.md).

## Intent

Independently check R2-design (merged on `main`: `docs/ARCHITECTURE.md`,
`docs/design/`, `evidence/R2-design.md`) before implementation (E-06).

## Owned writes

Evidence file only. No design edits; findings go to the orchestrator.

## Check

1. Every slice behaviour traces to SPEC IDs without reinterpretation; the
   easy-to-get-wrong rules in AGENTS.md are respected (variant/account
   boundaries, Forever identity, WoW-running gate, publication ≠ applied,
   newer-compatible ordering without timestamp guessing, recovery before
   change, no Lua execution).
2. Policy lives only in the platform-free core (E-08); adapters hold no policy.
3. Formats: versioned, atomic publish, partial/corrupt input refused safely.
4. Work split: R2-core / R2-adapters / R2-slice file ownership is disjoint and
   each packet is implementable in one pass; interfaces are sufficient.
5. Required test cases cover negative/failure paths.
6. Undecided owner questions are listed, not silently decided.

Findings: severity (blocker / major / minor), location, why, suggested fix.
Verdict: ready / ready with fixes / not ready.
