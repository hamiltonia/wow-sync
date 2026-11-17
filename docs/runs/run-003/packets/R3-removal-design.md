# R3-removal-design (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `docs/design/removal.md`, which is new;
- new questions appended to `docs/design/questions.md`, starting at Q-40 to
  avoid colliding with R3-ui-design.

Do not edit `domain.md` or `docs/design/README.md`. R3-compat owns
`domain.md` §6, and the orchestrator adds the README rows. State any needed
`domain.md` Q-2 changes in `removal.md` as "supersedes domain.md §6 Q-2
when implemented".

**Inputs:**
- SPEC R-06, R-07, R-09, R-10 and R-11.
- AGENTS.md product rules ("Additions/removals propagate", "Uninstall
  removes active code and settings; recovery must preserve and restore
  both", "Three states", "Rollback rejects…", "Retired computers…").
- ACCEPTANCE: the rows covering removal, uninstall, recovery and three
  states.
- `docs/design/{domain,publication,apply}.md`.
- The current core code.

## Goal

Produce a design that a later run can implement, covering addon removal and
uninstall across the group.

## Must cover

1. **Intent.** Telling an intentional removal on one computer apart from:
   - "never had it";
   - a failed or partial scan;
   - an unreadable folder;
   - a non-ASCII name (Q-14);
   - a provider that is offline or stale.

   Never infer a removal from absence alone. State the required evidence.
2. **Records.** The publication record changes: a tombstone, which states
   carry it, and how long it lives.
3. **Conflicts.** Competing removal versus update on different computers
   (R-06): when it needs a decision, and the safe default.
4. **Apply.** Removal on receivers: WoW-running gate, staging/journal,
   crash idempotency (extending the `apply.md` §4.1 table), the
   multi-folder package removal unit, and dependency effects.
5. **Settings.** Saved settings for removed addons: preserved, and
   restorable together with the code, as code plus settings recovery. The
   settings design does not exist yet; define the interface it must
   satisfy.
6. **History.** Three-state history and recovery for removals (current plus
   two previous), and the visible temporary exceptions.
7. **GUI.** How removals appear and how they are undone. Coordinate by
   naming the needs and leaving screen design to `ui/`.
8. **Rollback and retire.** How removal interacts with rollback/rejection
   and with retired computers.
9. **Tests.** Test obligations for the later implementation: a list of
   cases, mapped to acceptance IDs.

## Done

Evidence: `docs/runs/run-003/evidence/R3-removal-design.md`, with a coverage
table and open questions. `cargo xtask verify --tier offline` must pass.
