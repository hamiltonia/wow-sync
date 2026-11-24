# R10-app: show what changed, live progress, logs, v0.3.3

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goals 1–3 (UI and logs) and 6 in [../PLAN.md](../PLAN.md).

**You own:** `crates/wow-sync-app`, the version bump to 0.3.3,
`docs/release/v0.3.3.md`, and user docs whose behaviour text changes. **You
consume** R10-history's change-record and progress API. The orchestrator
tells you when its interface commit is on `main`; rebase onto it. Until then
you may build against a local stub of the same shape. Do not edit
`wow-sync-runtime` or `wow-sync-adapters`. If you need a change there, ask
the orchestrator.

Read first:
- `backend.rs` `run()` and `refresh()`;
- `model.rs` headline (`waiting_sentence`), the tray text and the Activity
  page;
- PLAN "Problem".

## Work

1. **Activity page.**
   - Group records by pass, newest first: "Today 3:35 PM — 15 add-ons updated
     from RYZENFISH (+1.8 GB)".
   - Each group expands to rows: add-on, old → new version, +a ~c −r files,
     size change, reason ("you chose RYZENFISH's copy", "newer release",
     "removed on RYZENFISH", "settings master", …).
   - A row can expand to the capped list of changed files.
   - Older plain lines still show.
2. **Add-on page.** A "Recent changes" section shows that add-on's records.
3. **Live progress.**
   - The headline and the tray tooltip show the phase and progress from the
     event stream: "Checking files: 12,000 of 36,000", "Installing 15 add-ons
     from RYZENFISH: 1.2 of 1.8 GB (SpokenQuestsAudioShared)", "Publishing
     changes".
   - Show the incoming summary before the apply.
   - Replace a decision headline at once when its package starts applying, or
     when the decision is no longer pending.
   - Update the UI and tray no more than about 2× per second.
   - After the pass, the headline briefly says "Updated 15 add-ons from
     RYZENFISH — see Activity".
4. **Logs** (`log::info!`):
   - pass start and its trigger;
   - each phase start;
   - each package applied or removed, with name, source computer, old → new
     version, file counts, bytes and ms;
   - the pass summary.
   Follow the privacy rule in COMMON. Write lines as events arrive, not only
   at the end of the pass.
5. **Release:** bump to 0.3.3 and write `docs/release/v0.3.3.md`.

Tests:
- Activity grouping and row text for each kind and reason, including a legacy
  line.
- The decision headline is replaced by progress when an apply starts.
- Throttling.
- Log lines contain no absolute paths.
- The add-on page history.

Include GUI screenshots of the Activity page and the progress headline (on
synthetic data) in your evidence.

Evidence: `docs/runs/run-010/evidence/R10-app.md`.
