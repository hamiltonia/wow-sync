# R4-core-settings evidence (run-004, phase C step 2)

Branch `run004/R4-core-settings` (base main b8707ae). Synthetic data only; no real WoW/Dropbox data touched.

## Implemented

| Area | Files | Tests |
|---|---|---|
| Rejection / lift / rollback planning, four-branch recovered chain (R4D-32) | core `reject.rs` | `reject_tests` (13) |
| Settings scope, capture classes, authority, revision graph, target cases, epoch/publish, paired selection, retention, resume, hooks | core `settings.rs` | `settings_tests` (25) |
| WTF scan (two shapes only, no code run, junction-safe, size cap) | adapters `wtf.rs` | 4 |
| Blob/manifest/local store (hash-verified, manifest refused without blob) | adapters `settings_store.rs` | 3 |
| Scope observation, capture, settings apply plan | runtime `settings_cycle.rs` | 4 |
| Rollback settings result reflects what was restored | runtime `rollback_flow.rs` | 1 |

Cases are registered in `tests/required-cases.toml` (`R4-core-settings:settings-core`, `:rejection-rollback`, `:adapters-and-cycle`).
Counts: core 226 -> 264 (+13 +25); adapters 52 -> 59 (+7); runtime +5.

Q-49/Q-50 preserved. Q-77 has no approved value: `MAX_SETTINGS_FILE_BYTES` stays `None`, so real-data settings sync is disabled (`sync_enabled == false`; tested). Tests use a test-only cap.

## Verification (actual)

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo xtask verify --tier native`: PASS.

## A-38 negative controls (mutate, run, restore; nothing committed)

| Mutation | Result |
|---|---|
| epoch check `<=` -> `<` (not strict) | FAILED (exit 101): `the_authority_epoch_holds_an_experiment_and_needs_a_strictly_later_capture` |
| dirty-epoch check disabled | FAILED (exit 101), same test |
| reject.rs: "all copies rejected" -> "any" | FAILED (exit 101): `the_recovered_chain_decides_from_sources_in_four_branches` |

Not run as mutations (time budget): non-authority publish, missing-parent-as-root, collapse equal heads, policy-by-package, yield-on-unchanged, basis-shrunk, single-witness, historical author re-check, auto-elect master, exact-content match, three-state limit, different-account apply. Those behaviours have positive/negative unit tests but no mutation evidence.

## Deviations / open issues

- `ledger.rs` `kind_rules` (not owned) accepts a `SettingsRevision` parent only if unit and kind both equal; mixed-kind chains (push -> auto) become invalid, contradicting settings.md §3.3. Needs a fix by the ledger owner. My tests exercise abstract `RevNode` graphs, not ledger-level mixed-kind chains.
- `target::compose` / `service.rs` do not call `ConstraintHooks`/`SettingsHooks`; wiring is outside my files. Hook signatures lack `Derived`/sources, so `RejectBehaviour` carries a valid set and `SettingsBehaviour` per-unit facts.
- `SettingsLocal` lacks epoch/seq/capture-kind fields; held in memory via `UnitFacts`; persistence not done.
- `CodeState` has no features; Q-84 uses caller-supplied features. `Explanation` has no rollback variant (uses `AppliedHere`). `plan_settings` returns empty (runtime composes the apply).
- `rollback_flow.rs` is only the result-classification helper; building `RollbackInput` from live scans, the allow-again lift flow, and offline-resurrection wiring are deferred (core logic and tests exist in `reject.rs`).
- No ledger-level test for `policy_heads` / `graph_from_ledger`.
- Deferred / foundation-covered only: T-S02/S04 capture frequency, T-S23/S24 crash/WoW-start between renames, T-S28 replica fault injection, T-S31 group status. GUI-only T-S32 and T-R18 handed to R4-app.
- Orphan blobs possible if a manifest write fails after blobs stored; unreferenced and harmless.
