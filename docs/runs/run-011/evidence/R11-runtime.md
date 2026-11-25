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

## Final round (RV11-12/13/14, SharedFromHere, unit-level versions, full-app memory)

### Full-app memory (real release app per lab computer, 113 add-ons, one 31.9 MiB file)
| Phase | Private commit | Working set | Private working set |
|---|---|---|---|
| Idle app (synced) | 83-106 MiB | 90-112 MiB | 57 MiB |
| Settings pass (31.9 MiB edit) | 117 MiB | 120 MiB | 86 MiB |
Private working set stays under 100 MiB (peak 86 MiB per process). Private commit peaks at 117-131 MiB; most of it is the GUI baseline (about 84-106 MiB at idle), not the settings path (streaming; see OD-30). One real computer runs one process; the lab ran two (one per simulated computer), each measured alone.

### Fails-without-fix (this round)
| Guard | Test | Without the guard |
|---|---|---|
| SharedFromHere reason | a_share_from_a_computer_that_is_not_master_says_shared_from_here | FAILS |
| Master-only ShareSettingsWhenReady | only_the_master_shares_in_bulk_but_any_computer_may_push_one_unit | FAILS |
| Master-only ShareSettingsNow | same test | FAILS |
| Touch on reuse (RV11-12) | a_reused_old_copy_starts_its_age_again_when_content_returns_to_it | FAILS |
| Keep recent records (RV11-12) | cleanup_keeps_every_copy_a_recent_revision_record_names_even_if_unread_here | FAILS |
| Waiting for shared copy | a_revision_whose_copy_is_missing_waits_truthfully_and_applies_when_it_returns | FAILS |
| Omit unavailable history (RV11-13) | settings_history_does_not_offer_a_copy_the_shared_folder_has_lost | FAILS (rewritten: the ledger keeps the entries, the shared folder loses the copies) |

### Decisions
- Decision: bulk share commands only for the effective master; ShareSettings (one unit) stays open. Why: design 5.2 / RV11-14. Reversible: yes.
- Decision: reused blobs and manifests are touched; GC also keeps every checkpoint named by a provider record under 30 days old. Why: Dropbox lag (RV11-12). Reversible: yes.
- Decision: a record's versions are the unit's manifest (checkpoint) id and total size. Why: units are single-file in practice; the id covers the whole unit. Reversible: yes.
- Decision: history omits entries whose copy is gone. Why: no dead restores. Reversible: yes.
### Is the settings-pass memory rise file-size related? (follow-up)
Same lab and real release app, only the big file shrunk to 100 KB (one byte edited on A, B applies):
| File | Idle private WS | Pass private WS | Rise | Pass private commit |
|---|---|---|---|---|
| 31.9 MiB | 57 MiB | 86 MiB | 29 MiB | 117 MiB |
| 100 KB | 73 MiB | 99 MiB | 26 MiB | 121 MiB |
The rise is the same with a tiny file, so it is not proportional to file size and nothing in the settings path still reads whole files; it comes from the app's per-pass work on the 113-add-on lab (status snapshots / GUI), not from file contents. Idle varies run to run (57-94 MiB), and the 100 KB run peaked at 99 MiB, close to the 100 MiB gate: the headroom is thin and comes from the app baseline, which is outside the runtime's files. Flagged as an open issue for R11-app / the owner.

### Memory regression investigation (private working set, same 113-add-on / 36k-file lab, real release app)

| Variant | Idle | Pass peak |
|---|---|---|
| (a) v0.3.3 (settings state removed) | 76.2 MiB | 76.4 MiB |
| (b) main, settings off | 79.4 | 79.6 |
| (c) main, settings on (before) | 87.0 | 98.3 |
| (c) after lazy publish (this change) | 74.7 | 83.2 |

Findings: the regression predates run-011 (v0.3.3 is already 76 MiB on this lab; earlier 37-40 MiB figures were from labs with ~130 files; here per-file memory dominates, and about 24 MiB is GUI native memory). Settings added ~8 MiB at idle and ~11-19 MiB per edit pass, because a settings publish moves the authority head and so re-plans a full code-state publish, which built every object for all 113 folders at once (encoded trees ~12 MiB, ~36k ObjectRefs ~16 MiB, a full inventory clone ~4 MiB; ~33 MiB at the heap peak).
Fix: the code publish now streams its objects one folder at a time (blobs, then that folder's tree, state record last) and its state carries no local file lists. Result: (c) pass peak 98.3 -> 83.2 MiB, now about the same as settings off (79.6). The 70 MiB target is NOT reached: the remaining ~76 MiB is the pre-existing per-file baseline (scan/inventory/hash-cache transients and GUI), which would need a separate reduction pass (Arc'd scans, no tree decode for unchanged trees); not attempted here.

