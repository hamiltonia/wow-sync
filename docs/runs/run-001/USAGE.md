# Run 001 usage ledger

Cumulative across sessions, workers and retries (E-02). Source: Copilot CLI
local session store, `assistant_usage_events` (actual `model`, `total_nano_aiu`
per `agent_id`). Values in AIU = `total_nano_aiu / 1e9`.

Credit ceiling: **10,000 AIU cumulative** (D-16, 2026-11-15; was 2,500 under
D-13). Stop launching at ≥9,500; in-flight work may settle up to 11,000.

## Workers

| # | Packet | Requested model | Actual model(s) | AIU | Status |
|---|---|---|---|---|---|
| 1 | P0-trial | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `c52c05a7`, 13 calls) | 21.70 | Done. Merged to `main` in `d3b2df1` after orchestrator re-verification |
| 2 | P1-verify | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `a83d3830`, 30 calls incl. fixes 1–2) | 97.00 | Done `44f170e`; fix 1 `59199af` (inline-code link false positive); fix 2 `08163c1` (P1-check false green, report freshness, link-scan scope) |
| 3 | P1-ci | claude-sonnet-5.5 | claude-sonnet-5.5 (agent `d37350b4`, 10 calls) | 23.11 | Done `34c8f50` |
| 4 | P1-map | claude-opus-5.5 | claude-opus-5.5 (agent `6e7b0d6d`, 25 calls incl. fix 1) | 165.27 | Done `4b1dd04`; fix 1 `a154850` (stale REPO-MAP) |
| 5 | P1-check | gpt-6.1-sol | gpt-6.1-sol (agent `ed40bcc0`, 62 calls incl. re-verification) | 95.30 | Done `98ea632`; re-verification `d397ac7` |

Worker count: 5 / 20. Concurrent max: 5 (3 used). Retries: P1-verify 2 / 2
(exhausted), P1-map 1 / 2.

Independent check (orchestrator, clean clone of `main` + the three branches,
2026-11-15): first run `offline/markdown-links` FAIL (false positive on inline
code in P1-map evidence); after `59199af`: offline PASS exit 0; `all` BLOCKED
exit 1 (native, provider); `all --allow-blocked` exit 0 still reporting BLOCKED.

Independent check (orchestrator, clean clone of `01dfbdc`, 2026-11-14): fmt,
clippy and build pass; `cargo test` failed `link.exe not found` (exit 101)
until the owner installed Build Tools 17.14.45. After install, all step-6
commands pass (0 tests run). Combined checks rerun on merged `main`: pass.

## Orchestrator

| Session | Model(s) | AIU (cumulative at time of entry) | Recorded |
|---|---|---|---|
| `fea20f53` | claude-opus-5.5 266.45, gpt-6.1-sol 8.14 (pre-switch) | 274.59 | 2026-11-14 21:07Z, before P0 |
| `fea20f53` | claude-opus-5.5 301.83, gpt-6.1-sol 8.14 | 309.97 | After P0 and its verification |
| `fea20f53` | claude-opus-5.5 393.86, gpt-6.1-sol 8.14 | 402.00 | After P0 merge |
| `fea20f53` | claude-opus-5.5 508.79, gpt-6.1-sol 8.14 | 516.93 | Final (session closed after handoff) |
| `961f3baa` | gpt-6.1-sol 27.14 (session started on wrong model; stopped per D-04), claude-opus-5.5 41.36 | 68.50 | 2026-11-15 00:26Z, Phase 1 start |
| `961f3baa` | gpt-6.1-sol 27.14, claude-opus-5.5 424.38 | 451.52 | 2026-11-15, Phase 1 close (excludes final ledger edits) |
| `961f3baa` | session totals incl. workers 1–5 run in it: opus 766.17, sol 122.44, sonnet 120.12 | — | 2026-11-15 ~20:00Z, Phase 2 start under D-16 (same session; D-13 fresh-session rule waived) |

**Run total at Phase 2 start: ≈1,547 AIU** (all run sessions `fea20f53` +
`961f3baa`, all models, workers included).

**Run total at Phase 1 start: 607.13 AIU** (orchestrator 585.43 + workers 21.70).
Excluded: session `13d6d36b` (`D:\GitHub\shell`, unrelated repository).

**Run total at Phase 1 close: ≈1,371 AIU** (orchestrator 516.93 + 451.52;
workers 21.70 + 380.68). 55% of ceiling; 629 AIU until the 2,000 pause.

Phase 1 actual vs estimate: workers 380.68 vs 200 (two verifier-driven fix
rounds); orchestrator ≈452 for one phase vs 800 budgeted for all phases.

Independent check (orchestrator, clean clone of `main` + P1-check, P1-verify
fix 2, P1-map fix 1, 2026-11-15): offline PASS exit 0; `all` BLOCKED exit 1;
P1-check's false-green scenario (all xtask test modules disabled, inventory
emptied) now FAIL exit 1.

## Ceiling (D-13): **approved 2,500 AIU cumulative**, 2026-11-14

Observation: P0 cost 21.70 AIU, but orchestrator overhead dominates because
per-call cost grows with session context. Mitigation: start a fresh
orchestrator session per phase, resuming from these repository records.

| Item | Estimate (AIU) |
|---|---|
| Already spent | 424 |
| Phase 1: 4 workers (3 implementation, 1 verifier) | 200 |
| Phase 2: 8 spikes plus verifier passes (GPUI/memory heaviest) | 700 |
| Phase 3: review | 60 |
| Retry headroom | 250 |
| Orchestrator, fresh session per phase | 800 |
| **Proposed run-001 ceiling (cumulative)** | **2,500** |

Pause and ask at 2,000 (80%); stop at 2,500. This is an estimate from one
small trial; Phase 1 actuals will be reported before Phase 2 starts.
