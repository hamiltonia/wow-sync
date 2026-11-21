# R4-review: independent integration review (phase E)

Source: `9c64f47` (main), branch `run004/R4-review`, checkout
`D:\GitHub\wow-sync.worktrees\R4-review`. Reviewer role R4-review
(integration review, assigned opus-5.5). The assignment supplies the model;
this worker cannot introspect its runtime model or meter, so the orchestrator
must attach both. No nested agents. Windows 11 x64 dev PC, Rust 1.100.0 (pinned).
No product, test, tool or other doc change committed. Only this file is written.

Authority read: AGENTS, COMMON, R4-review packet, R4-check (sections 1–6,
Recheck, Recheck 2), USAGE, PLAN head, questions.md Q-77/Q-88, ops.md §1.5 and §5,
and the deferred sections of R4-core-*, R4-integrate and R4-notify-diag evidence.
I relied on code and on tests I ran myself, not on author or verifier prose.

## Verdicts

| Area | Verdict | Summary |
|---|---|---|
| 1 E-06 security and data | **CONCERN** | Provider/recovery input is bounded and validated, no Lua executed, update check is locked down, mac kit guarded, deps pinned and `deny` passes. Gaps: Save rebuilds instead of saving the previewed bytes (RV-01); the redaction context omits WTF/computer/Dropbox names (RV-02); the opt-in debug log records account names and paths, and MAC-OWNER asks for it in reports (RV-03). |
| 2 Product safety rules | **CONCERN** | Variant, retired-publish, no-silent-master, group rejection/offline resurrection, game-running (two layers) and three-state retention hold in code and tests. Uninstall does **not** capture or remove settings at all (RV-04). This is unbuilt, not only blocked by Q-77. The same-version rollback confirmation omits the group block (RV-05). |
| 3 E-08 repository | **FAIL** | README and REPO-MAP still point a new agent at run-002/run-001 as current state; USAGE is behind the last fix rounds and omits Q-88 (RV-06, RV-07). The required-case inventory matches real test names (mechanically checked, PASS). |
| 4 Test honesty | **PASS with one concern** | Ten required cases sampled. Three reviewer A-38 controls fail their named tests with exit 101. One new required case is near-trivial (RV-08). |
| 5 Open items | listed below | — |

## Commands run (exactly)

