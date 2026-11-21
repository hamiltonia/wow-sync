# R5-decide: Q-88 (a), DataAtRisk producer, diagnostics follow-ups

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R5-decide`, branch `run005/R5-decide`.

## Goals

1. **Q-88 option (a) (D-26; questions.md Q-88; contracts.md §5.3, ops.md
   §1.5; R4-check V-01).** After a live keep of V, if this computer has a
   demonstrably newer (or unordered) version W installed that is not in the
   keep's `considered` set and was never published:
   - W stays installed; the kept tree is **not** reinstalled over it;
   - W is **not** published to the group (no release record, no shared
     inventory claiming W as the group's code);
   - `NeedsDecision(NewVersionAfterDecision)` stays open on every cycle
     until the user answers (Keep V, Use W, Follow newest again), not one
     cycle;
   - peers keep V; answering "Use W" publishes W normally; answering
     "Keep V" then installs V (with W kept for recovery as usual);
   - the decision view carries enough for the app to say that W exists
     only on this computer.
   Tests: the V-01 reproduction from R4-check evidence now keeps W across
   several cycles and never publishes it; each answer does the right
   thing; a peer-published W keeps today's behaviour. A-38 control.
2. **DataAtRisk producer (ops.md §3, ui/interaction.md:147, A-27).** Raise
   `NoticeKind::DataAtRisk` when local state is unreadable, and when a
   recovery copy cannot be written because storage is full or the write
   fails. Feed it through the existing notice planner (`notices.rs`) so
   Q-85/Q-86 rules apply (never suppressed by recurrence; respects the OS
   toggle; always in the app snapshot). Expose it in the snapshot so the
   app can show it in the headline, tray and Needs attention. Tests with
   injected effect failures; A-38 control.
3. **RV-09.** Diagnostics "Include file fingerprints" fills real
   fingerprints from the inventory (`runtime/src/diag.rs:236`), still
   redacted and bounded. The build commit comes from `WOWSYNC_BUILD_COMMIT`
   at compile time (R5-release sets it in the release workflow); `dev`
   stays the fallback. Test both.
4. **Drop `Command::SaveDiagnostics`** (rebuild-on-save). The app already
   sends `SavePreviewedDiagnostics`. Remove the variant, its service arm
   and tests that exercise it, and keep or port their assertions to the
   previewed path. If any app file still references it, report it; do not
   edit app files.
5. **RV-11 (adapters part).** `crates/wow-sync-adapters/src/sysinfo.rs`
   uses absolute macOS helper paths (`/usr/bin/sw_vers`, `/usr/sbin/scutil`
   and so on).

## Files you own

- core `choice.rs`, `choice_tests.rs`, `target.rs`, `decide.rs`,
  `decide_tests.rs`, `decision.rs`, `notice.rs`, `diagnostics.rs`.
- runtime `control.rs`, `notices.rs`, `diag.rs`, `snapshot*.rs`,
  `tests/public_api.rs`, `e2e.rs` (new tests in a clearly marked block;
  edits to existing `SaveDiagnostics` tests).
- runtime `service.rs`: small, local edits only. R5-remove also edits it.
- `crates/wow-sync-adapters/src/sysinfo.rs`.
- Your required-cases section; renames/removals in other sections only for
  tests you deleted under goal 4 (state them).

Do not edit `removal.rs`, `settings.rs`, `exec.rs`, `records.rs` (R5-remove)
or any app file. If you need one, stop and report.

## Done

Both tiers pass; evidence `docs/runs/run-005/evidence/R5-decide.md`; reply
with SHA and a short API note for R5-app (new snapshot fields, decision
view fields). Budget about 1,300 AIU; stop and report at 1,700.
