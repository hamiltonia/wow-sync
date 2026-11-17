# R3-infra (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `xtask/**`;
- `.github/workflows/**`;
- `deny.toml`;
- root `Cargo.toml`/`Cargo.lock` and `spikes/**/Cargo.lock`, for dependency
  bumps only;
- `docs/VERIFY.md`;
- `tests/required-cases.toml`, for tier moves only;
- test-only code in `crates/wow-sync-adapters/src/**`, for the flaky-test fix
  only. Do not change production behaviour there; if the fix needs a product
  change, stop and report.

## Tasks

1. **Native tier (Q-17).**
   - Make `cargo xtask verify --tier native` execute on Windows: run the
     `native`-tier required cases and fail if any listed test did not run or
     failed.
   - On non-Windows it reports BLOCKED, never PASS.
   - Add it to the Windows CI job.
   - Provider tests stay BLOCKED until owner Check 2 is done.
   - Add an xtask self-test proving that a missing native test fails the tier.
2. **Flaky test.** Windows CI on commit `7da6d0f` (run 43821566823) failed in
   the `wow-sync-adapters` lib tests without naming the test. It passed on the
   next run with the adapters crate unchanged.
   - Make xtask print the failing test names when `cargo test` fails, so CI
     logs are diagnosable.
   - Find the likely flaky test by running the adapters tests at least 30
     times locally (`--test-threads` default and 1), and by reviewing for
     timing, temp-dir, process or file-locking races.
   - Fix the root cause in test code. If it cannot be reproduced, add the
     diagnostics, record what you tried, and do not paper over it with
     retries.
3. **Dependabot (D-18).**
   - List alerts with `gh api repos/hamiltonia/wow-sync/dependabot/alerts`.
   - Fix each alert by a minimal version bump when one exists, including
     `spikes/**` lockfiles.
   - If no fix exists or the spike is obsolete, record a recommendation; do
     not dismiss alerts.
   - `cargo deny` must still pass.
4. Keep CI green. You cannot push, but the orchestrator will run CI after
   merge. Reason about macOS.

## Evidence

`docs/runs/run-003/evidence/R3-infra.md`, covering:
- the native tier output;
- the flaky-test investigation, with counts;
- the alert table: package, from, to, status.
