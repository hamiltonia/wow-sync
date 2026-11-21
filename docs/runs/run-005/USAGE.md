# Run 005 usage ledger

Ceiling: 10,000 AIU from run-005 start (D-26). Stop new work at ≥9,500.

Measured from the orchestrator session store. Cumulative at run-005 start:
**34,088.6 AIU**. Run-005 usage = cumulative − 34,088.6.

| # | Packet | Role / model | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R5-remove | Implementation / claude-sonnet-5.5 | `e864879c` | 633.8 | merged `1c543ce`; F1 fix `bd39010`; §7.4 restore `9d3a144`; restore progress `debc58f` |
| 2 | R5-decide | Implementation / claude-sonnet-5.5 | `f633950f` | 328.8 | merged `1c543ce`, follow-up `7a4df23` |
| 3 | R5-app | Implementation / claude-sonnet-5.5 | `87c675a0` | 332.4 | merged `1fb0234` (A), `559cb4d` (A2), `4da4477` and `f478fca` (restore GUI) |
| 4 | R5-release | Implementation / claude-sonnet-5.5 | `fd553a79` | 119.1 | merged `4ec8104`, packaging fix `4675f08`, notes F2 `103a02f` |
| 5 | R5-check | Verification / gpt-6.1-sol | `bb375372` | 313.8 | evidence `861f2ff`; recheck running |
| 6 | R5-review | Integration / claude-opus-5.5 | `9ce74507` | | running from `f478fca` |

## Checkpoints

| Point | Cumulative | Run-005 |
|---|---|---|
| Plan approved (D-27); packets written | 34,088.6 | 0.0 |
| Phase A merged (`4675f08`); R5-check launched | 35,436.0 | 1,347.4 |
| Fix round merged (`f478fca`); recheck and R5-review launched | 36,668.3 | 2,579.7 |
