# Test strategy and required cases

Part of the [slice design](README.md). Tests are behavioral: each name below
must fail for a plausible broken implementation (A-38). Names are libtest
names without the crate (`<module>::tests::<name>`), unique workspace-wide
([crates.md](crates.md) §2). Each packet adds its own `[[case]]` blocks to
`tests/required-cases.toml` in the same commit as the tests, at the place
given in [crates.md](crates.md) §5.

## 1. Tiers and case IDs

| Tier | Runs where | Use |
|---|---|---|
| `offline` | `cargo xtask verify --tier offline` on Windows **and** macOS CI and the dev PC | Everything portable: all core tests; adapter tests that use only `std::fs` in a temp dir; runtime end-to-end tests with an injected activity source |
| `native` | Windows only (`#[cfg(windows)]` tests). `cargo test --workspace` executes them on Windows, but the `native` tier of `verify` is still BLOCKED (VERIFY.md) | Reparse/junction, in-use rename failure, cloud attributes, process observation |
| `provider` | Real Dropbox test folder, `#[ignore]` by default | One publish/read round trip |

Windows-only tests must **not** be listed under `offline`: the offline tier
also runs on macOS, where they do not exist and would be reported missing.
Making `native` executable in xtask is outside R2-core/R2-adapters (Q-17).
Until then R2-check confirms from the Windows `cargo test` output that every
listed `native` test ran and passed.

Case IDs: `"<acceptance-id>:<packet-part>"`, e.g. `"A-08:core"`,
`"A-23:adapters"`, `"A-23:adapters-native"`. Each `(id, tier)` pair is unique. In the tables,
text after ` / ` in the ID column is traceability only, not part of the ID.

## 2. Fixtures

- **Core**: `core::testkit` builders (feature `testkit`, also `cfg(test)`):
  `toc_bytes(version, deps, ids)`, `folder(name, files, toc)`,
  `inventory([...])`, `member_state(computer, seq, inventory)`,
  `variant_obs_forever()`, and `FakeEffects` implementing both port traits
  over an in-memory map of `Slot → (tree hash, fingerprint, recently_modified)`,
an in-memory provider object map (`rel → bytes`, with injectable corruption /
cloud-only), with:
  `activity_script: VecDeque<Activity>` (one value per `activity()` call,
  last value repeats), `fail_on: Option<(Step, n)>` (inject an error on the
  n-th call of a step), `crash_after: Option<usize>` (panic-free "stop" after
  k effects, returning `EffectError::Crashed`; state is kept so
  `recover_interrupted` can be called on the same fake), and an ordered
  `log: Vec<Effect>` for ordering assertions, and
  `mutate_at: Option<(Step, n, Mutation)>` (change a live/recovery/source slot's
  tree or fingerprint just before the n-th call of a step, simulating an
  external writer).
- **Real metadata**: `core/src/testkit/s_addon.rs` holds a small table of
  folder names, toc versions, ids and deps copied from
  [S-addon folders.csv](../runs/run-001/evidence/S-addon/folders.csv)
  (D-14 derived metadata; no addon code).
- **Adapters**: `adapters::testkit::{TempDir, SyntheticInstall,
  synthetic_dataset, FakeClient}` ([adapters-api.md](adapters-api.md) §8).
  All temp data under the OS temp dir; never real WoW or Dropbox paths.

## 3. R2-core (`wow-sync-core`, tier `offline`)

