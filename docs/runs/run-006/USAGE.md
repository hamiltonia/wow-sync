# Run 006 usage ledger

Ceiling: 10,000 AIU from run-006 start (D-28). Stop new work at ≥9,500.

Measured from the orchestrator session store. Cumulative at run-006 start:
**37,617.3 AIU**, the run-005 close figure. Post-close triage and
planning count against run-006. Run-006 usage = cumulative − 37,617.3.

| # | Packet | Role / model | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R6-gate | Implementation / claude-sonnet-5.5 | `113ec187` | 568.1 | merged; RV6 fixes, G2, H1 |
| 2 | R6-app | Implementation / claude-sonnet-5.5 | `5655a956` | 503.7 | stages A, A2; RV6 fixes, G1 |
| 3 | R6-release | Implementation / claude-sonnet-5.5 | `e075f3c8` | 68.2 | `url` fix, 0.2.0, notes |
| 4 | R6-review | Review / claude-opus-5.5 | `c843f88e` | 201.5 | 0 critical/major, 9 fixed |
| 5 | R6-check | Verification / gpt-6.1-sol | `3820f9e4` | 396.9 | G1, G2, G3 |
| 6 | R6-recheck | Verification / gpt-6.1-sol | `c99ee87f` | 497.5 | H1, H2; final passes |
| 7 | R6-settings | Implementation / claude-sonnet-5.5 | `db1bd44a` | 77.4 | H2 fix |

## Checkpoints

| Point | Cumulative | Run-006 |
|---|---|---|
| Plan approved (D-28); packets written | 37,911.9 | 294.6 |
| Run closed, `v0.2.0` published | 41,396.7 | **3,779.4** |

Worker total 2,313.3; the rest is orchestrator usage.
