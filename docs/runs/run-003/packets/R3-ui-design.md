# R3-ui-design (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first.

**Inputs:**
- SPEC R-02, R-03, R-06, R-07, R-10, R-12, R-13 and R-14.
- ACCEPTANCE A-01, A-24, A-25, A-26, A-42, A-45 and A-46.
- OWNER-DECISIONS D-18 and D-19.
- `docs/design/*`.
- `crates/wow-sync-app/src/ui.rs`, the current slice shell.
- `crates/wow-sync-core/src/status.rs`.
- Run-002 R2-pin evidence (GPUI/AccessKit capabilities).

**Owned paths:**
- `docs/design/ui/**`, which is new;
- questions you raise, appended to `docs/design/questions.md`, starting at
  Q-26.

The orchestrator adds the `docs/design/README.md` row.

## Goal

Design the real UI so that a prototype can be built from your files alone,
without guessing. The owner's direction (D-19) is:
- a **roomier dashboard**, in the spirit of **Syncthing**;
- the theme **follows the system light/dark setting**;
- tray presence with honest pause and quit behaviour.

## Deliverables

1. **`ui/README.md`, the information architecture.** Cover:
   - the window layout: a sidebar or sections, a headline, and panes;
   - the navigation map;
   - the tray menu;
   - window close versus quit versus pause, per R-13;
   - first-run onboarding.
2. **`ui/screens.md`, one section per screen.** For each screen give its
   purpose, the data it shows (mapped to `core::status` or runtime fields,
   with any missing fields named as gaps), its actions, and every state:
   empty, working, waiting, success, needs-help, error, offline/stale peer,
   and WoW running. Screens are at least:
   - dashboard / this computer;
   - other computers, distinguishing last-confirmed from unknown-present
     (A-24);
   - addons list and addon detail ("why this version", with the evidence and
     the next action, A-25);
   - setup/enrollment and reconfirm (guided identity and scope, A-46);
   - needs-help / decision;
   - activity and history;
   - settings-sync placeholders (master, exceptions, push), marked future;
   - app settings (startup, pause, theme, notifications);
   - about / updates (A-28).
3. **`ui/style.md`, the visual system.**
   - Light and dark colour tokens with contrast ≥ 4.5:1 for text. List the
     ratios.
   - Spacing scale, type scale, status colours and icons, focus ring.
   - High-DPI rules.
4. **`ui/interaction.md`, interaction rules.**
   - Keyboard map and focus order for each screen.
   - Accessible names and roles for AccessKit. Narrator is a nice-to-have
     (D-18).
   - Notification grouping and deferral while gaming (R-13).
5. **Wireframes.** One SVG per main screen in `ui/wireframes/`,
   hand-authored, simple and text-legible, plus a light and a dark variant of
   the dashboard.
6. **Prototype brief.** `ui/prototype.md` lists exactly what R3-proto must
   build: screens, the fake data scenarios that show each state, and the
   acceptance checklist the owner will use.

## Constraints

- **GPUI:** only what GPUI `v1.28.1` can plausibly do. Flag risky widgets
  (virtualized lists, menus) as open questions rather than assume them.
- **Logic stays in core:** no new product behaviour or policy. If a screen
  needs data that core does not expose, list it as a gap with a suggested
  core field.
- **Wording:** plain-English user text. Show no internal IDs unless they are
  in an expandable "details" area.

## Done

Commit your files with evidence `docs/runs/run-003/evidence/R3-ui-design.md`.
The evidence covers:
- the files written;
- a coverage table of A-IDs against screens;
- the gaps and questions.

Docs-only changes still need `cargo xtask verify --tier offline` to pass,
for the markdown-link check.