| Case ID | Tests |
|---|---|
| `R-02:core` / A-02, A-46 | `bpsv::tests::parses_header_and_rows`, `bpsv::tests::wrong_field_count_is_unparseable`, `variant::tests::forever_metadata_is_recognized`, `variant::tests::flavor_build_info_disagreement_refused`, `variant::tests::missing_flavor_info_refused`, `variant::tests::inactive_product_refused`, `variant::tests::ambiguous_build_rows_refused`, `variant::tests::known_product_in_wrong_folder_refused`, `variant::tests::invalid_product_code_refused`, `variant::tests::unfamiliar_product_requires_confirmation`, `variant::tests::wowf_label_is_forever_not_classic_era` |
| `A-46:core` / M-02 | `variant::tests::unknown_product_with_agreeing_metadata_blocked_without_layout_validation`, `variant::tests::consent_alone_does_not_confirm`, `variant::tests::stale_digest_rejected`, `variant::tests::suffixed_toc_folders_blocked_for_unfamiliar`, `variant::tests::duplicate_product_folder_unsupported`, `variant::tests::already_enrolled_variant_unsupported`, `variant::tests::install_without_synthetic_marker_refused` |
| `A-08:core` / M-03 | `version::tests::dotted_numeric_orders_numerically`, `version::tests::missing_parts_are_zero_padded`, `version::tests::prerelease_orders_below_release_same_channel_only`, `version::tests::prerelease_vs_release_is_ambiguous`, `version::tests::unknown_prerelease_word_is_ambiguous`, `version::tests::git_describe_is_newer_than_its_exact_tag`, `version::tests::divergent_describe_branches_ambiguous`, `version::tests::integer_revision_date_order_within_scheme`, `version::tests::tagged_requires_same_tag`, `version::tests::cross_scheme_is_ambiguous`, `version::tests::missing_token_hash_other_are_ambiguous`, `version::tests::s_addon_labels_classify_as_expected` |
| `toc:core` | `toc::tests::parses_header_keys_with_bom_and_crlf`, `toc::tests::non_utf8_toc_is_error_not_empty`, `toc::tests::oversized_toc_is_error`, `toc::tests::plain_only_folder_selected`, `toc::tests::suffixed_tocs_unvalidated_for_recognized_client`, `toc::tests::unfamiliar_with_suffixed_tocs_is_unvalidated`, `toc::tests::equal_summary_suffixed_tocs_still_unvalidated`, `toc::tests::only_foreign_suffix_is_unvalidated`, `toc::tests::no_candidate_is_none`, `toc::tests::unreadable_candidate_is_error` |
| `A-23:core` | `paths::tests::traversal_and_absolute_paths_rejected`, `paths::tests::reserved_device_names_rejected`, `paths::tests::trailing_dot_space_and_control_rejected`, `paths::tests::non_ascii_is_unsupported`, `inventory::tests::case_collision_marks_both_unsupported`, `records::tests::tree_with_unsafe_path_rejected` |
| `A-09:core` / M-04 | `package::tests::shared_curse_id_groups_dbm`, `package::tests::dependency_edge_requires_stem_prefix`, `package::tests::name_prefix_groups_languages_folder`, `package::tests::family_edge_groups_questie`, `package::tests::family_edge_needs_five_char_stem`, `package::tests::grouping_is_deterministic_under_input_order`, `package::tests::membership_change_changes_hash`, `package::tests::identical_trees_compare_same`, `package::tests::newer_member_makes_package_greater`, `package::tests::mixed_member_orders_are_ambiguous`, `package::tests::same_labels_different_content_is_ambiguous`, `package::tests::modified_content_same_label_ambiguous`, `package::tests::unusable_member_is_ambiguous`, `package::tests::unversioned_member_follows_versioned_result`, `package::tests::set_relation_reported` |
| `hash:core` | `hash::tests::tree_hash_independent_of_input_order`, `hash::tests::tree_hash_changes_with_path_size_or_content`, `hash::tests::tree_and_package_hash_golden_vectors` |
| `records:core` | `records::tests::round_trip_all_schemas`, `records::tests::unknown_major_refused`, `records::tests::unknown_fields_ignored_within_major`, `records::tests::oversized_record_refused_before_parse`, `records::tests::state_for_other_group_or_product_rejected`, `records::tests::state_name_must_match_body`, `records::tests::tree_record_hash_recomputed_on_read` |
| `A-20:core-admit` / M-05 | `admit::tests::newest_valid_state_admitted`, `admit::tests::newest_malformed_no_fallback`, `admit::tests::newest_unsupported_major_no_fallback`, `admit::tests::newest_cloud_only_no_fallback`, `admit::tests::regression_after_seeing_newer`, `admit::tests::enrollment_change_resets_watermark`, `admit::tests::only_newest_name_is_read` |
| `A-07:core` / R-06, M-03, M-04 | `target::tests::newest_demonstrable_version_chosen`, `target::tests::independent_packages_converge`, `target::tests::ambiguity_blocks_only_that_package`, `target::tests::dominating_candidate_chosen_despite_other_ambiguity`, `target::tests::ineligible_dominating_candidate_no_fallback`, `target::tests::result_independent_of_input_order_and_duplicates`, `target::tests::record_timestamps_cannot_influence_target`, `target::tests::other_variant_state_ignored`, `target::tests::different_client_build_needs_decision`, `target::tests::foreign_suffix_only_candidate_not_installed`, `target::tests::unvalidated_mapping_candidate_not_installed`, `target::tests::partial_suite_candidate_needs_decision`, `target::tests::partial_candidate_not_installed_on_empty_peer`, `target::tests::complete_candidate_installed_on_empty_peer`, `target::tests::corrupt_tree_makes_package_unavailable` |
| `A-04:core` / R-04 | `target::tests::absent_package_is_not_removal`, `target::tests::unavailable_member_is_not_absence`, `plan::tests::extra_local_folders_are_left_alone`, `plan::tests::local_newer_or_same_is_not_downgraded` |
| `A-21:core` / B-01 | `plan::tests::game_running_blocks_apply_plan`, `plan::tests::unknown_activity_blocks_apply_plan`, `plan::tests::unstable_inventory_blocks_publish_and_apply`, `plan::tests::any_journal_blocks_apply_plan`, `exec::tests::activity_checked_before_every_live_rename`, `exec::tests::launch_before_commit_changes_nothing`, `exec::tests::no_live_mutation_after_launch_mid_commit`, `exec::tests::interrupted_package_recovers_after_game_exit`, `exec::tests::verify_with_game_running_waits`, `exec::tests::recovery_waits_while_game_running`, `exec::tests::rollback_gates_every_rename`, `exec::tests::rollback_refuses_to_move_externally_changed_live` |
| `A-22:core` / B-02, B-03 | `exec::tests::destination_changed_during_staging_replans`, `exec::tests::destination_changed_before_rename_interrupts`, `exec::tests::rename_failure_interrupts_and_keeps_slots`, `exec::tests::staged_hash_mismatch_fails_without_live_change`, `exec::tests::missing_blob_skips_package_others_continue`, `exec::tests::live_mismatch_needs_help_and_pins`, `exec::tests::recovery_copy_modified_needs_help`, `exec::tests::crash_before_verify_reverifies_on_restart`, `exec::tests::crash_after_verify_before_finalize`, `exec::tests::restore_failure_keeps_journal_and_slots`, `exec::tests::restore_mismatch_needs_help`, `exec::tests::classify_slots_table`, `exec::tests::crash_at_every_effect_recovers_to_old_or_new`, `exec::tests::crash_during_recovery_at_every_effect_is_reentrant`, `exec::tests::staging_phase_crash_discards_partial_staging`, `exec::tests::each_package_has_own_op`, `exec::tests::settling_one_op_keeps_other_ops_evidence`, `exec::tests::needs_help_op_blocks_new_apply_and_is_kept`, `exec::tests::unknown_journal_state_needs_help`, `exec::tests::recovery_points_pruned_to_two_per_package`, `exec::tests::pinned_recovery_points_exceed_two_visibly` |
| `A-20:core` / B-02, M-05 | `plan::tests::publish_uploads_missing_blobs_and_writes_state_last`, `exec::tests::publish_failure_leaves_no_state_record`, `exec::tests::corrupt_existing_blob_replaced_before_state`, `exec::tests::corrupt_existing_tree_replaced`, `exec::tests::same_size_mtime_corrupt_object_rehashed_and_replaced`, `exec::tests::publish_aborts_when_source_changes`, `plan::tests::publish_prunes_to_three_own_states`, `plan::tests::seq_never_reused` |
| `A-24:core` / R-12 | `status::tests::published_is_not_confirmed`, `status::tests::confirmation_requires_peer_apply_report_and_trees`, `status::tests::interrupted_or_needs_help_report_not_confirmed`, `status::tests::peer_clock_never_makes_member_current`, `status::tests::needs_decision_shows_evidence`, `status::tests::headline_priority` |
| `A-43:core` | `arch::tests::core_sources_use_no_os_io_or_clock` |

