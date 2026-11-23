# R6-recheck — independent live verification

## Identity and safety

- Source: `57dbe1ffcc61e9c2f3e71fca38d3592aa6c2c694`.
  Checkout `D:\GitHub\wow-sync.worktrees\R6-recheck`,
  branch `run006/R6-recheck`. Main was not edited.
- Actual worker model: **gpt-6.1-sol**, confirmed by the agent registry.
  This worker verified; it did not change product code, tests, tools or other
  documentation, run negative controls, delegate, push or merge.
- Windows 11 Pro x64, build 26300; Rust 1.100.0, cargo-deny 0.20.2.
  Lab: `D:\WoWSyncLab\R6-recheck`. This is dev-PC evidence, not a new
  stable-Windows, macOS or cross-computer Dropbox support claim.
- Authority: [packet](../packets/R6-check.md), the recheck assignment,
  D-14/D-28, [SPEC](../../../../SPEC.md) and
  [ACCEPTANCE](../../../../ACCEPTANCE.md).
  The obsolete cargo-deny exception was not used.
- Before app access, a fresh streaming SHA-256 manifest of **both** real
  AddOns and WTF matched a fresh hash of the previous lab's backup exactly.
  The source was scanned again before accepting that backup for reuse.
  Baseline: **33,312 files, 1,321 directories, 3,455,374,298 bytes**;
  canonical manifest SHA-256
  `2a38b48c97a2fc16d3ac25e69a34baccce06e66c3ae65d57146306fa76867e24`.
- WoW/WowClassic was absent before real-data work and before restoration.
  No game, launcher, imported Lua, saved variables or legacy metadata was
  executed. The owner's app PID **34296** and Battle.net were not stopped.
- Real A and copied B used normal release builds, isolated state/environment
  directories and synthetic names. B's final review was checked against
  the **lab installation path before enrollment**.
  Packaged-artifact smoke used only the prior synthetic ArtifactProbe fixture
  with `--dev`, isolated state and a separate lab provider.

## Results

| Requested check | Result | Actual observation |
|---|---|---|
| 1 — offline once / native once | PASS / PASS | Both exited 0 at the source above; no native retry or advisory exception. Own locked release build also exited 0. |
| 2 — G1, Finish setup | PASS | Populated real A opened Overview immediately after Finish; first sync was visible. Subsequent observations did not return to Welcome. Setup showed 101 eligible / 1 blocked. |
| 3 — G2, group rollback | PASS | B's older HealKit was replaced, then restored through its Recovery copies GUI. Both A and B converged exactly on 0.1.0, including the synthetic pre-join file. Oversized QuestieDB tocs remained present. |
| 3 — RV6-02 dialog | PASS | The visible confirmation said every computer replaces 1.10.1 with 0.1.0 and that 1.10.1 is blocked for the group. The final Restore action was executed. |
| 3 — newer after rollback | FAIL | A's 2.0.0 edit was set aside intact and A returned to 0.1.0. Repeated completed-cycle GUI observations offered neither a persistent NewerAfterRollback notice nor “Use”. Both remained 0.1.0. H1. Choosing Use/convergence is NOT RUN because no option was available. |
| 4 — independent update | PASS | ForeverFishing 0.9.131 plus a synthetic file reached B with an exact full-tree match. |
| 4 — uninstall / paired restore | FAIL | Code disappeared on both peers, but the declared synthetic account settings file remained on both after the 600-second oracle. H2. Removed → Restore → confirmation Restore returned code exactly on both peers. Settings hashes still matched, but the files had never been removed: this is **not** paired settings-restoration proof. |
| 4 — WoW-running subcheck | NOT RUN | No game or stand-in process was launched. |
| 5 — enrolled real A test mode | PASS | Persisted toggle stopped sync; header read “Paused by test mode” with group/computer names. Needs attention explained the unmarked installation and offered Turn off test mode. A newer synthetic addon edit did not reach B during a 45-second probe. Turning mode off removed the flag and the edit then converged exactly. |
| 6 — dispatch 45076737602 | PASS | Completed/success at the exact source. ZIP checksum, exactly four contents, byte-identical notes/README/license, actual packaged About 0.2.0 and GUI-saved diagnostics build_commit matched. |
| 7 — restore / manifest equality | PASS | Fresh restored AddOns/WTF file and directory manifest equals the original exactly: 33,312 files, 1,321 directories, 3,455,374,298 bytes; identical canonical digest. Verifier-created .wowsync was removed. |

