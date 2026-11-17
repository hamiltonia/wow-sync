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
| 1 | R3-compat | sonnet-5.5 / claude-sonnet-5.5 | `60ed9f84` | 66.7 | Merged `944141a`; offline verify PASS |
| 2 | R3-removal-design | opus-5.5 / claude-opus-5.5 | `c77e59a2` | 212.7 | Merged; Q-40–Q-48 raised |
| 3 | R3-ui-design | opus-5.5 / claude-opus-5.5 | `887e9fa4` | 467.4 | Merged; Q-26–Q-39, gaps UG-1–UG-24 |

## Checkpoints

| When | Cumulative AIU | Run-003 AIU |
|---|---|---|
| D-20 approval | 9,513.4 | 0 |
| R3-compat merged | 10,333.7 | 820.3 |
| R3-removal-design merged | 10,471.0 | 957.6 |
| R3-ui-design merged | 10,620.4 | 1,107.0 |
