# R11-review: independent review of settings sync (run-011)

- Reviewer: R11-review, policy model claude-opus-5.5.
- Reviewed revision: `main` at `f3fc1a0` (R11-runtime step 2 and R11-app guided offer/consent/status).
- Worktree: `D:\GitHub\wow-sync.worktrees\R11-review`, branch `run011/R11-review`.
- Scope: R-07 to R-10; A-11 to A-17, A-21, A-24 and A-25; [settings design](../../../design/settings.md);
  OD-01 to OD-22 in [DECISIONS-FOR-REVIEW.md](../DECISIONS-FOR-REVIEW.md).
- Data boundaries: everything ran in synthetic e2e labs (temp dirs). Logs and the probe patch are
  in `D:\WoWSyncLab\R11-review\` and are not committed. No real game, Dropbox or demo folder was
  read or written, and no add-on Lua was executed.

## Verdict

**Open majors: yes (5). No criticals.** Settings sync must not be released as "safe" until
RV11-01 to RV11-05 are fixed or the owner accepts them explicitly:

- RV11-01: a replaced copy cannot be restored from the GUI. OD-20 is refuted in part.
- RV11-02: recovery points of settings and code evict each other.
- RV11-03: a zero-byte or incomplete capture is published and applied group-wide.
- RV11-04: the status says "syncing" for a unit that differs and is not shared.
- RV11-05: "Use shared settings" cannot be reached from the GUI.

## How the majors were confirmed

I added three temporary probe tests to `crates/wow-sync-runtime/src/e2e.rs` and reverted them with
`git checkout`; the product code was not changed. The probes reuse the existing `settings_pair`
harness: `a` is the master, `b` the peer, and the file is the account-level `Alpha.lua`. The patch
is kept at `D:\WoWSyncLab\R11-review\rv11-probes.diff`. The output is in
`D:\WoWSyncLab\R11-review\probes-3.log`.

Command, from the worktree root:
`cargo test -p wow-sync-runtime --lib rv11_probe -- --nocapture --test-threads=1`.
Result: all three probes FAILED, which is the expected outcome here: each probe asserts the
required behaviour, so a failure confirms the defect.

| Probe | Key output |
| --- | --- |
| `rv11_probe_a_replaced_copy_is_not_offered_by_gui_lists` | Before the yield, b's edit is `latest_capture` and the status is `Active`. Right after the yield, the on-disk recovery copy exists, the restore list holds 2 entries, `mine_in_list=false` and `latest_is_mine=true`. On the next idle cycle, `latest_is_mine=false` and `mine_in_list=false`. After two more master applies, the on-disk recovery copy holds mine: **false**. A runtime `RestoreSettings(mine)` returns `Ok(Written)`, so the runtime can restore the copy but the GUI never offers it. |
| `rv11_probe_b_settings_applies_keep_code_recovery` | Code recovery of `alpha-v1`: present before (`true`), gone after five settings applies (`false`). `status().recovery_points` has 0 rows. In reverse, the settings recovery copy of `v = 4` is present (`true`) before two code updates and gone after them (`false`). |
| `rv11_probe_c_zero_byte_master_file_with_bak_is_not_shared` | The master's `Alpha.lua` was truncated to 0 bytes with `Alpha.lua.bak` beside it. Result: the master published a third revision, the peer's file is now `""`, and both units show `Active`. |

## Findings

### RV11-01 (major): a non-master's replaced copy is on disk but not restorable from the GUI; OD-20 refuted in part

OD-20 claims "the replaced copy is kept in recovery". That holds on disk, but only briefly, and
the GUI cannot reach it.

**On disk.** The yield writes `.wowsync\settings-recovery\<op>\0`, and `capture_unit` puts the
bytes in the local settings store (`settings_cycle.rs:75-110`). Probe A confirmed this. The
on-disk copy is pruned after two later applies of the same add-on (RV11-02). Probe A: "after two
more master applies ... false".

**In the GUI.** The copy cannot be restored there:

- `restore_controls` (`crates/wow-sync-app/src/settings_ui.rs:234`) lists only `row.history`.
  That list is the ledger's shared revisions with `!is_head` (`crates/wow-sync-app/src/features.rs:784-787`).
  The peer's routine edit was never a revision.
- `latest_capture` is `captures.first()` (`settings_cycle.rs:1058`). Applied copies are pushed as
  new captures on the next cycle (`settings_cycle.rs:764-772`). So the replaced copy is "Share my
  settings" for exactly one cycle, then disappears from every GUI list. The only GUI path to it
  would anyway be a group-wide publish, not a local restore.
- The local recovery view does not list settings recovery copies.

The runtime would accept the checkpoint (`RestoreSettings(mine)` → `Ok(Written)`), but the GUI
cannot name it.

This breaks:

- A-12: the replaced state is recoverable.
- A-16 and R-10: recovery goes through the GUI.
- The AGENTS rule "Supported recovery must work through the GUI".

The existing tests only check that the text exists somewhere on disk, at e2e
`a_non_master_routine_write_yields...` and `use_shared_settings_replaces...`.

The OD-16 behaviour change (RV11-06) makes this path routine rather than rare: every
non-master edit made before the master's first share is yielded on first enable.

**Fix.**

- Record a `before_change` capture as a local, listable recovery state (§2.1 item 3, §7). Expose
  local captures in `SettingsUnitRow`, either as local entries in `history` or as a separate
  `local_copies`. The GUI can then offer "Restore this computer's copy", running `RestoreSettings`
  with a local-store checkpoint.
- Pin that state against pruning (RV11-02).
- Add an e2e test that restores the yielded copy using only the checkpoints the snapshot rows
  expose.

### RV11-02 (major): settings and code recovery points evict each other and one another's units

- `plan_settings_apply` journals a settings apply as package `unit.folder`
  (`settings_cycle.rs:114-124`).
- `settle_points` keeps only `KEEP_RECOVERY = 2` points per package, whatever their kind
  (`crates/wow-sync-core/src/exec.rs:884-904`, `exec.rs:926`).
- `remove_recovery` also deletes `settings-recovery/<op>` (`crates/wow-sync-runtime/src/effects.rs:638-646`).

Results:

- Two settings applies (a yield, or any master update) delete the add-on's code rollback point.
  The Recovery/rollback list then has no row (probe B: `rows=0`).
- Two code updates delete settings recovery copies.
- Account and character units of the same add-on share one budget of two, so one cycle that
  applies several characters can evict another character's copy in the same pass.

This breaks:

- R-10/A-16: code and settings recovery are independent.
- A-17: three states per add-on for code and, separately, for settings.
- OD-20's "kept in recovery".

**Fix.**

- Key recovery points by kind and unit. For example, settle settings points under a
  `settings:<folder>:<identity-hash>` key, or filter `settle_points` by `RecoveryPointV2.kind`.
- Keep the three-state rule of §7 (`KEEP_SETTINGS_STATES`), with pinned exceptions.
- Add a regression test: a code update, then N settings applies, then the code rollback row still
  exists; and the reverse.

### RV11-03 (major): no settle and no suspect classification, so a truncated or zero-byte file is published and applied everywhere

Design §2.1/§2.2 (Q-71) requires two stable settle scans. A capture is suspect if a file changed,
shrank to zero, or vanished while a `.lua.bak` exists. A suspect capture is never published.

The runtime does none of this:

- The settings cycle runs on every pass when WoW is not running.
- `publish_decision` gets hard-coded `complete: true` and `changed_in_session: true`
  (`settings_cycle.rs:807-809`).
- Core's `classify_capture` and `CaptureClass` (`crates/wow-sync-core/src/settings.rs:346`) are
  never called from the runtime.
- There is no `Suspect` status (`settings_cycle.rs:1076-1097`).

Probe C: an empty master file next to a `.lua.bak` was published as a revision and replaced the
peer's file with zero bytes; both sides show `Active`.

The same happens for a crash mid-write, or for any partial write seen by a single scan in the gap
right after WoW exits. A peer only loses data once a valid shared revision has been replaced.
That earlier revision stays in history and is restorable from the GUI, so the loss is
recoverable. That is why this is major and not critical, but it spreads automatically to every
computer.

OD-19 describes "a brief settle wait after closing WoW" that does not exist in code. It should be
corrected.

This breaks R-07 ("an interrupted/incomplete write is not captured"), A-11 and A-21.

**Fix.**

- Before capture, run `classify_capture` against two settle scans (reuse the Q-41 quiescence),
  plus the zero-byte and `.lua.bak` checks.
- Keep a suspect capture locally only, and surface `Suspect` with "Check again" and "Restore
  previous copy".
- Pass the real `complete` and `changed_in_session` flags.
- Add an e2e test from T-S03 Neg.

### RV11-04 (major): status says "Settings are syncing here" when the unit differs locally and is not shared

`settings_units` only ever reports these statuses (`settings_cycle.rs:1076-1097`): Retired,
Disabled, NotShared, PolicyConflict, WaitingForWow, TooLarge, OwnedBySeveral, Paused,
NeedsChoice, WaitingForIdentity and **Active**.

The core `target_case` results `LocalDifferent`, `NoSharedCopy` and `InSync`, and the `Suspect`
class, are never mapped. `live = status == Active` (`settings_cycle.rs:1105`).

Probe A: after the peer's routine edit, which is not published and differs from the head, the row
is `Active`. The app shows "Settings are syncing here" (`settings_ui.rs` `status_text`). The app
already has `LocalDifferent`, `NoSharedCopy`, `InSync` and `Suspect` texts (§9.1), but the
runtime never sends them; the comment at `settings_ui.rs:12` says so.

This breaks A-24/A-25 truthful status: a difference that is not shared must be visible ("Changed
here, not shared"). The group roll-up then also says "in sync" while it is not.

**Fix.** Carry the last `Case` per unit in `SettingsFacts` and map `LocalDifferent(reason)`,
`NoSharedCopy`, `InSync{from}`, `Waiting{from}` and `Suspect` into the snapshot. Add a test.

### RV11-05 (major): "Use shared settings" cannot be reached from the GUI

`unit_controls` offers "Use shared settings" only for `UnitStatus::LocalDifferent`
(`settings_ui.rs:194-206`). Because of RV11-04, the runtime never reports `LocalDifferent`, so the
button never appears. The runtime command (`Command::UseSharedSettings`, used in e2e
`use_shared_settings_replaces...`) works.

A non-authority computer with a held difference has no GUI way to adopt the shared copy until
the master writes again. Same AGENTS rule and A-16 as RV11-01.

I confirmed this by code path; probe A shows the row is `Active` while it differs. It needs no
separate probe because it follows directly from RV11-04.

**Fix.** RV11-04. Also add an app-level test: given a snapshot row with a local difference, the
GUI offers "Use shared settings".

### RV11-06 (minor, owner decision): OD-16 changes approved intent Q-75

Design §4.1/§5.2 (Q-75, R4D-19): becoming the master never publishes existing local differences.
The activation capture is never published, and "Share these now" is off by default.

OD-16 (`settings_cycle.rs:735-757`, `first_share`) publishes the master's current files for
every unit with no shared node, on first enable. It is defensible under §4.2 case 2
(`NoSharedCopy`), and consent plus choosing a master is an explicit act. But:

- It is a change of approved intent, recorded only as a run-time decision.
- Combined with RV11-01, every peer's pre-existing settings are routinely replaced on enable, with
  no GUI recovery.

I do not reject OD-16. The owner should confirm it. It should be gated on RV11-01, and
preferably also on a consent-sheet line saying that the master's settings replace other
computers' settings for add-ons not yet shared.

### RV11-07 (minor): settings retention is unbounded and three-state pruning is not implemented

- `captures.truncate(8)` (`settings_cycle.rs:771`) drops local references, but local-store blobs
  and manifests are never removed. No runtime caller uses `remove_blob`, `prune_states` or
  `gc_roots` (searched in `crates/wow-sync-runtime/src`), so provider-side settings blobs have no
  collection path from the runtime either.

This breaks A-17 (current plus two previous states, with visible exceptions). Storage grows
without bound at the play-session rate. This loses no data, so it is minor.

**Fix.** Implement §7 with `prune_states` and `gc_roots`, keeping pinned roots, and add a test.

### RV11-08 (minor): identity-bearing paths can reach the activity and diagnostic logs on failure

- The executor uses `o.live_rel` as the item label for settings ops (`exec.rs:1272`), for
  example `Account/<account>/<realm>/<character>/SavedVariables/X.lua`.
- That label goes into `Explanation::DestinationChanged`, `StagedMismatch` and `LiveMismatch`
  (`exec.rs:1568-1620`, `1854-1941`).
- `FsError` texts carry absolute paths.
- These reach `report.applied` and `blocks`, then the activity audit (`service.rs:4103-4114`)
  and `wow-sync.log` (`crates/wow-sync-app/src/backend.rs:435`, "sync held: ...").
- `diag_log::redact` (`crates/wow-sync-app/src/diag_log.rs:136`) only replaces `USERPROFILE`.

This contradicts the runtime's own rule that an identity "is never logged"
(`settings_cycle.rs:1020-1022`) and the privacy intent of §9.2. It only shows up on a race or
failure.

I confirmed this by code path, not by a probe. As a minor it does not need one.

**Fix.** Give settings items a neutral label: the add-on folder plus "account settings" or
"character settings" with an index. Redact `WTF/Account/...` segments in `diag_log::redact`.

### RV11-09 (minor): settings rename checks the destination, then replaces it

`fs_install::settings_rename` (`crates/wow-sync-adapters/src/fs_install.rs:554-594`) checks that
the destination is absent, then calls `std::fs::rename`. On Windows that call replaces an
existing file. A writer that creates the destination in between would be silently overwritten.
The game gate before each rename makes the window narrow.

**Fix.** Use a no-replace move: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, or `std::fs::hard_link` and then remove.

### RV11-10 (note): known defect, not re-proven

The R11-runtime follow-up is fixing a known defect: a peer missing one character folder marks
every add-on `WaitingForIdentity` and applies nothing. I did not spend effort on it.
`identity_present` is evaluated per unit (`settings_cycle.rs:266-276`), so the cause is more
likely in how facts and the status are aggregated or in the apply gating than in the per-unit
check. The fix should come with a mixed test: one present character and one missing character
both apply correctly.

### RV11-11 (note): areas checked with no finding

- **Consent.** The cycle returns early without `local.consent` or without an approved cap.
  `EnableSettings` requires a non-empty account list. `StartSettings` sends consent before the
  policy (`backend.rs:59-75`), so a failure in between leaves a safe state.
- **Guided offer.** `settings_ui::offer` and `headline_offer` appear only when enrolled, active,
  with accounts, no conflict and a calm headline. "Not now" persists in `AppSettings`
  (`app.rs:835`, `940`). The account list comes from the runtime's snapshot, and only listed
  accounts can be ticked.
- **Folder creation.** `settings_rename` refuses to create account, realm or character folders
  (OD-17 holds).
- **Consent text.** It matches §5.4.

## Judgment calls

- **RV11-03 severity.** Rated major, not critical, because the previous shared revision stays
  in the ledger history and the GUI can restore it. It would be critical if history pruning
  (RV11-07, once implemented) or the eviction in RV11-02 removed that path.
- **RV11-05.** Kept separate from RV11-04 because the missing recovery action is a distinct
  A-16 failure, even though one fix closes both.
- **OD-16.** Not rejected; escalated to the owner because it changes approved intent (Q-75).

## Verification

| Tier | Command (from the worktree root) | Result |
| --- | --- | --- |
| Probes | `cargo test -p wow-sync-runtime --lib rv11_probe -- --nocapture --test-threads=1` | 3 of 3 failed, confirming RV11-01, -02 and -03 (and RV11-04's status). The probes are reverted; `git status` was clean before this file was added. |
| Offline | `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links). Run on `f3fc1a0` plus this file. |
| Native | `cargo xtask verify --tier native` | Not run: this is a review-only change (one markdown file); the probes used the offline e2e harness. |

