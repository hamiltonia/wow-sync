# Evidence P3-review: integration and maintainability review of `main`

Worker P3-review, run-001 Phase 3. Packet: [P3-review.md](../packets/P3-review.md)
(data rules from [S-common.md](../packets/S-common.md)). Requested model
claude-opus-5.5 (D-04); actual runtime model is recorded by the orchestrator.

- Checkout `D:\GitHub\wow-sync.worktrees\P3-review`, branch `run001/P3-review`,
  based on `main` `5be6ba0` (all Phase 2 spikes merged; S-forever report
  intentionally not on `main`).
- Environment: Windows 11 x64 dev PC (Insider 26300), Rust 1.100.0, cargo-deny
  0.20.2, `gh` authenticated read-only use. Review only: nothing fixed, merged,
  dismissed, closed or pushed. No spike or GPUI app was built or run (S-check
  was measuring memory in parallel); only lockfiles, metadata, `cargo metadata
  --offline` and `cargo deny check` were used.
- Scope: [ENGINEERING.md](../../../../ENGINEERING.md) E-06 (security, data,
  dependencies) and E-08 (maintainability) applied to the merged tree.

## Summary

No product-code security issue was found: the product workspace
(`crates/wow-sync-core`, `xtask`, 22 locked packages) is unaffected by every
open alert, and no secrets, user paths, SIDs or WTF names were found in tracked
files. The findings are about repository hygiene, the dependency policy the
product will meet when it adopts GPUI, and spike assumptions that disagree.

| ID | Severity | Finding | Owner of fix |
|---|---|---|---|
| M1 | Medium | Root `.gitignore` is `/target` (root only). S-addon once committed 176 `spikes/addon/target/` files; amended away before merge, but ~647 MiB of unreachable build objects remain in the shared `.git` | Orchestrator |
| M2 | Medium | 4 open Dependabot alerts (not 2), all in spike lockfiles; Dependabot security jobs fail on `main` (`update_not_possible`); cargo-deny does not see the `grid` advisory | Orchestrator, owner decision |
| M3 | Medium | `deny.toml` will reject any GPUI product build: 5 non-allowlisted licenses, unmaintained advisories, and (for a git pin) git sources | Owner decision, then orchestrator |
| M4 | Medium | Spikes disagree on `.toc` selection; no spike verified which `.toc` the Forever client loads | Design packet + owner-run check |
| M5 | Medium | Three GPUI identities in evidence (crates.io 0.2.2; Zed `main` `24d3772…`; recommended tag `v1.28.1`); memory and tray results hold for 0.2.2 only | Owner decision (D-12), then re-check packet |
| L1 | Low | `HANDOFF.md` is stale (Phase 2 table "launching/pending"; next action 4 already done) | Orchestrator (P3-report) |
| L2 | Low | S-gpui evidence still says Windows ARM64 BLOCKED and "the run will show it"; CI run `43756896472` built all 4 targets (build-only) and is recorded only in `USAGE.md` | Orchestrator (P3-report) |
| L3 | Low | Some S-a11y screenshots include edge bleed of other desktop windows (unreadable fragments of unrelated text) | Owner decides; future packets capture client area only |
| L4 | Low | `spike-gpui.yml`: no concurrency group; not listed in `REPO-MAP.md` | Orchestrator |
| L5 | Low | Spike code patterns that must not be ported: duplicate `.toc` parsers, unreadable `.toc` files treated as empty, no `license` field, mixed editions/naming | Future product packets |
| L6 | Low | CI tooling maintenance: runner images `-latest`, no documented update procedure for the pinned action SHA / cargo-deny version | Orchestrator |

PASS / FAIL / BLOCKED labels for each check are in the sections below.

## 1. Repository structure

