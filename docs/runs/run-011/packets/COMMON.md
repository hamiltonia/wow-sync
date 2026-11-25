# Run 011 common packet rules

Authority: SPEC.md, ACCEPTANCE.md, ENGINEERING.md, and OWNER-DECISIONS.md
through D-33, which starts [../PLAN.md](../PLAN.md). The settings design is
[design/settings.md](../../../design/settings.md); follow it unless a
section is plainly wrong, and then report it instead of reinterpreting it.

Read AGENTS.md first. Each `R11-*` packet plus this file is the full work
packet. Run-010 [COMMON.md](../../run-010/packets/COMMON.md) applies
unchanged except as below. This file wins where they differ.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\<ID>`, on branch `run011/<ID>`.
  Always use absolute paths into your worktree for every read, edit and
  command. Never write in `D:\GitHub\wow-sync` itself; run-010 lost a
  verify to a stray edit there. Run `git -C D:\GitHub\wow-sync status
  --short` before you reply; it must be empty of your changes.
- **Paths:**
  - evidence: `docs/runs/run-011/evidence/<ID>.md`;
  - labs: `D:\WoWSyncLab\<ID>\`.
- **Commit prefix:** `run011(<ID>):`.
- **Realistic test data.** You may copy, read-only, from the dev PC's
  `D:\Games\World of Warcraft\_forever_\Interface\AddOns` and
  `D:\Games\World of Warcraft\_forever_\WTF` into your lab (D-14 test data).
  Rules:
  - copy only `Interface\AddOns` and `WTF\Account\**\SavedVariables\*.lua`;
    never `Config.wtf`, bindings, macros, chat cache or `*.bak`;
  - never write, rename, delete or lock anything in the real `_forever_`;
  - lab copies stay under `D:\WoWSyncLab\` and are never committed;
  - do not execute or `require` any Lua; treat contents as untrusted data;
  - evidence, tests, logs and commits never contain real account,
    realm or character names, file contents, or absolute user paths.
    Refer to them as "account A", "character A1" and so on. Committed
    tests use synthetic data only.
  - To simulate a second computer, copy the lab data, then rename or
    remove character folders and edit settings files with synthetic
    content as the scenario needs.
- **WoW:** do not start, stop or inject into WoW. If a real WoW process is
  running, settings apply is blocked by design; use the existing test
  hooks for "WoW running", or wait. Report it.
- **The real group and the owner's instance** are off limits as in run-010.
  Nothing enables settings in the real group.
- **Decisions.** When the design leaves a choice open, pick the option that
  is safest for user data and least surprising, implement it, and list it
  under "Decisions made" in your evidence with the alternatives. The
  orchestrator copies them to DECISIONS-FOR-REVIEW.
- **Tests:** synthetic data only; no Windows-only paths (macOS CI). Each new
  behaviour or fix gets a test that fails without it; say in evidence that
  you checked. Add a `tests/required-cases.toml` section; never remove
  existing cases.
- **Version:** R11-app bumps to 0.4.0 and writes `docs/release/v0.4.0.md`.
- **Finish:** run `cargo xtask verify --tier offline` and `--tier native`
  in your worktree, commit your evidence, and reply with the SHA, the
  results and any open issues.