- Decision: stream publish objects lazily; keep `plan_publish` returning full objects for tests. Why: removes the largest transient (~25 MiB). Reversible: yes.
- Decision: a source file whose hash no longer matches at upload aborts the publish as Unstable (was a failure). Why: same meaning as the existing source-changed check. Reversible: yes.
- Heap harness `examples/heap_lab.rs` (lab path as argument) is committed.

## R11-check fix round (G2–G7, OD-40..44)

Interface commit 8bb2205 (G4 `PauseAllSettings`/`ResumeAllSettings`, G3 `HeadChoice` fields,
`settings_decisions_pending`, G5 `RestoredFromRecovery`, `RecoveryCopy`); behaviour in the
commit after it.

### Fails-without-fix (this round)

| Guard | Test | With the guard reverted |
|---|---|---|
| Restore-kind revision applied from a peer is `RestoredFromRecovery` | `a_copy_replaced_by_the_masters_is_listed_restorable_and_survives_later_applies` | fails |
| Restore-kind revision published here is `RestoredFromRecovery` | same | fails |
| `paused_all` pauses every unit, also later-discovered add-ons | `pausing_all_settings_persists_covers_later_addons_and_resumes` | fails |
| Headline never says "up to date" with a pending settings decision | `the_snapshot_units_carry_the_heads_choose_takes_and_the_history_restore_takes` | fails |
| Settings records carry the add-on's folder key (applied) | `a_settings_record_is_found_by_the_addons_own_folder_name` | fails |
| Settings records carry the add-on's folder key (published) | same | fails |
| Head choices carry source/kind/time/size; recovery entries carry kind/source/time/size | the two tests above | fail against the interface stubs (all `None`/`Unknown`) |
| G7 shared variable | `two_addons_declaring_one_variable_with_different_bytes_are_owned_by_several_from_the_first_pass` | **passes without any change** (see below) |
| OD-44 export privacy | `the_diagnostics_export_with_settings_on_names_no_account_realm_character_or_path` | **passes without any change** (see below) |

### G7 and OD-44: not reproduced in the runtime

Both add-ons declaring one SavedVariables name, different bytes on A and B, shared by the
master: every unit row on both computers is `OwnedBySeveral` and names the other add-on
from the first pass, and nothing is published. The same holds when one TOC also lists
other variables. The runtime already produces the status; "Not shared yet" is the app's
text for an add-on with no unit rows or for `NoSharedCopy`. If R11-check still sees it,
the app side (R11-app) is where to look: `addon_settings_line` in `settings_ui.rs` falls
back to "Not shared yet" when the add-on has no row. The test is kept as a regression guard.

The diagnostics export with settings on, character-scoped units present and synthetic
realm/character folders planted contains none of those names and no variant or provider
path, on both computers. The redaction context already reads the WTF folder names.

### G2 memory (release app, 113 add-ons, ~36k files, private working set)

Sampler: `QueryWorkingSet` private pages every 25 ms (about 6,000 samples per run), checked
against the `\Process(*)\Working Set - Private` counter (69.6 vs 69.6 MiB in run 1). Each run
starts the window visible on the page named, lets a settings edit and a code edit run while
visible, hides the window at 40 s, then in the hidden state runs: idle 30 s, a settings
pass, a code update, an add-on add, an add-on remove and a second settings pass.

| Run | Window / page | Visible peak | Hidden idle after UI use | Hidden-only peak (incl. all passes) |
|---|---|---|---|---|
| 1 | Settings sync, zoom 1.0 | 85.1 | 67.9 | 73.8 |
| 2 | Add-on detail then Add-ons list, zoom 1.0 | 85.3 | 71.1 | 76.3 |
| 3 | Settings sync, zoom 1.5 | 93.7 | 87.1 | 93.2 |

All hidden samples are at or below 100 MiB (maximum 93.2 MiB), but this scripted scenario
cannot click, so it does not perform the GUI share/choose/restore clicks the checker did;
the settings and code passes themselves ran in all three.

What is resident: the visible peaks are the start-up scan (about 85 MiB in the first 5 s,
before the window is shown). After hiding, a window that was shown stays about 10–14 MiB
above one that never was: at zoom 1.5, started hidden, hidden max 79.8 MiB, against 92.4 MiB
for shown then hidden. The window is only `SW_HIDE`n, so its renderer buffers remain while
the process lives. Skipping the repaint request while hidden made no difference (92.4 MiB),
so that was not applied. Releasing it needs closing and recreating the GPUI window on
hide/show, which `Shell` (window-bound handles and focus) does not support today. Not done;
for the owner/orchestrator to decide. Manifest rewrites, hash-cache writes and the settings
pass add only about 4–6 MiB over idle (visible in the 5 s maxima: 67–68 MiB idle, 73–74 MiB
in a pass).

