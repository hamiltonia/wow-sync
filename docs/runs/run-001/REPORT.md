# Run 001 report: bootstrap and feasibility

Prepared 2026-11-15 by the Opus 5.5 orchestrator (session `961f3baa`) as
packet P3-report (PLAN §3). Authority: [PLAN.md](PLAN.md),
[../../OWNER-DECISIONS.md](../../OWNER-DECISIONS.md) (D-13, D-15, D-16).
Ledger: [USAGE.md](USAGE.md). Resume point: [HANDOFF.md](HANDOFF.md).

This report summarizes evidence; the linked evidence files are authoritative.
"Feasible" below means "no blocker found within the spike's scope", never a
product acceptance pass.

## 1. Outcome

- Phases 0–2 and Phase 3 review complete. All spikes merged to `main` except
  the S-forever report, which is **uncommitted awaiting owner review**
  (PLAN §7) in `D:\GitHub\wow-sync.worktrees\S-forever`.
- Every Phase 2 spike was independently checked by gpt-6.1-sol
  ([S-check](evidence/S-check.md)) and the merged state reviewed by opus-5.5
  ([P3-review](evidence/P3-review.md)). No high-severity integration issue;
  no product code affected.
- Workers: 17 of 20. Concurrency max 5. No nesting. Cumulative ≈3,140 AIU of
  10,000 (D-16) at close; see [USAGE.md](USAGE.md).

## 2. Acceptance targets of this run

| ID | Result | Evidence |
|---|---|---|
| A-37 (actual Opus orchestration, model matrix, cumulative budget) | PASS for run-001: actual model per worker recorded from usage records, all 17 matched D-04; a session that started on the wrong model stopped before launching workers; cumulative budget kept across sessions | [USAGE.md](USAGE.md) |
| A-38 initial (verifier shows broken implementation fails) | PASS: seeded negative controls fail the gate | [P1-check](evidence/P1-check.md) |
| A-39 initial (clean-checkout verification; missing tests fail) | PASS after fix 2 (zero-test/empty-inventory false green closed); native/provider tiers report BLOCKED, not pass | [P1-check](evidence/P1-check.md) |
| A-40 (fresh-worker onboarding, clean handoff) | PASS after doc fixes; last failure (stale USAGE) corrected, not re-verified by a verifier | [P1-check](evidence/P1-check.md) |
| A-43 initial (reproducible fmt/lint/deny/docs checks) | PASS for checks via `cargo xtask verify --tier offline` and CI; domain-change part NOT RUN (no domain code yet) | [VERIFY](../../VERIFY.md) |
| A-44 (pause on missing prerequisite) | PASS as observed: run paused for the wrong orchestrator model and for the missing `workflow` token scope, with precise requests | [USAGE.md](USAGE.md), [HANDOFF.md](HANDOFF.md) |

## 3. Feasibility answers (SPEC §7)

Labels: Windows x64 dev PC (Insider 26300) unless stated. macOS runtime is
BLOCKED everywhere (no Mac yet).

