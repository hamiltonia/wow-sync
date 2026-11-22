# Run 006: real installs, Forever toc rule, plain refusals, `v0.2.0`

Status: **approved** (D-28). Ceiling 10,000 AIU from run-006 start, stop
launching new work at ≥9,500. Baseline in [USAGE.md](USAGE.md).

- Drafted 2026-11-22 by the Opus 5.5 orchestrator. Trigger: the owner ran
  `v0.1.0` on the dev PC's `_forever_` install and setup stopped on "This
  folder cannot be used: NotSyntheticInstall" with no usable explanation.
- Inputs: D-28, [../run-005/REPORT.md](../run-005/REPORT.md),
  `docs/design/domain.md` §2.1 and §3, questions Q-3, Q-20, Q-22, Q-24, Q-68.

## 1. Goals

1. **Real installs enroll without opt-in (Q-24, D-28).** `validate_variant`
   no longer requires `.wowsync-synthetic`. Identity, layout, guided
   confirmation and every other check stay.
2. **Forever toc rule (Q-3, Q-20, D-28).** For `wowf` the mapping becomes the
   profile rule of domain.md §3, extended with the `Mainline` family suffix:
   - first the game-type suffixes `Forever` and `Camelot`;
   - then `Mainline`;
   - then the plain `F.toc`;
   - other suffixes are ignored.
   Disagreement among the game-type candidates is `Ambiguous`. Unfamiliar
   products keep `validated-plain/1`. Existing enrollments whose layout
   digest changes go through the existing reconfirmation flow, without data
   loss.
3. **First run loses nothing (D-28).** Joining a populated group, or starting
   a new group from a populated install, never deletes or overwrites anything
   without a GUI-restorable recovery copy. This includes local add-ons that
   the group does not have, local add-ons the group has removed, and older or
   different local versions. Prove it with tests. Fix any gap that is found.
4. **Plain-language setup.** No internal identifiers such as
   `NotSyntheticInstall` or `{:?}` output in user text (cause:
   `runtime/src/service.rs` ~1618).
   - Every refusal and layout reason has a sentence that says what is wrong
     and what to do.
   - "Try again" appears only when an outside fix can change the result.
   - The variant card shows a heading, not a blank line.
   - Blocked add-ons are listed with a readable reason.
5. **Test mode (nice-to-have, D-28).** A setting, off by default, restores
   the marker gate so only marker-carrying test folders can be enrolled. The
   setup text explains the refusal when it applies.
6. **Live verification on the dev PC (D-14, D-28).** An independent verifier
   enrolls the real `_forever_` install through the GUI, after a hashed
   backup. It checks:
   - eligibility counts;
   - sync to a second computer simulated by a copy under `D:\WoWSyncLab`;
   - removal and restore with settings;
   - first-run safety;
   - test mode.
   The provider is under `D:\Dropbox\Gaming Sync\WoW Sync Test`. Afterwards
   the verifier restores the backup and proves the restore by hash. Evidence
   is redacted.
7. **Pre-release `v0.2.0`.** Version 0.2.0 and release notes that cover:
   - real-install support;
   - the Forever toc rule and its documentary source;
   - test mode;
   - the "make a backup" advice;
   - the remaining gaps.
   The release is published only after the gates in §3.

## 2. Non-goals

- No installer, signing, macOS binary or Dropbox API.
- No other WoW flavors beyond the existing unfamiliar-product path.
- No execution of add-on Lua or saved-variable files. No WoW automation.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R6-gate | Implementation / claude-sonnet-5.5 | Goals 1–3 and the core/runtime half of 5: `variant.rs`, `toc.rs`, first-run planning, runtime wiring, design docs |
| A | R6-app | Implementation / claude-sonnet-5.5 | Goal 4 and the GUI/settings half of 5 (stage A2 after R6-gate's API merges) |
| A | R6-release | Implementation / claude-sonnet-5.5 | Goal 7 (version, notes, README), devkit/testkit marker paths, existing-test updates outside core/runtime |
| B | R6-check | Verification / gpt-6.1-sol | Goal 6 plus negative cases; artifact smoke test of the release build |
| C | R6-review | Integration / claude-opus-5.5 | Safety review of goals 1–3 and 5 against SPEC/AGENTS rules before the tag |

`runtime/src/service.rs` is shared by R6-gate (gate/toc wiring) and R6-app
(refusal text). Keep edits small and local; the second to merge runs
`git merge main`. Gates for the tag:
- both verify tiers;
- green CI;
- a dispatch build;
- R6-check PASS on goal 6 and the artifact;
- R6-review with no open critical or major finding.

Planned workers: 5. Up to 5 more for fixes and rechecks (D-05 limits:
5 concurrent, 20 total, no nesting, 2 retries per task).

## 4. Budget estimate

| Item | AIU |
|---|---|
| R6-gate | 1,800 |
| R6-app (A and A2) | 1,200 |
| R6-release | 400 |
| R6-check | 1,200 |
| R6-review | 500 |
| Orchestrator | 1,500 |
| Fixes, rechecks, retries | 2,400 |
| **Total** | **9,000** |

## 5. Stop conditions

- Stop at ≥9,500 AIU, at an owner barrier, or on any unapproved scope,
  credential or user-data need.
- **Stop all work** if a COMMON rule is broken, or if any real dev-PC data is
  changed without a verified backup.
- Every `main` push is followed by a CI watch. No tag without the §3 gates.

## 6. Owner questions

None blocking.
- Optional: the owner may confirm Q-3 in the Forever client.
- Optional: the owner may correct the D-28 reading of "test opt-out".
New product questions go to `docs/design/questions.md`; dependent work stops.

## 7. Packet rules

[packets/COMMON.md](packets/COMMON.md).
