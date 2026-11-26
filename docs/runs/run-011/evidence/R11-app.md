# R11-app evidence

Branch `run011/R11-app`. Step 1 (inventory) only so far.

## §9.3 inventory: what the GUI already does

| §9.3 row | Existing |
|---|---|
| Settings sync page | `pages.rs::settings_sync`: enable panel with account checkboxes (none ticked by default), master panel and "Change master…", per-add-on exceptions, units list with Share/Use shared/Choose/Restore, per-folder Pause/Resume, banner (`settings_ui::banner_text`). |
| Add-on detail | `pages.rs` ~1251: restore controls per identity. No per-add-on settings status line yet. |
| Decision | `features.rs` maps `DecisionScope::Settings`; decisions UI exists. |
| Needs attention | no settings-specific items found beyond data-at-risk. |
| Activity | `history_ui.rs` groups `SettingsApplied`/`SettingsPublished` ("saved settings shared/updated") and the `SettingsMaster` reason; no identity on the detail row. |
| Computers | master shown; retire consequence exists. |
| Setup/consent | no §5.4 consent sheet; enable panel only. |

Gaps to build: guided offer on headline, §5.4 consent with exact text and
all accounts ticked by default, "use master's settings here" confirmation,
persisted "Not now", per-add-on settings status in the Add-ons list/detail,
identity on history detail, settings phase in progress/tray, release notes.

## Runtime interface needed

1. `AppSettings.settings_offer_dismissed: bool` (persisted; load/save in
   `control.rs`), settable through the existing `Cmd::SaveSettings`.
2. A status reason for a unit whose character folder is missing on this
   computer (e.g. `SettingsUnitStatus::WaitingForCharacter`).
3. Guided-offer state is derivable today from `SettingsSnapshot`
   (`enabled`, `master`, `master_name`, `available_accounts`); if
   R11-runtime adds a different query, tell the app its signature.

## Decisions made

- StartSettings enables consent before setting the policy.
- All accounts ticked by default on both computers (OD-11).
- Old enable panel kept only when there is no offer.
- Add-ons list line is generic; detail line names the identity.
- Not now persisted via settings_offer_dismissed.
- Folder match is case-insensitive.
- Retired computer or retired master gets no offer.

Verify: offline tier PASS (fmt, clippy, test, required-cases, deny, links). Native tier PASS.

Fail-without-change: each new test was run against a deliberate mutation of the code it covers (removing the Active-standing check, the dismissed check, the enabled check, flipping the default master pick and tick filter, giving the confirming computer a master, dropping the identity from the Activity line, lowering the waiting urgency) and FAILED each time; the code was restored by git checkout each time.

Lab: synthetic two-computer lab prepared (synthetic add-ons, accounts and characters, own state and provider); screenshots wait on the runtime accounts list.


## Interface 2 and live lab

- Merged main (cf5b761). Real wording for Phase::Settings (Saving add-on settings / Using settings from <computer>, counts and add-on), ChangeRecord.identity on the Activity row, and the TooLarge, OwnedBySeveral and WaitingForWow statuses (list line most-urgent; detail lines name them). New tests: settings progress wording, attention statuses; the Activity identity test now goes through the record.
- Live lab (two synthetic computers, synthetic accounts and characters, own state and provider, debug build, UI Automation clicks): computer A showed the headline offer, opened the consent with both accounts ticked and this computer as master, and Share settings enabled sharing; computer B then showed 'A-demo is now the settings master. Use its add-on settings here?', its consent had both accounts ticked, and Use settings confirmed it without choosing a master. Processes were stopped by captured PID.
- Not observable live yet: per-add-on statuses (Add-ons list stays 'Not shared yet'), settings progress and Activity identity rows, because the runtime still emits stub units, no Settings phase and no identity (its Step 2).
- Screenshots: the screen was locked, so CopyFromScreen captured the lock screen (deleted); PrintWindow was used. Kept in the lab, not committed: a4-consent, a5-shared, a6-master, b2-consent, b3-after-use, b4-addons, b5-activity. The overview shot showing a lab path was deleted.
- Verify: offline PASS, native PASS.


## Final live run (after merging 84b9beb)

