# Run 005: close run-004 gaps, owner answers, first pre-release

Status: **approved** (D-27). Ceiling and owner answers are from D-26.

- Ceiling: 10,000 AIU, counted from run-005 start (cumulative baseline
  34,088.6, see [USAGE.md](USAGE.md)).
  - Stop launching new work at ≥9,500.
- Drafted 2026-11-21 by the Opus 5.5 orchestrator from
  [../run-004/REPORT.md](../run-004/REPORT.md),
  [../run-004/HANDOFF.md](../run-004/HANDOFF.md) and D-26.

## 1. Goals

1. **Uninstall removes and recovers settings (RV-04, A-10, R-07):**
   removal captures the add-on's saved settings into a recovery checkpoint,
   removes them with the code, and restore brings back both. Plan:
   [R4-integrate evidence](../run-004/evidence/R4-integrate.md) "RV-04 plan".
2. **Q-77 = 32 MiB (D-26):** `MAX_SETTINGS_FILE_BYTES = Some(33_554_432)`.
   Larger files are `TooLarge` for that unit. The Q-24 marker gate stays.
3. **Q-88 (a) (D-26):** after a live keep, a demonstrably newer version
   installed only on this computer stays installed and unshared. The kept
   tree is not reinstalled over it, and the decision stays open and visible
   until the user answers.
4. **DataAtRisk producer (A-27, ops.md §3):** the runtime raises
   `DataAtRisk` for the conditions ops.md names, and the app shows it.
5. **Diagnostics follow-ups:** RV-09 (file fingerprints, build commit),
   removal of the obsolete `Command::SaveDiagnostics`. Saving never silently
   overwrites an earlier export.
6. **GUI correctness:** Keep sends the decision's real reason and
   allow-again scope, not `Ambiguous` with an empty list.
7. **Hygiene:** RV-10 (E-06 licence/maintenance notes, `--locked` in
   xtask), RV-11 (absolute macOS helper paths).
8. **First pre-release `v0.1.0` (D-26):** Windows x64, marked pre-release,
   built by a tag-triggered workflow with the build commit set, and release
   notes that state the known gaps, including the Q-24 synthetic-only gate.
9. **Independent checks:** GUI recheck of the V-02 group fix (`2cc8858`) and
   RV-01, plus everything above, then an E-06/E-08 review.

## 2. Non-goals

- No installer (A-35), no code signing, no macOS binary in the release.
- No real-data enrollment: the Q-24 marker gate stays.
- No Dropbox API. No new network calls beyond the D-24 Releases check.
- No new dependencies unless a packet justifies one under E-06 and
  `cargo deny` passes.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R5-remove | Implementation / claude-sonnet-5.5 | RV-04 and the Q-77 cap. Core `removal.rs`, `settings.rs`, `exec.rs`, `records.rs` and related tests; runtime removal and restore wiring |
| A | R5-decide | Implementation / claude-sonnet-5.5 | Q-88 (a), DataAtRisk producer, RV-09, drop `Command::SaveDiagnostics`, RV-11 in `adapters/src/sysinfo.rs`. Core decision/choice/notice code; runtime cycle, notices, diag, control |
| A | R5-app | Implementation / claude-sonnet-5.5 | Keep reason fix, no-overwrite diagnostics save, RV-11 in `app/src/platform.rs`; then (A2) GUI for Q-88 and DataAtRisk once the core API merges |
| A | R5-release | Implementation / claude-sonnet-5.5 | `.github/workflows/release.yml`, version 0.1.0, release notes, RV-10, check the update notice when only a pre-release exists |
| B | R5-check | Verification / gpt-6.1-sol | Independent GUI drive and negative cases for goals 1–6 and 9; smoke test the release artifact; memory re-run |
| C | R5-review | Integration / claude-opus-5.5 | E-06/E-08 review; the orchestrator writes the report |

R5-remove and R5-decide both edit `runtime/src/service.rs`. They keep
those edits small and local. The orchestrator merges the first one that
finishes; the second runs `git merge main` and resolves its own conflicts
before handing back. Every packet appends to `tests/required-cases.toml`
only inside its own section markers.

The orchestrator pushes the `v0.1.0` tag only after R5-check passes the
artifact smoke test. The tag push is the publishing step approved by D-26.

Planned workers: 6. Up to 4 more for fixes and rechecks under the D-05
limits: 5 concurrent, 20 total, no nesting, 2 retries per task.

## 4. Budget estimate

| Item | AIU |
|---|---|
| R5-remove | 1,800 |
| R5-decide | 1,300 |
| R5-app (A and A2) | 1,300 |
| R5-release | 500 |
| R5-check | 800 |
| R5-review | 400 |
| Orchestrator | 1,500 |
| Fix chain and retry headroom | 1,400 |
| **Total** | **9,000** |

## 5. Stop conditions

- Stop at ≥9,500 AIU, at an owner barrier, or on any unapproved scope,
  credential or user-data need.
- **Stop all work** if a worker breaks a COMMON rule.
- Every `main` push is followed by a CI watch.
- No tag is pushed unless the release workflow, both tiers and the
  artifact smoke test pass.

## 6. Owner questions for this plan

None. D-26 answered the run-004 barriers. A new product question found
during the run is recorded in `docs/design/questions.md`, and work that
depends on it stops.

## 7. Packet rules

[packets/COMMON.md](packets/COMMON.md).
