# Owner decision register

Authoritative record of owner decisions required by ENGINEERING.md E-02 and
SPEC.md §8. A decision is binding only when recorded here with a date and
revision. Unanswered or deferred entries are not consent. Changes to a recorded
decision require a new dated entry; do not edit history.

Recorded by the Opus 5.5 orchestrator from owner (@hamiltonia) answers given
in a Copilot CLI session on 2026-11-14, against spec revision `7a2ccdf`.

## Decided

### D-01: Repository and product name

- `hamiltonia/wow-sync` is the permanent project repository; no separate
  repository will be created.
- Product name: **WoW Sync**.

### D-02: Distribution channel

- First public release: GitHub Releases.
- winget and Homebrew may be added later.
- Publishing still requires separate explicit authorization (E-09).

### D-03: Specification approval

- SPEC.md, ACCEPTANCE.md and ENGINEERING.md are approved as-is at `7a2ccdf`.
- This satisfies the E-09 "owner alignment" gate's spec half. The run-plan half
  is D-05/D-13.

### D-04: Worker role/model policy (version 1)

| Role | Model |
|---|---|
| Orchestrator (coordinates only, E-01) | claude-opus-5.5 |
| Implementation | claude-sonnet-5.5 |
| Independent verification | gpt-6.1-sol |
| Design/usability | claude-opus-5.5 |
| Integration/maintainability review | claude-opus-5.5 |
| Security/privacy review | claude-opus-5.5 |

- No alternates are approved. If a required model is unavailable, stop and ask.
- Actual runtime model per worker must be recorded, not inferred from a label.

### D-05: First run limits (bootstrap and feasibility)

- Scope: repository bootstrap plus SPEC §7 feasibility spikes. No product features.
- Maximum 5 concurrent workers, 20 total, no nested workers.
- Maximum 2 retries per task before escalating to the owner.
- No elapsed-time limit; AI credits are the only budget.
- AI-credit ceiling: **not yet set** (see D-13).

### D-07: Test machines

| Target | Machine | Status |
|---|---|---|
| Windows 11 x64 | Primary dev PC: Windows 11 Pro build 26300 (appears to be Insider), i7-13700HX, 64 GB RAM, NVMe SSDs, NVIDIA RTX 2000 Ada + Intel UHD | Available for development |
| macOS Apple Silicon | MacBook Pro M3, 48 GB RAM | Available later |
| Windows 11 ARM64 | None | Build in CI; untested until hardware exists |
| macOS Intel | None | Build in CI; untested until hardware exists |

- Targets are retained. Untested targets cannot be claimed as supported (A-35).
- The dev PC's Insider build is for development. Official support evidence comes
  from CI runners or stable-release machines.
- Workers may install rustup, the Rust toolchain and, if missing, MSVC build
  tools on the dev PC. Rust was not installed as of 2026-11-14.

### D-08: Dropbox access

- Use the existing Dropbox desktop client on the dev PC (`D:\Dropbox`).
  No Dropbox API credentials are needed or authorized.
- Agent scope is **only** `D:\Dropbox\Gaming Sync\WoW Sync Test` (create it if
  absent). Only synthetic data may be placed there.
- Every other entry in `D:\Dropbox\Gaming Sync` is out of scope: do not read,
  write, move or delete it. This includes `WoW` and `Sync`, which hold the
  owner's current legacy sync tool data (A-36, R-16), and other games' data.
- Multi-computer real-provider tests (A-34) wait for the Mac to join.

### D-09: WoW installation access

- Installation root: `D:\Games\World of Warcraft`. Only `_forever_` is installed
  on the dev PC; no `_retail_` or `_classic_` client is present there.
- Superseded in part by D-14 (2026-11-14): this installation is dev/test data.
- Original rule (still applies to any non-dev machine): read installation
  metadata only; never write; do not read `WTF`, account/character names or
  addon contents without a further decision.

### D-06: Run-state location (orchestrator default within approved scope)

- Run plans, work packets, handoffs and cumulative usage: `docs/runs/<run-id>/`.
- Evidence retention policy will be proposed during bootstrap.

## Open

### D-13: AI-credit ceiling for the first run

- Owner chose: run one small bootstrap trial task, measure its credit cost, and
  then have the orchestrator propose a ceiling for approval.
- Only that single trial task is authorized before the ceiling is approved.
- Reporting: usage per worker and cumulative; pause and ask at 80%; stop at 100%.

### D-10: Representative addon dataset

- Resolved by D-14: the dev PC's `_forever_` addons (102 folders, ~3.3 GB as of
  2026-11-14) are the representative dataset for this phase. Testing with the
  owner's real gaming data comes later and needs a new decision.

### D-14: Dev PC WoW data is test data (2026-11-14)

- Owner: the dev PC is not a primary gaming system; its addons are there for
  agents to work with, and `WTF` is in a basic state ready for testing.
- Workers may read and modify `D:\Games\World of Warcraft\_forever_\Interface\AddOns`
  and `...\_forever_\WTF` for development and tests.
- Still required: never execute addon Lua, saved-variable files or legacy
  scripts (AGENTS.md); do not run, kill or inject into WoW or launchers;
  snapshot before destructive tests so the dev install can be restored.
- Do not commit addon contents or saved settings to the repository. Third-party
  addon code is not ours to redistribute (E-06). Committed fixtures must be
  synthetic or minimal derived metadata (names, versions, interface numbers).
- This does not extend to the owner's other machines or real gaming data.

### D-15: Run-001 plan approved (2026-11-14)

- `docs/runs/run-001/PLAN.md` approved as committed in `baa81e3`.
- Phase 0 may start; Phase 1 still waits for the D-13 credit ceiling.
- S-forever and S-addon may use the dev PC data under D-14.

## Deferred

- D-11: Telemetry provider and schema (R-15). Propose after feasibility.
- D-12: Screen-reader limitation policy (R-13, E-07). Decide after the
  accessibility spike reports.

## Decision history

| Date | IDs | Summary | Revision |
|---|---|---|---|
| 2026-11-14 | D-01–D-09 | Initial owner answers recorded; D-10, D-13 open | `7a2ccdf` |
| 2026-11-14 | D-10, D-14, D-15 | Dev PC WoW data is test data; run-001 plan approved | `baa81e3` |
