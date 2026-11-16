# Apply, safety gates, recovery and status

Part of the [slice design](README.md). Decisions are in core (`admit`,
`plan`, `exec`, `status`); effects are adapter calls through the core port
traits ([core-api.md](core-api.md) §7). Revised in design retry 1 for review
findings B-01..B-03, M-02..M-06 (see
[R2-design-fix evidence](../runs/run-002/evidence/R2-design-fix.md)).

## 1. Sync cycle (runtime, R2-slice)

Single background loop, every 3 s (a change is noticed well within the
10-second start requirement, A-30), plus on demand from the GUI:

1. If any journal exists: `recover_interrupted` (§4). If it leaves any
   package unresolved, no new apply starts this cycle (publish still may).
2. Read local enrollment; open provider root; read group, members and the
   state-file listing; `admit_member_states` with the persisted watermarks
   ([publication.md](publication.md) §6). Persist the returned watermarks.
3. Scan AddOns ([adapters-api.md](adapters-api.md) §3) → `build_inventory`.
   Any `Unstable` folder → that folder is not published or replaced this
   cycle; the AddOns top-level listing itself unstable → nothing is published
   or applied this cycle (`Waiting(unstable)`).
4. If the stable inventory differs from the last published one → publish
   ([publication.md](publication.md) §4), including source revalidation.
5. Read needed trees → `compute_target`.
6. Observe activity → `plan_apply` with `ApplyGate`. `Running` / `Unknown` →
   status `Waiting`, nothing written to AddOns.
7. `run_apply`; append audit entries; if any package reached `Verified`,
   rescan and publish (the `apply_report` is the confirmation signal).
8. `derive_status` → GUI.

Reading AddOns and publishing while WoW is running is allowed (reads only;
Q-5). Game-file changes — including rollback and recovery renames — are only
made after a fresh `NotRunning` observation (SPEC R-11; an idle launcher
outside the variant folder does not block).

## 2. Apply algorithm (`run_apply`)

Preconditions checked by `plan_apply`: `ConfirmedVariant`, the package's
folders stable in the current scan, every plan precondition derivable
(§2.1), `Activity::NotRunning`, no unresolved journal. Then `fx.lock()`
(app install lock; it excludes other WoW Sync instances only, not
CurseForge or other writers — those are handled by §2.1/§2.3).

### 2.1 Plan preconditions (B-02)

Every `FolderApply` carries `expected_live`:

| `expected_live` | Meaning |
|---|---|
| `Absent` | folder did not exist in the scan used for planning |
| `Tree { tree, fingerprint }` | folder existed with this content hash and this metadata fingerprint |

`fingerprint` = SHA-256 over the sorted list of every entry in the folder
(files **and** directories) as `path \t kind \t size \t mtime_ns` — so a
change of membership (added/removed file or directory), size or mtime
changes it. The fingerprint is cheap (metadata only); the tree hash is the
content proof. Both are recorded in the journal.

### 2.2 Per package

For each `PackageApply` in key order (`G(x)` = the **gate**: call
`activity()`; continue only on `NotRunning`):

Shorthand used below, all on `ApplyEffects` ([core-api.md](core-api.md) §7):
`observe_live(f)` = `observe(Slot::Live{f})`; `hash_live_folder(f)` =
`tree_of(Slot::Live{f})`; `verify_staged` = `tree_of(Slot::Staging{..})`;
`move_<a>_to_<b>` = `rename(Slot::<A>, Slot::<B>)`; `prune_recovery` =
`list_recovery` + core decision + `remove_recovery`; `exists(slot)` =
`observe(slot).is_some()`. Recovery points are written with
`write_recovery_point` (`pinned` is a field of the record).

1. **Stage.** Journal phase `staging` written. For each folder: `stage_folder`
   (copy each blob from the provider into `staging\<op>\<folder>`, verify each
   file hash), then `verify_staged` (tree hash equals target). Missing /
   cloud-only blob → package `Skipped(NotAvailableYet)`, staging discarded,
   next package. Hash mismatch → `Failed`, staging discarded.
