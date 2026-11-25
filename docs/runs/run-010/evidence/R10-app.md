# R10-app evidence

Branch run010/R10-app on main 33261fe. Version bumped to 0.3.3.

## Implemented
Activity grouped by pass with per-add-on rows and capped file lists; add-on
"Recent changes"; throttled (500 ms) progress headline/tray with incoming
summary and a 20 s "Updated N add-ons" note; log lines for pass start, phases,
each package (with ms) and pass summary. Code: history_ui.rs, progress_ui.rs,
world.rs, model.rs, backend.rs, app.rs, pages.rs.

## Results
- cargo test -p wow-sync-app: passed (173 + new tests), Windows, local.
- cargo xtask verify --tier offline / --tier native: both PASS (after fmt/clippy fix), Windows local.

## Not run / blocked
- GUI screenshots on synthetic data: NOT RUN. The runtime history accessors
  return empty until R10-history step 2 lands, so a live Activity page would
  show nothing to capture; the view logic is covered by fake-driven unit tests.
- No instance was started; owner's instance and data untouched.

## Open issues
- decision_satisfied matching assumes folder key or add-on title strings.
- Service-emitted Idle/Scanning events unverified; backend emits its own Idle.
