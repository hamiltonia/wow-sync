# R4-settings-design evidence

- Packet: [R4-settings-design](../packets/R4-settings-design.md) with
  [COMMON](../packets/COMMON.md).
- Role and model: Design worker, Opus 5.5 (as assigned by the orchestrator;
  runtime selection not independently verifiable from inside the worker).
- Worktree: `D:\GitHub\wow-sync.worktrees\R4-settings-design`, branch
  `run004/R4-settings-design` from `main` (`573615b`).
- Environment: Windows 11 x64 dev PC.

## What was built

Design only. No code, tests or required cases changed.

| File | Change |
|---|---|
| [docs/design/settings.md](../../../design/settings.md) | New. Settings sync: scope, opaque bytes, identities, capture at session boundaries, formats, publish/receive/apply, competing choices, master and exceptions, pause, three states, removal interface, status/GUI, implementation map, tests T-S01..T-S32 |
| [docs/design/rollback.md](../../../design/rollback.md) | New. Rejection and lift records, target precedence, offline peers, paired/code-only settings, settings rejection, removal/re-add interaction, three states, GUI actions, implementation map, tests T-R01..T-R18 |
| [docs/design/questions.md](../../../design/questions.md) | Appended section "Settings and rollback design questions" with Q-54..Q-71 |

Not edited: `removal.md` (being rewritten by R4-removal-design; referenced by
concept only), `ops.md` (being written by R4-ops-design; referenced as a code
span so no broken link exists on this branch), `docs/design/README.md` (the
orchestrator adds index rows).

## Requirement-to-section map

| Packet requirement | settings.md / rollback.md | SPEC / ACCEPTANCE |
|---|---|---|
| One settings master per group; assign/reassign explicit; never re-elected; retirement and offline master | settings §5.1–§5.3 | R-07, R-08; A-11, A-45 |
| Per-addon exceptions (follow master / other computer / local only) | settings §3.2 policy, §5.1, §5.2 | R-07; A-11 |
| Explicit peer push | settings §4.1, §4.2, §4.5 | R-08; A-13 |
| Routine non-master writes yield to master, yielded copy recoverable | settings §4.1, §4.2 (yield), §7 | R-08; A-12 |
| Competing intentional choices never merged; decision | settings §4.4, §4.5; Decision page §9.3 | R-09; A-14 |
| Three states plus visible exceptions | settings §7; rollback §7 | R-10; A-16, A-17 |
| Saved-variable files per account/character within variant boundaries | settings §1.1, §1.2, §1.4 | R-07, R-10; A-23 |
| Atomic replacement | settings §4.3 (journal, stage, `G` before every rename, slot recovery) | R-11; A-21, A-22 |
| Never parsed or executed; opaque bytes, metadata only | settings §1.3, §1.1 (`.toc` metadata only) | R-07, R-15; A-23 |
| WoW-running block; changes from a game exit detected and published | settings §2.1, §2.2, §4.3 step 1 | R-07, R-11; A-11, A-21 |
| Pause per addon / pause all; resume explains | settings §6 | R-08; A-15 |
| Rejecting a bad release (code or settings) group-wide | rollback §1, §2.1, §5 | R-10; A-18 |
| Tombstone/rejection so offline peers cannot resurrect | rollback §2.3, §3, §3.2 | R-10; A-18, A-20 |
| Code-only fallback with distinct warning | rollback §4 | R-10; A-19 |
| Interaction with removal and re-add | rollback §6; settings §8 (removal's settings interface) | R-04, R-10; A-09, A-18 |
| Interaction with three-state history | rollback §7; settings §7 | R-10; A-16, A-17 |
| GUI recovery actions | rollback §8; settings §9.3 | R-12, R-13; A-25, A-26 |
| Types/functions mapped to modules | settings §10; rollback §9 | — |
| On-disk formats and compatibility | settings §3 (incl. §3.4); rollback §2 | Q-13; A-20 |
| Test list with negative cases | settings §11 (T-S01..T-S32 + A-38 controls); rollback §10 (T-R01..T-R18 + A-38 controls) | A-11..A-26, A-45, A-38 |
| New product questions with safe defaults | questions.md Q-54..Q-71 | — |

Note: the packet attributes "offline peers cannot resurrect" to A-19. In
ACCEPTANCE.md that is A-18; A-19 is the code-only fallback. Both are
covered (rollback §3.2 and §4).

## Commands run

| Command | Environment | Result |
|---|---|---|
| `cargo xtask verify --tier offline` (PATH prefixed with `%USERPROFILE%\.cargo\bin`), worktree root, after all files were written | Windows 11 x64 dev PC | **PASS**: fmt, clippy, test, required-cases, deny, markdown-links all PASS; `overall: PASS (allow-blocked: false) -> exit 0` (new files staged so the link check covered them) |

## Limits

- Not reviewed. An independent design review is still required.
- The WTF layout (§1.1 of settings.md) is the documented WoW layout and is
  not verified on any real client; real-client checks are deferred by D-23
  (Q-55). Settings sync is synthetic-only until verified.
- The retirement validity rules are referenced from the parallel `ops.md`
  and the tombstone/ledger rules from the parallel removal rewrite. The
  integration review must check the terms and cycle steps still match after
  both merge.
- Compatibility claim settings §3.4 (old readers skip unknown authority
  schemas) must be verified in code (T-S30); if the shipped reader refuses
  unknown schemas, a group-level capability gate (as in rollback §2.3) is
  needed before first use.
- No tests were added, so no A-38 negative controls were run. macOS CI: NOT
  RUN (no push).

## Open questions raised

In [questions.md](../../../design/questions.md): Q-54 (ownership), Q-55
(excluded files, synthetic-only), Q-56 (identity matching), Q-57 (start and
per-computer opt-in), Q-58 (complete/suspect capture), Q-59 (missed
sessions), Q-60 (concurrent policy changes), Q-61 (push with master
offline), Q-62 (new master does not publish old differences), Q-63 (where
copies are stored), Q-64 (64 MiB file limit, numeric gate), Q-65 (exact-hash
rejection, look-alikes), Q-66 (lift), Q-67 (paired checkpoint selection),
Q-68 (recovered-release constraint), Q-69 (settings rejection), Q-70
(pause-all scope), Q-71 (mixed app versions). Numbers may be renumbered by
the orchestrator if another run-004 design worker also appended Q-54+.