2. **Destination revalidation (content).** For every folder:
   `observe_live(folder)` must match `expected_live` (absent, or same
   fingerprint **and** `hash_live_folder` equal to the expected tree, and no
   entry modified within the quiescence window, Q-15). Any difference or
   read error → package `Replanned(DestinationChanged)`: staging discarded,
   journal removed, nothing live touched; the next cycle rescans and replans.
   This is what prevents replacing an update made by CurseForge (or the user)
   while staging ran.
3. **Commit.** Journal phase `committing` (written and flushed before any
   live change). For each folder in name order, each rename separately gated:
   a. `G`; then `observe_live(folder)` fingerprint must still equal the value
      from step 2, else stop (b. not executed) → §2.3 interruption.
   b. If live exists: `move_live_to_recovery`. Journal folder state `moved_old`.
   c. `G`; then `move_staged_to_live`. Journal folder state `installed_new`.
   Any rename error → §2.3 with reason `RenameFailed`.
4. Journal phase `committed` (all folders `installed_new`); write the
   recovery point record (`pinned: true` until verification).
5. **Verify.** `G`; then for every folder: `hash_live_folder` equals the
   target tree, and the recovery copy of each replaced folder hashes to its
   `expected_live` tree (detects a writer that modified the old folder during
   the residual window). All equal → journal phase `verified`.
   Otherwise → journal phase `needs_help` with the reason (`LiveMismatch`,
   `RecoveryMismatch`); both alternatives kept; the recovery point stays
   pinned; package result `NeedsHelp`. If `G` fails here the journal stays
   `committed` (`Waiting(GameRunning)`) and verification resumes in a later
   cycle (§4).
6. **Finalize — only from `verified`.** Recovery point `pinned: false`;
   `prune_recovery` (§2.4); journal removed; staging discarded; package
   result `Applied`. Only `Applied` packages appear as `applied` in the next
   `apply_report`.

A package never changes live folders without a journal that records which
folder is in which slot. Folders of the package present locally but absent
from the target cannot occur in a plan: such a package is `NeedsDecision`
(no mixing, [domain.md](domain.md) §6). Nothing in `AddOns` is deleted;
replaced folders move to recovery by same-volume rename.

### 2.3 Interruption (B-01)

If `G` returns `Running` or `Unknown` (or a rename fails) after the
`committing` journal was written:

- **No further live mutation of any kind** in this run — no rollback, no
  completion. `run_apply` returns immediately with
  `stopped = GameRunning | ActivityUnknown | RenameFailed` and the package
  result `Interrupted`.
- The journal (phase `committing`, per-folder states) and all three slots
  (live, staging, recovery) are left as they are. Recovery points stay pinned.
- Status: `NeedsHelp` headline only if the rename failed; for game activity
  `Waiting`: "WoW started while <addon> was being updated; it will be
  restored to its previous version after WoW exits." The mixed live state of
  that one package while WoW runs is the reported residual race (PLAN §6.7).
- Recovery happens in a later cycle via §4 after a fresh `NotRunning`.

The residual window that remains (honestly reported, PLAN §6.7): WoW (or an
external writer) may start between a `NotRunning` observation and the very
next rename; an external writer may modify a folder between its fingerprint
check and its rename. Step 5 detects the latter for replaced folders.

### 2.4 Retention (B-03, SPEC three states)

- Normal pruning (only in step 6): per package keep the newest **2** unpinned
  recovery points (current live + two previous states).
- Never pruned: any recovery point with `pinned: true`, and any point
  referenced by an existing journal. These are visible temporary exceptions
  in status ("kept for recovery: N extra copies", A-17), so more than two can
  exist.

## 3. Rolling a package back (used only by §4)

Requires a fresh `NotRunning` before **each** rename (`G`). Using the journal
entry, for each folder in reverse name order:

| Journal folder state + observation (`exists` per slot) | Action |
|---|---|
| `pending`, live = expected | nothing |
| `moved_old`: live absent, recovery has old | `move_recovery_to_live` |
| `installed_new`: live = new, recovery has old | `move_live_to_staging`, then `move_recovery_to_live` |
| `installed_new` for an install (no old): live = new | `move_live_to_staging` |
| anything else (both missing, unexpected content) | stop: `needs_help(UnknownInterruptedState)` |

