# R9-review evidence

Reviewer: R9-review (review / claude-opus-5.5). Worktree
`D:\GitHub\wow-sync.worktrees\R9-review`, branch `run009/R9-review`, at `main`
`7e4a43d`. Reviewed `git diff 64830b1..7e4a43d`, which covers merges `b9fdc77`
(R9-decide r1), `52b4c81` (R9-app r1), `837a925` (R9-decide r2) and `7e4a43d`
(R9-app r2). The diff was checked against [PLAN.md](../PLAN.md) goals 1–8,
SPEC (including D-31 change detection), ACCEPTANCE, and D-31 itself. It also
covers the post-packet scope:

- kept_by_rival
- the Decide re-judge
- the kept_copy panic fix
- enrolled-from-saved-target
- catch_unwind
- the second-enroll refusal
- app_version

Product code was not changed. All probes were temporary tests and were
reverted with `git checkout`. `git status` was clean before the commit.

## What was run

Toolchain: `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"`,
`CARGO_TARGET_DIR=D:\WoWSyncLab\R9-review\target`.

| Check | Result |
|---|---|
| Probe 1: temporary test in `runtime/src/e2e.rs`, `cargo test -p wow-sync-runtime --lib rv9_probe`. Two synthetic computers A and B with mutual keeps of `alpha`, synced until both offer `ChoicesDiffer`. A decides copy[0]. Without syncing, B then answers its **stale** offered item with copy[1]. Then 4 sync rounds. | B's command returned `Ok(Written(..))`. B's record count went from 2 to 3. After the 4 rounds, A and B **both** offered `[ChoicesDiffer]` again. Reproduces RV9-01. |
| Probe 3: same file. A answers while paused and gets `Ok(Kept)`. A resumes and runs one `sync_once`, then answers again with the other copy. Then 4 sync rounds. | The item was **offered again** after the resume pass. The second answer returned `Ok(Written(..))`. After 4 rounds A still offered `[ChoicesDiffer]`. Related to RV9-01 (see there). |
| Probe 2: temporary test in `app/src/backend.rs`, `cargo test -p wow-sync-app --bin wow-sync rv9_probe`. Enroll a synthetic computer through the backend, quit, delete `state\enrollment.json` (keep `target.txt`), restart the backend. | `enrolled_now=true`, `snapshot.enrolled=true`, `world.enrolled=true`, headline "Checking add-ons…", `fault=None`. It stayed that way across further loop passes: no setup screen and no error. Reproduces RV9-02. |
| Code reading | The diff for choice.rs, decide.rs, reject.rs, control.rs, target.rs (package_copies/apply_constraints, read for context), service.rs (Command::Decide, settle_kept, status_inner, sync_once), detail.rs, diagnostics.rs, hash_cache.rs, fs_scan.rs, provider.rs (upload verification), backend.rs, app.rs, model.rs, world.rs, computers_ui.rs, pages.rs and docs/release/v0.3.2.md. |

