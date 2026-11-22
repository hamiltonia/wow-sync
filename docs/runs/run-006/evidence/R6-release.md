# R6-release evidence

- Goal 0: `url` 2.5.9 at `ef3a92e` (both tiers passed; merged to main).
- Goal 1: workspace version 0.2.0. `release.yml` reads it from `Cargo.toml`;
  About and diagnostics use `env!("CARGO_PKG_VERSION")`. Hard-coded `0.1.0`
  strings remain only in tests and prototype fixtures. No tag created.
- Goal 2: `docs/release/v0.2.0.md`. First-run safety, Forever rule
  (Q-3 unchecked), reconfirmation and Test mode wording follow
  `evidence/R6-gate.md`; `TBD-R6-gate` is gone.
- Goal 3: `release.yml` already uses `docs/release/$TAG.md` (publish) and
  `docs/release/v$VERSION.md` (zip); no change needed.
- Goal 4: README status updated; run-005 report link kept; the HANDOFF link
  points to the existing `docs/runs/run-006/HANDOFF.md`.
- Goal 5: `devkit.rs` wording updated; marker writer kept. No other docs or
  scripts say the marker is required.

## Verification (E-08)

Environment: Windows 11 (NT 10.0.26300), x64 dev PC, rustc 1.100.0, run from
`D:\GitHub\wow-sync.worktrees\R6-release` after `git merge main` (`6336525`),
with the 0.2.0 bump, the notes and the RV6-01/RV6-02 wording fixes in the
tree. Prefix: `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Command | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) |
| `cargo xtask verify --tier native` | PASS |

The same two commands also passed after the 0.2.0 bump (before the notes
were written) and after the first notes commit (offline only).