After rollback, each restored folder is hashed and must equal its
`expected_live` tree; otherwise `needs_help(RestoreMismatch)`. A restore
rename failure → `needs_help(RestoreFailed)` with journal and all slots kept.
Successful rollback → journal phase `rolled_back`, recovery point record of
the op deleted (its folder moved back), staging discarded, journal removed.

## 4. Crash and interruption recovery (`recover_interrupted`)

The journal holds one `phase` **per package** ([publication.md](publication.md)
§2). `recover_interrupted` loads every journal, and for each package in key
order applies the rule for its phase independently (one package's
`needs_help` does not stop recovery of the others, but no new apply starts
while any package is unresolved):

| Package phase | Action |
|---|---|
| `staging` | discard staging for the op; mark package done |
| `committing` | `G`; if not `NotRunning` → `Waiting`, retry next cycle. Else roll back per §3 |
| `committed` | `G`; if not `NotRunning` → `Waiting`. Else run verification (§2.2 step 5) — **never** finalize without it. Success → `verified`, then finalize (step 6); failure → `needs_help` |
| `verified` | finalize (step 6) |
| `needs_help` | leave as is; report; recovery points stay pinned. Resolution is GUI work outside the slice; the state is shown and preserved |
| `finalized` / `rolled_back` | nothing |

When every package of a journal is `finalized` or `rolled_back`, the journal
is removed (`remove_journal`). An unparseable journal → whole op
`NeedsHelp(UnknownInterruptedState)`, no apply until resolved. Each phase
transition is written (temp + flush + rename) **before** the effects it
announces, so a crash at any point leaves a journal that §3/§4 can interpret.

## 5. Activity and quiescence

- Activity per [adapters-api.md](adapters-api.md) §6. `Unknown` blocks all
  live mutations (fail closed), including a possible client whose image path
  cannot be inspected (M-06, Q-16).
- Quiescence (B-02): a folder is stable only if a complete before/after
  metadata snapshot (membership, size, mtime of every entry) is unchanged
  across hashing and no entry was modified within the window (Q-15). No
  exclusive open probe (S-proc showed it disturbs the client). Stability is
  re-observed immediately before replacement (§2.2 steps 2 and 3a).
- Writes are authorized only to: AddOns top-level folder renames,
  `<variant>\.wowsync\**`, `<shared>\WoWSync\**`, and the local state
  directory. Never `WTF`, never other variants, never outside these roots.

## 6. Status and explanations (R-12)

Per package (`PackageStatus`):

| Status | Meaning |
|---|---|
| `InSync` | local package hash equals target |
| `Available` | target differs and is newer; not applied yet (with block reason if any) |
| `AppliedHere` | applied and verified in this computer's latest apply report |
| `ConfirmedBy(ids)` | peers whose own latest state confirms the target ([publication.md](publication.md) §6) |
| `NeedsDecision(reason)` | ambiguous, incompatible, partial or unvalidated; evidence listed; nothing applied for this package |
| `Unavailable` | target data not yet present locally (cloud-only/missing), or a source member's newest state unusable |
| `Interrupted` | journal unresolved, waiting for WoW to exit or for verification |
| `NeedsHelp(reason)` | unresolved journal state; both copies preserved |
| `Blocked(reason)` | game running, activity unknown, unstable |

Headline: `NeedsHelp` if any needs-help state; `Error` if provider/local
state unreadable; `Working` during publish/apply; `Waiting` if blocked,
interrupted or data not yet available; else `Ok`. Members show display name,
last admitted `seq`, "reported at" (peer clock, labeled as such) and "seen
here at" (local clock). The UI never says a peer is "current" or "up to
date" on the basis of a record time.

Audit (`wowsync.audit/1` lines in local state, rotated at 1 MiB, 2 files):
enroll, variant validation and confirmation, publish (seq) or publish
aborted, apply start / each phase transition / package result, rollback,
recovery, block reasons (deduplicated per cycle).