| Check | Result | Evidence |
|---|---|---|
| Each Rust spike has its own empty `[workspace]` | **PASS** (dev PC, file read) | `spikes/{a11y,a11y-main,addon,gpui,mem,proc,state}/Cargo.toml`; `spikes/dropbox` is a script only |
| Root workspace members are only `crates/wow-sync-core`, `xtask` | **PASS** | root `Cargo.toml` |
| Root verify does not build spikes | **PASS** (by reading `xtask/src/main.rs`: `cargo fmt --all`, `clippy/test --workspace`, cargo-deny on the root manifest) | — |
| CI builds spikes only via `spike-gpui.yml` | **PASS** | `ci.yml` uses `--workspace` from root; `spike-gpui.yml` has `working-directory: spikes/gpui` |
| Spike `target/` ignored | **PASS now, by per-spike `.gitignore` only** (`git check-ignore -v`) | root `.gitignore` is `/target`, which does not match `spikes/*/target`. [S-common.md](../packets/S-common.md) says the root file covers `target/`; it does not |
| No build output in history reachable from any ref | **PASS** (`git rev-list --objects --all`; largest blob 190 KB, a Cargo.lock) | — |
| Unreachable build output in the shared object store | **FAIL (hygiene, local only)** | `git count-objects -vH`: 4,430 loose objects, 647 MiB. Unreachable commit `44c29c9` ("run001(S-addon): …") contains 176 `spikes/addon/target/` paths; largest loose blobs are `.rlib`/`.rmeta` (43 MB, 42 MB, 19 MB). It was amended before `33d6f83`, never merged or pushed |
| Committed size | **PASS** (acceptable) | 200 tracked files, 1.9 MB: four GPUI lockfiles ≈ 724 KB, S-a11y evidence 475 KB (19 PNG ≈ 25 KB each), S-mem raw 144 KB; every evidence folder < 1 MB |

M1 cause: the root ignore rule only covers the root `target/`; S-addon's first
commit staged its build tree before adding `spikes/addon/.gitignore`. The
per-spike files fixed it, but any new spike without one repeats the risk, and
a public push would have published compiled artifacts.

## 2. Dependencies and Dependabot

### Alerts (`gh api repos/hamiltonia/wow-sync/dependabot/alerts`, 2026-11-15)

| # | Crate | Advisory | Severity | Manifest | Patched | Product affected? |
|---|---|---|---|---|---|---|
| 1 | `glib 0.18.5` | GHSA-wrw7-89jp-8q8g (RUSTSEC-2024-0429), unsound `VariantStrIter` | medium | `spikes/gpui/Cargo.lock` | 0.20.0 | No. Reached only via `tray-icon 0.26.1` → `muda`/`libappindicator` → `gtk 0.18`, which are Linux-only target dependencies; Linux is not a target in `rust-toolchain.toml`/`deny.toml` and the code is never compiled for Windows/macOS |
| 2–4 | `grid 0.18.0` | GHSA-38c5-483c-4qqp (CVE-2026-42199), integer overflow in `Grid::expand_rows` | medium | `spikes/gpui`, `spikes/a11y`, `spikes/mem` lockfiles | 1.0.1 | Not today (product lockfile has 22 packages, no GPUI). **Would be** if the product adopts crates.io `gpui 0.2.2`: `gpui 0.2.2` → `taffy 0.9.0` → `grid 0.18`. Not present with the Zed git pin (`taffy 0.15.0`, no `grid`; `spikes/a11y-main/Cargo.lock`) |

- Dependabot PRs: **none** (`gh pr list --state all` empty). Repo has
  Dependabot security updates enabled, no `.github/dependabot.yml`.
- Dependabot security-update runs on `main` (e.g. `43761113956`) end
  "Requirements to unlock update_not_possible … latest possible version of
  grid … is 0.18.0", reported as **failed** "Dependabot Updates" runs. They
  will keep failing while `taffy 0.9` pins `grid 0.18`.
- `cargo deny check advisories` (read-only, root `deny.toml`, run in each
  spike): `spikes/gpui` and `spikes/a11y-main` FAIL on *unmaintained* only;
  **neither `grid` nor `glib` is reported**. `glib` is filtered because Linux is
  not in `[graph] targets` (correct). `grid` has no RustSec entry in the local
  advisory DB (`~/.cargo/advisory-dbs/.../crates/grid` absent): a GHSA-only
  advisory that the CI gate cannot see. Dependabot/GHSA is therefore a
  complementary signal, not redundant.

### Licenses and sources against `deny.toml` (dev PC, `cargo deny check licenses sources`)

