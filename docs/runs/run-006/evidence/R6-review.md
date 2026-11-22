# R6-review evidence (independent integration review)

Reviewer: R6-review, read-only. Worktree `D:\GitHub\wow-sync.worktrees\R6-review`,
branch `run006/R6-review`, from `main` `6e87a35`. Range reviewed:
`git diff c77d050..6e87a35` (R6-gate, R6-app stages A and A2, R6-release).
Only this file was written.

**Limit:** R6-check (live verification on the dev-PC `_forever_` install)
ran in parallel, and its evidence was not available. Statements below are
checked against the code, the tests and the authors' evidence
(`evidence/R6-gate.md`, `R6-app.md`, `R6-release.md`), not against live
results. Focus 5 ("accurate against R6-check evidence") is therefore
**NOT RUN** for the live part.

## Verification run by the reviewer

| Command (worktree root, Windows 11 x64, Rust 1.100.0) | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, test, required-cases, deny (no `url` exception needed), markdown-links |
| `cargo xtask verify --tier native` | NOT RUN (read-only review; the authors report PASS) |

## Summary

No critical or major findings. Four moderate and five minor findings follow.
None of them is a data-loss path. Release-blocking rule (packet): no critical
or major finding is open.

## Focus results

1. **Gate removal (Q-24, D-28): holds.** Every production
   `validate_variant` call goes through `SliceService::marker_policy()`
   (`service.rs:1644`, `service.rs:1663`), which reads `test-mode.txt` each
   time. Missing file means `NotRequired`; an unreadable file means
   `Required` (`control.rs:106-112`). No production path hard-codes a
   policy. `testkit` uses `NotRequired` explicitly.
   - Test mode enforces `Required` for preview, enroll, join and
     reconfirm (all go through `assess()`). It also applies to every cycle
     and every group command through `observe_world` → `rederive_variant`
     (`service.rs:1998-2004`). That path returns `TestModeBlocked` before
     anything is written (e2e `test_mode_persists_and_blocks_unmarked_installs_both_ways`).
   - Identity, product/folder, AddOns layout, duplicate-product and
     already-enrolled checks are unchanged. Only the marker check became
     conditional (`variant.rs:214`).
   - Unfamiliar clients still need guided identity, scope and digest
     (`consent_gap`/`check_consent` unchanged). They keep
     `validated-plain/1`.
2. **Forever toc rule: matches D-28 and domain.md §3.**
   - Tiers: `Forever`/`Camelot`, then `Mainline`, then plain.
   - Within a tier, disagreement on version, hard deps (as sets) or ids is
     `Ambiguous`. A read or parse error in any candidate of the deciding
     tier is `Error`, which fails closed (`toc.rs:200-245`).
   - Other suffixes are ignored, and a folder with only other suffixes is
     `None`.
   - Unfamiliar products stay plain (`variant.rs:312-314`; `profile` is set
     only for `KNOWN_PROFILES`).
   - All three non-test `select_toc` callers use the variant's mapping:
     `inventory.rs:189`, `variant.rs:380` (via `assess_layout`) and
     `settings_cycle.rs:467` (now passed `variant.mapping()`).
   - Reconfirmation is safe and visible. A stored `validated-plain/1` gives
     `StaleConfirmation` → `NeedsReconfirmation`, so syncing stops before
     any write. The status snapshot offers the reconfirmation, and the GUI
     shows "Review needed". Identity is required again. Nothing is lost
     (core test plus e2e `enrolled_forever_with_the_old_plain_mapping_is_reconfirmed_and_loses_nothing`).
3. **First run loses nothing: holds for every path I traced; the tests cover
   only part of it** (RV6-08).
   - A joiner's older add-on is replaced only through the apply path with a
     recovery slot. It is restored byte for byte through `Command::Rollback`
     (but see RV6-02).
   - A local-only add-on is kept and published.
   - Same label with different content stays `Ambiguous` on both sides.
   - A create changes nothing locally.
   - A joiner's copy of an add-on the group removed earlier is kept.
   - Settings: a non-master's differing live file is held as
     `LocalDifferent` until an explicit "Use shared settings", which
     captures it first (`settings_cycle.rs:641-666`).
   - I found no delete or overwrite without a recovery copy.
