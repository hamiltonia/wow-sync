# R5-review independent integration review (run-005, phase C)

Reviewer: R5-review (claude-opus-5.5). Worktree
`D:\GitHub\wow-sync.worktrees\R5-review`, branch `run005/R5-review`, reviewed
tree `main` at `f478fca` (later `main` `8f17099` changes only HANDOFF/USAGE).
Windows 11 x64 dev PC, Rust 1.100.0. Synthetic data only; no real WoW,
Battle.net, Dropbox or `D:\WoWSyncDemo` data touched. No product code, test or
doc changed except this file. I did not write the code under review.

## Verdicts

| Area | Verdict | Summary |
|---|---|---|
| 1. E-06 security and data | PASS with CONCERN | §7.4 restore input is validated: tree records are name-hash checked, folder names and file paths are validated at decode, staged files are hash-verified, and the plan matches the removal manifest and `removed_hash`. A corrupted shared blob installs nothing (confirmed by a temporary probe), but the failed restore stays queued and looks pending forever (RV5-01). Release workflow, pins, update-check 404 and diagnostics are sound. Dependencies match `docs\DEPENDENCIES.md`. |
| 2. Product safety rules | PASS with CONCERN | Uninstall moves code and owned settings, and restore returns both (e2e round trip, with A-38 controls). Q-88 (a), DataAtRisk-ignores-toggles, rejected-version refusal, WoW-running block and offline-peer non-resurrection are covered by asserting tests. The CONCERN is RV5-01 (status honesty after a failed restore) and a test gap for multi-identity capture (RV5-07). |
| 3. E-08 repository | CONCERN | README and REPO-MAP still name run-004 as current (RV5-03). Design docs still say Q-77 is PENDING OWNER (RV5-04). `tests\required-cases.toml` has no drift: the verifier matched 133 offline cases against 757 test results. |
| 4. Test honesty | PASS | Eight run-005 required tests sampled across core, runtime and app. Each asserts the observable outcome and has a negative case. Two of my own A-38 controls on §7.4 (not used by R5-check) failed with exit 101 and were reverted. |
| 5. Release readiness | NOT YET (conditional) | No blocker in the code reviewed, but the evidence gates for the tag are not met on the current tree. See §5. |
| 6. Open items | listed | See §6. |

## Findings

| ID | Severity | File:line | Impact | Smallest fix |
|---|---|---|---|---|
| RV5-01 | Moderate | `crates\wow-sync-runtime\src\service.rs:2657-2680`; `crates\wow-sync-runtime\src\restore_flow.rs:209-240`, `374-400` | If the shared blobs pass the plan-time "present" check but fail verification at staging (corrupt or substituted content), the executor safely installs nothing and leaves 0 journals. But: (a) the cycle note says "Settings are restored with the code" because it is set when the restore is planned, before it runs; (b) the request stays queued, so every cycle re-downloads and fails again; (c) the status shows "Restoring alpha: it will run on the next sync." with the headline "This computer is up to date" indefinitely. The only failure signal is the per-cycle `AppliedLine` "failed - the downloaded copy of Alpha did not verify; nothing was changed", which reaches the user only as a Summary notice when that toggle is on. Reproduced with a temporary e2e probe (§3, probe 1). | Set the settings note only after the restore operation finalizes. When an `OpKind::Restore` package fails in `run_apply`, turn the request into a `RestoreIssue` with the executor's reason, so it shows as Unavailable with Try again / Dismiss. Add an e2e case with a present but corrupted blob. |
| RV5-02 | Low | `crates\wow-sync-runtime\src\restore_flow.rs:318-334`; `crates\wow-sync-runtime\src\effects.rs:483-489` | The fallback to a local recovery point happens only when a blob is `NotFound`. A present but corrupted shared blob is not retried from a valid local copy, so the restore fails (safely) even though this computer holds the exact release. | In `stage_folder`, also fall back to `stage_from_recovery` when the provider copy fails with `HashMismatch`. |
| RV5-03 | Low (E-08) | `README.md:3-10`; `docs\REPO-MAP.md:10`, `37-46` | A new agent following AGENTS.md → README is sent to run-004 REPORT/HANDOFF as the current state. Run-005 is active and the next step is a release tag. README:5 still says settings sync is disabled "until Q-77", which D-26 decided. | Point README and REPO-MAP at `docs/runs/run-005/HANDOFF.md`. State that Q-77 = 32 MiB (D-26) and that the Q-24 gate still applies. |
| RV5-04 | Low (E-08) | `docs\design\settings.md:104`, `109`, `992`; `docs\design\contracts.md:1223` | These still say Q-77 is PENDING OWNER and that settings sync is blocked, which contradicts D-26 and `questions.md:146`. | Replace with "Q-77 owner-decided (D-26): 32 MiB". |
| RV5-05 | Low | `docs\release\v0.1.0.md` (Known gaps) | The 32 MiB gap line covers sync only. It does not say that an uninstall leaves an over-cap settings file in place, so that file is not removed with the code and is not part of the recovery copy (`e2e::tests::a_settings_file_over_the_cap_is_not_moved_by_an_uninstall`). | Add one sentence to that known-gap line. |
| RV5-06 | Info | `.github\workflows\release.yml:76`, `108` | The comments say the artifact action SHAs must be verified before the first tag. I verified them read-only: `gh api` tag refs give upload-artifact v4.6.2 = `ea165f8…`, download-artifact v4.3.0 = `d3f86a1…` and checkout v7.0.1 = `3d3c42e…`, all matching the pins. | Drop or update the TODO comments. |
| RV5-07 | Low (test gap) | `crates\wow-sync-core\src\settings_tests.rs:252`; `crates\wow-sync-runtime\src\e2e.rs` uninstall cases | The uninstall capture and restore tests use one included account and no per-character file. The identity boundary rests on the generic `account_unit_never_matches_another_identity` (settings_tests.rs:885) and the included-account scope test (:134). No run-005 test shows that an uninstall leaves another account's or another identity's file alone, and restores each unit only to its own identity. | Add one e2e case with two accounts (one excluded) plus a character-scoped file across uninstall and restore. |

