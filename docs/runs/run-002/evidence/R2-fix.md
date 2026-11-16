# R2-fix evidence (run-002)

Branch `run002/R2-fix`. Policy lives in `wow-sync-core`; runtime/app only call it.

## Finding → change → test

| ID | Change | Test(s) |
|----|--------|---------|
| M1 later folder stales enrollment | core `confirm_enrolled` re-derives each cycle and ignores the digest; only product/folder/mapping/class change → `StaleConfirmation`. Runtime maps that to `NeedsReconfirmation`; status carries a reconfirm preview; `reconfirm()` + GUI "Confirm and continue" (core `reconfirm_variant`). | core `later_folders_do_not_invalidate_enrollment`, `enrollment_goes_stale_only_for_product_folder_or_class_change`, `reconfirmation_needs_same_client_and_both_confirmations`; e2e `enrolled_unfamiliar_client_keeps_syncing_after_new_folder` |
| M2 blocked folders published | `FolderState::transfer_eligible()`; `plan_publish` and `inventory_complete` use it; blocked local folder → `PackageTarget::NotSynced` (status "not synced", headline-neutral). | core `blocked_folders_are_never_published`, `blocked_local_folder_is_not_synced_not_up_to_date`; e2e `blocked_folder_is_not_published_and_shows_not_synced` |
| M3 publish/join rules in runtime | core `check_join`, `publish_needed`; runtime's string compare and `same_folders` removed. | core `join_requires_equal_product`, `publish_needed_decisions`; existing e2e `join_refused_for_other_product` |
| M4 consent disclosures | core `consent_disclosures` (install/AddOns path, validated-plain rule, auto install/replace, missing ≠ removal, recovery path, WTF exclusion); `EnrollPreview.disclosures` shown in the GUI before the checkboxes. | core `consent_disclosures_cover_...`; e2e `enrollment_preview_discloses_scope_and_uses_canonical_label` |
| M5 ambiguity evidence | `AmbiguousVersions` carries candidates (version label, source computers, here); rendered in status. | core `needs_decision_shows_evidence`; e2e `ambiguity_status_names_versions_and_computers` |
| m1 label | `VariantIdentity::display_label()` ("World of Warcraft: Forever (wowf)"). | core `identity_label_is_canonical_not_classic_era`; e2e above |
| m2 headline | `render_headline` → "Waiting — WoW is running". | core `headline_names_running_game_while_waiting`; e2e `running_game_headline_and_applied_line_name_source` |
| m3 post-apply line | `applied_here` keeps the plan explanation; status renders `AppliedFrom` (source, versions when known). | core status post-apply assertion; e2e (same test, "from computer") |
| m4 duplicate/empty text | `StatusSnapshot::who_line()` is None when no group/computer; backend no longer repeats the error as message when `provider_error` is set. | e2e `…canonical_label` (who_line None). Backend dedupe: no automated test (GUI thread). |
| m5 interruption cause | `Interrupted{package}`; blamed on the game only when `game_running`. | core `interruption_blames_the_game_only_when_it_is_running` |

## Commands

- `cargo xtask verify --tier offline` — PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo test --workspace` — PASS: core 163, runtime 21, adapters 45 (3 ignored), xtask 24.
- New cases registered in `tests/required-cases.toml` (`R2-fix:core`, `R2-fix:slice`).

## Not done / unresolved

- Fails-without-fix by temporary revert: NOT RUN (tests were written against the new APIs; M1–M3 tests call APIs that did not exist before).
- GUI check with the built app (disclosures, reconfirm panel, ambiguity, headline, error display): NOT RUN — no interactive window available to this worker. `cargo build --workspace --all-targets` compiles the UI changes (PASS). The reconfirm panel and backend dedupe are therefore unverified visually.
- m4 raw `NotPlainFile` wording comes from adapter `FsError` Display; adapters not changed.
- Memory gate untouched (not measured).
- Design decisions: build_version change alone does not force reconfirmation; `inventory_complete` now means all folders are transfer-eligible; a blocked local folder is "not synced" and does not alter the headline.
