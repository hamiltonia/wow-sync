# Rollback and rejected releases

Status: **run-004 design, revised by packets
[R4-design-fix](../runs/run-004/packets/R4-design-fix.md) and R4-design-fix2
(after the [recheck](../runs/run-004/evidence/R4-design-recheck.md)).** It
fixes R4D-9, R4D-20 (folder-changing rollback), the rollback part of
R4D-21 (per-folder constraints) and the rollback parts of R4D-1, R4D-4 and
R4D-10 of the
[R4-design-review](../runs/run-004/evidence/R4-design-review.md). **Not
implemented.** Nothing here changes the running slice: its recovery points
stay local (Q-18) and its target has no rejection step. Choices marked
**PENDING OWNER** use the conservative default in
[questions.md](questions.md). They are not owner decisions.

**Contract.** [contracts.md](contracts.md) alone defines:
- the record envelope, identity (`sha256` of the exact bytes), references,
  `Known` coverage, validity, learning, mirroring and capabilities (§1);
- the apply journal and recovery point, including the `rollback` kind and
  the `paired` field (§2);
- folder-keyed addon identity (§3);
- target precedence and constraint composition (§4);
- the code-choice family and its decision reasons (§5).

This file defines only the bodies of the `rejection` and `lift` kinds, the
recovered-target constraint and the rollback behaviour. Where it seems to
differ from contracts.md, contracts.md wins. Phase C: the shared parts are
built by **R4-core-contracts**, the rollback behaviour by
**R4-core-settings** (contracts.md §9, this file §9).

Product authority: SPEC R-10, R-11 and R-12 ([SPEC.md](../../SPEC.md)).
Acceptance: A-16 to A-20, A-23, A-25, A-26 and A-45
([ACCEPTANCE.md](../../ACCEPTANCE.md)). Settings parts consume
[settings.md](settings.md); removal parts consume [removal.md](removal.md).
Open choices are **(Q-n)** in [questions.md](questions.md).

## 0. Vocabulary

| Term | Meaning |
|---|---|
| release | one exact content of a set of addon folders: the package hash over its folder trees ([publication.md](publication.md) §3), keyed by **folder keys** (contracts.md §3.1). A version label ("2.1") is display text, not identity |
| rollback | the user action "this release is bad, go back to an earlier one" for one addon in one group |
| rejection | the group-visible record that a release must not be installed in the group (the "tombstone" of a release) |
| recovered target | the earlier release that the rollback installs and distributes |
| lift | the explicit user action "allow this release again", recorded as its own record |
| paired restore | a rollback that also restores, per settings unit, the settings saved with the recovered target |
| code-only rollback | a rollback where some or all units have no such settings; current settings stay and a warning follows |

## 1. What rollback does (R-10, A-18)

On one computer, for the addon with folder keys F, from the Add-on detail
history:

1. The user picks an earlier history state R0 (current plus two previous,
   [apply.md](apply.md) §2.4) and chooses "Go back to this version…". The
   option is shown only if the group-wide preconditions hold (§2.3). R0
   may have different folder keys from the current release R1 (an
   ordinary release that added or dropped a folder, R4D-20). Let F0 and F1
   be their folder keys; F = F0 ∪ F1.
2. The confirmation names R1 and R0 and the scope: "Version 2.1 will be
   blocked on all your computers for <variant>. Version 2.0 will be
   installed everywhere." If the folders differ it also says "Details_Extra
   (added in 2.1) will be removed and kept in recovery" or "Details_Old
   (removed in 2.1) will be put back". It also says which settings will be
   restored (§4).
3. On confirmation the app reserves its next seq and builds, all in one
   `rollback` journal's `pending_records` (contracts.md §2.2):
   - a **rejection** of R1 (§2.1) with `folders` = F;
   - if F1 \ F0 ≠ ∅: a **removal** with `evidence: rollback`, `rejection`
     = that rejection's hash, naming F1 \ F0 with R1's trees, plus this
     computer's own `removed` ack for it ([removal.md](removal.md) §2.3);
   - if F0 \ F1 ≠ ∅: a **re-add** with `evidence: rollback`, naming
     F0 \ F1 with R0's trees, whose `known` covers the folder-drop
     removals that removed them;
   - per settings unit to restore, a settings `restore` revision (§4).
