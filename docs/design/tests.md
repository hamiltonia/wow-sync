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
  over an in-memory map of `Area × folder → tree hash`, with:
  `activity_script: VecDeque<Activity>` (one value per `activity()` call,
  last value repeats), `fail_on: Option<(Step, n)>` (inject an error on the
  n-th call of a step), `crash_after: Option<usize>` (panic-free "stop" after
  k effects, returning `EffectError::Crashed`; state is kept so
  `recover_interrupted` can be called on the same fake), and an ordered
  `log: Vec<Effect>` for ordering assertions.
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
| `A-08:core` | `version::tests::dotted_numeric_orders_numerically`, `version::tests::missing_parts_are_zero_padded`, `version::tests::prerelease_orders_below_release`, `version::tests::unknown_prerelease_word_is_ambiguous`, `version::tests::git_describe_is_newer_than_its_tag`, `version::tests::integer_revision_date_order_within_scheme`, `version::tests::tagged_requires_same_tag`, `version::tests::cross_scheme_is_ambiguous`, `version::tests::missing_token_hash_other_are_ambiguous`, `version::tests::s_addon_labels_classify_as_expected` |
| `toc:core` | `toc::tests::parses_header_keys_with_bom_and_crlf`, `toc::tests::non_utf8_toc_is_error_not_empty`, `toc::tests::oversized_toc_is_error`, `toc::tests::forever_suffix_preferred_over_plain`, `toc::tests::forever_camelot_identical_selects_forever`, `toc::tests::forever_camelot_disagree_is_ambiguous`, `toc::tests::only_foreign_suffix_is_none`, `toc::tests::unfamiliar_variant_uses_plain_toc_only`, `toc::tests::unreadable_candidate_is_error` |
| `A-23:core` | `paths::tests::traversal_and_absolute_paths_rejected`, `paths::tests::reserved_device_names_rejected`, `paths::tests::trailing_dot_space_and_control_rejected`, `paths::tests::non_ascii_is_unsupported`, `inventory::tests::case_collision_marks_both_unsupported`, `records::tests::tree_with_unsafe_path_rejected` |
| `A-09:core` | `package::tests::shared_curse_id_groups_dbm`, `package::tests::dependency_edge_requires_stem_prefix`, `package::tests::name_prefix_groups_languages_folder`, `package::tests::grouping_is_deterministic_under_input_order`, `package::tests::identical_trees_compare_same`, `package::tests::newer_member_makes_package_greater`, `package::tests::mixed_member_orders_are_ambiguous`, `package::tests::same_labels_different_content_is_ambiguous`, `package::tests::unusable_member_is_ambiguous`, `package::tests::unversioned_member_follows_versioned_result` |
| `hash:core` | `hash::tests::tree_hash_independent_of_input_order`, `hash::tests::tree_hash_changes_with_path_size_or_content`, `hash::tests::tree_and_package_hash_golden_vectors` |
| `records:core` | `records::tests::round_trip_all_schemas`, `records::tests::unknown_major_refused`, `records::tests::unknown_fields_ignored_within_major`, `records::tests::oversized_record_refused_before_parse`, `records::tests::state_for_other_group_or_product_rejected`, `records::tests::state_name_must_match_body` |
| `A-07:core` / R-06 | `target::tests::newest_demonstrable_version_chosen`, `target::tests::independent_packages_converge`, `target::tests::ambiguity_blocks_only_that_package`, `target::tests::dominating_candidate_chosen_despite_other_ambiguity`, `target::tests::result_independent_of_input_order_and_duplicates`, `target::tests::record_timestamps_cannot_influence_target`, `target::tests::other_variant_state_ignored` |
| `A-04:core` / R-04 | `target::tests::absent_package_is_not_removal`, `target::tests::unavailable_member_is_not_absence`, `plan::tests::extra_local_folders_are_left_alone`, `plan::tests::local_newer_or_same_is_not_downgraded` |
| `A-21:core` | `plan::tests::game_running_blocks_apply_plan`, `plan::tests::unknown_activity_blocks_apply_plan`, `plan::tests::unstable_inventory_blocks_publish_and_apply`, `exec::tests::apply_rechecks_activity_before_each_commit_rename`, `exec::tests::launch_before_commit_changes_nothing`, `exec::tests::launch_mid_package_rolls_back_package_and_stops`, `exec::tests::launch_after_commit_needs_help_and_keeps_recovery`, `exec::tests::recovery_waits_while_game_running` |
| `A-22:core` | `exec::tests::rename_failure_rolls_back_package`, `exec::tests::staged_hash_mismatch_fails_without_live_change`, `exec::tests::missing_blob_skips_package_others_continue`, `exec::tests::post_apply_hash_mismatch_needs_help`, `exec::tests::crash_at_every_effect_recovers_to_old_or_new`, `exec::tests::unknown_journal_state_needs_help`, `exec::tests::recovery_points_pruned_to_two_per_package` |
| `A-20:core` | `plan::tests::publish_uploads_missing_blobs_and_writes_state_last`, `exec::tests::publish_failure_leaves_no_state_record`, `plan::tests::publish_prunes_to_three_own_states`, `plan::tests::seq_never_reused` |
| `A-24:core` / R-12 | `status::tests::published_is_not_confirmed`, `status::tests::confirmation_requires_peer_apply_report_and_trees`, `status::tests::peer_clock_never_makes_member_current`, `status::tests::needs_decision_shows_evidence`, `status::tests::headline_priority` |
| `A-43:core` | `arch::tests::core_sources_use_no_os_io_or_clock` |

