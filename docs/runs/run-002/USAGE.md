# Run 002 usage ledger

Source: local session store `assistant_usage_events` (sessions `961f3baa`,
`fea20f53`), AIU = `total_nano_aiu / 1e9`, grouped by `agent_id`.
Cumulative D-16 budget is shared with run-001 (close ≈3,140 AIU, see
[../run-001/USAGE.md](../run-001/USAGE.md)). Stop launching at ≥9,500.

## Workers

| # | Packet | Model (policy / actual) | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R2-pin | sonnet-5.5 / sonnet-5.5 | `5e25641f` | 199.4 | Done `dd77530`; merged |
| 2 | R2-design (+fix, fix2 retries) | opus-5.5 / opus-5.5 | `95df4a3b` | 1,119.5 | Merged `d35706e`, `b80c685`, `f114912` |
| 3 | R2-design-review (+recheck, recheck2) | gpt-6.1-sol / gpt-6.1-sol | `175188c4` | running | Not ready ×2; recheck2 running |

## Checkpoints

| When | Cumulative AIU | Orchestrator |
|---|---|---|
| Run-002 start | ≈3,370 | — |
| After design retry 2 / R2-pin merge | ≈5,400 (est.) | 2,149 |
