# R2-recheck: independent verification after R2-fix

## Verdict: PASS WITH FINDINGS

M1–M5 and m1/m2/m3/m5 are resolved in the original reproductions.
m4 is partially resolved: duplicate error and empty identity are gone,
but a raw filesystem enum and empty activity line remain. No new blocker
or major was observed in this bounded re-check.

**Memory: NOT RUN.** V1 remains pending owner review. This verdict is for
the requested fix re-check, not run-002 acceptance or closure of V1.

## Authority, environment and scope

- Independent verification worker, approved `gpt-6.1-sol` role under
  D-04/D-17; same runtime selection verified in R2-check.
- Owned checkout: `D:\GitHub\wow-sync.worktrees\R2-recheck`,
  branch `run002/R2-recheck`. Read AGENTS, updated COMMON, PLAN, original
  findings, fix packet/evidence, and applicable GUI/design criteria.
- Tested **clean clone of main** at
  `de6ffc9ac62f2ae03647d9980bcf81a6dcabc85f` (R2-fix merged).
  Windows 11 x64 development PC, build 26300; pinned Rust/Cargo 1.100.0,
  cargo-deny 0.20.2. No dependency installations or policy changes.
- Disposable clone and synthetic installs/provider were created inside the
  owned `R2-recheck\clean` and `R2-recheck\lab` directories, not OS temp.
  This respects the session restriction against scratch writes outside the
  assigned checkout. Neither scratch directory is retained in the commit.
- No real WoW or Dropbox data, account/settings files, credentials, or
  S-forever worktree was accessed. Synthetic Lua files were hashed/copied,
  never executed. No nested agents, push/merge, git maintenance or history
  rewrite was used.
- Safety: only directly launched `Diagnostics.Process` objects are stopped,
  with `Stop-Process -Id` using their launch-returned PID after checking
  their own `HasExited`. No ParentProcessId query/traversal, inferred
  descendant termination, process-name termination, or memory counters.

## Independent execution

Clean clone command context:
`D:\GitHub\wow-sync.worktrees\R2-recheck\docs\runs\run-002\evidence\R2-recheck\clean`.
Fresh shells set
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Command | Result |
|---|---|
| `git clone D:\GitHub\wow-sync <owned-evidence>\clean`; `git checkout main` | PASS, SHA above |
| `cargo xtask verify --tier offline` | PASS: fmt, clippy, tests, required-cases, deny, markdown-links |
| `cargo test --workspace` | PASS: core 163, runtime 21, adapters 45, xtask 24 = **253 passed**, 3 ignored provider tests; app/devkit/doc targets zero tests |
| `cargo build --release -p wow-sync-app` | PASS, 2m00s |
| Changed-core negative controls, restore, `cargo test --workspace` | PASS after restoration: same 253 passed, 3 ignored |
| Owned-worktree `cargo xtask verify --tier offline` | PASS: all six checks, staged evidence Markdown links included |

Logs: [offline](R2-recheck/clean-offline.log),
[workspace](R2-recheck/clean-workspace.log),
[release](R2-recheck/clean-release.log),
[restored workspace](R2-recheck/clean-restored-workspace.log).
Exact source cleanliness and release executable hash:
[environment.json](R2-recheck/environment.json).

Native/provider tier reruns, cross-machine/cloud delivery, other platforms,
screen-reader speech and G7/G9 repetition were outside this bounded request
and are **NOT RUN**, not newly claimed passes.

## Finding-by-finding re-check

The GUI checks below used the independently built release app in developer
mode, real adapters, synthetic A/B/C installs and one local provider folder.
The [retained harness](R2-recheck/gui.ps1) drives the actual app via UI
Automation and explicit scrolling. These notes are not substituted headless
tests or copied author claims.