No Critical or Major findings. R5-check F1 is fixed. `e2e::tests::an_uninstall_after_a_group_restore_converges_on_every_computer` is required and passes, but the GUI re-observation is R5-check's parallel recheck and is not credited here. R5-check F2 is fixed: the notes now describe the 404 as "No release has been published yet." and match `updates.rs:33` and `updates.rs:407`.

## 1. E-06 security and data

**§7.4 restore (new trust boundary), read in full:** `restore_flow.rs`, core
`removal.rs:733-860`, `effects.rs:185-263`, `480-504`, `service.rs:913-955`,
`2632-2683`.

- **Hash checks.**
  - The tree record is read with a 32 MiB limit and accepted only by
    `decode_tree`. That checks that the files are sorted and unique, that
    `tree_hash(files) == tree` (`records.rs:301-312`), and that the tree
    equals the name hash (`records.rs:315-323`).
  - The local source (`local_release`) is accepted only if the recomputed
    tree hash equals the manifest tree (`effects.rs:255`).
  - Every staged file goes through `stage_file(…, sha256, size)`.
  - `plan_restore` requires each folder to equal a manifest entry
    (key + name + tree), and `package_hash == removed_hash`
    (`removal.rs:797-816`).

  The last check is a consistency check of the record itself; content
  integrity comes from the tree and file hashes. Probe 1 confirmed that a
  corrupted present blob is rejected at staging.
- **No partial install.**
  - Install goes through the journaled executor. A missing blob is refused
    at command time and nothing is queued
    (`a_restore_with_a_missing_blob_is_unavailable_and_leaves_nothing_partial`).
  - A corrupt blob is rejected at staging with no folder and 0 journals
    (probe 1).
  - The `user_restore` re-add is a pending record of the operation, written
    only at finalize (`removal.rs:818-858`).
- **Path safety.**
  - Folder names in `Removal.folders` pass `manifest_ok`, which requires
    `validate_folder_name(name) == key` (`kinds.rs:120-127`, applied by
    `validate_removal`, `kinds.rs:497-499`).
  - File paths are `RelPath`, and their `Deserialize` runs
    `validate_rel_path` (`paths.rs:101-106`).
  - The recovery root is opened with `ReparsePolicy::RejectAll`
    (`effects.rs:188`, `229`).
- **Rejected-version refusal.** An active rejection naming any (key, tree) of
  the manifest refuses, both at the command and again at cycle time
  (`restore_flow.rs:286-298`). My A-38 control 1 is below.
