# Evidence: R4-design-fix

- **Packet:** [R4-design-fix](../packets/R4-design-fix.md)
- **Review fixed:** [R4-design-review](R4-design-review.md) (FAIL, R4D-1 to
  R4D-14)
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-design-fix`
- **Branch:** `run004/R4-design-fix`, from `main` at `80c4a5e`
- **Environment:** Windows x64 owner workstation

This packet is design only. No product code, test code, build or CI file
changed. The owner was unavailable: nothing here is recorded as an owner
decision. Every owner question is designed to the packet's conservative
default and marked **PENDING OWNER**. This evidence is the author's claim;
an independent design recheck is still required.

## Files changed

| File | Change |
|---|---|
| [docs/design/contracts.md](../../../design/contracts.md) | **New.** Single shared contract: C-AUTH record family (identity = sha256 of exact bytes, refs, authority index, ledger segments, `Known` coverage, bounded sizes, historical membership and retirement barrier, validation, learning/mirroring/repair, capabilities and downgrade), C-JRN (`ApplyJournal/2`, `RecoveryPoint/2` against `record!` major 1), C-IDS, C-TGT precedence and constraint composition, code-choice family, `decide` freshness, admission order, diagnostics boundary, technical caps vs product gates, phase C ownership and build order, contract tests T-C01 to T-C15 |
| [docs/design/removal.md](../../../design/removal.md) | Rewritten to consume contracts.md only: folder keys, per-folder supersession by `known`, per-`(enrollment, removal, folder)` obligations, ack/discharge ordering, holds instead of removal decisions, explicit add-back, observing-computer settings cleanup |
| [docs/design/settings.md](../../../design/settings.md) | Rewritten: `SettingsUnit{folder, identity}` keys, policy/revision/state bodies only, frontier parents, missing-parent waiting, retired-basis handling, authority epoch, paired selection, GC roots, size cap as a pending gate |
| [docs/design/rollback.md](../../../design/rollback.md) | Rewritten: rejection and lift bodies without ids/notes, recovered-target constraint with chain following, concurrency table, look-alike hold, no settings rejection, group-rollback precondition |
| [docs/design/ops.md](../../../design/ops.md) | Rewritten: code-choice decision actions with no Interface gating and no override of unusable metadata, retirement on the contracts barrier, notices with caps vs gates, update transport hardening and exact tag URL, diagnostics without source text |
| [docs/design/questions.md](../../../design/questions.md) | PENDING OWNER carve-out in the intro; Q-55/57/58/60/63/64/77/78/81/82/84 marked PENDING OWNER with the conservative default; Q-49/50/52/53/56/61/69/72/75/76/79/80 revised; new Q-85 to Q-87 |
| [docs/design/README.md](../../../design/README.md) | contracts.md row; refreshed descriptions |
| [docs/design/ui/screens.md](../../../design/ui/screens.md), [docs/design/ui/questions.md](../../../design/ui/questions.md) | `NoCompatibleToc` → `UnusableMetadata` note; `HoldScope::Addons` name |

## R4D-1 to R4D-14

| Finding | Fix | Counterexample tests |
|---|---|---|
| R4D-1 (blocker) no common authority/recovery wire contract | contracts.md is the one authority: kinds, location, sha256-of-bytes identity (no `id`, no `{id, hash}`, no domain prefix), envelope, refs, index, ledger, `Known`, sizes, membership, validation, mirroring and repair of own bad copies, capabilities/downgrade; one `ApplyJournal/2` + `RecoveryPoint/2` for settings-only, mixed and code-only-fallback, keeping non-empty `folders`, versioned against `record!` major 1 (`/2` under the slice → `NeedsHelp`). The four designs define only bodies and V-KIND rules | T-C01 to T-C07 (golden bytes, retired forms, bad copy, own-only repair, v1 byte-identity, `/2` under slice, pending report v2), T-C12, T-C14 |
| R4D-2 partial ack releases untouched stale folders | obligations keyed per `(enrollment, removal, folder)`; a partial re-add supersedes only its own folders; discharge is per folder (removal.md §3.4, §3.5) | T-R4D2-partial-readd-both-orders |
| R4D-3 ack rehabilitates an older stale inventory | discharge only from the first state after the ack's `state_seq` that is absent for the folder; later reappearance is `ReinstallObserved` hold (removal.md §3.4, §5.6) | T-R4D3-ack-before-state |
| R4D-4 finite limits make a re-add unwriteable | unbounded `supersedes`/`knows` lists replaced by `Known{seen ≤ 512, extra ≤ 128}` coverage; ledger in 4 MiB segments, no drop (contracts.md §1.7–§1.9) | T-R4D4-1024-removals-readd-bound, T-R4D4-ledger-overflow-16MiB, T-C08, T-C09 |
| R4D-5 passive uninstall keeps origin's settings | observing computer checkpoints then removes its owned settings before its ack says `settings: removed`; crash-safe via `/2` recovery point (removal.md §6.3) | T-R4D5-passive-uninstall-settings |
| R4D-6 package keys identify shared settings policy | stable `SettingsUnit{folder, identity}`; package key is display only; NFC collision and case-mismatch diagnoses (contracts.md §3) | T-R4D6-regroup-policy-stable, T-C11 |
| R4D-7 equal-head collapse invents a conflict | head frontier: a new revision parents every head of the same state; equal heads are one state (settings.md §4.2) | T-R4D7-equal-heads-frontier |
| R4D-8 no ancestry/dependency semantics | complete graph rules: missing parent → `Waiting(MissingParent)`, retired-cutoff basis invalid, restore parents the basis set, zero heads → `NoSharedCopy`; authority epoch for startup captures (settings.md §2.1, §4.1, §4.2) | T-R4D8-child-before-parent, T-R4D8-retired-basis, T-R4D8-undo-then-next-session, T-R4D8-zero-heads, T-R4D-startup-after-reassign |
| R4D-9 concurrent rollbacks impose incompatible constraints | constraint composition by `known` coverage; uncovered differing targets → `RollbackTargetsDiffer` decision; chain following with visited set; rejection always wins (contracts.md §4.2, rollback.md §3.2) | T-R4D9-concurrent-rollbacks, T-R4D9-choice-vs-rollback |
| R4D-10 (blocker) ops depends on deleted removal decisions | new `wowsync.code-choice/1` family (keep/resume; allow-again = rejection lift), precedence rejection > removal > choice > ordinary, concurrency, R3D-10/R3D-16 retraced; removal has no decisions; no Interface gating (advisory text only); `UnusableMetadata` has no options and no "use anyway" (contracts.md §5, ops.md §1) | T-R4D10-keep-considered, T-R4D10-resume-declines-nothing, T-R4D10-unusable-metadata-no-override, T-R4D10-interface-advisory, T-C15 |
| R4D-11 late retirement un-retires another computer | monotone barrier: every enrollment named by a valid retirement stays retired, retirement records exempt from their issuer's retirement, cutoff = max; no `authority_known`, no grounded labelling (contracts.md §1.10, ops.md §2.1) | T-R4D11-late-retirement-three-issuers, T-C10, T-C13 |
| R4D-12 diagnostics carry private source text | closed enums, numbers, version classes only; no labels, titles, notes, record titles or error strings; folder names only if they match a plain-name pattern and the box stays checked; fingerprints off by default; NFC + case-folded scan (contracts.md §7, ops.md §5) | T-R4D12-planted-secret |
| R4D-13 defaults cross unapproved barriers | PENDING OWNER carve-out in questions.md; conservative defaults below; caps vs gates table (contracts.md §8); unapproved gates block features (settings sync on real data) or are reported blocked, never passed | T-S-size-cap-pending, T-27-4, T-27-7, T-27-8, T-27-11 |
| R4D-14 unprefixed tags link to a different tag | release URL is built from the exact validated tag (`v0.2.0` or `0.2.0`); `html_url` ignored (ops.md §4.3) | T-R4D14-tag-forms |

The review's extra required tests from the cross-packet table are in the
design test lists: contracts.md §10, removal.md §12, settings.md §11,
rollback.md §10, ops.md §7.

## PENDING OWNER questions and defaults used

| Question | Conservative default designed |
|---|---|
| Q-55 | no automatic re-add; reappearance only offers **Add back to all computers** / **Remove again** |
| Q-57 | "Continue without it" on this computer only, with risk text; recovery kept; missing information never treated as absent |
| Q-58 | only folders named by a removal record are removed |
| Q-60 | retired computer stops publishing and applying, keeps local data, rejoins only explicitly |
| Q-63 | native macOS notifications required; failure is A-27 FAILED, not a waiver |
| Q-64 | addon folder names included with an uncheck box (plain-name pattern only); version labels and free text never exported; fingerprints off |
| Q-77 | named cap `MAX_SETTINGS_FILE_BYTES`, no value; settings sync on real data blocked until set |
| Q-78 | exact-content rejection; same label with other content is a look-alike hold |
| Q-81 | only an explicit lift (or explicit code choice) ends the recovered-version constraint; an unconsidered demonstrably newer release only raises `NewerAfterRollback` |
| Q-82 | no settings-checkpoint rejection; ordinary `restore` revision stays |
| Q-84 | block group rollback while a member lacks `rejection/1`; explain the update prerequisite |
| Q-85 (new) | 6 h suppression of an unchanged recurring notice; never `DataAtRisk`; changed fingerprint always notifies |
| Q-86 (new) | `DataAtRisk` respects the notification toggle for OS notices; always shown in the app |
| Q-87 (new) | automatic update check at most every 24 h |

**Deviation, Q-81.** The packet's default lets "a release created after the
rollback" end the constraint. Creation time is not provable (no trusted
clock; the review notes "newly seen" is not proof), so the design never
ends the constraint automatically; a non-considered, demonstrably newer
release raises a notice with a one-click explicit choice. This is stricter
than the packet default and needs the recheck's acceptance.

**Deviation, Q-64.** "Addon names" is narrowed to folder names matching
`^[A-Za-z0-9_.\- ]{1,64}$`, because folder names are the only name source
not otherwise free text (R4D-12); other names become `Add-on N`.

## Phase C ownership and build order

From contracts.md §9:

| Owner | Surfaces |
|---|---|
| R4-core-removal | C-AUTH module; `records` majors, `ApplyJournal/2`, `RecoveryPoint/2`, `PendingReport/2`; plan/exec journal dispatch; adapter/runtime effects journal and authority I/O; `target::compose`; removal kinds |
| R4-core-ops | historical membership, retirement kind and barrier, `admit` order incl. settings watermark; code-choice kind, `decide` freshness, status/view/explain, runtime service/control; diagnostics boundary |
| R4-core-settings | `ids` additions and NFC/case diagnoses; settings module, exec ops, policy and revision kinds; rollback reject/lift and recovered-target constraint |

Order: (1) R4-core-removal foundation (C-AUTH, C-JRN, compose skeleton,
T-C01 to T-C07), nothing else merges first; (2) R4-core-ops and the rest of
R4-core-removal in parallel; (3) R4-core-settings (settings, then
rollback); (4) integration and cross-feature tests.

## Commands and results

| Command (in the worktree) | Result |
|---|---|
| Consistency greps over `docs\design` for withdrawn forms (`knows`, `supersedes` lists, `authority_known`, `{id, hash}`, `wowsync-authority-v1`, `package_key`, `observed_reinstall`, `Use it anyway`, `chose.keep`, `declined`) | remaining hits only state that the form is withdrawn, or are unrelated verbs |
| Section-reference check (script over the six design files against their headings) | one stale ref (Q-53 → removal §2.7) fixed; all others resolve |
| `cargo xtask verify --tier offline` (rustc 1.100.0) | PASS: fmt, clippy, test, required-cases, deny, markdown-links; overall PASS (allow-blocked: false), exit 0. Re-run after this evidence file was written: same result |