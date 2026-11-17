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
| `native` | `cargo test --workspace`, then the `native` required cases (Windows junction, locked-file, process and fake-client tests) | runnable on Windows; BLOCKED on other hosts |
| `provider` | Real Dropbox test folder with enrolled peers | BLOCKED: requires D-08 Dropbox test folder and enrolled peers |

A tier that cannot run is BLOCKED, never PASS. `required-cases` is SKIPPED when
`cargo test` did not pass, because listed cases then cannot be verified.
The `native` tier FAILS if any listed native test did not run, was ignored or
failed. When `cargo test` fails, xtask prints a `failed tests:` list of the
failing test names after the output tail.

## Required cases

`tests/required-cases.toml` maps case IDs to the libtest names that cover them:

```toml
[[case]]
id = "A-39"            # acceptance or case ID
tier = "offline"       # offline | native | provider
tests = ["inventory::tests::zero_tests_with_listed_cases_is_a_problem"]
```

Names are exactly as `cargo test` prints them (`module::test_name`, without the
crate). The `required-cases` check of an executable tier (today `offline`)
FAILS when any of these hold, independent of the inventory contents:

- the tier executed zero tests (ignored tests do not count as executed);
- the inventory lists no cases for that tier (an empty inventory is a missing
  obligation, not success);
- a listed test did not run, was ignored, or failed.

To add a case, append a `[[case]]` entry in the same change as the test.
Unknown tiers, duplicate ID/tier pairs, empty test lists and unknown keys are
rejected. Blocked tiers (`native`, `provider`) are not subject to this check
because they do not run.

## Markdown link check

Only git-tracked `*.md` files (`git ls-files -z -- '*.md'`) are scanned, so
untracked caches and vendored files are ignored; symlinks and junctions are
never followed. Inline relative links outside fenced code blocks and inline
code spans must resolve; missing targets fail. External URLs and `#anchor`
fragments are not validated. If git is unavailable the check is BLOCKED.

## Report validity

`target/verify/report.json` includes `git_sha`, `git_dirty` and `generated_at`
(UTC). A report is valid only for the commit it names (and only if
`git_dirty` is false when claiming a clean commit); compare it with
`git rev-parse HEAD` and check the timestamp. xtask deletes any existing
report before running checks, so a missing report means the run did not
complete. If `xtask` itself fails to compile, it cannot run and cannot delete
the old report: treat a report whose SHA/timestamp does not match as stale.