| Question | Answer | Independent check | Evidence |
|---|---|---|---|
| GPUI window + tray lifecycle | Feasible on Win x64 with `gpui` 0.2.2 + `tray-icon` 0.26.1: show, hide-to-tray, reopen, close→hide, single instance, quit PASS (tray clicks synthetic; real shell clicks NOT RUN, owner check list provided). Intel iGPU PASS; WARP NOT RUN | PASS for consistency (S-check) | [S-gpui](evidence/S-gpui.md) |
| Four architectures | Build-only PASS for all 4 targets in CI run `43756896472`; local ARM64 cross-build BLOCKED (clang/ARM64 MSVC missing). No runtime evidence for ARM64/macOS | Orchestrator checked CI run | [S-gpui](evidence/S-gpui.md) |
| Idle memory ≤ 100 MiB (R-14) | Main-process private working set 13–27 MiB idle incl. after 20 cycles (no trim). GPUI 0.2.2 quits on last-window close on Windows; keep-alive needs an anchor window (+~4.5 MiB) or a different pin | CONFIRMED main-PID numbers; "no child processes" REFUTED (console host); aggregate PARTIAL | [S-mem](evidence/S-mem.md), [S-check](evidence/S-check.md) |
| Accessibility | crates.io gpui 0.2.2 exposes **nothing** to UIA. Zed `main` with AccessKit exposes labelled controls (16 nodes; Invoke/Toggle work; SetValue no-op). Narrator speech NOT RUN | PARTIAL (consistency) | [S-a11y](evidence/S-a11y.md), [S-a11y-design](evidence/S-a11y-design.md) |
| Variant identity, "Forever" | Distinct product code `wowf` (not Classic Era), consistent across `.build.info`, `.product.db`, `.flavor.info` | CONFIRMED | S-forever report (uncommitted, owner review) |
| Addon identity / version ordering | 95/102 folders have a recognizable version scheme; only 29/102 carry external ids; folder name is the practical key; same-scheme ordering only, never timestamps. No real release pairs → real ordering correctness untested | PARTIAL (grouping overstated) | [S-addon](evidence/S-addon.md) |
| Addon-owned saved state (R-07) | Feasible with a non-executing top-level-assignment scanner + `.toc` declarations; dev PC 47 owned files, 0 unknown/shared/orphan; synthetic perturbations behave as designed | PARTIAL (conditional on client conventions; union-of-TOCs may over-accept) | [S-state](evidence/S-state.md) |
| Session/updater stability, launch races (R-11) | Path-based process detection and Restart Manager work; stability needs quiescence + exclusive probe; launch-during-write race reducible (86→1 in 20 trials) but **not closable**; probes can disturb writers | PARTIAL: cannot be read as satisfying strict R-11; design blocker | [S-proc](evidence/S-proc.md) |
| Dropbox cloud-file states | Folder is a cfapi sync root; placeholder conversion in ~8 s; temp-then-rename safe locally; "fully uploaded" not locally observable; hydration and conflicted copies BLOCKED (need manual online-only toggle / second client) | PARTIAL (some causation overclaims) | [S-dropbox](evidence/S-dropbox.md) |
| Recovery associations, settings-master policy, performance budgets, telemetry, packaging | No run-001 spike (PLAN scope); open design items | — | [P1-map](evidence/P1-map.md) |

## 4. Decisions needed from the owner

1. **S-forever report**: review the uncommitted report (contains public build/
   CDN keys and region tags, no personal data per S-check) and approve
   committing it, with or without redaction.
2. **GPUI source** (feeds D-12): crates.io 0.2.2 (no screen-reader exposure)
   vs a pinned Zed release tag with AccessKit (S-a11y-design option B,
   recommended by the design worker). Changing the pin invalidates S-gpui/S-mem
   evidence and requires a re-run.
3. **D-12 screen-reader policy**: after the 10-minute Narrator check in
   [S-a11y](evidence/S-a11y.md).
4. **Dependency policy**: allow the extra licence families GPUI needs
   (BSD-2-Clause, Zlib, MPL-2.0, CC0-1.0, NCSA), an unmaintained-advisory
   policy, and `allow-git` if pinned ([P3-review](evidence/P3-review.md) M3).
5. **Dependabot alerts** (4, spike lockfiles only): dismiss with reason, or
   delete spikes after the GPUI decision.
6. **Memory gate method**: confirm "Working Set - Private" summed over all
   app-owned processes (incl. console host) is the counter, and that forced
   working-set trimming cannot count as a pass.
7. **R-11 residual race**: accept a design that minimizes but cannot eliminate
   the launch-during-apply window (staging + atomic rename + re-check +
   recovery), or require further evidence with a real client.
8. **Owner-run manual checks** (agents cannot do these): real tray clicks,
   Narrator speech, real WoW client launch/logout timing (S-proc, S-state),
   Dropbox online-only toggle and a second-client conflict.
9. **Optional housekeeping**: `git gc --prune=now` to drop ~647 MiB of
   unreferenced build objects from an amended pre-merge commit (no history
   rewrite).

## 5. Next-phase proposal (not approved; needs owner decision)

Run-002 "Design and first vertical slice", only after decisions 2, 4 and 6:

| Item | Estimate (AIU) |
|---|---|
| Re-run GPUI lifecycle + memory + 4-target build on the chosen pin | 250 |
| Design: protocol/snapshot format, recovery associations, settings-master policy, `.toc` selection rule (opus) | 600 |
| First vertical slice (Windows, synthetic data): enroll, identify variant, inventory addons, publish/apply to test Dropbox folder | 1,500 |
| Independent verification passes (sol) | 500 |
| Orchestrator | 1,200 |
| Retry headroom | 450 |
| **Proposed ceiling** | **4,500** |

Observed in run-001: orchestrator cost ≈ half of all usage; verifier passes
found real defects each phase (false green, doc drift, overclaims) and are
worth their cost.
