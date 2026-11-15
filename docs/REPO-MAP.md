# Repository map

Where things live in `hamiltonia/wow-sync`, and how a worker finds its scope.
This map links to authoritative documents; it does not restate them. If this
map and a linked document disagree, the linked document wins; report the
discrepancy (README "Authority and approval").

Status: written in run-001 packet P1-map (E-08, A-40). Items marked
**planned** are being created in parallel by other Phase 1 packets and may not
exist yet in your checkout.

## Start here

1. [AGENTS.md](../AGENTS.md): agent entry point, roles, easy-to-miss product rules.
2. [README.md](../README.md): status and approval boundaries.
3. [.github/copilot-instructions.md](../.github/copilot-instructions.md): short
   Copilot entry that points back to AGENTS.md.

## Intent, acceptance and engineering contract

| What | Where | IDs |
|---|---|---|
| Product intent, constraints, feasibility questions | [SPEC.md](../SPEC.md) | G-, R-01..R-16, §7 |
| Observable acceptance obligations | [ACCEPTANCE.md](../ACCEPTANCE.md) | A-01..A-46 |
| Roles, budgets, evidence, repository contract | [ENGINEERING.md](../ENGINEERING.md) | E-01..E-09 |
| Owner decisions, model policy, run limits, data scope | [docs/OWNER-DECISIONS.md](OWNER-DECISIONS.md) | D-01.. |
| Architecture proposal (not yet approved) | [docs/ARCHITECTURE.md](ARCHITECTURE.md) | — |

Data-access scope (Dropbox, WoW installation) is defined only in
OWNER-DECISIONS D-08, D-09 and D-14. Read those before touching any path
outside the repository.

## Runs, packets, evidence

Run state lives under `docs/runs/<run-id>/` (D-06). For run-001:

| What | Where |
|---|---|
| Approved run plan (phases, packet table, spikes, stop conditions) | [PLAN.md](runs/run-001/PLAN.md) |
| Resume point for a fresh orchestrator | [HANDOFF.md](runs/run-001/HANDOFF.md) |
| Cumulative AI-credit and model usage ledger | [USAGE.md](runs/run-001/USAGE.md) |
| Work packets, one per worker | [packets/](runs/run-001/packets/) |
| Worker and verifier evidence, one file per packet | [evidence/](runs/run-001/evidence/) |

### How a worker finds its scope

1. Your packet is `docs/runs/<run-id>/packets/<packet-id>.md`; its ID is in your
   assignment and in the PLAN.md packet table. The packet is authoritative for
   your owned writes, checkout, dependencies, commands and stop conditions (E-05).
2. Work only in the worktree and branch named in the packet, conventionally
   `D:\GitHub\wow-sync.worktrees\<packet-id>` on `run<NNN>/<packet-id>`
   (PLAN §4).
3. Write your report to `docs/runs/<run-id>/evidence/<packet-id>.md`, using the
   status vocabulary pass/fail/blocked/skipped/not run (E-04).
4. Commit with the prefix and trailer your packet specifies. Do not push or
   merge; the orchestrator integrates after independent verification (E-05).

## Verification

| What | Where | Status |
|---|---|---|
| Verification guide (prerequisites, tiers, adding required cases) | `docs/VERIFY.md` | **Planned**, packet [P1-verify](runs/run-001/packets/P1-verify.md) |
| Single entry point `cargo xtask verify --tier offline\|native\|provider\|all` | `xtask/` | **Planned**, P1-verify (interface fixed in that packet) |
| Required-case inventory (case IDs to tests and tiers) | `tests/required-cases.toml` | **Planned**, P1-verify |
| Dependency/license policy for `cargo deny` | `deny.toml` | **Planned**, P1-verify |
| Verifier report output | `target/verify/report.json` (not committed) | **Planned**, P1-verify |
| CI workflows (offline tier on Windows/macOS, build-only target matrix) | `.github/workflows/` | **Planned**, packet [P1-ci](runs/run-001/packets/P1-ci.md) |

Until those land, the only reproduced commands are the P0 ones recorded in
[evidence/P0-trial.md](runs/run-001/evidence/P0-trial.md). Once `docs/VERIFY.md`
exists, it is the authority for commands; this table only says where to look.

## Code

| What | Where | Status |
|---|---|---|
| Pinned toolchain and targets | [rust-toolchain.toml](../rust-toolchain.toml) | Exists (P0) |
| Cargo virtual workspace, shared package metadata | [Cargo.toml](../Cargo.toml), `Cargo.lock` | Exists (P0); members change only via owning packets |
| Platform-free domain core | [crates/wow-sync-core/](../crates/wow-sync-core/) | Exists as an empty skeleton; intended role in [ARCHITECTURE.md](ARCHITECTURE.md) |
| Platform adapters, GPUI app | Not yet created | Boundaries proposed in ARCHITECTURE.md |

## Spikes

Phase 2 feasibility spikes (S-gpui, S-mem, S-a11y, S-forever, S-addon,
S-state, S-proc, S-dropbox) live in `spikes/<name>/`, outside the product
workspace, and may be deleted once findings are recorded (PLAN §3, Phase 2).
None exist yet. Findings go to the spike's evidence file under
`docs/runs/<run-id>/evidence/`. ARCHITECTURE.md maps SPEC §7 questions to these
spike IDs.
