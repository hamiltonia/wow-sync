# R6-check — independent live verification

## Identity, authority and scope

- Checkout: `D:\GitHub\wow-sync.worktrees\R6-check`, branch
  `run006/R6-check`, source `6e87a35fb82e00e1a3fea012fb7919265443c545`.
- Windows 11 x64 dev PC; Rust 1.100.0, cargo-deny 0.20.2, locked
  dependencies including `url` 2.5.9. No obsolete advisory exception used.
- Authority: [packet](../packets/R6-check.md), [COMMON](../packets/COMMON.md),
  D-28 and the approved run-006 plan. This is verification, not implementation.
  No nested agents, product changes, new dependencies, push or merge.
- Normal release-profile apps used isolated app-data directories under
  `D:\WoWSyncLab\R6-check`; no `--dev` enrollment. Computer names were synthetic.
  Main checkout remained untouched.
- Provider: `D:\Dropbox\Gaming Sync\WoW Sync Test\R6-check-2026-11-22`.
  Kept for G2. No other Gaming Sync entry or `D:\WoWSyncDemo` was touched.
- No addon Lua, saved-variable file or imported metadata was executed.
  WoW/WowClassic was absent before real-data work and again before restoration.
  Owner Battle.net processes and the owner's existing app were never stopped.

The packet assigns gpt-6.1-sol. Actual runtime selection and AIU accounting are
coordinator/harness records, not exposed by these verification tools; no numeric
budget claim is manufactured here.

## Results

| Check | Result | Observable evidence / limit |
|---|---|---|
| 1 — tiers and CI | PASS | Initial offline once and native three times passed; own locked release build passed. Latest main CI observed with `gh`: run 45069233956, head `ca929fca9fa14cdb778a3af2f60fa91ab212271b`, completed/success. |
| 2 — real A enrollment | PASS | Unmarked real Forever accepted normally. Identity and scope confirmations were separate. Independent toc oracle and GUI both showed 101 eligible / 1 blocked; readable blocked reason. G1 is a presentation finding after Finish setup. |
| 3 — simulated B | PASS | Backup-derived B, separate app state, matching group, missing addons, older HealKit and local-only synthetic addon. Final review showed the lab installation, not real A. |
| 4 — first run/recovery | FAIL | First-run preservation and local GUI recovery passed with hashes. B recovered its sole replaced addon exactly, but the GUI-promised group rollback did not reach A; G2. |
| 5 — update/removal/restore | BLOCKED | Independent ForeverFishing update reached B exactly. Uninstall and paired code/settings GUI restore were NOT RUN after G3; fixture preparation is not removal evidence. WoW-running subcheck NOT RUN. |
| 6 — test mode | BLOCKED | Marked synthetic install published while mode was on; fresh real-install setup refused readably and its Turn off test mode offer removed the flag and accepted inspection. Existing enrolled real A stop/resume subchecks NOT RUN after G3. |
| 7 — refusal screen | PASS | Captured/reviewed C refusal: nonblank `_forever_` heading, readable explanation, Turn off test mode, no internal enum and no useless Try again. Screenshot retained in lab. |
| 8 — dispatched artifact | PASS | Run 45067079956 succeeded at the verified source. ZIP checksum, four contents, byte-identical notes, packaged About 0.2.0 and GUI-saved diagnostics build commit matched. |
| 9 — restore/equality | PASS | Fresh restored manifest equals original exactly: 33,312 files, 1,321 directories, 3,455,374,298 bytes; identical canonical SHA-256. Verifier-created `.wowsync` metadata removed. |

This is not an overall product/release acceptance PASS. In particular,
[A-18](../../../../ACCEPTANCE.md) and the unexecuted A-10/A-21/A-26 scenarios
are not satisfied by the tier results or local-only recovery.

## Commands and retained evidence