## Round 2: recheck at `faddd5d`

- Revision: `faddd5d`, which is `main` with `d9a2690` merged into `run011/R11-review`. Scope:
  the RV11 fixes and the new code covered by OD-23 to OD-35.
- Data boundaries: as in round 1. I used synthetic e2e labs only, and the logs are in
  `D:\WoWSyncLab\R11-review\r2-*.log`.
- Not reviewed: R11-runtime's in-flight round (the SharedFromHere reason, multi-file record
  versions and the full-lab memory check).

### Round 2 verdict

**One new open major (RV11-12). No criticals.** All five round-1 majors are fixed. I confirmed each
with the tests that were added for it, and I read the code paths they cover.

### Status of the round-1 findings

| ID | Was | Now | Evidence |
| --- | --- | --- | --- |
| RV11-01 | major | **Fixed** | Displaced copies are kept per unit (`kept`, newest 3) and shown as `local_copies`. `RestoreSettings` accepts them. Test `a_copy_replaced_by_the_masters_is_listed_restorable_and_survives_later_applies` (it is my round-1 scenario plus three more applies) passes. The app test `this_computers_replaced_copy_can_be_restored` passes. OD-20 is now confirmed. |
| RV11-02 | major | **Fixed** | `settle_points` keeps `KEEP_RECOVERY` per `rp.class()` (`exec.rs:919-930`). Test `settings_and_code_recovery_points_never_evict_each_other` passes. |
| RV11-03 | major | **Fixed** | `suspect_units` (`settings_cycle.rs:349`) waits for the settle period and looks a second time. Test `a_zero_byte_file_next_to_a_backup_is_suspect_and_never_published` passes. |
| RV11-04 | major | **Fixed** | The statuses LocalDifferent, InSync, Suspect and the others are reported. Tests `a_local_edit_that_is_not_shared_is_local_different_not_syncing` and the app test `the_runtimes_new_statuses_and_local_copies_reach_the_controls` pass. |
| RV11-05 | major | **Fixed** | OD-28 offers "Use shared settings" for every LocalDifferent reason. App test `every_local_difference_offers_share_and_use_shared` passes. |
| RV11-06 | minor | **Fixed** | Being made master no longer publishes anything; sharing is an explicit "Share these now" (OD-23, OD-31, OD-33). Test `becoming_master_publishes_nothing_and_share_now_publishes_the_listed_units` passes. |
| RV11-07 | minor | **Fixed, but see RV11-12** | Provider GC was added (OD-34), and the local store is pruned. |
| RV11-08 | minor | **Fixed** | Test `a_settings_note_never_names_an_account_realm_or_character` passes. |
| RV11-09 | minor | **Fixed** | `MoveFileExW` is called without `MOVEFILE_REPLACE_EXISTING` (`fs_install.rs:613-627`). |
| RV11-10 | note | **Fixed** | Test `one_missing_character_waits_alone_and_everything_else_still_applies` passes. |

