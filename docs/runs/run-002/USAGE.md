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
| 3 | R2-design-review (+recheck, recheck2) | gpt-6.1-sol / gpt-6.1-sol | `175188c4` | 160.4 | Not ready ×2, then ready with fixes; merged |
| 4 | R2-core | sonnet-5.5 / sonnet-5.5 | `deace31c` | 519.0 | 152 tests; merged `85fdc73` |
| 5 | R2-adapters | sonnet-5.5 / sonnet-5.5 | `650a13c2` | 208.5 | 45 tests (+3 ignored provider); merged `ddf8c4d` |
| 6 | R2-slice (+macOS clippy fix) | sonnet-5.5 / sonnet-5.5 | `5285fbcf` | 407.1 | 16 e2e; G1–G9 author PASS; merged `ccd3c65`, `ea9692d` |
| 7 | R2-check (+recheck) | gpt-6.1-sol / gpt-6.1-sol | `1ca392cb` | 280.4 | FAIL M1–M5; recheck PASS WITH FINDINGS; merged `5f20fb4` |
| 8 | R2-fix (+fix2 retry) | sonnet-5.5 / sonnet-5.5 | `1a8cf6c8` | 274.4 | M1–M5, MJ-1, m4, mi-3, A-38 controls; merged `de6ffc9`, `569a8b6` |
| 9 | R2-review | opus-5.5 / opus-5.5 | `60c26adc` | 246.3 | Accept with follow-ups; merged `7b39d6d` |
| 10 | R2-fix3 | sonnet-5.5 / pending | `164309ef` | pending | Follow-ups #6, #9 |

## Checkpoints

| When | Cumulative AIU | Orchestrator (session `961f3baa`) |
|---|---|---|
| Run-002 start | ≈3,370 | 1,371.5 |
| R2-slice launch | ≈5,984 | 1,794.0 |
| R2-fix3 launch (after `569a8b6`) | 7,933.7 | 3,035.8 (all sessions, NULL agent) |
