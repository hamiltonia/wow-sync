# Run 001 usage ledger

Cumulative across sessions, workers and retries (E-02). Source: Copilot CLI
local session store, `assistant_usage_events` (actual `model`, `total_nano_aiu`
per `agent_id`). Values in AIU = `total_nano_aiu / 1e9`.

Credit ceiling: **not set** (D-13). Only P0-trial is authorized until it is.

## Workers

| # | Packet | Requested model | Actual model(s) | AIU | Status |
|---|---|---|---|---|---|
| 1 | P0-trial | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `c52c05a7`, 13 calls) | 21.70 | Done. Merged to `main` in `d3b2df1` after orchestrator re-verification |

Worker count: 1 / 20. Concurrent max: 5.

Independent check (orchestrator, clean clone of `01dfbdc`, 2026-11-14): fmt,
clippy and build pass; `cargo test` failed `link.exe not found` (exit 101)
until the owner installed Build Tools 17.14.45. After install, all step-6
commands pass (0 tests run). Combined checks rerun on merged `main`: pass.

## Orchestrator

| Session | Model(s) | AIU (cumulative at time of entry) | Recorded |
|---|---|---|---|
| `fea20f53` | claude-opus-5.5 266.45, gpt-6.1-sol 8.14 (pre-switch) | 274.59 | 2026-11-14 21:07Z, before P0 |
| `fea20f53` | claude-opus-5.5 301.83, gpt-6.1-sol 8.14 | 309.97 | After P0 and its verification |
| `fea20f53` | claude-opus-5.5 393.86, gpt-6.1-sol 8.14 | 402.00 | After P0 merge |

**Run total so far: 423.70 AIU** (orchestrator 402.00 + workers 21.70).

## Ceiling proposal (D-13, awaiting owner approval)

Observation: P0 cost 21.70 AIU, but orchestrator overhead dominates because
per-call cost grows with session context. Mitigation: start a fresh
orchestrator session per phase, resuming from these repository records.

| Item | Estimate (AIU) |
|---|---|
| Already spent | 424 |
| Phase 1: 4 workers (3 implementation, 1 verifier) | 200 |
| Phase 2: 8 spikes plus verifier passes (GPUI/memory heaviest) | 700 |
| Phase 3: review | 60 |
| Retry headroom | 250 |
| Orchestrator, fresh session per phase | 800 |
| **Proposed run-001 ceiling (cumulative)** | **2,500** |

Pause and ask at 2,000 (80%); stop at 2,500. This is an estimate from one
small trial; Phase 1 actuals will be reported before Phase 2 starts.
