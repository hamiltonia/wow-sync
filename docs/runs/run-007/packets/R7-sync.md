# R7-sync: header-only `.toc` reading and syncing unrecognised add-ons

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Worktree `D:\GitHub\wow-sync.worktrees\R7-sync`, branch `run007/R7-sync`.

## Why

On the dev PC, QuestieDB's `QuestieDB_Forever.toc` and `QuestieDB_Camelot.toc`
are 17.6 MB each. The 256 KiB cap makes the folder non-transfer-eligible:
- `fs_scan.rs` `read_bounded` takes `max_toc_bytes`;
- `toc.rs` `TOC_LIMIT` and `parse_toc` return `TooLarge`;
- `inventory.rs` `FolderState::transfer_eligible` requires
  `TocKind::Selected`.

D-29 says to sync everything. SPEC R-05 (amended) says how:
- add-ons with an unrecognisable description sync by content lineage;
- a one-time decision is raised only when ordering cannot be shown;
- oversized `.toc` files are recognised from their header.

## Goals

1. **Header-only toc (PLAN goal 2, A-48).** A `.toc` over the cap is read up
   to the cap. Parse only the complete lines in that prefix: drop the trailing
   partial line, so a cut UTF-8 sequence cannot cause `NotUtf8`.
   - Do this wherever a toc is parsed: `toc.rs` (`FileObs`, `parse_toc`),
     `fs_scan.rs`, `runtime/effects.rs` ~344 (recovery version), and
     `runtime/detail.rs` ~123.
   - Never read a whole oversized file into memory.
   - The tree hash still covers the full file content, as it does today.
   - Tests:
     - a synthetic 1 MB+ toc whose header comes first: Selected, with the
       version and SavedVariables read;
     - a cut through a multi-byte character;
     - a file whose header lines sit beyond the cap: the result is not
       Selected, and goal 2 handles it.
2. **Unrecognised add-ons sync (PLAN goal 3, A-48).** A folder with
   `status == Ok` whose toc kind is `None`, `Ambiguous`, `Unvalidated` or
   `Error` becomes transfer-eligible as unrecognised.
   - Pick a representation, for example a new `TocKind::Unrecognised`, or an
     eligibility rule plus a flag. It must stay visible in the status
     snapshot and detail, labelled "Unrecognised: synced by content", with the
     original reason.
   - `Unreadable`, `Unsafe`, `Unstable` and `Unsupported` folders stay
     blocked.
   - **Ordering (SPEC R-05, no timestamps, no machine preference):**
     - identical trees converge;
     - when exactly one side changed from the last shared release that the
       others still hold, the changed copy wins automatically;
     - in every other disagreement (first join with different copies, or both
       sides changed), raise one ChooseCopy decision for that package, with
       the alternatives preserved. Reuse the existing ambiguity path
       (`package.rs` `AmbiguityReason`, `target.rs` `NeedsDecision`) and add a
       reason such as `UnrecognisedDiffers` if needed.
   - Check that the existing release/lineage records already give "the last
     shared release". If they cannot, stop and report rather than invent
     timestamps.
   - Removal, restore, rollback, the three-state history, first-run safety
     (D-28) and `inventory_complete` behave as for recognised add-ons. Add e2e
     tests: two computers; an unrecognised add-on only on A; identical on
     both; changed on A only; changed on both; and removal then restore.
   - Multi-folder packages: an unrecognised folder is its own package, unless
     the existing hard-dependency grouping already places it in one.
   - Settings: report how ownership treats an unrecognised add-on today. Do
     not add new settings ownership; if it is unsynced, say so in your
     evidence for the release notes.
3. **Docs.** Update as built:
   - `docs/design/domain.md`: toc reading and eligibility;
   - `docs/design/questions.md`: Q-3, Q-20, Q-22, and a new entry for D-29;
   - `docs/design/contracts.md` and `core-api.md` if types change.
   Remove statements that unrecognised add-ons are never synced.

## Files you own

- `crates/wow-sync-core/src/**` (all).
- `crates/wow-sync-adapters/src/{fs_scan.rs,testkit.rs}`.
- `crates/wow-sync-runtime/src/**`, except `discover.rs`, which belongs to
  R7-setup.
- `crates/wow-sync-app/src/{features.rs,model.rs,removed_ui.rs}`, only for the
  user text of the new status and decision reason. Keep edits small. Do not
  touch `pages.rs`, `setup.rs`, `main.rs` or `app.rs`; R7-setup owns them.
- `docs/design/**` and your required-cases section.

## Done

Both tiers pass. Evidence at `docs/runs/run-007/evidence/R7-sync.md` covers:
- the representation chosen;
- the ordering rules, with test names;
- the settings finding;
- the user text, which R7-setup needs for the release notes.

Reply with the SHA. Budget about 1,000 AIU; stop and report at 1,300.
