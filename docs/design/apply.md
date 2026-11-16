# Apply, safety gates, recovery and status

Part of the [slice design](README.md). Decisions are in core (`plan`, `exec`,
`status`); effects are adapter calls through the core port traits
([core-api.md](core-api.md) §7).

## 1. Sync cycle (runtime, R2-slice)

Single background loop, every 3 s (so a change is noticed well within the
10-second start requirement, A-30), plus on demand from the GUI:

1. `recover_interrupted` if any journal exists (§4); stop the cycle if it
   reports needs-help.
2. Read local enrollment; open provider root; read group and members.
3. Scan AddOns → `build_inventory`. Unstable → status `Waiting(unstable)`,
   no publish/apply this cycle.
4. If the inventory differs from the last published one → publish
   ([publication.md](publication.md) §4).
5. Read members' latest states and needed trees → `compute_target`.
6. Observe activity → `plan_apply` with `ApplyGate`. `GameRunning` /
   `ActivityUnknown` → status `Waiting`, nothing written to AddOns.
7. `run_apply`; append audit entries; if anything was applied, rescan and
   publish (the `apply_report` is the confirmation signal for peers).
8. `derive_status` → GUI.

Reading AddOns and publishing while WoW is running is allowed (reads only;
Q-5). Only changes to game files are blocked (SPEC: WoW running blocks
game-file changes; an idle launcher does not).

## 2. Apply algorithm (`run_apply`)

Preconditions checked by `plan_apply`: confirmed variant, stable inventory,
`Activity::NotRunning`. Then `fx.lock()` (install lock; failure → stop).

For each `PackageApply` in key order:

1. **Stage**: write journal phase `staging`. For each folder: `stage_folder`
   (copy each blob from the provider into `staging\<op>\<folder>`, verifying
   every file hash); then `verify_staged` (tree hash equals target).
   A missing / cloud-only blob → package `Skipped(NotAvailableYet)`, staging
   discarded, continue with next package. Hash mismatch → `Failed`.
2. **Gate**: `activity()` must be `NotRunning`, else stop the whole apply
   (`stopped = GameRunning`), discard staging, nothing changed.
3. **Commit**: journal phase `committing` (written before any live change).
   For each folder in name order:
   a. `activity()` again; if not `NotRunning` → **roll back this package**
      (§3) and stop all further packages (Q-4).
   b. If live exists: `move_live_to_recovery`.
   c. `move_staged_to_live`. Any rename error → roll back package, `Failed`.
4. Journal phase `committed`; `write_recovery_point` (replaced folders with
   their previous trees, installed folders with target trees).
5. **Post-check**: `activity()`; if the client appeared during the commit the
   result is `NeedsHelp(LaunchDuringApply)`: recovery point kept, nothing
   rolled back automatically (files may be open), reported as a residual
   race (PLAN §6.7). Otherwise `hash_live_folder` for each folder must equal
   the target; mismatch → `NeedsHelp(HashMismatch)` (recovery kept).
6. Journal phase `finalized`; `prune_recovery` keeping the newest **2**
   recovery points for the package (= current live + two previous states,
   SPEC three states); remove journal; discard staging.

A package never partially replaces its folders without a journal that can
restore them. Folders of the package present locally but absent from the
target are untouched (Q-2). Nothing in `AddOns` is ever deleted; replaced
folders move to recovery by rename on the same volume.

## 3. Roll back a package

Using the journal entry, for each folder in reverse order:

| Observed (`exists`) | Action |
|---|---|
| staging only (not yet moved) | nothing |
| live is new, recovery has old | `move_live_to_staging`, `move_recovery_to_live` |
| live is new, no old (install) | `move_live_to_staging` |
| live absent, recovery has old | `move_recovery_to_live` |
| live old, no recovery | nothing |

Rollback runs only with `Activity::NotRunning`, except in step 3a where the
client was just detected: rollback renames are attempted immediately (at
most one folder was changed since the last check); a rename failure →
`NeedsHelp(RollbackBlocked)` with the journal kept for crash recovery.
Package result `RolledBack(RolledBackLaunch)`.

## 4. Crash recovery (`recover_interrupted`)

At startup and at the start of each cycle, for every journal:

- phase `staging` → discard staging, remove journal.
- phase `committing` → requires `NotRunning` (else `Waiting`, retry next
  cycle); roll back each package per §3 using `exists` observations; record
  `RolledBack(InterruptedApply)` in audit; remove journal.
- phase `committed` → keep the new live state; write recovery point if
  missing; finalize as §2.6.
- phase `finalized` / `rolled_back` → remove journal.
- unparseable journal or an observation not covered by §3 →
  `NeedsHelp(UnknownInterruptedState)`; no further apply until the user
  acts (GUI recovery is out of slice; the condition is shown and preserved).

## 5. Activity and quiescence

- Activity per [adapters-api.md](adapters-api.md) §6. `Unknown` blocks all
  writes to AddOns (fail closed).
- Inaccessible processes are not treated as running (limitation, Q-16);
  shown in diagnostics with the count.
- Quiescence: files modified within 2 s or changed during hashing make the
  folder `Unstable` (Q-15). No exclusive open probe.
- Writes are authorized only to: AddOns top-level folder renames,
  `<variant>\.wowsync\**`, `<shared>\WoWSync\**`, and the local state
  directory. Never `WTF`, never other variants, never outside these roots.

## 6. Status and explanations (R-12)

Per package (`PackageStatus`):

| Status | Meaning |
|---|---|
| `InSync` | local package hash equals target |
| `Available` | target differs and is newer; not applied yet (with block reason if any) |
| `AppliedHere` | applied in this computer's latest apply report |
| `ConfirmedBy(ids)` | peers whose own latest state confirms the target (publication.md §5) |
| `NeedsDecision` | ambiguous candidates; evidence listed; nothing applied for this package |
| `Unavailable` | target data not yet present locally (cloud-only/missing) |
| `Blocked(reason)` | game running, activity unknown, unstable, needs help |

Headline: `NeedsHelp` if any needs-help/unknown-interrupted state; `Error`
if provider/local state unreadable; `Working` during publish/apply;
`Waiting` if blocked or data not yet available; else `Ok`. Members show
display name, last `seq`, "reported at" (peer clock, labeled as the peer's
clock) and "seen here at" (local clock). The UI never says a peer is
"current" or "up to date" on the basis of a record time.

Audit (`wowsync.audit/1` lines in local state, rotated at 1 MiB, 2 files):
enroll, variant confirmed, publish (seq), apply start/package result,
rollback, recovery, block reasons (deduplicated per cycle).
