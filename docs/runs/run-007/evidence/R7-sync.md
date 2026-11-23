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
3. **Ordering.** Differing unrecognised copies compare as
   `Ambiguous(UnrecognisedDiffers)` (renamed from `TocNotSelected`). The
   dominator rule gains one arm: the copy whose competitor equals the recorded
   shared release is newer. No shared record, or both changed: `NeedsDecision`.
4. **Judgement call (flag).** A new local, unpublished record `SharedReleases`
   (`shared-releases.json`, `wowsync.shared-releases`). No existing record
   holds "last shared release" (admission reads only each peer's latest state).
   It is updated at observation when the local package is complete and an
   admitted peer holds an identical hash; entries for vanished packages are
   pruned; entries are filtered by group id.
5. Docs: domain.md, questions.md (Q-3/Q-20/Q-22 text, new Q-89), contracts.md,
   core-api.md, publication.md, removal.md, tests.md, ui/screens.md.

## Intentional behaviour changes (D-29)

- Folders previously blocked for a toc reason now sync (e.g. a suffixed-only
  folder on an unfamiliar client; OnlyVanilla under Forever).
- The "not synced" status for toc-less folders is gone.
- Unfamiliar-client "blocked" assertions flipped
  (`variant::tests::unrecognised_folders_are_eligible_for_unfamiliar`).
- `tests/required-cases.toml`: entries in other sections renamed to the new
  test names; new section `R7-sync` (`R7-sync:toc-header`,
  `R7-sync:unrecognised-by-content`). R6-check markers preserved.

## Tests added or renamed (key ones)

- toc: `oversized_toc_is_read_from_its_header`,
  `cut_inside_a_multibyte_character_keeps_the_header`,
  `header_beyond_the_window_is_not_selected`; adapters
  `oversized_toc_is_read_as_a_bounded_prefix`.
- inventory: `eligibility_follows_status_and_unrecognised_folders_say_why`.
- target: `unrecognised_add_on_only_on_a_peer_is_installed`,
  `identical_unrecognised_copies_are_in_sync`,
  `differing_unrecognised_copies_without_lineage_need_a_decision`,
  `unrecognised_copy_changed_here_since_the_shared_release_wins`,
  `unrecognised_copy_changed_on_the_peer_since_the_shared_release_is_installed`,
  `unrecognised_copies_both_changed_since_the_shared_release_need_a_decision`,
  `foreign_suffix_only_copy_is_unrecognised_not_blocked`,
  `shared_release_is_recorded_only_when_a_peer_holds_the_same_copy`.
- plan: `unrecognised_folders_are_published_by_content`.
- e2e: `unrecognised_folder_is_published_by_content_and_says_so`,
  `unrecognised_folder_only_on_a_installs_on_b_and_later_edits_follow`,
  `unrecognised_folder_changed_on_both_is_never_overwritten_and_asks`,
  `unrecognised_folders_that_differ_at_first_join_are_not_overwritten`,
  `unrecognised_folder_removal_and_restore_round_trip`,
  `forever_suffix_folders_sync_by_the_forever_rule_and_unrecognised_ones_by_content`.

## A-38 controls (mutation, then failing tests, then revert)

| Goal | Mutation | Failed under mutation | Reverted |
|---|---|---|---|
| toc header | `parse_toc` returns `TooLarge` for any oversized file | `oversized_toc_is_read_from_its_header`, `cut_inside_a_multibyte_character_keeps_the_header` | yes |
| eligibility | `transfer_eligible` requires `toc.kind == Selected` | `inventory::...eligibility_follows_status...`, `plan::...unrecognised_folders_are_published_by_content` | yes |
| lineage | dominator lineage arm returns `false` | the two lineage tests in `target`, e2e `unrecognised_folder_only_on_a_installs_on_b_and_later_edits_follow` | yes |

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

## Commands and results

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` (fmt, clippy, test, required-cases, deny, markdown-links) | PASS |
| `cargo xtask verify --tier native` | PASS |

Counts (workspace): adapters 77 passed (5 ignored), app 119, core 370, proto 11,
runtime 197 + 2 (public_api), xtask 26; 802 passed, 0 failed.
GUI/manual run on a real WoW install: NOT RUN.

## Limits

- The shared release is learned only on observation; a change made before both
  computers observed the same copy falls back to the decision (conservative).
- No cleanup of `shared-releases.json` on unenroll (entries are group-filtered).
- Under Forever, a foreign-suffixed-only folder reads as toc kind None, not Unvalidated.
- Unrecognised add-on settings are not synced.
