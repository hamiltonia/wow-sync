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

## Retry 1 (R3-compat-fix): R3D-1, R3D-2, R3D-3

Environment: Windows 11 x64, Rust 1.100.0, worktree `R3-compat-fix`, branch `run003/R3-compat-fix`.

- **R3D-1 fixed.** `TocInfo`/`TocSummary` gain `interface_valid` (true only for at least one
  header, every token a valid `u32`, repeated headers agreeing). Empty, non-numeric,
  overflowing tokens and conflicting repeated headers fail closed. Wire: the field is
  `#[serde(default)]`, so records written earlier read as invalid (cross-build candidates
  from them need a decision until republished); older readers ignore the new field.
- **R3D-2 fixed.** Only two present, equal builds skip the interface rule.
- **R3D-3 rule A (orchestrator interim, owner to confirm).** A different/unknown-build
  candidate installs only if the same package is installed locally and, for every candidate
  folder, the local copy's valid Interface set exactly equals the candidate's. Union
  evidence was removed.
- Tests: core `malformed_interface_tokens_fail_closed`, `missing_builds_apply_the_interface_rule`,
  `multi_client_local_toc_does_not_authorize`, `stale_unrelated_local_addon_does_not_authorize`,
  `new_addon_or_added_folder_on_other_build_needs_decision`, toc
  `interface_validity_requires_clean_agreeing_headers`, inventory
  `summary_without_interface_valid_reads_as_invalid`; e2e `different_build_same_interface_applies`
  (same addon updated), `..._different_interface_...`, `..._malformed_interface_...`,
  `..._new_addon_...`. Existing positive tests kept (same-addon match still applies).

A-38 negative controls (mutate, observe exit 101, restore; none committed):

| Mutation | Failing tests |
|---|---|
| `interface_valid` ignores bad tokens/conflicts | `malformed_interface_tokens_fail_closed`, `interface_validity_requires_clean_agreeing_headers`, `parses_header_keys_with_bom_and_crlf`, e2e `different_build_malformed_interface_needs_decision` |
| build check skips when both builds are `None` | `missing_builds_apply_the_interface_rule` |
| evidence = any local folder with an equal set (union style) | `stale_unrelated_local_addon_does_not_authorize`, `new_addon_or_added_folder_on_other_build_needs_decision`, e2e `different_build_new_addon_needs_decision` |
| subset instead of exact equality | `multi_client_local_toc_does_not_authorize` |
| remove `#[serde(default)]` | `summary_without_interface_valid_reads_as_invalid` |

Results: `cargo xtask verify --tier offline` PASS (Windows 11 x64, local; macOS NOT RUN, CI only).
`cargo test --workspace`: core 185, runtime 30, adapters 45 (3 ignored), 0 failed.
