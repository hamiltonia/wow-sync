# R3-review: integration review of run-003 main

- Packet: [R3-review](../packets/R3-review.md) (integration review role, opus-5.5).
- Checkout: `D:\GitHub\wow-sync.worktrees\R3-review`, branch `run003/R3-review`,
  base `ce15e24` (main after the phase F merge, which includes R3-ui-build `17a360a`).
- Environment: Windows 11 x64 dev PC, Rust 1.100.0. This was a read-only review.
  No app process was started or stopped, and no code or other docs were changed.
- Scope: ENGINEERING E-01 to E-09 (focus on E-06 and E-08), ACCEPTANCE A-37 to
  A-44, and D-21 conformance of the merged code and docs. Run-003 range: `b420717..ce15e24`.

## Verdict

**FAIL. No blocker, 7 major and 5 minor findings.**

The merged state builds and passes the offline tier. Its safety-relevant behaviour
(variant and account boundaries, WoW-running block, harness process safety) has no
regression that I found. The verdict is FAIL because several majors break project
rules that a first release candidate cannot carry:

- the app re-derives next actions and explanations instead of using core (E-08, A-25);
- the UI copy still implies build gating, which contradicts D-21;
- an owner-decision history entry was edited (E-05, A-40);
- the architecture and crate docs no longer describe the code (E-08).

None of these needs owner input. All can be fixed in one bounded fix packet,
plus a records update by the orchestrator.

## Commands run

