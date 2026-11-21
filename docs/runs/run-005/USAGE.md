# Run 005 usage ledger

Ceiling: 10,000 AIU from run-005 start (D-26). Stop new work at ≥9,500.

Measured from the orchestrator session store. Cumulative at run-005 start:
**34,088.6 AIU**. Run-005 usage = cumulative − 34,088.6.

| # | Packet | Role / model | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R5-remove | Implementation / claude-sonnet-5.5 | `e864879c` | 268.2 | merged `1c543ce` |
| 2 | R5-decide | Implementation / claude-sonnet-5.5 | `f633950f` | 328.8 | merged `1c543ce`, follow-up `7a4df23` |
| 3 | R5-app | Implementation / claude-sonnet-5.5 | `87c675a0` | 202.9 | merged `1fb0234` (A), `559cb4d` (A2) |
| 4 | R5-release | Implementation / claude-sonnet-5.5 | `fd553a79` | 86.1 | merged `4ec8104`, packaging fix `4675f08` |
| 5 | R5-check | Verification / gpt-6.1-sol | `bb375372` | | running |

## Checkpoints

| Point | Cumulative | Run-005 |
|---|---|---|
| Plan approved (D-27); packets written | 34,088.6 | 0.0 |
| Phase A merged (`4675f08`); R5-check launched | 35,436.0 | 1,347.4 |
