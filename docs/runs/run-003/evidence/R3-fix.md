# R3-fix evidence (PARTIAL)

Environment: Windows 11 x64, Rust 1.100.0. Branch run003/R3-fix.

Commands run: `cargo test -p wow-sync-core -p wow-sync-adapters -p wow-sync-runtime -p wow-sync-proto`
(all passed: 48/177/10/46) and `cargo test -p wow-sync-app` (16 passed).
`cargo xtask verify` tiers, the A-38 negative control, screenshots/UIA and the A-29 memory re-run
were NOT RUN.

Out-of-scope file touched: `crates/wow-sync-core/src/view.rs` (ChooseCopy sentence, RV-4).

## Findings
- RV-1 (UI follows core next/why, no remapping): PARTIAL. `world.rs`/`model.rs` use core `next` and `why`; no mapping test with a negative control yet.
- RV-2 (pause damaged state): DONE in code. `control.rs` test `a_damaged_pause_file_holds_sync_and_is_an_error_not_a_time`; UI shows a held banner/headline (model test asserts headline).
- RC2-1 (next check time): DONE in code (`backend.rs`, `world.rs`, `pages.rs`); no dedicated test.
- RV-4 (invented continuation/affects text removed): DONE in code; no screenshot.
- RV-6 (this computer confirms only applied packages): DONE. `detail.rs` test `this_computer_confirms_only_applied_or_in_sync_packages`.
- RV-7 (probe port for discovery): DONE. `adapters/probe.rs`, `discover.rs` test `an_oversized_dropbox_info_file_is_not_read`.
- RV-5 (headline used as is): DONE in code; open question not written.
- RV-8 (D-21 wording per reason): DONE in app. `model.rs` test `every_decision_reason_has_its_own_build_free_wording`.
- RC2-3, RC2-2, RV-3, RV-9, RC2-4, RV-12, goal 12: NOT DONE (budget).
- RV-10/11: not addressed.
- Proto crate D-21 copy: NOT DONE.
