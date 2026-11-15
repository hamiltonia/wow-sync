# Run 001: Bootstrap and feasibility

Status: **approved 2026-11-14 (D-15); ceiling 10,000 AIU (D-16).** Phases 0–3
done; report [REPORT.md](REPORT.md); S-forever report awaits owner review.
Current state: [HANDOFF.md](HANDOFF.md).
Prepared 2026-11-14 by the Opus 5.5 orchestrator against `ee880db`.
Authority: SPEC.md, ACCEPTANCE.md, ENGINEERING.md and
[../../OWNER-DECISIONS.md](../../OWNER-DECISIONS.md) (D-01..D-09, D-13).

## 1. Goal

Reach the E-09 "Feasibility/bootstrap" gate exit:

- A clean, pinned, reproducible repository with one verification entry point.
- Evidence and a recommendation for each SPEC §7 feasibility question that can
  be answered on the available machines, and explicit blockers for the rest.
- Proposed budgets and owner decisions for the next phase.

Acceptance targets in this run: A-37, A-38 (initial), A-39 (initial), A-40,
A-43 (initial), A-44. Feasibility touches R-01, R-02, R-05, R-07, R-11, R-13,
R-14 without claiming any product acceptance case.

## 2. Non-goals

- No product features, sync engine, protocol, snapshot format or real UI flows.
- Dev PC `_forever_` AddOns and WTF may be read/modified as test data (D-14);
  no Lua execution, no committed addon contents or saved settings.
- No access to `D:\Dropbox\Gaming Sync` outside `WoW Sync Test` (D-08).
- No telemetry service, paid service, new credentials or public release.
- No support claim for Windows ARM64 or macOS Intel (D-07).
- macOS runtime spikes wait for the M3 MacBook; they are reported as blocked.

## 3. Phases and work packets

Each packet becomes `docs/runs/run-001/packets/<id>.md` when launched, with the
full E-05 fields. Summary:

### Phase 0: Trial (the only work D-13 authorizes before a ceiling)

