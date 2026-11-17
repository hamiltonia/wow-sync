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
| 4 | R3-infra | sonnet-5.5 / claude-sonnet-5.5 | `94d988ca` | 82.1 | Merged; native tier PASS; 6 alerts need owner decision |
| 5 | R3-design-review (+ recheck) | sol / gpt-6.1-sol | `af43de1a` | 172.7 | Review FAIL; recheck: UI PASS, Q-21 owner-pending, removal FAIL |
| 6 | R3-mem | sonnet-5.5 / claude-sonnet-5.5 | `7cbd7f5f` | — | Running |
| 7 | R3-proto | sonnet-5.5 / claude-sonnet-5.5 | `ca4c82b8` | — | Running |

Retries reuse the original agent; their usage is included in that row
(compat 158.7, ui 879.5, removal 563.0 at this point).

## Checkpoints

| When | Cumulative AIU | Run-003 AIU |
|---|---|---|
| D-20 approval | 9,513.4 | 0 |
| R3-compat merged | 10,333.7 | 820.3 |
| R3-removal-design merged | 10,471.0 | 957.6 |
| R3-ui-design merged | 10,620.4 | 1,107.0 |
| R3-infra merged | 10,720.8 | 1,207.4 |
| Recheck merged; proto launched | 12,169.5 | 2,656.1 |
| Proto merged; phase D barrier | 13,048.6 | 3,535.2 |

Worker totals at the phase D barrier, in AIU, including retries:

| Worker | AIU |
|---|---|
| ui | 879.5 |
| removal | 747.1 |
| proto | 458.8 |
| review | 210.5 |
| compat | 158.7 |
| mem | 116.2 |
| infra | 82.1 |