4. It installs R0 with that one `rollback` operation (`apply-journal/2`):
   `replace` for F0 ∩ F1, `install` for F0 \ F1, `remove` for F1 \ F0
   (listing the new removal in `removals`). The WoW-not-running gate, the
   journal and the game check before every live rename apply as for any
   apply. R1 moves into a `/2` recovery point with the derived `paired`
   value (contracts.md §2.3). R0 comes from the local recovery point, or
   from provider trees if some member's state or a §7 upload still has it.
   The records are written only after verify; if the op rolls back, none
   is written.
5. It publishes its code state, whose authority index lists the new
   records, and its settings state.
6. Every other computer learns the records (contracts.md §1.12),
   computes target R0 per folder (§3): F1 \ F0 removed by the removal,
   F0 \ F1 added by the re-add, F0 ∩ F1 constrained to R0's trees, and
   applies it when WoW is not running there.

Rollback never touches other folders, and never another variant: records
live in the group folder, and one group is one variant (A-18). A
folder-changing rollback also needs `removal/1` on every non-retired
member (Q-84 PENDING OWNER default, §2.3).

## 2. Records

Both kinds use the contract envelope, identity, references, size limit and
validity (contracts.md §1.3, §1.4, §1.11). Neither has an `id` field.

### 2.1 Rejection (`wowsync.rejection/1`)

```json
{
  "schema": "wowsync.rejection/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 60 },
  "folders": ["details", "details_extra"],
  "rejected": { "package_hash": "<sha256>", "version_label": "2.1",
                "folders": [ { "key": "details", "tree": "<sha256>" },
                             { "key": "details_extra", "tree": "<sha256>" } ] },
  "recovered": { "package_hash": "<sha256>",
                 "folders": [ { "key": "details", "tree": "<sha256>" } ] },
  "considered": [ { "package_hash": "<sha256>",
                    "folders": [ { "key": "details", "tree": "<sha256>" } ],
                    "holders": ["<enrollment id>"] } ],
  "settings": "paired|partial|code_only|not_applicable",
  "known": { "seen": [], "extra": [], "parts": [] },
  "considered_parts": [],
  "written_unix_ms": 1767225600000
}
```

- `folders`: F = the union of R1's and R0's folder keys. It is the scope
  for coverage and constraint composition, per folder (contracts.md §1.8,
  §4.2).
- `rejected`: the exact content rejected (Q-78 PENDING OWNER). Its
  `version_label` is the untrusted display string allowed by contracts.md
  §1.4. It is used for display and for the look-alike hold only (§3.3).
  Level 2 uses contracts.md §4.2 `matches` against it.
- `recovered`: the exact content the user chose (§3.1), possibly with
  fewer or other folders than `rejected`.
- `considered`: every candidate for `folders` that the issuer knew at the
  time, with its folder trees (same item type as the code-choice
  `considered`, contracts.md §5.2; partable). Used by the newer-release
  notice (§3.1).
- `settings`: the outcome of §4, shown in history and audit. The
  restore revisions are ordinary settings revisions and are not referenced.
- `known`: what the issuer had learned (contracts.md §1.8). It decides
  which earlier constraints this rollback covers (§3.2).
- There is no free-text note (contracts.md §1.4).

**V-REJECTION** (V-KIND):
1. `folders` non-empty, sorted, unique, ≤ 256.
2. The union of the folder keys of `rejected.folders` and
   `recovered.folders` equals `folders`; each list is non-empty, sorted
   and unique.
3. Each `package_hash` is the package hash of its folder trees, and
   `rejected.package_hash ≠ recovered.package_hash`.
4. `considered` (inline plus parts, contracts.md §1.9) is sorted, unique
   by package hash, and contains entries matching `rejected` and
   `recovered`.
5. `version_label` ≤ 128 bytes of valid UTF-8.

### 2.2 Lift (`wowsync.rejection-lift/1`)

