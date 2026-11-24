# R9-decide evidence

Goals 1–2 of [../PLAN.md](../PLAN.md). Branch `run009/R9-decide`.

## Root causes

1. **Conflict offered no options.** The two real records are valid, live keeps over
   `hideanything`, and each excludes the other's package. `ChoiceView::excluded`
   (`crates/wow-sync-core/src/choice.rs`) returned `Same` for both copies, so
   `package_copies` (`target.rs:~780`) dropped every copy as "excluded". `conflict_item`
   (`decide.rs:~326`) then had no copy to build a keep from, so `planned` was empty and it
   returned `None`; `apply_constraints` (`target.rs:~925`) still raised
   `NeedsDecision{ChoicesDiffer}`, so the app showed only "Decide later".
   `build_keep`, `CopyEntry::from_copy` and `validate_choice` were not at fault.
2. **Second click wrote a record unaware of the first.** `Command::Decide`
   (`crates/wow-sync-runtime/src/service.rs:~1146`) judged the click against `self.offered`,
   which is replaced only by a full cycle. After the first write the old item, its evidence and
   its pre-built plan (built over the older ledger and sequence) stayed valid, so a second click
   wrote a second record whose `known` lacked the first.

## Fixes

- `ChoiceView::excluded` skips an exclude entry that names the release another live keep over
  the package pins (`kept_by_rival`). Competing keeps no longer hide each other's copies, so
  `decision_items` offers a keep of each copy plus keep-on-this-computer. A chosen copy's
  `known` covers both records (`known_for`), leaving one live choice; the kept copy is never in
  its own exclude list.
- `Command::Decide` re-judges against the fresh observation (`w.offers`): if the group now
  offers the item, the effect comes from that current plan and a changed item is refused as
  `Changed`. After any successful decide the item is dropped from `self.offered`, so a second
  answer on the stale view is refused as `Changed`.

## Tests (each fails without its fix)

- `decide_tests::conflicting_keeps_do_not_hide_each_others_copies_and_a_choice_supersedes_both`
  (core; failed at the `excluded` assertion before the fix).
- `e2e::tests::mutually_excluding_keeps_are_decided_once_and_a_second_click_is_refused`
  (two synthetic computers; mutual-exclude records; fails with "conflict offers a decision"
  without fix 1 and with `Written` instead of `Changed` without fix 2; after the first answer
  both computers converge and neither shows `ChoicesDiffer`).
- Required-cases section `R9-decide` added.

## App hook

None required. `Command::Decide` already returns `CommandError::Decide(DecideError::Changed)`
for the second click. The app should treat `Changed` as "refresh the decision list";
`offered_decisions()` no longer lists an answered item immediately.

## Not run

The real group under `D:\Dropbox\WoW Sync` was only read to establish the record shape; no
live instance was started.

## Round 2: RYZENFISH panic (`choice.rs:374` in v0.3.1)

**Root cause.** `NewVersion::kept_copy` was `self.absent_kept.as_ref().unwrap_or(&copies[self.kept])`.
`unwrap_or` evaluates its argument eagerly. When the kept release is not among the copies,
`new_versions` sets `absent_kept = Some(..)` and `kept = copies.len()`, so `copies[kept]` indexed
one past the end ("len is 1 but the index is 1") even though the stand-in was about to be used.
RYZENFISH hit it because the rival keep excluded its own kept copy, leaving one copy listed; every
pass called `decision_items` → `new_version_item` → `kept_copy`. The round-1 `kept_by_rival` change
keeps both copies listed so this path is not reached for that shape, but the panic was reachable
for any live keep whose release no copy holds (Q-88).

**Fixes.**
- `kept_copy` returns `Option` and never indexes eagerly; `NewVersion::local_only` and
  `NewerAfterRollback::local_only` use `get`; `dismissal_key` skips unknown indexes.
- `new_version_item` and `rollback_item` (`decide.rs`) return an error / `None` instead of indexing
  `copies`/`shown`/`recovered`; `withheld_local_versions` (`control.rs`) skips an unknown index.
  A remaining `copies[i]` in the sync path is only over indexes produced from the same list in the
  same call (`reject.rs` `recovered`, `decide.rs` conflict loop `shown[i]`).

**Tests.**
- `decide_tests::a_keep_whose_release_no_copy_holds_still_builds_its_item_without_panicking`
  (absent kept release with one copy: the exact len-1/index-1 shape; mismatched indexes yield
  `None`/empty/error).
