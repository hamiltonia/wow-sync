# Work packet R2-pin: re-check GPUI on a pinned Zed release

Role: implementation. Model: claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

PLAN §6.2 provisionally selects a pinned Zed release tag (AccessKit-capable)
over crates.io gpui 0.2.2. Run-001 lifecycle/memory evidence applies only to
0.2.2 ([S-a11y-design](../../run-001/evidence/S-a11y-design.md),
[P3-review](../../run-001/evidence/P3-review.md) M5). Re-establish it on the
pin before product UI depends on it. A-01 (feasibility), A-29 method (§6.6),
R-13.

## Owned writes

- `spikes/gpui-pin/` (new; own `[workspace]` and `Cargo.lock`; may copy code
  from `spikes/gpui`, `spikes/mem` and `spikes/a11y-main`).
- `.github/workflows/spike-gpui.yml` (add the pin spike to the build-only
  matrix; keep existing pins/permissions style, add a `concurrency` group).
- Evidence files.

## Work

1. Choose the newest Zed **release tag** whose gpui includes AccessKit
   (S-a11y-design named `v1.28.1` / `e8bc16a`; verify via `git ls-remote` and
   source). Pin by `rev` = the tag's commit; record tag, SHA, why.
2. One spike app combining: tray (`tray-icon`), hide/show/close→hide,
   single instance, quit; keep-alive with no visible window (check whether the
   pin still quits on last-window close; use the least-cost keep-alive);
   AccessKit labels on a heading, button, toggle, 10-item list.
3. Verify on Win x64 (release): lifecycle via external observation (as in
   S-gpui `drive.ps1`); UIA tree exposes the labelled controls (as in S-a11y);
   memory per §6.6: sum "Working Set - Private" over **all** app-owned
   processes (include any conhost/child; launch so no console is attached, e.g.
   `windows_subsystem = "windows"`), scenarios a/b/c from S-mem, 3 runs each,
   no trimming. Record GPU memory separately.
4. Local ARM64 build attempt (record result). Do not push; the orchestrator
   pushes and records the CI run.
5. Run cargo-deny inside the spike with a candidate policy: list exactly what
   a product `deny.toml` would need (licences, `allow-git` sources,
   advisories) — write the proposal in evidence; do not edit root `deny.toml`.

## Done when

Evidence gives PASS/FAIL per lifecycle item, UIA exposure, memory per scenario
vs 100 MiB, build results, and the deny proposal.