`exec::tests::crash_at_every_effect_recovers_to_old_or_new` loops `k` over
every effect index of a two-package apply, crashes after `k` effects, runs
`recover_interrupted`, and asserts each package's live trees equal either
the complete old or complete new set, never a mix, and no live folder was
lost.

## 4. R2-adapters (`wow-sync-adapters`)

Tier `offline` (portable):

| Case ID | Tests |
|---|---|
| `A-23:adapters` | `fs_safe::tests::join_rejects_traversal_and_absolute`, `fs_install::tests::deletion_only_inside_work_area`, `fs_install::tests::stage_file_rejects_hash_mismatch` |
| `scan:adapters` | `fs_scan::tests::scan_hashes_files_and_reads_tocs`, `fs_scan::tests::top_level_files_ignored`, `fs_scan::tests::recently_modified_file_is_unstable`, `fs_scan::tests::read_variant_files_reports_missing_and_bytes`, `fs_scan::tests::list_variant_folders_matches_pattern` |
| `A-22:adapters` | `fs_install::tests::rename_refuses_existing_destination`, `fs_install::tests::second_lock_fails_while_held`, `fs_install::tests::journal_write_is_atomic_and_listed` |
| `A-20:adapters` | `provider::tests::write_atomic_replaces_without_partial_file`, `provider::tests::put_blob_is_write_if_absent_and_verified`, `provider::tests::existing_blob_with_wrong_hash_is_reported`, `provider::tests::temp_and_conflicted_entries_classified`, `provider::tests::read_refuses_oversized_file`, `provider::tests::remove_stale_temps_only_own` |
| `local:adapters` | `localstate::tests::audit_rotates_at_limit_keeping_two`, `localstate::tests::second_instance_guard_fails`, `sysinfo::tests::random_ids_are_32_hex_and_distinct`, `winproc::tests::image_prefix_requires_separator` |
| `fixture:adapters` | `testkit::tests::synthetic_install_has_valid_variant_metadata`, `testkit::tests::synthetic_dataset_respects_cap_and_is_deterministic` |

`winproc::image_is_under(variant_dir, image) -> bool` is a pure helper so the
`_forever_` vs `_forever_x` prefix rule is tested on every platform.