Repository commands ran from the exact worktree/branch above, with cargo added
to the fresh shell PATH and TEMP/TMP directed to the lab's `scratch` directory:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = $env:TMP = 'D:\WoWSyncLab\R6-check\scratch'
cargo xtask verify --tier offline
cargo xtask verify --tier native
cargo xtask verify --tier native
cargo xtask verify --tier native
cargo build --locked --release -p wow-sync-app
gh run list --repo hamiltonia/wow-sync --branch main --workflow ci.yml --limit 1 --json databaseId,headSha,status,conclusion,url,updatedAt
```

The lab `run.py` captured complete stdout/stderr for these cargo commands.
The tier runner's emitted logs are summaries, not hidden full child-test logs;
their actual complete stdout is preserved in
[tier-stdout.log](../../../../evidence/R6-check/tier-stdout.log).
Release-build and positive-suite logs remain in the lab. After reverting all
controls, full core/runtime positive suites passed: 358 and 187 tests.
Additional offline/native pairs passed during evidence preparation.
The final precommit pair covers the completed evidence files and links.
No native failure required a retry; source/tests stayed clean.

Safe raw measurements are in
[results.json](../../../../evidence/R6-check/results.json).
Full per-file manifests, private account paths, original addon/settings bytes,
app states and original screenshots remain only in the approved lab/provider.
Redacted/cropped screenshots are retained under
`D:\WoWSyncLab\R6-check\redacted-evidence`; they are not repository file copies.
Their portable hash record is
[screenshot-hashes.json](../../../../evidence/R6-check/screenshot-hashes.json).
This avoids publishing user-data manifests or writing repository artifacts
through shell file APIs. Committed artifacts are created only by the edit tool.

### Independent negative controls (A-38)

Each temporary mutation was applied separately, its test failed with exit 101,
then the source file was reverted and its diff verified clean. No assertions
were removed, limits widened or required cases skipped. The R6-check
required-cases section is unchanged/empty.

| Control | Temporary behavior fault | Observed failing test selector |
|---|---|---|
| 1 | Ignore the default marker policy and refuse an unmarked install | `variant::tests::unmarked_install_is_accepted_by_default` |
| 2 | Prefer plain toc over the Mainline fallback | `toc::tests::forever_profile_suffix_mixes_observed_on_the_dev_pc` |
| 3 | Disable equal-version ambiguity detection | `e2e::tests::first_run_join_of_a_populated_install_loses_nothing` |
| 4 | Replace the readable refusal with its internal debug enum | `service::reason_text_tests::every_refusal_reads_as_a_sentence_that_says_what_to_do` |
| 5 | Disable the persisted test-mode state | `e2e::tests::test_mode_persists_and_blocks_unmarked_installs_both_ways` |

Full control stdout/stderr is in lab `control-1.log` through `control-5.log`.
Selected actual failures are also retained in
[negative-control-excerpts.log](../../../../evidence/R6-check/negative-control-excerpts.log).
Those logs record the selected test and assertion but not the original shell
command line; the table intentionally reports selectors, not invented flags.

## Real enrollment and independent oracles

Before any app touched A, a streaming SHA-256 manifest and backup covered
AddOns and WTF: **33,312 files, 1,321 directories, 3,455,374,298 bytes**.
Backup and a second source scan matched the original exactly. Canonical
manifest SHA-256:
`2a38b48c97a2fc16d3ac25e69a34baccce06e66c3ae65d57146306fa76867e24`.
The real variant initially had neither a synthetic marker nor `.wowsync`.

The independent oracle implements the approved
[domain toc rule](../../../design/domain.md): Forever/Camelot tier first,
then Mainline, then plain; disagreement inside a tier is ambiguous. There
were **zero count differences**. QuestieDB was the sole blocked addon:
its Forever and Camelot toc files were each 18,428,311 bytes, over the 256 KiB
inspection cap. The GUI called this an unreadable addon-description (.toc)
file. Inventory folder status `ok` alone does not mean a toc was selected.

B removed ForeverFishing and PriestHUD, changed HealKit to 0.1.0, planted
one synthetic HealKit marker, and added local-only R6LocalOnly. First sync
installed the missing addons and selected A's HealKit 1.10.1. Independent
full-file comparisons found **zero original A files changed or missing**.
Only B's HealKit tree had an original file changed/displaced. R6LocalOnly
survived and reached A with equal payload hash.

B's supported GUI flow, HealKit → Recovery copies → Restore this version →
Restore, recovered the exact older B tree: 68 files / 1,133,584 bytes;
tree digest `6b07b18557a1ac6c72ea61bb60fd3289a5bb0be82442f4ad7014d02b959f937d`.
The same operation's group promise failed (G2), rather than local recovery
being called a complete group success.

An unrelated ForeverFishing edit, 0.9.130 → 0.9.131 plus one synthetic file,
reached B: 306 files, matching tree digest
`c0a91f472e275b8973756933f1f2b180c45fdc8e193e505ad1960111dba8531e`.
One synthetic owned settings file was prepared in a matching account scope,
and both persisted ownership declarations were validated. No private scope
name or contents is reported. This preparation does **not** prove removal.

The first settings waiter incorrectly expected the declaration-book mtime
to change after a settings-file write. Source inspection showed that
`SettingsPass::open` saves changed declarations, not every file write.
That verifier prerequisite was discarded and replaced by actual declaration
validation; its timeout is not reported as a product failure.

## Findings

### G1 — MINOR: setup returns to Welcome while enrolled sync is running

Intent: [R-12/R-13](../../../../SPEC.md), A-25/A-26.

Reproduction: enroll a populated normal A or B, then Finish setup. The UI
reset to Welcome while enrollment was already persisted and the first
background sync was progressing. Welcome says nothing changes until setup
confirmation, although that confirmation already happened. Observed on both
instances; `A-first-sync.png` retains the capture. In `app.rs`,
`SetupAct::Finish` resets the UI. This is misleading onboarding state, not
evidence that the earlier identity/scope confirmations were bypassed.

### G2 — MAJOR: unrelated blocked toc freezes rejected-addon convergence

Intent: [R-05/R-10](../../../../SPEC.md), A-18; partial-inventory isolation.

Reproduction:

1. Enroll two matching Forever installs containing healthy HealKit and an
   unrelated QuestieDB with an oversized/unselectable toc.
2. Let B's HealKit 0.1.0 be replaced by A's 1.10.1, then restore B's prior
   release through the GUI. The dialog promises group rollback/rejection.
3. B restores exactly, but A remains 1.10.1, showing “Checking a copy against
   a blocked version”. A/B convergence did not happen within 600 seconds.
4. Change A's healthy HealKit to 2.0.0. Its publication occurred, but B stayed
   0.1.0; a second 600-second convergence oracle failed. An unrelated healthy
   ForeverFishing update still converged.

This is not inferred solely from elapsed time. Both actual provider state
headers have `inventory_complete: false`, despite readable healthy HealKit.
The source chain at the verified commit is:

- `plan.rs:148`: completeness requires **all** folders transfer-eligible.
  `inventory.rs:110`: that includes a selected toc; QuestieDB fails it.
- `target.rs`, `Source::from_state`: combines admission with whole-inventory
  completeness. `Source::content` returns Unknown for **every** folder of
  such a source, even a healthy folder with a known tree.
- `reject.rs:330`: rejected-release comparison propagates that Unknown.
- `target.rs:310–332`: any screened Wait holds the package in
  CheckingAgainstBlocked, even after other eligible candidates were found.

The unrelated unsupported addon therefore poisons release identity for
HealKit during rejection screening. No implementation fix was made. The
provider is kept for this finding, including the incomplete headers and
rejection/publication history.

### G3 — verification BLOCKER: late GUI actions could not be dispatched

Related coverage: R-04/R-10/R-13, A-10/A-21/A-26.

Reproduction in this session: with enrolled A/B running, attempt A's Pause
using physical coordinates after supported window-show refresh, then the
advertised UI Automation Invoke pattern. Both returned without a new app
TogglePause action or persisted pause flag. Earlier onboarding/recovery
actions had worked. B's ForeverFishing still existed and the chained
uninstall helper never reported reaching its pause prerequisite.

Some earlier helper captures selected a 1×1 IPC window; largest-window
selection and restored bounds corrected that tooling issue. Supported
single-instance wake events refreshed app visibility, but the two later
control approaches still did not dispatch Pause. The pending helper was
stopped per COMMON's stopping rule. No uninstall/restore proof was produced.

This is **not a confirmed product defect**: focus/input/tooling remains a
possible cause. Existing-A test-mode stop/resume and remaining removal/
restore/game-blocking tasks need a working interaction rerun, rather than
more injection attempts or a claimed PASS. No real WoW or stand-in was
launched for the unexecuted game-blocking subcheck.

## Artifact and refusal details

Artifact ZIP SHA-256:
`2170cc0150b38971574e1c6ff490320edc2bcadff673de4019cb8cae05183acf`.
It contained exactly LICENSE, README.md, RELEASE-NOTES.md and wow-sync.exe.
Notes matched [v0.2.0 notes](../../../release/v0.2.0.md) byte-for-byte;
the packaged app's actual About and GUI-saved diagnostics confirmed version
and build provenance, not merely filenames. Hashes are in results.json.

Fresh isolated C with test mode on reproduced the owner's refusal safely.
Its Turn off test mode action removed the persisted flag and subsequent
inspection accepted the real install. The marked synthetic artifact lab
also published an edited 2.0.0 addon while mode was on: publisher audit
sequence 2, three objects. Neither observation substitutes for the blocked
existing-A stop/resume checks.

Original C refusal capture, retained in the lab, SHA-256:
`96c52a06a8e37374bcea78c8d5b6c6a1660d3b87f488447ea74decc79be69eb0`.
No account/realm/character/user names appeared in the reviewed captures.

## Cleanup and final verification

Restore command: `python D:\WoWSyncLab\R6-check\data.py restore`, after owned
app shutdown and a fresh no-WoW check. It re-hashed the backup before deletion,
restored both AddOns and WTF, and generated a new complete file/directory
manifest. **PASS: fresh manifest equals original exactly**, including relative
paths, sizes, file SHA-256 values and directory lists. Counts/bytes and the
canonical digest equal the original values above. No restore assertion failed.
The safe [restore proof](../../../../evidence/R6-check/restore-proof.json) is
committed; private original/restored per-file manifests remain in the lab.

Only the `.wowsync` directory created by this run, known absent at baseline,
was removed from the variant root after restore. No original variant metadata
or another install was changed. The isolated B/artifact fixtures and real-data
backup remain in the lab; real A is back at its exact baseline.
Owned app PIDs A 31340/B 39468 were matched to captured executable and launch
time before stopping. Previously owned C/artifact instances were already
stopped, and wake secondaries exited themselves. No owner process was stopped.

Precommit commands `cargo xtask verify --tier offline` and
`cargo xtask verify --tier native` exited 0 from the worktree. Complete emitted
stdout is appended to tier-stdout.log; lab `final-offline.log` and
`final-native.log` retain stdout/stderr. Product/tests/tools diff was empty,
the required-cases section unchanged, and main status was clean.

The hashed backup/private manifests remain in the lab. Provider retention
is intentional for G2, not a forgotten cleanup task. Remaining blocked
scenarios require new independent evidence before release acceptance.
