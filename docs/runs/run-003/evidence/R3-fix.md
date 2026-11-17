# R3-fix evidence (retry 1)

Environment: Windows 11 x64, Rust 1.100.0, branch run003/R3-fix. Lab root `D:\WoWSyncLab\R3-fix` (synthetic data only).

## Commands and results
- `cargo xtask verify --tier offline`: PASS (fmt, clippy, test, required-cases, deny, markdown-links).
- `cargo xtask verify --tier native`: PASS.
- `powershell -File tools\mem\run.ps1 -LabRoot D:\WoWSyncLab\R3-fix\mem -OutDir docs\runs\run-003\evidence\R3-fix\mem` (scale 1, 3 runs x 3 scenarios): A-29 PASS. Steady medians 28.44 / 35.45 / 35.5 MiB; whole-run **peak 37.26 MiB** (limit 100). Sync start median 0.26 s (max 0.28 s). Raw data in `R3-fix/mem/`.
- A-38 negative control (RV-1): `world.rs` `why` hard-coded to "An update was interrupted." for `PackageStatus::Interrupted`; `cargo test -p wow-sync-app interrupted_text` FAILED, exit 101 (left "An update was interrupted." vs right "Updating Demo was interrupted; it will be finished or restored on the next sync."). Restored `why: p.sentence.clone()`; tests pass again.
- RC2-4: `$out = & devkit.exe fake-client <variant> 2>&1` returned in 69 ms with the stand-in PID; stopped by that exact PID and confirmed gone.

## Findings (IDs as in R3-review.md / R3-check2.md)
- RV-1 DONE. UI text/next step come from core (`world.rs` `why`/`next`). Tests `interrupted_text_follows_core_and_differs_by_cause`, `next_step_text_is_core_s_for_other_actions`; negative control above.
- RV-2 DONE. Per-reason D-21 wording in `wow-sync-app` (`model.rs`, test `every_decision_reason_has_its_own_build_free_wording`) and `wow-sync-proto/src/model.rs` (test `every_decision_reason_has_distinct_build_free_wording`, incl. S7). `scenario.rs` already build-free. The proto's separate identity-reconfirm copy that mentions a game version was left alone.
- RV-3 DONE (docs). `screens.md`, `core-api.md`, `removal.md`, `prototype.md`, both wireframes use `DifferentTocMapping` / D-21 wording.
- RV-4 DONE in code (no invented continuation or Affects text; scope shown only when core exposes it). Core `view.rs` ChooseCopy sentence touched. No screenshot.
- RV-5 DONE. Headline used as is; open question UQ-1 in the new `docs/design/ui/questions.md` (UQ-2 typed scope).
- RV-6 DONE. `detail.rs` test `this_computer_confirms_only_applied_or_in_sync_packages`.
- RV-7 DONE. `adapters/probe.rs`, `LocalState::remove`, bounded info.json read (`an_oversized_dropbox_info_file_is_not_read`), damaged pause state (`a_damaged_pause_file_holds_sync_and_is_an_error_not_a_time`).
- RV-8 DONE (docs). `ARCHITECTURE.md`, `design/crates.md`, `REPO-MAP.md`.
- RV-9 DONE (note). Prototype frozen note in `prototype.md`, `ARCHITECTURE.md` §2.2a, `crates.md`.
- RV-12 DONE. `tools/mem/run.ps1 -LabRoot` (or `WOWSYNC_LAB_ROOT`), default unchanged; used above.
- RC2-1 DONE (code + test `last_check_text_uses_the_scheduler_or_says_unavailable`; no literal next-check or "for 2 add-ons" text).
- RC2-2 PARTIAL. `toggle()` registers an a11y Click action that changes and persists the option (`app.rs`). Not observed in the real app.
- RC2-3 PARTIAL. Tray click and single-instance show request both call `show()` (`app.rs`). Not observed in the real app.
- RC2-4 DONE. `devkit.rs` clears inheritance on std handles before spawn; observed above.
- Real-app screenshots and UIA text (dashboard, decision, interrupted add-on, damaged pause): NOT DONE. Needs synthetic provider scenarios not built in this retry; coverage is by view-model tests only.
- Goal 12 (local icon override): NOT DONE (optional, last priority).
- RV-10/11: orchestrator's, not touched.

## Out-of-scope touches
`crates/wow-sync-core/src/view.rs` (ChooseCopy sentence); `docs/design/ui/questions.md` created; `tests/required-cases.toml` (new R3-fix:app case, renamed one test); removed the unused `Addon.source` field.
