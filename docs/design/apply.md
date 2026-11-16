# Apply, safety gates, recovery and status

Part of the [slice design](README.md). Decisions are in core (`admit`,
`plan`, `exec`, `status`); effects are adapter calls through the core port
traits ([core-api.md](core-api.md) §7). Revised in design retry 1 for review
findings B-01..B-03, M-02..M-06 and in retry 2 for N-01/N-03 (see
[R2-design-fix evidence](../runs/run-002/evidence/R2-design-fix.md)).

## 1. Sync cycle (runtime, R2-slice)

Single background loop, every 3 s (a change is noticed well within the
10-second start requirement, A-30), plus on demand from the GUI:

1. If any journal exists: `recover_interrupted` (§4). If any journal
   remains afterwards, no new apply starts this cycle (publish still may; Q-25).
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

Revised in design retry 2 (N-01, N-03): **one package per operation**
(`OpId`), and recovery decides from **observed slot contents**, not from a
per-folder progress field. This removes all multi-package journal,
staging and cleanup coupling and makes every step re-entrant.

Preconditions checked by `plan_apply`: `ConfirmedVariant` (synthetic install
in the slice, [domain.md](domain.md) §2.1), the package's folders stable in
the current scan, every plan precondition derivable (§2.1),
`Activity::NotRunning`, **no journal of any operation present** (Q-25).
Then `fx.lock()` (app install lock; it excludes other WoW Sync instances
only, not CurseForge or other writers — those are handled by §2.1/§2.2).

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

### 2.2 One operation = one package

`run_apply` processes the plan's `PackageApply` entries in key order; each
has its **own** `OpId`, journal file, staging directory and recovery
directory. Nothing in one operation refers to another. `G` = the **gate**:
call `activity()`; continue only on `NotRunning`.

Shorthand, all on `ApplyEffects` ([core-api.md](core-api.md) §7): for folder
`f` of op `o` and package `p`: `L` = `Slot::Live{f}`, `S` =
`Slot::Staging{o,f}`, `R` = `Slot::Recovery{p,o,f}`; `present(X)` =
`observe(X).is_some()`; `content(X)` = `tree_of(X)`; `move(X→Y)` =
`rename(X, Y)` (fails if `Y` exists). `O` = the folder's old tree
(`expected_live`), `N` = the target tree.

1. **Journal `staging`** written (temp + flush + rename) **before** creating
   the staging directory. For each folder: `stage_folder` (copy each blob
   from the provider, verify each file hash), then `content(S) == N`.
   Missing / cloud-only blob → `Skipped(NotAvailableYet)`; hash mismatch →
   `Failed`. Both: journal → `rolled_back` (`from_phase: staging`), then
   cleanup (§2.5).
2. **Destination revalidation.** For every folder: `observe(L)` matches
   `expected_live` (absent, or same fingerprint **and** `content(L) == O`,
   no entry newer than the quiescence window, Q-15). Any difference →
   `Replanned(DestinationChanged)`: journal → `rolled_back`
   (`from_phase: staging`), cleanup (§2.5);
   nothing live touched.
3. **Recovery point record** written with `pinned: true`
   (`write_recovery_point`), then **journal `committing`**. Both are written
   before any live change.
4. **Commit.** For each folder in name order:
   a. If `O` is a tree: `G`; `observe(L).fingerprint` still equals the step-2
      value (else stop, §2.3); `move(L→R)`.
   b. `G`; `move(S→L)`.
   Any `G` failure, fingerprint mismatch or rename error → §2.3.
5. **Journal `committed`.**
6. **Verify.** `G`; then for every folder the configuration is `Done` (§3.1)
   with `content(L) == N` and, if `O` is a tree, `content(R) == O`. All true →
   **journal `verified`** → step 7. Otherwise → journal `needs_help` with the
   reason (`LiveMismatch` / `RecoveryMismatch`); nothing moved; package
   result `NeedsHelp`. If `G` fails the journal stays `committed`
   (`Waiting`); verification resumes in a later cycle (§4).
7. **Finalize** (only from `verified`, §2.5): recovery point `pinned: false`;
   prune; `discard_staging(o)` (empty); `remove_journal(o)`. Result
   `Applied`. Only `Applied` packages appear as `applied` in the next
   `apply_report`.

