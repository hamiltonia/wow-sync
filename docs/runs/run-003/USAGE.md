# Run 003 usage ledger

**Source:** local session store `assistant_usage_events` for sessions
`961f3baa` and `fea20f53`. AIU = `total_nano_aiu / 1e9`, grouped by
`agent_id`.

**Thresholds:**
- Baseline at D-20: 9,513.4 AIU cumulative.
- Stop launching new work at ≥19,013.
- Settle up to ≈20,513.

## Workers

| # | Packet | Model (policy / actual) | Agent | AIU | Result |
|---|---|---|---|---|---|

## Checkpoints

| When | Cumulative AIU | Run-003 AIU |
|---|---|---|
| D-20 approval | 9,513.4 | 0 |
