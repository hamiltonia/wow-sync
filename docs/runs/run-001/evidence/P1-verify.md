# P1-verify evidence

Run 001, packet P1-verify. Machine: Windows 11 x64, PowerShell. Rust 1.100.0
(pinned), MSVC linker working. `cargo` was put on PATH via `~/.cargo/bin`.
cargo-deny installed locally with `cargo install --locked cargo-deny`:
**cargo-deny 0.20.2**.

## Commands and results (all executed)

| Command | Result |
|---|---|
| `cargo fmt --all` then `cargo xtask verify --tier offline` | PASS, exit 0: fmt, clippy, test, required-cases, deny, markdown-links all PASS |
| `cargo xtask verify --tier all` | exit 1, overall BLOCKED: offline all PASS; `native/tier` BLOCKED "no native cases defined yet"; `provider/tier` BLOCKED "requires D-08 Dropbox test folder and enrolled peers" |
| `cargo xtask verify --tier all --allow-blocked` | exit 0, report still says `"overall": "BLOCKED"`, `"allow_blocked": true` |
| `cargo test -p xtask` | PASS, 17 passed, 0 ignored |

`target/verify/report.json` was written (fields: tiers, allow_blocked, overall,
exit_ok, checks[tier,name,status,detail]).

## Negative demonstration (executed, then source restored)

1. Added `#[ignore]` to listed test
   `inventory::tests::zero_tests_with_listed_cases_is_a_problem`:
   `offline/test` PASS (cargo itself is green), but
   `[FAIL] offline/required-cases: A-39: listed test '...' was ignored`,
   overall FAIL, exit 1.
2. Renamed that test so the listed name no longer exists: 
   `[FAIL] offline/required-cases: A-39: listed test '...' did not run`, exit 1.

The zero-test-with-listed-cases and failed-test paths, and the missing-tool
BLOCKED path (`unavailable_tool_is_blocked_with_install_hint`, using a
nonexistent program), are covered by unit tests only; they were not
demonstrated end to end (cargo-deny was installed, so the real BLOCKED output
for a missing cargo-deny was not observed).

## Dependencies (xtask only)

| Crate | Why | License |
|---|---|---|
| serde 1.0.229 (derive) | typed inventory parsing and report serialization | MIT OR Apache-2.0 |
| serde_json 1.0.154 | `report.json` output | MIT OR Apache-2.0 |
| toml 0.8.23 | `tests/required-cases.toml` parsing | MIT OR Apache-2.0 |

22 packages in the lock graph in total. `cargo deny check` passed with
`deny.toml` (allow: MIT, Apache-2.0, Unicode-3.0, BSD-3-Clause, ISC; wildcards
denied; unknown registries/git denied). No process or tokio-like extras; link
checking and argument parsing are hand-written to avoid more crates.

## Limitations / deviations

- `native` and `provider` are placeholders and always BLOCKED.
- The required-case inventory contains only A-39 entries that point at
  xtask's own tests; real product cases do not exist yet.
- Test-name matching is by libtest name without crate, so same-named tests in
  different crates are indistinguishable.
- Link check is inline links only (no reference-style links, no anchors).
- `deny.toml` is in this packet's owned writes, and P1-ci also lists it in its
  plan; expect a merge conflict to resolve there.
- cargo-deny fetches the advisory database over the network, so `deny` needs
  network access (it passed here).
- The `target` list in `deny.toml` covers the four pinned targets.
- Only Windows x64 was exercised; macOS not run.
