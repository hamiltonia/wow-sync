# Work packet R2-adapters: Windows and provider-folder adapters

Role: implementation. Model: claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

Implement `crates/wow-sync-adapters` exactly as specified by
[adapters-api.md](../../../design/adapters-api.md) and the rest of the slice
design as amended by design-review fixes on `main`. Adapters report
observations and perform requested effects inside authorized roots; they make
no policy decisions (E-08). R-04, R-05, R-11; S-proc, S-dropbox findings.

## Owned writes

Per [crates.md](../../../design/crates.md) §4: `crates/wow-sync-adapters/**`;
root `Cargo.toml` (exact §3 text); `Cargo.lock` (own dependencies);
`tests/required-cases.toml` (R2-adapters block appended at the end, §5.2);
evidence.

## Work

1. Modules from crates.md §2 and adapters-api.md (fs_safe containment and
   reparse refusal; scan with hashing and stability; install area lock,
   staging, slot rename, journals; provider root with atomic writes and cloud
   state; winproc by image path; localstate; testkit synthetic installs).
2. Required tests from tests.md for R2-adapters, unit tests inside modules,
   listed in `tests/required-cases.toml` with the tier the design gives
   (Windows-only ones under `native`). Negative cases: path escape, reparse
   point, partial write, locked file, unknown process access.
3. Tests use temp directories only. One optional ignored test may use
   `D:\Dropbox\Gaming Sync\WoW Sync Test` with ≤ 50 MB synthetic data; run it
   once and record the result; it must clean up after itself.
4. Never open, scan or modify real WoW folders. Process observation tests must
   not start WoW or Battle.net; use a harmless test executable copy.
5. After the orchestrator merges R2-core, follow crates.md §5.3 when asked.

## Done when

`cargo xtask verify --tier offline` passes and `cargo test -p
wow-sync-adapters` passes on Windows; evidence lists every required case
(offline and native) with its result and design gaps.