- **WoW-running block.** The cycle plans the restore only when
  `standing == Active && !game_blocks` (`service.rs:2638-2641`), and the
  executor has its own gate.
  `a_restore_waits_while_wow_runs_and_installs_when_it_closes` asserts that
  nothing is installed while WoW runs and the code is installed after it
  closes.
- **Offline peers.** The re-add's `known` summary covers only removals this
  ledger has learned, so a removal written later or unseen still wins
  (`removal_tests::restore_readd_covers_exactly_what_was_seen_and_a_later_removal_still_wins`,
  `an_offline_peer_that_still_owes_cannot_resurrect_a_removed_package`,
  e2e `a_removal_published_by_a_peer_is_applied_and_an_offline_peer_cannot_resurrect_it`,
  `a_rollback_installs_the_recovered_release_and_an_offline_peer_cannot_bring_the_rejected_one_back`).
  A restore of a rejected tree is refused (above).
- **Gap:** RV5-01 and RV5-02.

**Other run-005 boundaries:**

- **Uninstall settings capture.**
  - `removal_targets` takes only unshared, unpaused, owned, under-cap files of
    the removed folders (`settings_tests.rs:252-298`, with negatives for
    Keep, Paused, SharedFile and TooLarge).
  - The R5-check controls cover the round trip and the exact-cap boundary.
- **Q-77 cap and Q-24 gate.**
  - `the_approved_cap_is_32_mib_and_a_file_at_the_cap_syncs_but_one_byte_more_does_not`
    and `without_a_cap_value_sync_stays_disabled` are required.
  - R5-check exercised actual 33,554,432- and 33,554,433-byte files.
  - The marker gate is unchanged, and the release notes state it.
- **DataAtRisk.**
  `model::tests::data_at_risk_shows_in_headline_tray_and_needs_attention_whatever_the_toggles_say`
  sets all three toggles off. A refused restore is explicitly not Risk
  (`model.rs:1409`).
- **Diagnostics.**
  - `fingerprints_from` hashes only the folder keys and tree hashes of the
    inventory (`diag.rs:169-188`).
  - `build_commit` is `option_env!("WOWSYNC_BUILD_COMMIT")` or `dev`
    (`diag.rs:159-165`, `280`).
  - No path or SID input is added in run-005. Computer display names are the
    pre-existing, owner-chosen names.
- **Update 404.** `FetchError::Http(404)` maps to `NoRelease`
  (`updates.rs:33`). The client follows https redirects only, at most 3, with
  a 10 s connect / 20 s total timeout and a size-capped body
  (`update_http.rs:16-19`, `167-213`).
- **Release workflow.**
  - Top-level `permissions: contents: read`. Only `publish` has
    `contents: write`.
  - `publish` runs only for `push` of a `refs/tags/v*` ref. Its `if:` means
    `workflow_dispatch` never publishes.
  - Checkouts use `persist-credentials: false`.
  - All three actions are SHA-pinned and verified (RV5-06). cargo-deny is
    pinned `--locked`.
  - The only secret is `GITHUB_TOKEN`, used in the publish step.
  - The tag must equal `v` + workspace version, and `docs/release/$TAG.md`
    must exist. `gh release create --verify-tag --prerelease`.
  - The zip holds `wow-sync.exe`, README.md, RELEASE-NOTES.md and LICENSE*.
    R5-check observed the four files in dispatch run 44945165878.
- **Dependencies.** `cargo metadata --offline --locked --no-deps` lists the
  same direct external crates and pins as `docs\DEPENDENCIES.md` (adapters:
  getrandom, native-tls, notify-rust, sha2, ureq, windows-sys; app: gpui,
  gpui_platform, png, raw-window-handle, tray-icon, embed-resource; core:
  serde, serde_json, sha2, unicode-normalization; runtime: serde,
  serde_json; xtask: serde, serde_json, toml). `cargo deny` passes in the
  offline tier.

## 2. Product safety rules

These are judged from code and tests (see §1 and §4):

- **Uninstall removes code and settings, and recovery restores both.**
  `an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both`
  is now a real round trip. Code comes from the provider manifest, because
  the originating computer holds no recovery point. Settings come back byte
  for byte per computer.
