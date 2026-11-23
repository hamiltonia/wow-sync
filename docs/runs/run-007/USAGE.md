# Run 007 usage

- **Baseline:** 41,858.9 AIU cumulative (sessions `961f3baa`, `fea20f53`),
  taken at approval on 2026-11-23.
- About 462 AIU between the run-006 close (41,396.7) and this baseline went on
  orchestrator investigation before the run was approved. That is outside the
  run-007 ceiling.
- **Ceiling:** 4,000 AIU from the baseline; stop new work at ≥3,700
  (cumulative 45,558.9).

Closed at 44,943.3 cumulative: **3,084.4 AIU used** (77% of the ceiling).
Per-agent figures come from the local usage store (`assistant_usage_events`
by `agent_id`).

| Agent | Model | AIU |
|---|---|---|
| R7-sync (`ea10655a`), phase A and fixes for RV7-01..04, 08, 10..12, 14, G1, G2 | claude-sonnet-5.5 | 817.3 |
| R7-setup (`0f80b769`), phase A, follow-up, RV7-05..07, 09, 13 | claude-sonnet-5.5 | 336.4 |
| R7-review (`5dccb8ca`), review and two rechecks | claude-opus-5.5 | 540.3 |
| R7-check (`be714bcc`), verification and two rechecks | gpt-6.1-sol | 418.5 |
| Orchestrator (planning, gates, merges, records) | claude-opus-5.5 | 971.9 |
| **Total** | | **3,084.4** |
