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