- **Q-88 (a).**
  `a_newer_local_only_version_after_a_keep_stays_installed_unshared_and_is_asked_every_cycle`
  (R5-check A-38 control).
- **Variant and account boundaries.** Capture uses `settings_scope`, which is
  restricted to included accounts. Units are identity-keyed and restore is
  same-unit (removal.md §6.2 rule 6). The run-005-specific multi-identity
  test is missing (RV5-07).
- **DataAtRisk regardless of toggles.** Confirmed in §1.
- **Restore of a removal while the folder is installed here.** One case
  replaces it: a folder of the removed package that is still installed here
  at another tree (`live: Some`, `FolderAction::Replace`). This is the
  owner's explicit restore. The executor keeps the prior live tree as a
  recovery point. This matches §7.4 and is not a Q-88 violation, because no
  automatic choice is made.

## 3. Temporary probes and A-38 controls (all reverted)

Each mutation was made with the edit tool and run in a separate call, then
reverted. `git diff --exit-code` returned 0 after each revert and before the
commit.

| # | Temporary change | Command | Result |
|---|---|---|---|
| Probe 1 | Added temporary test `rv5_tmp_corrupt_blob_restore` to `e2e.rs`. It flips the first byte of every removed blob in the lab provider, queues `RestoreRemoved`, and runs 4 cycles. | `cargo test -p wow-sync-runtime --lib rv5_tmp_corrupt_blob_restore -- --nocapture` | Passed (1 test): `Alpha` absent and 0 journals, so nothing partial. Each cycle: `applied = "failed - the downloaded copy of Alpha did not verify; nothing was changed"`, `apply_note = "Settings are restored with the code"`, `restores = [Waiting{NextCycle} "Restoring alpha: it will run on the next sync."]`, headline "This computer is up to date", `blocks = []`. This is RV5-01. |
| A-38 control 1 | `restore_flow.rs` `if rejected {` → `if rejected && false {` | `cargo test --locked -p wow-sync-runtime --lib a_restore_of_a_rejected_version_is_refused` | FAILED at `e2e.rs:2803`, exit 101. Reverted, diff clean. |
| A-38 control 2 | core `removal.rs` `if package_hash(&pairs) != removal.removed_hash {` → `… && false {` | `cargo test --locked -p wow-sync-core --lib a_restore_whose_package_hash_is_not_the_removed_hash_is_refused` | FAILED at `removal_tests.rs:466`, exit 101. Reverted, diff clean. |

R5-check's controls targeted the restore-ops guard, the cap, Q-88,
DataAtRisk, Keep reason, cleanup ops and the exact-size boundary. Neither of
mine overlaps them. R5-remove's own control 1 also targeted the
`removed_hash` comparison, from the opposite side (`if false && …`).

## 4. Test honesty (sample)

| Test | Crate | Observable asserted | Negative |
|---|---|---|---|
| `a_restore_whose_package_hash_is_not_the_removed_hash_is_refused` | core | `HashMismatch` and `ManifestMismatch` errors | substituted tree is refused (control 2) |
| `an_uninstall_takes_only_unshared_unpaused_owned_files_of_the_removed_folders` | core | exactly `Gone` is taken | Paused, SharedFile, TooLarge and untouched Keep |
| `a_restore_of_a_rejected_version_is_refused` | runtime | `Service("…rejected…")`, no code installed | control 1 |
| `a_restore_with_a_missing_blob_is_unavailable_and_leaves_nothing_partial` | runtime | error, no folder, 0 journals | (it is the negative) |
| `a_restore_waits_while_wow_runs_and_installs_when_it_closes` | runtime | absent while running, present after | running state |
| `an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both` | runtime | code and settings gone on both peers, then back byte for byte | the peer keeps its own capture; no second removal |
| `a_refused_restore_is_a_needs_item_and_a_waiting_one_is_only_a_status_line` | app | Needs item text, tone and target | waiting is not a need, refusal is not Risk |
| `data_at_risk_shows_in_headline_tray_and_needs_attention_whatever_the_toggles_say` | app | shown with all toggles off | baseline without risk is empty |

Observation: `a_restore_with_a_missing_blob_…` fails at command time, so the
executor's staging-failure path for restore is not covered by any required
test (RV5-01 fix).

