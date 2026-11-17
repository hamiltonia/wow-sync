# R3-compat evidence (run-003)

Environment: Windows 11 x64 dev PC, Rust 1.100.0, worktree `wow-sync.worktrees\R3-compat`,
branch `run003/R3-compat`. Spec links: R-05, R-06, Q-21 (D-18), domain.md §6.

## Built

- `interface_supported` (pure, `wow-sync-core/src/target.rs`) used by `eligible_holders`, so
  plan and executor share it. Rule: when the holder's `build_version` differs from the local
  one, the candidate installs automatically only if **every** folder has a selected `.toc`
  with at least one `## Interface:` number and **every** number equals a number found in the
  receiving client's own installed folders' selected `.toc` files (status `Ok`). Otherwise
  `NeedsDecision(DifferentClientBuild)`. Missing numbers or no local evidence → decision.
  `DifferentTocMapping` still needs a decision; same build never consults interface numbers.
- Docs: domain.md §6, questions.md Q-21, tests.md §5a. Testkit helper `addon_interface`.
- Existing tests `different_client_build_needs_decision` and
  `ineligible_dominating_candidate_no_fallback` now use a differing interface number so they
  still exercise the decision path (the old ones had equal numbers).
- Required cases `Q-21:core` (6 tests) and `Q-21:slice` (2 tests), tier offline, all OSes
  (no Windows-only code).

## New tests and A-38 negative controls

Each mutation was applied, the named tests failed with exit 101, and it was restored.
No mutation is committed.

| Mutation | Tests that failed |
|---|---|
| remove the interface check (old behaviour) | `different_build_same_interface_installs_automatically`, `multi_folder_package_needs_every_folder_supported`, e2e `different_build_same_interface_applies` |
| `interface_supported` always true | `different_build_without_local_interface_evidence_needs_decision`, `different_build_different_or_missing_interface_needs_decision`, `different_client_build_needs_decision`, `multi_folder_package_needs_every_folder_supported`, e2e `different_build_different_interface_needs_decision` |
| per-toc `all` → `any` number | `different_build_different_or_missing_interface_needs_decision` |
| interface check without the build-differs guard | `same_build_ignores_interface_numbers` |
| skip the toc-mapping check | `different_mapping_still_needs_decision_with_same_interface`, `different_client_build_needs_decision` |

## Results (Windows 11 x64, local)

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny,
  markdown-links), exit 0.
- `cargo test --workspace`: core 178 passed, runtime 28 passed (adapters 45 passed, 3 ignored;
  other crates 24 + 2), 0 failed.
- macOS: NOT RUN locally (CI only; no Windows-only code in the new tests).

## Limits

- A stale local addon with an old interface number counts as evidence for that number.
- Exact equality only; no ranges or "newer is fine".
- Multi-number tocs need every number supported (conservative).
