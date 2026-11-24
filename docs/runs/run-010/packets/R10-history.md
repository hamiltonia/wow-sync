# R10-history: change records and progress events

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goal 1 (data) and goal 2 (events) in [../PLAN.md](../PLAN.md).

**You own:** `crates/wow-sync-runtime` (activity log, apply/report/publish
paths, `service.rs`), and `crates/wow-sync-core` where a change summary must
be computed. **You do not own** `wow-sync-app` (R10-app) or
`wow-sync-adapters` scanning (R10-speed). R10-speed may make small edits to
scan call sites in `service.rs`. Rebase onto them when told.

Read first:
- `log_activity` and `ActivityEntry` in `wow-sync-runtime`;
- the apply path that writes `pending-report.json` and `applied` lines;
- PLAN "Problem".

## Step 1: interface commit (do this first and report the SHA)

Commit only the public types and signatures, with `todo!()` free stubs that
compile and keep tests green:
- **A change record:**
  - when;
  - pass id;
  - kind: applied, installed-new, removed, rolled-back, kept, published,
    settings-applied, settings-published, recovery-restored;
  - add-on display name and folders;
  - source computer: friendly name or this computer;
  - old and new version: TOC `## Version` or a short content id;
  - files added, changed and removed, and the byte delta;
  - reason: newer-release, choice(by computer), settings-master, peer-push,
    removal, rollback, recovery;
  - an optional capped list of changed relative paths.
- **A query API:** recent records, records for one package, grouped by pass.
- **A progress event stream the app can subscribe to:**
  - phase: scanning, deciding, downloading/staging, installing, publishing,
    idle;
  - current add-on, done and total items, done and total bytes;
  - source computer;
  - an incoming summary before apply: add-ons, source and bytes.
Send the orchestrator the SHA, then continue.

## Step 2: implementation

- Compute the summary from the old and new trees (paths, sizes, hashes) the
  apply already has. Do not re-read the disk for it.
- Persist the records in the state dir: bounded (for example the last 2,000
  records or 30 days), crash-safe, and readable next to older `audit.jsonl`
  lines. Keep the old lines readable through the same API.
- Emit progress at no more than about 2 updates per second. The emission must
  not block the pass.
- When a pass starts applying a package that had a pending decision, emit an
  event, so the app can drop the stale decision text.
- Tests:
  - each kind records its source, versions, counts, bytes and reason;
  - records survive a restart;
  - older `audit.jsonl` lines are still read;
  - progress is emitted in order and throttled;
  - a two-computer synthetic test where B applies A's update and B's record
    names A.

Evidence: `docs/runs/run-010/evidence/R10-history.md`.