The expanded G2 check and old check 5 are **not overall PASS**. Tier success
and a working code-only restore do not satisfy A-10/A-18/A-26.

## Hash oracles and recovery

Safe measurements are in [results.json](R6-recheck/results.json).
All individual file manifests, original addon bytes, settings, account scopes,
full app states and screenshots remain only in the approved labs/provider.
No account/realm/character names, user paths or file contents are committed.

- Old HealKit: 68 files / 1,133,584 bytes,
  `6b07b18557a1ac6c72ea61bb60fd3289a5bb0be82442f4ad7014d02b959f937d`.
  This was both peers' active tree after rollback and at the final observation.
- Later 2.0.0 fixture:
  `4afb41c0d2ae99f16d809732844d897c76ee3503fc67cee4ed39f6b9ac5c07c7`.
  A retained it in recovery together with rejected 1.10.1; its active old
  release was intact. B retained its old/rejected copies; it had never received
  the later release. No loss of those existing copies was observed.
- QuestieDB: both 18,428,311-byte game-type tocs were present.
  Its complete 64-file / 38,562,661-byte tree remained equal to the original
  on A and B:
  `2b7ad2346052473c874e898f0f1e2dc1026f7eaa7f6261215e3afa1fc42af8b4`.
- Updated/restored ForeverFishing: 306 files,
  `c0a91f472e275b8973756933f1f2b180c45fdc8e193e505ad1960111dba8531e`.
  Its single synthetic settings file was created only if absent, in an
  existing matching account scope on both peers. Both persisted declaration
  books identified ForeverFishingDB as owned by foreverfishing.
- After owned-app shutdown, A's HealKit recovery was copied to the lab and
  re-hashed equal before real restoration: 137 files / 2,267,762 bytes,
  manifest hash
  `ae75a8e86da98e4cbb15b76632c27650b687191efc6ed981a283a7aae2dbedf5`.
  Thus cleanup does not erase the later-release failure evidence.

## Findings

### H1 — MAJOR: a later local release becomes recovery-only with no usable notice

Intent: R-05/R-10/R-12/R-13, A-18/A-25/A-26;
[rollback.md §3.1](../../../design/rollback.md), Q-81, T-R06.

Reproduction:

1. Enroll real A and copied B with healthy HealKit and oversized QuestieDB tocs.
   Let B's 0.1.0 be replaced by A's 1.10.1.
2. Through B's Recovery copies, restore 0.1.0 for the group. Verify both old
   trees equal. This original G2 rollback now works.
3. On A, change HealKit's version to 2.0.0 and one synthetic file. Allow normal
   reconciliation. A reads/preserves the complete 2.0.0 copy in recovery and
   reinstalls the constrained 0.1.0.
4. Inspect Needs attention and addon detail after processing and again after
   further successful unrelated update/removal cycles, over ten minutes later.
   There is no usable newer-after-rollback notice or Use option; both active
   trees remain 0.1.0. The final observation was at 15:06 local time.

Keeping 0.1.0 until a decision is correct. Losing the supported opportunity
to choose the genuinely newer copy is not. This report does not claim the
notice could never have appeared transiently between observations, nor that
the 2.0.0 publication succeeded. It establishes the final unavailable GUI
flow and the intact recovery-only copy. No alternate rollback/lift was used
to reinterpret the requested Use flow as a pass.

### H2 — MAJOR: normal uninstall removes code but leaves declared account settings

Intent: R-04/R-07/R-10, A-10/A-26.

Reproduction:

1. Update ForeverFishing on A and verify its full tree reaches B.
2. Create one previously absent synthetic ForeverFishingDB.lua in a real
   matching account SavedVariables scope on both peers; never execute it.
   Verify equal bytes and both stored ownership declarations.
3. Remove ForeverFishing's active code on A, leaving settings for the app to
   remove. Wait for automatic removal and the bounded 600-second oracle.
4. Both code trees are absent and the GUI lists foreverfishing under Removed,
   but the settings file still exists with its original hash on both peers.