| Original item | Result | Independent reproduction and evidence |
|---|---|---|
| **M1** unfamiliar additions invalidate enrollment | **Resolved** | Enrolled `_test_`/`wow_test` with both confirmations, added plain-toc Extra, observed usable status and published seq 2 containing Extra. Restart + GUI Sync now remained usable, without StaleConfirmation. [addition](R2-recheck/gui/M1-addition.txt), [published state assertion](R2-recheck/gui/M1-publication.txt), [restart](R2-recheck/gui/M1-restart.txt) |
| **M2** blocked code published / shown up to date | **Resolved** | Good eligible, Blocked with only `Blocked_Unknown.toc`; consent lists Blocked as excluded. After enrollment and another publication, Blocked's unique main.lua object **and its folder-tree record are absent** from the group. Status explicitly says its client-specific tocs are unvalidated and it “is not synced”, not up to date. Inventory metadata remains for status; no claim that metadata is excluded. [status](R2-recheck/gui/G2-M2-status.txt), [object/tree assertions](R2-recheck/gui/M2-publication.txt) |
| **M3** join/publication decisions outside core | **Resolved** | Read actual call sites: runtime `service.rs` calls core `check_join` at 591 and `publish_needed` at 1161; former join string guard and `same_folders` implementation removed. Core owns the comparisons in `variant.rs:540` and `plan.rs:167`. GUI C's attempted wowf join was refused and wrote no enrollment; normal changes and apply reports published. Negative controls below fail the new core tests when weakened. [cross-product refusal](R2-recheck/gui/M3-cross-product.txt) |
| **M4** incomplete consent/scope disclosures | **Resolved** | Fresh recognized and unfamiliar screens show actual chosen install/full AddOns paths, `validated-plain/1` rule, automatic replacement, missing ≠ removal, recovery preservation/path and WTF-layout exclusion, before consent controls. Without consent, actual Create is refused and provider/local-state files remain absent. [G1](R2-recheck/gui/G1-M4-m1.txt), [G2](R2-recheck/gui/G2-M4.txt), [no-consent refusal](R2-recheck/gui/G1-no-consent.txt) |
| **M5** ambiguity lacks candidate sources | **Resolved** | A/B published equal 4.0 labels with different bodies. Both were unchanged; status associates each 4.0 candidate with “this computer” or the other synthetic computer's name. [A](R2-recheck/gui/G5-M5-A.txt), [B](R2-recheck/gui/G5-M5-B.txt) |
| **m1** noncanonical Forever label | **Resolved** | Fresh screen says “World of Warcraft: Forever (wowf) - folder _forever_, build 1.0.0.1”. [G1](R2-recheck/gui/G1-M4-m1.txt) |
| **m2** running-game headline wording | **Resolved** | Exact “Waiting — WoW is running”. B stayed a2 while directly launched FakeClient ran and A published a3. Stopping only that FakeClient's launch PID resumed apply to a3. [wait](R2-recheck/gui/G4-m2-wait.txt), [resume](R2-recheck/gui/G4-m2-resume.txt) |
| **m3** post-apply line loses source/versions | **Resolved** | B's actual files match A by SHA-256; line says “updated from 1.0 to 2.0 from computer Recheck-A and verified on this computer”, with recovery count 1. After next update it correctly says 2.0→3.0, count 2; recovery later correctly says 4.0→5.0. [initial apply](R2-recheck/gui/G3-m3.txt), [later apply](R2-recheck/gui/G4-m2-resume.txt), [recovered apply](R2-recheck/gui/G8-m5-recovered.txt) |
| **m4** empty/duplicate/raw error display | **Partially resolved** | Replaced provider directory with a file. “Something needs attention” plus exactly one Problem line; former “ on ()” and duplicate error are gone. Still shows `NotPlainFile` and a blank “Game activity:” line. [G6](R2-recheck/gui/G6-m4.txt). Residual below. |
| **m5** non-game interruption blames WoW | **Resolved** | Injected `--crash-after 2` with no FakeClient running: Waiting, one interrupted operation, two retained points, and neutral “updating alpha was interrupted; it will be finished or restored on the next sync”. GUI Sync now recovered/applied, zero interrupted operations; no false WoW-start claim. [interrupted](R2-recheck/gui/G8-m5-interrupted.txt), [recovered](R2-recheck/gui/G8-m5-recovered.txt) |

### Affected G-items

| Item | Re-check |
|---|---|
| G1 recognized consent | PASS: canonical label, actual scope/disclosures, refusal/no writes before consent |
| G2 unfamiliar consent | PASS: scope-only and identity-only each refused; both required; blocked folder listed and remains not synced. [scope-only](R2-recheck/gui/G2-scope-only.txt), [identity-only](R2-recheck/gui/G2-identity-only.txt) |
| G3 safe apply | PASS: all Alpha file hashes equal A, source/from/to versions and recovery count visible |
| G4 running wait/resume | PASS: exact headline, unchanged while FakeClient runs, resumes after its launch PID stops |
| G5 ambiguity | PASS: labels/sources visible, both bodies unchanged |
| G6 provider/error, m4 | PASS against the observable error/reason criterion, with residual minor presentation finding below |
| G8 interruption/recovery | PASS: truthful non-game reason, retained copies, GUI recovery |

### Residual m4 — MINOR

- Locations: `crates/wow-sync-app/src/ui.rs` activity paragraph in enrolled
  status, and unchanged `crates/wow-sync-adapters/src/lib.rs:66` filesystem
  error Display used by runtime provider errors.