| Spike | Licenses | Sources | Unmaintained (RustSec) |
|---|---|---|---|
| `spikes/gpui` (0.2.2) | **FAIL**: BSD-2-Clause (`arrayref`, `av1-grain`, `rav1e`, `v_frame`), MPL-2.0 (`dwrote`, `option-ext`, `cbindgen` build-time), Zlib (`foldhash`, `nanorand`, `slotmap`, `zlib-rs`), CC0-1.0 (`hexf-parse`, `tiny-keccak`), NCSA in `libfuzzer-sys`; spike crate itself unlicensed | PASS | async-std, paste, pin-utils, proc-macro-error2, rustls-pemfile, rustybuzz, ttf-parser |
| `spikes/a11y-main` (git `24d3772…`) | **FAIL**: same families minus CC0/`nanorand` | **FAIL**: Zed git sources not in `allow-git` | rustybuzz, ttf-parser |
| `spikes/proc` (`windows-sys`) | FAIL only because the spike crate has no `license` field | PASS | none |

All five license families are permissive or file-level copyleft (MPL-2.0) and
are generally used with MIT projects, but adding them to `deny.toml` is an
E-06 decision (license compatibility, MPL source-availability duty for
`dwrote` if distributed). This is expected for spikes; it becomes a gate the
first time the product depends on GPUI (M3).

### Recommendation (M2)

1. **Accept for spikes now** (recommended): the alerts do not reach product
   code. Dismiss alerts 1–4 as "vulnerable code not used / tolerable risk in
   spike code", referencing this file, with owner approval (dismissal is a
   repository security action; not done here).
2. **Delete spike code after findings** when the next phase starts product
   GUI work (REPO-MAP already allows it): removes the lockfiles and the
   failing Dependabot jobs; history keeps the evidence. Keep
   `spikes/a11y-main` (or its successor pin) until the GPUI pin is decided.
3. **Do not exclude spikes from Dependabot** as the primary answer: excluding
   paths needs a `dependabot.yml` and would also hide GHSA-only advisories that
   cargo-deny misses. If noise matters more, prefer option 2.
4. For the product: if crates.io `gpui 0.2.2` is chosen, `grid` cannot be
   updated without a newer `taffy`/GPUI; record an explicit, owner-approved
   advisory exception or move to a pin without `grid`.

## 3. Workflows

| Check | `ci.yml` | `spike-gpui.yml` |
|---|---|---|
| `permissions: contents: read` at top level | PASS | PASS |
| Actions pinned to full SHA | PASS: `actions/checkout@3d3c42e…` = tag `v7.0.1` (verified with `gh api repos/actions/checkout/git/ref/tags/v7.0.1`) | PASS, same SHA |
| `persist-credentials: false` | PASS | PASS |
| Secrets, `pull_request_target`, artifact upload | none | none |
| Triggers | push/PR to `main` | `workflow_dispatch`, push touching `spikes/gpui/**` or itself (any branch; no PR trigger) |
| Tooling pinned | toolchain via `rust-toolchain.toml`; cargo-deny `0.20.2 --locked` | toolchain via root `rust-toolchain.toml` (resolved upward); `--locked` |
| Concurrency | yes | **none** (L4) |
| Last result | `main` `f9bf9e3` success | `43756896472` on `88242f1`: 4/4 build-only success |

Should `spike-gpui.yml` remain? **Yes, until the GPUI pin decision (M5)**: it
is the only CI evidence that GPUI builds for Windows ARM64 and both macOS
targets. After the decision, either retarget it to the chosen pin or replace
it with a product GUI build in `ci.yml`, then delete it with the spike. It
carries no secrets and only read permission, so the residual risk is CI
minutes. Add it to `REPO-MAP.md` and a concurrency group (L4).

## 4. Data hygiene (tracked files on `main`)

| Check | Result |
|---|---|
| User profile paths, SIDs (`S-1-5-21`), e-mail addresses, BattleTag-like `Name#1234` | **PASS** (regex scan of `docs/` and `spikes/`; only `Copilot@users.noreply.github.com` trailers and UIA class `#32769`) |
| WTF account/realm/character names | **PASS**: S-state TSVs use `<ACCOUNT-n>`/`<REALM-n>`/`<CHARACTER-n>`; `spikes/state/src/main.rs` redacts when printing |
| Addon contents | **PASS**: only derived metadata (folder names, `.toc` fields: versions, interface numbers, public project ids, dependency names) in `S-addon/folders.csv` and `coverage.md`; saved-variable *file names* only in S-state. Note: the list reveals which public addons the owner installs (low sensitivity, D-14 scoped) |
| Fixtures | **PASS**: `spikes/addon/fixtures/Syn*` and `spikes/state/fixtures/*.lua` are synthetic |
| Absolute paths | **PASS**: only D-09/D-14/Dropbox-test roots and `C:\Program Files (x86)\Dropbox\Client` (product install path) |
| Spike scripts touching real data | **PASS** by reading: `spikes/proc` spawns copies of itself under `%TEMP%\wowsync-proc` and only enumerates processes under the real root; `spikes/dropbox/observe.ps1` writes only under the test root; `spikes/gpui/gpu-pref.ps1` writes an HKCU GPU preference and restores/removes it |
| Screenshots | **Low (L3)**: `S-a11y/*.png` are window captures (496×559); the right and bottom edges show fragments of other desktop windows. No identifier was readable in the two images inspected, but captures should be client-area only |
| S-forever report (uncommitted, read-only review in its worktree) | Not on `main` by design (PLAN §7). It records `.build.info` build/CDN key hashes and region/locale tags. These are public CDN identifiers, not secrets, but the owner's review should confirm they may be published |

