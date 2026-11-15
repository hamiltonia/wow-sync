# Run 001 usage ledger

Cumulative across sessions, workers and retries (E-02). Source: Copilot CLI
local session store, `assistant_usage_events` (actual `model`, `total_nano_aiu`
per `agent_id`). Values in AIU = `total_nano_aiu / 1e9`.

Credit ceiling: **2,500 AIU cumulative** (D-13, approved 2026-11-14).

## Workers

| # | Packet | Requested model | Actual model(s) | AIU | Status |
|---|---|---|---|---|---|
| 1 | P0-trial | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `c52c05a7`, 13 calls) | 21.70 | Done. Merged to `main` in `d3b2df1` after orchestrator re-verification |
| 2 | P1-verify | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `a83d3830`, 18 calls incl. fix 1) | 53.35 | Done `44f170e` + fix `59199af` (inline-code link false positive found at integration) |
| 3 | P1-ci | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `d37350b4`, 10 calls) | 23.11 | Done `34c8f50` |
| 4 | P1-map | claude-opus-5.5 | claude-opus-5.5 (agent `6e7b0d6d`, 14 calls) | 88.55 | Done `4b1dd04` |

Worker count: 4 / 20. Concurrent max: 5 (3 used). Retries: P1-verify 1 / 2.

Independent check (orchestrator, clean clone of `main` + the three branches,
2026-11-15): first run `offline/markdown-links` FAIL (false positive on inline
code in P1-map evidence); after `59199af`: offline PASS exit 0; `all` BLOCKED
exit 1 (native, provider); `all --allow-blocked` exit 0 still reporting BLOCKED.

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
| `fea20f53` | claude-opus-5.5 508.79, gpt-6.1-sol 8.14 | 516.93 | Final (session closed after handoff) |
| `961f3baa` | gpt-6.1-sol 27.14 (session started on wrong model; stopped per D-04), claude-opus-5.5 41.36 | 68.50 | 2026-11-15 00:26Z, Phase 1 start |

**Run total at Phase 1 start: 607.13 AIU** (orchestrator 585.43 + workers 21.70).
Excluded: session `13d6d36b` (`D:\GitHub\shell`, unrelated repository).

**Run total after P1 merges: 840.70 AIU** (orchestrator 516.93 + 137.05;
workers 21.70 + 165.02). 34% of ceiling.

## Ceiling (D-13): **approved 2,500 AIU cumulative**, 2026-11-14

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
