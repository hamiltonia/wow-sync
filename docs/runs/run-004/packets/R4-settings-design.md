# R4-settings-design (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `docs/design/settings.md` (new);
- `docs/design/rollback.md` (new);
- `docs/design/questions.md` (append only);
- your evidence file.

Read-only inputs:
- SPEC.md (settings, rollback and three-state rules);
- ACCEPTANCE A-11 to A-15, A-18, A-19, A-20 and A-23 to A-26;
- `docs/design/{domain,publication,apply,core-api,removal}.md`;
- `docs/design/ui/{screens,interaction}.md` and
  `ui/wireframes/settings-sync.svg`;
- D-21 and D-23;
- `crates/wow-sync-core` (names and structure only).

## Goals

1. **Settings sync (`settings.md`):**
   - **Master:** one settings master per group or scope.
     - Assigning and reassigning it is always explicit.
     - It is never silently re-elected. Covers retirement (A-45) and an
       offline master.
   - **Per-addon exceptions:** an addon follows the master or is local
     only.
   - **Explicit peer push:** a non-master computer offers its settings to
     the group.
   - **Routine writes from non-master computers** yield to master changes.
     The yielded local copy is kept for recovery.
   - **Competing intentional choices** are never merged. They become a
     decision.
   - **Three states:** current plus two previous per addon settings, with
     visible temporary exceptions.
   - **Saved-variable files:**
     - which files sync, per account and per character, within the
       variant and account/character boundaries;
     - atomic replacement;
     - files are never parsed or executed as Lua. Files are opaque bytes;
       only metadata is used.
   - **Timing:** the WoW-running block applies. Changes from a game exit
     are detected and published.
2. **Rollback (`rollback.md`):**
   - rejecting a bad release (code or settings) for the whole group;
   - a tombstone or rejection record, so offline peers cannot resurrect
     the release (A-19);
   - interaction with removal and re-add, and with three-state history;
   - GUI recovery actions.
3. **Implementation-ready sections:**
   - types and functions mapped to existing modules;
   - on-disk formats and their compatibility;
   - the test list mapped to the A-IDs, including negative cases;
   - which GUI screens are touched.
4. Record new product questions as `Q-NN` with safe defaults. Do not block
   on them.

## Done

- The evidence maps every requirement to a design section and lists the
  open questions.
- `cargo xtask verify --tier offline` passes.
- Committed on your branch.
