# R11-runtime evidence

Names are redacted (account A, character A1). No file contents, SIDs or user paths recorded.
Lab: a read-only copy of a Classic-variant install (113 add-ons, 63 settings units across one
account) in two lab computers A and B, each with its own `--state`; nothing outside the lab was
touched.

## Trial on the old build (defects found)

| ID | Severity | Finding |
| --- | --- | --- |
| D1 | major | After the master enabled sharing nothing was published (activation capture never publishes). |
| D2 | minor | Settings record names carried raw texture codes. |
| G2 | minor | Settings records had empty versions, zero size, no identity. |
| D3 | major (safety) | A missing character folder on a peer was created by the settings apply. |
| N1 | minor | Cycle notes dropped; blocked units showed no reason. |

## Lab re-run on the final build

| Scenario | Result |
| --- | --- |
| A becomes master, 2 syncs | 63 units `NoSharedCopy`, `to_share` 63, `group_has_shared_settings` false: nothing published. |
| `ShareSettingsNow` with the listed units | 63 pushes; A then `InSync` 63, `to_share` 0. |
| B applies | 47 `InSync`; 16 `WaitingForIdentity` (characters absent on B). Only the missing identities wait; no folder was created. |
| Edit on A, B with the game "running" | All B units `WaitingForWow`, file unchanged; after "closing", the edit applied and hashes match. |
| Zero-byte file beside a `.bak` on A | Unit `Suspect`; never published; B's copy untouched. |
| File over the 32 MiB cap | Unit `TooLarge`, not published. |

### Memory (private commit / working set of the process, sampled every 20 ms)

| Pass | Peak private commit | Peak working set |
| --- | --- | --- |
| Idle status | 26-39 MiB | 31-45 MiB |
| First capture of 63 units / peer apply | 59-63 MiB | 57-63 MiB |
| Pass with a 28 MiB add-on file changed | **104 MiB** | **95.6 MiB** |
| Pass with a 34 MiB file (refused, TooLarge) | 33 MiB | 39 MiB |

The gate is 100 MiB aggregate private resident memory. Typical passes stay near 62 MiB; a pass that
reads, hashes and stores a file near the 32 MiB cap holds several whole copies and reaches 96 MiB
working set (104 MiB commit). A 32 MiB file would very likely exceed the gate. Superseded by OD-30 below.

### Memory with streaming (OD-30): one add-on file at the cap, before and after

Lab: two computers, one account, one add-on file of the given size; every step is a separate
process, sampled every 10 ms. Values are the peak of any step (capture, share, download, apply,
edit-and-republish). "Before" is main 9ac5912, "after" is this commit.

| File | Build | Peak private commit | Peak working set |
| --- | --- | --- | --- |
| 32 MiB (the cap) | before | 130.9 MiB | 128 MiB |
| 32 MiB (the cap) | **after** | **1.7 MiB** | **8.2 MiB** |
| 31.9 MiB (cap edge) | before | 66.8 MiB | 72.8 MiB |
| 31.9 MiB (cap edge) | **after** | **1.5 MiB** | **8.2 MiB** |

Files were equal on both computers after apply and after a one-byte edit republish (all four runs).
The per-process figure excludes the add-on listing a real install adds (62 MiB typical pass, see
above), which is unchanged; the file-size-dependent part is now constant.
## Fails-without-fix table

Each row: the guard was reverted by a one-line source change, the named test run, the guard restored.