Notes on the exec cases (all with `FakeEffects`):

- `activity_checked_before_every_live_rename`: the `log` shows an
  `activity()` call returning `NotRunning` immediately before every
  `rename` whose source or destination is a `Live` slot, in `run_apply`
  **and** in `recover_interrupted`.
- `no_live_mutation_after_launch_mid_commit`: activity flips to `Running`
  after the first folder of a two-folder package was moved; asserts no
  further `rename` (no rollback either), journal phase `committing`, result
  `Interrupted`. `interrupted_package_recovers_after_game_exit` continues on
  the same fake with `NotRunning` and asserts the old trees are live.
- `crash_at_every_effect_recovers_to_old_or_new` loops `k` over every effect
  index of a two-package (two-op) apply with one replace and one install
  folder each, crashes after `k` effects, runs `recover_interrupted` (with
  `NotRunning`) until no journal remains, and asserts each package's live
  trees equal either the complete old or complete new set (never a mix), no
  live folder was lost, no journal is left in a non-terminal phase, and a
  package ends `Applied` only via a recorded verification.
- `crash_during_recovery_at_every_effect_is_reentrant`: for every first crash
  point `k` (as above) and every effect index `m` of the following
  `recover_interrupted`, crash recovery after `m` effects, run it again (also
  once with a `Running` activity in between); same assertions (apply.md §4.1).
