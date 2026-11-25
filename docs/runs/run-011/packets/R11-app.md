# R11-app: guided settings sharing and settings visibility

Role: implementation, claude-sonnet-5.5. Read [COMMON.md](COMMON.md).
Owns `wow-sync-app`, the version bump to 0.4.0, `docs/release/v0.4.0.md`
and user docs. Do not edit runtime, core or adapters; ask the orchestrator
for any runtime change you need.

## Work

1. **Read first:** the Settings sync page (`pages.rs`), `settings_ui.rs`,
   design/settings.md §5.4 and §9, and run-010's `progress_ui.rs` and
   `history_ui.rs`. List in evidence what the GUI already does for each
   §9.3 row.
2. **Guided offer (PLAN goal 1):**
   - When add-ons are up to date, no decision waits, and settings are off
     with no policy: show "Share add-on settings between your computers?"
     in the headline area with "Set up…" and "Not now".
   - "Set up…" opens the §5.4 consent: the exact text, the account list
     with checkboxes (all included on this computer by default), and a
     master choice listing enrolled computers with this computer
     pre-selected. "Share settings" confirms; "Cancel" changes nothing.
   - When a policy exists and this computer has not confirmed: "<computer>
     is now the settings master. Use its add-on settings here?" with this
     computer's account list, "Use settings" and "Not now".
   - "Not now" is remembered and the offer appears only on the Settings
     sync page until settings are turned on.
   - Use the runtime interface commit from R11-runtime when it lands; until
     then build against the existing API and stubs.
3. **Visibility:**
   - The Add-ons list and Add-on detail show settings status per add-on in
     plain words (§9.1) with the identity in plain words on detail only.
   - Activity and history render settings `ChangeRecord`s as for code
     (source, reason, size) with the identity on the detail row.
   - Progress headline and tray cover the settings phase.
4. **Release notes `docs/release/v0.4.0.md`:** what settings sharing does
   and does not share; step-by-step how to turn it on (which computer to
   pick as master and why); what "waiting for a character to log in"
   means; how to pause, restore and undo; known limits. Update user docs.
5. **Live lab:** run the GUI on two synthetic computers with the lab data
   from R11-runtime's recipe (or your own per COMMON). Take screenshots of
   the offer, consent, the other computer's offer, statuses, and Activity
   entries. Screenshots must not show real account or character names:
   rename lab account and character folders to synthetic names first.

## Done when

All of the above work in the live lab and have tests (offer state
transitions, "Not now", consent defaults, status wording, history
rendering), both tiers pass, and evidence lists decisions made.
