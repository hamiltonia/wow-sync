# Work packet P3-review: integration review

Role: integration/maintainability review. Model: claude-opus-5.5 (D-04).
Run 001, Phase 3. Common rules: [S-common.md](S-common.md) where applicable.

## Intent

Review the merged `main` against E-06 (security, data, dependencies) and E-08
(maintainability) before the run report. Not a rewrite.

## Checkout

- Worktree `D:\GitHub\wow-sync.worktrees\P3-review`, branch `run001/P3-review`.
- Writes: only `docs/runs/run-001/evidence/P3-review.md`.

## Work

1. Repository structure: product workspace vs `spikes/` isolation (each spike
   its own `[workspace]`, not built by root verify/CI except
   `spike-gpui.yml`); `.gitignore` coverage of spike `target/`; committed size.
2. Dependencies: GitHub reported 2 moderate Dependabot alerts on `main` after
   spike lockfiles merged (`gh api repos/hamiltonia/wow-sync/dependabot/alerts`
   if permitted; otherwise `gh` UI listing or `cargo audit`-equivalent via
   cargo-deny run inside each spike, read-only). Identify the crates, whether
   they affect product code (they should not), and recommend: accept for
   spikes, exclude spikes from Dependabot, or delete spike code after
   findings. Check whether Dependabot opened PRs; do not merge/close them.
3. Workflows: `.github/workflows/*.yml` permissions, pinned SHAs, no secrets;
   whether `spike-gpui.yml` should remain.
4. Data hygiene across `docs/runs/run-001/evidence/` and `spikes/`: no addon
   contents beyond derived metadata, no WTF names, no user paths/SIDs.
5. Cross-spike consistency: conflicting assumptions (e.g. gpui 0.2.2 vs git pin
   from S-a11y-design; `.toc` selection rules between S-addon/S-state/
   S-forever) and items the next phase must decide.
6. Commit (`run001(P3-review):`, Copilot trailer). Do not push or fix; list
   recommended fixes with owners.
