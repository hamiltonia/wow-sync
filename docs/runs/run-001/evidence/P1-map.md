# P1-map evidence

Run 001, packet [P1-map](../packets/P1-map.md). Role: design. Requested model:
claude-opus-5.5 (D-04); actual runtime model is recorded by the orchestrator
in [USAGE.md](../USAGE.md) from session records, not by this worker.

Checkout: `D:\GitHub\wow-sync.worktrees\P1-map`, branch `run001/P1-map`,
based on `e59e82e`. Documentation only; no code, no sub-agents, no WoW or
Dropbox data accessed.

## Delivered

| File | Purpose | Grounding |
|---|---|---|
| [docs/REPO-MAP.md](../../../REPO-MAP.md) | Where intent, decisions, runs, packets, evidence, verification, code and spikes live; how a worker finds its packet | E-05, E-08, A-40; D-06; PLAN §3–§4; P1-verify and P1-ci packets for planned items |
| [docs/ARCHITECTURE.md](../../../ARCHITECTURE.md) | PROPOSAL: pure domain core as single policy authority, shell, adapter may/may-not table, invariant enforcement table, SPEC §7 to spike map, unchosen list | E-08, A-43, SPEC §2/§5/§7/§8, R-02..R-15, PLAN §3 |
| this file | Report | — |

## How statements are grounded

- Every adapter "may not" and every invariant row cites an R-, A-, E- or D- ID
  that states the rule; none adds a requirement.
- Planned items (`xtask/`, `docs/VERIFY.md`, `tests/required-cases.toml`,
  `deny.toml`, `.github/workflows/`) are labeled **Planned** and are not linked,
  because they do not exist in this branch. Their interface is quoted only from
  the P1-verify/P1-ci packets.
- The core/shell/adapter split is a proposal derived from E-08 ("single
  authority", "testable domain policy") and A-43; the shell's existence, crate
  layout and process model are explicitly left unchosen.
- SPEC §7 items without a run-001 spike are shown as gaps, not filled.

## Link check

Ad hoc Python check (not committed): every `](target)` outside code fences in
the three files, excluding URL schemes, resolved with `os.path.exists` relative
to the containing file. Result: **PASS**, 28 relative links, 0 broken
(Windows dev PC, 2026-11-14). Directory links (`packets/`, `evidence/`,
`crates/wow-sync-core/`) resolve to existing directories. Anchors were not
checked; no in-page anchors are used. This is not the planned P1-verify link
check, which has not run on these files.

## Questions (recorded, not resolved)

1. **Reading/capture while WoW runs.** R-11 blocks *game-file changes* while
   WoW runs; R-07 captures settings at completed session boundaries. Is reading
   AddOns/WTF to detect or publish changes permitted while WoW is running, or
   only after a completed session? Affects the process adapter and shell.
2. **Location of recovery history.** R-10 requires current plus two previous
   per addon for code and settings and group-wide rejection. SPEC does not say
   whether recovery points are shared through Dropbox, kept per computer, or
   both. Affects the Dropbox adapter and storage design; left unchosen.
3. **Account/character identity matching.** A-03 covers case/Unicode
   differences, but SPEC does not define when two identities "match" across
   computers (R-02). Affects core scoping keys.
4. **SPEC §7 items with no run-001 spike:** recovery-point associations,
   rejection persistence and retention; master reassignment/retirement,
   concurrent policy decisions and observed vs intentional pushes; telemetry
   (deferred, D-11); reproducible packages and unsigned install paths; non-memory
   performance budgets. Should these be covered by a later run, or by design
   documents in this run? The E-09 feasibility gate lists
   "identity/version/state/provider investigation" but not all of these.
5. **Session boundary definition.** R-07 leaves "safe initialization and
   missed-session handling" to technical design; it is unclear whether S-proc
   is expected to propose it or only report observable signals.
6. **CLI scope.** R-13 allows an optional advanced CLI that must not duplicate
   the engine. Is a CLI in first-release scope at all, or only permitted?
   ARCHITECTURE.md treats it as an optional presentation adapter.

## Not done

- No verification tier run: P1-verify's entry point does not exist in this
  branch. Docs-only change; only the ad hoc link check above was run.