| ID | Role / model | Work | Owned writes | Done when |
|---|---|---|---|---|
| P0-trial | Implementation / claude-sonnet-5.5 | Install rustup and MSVC build tools if missing; add `rust-toolchain.toml` (pinned version, 4 targets), minimal Cargo workspace, `.gitignore`; `cargo build` and `cargo test` pass on dev PC | `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, `crates/` skeleton, `.gitignore` | Build/test independently rerun from a clean clone; actual model and credit usage recorded |

After P0 the orchestrator proposes the credit ceiling (D-13). **Stop for approval.**

### Phase 1: Bootstrap (after ceiling approval)

| ID | Role / model | Work | Owned writes |
|---|---|---|---|
| P1-verify | Implementation / sonnet-5.5 | Single verification entry point with tiers `offline`, `native`, `provider`; unavailable tiers report **blocked**, never pass; required-case inventory detects zero-test runs (A-39) | `xtask/` or `scripts/verify*`, `docs/VERIFY.md` |
| P1-ci | Implementation / sonnet-5.5 | GitHub Actions: fmt, clippy, tests, `cargo-deny` (licenses/advisories), docs/link check; build all 4 targets; minimal permissions, no secrets (E-06) | `.github/workflows/` (`deny.toml` is owned by P1-verify per its packet) |
| P1-map | Design / opus-5.5 | Repository map and architecture skeleton: platform-free domain core, single policy authority, platform-adapter boundaries (docs only) | `docs/ARCHITECTURE.md`, `docs/REPO-MAP.md` |
| P1-check | Verification / gpt-6.1-sol | Independently run verification from a clean clone; seed a broken/no-op test and a zero-test condition and show the gate fails (A-38, A-39); fresh-worker onboarding check (A-40) | `docs/runs/run-001/evidence/` only |

### Phase 2: Feasibility spikes (parallel, after P1-verify and P1-ci merge)

Spike code lives in `spikes/<name>/`, outside the product workspace, and may be
deleted after its findings are recorded.

| ID | SPEC §7 question | Role / model | Executable on | Output |
|---|---|---|---|---|
| S-gpui | GPUI window and tray lifecycle (hide/reopen/quit); build for 4 targets | sonnet-5.5 | Win x64; others build-only (labeled) | Findings and recordings |
| S-mem | Idle private resident memory after UI open/close cycles; GPU reported separately (R-14) | sonnet-5.5, verified by gpt-6.1-sol | Win x64 | Raw measurements, method |
| S-a11y | Screen-reader feasibility with GPUI (Narrator) | opus-5.5 design + sonnet-5.5 | Win x64; VoiceOver blocked | Findings and options (feeds D-12) |
| S-forever | Identity of `_forever_` from install metadata only (D-09) | sonnet-5.5, verified by gpt-6.1-sol | Win x64, read-only | Sanitized report; **owner reviews before commit** |
| S-addon | Addon/package identity and version ordering from `.toc` metadata | sonnet-5.5 | Dev PC `_forever_` addons (D-14) plus synthetic fixtures | Coverage report, ambiguity catalog |
| S-state | Whether addon-owned saved-variable files can be identified safely without executing Lua (R-07) | opus-5.5 | Synthetic fixtures only | Findings, unknown/shared cases |
| S-proc | Detect WoW/updater activity and file stability without killing or injecting (R-11) | sonnet-5.5 | Win x64 | Findings, race notes |
| S-dropbox | Local Dropbox client behavior: online-only files, partial files, conflicted copies, rename visibility | sonnet-5.5, verified by gpt-6.1-sol | `D:\Dropbox\Gaming Sync\WoW Sync Test` only, synthetic data | Observed provider behavior log |

### Phase 3: Integration and report

| ID | Role / model | Work |
|---|---|---|
| P3-review | Integration review / opus-5.5 | Review merged state against E-06/E-08 maintainability and dependency rules |
| P3-report | Orchestrator | `docs/runs/run-001/REPORT.md`: what ran where, pass/fail/blocked/not-run, decisions needed, next-phase budget proposal |

Planned workers: 1 + 4 + 8 + 1 = 14, within the 20-total limit (D-05).
Retries use the remaining headroom.

## 4. Integration and isolation

- Each implementation worker uses branch `run001/<packet-id>` in a dedicated
  worktree under `D:\GitHub\wow-sync.worktrees\<packet-id>`.
- Workers write only their owned paths. `Cargo.toml` and `Cargo.lock` belong to
  P0/P1; spikes do not modify them.
- The orchestrator does not edit code. Merges to `main` occur only after
  independent verification; conflicts return to the owning worker.
- Combined checks rerun on `main` after each merge.

## 5. Model and usage verification (E-01, A-37)

- Each worker is launched with an explicit model matching D-04.
- Per Copilot CLI 1.0.112 documentation, `subagents.maxConcurrency` and
  `subagents.maxDepth` are ignored on plans without usage-based billing.
  Concurrency, totals and no-nesting are therefore **enforced by the
  orchestrator**, not the harness, and tracked in `USAGE.md`.
- Actual model and credit use per worker are taken from Copilot session usage
  records, not from the requested model name. P0 confirms this works; if actual
  models cannot be established, the run stops (E-01).

## 6. Budget (D-05, D-13)

| Limit | Value | Enforced by |
|---|---|---|
| Concurrent workers | 5 | Orchestrator |
| Total workers | 20 | Orchestrator |
| Nesting | None | Orchestrator; packets forbid spawning agents |
| Retries per task | 2, each with a new hypothesis | Orchestrator |
| Elapsed time | None | — |
| AI credits | 10,000 AIU cumulative (D-16; was 2,500 under D-13) | Orchestrator; post-paid, soft |

Cumulative usage is recorded in `docs/runs/run-001/USAGE.md` after each worker
completes and survives session restarts. Under D-16: no intermediate pause;
stop launching at ≥9,500; in-flight work may settle up to 11,000.

## 7. Stop and escalation conditions

Stop, preserve state and ask the owner when:

- P0 completes (ceiling needed), or the D-16 stop point (≥9,500 AIU) is reached.
- A required model is unavailable or the actual model cannot be established.
- A task fails after 2 retries.
- An action would touch data outside D-08/D-09 scope, need new credentials or a
  paid service, or change approved product meaning.
- A feasibility result suggests an approved requirement may be infeasible
  (for example, GPUI cannot meet memory or tray needs). Report options; do not
  switch frameworks or drop targets.
- S-forever output is ready for owner review.

## 8. Owner prerequisites

| Item | Needed by | Status |
|---|---|---|
| Approve this plan | P0 | Done (D-15) |
| Credit ceiling | Phase 1 | Done: 2,500 AIU (D-13) |
| Addon dataset (D-10) | S-addon | Done: dev PC data (D-14) |
| M3 MacBook available to agents | macOS runtime spikes | Later; reported blocked |

## 9. Approval

| Date | Decision | Plan revision |
|---|---|---|
| 2026-11-14 | Approved by owner (D-15) | `baa81e3` |
| 2026-11-15 | Record-only updates by orchestrator: status, D-13 ceiling, `deny.toml` ownership matched to packet P1-verify. No change to scope, limits or meaning. | `4d1b14a` |
| 2026-11-15 | Owner raised ceiling to 10,000 AIU (D-16); §6/§7 credit rules updated to match. | this commit |