### New findings

#### RV11-12 (major): provider GC deletes a reused old blob that a newer revision needs

- **What happens.**
  - `upload_checkpoint` (`settings_cycle.rs:581-587`) skips any blob or manifest already on the
    provider, so a reused object keeps its old mtime.
  - `sweep_provider` (`settings_cycle.rs:502-541`) runs on every computer with consent. It deletes
    any settings object older than `GC_MIN_AGE_MS` (30 days) that is outside its *own* ledger's
    keep set: heads plus the newest `KEEP_CAPTURES` per unit.
  - So when a computer republishes content that existed more than 30 days ago, the new revision
    points at old objects. Examples: a setting toggled back, a restore of an old revision, or a
    share of an old local copy. A peer whose ledger has not yet received that revision's record
    sweeps those objects. Dropbox delivers files in no particular order, so the peer can get the
    sweep stamp's turn before it gets the record.
  - After that the revision is a dangling head. No computer re-uploads it, peers never apply it,
    and the status shows **Active**, not a problem. This breaks A-12 (changes propagate) and the
    honest-status rule.
- **Probe.**
  - A temporary e2e test, since reverted: `a` (master) publishes v0..v5 and `b` applies v5. All
    provider settings files are aged to 60 days.
  - `a` republishes `v = 1`. The provider still holds 12 files: the manifest and blob were
    reused, not rewritten.
  - The three new member records of `a` are held back to simulate Dropbox lag, `b`'s sweep stamp
    is forgotten, and `b` syncs. The sweep leaves 6 files, deleting the `v = 1` objects.
  - The records are then released and `b` syncs twice. `b` still has `v = 5`, and its status is
    `Active`; the assertion for `v = 1` failed. The log is `D:\WoWSyncLab\R11-review\r2-gc.log`.
