# Run 001 usage ledger

Cumulative across sessions, workers and retries (E-02). Source: Copilot CLI
local session store, `assistant_usage_events` (actual `model`, `total_nano_aiu`
per `agent_id`). Values in AIU = `total_nano_aiu / 1e9`.

Credit ceiling: **not set** (D-13). Only P0-trial is authorized until it is.

## Workers

| # | Packet | Requested model | Actual model(s) | AIU | Status |
|---|---|---|---|---|---|
| 1 | P0-trial | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `c52c05a7`, 13 calls) | 21.70 | Partial: blocked on MSVC install (UAC, exit 1602). Commit `01dfbdc` not merged |

Worker count: 1 / 20. Concurrent max: 5.

Independent check (orchestrator, clean clone of `01dfbdc`, 2026-11-14): fmt,
clippy and build pass; `cargo test` fails `link.exe not found` (exit 101),
matching the worker's report.

## Orchestrator

| Session | Model(s) | AIU (cumulative at time of entry) | Recorded |
|---|---|---|---|
| `fea20f53` | claude-opus-5.5 266.45, gpt-6.1-sol 8.14 (pre-switch) | 274.59 | 2026-11-14 21:07Z, before P0 |
| `fea20f53` | claude-opus-5.5 301.83, gpt-6.1-sol 8.14 | 309.97 | After P0 and its verification |