```json
{
  "schema": "wowsync.rejection-lift/1",
  "group_id": "…",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 75 },
  "folders": ["details"],
  "lifts": [ "<rejection hash>" ],
  "known": { "seen": [], "extra": [], "parts": [] },
  "written_unix_ms": 1767225600000
}
```

**V-LIFT** (V-KIND): `lifts` non-empty, ≤ 64, sorted, unique; V-REF with
kind `rejection` for each; `folders` is the sorted union of the named
rejections' `folders`.

- A lift names exact rejection records (plain hashes). It never lifts a
  rejection it does not name, so a concurrent new rejection of the same
  content stays in force (Q-79).
- Two lifts naming the same rejection are harmless: the rejection is
  lifted once.
- A lift ends only the named rejections and their recovered-target
  constraints. It affects no removal, choice or settings record. In
  particular it does not end a rollback-evidence removal (removal.md
  §2.3).
- **"Allow again…" after a folder-changing rollback** writes the lift and,
  in the same act, a re-add (`user_add_back`) of R1's added folders
  F1 \ F0 whose `known` covers the rollback removal, so R1 can be complete
  again. If R1 then wins ordinary selection while some of its folders are
  still missing on a computer, that computer gets `PartialCandidate`
  (contracts.md §5.1).
- "Allow again…" is the only GUI action that writes a lift. Any
  non-retired member may write one (Q-79).

### 2.3 Lifetime and group-wide preconditions (Q-84)

- A rejection is **active** iff it is `valid` (contracts.md §1.11) and no
  valid lift names it. Records are grow-only and mirrored (contracts.md
  §1.2, §1.12), so a rejection survives its issuer's folder becoming
  unusable and its issuer's later retirement (if it was issued at or below
  the cutoff, contracts.md §1.10).
- **Q-84 PENDING OWNER default: block group rollback** when any
  non-retired member's newest admitted code state does not list
  `rejection/1` in `features` (contracts.md §1.13), and, for a
  folder-changing rollback (§1), also when one lacks `removal/1`.
  "Go back to this version…" is then disabled with "Laptop runs an older
  WoW Sync that can't block versions. Update WoW Sync on Laptop first."
  There is no "this computer only" variant: ordinary selection would
  reinstall R1 at the next cycle.
- A member that is downgraded after a rollback publishes states without
  `rejection/1`. It may reinstall R1 locally, because it ignores
  rejections. It is shown as "runs an older WoW Sync that does not apply
  blocked versions", never counts as confirmed for R0, and every
  up-to-date member still drops R1 (§3). Nothing is installed from it.

## 3. Target

Rollback fills two levels of `target::compose` (contracts.md §4.1):

- **Level 2, rejection.** A candidate that `matches` (contracts.md §4.2)
  `rejected` of an active rejection is never a candidate, never a source
  and never kept as a target for the common folders. A computer whose
  installed copy is rejected is never a source of it.
- **Level 6, recovered target**, composed per folder with code choices by
  contracts.md §4.2 and defined below.

### 3.1 The recovered-target constraint

Rejecting R1 alone would let ordinary selection pick another release newer
than R0 that the user saw and did not choose (for example 2.1.5 on a third
computer). So each active rejection X adds, **for each folder f of
`X.recovered.folders`**, the constraint "install `X.recovered`'s tree for
f". Folders of `X.folders` that R0 lacks (F1 \ F0) carry no tree; they are
removed by the rollback removal (§1) instead.

- **Live constraints, per folder.** X's constraint is live for f unless
  another valid code choice, rejection or re-add that names f covers X
  through its `known` (contracts.md §4.2). A record covering X for some
  folders leaves X live for the others (R4D-21). A resume therefore ends
  the recovered target for its folders but not the rejection itself.
- **Recovered chain.** If `X.recovered` is itself rejected for f by active
  rejections Y1…Yn, X's constraint for f stands for their recovered trees
  instead, repeated with a visited set (no cycles). Rejected contents are
  dropped. If nothing remains, X imposes no constraint on f and ordinary
  selection among non-rejected candidates applies.
