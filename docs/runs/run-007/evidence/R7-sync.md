# R7-sync evidence (run-007)

Scope: SPEC R-05 as amended by D-29. Worktree `~/GitHub/wow-sync.worktrees/R7-sync`,
branch `run007/R7-sync`, from main 8b0dee0. Environment: Windows, local dev PC,
synthetic data only.

## What changed

1. **`.toc` header reading.** `parse_toc` reads an oversized file (> `TOC_LIMIT`,
   256 KiB) from the complete lines of its first 256 KiB. `fs_scan::read_bounded`
   reads a `limit+1` prefix instead of failing; the whole file is still hashed.
   A header beyond the window, or a window with no metadata, is `TooLarge`
   (the folder is then unrecognised). A cut multi-byte character does not fail.
2. **Unrecognised add-ons sync by content.** Eligibility is `status == Ok`;
   a toc kind of None/Ambiguous/Unvalidated/Error is "unrecognised", not blocked.
   Unreadable/Unsafe/Unstable/Unsupported stay blocked. `TocKind` is unchanged
   (serde compatible). The four toc `BlockReason` variants were removed.
3. **Ordering (reworked after review RV7-01).** Differing unrecognised copies
   compare as `Ambiguous(UnrecognisedDiffers)`. The dominator rule's one extra
   arm uses `LineageGraph::newer`: X is newer than Y only if Y is reachable from
   X through **published** ancestors and X is not reachable from Y. Unrelated,
   mutual or unpublished ancestry gives one `NeedsDecision(Ambiguous)`.
4. **Published lineage (no local memory).** `code-state/1` gains an additive,
   optional `lineage: [PackageLineage { package, hash, ancestors }]` (at most 32
   ancestors, newest first; validated). `next_lineage` publishes for each
   unrecognised complete package the replaced hash (from the computer's own
   previous published state) plus what admitted records already know.
   `SharedReleases`, `shared_releases()`, `TargetInput.shared` and
   `shared-releases.json` are removed; nothing reads local memory for ordering.
   The record model could carry this, so no stop was needed.
5. Docs: domain.md, questions.md (Q-89), contracts.md (§1.6, §5.1), core-api.md,
   tests.md, publication.md, removal.md, ui/screens.md.

## Review findings (run-007 review)

| ID | Resolution |
|---|---|
| RV7-01 | Lineage published in `code-state/1.lineage`; every computer derives the same graph. An ancestor of the group's copy never wins; a returning offline peer is replaced (copy kept in recovery). Unrelated/both-changed/first-join: one decision. After a decision the loser is an ancestor/considered hash, so it cannot return. Tests: `a_peer_that_was_offline_while_the_group_moved_on_is_replaced_not_obeyed` (the reviewer's probe, three computers), `a_returning_peer_with_a_changed_copy_gets_one_decision_and_nothing_is_overwritten`, `a_copy_that_lost_a_decision_cannot_come_back_from_an_offline_peer`, `unrecognised_copy_unrelated_to_the_group_copy_is_not_installed_or_overwritten`, `a_superseded_unrecognised_copy_never_wins_again`, `unrecognised_copies_both_changed_since_the_common_copy_need_a_decision`. |
| RV7-02 | `compare_package` checks recognised folders' version evidence first; a contradiction is `MembersDisagree`. Test `lineage_never_overrides_contradicting_version_evidence_in_a_package`. |
| RV7-03 | New `Explanation::ContentLineage`: "changed from the copy last shared, on <name>; replaces <short hash or 'the earlier copy'>"; never "version unversioned". Asserted in `unrecognised_copy_changed_on_the_peer_is_installed_and_explains_itself`. |
| RV7-04 | An oversized `.toc` stays `Selected` only when the window shows a non-blank, non-`#` line (header ended); else `TooLarge` (unrecognised). Test `cut_header_without_a_file_line_is_not_selected` (both cases). |
| RV7-08 | The e2e tests assert exactly one `Ambiguous` decision (and none where none is expected) and that recovery copies are kept (`recovery_holds`), including the three-computer offline cases. |

## Recheck fixes (RV7-10, 11, 12)

| ID | Resolution |
|---|---|
| RV7-10 | `TargetInput.local_prev` carries this computer's own latest published state (read in `observe_world`; `members` skip this computer). Service-path e2e tests: `an_author_whose_peers_are_all_offline_publishes_the_change_without_a_decision` (no decision, peers install), `a_revert_to_an_earlier_copy_is_not_undone_silently_and_asks_once` (revert stays, one `Ambiguous`). Control: passing `None` failed both. Documented in domain.md §6. |
| RV7-11 | `next_lineage` keeps ancestors still held by an admitted peer ahead of the 32 cap. e2e `more_changes_than_the_lineage_cap_while_a_peer_is_offline_do_not_stall_the_rest` (36 changes, no decision, c converges on return); unit `ancestors_still_held_by_a_peer_survive_the_cap`. Dropped older ancestors can only turn a win into a decision. |
| RV7-12 | `LineageGraph::from_states` accepts an entry only when its hash equals the package hash its publisher holds. Test `a_lineage_entry_for_a_copy_its_publisher_does_not_hold_is_ignored`. |

Counts after the fixes: core 377, runtime 210 (+2 public_api). Verify tiers: see the final commit message.

## Intentional behaviour changes (D-29)

- Folders previously blocked for a toc reason now sync (e.g. a suffixed-only
  folder on an unfamiliar client; OnlyVanilla under Forever).
