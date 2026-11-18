# R4-design-fix (Design, opus-5.5)

Read [COMMON.md](COMMON.md) first. Input:
[R4-design-review](../evidence/R4-design-review.md), which FAILed with
findings R4D-1 to R4D-14.

One worker owns all four designs, so that the shared contracts become
consistent. Independent per-author fixes would recreate R4D-1.

**Owned paths:**
- `docs/design/contracts.md` (new);
- `docs/design/{removal,settings,rollback,ops}.md`;
- `docs/design/questions.md`;
- `docs/design/README.md` (index row for `contracts.md`);
- your evidence file.

## Goals

1. **R4D-1, blocker: one shared contract** in a new `contracts.md`. It
   covers:
   - authority record family (one dispatched family that removal,
     re-add, ack, code choice, rejection/rollback, settings policy and
     revisions, and retirement all extend);
   - file naming;
   - hashing;
   - ref types;
   - issuer validity;
   - historical membership;
   - mirroring and repair, including repair of an existing
     hash-mismatched file;
   - capabilities and downgrade behaviour.

   It also defines one `ApplyJournal`/`RecoveryPoint` schema covering:
   - settings-only, mixed code/settings, and code-only-fallback recovery;
   - the existing validator's rule that `folders` must not be empty;
   - versioning against the current `records::record!` major 1.

   Rewrite the four designs to use only this contract. Delete conflicting
   definitions.
2. **R4D-10, blocker: a code-choice decision family.**
   - Define keep, resume and allow-again, concurrent choices, and
     precedence against rejection and removal.
   - Remove obsolete removal choices.
   - Ops must not reinstate Interface gating (D-21). Separate advisory
     Interface text from genuinely unsafe or missing metadata. A generic
     "use anyway" button must not make unvalidated content installable
     (R-02).
   - Re-run the R3D-10 and R3D-16 traces against the new protocol.
3. **R4D-2 to R4D-9, R4D-11, R4D-12 and R4D-14:** fix each one as the
   review suggests, or with a better fix you justify. Add each finding's
   counterexample as a named test.
4. **R4D-13 and the owner questions.** The owner is unavailable.
   - For every question the review marks "owner answer needed" or
     "approval", design to the **conservative default** below. Mark it
     **PENDING OWNER** in `questions.md`. Do not record it as decided.
   - Q-55: only an explicit "Add back to all computers" re-adds. A
     reappearance only offers that choice.
   - Q-57: allow "Continue without it", local to this computer only,
     with the risk explained and recovery kept.
   - Q-58: only named folders are removed. Confirmation is pending.
   - Q-60: a retired computer stops publishing and applying, keeps its
     local data, and rejoins only explicitly.
   - Q-63: native macOS notifications are required. A fallback is a
     FAILED A-27 item, not a waiver.
   - Q-64: addon names are included with an uncheck box. Version labels
     and free text are redacted per R4D-12.
   - Q-77: no cap is approved. Design the behaviour with the cap as a
     named parameter, and block the feature on the owner's value.
   - Q-81: only an explicit lift, or a release created after the
     rollback, ends it.
   - Q-82: no settings-checkpoint rejection. Remove that feature;
     ordinary settings restore stays.
   - Q-84: block group rollback and explain the update prerequisite.
   - DataAtRisk notifications: they respect the user's notification
     toggle until the owner decides otherwise. They always show in the
     app.
   - Separate **technical defensive caps** (timeouts, response size,
     diagnostic export size, history list length) from product or
     performance gates.
     - Technical caps are engineering defaults, documented as such.
     - Anything that suppresses user-visible notices (the 6 h repeat
       suppression) is PENDING OWNER, and it must never suppress
       DataAtRisk.
5. **Shared surfaces.** Adopt the review's table "Cross-packet contracts".
   For each shared surface, name **one** owning phase C packet among:
   - R4-core-removal;
   - R4-core-settings (settings plus rollback);
   - R4-core-ops.

   Name the build order: the contract owner lands first. Add the review's
   extra required tests to the design test lists.

## Done

- The evidence has a table of R4D-1 to R4D-14 with how each was fixed.
- It lists every PENDING OWNER question with the conservative default
  used.
- It gives the phase C ownership and order.
- `cargo xtask verify --tier offline` passes.
- Committed on your branch.
