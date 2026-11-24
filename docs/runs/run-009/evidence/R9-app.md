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

## Round 4

- RV9-13 (major): fixed. `open` is now the single constructor: it applies `set_read_ctl(read_ctl(t))` (hash cache, quit cancel, command interrupt), so restart, reopen after panic, open retry and Enroll all get them. Test `a_service_opened_from_a_saved_enrollment_or_after_a_panic_still_uses_the_hash_cache` restarts from a saved enrollment, asserts the cache file is written, that Check all files finds the cache (counter hook, no more "nothing is set up to check"), then injects a panic and asserts the reopened service rewrites the cache. With the fix removed the test fails (verified). Limit: quit cancel and interrupt are not asserted separately; they are fields of the same ScanCtl built in the same closure.
- Flaky `after_a_panic_the_service_is_reopened_from_disk`: cause was shared test state. `FAULT_RETRY_MS` was one global that each of three tests set and then reset to 0 (a test finishing reset the retry delay of one still waiting, leaving the 300 s default); `PANIC_NEXT_PASS` was a single slot that a parallel test could overwrite before it was consumed; and the test asserted the open counter right after the fault appeared, before the reopen that follows the fault publish. Now retry delay, open count and panic hooks are keyed by state folder, and the test waits for the reopen. 8 consecutive runs of the backend tests pass.
- G2 (major): the Quit/first-close confirmation was a child of the scrolled column. It is now above it in an outer column, always visible. Test `app::log_tests::the_quit_sheet_is_outside_the_scrolled_content` is a source-structure test only (no headless gpui layout); it cannot prove pixel positions. Needs live UIA recheck.
- RV9-10 remainder: Known limits rewritten to match the retry bullets.
- Click latency, measured on the real `_forever_` tree (read-only, isolated state and synthetic shared folder, release build, Export diagnostics click): idle 2.6-3.1 s; right after SyncNow 2.9-3.9 s; during the first upload pass 81-109 s (an upload step runs whole). The hash cache is used (observe 0.13-0.27 s). The remaining ~1.3-2.6 s per click is in service.rs, not the app: `status()` spends ~1.2 s in `recovery_rows` (re-hashes every recovery copy on each status) and diagnostics export observes the world twice (a second observe even when fingerprints are not requested). Not changed here: the command path is R9-decide's (G1). One extra fix kept: the post-apply re-observe in sync_once keeps the hash cache.

## Round 5

- RV9-16: `request_close` now uses pure `close_step`. In setup mode (not enrolled, or the finished-setup screen) it hides straight to the tray, the same as the existing not-enrolled branch, because only the main view draws sheets. The first-close notice is still shown on the first close from the main view. Test `app::close_tests::closing_the_set_up_screen_hides_without_a_sheet_the_setup_pages_cannot_draw` covers the decision table; no GUI run.
- RV9-15: the request is persisted as `hashcache-<tag>.full` in the state folder (written by `request_full`, removed when a complete non-routine pass finishes, read in `HashCache::open`; `prune_stale` also removes stale markers). Only hash_cache.rs changed; service.rs untouched. Test `hash_cache::tests::a_check_all_files_request_survives_a_reopen_until_a_full_pass_completes` (reopen, routine pass, cancelled pass keep it; full pass clears it).
- RV9-10: release notes: literal `\n` removed, stale R9-decide bullet removed, reviewer-checked claim replaced with what was not measured; bullets added for the two fixes above.
