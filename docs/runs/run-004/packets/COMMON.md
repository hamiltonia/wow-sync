# Run 004 common packet rules

Authority:
- SPEC.md, ACCEPTANCE.md and ENGINEERING.md.
- OWNER-DECISIONS.md: D-04, D-05, D-08, D-09, D-14, D-18, D-19, D-21, D-22,
  D-23, D-24 and D-25. D-21 overrides earlier Q-21 and removal defaults. D-23
  sets the run-004 scope and the R3D-13 simplification. D-25 accepts the
  run-004 design defaults (Q-55 to Q-87).
- [../PLAN.md](../PLAN.md).

Read AGENTS.md first. Each `R4-*` packet plus this file is the full work
packet (E-05).

## Checkout and writes

- **Where to work:** worktree `D:\GitHub\wow-sync.worktrees\<ID>`, on branch
  `run004/<ID>` from `main`, as created by the orchestrator. Never edit any
  other checkout, including `D:\GitHub\wow-sync`.
- **What you may write:**
  - only the paths your packet owns;
  - your evidence file `docs/runs/run-004/evidence/<ID>.md`;
  - small raw data in `evidence/<ID>/` (under 1 MB; screenshots allowed).
- **Root files:** change `Cargo.toml`, `Cargo.lock`, `deny.toml` or CI only
  if your packet owns them.
- **Required tests:** new required product tests are listed in
  `tests/required-cases.toml` (see docs/VERIFY.md) by the packet that owns
  them. A test that only runs on Windows must not be listed in the `offline`
  tier unless it also runs on macOS.
- **Paths in tests:** never use a literal `\\` or `/` separator in test
  paths. Use `Path::join` (run-003 macOS CI break).
- **Before you commit:** `cargo xtask verify --tier offline` must pass from
  the worktree root. Code packets also run `--tier native`.

## Forbidden actions (stop-the-run violations)

- Never run `git gc`, `git prune`, `git reset` against a shared ref, reflog
  expiry, rebase, or any history rewrite. `.git` is shared by all worktrees.
- Stop only processes you started yourself, using the PID returned at launch.
  Never stop processes found through ParentProcessId, name matching or any
  other traversal. PIDs are recycled (run-002 incident V1).
- Never touch `D:\WoWSyncDemo` or the processes it launches.
- Do not write files with `Set-Content`, `Out-File` or redirection. Use the
  edit/create tools or `cargo fmt` (incident I-1).

## Environment

- Windows 11 x64 dev PC. Fresh shells lack cargo on PATH; fix with
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
- Pinned versions: Rust 1.100.0 (root `rust-toolchain.toml`), cargo-deny
  0.20.2, GPUI at the Zed rev pinned in `Cargo.toml` (D-18). Do not change
  the GPUI pin.
- macOS is verified only by CI, plus the owner's Mac runs (D-23). Do not
  push.

## Data and safety (non-negotiable)

- Run-001 [S-common.md](../../run-001/packets/S-common.md) "Data and safety"
  applies:
  - Never execute Lua.
  - Never start, kill or automate WoW or Battle.net.
  - Dropbox only under `D:\Dropbox\Gaming Sync\WoW Sync Test`.
- Never commit any of the following:
  - addon contents or saved settings;
  - account, realm or character names;
  - user names, SIDs or user-profile paths;
  - secrets.
- Product tests use synthetic installs under a temp directory. Manual labs
  use `D:\WoWSyncLab\<ID>\`.
- The Q-24 synthetic marker gate stays.
- No Dropbox API calls (D-23). The only allowed network call is the
  GitHub Releases check (D-24, P4-1), and only in its owning packet.

## Evidence and done

- **Evidence must state:**
  - what was built;
  - the exact commands run;
  - real results labelled PASS / FAIL / BLOCKED / NOT RUN, with the
    environment;
  - limits and open issues;
  - links to the relevant SPEC/ACCEPTANCE IDs.
- **New behaviour tests:** record A-38 negative controls. Temporarily mutate
  the production code, observe a named test fail with exit 101, then restore.
  Never commit a mutation.
- **New dependencies:** justify each under E-06 (licence, maintenance, why
  it's needed). `cargo deny` must pass.
- **Commits:** prefix `run004(<ID>):` and end with the trailer
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
  - Do not push, merge or open PRs.
  - Do not launch sub-agents.
- **When stuck:** after two failed approaches to the same problem, stop and
  report. Never weaken tests, thresholds or required cases to get green.
- **Final reply:** a short summary, the final SHA, counts, and anything
  skipped or blocked.
