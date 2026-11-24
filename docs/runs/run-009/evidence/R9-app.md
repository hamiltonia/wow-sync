# R9-app evidence

Branch `run009/R9-app` (from main 64830b1, merged with main b9fdc77). Windows x64.

| Goal | Result | Evidence |
|---|---|---|
| Hash cache | passed | Real `_forever_` AddOns, read-only, isolated state `D:\WoWSyncLab\R9-app\bench`, release build: pass 1 (cold cache) 6926 ms, 34,359 files hashed; pass 2 **150 ms**, all hits; pass 3 159 ms. Cache file 5.3 MB. The owner's ~45 s pass on RYZENFISH was not re-measured there. |
| Instant clicks | unit level only | `a_pending_command_stops_a_scan_only_while_it_is_armed`. Live GUI latency: **not run**. |
| Pause toggle / Group busy | implemented, **not run** in the GUI | Repeat clicks ignored while in flight. |
| Result logging | passed | `every_group_result_gets_a_log_line_without_paths`, including the stale-choice line. |
| Stale choice | passed (unit) | `DecideError::Changed` reads "out of date: the list was refreshed"; the backend refreshes after every group command. |
| Honest headline | passed (unit) | Names the computer that has not shared; 30 min grace (in-memory timer); a decision outranks it. |
| Version skew | passed (unit) | Shown on the computer page. |
| Check all files | implemented, **not run** in the GUI | |

Memory not measured against the 100 MiB gate (**not run**).
Verify: offline PASS, native PASS (after merging main b9fdc77). One intermediate offline run reported FAIL without a captured cause; two reruns passed.
Not done: live GUI latency/memory runs; e2e that other packages apply while one is Unavailable; peer versions in the diagnostics export (core).


## Round 2

| Item | Result | Notes |
|---|---|---|
| Saved enrollment never shows setup | passed (unit/backend) | Backend publishes enrolled for a saved target at start; headline "Checking add-ons…" until the first snapshot. Test restarts a backend on an enrolled state. |
| Panic resilience | passed (backend test with injected panic) | Pass, command and status reads run under catch_unwind; fault shown as headline, commands still answered, retry after 5 min (300 ms in the test) clears it. |
| Second enrollment guard | passed | Enroll when `is_enrolled()` is refused with a message; core already returned AlreadyEnrolled. |
| Hash cache after restart | persistence passed (unit reopen test in fs_scan) | Round 1 real `_forever_` AddOns: 6926 ms cold, 150 ms with cache. No new restart timing on the owner's 3.2 GB tree (**not run**). |
| Diagnostics peer version | passed (unit) | `app_version=` per computer row; non-version text reads `unknown`. |

Not done: live GUI check of the restart on the owner's machine; the core panic itself is R9-decide's.
## Round 3

- RV9-02: enrolled at start only if enrollment loads (nrollment_loads); otherwise LOST_ENROLLMENT message plus setup; open failures retry with 5/10/20 min backoff. Test: lost-enrollment test. Open-fail retry has no automated test.
- RV9-03: checking excludes headline/provider/reconfirm. Test: world::tests::checking_does_not_hide_a_provider_error.
- RV9-05: cache cap 12 MiB (measured 167 B/entry, 34k files ~6 MB); stale caches pruned. 2 tests.
- RV9-07: service reopened after a caught panic. Test: after_a_panic_the_service_is_reopened_from_disk.
- RV9-10: release notes rewritten. RV9-06 accepted (D-31).
- Not run: live GUI, RSS on a real tree, new restart timing.
