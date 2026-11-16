# Crates, modules and file ownership

Part of the [slice design](README.md). Goal: R2-core and R2-adapters build in
parallel without touching the same files; R2-slice wires them afterwards.

## 1. Crates

```text
crates/
  wow-sync-core/      R2-core      platform-free policy: identities, parsing, ordering,
                                    records, target, plans, executor over port traits,
                                    status/explanations. No OS I/O, no clock, no RNG.
  wow-sync-adapters/  R2-adapters  effect libraries: safe filesystem, scanning + hashing,
                                    install staging/rename/journal files, provider folder,
                                    Windows process observation, clock/RNG, local state,
                                    synthetic-install testkit. Does NOT depend on core.
  wow-sync-runtime/   R2-slice     glue: implements core port traits with adapters;
                                    SliceService (enroll / sync_once / status) used by
                                    the GUI; headless end-to-end tests.
  wow-sync-app/       R2-slice     GPUI tray/window binary on the R2-pin GPUI pin; calls
                                    SliceService only; holds no policy.
```

Dependency graph (arrows = "depends on"):

```text
wow-sync-app ──► wow-sync-runtime ──► wow-sync-core
                        └───────────► wow-sync-adapters
```

`wow-sync-adapters` does not depend on `wow-sync-core`, so the two can be built
in parallel. Its API uses primitive types (strings, `[u8; 32]`, paths, small
enums) that mirror the core port traits one-to-one ([adapters-api.md](adapters-api.md)
§8 gives the mapping), so the runtime glue is mechanical. This also keeps every
policy decision in core: adapters never decide what is newer, deletable,
complete or safe; they report observations and perform requested effects
inside authorized roots (ARCHITECTURE §2.3).

Why the executor (step sequencing, revalidation, crash recovery) is in core:
it must make no policy choice, but its ordering is safety-critical (R-11,
A-22) and must be testable with in-memory fakes and failure injection. It is
generic over port traits and performs no I/O itself.

Process model for the slice: one process (GPUI app with the sync loop on a
background thread), per S-mem option 1. Not a final decision.

## 2. Modules

### `wow-sync-core` (R2-core)

| Module | Contents |
|---|---|
| `lib.rs` | `#![forbid(unsafe_code)]`, module declarations, re-exports |
| `ids` | `ComputerId`, `EnrollmentId`, `GroupId`, `OpId` (32 lowercase hex), `Sha256` |
| `hash` | incremental SHA-256 wrapper (`sha2`), tree and package hash composition |
| `paths` | relative-path and folder-name validation, `FolderKey` |
| `bpsv` | parser for Blizzard pipe-separated metadata (`.build.info`, `.flavor.info`) |
| `variant` | variant observation, validation verdicts, known profiles, confirmation |
| `toc` | `.toc` header parser and the single `.toc` selection rule |
| `version` | version scheme classification and ordering |
| `package` | folder metadata, package grouping, package comparison |
| `inventory` | builds a validated local inventory from adapter scan results |
| `records` | versioned JSON records (encode/decode/validate) |
| `layout` | provider-folder and install work-area relative paths |
| `target` | group target computation |
| `plan` | publish plan and apply plan (pure) |
| `exec` | port traits `PublishEffects`, `ApplyEffects`; `run_publish`, `run_apply`, `recover_interrupted` |
| `status` | status derivation for local computer, packages and members |
| `explain` | `Explanation` enum and English rendering; audit entries |
| `testkit` | builders and `FakeEffects`; `#[cfg(any(test, feature = "testkit"))]` |
| `arch` | test-only architecture check (no OS I/O in core sources) |

### `wow-sync-adapters` (R2-adapters)

| Module | Contents |
|---|---|
| `lib.rs` | module declarations; `FsError` |
| `fs_safe` | `Root` (authorized directory), containment, link/reparse refusal |
| `fs_scan` | AddOns scan with hashing and stability check; variant metadata reads |
| `fs_install` | `InstallArea`: lock, staging, slot renames, journals, recovery records |
| `provider` | `ProviderRoot`: bounded reads, atomic writes, listings with cloud state, blob put/copy |
| `winproc` | WoW client observation by image path (`cfg(windows)`); `Unknown` elsewhere |
| `sysinfo` | `now_unix_ms`, `random_id_hex` |
| `localstate` | per-computer app state directory: atomic named files, bounded audit append |
| `testkit` | `TempDir`, `SyntheticInstall`, synthetic dataset generator; `#[cfg(any(test, feature = "testkit"))]` |

Test module names must be unique across the workspace because
`tests/required-cases.toml` matches libtest names without the crate
([../VERIFY.md](../VERIFY.md)). Core and adapter module names above are
disjoint; keep required tests in `#[cfg(test)] mod tests` inside each module
(printed as `<module>::tests::<name>`). Do not put required tests in
`tests/*.rs` integration files.

## 3. Dependencies

Root `Cargo.toml` after R2-core and R2-adapters (both write exactly this file,
byte for byte, LF line endings, final newline; see §5):