4. **User text: mostly clean.**
   - `refuse_text`, `layout_text` and `block_text` replace every `{:?}`
     that reached setup, and tests check every variant.
   - "Try again" appears only when `retryable` is true. Those cases need an
     outside fix (flavor/build info, product inactive, AddOns missing,
     duplicate folder), and trying again can then work.
   - The test-mode refusal offers "Turn off test mode" instead.
   - Remaining issues: internal mapping ids in the consent text (RV6-05),
     and the "Waiting" header while blocked (RV6-06).
5. **Release notes and docs:** RV6-01, RV6-02, RV6-03, RV6-04. The live part
   is NOT RUN (see Limit).
6. **E-06/E-08 hygiene: OK, with one evidence gap (RV6-09).**
   - `Cargo.lock` changes only `url` 2.5.8→2.5.9 and `form_urlencoded`
     1.2.2→1.2.3 (its transitive companion), plus the 0.2.0 version bumps.
     No new dependency.
   - `deny.toml` is unchanged, and `offline/deny` passes without an
     exception.
   - `required-cases.toml`: outside the R6 sections, only removed or renamed
     tests were changed. `toc::tests::equal_summary_suffixed_tocs_still_unvalidated`
     was dropped because its behaviour changed. The equal-summary case is
     now covered by `forever_profile_suffix_mixes_observed_on_the_dev_pc`
     (row 4) and the end of `forever_camelot_with_different_summaries_is_ambiguous`.
     No assertion was weakened beyond the D-28 behaviour change.

## Findings

