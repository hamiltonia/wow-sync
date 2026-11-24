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
