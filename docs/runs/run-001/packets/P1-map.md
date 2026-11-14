# Work packet P1-map: repository map and architecture skeleton

Run: 001, Phase 1. Role: design. Model: claude-opus-5.5 (D-04).
Documentation only. Independent of other Phase 1 packets.

## Intent

Make the repository navigable by a fresh agent (E-08, A-40) and set the
architectural boundaries that the feasibility spikes will test (SPEC §7).

## Checkout

- Worktree `D:\GitHub\wow-sync.worktrees\P1-map`, branch `run001/P1-map` from
  `main`.

## Owned writes

- `docs/ARCHITECTURE.md`, `docs/REPO-MAP.md`
- `docs/runs/run-001/evidence/P1-map.md`

## Work

1. `docs/REPO-MAP.md`: where intent, decisions, runs, packets, evidence,
   verification, code and spikes live; how a worker finds its packet and
   commands. Link, do not duplicate, authoritative docs.
2. `docs/ARCHITECTURE.md` (proposal, marked as such):
   - Platform-free domain core holding the single reconciliation/policy
     authority (R-05..R-10), testable without GUI/network/provider (A-43).
   - Adapter boundaries: filesystem, Dropbox local folder, process/game
     activity, tray/GUI, notifications, telemetry. What each may and may not do.
   - Invariants from SPEC (variant isolation, no Lua execution, confirmed vs
     published, WoW-running block, rejection persistence) and where enforced.
   - Open questions per SPEC §7 mapped to spike IDs in PLAN.md.
   - Explicitly unchosen: protocol, storage format, libraries.
3. Ground statements in SPEC/ACCEPTANCE IDs. Do not invent requirements.
4. Commit (`run001(P1-map):` prefix, Copilot co-author trailer). Do not push.

## Stop conditions

No sub-agents. No code. If a SPEC ambiguity blocks a boundary, record it as a
question in the evidence file rather than resolving it.