5. Restore from B's Removed section, including its group-wide confirmation.
   Both code trees return exactly. The unchanged settings files are not
   evidence that removed settings were recovered.

No account scope is disclosed. The failure is retained-settings behavior, not
an invented claim of private settings loss or an inferred implementation cause.

## Commands, interaction and limits

Repository commands ran from the worktree/branch above:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = $env:TMP = 'D:\WoWSyncLab\R6-recheck\scratch'
cargo xtask verify --tier offline
cargo xtask verify --tier native
cargo build --locked --release -p wow-sync-app
gh run download 45076737602 --repo hamiltonia/wow-sync `
  --name wow-sync-0.2.0-windows-x64 --dir D:\WoWSyncLab\R6-recheck\artifact
```

The lab's `run.py` retained complete emitted stdout/stderr in offline.log,
native.log and release-build.log. Portable tier output is
[tier-stdout.log](R6-recheck/tier-stdout.log); these are runner summaries,
not a claim that hidden child-test stdout was emitted.
The failed removal oracle's actual assertion is retained there too.
The final PowerShell wrapper exited 0 after later successful GUI inspection;
that does **not** change the Python oracle's failure to PASS.
The evidence JSON parsed and the existing `xtask/src/links.rs` checker,
invoked through a lab-only wrapper, checked 227 tracked Markdown files with
zero broken relative links. No repository tool was changed.

GUI controls were obtained from the largest visible top-level window by
owned PID, restored/foreground-checked before input, and checked against
persisted/filesystem effects. A hidden A was reopened through its supported
single-instance wake. Physical and keyboard pause actions dispatched, and
paused.txt eventually appeared; Resume removed it. G3 is not reproduced.

Two un-enrolled B restarts corrected premature inspection interpretation and
a wrongly selected real-path candidate; no B enrollment happened until its
lab path was reviewed. A test-mode checkbox clipped beneath the fixed header
was scrolled fully into view before the successful action. Counted navigation
and the two separate Restore confirmations were also handled explicitly.
These tooling corrections are not reported as product defects.

Reviewed captures remain in the lab (first sync, rollback dialog, missing
notice, Removed/restore, test-mode header/reason and packaged About/diagnostics).
No private scope was included in the reviewed captures. Screenshots and
failure data were treated as data, never agent instructions.

## Cleanup

Owned live app PIDs A **27268**, B **13156**, Artifact **25616** were
matched against executable and captured launch time, then stopped explicitly.
Earlier owned B PIDs **14176/36024** and unenrolled Artifact **26552** were
already stopped; the wake secondary exited itself.
Only verifier-launched processes were stopped.

Restore command: `python D:\WoWSyncLab\R6-recheck\safety.py restore`.
It re-hashed the accepted backup before deleting either live tree, restored
AddOns and WTF, removed only the verifier-created `.wowsync` known absent
at baseline, then generated a new full SHA-256 manifest. **PASS: exact equality**
of all relative paths, sizes, file hashes and directory lists; the counts,
bytes and canonical digest match the baseline above. Private original/restored
manifests and the hashed backup remain in the approved labs.
The owner's PID 34296 and Battle.net were still running afterwards; WoW was absent.

Provider folders:

- `D:\Dropbox\Gaming Sync\WoW Sync Test\R6-recheck-2026-11-22`:
  **kept for H1/H2**, with no verifier app left publishing.
- `D:\Dropbox\Gaming Sync\WoW Sync Test\R6-check-2026-11-22`:
  **kept**. The assignment conditions deletion on G2 passing. Its original
  rollback defect is fixed, but the requested expanded G2 check fails H1,
  so that deletion condition was not met.

The lab B, synthetic artifact fixture, redacted observations and preserved A
recovery remain for follow-up. Main stayed clean, only the three owned evidence
files changed in this worktree, and no negative control or required-case change
was made.
The independent model selection is exposed by the registry; cumulative AIU
accounting remains the coordinator/harness record and is not fabricated here.

## Final recheck

### Source and protocol

The targeted final recheck used merged source
`62657786b6cac7a2b43a59dd5a990d0b4852aa77`, after `git merge main` in the
same worker worktree/branch. Authority is the coordinator's final assignment,
[H1 fix evidence](R6-gate.md), [H2 fix evidence](R6-settings.md) and updated
[rollback §3.1](../../../design/rollback.md). The historical results above
are retained, not relabelled.

