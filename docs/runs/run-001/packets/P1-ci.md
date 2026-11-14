# Work packet P1-ci: GitHub Actions

Run: 001, Phase 1. Role: implementation. Model: claude-sonnet-5.5 (D-04).
May run in parallel with P1-verify; depends only on its interface.

## Intent

Repeatable CI verification (E-01, E-06, E-08) with minimal privileges, and
build evidence for all four targets, honestly labeled.

## Checkout

- Worktree `D:\GitHub\wow-sync.worktrees\P1-ci`, branch `run001/P1-ci` from
  `main`. Do not edit any other checkout.

## Owned writes

- `.github/workflows/` (new files only; do not edit `.github/copilot-instructions.md`)
- `docs/runs/run-001/evidence/P1-ci.md`

## Work

1. Workflow `ci.yml` on push and pull_request to `main`:
   - `permissions: contents: read` at top level; no secrets; pin third-party
     actions to full commit SHAs with a version comment.
   - Job `verify-offline` on `windows-latest` and `macos-latest`: install the
     toolchain from `rust-toolchain.toml`, install cargo-deny (pinned version),
     run `cargo xtask verify --tier offline` (interface from P1-verify packet).
   - Job `build-targets`: build the workspace for `x86_64-pc-windows-msvc`,
     `aarch64-pc-windows-msvc` (on Windows) and `x86_64-apple-darwin`,
     `aarch64-apple-darwin` (on macOS). Name the jobs/steps "build-only";
     upload nothing implying runtime support.
   - Use GitHub-hosted ARM64 runners only if available on this repo's plan;
     otherwise document their absence. Do not claim ARM64 runtime evidence.
2. Do not add release, publishing, or deployment workflows.
3. Validate workflow syntax locally if a tool is available (e.g. actionlint);
   otherwise say not run. Do not push; the orchestrator pushes after review
   and records the first real CI run.
4. Commit (`run001(P1-ci):` prefix, Copilot co-author trailer). Do not push.

## Evidence

`docs/runs/run-001/evidence/P1-ci.md`: workflow design, pinned action SHAs and
sources, what each job proves and does not prove, local validation results.

## Stop conditions

No sub-agents. Two failed attempts: stop. No secrets, tokens, or repository
settings changes.
