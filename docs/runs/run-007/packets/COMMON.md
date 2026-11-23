# Run 007 common packet rules

Authority:
- SPEC.md, ACCEPTANCE.md and ENGINEERING.md.
- OWNER-DECISIONS.md, in particular D-04, D-05, D-08 as amended by D-28,
  D-14, D-21 to D-29. D-29 amends SPEC R-03 and R-05, adds A-47 and A-48, and
  approves [../PLAN.md](../PLAN.md).

Read AGENTS.md first. Each `R7-*` packet plus this file is the full work
packet (E-05). Run-006 [COMMON.md](../../run-006/packets/COMMON.md) applies
unchanged except as below; this file wins where they differ.

- **Worktree:** `D:\GitHub\wow-sync.worktrees\<ID>`, branch `run007/<ID>`,
  created by the orchestrator from `main`.
- **Paths:**
  - evidence goes in `docs/runs/run-007/evidence/<ID>.md`;
  - small raw data goes in `evidence/<ID>/`;
  - labs go under `D:\WoWSyncLab\<ID>\`.
- **Commit prefix:** `run007(<ID>):`.
- The run-006 `url` advisory exception no longer applies; `offline/deny` must
  pass.
- **Real dev-PC data:** only R7-check writes to
  `D:\Games\World of Warcraft\_forever_`, and only after a hashed backup whose
  restore it proves. Implementation packets may read metadata only (folder
  names, `.toc` headers, file sizes).
- **Drive letters:** a `subst` letter you create, you remove, and you record
  both steps in your evidence. Never touch existing letters.
- **Logs and evidence must never contain:**
  - user-profile paths (write `~`);
  - account, realm or character names;
  - add-on or settings contents.