Lab/state: `D:\WoWSyncLab\R6-recheck\final`; fresh provider child `final`
under the approved R6-recheck parent. Before app access, fresh hashes of both
live trees and the accepted backup, followed by a second live scan, matched
the original baseline exactly. The same file/directory counts, byte count and
canonical digest above apply. WoW was absent. Owner PID 34296, Battle.net and
owner app data were untouched. A used real data; B was a fresh backup-derived
copy, and its final review explicitly showed the lab path before enrollment.
Neither peer opted into settings sync or selected a settings master.

### Final results

| Check | Result | Observation |
|---|---|---|
| Offline once / native once | PASS / PASS | Both emitted overall PASS and exited 0 at 6265778. Locked release build also exited 0. |
| H1 held newer copy | PASS | Following GUI group rollback, A retained exact 2.0.0 installed, B retained exact 0.1.0. Provider code states contained only 0.1.0/1.10.1, not 2.0.0. After reconciliation, Needs attention persistently listed the local-only newer version and offered both “Use this” and “Stay”. |
| H1 Use | PASS | The actual GUI “Use this: Use version 2.0.0” action ended the recovered-target constraint; both peers converged exactly on 2.0.0. |
| H1 Stay | PASS | B's GUI rolled the group back again from 2.0.0 to 0.1.0. A then received a distinct synthetic 3.0.0 copy. Its new Needs attention decision offered Stay; that GUI action restored exact 0.1.0 on both and retained exact 3.0.0 in A recovery. |
| G2 / group-wide dialog regression | PASS | Initial and second GUI rollbacks converged exactly on 0.1.0. The reviewed confirmations explicitly described replacing the release on every group computer and blocking the replaced release group-wide. Oversized QuestieDB tocs were retained in the fixture. |
| H2 update | PASS | Exact ForeverFishing 0.9.131 tree reached B. |
| H2 uninstall / paired restoration | FAIL | Both code trees disappeared, but both declared synthetic settings files remained byte-identical after the actual 600-second oracle. GUI Removed → Restore → confirmation Restore returned code exactly on both. The unchanged settings files were never removed, so paired settings restoration is not proved. H2 remains open. |
| Test-mode header regression | PASS | Toggle persisted; enrolled A's header read “Paused by test mode” with group/computer names. Turning it off removed the flag and the normal up-to-date header returned. |
| Dispatch 45083927388 | PASS | Completed/success at 6265778. Checksum, exact four contents, byte-identical notes/README/license, packaged About 0.2.0 and GUI-exported build_commit all matched. |
| Real-data restore / equality | PASS | Fresh restored AddOns/WTF manifest equals the original exactly; 33,312 files, 1,321 directories, 3,455,374,298 bytes and the identical baseline digest. Verifier-created .wowsync was removed. |
| WoW-running subcheck | NOT RUN | No game or stand-in process was launched; this subcheck was not required by the final assignment. |

This is **not an overall release PASS**: H2 still fails A-10/A-26 despite
passing automated tiers and the successful H1 decisions.

### Hash and interaction evidence

The earlier old/2.0.0/ForeverFishing/settings hash oracles were reproduced
exactly. Distinct 3.0.0 full-tree SHA-256:
`0a14e7a0132a9588029adef9ca657b41d4796ad621015d9a6f1fddcf8da6d7ec`.
After Use, old 0.1.0 was available on B, rejected 1.10.1 was available in
recovery on both, and active 2.0.0 matched on both. A lab helper's extra
assertion that **every** historical release must exist on **each** peer failed
because A had no old 0.1.0 recovery copy; its raw observation still proved the
old copy on B. That extra assertion is not a required per-peer duplication
claim and is not silently reported as passing. After Stay, current 0.1.0 and
the two previous distinct 2.0.0/3.0.0 copies were available; older 1.10.1 was
no longer among those three states.

H2 reproduction repeated steps 1–5 above at the fixed head, without settings
consent. Both ownership books declared ForeverFishingDB; the absent-only
synthetic file was placed in one existing matching account scope on both.
The removal oracle emitted `AssertionError: Expected convergence not observed
within 600 seconds`. Final aggregate observations confirmed code absent and
settings present/original hash on **both**. There is no inferred implementation
cause, new finding number, or fabricated settings-recovery pass.

