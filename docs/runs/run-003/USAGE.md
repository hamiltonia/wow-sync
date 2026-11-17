# Run 003 usage ledger

**Source:** local session store `assistant_usage_events` for sessions
`961f3baa` and `fea20f53`. AIU = `total_nano_aiu / 1e9`, grouped by
`agent_id`.

**Thresholds:**
- Baseline at D-20: 9,513.4 AIU cumulative.
- Stop launching new work at ≥19,013.
- Settle up to ≈20,513.

## Workers

| # | Packet | Model (policy / actual) | Agent | AIU (incl. retries) | Result |
|---|---|---|---|---|---|
| 1 | R3-compat | sonnet-5.5 / claude-sonnet-5.5 | `60ed9f84` | 158.7 | Merged; superseded by D-21 (R3-d21) |
| 2 | R3-removal-design | opus-5.5 / claude-opus-5.5 | `c77e59a2` | 747.1 | Merged; R3D-13 open, retries exhausted |
| 3 | R3-ui-design | opus-5.5 / claude-opus-5.5 | `887e9fa4` | 879.5 | Merged; UI PASS on recheck |
| 4 | R3-infra | sonnet-5.5 / claude-sonnet-5.5 | `94d988ca` | 82.1 | Merged; native tier PASS |
| 5 | R3-design-review (+ 2 rechecks) | sol / gpt-6.1-sol | `af43de1a` | 210.5 | FAIL, then UI PASS; removal R3D-13 open |
| 6 | R3-mem | sonnet-5.5 / claude-sonnet-5.5 | `7cbd7f5f` | 116.2 | Merged `8e82673`; A-29 PASS |
| 7 | R3-proto | sonnet-5.5 / claude-sonnet-5.5 | `ca4c82b8` | 458.8 | Merged; owner-approved (D-21) |
| 8 | R3-d21 | sonnet-5.5 / claude-sonnet-5.5 | `4f318e51` | 77.7 | Merged; gating removed, spikes deleted |
| 9 | R3-dropbox-api | opus-5.5 / claude-opus-5.5 | `403a40c2` | 193.6 | Merged; keep desktop folder for v1 |
| 10 | R3-check | sol / gpt-6.1-sol | `12077eaa` | 164.2 | Merged; RC-1 to RC-3 |
| 11 | R3-ui-build (+ retry 1) | sonnet-5.5 / claude-sonnet-5.5 | `ec402443` | 863.8 | Merged `17a360a`; A-29 PASS (37.7 MiB) |
| 12 | R3-review | opus-5.5 / claude-opus-5.5 | `92b95e84` | 273.0 | Merged; FAIL with 7 major and 5 minor findings, no blockers |
| 13 | R3-check2 | sol / gpt-6.1-sol | `0007206a` | 164.0 | Merged `3b27ec7`; FAIL on RC2-1 (invented next-check time); RC2-1 to RC2-4 go to R3-fix |
| 14 | R3-fix (+ retry 1) | sonnet-5.5 / claude-sonnet-5.5 | `8544314f` | 319.0 | Merged `042b44c`. RV-1 to RV-9, RV-12, RC2-1 and RC2-4 done. RC2-2 and RC2-3 in code only. No real-app screenshots. Icon goal not done |
| 15 | R3-check3 | sol / gpt-6.1-sol | `c9623d49` | 207.1 | Merged `d59fafc`. Toggle, second-instance show, pipe capture, tiers and memory (37.34 MiB) PASS. Tray click BLOCKED. FAIL on RC3-1 (medium) and RC3-2 and RC3-3 (low) |
| 16 | R3-macfix | sonnet-5.5 / claude-sonnet-5.5 | `b2ece888` | 14.5 | Merged `0e86e74`; macOS CI green (PR #1) |
| 17 | R3-fix2 (+ retry 1 as R3-fix2b) | sonnet-5.5 / claude-sonnet-5.5 | `d65af3cf` | 148.2 | Merged `8afde5d` and `a513fc8`. RC3-1 to RC3-3 and RC4-1 done, with tests and negative controls. Disclosed edits outside owned paths (core exec, records) |
| 18 | R3-check4 (+ follow-up R3-check4b) | sol / gpt-6.1-sol | `01169862` | 213.8 | Merged `025a1cd` (FAIL on RC4-1) and `00a9837` (RC4-1 closed in the real app, RC3-1 regression PASS) |

The orchestrator's row (agent NULL) covers all sessions since run-001: 5,313.3
at the R3-review merge.

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
| R3-review merged | 15,442.0 | 5,928.6 |
| R3-check2 merged; R3-fix launched | 15,669.3 | 6,155.9 |
| R3-fix merged; R3-check3 launched | 16,098.4 | 6,585.0 |
| R3-check3 and R3-macfix merged; R3-fix2 launched | 16,483.8 | 6,970.4 |
| R3-fix2 merged; R3-check4 launched | 16,690.8 | 7,177.4 |
| Run closed (R3-check4b merged) | 17,029.2 | 7,515.8 |

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
