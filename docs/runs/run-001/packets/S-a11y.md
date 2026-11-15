# Work packet S-a11y: screen-reader feasibility with GPUI

Two workers (PLAN §3): `S-a11y` implementation (claude-sonnet-5.5) builds and
measures; `S-a11y-design` (claude-opus-5.5) then writes options for D-12.
Common rules: [S-common.md](S-common.md). Spike dir `spikes/a11y/`.

## Intent

SPEC §7 "accessibility"; R-13, A-26, E-07; feeds deferred decision D-12
(screen-reader limitation policy). What does GPUI expose to Windows UI
Automation (UIA) and Narrator today?

## S-a11y (implementation) work

1. Minimal GPUI window in `spikes/a11y/` (record crate/version) with: a
   heading, a text field, two buttons, a checkbox-like toggle, a list of 10
   items. Also a tiny comparison window using a toolkit with known UIA support
   only if cheap (e.g. a native Win32 message box/dialog) as a baseline.
2. Inspect via UIA from outside (PowerShell
   `System.Windows.Automation` or `UIAutomationClient` COM; or Accessibility
   Insights CLI if already installed — do not install large tools without
   recording): tree depth, control types, names, roles, focus changes on
   Tab, invoke patterns. Save the UIA tree dumps (text) as evidence.
3. Search GPUI/zed source and issue tracker (public) for accessibility
   status (AccessKit integration, plans); cite URLs and commit/date.
4. Narrator with speech: NOT RUN by an agent (cannot hear). Provide an owner
   manual check script (steps, expected announcements).
5. Keyboard-only operation and focus visibility: what works.
6. Commit as ID `S-a11y`.

## S-a11y-design work (after S-a11y merges)

Read S-a11y evidence and public sources. Write
`docs/runs/run-001/evidence/S-a11y-design.md`: options (e.g. contribute/adopt
AccessKit for GPUI, parallel accessible native shell for key flows, documented
limitation per D-12), cost/risk, and a recommendation for the owner. No code.
Commit as ID `S-a11y-design` on branch `run001/S-a11y-design`.

## Owned writes

`spikes/a11y/`, `evidence/S-a11y.md` (+ dir), `evidence/S-a11y-design.md`.
