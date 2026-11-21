# R5-decide evidence

Branch `run005/R5-decide`, base fe13e3d. Environment: Windows, local checkout of the worktree only.
Requirements: A-27, A-38, Q-88 (a) / D-26, RV-09, RV-11, V-01 (R4-check).

## Built

1. **Q-88 (a).** A kept version V whose copy is no longer held anywhere now yields a synthesized
   kept copy (`choice.rs` `absent_copy`, `NewVersion.absent_kept`), so `NewVersionAfterDecision`
   is raised every cycle. A newer version held only by this computer is withheld: not applied
   over (`target_without`), not published (`inventory_without`). It stays installed. Answers:
   "Use W" shares it, "Keep V" returns to V (W goes to recovery), "Follow newest" resumes.
2. **DataAtRisk producer** (`notices.rs`): `DataAtRisk`/`RiskKind`, fed from unreadable local
   state (`pending-report.json` unreadable/corrupt), failed applies naming a recovery path, and
   full-disk errors. Fed through `NoticeRunner` (Q-85/Q-86 rules) and exposed as
   `StatusSnapshot.data_at_risk`.
3. **RV-09**: real fingerprints (`package_hash` over folder trees) only with
   `include_fingerprints`; build commit from `WOWSYNC_BUILD_COMMIT` (`dev` if unset/empty).
4. `Command::SaveDiagnostics` removed (use `SavePreviewedDiagnostics`); no app references.
5. **RV-11**: `/usr/bin/sw_vers` in `adapters/src/sysinfo.rs`. `platform.rs` (`date`) is another crate, not touched.

## Verification (actual runs)

- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo xtask verify --tier native`: PASS.
- New tests (PASS): 2 Q-88 e2e, 2 DataAtRisk e2e, 1 notices unit, 1 diag unit, 1 fingerprint e2e.
  Runtime lib suite 149 before / 156 after; core choice tests updated.
- NOT RUN: macOS (sw_vers path is compile-checked on Windows only); a real full-disk failure (classified by message text only, unit-tested).

## A-38 controls (mutate, run test, revert, `git diff --exit-code`)

| Mutation | Test | Result | Revert diff |
|---|---|---|---|
| `publishable = w.inventory.clone()` (service.rs) | `a_newer_local_only_version_after_a_keep_stays_installed_unshared_and_is_asked_every_cycle` | exit 101 | clean |
| drop `local_state_risk` producer (service.rs) | `unreadable_local_state_is_a_data_at_risk_condition_in_the_snapshot_and_a_notice` | exit 101 | clean |
| fingerprints forced to none (diag.rs) | `diagnostics_carry_real_file_fingerprints_only_when_asked` | exit 101 | clean |

## Notes and disclosures

- **Behavior change:** with the kept copy absent, order is unknown, so even an older unconsidered
  copy raises the decision. `choice_tests` assertion "without the kept copy nothing can be
  compared" was replaced accordingly. contracts.md §5.3 / ops §1.5 should be updated (not owned).
- Recovery-write failures are recognised from the error path (`/recovery/`), not a typed error,
  because `exec.rs` is not owned; a typed kind would be more robust.
- **.NET file write:** yes. Early in the run I tried a `[IO.File]::WriteAllText` script on
  `diag.rs` (relative path). The replacements did not match and nothing changed in the worktree;
  I redid the edit with the edit tool. All later writes used edit/create only. I cannot rule out
  that this call is the stray write the orchestrator found in main; main was clean on my checks.
- Did not add core unit tests for `NewVersion::local_only` / `Source::from_release`; covered by e2e.

## Follow-up: update check with no full release (D-26 pre-release)

- HTTP 404 from `releases/latest` now yields `UpdateResult::NoRelease { checked }` (core `update.rs`;
  `runtime/updates.rs::fail_kind` maps `FetchError::Http(404)` to the internal carrier
  `UpdateFailKind::NoRelease`). No activity line, no notice. Other failures and pre-/draft bodies are
  unchanged (still `Failed`, never offered). Adapter unchanged.
- Text: `wow_sync_core::update::NO_RELEASE` = "No release has been published yet."
  (`UpdateResult::text()`, `UpdateView.text`).
- Tests: `updates::tests::a_404_means_no_release_yet_and_is_neither_a_failure_nor_a_notice`,
  `updates::tests::other_http_errors_and_a_prerelease_body_still_fail_and_are_never_offered`,
  core `update::tests::results_map_to_state_and_text`.
- As-built notes added to contracts §5.3 and ops §1.5 (Q-88/D-26).
- .NET file write: none in the follow-up (edit/create tools only).
