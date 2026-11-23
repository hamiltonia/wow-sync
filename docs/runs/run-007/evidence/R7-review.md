# R7-review evidence (run-007)

Role: integration review (claude-opus-5.5). Worktree
`D:\GitHub\wow-sync.worktrees\R7-review`, branch `run007/R7-review`, at main
`b82eb0d`. Reviewed `git diff 8b0dee0..b82eb0d` against SPEC R-03, R-04, R-05
(as amended by D-29), R-06, AGENTS.md product rules, A-47 and A-48. Product
code was read only. The R7-sync and R7-setup evidence files were treated as
claims and checked against the code.

**Verdict: not ready for `v0.3.0`.** RV7-01 is a release blocker: the
unrecognised add-on lineage rule lets an offline computer's stale copy silently
revert a deliberate update on every other computer.

## Findings

| ID | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| RV7-01 | critical | `crates/wow-sync-core/src/target.rs:368` (also `:205` `shared_releases`, `crates/wow-sync-runtime/src/service.rs:2246`) | "Lineage" is not proven from records. It is inferred from a local, unpublished memory: "x equals the hash I last held identically with some peer". That does not show that the other copy descends from x. Once two computers share a newer copy, any third computer whose published state still holds an older copy, or a computer joining with an unrelated copy, "dominates" it and is installed everywhere without a decision. Different computers also hold different records, so they compute different winners (R-06 determinism). This is an offline peer resurrecting a superseded copy (AGENTS rule, packet Q1b/Q1c) and breaks A-48. Reproduced: see Probes. | Base lineage on published records. Each computer publishes, for each unrecognised package, the hash its current copy was derived from. `c` beats `x` only if `c`'s holder published `x.hash` as its parent. A hash that the group has ever superseded is never "newer". Add a 3-computer test with one computer offline. |
| RV7-02 | minor | `crates/wow-sync-core/src/package.rs:364` (early return at `:404`) | `UnrecognisedDiffers` is recorded first and returned before any recognised-folder version evidence. In a mixed multi-folder package, the lineage arm can therefore choose a copy whose recognised folder is demonstrably older (`Less`). Contradictory evidence should need a decision. | Return `UnrecognisedDiffers` only when no recognised folder compares as `Less`, `Greater` or ambiguous. Otherwise use `MembersDisagree` or `VersionAmbiguous`. |
| RV7-03 | minor | `crates/wow-sync-core/src/target.rs:119` (via `choose`) | A copy chosen by lineage is explained as `NewerVersion { from: "unversioned", to: "unversioned" }`, which renders as "version unversioned from X is newer than unversioned here". R-05 requires a reviewable explanation of the evidence. | Add an `Explanation::ContentLineage { folder, source }` that names the shared copy it replaced, and use it on this path. |
| RV7-04 | minor | `crates/wow-sync-core/src/toc.rs:63` | An oversized `.toc` is accepted as `Selected` when any `##` line is in the first 256 KiB. Directives after the window are silently dropped, including `SavedVariables*` and `Dependencies`. That gives a partial settings declaration and possibly a split package. Variant selection is filename-based, so truncation cannot change variant. Settings ownership can be incomplete, but in the safe direction (undeclared, not mis-owned). | When the file is cut, require a non-directive line after the last `##` line in the window (proof the header ended). Otherwise return `TooLarge`. |
| RV7-05 | minor | `crates/wow-sync-app/src/setup.rs:59` | `apply_dropbox_choice` accepts any folder as "Dropbox". It does not check that the folder is inside a Dropbox-managed location (`info.json` paths), and it forces `online_only = false`. R-03 requires the Dropbox client. A non-synced local folder would make publication silently go nowhere. | Accept the choice only if it is, or is inside, a path from `dropbox_info_files()`. Otherwise show a warning that names the folder. |
| RV7-06 | minor | `crates/wow-sync-app/src/main.rs:261` | `detect()` runs synchronously on every non-dev start, including when already enrolled, before the sync thread is spawned. It now probes 4 paths on every fixed drive. USB disks often report `DRIVE_FIXED`, and a spun-down disk can take seconds to wake. The 10-second sync-start gate was not measured with this change. | Run discovery only when not enrolled, or on the backend thread after the first sync has started. |
| RV7-07 | minor | `crates/wow-sync-app/src/app.rs:1005` | A picked folder is checked with `install_from_choice` (directory reads) on the UI thread. A picked UNC path or disconnected network drive can freeze the window. Automatic discovery excludes such drives; the picker path does not. | Run the check on a background task. Optionally reject or warn on UNC paths and non-fixed drive letters. |
| RV7-08 | minor | `crates/wow-sync-runtime/src/e2e.rs:530`, `:556` | The A-48 "differ without lineage" tests assert only that files were not overwritten. They do not assert that exactly one decision is offered for that add-on, or that recovery copies are kept when it is resolved. There is no test with three or more computers or an offline peer, which is how RV7-01 escaped. | Assert one `ChooseCopy` decision for `bare` and recovery points after resolving it. Add the 3-computer offline case. |
| RV7-09 | minor (provisional) | `docs/release/v0.3.0.md:43`, `:61` | The release notes still have the `TODO(R7-setup)` marker. Line 61 says the log records "each add-on that is blocked or not recognised", but the code logs blocked add-ons only. The notes do not say that unrecognised add-ons' settings are not synced (R-07 consent scope). The R7-setup follow-up is in progress, so recheck after it lands. | Fill in the section, including "settings of unrecognised add-ons are not synced". Either log unrecognised folders or drop the claim. |