- Reproduce: enroll synthetic A, replace the provider directory with a
  regular file, wait one cycle.
- Observed error:
  `Problem: shared folder problem: NotPlainFile at <synthetic-lab>\provider: not a directory`.
  The enum is still raw/technical, though “not a directory” is a plain-English
  reason. The now-contextless `Game activity:` line is empty.
- No duplicate error or empty identity line remains. This is not a new
  blocker/major and does not invalidate G6's error/reason criterion.

## Changed-area negative controls (A-38)

Ten cheap representative mutations, limited to changed core areas, were
applied individually on clean-clone scratch branch
`verification-recheck-controls`. Each used
`cargo test -p wow-sync-core <name> -- --exact --nocapture`.
Every row produced the actual named test failure, exit 101; none survived
or merely failed compilation. Every source file was restored byte-for-byte
in `finally`; clean clone returned to main, then workspace tests passed.
No tests, assertions, required cases or thresholds were edited.

| Mutation | Observed failing test |
|---|---|
| Reintroduce digest-stale enrollment | `variant::tests::later_folders_do_not_invalidate_enrollment` |
| Publish Ok-but-blocked folders | `plan::tests::blocked_folders_are_never_published` |
| Permit cross-product join | `variant::tests::join_requires_equal_product` |
| Drop publication carrying an apply report | `plan::tests::publish_needed_decisions` |
| Accept partial candidate | `target::tests::partial_suite_candidate_needs_decision` |
| Prefer ambiguous peer by timestamp | `target::tests::record_timestamps_cannot_influence_target` |
| Confirm from member/state presence alone | `status::tests::published_is_not_confirmed` |
| Omit candidate-source rendering | `status::tests::needs_decision_shows_evidence` |
| Always blame interruption on game | `status::tests::interruption_blames_the_game_only_when_it_is_running` |
| Restore generic running-wait headline | `status::tests::headline_names_running_game_while_waiting` |

Exact substitutions: [mutations.py](R2-recheck/mutations.py).
Results: [negative-controls.json](R2-recheck/negative-controls.json);
numbered per-mutation logs alongside it. Unchanged version/admission/exec,
TOC, native filesystem and process controls were intentionally not rerun
as separate mutations.

## GUI observation limits, safety and handoff

- Expanded disclosures place consent controls below the first viewport.
  Initial UIA actions silently did nothing there; UIA reported
  `IsOffscreen=False` even though Create's bounds were below the window.
  After observing the geometry, the completed run used actual wheel
  scrolling and checked coordinates against the window instead of trusting
  that flag. This is an automation limitation, not evidence that ordinary
  scrolling/enrollment fails.
  [geometry](R2-recheck/gui/enrollment-geometry.txt),
  [cropped initial viewport](R2-recheck/gui/enrollment-before-scroll.png).
- Screenshot was inspected: app-window crop only, synthetic display name,
  owned project paths, no real user/machine names or profile paths. Notes
  replace the owned lab path with `<synthetic-lab>`.
- New synthetic stale-mapping/reconfirmation-panel flows were not forced:
  they are outside the original reproduction. No claim is made that every
  new GUI branch was independently exercised.
- All directly launched app/FakeClient processes were stopped through their
  retained launch objects/PIDs. No process traversal was performed.
- Memory **NOT RUN**, unchanged owner barrier. Do not reuse this re-check as
  memory acceptance or closure of the previous verification incident.
- Handoff: m4 residual is cosmetic; M1–M5/m1–m3/m5 original repros pass.
  Owner review of V1 and broader acceptance/platform/provider work remain
  separate obligations.

## Final validation and cleanup

- Owned-worktree offline tier: **PASS**, all six checks.
  [Output](R2-recheck/worktree-offline.log),
  [report](R2-recheck/worktree-verify-report.json). Report names the tested
  source SHA and `git_dirty=true` for staged evidence, not a fictional clean
  post-commit run.
- Build artifacts were reused only within the owned scratch target. The
  first worktree command reused an xtask binary with the clone path baked
  into `CARGO_MANIFEST_DIR`; this was identified from report location and
  `git_dirty=false`. `cargo clean -p xtask --quiet` forced a rebuild before
  the authoritative worktree check above. The final report is from the
  actual staged worktree, not the clean clone.
- Disposable clone, compiled artifacts inside it and synthetic lab were
  removed after all directly launched GUI/FakeClient processes had been
  stopped through their own launch-returned PIDs. Retained artifacts are
  small evidence only; no fixture addon contents are committed.
