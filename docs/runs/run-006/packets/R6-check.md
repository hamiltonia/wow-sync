# R6-check (verification, gpt-6.1-sol): phase B, live dev-PC verification

Read [COMMON.md](COMMON.md) and `AGENTS.md` first. You are an independent
verifier: re-run everything yourself and never trust the authors' claims.
The authors' evidence is in `docs\runs\run-006\evidence\`: R6-gate, R6-app
and R6-release.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\R6-check`, branch
  `run006/R6-check`, from the `main` commit named in your prompt.
- **Owned paths:**
  - `docs\runs\run-006\evidence\R6-check.md`;
  - `evidence\R6-check\`, under 1 MB, redacted;
  - your required-cases section, left empty.
- **Do not change** product code, tests, tools or other docs. Negative
  controls are temporary edits; revert them and show `git status` clean.
- **Lab root:** `D:\WoWSyncLab\R6-check\`.
- **Network:** `gh` is allowed read-only for checks 1 and 8.

## Real-data protocol (D-14, D-28). Breaking it stops the run

1. **Before any app touches it,** copy
   `D:\Games\World of Warcraft\_forever_\Interface\AddOns` and
   `...\_forever_\WTF` to `D:\WoWSyncLab\R6-check\backup\`. Record a
   SHA-256 manifest of both trees: relative path, size, hash. Keep the
   manifest in the lab, not in the repository.
2. Confirm WoW is not running. Never start, kill or automate WoW or
   Battle.net.
3. **At the end,** stop every app instance you launched (by PID). Restore
   both trees from the backup: delete what the app added, copy the backup
   back. Then prove that a fresh manifest equals the original. Record the
   counts and the equality result in the evidence.
4. **Provider:** use
   `D:\Dropbox\Gaming Sync\WoW Sync Test\R6-check-<date>` (real data
   allowed, D-28). Never touch other `Gaming Sync` entries. Delete your
   provider folder at the end unless a finding needs it kept; say which.
5. **Evidence redaction:** add-on folder names and counts are fine. Never
   record account, realm or character names, file contents, user names or
   profile paths. Redact screenshots that show them.

## Checks

1. **Tiers and CI:** run the offline tier once and the native tier three
   times, with logs. Record the latest `main` CI run status with `gh`.
2. **Real enrollment, computer A:** run the built app (release profile)
   with an isolated app-data directory. Enroll the real `_forever_` through
   the GUI with no opt-in. Check:
   - plain-language identity and scope steps;
   - eligible and blocked counts, compared with your own count computed
     from the toc rule in domain.md §3 (D-28). Explain every difference.
     Blocked add-ons show readable reasons.
3. **Second computer B, simulated:** copy the backup to
   `D:\WoWSyncLab\R6-check\B\World of Warcraft\_forever_` with its
   `.build.info` and `.flavor.info`. Make B differ from A: remove some
   add-ons, add one local-only synthetic add-on, change one version. Run a
   second app instance with its own app-data directory, and enroll B into
   the same group.
4. **First run loses nothing (D-28):** after B's first sync, show the
   following, with hashes:
   - every add-on that B lost or had replaced can be restored through the
     GUI;
   - the local-only add-on was kept and published to A;
   - A gained nothing destructive.
5. **Sync, removal, restore on real data:**
   - update an add-on on A (edit its toc `## Version` and one file) and see
     it reach B;
   - uninstall an add-on on A and see it removed on B, settings included;
   - restore it from Removed on B and confirm code and settings come back
     with matching hashes;
   - WoW-running blocking: use the documented test mechanism the earlier
     checks used, if one exists; otherwise mark NOT RUN.
6. **Test mode:**
   - turn it on and confirm the unmarked real install stops syncing, with
     a readable reason;
   - confirm a marked synthetic install still works;
   - turn it off and confirm sync resumes;
   - check that setup with test mode on refuses the real install with
     readable text and a "Turn off test mode" offer.
7. **Refusal screens:** reproduce the owner's original screen on purpose
   (test mode on, real install). Screenshot it, redacted, and confirm there
   are no internal identifiers, no blank heading and no useless "Try again".
8. **Artifact:** once the orchestrator names a dispatch run, run the
   smoke test: checksum, contents, notes byte-identical to
   `docs/release/v0.2.0.md`, About shows 0.2.0, and `build_commit` matches.
9. **Restore and prove** per protocol step 3.

Label each check PASS / FAIL / BLOCKED / NOT RUN. Give findings IDs (G1, G2,
…) with severity and reproduction steps. Commit the evidence
(`run006(R6-check): …`) and reply with the SHA. Budget about 1,200 AIU;
stop and report at 1,600.