### Decisions (this round)

- Decision: any Restore-kind revision (history restore or recovery-copy restore) is
  `RestoredFromRecovery` on the restoring and the receiving computer. Why: one command, one
  kind; both are "put an earlier copy back". Reversible: yes (one match arm in `changes.rs`).
- Decision: settings change records list the folder key next to the display name. Why: the
  add-on's own "Recent changes" matches by folder key. Reversible: yes.
- Decision: while a settings decision is pending the runtime headline reads "Saved settings
  need your choice" instead of the OK headline. Why: never "up to date" with a decision
  waiting. Reversible: yes.
- Decision: G7 and OD-44 are covered by regression tests only; no runtime change. Why: not
  reproducible in the runtime (above). Reversible: n/a.
- Decision: no renderer change this round (above). Reversible: n/a.
## Round G9-G12 (restore, history entries, memory)

### Fails-without-fix

| Fix | Test | Without the fix |
|---|---|---|
| G11 restore parents include the issuer's own heads (core `build_restore`) | `a_master_restore_after_a_decision_does_not_create_a_decision_between_its_own_copies` | NeedsChoice between the master's own copies |
| G9 rows for every declaring add-on (`settings_cycle`) | `two_addons_declaring_one_variable_both_get_rows_even_without_their_own_files` | no row names the add-ons |
| G10 entries carry origin, source, size, seq | `history_entries_carry_origin_source_size_and_sequence` | stub has source None |
| `a_master_restore_then_a_choice_leaves_both_computers_on_the_selected_bytes` | guard; passes either way | n/a |

G12 changes are allocation reductions with no behaviour change; the existing suites cover them
(no new test: a heap-size assertion would be machine-dependent).

### Heap (dhat, accumulated lab: 113 add-ons, ~36.9k files, ~243 settings revisions, a 31.9 MiB file)

Ledger and settings history are ~0.13 MiB; what scales is the per-file inventory.

| | Before | After |
|---|---|---|
| Peak live heap (max) | 41.9 MiB | 30.8 MiB |
| Live after a pass | 17.0 MiB | 17.0 MiB |

Peak contributors before: inventory `Vec<FileEntry>` clones 6.4, hash-cache map growth 6.1,
`Vec<ScannedFile>` clones 3.7+3.7+3.7, FolderState clones 2.8. Fixes: `file_entries` and
scan `files` are `Arc<Vec<_>>` (clones share), and the hash-cache parse pre-sizes its map
instead of doubling. Records on disk and on the provider are unchanged (v0.3.3 compatible).
Remaining at peak: hash cache 2.1 (retained), tree record decode 3.6, scan 2.3+1.4,
build_inventory 2.2+2.2+1.4.

### Memory (release build, accumulated state, private working set, 25 ms samples with
per-sample window visibility via EnumWindows/IsWindowVisible; three fresh processes on the
same growing state; visible at start, hidden after 40 s, then settings and code edit, add and
remove passes)

| Run | Visible peak | Hidden-after-UI peak (all hidden samples) | Hidden idle after UI |
|---|---|---|---|
| 1 | 85.4 | 86.4 | 80.2 |
| 2 | 91.6 | 88.3 | 82.0 |
| 3 (zoom 1.5) | 92.1 | 89.0 | 83.2 |

Hidden-only (never shown) was not re-measured on this state; the window starts visible.
Reading: all hidden samples are at or under 90 MiB, but only 1 to 3.6 MiB of drift upward over
the three runs (the state also grows by a pass each run), so margin to 100 is ~11 MiB and the
headroom is not large. Idle floor (~80) is mostly the renderer (~10-14 MiB retained after hide)
plus the 17 MiB inventory. Further reduction would need a compact per-file inventory
(interned paths, fixed hashes) or dropping the resident inventory between passes.
No working-set trimming, helper process or measurement change was used. I did not measure the
checker's before-state on this lab, so the before/after memory comparison is the heap table.

### Decisions
- Decision: restore parents also include this issuer's own current heads. Why: otherwise a
  restore right after the master's own choice competes with it. Reversible: yes.
- Decision: G9 emits account-identity rows for each declaring add-on even without own files.
  Why: the checker's shape has a file no folder owns. Reversible: yes.
- Decision: history sizes come from a bounded (4096) process cache of immutable checkpoint
  sizes. Why: avoids re-reading manifests per query. Reversible: yes.
- Decision: share inventory entries by `Arc` instead of compacting. Why: removes the clones
  with no change to records or logic. Reversible: yes.