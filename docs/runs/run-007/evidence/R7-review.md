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
