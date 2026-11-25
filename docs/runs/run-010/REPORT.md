# Run 010 report

Approval: D-32. Result: **`v0.3.3` pre-release**. No critical or major findings
remain open. The independent check passes the gated items on the dispatch
artifact; owner-accepted exceptions are listed below.

## Owner requests and delivery

| Owner request | Delivered |
|---|---|
| "a lot more visibility into what changed" | A per-add-on change history (Activity), with the source computer, versions, bytes and reason (new release, peer choice, rollback, recovery, keep) on every row (R10-history) |
| Sync looked stuck | Live progress: phase, item counts, current add-on, source computer and bytes, in the window and in the tray tooltip (R10-app) |
| Logs that can be collected | Every pass is logged with its trigger, and passes that change something are logged in full. The Dropbox log-copy helper is kept. |
| Idle passes were slow | Quiet scheduled passes skip unchanged work: 291 ms against 927 ms median on a faithful v0.3.2 port, a **3.19×** improvement (R10-speed) |

## Evidence

- **Builds and fixes:**
  - [evidence/R10-history.md](evidence/R10-history.md)
  - [evidence/R10-speed.md](evidence/R10-speed.md)
  - [evidence/R10-app.md](evidence/R10-app.md)
- **Review:** [evidence/R10-review.md](evidence/R10-review.md). No critical or
  major findings are open.
  - RV10-12 is fixed (bytes now cumulative per add-on).
  - RV10-13 is fixed (the recovery source is persisted).
  - RV10-14 is accepted (run-011 OD-07).
  - RV10-15 is deferred (OD-08).
- **Independent check:** [evidence/R10-check.md](evidence/R10-check.md), on the
  dispatch artifacts for `374a1a3` and `e3c1054`.
  - **G1, G3, G4:** PASS after the fix round.
  - **Speed:** PASS (3.19×).
  - **Smoke test:** PASS.
  - **Memory:** 39.14 MiB, against the 100 MiB limit.
- **macOS CI flake** (predates run-010). R10-app found two causes:
  - fine-grained mtimes made a just-written fixture look unstable;
  - a global trigger was shared by parallel tests.

  Both were fixed in test code only. macOS passed three runs in a row before
  the merge (`e7c93f0`).

## Deferred or accepted (owner review list in run-011)

| Item | Disposition |
|---|---|
| G2 | A settings-published record has empty versions. Fixed in run-011 (OD-12). |
| G5 | Native TLS tests flaked once and did not recur. Cause unknown. |
| G6, G7 | Minor: the rollback Activity row omits the source, and a local Keep entry omits versions. Run-011 (OD-15). |
| Tray tooltip | Not observable in the check environment; covered by unit tests (OD-14). |
| Speed | Judged headless (OD-13). |

## Process notes

- The test helper `install_files` waits 25 ms. A time-based wait in a test is a
  residual flake risk.
- Run-010 used about 2,781 AIU (51,604.6 → 54,385.1 when run-011 began). Later
  close-out work overlaps run-011; see [USAGE.md](USAGE.md).