- `settling_one_op_keeps_other_ops_evidence` (N-03): three ops present —
  `needs_help`, `committing` and `staging`; recovery settles the latter two
  and asserts the `needs_help` op's journal, staging and recovery slots are
  untouched and that the other ops removed only their own directories.
- `same_size_mtime_corrupt_object_rehashed_and_replaced` (M-05): an existing
  provider object is corrupted keeping size and mtime; publish hashes it,
  replaces it and only then writes the state record.
- `pinned_recovery_points_exceed_two_visibly`: three earlier points of
  which one is pinned plus a new apply → three unpinned-or-pinned points
  remain and `StatusView::retained_extra_recovery == 1`.

## 4. R2-adapters (`wow-sync-adapters`)

Tier `offline` (portable):

| Case ID | Tests |
|---|---|
| `A-23:adapters` / M-01 | `fs_safe::tests::join_rejects_traversal_and_absolute`, `fs_safe::tests::classify_tag_cloud_vs_link_vs_other`, `fs_safe::tests::unavailable_attribute_combinations` (each of OFFLINE / RECALL_ON_OPEN / RECALL_ON_DATA_ACCESS alone and combined, for file and directory entries), `fs_safe::tests::unavailable_directory_not_enumerated` (fake lister), `fs_install::tests::deletion_only_inside_work_area`, `fs_install::tests::stage_file_rejects_hash_mismatch` |
| `scan:adapters` / B-02 | `fs_scan::tests::scan_hashes_files_and_reads_tocs`, `fs_scan::tests::top_level_files_ignored`, `fs_scan::tests::recently_modified_file_is_unstable`, `fs_scan::tests::file_changed_after_hash_marks_unstable`, `fs_scan::tests::file_added_or_removed_during_scan_marks_unstable`, `fs_scan::tests::top_level_listing_change_detected`, `fs_scan::tests::fingerprint_covers_dirs_size_and_mtime`, `fs_scan::tests::read_variant_files_reports_missing_and_bytes`, `fs_scan::tests::list_variant_folders_matches_pattern` |
| `A-22:adapters` | `fs_install::tests::rename_refuses_existing_destination`, `fs_install::tests::second_lock_fails_while_held`, `fs_install::tests::journal_write_is_atomic_and_listed`, `fs_install::tests::hash_and_snapshot_any_slot` |
| `A-20:adapters` / M-05 | `provider::tests::write_atomic_replaces_without_partial_file`, `provider::tests::put_object_verifies_temp_before_rename`, `provider::tests::put_object_replaces_corrupt_object`, `provider::tests::check_object_reports_wrong_hash`, `provider::tests::temp_and_conflicted_entries_classified`, `provider::tests::read_refuses_oversized_file`, `provider::tests::remove_stale_temps_only_own` |
| `local:adapters` / M-06 | `localstate::tests::audit_rotates_at_limit_keeping_two`, `localstate::tests::second_instance_guard_fails`, `sysinfo::tests::random_ids_are_32_hex_and_distinct`, `winproc::tests::image_prefix_requires_separator`, `winproc::tests::inaccessible_possible_client_is_unknown`, `winproc::tests::inaccessible_unrelated_process_ignored`, `winproc::tests::accessible_image_under_variant_is_running` |
| `fixture:adapters` | `testkit::tests::synthetic_install_has_valid_variant_metadata`, `testkit::tests::synthetic_install_has_marker`, `testkit::tests::synthetic_dataset_respects_cap_and_is_deterministic` |

