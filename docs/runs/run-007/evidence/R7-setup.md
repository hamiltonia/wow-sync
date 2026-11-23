# Evidence: R7-setup (run-007)

Branch `run007/R7-setup` (from main 8b0dee0). Links: PLAN goals 1, 4, 5; ACCEPTANCE A-47.

## Built
- Discovery: all fixed drives (4 locations each) plus Battle.net `product.db` paths
  (bounded 4 MiB read, bytes treated as untrusted, no code executed), case-insensitive dedupe.
- Folder picker (GPUI `prompt_for_paths`, no new dependency) in step 2 (Dropbox) and step 3
  (game); a refused folder says why and shows its path.
- Diagnostic log, on by default: `%LOCALAPPDATA%\WoWSync\state\logs\wow-sync.log` (or `<--state>\logs`),
  `WOWSYNC_LOG` overrides. Line format `YYYY-MM-DDTHH:MM:SSZ pid=N LEVEL message`; UTC;
  rotates at 1 MiB to `wow-sync.log.1`; `%USERPROFILE%` written as `~`.
  Logs: start/version, drives searched, product.db result, each probe, setup steps, picker events,
  inspect/enroll/sync errors, blocks and provider errors.
- "Open data folder" / "Open shared folder" buttons now act.
- Version 0.3.0, `docs/release/v0.3.0.md`, README status.

## Results (Windows, this machine)
| Check | Result |
|---|---|
| `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) |
| `cargo xtask verify --tier native` | PASS |
| `cargo test --workspace` | PASS (all suites ok, 0 failed) |
| New required cases `R7-setup:*` | PASS (13 tests) |
| Dev-mode window, step 3, log written | PASS: `D:\WoWSyncLab\R7-setup\shots\step3-dev-window.png` |
| Step 3 empty state screenshot | NOT RUN: `--dev` always supplies an install row; covered by unit test `the_empty_state_says_what_to_do_in_plain_words` |
| Screenshot after picking | BLOCKED: native OS dialog not automated; covered by `setup::tests::a_chosen_*` |
| Real-machine drive/product.db discovery | NOT RUN (would read the owner's real WoW location); unit tests use synthetic bytes |

Logs: `D:\WoWSyncLab\R7-setup-offline.log`, `D:\WoWSyncLab\R7-setup-native.log`.

Sample log (lab):
```
2026-11-23T17:03:52Z pid=20352 INFO WoW Sync 0.3.0 starting, dev=true, state D:\WoWSyncLab\R7-setup\state
2026-11-23T17:03:52Z pid=20352 INFO sync thread started, dev=true
2026-11-23T17:03:53Z pid=20352 INFO appearance changed: system=Dark
```

## Open
- TODO: release notes "Add-ons that cannot be recognised" text awaits R7-sync's text from the orchestrator.
- The log does not yet list R7-sync's unrecognised add-ons separately (format unknown to this worker).

## Review follow-up (RV7-05/06/07/09)
- RV7-05: a picked Dropbox folder that is not, or inside, a path from Dropbox's info.json (or the default folder) is accepted with the warning in setup::NOT_DROPBOX_TEXT; never blocked. Tests: discover::tests::a_picked_folder_is_dropbox_only_when_it_is_or_is_inside_a_reported_one, setup::tests::a_picked_folder_not_reported_by_dropbox_is_a_warning_not_a_refusal.
- RV7-06: the all-drives search runs before the window only for a computer with no saved enrollment. An enrolled computer starts the service first; backend::detect_later runs the search on its own thread after the first status, and only if not enrolled. Design argument, no timing measured (NOT RUN: no timing test).
- RV7-07: pick validation (install_from_choice, info.json read) runs on the background executor; the page shows 'Checking...' meanwhile. No GUI test (BLOCKED: no GPUI render tests); logic covered by the setup tests.
- RV7-09: the log now records each add-on whose status text says 'not synced' or 'Unrecognised' once per change; the notes describe lineage only as 'the copy that changed since the last shared copy wins; otherwise you choose once'.
- Both verify tiers PASS.