- **Suggested fix.** Do both:
  1. When `upload_checkpoint` reuses an existing object, refresh its mtime: rewrite it or touch
     it, as a provider-allowed write of the uploader's own content.
  2. In `sweep_provider`, keep every object referenced by any manifest on the provider that is
     younger than the minimum age, not only the sweeper's own keep set.

  A self-healing step would also help: when a head's blob is missing, the issuer re-uploads it
  from its local store and the status says "shared settings are not available yet". Add the
  probe above as a regression test.

#### RV11-13 (minor): history can offer a restore that GC has made impossible

- Every revision in the ledger history is listed as restorable (`restore_possible`,
  `settings_cycle.rs:1565`).
- The provider and local GC keep only heads plus the newest three per unit (`settings_cycle.rs:460-481`).
- Restoring an older entry is refused with `UnknownCheckpoint` (`service.rs` `share_checkpoint`).
  That is safe, but the GUI offers an action that cannot work.
- Fix: mark history entries whose checkpoint is in neither the local store nor the provider as
  not restorable, or list only the retained ones.

#### RV11-14 (note): ShareSettingsWhenReady is not tied to master authority in the runtime

- `share_when_ready` (`service.rs`, about line 1647) pushes every `NoSharedCopy` unit once facts
  exist. It checks only the flag and consent; `ShareSettingsWhenReady` itself is refused without
  consent (`NotAllowed`).
