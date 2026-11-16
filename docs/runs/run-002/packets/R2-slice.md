# Work packet R2-slice: runtime glue and minimal GUI shell

Role: implementation. Model: claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

Wire core and adapters into a usable thin slice per
[crates.md](../../../design/crates.md) §1, §4, §5 and §7 and
[tests.md](../../../design/tests.md) §5 and §5b: enroll, status, publish and
apply with two synthetic installs, no terminal needed for supported actions.
R-02..R-05, R-11, R-12; A-06..A-09, A-20..A-22, A-24..A-26, A-46 (slice
level only); A-29 method per PLAN §6.6.

## Owned writes

`crates/wow-sync-runtime/**`, `crates/wow-sync-app/**`; root `Cargo.toml`
(`[workspace.dependencies]` additions only); `Cargo.lock`;
`tests/required-cases.toml` (append after the R2-adapters block);
`deny.toml` (only the R2-pin proposal adopted in PLAN §6.4);
`docs/REPO-MAP.md` (crate rows); `.github/workflows/*` only if the app crate
breaks an existing build job (record why); evidence and
`docs/runs/run-002/evidence/R2-slice/` (synthetic-data screenshots only).

## Work

1. `wow-sync-runtime`: port-trait implementations over adapters,
   `SliceService` (enroll / sync_once / status), failure-injecting effects
   wrapper, `e2e` required cases from tests.md §5. No ordering, grouping,
   target, plan or status logic outside core.
2. `wow-sync-app`: GPUI shell on the R2-pin pin (see
   `docs/runs/run-002/evidence/R2-pin.md` and `spikes/gpui-pin`): tray,
   close-to-hide, single instance, windowed subsystem, AccessKit labels, and
   the screens needed for G1–G9; sync loop on a background thread; calls
   `SliceService` only.
3. A developer launch mode pointing at a synthetic install pair and a temp
   provider folder (configuration file or command-line arguments read by the
   app; document it in evidence). Never default to real WoW or Dropbox paths.
4. Run G1–G9 once yourself (tests.md §5b), recording pass/fail/blocked per
   item. Measure memory once per PLAN §6.6 (idle, window hidden).
5. Stop processes you start by PID.

## Done when

`cargo xtask verify --tier offline` passes; `cargo test --workspace` passes
on Windows; evidence lists every slice case, G1–G9 results, memory, and
known gaps.
