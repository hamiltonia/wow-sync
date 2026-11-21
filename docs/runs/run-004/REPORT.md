# Run 004 report: removal, settings, rollback, retire, decisions, notices, diagnostics

Status: **closed** (2026-11-21) after phases A to E and the verification fix
rounds. It is not a release and still uses synthetic data only (Q-24 marker
gate).

- Usage: about 16,500 run-004 AIU at close (cumulative 33,909.6); see
  [USAGE.md](USAGE.md). Ceiling 20,000 AIU (D-23/D-24); stop line 19,000 was
  not reached.
- Workers: 15 of 20 used.
- Orchestrator: claude-opus-5.5, session `961f3baa`.
- Plan: [PLAN.md](PLAN.md) (D-24). Owner barrier: D-25 (defaults accepted).
- State and resume point: [HANDOFF.md](HANDOFF.md).

## 1. Outcome

The Windows app now carries the run-004 product features end to end on
synthetic data, driven through the real GUI by an independent verifier:

- removal that propagates and is recoverable (code; see RV-04 for settings);
- rollback that rejects a release for the group, with refusal when the
  earlier files are gone and local recovery restore through the GUI;
- actionable decisions (choose, keep, resume, allow again) with distinct
  accessible names;
- retire and explicit re-enrollment, with no silent settings-master election;
- settings sync model, consent with an account list, master, exceptions,
  pause/resume and recovery, **disabled on real data until Q-77 has a value**;
- OS notifications, an update-check notice and a previewed, redacted
  diagnostics export;
- a macOS owner kit (`MAC-OWNER.md`, `tools/mac/`), not yet run.

A-29 memory on Windows x64: PASS, 37.26 MiB aggregate, sync start 2.28 s
(R4-check). CI is green on Windows and macOS (offline tier) and on all four
build-only targets.

## 2. What was built

| Area | Result | Evidence |
|---|---|---|
| Design | removal (closes R3D-13 under D-23), settings, rollback, ops; independent review PASS on recheck 4 | [R4-design-recheck4](evidence/R4-design-recheck4.md) |
| Core | contracts foundation, removal, settings, ops (retire, decisions, typed scope) | [R4-core-contracts](evidence/R4-core-contracts.md), [removal](evidence/R4-core-removal.md), [settings](evidence/R4-core-settings.md), [ops](evidence/R4-core-ops.md) |
| Runtime integration | authority plane, commands, settings apply (test-only cap), rollback staging and refusal, recovery points, held dates, accounts, diagnostics fill and previewed save | [R4-integrate](evidence/R4-integrate.md) |
| Notices and diagnostics | OS notifications, update check, diagnostics export | [R4-notify-diag](evidence/R4-notify-diag.md) |
| GUI | all run-004 screens wired to the runtime snapshot and commands | [R4-app](evidence/R4-app.md) |
| macOS kit | build, bundle, run and memory scripts plus owner steps | [R4-mac-kit](evidence/R4-mac-kit.md), [MAC-OWNER.md](MAC-OWNER.md) |
| Verification | R4-check (12 findings), recheck, recheck 2; R4-review (11 findings) | [R4-check](evidence/R4-check.md), [R4-review](evidence/R4-review.md) |

## 3. Verification findings and their state

R4-check V-01..V-12:

- **Closed by an independent recheck:** V-03, V-04, V-05 (consent), V-06
  (OS, memory, journals), V-07, V-08, V-10, V-11, V-12 (procedure).
- **V-02:** local recovery restore through the GUI PASS. The group did not
  follow in recheck 2; fixed by `2cc8858` (rollback.md §3.2), covered by an
  e2e test, **not rechecked through the GUI**.
- **V-09:** named versions PASS. A release held only by another computer has
  no date, because no shared record carries one (accepted gap).
- **V-01:** a design gap, now owner question **Q-88**.

R4-review RV-01..RV-11:

- Fixed and merged, not independently rechecked: RV-01 (save writes the
  previewed bytes; runtime `32503e0`, app `a2d24fe`), RV-02 (redaction context), RV-03 (log names only the
  action kind), RV-05 (same-version confirmation says the current copy is
  blocked for the group), RV-08 (real-release negative test).
- Fixed by the orchestrator: RV-06 (README, REPO-MAP), RV-07 (ledger).
- **Open:** RV-04 (uninstall does not capture or remove settings; plan in
  [R4-integrate](evidence/R4-integrate.md) "Pass 17"), RV-09 (diagnostics
  fingerprints and build commit), RV-10 (dependency notes, `--locked` in
  xtask), RV-11 (absolute paths for macOS helpers).

## 4. Not done or blocked

- **Real settings sync:** disabled until Q-77 has an approved value.
- **Uninstall settings capture (RV-04):** not built; A-10 settings half open.
- **macOS runtime:** NOT RUN; needs the owner to run the kit.
- **Update notice against a real release:** the live check returned 404
  because no GitHub release exists.
- **DataAtRisk notice producer:** deferred.
- **Diagnostics save:** fixed Documents path, no picker; a new export
  overwrites the previous one.
- **GUI Keep metadata:** sends `ChoiceReason::Ambiguous` with an empty
  `allow_again` (minor against ops.md §1.5).
- Not in scope: installer (A-35), real-data enrollment, Dropbox API,
  telemetry, A-31 budgets.

## 5. Incidents

- Workers twice committed a temporary A-38 mutation by running the edit and
  the commit in one batch; both were caught and corrected in the next
  commit. Packets now require separate steps.
- One worker used `Add-Content` for a blank line in an evidence file;
  disclosed, harmless.
- R4-check's first pass did not enforce temp/network isolation (V-12);
  rechecks enforced it from the start.

## 6. Owner input needed

1. **Q-88:** newer local install after a keep (options in
   [questions.md](../../design/questions.md)).
2. **Q-77:** settings-file size cap value.
3. **macOS:** run [MAC-OWNER.md](MAC-OWNER.md).
4. **First GitHub release** for the update notice.
5. A run-005 plan and credit ceiling, including RV-04 and a GUI recheck of
   the V-02 group fix and RV-01.
