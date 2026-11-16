# Owner and design questions

Part of the [slice design](README.md). Each item is a choice the approved
intent does not settle. Implementers follow the **recommended default** until
the owner (product meaning) or orchestrator (engineering) records otherwise in
[OWNER-DECISIONS.md](../OWNER-DECISIONS.md) or the run plan. "Owner" items
change product behavior; "design" items are engineering defaults the
orchestrator may accept.

| # | Question | Recommended default | Who | Used in |
|---|---|---|---|---|
| Q-1 | How are folders grouped into one addon package? | Union of shared external id, hard dependency whose stem prefixes the dependent's name, and `X_`/`X-` name prefix; over-grouping preferred to mixing parts. `Questie`/`QuestieDB`-style relatives stay separate | owner | [domain.md](domain.md) §5 |
| Q-2 | When the winning version of a package lacks a folder this computer has, remove it? | No. Keep it (slice has no removal authority); show it in the package details. Removal waits for the uninstall design | owner | [apply.md](apply.md) §2 |
| Q-3 | For `wowf`, which `.toc` does the client load: `_Forever`, `_Camelot`, plain? | Assume `Forever`, then `Camelot`, then plain; disagreement between Forever and Camelot fails closed as ambiguous. **Needs an owner-run client check** (e.g. a test addon with differing tocs) before release | owner | [domain.md](domain.md) §3 |
| Q-4 | If WoW starts between two folder renames of one package, roll back or complete? | Roll back that package to its previous folders and stop the apply; report the residual race. If WoW appears only after the last rename, keep the new state and mark needs-help | owner | [apply.md](apply.md) §2–§3 |
| Q-5 | May the app read AddOns and publish while WoW is running? | Yes (reads only; SPEC blocks only game-file changes). Unstable files defer publishing | owner | [apply.md](apply.md) §1 |
| Q-6 | Should the app force-download (hydrate) online-only provider files? | No. Treat them as not yet available and wait; user can make the folder "available offline" in Dropbox | owner | [adapters-api.md](adapters-api.md) §5 |
| Q-7 | Provider storage: per-file blobs, no compression, no garbage collection in the slice? | Yes for the slice; GC/retention of unreferenced blobs must be designed before release (R-14) | design | [publication.md](publication.md) §5 |
| Q-8 | Provider-tier test data size and location | Synthetic only, ≤ 50 MB per run, under `D:\Dropbox\Gaming Sync\WoW Sync Test\<run-subdir>`, removed afterwards | owner | [tests.md](tests.md) §4 |
| Q-9 | Where do staging, journals and recovery points live? | `<variant>\.wowsync\` (same volume as AddOns, outside the AddOns folder so WoW does not see them) | owner | [apply.md](apply.md) §2 |
| Q-10 | Are semver-like, dotted-numeric and git-describe labels one comparable family? | Yes (spike data shows authors move between them); unknown pre-release words stay ambiguous | owner | [domain.md](domain.md) §4 |
| Q-11 | Must `.product.db` also agree for variant validation? | Not in the slice (binary format); `.flavor.info` + `.build.info` must agree. Revisit if a client is found where they diverge | design | [domain.md](domain.md) §2 |
| Q-12 | Which product codes are "known"? | Only `wowf` (Forever). Every other client, including retail/classic, goes through guided unfamiliar-client confirmation until a profile is verified | owner | [domain.md](domain.md) §2 |
| Q-13 | Record compatibility | Unknown fields ignored within a schema major; unknown major refused and shown as "written by another app version" | design | [publication.md](publication.md) §2 |
| Q-14 | Non-ASCII addon folder/file names | Unsupported in the slice (listed, never transferred). Needs a Unicode normalization/case-fold design before release | owner | [domain.md](domain.md) §5 |
| Q-15 | Quiescence window | Files modified in the last 2 s, or changed while hashed, make the folder unstable for this cycle | design | [apply.md](apply.md) §5 |
| Q-16 | Processes the app cannot inspect (access denied) | Not treated as WoW; count shown in diagnostics. Alternative (block on any inaccessible process) would block always on typical PCs | owner | [apply.md](apply.md) §5 |
| Q-17 | Windows-only tests are listed under tier `native`, which `verify` does not execute yet | Keep them `native`; R2-check verifies them from Windows `cargo test` output; a later packet makes `native` executable in xtask | design (orchestrator) | [tests.md](tests.md) §1 |
| Q-18 | Recovery points are local-only (not uploaded) in the slice (P1-map Q2) | Yes; group-wide rollback is out of slice and will need its own design | owner | [apply.md](apply.md) §2 |
| Q-19 | Process model | One process (GUI + background sync thread) for the slice, per S-mem option 1; not a release decision | design | [crates.md](crates.md) §1 |
