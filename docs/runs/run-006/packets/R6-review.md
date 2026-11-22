# R6-review (integration review, claude-opus-5.5): phase C

Read [COMMON.md](COMMON.md), `AGENTS.md`, SPEC.md and D-28 first. Review
the run-006 changes on `main` from `c77d050` up to the commit named in your
prompt. Read-only: do not change code, tests or docs. Write only:
- `docs\runs\run-006\evidence\R6-review.md`;
- your worktree `D:\GitHub\wow-sync.worktrees\R6-review` (branch
  `run006/R6-review`), for that file.

## Focus

1. **Gate removal (Q-24, D-28):** is every production call site on the
   default `NotRequired` path? Does test mode really enforce `Required`?
   Do identity, scope, variant and account/character boundaries still hold
   for real installs? Could an unfamiliar client enroll without guided
   validation?
2. **Forever toc rule:** does it match D-28 and domain.md §3? Check:
   - disagreement is `Ambiguous`;
   - other suffixes are ignored;
   - unfamiliar products stay plain;
   - every `select_toc` caller uses the variant mapping;
   - the reconfirmation of existing enrollments is safe and visible.
3. **First run loses nothing:** do the tests prove the claim for every
   first-sync path? Look for any delete or overwrite without a recovery copy.
4. **User text:** no internal identifiers in user-visible text. "Try again"
   appears only when it can work.
5. **Release notes and docs** are accurate against the code and R6-check
   evidence. Stale "synthetic only" or Q-24 statements are removed.
6. **E-06/E-08 hygiene:** the `url` update, new dependencies, and the
   required-cases changes.

Give findings IDs RV6-NN with severity (critical, major, moderate, minor),
the file and line, and a suggested fix. No critical or major finding may
remain open before the tag. Commit `run006(R6-review): …` and reply with the
SHA. Budget about 500 AIU; stop and report at 700.
