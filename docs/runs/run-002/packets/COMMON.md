# Run 002 common packet rules

Authority: SPEC.md, ACCEPTANCE.md, ENGINEERING.md, OWNER-DECISIONS.md (D-04,
D-05, D-08, D-09, D-14, D-16, D-17), [../PLAN.md](../PLAN.md). Read AGENTS.md
first. Each `R2-*` packet plus this file is the full work packet (E-05).

## Checkout and writes

- Worktree `D:\GitHub\wow-sync.worktrees\<ID>`, branch `run002/<ID>` from
  `main` as created by the orchestrator. Never edit any other checkout.
- Write only the paths your packet owns, plus your evidence
  `docs/runs/run-002/evidence/<ID>.md` (and `evidence/<ID>/` for small raw
  data, < 1 MB). Root `Cargo.toml`/`Cargo.lock` only if the packet says so.
- `cargo xtask verify --tier offline` from the worktree root must pass before
you commit. Never run `git gc`, `git prune`, reflog expiry or any history
rewrite of pushed commits (shared `.git`; owner-held, PLAN §6 #8). New product tests that are required must be listed in
- Stop only processes you started yourself, by the PID returned at launch.
Never stop processes inferred from ParentProcessId or other traversal
(PIDs are recycled; R2-check V1 incident). New product tests that are required must be listed in
  `tests/required-cases.toml` (see docs/VERIFY.md) by the packet that owns them.

## Environment

- Windows 11 x64 dev PC (Insider 26300). Fresh shells lack cargo on PATH:
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
- Rust 1.100.0 pinned (root `rust-toolchain.toml`); cargo-deny 0.20.2.

## Data and safety (non-negotiable)

Same as run-001 [S-common.md](../../run-001/packets/S-common.md) "Data and
safety": no Lua execution; never start/kill/automate WoW or Battle.net; WoW
data only per D-09/D-14; Dropbox only `D:\Dropbox\Gaming Sync\WoW Sync Test`
via exact paths with synthetic data; never commit addon contents, saved
settings, account/realm/character names, user names, SIDs or secrets.
Product tests use synthetic installs under a temp directory.

## Evidence and done

- Evidence states what was built, exact commands, real results labelled
  PASS / FAIL / BLOCKED / NOT RUN with environment, limits and open issues,
  linked to SPEC/ACCEPTANCE IDs.
- Commit on your branch with prefix `run002(<ID>):` and trailer
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
  Do not push, merge or open PRs. No sub-agents.
- Two failed approaches to the same problem: stop and report. Never weaken
  tests, thresholds or required cases to get green.