```toml
[workspace]
resolver = "2"
members = ["crates/*", "xtask"]

[workspace.package]
version = "0.0.0"
edition = "2021"
license = "MIT"
rust-version = "1.100.0"

[workspace.dependencies]
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.154"
sha2 = "=0.10.9"
getrandom = "=0.3.4"
windows-sys = "=0.59.0"
```

| Crate | May depend on |
|---|---|
| `wow-sync-core` | `serde`, `serde_json`, `sha2` (workspace). Nothing else; `arch` test enforces it |
| `wow-sync-adapters` | `sha2`, `getrandom`; `windows-sys` under `[target.'cfg(windows)'.dependencies]` with features `Win32_Foundation`, `Win32_System_Threading`, `Win32_System_Diagnostics_ToolHelp`, `Win32_Storage_FileSystem`. No core dependency. No `tempfile` (testkit has its own `TempDir`) |
| `wow-sync-runtime` | core, adapters (path deps), `serde_json` if needed |
| `wow-sync-app` | runtime, GPUI pin and tray crates per R2-pin evidence |

Versions: serde/serde_json are the ones already locked by `xtask`; sha2,
getrandom and windows-sys are versions already present in the dev PC cargo
cache (windows-sys 0.59 was used by the S-proc spike). All are MIT/Apache-2.0
licensed and must pass the existing `deny.toml` unchanged; if `cargo deny`
rejects one, stop and report rather than editing `deny.toml`.

## 4. File ownership

| Path | Owner | Rule |
|---|---|---|
| `crates/wow-sync-core/**` | R2-core | — |
| `crates/wow-sync-adapters/**` | R2-adapters | — |
| `crates/wow-sync-runtime/**`, `crates/wow-sync-app/**` | R2-slice | — |
| root `Cargo.toml` | R2-core **and** R2-adapters | Both write the identical file in §3 (identical edits merge cleanly). R2-slice may later add `[workspace.dependencies]` entries only |
| `Cargo.lock` | each packet for its own dependencies | Conflict procedure §5 |
| `tests/required-cases.toml` | R2-core, R2-adapters, R2-slice | Placement rule §5 |
| `deny.toml` | R2-slice only | Only the changes proposed by R2-pin evidence and listed as provisional in PLAN §6.4; nothing else |
| `.github/workflows/*` | nobody in R2-core/R2-adapters | R2-slice only if the app crate breaks the existing 4-target build job; record why |
| `docs/REPO-MAP.md` | R2-slice | Add the four crate rows once they exist |
| `docs/ARCHITECTURE.md`, `docs/design/**` | R2-design | Review findings return to a design retry, not to implementers |
| `docs/runs/run-002/evidence/<ID>.md` | each packet | — |

## 5. Merge procedure (collision avoidance)

Merge order: R2-core, then R2-adapters, then R2-slice.

1. **Root `Cargo.toml`**: R2-core and R2-adapters each replace the root file
   with the exact text in §3. Do not reformat or reorder it.
2. **`tests/required-cases.toml`** (currently: 3 header comment lines, a blank
   line, then the `A-39` case):
   - R2-core inserts its block **after the blank line 4 and before the
     `# A-39` comment** (top of file).
   - R2-adapters appends its block **at the end of the file**, after the
     `A-39` case.
   - R2-slice appends after the R2-adapters block (it starts after both merged).
   The two hunks are separated by unchanged lines, so git merges them cleanly.
   Case IDs are distinct per packet ([tests.md](tests.md) §1), so the
   duplicate `(id, tier)` check cannot trip.
3. **`Cargo.lock`**: expected to conflict between R2-core and R2-adapters.
   The orchestrator merges R2-core first, then asks R2-adapters (not the
   orchestrator, E-01/E-05) to merge `main` into `run002/R2-adapters`, take
   `main`'s `Cargo.lock`, run `cargo check --workspace` (without `--locked`;
   adds only missing packages, does not upgrade locked ones because all new
   versions are `=` pinned), rerun `cargo xtask verify --tier offline`, and
   commit. R2-slice starts from `main` after both merges.
4. Nobody edits `crates/wow-sync-core/src/lib.rs` except R2-core (the current
   skeleton line is replaced).

## 6. Architecture checks (A-43)

- `wow-sync-core` has `#![forbid(unsafe_code)]`.
- `arch::tests::core_sources_use_no_os_io_or_clock` reads the crate's own
  `src/**/*.rs` (test-time `std::fs` is allowed in that test only) and fails
  if any file other than `arch.rs` and `testkit.rs` contains one of:
  `std::fs`, `std::process`, `std::net`, `std::env`, `std::thread`,
  `SystemTime`, `Instant::now`, `File::`, `getrandom`, `windows_sys`, `gpui`.
  It also parses `crates/wow-sync-core/Cargo.toml` and fails if
  `[dependencies]` contains anything except `serde`, `serde_json`, `sha2`.
- `wow-sync-runtime` and `wow-sync-app` must not re-implement ordering,
  grouping, target, plan or status logic; R2-review checks this by reading.
