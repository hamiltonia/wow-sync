# Run 001 handoff

Resume point for a fresh orchestrator session. Repository records are
authoritative; prior chat is not (AGENTS.md).

## State (2026-11-15, session `961f3baa`, Phase 1 close)

- Checkout: `D:\GitHub\wow-sync`, branch `main`.
- Spec approved (D-03). Run-001 plan approved (D-15). Ceiling 2,500 AIU
  cumulative (D-13). Model policy D-04. Worker limits D-05.
- Phase 0: **done**, merged `d3b2df1`.
- Phase 1: **done.** P1-verify, P1-ci, P1-map, P1-check merged and pushed.
  Fixes for P1-check findings merged at `4d1b14a`; CI on `main` green
  (offline verify Win/macOS + 4 build-only targets).
- P1-check (gpt-6.1-sol) re-verification `d397ac7` on `4d1b14a`:
  A-38 initial PASS; A-39 initial PASS (false green closed; orchestrator also
  reproduced it FAIL/exit 1); E-08 controls PASS; native/provider BLOCKED;
  A-40 FAIL only for stale `USAGE.md`, corrected in the commit that adds this
  handoff.
- Known limitation: if xtask fails to compile, an older
  `target/verify/report.json` can remain; reports carry `git_sha`/`generated_at`
  and `docs/VERIFY.md` says to treat mismatches as stale.
- Spent ≈1,371 AIU (55%); see `USAGE.md`. Workers 5 / 20.

## Phase 1 report to owner (D-13 reporting; not an approval gate)

- Workers cost 381 AIU vs 200 estimated; the verifier found a real false green
  and doc drift, each needing a fix round. Independent checking worked.
- Remaining to the 2,000 pause: ≈629 AIU. Phase 2 (8 spikes) will likely
  reach the pause before finishing; the pause rule then applies.
- P1-map recorded six design questions (`evidence/P1-map.md`), including five
  SPEC §7 items with no run-001 spike. None block Phase 2.
- Final `main` CI run `43662904223` on `0f33116`: success.

## Next actions for the orchestrator

Phase 2 is authorized by D-15 (plan) and D-13 (ceiling, pause at 2,000). No
further owner approval is needed to start. D-13 requires a **fresh
orchestrator session** for it.

1. Start a new Copilot CLI session on claude-opus-5.5 in `D:\GitHub\wow-sync`
   (`main`) and resume from this handoff. Record its start in `USAGE.md`.
2. Write Phase 2 packets (`packets/S-*.md`, E-05 fields, PLAN §3 table).
   Order by information value per AIU so the most decision-relevant spikes
   finish before the 2,000 pause: S-gpui, S-mem, S-addon, S-proc first;
   then S-state, S-forever, S-dropbox, S-a11y.
3. Launch at most 5 concurrently with D-04 models; Windows-only execution;
   macOS runtime items reported BLOCKED. S-forever output waits for owner
   review before commit (PLAN §7).
4. Recheck cumulative usage after each worker; at ≥2,000 AIU, settle in-flight
   workers, update this handoff and pause for the owner.
5. Correct in P1-ci evidence at next touch: on `macos-latest` (Apple Silicon)
   `aarch64-apple-darwin` is the native build and `x86_64-apple-darwin` the
   cross-compile; both remain build-only.

## Known issues and risks

- Copilot CLI ignores `subagents.maxConcurrency`/`maxDepth` without
  usage-based billing; the orchestrator enforces limits manually.
- Orchestrator context cost dominates usage; keep sessions short.
- Dev PC is an Insider build (26300); not official support evidence.
- S-forever output requires owner review before commit (PLAN §7).

## Open owner items

- None blocking. Optional: raise the ceiling if all 8 spikes should finish
  without a pause.
- Mac availability needed for macOS runtime spikes.