## 5. Cross-spike consistency and items to decide

### `.toc` selection (M4)

| Spike | Rule used |
|---|---|
| [S-addon](S-addon.md) | Parses each `.toc` whose name is `Folder.toc` / `Folder_Suffix.toc` / `Folder-Suffix.toc`; other names are listed as unmatched and ignored; an unreadable toc becomes an empty toc (`unwrap_or_default`). Notes upstream says suffixed tocs win over plain; `_Camelot`/`_Forever` are undocumented suffixes. Not verified on the client |
| [S-state](S-state.md) | Union of declarations over **every** `*.toc` in the folder, including names that do not match the folder; unreadable tocs are skipped silently (`filter_map(… .ok())`). Flags `flavor-dependent`. States the product should use the loaded toc instead |
| S-forever (uncommitted) | Variant identity from `.flavor.info` + `.build.info` + `.product.db`; explicitly not from Interface numbers. Counts `16001` across tocs at depth ≤ 2 (112 occurrences, a different unit from S-addon's 82 folders / 39 suffixed tocs) |

Consequences: version selection (S-addon ambiguity 5, a folder whose tocs
disagree on version) and ownership (S-state) both depend on which toc the
Forever client loads, and the two spikes would select differently for a folder
containing a stray `.toc`; both treat an unreadable toc as "no declarations"
rather than an error. **Decide in design:** one product `.toc` module and
one documented selection order (Forever: `_Forever`? `_Camelot`? plain?
hyphen and underscore), with unknown cases failing closed; confirm the order by
an owner-run client check.

### GPUI identity (M5)

- S-gpui (tray, hide/show) and S-mem (13–27 MiB private WS) ran on crates.io
  `0.2.2`; S-a11y's positive result ran on Zed `main` `24d3772…`;
  S-a11y-design recommends the SHA of tag `v1.28.1` (`e8bc16a…`, source
  inspection only, not built). Each is a different dependency tree (709 vs 585
  packages; `taffy` 0.9 vs 0.15; `blade` vs `wgpu`). S-a11y-design already
  states the memory and tray results must be re-run on the chosen pin; this
  review agrees and adds that M2/M3 differ by choice (0.2.2 brings `grid`;
  the pin needs `allow-git`).
- S-mem found that 0.2.2 quits when its last window closes on Windows;
  S-gpui keeps the window alive by hiding it. Whether the chosen pin behaves
  the same is unknown.
- Memory metric: S-mem uses perf counter "Working Set - Private"; S-gpui's
  59–62 MiB is total Working Set (informational). Private Bytes was
  55–82 MiB. A forced `EmptyWorkingSet` run read 0.1 MiB.

### Other cross-spike items for the next phase

1. Client write timing and `.bak` rotation (S-state NOT RUN) and live client
   detection (S-proc §4 BLOCKED) need one owner-run session with the real
   client; both feed R-07 capture and the "WoW running" block.
2. Dropbox online-only and conflicted copies (S-dropbox BLOCKED) need owner
   action or a second machine.
3. Character/account identity across computers (S-state decision 5) and
   multi-folder package grouping (S-addon) both use case-folded folder names;
   macOS case-sensitivity is untested (BLOCKED, no Mac).

## 6. Maintainability notes (E-08)

- `HANDOFF.md` still shows the Phase 2 wave table as "launching/pending" and
  lists a P1-ci correction that `P1-ci.md` already contains (L1). A fresh
  orchestrator would mis-resume; update it in P3-report.
- S-gpui evidence should cross-reference CI run `43756896472` (L2): local
  ARM64 build BLOCKED (no clang/ARM64 MSVC on the dev PC) vs CI ARM64
  build-only PASS are different claims.
- Spike code is proportional for spikes, but the following must not be
  copied into product code (L5): two independent `.toc` parsers
  (`spikes/addon/src/toc.rs`, `spikes/state/src/toc.rs`), silent
    `.ok()`/`unwrap_or_default()` handling of unreadable inputs
    (success-shaped fallback), `expect`/`unwrap`
  on filesystem calls, missing `license` fields, edition 2024 in one spike vs
  workspace edition 2021, and inconsistent crate names.
- CI tooling (L6): `windows-latest`/`macos-latest` move over time and there is
  no written procedure for updating the pinned checkout SHA and cargo-deny
  version. Document it (E-08 "maintenance") or add a `github-actions`-only
  Dependabot version-update config.

## Recommended fixes and owners

| Fix | For | Owner |
|---|---|---|
| Change root `.gitignore` to `target/` (any depth) and correct the S-common wording | M1 | Orchestrator |
| Optionally prune unreachable objects in the shared clone (`git gc --prune=now`; affects no ref) after all worktrees are settled | M1 | Orchestrator, owner OK |
| Dismiss alerts 1–4 with reason and link to this file, or delete spikes after the pin decision; do not add a path-exclusion Dependabot config | M2 | Owner decides; orchestrator executes |
| Record that cargo-deny misses GHSA-only advisories; keep Dependabot alerts on for product manifests | M2 | Orchestrator (VERIFY/ARCHITECTURE note) |
| Before the first GPUI product dependency: owner-approved `deny.toml` changes (licenses, unmaintained exceptions with reasons, `allow-git` + `required-git-spec = "rev"` if pinned) | M3 | Owner, then orchestrator |
| One design packet: single `.toc` module and selection order; owner-run client check of Forever toc loading | M4 | Design worker + owner |
| Pin decision, then one packet re-running S-gpui lifecycle, S-mem memory and 4-target CI on the pin | M5 | Owner (D-12), worker |
| Update `HANDOFF.md`, S-gpui CI cross-reference, `REPO-MAP.md` entry for `spike-gpui.yml` | L1, L2, L4 | Orchestrator |
| Client-area-only screenshots in future GUI evidence; owner decides whether to recapture current PNGs | L3 | Owner / future GUI packets |

## Decisions needed (owner unless noted)

1. GPUI source: crates.io `0.2.2` vs SHA-pinned Zed release tag (D-12, M5),
   which also decides `grid` exposure and `allow-git`.
2. License allowlist additions for GPUI (BSD-2-Clause, Zlib, MPL-2.0, CC0-1.0,
   NCSA) and policy for RustSec *unmaintained* advisories (M3).
3. Dependabot alerts on spikes: dismiss now, or delete spikes after findings (M2).
4. Memory gate counter: confirm "Working Set - Private" summed over app-owned
   processes as "private resident memory", and that forced working-set
   trimming does not count toward a pass (E-07 "no fake fast path").
5. Quit-on-last-window handling (anchor window vs pin behavior) (design).
6. `.toc` selection order for Forever (design + owner-run check, M4).
7. S-forever publication content (build/CDN keys, region tags) at owner review.

## Commands run (dev PC, Windows 11 x64)

```
git ls-files; git rev-list --objects --all | git cat-file --batch-check=...
git count-objects -vH; git fsck --unreachable --no-reflogs; git ls-tree -r 44c29c9
git check-ignore -v spikes/<name>/target/x
gh api repos/hamiltonia/wow-sync/dependabot/alerts
gh api repos/hamiltonia/wow-sync (security_and_analysis); gh pr list --state all
gh run list / gh run view 43761113956 --log (Dependabot); gh run view 43756896472
gh api repos/actions/checkout/git/ref/tags/v7.0.1
cargo deny --manifest-path spikes\<gpui|a11y-main|proc>\Cargo.toml --config deny.toml check advisories licenses sources
cargo metadata --manifest-path spikes\<gpui|a11y-main>\Cargo.toml --format-version 1 --locked --offline
```

Not run: any spike build or binary, cargo-deny on `spikes/a11y`/`spikes/mem`
(same `gpui 0.2.2` tree as `spikes/gpui` minus `tray-icon`, per lockfiles),
macOS anything (BLOCKED, no Mac).