- `e2e::tests::the_holder_of_one_mutually_excluded_copy_syncs_offers_both_and_converges_when_it_decides`
  (three synthetic computers, real record shape, decided from the holder of one copy; all converge
  and none still shows a decision).

## Round 3: stale Decide and Resume-vs-Keep (RV9-01, RV9-04)

Root causes
- RV9-01: Command::Decide (service.rs) fell back to the cached self.offered plan when the fresh observation no longer held the item, so a peer's settlement was ignored and a record with a stale issued_at_seq was written (ChoicesDiffer again). An answer made while paused stayed in the offers after resume.
- RV9-04: kept_by_rival let a live Keep silently beat a live Resume's exclusion; competing intentional choices must not be merged.

Fixes
- Decide re-reads the ledger from the provider (observe_world) and judges only against the fresh offers; a missing item or different evidence token returns DecideError::Changed. No stale fallback.
- hide_answered removes offers whose scope matches a queued (paused) answer, in command and in sync_once; the answered item is also dropped after a successful Decide.
- New hook ConstraintHooks::choices_conflict (ChoiceView::resume_excludes_a_live_keep); pply_constraints raises ChoicesDiffer for it. xcluded stays lenient so both copies remain listed.

Tests: 2e::tests::a_stale_answer_after_a_peer_settled_the_conflict_writes_nothing (fails without the service.rs fix: record written instead of Changed), 2e::tests::an_answer_kept_while_paused_is_written_once_and_not_offered_again, decide_tests::a_live_resume_excluding_what_a_concurrent_keep_pins_is_a_conflict_not_a_silent_winner. The old synthetic-offer test decide_command_is_checked_for_freshness_and_kept_while_paused was replaced because Decide now only accepts items present in the fresh world; unknown option, stale token and paused cases are covered by the new paused test.

App hook: none; treat DecideError::Changed as "refresh".

## Round 4: untested guard and one-pass re-offer (RV9-12, RV9-11)

- RV9-12: new 2e::tests::a_paused_answer_stays_hidden_after_a_pass_and_is_not_offered_after_the_resume_pass follows the reviewer's probe: pause, answer (Kept), run a pass while paused, assert the item is not offered and two more answers are refused as Changed, then resume, one pass, assert one extra record.
- RV9-11: sync_once now captures the queued answer scopes before settle_kept writes them and hides those scopes from that pass's offers (queued_scopes / hide_scopes); previously the queue was already empty when hide_answered ran, so the item showed for one more pass. Clean, no behaviour change otherwise.
- Mutation results (temporary, restored; git diff shows only the intended change):
  - M1 hide_scopes made a no-op: the new test FAILS ("a pass while paused re-offered the answered item").
  - M2 pre-settle hiding removed: the new test FAILS ("the pass that wrote the answer listed it once more").

## Round 5: command dispatch no longer re-scans (G1)

Root cause: `Command` dispatch, `status()` and diagnostics each called `observe_world`, a full scan with the pass's read control (interrupt disarmed by then). Files whose cache entry was missing were re-hashed, a pending "check all files" request made every file a miss, and a complete command scan cleared that request. Diagnostics scanned twice (`status()` plus its own look).

Fix
- `ScanCtl.routine` (adapters): a routine read uses the hash cache without honouring a pending full request and does not clear it (`HashCache::lookup/finish` take the flag). Unchanged files are hits (size + mtime, D-31); the request stays for the next pass.
- `observe_for_command(reuse_local)` in `service.rs` is used by `command()`, `status()`, diagnostics and `re_enroll`. With `reuse_local` it answers from the newest complete scan and does not touch the install: ExportDiagnostics/SavePreviewedDiagnostics (one look, not two), Retire, AllowAgain, SetSettingsPolicy, ReEnroll. Every other command, including Decide, re-reads peers' ledger records and looks at local files only through the cache, so RV9-01 stays closed and a changed local file still changes the evidence token (-> `Changed`).
- Cost: one extra copy of the last scan (file paths + hashes) is held in memory.

Tests (each fails with the mutation `routine: false` + no reuse; run and restored)
- `e2e::tests::a_command_reads_through_the_hash_cache_and_leaves_a_pending_full_check_for_the_pass`: zero cache misses during status and Decide; the full request is still pending after them and cleared by the next pass.
- `e2e::tests::diagnostics_and_ledger_only_commands_answer_from_the_last_scan`: cache hits+misses unchanged across ExportDiagnostics and AllowAgain.

Not measured: the live 1 s gate on a 3 GB tree (needs the owner's/checker's BigLab); changed files in a command's look are still hashed once. backend.rs untouched.