- Cycle note: the app ignored `SyncReport.apply_note`; now `backend` keeps notes starting "some saved settings were not shared" in `Shared.settings_note`, `World.settings_note` carries it, and Settings sync shows it as a sentence (test added; not mutation-checked).
- Lab: A (accounts ACCOUNTONE/ACCOUNTTWO, a 34 MB Plater file) showed the guided offer, consent (both accounts ticked, this computer as master), Share settings; Add-ons then read "Settings are syncing here" for three add-ons and "Too large to share automatically (34.0 MB)" for Plater; Activity showed "Saved settings shared" with a detail row.
- B joined, showed "A-demo is now the settings master. Use its add-on settings here?", consent with both accounts ticked, Use settings; Add-ons then read "Waiting for a character to log in on this computer" for all four add-ons (B lacks one character; the runtime reports it for every add-on) and Activity has no settings rows yet because nothing could be applied.
- Not observed live: settings progress text (passes were too short to capture), the cycle note on screen, "Using settings from <computer>" (runtime source pending), an applied peer row with identity.
- Lab bug found and fixed in the lab only: a stale `.wowsync\lock` directory made the first pass "held".
- Verify: offline PASS earlier; native PASS; offline re-run after evidence links fixed (below).

## Decisions made (final run)

- Decision: carry the cycle note via Shared/World and show it on Settings sync only. Why: it is a settings-level notice, not an add-on status. Reversible: yes.
- Decision: Add-ons list line stays generic, detail names the identity. Why: column width. Reversible: yes.
- Decision: "Not now" persisted via `settings_offer_dismissed`. Why: runtime field exists. Reversible: yes.
- Decision: narrowed the other Add-ons columns. Why: Name was squeezed to an ellipsis. Reversible: yes.

## Screenshots

Each was viewed and contains only synthetic names (A-demo, B-demo, ACCOUNTONE, ACCOUNTTWO, Bagnon, Details, Plater, Questie) and no path. Directory: [R11-app/](R11-app/). Overview shots were not kept because they show the lab folder path.

- [a2-consent.png](R11-app/a2-consent.png): computer A, headline offer and consent.
- [a3-addons.png](R11-app/a3-addons.png): A, Add-ons with syncing and too-large rows.
- [a5-activity.png](R11-app/a5-activity.png): A, Activity with "Saved settings shared".
- [b2-consent.png](R11-app/b2-consent.png): B, the other computer's offer and consent.
- [b4-addons.png](R11-app/b4-addons.png): B, waiting-for-a-character rows.

## Review fixes (RV11-01, -04, -05, -06), app side

- RV11-06 / Q-75 / OD-23: `settings_ui::{share_candidates, first_adoption, share_now_on, pushes, make_master_control}`. The guided consent (this computer as master) and a new "Make this computer the master…" confirmation on Settings sync list units that differ here (`LocalDifferent`) or have no shared copy (`NoSharedCopy`), with a "Share these now" box: off normally, on only when no unit shows any shared state (first adoption). `GroupAction::{StartSettings, SetSettingsPolicy}` carry `share_now`; `backend::dispatch` issues one `ShareSettings` per ticked unit after the policy. The direct "Use this computer as the settings master" button is gone (it skipped the confirmation).
- RV11-05: "Use shared settings" is offered for every `LocalDifferent` reason (the design table excluded `LocalChange`; the table needs the same edit).
- RV11-04: Suspect wording is "This add-on's settings file looks damaged; it was not shared". InSync/LocalDifferent/NoSharedCopy wording already existed; they show only when the runtime sends them.
- RV11-01: `UnitRow.local_copies` and "Restore this computer's copy from <day>…" (confirm names the recovery copy that is kept). Not fed yet.
- Tests (all fail without their change; checked by mutation: default-on always, push for another master, NoSharedCopy not listed; can_share ignored was caught after adding a non-shareable unit): `share_these_now_lists_only_…`, `making_this_computer_master_pushes_only_…`, `the_guided_consent_pushes_only_when_…`, `this_computers_replaced_copy_can_be_restored`, `a_damaged_settings_file_…`, `every_local_difference_offers_share_and_use_shared`, and the master-choice test now expects no direct "this computer" button.