All commands ran from the checkout above, after
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` | **PASS**, exit 0. fmt, clippy, test, required-cases, deny and markdown-links all PASS. |
| `cargo xtask verify --tier native` | **PASS**, exit 0 |
| `git diff --exit-code` after each control revert | exit 0, three times |

Disclosure: TEMP/TMP were not redirected. `cargo deny check` ran without
`--offline`, so it may have refreshed its advisory DB over the network.
This is the same procedural gap as R4-check V-12. No GitHub API, `gh`, live
update GET, GUI launch, real WoW/Battle.net/Dropbox data or `D:\WoWSyncDemo`
was used. No process was started apart from cargo, which exited by itself.

### Reviewer A-38 controls (my choice; none was used by R4-check)

Each mutation was made with the edit tool and reverted with the edit tool,
then checked with `git diff --exit-code` (exit 0). Nothing was committed.

| # | Rule | Temporary mutation | Test command | Result |
|---|---|---|---|---|
| R1 | three states (current + 2 previous) | `core/src/exec.rs:914` `KEEP_RECOVERY` 2 → 3 | `cargo test -q -p wow-sync-core --lib -- --exact exec::tests::recovery_points_pruned_to_two_per_package exec::tests::pinned_recovery_points_exceed_two_visibly` | both FAILED (3≠2, 4≠3), **exit 101** |
| R2 | WoW running blocks game-file changes | `core/src/plan.rs:318` `Running => return Err(GameRunning)` → `Running => {}` | `cargo test -q -p wow-sync-core --lib -- --exact plan::tests::game_running_blocks_apply_plan` | FAILED, **exit 101** |
| R2b | same mutation, defence in depth | (same) | `cargo test -q -p wow-sync-runtime --lib -- --exact e2e::tests::game_running_on_target_blocks_apply_then_resumes` | passed, exit 0. This is expected: `exec.rs:335` `gate()` re-checks activity before any change. It is an honest second layer, not a weak test. |
| R3 | V-02 round-4 fix (rollback covers a live keep) | `runtime/src/rollback_flow.rs` `known.extra.extend(choices.extra);` → `let _ = choices;` | `cargo test -q -p wow-sync-runtime --lib -- --exact e2e::tests::restoring_a_copy_a_keep_displaced_converges_the_group_on_it` | FAILED at `e2e.rs:3654` ("different computers recorded different choices"), **exit 101**. This fix landed after Recheck 2 and has no GUI recheck. |

## Findings

| ID | Severity | File:line | Impact | Smallest fix |
|---|---|---|---|---|
| RV-01 | major | `runtime/src/service.rs:633-635`; `app/src/about_ui.rs:147` | Save sends `SaveDiagnostics{options,dest}` and the runtime **rebuilds** the export from live state. The saved bytes can differ from the previewed bytes, for example after a new activity row or a changed report age. That breaks ops.md §5.1 step 3 ("the exact bytes that will be saved") and T-33-10. The app test comment "a save is only of what was shown" is untrue end to end. Redaction still runs on the rebuilt text, so this is a consent/accuracy defect, not a leak. | Make the save command carry the previewed `Export` (or its hash) and write exactly that text. Refuse if it is stale. Add a byte-compare test. |
| RV-02 | minor | `runtime/src/service.rs:609` (`..Default::default()`) | The second-defence scan never learns WTF account/realm/character names, the OS computer name or the Dropbox root. ops.md §5.3 requires them. Typed fields keep the primary defence intact, so there is no observed leak. The names are already available: `settings.available_accounts`. | Fill `wtf_account_names` from `available_accounts`, `computer_names` from the host name and `dropbox_root` from discovery. Add a planted-account test through `input_from`. |
| RV-03 | minor | `app/src/app.rs:638`; `docs/runs/run-004/MAC-OWNER.md:157` | `log("act {act:?}")` writes `GroupAction` Debug, which includes `EnableSettings.included_accounts` and `SaveDiagnostics.dest` (a home path, so a user name). The log is opt-in (`WOWSYNC_LOG`), but MAC-OWNER tells the owner to copy `app.log` into a report. | Log `a.kind()` for `Act::Group`, never its payload. |
| RV-04 | major | `core/src/removal.rs:969` (`settings: RemovalSettings::None`); no planner emits `OpKind::SettingsCleanup` | Removals never capture or remove saved settings. The AGENTS/SPEC rule "uninstall removes active code and settings; recovery preserves and restores both" is **unbuilt**. R4-check lists the settings half only as "BLOCKED (Q-77)"; an approved cap would not make it work. R4-integrate does record it as DEFERRED. Nothing is lost today, because settings are never touched. | Record it as DEFERRED (unbuilt) in the run report, separately from Q-77. A later packet builds the removal-capture and `SettingsCleanup` planner. |
| RV-05 | minor | `app/src/rollback_ui.rs:183` `same_version_text` | When the installed and earlier copies share a version number, the runtime still writes a group rejection of the current copy. `e2e.rs:3648` asserts "rejection written". The confirmation says only that the current copy "is kept for recovery". It drops the "blocked for the group … allow it again later" sentence that the other branch shows. That consequence is hidden before consent. | Add the blocked sentence (for example "The copy installed now is blocked for the group. You can allow it again later.") and assert it in the existing test. |
| RV-06 | major (E-08) | `README.md:3-11, 98-101`; `docs/REPO-MAP.md:35-43` | A new agent following AGENTS → README is sent to `run-002/HANDOFF.md` as "Current state". README also says "No product features … beyond the approved run-001 scope". REPO-MAP lists only run-001 run state. None of them mention run-003/run-004, the D-24 approval or the open owner barriers. | When the run report lands, point README "Current state" at the run-004 report or handoff and add a generic "latest run" pointer in REPO-MAP. |
| RV-07 | minor (E-08) | `docs/runs/run-004/USAGE.md:55,57` | The ledger stops at "R4-check evidence merged; fix round sent". There are no rows for Recheck, Recheck 2, round 4 (V-02 convergence) or this review. The closing sentence lists only Q-77 and macOS as barriers and omits Q-88 and the first GitHub release. | The orchestrator appends the missing rows and adds Q-88 and first release to the barrier sentence. |
| RV-08 | minor (test honesty) | `runtime/src/e2e.rs:3666` | `a_rollback_of_a_copy_the_group_never_displaced_is_still_refused` uses a never-existing hash `[7;32]`, which `UnknownCopy` refused before the round-4 change. It does not exercise the new scope widening, for example a real held release of another add-on, or a release outside the choice scope. | Use a real but unrelated release, or one from another package, and assert refusal and no rejection file. |
| RV-09 | minor | `runtime/src/diag.rs:236` (`fingerprint: None`); `diag.rs:5` build commit `dev` | "Include file fingerprints" is offered but never populated, and release builds report `build_commit: dev` unless `WOWSYNC_BUILD_COMMIT` is set. This is the V-06 residual, still open. | Populate from inventory fingerprints; set the env in CI release builds. |
| RV-10 | minor (E-06 record) | `R4-notify-diag.md` "Dependencies (E-06)"; `R4-core-contracts.md:50` | The justifications for `notify-rust`, `ureq`, `native-tls` and `unicode-normalization` give the purpose but not the licence or maintenance status COMMON requires. `Cargo.lock` was hand-edited for `unicode-normalization`. The verify tiers run clippy/test without `--locked`; only CI build jobs pass `--locked`. Versions are exact-pinned and `deny` passes. | Add licence/maintenance lines. Add `--locked` to xtask clippy/test. |
| RV-11 | info | `app/src/platform.rs:42,62`; `adapters/src/sysinfo.rs:36` | macOS helpers run `scutil`, `hostname`, `date` and `sw_vers` by PATH lookup. This is low risk for a per-user desktop app. | Use absolute `/usr/sbin/scutil`, `/bin/hostname`, `/bin/date`, `/usr/bin/sw_vers`. |

No blocker found. No product fixes made.

## Area 1 detail (E-06)

- **Local recovery restore:** `runtime/src/effects.rs` `local_points` opens
  `.wowsync` with `ReparsePolicy::RejectAll` and validates package/folder names
  with `paths::validate_folder_name`. It re-hashes each held slot and requires
  `tree_hash == recorded tree`. Partial or mismatched points are skipped, and the
  TOC read is capped at 256 KiB. Malicious metadata cannot redirect a restore. PASS.
- **Provider records** (rollback/removal/settings/recovery): `core/src/kinds.rs:13-25`
  caps record (64 KiB), index, folders, parts and labels.
  The rollback request and outcome are local state read with a 64 KiB cap
  (`rollback_flow.rs:83,98`). An unparseable request is ignored, and a refused one is
  consumed with a visible outcome. PASS.
- **Settings consent/accounts:** `settings_cycle.rs:325` lists only real,
  non-reparse dirs under the local variant's `WTF\Account`. `e2e.rs:3041` covers
  missing WTF, a stray file, another computer and a retired computer. PASS (RV-03 aside).
- **Diagnostics:** typed allow-list plus the fail-closed scan (R4-check control 5).
  The destination guard canonicalises and refuses shared/install roots; the write
  is temp + rename. RV-01 and RV-02 apply.
- **Update check:** `adapters/src/update_http.rs` fixes one URL, https only,
  two hosts, 3 redirects, 256 KiB, 10/20 s timeouts and an OS trust store. Core reads
  only `tag_name/draft/prerelease`; no URL from the response is opened. PASS.
- **Notifications:** typed planner; no path data. Not driven visually here.
- **macOS kit:** `lib.sh` rejects relative, `/`, Dropbox/WoW/Battle.net/Library
  lab roots. `rm -rf` only runs on marker-bearing lab dirs or a bundle whose plist
  carries `com.wowsync.app`, and processes are stopped by captured PID. PASS.
- **Lua:** no executor; addon/TOC content is only parsed as data. Repo grep found no
  personal profile path, user name or real SID; planted strings are synthetic.

## Area 2 detail (product rules, code + tests)

| Rule | Evidence | Verdict |
|---|---|---|
| Variant boundary | `target::tests::other_variant_state_ignored` (R4-check control 7); accounts listed only from the local variant | PASS |
| Retired cannot publish | `service.rs::publish_step` guard (R4-check control 8) | PASS |
| No silent master election | `retire_tests.rs:93` (`reassign_master == None`); `settings_ui.rs:861` | PASS |
| Rollback group-wide; offline cannot resurrect | `reject.rs` (R4-check controls 3, 9); R3 above for the keep-covered case | PASS |
| WoW running blocks changes | plan gate (R2) and exec gate `exec.rs:335` (R2b); removal/settings/rollback also gated by `game_blocks` (`service.rs` cycle) | PASS |
| Three states | `KEEP_RECOVERY = 2` + current; `KEEP_OWN_STATES = 3`; R1 | PASS |
| Uninstall removes code and settings, with recovery | code half PASS (R4-check A-10 GUI); settings half unbuilt (RV-04) | CONCERN |

**Known accepted gaps: are they acceptable?**

- **Peer-only held release has no date:** acceptable. No shared record
  carries a hold date. Inventing one would be worse, and local points are
  dated. Keep it DEFERRED until a record field exists.
- **Diagnostics save without a picker:** acceptable as an interim safety-wise,
  because the destination refusal works (R4-check). It is still a deviation from ops.md §5.1
  step 5 (OS dialog, timestamped name). The fixed name silently overwrites the previous
  export. Keep it DEFERRED. RV-01 is the more important diagnostics defect.
- **Keep from the GUI uses `ChoiceReason::Ambiguous`:** acceptable. The reason
  is record metadata. Core logic does not branch on a written choice's
  reason, and diagnostics use package status, not the record reason.
- **Resume from the GUI sends an empty `allow_again`:** minor defect against
  ops.md §1.5. The GUI never lists the carried exclusions or offers "Allow these
  again" (`change_ui.rs:69,77`). The text honestly says declined versions stay
  excluded, and a workaround exists: an explicit Keep of the declined version
  while someone holds it. Record it as DEFERRED.

## Area 3 detail (E-08)

- `tests/required-cases.toml` against real test names: the `required-cases`
  check (`xtask/src/inventory.rs::check_required`) requires every listed name to
  run and pass in its tier. Both tiers PASS today, so there is no drift.
- Overclaims: R4-check labels the paired settings recovery as BLOCKED by Q-77.
  RV-04 shows it is also unbuilt. Recheck 2's V-02 FAIL was fixed in round 4
  (R3 confirms the runtime test), but **no GUI recheck** of that fix exists.
  The run report must not call V-02 GUI-verified.
- Stale status: RV-06 and RV-07.

## Area 4 detail (test honesty sample, ten cases)

| Crate | Test | Observable + negative? | Notes |
|---|---|---|---|
| core | `exec::tests::recovery_points_pruned_to_two_per_package` | yes | R1 caught |
| core | `exec::tests::pinned_recovery_points_exceed_two_visibly` | yes | R1 caught |
| core | `plan::tests::game_running_blocks_apply_plan` | yes (NotRunning ok, Running refused) | R2 caught |
| core | `retire_tests::retiring_the_settings_master_never_picks_a_replacement` | yes (master and non-master) | read |
| runtime | `e2e::tests::game_running_on_target_blocks_apply_then_resumes` | yes (Running, Unknown, then NotRunning) | two layers (R2b) |
| runtime | `e2e::tests::restoring_a_copy_a_keep_displaced_converges_the_group_on_it` | yes (bytes on both PCs, no open decision) | R3 caught |
| runtime | `e2e::tests::a_rollback_of_a_copy_the_group_never_displaced_is_still_refused` | weak | RV-08 |
| runtime | `e2e::tests::the_snapshot_lists_the_account_folders_consent_can_include` | yes (4 negatives) | read |
| app | `notice::tests::a_rollback_notice_stays_on_its_add_on_and_settles_when_done` | yes (other add-on, no add-on, not-done) | read |
| app | `rollback_ui::tests::a_rollback_request_is_done_when_installed_or_an_outcome_is_set_and_not_before` | yes | read; outcome is cleared on each new request (`service.rs:821,2468`), so a stale outcome cannot settle a new notice |

## Open items for the run report

**PENDING OWNER**
- Q-77: settings file size cap (`records.rs:901` `None`). Real settings sync stays disabled, as intended.
- Q-88: newer local install after a live keep (R4-check V-01). Not touched here.
- macOS runs per `MAC-OWNER.md`: T-27-11, T-28-13, A-29, A-30, build/verify, bundle, tray, etc. All NOT RUN.
- First GitHub release (D-24 update source returns 404 until one exists, R4-notify-diag T-28-13).

**DEFERRED (source)**
- Removal capture of settings and the `SettingsCleanup` planner: R4-integrate DEFERRED, R4-core-removal; RV-04.
- `ShareSettings` does not upload its checkpoint: R4-integrate DEFERRED (not re-verified; unreachable under Q-77).
- "Pause all settings" is not modelled; there is no capture command producing a checkpoint: R4-integrate.
- Held-release date for peer-only copies: R4-integrate pass 12; R4-check V-09.
- DataAtRisk producer (no design trigger): R4-integrate.
- Diagnostics file picker/timestamped name (ops.md §5.1); fingerprints and build commit: R4-check V-06 residual; RV-09.
- Resume "Allow these again" GUI: ops.md §1.5; R4-core-ops (empty `allow_again`); this review.
- Native T-C11 identity collisions NOT RUN: R4-core-contracts.
- Settings T-S02/S04, T-S23/S24, T-S28, T-S31 foundation-only: R4-core-settings.
- Literal "Review needed" window state, real Pending/NeedsHelp journal rows, About checking/newest/available window states, screen-reader session, Windows startup notification visual: BLOCKED/NOT RUN in R4-check.
- Live CI confirmation of the latest main: BLOCKED in R4-check (no network permission).
- GUI recheck of the round-4 V-02 convergence fix and notice scoping: not run by anyone.
- Remaining review findings RV-01..RV-11 above.

## Budget

Allowance about 1,000 AIU. Metered usage is unavailable to this worker; the
orchestrator must attach it. All five areas were reviewed. No area was skipped.
Recorded at 2026-11-21T01:16Z.