| Guard | Test | Without the guard |
| --- | --- | --- |
| Adapter refuses to create owner folders | `fs_install::settings_files_stage_move_and_hash_inside_the_wtf_scope` | fails |
| Cycle never plans an apply for a missing identity (also guarded by the adapter) | `a_character_missing_here_waits_and_is_never_created…` | fails |
| `OwnedBySeveral` + `shared_with` | `a_variable_declared_by_two_addons_is_owned_by_several…` | fails |
| `TooLarge` | `a_character_missing_here_waits…` | fails |
| `WaitingForWow` | `with_the_game_running_settings…` | fails |
| G2 versions/size/identity | `a_masters_settings_change_is_recorded_on_both_computers…` | fails |
| G6 rollback source | `every_reason_has_plain_words` | fails |
| G7 kept versions | `restoring_a_copy_a_keep_displaced…` | fails |
| Progress `source` | `settings_progress_counts_units…` | fails |
| Texture codes stripped | `record_text_never…` | fails |
| RV11-06 `to_share` lists nothing | `becoming_master_publishes_nothing…` | fails |
| RV11-06 automatic publish at activation restored | `becoming_master_publishes_nothing…` | fails |
| RV11-04 NoSharedCopy / InSync / LocalDifferent not reported | `becoming_master…`, `a_local_edit_that_is_not_shared…` | fails |
| RV11-03 suspect ignored; `can_share` for Suspect | `a_zero_byte_file_next_to_a_backup…` | fails |
| RV11-03 settle wait skipped | `a_file_still_changing_across_the_settle_wait…` | fails |
| RV11-01 replaced copy not kept / evicted / dropped by prune | `a_copy_replaced_by_the_masters_is_listed_restorable…` | fails (3 variants) |
| RV11-02 one shared retention budget | `settings_and_code_recovery_points_never_evict_each_other` | fails |
| RV11-07 store never pruned | `old_settings_copies_are_pruned_to_the_retention` | fails |
| RV11-08 error text carries the path | `a_settings_failure_names_no_account_realm_character_or_path` | fails |
| RV11-09 no-replace move may replace (Windows `MoveFileExW` flag) | `a_move_never_replaces_a_file_created_after_the_check` | fails |
| D2 pushes recorded in Activity | `explicit_pushes_and_peer_applies_are_recorded_with_versions_size_and_identity` | fails |
| D1 "Share these now" ticked at consent | `share_these_now_ticked_at_consent_pushes_the_first_captures_once` | fails |
| D4 master says nothing "from itself" | `becoming_master_publishes_nothing…` | fails |
| One missing character | `one_missing_character_waits_alone…` | regression guard only: the all-wait defect did not reproduce in the e2e or in the lab |
| OD-30 whole-file buffers (alloc peak < 3 MiB, largest < 1 MiB over a 24 MiB file) | stream_guard::a_large_settings_file_is_hashed_stored_uploaded_and_staged_in_fixed_buffers | fails (buffer set to file size) |
| OD-30 no whole-file read calls in the settings path | settings_cycle::the_settings_path_never_reads_a_whole_file_into_memory | fails (source scan) |
| Provider GC keeps three retained per unit | 	he_shared_folder_drops_old_unreferenced_settings_copies_and_keeps_the_retained_three | fails (retention 0) |
| Provider GC age gate (30 days) | _recent_settings_copy_is_never_swept_even_when_nothing_retains_it | fails |
| Provider GC sweep runs | 	he_shared_folder_drops_old_unreferenced_settings_copies… | fails (call removed) |
| A publish from here is not PeerPush | xplicit_pushes_and_peer_applies_are_recorded_with_versions_size_and_identity | fails |

## Decisions made

- Decision: the "first share" is removed (OD-16 withdrawn). Why: owner intent, design §5.2/Q-75.
  Reversible: yes.
- Decision: the consent-step "Share these now" is `Command::ShareSettingsWhenReady`, recorded
  locally after consent; the first pass after the units are captured pushes each `NoSharedCopy`
  unit once, then forgets it. Why: units only exist after consent, so no list can exist before.
  Reversible: yes.
- Decision: pushes written by command (`ShareSettings`, `ShareSettingsNow`, choice, restore)
  write the Activity row with versions, size and identity. Why: they bypass the cycle.
  Reversible: yes.
- Decision: `in_sync_from` is `None` when the copy in use came from this computer. Reversible: yes.
- Decision: Windows no-replace move uses `MoveFileExW` without replace (works on NTFS, FAT32,
  exFAT); other platforms keep link-then-unlink with a re-check fallback. Reversible: yes.
- Decision: retention is per class: settings captures 3, kept copies 3, code recovery separate (2
  previous). `local_copies` are synthetic entries from kept copies only. Reversible: yes.
- Decision: Suspect units do nothing in the cycle; a master's later edits still auto-publish.
  Reversible: restrict to LocalDifferent.
- Decision: cycle notes use plain kinds, never error text. Reversible: yes.
- Decision: keep the 32 MiB cap; capture hashing, blob store, provider upload, download, staging and
  local fallback stream through 256 KiB (store) and 64 KiB (hash) buffers. get_blob is test-only.
  Why: OD-30. Reversible: yes.
- Decision: regression guards are a counting allocator over a 24 MiB file in the adapters tests
  (runtime forbids unsafe) and a source scan of the runtime settings path. Reversible: yes.
- Decision: provider GC is implemented, conservatively: only settings blobs/manifests that no
  retained revision (heads + 3 newest per unit + removal captures) references AND older than 30 days;
  at most daily; gives up if a retained manifest cannot be read. Why: peers may lag or upload before
  their record lands. Residual risk: a revision older than 30 days this computer never saw could
  lose its blob. Reversible: remove the gc_provider call.
- Decision: a publish from this computer is recorded with the existing ChangeReason::SettingsMaster
  and source this computer; only an applied Push is PeerPush with the origin as source. Why: no new
  variant, so the GUI needs no change. Caveat: the label reads "settings master" even for a non-master
  explicit push; a SharedFromHere variant would need a history_ui arm. Reversible: yes.
## Open issues

- FAT32/exFAT volumes are untested; the Windows move uses MoveFileExW which supports them.
- Only the first file of a multi-file unit appears in a record's versions.
- No pre-consent share preview (nothing of WTF is read before consent, by design).
- `ShareSettingsWhenReady` takes effect on the pass after the first capture (one cycle later).
- Unix no-replace is link-then-unlink; not exercised on this machine.
