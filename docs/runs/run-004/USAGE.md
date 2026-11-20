# Run 004 usage

Ceiling: 20,000 AIU from run-004 start (D-23/D-24). Stop new work at
≥19,000.

Baseline: the cumulative orchestrator-session total at run-004 start is
**17,413 AIU**. Run-004 usage = cumulative − 17,413.

| # | Packet | Role / model | Agent | AIU | Result |
|---|---|---|---|---|---|
| 1 | R4-removal-design | design / claude-opus-5.5 | `17391135` | 346.6 | Merged; R3D-13 closure claimed (review pending); Q-54..58 |
| 2 | R4-settings-design | design / claude-opus-5.5 | `4c649513` | 397.1 | Merged; settings.md + rollback.md; Q renumbered to Q-67..84 |
| 3 | R4-ops-design | design / claude-opus-5.5 | `07b3d707` | 394.3 | Merged; ops.md; Q renumbered to Q-59..66 |
| 4 | R4-design-review (+ recheck, recheck2) | verification / gpt-6.1-sol | `07fd2769` | 511.0 | Review FAIL; recheck FAIL; recheck2 FAIL `e1fe458`; recheck3 FAIL (R4D-28) `6dfd530`; recheck4 **PASS** (minor R4D-32) `99e4877` |
| 5 | R4-design-fix (+ retry 1 fix2, retry 2 fix3) | design / claude-opus-5.5 | `961ccd8a` | 2,835.7 | Merged `c939b30`; fix2 `3751253`; fix3 `e6a6790`; bounded R4D-28 amendment fix4 `832492d` (owner "continue" at the barrier, taken as option 1) |
| 6 | R4-core-contracts | implementation / claude-sonnet-5.5 | `f645c783` | 930.5 | Merged `b8707ae` (3 passes; amendment 1; I-1 slip noted); both verify tiers PASS; T-C11 native NOT RUN (R4D-22) |
| 7 | R4-core-removal | implementation / claude-sonnet-5.5 | `e741ccb0` | 380.5 | Merged (`4162541`); lost uncommitted removal.rs via `git checkout --` during a mutation control, re-implemented as a same-task follow-up; 6 A-38 controls fail as expected; both tiers PASS; service.rs wiring + target/explain/status variants DEFERRED |
| 8 | R4-core-settings | implementation / claude-sonnet-5.5 | `50cfd494` | 406.1 | Merged (`5ba828a`); both tiers PASS; only 3 of 15 A-38 controls run; ledger.rs rejects mixed-kind parents (contradicts settings.md §3.3, needs fix); hooks/rollback_flow wiring thin; Q-77 keeps real settings sync disabled |
| 9 | R4-core-ops | implementation / claude-sonnet-5.5 | `8aee2e45` | 281.8 | Merged `c706ab7`; both tiers PASS; 5 A-38 controls fail as expected; hooks not wired into target/service/control (integration gap), kept queue not persisted (runtime lacks serde_json), 3 item builders missing |
| 10 | R4-integrate | implementation / claude-sonnet-5.5 | `47b27b1b` | 2,588.8 (passes 1–12) | Pass 1 stopped with no changes: the runtime has no authority plane or §9.1 control surface, and code-state lacks `authority_index` (unmet C1). Scope widened by the orchestrator (records/plan, ledger serde, minimal proto/app match arms) and staged A–D. Pass 2: stage A + code-state `authority_index` merged `735fa36` (A `affc3a4`, B-part-1 `7abfb7d`); stage B sent as a follow-up. Pass 3–4: stage B merged `7a0cdb1` (B1 `d1fe2da`, B2 `0840f59`, B3a `ff656ff`, B3b `6b17fff`; orchestrator-approved `compose` constraints parameter; 3 A-38 wiring controls; `decision.rs` arms and `e2e.rs` tests accepted as in-intent); stage C sent. Pass 5: Decide/Retire/AllowAgain merged `8d3eb65` (`3e8572d`); C0 settings apply and other commands deferred; narrower pass 6 sent (producer, keep/resume, re-enroll, removal restore, settings commands). Pass 6 merged `93724ed` (`77d65f4`..`675a4c8`); pass 7 sent (C0 settings apply, rollback, settings commands, crash fix, TooLarge). Pass 7 merged `7b60195` (settings apply `b7dce06` with test-only cap, rollback `aec6ce4`); pass 8 sent (R0 staging gap with `effects.rs` approved, crash fix, settings commands, TooLarge with `kinds.rs` approved). Pass 8 merged `3282c15` (R0 local staging `fb6c8b4`, removal replay `afa7f59`); Pass 9 merged `e7beaeb` (`d843039`, `636a31f`, `16064b8`); Pass 10 merged `979b8de` (exports `13696d2`) and `fba5b63` (snapshot `10413c5`, notify/update/diag wiring `ac9cf40` incl. a `notices.rs` delivery fix, handover `5412450`); toggles live in a new `AppToggles` file (accepted; avoids editing the app crate) Pass 11 merged `f2597cf` (macOS notification identity `com.wowsync.app`). Pass 12 item 1 merged `2c94ceb` (snapshot peers `4d6885b`); items 2–3 (settings state, restore data) sent; item 4 (diagnostics fill, DataAtRisk) deferred |
| 11 | R4-notify-diag | implementation / claude-sonnet-5.5 | `d94edf6a` | 357.2 | Merged `4dd9d7b` (`fe9a626`); macOS fix merged `cfa718b` (`2a6024c`), CI green on all platforms at `979b8de`. T-27-11 Windows toast accepted (name not seen on screen), T-28-13 one live GET returned 404 (no release), macOS rows to R4-mac-kit, T-33-10 to R4-app. macOS CI failed on a Windows-only test helper (`update_http.rs:148`); follow-up sent |
| 12 | R4-app | implementation / claude-sonnet-5.5 | `9215a078` | 632.9 | Packet `45a1c3e`. Merged `c62dc6c`: control map, decisions, retire/allow again, rollback, settings, snapshot connect, headline/tray, removed section, About/diagnostics, `attach_os_services` call (fixes OS services never attaching), macOS app behaviour (NOT RUN). macOS CI failed on a symlinked temp dir in a test; fix `fbd4237` merged `023ebbd`, CI green on all jobs. Snapshot gaps routed to R4-integrate pass 12 |
| 13 | R4-mac-kit | implementation / claude-sonnet-5.5 | `04f2b6e1` | 151.1 | Merged `846dc43` (scripts, `MAC-OWNER.md`) and the bundle follow-up (`bcda6d8`, `d552364`). All macOS rows NOT RUN until the owner runs the kit. Gaps routed to R4-app and R4-integrate |

