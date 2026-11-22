# Run 006 common packet rules

Authority:
- SPEC.md, ACCEPTANCE.md and ENGINEERING.md.
- OWNER-DECISIONS.md, in particular D-04, D-05, D-08 as amended by D-28,
  D-09, D-14, D-18, D-21 to D-28. D-28 removes the Q-24 marker gate by
  default, sets the Q-3 working answer, makes "first run loses nothing" a
  requirement and approves [../PLAN.md](../PLAN.md).
- Design: `docs/design/` (domain.md §2–3, removal.md, settings.md,
  contracts.md, questions.md).

Read AGENTS.md first. Each `R6-*` packet plus this file is the full work
packet (E-05). Run-005 [COMMON.md](../../run-005/packets/COMMON.md) applies
unchanged except as below; this file wins where they differ.

## Checkout and writes

- **Worktree:** `D:\GitHub\wow-sync.worktrees\<ID>`, branch `run006/<ID>`,
  created by the orchestrator from `main`. Never edit any other checkout,
  including `D:\GitHub\wow-sync`.
- **Write only:**
  - the paths your packet owns;
  - `docs/runs/run-006/evidence/<ID>.md`;
  - small raw data in `evidence/<ID>/` (under 1 MB; screenshots allowed);
  - your section of `tests/required-cases.toml`, between
    `# >>> section: <ID>` and `# <<< section: <ID>`.
  Outside your section, you may only rename or remove a case whose test you
  deleted in your own files. Say so in the evidence.
- **File writes (incident I-5):** only the edit/create tools with
  **absolute** paths under your worktree or lab root, or `cargo fmt`. Never
  .NET file APIs, `Set-Content`, `Out-File` or redirection into repository
  files. Before every commit, `git -C D:\GitHub\wow-sync status --short`
  must be empty. If it is not, stop and report; do not clean it.
- **Before you commit:** `cargo xtask verify --tier offline` and
  `--tier native` both pass from the worktree root. If the native tier fails
  once, rerun it, keep both full logs, and report the failing test.
  Exception until the orchestrator says R6-release's `url` 2.5.9 update is
  merged: `offline/deny` may fail **only** on the `url` 2.5.8 advisory.
  Record that in your evidence, and `git merge main` once it lands.
- **If `main` moves:** `git merge main` (never rebase) when asked, resolve
  conflicts in your own files, re-run both tiers.

## Forbidden actions (stop-the-run violations)

- Never `git gc`, `git prune`, `git reset` against a shared ref, reflog
  expiry, rebase, `git push`, `git tag` or any history rewrite.
- Stop only processes you started, by the PID captured at launch.
- Never touch `D:\WoWSyncDemo` or the processes it launches.
- Do not launch sub-agents.

## A-38 controls

Make the temporary mutation with one tool call. Run the named test in a
**separate** call and observe the failure. Revert, then confirm
`git diff --exit-code` on that file before any commit. Never put an edit
and a commit in the same batch.

## Environment

- Windows 11 x64 dev PC. Fresh shells lack cargo on PATH; fix with
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` in the same command.
- Rust 1.100.0, cargo-deny 0.20.2, GPUI pinned (D-18). Do not change pins.

## Data and safety (non-negotiable)

- Never execute Lua or saved-variable files. Never start, kill or automate
  WoW or Battle.net.
- **Real dev-PC data (D-14, D-28):**
  - Only R6-check touches `D:\Games\World of Warcraft\_forever_`, and only
    after a hashed backup.
  - Implementation packets read its metadata at most (folder names, `.toc`
    headers) and never write to it.
- **Dropbox:** only `D:\Dropbox\Gaming Sync\WoW Sync Test`. Real data is
  allowed there (D-28); nothing else under `D:\Dropbox\Gaming Sync` is in
  scope.
- **Never commit:**
  - add-on contents, saved settings, or account, realm or character names;
  - user names, SIDs or user-profile paths;
  - secrets.
  Add-on folder names and toc metadata of public add-ons are fine.
- Product tests use synthetic installs under a temp directory. Manual labs
  use `D:\WoWSyncLab\<ID>\`.
- No Dropbox API calls. The only network call in product code is the D-24
  GitHub Releases check.

## Evidence and done

- **Evidence states:**
  - what was built;
  - the exact commands;
  - results labelled PASS / FAIL / BLOCKED / NOT RUN, with the environment;
  - limits and open issues;
  - links to SPEC/ACCEPTANCE IDs.
- New behaviour gets tests, plus at least one A-38 control per goal.
- New dependencies: justify under E-06.
- **Commits:** prefix `run006(<ID>):`, end with the trailer
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
  Leave the tree clean.
- **When stuck:** after two failed approaches, stop and report. A product
  question that the design and D-28 do not answer: stop that item and
  report it. Never weaken tests, thresholds or required cases.
- **Final reply:** a short summary, the final SHA, counts, and anything
  skipped or blocked.