- The app sends the command only when this computer is the chosen master (OD-33; app test
  `the_guided_consent_pushes_only_when_this_computer_is_the_master`).
- The risk is small:
  - The flag is cleared after the first pass that has facts.
  - Only units with no shared copy are pushed, so the push cannot override a master's copy.
- Suggestion: drop the flag when the policy is not "this computer is master" at the time of use.

#### RV11-15 (note): ShareSettings and ShareSettingsNow check standing, not consent

- Both commands check only that the standing is Active.
- Captures exist only after consent, and there is no command that withdraws consent. In practice
  nothing unconsented can be pushed today.
- Add a consent check if withdrawing consent is ever added.

### Areas checked clean (round 2)

- **Streaming I/O (OD-30).**
  - `put_blob_stream` and `put_object_reader`/`put_stream` (`provider.rs:232-270`) write a temp
    file, call `sync_all`, re-hash the temp file and only then rename. A size or hash mismatch
    leaves the target untouched.
  - `stage_settings` (`fs_install.rs:518`) verifies the size and hash of the staging copy.
    Recovery and the journaled renames happen before any live file is replaced (`exec.rs`).
- **Consent sheet.** The add-on list comes from the installed add-ons (`w.addons`,
  `settings_ui.rs:786-801`), not from WTF.
  - Judgment call: `available_accounts` lists the `WTF\Account` folder names before consent
    (`service.rs:2883`). I accept this because account choice is part of consent (V-05); only
    directory names are read, never file contents.