Tier `native` (`#[cfg(windows)]`):

| Case ID | Tests |
|---|---|
| `A-23:adapters-native` | `fs_safe::tests::junction_inside_root_is_refused`, `fs_scan::tests::nested_junction_marks_folder_unsafe` (junction via `cmd /c mklink /J`) |
| `A-22:adapters-native` | `fs_install::tests::rename_fails_while_live_file_is_open` (held handle without delete sharing), `fs_scan::tests::locked_file_marks_folder_unreadable` |
| `A-21:adapters-native` | `winproc::tests::fake_client_under_variant_is_running`, `winproc::tests::process_outside_variant_is_not_running` (copy of `ping.exe` at `<root>\Launcher.exe`) |

Tier `provider` (`#[ignore]`):

| Case ID | Tests |
|---|---|
| `A-20:adapters-provider` | `provider::tests::dropbox_test_root_round_trip` |

It runs only when `WOWSYNC_PROVIDER_TEST_ROOT` is set to a directory under
`D:\Dropbox\Gaming Sync\WoW Sync Test` (the test refuses any other path),
writes a run-unique subdirectory with ≤ 50 MB synthetic data (Q-8), reads it
back, reports cloud-state classification, and removes only its own
subdirectory. It proves local provider-folder semantics only, **not**
cross-computer transfer (A-34 stays NOT RUN).

## 5. R2-slice (`wow-sync-runtime`, mostly `offline`)

Runtime end-to-end tests in `wow-sync-runtime` `mod e2e` use two
`SyntheticInstall`s, two `LocalState` dirs and one temp provider folder in the
same process, with an injectable `ActivitySource` (default: `winproc`):

| Case ID | Tests |
|---|---|
| `A-06:slice` | `e2e::tests::two_installs_publish_and_apply_through_temp_provider` (bytes of applied folders equal the source; status explains source and versions) |
| `A-07:slice` | `e2e::tests::independent_updates_converge` |
| `A-08:slice` | `e2e::tests::ambiguous_addon_stops_only_that_addon` |
| `A-21:slice` | `e2e::tests::game_running_on_target_blocks_apply_then_resumes` (scripted activity source) |
| `A-46:slice` | `e2e::tests::unfamiliar_variant_blocked_until_confirmed`, `e2e::tests::join_refused_for_other_product` |
| `A-22:slice` | `e2e::tests::crash_mid_apply_recovers_on_restart` (effects wrapper stops after k renames; new service instance recovers) |
| `A-24:slice` | `e2e::tests::peer_confirmation_only_after_apply` |
| `A-21:slice-native` | `e2e::tests::fake_client_blocks_real_apply` (`native`, uses `FakeClient`) |

GUI checks are R2-gui's (PLAN), not listed here.

## 6. Negative controls for R2-check (A-38)

Each mutation must make at least one listed test fail; R2-check applies them
on a scratch branch, records the failing names, and reverts:

| Mutation | Expected failing test (at least) |
|---|---|
| `version::compare` returns `Greater` across schemes when numbers are larger | `version::tests::cross_scheme_is_ambiguous` |
| `exec` skips the activity re-check before commit renames | `exec::tests::apply_rechecks_activity_before_each_commit_rename` |
| `target` treats a package missing on a member as a removal | `target::tests::absent_package_is_not_removal` |
| target prefers the record with the larger `written_unix_ms` on ambiguity | `target::tests::record_timestamps_cannot_influence_target` |
| `select_toc` falls back to plain toc when Forever/Camelot disagree | `toc::tests::forever_camelot_disagree_is_ambiguous` |
| `status` marks confirmed when the peer's member record exists | `status::tests::published_is_not_confirmed` |
| `exec::run_publish` writes the state record before blobs/trees | `plan::tests::publish_uploads_missing_blobs_and_writes_state_last` or `exec::tests::publish_failure_leaves_no_state_record` |
