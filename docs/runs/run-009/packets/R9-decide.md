# R9-decide: conflicting choices and stale-evidence choices

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goals 1–2 in [../PLAN.md](../PLAN.md).

**You own:** `crates/wow-sync-core` (decide, choice, target, decision) and the
runtime decision paths in `crates/wow-sync-runtime`. **You do not own**
`crates/wow-sync-app` (R9-app has it). If goal 2 needs an app-side hook,
build the core or runtime API, then describe the exact call the app must
make in your evidence and reply. Do not edit app files unless a one-line
change is unavoidable, and if you do, say so.

## 1. Reproduce the owner's conflict

- Copy the two real records into a fixture shape (synthetic hashes are fine):
  `members\*\authority\3f8ec32b….json` and `b627adcb….json` in the real group.
  Both are `reason: ambiguous`, `action: keep`, over `hideanything`, from the
  same issuer at the same `issued_at_seq`. One keeps tree `af74…`, the other
  keeps `e445…`, each excludes the other, and neither lists the other in
  `known`.
- Write a failing test:
  - target gives `NeedsDecision { ChoicesDiffer }`;
  - `decision_items` returns no item for the package today, which is the bug.
- Find exactly why `conflict_item` / `build_keep` / `CopyEntry::from_copy`
  yield nothing: an error variant, `validate_choice` shape, missing copies,
  or `ChoicesDiffer` copies not being passed. Record it.

## 2. Fix goal 1

- `ChoicesDiffer` and `RollbackTargetsDiffer` items offer a keep of each
  distinct copy held by a member, plus keep-on-this-computer.
- Choosing a copy writes a keep whose `known` covers both conflicting
  records, so `live_choices` then has one live choice and the target becomes
  `Chosen`.
- Test end to end in the runtime e2e harness (two synthetic computers):
  1. both records exist;
  2. decide on computer A;
  3. both A and B converge, and B no longer shows a decision.
- Excludes inherited from the conflicting records must not exclude the copy
  being kept (check the `validate_choice` keep/exclude rule).

## 3. Fix goal 2

- Find why the second click on RYZENFISH wrote a record that does not know
  the first. Likely causes:
  - the `ChoiceView` or ledger used for `decide` is a snapshot from before
    the first write;
  - the `FreshEvidence` check does not include this computer's own newly
    written records.
- The fix must make the second write either refused as stale (the UI then
  refreshes) or a superseding record that lists the first in `known`. One
  computer must never produce two mutually unknown choices over the same
  scope. Test the double-click case.

## Done

- Both verify tiers pass. Add a required-cases section.
- Evidence: root causes with file and line references, failing-then-passing
  tests, and any app hook R9-app or the orchestrator must wire.
- Commit on `run009/R9-decide`, then reply with the SHA and a summary.
