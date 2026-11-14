# Run 001 usage ledger

Cumulative across sessions, workers and retries (E-02). Source: Copilot CLI
local session store, `assistant_usage_events` (actual `model`, `total_nano_aiu`
per `agent_id`). Values in AIU = `total_nano_aiu / 1e9`.

Credit ceiling: **not set** (D-13). Only P0-trial is authorized until it is.

## Workers

| # | Packet | Requested model | Actual model(s) | AIU | Status |
|---|---|---|---|---|---|
| | | | | | |

Worker count: 0 / 20. Concurrent max: 5.

## Orchestrator

| Session | Model(s) | AIU (cumulative at time of entry) | Recorded |
|---|---|---|---|
| `fea20f53` | claude-opus-5.5 266.45, gpt-6.1-sol 8.14 (pre-switch) | 274.59 | 2026-11-14 21:07Z, before P0 |