Evidence claims: R5-remove round 3/4 and R5-app restore sections match the
code I read. R5-remove's "not done" items (a) to (d) are superseded by round
4 and the R5-app restore commits. R5-app.md:18 "Q-77 banner: NOT DONE" is
superseded later in the same file (:37). R5-check honestly marks its GUI gaps
BLOCKED.

## 5. Release readiness for `v0.1.0`

**Verdict: NOT YET. Tag only after the gates below.** Nothing in the
reviewed code is a release blocker for a synthetic-only, marker-gated
pre-release, but:

1. **CI.** No CI run has passed on any commit containing the §7.4 restore.
   The runs on `9d3a144`, `4da4477`, `debc58f` and `f478fca` were cancelled
   by later pushes. The run on current `main` `8f17099` (44952756402) was
   in progress at review time: macOS offline and the four build-only jobs
   succeeded, Windows offline was pending. The last green CI is `dccdf9e`,
   which predates §7.4.
2. **Artifact smoke test (PLAN §3/§5).** R5-check's pass was on dispatch run
   44945165878 at `4675f08`, which predates F1, F2 and §7.4. The tag builds
   current `main`, so a fresh `workflow_dispatch` on the commit to be tagged,
   with a smoke test, is needed.
3. **GUI recheck.** The R5-check GUI recheck (F1 and the new restore UI) is
   running in parallel and is not credited here.
4. **RV5-01.** Either fix it, or add one known-gap line ("if the shared copy
   is damaged, a restore stays pending and retries; check the activity
   summary"). I recommend the fix, because the status is misleading but no
   data is harmed.

**Release notes (`docs\release\v0.1.0.md`).**
- No false claim found. "Removal that propagates and is recoverable,
  including the add-on's saved settings" is now backed by the §7.4 round
  trip.
- The 404 text matches the code (F2 fixed).
- The memory figure is labelled run-004. R5-check re-measured 37.94 MiB in
  run-005.
- Missing: RV5-05, and optionally the RV5-01 caveat.
- Stated gaps: Q-24 gate, 32 MiB cap, latest-release/pre-release update
  behaviour, released-version dates, diagnostics location, not built
  (installer, signing, telemetry, real data, macOS runtime), macOS owner run.

## 6. Open items

**PENDING OWNER**
- macOS kit run of `docs\runs\run-004\MAC-OWNER.md` (HANDOFF owner barrier,
  D-26).
- `v0.1.0` tag push, after the gates in §5 (PLAN §3/§5, HANDOFF).
- R5-app open point: should Resume lift specific rejections? That needs
  snapshot data (R5-app.md:11-12). Not a new design question unless the
  orchestrator raises it.
- No new product question from this review.

**DEFERRED**
- D-11 telemetry provider and schema (OWNER-DECISIONS "Deferred").
- DA-2 to DA-4 (OWNER-DECISIONS:253, D-23).
- A-31 numeric budgets beyond memory and start time (OWNER-DECISIONS:260,
  D-23).
- Q-68 real-client WTF layout checks (questions.md:137, D-23).
- Real-data enrollment (Q-24 gate; release notes).

**BLOCKED observations carried from R5-check**
- Q-77 over-32-MiB GUI window.
- DataAtRisk native tray tooltip.
- GUI HTTP-404 About state.

## Exactly what I ran

All commands were run from `D:\GitHub\wow-sync.worktrees\R5-review` with
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

- `cargo xtask verify --tier offline` (first run): PASS. It overlapped
  probe 1's temporary edit, so I discarded it and reran it on the clean tree
  below.
- `cargo xtask verify --tier native`: PASS (`native/tier`), clean tree at
  `f478fca`.
- `cargo xtask verify --tier offline`: PASS, clean tree
  (`git_dirty: false`, `f478fca`). Checks: fmt, clippy, test, required-cases
  ("133 listed offline case(s) verified against 757 test result(s)"), deny,
  and markdown-links (209 files).
- Probe 1 and A-38 controls 1 and 2, as in §3.
- `cargo metadata --offline --locked --format-version 1 --no-deps` (dependency
  cross-check).
- `gh api repos/actions/{upload-artifact,download-artifact,checkout}/git/ref/tags/…`
  (pin check).
- `gh run list` and `gh run view 44952756402` (CI state).
- `gh release list` (no release exists).
- `git fetch`, `git log` and `git diff` (read-only).

Not run: GUI (R5-check owns it), release dispatch, and anything on macOS.
