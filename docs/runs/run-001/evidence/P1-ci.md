# Evidence: P1-ci (GitHub Actions)

Branch `run001/P1-ci`. Workflow: `.github/workflows/ci.yml` (only new file; no
release, publish or deploy workflow). Not pushed; no workflow has run yet.

## Design

- Triggers: push and pull_request to `main`. Top-level `permissions: contents: read`;
  no secrets referenced; checkout uses `persist-credentials: false`.
- Toolchain: `rustup toolchain install --profile minimal` with no argument reads
  `rust-toolchain.toml` (1.100.0, rustfmt, clippy, four targets). No third-party
  toolchain action is used, so the only action is `actions/checkout`.
- cargo-deny: `cargo install cargo-deny --version 0.20.2 --locked` (env
  `CARGO_DENY_VERSION`). 0.20.2 is the latest GitHub release of
  EmbarkStudios/cargo-deny (`gh api repos/EmbarkStudios/cargo-deny/releases/latest`),
  tag commit `87da103c554376c89a641116f835a41073a9d774`; crates.io API shows
  0.20.2 not yanked, rust_version 1.88.0.
- Jobs
  - `verify-offline` on `windows-latest` and `macos-latest`: runs
    `cargo xtask verify --tier offline` (interface from P1-verify) and prints
    `target/verify/report.json`.
  - `build-targets` (job name `build-only <target>`): `cargo build --workspace
    --locked --target <t>` for `x86_64-pc-windows-msvc` and
    `aarch64-pc-windows-msvc` on `windows-latest`, `x86_64-apple-darwin` and
    `aarch64-apple-darwin` on `macos-latest`. Nothing is run or uploaded.
  - Correction note (P3-fix): `macos-latest` is Apple Silicon, so
    `aarch64-apple-darwin` is the native build and `x86_64-apple-darwin` is a
    cross-compile. Both remain build-only; neither is executed.

## Pinned actions

| Action | Version | Full SHA | Source |
|---|---|---|---|
| actions/checkout | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` | `gh api repos/actions/checkout/releases/latest` -> v7.0.1; `gh api repos/actions/checkout/git/ref/tags/v7.0.1` (object type commit); confirmed by `git ls-remote https://github.com/actions/checkout refs/tags/v7.0.1` |

## What each job proves / does not prove

- `verify-offline`: on the GitHub-hosted Windows x64 and macOS (arm64 image,
  per GitHub docs) runners, fmt/clippy/test/deny/link checks defined by the
  offline tier pass (when P1-verify is merged), and the pinned toolchain
  installs. Does not prove native, provider, GUI, WoW, Dropbox or multi-computer
  behavior; BLOCKED tiers are not run here.
- `build-only <target>`: the workspace compiles (not links-and-runs) for the
  target. It does not prove runtime behavior, installability or support. The
  aarch64-windows and x86_64-darwin builds are cross-compiles on x64 Windows and
  arm64 macOS runners; no binary is executed. This fits D-07/A-35: those
  targets are "build in CI; untested".

## ARM64 runners

GitHub docs ("GitHub-hosted runners reference", fetched 2026-11-14) list
`windows-11-arm` and macOS Intel (`macos-15-intel`) as standard runners free for
public repositories; the repo is public (`gh api repos/hamiltonia/wow-sync`:
visibility public). They are available but deliberately not used: running tests
there is out of this packet's scope and would need separate acceptance. No
ARM64 or Intel-mac runtime evidence is claimed.

## Local validation

- actionlint v1.7.12 (release binary from rhysd/actionlint via `gh release
  download`, stored outside the repo): `actionlint -verbose
  .github\workflows\ci.yml` exit 0, 0 errors. shellcheck and pyflakes were
  not installed, so `run:` scripts were not shell-linted.
- The workflow itself was NOT executed (not pushed). Not run: any CI job,
  `cargo xtask verify`, builds for any target.

## Risks / notes

- `cargo xtask` and its `.cargo/config.toml` alias come from P1-verify; the
  `verify-offline` job fails until that branch is merged.
- `--locked` requires a committed, up-to-date `Cargo.lock` (present on main).
- Actually building `aarch64-pc-windows-msvc` needs the MSVC ARM64 build tools
  on `windows-latest`; unverified until the first real run.
- `cargo install cargo-deny` compiles from source each run (no caching) and
  may be slow; acceptable for now.
- The pinned `deny.toml` is owned by P1-verify.