Counts: **critical 1, major 0, minor 8** (RV7-09 provisional).

## Probes (temporary, reverted; `git diff --exit-code` clean afterwards)

1. Unit test in `target.rs`. Local and peer 2 hold `s`, which is recorded as
   the shared release; peer 3 holds older `o`. Result: `Chosen` from peer 3,
   explained as `NewerVersion "unversioned" → "unversioned"`. A second scenario
   gave peer 3 an unrelated copy `x`, as a new joiner. Result: `Chosen` (true).
2. E2E in `e2e.rs` with Lab computers a, b, c, all in sync at `v1`. Then a
   changes to `v2`.
   - All three syncing: `v2` everywhere after round 0, and it stays.
   - c not syncing for rounds 0–3, with its `v1` state still published:

     | Round | a | b | c |
     |---|---|---|---|
     | 0 | `v2` | `v2` | `v1` |
     | 1 | `v2` | `v1` | `v1` |
     | 2 onward | `v1` | `v1` | `v1` |

     The deliberate update was reverted on both online computers with no
     decision.

Command: `cargo test -q -p wow-sync-core zz_probe` and
`cargo test -q -p wow-sync-runtime zz_probe` (Windows, this dev PC,
synthetic Lab data). Both probes were removed with `git checkout --` on the
touched file.

## Checked without a finding

- **Clock and mtime.** No path in the diff uses mtime, clock or machine
  preference to order copies. The ordering is wrong for the reason in RV7-01,
  not because of a timestamp.
- **Decision scope.** `NeedsDecision` is per package. Other packages continue
  (R-06).
- **Rejection.** Rejected hashes are screened `Out` before the dominator, so a
  rollback still holds. Removal logic is unchanged by the diff; the e2e removal
  round trip passes per the R7-sync evidence (not re-run here).
- **Header reading.** Reads are bounded to `TOC_LIMIT + 1` bytes in both
  `fs_scan::read_bounded` and the recovery path in `effects.rs`.
  `header_window` and the parse are linear in 256 KiB, so a crafted `.toc`
  cannot exhaust memory or time beyond the existing per-file bound. The whole
  file is still hashed by streaming.
- **product.db.** The read is bounded at 4 MiB and treated as bytes, with no
  code execution. At most 32 paths of at most 400 characters are taken. `..`
  is rejected, and only paths on fixed drive letters are probed.
  Network, removable and optical drives are excluded from automatic discovery.
- **Picked folders.** A picked game folder only adds candidate rows
  (`inspected = false`), which still pass inspection and identity and scope
  confirmation. Variant crossing is prevented by `valid_peers` matching
  `product`.
- **Log privacy.** The log is synchronous, unbuffered and append-only, so it
  holds no memory. Each file rotates at 1 MiB with one older file kept, so the
  bound is about 2 MiB plus one line. The `USERPROFILE` prefix becomes `~`,
  case-blind and in both slash styles. Control characters are flattened. Only
  folder names, paths and reasons are logged; no add-on or settings contents.
- **Not checked.**
  - A picked root that is itself a junction to another volume.
  - Non-ASCII profile names whose lower-case form changes byte length (the
    redaction then falls back to exact matching).

