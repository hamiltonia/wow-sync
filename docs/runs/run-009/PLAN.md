# Run 009 plan

Approval: D-31 (2026-11-24). There is no AIU ceiling; usage is tracked in
[USAGE.md](USAGE.md). Target: the `v0.3.2` Windows pre-release.

## Problem

`v0.3.1` is live on RYZENFISH and LENOVOFISH (dev PC). Both are enrolled in
one Dropbox group with `_forever_`, 113 add-on folders each.

- **Misleading headline.** For about 15 h RYZENFISH said "Waiting for Dropbox
  to finish downloading".
  - LENOVOFISH, then on `v0.2.0`, had published a QuestieDB tree that was
    never uploaded. `target.rs` (~440, ~985) reports that as Unavailable.
  - `model.rs` `waiting_sentence` (~216) gives Unavailable a Dropbox wording,
    with no add-on or computer named, ranked above a pending decision.
- **Click lag.** `backend.rs` `run()` (~551) reads commands from
  `recv_timeout` only between `sync_once()` calls. Each pass re-hashes every
  add-on file (about 3.2 GB, ~45 s on RYZENFISH), so Decide, Pause and Export
  diagnostics look ignored. Repeated Pause clicks then toggle several times.
- **Conflicting choices.** At 08:42 and 08:44 local time, the owner's two
  Hide Anything clicks on RYZENFISH wrote two `wowsync.code-choice/1` keeps:
  - each keeps a different copy and excludes the other;
  - both have issuer `ea4972…` and `issued_at_seq` 2, and neither lists the
    other in `known`.
  - `apply_constraints` (`target.rs` ~893) then sets `ChoicesDiffer` with no
    candidates. `decide.rs` `conflict_item` (~321) returns no item. The app
    falls back to `pages.rs` `decision_view` (~1507), which shows only
    "Decide later". There is no test of the `ChoicesDiffer` path.
- **No result logging.** `Cmd::Group` (`backend.rs` ~709) logs nothing about
  what a click did.
- **Owner direction (D-31):** stop re-reading unchanged files on every pass.

## Goals

1. **Conflicting choices can be resolved in the GUI.**
   - A `ChoicesDiffer` (and `RollbackTargetsDiffer`) package offers a keep of
     each copy, plus keep-on-this-computer. A new keep covers every
     conflicting record, so the group converges.
   - Reproduce the owner's exact shape as a test: two keeps of different
     copies, same issuer, same `issued_at_seq`, neither knowing the other.
     Resolving it must clear the decision on both computers.
2. **No second choice from stale evidence.** When a computer has just
   written a choice, its next Decide for that scope must see that record:
   - it is checked against fresh evidence, so it is refused as stale or
     re-offered;
   - or it supersedes the earlier record, because the new record lists it
     in `known`.
   A double click or a second click must never produce two conflicting
   records from one computer.
3. **Clicks respond at once.**
   - User commands are handled within about 1 s while a pass runs: checked
     between steps or packages, or the pass is interrupted and resumed.
   - The UI shows "working…" feedback immediately.
   - Pause is idempotent per click: one click is one toggle, with a
     disabled or pending state until it is applied.
4. **Honest headline, others keep syncing.**
   - An Unavailable package is named ("QuestieDB: waiting for LENOVOFISH to
     share its files"). Only real cloud-only or partly downloaded blobs say
     "Dropbox is downloading".
   - After a reasonable timeout, it escalates to "<computer> hasn't shared
     <add-on>", with a next step.
   - A pending decision is not hidden behind it.
   - A test shows that the other packages are applied while one is
     Unavailable.
5. **Log click results.** Log Decide (accepted, refused and why, the record
   written), Pause/Resume, Export diagnostics (preview and save, ok or
   error) and every other `Cmd::Group` result. No profile paths, names or
   contents.
6. **Hash cache (D-31).**
   - Routine scans re-hash a file only when it is new or its size or
     modification time changed. The cache is stored in the state dir, is
     bounded, and survives a restart.
   - An optional "Check all files" action does a full re-read.
   - Modification time is never ordering evidence.
   - The background memory limit (100 MiB) still holds.
   - Show the speed-up on the 3.2 GB real folder (read-only) and on a
     synthetic one.
7. **Version skew.** Each member publishes its app version. A member running
   a different version is shown on that computer's page, and older-version
   problems are noted.
8. Publish `v0.3.2` with release notes, including the upgrade note: update
   every computer.

## Packets and order

| ID | Role / model | Owns | Depends on |
|---|---|---|---|
| R9-decide | implementation / claude-sonnet-5.5 | goals 1–2: `wow-sync-core` decide/choice/target, runtime decision paths | — |
| R9-app | implementation / claude-sonnet-5.5 | goals 3–7: `wow-sync-app`, `fs_scan` / adapters, member version | — |
| R9-review | review / claude-opus-5.5 | both merges | R9-decide and R9-app merged |
| R9-check | verification / gpt-6.1-sol | `v0.3.2` candidate | both merged, dispatch artifact |

- R9-decide and R9-app run in parallel. Files that both packets need (for
  example `backend.rs` for goal 2) belong to R9-app, and R9-decide reports
  the hook it needs to the orchestrator.
- The orchestrator gates, merges with `--no-ff`, runs both verify tiers,
  pushes and watches CI.

## Gates for `v0.3.2`

- Both verify tiers pass; CI is green, including macOS `verify-offline`.
- The release dispatch build succeeds.
- R9-check reports PASS for:
  - the conflicting-choices reproduction now resolving through the GUI on
    two synthetic computers;
  - no second conflicting record from repeated clicks;
  - Decide, Pause and Export responding within about 1 s during a pass;
  - the headline for an unavailable tree, and other add-ons still applied;
  - log lines;
  - the hash cache: a second pass is at least 5× faster on unchanged data,
    a changed file is re-hashed, and the optional full check works;
  - version skew shown;
  - the artifact smoke test.
- No open critical or major review findings.

## Stop conditions

- A fix needs a product-intent change beyond D-31 → ask the owner.
- Any write to the real Dropbox group, or to the owner's running instances
  → forbidden; ask the owner.
- Any irreversible action on user data → ask the owner.
