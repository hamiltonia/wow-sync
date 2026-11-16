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
| 10 | R2-fix3 | sonnet-5.5 / sonnet-5.5 | `164309ef` | 618.3 | Follow-ups #6, #9; core 171, runtime 25; merged `7da6d0f` |
| 11 | R2-recheck3 (R2-check agent reused) | gpt-6.1-sol / gpt-6.1-sol | `1ca392cb` | in row 7 | FAIL R3-M1; merged `b73b60e` |
| 12 | R2-fix4 (R2-fix3 agent reused) | sonnet-5.5 / sonnet-5.5 | `164309ef` | in row 10 | R3-M1 fixed; core 172, runtime 26; merged `63e0f54` |
| 13 | R2-recheck4 (R2-check agent reused) | gpt-6.1-sol / gpt-6.1-sol | `1ca392cb` | in row 7 | PASS: R3-M1 resolved, mutation caught |
| 14 | R2-fix5 (R2-fix3 agent reused) | sonnet-5.5 / sonnet-5.5 | `164309ef` | in row 10 | macOS-portable R3-M1 test; CI green `41e14ca` |

## Checkpoints

| When | Cumulative AIU | Orchestrator (session `961f3baa`) |
|---|---|---|
| Run-002 start | ≈3,370 | 1,371.5 |
| R2-slice launch | ≈5,984 | 1,794.0 |
| R2-fix3 launch (after `569a8b6`) | 7,933.7 | 3,035.8 (all sessions, NULL agent) |
| R2-recheck3 launch (after `7da6d0f`) | 8,613.7 | — |
| R2-recheck4 launch (after `63e0f54`), last launch | 8,887.0 | — |
| **Run-002 close** (after recheck4 merge) | **≈8,940** (+ closing commit) | 3,237.1 (NULL agent, both sessions) |
| **Final** (after R2-fix5 CI repair, CI green `41e14ca`) | **9,198.5** | — |

Run-002 total ≈ 9,200 − 3,370 ≈ 5,830 AIU. About 300 AIU remained below
the 9,500 stop line; it was left unused because it is too small for another
fix-and-check cycle.

Per-agent at recheck4 close: R2-check/recheck×3 `1ca392cb` 357.3; R2-fix3/fix4
`164309ef` 727.5 (R2-fix5 added afterwards; included in the final total).