Not run:
- the full workspace test suite (the workers' evidence reports it)
- a live GUI
- memory measurement
- anything on RYZENFISH

Read only: the worker evidence files, as untrusted data. No WoW Sync instance
was started outside test harnesses. `D:\WoWSyncDemo`, real `_forever_` and
`D:\Dropbox\WoW Sync` were not touched.

## Findings

| ID | Severity | Where | Finding | How to reproduce / reasoning |
|---|---|---|---|---|
| RV9-01 | **major** | `crates/wow-sync-runtime/src/service.rs` ~1156–1185 (`Command::Decide`, `None =>` fallback to `self.offered`); ~1414–1470 (write path, `issued_at_seq`) | **A stale Decide click on one computer re-creates a conflict that a peer already settled.** The re-judge looks the item up in the fresh `w.offers`. When the item is no longer offered (because a peer's newer decision has resolved it), the code falls back to the **stale** `self.offered` plan and writes it. That plan's `known` set does not include the peer's new record, and its `issued_at_seq` comes from the old cycle. The written record is therefore concurrent with the peer's decision, and both computers get `ChoicesDiffer` again. PLAN goal 2 requires such a click to be "refused as stale or re-offered", not written. The stale seq also risks reusing an issuer seq. Probe 3 shows a related gap: an answer queued while paused (`Kept`) does not remove the item from the offer. After resume the same item is offered again, a second answer is written, and the conflict persists. A same-computer double click with no Kept step is correctly refused, because the item is removed from `self.offered` after a write (~1462). | Probe 1 and Probe 3 above. Fix direction: when the item is absent from the fresh offers, return `Refused(Changed)` (or re-offer) instead of using the stale plan. Hide or mark an item that already has a queued Kept answer. Add an e2e test for "peer decided, then stale click here". |
| RV9-02 | **major** | `crates/wow-sync-app/src/backend.rs` ~720–748 (a saved target sets `enrolled_now=true` and `snapshot.enrolled=true`); ~984–987 (`svc` None → `continue`; `!s.is_enrolled()` → `continue` with no status refresh); `world.rs:424` (`confirm_enrolled` only ORs, is never cleared) | **A saved target pins the window to "enrolled" with no recovery path.** If `target.txt` exists but the enrollment is missing or unreadable (lost or deleted `enrollment.json`, a state folder partly restored, a computer retired and reset), the main view shows "Checking add-ons…" forever. Setup is unreachable and no error is shown. In v0.3.1 `detect_later` showed setup in this case. SPEC requires supported recovery through the GUI. A related case: when `open_service` fails with a saved target, `fault = "opening the sync service"` is set and the subtitle says it "will try again in a few minutes", but nothing reopens the service. The fault can only be cleared by restarting the app. A retired computer is **not** shown as active: its first real snapshot carries its standing, and `route` refuses commands by standing, so the transient "Checking…" state is harmless there. Trigger is rare, so this could be argued down to minor, but the result is a silent dead end. | Probe 2 above. Fix direction: after the first service open or status, clear the optimistic flag when `is_enrolled()` is false and show setup or a clear error. Retry `open_service` on the 5-minute fault timer, or change the wording. |
| RV9-03 | minor | `crates/wow-sync-app/src/model.rs:163`; `world.rs:433` (`checking = s.detail.is_none()`); runtime `service.rs` ~2748–2823 (`status_inner` returns no detail for provider error, VariantChanged, NeedsReconfirmation, InstallArea error, Cancelled) | **"Checking add-ons…" outranks real errors in the headline.** Every status without detail is treated as "still checking". The Working-tone headline then takes priority over the core Error/NeedsHelp headline (the `core_blocks_pause` path) for as long as the error lasts. The red banner and the Needs item still show the error, but the headline says the app is busy. Before this change the headline read "Waiting". This is a goal-4 honesty issue. | Reasoning from code: the provider is unreachable, status returns Err-standing with no detail, and the headline reads "Checking add-ons…". Fix direction: set `checking` only before the first status, or only when the status is `Cancelled`/in progress. |
| RV9-04 | minor | `crates/wow-sync-core/src/choice.rs:205–263` (`excluded`, `kept_by_rival`); `target.rs` ~896 (`apply_constraints`) | **kept_by_rival can override a live Resume.** An exclusion is ignored whenever some other live **Keep** pins the excluded release. `apply_constraints` raises `ChoicesDiffer` only when Keep trees differ. A live Resume (whose exclusion names X) concurrent with a live Keep X therefore auto-selects X with no decision, which silently resolves competing intentional choices. Before the fix that shape was stuck as Unavailable instead. Keep-vs-keep (the owner's RYZENFISH shape) always raises ChoicesDiffer, so the fix is correct there. Checked and unaffected: a **superseded** keep (covered nodes are not live), **rollback rejections** (`rejected()` is a separate hook, Neutral in ChoiceBehaviour), and a new keep excluding its own copy (`build_keep` ~595 drops exclusions the kept source is Same as). Edge: liveness is per folder, so an older keep still live on another folder of the package can override a narrower later keep's exclusion. This is theoretical because scopes normally equal package members. | Reasoning. Suggested test: A writes Resume (excluding X), B concurrently writes Keep X. Expect a decision, not X applied. Fix direction: restrict `kept_by_rival` to rivals that `apply_constraints` reports as a conflict, or raise ChoicesDiffer for Resume-vs-Keep. |
| RV9-05 | minor | `crates/wow-sync-adapters/src/hash_cache.rs` (`MAX_ENTRIES` 400k, `render` builds a full `String`, read cap 96 MiB) | **Cache memory at the cap can exceed the 100 MiB background limit.** About 400k entries make a ~60 MB map, plus the full rendered string at save time, plus the read buffer at load. Realistic trees are fine: 34k files give a ~5.3 MB file. Memory was not measured (R9-app reports it not run). Cache files for old install roots or variants are never pruned. | Reasoning or arithmetic. Suggest a lower cap, streaming render, and measuring under the A-xx memory obligation. |
| RV9-06 | minor | `hash_cache.rs` (key = path + size + mtime); `fs_scan.rs` ~296–325 | **Cache misses same-size, same-mtime changes.** D-31 accepts this as "very low probability". Updaters, zip extraction that preserves or normalises timestamps, and robocopy or backup restores can still produce it. The key has no file ID or ctime. Consequences are bounded: provider upload (`provider.rs` ~245) and `fs_install` copies re-verify sha256 (HashMismatch), so no corrupt blob is stored. However, a recurring HashMismatch or a published tree that reflects the old content is possible, and the error does not suggest "Check all files". Checked and OK: coarse mtime (the 2 s `recent_ms` settled check means a later write lands in a newer FAT 2 s bucket); moved root (scope = `install_root.display()|variant`, so the cache is cold); mtime is not used as ordering evidence anywhere new. `finish(complete)` from any full scan (status or Inspector) clears the full-check flag. The Inspector opens a second `HashCache` on the same file (benign, last writer wins). | Reasoning. Suggest that on HashMismatch the entry is dropped and a full check is advised in the message. |
| RV9-07 | note | `backend.rs` ~362 (`guarded`, `AssertUnwindSafe`); runtime `service.rs` ~2908 (`recover_interrupted` at every pass start) | **catch_unwind is reasonably safe, but in-memory state can be half-updated.** Checked: the app `lock()` handles poisoning (`unwrap_or_else(into_inner)`); the HashCache mutex does the same; the install lock is RAII; an interrupted apply leaves a journal that the next pass recovers; `gate.end_step()`/busy/armed are reset after a panic, so quit-when-safe still completes. The workspace has no `panic = "abort"` profile. Residual: SliceService fields (`offered`, `at_risk_apply`, `last_blocks`) may be half-updated, and commands still run against them during the 5-minute fault window. A panic in `command()` between `write_own_record` and `reserve_seq` could reuse a seq. `activity.rs:31/47` use `.expect("activity lock")`, so a poisoned activity mutex re-panics on every use (caught, but sticky). | Reasoning. Consider refusing commands while faulted, and reserving the seq before the write. |
| RV9-08 | note | `backend.rs` ~992–999 (interrupt / `armed`) | `armed` stays true for the whole `sync_once`, not only the read phase. This is safe: only read-only scans consult `ctl.cancelled()`, and the post-apply `observe_world` treats Cancelled as "no change". A continuous stream of commands could delay passes indefinitely (starvation), but that is not a safety issue. | Reasoning. |
| RV9-09 | note | `backend.rs` `group_log_line`; `diagnostics.rs` `version_text` | Log and privacy checked and OK. `Debug` is used only for content-free enums. No paths or add-on names are logged. Peer `app_version` is sanitised, older peers show "unknown", and the skew note appears only when known. | Reasoning. |
| RV9-10 | minor | `docs/release/v0.3.2.md` | The release notes are incomplete or inaccurate. The conflict fixes and stale-click fixes are deferred to "the R9-decide notes" instead of being stated for owners. "Tries again after five minutes" is false for a service-open failure (RV9-02). The 30-minute full-check timer resets on restart (disclosed). Not run per R9-app: live GUI latency, the 100 MiB memory check, and an e2e test that other packages still apply while one is Unavailable. | Read the notes against the code. |

## Verdict

**Two open major findings: RV9-01 and RV9-02.** No critical findings.

- **RV9-01:** a stale cross-computer Decide (and a re-answer after a Kept
  answer) writes a conflicting record. That re-creates ChoicesDiffer, which
  is against goal 2.
- **RV9-02:** a saved target with a lost enrollment, or a failed service open,
  leaves "Checking add-ons…" forever with no setup path and no retry, which is
  against GUI recovery.

The kept_copy panic fix, the index hardening, kept_by_rival for keep-vs-keep,
and the hash cache are acceptable. Their minor caveats are listed above.

## Recheck (main 7101088)

Rechecked on 2026-11-24 at `main` `7101088`. That commit includes round 3:
R9-decide `708b0ef` (RV9-01, RV9-04) and R9-app `e7d4e57` (RV9-02, 03, 05, 07,
10). The `R9-review` worktree was fast-forwarded to `7101088` and the original
probes were re-run. Every probe and mutation was temporary and reverted, and
`git status` was clean before the commit. All data was synthetic, from the
test labs. No live instance, `D:\WoWSyncDemo` or Dropbox was used.

### What was run

| Check | Result |
|---|---|
| Probe 1: same shape as before (A decides, then B answers its stale item) | B → `Err(Decide(Changed))`. B's records stayed at 2→2. After 4 rounds A and B offer `[]`. **Fixed.** |
| Probe 3: B answers while paused (`Kept`), runs one pass while paused, answers again, resumes, then runs one pass | While paused, the item is hidden after the command and after a pass. A second answer → `Changed`. After the **first** resume pass B still lists `[ChoicesDiffer]`; the record was written that pass (2→3). Clicking that listed item → `Changed`. After the rounds A and B offer `[]` and have 3 records. Safe. See RV9-11. |
| Mutation M1: `hide_answered` returns at once (no-op), with probe 3 | `an_answer_kept_while_paused_is_written_once_and_not_offered_again` and `a_stale_answer_after_a_peer_settled_the_conflict_writes_nothing` both **still pass**. Probe 3 now shows the item offered while paused, and **two more** `Kept` answers are accepted. After resume B has 5 records, and A and B offer `[ChoicesDiffer]` again. So `hide_answered` is needed but no worker test exercises it. See RV9-12. |
| Mutation M2: `resume_excludes_a_live_keep` returns `false` | `a_live_resume_excluding_what_a_concurrent_keep_pins_is_a_conflict_not_a_silent_winner` **fails** at line 537. The RV9-04 core test is exercised. |
| Probe 4 (core, `decide_tests.rs`): the worker's Resume + concurrent Keep ledger, then `decision_items` with copies 1.0 and 2.0 | `choices_conflict = true`. Neither copy is excluded. One `ChoicesDiffer` item with options `[Release 1.0, Release 2.0, KeepOnThisComputer]`, so it is answerable, not "Decide later" only. Deciding option 0 gives `choices_conflict = false` and one live choice. The existing keep-vs-keep world (`conflicting_world`) gives a hook result of `false`, so it is unchanged. |
| Probe 2 / worker test `a_saved_target_without_a_loadable_enrollment_offers_setup_and_does_not_enroll` (same steps as probe 2) | PASS: setup offered, `enrolled_now = false`, LOST_ENROLLMENT message. |
| Probe 5 (app `backend.rs`): enroll, quit, delete `state\hashcache-*.txt`, restart with the saved target, two passes, quit | After the first run: `["hashcache-c750a1a00856a988.txt"]`. After the restart and two passes: **`[]`**, with `enrolled = true`. The restarted service runs without the hash cache. See RV9-13. |
| `cargo test -p wow-sync-app --bin wow-sync -- a_saved_target_without after_a_panic checking_does_not_hide`; `cargo test -p wow-sync-adapters --lib hash_cache` | PASS 3/3; PASS 2/2 |

### Per finding

| ID | Status | Severity | Notes |
|---|---|---|---|
| RV9-01 | **closed** | — | `service.rs` Command::Decide no longer falls back to `self.offered`. An item missing from the fresh offers returns `Changed`. Probes 1 and 3 confirm this. The leftovers are RV9-11 and RV9-12. |
| RV9-02 | **closed** | — | `enrolled_at_start` requires `enrollment_loads()`, or the file to exist if the service did not open. A lost enrollment shows setup (`report_lost`, `detect_later(.., false)`). (c) Open-failure retry, by code reading: the retry happens only while a fault is set and `!abort`. The wait is `fault_retry() × 1/2/4` (5/10/20 min), and `fault_streak` rises only on a timeout. There is no tight loop, but any command that wakes `recv_timeout` (for example a Group action) also triggers an immediate open attempt (note). A successful open clears the fault, or shows setup if the enrollment does not load. A failed open keeps the fault and shows the error text, so "will try again" is now true. Edge: when the open fails **and** `enrollment.json` is missing, the open error text is replaced by LOST_ENROLLMENT and there is no retry. Setup is still offered (note). No automated test (disclosed). |
| RV9-03 | **closed** | — | `checking` now also requires an empty headline, no provider error and no reconfirmation. Tested. |
| RV9-04 | **closed** | — | (b) Probe 4: the new ChoicesDiffer has both releases plus Keep on this computer. Deciding it clears the conflict. `resume_excludes_a_live_keep` considers only non-Keep live choices, so the owner's two keep-vs-keep records cannot trigger it, and their item is unchanged. A resume that **covers** an older keep (the worker test's "a resume alone") is not a conflict. Residual: an old *Resume* that is still live on the owner's group, plus a concurrent keep of a copy it excludes, now raises a decision. That is intended. |
| RV9-05 | **closed** | — | 12 MiB map budget by estimated cost, 16 MiB file read cap, and old-scope caches pruned at start and at enroll. The transient rendered string at save is ≤ the file size. Memory on a real tree was not measured (disclosed). |
| RV9-06 | accepted | minor | Unchanged. The worker records it as accepted under D-31. |
| RV9-07 | **closed** | note | (d) After a caught panic in a pass or a Group command, the service is rebuilt from disk (`svc = open(t).ok()`), so half-updated memory is discarded. Locks: `InstallLock` and the state locks are file handles held in RAII values, so unwinding closes them. `SliceService::new` takes no exclusive lock, and the old value is dropped after the new one is built, without conflict (tested). Publishing: provider writes are per-record and the pass is abandoned at the panic, so what is on disk equals what a crash leaves. That case is handled by `recover_interrupted` and the crash-injection tests. Pre-existing residual: in `command()` the seq is reserved only **after** the record write (`service.rs` ~1454–1459). A panic or I/O error between them can reuse an issuer seq. A real crash has the same window (note). Each reopen re-runs `attach_os_services` (notifier and update service); the old service is dropped (note). **But see RV9-13:** the reopened service has no read control. |
| RV9-08 | note | note | Unchanged. |
| RV9-09 | note | note | Unchanged. |
| RV9-10 | **partly closed** | minor | The new notes state the conflict and stale-click fixes and the 5/10/20 min retries. "Known limits" still says "The decision and choice fixes of this release are described in the R9-decide notes", which is now stale and contradicts the new bullets. |
| RV9-11 | **new** | minor | Probe 3: after the resume pass that writes the queued answer, the same item is listed once more. `settle_kept` writes the record after the offers were computed from the pre-write ledger, and `hide_answered` then finds an empty queue. A click is refused as `Changed` and the item disappears next pass, so this is cosmetic. It is still a visible stale offer on the owner's "Decide" page for one pass interval. |
| RV9-12 | **new** | minor | (a) `hide_answered` is not exercised. With it made a no-op (M1), both round-3 e2e tests still pass, because the paused test syncs twice before asserting. Without it, a second and third answer while paused are each accepted as `Kept`, and after resume the group is back in `ChoicesDiffer` with 5 records. The code is correct today, but the guard is untested. Suggested test: pause, answer, run `sync_once`, then assert the item is not offered and that a second answer is refused. |
| RV9-13 | **new** | **major** | `backend.rs` ~722–738 and ~741 (`open` closure), ~1044, ~963, ~1108. Round 3 replaced the start-up open with an `open` closure that never calls `s.set_read_ctl(read_ctl(t))`. Only the setup Enroll path (~848) still sets it. So on **every normal restart** of an enrolled computer, and after every panic reopen or open retry, the service runs with `ScanCtl::default()`. That means: (1) no hash cache, so every pass re-hashes the whole tree, which undoes D-31 / PLAN goal 1 on the owner's machine (measured round 1: 6926 ms cold vs 150 ms cached); (2) no `abort` cancel, so Quit waits for read-only scans again (the RV8-01 / D-30 regression); (3) no `interrupt`, so commands wait for whole passes. Probe 5 reproduces (1): the cache file is never re-created after a restart. (2) and (3) follow from the same missing `ScanCtl` (code reading). Worker tests use `enrolled_lab`, which enrolls through `Cmd::Enroll`, so they never exercise the saved-target path. Fix: call `set_read_ctl(read_ctl(t))` inside `open`, and add a restart test that asserts the cache is used or Quit is prompt. |

### Recheck verdict

**Release-blocking: yes.** One new **major** finding, RV9-13: a restart, a
reopen after a panic, or an open retry leaves the service without the hash
cache, Quit cancellation and command interrupt. It regresses D-31 and
D-30/RV8-01 on the owner's normal start path. RV9-01, 02, 03, 04, 05 and 07 are
closed. RV9-10 is partly closed (minor). New minor findings: RV9-11 (one-pass
stale offer, refused on click) and RV9-12 (`hide_answered` untested). RV9-06 is
accepted. No critical findings.

### Addendum: relayed from R9-check (G2)

R9-check reported this on the real-copy lab at `7101088` (their evidence,
not mine). Scroll App settings to the bottom, then click Quit while a sync is
busy. The Quit sheet is prepended at the top of the same scroller, and the
scroll position is not reset. "Quit when safe" ends up off-screen (UIA
y=-1021) while the sheet makes every other action modal-blocked. The process
was still running more than 48 s later. Scrolling up revealed the button.

Code reading confirms the cause. `app.rs` ~2216–2234 (`content_view`) adds
`sheet_view` as the first child of the `content-scroll` `overflow_y_scroll`
div, with no scroll reset or pinned overlay. The same placement already
exists at `64830b1` (`app.rs` 2156–2167). It is pre-existing, not introduced
by run-009.

| ID | Status | Severity | Notes |
|---|---|---|---|
| RV9-14 | **new (pre-existing)** | **major** (per R9-check G2) | A modal sheet can be off-screen while it blocks every action, so Quit gives no visible confirmation. It is recoverable by scrolling. Fix: render the sheet outside the scroller (pinned), or scroll `content-scroll` to the top when a sheet opens. Add a UIA or layout check that the sheet's default button is within the viewport. |

### Addendum: RV9-13 independently confirmed (R9-check G3)

R9-check found the same defect on the real `7101088` artifact (restarted
enrolled PID 18864, after adding a 3.35 GB lab package). They saw no
hash-cache file, repeated long scans, and **Settings → Check all files**
logging `WARN check all files: nothing is set up to check` even though the
computer is enrolled. That last symptom follows from the same cause: the
`cache` `RefCell` is filled only by `read_ctl`, and `backend.rs` ~985–990
falls to the `None` arm. RV9-13 therefore also breaks the GUI "Check all
files" recovery action after any restart. It stays **major** and
release-blocking. The fix is unchanged: call `set_read_ctl(read_ctl(t))`
inside the `open` closure. Tests should cover restart, then cache use, Quit
promptness and Check all files.
