# R11-runtime evidence

Names are redacted (account A, character A1). No file contents or user paths recorded.

## Trial (old build, lab copy of dev add-ons and SavedVariables, 58 add-ons, 2 computers)

| ID | Severity | Finding |
| --- | --- | --- |
| D1 | major | After the master enables sharing nothing is published (activation capture never publishes). |
| D2 | minor | Settings record names carried raw texture codes. |
| G2 | minor | Settings records had empty versions, zero size, no identity. |
| D3 | major (safety) | A missing character folder on a peer was created by the settings apply. |
| N1 | minor | Cycle notes dropped; blocked units showed no reason (all "Active"). |

Confirmed working: a master's change reaches the peer, a peer's local edit is replaced with a
recovery copy kept, out-of-scope files untouched.

## Decisions made

- Decision: first share. A unit with no shared revision, a live file and not paused is published on
  activation by the master. Why: consent plus master choice is the explicit act; a shared copy
  present still means no publish (Q-75). Reversible: remove `first_share` in `settings_cycle.rs`.
- Decision: a unit with no local file and a missing account/realm/character folder is never applied
  (status WaitingForIdentity); the adapter refuses to create owner folders. Why: WoW creates them.
  Reversible: yes.
- Decision: notes surface through the report's apply note. Reversible: yes.
- Decision: Q-71 "two stable scans" is not shown separately; WaitingForWow covers any non-idle state.
- Decision: progress events carry no `source`.

## Results

- `cargo test -p wow-sync-runtime --lib`: 255 passed. Offline and native tiers pass.
- New tests: missing character / over-cap, game running.
- Memory: NOT re-measured on the new build (lab re-run not done). Open issue.
- Lab re-run on the new build: not run.