- **Composition.** The resulting trees of all live constraints for f are
  composed by contracts.md §4.2: one tree installs it; several
  different trees give `NeedsDecision(RollbackTargetsDiffer)`.
- **Unavailable target.** If the constrained content has no source (no
  local recovery point, no member's admitted state and no §7 upload), the
  folders get `NeedsDecision(RecoveredUnavailable)` (contracts.md §5.1):
  "Version 2.0 is no longer available on any computer." Options: one "Use
  <version>" per available non-rejected candidate (a code choice whose
  `known` covers X), or "Decide later". Meanwhile nothing is changed on
  disk. A rejected copy that is still installed stays installed with the
  status "Blocked version still installed", and is never a source.
- **Newer releases (Q-81 PENDING OWNER default).** Only an explicit lift
  ("Allow again…") or an explicit code choice ends the constraint. A
  non-rejected candidate **not** in `X.considered` that is demonstrably
  newer than the recovered target (domain.md §6) does not install. It
  raises the non-blocking notice `NewerAfterRollback` (contracts.md §5.1):
  "Details 2.2 is available. You went back to 2.0 on <date>." Options:
  "Use 2.2" (a keep whose `known` covers X) or "Stay on 2.0" (a local
  dismissal of that candidate only). Candidates that match a `considered`
  entry never raise the notice.

### 3.2 Concurrent, late and conflicting records (R4D-9)

Validity and coverage are pure functions of the ledger (contracts.md
§1.11), so every computer with the same ledger reaches the same target in
any arrival order.

| Situation | Result |
|---|---|
| A and B both reject R1 and both recover R0, concurrently | two rejections, one constrained content R0. Target R0 |
| A rejects R1 → R0, B rejects R1 → R0′, concurrently | `NeedsDecision(RollbackTargetsDiffer)` with "Use 2.0" and "Use 2.0.1". R1 is rejected and spreads nowhere. Copies stay as they are until the choice |
| A rejects R2 → R1, B concurrently rejects R1 → R0 | R2 and R1 rejected; A's target follows the chain to R0. Target R0 |
| A rejects R1 → R0, B concurrently rejects R0 → R1 | both rejected, the chain is empty. Ordinary selection among non-rejected candidates. If none exists: "The only copies of Details are blocked versions" with "Allow again…" |
| Y (whose `known` covers X) delivered before X | X's constraint is covered by Y as soon as both are known, so Y's target wins. Before X arrives, Y's target already applies. X's rejection is still active at level 2 |
| Recovered target unavailable | `RecoveredUnavailable` (§3.1) |
| Two lifts of the same rejection, concurrently | one lift effect |
| A lifts X while B writes a new rejection X′ of the same content | X′ is not named by the lift and stays active (Q-79) |
| Rollback on A, removal of the same folders on B, concurrently | removal (level 4) beats the constraint (level 6): the folders are removed into recovery everywhere. The rejection stays. A later re-add ends the constraints its `known` covers; ordinary selection among non-rejected candidates follows |
| Rollback and code choice on overlapping folders | coverage decides. Concurrent ones with different contents give a decision (contracts.md §4.2). A keep of rejected content is inert (level 2) |
| Rejection issued by a computer later retired | valid iff `issued_at_seq ≤ Cutoff` (contracts.md §1.10) |

### 3.3 Offline and late peers, look-alikes (A-18, A-20)

- A peer that was offline still has R1. R1 is not a candidate (level 2).
  When it learns the rejection, its target is R0 and it replaces R1 with
  R0, keeping R1 as a blocked previous state. It cannot reintroduce R1,
  however high R1's version label is.
- A peer that publishes R1 **before** it learns the rejection still cannot
  win: every computer that has the rejection drops R1, and the peer learns
  the rejection from any member folder at its next cycle.
- **Look-alike (Q-78 PENDING OWNER default).** A candidate whose version
  label equals `rejected.version_label` of an active rejection over the
  same folders, but whose content differs, gets
  `NeedsDecision(LookalikeOfRejected)`: "This looks like the blocked
  version 2.1, but its files differ." Options: "Use this copy" (a keep),
  "Block this copy too" (a new rejection with the same recovered target),
  "Decide later". Until then the constraint stays.
