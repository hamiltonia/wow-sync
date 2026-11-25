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