- The "not synced" status for toc-less folders is gone.
- Unfamiliar-client "blocked" assertions flipped
  (`variant::tests::unrecognised_folders_are_eligible_for_unfamiliar`).
- `tests/required-cases.toml`: section `R7-sync` (`R7-sync:toc-header`,
  `R7-sync:unrecognised-by-content`) lists the current test names. R6-check markers preserved.

## Tests added or renamed (key ones)

- toc: `oversized_toc_is_read_from_its_header`,
  `cut_inside_a_multibyte_character_keeps_the_header`,
  `header_beyond_the_window_is_not_selected`,
  `cut_header_without_a_file_line_is_not_selected`; adapters
  `oversized_toc_is_read_as_a_bounded_prefix`.
- inventory: `eligibility_follows_status_and_unrecognised_folders_say_why`.
- target: `unrecognised_add_on_only_on_a_peer_is_installed`,
  `identical_unrecognised_copies_are_in_sync`,
  `differing_unrecognised_copies_without_lineage_need_a_decision`,
  `unrecognised_copy_changed_here_from_the_peer_copy_wins`,
  `unrecognised_copy_changed_on_the_peer_is_installed_and_explains_itself`,
  `unrecognised_copies_both_changed_since_the_common_copy_need_a_decision`,
  `unrecognised_copy_unrelated_to_the_group_copy_is_not_installed_or_overwritten`,
  `a_superseded_unrecognised_copy_never_wins_again`,
  `unrecognised_copies_that_descend_from_each_other_need_a_decision`,
  `lineage_never_overrides_contradicting_version_evidence_in_a_package`,
  `next_lineage_names_the_replaced_copy_and_what_the_records_know`,
  `foreign_suffix_only_copy_is_unrecognised_not_blocked`.
- plan: `unrecognised_folders_are_published_by_content`.
- e2e: `unrecognised_folder_is_published_by_content_and_says_so`,
  `unrecognised_folder_only_on_a_installs_on_b_and_later_edits_follow`,
  `a_peer_that_was_offline_while_the_group_moved_on_is_replaced_not_obeyed`,
  `a_returning_peer_with_a_changed_copy_gets_one_decision_and_nothing_is_overwritten`,
  `a_copy_that_lost_a_decision_cannot_come_back_from_an_offline_peer`,
  `unrecognised_folder_changed_on_both_is_never_overwritten_and_asks`,
  `unrecognised_folders_that_differ_at_first_join_are_not_overwritten`,
  `unrecognised_folder_removal_and_restore_round_trip`,
  `forever_suffix_folders_sync_by_the_forever_rule_and_unrecognised_ones_by_content`.

## A-38 controls (mutation, then failing tests, then revert)

| Goal | Mutation | Failed under mutation | Reverted |
|---|---|---|---|
| toc header | `parse_toc` returns `TooLarge` for any oversized file | `oversized_toc_is_read_from_its_header`, `cut_inside_a_multibyte_character_keeps_the_header` | yes |
| eligibility | `transfer_eligible` requires `toc.kind == Selected` | `inventory::...eligibility_follows_status...`, `plan::...unrecognised_folders_are_published_by_content` | yes |
| lineage | `LineageGraph::newer` always `false` | 3 `target` tests (`...changed_here_from_the_peer_copy_wins`, `...changed_on_the_peer_is_installed...`, `a_superseded_...`), e2e `...later_edits_follow`, `a_peer_that_was_offline...` | yes |
| publisher-held check (RV7-14) | `from_states` accepts every entry (`if true \|\| held == Some(l.hash)`); the test now has peer 3 hold v3 | `a_lineage_entry_for_a_copy_its_publisher_does_not_hold_is_ignored` fails with `Chosen` v1 instead of `NeedsDecision`; passes with the check | yes |
| header ended | `header_has_ended` always `true` | `cut_header_without_a_file_line_is_not_selected` | yes |

## Settings finding

`settings_cycle` declares SavedVariables only for a `Selected` toc. An
unrecognised add-on declares none, so its **settings are not synced**; no
ownership was added. Oversized tocs that now parse as Selected declare theirs,
so those add-ons' settings can now sync.

## User-facing text

- Label: "Unrecognised: synced by content"; full form
  "Unrecognised: synced by content — <reason>." Reasons: "no description (.toc)
  file for this game version"; "several description (.toc) files and no way to
  tell which applies"; "description (.toc) files only for other game versions";
  "its description (.toc) file cannot be read". Several folders append
  " (and N more folders)". Shown on the status package line, `PackageRow.sentence`
  and `CandidateRow.differences`.
- Decision evidence: "unrecognised add-on: no version to compare, the copies differ".
- Lineage choice: "<folder>: changed from the copy last shared, on <computer>;
  replaces <8-char hash or 'the earlier copy'>".

## Commands and results

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` (fmt, clippy, test, required-cases, deny, markdown-links) | PASS |
| `cargo xtask verify --tier native` | PASS |

Counts (workspace): adapters 79 passed (5 ignored), app 130, core 375, proto 11,
runtime 206 + 2 (public_api), xtask 26; 829 passed, 0 failed.
GUI/manual run on a real WoW install: NOT RUN.

## Limits

- A peer whose state was published before this feature carries no lineage, so
  its copy is unrelated and gets one decision (conservative).
- A reverted copy (mutual ancestry) gets a decision, not an automatic winner.
- A replaced copy is always kept in recovery (existing replacement behaviour).
- Under Forever, a foreign-suffixed-only folder reads as toc kind None, not Unvalidated.
- Unrecognised add-on settings are not synced.
