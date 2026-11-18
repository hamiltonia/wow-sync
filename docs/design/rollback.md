# Rollback and rejected releases

Status: **design for a later run** (run-004 packet
[R4-settings-design](../runs/run-004/packets/R4-settings-design.md)). Not
yet implemented and not yet independently reviewed. Nothing here changes the
running slice. The slice's recovery points stay local (Q-18) and its target
has no rejection step.

Product authority: SPEC R-10, R-11 and R-12 ([SPEC.md](../../SPEC.md)).
Acceptance: A-16 to A-20, A-23, A-25, A-26 and A-45
([ACCEPTANCE.md](../../ACCEPTANCE.md)). Settings parts use
[settings.md](settings.md). Removal parts use the removal design
([removal.md](removal.md), its "rejection and rollback" and "history and
recovery" sections). Formats and the authority-object family follow
[publication.md](publication.md) and the removal design's "immutable
authority objects" and "authority ledger" sections. Open choices are
**(Q-n)** in [questions.md](questions.md).

## 0. Vocabulary

| Term | Meaning |
|---|---|
| release | one exact package content: the package hash over its folder trees ([publication.md](publication.md) §3). A version label ("2.1") is display, not identity |
| rollback | the user action "this release is bad, go back to an earlier one" for one addon in one group |
| rejection | the group-visible, immutable record that a release must not be installed in the group. Also the "tombstone" of a release |
| recovered release | the earlier release that the rollback installs and distributes |
| lift | the explicit user action "allow this release again", recorded as its own object |
| paired restore | rollback that also restores the settings checkpoint taken with the recovered release, for each identity |
| code-only rollback | rollback where some or all identities have no such checkpoint; current settings stay and a warning follows |

## 1. What rollback does (R-10, A-18)

On one computer, for package P, from the Add-on detail history:

1. The user picks an earlier history state R0 of P (current plus two
   previous, [apply.md](apply.md) §2.4) and chooses "Go back to this
   version…". The confirmation names the current release R1 and says:
   "Version 2.1 will be blocked on all your computers for <variant>.
   Version 2.0 will be installed everywhere." It also states which
   settings will be restored (§4) before the user confirms.
2. The app writes a **rejection** of R1 (§2) and, for each identity with
   shared settings, the settings restore revisions (§4).
3. It restores R0 locally from its local recovery point, or from provider
   trees if some member's published state still has R0, using the normal
   apply operation (WoW-not-running gate, journal, `G` before every live
   rename). R1 moves into a recovery slot as an ordinary previous state,
   marked rejected.
4. It publishes its code state (R0, with R1's rejection referenced) and
   settings state.
5. Every other computer learns the rejection, computes target R0 (§3), and
   applies R0 when WoW is not running there.

Rollback never touches other packages, and never another variant: the
rejection lives in the group folder, and one group is one variant (A-18).

## 2. Records

### 2.1 Rejection (`wowsync.rejection/1`)

An immutable authority object in
`groups\<g>\members\<computer>\authority\<id>.json`, with the common fields
and object-hash rule of the removal design's authority objects
(write-if-absent, verbatim mirrors, durable grow-only local ledger):

```json
{
  "schema": "wowsync.rejection/1",
  "id": "<32 hex>",
  "issuer": { "computer_id": "…", "enrollment_id": "…", "issued_at_seq": 60 },
  "kind": "code",
  "package_key": "details",
  "rejected": { "package_hash": "<sha256>", "version_label": "2.1",
                "folders": [{ "key": "details", "tree": "<sha256>" }] },
  "recovered": { "package_hash": "<sha256>", "version_label": "2.0" },
  "considered": ["<package hash of every candidate of P the issuer knew>"],
  "settings": { "kind": "paired|partial|code_only|not_shared",
                "restores": [{ "id": "…", "hash": "…" }] },
  "knows": [{ "id": "…", "hash": "…" }],
  "note": "",
  "written_unix_ms": 1767225600000
}
```

- The rejection is of the **exact package hash** (Q-78). A version label is
  kept only to explain and to detect look-alikes (§3.2).
- `recovered` is the release the user chose. With `considered` it forms
  the selection constraint of §3.1.
- `settings.restores` references the settings `restore` revisions issued
  with the rollback ([settings.md](settings.md) §3.2).
- `knows` lists every rejection and lift for P in the issuer's ledger.
- `note` is optional user text, at most 200 characters, shown escaped as
  plain text, never interpreted (A-23).

**Settings rejection** uses `kind: "settings"` with
`unit {package_key, identity}`, `rejected {checkpoint}` and `recovered
{checkpoint}` instead of the package fields (§5).

### 2.2 Lift (`wowsync.rejection-lift/1`)

```json
{ "schema": "wowsync.rejection-lift/1", "id": "<32 hex>",
  "issuer": { "…": "…" }, "package_key": "details",
  "lifts": [{ "id": "<rejection id>", "hash": "<object hash>" }],
  "written_unix_ms": 1767225600000 }
```

A lift names exact rejection objects. It never lifts a rejection it does
not name, so a concurrent new rejection of the same release stays in force
(Q-79).

### 2.3 Validity and lifetime

- Valid if: schema and size checks pass; the issuer was a valid member at
  `issued_at_seq` (retirement rules in `ops.md`: objects a retired computer
  issued before retirement stay valid, later ones do not); `package_key`
  and hashes are well-formed.
- **Grow-only.** Rejections and lifts are never deleted or rewritten while
  the group exists, like removal authority. A rejection is **active** when
  valid and not named by a valid lift.
- Every computer that learns a rejection or lift mirrors it into its own
  `authority\` folder (Q-51 applies), so the rejection survives the
  issuer's folder becoming unusable or the issuer being retired.
- Old readers: an app without rejection support does not know the schema.
  Those apps must not be in a group that uses rollback, so the first
  rejection in a group requires every non-retired member's newest settings
  state to list `"rejection/1"` in its `capabilities` field
  ([settings.md](settings.md) §3.2); otherwise the
  rollback is offered as "local only" (restore on this computer, no group
  rejection) and the GUI says why (Q-84).

## 3. Target: rejections come first

In `compute_target` the order for each package is (the removal design's
cycle precedence, with this design filling the first step):

1. **Rejections.** Drop every candidate whose package hash is rejected by
   an active rejection. A computer whose installed release is rejected is
   never a source.
2. Held/pending items, ledger removal authority, selection constraints
   (including §3.1), ordinary ordering, as in the removal design.

### 3.1 The recovered release as a selection constraint

Rejecting R1 alone would let ordinary ordering pick another release newer
than R0 that the user also saw and did not choose (for example R1.5 on a
third computer). So an active rejection with `recovered = R0` adds a
**selection constraint**: the target is R0 while the only candidates newer
than R0 are releases in `considered`. A release that is **not** in
`considered` and newer by ordinary ordering (a genuinely new release that
appears later) ends the constraint, and ordinary ordering resumes (Q-81).
If R0 itself becomes unavailable (no recovery point and no published copy)
the package is held with "Version 2.0 is no longer available on any
computer" and the user chooses.

### 3.2 Offline and late peers (A-18, A-20)

- A peer that was offline still has R1. Its published state lists R1; R1
  is not a candidate (step 1). It learns the rejection, its target is R0,
  and it replaces R1 with R0, keeping R1 as a rejected previous state. It
  cannot reintroduce R1, however high R1's version label is.
- A peer that publishes R1 **before** it learns the rejection still cannot
  win: every computer that has the rejection drops R1, and the peer's next
  cycle learns the rejection from any member folder.
- A peer whose copy has the **same version label but a different hash**
  than R1 (a different build, or local edits) is not covered by the exact
  hash. Ordinary ordering would treat it as new. Safe default: a candidate
  whose version label equals the label of an active rejection of P is
  held as an addon-level decision ("This looks like the blocked version
  2.1 but its files differ") (Q-78).
- A joining computer with R1 replaces it with R0, as any member does.
- A user who installs R1 again by hand on a computer: WoW Sync replaces it
  with R0 (keeping R1 in recovery) and the Activity entry says "Version 2.1
  is blocked for this group. To use it again, choose Allow again."

### 3.3 Different variants

Rejections are group objects and one group is one variant, so another
variant's group never sees them (A-18). Rejecting the same release in two
variants is two rollbacks.

## 4. Settings during a code rollback (R-10, A-18, A-19)

For each **included identity** I of the package on the issuing computer:

1. Find the **paired checkpoint**: the newest capture (by `local_seq`) of
   unit `(P, I)` with `code_package_hash == R0`, in local settings history,
   or a revision in the ledger whose `code_package_hash == R0` and whose
   checkpoint's blobs are present in the provider (Q-80).
2. If found and the addon is shared ([settings.md](settings.md) §5.1):
   capture the current files (`before_change`), issue a `restore` revision
   with that checkpoint and parent = current head, apply it locally after
   the code (same WoW-not-running window, separate settings op). Every
   other computer receives it as a normal revision: it yields its current
   settings into recovery and applies it. A `restore` that competes with
   another intentional revision is a settings choice, as any other
   ([settings.md](settings.md) §4.4).
3. If found but the addon is not shared (no master yet, `not_shared`, or
   settings off on this computer): restore locally only. The summary says
   "Settings were restored on this computer only."
4. If **not found**: do not touch the current settings for I. The unit is
   `code_only`.

The rejection records `settings.kind`: `paired` (every identity restored),
`partial` (some), `code_only` (none) or `not_shared`.

**Afterwards** the app shows a summary that distinguishes the outcomes
(R-10, A-19):

- Paired: "Details is back to version 2.0, with the settings you had with
  2.0 (account ERIC, Thrall on Argent Dawn)."
- Partial or code-only, as a warning, never as success: "Details is back to
  version 2.0. Your settings were **not** restored for Thrall on Argent
  Dawn, because no copy from version 2.0 is left. Your current settings are
  kept. They were made with 2.1 and may not work with 2.0." with "Show
  settings history" and "OK".

The same distinction is recorded in the local recovery point
(`paired: paired|code_only|settings_not_applicable`, as the removal design
introduces) and in the audit.

## 5. Rejecting settings

"Undo this settings change" on a settings history entry of a shared unit
means "go back to the previous shared copy and do not let the bad one
return":

- a `restore` revision with the earlier checkpoint (parent = head);
- a `wowsync.rejection/1` with `kind: "settings"` naming the bad checkpoint
  hash. In the settings target, a revision whose checkpoint is rejected is
  invalid ([settings.md](settings.md) §4.2 step 1). An `auto` capture on the
  master whose content equals the rejected checkpoint is not published,
  and shows "This copy was undone on <computer>; it is not shared again."
  (Q-82).

A settings rejection does not touch code. A code rollback does not reject
settings; it restores (§4).

## 6. Removal, re-add and keep decisions

These keep the removal design's rollback hooks:

| Situation | Rule |
|---|---|
| Rollback of a removed package | It is a restore ("undo removal"), offering only non-rejected states |
| A removal | is not a release and cannot be rejected. Undo it with "undo removal" |
| A rejected release and removal coverage | A rejected release counts as covered for removal: a removal of P is not contested by a computer that only has the rejected R1 |
| Rejections vs removal authority | Independent grow-only sets keyed by package. Neither expires the other. A re-add supersession does not lift a rejection; a lift does not affect removal |
| Re-adding P after removal | Ordinary ordering among non-rejected candidates. If the only copies are R1, P is held: "The only copies of Details are version 2.1, which is blocked." with "Allow again…" |
| A keep decision (user kept a release) | Rejecting the kept release ends that selection constraint; the rejection's own constraint (§3.1) applies |
| Restore from history | Offers rejected states only behind "Allow again…", which issues a lift first |

## 7. Three-state history (R-10, A-16, A-17)

- A rejected release stays in its computer's code history as an ordinary
  previous state, labelled "Blocked", and counts toward current plus two
  previous. It is pruned like any other state. Its rejection record stays
  (grow-only), so pruning the files never lifts the rejection.
- The recovery point of R0 used for the rollback becomes the current state.
  The paired settings checkpoint stays pinned while the code recovery point
  that promises it is retained ([settings.md](settings.md) §7).
- If R0's local recovery point is the only copy anywhere and the rollback
  is still distributing it (some non-retired computers have not reported
  `applied` R0), R0's trees and blobs are uploaded and kept in the provider
  until every non-retired member confirms R0 or a newer non-rejected
  release; the GUI shows this as a temporary exception with its size
  (A-17). This is the only case where a recovery point's content is
  uploaded (Q-18 otherwise stands).

## 8. GUI recovery actions (R-12, R-13, A-25, A-26)

| Screen ([ui/screens.md](ui/screens.md)) | Action / view |
|---|---|
| Add-on detail, history | "Go back to this version…" on each non-current, non-blocked state; "Allow again…" on blocked ones; settings history with "Undo this settings change…" and "Restore" |
| Rollback confirmation | current and recovered versions, the group scope ("all your computers for <variant>"), which identities get paired settings and which stay as they are |
| Rollback summary | paired / partial / code-only as in §4, with the warning style for partial and code-only |
| Add-ons list | "Blocked: 2.1" chip on the addon row while a rejection is active |
| Activity | "You went back to Details 2.0 and blocked 2.1", "Laptop replaced blocked Details 2.1 with 2.0", lifts |
| Needs attention | recovered release unavailable; look-alike of a blocked version (§3.2); only blocked copies on re-add; rollback not possible group-wide (Q-84) |
| Decision | the look-alike decision (§3.2) |

All of these are GUI flows; none needs a terminal (A-26).

## 9. Implementation map

| Crate / module | Additions |
|---|---|
| core `records` | `RejectionObject`, `RejectionLiftObject` via `record!`; rejection kinds `Code`, `Settings` |
| core **new `reject`** | `active_rejections(&Ledger, &PackageKey, &Validity) -> Rejections`; `is_rejected(&Rejections, &PackageHash) -> bool`; `lookalike(&Rejections, &VersionLabel, &PackageHash) -> bool`; `selection_constraint(&Rejections, &Candidates) -> Option<Constraint>` |
| core `target` | `compute_target` step 1 drops rejected candidates and applies the constraint; new `DecisionReason::{LookalikeOfRejected, RecoveredUnavailable, OnlyRejectedCopies}` |
| core `settings` | `paired_checkpoint(&SettingsLocal, &Ledger, unit, &PackageHash) -> Option<CheckpointRef>`; rejected checkpoints in the revision validity check |
| core `plan` / `exec` | rollback is a normal apply op plus a settings op; recovery point `paired` flag |
| core `status` / `explain` / `view` | `PackageStatus::Blocked`, rollback summary `Explanation`s (paired, partial, code-only), `NextAction::{AllowAgain, ChooseRecovered}` |
| adapters `provider` | write/list rejection and lift objects; upload a recovery point's trees for §7 |
| runtime `control.rs` | `Rollback { package, to }`, `UndoSettingsChange { unit, revision }`, `AllowAgain { rejection }` |
| runtime `service.rs` | step 2a learns rejections/lifts; step 5 uses them |
| app | screens of §8 |

## 10. Test obligations

| ID | Test | A-ID |
|---|---|---|
| T-R01 | rollback on A with paired settings: B and C install R0 and the restored settings; R1 is a blocked previous state everywhere | A-18 |
| T-R02 Neg | offline C returns with R1 and a higher version label: C does not republish R1 as a candidate; target stays R0; C replaces R1 | A-18, A-20 |
| T-R03 Neg | C publishes R1 before learning the rejection: no other computer installs R1 | A-18, A-20 |
| T-R04 Neg | other variant's group with the same R1 is unaffected | A-18 |
| T-R05 | the issuer's folder deleted after mirroring: rejection still active | A-20 |
| T-R06 | R1.5 known at rollback is not chosen; a later unknown R2 ends the constraint | A-18 |
| T-R07 Neg | same label, different hash: decision, not install | A-18, A-23 |
| T-R08 | no paired checkpoint (deleted): code-only rollback, current settings byte-identical, warning differs from paired summary | A-19 |
| T-R09 | paired for one identity, missing for another: partial warning lists the missing identity | A-19 |
| T-R10 | lift then restore R1; a concurrent new rejection not named by the lift stays active | A-18, A-20 |
| T-R11 | rollback of a removed package is a restore offering only non-rejected states; removal cannot be rejected | A-18 |
| T-R12 | rejections and removal authority are independent; re-add does not lift | A-20 |
| T-R13 | history after rollback: current plus two previous, blocked state labelled, paired checkpoint pinned while its recovery point exists | A-16, A-17 |
| T-R14 | settings rejection: master's capture equal to the rejected checkpoint is not published | A-18 |
| T-R15 Neg | rejection issued by a retired computer after retirement: invalid | A-45 |
| T-R16 Neg | malformed rejection, oversize `note`, note text with markup or Lua: refused or shown as plain text | A-23 |
| T-R17 | WoW starts during a rollback apply: stops before next rename; no process killed | A-21, A-22 |
| T-R18 (GUI) | go back, summary (paired and code-only), allow again, undo settings change, all without a terminal | A-25, A-26 |

A-38 controls to include: skip the rejection step in `compute_target`
(T-R02 must fail); pick the higher version label over the rejection
(T-R02); report a code-only rollback as paired (T-R08); let a lift remove a
rejection it does not name (T-R10).