- A joining computer with R1 replaces it with R0, like any member.
- A user who reinstalls R1 by hand: WoW Sync replaces it with R0, keeping
  R1 in recovery, and Activity says "Version 2.1 is blocked for this group.
  To use it again, choose Allow again."

### 3.4 Different variants

One group is one variant, so another variant's group never sees these
records (A-18). Rejecting the same release in two variants is two
rollbacks.

## 4. Settings during a code rollback (R-10, A-18, A-19)

For each settings unit of the folders F on the issuing computer
([settings.md](settings.md) §1.1):

1. Pick the settings saved with R0 by paired selection
   ([settings.md](settings.md) §4.6). Different computers' counters and
   clocks are never compared.
2. If one is found and the unit is shared ([settings.md](settings.md)
   §5.1): capture the current files first, then write a `restore` revision
   ([settings.md](settings.md) §3.3) with that checkpoint,
   `code_package_hash` = R0 and the issuer's basis set as parents. Apply
   it in the same WoW-not-running window as the code, inside the same
   `rollback` operation. Other computers receive it as a normal revision.
   A restore that competes with another intentional revision is a settings
   choice ([settings.md](settings.md) §4.4).
3. If one is found but the unit is not shared on this computer: restore it
   locally only. The summary says "Settings were restored on this computer
   only."
4. If none is found: the unit's current settings are not touched.

The recovery point's `paired` value is **derived** from its code and
settings roots (contracts.md §2.3), never set by this flow; the
rejection's `settings` field copies it: `paired` (every unit restored),
`partial` (some), `code_only` (none) or `not_applicable` (the addon has no
settings units).

**Afterwards** the app shows a summary that keeps the outcomes apart
(R-10, A-19):

- Paired: "Details is back to version 2.0, with the settings you had with
  2.0 (account ERIC, Thrall on Argent Dawn)."
- Partial or code-only, as a warning, never as success: "Details is back to
  version 2.0. Your settings were **not** restored for Thrall on Argent
  Dawn, because no copy from version 2.0 is left. Your current settings are
  kept. They were made with 2.1 and may not work with 2.0." with "Show
  settings history" and "OK".

## 5. Settings changes are not rejected (Q-82)

**Q-82 PENDING OWNER default: there is no settings-checkpoint rejection.**
"Undo this settings change" on a shared unit writes a `restore` revision of
the earlier checkpoint ([settings.md](settings.md) §3.3). If the master's
game later saves the same bytes again, its next completed session shares
them again like any capture. The unit's history explains it ("Undone on
Laptop; the master's later session changed it again"). A code rollback
does not reject settings; it restores them (§4).

## 6. Removal, re-add and code choices

| Situation | Rule |
|---|---|
| Rollback of a removed addon | not offered. Use "Undo removal" ([removal.md](removal.md)), which offers only non-rejected states |
| A removal | is not a release and cannot be rejected |
| Rejected content and removal coverage | a copy that is only rejected content counts as covered for removal; a removal is not contested by a computer that has only the rejected R1 |
| Rejections vs removal records | independent. Neither expires the other. A re-add does not lift a rejection; a lift does not affect removal |
| Re-adding after removal | ordinary selection among non-rejected candidates. If the only copies are rejected: "The only copies of Details are version 2.1, which is blocked." with "Allow again…" |
| Code choices | contracts.md §5. A keep of rejected content is inert. "Resume automatic updates" ends recovered-target constraints it covers, never a rejection |
| Restore from history | rejected states are offered only behind "Allow again…", which writes a lift first |

## 7. Three-state history (R-10, A-16, A-17)

- A rejected release stays in its computer's code history as an ordinary
  previous state, labelled "Blocked", and counts toward current plus two
  previous. It is pruned like any other state. Its rejection record stays,
  so pruning the files never lifts it.
- R0's recovery point becomes the current state. Settings paired with a
  retained recovery point stay pinned with it ([settings.md](settings.md)
  §7).