Folders of the package present locally but absent from the target cannot
occur in a plan (complete-package rule, [domain.md](domain.md) §6). Nothing
in `AddOns` is deleted; replaced folders move to recovery by same-volume
rename.

### 2.3 Interruption (B-01)

If `G` returns `Running`/`Unknown`, a fingerprint check fails, or a rename
fails after the `committing` journal was written:

- **No further live mutation** in this run — no rollback, no completion.
  `run_apply` returns immediately with `stopped = GameRunning |
  ActivityUnknown | DestinationChanged | RenameFailed` and the package result
  `Interrupted`; later packages of the plan are not started.
- The journal (phase `committing`) and all slots are left as they are.
- Status `Waiting` for game activity ("WoW started while <addon> was being
  updated; it will be restored to its previous version after WoW exits"),
  `NeedsHelp` only if the next recovery cannot classify the slots (§3.1).
- Recovery happens in a later cycle via §4 after a fresh `NotRunning`.

Residual window (honestly reported, PLAN §6.7): WoW (or an external writer)
may start between a `NotRunning` observation and the very next rename; an
external writer may modify a folder between its fingerprint check and its
rename. Step 6 detects the latter for replaced folders.

### 2.4 Retention (B-03, SPEC three states)

- Normal pruning (only in finalize): per package keep the newest **2**
  unpinned recovery points (current live + two previous states).
- Never pruned: any recovery point with `pinned: true`, and any point whose
  op has a journal. These are visible temporary exceptions in status ("kept
  for recovery: N extra copies", A-17), so more than two can exist.

### 2.5 Cleanup is per operation and idempotent (N-03)

Cleanup touches only the op's own directories and only from a terminal
phase, journal last:

| From phase | Steps, each skipped if already done |
|---|---|
| `verified` (finalize) | set recovery point `pinned: false` (rewrite is idempotent); prune other unpinned points of the package beyond 2; `discard_staging(o)`; `remove_journal(o)` |
| `rolled_back` with `from_phase: staging` | `discard_staging(o)` unconditionally (no live folder was ever moved into `S`; it holds only provider copies, possibly partial); `remove_recovery(p, o)` (holds at most the record); `remove_journal(o)` |
| `rolled_back` with `from_phase: committing` | `discard_staging(o)` **only if** every `S` is absent or `content(S) == N` (provider-reproducible bytes); else journal → `needs_help(UnexpectedStaging)`; `remove_recovery(p, o)` **only if** no `R` of the op is present (it then holds only the record); `remove_journal(o)` |

`needs_help` has no cleanup: the journal, staging and recovery of that op
are kept as evidence until a later GUI resolution (out of slice, Q-25).
The journal is written before the op's staging or recovery directory is
created, so neither exists without a journal; any such directory found
(external interference) is reported in diagnostics and left in place.

## 3. Slot reconciliation (N-01)

### 3.1 Configurations

Atomic same-volume renames move a whole folder; the slice never edits a
folder in place. For one folder, the only moves WoW Sync makes are
`L→R`, `S→L` (commit) and `L→S`, `R→L` (rollback). Classify by presence
`(L, S, R)`:

Replace (`O` is a tree):

| Config | `L` | `S` | `R` | Meaning |
|---|---|---|---|---|
| `Untouched` | any | present | absent | we have not moved this folder (or rolled it back fully); `L` is whatever is there |
| `Half` | absent | present | present | old moved out, new not yet in (commit step 4a done, or rollback step b pending) |
| `Done` | present | absent | present | new in, old in recovery |
| `Unknown` | any other combination | | | e.g. both `S` and `R` absent, or all three present |

Install (`O` absent; `R` is never used):

| Config | `L` | `S` | Meaning |
|---|---|---|---|
| `Untouched` | any | present | not moved |
| `Done` | present | absent | new in |
| `Unknown` | absent | absent | — |

Content checks are applied where the next move would **remove** something
from live: before `L→S` in rollback, `content(L) == N` (otherwise an
external writer changed the new folder → `needs_help(LiveChanged)`, nothing
moved). After a restoring `R→L`, `content(L) == O` (else
`needs_help(RestoreMismatch)`; the bytes are live, the journal is kept).

### 3.2 Rollback (phase `committing`)

For each folder in **reverse** name order, re-observing every time:

| Config | Action |
|---|---|
| `Untouched` | none |
| `Half` | `G`; `move(R→L)`; check `content(L) == O` |
| `Done` (replace) | check `content(L) == N`; `G`; `move(L→S)` → now `Half`; `G`; `move(R→L)`; check |
| `Done` (install) | check `content(L) == N`; `G`; `move(L→S)` → `Untouched` |
| `Unknown` | journal → `needs_help(UnknownInterruptedState)`; stop this op |

When every folder is `Untouched`: journal → `rolled_back`
(`from_phase: committing`), then cleanup
(§2.5). Any `G` failure → stop, journal unchanged (`Waiting`).

### 3.3 Completion check (phase `committed`)

Every folder must be `Done` with the content checks of §2.2 step 6;
otherwise `needs_help`. No rename is made in this phase.

## 4. Crash and interruption recovery (`recover_interrupted`)

Runs at the start of every cycle, for each journal (one op = one package)
in op order, independently. It re-observes the slots every time and holds
no state between calls.

| Journal phase | Action |
|---|---|
| unparseable | `needs_help(UnknownInterruptedState)` shown; nothing touched |
| `staging` | journal → `rolled_back`; cleanup (§2.5). No live slot was touched in this phase |
| `committing` | `G`; if not `NotRunning` → `Waiting`. Else rollback §3.2 |
| `committed` | `G`; if not `NotRunning` → `Waiting`. Else §3.3 → `verified` → finalize, or `needs_help` |
| `verified` | finalize (§2.5) |
| `rolled_back` | cleanup (§2.5) |
| `needs_help` | leave as is; report; recovery point stays pinned |

While any journal exists, no new apply is planned (Q-25); publish still runs.

### 4.1 Idempotency argument (N-01)

Write order per op, with the crash points between effects (`†`):

```text
J:staging † stage S(f1) † … † [revalidate, reads only] † RP(pinned) † J:committing
† f1: L→R † S→L † f2: L→R † S→L † … † J:committed † [verify, reads only]
† J:verified † RP(unpinned) † prune † discard S † remove J
```

Journal and record writes are temp + flush + rename, so each is atomic:
after a crash the journal shows the old or the new phase, never a partial
file. Each rename is atomic. Hence after any crash:

1. **Before `J:staging` lands**: no journal, no staging directory
   (it is created only after), nothing to do.
2. **Phase `staging`** (crash anywhere up to and including the recovery
   point write): live untouched (all live writes come after `J:committing`).
   Recovery: journal → `rolled_back` with `from_phase: staging` →
   cleanup, which discards staging unconditionally (it may be partial, and
   in this phase no live folder was ever moved into `S`). `from_phase` is
   in the journal so a crash during cleanup keeps the distinction.
3. **Phase `committing`** (crash after `J:committing`, at any rename
   boundary): every folder is in `Untouched`, `Half` or `Done` by
   construction (the sequence `Untouched →(L→R) Half →(S→L) Done` for
   replace, `Untouched →(S→L) Done` for install). Rollback (§3.2) moves
   each folder along the same chain backwards (`Done →(L→S) Half →(R→L)
   Untouched`), so a crash **during rollback** also leaves only these
   configurations, and the next run applies §3.2 again from whatever it
   observes. Each rollback action is chosen from the observation, not from
   memory, so re-running it after a crash or after a `G` stop never repeats
   a completed move and never skips one.
4. **Crash between the last `S→L` and `J:committed`**: phase `committing`,
   all folders `Done` → rolled back (safe; the update is retried next
   cycle). Accepted cost.
5. **Phase `committed`**: all folders `Done` (or an external change →
   `needs_help`). §3.3 makes no moves, so it can run any number of times.
6. **Phase `verified`**: finalize steps are each "skip if done": rewriting
   `pinned:false`; pruning removes only unpinned points beyond 2 (repeatable);
   discarding an absent staging dir is a no-op; journal removal is last, so
   a crash before it re-runs finalize.
7. **Phase `rolled_back`**: cleanup steps are each "skip if done"; journal
   removal last.
8. **Phase `needs_help`**: no action; repeatable trivially.

Game activity: every live rename in §2.2/§3.2 is preceded by `G`; a stop
leaves one of the configurations above, so pausing is equivalent to a crash
at that point. The required tests sweep a crash after every effect of
apply **and** after every effect of recovery (including a second crash
during recovery), with both fakes and real adapters ([tests.md](tests.md)
§3, §5).
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
