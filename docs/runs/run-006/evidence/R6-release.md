# R6-release evidence

- Goal 0: `url` 2.5.9 at `ef3a92e` (both tiers passed; merged to main).
- Goal 1: workspace version 0.2.0. `release.yml` reads it from `Cargo.toml`;
  About and diagnostics use `env!("CARGO_PKG_VERSION")`. Hard-coded `0.1.0`
  strings remain only in tests and prototype fixtures. No tag created.
- Goal 2: `docs/release/v0.2.0.md`. **Placeholder `TBD-R6-gate`** remains in
  the First-run safety section, pending R6-gate evidence.
- Goal 3: `release.yml` already uses `docs/release/$TAG.md` (publish) and
  `docs/release/v$VERSION.md` (zip); no change needed.
- Goal 4: README status updated; run-005 report link kept; the HANDOFF link
  points to the existing `docs/runs/run-006/HANDOFF.md`.
- Goal 5: `devkit.rs` wording updated; marker writer kept. No other docs or
  scripts say the marker is required.
