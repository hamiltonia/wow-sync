# Evidence: R4-core-ops

Branch `run004/R4-core-ops`, based on main b8707ae. Implementation commit 8587ca4; this file is added in the following commit.

## Implemented

| ID | What | Tests |
| --- | --- | --- |
| Q-49, T-26-2, T-R4D10, T-R4D21, T-R4D23 | `choice.rs`: keep/resume, pinning (an unconsidered newer copy raises a decision and never replaces the keep), carried exclusions, `ConstraintHooks` for `live_constraints` and `excluded` | `choice_tests` (7) |
| Q-50 | Resume ends the pin, keeps inherited exclusions; `allow_again` must be a subset of the inherited hashes | `choice_tests` |
| T-C15, T-26-1/3/4/5/8/9 | `decide.rs`: `DecideBehaviour`, evidence token, stale item gives `Changed` and writes nothing, blocked/retired gives `NotAllowed` | `decide_tests` (5) |
| Q-60, Q-61, T-45-3..8, R4D-31 (core) | `retire.rs`: retirement plan, cutoffs, held-self publishes nothing, master never auto-replaced | `retire_tests` (8) |
| T-26-6/7, T-R4D31, T-45-7 (runtime) | `decisions.rs`: `route`, kept queue, `write_record` | 6 tests in `decisions::tests` |

Required-cases entries (new): `R4-core-ops:code-choice`, `:decide`, `:retirement`, `:runtime-routing`.

## Verification (actual)

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links), after the final code state.
- `cargo xtask verify --tier native`: PASS.
- Counts: core lib 246 (226 baseline + 20 new), runtime 56 (50 + 6), 26 new tests.
- Native T-C11: NOT RUN (as in the foundation evidence).

## A-38 negative controls (mutate, observe failure, restore; nothing committed)

| Control | Mutation | Failing test | Result |
| --- | --- | --- | --- |
| Q-49 pinning | `if false && !all_different` in `new_versions` | `choice_tests::unconsidered_newer_copy_raises_a_decision_and_never_replaces_the_keep` | exit 101 |
| Q-50 exclusions | `build_resume` drops inherited excludes | `choice_tests::resume_ends_the_pin_but_keeps_inherited_exclusions` | exit 101 |
| Stale evidence | skip `current.evidence != *shown` | `decide_tests::a_changed_item_refuses_and_writes_nothing` | exit 101 |
| Retired not publishing | skip standing check in `plan_retirement` | `retire_tests::a_retired_computer_plans_no_retirement_and_pauses_publishing`, `...held_self_is_neither_active_nor_retired_and_publishes_nothing` | exit 101 |
| Held treated as retired | `settle` merges Held with Retired | `decisions::tests::held_record_is_written_with_reserved_bytes_when_the_retirement_turns_void` | exit 101 |

## Deviations and deferred

- Hooks are not wired into `target::compose`, `service.rs` or `control.rs` (outside my files; no decide/retire control variants exist). Handover to the integration step.
- The `NewVersion` item option "follow newest" has no label; `ChangeDecision` (resume with empty `allow_again`) is used.
- No removal record is written for the smaller-copy keep (single-record `DecisionEffect`); `FolderDrop` handover to R4-core-removal.
- Writer part-packing is unsupported (`TooLarge`); readers support parts.
- The kept queue is not persisted (no `serde_json` in the runtime crate; `Cargo.toml` is not mine).
- `decisions.rs` has `#![allow(dead_code)]` until the service wires it in.
- Ambiguous, PartialCandidate and ChoicesDiffer item builders are not implemented; only `new_version_item` and generic `build_item`.
- GUI-only rows (T-26-10..14, T-45-9, UQ-1/UQ-2 text) go to R4-app; T-27, T-28, T-33 go to R4-notify-diag.
