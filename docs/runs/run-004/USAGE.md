# Run 004 usage

Ceiling: 20,000 AIU from run-004 start (D-23/D-24). Stop new work at
≥19,000.

Baseline: the cumulative orchestrator-session total at run-004 start is
**17,413 AIU**. Run-004 usage = cumulative − 17,413.

| # | Packet | Role / model | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R4-removal-design | design / claude-opus-5.5 | `17391135` | 346.6 | Merged; R3D-13 closure claimed (review pending); Q-54..58 |
| 2 | R4-settings-design | design / claude-opus-5.5 | `4c649513` | 397.1 | Merged; settings.md + rollback.md; Q renumbered to Q-67..84 |
| 3 | R4-ops-design | design / claude-opus-5.5 | `07b3d707` | 394.3 | Merged; ops.md; Q renumbered to Q-59..66 |
| 4 | R4-design-review (+ recheck, recheck2) | verification / gpt-6.1-sol | `07fd2769` | 511.0 | Review FAIL; recheck FAIL; recheck2 FAIL `e1fe458`; recheck3 FAIL (R4D-28) `6dfd530`; recheck4 **PASS** (minor R4D-32) `99e4877` |
| 5 | R4-design-fix (+ retry 1 fix2, retry 2 fix3) | design / claude-opus-5.5 | `961ccd8a` | 2,835.7 | Merged `c939b30`; fix2 `3751253`; fix3 `e6a6790`; bounded R4D-28 amendment fix4 `832492d` (owner "continue" at the barrier, taken as option 1) |
| 6 | R4-core-contracts | implementation / claude-sonnet-5.5 | `f645c783` | 930.5 | Merged `b8707ae` (3 passes; amendment 1; I-1 slip noted); both verify tiers PASS; T-C11 native NOT RUN (R4D-22) |
| 7 | R4-core-removal | implementation / claude-sonnet-5.5 | `e741ccb0` | — | Running (phase C step 2) |
| 8 | R4-core-settings | implementation / claude-sonnet-5.5 | `50cfd494` | — | Running (phase C step 2) |
| 9 | R4-core-ops | implementation / claude-sonnet-5.5 | `8aee2e45` | — | Running (phase C step 2) |

Follow-up R4-design-recheck (worker 4, same agent): FAIL. Blockers
R4D-1, R4D-10 and R4D-17; 8 majors.

| Checkpoint | Cumulative | Run-004 |
|---|---|---|
| Phase A merged | 18,873.7 | 1,460.7 |
| Review merged | 19,058.2 | 1,645.2 |
| Recheck merged | 20,570.7 | 3,157.7 |
| Recheck2 merged | 21,741.1 | 4,328.1 |
| Recheck3 merged (owner barrier) | 22,493.0 | 5,080.0 |

Stopped at the owner barrier: R4D-28 needs an owner-authorized bounded amendment (design retries exhausted) and the PENDING OWNER questions need answers before phase C.

| Checkpoint | Cumulative | Run-004 |
|---|---|---|
| Design PASS (recheck4 merged) | 23,002.6 | 5,589.6 |
| C1 foundation merged | 24,397.4 | 6,984.4 |

Design review passed. Phase C remains blocked only on the PENDING OWNER product questions.
