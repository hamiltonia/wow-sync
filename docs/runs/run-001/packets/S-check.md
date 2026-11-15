# Work packet S-check: independent verification of Phase 2 spikes

Role: independent verification. Model: gpt-6.1-sol (D-04). Common rules:
[S-common.md](S-common.md) (data/safety rules apply in full). ID `S-check`.

## Intent

PLAN §3 assigns gpt-6.1-sol verification to S-mem, S-forever and S-dropbox.
Also spot-check that every Phase 2 evidence file labels claims honestly
(PASS/FAIL/BLOCKED/NOT RUN) and that committed data respects D-08/D-09/D-14.
Authors' reports are not evidence.

## Checkout

- Read-only inputs: `main` (all merged spikes) and the uncommitted report
  `D:\GitHub\wow-sync.worktrees\S-forever\docs\runs\run-001\evidence\S-forever.md`
  (read only; do not copy its values into your evidence beyond what is needed
  to state agreement/disagreement).
- Writes: worktree `D:\GitHub\wow-sync.worktrees\S-check`, branch
  `run001/S-check`, only `docs/runs/run-001/evidence/S-check.md` (+ optional
  `evidence/S-check/` small raw data).

## Work

1. **S-mem:** from `spikes/mem/` on `main`, rebuild release and rerun at
   least scenarios b and c (and a if cheap) with the committed harness, on
   an otherwise quiet machine (no other cargo builds; record load). Compare to
   the committed numbers; confirm the counter really is private resident
   memory and that conclusions vs 100 MiB follow from the data.
2. **S-forever:** independently read the same installation metadata
   (read-only, D-09) and confirm or refute the product code / flavor / version
   claims and the identity-derivation recommendation. Check the report for
   personal data (account/Battle.net ids, user names, emails).
3. **S-dropbox:** rerun the safe parts of `spikes/dropbox/observe.ps1` (only in
   `D:\Dropbox\Gaming Sync\WoW Sync Test`, synthetic data, clean up after;
   use exact paths, never enumerate other Dropbox folders) — at least
   placeholder attribute transition and temp-then-rename visibility.
4. **All Phase 2 evidence** (`evidence/S-*.md`): check each headline claim's
   label against the data shown; flag any overclaim (e.g. synthetic results
   presented as real-client results, build-only presented as runtime).
   Check committed files for account/realm/character names (compare against
   the dev PC WTF folder names without printing them), user names, SIDs,
   secrets.
5. Commit (`run001(S-check):` prefix, Copilot trailer). Do not push or fix
   defects; report them.

## Stop

No sub-agents. Two failed attempts at a step: report BLOCKED.