## Budget

Within the about-300-AIU guide; no sub-agents.

## Recheck (main `77ddb72`)

Re-reviewed on `run007/R7-review`, fast-forwarded to `77ddb72`. Probes were
temporary `zz_probe_*` tests in `e2e.rs` and `target.rs`, using synthetic
content only. All probes were reverted, and `git diff --exit-code` is clean.
Baseline: `cargo test -q -p wow-sync-core -p wow-sync-runtime` passes
375 + 207 + 2 tests.

### Status of RV7-01 to RV7-09

| ID | Status | Evidence |
|---|---|---|
| RV7-01 | **resolved** (this exact scenario) | `shared-releases.json` and `shared_at` are gone; no references remain outside `docs/runs`. Lineage is now published in `code-state/1` `lineage`. The `lineage.rs` BFS uses a seen-set, so cycles terminate. `newer(x, y)` needs y reachable from x and not the reverse. I re-ran the exact probe: `three_at_v1`, a moves to v2, a and b sync every round, and c syncs only from round 4. Rounds 0–3: a=v2, b=v2, c=v1. Rounds 4–7: all v2. No decisions after round 0. Round 0 showed a transient `Ambiguous` on a; see RV7-10. |
| RV7-02 | resolved | `compare_package` returns version ambiguity first. Then any `Less`/`Greater` gives `MembersDisagree`. Only otherwise does it return `UnrecognisedDiffers`. `lineage_never_overrides_contradicting_version_evidence_in_a_package` passes. |
| RV7-03 | resolved | `Explanation::ContentLineage` reads "changed from the copy last shared, on X; replaces <hash>". The test asserts the text has no "unversioned". The decision-candidate label still says "unversioned"; that is acceptable there. |
| RV7-04 | resolved | `toc.rs` `header_has_ended` requires a non-blank, non-`#` line inside the window; otherwise the result is `TooLarge`. Covered by `cut_header_without_a_file_line_is_not_selected`. |
| RV7-05 | partly resolved | `is_dropbox_folder` checks the choice against `info.json`/default roots (tested), and the app shows `NOT_DROPBOX_TEXT` and logs a warning. It still accepts the folder (a warning, not a refusal) and forces `online_only = false` (`setup.rs:87`). This is a deliberate trade-off; it stays minor and needs owner acceptance. |
| RV7-06 | resolved | `main.rs` runs `detect()` only when there is no saved target. `backend::detect_later` searches on its own thread after the first status, and only if not enrolled. |
| RV7-07 | resolved | `pick_folder` runs `check_picked` on `background_executor`, shows "Checking…", and ignores re-entry while checking. UNC paths are not rejected, but they no longer block the UI thread. |
| RV7-08 | resolved | New three-computer and offline e2e tests assert `offered_kinds == [Ambiguous]`, recovery copies, and that the loser does not resurface. Gap: no e2e test has the author's change while every peer is offline, and none has a revert. That is how RV7-10 escaped. |
| RV7-09 | resolved | The TODO is gone. Unrecognised settings not being synced is disclosed. `backend::log_addon_notes` now logs "not synced" and "Unrecognised" add-ons once per change, by key and reason only, which matches the log claim. Remaining note wording issues are in RV7-13. |

### Break attempts

| Attempt | Result |
|---|---|
| Cyclic or mutual lineage (unit) | `unrecognised_copies_that_descend_from_each_other_need_a_decision` gives `Ambiguous`. A core probe with local v1, own published v2, and peer v2:[v1] also gives `NeedsDecision(Ambiguous)`. **But this only holds in core; see RV7-10.** |
| Revert (e2e P3) | All three at v2; then a restores v1 locally. After a's **first** sync, a is at v2 again with no decision offered. v1 is only in a's recovery copies (`recovery_holds(a,"v1") = true`). |
| Author alone (e2e P5) | a and b at v1; b offline; a changes to v2. On a, 4 syncs each offered `[Ambiguous]` for a's own change. It cleared only when b came back and echoed v2. |
| Ancestor cap (e2e P2) | Three at v1; c offline; a makes 40 changes, and a and b sync after each. After about 32 changes, b stalls at e31 and both a and b are offered `Ambiguous`. c's v1 is no longer reachable from the current copy's truncated list. When c returns, everything converges to e39 with no decisions, because b's e31 entry still links to v1. Safe, but see RV7-11. |
| Forged lineage (unit) | Local v3; peer 2 holds v1 with no lineage; peer 3 holds v3 and publishes an entry `v1:[v3]` for a hash it does not hold. Result: `Chosen` v1, so the local v3 is replaced by the stale copy. See RV7-12. |
| Decision loser (e2e P4) | a decides for v1 over `from-a`/`from-b` while b is offline. b returns and gets v1; `from-b` does not resurface. a's later edit of the winner raises the existing `NewVersionAfterDecision` item (pre-existing keep semantics). No defect. |
| Old-version peer, no lineage | Never `newer`, so it gives a decision, never a win (`differing_unrecognised_copies_without_lineage_need_a_decision`). v0.2.0 `decode` has no `deny_unknown_fields`, so old readers ignore `lineage`. |
| Contradicting version evidence | Covered by the RV7-02 test; it gives `MembersDisagree`, never a lineage win. |

