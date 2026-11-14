# Work packet P1-verify: verification entry point

Run: 001, Phase 1. Role: implementation. Model: claude-sonnet-5.5 (D-04).

## Intent

One documented verification entry point with honest tiers (E-08, A-39, A-43).
Unavailable tiers report **blocked**, never pass. Required-case discovery must
catch zero-test and missing-case false greens.

## Checkout

- Worktree `D:\GitHub\wow-sync.worktrees\P1-verify`, branch `run001/P1-verify`
  from `main`. Do not edit any other checkout.

## Owned writes

- `xtask/` (new workspace member), root `Cargo.toml` (members only),
  `Cargo.lock`, `.cargo/config.toml` (alias), `deny.toml`
- `tests/required-cases.toml` (inventory; may start empty)
- `docs/VERIFY.md`, `docs/runs/run-001/evidence/P1-verify.md`

## Interface (other packets depend on this; do not change it)

- `cargo xtask verify --tier offline|native|provider|all`
- Exit 0 = every requested tier passed. Non-zero otherwise.
- Prints a per-check summary with status PASS / FAIL / BLOCKED / SKIPPED and
  writes the same as JSON to `target/verify/report.json`.
- A BLOCKED or SKIPPED required check makes the run non-zero unless invoked
  with an explicit `--allow-blocked` flag, and then the report still says so.

## Work

1. `offline` tier: `cargo fmt --all --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`
   (licenses compatible with MIT, advisories, bans, sources), and a Markdown
   relative-link check over repository docs. Missing tools (e.g. cargo-deny not
   installed) are BLOCKED with the install command, not silently skipped.
2. Required-case inventory: `tests/required-cases.toml` lists case IDs with the
   test name(s) that cover them and the tier. The verifier parses test output
   (`--format` or libtest output) and fails if a listed test did not run or
   was ignored, or if a tier ran zero tests while it has listed cases.
3. `native` and `provider` tiers: for now each reports BLOCKED with reason
   ("no native cases defined yet" / "requires D-08 Dropbox test folder and
   enrolled peers") rather than passing.
4. xtask's own logic gets real unit tests (inventory parsing, zero-test and
   missing-case detection, status aggregation). No dummy tests.
5. Dependencies: minimal, justified in the evidence file, and passing deny.
6. `docs/VERIFY.md`: prerequisites (rustup, MSVC Build Tools / Xcode CLT,
   cargo-deny install command), commands, tier meanings, how to add a required
   case.
7. Commit (`run001(P1-verify):` prefix, Copilot co-author trailer). Do not push
   or merge.

## Evidence

`docs/runs/run-001/evidence/P1-verify.md`: commands run, real abbreviated
output, a demonstration that removing/ignoring a listed test makes verify fail,
dependency justification, limitations.

## Stop conditions

No sub-agents. Two failed attempts at a step: stop and report. No writes outside
owned paths; no credentials or paid services.
