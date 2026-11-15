# Verification (`cargo xtask verify`)

Single entry point for repository verification (ENGINEERING E-08, ACCEPTANCE
A-39/A-43). Implemented in the `xtask` workspace member.

## Prerequisites

- rustup with the toolchain pinned in `rust-toolchain.toml` (fetched
  automatically on first `cargo` use; includes rustfmt and clippy).
- A native linker: MSVC Build Tools with the C++ workload on Windows, or Xcode
  Command Line Tools on macOS.
- cargo-deny: `cargo install --locked cargo-deny`. If missing, the `deny` check
  reports BLOCKED with this command; it is never skipped silently.

## Commands

```text
cargo xtask verify --tier offline|native|provider|all [--allow-blocked]
```

- Prints one line per check: `PASS`, `FAIL`, `BLOCKED` or `SKIPPED`, and writes
  the same data as JSON to `target/verify/report.json`.
- Exit 0 only if every requested check passed. Any FAIL exits non-zero.
  BLOCKED or SKIPPED exits non-zero unless `--allow-blocked` is given; the
  report and printout still say BLOCKED/SKIPPED.
- Exit 2 means a usage error.

## Tiers

| Tier | Checks | Status today |
|---|---|---|
| `offline` | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, required cases, `cargo deny check`, Markdown relative links | runnable |
| `native` | Real OS/WoW-file behavior on the local OS | BLOCKED: no native cases defined yet |
| `provider` | Real Dropbox test folder with enrolled peers | BLOCKED: requires D-08 Dropbox test folder and enrolled peers |

A tier that cannot run is BLOCKED, never PASS. `required-cases` is SKIPPED when
`cargo test` did not pass, because listed cases then cannot be verified.

## Required cases

`tests/required-cases.toml` maps case IDs to the libtest names that cover them:

```toml
[[case]]
id = "A-39"            # acceptance or case ID
tier = "offline"       # offline | native | provider
tests = ["inventory::tests::zero_tests_with_listed_cases_is_a_problem"]
```

Names are exactly as `cargo test` prints them (`module::test_name`, without the
crate). The verifier fails when a listed test did not run, was ignored, or
failed, or when a tier ran zero tests while it has listed cases. To add a case,
append a `[[case]]` entry in the same change as the test. Unknown tiers,
duplicate ID/tier pairs, empty test lists and unknown keys are rejected.
Currently only the `offline` inventory is enforced by running tests.

## Markdown link check

All `*.md` files (except `target/` and `.git/`) are scanned for inline
relative links outside fenced code blocks and inline code spans; missing targets fail. External URLs
and `#anchor` fragments are not validated.
