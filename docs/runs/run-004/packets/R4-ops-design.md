# R4-ops-design (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `docs/design/ops.md` (new);
- `docs/design/ui/questions.md` (mark UQ-1 and UQ-2 as decided by D-23);
- `docs/design/questions.md` (append only);
- your evidence file.

Read-only inputs:
- SPEC.md;
- ACCEPTANCE A-26, A-27, A-28, A-33 and A-45;
- `docs/design/{domain,core-api,publication}.md`;
- `docs/design/ui/*`;
- D-21, D-23 and D-24;
- `crates/wow-sync-core/src/view*` and `crates/wow-sync-app` (names and
  structure only).

## Goals

1. **Decision resolution (A-26):**
   - the actions per decision kind, and their preconditions;
   - what core records;
   - how peers see each action;
   - undo and recovery.
   - **UQ-2:** core exposes the typed scope ("only this addon" versus
     "the whole group is held").
   - **UQ-1:** a pending guided reconfirmation promotes the headline to
     "Review needed". Pause still never hides a `NeedsHelp` or `Error`
     headline.
2. **Retire a computer (A-45):**
   - a GUI flow that keeps history;
   - currentness no longer waits for the retired computer;
   - stale or offline publications are ignored;
   - re-enrollment is explicit;
   - retiring the settings master shows the reassignment guidance and
     never auto-switches.
   - Coordinate with `settings.md` terms: its author works in parallel on
     R4-settings-design. Use the names "settings master" and "reassign".
3. **Notifications (A-27):**
   - which events notify;
   - deduplication and rate limits;
   - quiet while in game, if possible;
   - the setting to turn them off.
   - Choose a crate candidate (P4-2) and justify it.
4. **Update notice (A-28, D-24 P4-1):**
   - an anonymous HTTPS GET of the latest release of
     `hamiltonia/wow-sync`, at most once a day;
   - it fails silently and offline;
   - no identifiers;
   - an opt-out;
   - a TLS approach that passes `cargo deny` on all four targets.
5. **Diagnostics export (A-33):**
   - the user requests it;
   - a preview is shown;
   - the redaction rules (paths, names, account, realm and character,
     SIDs);
   - a size bound;
   - the format;
   - what is never included.
6. **Implementation-ready sections:**
   - types and functions mapped to modules;
   - the test list mapped to the A-IDs, including negative cases (planted
     private strings must not appear in the export);
   - GUI screens.

## Done

- The evidence maps every requirement to a section and lists the open
  questions.
- `cargo xtask verify --tier offline` passes.
- Committed on your branch.
