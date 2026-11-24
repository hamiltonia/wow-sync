# Run 009 report

Approval: D-31. Result: **`v0.3.2` published**. No critical or major findings
remain open, and every independent check passes on the final artifact.

## Problems and root causes

| Owner report | Root cause | Fix |
|---|---|---|
| Hide Anything showed only "Decide later" | Two clicks wrote competing keeps. Each copy counted as excluded by the rival keep, so no options were left. | `kept_by_rival` keeps both copies on offer (R9-decide) |
| A second click re-conflicted | Decide judged against a stale offer list | Decide reloads peers' records and refuses stale or answered items with `Changed` (RV9-01, RV9-11, RV9-12) |
| RYZENFISH "lost all settings" after a reboot | `NewVersion::kept_copy` panicked (index out of bounds) whenever the kept release was absent. The UI showed setup until a first pass succeeded, and the pass always panicked. Settings were never lost. | `kept_copy` returns `Option`; indexing hardened; a saved enrolment shows the main view at start; passes are guarded by `catch_unwind`, and the service reopens from disk after a panic (RV9-07) |
| Sync stuck for hours / click lag | Commands queued behind full re-hash passes | Hash cache (size+mtime, D-31); commands interrupt a pass; command reads are scoped and cached; publishing yields between objects |

## Delivered

- Competing keeps and Resume-vs-Keep both raise an answerable decision (RV9-04).
- A stale click is refused and writes nothing.
- Lost or unreadable `enrollment.json` routes to setup. A failed service open
  retries after 5, 10 and 20 minutes and says so (RV9-02).
- Every service open applies the scan options (cache, Quit cancel, command
  interrupt) (RV9-13 / G3).
- The hash cache is capped at 12 MiB, and stale caches are pruned (RV9-05).
- A "Check all files" request is saved until a full pass completes (RV9-15).
- The Quit confirmation is always on screen (G2). Closing on a setup screen
  hides to the tray (RV9-16, predates run-009).
- Honest "waiting for <Computer> to finish sharing <Addon>" headline, which
  escalates after 30 minutes. Version skew is shown on the computer page;
  group command results are logged; diagnostics include `app_version`.

## Evidence

- **Fixes:** [evidence/R9-decide.md](evidence/R9-decide.md) (7 rounds) and
  [evidence/R9-app.md](evidence/R9-app.md) (6 rounds). Every new guard has a
  test shown to fail with the guard removed.
- **Review:** [evidence/R9-review.md](evidence/R9-review.md), with 3 rechecks.
  No critical or major findings are open. RV9-06 is accepted under D-31.
  RV9-08, 09, 17, 18 and 19 are notes.
- **Independent check:** [evidence/R9-check.md](evidence/R9-check.md), run on
  packaged artifacts `7e4a43d`, `7101088`, `f95def1` and `104d238`. Results on
  the final artifact:
  - Decide during a pass over 3.35 GB of invalidated BigLab files: 129 ms
    (was 2,209–2,585 ms);
  - clicks during the first upload: Decide 81 ms, Preview 163 ms;
  - restart with a saved enrolment: PASS.

  Results on `f95def1`:
  - idle and post-sync clicks: about 100–150 ms;
  - the Quit confirmation is on screen;
  - closing on the setup screen, and Check all files across a restart: PASS;
  - BigLab memory: 37.37 MiB, against the 100 MiB limit;
  - genuine 30-minute escalation: PASS.
- **Builds:** both verify tiers pass on every merge. The final CI run is green
  on all OSes.

## Remaining risks

- Not confirmed on RYZENFISH yet. The owner's two existing competing Hide
  Anything keeps must be answered once on `v0.3.2`.
- Panic resilience (check 12) is shown by unit tests and code only.
- The open-failure retry has no automated test; it was code-reviewed only.
- A tampered recovery copy with unchanged size and mtime can appear in the
  recovery list, but restore re-hashes and refuses it (D-31).
- **Process incident:** an R9-decide script wrote an uncommitted `e2e.rs` edit
  into the main checkout through a relative .NET path. It was saved, found to
  duplicate the branch, and discarded. Workers now use full paths.
- **Process incident:** macOS CI failed on new tests that used a symlinked temp
  dir. The tests were fixed and the product code was unchanged.