### Runtime interface delivered (419908f) and wired

- `SettingsSnapshot.to_share` feeds the confirmation list (copied into the app's `ShareItem`; `ShareOffer` is not exported). One `Command::ShareSettingsNow{units}` is sent after the policy, or nothing when the list is empty. `group_has_shared_settings` drives the OD-23 default (on only when false). The four statuses, `in_sync_from` and `local_copies` (hash = checkpoint for `RestoreSettings`) are mapped in `features::settings_unit_rows`. TODOs removed.
- Mutation checks, all caught by tests (reverted afterwards): default-on always; first adoption ignoring `group_has_shared_settings`; push for another master; Use shared settings excluded for LocalChange; local copies not listed. New mapping test: `the_runtimes_new_statuses_and_local_copies_reach_the_controls`.
- NOT DONE in this pass: a fresh live lab run and screenshots for the confirmation, LocalDifferent, Suspect and local-copy restore (unit tests only). Earlier screenshots remain from commit 1607acf.

### Decisions (Decision / Why / Reversible)

- Decision: "Use shared settings" for all LocalDifferent reasons. Why: RV11-05; explicit and keeps a recovery copy. Reversible: yes.
- Decision: first adoption = `!group_has_shared_settings`. Why: the runtime now says so. Reversible: yes.
- Decision: a single `ShareSettingsNow` with the ticked units, after the policy. Why: matches the runtime API; a refusal shows once. Reversible: yes.
- Decision: share-candidate lines use the add-on title when known, else the folder name. Why: plain words. Reversible: yes.
## Live lab run (synthetic names, own lab, v0.4.0 debug build)

Screenshots (privacy-checked by viewing): b-master-confirm, a-master-confirm-nonfirst, b-bagnon-waiting, b-ss-localdiff-suspect, b-restore-confirm.

Observed working: master confirmation on first adoption lists 16 units with "Share these now: yes" and publishing created 16 revisions; the peer applied them; non-first adoption defaulted to "no" and, ticked, published only the missing character's units; a missing character shows "Waiting for <character> to log in on this computer" while other units are up to date; a locally edited file shows "Changed here, not shared" with "Use shared settings", which replaced it with the shared copy; an emptied file with a .bak shows "This add-on's settings file looks damaged; it was not shared"; "Restore this computer's copy from <day>" confirmed and restored the file (33 to 50 bytes).
Fixed in app: confirmation read "copy of copy from ..."; now "Restore this computer's copy from ...". Test extended (not mutation-run separately; old text fails the new assertion by construction).
Both tiers pass.

### Defects for the runtime (steps / expected / actual)
- D1: Guided consent on first adoption, before settings sync is on: expected a "Share these now" list ticked by default (OD-23); actual `to_share` empty, no list, nothing shared. Units only exist once settings sync is enabled.
- D2: Publishing via ShareSettingsNow (16 revisions), a peer applying them, and a later push produced no settings rows in Activity or changes.jsonl; expected "Saved settings shared/applied" rows with identity detail. Only add-on code rows appeared.
- D3: Phase::Settings progress text not observed; passes finished too fast to catch (not verified).
- D4 (app wording): "Settings up to date (from <this computer's own name>)" on the master; consider omitting self as source. Not changed.
- Observation: Add-ons list shows the worst unit status per add-on (e.g. "Waiting for a character" although most units are in sync). A UIA click on the confirm button once appeared to do nothing; not reproducible.

### Decisions
- Decision: drop "of" in the local-copy restore sentence. Why: label already says "copy from <day>". Reversible: yes.
- Decision: leave D4 unchanged. Why: runtime/design question about self-source; low impact. Reversible: yes.

## Re-check against runtime 9ac5912 (D1, D2, D4)

Wired: `StartSettings.share_when_ready` (this computer is master and "Share these now" is ticked, on by default on first adoption) sends `Command::ShareSettingsWhenReady` after `EnableSettings` and the policy. The consent sheet says "The settings of the add-ons installed on this computer (<names>) will be shared once WoW Sync has read them." `UnitStatus::InSync.from` is now optional; the master's own copy reads "Settings up to date".
Tests that fail without the change (mutation-checked): `the_guided_consent_asks_to_share_once_ready_only_for_this_master` (when_ready forced false fails), `the_runtimes_new_statuses_and_local_copies_reach_the_controls` (own-copy mapping given a source fails), `status_words_follow_the_design`.
Live (fresh synthetic lab, own state): guided first adoption with the box ticked shared all units without any further click (a2-consent); Activity shows "Saved settings shared" rows with identity, hash version, size (a-activity-expanded); the master reads "Settings up to date" and Plater's 34 MB file reads "Too large to share automatically (34.0 MB)" (a-master-wording). Privacy-checked by viewing.
Open (runtime): a settings publish from this computer shows the source text "pushed to this computer" (reason PeerPush, no source). Expected wording that says it was shared from this computer; the reason or source for a ShareSettingsWhenReady push needs to say so.
Decision: when no settings units exist yet the consent lists installed add-on titles (at most 6, then "and N more"). Why: nothing in WTF may be read before consent. Reversible: yes.
Decision: `share_when_ready` is only requested when this computer is the chosen master. Why: another master's copy must never be pushed from here. Reversible: yes.

## Merge of 36fcbd0 (WaitingForSharedCopy, SharedFromHere)

Decision: wording "Waiting for the shared settings to arrive". Why: the status carries no computer name, so the suggested "<computer>" form can't be filled honestly; it matches "Waiting for <character> to log in" in tone. Reversible: yes.
Decision: WaitingForSharedCopy keeps urgency 6 (same as other waiting states), so the Add-ons list never reads "up to date" while any identity waits for its copy. Test: `the_new_attention_statuses_read_in_plain_words` (mutation: urgency 1 fails).
Activity: a non-master's single share reads "shared, ... · shared from this computer", never "pushed". Test: `a_single_share_from_a_non_master_computer_reads_as_shared_from_here` (mutation: old wording fails). No screenshot refresh needed (no layout change).

## Fix round: G3, G4, G5, G7 and OD-46 (commits a6f3cd8, 8ae856c, this one)

Implemented and tested (206 app tests): G3 distinct choices (kind, source, date+time, size) and pending-decision headline/Needs attention; G4 Pause all / Resume all settings; G5a add-on detail settings history; G5b restore wording on both computers; G5c recovery list with source, kind, time, size; G7 fallback.
Mutation checks (each fails without the change): head_label kind text; restore arm in history_ui; shared_by_other gate; ResumeAllSettings mapped to PauseAllSettings.
G7 root cause: the runtime creates OwnedBySeveral only for blocked units that also have a row; a blocked add-on with no row fell back to "Not shared yet". App fix: `shared_by_other` finds another row whose OwnedBySeveral names this add-on (key or title, case-insensitive). Test with two add-ons declaring the same variables.

### OD-46 (window close on hide): NOT shipped, blocker reported
I built close-on-hide (window destroyed, recreated on show, Shell held in a global) and A/B-measured it in the same release binary (113-add-on lab, zoom 1.5, start visible, Settings sync, hide at ~38 s, private working set, 25 ms samples, 2 runs each). Per-second maxima after hide: close 57-91 MiB, keep (SW_HIDE) 58-94 MiB; same sawtooth (about 60 idle, 84-87 during passes) in both. No measurable benefit; run-to-run variance (about 13 MiB, allocator retention after start-up scan) exceeds any effect. Earlier runs: close z1.0 hidden max 76.8; close z1.5 87.6/90.8; start-hidden z1.5 76.6; old build z1.5 93.1.
Decision: reverted the refactor (app.rs/main.rs unchanged from 8ae856c). Why: risk without demonstrated gain. Reversible: yes. The remaining excess is allocator/scan retention, not window renderer memory.
Counter vs sampler: they agree within 0.1 MiB at steady state; differ up to 10 MiB only at pass transitions because the perf counter is a ~1 s lagging instantaneous value while QueryWorkingSet samples every 25 ms.
Not done: show/hide cycle test (moot after revert) and refreshed live-lab screenshots for G3-G7 (not captured this round; tests cover the wording).

## Fix round: G9 (main 3c720d2): blocked on a runtime field

Rebuilt the checker's shape in my lab with the release build: two add-ons (titles CheckSharedOne/CheckSharedTwo), each declaring `## SavedVariables: CheckShared` and `## SavedVariablesPerCharacter: CheckShared`, one account-wide `CheckShared.lua` file, group created, settings enabled and shared, app restarted. The Add-ons list reproduces the checker: both read "Not shared yet".
Cause (runtime, `settings_cycle.rs` ~1068 and ~1100): the `UnitFact::Several` fact is created for `scope.blocked` units, but the rows are built only from `scope.owned` units and units that have ledger revisions. When every declarer is blocked, neither add-on has any unit row, so `SettingsSnapshot` holds nothing naming either add-on: no row, no `shared_with`. My earlier app fallback only works when one declarer has a row (the runtime e2e shape, where Alpha has revisions), so it cannot cover this shape. The app cannot read TOC declarations itself (the snapshot has none).
Missing runtime field: a row (status `OwnedBySeveral`, `shared_with` filled) for every blocked unit that has a Several fact, or a snapshot list such as `shared_variables: Vec<{ folders/titles: Vec<String> }>`. With either, the app maps it to "Shared with <other add-on>, not synced" (existing `shared_by_other` and row wording). I did not add a test or screenshot for the fix, since no data exists to build it from.
G10 waits for the runtime fields.

## Fix round: G9 and G10 (main d396fdb)

G9 confirmed live (release build, checker shape rebuilt: two add-ons declaring `CheckShared` in both SavedVariables and SavedVariablesPerCharacter, one account file), including after an app restart: the Add-ons list reads "Shared with CheckSharedTwo, not synced" and "Shared with CheckSharedOne, not synced". Screenshot `R11-app/g9-shared-with-after-restart.png` (synthetic names; the `_forever_` text is the variant label). Test `rows_the_runtime_emits_for_two_blocked_declarers_both_read_as_shared_not_synced` passes through `settings_unit_rows`; mutation (drop `shared_with` in the mapping) fails it. The app needed no change: the runtime rows were enough.
G10: shared-checkpoint restore entries now read "Restore the copy from settings master A-demo (25 November 10:16:26, 33 B)…" (kind and source, time to the second, size; copies that still read alike get "earlier"/"later", or "n of N", ordered by written time then seq). Live lab (UI-automation text of the add-on detail, after two edits): two entries with different seconds and sizes. The detail's restore buttons were below the visible window area, so there is no image of them. Test `shared_checkpoint_entries_show_source_kind_size_and_tell_same_minute_copies_apart`; mutations: always tagging earlier/later fails it, dropping the source name fails it.
Decision: seconds always shown, "earlier/later" only when labels still collide. Why: keeps labels short. Reversible: yes.

## Fix round: G17 (main ca4902e, OD-58)

Settings choice labels (Settings sync page, add-on detail, anything built from the choice list) now say how the copy came about, the time to the second and the size; identical labels get "(older)"/"(newer)" by time then list order. No hash is shown.
Before: "Keep the copy from settings master RecheckA (25 November 18:26, 49 B)" twice.
After (shape): "Keep the restored copy from settings master <name> (25 November 18:26:07, 49 B)" and "Keep the copy edited in game from settings master <name> (25 November 18:26:41, 49 B)"; two copies equal in every respect end "(older)" and "(newer)".
How it came about comes from the revision kind of the choice's newest revision, looked up in the row's history (Restore = "restored copy", Choice = "chosen copy", Push = "pushed copy", Auto = "copy edited in game"); no runtime change needed. If a head's revision is not in the history the label falls back to "copy".
Test `equal_choices_show_how_they_came_about_seconds_and_newer_older`; mutations (no tie-breaker, restored word dropped, minutes instead of seconds) each fail it.
Screenshot: NOT captured. A two-choice list needs two diverged computers in my lab; I did not rebuild that setup this round. The label logic is covered by the test.

## Fix round: G18, memory growth per UI cycle (main with OD-61)

Result: no app-side retention found. The growth comes from the UI-automation client's queries, not from the app's UI code. No source change this round.

Method: own small lab (release build, zoom 1.5, ShowWindow reopen), private working set sampled every 50 ms. Baseline here is about 36 MiB (the 113-add-on lab was too slow to onboard in my lab), so growth is easy to see.

Bisect, driven through UI automation (each query walks the whole accessibility tree):

| Surface (UIA-driven) | After each hide / cycle | Trend |
|---|---|---|
| show/hide only, 10 cycles | 36.0 to 36.5 | flat |
| Settings sync page only, 10 cycles | 36.4 to 36.9 | about +0.05 per cycle |
| add-on detail open/close, 10 cycles | 36.2 to 39.8, commit 100 to 103.6 | about +0.4 per cycle |
| confirmation dialog open/cancel | 36.5 to 38.2 by cycle 5 | about +0.4 per cycle |
| full path, 10 cycles | 36.9 to 41.8, commit 100.7 to 105.6 | about +0.5 per cycle |
| detail without hiding, 5 opens per cycle, 8 cycles | 38.0 to 51.9 | about 0.4 per open |
| detail, client forces GC each cycle | 38.1 to 48.0 over 6 cycles | unchanged |
| NO navigation at all, 150 whole-tree queries per cycle, 6 cycles | 38.6 to 48.0, commit 102.7 to 112 | growth with no UI change |

Isolation (same lab, same build):
- Navigation and detail opens with no UIA queries, 8 cycles x 5 opens (40 detail opens): 35.8 to 36.5 (commit 99.6 to 101.0), flat. Done with posted mouse messages; a screenshot confirmed the detail opened.
- Same, with one UIA query first so the accessibility adapter stays active, 40 opens: 36.0 to 37.0, flat. So building and sending the accessibility tree every frame is not the cause.
- Four separate client processes, 150 whole-tree queries each, then exit: app commit 99.9, 100.3, 101.3, 100.4, 100.4, then 101.4 after 20 s. The growth disappears when the client exits; it is held while the client lives.

Root cause: each whole-tree UIA query makes the client hold references to the app's accessibility provider objects (the harness's PowerShell process keeps them; forcing GC in the client does not release them). Memory grows with the number of queries, so a harness that queries more per cycle (find, click, find) shows 10 to 17 MiB per cycle on a larger tree. It is not tied to hide/show, to a surface, or to this app's data (no unbounded cache found: the app's maps are keyed by control and page names and are small).

