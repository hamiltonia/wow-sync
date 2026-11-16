# Evidence: R2-design (run-002)

Packet: [R2-design](../packets/R2-design.md). Role: design. Model:
claude-opus-5.5 (as assigned; runtime selection to be confirmed by the
orchestrator from session records). Checkout:
`D:\GitHub\wow-sync.worktrees\R2-design`, branch `run002/R2-design`.

## Summary

Docs-only design for the run-002 vertical slice (enroll, inventory, publish
addon code through a provider folder, compute target, apply safely, status).

| Work item | Where |
|---|---|
| 1 Scope / out of scope | [design/README.md](../../../design/README.md) |
| 2 Crates, modules, trait sketches, file ownership, merge procedure | [crates.md](../../../design/crates.md), [core-api.md](../../../design/core-api.md), [adapters-api.md](../../../design/adapters-api.md) |
| 3 Domain: identities, variant validation, `.toc` rule, packages, versions | [domain.md](../../../design/domain.md) |
| 4 Publication format, hashes, atomic publish, what it proves, size | [publication.md](../../../design/publication.md) |
| 5 Apply algorithm, gates, staging, recovery, crash recovery | [apply.md](../../../design/apply.md) |
| 6 Status / provenance | [apply.md](../../../design/apply.md) §6, [publication.md](../../../design/publication.md) §6 |
| 7 Test strategy and required test names | [tests.md](../../../design/tests.md) |
| 8 Owner/design questions with defaults | [questions.md](../../../design/questions.md) |
| Overview + invariant map | [ARCHITECTURE.md](../../../ARCHITECTURE.md) |

Key structural choices (for reviewers):

- `wow-sync-adapters` does not depend on `wow-sync-core`, so R2-core and
  R2-adapters build in parallel; `wow-sync-runtime` (R2-slice) maps adapters
  onto core port traits.
- The step executor (journal-first commit, activity re-check before every
  commit rename, rollback, crash recovery) lives in core behind port traits so
  its ordering is tested with fakes and crash injection.
- Root `Cargo.toml` is written byte-identically by R2-core and R2-adapters;
  `tests/required-cases.toml` hunks are placed at opposite ends; `Cargo.lock`
  conflict resolved by R2-adapters after R2-core merges.
- Windows-only tests are listed under tier `native` because the `offline`
  tier also runs on macOS CI (Q-17).

Inputs used: SPEC, ACCEPTANCE, ENGINEERING, OWNER-DECISIONS, run-002 PLAN and
packets, previous ARCHITECTURE proposal, REPO-MAP, VERIFY, workspace manifests,
`tests/required-cases.toml`, run-001 REPORT and evidence (S-addon incl.
`folders.csv`, S-state, S-proc, S-dropbox, S-mem, S-check, S-gpui, P3-review,
P1-map), `spikes/addon/src/version.rs`. The S-forever report was **not** read;
only REPORT §3's summary (`wowf`) was used. No real WoW data, Dropbox folder
or internet was accessed. No code changed.

## Verification

Command (from the worktree root, after
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`):

```text
cargo xtask verify --tier offline
```

Result: see the final section below (recorded after the run).

## Owner / design questions

All 19 are in [questions.md](../../../design/questions.md) with recommended
defaults. Owner-facing ones that change product behavior: Q-1 package
grouping, Q-2 keep folders absent from the winner, Q-3 `wowf` toc order
(needs an owner-run client check), Q-4 rollback on launch mid-apply, Q-5
read/publish while WoW runs, Q-6 no hydration of online-only files, Q-8
provider test data, Q-9 staging location, Q-10 dotted-numeric family, Q-12
only `wowf` known, Q-14 non-ASCII names unsupported, Q-16 inaccessible
processes, Q-18 local-only recovery points. Design/orchestrator defaults:
Q-7, Q-11, Q-13, Q-15, Q-17, Q-19.

## Verify result

Run on Windows 11 x64 dev PC with all design files staged (tracked) before
commit: `fmt`, `clippy`, `test`, `required-cases`, `deny`, `markdown-links`
all PASS; `overall: PASS (allow-blocked: false) -> exit 0`. The report
reflects the staged tree (dirty) immediately before the commit that adds
this file; no non-doc file changed.