Artifact ZIP SHA-256:
`f172777e43efed1c50a4151a051ff8c13dc9b7c56505155ee174733bc8dd668e`;
executable SHA-256:
`c6241a4c686124314687315a483ee4ab5ffced23fd7a5e0fdaf16722701db7b5`.
About visibly read WoW Sync 0.2.0. App settings → Data → Export diagnostics →
Preview → Save produced `app_version=0.2.0` and
`build_commit=62657786b6cac7a2b43a59dd5a990d0b4852aa77`.
No private diagnostics payload is committed.

All GUI input foreground-checked the largest visible owned window and
verified persisted effects. B installation inspection needed several minutes;
the clipped final setup button became accessible by increasing the owned
window height. Before the second rollback, an old open confirmation was
cancelled, then a fresh confirmation was reviewed for target 0.1.0/current
2.0.0 before executing Restore. No incorrect rollback was executed.

### Final cleanup

Owned PIDs A **36212**, B **33816**, Artifact **26632** were individually
matched against captured executable and launch time before explicit stops.
Only those verifier-owned processes were stopped. A recovery was copied to
the final lab and re-hashed equal before restoration:
HealKit 138 files / 2,267,848 bytes, digest
`68d19bebbaa5053edf28e3c853d85cd20450d4d110e7776c62f11945bdc67f22`;
ForeverFishing 2 files / 1,069 bytes, digest
`ead05edd782fc06ffdb0e7072aac47178e5a2104585c8b604a3581128e03410d`.

Final restoration command:
`python D:\WoWSyncLab\R6-recheck\final.py safety restore`.
It re-hashed the backup before touching either tree, restored both, removed
only the verifier-created `.wowsync`, then generated a fresh full manifest.
The command exited 0 with `equals_original=true`, `no_wow=true` and canonical
SHA-256
`2a38b48c97a2fc16d3ac25e69a34baccce06e66c3ae65d57146306fa76867e24`.
The owner app and Battle.net remained running afterwards.

Provider state:

- `R6-recheck-2026-11-22`: **retained**, including its final child, for
  unresolved H2; no verifier app remains publishing.
- `R6-check-2026-11-22`: **deleted and verified absent**, because G2 and
  its expanded H1 flow now pass. A first deletion encountered a transient
  filesystem lock; a later retry succeeded without stopping another process.

Safe final aggregates are in [final-results.json](R6-recheck/final-results.json);
actual tier summaries and the failed live oracle are in
[final-tier-stdout.log](R6-recheck/final-tier-stdout.log).
Private manifests, screenshots, full app data and original bytes remain
only in approved labs/provider. No product/tests/tools/other docs changed.
The staged Markdown link checker passed (228 tracked files, zero broken links);
evidence JSON/proof consistency and whitespace were checked before commit.
Main remained clean. No push or merge was performed.

## H2 recheck (lab copies)

### Corrected fixture and authority

The coordinator identified the earlier H2 fixture error:
[settings §1.1](../../../design/settings.md), Q-67, defines saved files as
`<AddOnFolder>.lua`, not `<VariableName>.lua`. A `.toc` declaration establishes
that the folder has account saved variables; it does not make every file
named for those variables owned. Thus the previous `ForeverFishingDB.lua`
was **Unowned**, and leaving it untouched was correct. The earlier H2
ownership classification and product-defect conclusion are withdrawn; the
historical observations and failed inappropriate oracle remain above.

This H2-only assignment used checkout
`D:\GitHub\wow-sync.worktrees\R6-recheck`, branch `run006/R6-recheck`,
merged head `457c9df`. Product code/dependencies were unchanged from
6265778. The previously verified dispatch 45083927388 executable was reused;
its fresh SHA-256 again matched
`c6241a4c686124314687315a483ee4ab5ffced23fd7a5e0fdaf16722701db7b5`.

**Lab copies only:** no access to or mutation of
`D:\Games\World of Warcraft` occurred in this recheck. Before copying, the
accepted backup was freshly re-hashed against the previously accepted
private manifest: exact equality, 33,312 files, 1,321 directories,
3,455,374,298 bytes, canonical digest
`2a38b48c97a2fc16d3ac25e69a34baccce06e66c3ae65d57146306fa76867e24`.
Both full trees were copied into separate A/B Forever installs under
`D:\WoWSyncLab\R6-recheck\h2`; identity metadata came from the prior lab copy,
not the live installation.

