# Run 006: live testing on the dev PC (proposal)

Status: **proposed, not approved.** Nothing in this plan may start until the
owner approves it and a ceiling, and records the decisions in §4.

- Drafted 2026-11-22 by the Opus 5.5 orchestrator after the owner tried
  `v0.1.0` on the dev PC's `_forever_` install and was refused with the
  internal text "This folder cannot be used: NotSyntheticInstall".
- Owner input (2026-11-22): "I don't think the data is so precious that we
  can't test against live? I can easily create separate backups, and we're
  not even working from the primary gaming system where the real data is."
  This is consistent with D-14: the dev PC's WoW data is test data.
- Proposed ceiling: 2,000 AIU from run-006 start. Stop new work at ≥1,800.

## 1. Goals

1. **Opt-in real-install enrollment (Q-24):** setup keeps refusing
   unmarked installs by default. A guided opt-in lets the user enroll a real
   install after confirming that backups exist. Before the first change to
   that install, the app takes a restorable snapshot of `Interface\AddOns`
   and the in-scope `WTF` files. The decision, snapshot location and time
   are recorded and shown.
2. **Plain-language refusals:** setup never shows internal names such as
   `NotSyntheticInstall` (the cause is a `{r:?}` debug format in
   `crates/wow-sync-runtime/src/service.rs`). Each refusal says why and what
   to do. "Try again" appears only when fixing something outside the app
   can change the result. The card's heading line is not blank.
3. **Forever `.toc` rule (Q-3, Q-20):** the rule follows the owner's client
   check (§4). Until it is answered, add-ons with suffixed tocs stay blocked,
   and the app explains why instead of silently listing them.
4. **Live verification on the dev PC:** a verifier enrolls the dev PC's
   `_forever_` install, which D-14 already allows, after taking a snapshot.
   It checks scan, sync, removal and restore against the real add-ons and
   their settings, then restores the snapshot. It never launches, kills or
   injects into WoW.
5. **Release `v0.2.0`** as a pre-release. The release notes describe the
   opt-in, its risks, and how to restore from the snapshot.

## 2. Non-goals

- The primary gaming computer is out of scope until the Mac or another
  computer joins under a later decision.
- No Dropbox API, installer, signing or macOS binary.
- No execution of add-on Lua or saved-variable files.

## 3. Packets (proposed)

| ID | Role / model | Work |
|---|---|---|
| R6-gate | Implementation / claude-sonnet-5.5 | Goals 1 and 3: core gate, snapshot and restore, toc rule |
| R6-app | Implementation / claude-sonnet-5.5 | Goal 2 and the opt-in GUI flow; release notes and version (goal 5) |
| R6-check | Verification / gpt-6.1-sol | Goal 4 on the dev PC, plus negative cases: no opt-in, declined backup confirmation, snapshot failure, WoW running |
| R6-review | Integration / claude-opus-5.5 | Safety review of the opt-in and snapshot before the tag |

## 4. Owner decisions needed

1. **Gate shape.** Recommended: an opt-in with backup confirmation and an
   automatic snapshot, with the default unchanged for other users, because
   the release is public. Alternatives: remove the gate, or allow only the
   dev PC.
2. **D-08 amendment:** may real add-on data be placed in
   `D:\Dropbox\Gaming Sync\WoW Sync Test`? Today D-08 allows synthetic data
   only.
3. **Q-3 client check:** the owner installs a supplied test add-on that has
   different `.toc` variants and reports which one the Forever client loads.
   Without it, most real add-ons stay blocked.
4. **Ceiling:** approve 2,000 AIU, or set another.