- **Missing-character fix (RV11-10).** Only the affected unit waits.

### Round 2 verification

| Tier | Command (from the worktree root) | Result |
| --- | --- | --- |
| Recheck tests | `cargo test -p wow-sync-runtime --lib` (the ten settings tests named above), `cargo test -p wow-sync-core --lib settings_and_code_recovery`, `cargo test -p wow-sync-app` (six settings_ui tests) | All PASS. |
| GC race probe | `cargo test -p wow-sync-runtime --lib rvprobe -- --nocapture --test-threads=1` | FAILED as expected, confirming RV11-12. Reverted; `git status` was clean before this file was edited. |
| Offline | `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) on `faddd5d` plus this file. Native not run (review-only change). |

## Round 3: final recheck at `6da3e0f`

- Revision: `main` at `6da3e0f`, merged into `run011/R11-review`. Since round 2 this brings in:
  - the RV11-12, RV11-13 and RV11-14 fixes;
  - `SharedFromHere` and unit-level versions in records;
  - the lazy, per-folder code publish (`ea798cd`);
  - OD-36 to OD-39.
- Data boundaries: as before, synthetic labs only. The logs are
  `D:\WoWSyncLab\R11-review\r3-*.log`.

### Round 3 verdict

**No open critical or major.** RV11-12 to RV11-14 are fixed. The lazy code publish is safe: it
uploads in the right order and never writes a state that names missing objects. Its state bytes
are unchanged on the wire, so v0.3.3 peers are unaffected. One new minor (RV11-16) is about
memory headroom.

### Status of the round-2 findings

| ID | Was | Now | Evidence |
| --- | --- | --- | --- |
| RV11-12 | major | **Fixed** | I re-ran my round-2 race probe (Dropbox lag on a reused 60-day-old blob) unchanged. `b`'s sweep now keeps all 12 settings files, and `b` applies `v = 1` with status `InSync`; in round 2 it deleted 6 files and `b` stayed on `v = 5` as `Active`. Two later sweeps still keep everything. R11-runtime's regression tests `a_reused_old_copy_starts_its_age_again_when_content_returns_to_it` and `cleanup_keeps_every_copy_a_recent_revision_record_names_even_if_unread_here`, plus `WaitingForSharedCopy`, cover it. Log: `r3-gc.log`. |
| RV11-13 | minor | **Fixed** | History leaves out entries whose copy is gone (test `settings_history_does_not_offer_a_copy_the_shared_folder_has_lost`). |
| RV11-14 | note | **Fixed** | `is_settings_master` (`service.rs`) guards both `ShareSettingsWhenReady` and `ShareSettingsNow`. The deferred share goes through `ShareSettingsNow`, so the master check applies again when it runs. OD-37 records this. |
| RV11-15 | note | Unchanged | It is still a note and needs nothing until a command that withdraws consent exists. |

### Review of `ea798cd` (lazy, streamed code publish)

I probed this with a temporary core test in `plan.rs`, since reverted. It drives
`run_publish_objects` with `publish_objects` and a recording `PublishEffects`. Log: `r3-publish.log`.

| Question | Result | Evidence |
| --- | --- | --- |
| Upload order: blobs, then the folder's tree, then the state record last | **Holds** | In the `publish_objects` stream, every tree comes after all its folder's blobs; a blob already produced for an earlier folder is not produced again (`plan.rs:96-136`, probe). `run_publish_objects` re-checks `source_fingerprints` and only then calls `write_record` (`exec.rs`). |
| A crash or abort mid-publish never leaves a published state with missing objects | **Holds** | The probe made each of the 5 uploads fail in turn, with both `HashMismatch` and `Io`. No run wrote a state record. The only provider-visible result is content-addressed objects, which the next attempt reuses (as before). The sequence number is persisted before publishing, so it is never reused. |
| A source hash that no longer matches aborts as `Unstable` and nothing partial is published | **Holds** | A `HashMismatch` on a `Folder` blob returns `Aborted(PlanBlock::Unstable)`. A failure on a tree (`Bytes`) or any other kind returns `Failed`. Neither writes a state. `put_object` hashes the temp file before the rename (`provider.rs` `put_stream`), so the mismatch is detected before the object is visible. |
| Dropping local file lists from the state breaks nothing | **Holds** | `FolderState::file_entries` is `#[serde(skip)]` (`inventory.rs`), so it was never on the wire. The probe confirmed that `encode(plan_publish_state(..).state)` is byte-identical to `encode(plan_publish(..).state)`. After publishing, the change records use `publishable` and `w.inventory`, not `plan.state` (`service.rs` publish arm). Peers, recovery and rollback read file lists from tree records, which are still uploaded. |
| v0.3.3 peers in the same group | **Compatible** | The state and tree record bytes are unchanged. Only the upload order differs, and readers follow the state record, which is still written last. |
| Progress total | OK | `count_publish_objects` uses the same de-duplication as the stream. |

