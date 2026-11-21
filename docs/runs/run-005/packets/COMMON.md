# Run 005 common packet rules

Authority:
- SPEC.md, ACCEPTANCE.md and ENGINEERING.md.
- OWNER-DECISIONS.md, in particular D-04, D-05, D-08, D-09, D-14, D-18,
  D-21 to D-27. D-26 sets Q-88 (a), Q-77 = 32 MiB and approves the `v0.1.0`
  Windows pre-release. D-27 approves [../PLAN.md](../PLAN.md).
- Design: `docs/design/` (removal.md, settings.md, rollback.md, ops.md,
  contracts.md, questions.md).

Read AGENTS.md first. Each `R5-*` packet plus this file is the full work
packet (E-05). Run-004 [COMMON.md](../../run-004/packets/COMMON.md) applies
unchanged except as below; this file wins where they differ.

## Checkout and writes

- **Where to work:** worktree `D:\GitHub\wow-sync.worktrees\<ID>`, on branch
  `run005/<ID>`, created by the orchestrator from `main`. Never edit any
  other checkout, including `D:\GitHub\wow-sync`.
- **What you may write:** only the paths your packet owns, your evidence
  file `docs/runs/run-005/evidence/<ID>.md`, and small raw data in
  `evidence/<ID>/` (under 1 MB; screenshots allowed).
- **Required tests:** add new required cases to `tests/required-cases.toml`
  only between your own pre-created markers `# >>> section: <ID>` and
  `# <<< section: <ID>`. Never edit other sections except
  to rename or remove a case whose test you deleted inside your own files,
  and say so in the evidence.
- **Before you commit:** `cargo xtask verify --tier offline` and
  `--tier native` must both pass from the worktree root.
- **If `main` moves under you:** run `git merge main` (never rebase) when
  the orchestrator asks, resolve conflicts in your own files, and re-run
  both tiers.

## Forbidden actions (stop-the-run violations)

- Never run `git gc`, `git prune`, `git reset` against a shared ref, reflog
  expiry, rebase, `git push`, `git tag`, or any history rewrite. `.git` is
  shared by all worktrees.
- Stop only processes you started yourself, using the PID returned at
  launch. PIDs are recycled (run-002 incident V1).
- Never touch `D:\WoWSyncDemo` or the processes it launches.
- Do not write files with `Set-Content`, `Out-File` or redirection. Use the
  edit/create tools or `cargo fmt` (incident I-1).
- Do not launch sub-agents.

## A-38 controls (run-004 lesson)

Make the temporary mutation with one tool call, run the named test in a
**separate** call and observe exit 101, revert, then confirm
`git diff --exit-code` on that file **before** any `git commit`. Never put
an edit and a commit in the same batch of calls.

## Environment

- Windows 11 x64 dev PC. Fresh shells lack cargo on PATH; fix with
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` in the same command.
- Rust 1.100.0 (`rust-toolchain.toml`), cargo-deny 0.20.2, GPUI pinned in
  `Cargo.toml` (D-18). Do not change the GPUI pin.
- macOS is verified by CI only; the owner will run MAC-OWNER.md later.

## Data and safety (non-negotiable)

- Never execute Lua or saved-variable files. Never start, kill or automate
  WoW or Battle.net. Dropbox only under `D:\Dropbox\Gaming Sync\WoW Sync Test`.
- Never commit addon contents, saved settings, account/realm/character
  names, user names, SIDs, user-profile paths or secrets.
- Product tests use synthetic installs under a temp directory. Manual labs
  use `D:\WoWSyncLab\<ID>\`. The Q-24 synthetic marker gate stays.
- No Dropbox API calls. The only network call in product code is the D-24
  GitHub Releases check.

## Evidence and done

- Evidence states what was built, exact commands, results labelled
  PASS / FAIL / BLOCKED / NOT RUN with the environment, limits and open
  issues, and links to SPEC/ACCEPTANCE IDs.
- New behaviour gets tests plus at least one A-38 control per goal.
- New dependencies: justify under E-06 (licence, maintenance, need).
- Commits: prefix `run005(<ID>):`, end with the trailer
  `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
  Leave the tree clean.
- When stuck: after two failed approaches, stop and report. If a product
  question appears that the design and D-26 do not answer, stop that item
  and report it; do not decide it. Never weaken tests, thresholds or
  required cases.
- Final reply: a short summary, the final SHA, counts, and anything
  skipped or blocked.