- If R0's local recovery point is the only copy anywhere and some
  non-retired member has not yet confirmed R0, R0's trees and blobs are
  uploaded and kept in the provider until every non-retired member
  confirms R0 or a newer non-rejected release. The GUI shows this as a
  temporary exception with its size (A-17). This is the only case where a
  recovery point's content is uploaded (Q-18 otherwise stands).

## 8. GUI recovery actions (R-12, R-13, A-25, A-26)

| Screen ([ui/screens.md](ui/screens.md)) | Action / view |
|---|---|
| Add-on detail, history | "Go back to this version…" on each non-current, non-blocked state (disabled with the reason when §1 or §2.3 does not hold); "Allow again…" on blocked ones; settings history with "Undo this settings change…" and "Restore" |
| Rollback confirmation | current and recovered versions, the group scope, which units get paired settings and which stay as they are |
| Rollback summary | paired / partial / code-only as in §4, warning style for partial and code-only |
| Add-ons list | "Blocked: 2.1" chip while a rejection is active |
| Activity | "You went back to Details 2.0 and blocked 2.1", "Laptop replaced blocked Details 2.1 with 2.0", lifts |
| Needs attention | `RollbackTargetsDiffer`, `RecoveredUnavailable`, `LookalikeOfRejected`; only blocked copies on re-add; group rollback blocked by an older member (§2.3) |
| Notices | `NewerAfterRollback` (§3.1) |

All of these are GUI flows; none needs a terminal (A-26).

## 9. Implementation map

File ownership and build order are [contracts.md](contracts.md) §9 only.

**Built by step 1, R4-core-contracts:** the rejection and lift DTOs with
V-REJECTION and V-LIFT (`kinds.rs`); the `rollback` journal kind with
mixed replace/install/remove actions and `pending_records` (`records.rs`,
`exec.rs`); the level-2 and level-6 hook calls in `target::compose` and
the `RollbackTargetsDiffer`, `RecoveredUnavailable`, `LookalikeOfRejected`
and `NewerAfterRollback` reasons (`target.rs`); `PackageStatus::Blocked`,
`NextAction::{AllowAgain, GoBack}` and the rollback explanations
(`status.rs`, `explain.rs`, `view.rs`); the `Rollback { folders, to }`,
`AllowAgain { rejections }` and `UndoSettingsChange { unit, revision }`
control commands (`control.rs`); the §7 provider upload IO
(`provider.rs`).

**Built by step 2, R4-core-settings** (with the settings files):

| File | Contents |
|---|---|
| core `reject.rs`, `reject_tests.rs` | `ConstraintHooks::rejected` and the rejection half of `live_constraints`: `active_rejections`, per-folder recovered constraints with the chain and coverage, `lookalike`, `newer_after_rollback`; `plan_rollback(R1, R0) -> PackageApply` building the rejection, rollback removal/re-add and own ack in `pending_records` (§1); the allow-again lift and re-add (§2.2) |
| runtime `rollback_flow.rs` | the `Rollback` and `AllowAgain` command bodies, paired settings selection through `settings.rs` (§4), the §7 upload trigger |

GUI screens of §8 are R4-app's. The decision reasons of §3 belong to the
code-choice family (contracts.md §5.1); this packet produces the items.

## 10. Test obligations

