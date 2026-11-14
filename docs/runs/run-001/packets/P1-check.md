# Work packet P1-check: independent verification of Phase 1

Run: 001, Phase 1. Role: independent verification. Model: gpt-6.1-sol (D-04).
Starts after P1-verify and P1-ci are merged to `main`.

## Intent

Independently establish A-38 (initial), A-39 (initial) and A-40. The authors'
reports are not evidence.

## Checkout

- Fresh clone of `main` into a temp directory for checks.
- Write only to worktree `D:\GitHub\wow-sync.worktrees\P1-check`, branch
  `run001/P1-check`, path `docs/runs/run-001/evidence/P1-check.md`.

## Work

1. From the clean clone, follow only `docs/VERIFY.md` and run
   `cargo xtask verify --tier all`. Record real output and the per-tier status.
   Expected: offline PASS; native and provider BLOCKED; overall non-zero.
2. Negative controls, each in the temp clone (never committed):
   a. Add a required case to `tests/required-cases.toml` naming a test that
      does not exist: verify must fail.
   b. Mark a listed test `#[ignore]`: verify must fail.
   c. Introduce a clippy warning and a formatting error: verify must fail.
   d. Add a dependency with an MIT-incompatible license (e.g. a GPL-3.0 crate)
      : verify must fail on deny.
   e. Break a relative Markdown link: verify must fail.
   Record which controls the verifier caught and which it missed.
3. Fresh-worker onboarding (A-40): starting only from AGENTS.md, locate intent,
   decisions, this run's plan and packets, verification commands and evidence
   locations. Report anything missing or contradictory.
4. Check the first real GitHub Actions run on `main` (`gh run list`/`gh run
   view`): per-job status, and whether job names avoid implying runtime support.
5. Commit the evidence file (`run001(P1-check):` prefix, Copilot co-author
   trailer). Do not push or merge.

## Stop conditions

No sub-agents. Do not fix defects; report them. No writes outside the evidence
file. Two failed attempts at a step: stop and report.