The B-02 scan tests use `scan_folder_with_hook` ([adapters-api.md](adapters-api.md)
§3) to mutate the temp folder between phases, so they are deterministic and
portable. The `winproc::classify` tests use synthetic `ProcEntry` lists.

Tier `native` (`#[cfg(windows)]`):

| Case ID | Tests |
|---|---|
| `A-23:adapters-native` / M-01 | `fs_safe::tests::junction_inside_root_is_refused`, `fs_scan::tests::nested_junction_marks_folder_unsafe` (junction via `cmd /c mklink /J`), `provider::tests::junction_under_provider_root_is_refused` |
| `A-22:adapters-native` | `fs_install::tests::rename_fails_while_live_file_is_open` (held handle without delete sharing), `fs_scan::tests::locked_file_marks_folder_unreadable` |
| `A-21:adapters-native` | `winproc::tests::fake_client_under_variant_is_running`, `winproc::tests::process_outside_variant_is_not_running` (copy of `ping.exe` at `<root>\Launcher.exe`) |

Tier `provider` (`#[ignore]`):

| Case ID | Tests |
|---|---|
| `A-20:adapters-provider` / M-01 | `provider::tests::dropbox_test_root_round_trip`, `provider::tests::dropbox_hydrated_tagged_entries_accepted`, `provider::tests::dropbox_online_only_entry_refused` |

They run only when `WOWSYNC_PROVIDER_TEST_ROOT` is set to a directory under
`D:\Dropbox\Gaming Sync\WoW Sync Test` (the tests refuse any other path).
The round trip writes a run-unique subdirectory with ≤ 50 MB synthetic data
(Q-8), reads it back, reports cloud-state classification, and removes only
its own subdirectory. `dropbox_hydrated_tagged_entries_accepted` waits (≤ 30 s)
for the written files/dir to carry a cloud reparse tag (observed in S-dropbox)
and asserts they are listed `Local` and readable. `dropbox_online_only_entry_refused`
needs an owner-prepared online-only file named in
`WOWSYNC_PROVIDER_ONLINE_ONLY`; if unset or the file is not online-only, it
panics with `BLOCKED: …` (the test fails rather than passing without a claim;
R2-check records it as BLOCKED, Q-23). Likewise the hydrated-tag test panics
with `BLOCKED: …` if no cloud reparse tag appears within 30 s. These prove local provider-folder
semantics only, **not** cross-computer transfer (A-34 stays NOT RUN).

## 5. R2-slice (`wow-sync-runtime`, mostly `offline`)

Runtime end-to-end tests in `wow-sync-runtime` `mod e2e` use two
`SyntheticInstall`s, two `LocalState` dirs and one temp provider folder in the
same process, with an injectable `ActivitySource` (default: `winproc`) and
the **real** adapters (no fake effects):