| Command | Result |
|---|---|
| `git --no-pager diff --stat b420717 HEAD -- Cargo.toml Cargo.lock deny.toml .github rust-toolchain.toml xtask tools crates/*/Cargo.toml` | 9 files. The only new crate is `wow-sync-proto`; it uses workspace dependencies only |
| `git --no-pager diff b420717 HEAD -- Cargo.toml deny.toml .github crates/*/Cargo.toml` and the Cargo.lock name/version lines | No new third-party package. CI added a Windows native tier. `spike-gpui.yml` dropped the deleted `spikes/gpui` |
| `git --no-pager diff 596f8fd 95bf21c -- tools/mem` | R3-ui-build changed only window-title matching and the wizard drive sequence. Stop logic is unchanged |
| `git --no-pager grep` for user profile paths, SIDs, user/computer name, secrets over `docs/runs/run-003`, `tools`, the app/runtime/proto crates and `docs/design` | No private data. See [E-06](#e-06-dependencies-permissions-data) |
| SHA-256 and line comparison of `crates/wow-sync-app/src/*` against `crates/wow-sync-proto/src/*` | See RV-9 |
| `git --no-pager show f92a61a -- docs/OWNER-DECISIONS.md` | Deletes the `### D-20` heading (RV-10) |
| `cargo xtask verify --tier offline` | See [Verification](#verification) |

## E-06: dependencies, permissions, data

- **Dependencies.** No new third-party crate in run-003. `wow-sync-proto` and
  `wow-sync-app` use workspace-pinned `gpui`, `gpui_platform`, `tray-icon` and
  `raw-window-handle`. `deny.toml` is unchanged in the run. **PASS.**
- **CI permissions.** `ci.yml` and `spike-gpui.yml` both use `permissions:
  contents: read`, `persist-credentials: false` and a SHA-pinned checkout. The only
  run-003 changes are a Windows-only native tier and removing a deleted spike path.
  **PASS.**
- **Process safety (run-002 incident V1).** `tools/mem/MemLib.ps1` stops only the
  launched root, after checking its PID and creation time again (`Stop-Launched`).
  Children are never stopped, and PostMessage goes only to windows of the tracked
  PID. R3-ui-build did not change this logic. **PASS.**
- **Data.** Evidence uses the synthetic computer `ComputerA` and synthetic installs
  under `target\uistate`. Committed UIA text and PNG shots show worktree paths
  (`D:\GitHub\wow-sync.worktrees\R3-ui-build\target\uistate\...`). These are
  repository paths, not user profile or Dropbox paths. No user name, computer name,
  SID or credential was found. **PASS.**
- **No Lua execution.** No Lua interpreter crate or call in the runtime or
  adapters. **PASS.**
- **Boundary note.** See RV-7: the runtime does some direct filesystem and
  environment reads outside the adapters.

## E-08: single authority and repository map

See RV-1, RV-4, RV-5, RV-7, RV-8 and RV-9. `docs/REPO-MAP.md` lists the app but not
`crates/wow-sync-proto` (part of RV-8).

## D-21 conformance

- Core: `DecisionReason::DifferentClientBuild` is gone (R3-ui-build `09be75e`), and
  `questions.md` Q-21 records D-21. **PASS.**
- UI copy and design docs: not conforming (RV-2, RV-3).
- Removal design: `removal.md` correctly says R3D-13 must be re-reviewed against
  D-21 before implementation. It stays open (see the remaining-work table).

## Findings

| ID | Severity | Area | Location | Finding (evidence) | Suggested fix |
|---|---|---|---|---|---|
| RV-1 | major | E-08 single policy authority; A-25 | `crates/wow-sync-runtime/src/detail.rs` (`PackageRow.next`, `.awaiting`); `crates/wow-sync-app/src/world.rs` `from_snapshot` (about 381-460); `crates/wow-sync-app/src/model.rs` `addon_story` (about 882-1090) | The runtime fills `PackageRow.next` from `core::view::next_action`. `from_snapshot` drops `next` and `awaiting`, so core's next action is unused in the app. `addon_story` hard-codes its own copies of `NextAction::sentence` plus its own "what/why/recovery/grouped" text. Some of that text is untruthful. Interrupted always says WoW started it, but core separates `Interrupted` from `InterruptedByGame`. InSync and ConfirmedBy say "Every computer that reported has the same files", which over-claims (A-24). Recovery says "No earlier copies are needed yet" even after applies. Grouping always says "they share a name prefix". This computer's row always has an Ok icon | Carry `next`/`awaiting` into the world model and render core's sentence. Derive "why" from core's `Explanation`. Remove the duplicated sentences. Add tests that the UI text follows core for `Interrupted` and `InterruptedByGame` |
| RV-2 | major | D-21 UI copy (R3-check RC-1 only partly fixed) | `crates/wow-sync-app/src/model.rs` 703-736 (`decision_title`, `decision_reason`); `crates/wow-sync-proto/src/model.rs` about 716 and 730-731; `crates/wow-sync-proto/src/scenario.rs` 734-737 | The fallback `_ => "the copies are for different game builds"` covers `DifferentTocMapping`, `NoCompatibleToc` and `UnvalidatedTocMapping`. `DifferentTocMapping`/`NoCompatibleToc` say "...or does not say it supports this game version". Both imply build or Interface gating, which D-21 removed. The prototype S7 (Questie) now uses `DifferentTocMapping`, so it still shows "different game builds". The app mapping also differs from `docs/design/ui/screens.md` 446-451 | Give each `DecisionReason` its own D-21 wording, matching screens.md (e.g. "game-version-specific files"). Fix both crates, or share the code (RV-9). Add a test that no decision text mentions builds or game versions |
| RV-3 | minor | D-21 docs not updated | `docs/design/core-api.md` 174, 283; `docs/design/ui/screens.md` 448-449; `docs/design/ui/prototype.md` 92; `docs/design/removal.md` 634; `docs/design/ui/wireframes/dashboard-dark.svg` 40; `docs/design/ui/wireframes/decision.svg` 55 | These still name `DifferentClientBuild` or "made for a different game build" | Update the docs to the D-21 reason set, or mark the wireframes superseded |
| RV-4 | major | Design conformance; truthfulness (A-24) | `crates/wow-sync-app/src/world.rs` 359; `crates/wow-sync-app/src/model.rs` 511-533 (`continuation`), 635-700 (`need_items`) | screens.md 72-74 says the build-out shows no continuation sentence until UG-25 exists. The app hard-codes `apply_scope = PerPackage` and always prints "Only X is affected. Other add-ons keep syncing". That is false when a journal NeedsHelp blocks all applies. Decisions also say "{title} only". There is a stale comment at 512 ("the prototype reads the fake typed scope") | Show no continuation sentence until core exposes a typed scope (UG-25), as screens.md says |
| RV-5 | minor | E-08 headline authority | `crates/wow-sync-app/src/model.rs` 128-167 | The app promotes Reconfirm to Help tone and lets "paused" override Error/NeedsHelp headlines. screens.md says the UI never promotes or demotes core's `Headline`. The code comment says this is "recorded as a design disagreement", but no such record exists in the docs | Use core's headline, or record the disagreement and a decision in screens.md/questions.md |
| RV-6 | minor | R-12 truthfulness (latent) | `crates/wow-sync-runtime/src/detail.rs` 294-308 (`member_rows`) | For this computer, `confirmed` is the count of all non-NotSynced packages, including Available, Blocked and NeedsDecision ones. The app's Computers page currently shows special text for this computer, so the wrong number is not visible in the shots. It is a wrong value in a public snapshot field | Count only applied/in-sync packages, or leave the field out for this computer |
| RV-7 | minor | E-08 / ARCHITECTURE §2.2-2.3 adapter boundary | `crates/wow-sync-runtime/src/control.rs` (`set_paused` uses `std::fs::remove_file`); `crates/wow-sync-runtime/src/discover.rs` (`read_dir`, unbounded `read_to_string` of Dropbox `info.json`, `APPDATA`/`LOCALAPPDATA`/`USERPROFILE` reads) | The runtime does filesystem and environment I/O directly, but ARCHITECTURE says adapters own it. A damaged pause file reads as `Some(0)` and is shown as paused since 1970 | Move this I/O behind adapter ports with a size limit. Treat an unparsable pause file as an error |
| RV-8 | major | E-08 docs match code; A-40 | `docs/ARCHITECTURE.md`; `docs/design/crates.md`; `docs/REPO-MAP.md` | ARCHITECTURE still says "pending independent review" (run-002 slice design). It does not mention `core::view`, the runtime snapshot/pause/quit/activity/discovery APIs or the proto crate. crates.md's module table lacks core `view` and runtime `control`, `detail`, `discover` and `activity`, and its crate graph lacks `wow-sync-proto`. REPO-MAP lacks `wow-sync-proto` | Update all three to the merged code |
| RV-9 | major | E-08 duplication | `crates/wow-sync-proto/src/*` vs `crates/wow-sync-app/src/*` | `theme.rs`, `ui.rs`, `assets.rs` and all 10 icons are byte-identical. Most lines are shared: `pages.rs` about 1455 of 1532, `model.rs` about 1216 of 1307, `app.rs` about 1544 of 1919, `setup.rs` about 423 of 495. There is no recorded plan to share or retire them, so fixes go to one copy only (RC-3 was fixed in the app, not the proto; RV-2 is in both) | Extract a shared UI crate (or feature-gate the fake world in the app), or record that the prototype is frozen and excluded from fixes |
| RV-10 | major | E-05/A-40 decision history edited | `docs/OWNER-DECISIONS.md`; commit `f92a61a` | The D-21 commit deleted `### D-20: Run-003 plan approved (2026-11-16)`. The D-20 bullets now sit under D-21 (line 173), and D-19 follows at 232. The history row (297) says "this commit" instead of `f92a61a` | Orchestrator restores the D-20 heading in place (no content change), keeps D-21 as its own section and sets the row to `f92a61a` |
| RV-11 | minor | Records (E-05, A-40) | `docs/runs/run-003/USAGE.md`; `docs/runs/run-003/HANDOFF.md` | USAGE still shows R3-mem and R3-proto as "Running" and has no rows for phase E/F workers or for checkpoints after the phase D barrier. HANDOFF next actions still say "record usage". HANDOFF says CodeQL alert 1 was dismissed, but R3-check says it was fixed (RC-2 partly open). The phase C table has bullets inside cells, which breaks it | Orchestrator updates USAGE and HANDOFF from the evidence files |
| RV-12 | minor | E-06 evidence hygiene | `docs/runs/run-003/evidence/R3-ui-build/shots/*.uia.txt` and PNGs | The shots contain full local worktree paths. They are not private, but future real-install shots would show user paths in the same way | Use a neutral lab root (e.g. `D:\WoWSyncLab`) for evidence runs, or redact paths in UIA dumps |

## Remaining work before a first release candidate

Status: **done** means met with evidence for the RC environments. **partial** means
some evidence exists, or it works only for the slice, Windows or fake data.
**not started** means nothing is built. Owner checks Q-3 and Q-23 are still pending
(OWNER-CHECKS.md).

| Acceptance ID(s) | Area | Status | What remains |
|---|---|---|---|
| A-01 | Install and enrol on every declared OS/architecture | partial | Real app setup wizard on Windows x64 only. macOS and ARM64 runtime not run; CI only builds them |
| A-02, A-46 | Side-by-side variants (Forever is not Classic Era); unfamiliar-client guided enrolment | partial | Variant/Q-24 marker gate is in core and the wizard confirms identity. Needs real-client validation and the full guided safe-scope flow for unfamiliar clients |
| A-03 | Matching/different account and character identities | partial | Scope rules are in core. Settings are not synced yet, so the account/character path is untested end to end |
| A-04 | Join with existing addons/settings and setup consent | partial | Code join with consent works in the slice. Settings join not built |
| A-05 | Unavailable cloud files, offline Dropbox, moved roots, denied access | partial | Folder-provider failure cases only. Needs real Dropbox (A-34) |
| A-06 to A-09 | Addon code sync, provenance, independent updates, ambiguous releases, multi-folder addons | partial | Code sync slice works. Decisions are read-only in the GUI (no resolve action). Fix RV-1, RV-2 and RV-4 |
| A-10 | Uninstall/removal propagation with recovery | not started | Design only. R3D-13 must be re-reviewed against D-21 first |
| A-11 to A-15 | Settings master, exceptions, routine yield, explicit push, competing pushes, pause | not started | Not built |
| A-16, A-17 | Three history states; extra copies for unresolved/recovery data | partial | Code-only retention. Settings history and GUI restore not built |
| A-18, A-19 | Reject a bad release and roll back, with or without settings checkpoint | not started | Not built |
| A-20 | Three replicas with reordered/delayed/duplicated delivery | partial | Fake/folder provider tests only. Needs real Dropbox |
| A-21 | WoW starts during detection/staging/application | partial | Block and interruption work in the Windows slice. macOS not run. RV-1 mislabels interruptions in the UI |
| A-22 | Crash, disk-full, permission failure and restart at mutation points | partial | Runtime failure-injection tests exist for the code slice. Settings, removal and rollback paths not built |
| A-23 | Malicious/malformed payloads, unsafe paths and links | partial | Path and payload checks exist for the code slice. Not extended to settings/removal |
| A-24 | Truthful per-computer status; publish is not apply | partial | Confirmations are shown, but RV-1, RV-4 and RV-6 over-claim |
| A-25 | Choices, conflicts, removals, failures and recovery explained in the GUI | partial | Fix RV-1 and RV-5. Deferred UI gaps UG-8, UG-9, UG-13, UG-15, UG-16, UG-18 to UG-21, UG-23, UG-25 and UG-26; UG-12 partly done |
| A-26 | Real GUI for setup, safe sync, settings, conflicts, rollback | partial | Setup and code status only. Settings, conflict resolution and rollback GUI not built |
| A-27 | Grouped notifications without repeats | not started | Tray only, no notifications |
| A-28 | Honest app-update notice and manual download guidance | not started | Not built |
| A-29 | Memory at most 100 MiB; sync starts within 10 s | partial | Windows PASS (max 37.7 MiB; start 0.18-0.26 s). macOS blocked |
| A-30 | Useful sync latency | partial | Proxy measurement only |
| A-31 | Owner-approved CPU/latency/throughput/soak budgets | not started | Values need owner approval first |
| A-32 | Default telemetry with planted private data | not started | Not built |
| A-33 | Diagnostic export, previewed and redacted | not started | Not built |
| A-34 | Installed-app transfers through real Dropbox | not started | Folder provider and API design spike only |
| A-35 | Platform installed-app smoke on every target | not started | No installers; only the Windows x64 dev build is exercised |
| A-36 | Incompatible shared data, stale app, competing old sync job | partial | Shared-format version checks in the slice. Stale-app and competing-job cases not demonstrated |
| A-45 | Retire a computer, including a settings master | not started | Not built |
| A-37 | Recorded orchestration, role/model matrix, cumulative usage | partial | Done in run-003, but USAGE/HANDOFF are stale (RV-11) |
| A-38 | Verifier shows a test fails for a broken implementation | partial | Done for earlier slices. Needs repeating for the RV fixes |
| A-39 | Clean-checkout documented verification | partial | Offline and Windows native tiers pass. Provider tier and macOS native not run |
| A-40 | Fresh worker finds authoritative intent and records | partial | RV-8 and RV-10 |
| A-41 | Reviewer rejects private data, payload instructions, unjustified dependencies | partial | This review found no such issue (E-06). A seeded-rejection exercise has not been run in run-003 |
| A-42 | Real GUI tasks with retained evidence; owner-approved prototype | partial | Prototype approved (D-21). Product UI diverges from it (RV-2, RV-4, RV-9). R3-check2 in progress |
| A-43 | Domain change testable without GUI/network/provider | partial | Core and runtime test without GUI. Blocked by RV-1 for UI text, which is not derived from core |
| A-44 | Failure or missing owner prerequisite pauses the run | partial | Followed in run-003 (R3D-13 retries stopped). Owner checks Q-3 and Q-23 still pending |

Top remaining work, in order:

1. One app fix packet for RV-1, RV-2, RV-4, RV-5 and RV-9 (single authority, D-21
   copy, no false continuation).
2. Orchestrator records fixes RV-8, RV-10 and RV-11.
3. Removal (A-10) after the R3D-13 re-review.
4. Settings sync (A-11 to A-15) and rollback (A-18, A-19).
5. Real Dropbox (A-34, A-05, A-20), macOS runtime and memory (A-01, A-29), and
   installers with platform smoke (A-35).

## Verification

| Command | Environment | Result |
|---|---|---|
| `cargo xtask verify --tier offline` (PATH prefixed with `%USERPROFILE%\.cargo\bin`), from the worktree root, after writing this file | Windows 11 x64 dev PC, Rust 1.100.0 | **PASS**: fmt, clippy, test, required-cases, deny and markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0` |

The native and provider tiers were not run; this review changed no code.