### New findings

#### RV11-16 (minor): memory headroom under the 100 MiB gate is thin and measured only once after the fix

- In `evidence/R11-runtime.md`, the follow-up table records a settings pass at **99 MiB** private
  working set (100 KB file), with idle varying from 57 to 94 MiB between runs. That measurement was
  taken before `ea798cd`.
- After `ea798cd` there is a single measurement: a peak of 83.2 MiB. OD-39 quotes that number as
  the release figure.
- The figure meets the gate (OD-38 measures private working set), but given the run-to-run spread
  it is not yet robust evidence for A-25.
- Suggestion: before release, repeat the full-lab pass at least three times on the release build
  and record the maximum. The GUI-baseline reduction is already proposed as a future run.

### Round 3 verification

| Tier | Command (from the worktree root) | Result |
| --- | --- | --- |
| Publish probe | `cargo test -p wow-sync-core --lib rvprobe -- --nocapture` | PASS: ordering, the abort and failure matrix, and identical state bytes. Reverted. |
| GC race probe | `cargo test -p wow-sync-runtime --lib rvprobe -- --nocapture --test-threads=1` | PASS (in round 2 the same probe failed). Reverted. |
| Offline | `cargo xtask verify --tier offline` | PASS (fmt, clippy, test, required-cases, deny, markdown-links) on `6da3e0f` plus this file. Native not run (review-only change). |