| Case ID | Tests |
|---|---|
| `A-06:slice` | `e2e::tests::two_installs_publish_and_apply_through_temp_provider` (bytes of applied folders equal the source; the target's effective package hash equals the source's exactly; status explains source and versions) |
| `A-07:slice` | `e2e::tests::independent_updates_converge` |
| `A-08:slice` | `e2e::tests::ambiguous_addon_stops_only_that_addon` |
| `A-09:slice` / M-04 | `e2e::tests::partial_suite_on_target_is_not_mixed`, `e2e::tests::empty_peer_does_not_receive_partial_suite` (three installs: full old, partial new, empty) |
| `A-21:slice` | `e2e::tests::game_running_on_target_blocks_apply_then_resumes` (scripted activity source) |
| `A-46:slice` | `e2e::tests::unfamiliar_variant_blocked_until_confirmed`, `e2e::tests::join_refused_for_other_product`, `e2e::tests::install_without_marker_refused` |
| `A-22:slice` / B-02, M-07 | `e2e::tests::crash_mid_apply_recovers_on_restart`, `e2e::tests::two_package_crash_at_every_effect_recovers_with_real_adapters` (effects wrapper over the real adapters stops after k effects; a new service instance recovers; same assertions as the core loop), `e2e::tests::destination_updated_during_staging_is_not_replaced`, `e2e::tests::crash_during_recovery_with_real_adapters_is_reentrant` |
| `A-20:slice` / B-02 | `e2e::tests::publish_aborts_when_source_changes` |
| `A-24:slice` | `e2e::tests::peer_confirmation_only_after_apply` |
| `A-21:slice-native` | `e2e::tests::fake_client_blocks_real_apply` (`native`, uses `FakeClient`) |
| `R2-fix3:slice` (follow-up #9, mi-1) | `e2e::tests::apply_report_survives_crash_before_publish` (crash at every effect k of B's first apply+publish cycle, restart, three syncs: B's published states carry exactly one `applied` result for the package with the source's package hash, the last sync publishes nothing, and A shows "applied and reported by"; asserts a crash point after the apply and before publication was exercised), `e2e::tests::recognized_enroll_without_scope_consent_is_refused` (no provider writes) |

Core unit tests for the same run-002 fix are listed as `R2-fix3:core` in
`tests/required-cases.toml` (`report::tests::*`, `variant::tests::consent_predicate_requires_scope_and_guided_identity`,
`plan::tests::own_publish_slot_never_reuses_a_number`, `status::tests::applied_here_is_rebuilt_from_own_report_after_restart`,
`status::tests::needs_help_journal_is_not_reported_as_interrupted`).

`R2-fix4:slice` (R3-M1) is `e2e::tests::pending_clear_failure_does_not_duplicate_report`.
It runs on Windows and macOS, the platforms in the CI matrix; it is not compiled
on other platforms. After B has published its report, the test restores that
report to B's pending store and restarts B. It then makes the store readable
but impossible to replace: on Windows, a handle that shares read access only
(`FILE_SHARE_READ`); on macOS, the owner-settable immutable flag (`chflags
uchg`). It adds
Beta and syncs twice. The test asserts exactly one Alpha `applied` result.
After the block is released, publication resumes: Beta is published, and
there is still exactly one Alpha result. The core predicate test is listed as
`R2-fix4:core`.

### 5a. No client-build or Interface gating (run-003, R3-d21; Q-21 by D-21)

D-21 replaces the D-18 interface rule and interim rule A: a game update or an
`## Interface:` declaration never withholds an addon; only the variant and the
matching account/character boundaries (and the mapping predicate) do. Core
tests (`target::tests::`): `different_build_syncs_whatever_the_interface_says`
(same, different, malformed, empty and missing Interface on a different build
all choose the candidate), `unknown_build_and_new_addon_sync`,
`multi_folder_package_syncs_across_builds_and_interfaces`,
`different_toc_mapping_needs_decision` (the mapping predicate is unchanged) and
`other_variant_state_ignored` (a different variant or group never syncs).
Runtime e2e with two synthetic installs on different builds:
`different_build_same_interface_applies`, `different_build_different_interface_applies`,
`different_build_malformed_interface_applies`, `different_build_missing_interface_applies`
and `different_build_new_addon_applies` all apply and leave no decision. Cases
`Q-21:core` and `Q-21:slice`. The A-38 negative control re-introduces a build-based hold.
## 5b. GUI verification (M-08; A-25, A-26)

Headless tests are **not** GUI evidence. GUI checks are manual/observed
checks on the dev PC with two synthetic installs, never real WoW data:

- **R2-slice** (developer checks, recorded in its evidence file) builds the
  shell and runs every item below once before handing over.
- **R2-check** (independent verification, gpt-6.1-sol) repeats every item
  from a clean clone and retains evidence: one screenshot or short note per
  item under `docs/runs/run-002/evidence/R2-check/gui/` (synthetic data
  only), with pass/fail/blocked per item.
- **R2-review** checks that the evidence exists and matches the claims.

| # | Item | Observable pass |
|---|---|---|
| G1 | Consent/enrollment, recognized Forever | Shows "World of Warcraft: Forever (wowf)", folder, the scope (AddOns code only, settings excluded); nothing is written before consent |
| G2 | Unfamiliar client | Identity + scope confirmations both required; blocked folders listed; consent alone cannot proceed |
| G3 | Safe apply | Peer's newer addon appears, applies, status explains source/version; recovery copy count visible |
| G4 | Running-game wait/resume | With `FakeClient` running: "Waiting — WoW is running", no change; after stop, apply resumes |
| G5 | Ambiguity | `NeedsDecision` addon shows evidence (labels, sources) and is not changed |
| G6 | Provider/error state | Provider folder missing/unwritable shows an error headline and plain-English reason |
| G7 | Truthful confirmation | "Published" vs "confirmed by <peer>" distinguished; no confirmation before the peer's apply report |
| G8 | Interrupted/needs-help | A forced interrupted package is shown as Waiting/NeedsHelp with retained copies |
| G9 | Keyboard/focus | All G1–G8 actions reachable by keyboard; visible focus; close/reopen window works |

## 6. Negative controls for R2-check (A-38)

Each mutation must make at least one listed test fail; R2-check applies them
on a scratch branch, records the failing names, and reverts:

| Mutation | Expected failing test (at least) |
|---|---|
| `version::compare` returns `Greater` across schemes when numbers are larger | `version::tests::cross_scheme_is_ambiguous` |
| `version::compare` orders a release above a prerelease of another channel | `version::tests::prerelease_vs_release_is_ambiguous` |
| `exec` skips the activity gate before one live rename | `exec::tests::activity_checked_before_every_live_rename` |
| `exec` rolls back immediately when the game starts mid-commit | `exec::tests::no_live_mutation_after_launch_mid_commit` |
| `exec` finalizes a `committed` package without verification | `exec::tests::crash_before_verify_reverifies_on_restart` |
| `exec` skips destination revalidation after staging | `exec::tests::destination_changed_during_staging_replans` |
| pruning ignores `pinned` | `exec::tests::pinned_recovery_points_exceed_two_visibly` |
| `target` treats a package missing on a member as a removal | `target::tests::absent_package_is_not_removal` |
| `target` installs a partial candidate | `target::tests::partial_suite_candidate_needs_decision` |
| target prefers the record with the larger `written_unix_ms` on ambiguity | `target::tests::record_timestamps_cannot_influence_target` |
| `admit` falls back to the highest valid older state | `admit::tests::newest_malformed_no_fallback` |
| publish trusts an existing object without hashing | `exec::tests::corrupt_existing_blob_replaced_before_state` |
| `select_toc` selects a suffixed toc for `wowf` | `toc::tests::suffixed_tocs_unvalidated_for_recognized_client` |
| completeness checked against local folders only | `target::tests::partial_candidate_not_installed_on_empty_peer` |
| recovery uses a remembered per-folder state instead of observing slots | `exec::tests::crash_during_recovery_at_every_effect_is_reentrant` |
| cleanup discards staging of all ops / removes another op's journal | `exec::tests::settling_one_op_keeps_other_ops_evidence` |
| `check_object` trusts size/mtime | `exec::tests::same_size_mtime_corrupt_object_rehashed_and_replaced` |
| `is_unavailable` ignores RECALL_ON_OPEN on files | `fs_safe::tests::unavailable_attribute_combinations` |
| synthetic marker required by default (D-28 removes that) | `variant::tests::unmarked_install_is_accepted_by_default`; test mode keeps it: `variant::tests::required_policy_refuses_missing_wrong_or_bom_prefixed_markers` |
| `confirm_variant` accepts consent without a Supported assessment | `variant::tests::consent_alone_does_not_confirm` |
| `classify_tag` accepts mount points as cloud files | `fs_safe::tests::classify_tag_cloud_vs_link_vs_other` |
| `winproc::classify` ignores inaccessible `Wow.exe` | `winproc::tests::inaccessible_possible_client_is_unknown` |
| `status` marks confirmed when the peer's member record exists | `status::tests::published_is_not_confirmed` |
| `exec::run_publish` writes the state record before blobs/trees | `plan::tests::publish_uploads_missing_blobs_and_writes_state_last` or `exec::tests::publish_failure_leaves_no_state_record` |