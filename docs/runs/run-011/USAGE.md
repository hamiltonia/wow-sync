# Run 011 usage

No AIU ceiling (D-33). Figures are cumulative AIU across the orchestrator
sessions, from the local session store.

| Point | Cumulative AIU | Run-011 so far |
|---|---|---|
| Baseline (plan written, before workers) | 54,385.1 | 0 |
| At close (v0.4.0 tagged) | 67,035.2 | 12,650.1 |

Per-agent figures were not recorded separately. The total covers four
workers (R11-runtime, R11-app, R11-review, R11-check), six independent
check rounds and the orchestrator.
