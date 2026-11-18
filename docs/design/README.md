# Slice design (run-002)

Status: **design for the run-002 vertical slice**, written by R2-design
(packet [R2-design](../runs/run-002/packets/R2-design.md)), revised in design
retry 1 for the findings in [R2-design-review](../runs/run-002/evidence/R2-design-review.md)
and in retry 2 for [R2-design-recheck](../runs/run-002/evidence/R2-design-recheck.md)
(disposition, including the slice limitations: [R2-design-fix](../runs/run-002/evidence/R2-design-fix.md)). Product meaning lives in [SPEC.md](../../SPEC.md)
and [ACCEPTANCE.md](../../ACCEPTANCE.md); this design only says how the slice
enforces it. Where the design needed a choice outside approved intent, it is
listed in [questions.md](questions.md) with a recommended default and is marked
**(Q-n)** at the point of use. Implementers follow the default until the owner
or orchestrator records otherwise.

## Documents

| File | Contents | Main readers |
|---|---|---|
| [crates.md](crates.md) | Crate/module layout, dependencies, file ownership, merge procedure | all |
| [domain.md](domain.md) | Identities, variant validation, `.toc` rule, addon/package identity, version ordering | R2-core |
| [core-api.md](core-api.md) | Rust type, trait and function sketches for `wow-sync-core` | R2-core, R2-slice |
| [adapters-api.md](adapters-api.md) | Rust API for `wow-sync-adapters` (Windows fs/process, provider folder) | R2-adapters, R2-slice |
| [publication.md](publication.md) | Provider-folder layout, versioned record schemas, hashing, atomic publish, what "published" proves | all |
| [apply.md](apply.md) | Apply algorithm, safety gates, staging/commit/recovery, crash recovery, status and audit | R2-core, R2-adapters |
| [tests.md](tests.md) | Required behavioral test names per packet, fixtures, end-to-end and provider tests | all, R2-check |
| [questions.md](questions.md) | Owner and design questions with recommended defaults | owner, orchestrator |
| [contracts.md](contracts.md) | Shared contract (run-004, R4-design-fix): record identity and hashing, known-coverage, authority/retirement barrier, `/2` journals and recovery points, settings identities, target composition and precedence, code-choice decisions, technical caps vs product gates, phase C ownership and build order. The four designs below consume only this | run-004 implementers |
| [removal.md](removal.md) | Removal/uninstall design (run-004, D-21/D-23): any removal wins, conflicting removals merge, removal holds (no decisions), explicit add-back, recovery | run-004 implementers |
| [settings.md](settings.md) | Settings sync design (run-004): settings master, per-addon exceptions, explicit push, authority epoch, non-master yield with recovery, three states, GC roots | run-004 implementers |
| [rollback.md](rollback.md) | Rollback design (run-004): exact-content group rejection and lift, recovered-version constraint, concurrency, offline peers cannot reinstall a rejected release, code-only fallback | run-004 implementers |
| [ops.md](ops.md) | Operations design (run-004): code-choice decision actions (UQ-1/UQ-2), retire, notifications, update notice, diagnostics export | run-004 implementers |
| [dropbox-api.md](dropbox-api.md) | Research spike (run-003, D-21): desktop-client folder vs Dropbox HTTP API vs hybrid; comparison, risks, recommendation (keep the folder; optional read-only hybrid later), owner questions DA-1..DA-4, trial plan needing approval | owner, orchestrator |
| [ui/README.md](ui/README.md) | GUI design (run-003): information architecture, screens, style, interaction, wireframes, prototype brief | R3-proto, later UI build |

[../ARCHITECTURE.md](../ARCHITECTURE.md) is the short overview and invariant
map; these files are the implementable detail.

## Slice scope (PLAN §1)

Windows x64, synthetic installs, one variant group, addon **code** only:

1. **Enroll** a computer: generate a computer identity, choose a shared
   (provider) location, choose a WoW install root and variant folder, validate
   the variant (R-02; guided confirmation for unfamiliar clients, A-46),
   create or join a group for that variant (R-03), write the member record.
2. **Inventory** the variant's `Interface\AddOns`: per-folder file hashes,
   one `.toc` selected per folder, packages grouped, versions parsed (R-05).
3. **Publish** the computer's observed addon-code state to the provider folder:
   content-addressed blobs and folder trees first, the state record last, each
   via temp + rename (R-03, R-12).
4. **Compute a group target** from all enrolled members' latest states
   (deterministic, newest demonstrable version per package, ambiguity becomes
   an addon-scoped decision, R-05/R-06).
5. **Apply** on another computer: stage, verify, revalidate the destination,
   check that WoW is not running before **every** live rename, rename into
   place, keep a pinned recovery point, verify, then finalize, report
   (R-11, R-10 partial). A launch mid-commit stops all game-file changes until
   WoW exits.
6. **Status and provenance**: published / available / applied / confirmed kept
   distinct; per-package explanation; local audit log (R-12).

## Explicitly out of the slice

| Out | Why / where it goes later |
|---|---|
| Settings (WTF) of any kind, settings master, pushes, pauses (R-07..R-09) | PLAN §2. The slice never reads or writes `WTF` |
| Deliberate removal / uninstall propagation (R-04 removal half) | PLAN §2. The slice has **no removal authority at all**: nothing is ever deleted from `AddOns` except by renaming a replaced folder into a recovery point |
| Rollback, rejection, restore from recovery through the GUI (R-10) | PLAN §2. Recovery points are created and retained so a later run can restore them |
| Conflict-resolution UI (choosing a candidate) | Ambiguity is shown read-only as "needs a decision" |
| Retirement, re-enrollment, master reassignment | Record fields are reserved (see [publication.md](publication.md)) |
| Blob/tree garbage collection in the provider folder | Gap; see Q-7 and R-14 "no unbounded growth" |
| Notifications, telemetry, packaging, app updates, CLI | PLAN §2 |
| macOS runtime; Windows ARM64 runtime | No hardware (D-07). Code must still **build** for all four targets; non-Windows process observation reports `Unknown`, which blocks writes |
| Real WoW data in tests | Product tests use synthetic installs under a temp directory (COMMON.md) |

## Traceability

| Requirement | Slice obligation | Where |
|---|---|---|
| R-02 | Variant identity from metadata, refuse on disagreement/unknown, guided confirmation for unfamiliar product codes, no cross-variant data | [domain.md](domain.md) §2, [publication.md](publication.md) §1 |
| R-03 | Enrollment, shared location, group create/join, consent text; absence is not uninstall | [domain.md](domain.md) §1, [apply.md](apply.md) §1 |
| R-04 | Additions propagate; absence never deletes | [domain.md](domain.md) §5, [apply.md](apply.md) §2 |
| R-05 | Newest demonstrable version per package; same-scheme strict ordering; no timestamps; ambiguity is addon-scoped | [domain.md](domain.md) §4–§6 |
| R-06 | Deterministic target; independent packages progress | [core-api.md](core-api.md) `target` |
| R-11 | WoW running blocks writes; re-check before each commit; staging; recovery; crash recovery; no Lua execution; path safety | [apply.md](apply.md) |
| R-12 | Published ≠ available ≠ applied ≠ confirmed; newest-state admission; explanations; audit | [apply.md](apply.md) §6, [publication.md](publication.md) §4, §6 |
| E-08, A-43 | Platform-free core is the single policy authority, testable without GUI/provider | [crates.md](crates.md) |