Proof without UIA, zoom 1.5, 10 cycles of [Computers, Activity, Overview, Add-ons, add-on detail x3, Settings sync, hide window, show window] by posted mouse messages and native hide/show:

| Cycle | hidden quiet (MiB) | commit (MiB) | running peak (MiB) |
|---|---|---|---|
| 1 | 37.3 | 101.5 | 38.3 |
| 2 | 36.8 | 100.9 | 38.3 |
| 3 | 36.5 | 100.6 | 38.6 |
| 4 | 36.5 | 100.6 | 38.6 |
| 5 | 36.7 | 100.8 | 38.6 |
| 6 | 36.5 | 100.5 | 38.6 |
| 7 | 36.5 | 100.6 | 38.6 |
| 8 | 36.5 | 100.6 | 38.6 |
| 9 | 36.7 | 100.8 | 38.6 |
| 10 | 36.5 | 100.6 | 38.6 |

Flat; hidden peak far under 100 MiB. Not covered UIA-free: the restore confirmation dialog (its buttons sit below the visible area and need scrolling) and the tray-menu reopen (ShowWindow is what the tray does in-process). The dialog was covered only in the UIA-driven bisect, where it behaved like every other UIA-driven surface.

Decision: no app change for G18. Why: the growth is reproduced with no UI activity at all and vanishes when the querying client exits; changing app code would hide nothing real. Reversible: yes.
Recommendation to the checker (not an app fix): drive the cycle without whole-tree queries per step (query once for coordinates or use a fresh client process per cycle), or measure after the client exits. The same UIA retention with a real screen reader is the screen reader's own cost and is not distinguishable from this harness effect.
Open, unverified: whether accesskit_windows itself releases provider objects promptly when a client disconnects; I only showed that exiting the client frees them. The harness scripts are in my lab (`mem\g18*.ps1`), not committed.