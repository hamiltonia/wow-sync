# R6-gate: real installs, Forever toc rule, first-run safety, test-mode core

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R6-gate`, branch `run006/R6-gate`.

## Why

D-28 removes the Q-24 synthetic marker gate by default. Today
`core/src/variant.rs` `validate_variant` returns
`Refused(NotSyntheticInstall)` for every real install. D-28 also gives a
working answer for Q-3, and with that answer most real add-ons stop being
blocked. On the dev PC, 40 of 102 real add-ons have suffixed tocs:
`_Camelot` (29), `_Forever` (10), `_Vanilla`, `_Mists`, `_TBC`, `_BCC`,
`_Wrath`, `_Cata` and `_Classic`. D-28 also makes "first run loses nothing"
a requirement.

## Goals

1. **Remove the default gate (PLAN goal 1).** `validate_variant` accepts an
   unmarked install. Keep the marker parser.
   - Add an explicit policy input, for example
     `MarkerPolicy::{NotRequired, Required}` or a `require_marker: bool`
     argument. `Required` keeps today's behaviour and refusal.
   - Every production call site uses the effective test-mode setting
     (goal 4), with `NotRequired` as the default.
   - Update the tests that assert the old default. Keep a test that
     `Required` still refuses missing, wrong or BOM-prefixed markers, as the
     existing cases do.
   - Remove or reword the "slice only"/Q-24 statements in
     `docs/design/domain.md` §2.1 and `questions.md` Q-24 so they describe
     D-28 as built.
2. **Forever toc rule (PLAN goal 2).** Implement domain.md §3 "profile rule"
   for `wowf`, extended per D-28:
   - game-type suffixes `Forever`, `Camelot` (in that listed order, either
     separator, case-insensitive);
   - then family `Mainline`;
   - then the plain `F.toc`;
   - other suffixes are ignored, so a folder with only `_Vanilla` is
     `None`.
   Step 2 of §3 (identical summary fields, otherwise `Ambiguous`) applies
   inside the game-type tier.
   - Add `Mainline` to the profile as a family tier. Bump `mapping_id` to
     `profile:wowf/2`.
   - `assess_layout` uses `TocMapping::Profile` for recognized `wowf`.
     Unfamiliar products keep `ValidatedPlain`.
   - Every other `select_toc` caller must use the variant's mapping, not a
     hard-coded plain mapping: `inventory.rs`, `runtime/settings_cycle.rs`,
     and any others you find. Check how settings ownership
     (`SavedVariables`) reads the selected toc.
   - An already-enrolled variant whose layout digest changes must not lose
     data or stop silently. Follow the existing reconfirmation path
     (`StaleConfirmation` / `NeedsHelp`) and show it with a test. If the
     existing path cannot handle it, stop and report.
   - Update domain.md §2.1's profile table and §3, and questions.md Q-3 and
     Q-20. Cite warcraft.wiki.gg "TOC format" (read 2026-11-22), and note
     that the owner's client check is optional confirmation.
   - Tests: a table over the observed dev-PC suffix mixes, using synthetic
     folders only (no real contents):
     - plain only;
     - `_Camelot` with plain;
     - `_Forever` with `_Camelot` (same and different summaries);
     - `_Mainline` with plain;
     - `_Vanilla` only;
     - `_TBC` with plain;
     - mixed separators and case.
3. **First run loses nothing (PLAN goal 3).** Audit the first-sync plans:
   - a new computer joining a populated group with a populated install;
   - a populated install creating a new group;
   - a joining install that has local-only add-ons, add-ons the group
     removed, older versions, and different same-label versions (Q-22).
   Every delete or overwrite on the first cycle must be preceded by a
   GUI-restorable recovery copy, or must not happen. Local-only add-ons
   must be kept and published (additions propagate).
   - Write e2e tests that prove each case byte-for-byte after a GUI-path
     restore command.
   - If you find a gap, fix it in your files and record it as a finding.
     If the fix needs a product decision (for example, whether a group
     removal applies to a joining computer's local copy), stop that item
     and report it.
4. **Test mode, core and runtime half (PLAN goal 5).** Add a per-computer
   persisted setting, off by default, that selects `MarkerPolicy::Required`.
   - Expose it as `Command::SetTestMode(bool)` (or a similarly named
     command) and as a field on `StatusSnapshot`. Persist it where the other
     per-computer preferences live (see `core/src/notice.rs` `NoticePrefs`
     and its runtime storage).
   - Turning it on does not un-enroll an already-enrolled real install.
     Instead it blocks further sync of an unmarked enrolled variant, with a
     visible reason. Turning it off resumes sync. Test both directions.
   - R6-app builds the GUI after your API merges. Document the API in your
     evidence.

## Files you own

- `crates/wow-sync-core/src/{variant.rs,toc.rs,inventory.rs,testkit.rs}` and
  their tests. Also the core first-run planning files you need
  (`target.rs`, `exec.rs`, `removal.rs`, `records.rs`): keep those edits
  minimal, with tests.
- `crates/wow-sync-core/src/notice.rs` only for the persisted preference,
  if that is where it belongs.
- `crates/wow-sync-runtime/src/{service.rs,control.rs,settings_cycle.rs,discover.rs,detail.rs,e2e.rs}`.
  In `service.rs`, keep edits small and local; R6-app changes the refusal
  text function near line 1618, so leave that function to them.
- `crates/wow-sync-adapters/src/{fs_scan.rs,testkit.rs}` if needed.
- `docs/design/{domain.md,questions.md,contracts.md,core-api.md}` for
  as-built changes.
- Your required-cases section.

Do not edit app crates, release docs or the README. If you need one, stop and
report.

## Done

Both tiers pass. Evidence at `docs/runs/run-006/evidence/R6-gate.md`
includes the test-mode API and a list of every remaining user-facing
refusal and layout reason, which R6-app needs. Reply with the SHA. Budget
about 1,800 AIU; stop and report at 2,300.