Follow-up R4-core-settings (worker 8, same agent): a #[cfg(windows)] junction test was listed in an offline case, so main CI `c0d6a93` failed on macOS. Fixed in `577a8ca` (moved to native case `R4-core-settings:adapters-native`), merged `cb9931e`.

Follow-up R4-design-recheck (worker 4, same agent): FAIL. Blockers
R4D-1, R4D-10 and R4D-17; 8 majors.

| Checkpoint | Cumulative | Run-004 |
|---|---|---|
| Phase A merged | 18,873.7 | 1,460.7 |
| Review merged | 19,058.2 | 1,645.2 |
| Recheck merged | 20,570.7 | 3,157.7 |
| Recheck2 merged | 21,741.1 | 4,328.1 |
| Recheck3 merged (owner barrier) | 22,493.0 | 5,080.0 |

Stopped at the owner barrier: R4D-28 needs an owner-authorized bounded amendment (design retries exhausted) and the PENDING OWNER questions need answers before phase C.

| Checkpoint | Cumulative | Run-004 |
|---|---|---|
| Design PASS (recheck4 merged) | 23,002.6 | 5,589.6 |
| C1 foundation merged | 24,397.4 | 6,984.4 |
| C2 step 2 merged (removal, settings, ops) | 25,581.2 | 8,168.2 |
| C3 partial (integrate A) merged | 26,022.0 | 8,609.0 |
| C3 stage B merged | 26,736.6 | 9,323.6 |
| C3 commands merged | 27,474.3 | 10,061.3 |
| C3 settings apply + rollback merged | 28,195.9 | 10,782.9 |
| C3 rollback staging + removal replay merged | 28,421.6 | 11,008.6 |
| D notify-diag + C3 settings commands merged; R4-app blocked | 28,888.3 | 11,475.3 |
| C3 snapshot + notify wiring merged; R4-app pass 4, R4-mac-kit launched | 29,610.5 | 12,197.5 |
| R4-app + mac-kit merged; CI green; integrate pass 12 running | 30,518.4 | 13,105.4 |

Design review passed. Phase C remains blocked only on the PENDING OWNER product questions.
