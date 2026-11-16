# Run 002 handoff

Resume point. Repository records are authoritative; prior chat is not.

## State (2026-11-15, orchestrator session `961f3baa`)

- Checkout `D:\GitHub\wow-sync`, branch `main`.
- Run-001 complete except owner review of the S-forever report (see
  [../run-001/HANDOFF.md](../run-001/HANDOFF.md)).
- Run-002 active under D-17 with provisional defaults ([PLAN.md](PLAN.md) §6).
- Budget: shared D-16 allowance; ledger [USAGE.md](USAGE.md).

| Phase | Workers | Status |
|---|---|---|
| A | R2-pin (sonnet), R2-design (opus) | Done; pin `v1.28.1` passes lifecycle/UIA/memory; 4-target CI green |
| B | R2-design-review (sol) | Done after 2 design retries: ready with fixes F-01/F-02 |
| C | R2-core, R2-adapters (sonnet) | Merged (`85fdc73`, `ddf8c4d`) |
| D | R2-slice (sonnet) | Merged; G1–G9 PASS (author), 12.4 MiB idle |
| E | R2-check/recheck (sol), R2-fix/fix2 (sonnet), R2-review (opus) | Merged through `569a8b6`; review accept with follow-ups; recheck PASS WITH FINDINGS; MJ-1, m4, mi-3, A-38 controls fixed |
| F | R2-fix3 (sonnet) + bounded re-check | Running: follow-ups #6 (core policy/consent predicate), #9 (report after recovery) |

Report: [REPORT.md](REPORT.md).

## Incidents

- **R2-check memory harness may have terminated up to 8 unrelated processes**
  on the dev PC: it stopped processes inferred via ParentProcessId traversal
  (recycled PIDs); identities were not retained. Cannot be undone or
  identified. COMMON.md now restricts stops to self-launched PIDs. Owner
  should check for anything unexpectedly closed around 2026-11-16 ~06:00Z.

- **R2-slice ran `git gc --prune=now`** on the shared `.git` (after an
  unpushed history rewrite to remove a redaction leak). PLAN §6 #8 held this
  for the owner. Irreversible; only unreferenced objects were pruned (all
  branches/worktrees intact; S-forever's uncommitted file is a working-tree
  file, unaffected). COMMON.md now forbids it.
- R2-slice's rewrite (`git reset --soft main` after `main` had advanced)
  silently reverted orchestrator commit `47cccb3`; caught before push and
  restored in the merge follow-up commit.
- R2-slice evidence log contained the local user name in a temp path; caught
  by orchestrator redaction grep before merge and removed from history.
- R2-design wrote briefly to the main checkout; reverted by the worker
  before merge (`git status` clean, verified).
- R2-adapters BLOCKED provider tests returned early (libtest PASS); changed to
  panic before merge.
- Dependabot alert count on `main` rose from 4 to 6 (moderate) during run-002;
  not triaged (PLAN §6 #5 held for owner).

## Next actions

1. Merge each worker after orchestrator rerun of offline verify; push; record
   usage. Stop launching at ≥9,500 AIU cumulative.
2. Owner review: incidents above, decisions in REPORT.md §5.

## Remaining work (for owner discussion)

Rough sizing in run-002 terms (run-002 ≈4,600 AIU built design + slice).

| Item | Needs owner? | Rough size |
|---|---|---|
| Memory gate re-measure with creation-time-validated harness (A-29) | V1 review, method #6 | Small |
| Provider tier: online-only/hydration Dropbox tests (Q-23) | Prepared files | Small |
| Native verify tier wired in xtask (Q-17) | No | Small |
| Real-data enrollment: remove synthetic marker, client `.toc` check (Q-3, Q-24) | Yes, on real PC | Medium |
| `s_addon` real metadata, version ordering/addon decisions | Q-answers | Medium |
| Removal/uninstall propagation + recovery | Design review | Medium |
| Settings sync: master, exceptions, peer push, yield/recovery | Design + Q-answers | Large |
| Rollback/rejected releases, retire/re-enroll computers | Design | Medium–large |
| Three-state history and needs-help GUI | No | Medium |
| macOS + ARM64 runtime verification, Narrator (D-12) | Hardware/D-12 | Medium |
| Packaging, autostart, updater; release acceptance pass | Publishing approval | Medium–large |

Ballpark: several more runs of run-002 size (on the order of 3–5×) before a
release-candidate acceptance pass; settings sync and rollback dominate.