### New findings

| ID | Severity | Location | Issue | Fix |
|---|---|---|---|---|
| RV7-10 | **major** | `crates/wow-sync-core/src/target.rs:228` with `crates/wow-sync-runtime/src/service.rs:2030` | `compute_inner` looks for the local computer's previous published state in `input.members`. The runtime never puts itself there: `read_members` skips `me`. So `local_prev` is always `None` in production, and the local copy never carries its "replaced" ancestor when the target is computed. Only the unit `Scenario::local_published` adds self, which is why core tests pass. Effects: (1) a computer that changes an unrecognised add-on is offered a "choose a copy" decision about its own change until some peer echoes it. If every peer is offline, the decision persists (P5). Choosing the other copy reverts the change. This violates A-48 ("a lineage-newer copy propagates automatically"). (2) A deliberate revert to an earlier copy is silently replaced by the group's copy on the next sync, with no decision (P3). That contradicts `domain.md:416` ("mutually-descended copies (a revert) … `NeedsDecision`") and the v0.3.0 notes ("the copy that changed … wins"). It is recovery-preserved, so there is no data loss. | Pass the computer's own latest published state (`own_latest`, already loaded for publishing) into `TargetInput`, for example as `local_prev`, and use it in `compute_inner`. Add e2e tests for author-alone-with-peers-offline (expect no decision) and for a revert (expect the documented outcome). |
| RV7-11 | minor | `crates/wow-sync-core/src/lineage.rs:119` (`truncate(MAX_ANCESTORS)`) | With more than 32 changes while a peer is offline, the online computers stall and are offered a decision, even though nobody changed concurrently (P2). It self-heals when the peer returns. It is safe (nothing is overwritten or resurrected), but the user gets a spurious prompt and the most recent change does not reach the online computers. | Keep ancestors that are still held by an admitted peer ahead of truncation, or document the limit in the notes and `domain.md`. |
| RV7-12 | minor | `crates/wow-sync-core/src/lineage.rs:53` (`LineageGraph::add`, via `from_states`) | A state's lineage entry is accepted for any hash, including one the publishing computer does not hold. One member can therefore make a stale copy held by another peer "newer" than the current copy, and it is `Chosen` (forged-lineage probe). Members are admitted and trusted, so this needs a bug or a hostile member. Validation still costs one comparison. | In `from_states` (and `CodeState` validation), accept a `PackageLineage` only when its `hash` equals that state's own package hash for `package`. |
| RV7-13 | minor (docs) | `docs/release/v0.3.0.md:38`, `:46`, `:53` | The oversized `.toc` change is described twice ("Large `.toc` files" and again under "Add-ons that cannot be recognised"). Line 53 ("the copy that changed since the last shared copy wins") is untrue for a revert and, today, for an author whose peers are offline (RV7-10). | Merge the two `.toc` paragraphs. After RV7-10, describe the actual outcome for a revert and for the 32-ancestor limit. |

### Recheck verdict

Counts: **critical 0, major 1 (RV7-10), minor 4 (RV7-11, RV7-12, RV7-13, and
RV7-05 still partly resolved)**. RV7-01 to RV7-04 and RV7-06 to RV7-09 are
resolved. RV7-10 is open and major: it should be fixed, with its e2e tests,
before `v0.3.0` is tagged.

Budget: recheck within the about-300-AIU guide; no sub-agents.