| ID | Test | A-ID |
|---|---|---|
| T-R01 | rollback on A with paired settings: B and C install R0 and the restored settings; R1 is a blocked previous state everywhere | A-18 |
| T-R02 Neg | offline C returns with R1 and a higher version label: C never offers R1 as a source; target stays R0; C replaces R1 | A-18, A-20 |
| T-R03 Neg | C publishes R1 before learning the rejection: no other computer installs R1 | A-18, A-20 |
| T-R04 Neg | another variant's group with the same R1 is unaffected | A-18 |
| T-R05 | the issuer's folder deleted after mirroring: rejection still active | A-20 |
| T-R06 | 2.1.5 known at the rollback is never chosen; a later unknown 2.2 raises `NewerAfterRollback` and does **not** install; "Use 2.2" installs it; "Stay on 2.0" keeps R0 (Q-81 default) | A-18 |
| T-R07 Neg | same label, different content: `LookalikeOfRejected`, not installed; "Block this copy too" writes a rejection | A-18, A-23 |
| T-R08 | no paired settings: code-only rollback, current settings byte-identical, warning differs from the paired summary | A-19 |
| T-R09 | paired for one unit, missing for another: partial warning lists the missing unit; recovery point `paired = partial` | A-19 |
| T-R10 | lift then restore R1; a concurrent new rejection not named by the lift stays active | A-18, A-20 |
| T-R11 | a removed addon offers "Undo removal" with only non-rejected states; a removal cannot be rejected | A-18 |
| T-R12 | rejections and removal records are independent; a re-add does not lift | A-20 |
| T-R13 | history after rollback: current plus two previous, blocked state labelled, paired settings pinned while their recovery point exists | A-16, A-17 |
| T-R14 | "Undo this settings change" writes a `restore` revision and no rejection; the master's later identical capture is shared and explained (Q-82 default) | A-18 |
| T-R15 Neg | rejection issued after its issuer's retirement cutoff: invalid; one issued before it stays valid | A-45 |
| T-R16 Neg | malformed rejection, `version_label` over 128 bytes, a label with markup, colour codes or Lua: refused, or shown as plain text; `rejected` ∪ `recovered` keys ≠ `folders`: invalid | A-23 |
| T-R17 | WoW starts during a rollback apply: stops before the next rename; no process killed | A-21, A-22 |
| T-R18 (GUI) | go back, summary (paired and code-only), allow again, undo settings change, the three decisions, all without a terminal | A-25, A-26 |
| T-R19 Neg | Q-84 default: a non-retired member without `rejection/1` (or, for a folder-changing rollback, without `removal/1`) disables group rollback with the named reason; no record is written; a downgraded member after a rollback is never counted as confirmed | A-18 |
| **T-R4D20-rollback-added-folder** | R1 = {details, details_extra} (2.1 added a folder), R0 = {details}; roll back on A; B and C hold R1; crash at every slot of the `rollback` journal | one journal: replace `details`, remove `details_extra`; after verify the rejection, a `rollback` removal of `details_extra` and A's own ack are written from `pending_records`, none if the op rolled back; B and C end with R0 and `details_extra` in recovery; "Allow again…" writes a lift plus a re-add of `details_extra`, and R1 becomes eligible and complete | A-18, A-20, A-22 |
| **T-R4D20-rollback-dropped-folder** | R1 = {details} (2.1 dropped `details_old` with a `folder_drop` removal), R0 = {details, details_old}; roll back on A; crash at every slot | one journal: replace `details`, install `details_old`; a `rollback` re-add of `details_old` covering the drop removal; every computer reinstalls `details_old` with R0's tree; a later unrelated removal of `details_old` still wins | A-18, A-20, A-22 |
| **T-R4D21-choice-over-rollback** | contracts.md §10: rejection on {a, b}; a later keep on {a} covering it | a follows the keep; b keeps the recovered tree; same result with a and b grouped or separate locally | A-18, A-20 |
| **T-R4D9-concurrent-rollbacks** | each row of §3.2 in both delivery orders on three computers: same target; different targets (decision, R1 spreads nowhere); chain R2→R1→R0; opposite rejections (ordinary or only-blocked hold); late delivery of the covered rejection; unavailable target (`RecoveredUnavailable`, nothing changed on disk); concurrent lifts; lift vs new rejection; rollback vs removal (removed everywhere, rejection kept, re-add selects non-rejected). Same target on every computer in every order | A-18, A-20 |
| **T-R4D9-choice-vs-rollback** | a keep that covers a rejection replaces the recovered target; a rollback that covers a keep replaces it; concurrent keep and rollback with different contents give a decision; a keep of rejected content is inert; resume ends the recovered target and R1 stays rejected | A-18, A-20 |

A-38 controls to include: skip level 2 in `compose` (T-R02 must fail);
pick the higher version label over the rejection (T-R02); install an
unconsidered newer release automatically (T-R06); report a code-only
rollback as paired (T-R08); let a lift remove a rejection it does not name
(T-R10); order concurrent rollbacks by `written_unix_ms`
(T-R4D9-concurrent-rollbacks must fail).