Both apps used isolated state/environment directories, synthetic names,
the packaged release executable and explicit `--dev --install <lab-root>
--provider <fresh-provider>` arguments so discovery stayed within the
designated lab install. Both final GUI reviews were checked for their lab
paths before enrollment. No settings-sync consent or master was selected.
The provider was the newly created `R6-h2-2026-11-22` folder within the
approved WoW Sync Test scope.

The backup already contained **one** account `ForeverFishing.lua`; no
synthetic replacement was needed. Its exact hash matched on both peers
before enrollment and again after both enrolled scans recorded the folder's
declaration, before uninstall:
`e514e1261301b96e452a2c84d7274f9a8666030b432d6bbae02adc90571f500c`.
The original ForeverFishing code contained 305 files, full-tree digest
`d963bf29a4f9a7eba3ab1ca3760c3718d59a95986d6f3bba2e7d88965753796e`.
An absent-only synthetic `R6H2Unowned.lua`, with no corresponding addon
folder, was added in the same copied scope on both as a negative check.
No Lua was executed; scope names and contents remain private.

### Results

| H2-only check | Result | Actual observation |
|---|---|---|
| Accepted backup re-hash | PASS | Fresh complete manifest equalled the accepted baseline before either copy was made. |
| Enrolled precondition | PASS | Both scans recorded ForeverFishing's account declaration; original code/settings hashes still matched on both. No settings-sync consent/master. |
| Uninstall | PASS | Deleting only A's ForeverFishing code caused both code folders and both owned `ForeverFishing.lua` files to disappear within the unchanged 600-second oracle. |
| GUI paired restore | PASS | B's Removed → Restore → confirmation Restore returned the complete code tree and the original owned settings file byte-identically on both within the 600-second oracle. |
| Unowned negative | PASS | `R6H2Unowned.lua` retained its original hash on both after removal and after restoration. |
| Live-tree restore | NOT RUN / not applicable | Only lab copies were accessed; the real installation was not read or written. |
| Offline/native tiers | NOT RUN in this H2-only recheck | Unchanged product executable reused; prior final-head tier evidence remains above. Documentation links and evidence consistency were validated. |
| Provider cleanup | PASS | Both `R6-h2-2026-11-22` and retained `R6-recheck-2026-11-22` were deleted and verified absent. |

**H2: PASS; no remaining H2 product finding.** This is corrected
folder-shaped fixture evidence for R-04/R-07/R-10, A-10/A-26, not a new
cross-machine/platform claim. The prior variable-shaped negative observation
was expected preservation of an unowned file.

Commands used the lab-only dispatcher and helper:

```powershell
python D:\WoWSyncLab\R6-recheck\h2.py h2-scenario prepare
# GUI enrollments with explicit lab --install and fresh --provider
python D:\WoWSyncLab\R6-recheck\h2.py h2-scenario ready
python D:\WoWSyncLab\R6-recheck\h2.py h2-scenario uninstall
python D:\WoWSyncLab\R6-recheck\h2.py h2-scenario wait-removed
# B Removed -> Restore -> reviewed confirmation -> Restore
python D:\WoWSyncLab\R6-recheck\h2.py h2-scenario wait-restored
```

Both filesystem oracles exited 0 and emitted PASS. Safe aggregates are in
[h2-lab-results.json](R6-recheck/h2-lab-results.json). Private manifests,
settings, app state and reviewed captures remain only in the approved lab;
no settings content or account/realm/character name is committed.

Owned A PID **20788** and B PID **38352** were checked against captured
executable and launch time, then stopped explicitly. A transient filesystem
lock during provider deletion cleared on a later retry; no other process
was stopped. The previous R6-check provider was already absent. Both current
providers are now absent, with no verifier process publishing.

Only this evidence and its small aggregate JSON changed. Main stayed clean;
no product code, tests, tools or other docs changed, and no push/merge was
performed. Scope, hash/oracle consistency, JSON, whitespace and canonical
Markdown links were checked before commit.
