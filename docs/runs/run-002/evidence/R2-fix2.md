# R2-fix2 evidence (run-002)

Branch `run002/R2-fix2`, from main after R2-fix, R2-review and R2-recheck.

## Changes

| Item | Change | Test |
|------|--------|------|
| MJ-1 consent pre-tick | New `crates/wow-sync-app/src/consent.rs`. `Consent::observe(panel)` clears both boxes whenever the enroll/reconfirm panel changes, disappears or is re-shown. Boxes are also cleared when a request is sent. `ui.rs` uses it for both panels. | `consent::tests::boxes_reset_when_the_panel_is_reshown_or_changes`, `clear_is_single_use` |
| m4 `NotPlainFile` text | Runtime `prov()` maps `NotPlainFile`/`NotFound` to a plain-English shared-folder message, and `CloudOnly` to an online-only message. Adapters are untouched. | e2e `provider_problem_is_plain_english` |
| m4 empty `Game activity:` | UI omits the line when the activity text is empty. | None (GUI-only; compile check only) |
| mi-3 changed product/folder | `confirm_enrolled` returns `VariantChanged` for a different product or folder (`StaleConfirmation` stays for recognition class or mapping). Runtime maps it to `ServiceError::VariantChanged`; status shows "Something needs your help" with a plain message and no reconfirm offer. | core `enrollment_goes_stale_only_for_product_folder_or_class_change` (updated), e2e `changed_product_needs_help_without_a_doomed_reconfirm_offer` |

## MJ-3 (A-38) negative controls

Each fix was mutated in place on top of commit f6d7144. Each listed test was run and each FAILED. The file was then restored with `git checkout -- <file>`. No mutation was committed, and `git status` was clean afterward.

| Finding | Mutation | Test | Outcome |
|---------|----------|------|---------|
| M1 | `confirm_enrolled` returns `StaleConfirmation` for any guided client | core `later_folders_do_not_invalidate_enrollment` | FAILED |
| M1 | same | e2e `enrolled_unfamiliar_client_keeps_syncing_after_new_folder` | FAILED |
| M2 | `transfer_eligible` ignores the toc kind | core `blocked_folders_are_never_published` | FAILED |
| M2 | same | e2e `blocked_folder_is_not_published_and_shows_not_synced` | FAILED |
| M3 | `check_join` always allows | core `join_requires_equal_product` | FAILED |
| M3 | same | e2e `join_refused_for_other_product` | FAILED |
| M3 | `publish_needed` always true | core `publish_needed_decisions` | FAILED |
| M4 | `consent_disclosures` returns empty | core `consent_disclosures_cover_scope_mapping_replacement_removal_and_recovery` | FAILED |
| M4 | same | e2e `enrollment_preview_discloses_scope_and_uses_canonical_label` | FAILED |
| M5 | ambiguity candidates emptied | core `needs_decision_shows_evidence` | FAILED |
| M5 | same | e2e `ambiguity_status_names_versions_and_computers` | FAILED |
| MJ-1 | `observe` never resets | app `boxes_reset_when_the_panel_is_reshown_or_changes` | FAILED |

Not mutated: the new `provider_problem_is_plain_english` and `changed_product_needs_help...` tests (not requested).

## Commands

See the final run at the end of this file.

## Not done

- GUI visual check of the new behaviour: NOT RUN (no interactive window). Compile and unit tests only.
- The pure consent test covers the state transition. The wiring in `render` is covered by review only.
- Memory not measured.