| ID | Severity | Where | Finding | Suggested fix |
|---|---|---|---|---|
| RV6-01 | moderate | `docs/release/v0.2.0.md:21` | "Add-ons with the **same name** but different content are left untouched" is wrong. A same-name add-on with a newer version label is replaced (line 18 says so). Only the same **version label** with different content is left alone (Q-22, `Ambiguous`). Users could expect their differing copies to be safe when they are not. | Say "the same version number but different content". |
| RV6-02 | moderate | `docs/release/v0.2.0.md:18`; `crates/wow-sync-runtime/src/control.rs:431-434`; SPEC R-10 | The first-run recovery path is `Rollback`. Rollback **rejects the newer release for the whole group** and sends the old version to every computer. The notes present it as a local undo. A user who only wants their old copy back would downgrade every computer and block the release group-wide. The e2e test (`first_run_join_of_a_populated_install_loses_nothing`) does not assert the effect on the peer. This meets D-28 ("recoverable through the GUI"), but the notes undersell the consequence. | State the group-wide effect in the notes, and check that the GUI rollback confirmation says it. If a local-only restore is wanted, that is an owner question, not something to build silently. |
| RV6-03 | moderate | `docs/design/settings.md:164-167`; `docs/design/questions.md:137` (Q-68), `:146` (Q-77) | Q-68 says settings sync for a variant runs **only on synthetic data** until its WTF layout is checked on a real client, "the Q-24 gate". The gate is gone by default, so the code now syncs real `WTF` saved variables for `wowf` with that layout check still recorded as not done. D-28 removes Q-24 and keeps "all other safety rules", so the design text and the code disagree. | Record R6-check's real-client WTF layout observation for `wowf` and update Q-68/§1.5. Or, if D-28 is meant to cover it, have the orchestrator record that reading in Q-68/Q-77. Do not leave both statements standing at the tag. |
| RV6-04 | minor | `docs/design/questions.md:44` (Q-22), `:71`; `docs/design/apply.md:44`; `docs/ARCHITECTURE.md:133`; `crates/wow-sync-app/src/main.rs:4` | Stale statements remain. Q-22: "runs only on synthetic installs (Q-24), so real-data behaviour waits". D-28 accepted Q-22. Limitation row: "Folders with any suffixed toc are never transferred, for every client" (now unfamiliar clients only). apply.md: "synthetic install in the slice". ARCHITECTURE: "suffixed-toc folders blocked for every client … only on installs with the synthetic marker". main.rs: "Developer mode works on synthetic installs only". | Update each to the D-28/`profile:wowf/2` wording. |
| RV6-05 | minor | `crates/wow-sync-core/src/variant.rs:645`, `:651` (shown at `crates/wow-sync-app/src/setup.rs:380` and in reconfirmation) | The consent text starts "Folder rule profile:wowf/2: …" or "Folder rule validated-plain/1: …". These internal mapping ids are user-visible. The pattern predates run-006, but run-006 rewrote this text. | Use "Folder rule for {label}:" (or omit the id). Keep `mapping_id` in diagnostics only. Update `profile_disclosure_names_the_suffix_order`. |
| RV6-06 | minor | `crates/wow-sync-runtime/src/service.rs:2613-2621`; `crates/wow-sync-app/src/world.rs:482`; `crates/wow-sync-app/src/model.rs:192`, `:600` | The R6-app note is confirmed. The `TestModeBlocked` snapshot is `Default` apart from the error, with no detail and no group, computer or identity name. The app uses `placeholder_view()`, so the headline is "Waiting" with blank names, while the banner and Needs-attention say test mode is blocking. There is no data risk and the banner explains it, but the header contradicts it. Also, `blocked_by_test_mode` matches on the Display prefix "Test mode is on", which is brittle. | In that branch, fill `group_name`, `computer_name` and `identity_line` from the local enrollment. Add a typed `StatusSnapshot` flag (or set the view headline to `NeedsHelp`) so `headline()` reads "Paused by test mode". Test it. |
| RV6-07 | minor | `crates/wow-sync-app/src/backend.rs:337-348`; `crates/wow-sync-app/src/app.rs:703-707`, `:920-924` | Test-mode command failures are silent. Errors from `command` and from `SliceService::new` are dropped. The banner action still shows "Test mode is off", and setup still re-inspects. If removing `test-mode.txt` fails (or, before enrollment, the service cannot be built), the user sees the same refusal with no reason. Enrolled state corrects itself on refresh, but the notice is wrong. | Return the outcome to the UI, and show a plain error when it fails. |
| RV6-08 | minor | `tests/required-cases.toml` `R6-gate:first-run-loses-nothing`; `crates/wow-sync-runtime/src/e2e.rs:750-860` | The first-run case covers add-on code only. Two paths are not in it. (1) Settings first adoption: the existing `e2e::tests::use_shared_settings_replaces_a_local_difference_and_keeps_it_as_a_recovery_copy` (`e2e.rs:3656`) covers it, but it is not linked. (2) A joiner that holds a release the group already rejected: by code reading this uses the same apply path with a recovery slot, but there is no test. | Add (1) to the case. Add a small e2e test for (2). |
| RV6-09 | minor | `docs/runs/run-006/evidence/R6-release.md` | E-08: no exact commands and no PASS/FAIL table with the environment for the 0.2.0 bump and the notes commits. Only goal 0 says "both tiers passed". | Add the commands and results for each commit, or one final run after the last commit. |

## Notes (no finding)

- `LayoutReason::NotSyntheticInstall` is mapped in `service.rs`, but core
  never produces it now (the marker is checked in `validate_variant`). It is
  harmless and is kept for the API.
- A 0.1.0 peer still publishing `validated-plain/1` is a
  `DifferentTocMapping` holder until it upgrades (R6-gate evidence). A
  one-line "upgrade every computer" in the release notes would help. On real
  installs a 0.1.0 build cannot enroll anyway.
- The consent text names only the `_` separator, although `-` also matches.
  The wording is acceptable.
- Turning test mode **on** pauses an enrolled real install at once, without
  confirmation. This is reversible and explained in Settings, so it is
  acceptable.

## Links

SPEC R-02, R-03, R-10; ACCEPTANCE A-02, A-38, A-46; E-06, E-08;
OWNER-DECISIONS D-14, D-28 (Q-3, Q-22, Q-24).
