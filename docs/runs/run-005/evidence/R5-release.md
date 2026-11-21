# R5-release evidence

Worktree `D:\GitHub\wow-sync.worktrees\R5-release`, branch `run005/R5-release`
(from main `fe13e3d`). Windows 11 x64. Nothing was pushed, tagged or published;
no GitHub API call was made. Links: D-26, PLAN goals 7 and 8, RV-10, RV-09 (the
workflow sets `WOWSYNC_BUILD_COMMIT`).

## Built

| Item | Result |
|---|---|
| Version 0.1.0 | Workspace `version` and every internal path dependency (`app`, `proto`, `runtime`) moved to 0.1.0. `cargo update -w` changed only the six workspace packages in `Cargo.lock`. Spikes are outside the workspace and unchanged. |
| `.github/workflows/release.yml` | Triggers `v*` tag push and `workflow_dispatch`. Job `build` (windows-latest): toolchain, pinned cargo-deny, both verify tiers, `cargo build --release --locked -p wow-sync-app --bin wow-sync` with `WOWSYNC_BUILD_COMMIT=${{ github.sha }}`, zip `wow-sync-<version>-windows-x64.zip` (exe, README, release notes as RELEASE-NOTES.md, `LICENSE*` if present) plus `.sha256`, artifact upload on every run. Job `publish` runs only on a tag push, has the only `contents: write`, checks tag == `v` + workspace version, then `gh release create --verify-tag --prerelease --notes-file`. Top-level permissions are `contents: read`. No secrets beyond `GITHUB_TOKEN`. |
| `docs/release/v0.1.0.md` | Honest notes: unsigned preview, SmartScreen, no installer, no macOS, Q-24 gate (no real WoW data), known gaps. |
| RV-10 | `--locked` added to the three xtask cargo invocations (clippy, offline test, native test). `docs/DEPENDENCIES.md` written from `cargo metadata --offline --locked`; REPO-MAP row added. |

## Commands and results

| Command | Result |
|---|---|
| `cargo update -w` | PASS, 6 workspace packages to 0.1.0 |
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links), Windows 11 |
| `cargo xtask verify --tier native` | PASS, Windows 11 |
| Workflow review | Read only; no `actionlint` available. NOT RUN: the workflow itself (needs GitHub). |

## Items for the orchestrator

1. **Unverified action SHAs.** `actions/upload-artifact` (v4.6.2,
   `ea165f8d…`) and `actions/download-artifact` (v4.3.0, `d3f86a10…`) were
   written from memory because there was no network. Confirm both against the
   release pages before pushing a tag; a wrong SHA fails the workflow. Run a
   `workflow_dispatch` first. The checkout SHA is copied from `ci.yml`.
2. **No LICENSE file exists in the repo** (only `license = "MIT"` in
   Cargo.toml). The zip step includes `LICENSE*` only if present, so the zip
   currently has none. Owner decision/file needed; not mine to add.
3. **Version extraction** uses `sed` over `Cargo.toml` (tolerates CRLF from
   Windows checkout). It could not be run locally (no sed on this PC); confirm
   on the first `workflow_dispatch`.
4. **Release notes content** assumes the other R5 packets land before the tag:
   Q-77 32 MiB cap and uninstall settings recovery (R5-remove), and the
   Documents-folder diagnostics save (R5-app; the notes avoid claiming
   no-overwrite). Adjust the notes if any is not merged.
5. The A-38 mutation controls were not applicable: no product behaviour was
   added. `--locked` is a flag; its effect is exercised by every verify run (an
   out-of-date lock now fails instead of updating).
6. `tests/required-cases.toml` not touched.

## What the update notice shows with only a pre-release

Analysis only; no product code changed.

- `adapters/src/update_http.rs` requests
  `https://api.github.com/repos/hamiltonia/wow-sync/releases/latest`. GitHub
  excludes pre-releases and drafts there, so with only `v0.1.0` published it
  answers **HTTP 404**.
- `fetch_latest` maps a non-2xx status to `FetchError::Http(404)`;
  `runtime/src/updates.rs` maps it to `UpdateFailKind::Http`, and
  `core/src/update.rs` `with_result` stores `UpdateResult::Failed`.
- **The user sees an error, not "no release yet":** About shows
  **"Could not check for updates."** and the activity log gets
  **"Update check failed: server refused"** (`UpdateCheckFailed`). No OS
  notice and no available-version text. The check is retried on the normal
  schedule and on every manual "Check for updates".
- Even if the endpoint returned a pre-release body, `parse_latest` rejects
  `prerelease: true` as `UpdateParseError::Prerelease`, which also becomes a
  failed check. So pre-releases can never produce an update offer.
- Route to the orchestrator: if "no release yet / newest" is wanted for 404,
  that is a product change (core `update.rs` or runtime `updates.rs`) and a
  design decision; it was not made here. The release notes already tell the
  user the message is expected.

## Limits

The release workflow has never run. Maintenance statuses in DEPENDENCIES.md
are not checked online (stated in the file).
