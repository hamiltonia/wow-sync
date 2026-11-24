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
