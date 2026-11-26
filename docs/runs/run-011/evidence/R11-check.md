# R11-check: independent candidate verification

Date: 2026-11-25. **Do not release on this evidence.**

Authority: [packet](../packets/R11-check.md), [plan](../PLAN.md),
[settings design](../../../design/settings.md). Verification worker; assigned
model gpt-6.1-sol; no nested agents. No product code or tests were changed.
The model assignment was supplied by the coordinator, not independently
introspected by this worker.

## Important verification incident

**G1 — critical, verification-procedure defect.** During CLI discovery I
executed the candidate with `--help` **without** `--dev`, `--state` or
`--provider`. This was a mistake, not an authorized smoke test. It returned
without console help. I subsequently discovered that this build does not
implement a help-only exit: `main.rs` initializes the default state/log and
logs startup **before** its single-instance check; a second instance signals
the first before exiting.

Consequently, a default-state log append was attempted and the owner's
instance may have received a show signal. I did **not** inspect, modify or
attempt to repair the protected default state to determine the outcome.
There is no evidence here proving the protected log unchanged. PID 8816 was
never stopped, and no real game process was started or stopped.

I recognized this unsafe invocation while reading the startup ordering near
the end of the run. On recognizing it, I stopped the compatibility driver,
all remaining owned app processes and the write/memory monitor. No further
product experiments were performed. Only lab evidence collection, repository
verification and this report followed. The remaining compatibility cases
are BLOCKED by this stop, not product failures. **The lab-scope assurance
failed; the coordinator must review this incident before authorizing a fresh,
strictly scoped verification run.** Functional observations below are retained,
not presented as a clean release qualification.

Reproduction of the procedural defect: starting this binary with only
`--help` follows normal startup. **Do not reproduce against default state.**
Any future CLI probe must itself specify an isolated state and provider.

## Environment and provenance

- Checkout: `D:\GitHub\wow-sync.worktrees\R11-check`;
  branch `run011/R11-check`, starting at
  `3cd15bb9fe7707c22f156b22dce89ae5d1c7f24b`.
- Windows 11 Pro x64, 10.0.26300; Intel Core i7-13700HX; lab on NTFS.
  PowerShell 7.7.0; Rust/Cargo 1.100.0, Windows MSVC toolchain.
- Candidate: GitHub Actions release dispatch **45601785797**, successful,
  head SHA exactly as above. Downloaded with `gh run download`; extracted
  ZIP, not a local application build.
- Candidate ZIP SHA-256, independently computed and equal to its sidecar:
  `d798bcf722a58fcc919ce0f27dd5a318e56c10d037719344256b06de64fc959b`.
- Extracted candidate executable SHA-256:
  `6e08d2a72ddaa2678449aa869632093707b361c1e9a28327b48521785e646227`.
- Published v0.3.3 peer ZIP downloaded with `gh release download v0.3.3`;
  computed SHA-256 equals its sidecar:
  `ce0445f9e7748de7023daf4f585348f98c9bf93094bdbb747d9a99c14cef9697`.
- Raw evidence is retained only in the worker's approved lab, referred to
  below as **LAB**. Screenshots use synthetic identities; no screenshots,
  provider data, copied Lua, private settings or raw logs are committed.

## Gate table

PASS means the stated bounded observation succeeded, not that G1 is waived.

| Gate | Result | Observed evidence |
|---|---|---|
| 1. Dispatch hash and artifact smoke | PASS | Sidecar matches; both artifact GUIs enrolled, joined one lab group, published and applied settings. |
| 2. Guided offers and consent | PASS | A's offer appeared once code was current; consent/account toggle/master/Share these now observed. B showed the named-master offer and its own account consent; Use settings applied A's copy. |
| 3. A-11 through A-15 in GUI | FAIL | A-11, A-12 and A-13 bounded flows passed; A-14 choice sources are indistinguishable and A-15 has no all-settings pause. G3/G4. |
| 4. Missing character | PASS | Initially absent on B; detail said waiting for that synthetic character. Folder remained absent until explicitly created by verifier; next apply matched A's hash. |
| 5. Over-cap and shared ownership shown, never synced | FAIL | Over-cap warning/refusal passed. Shared-ownership bytes stayed different but GUI said only Not shared yet, not why. G7. |
| 6. Activity/history/source/reason/size/restart | FAIL | Activity and persisted records survived three process restarts. Add-on Recent changes omitted settings records; restore wording lost its operation kind. G5. |
| 7. GUI recovery, including peer replaced copy | PASS | B's first-adoption and later yielded copies restored using GUI confirmation; both peers then matched the selected original bytes. |
| 8. Log/history privacy | FAIL | Character/realm canaries in persisted history; absolute lab paths in logs. No tested file-content canary appeared. G6. |
| 9. Write audit / approved-only scope | FAIL | G1 prevents the required assurance. Audited lab writes and complete path list retained; internal variant state also exceeds a literal WTF/explicit-state/provider-only allow-list. G8. |
| 10. Three fresh full-lab settings runs / 100 MiB | FAIL | All three completed with matching capture/apply hashes. Formal hidden-run maximum 83.727 MiB, but subsequent and earlier candidate samples exceeded 100 MiB; highest **105.848 MiB**. G2. |
| 11. v0.3.3 code compatibility, both directions | BLOCKED | Candidate-to-peer update and addition passed with hashes. Removal began but was interrupted by G1 stop; reverse cases NOT RUN. |
| Offline and native verify tiers | PASS | Both executed in this checkout; final offline execution includes this evidence file. |
| Candidate CI, including macOS offline | PASS | CI run 45598375667: Windows/macOS offline and all four architecture build-only jobs succeeded. No macOS runtime claim. |
| Release dispatch succeeds | PASS | Actions run 45601785797 concluded success at the requested SHA. |
| No open critical/major findings | FAIL | G1–G6 remain open; no fixes attempted. |

## Lab and interaction method

Each synthetic computer had its own install copy, explicit `--state`, and
the same lab provider. All intentional application experiments except G1 used:

```text
wow-sync.exe --dev --install <LAB computer> --state <LAB computer-state>
  --provider <LAB provider> --name CheckA|CheckB --zoom 0.75
  --interval-ms 4000 --recent-ms 0
```

Copied only the approved dev AddOns tree and SavedVariables Lua shapes,
read-only from the source. Account/realm/character folder names were mapped
to synthetic names. No Config.wtf, bindings, macros or backups were copied;
no addon Lua or saved settings were executed.

Baseline per computer: **113 folders, 36,142 files, 3,772,226,174 bytes**.
Five synthetic test addons added ten inert code files: memory runs used
**118 folders, 36,152 files, 3,772,227,020 bytes**. The account-wide and
per-character copied settings retained realistic shape. Additional synthetic
addons were introduced only after the three formal memory runs.

Windows UI Automation invoked the actual GUI controls. PrintWindow captured
the owned windows; screenshots were viewed to check the interactions and
absence of real names. UIA's offscreen flag was unreliable; harness actions
were adjusted to scroll controls into view. An attempted action without its
expected GUI/log result was not counted as success.

For session safety/conflicts, used the existing fake-client technique:
an inert copy of Windows PING named Wow.exe **inside A's lab variant**.
Its captured PID was stopped after the synthetic session. No real WoW was
started, stopped or injected into. The app visibly reported WoW running and
left the master's capture unchanged during eight repeated fixture writes.

## Settings flow observations and hashes

Hashes below are SHA-256 of live synthetic settings files, not checkpoint
identifiers. The LAB contains the complete before/after JSON and CSV records.

### Guided adoption

A displayed “Share add-on settings between your computers?” after code
publication completed. Its consent included the full exclusion/scope text,
the synthetic account, this computer selected as master, the future-addon
list, and “Share these now: yes”. Account exclusion/inclusion and the
Share-these-now toggle were exercised before cancellation, then consent was
confirmed. No settings consent was given to a real group.

B displayed “CheckA is now the settings master. Use its add-on settings
here?”. Its consent showed its own included account and “Use settings”,
without asking it to choose another master.

Screenshots: `A-consent.png`, `B-consent.png`.

### A-11, A-12, A-13 and recovery

- **A-11 PASS, bounded:** default master A automatically shared the stable
  session-end change. A GUI exception selected B for CheckPause; B's next
  edit automatically reached A. CheckNewAuto was discovered after consent;
  its initial activation was not shared, and its next stable settings edit
  published automatically and reached B. Both new-addon settings hashes:
  `04f54132fb7498c1222097879f7c4fa4e5e525bc63b9d51856b34c13c34abada`.
  This is not a full A-16/A-17 retention/low-storage test.
- **A-12 PASS:** B visibly showed “Changed here, not shared”; master A's
  session-end revision replaced it without an automatic B push.
  Before replacement B's hash was
  `f59cd2542c7f9ba833f3c126066d15a835c0538ccf47b6931287a44d3faef11a`.
  After replacement both had
  `633c05ca69ba476ad585736f21d88d60ab01e8ee4cd3fe38f59437d84036a2af`.
  B's “Restore this computer's copy…” followed by “Restore” brought back
  the former `f59c…` bytes on both computers.
- **A-13 PASS:** after a settled resume, B edited its copy and clicked
  “Share my settings”. The log recorded ShareSettings accepted and a
  revision written; A adopted it, and both remained at
  `6e147d0d881fa6e1bae81e78ed5d72811da5d54aabccad63cb170f71e75f1f68`
  through subsequent checks rather than A immediately republishing stale data.
- **Recovery PASS:** initial A settings hash was
  `bd16c0754bce884f65672c9638571c8678dd9a814a1452fc14a5e67de98af810`;
  GUI restoration of B's first-adoption recovery copy changed both to
  `d2aab4d0fa181501edc60046a0a08797b21e7feeb13c87c73073bed707680ad4`.
  The independent later yield recovery above also restored exact bytes.

Screenshots: `A-wow-settings.png`, `B-routine-local.png`,
`B-restore-confirm.png`, `B-yield-recovery.png`, `B-A13-share-command.png`.

### Missing identity and refused settings

B's CheckSettings detail explicitly waited for the synthetic missing
character. A filesystem existence check returned false before the verifier
created that character's SavedVariables directory. The app then applied the
character settings; both live hashes were
`111a9ee7a29c69e372c71220f4294d3cb58d73f31ac4085ae3d607d008679b28`.
Screenshots: `B-missing-detail.png`, `B-restore-confirm.png`.

The over-cap fixture was **33,554,433 bytes**, one byte over the approved
cap. GUI: “Too large to share automatically (32.0 MB)”. The verifier later
changed one byte on B; both refused copies stayed different:

| Fixture | A final live hash | B final live hash |
|---|---|---|
| Over cap | `bdf7fbb54387c24608fd757a1b31c32cabcd7b6bee8ff3345c897139fffc072a` | `a0584edc885376f49aa58f7058a4f22a8b7dd2f6c32e4eff2cec33f430ab57a4` |
| Shared ownership | `c7682b0077be011c72e8f265b0985f170e1650fcdb5c5a9c0028a8ee6f68af49` | `9db39a8ddee100d2b734005fb2aec853a7e17e50892190fe7af5e02ea01b240f` |

No shared-ownership settings publication was observed. Its missing visible
diagnosis is G7. Screenshots: `A-too-large.png`, `A-shared-ownership.png`.

## Findings

### G2 — major — candidate exceeds 100 MiB private working set

Reproduce the full-size two-computer lab above, enable settings, use settings
and recovery GUI flows and continue ordinary code publication. Both an
initial candidate process and the third fresh candidate process exceeded
the gate. This was not the old v0.3.3 peer's memory.

- PID 37220: **105,607,168 bytes = 100.715 MiB**, at
  11:59:59.9114935Z, around a completed apply and GUI PauseSettings command;
  four consecutive samples exceeded 100 MiB in that interval.
- PID 11260: **110,989,312 bytes = 105.848 MiB**, at
  12:16:22.5052603Z during the code-compatibility publication period,
  with settings still on. Associated private commit was 183,279,616 bytes
  (**174.789 MiB**, not gated).

Do not replace these values with the lower formal hidden-run maximum.

### G3 — major — intentional settings choice has no distinguishable sources

1. Keep A's inert fake client running; edit A's settings.
2. Edit B's settings and use its GUI Share my settings.
3. End only the owned fake client; allow A's stable session capture.
4. Open Settings sync on the candidate.

The unit says “Choose which settings to keep” but both controls read exactly
**“Keep the copy from 25 November”**. There is no computer, kind or time to
distinguish them. The headline simultaneously says **“This computer is up to
date”**; Needs attention does not present a named settings decision.

The underlying safety behavior did preserve both choices: first-trial live
hashes remained A `1dc8c0dc…05719e0` and B `1ab6810b…c46541`, unchanged
from their pre-conflict hashes. Unrelated CheckPause work reached both
computers at
`68ef53fa71e53966183d3db6770ad20dc7dacd378269f80bb3488752ff1295cf`.
A GUI choice resolved the first trial; a second trial reproduced the
indistinguishable controls. Screenshot:
`A-A14-indistinguishable-choices.png`.

### G4 — major — no all-settings pause in the GUI

Open Settings sync and inspect the Pause settings section through its end.
It offers one Pause settings button per folder, but no all-settings pause.
The toolbar's Pause syncing is not an equivalent: it also pauses code.

Per-addon protection did work once the GUI command was confirmed:
paused B retained its experiment at `8b003078…a6fa6c4`, while A published
`1ffb5abb…88e031`. Resume explained replacement and offered “Use shared,
keep mine as a recovery copy” and “Stay paused”. This partial success does
not satisfy A-15's separate all-settings obligation.
Screenshots: `B-settings-bottom.png`, `B-paused-confirm.png`,
`B-resume-choices.png`.

### G5 — major — settings change history is incomplete/misleading

After B's adoption/apply, open CheckSettings detail and scroll to Recent
changes: **“No changes to this add-on have been recorded yet.”** This was
shown despite corresponding settings-applied records in changes.jsonl.
Activity entries do exist, with source, hash versions, file delta and reason,
and remained visible after all three fresh-process restarts.

GUI recovery is recorded as “Saved settings shared” / “shared from this
computer”; the receiving restore is labelled **“settings master”**, rather
than identifying the deliberate restoration. The recovery lists also
present multiple same-day copies without source/size distinctions. Thus
successful byte recovery does not prove the required history explanation.

Screenshots: `B-missing-detail.png`, `B-settings-activity-expanded.png`,
`B-activity-after-restart.png`.

### G6 — major — history names and log paths violate the privacy gate

After sharing a character-scoped file, scan the persisted change history
and diagnostic logs for the planted synthetic identity tokens, absolute
drive paths, and synthetic file-content canaries:

| File class | Identity-token matches | Absolute-path matches | Tested content-canary matches |
|---|---:|---:|---:|
| A changes.jsonl | 66 | 0 | 0 |
| B changes.jsonl | 2 | 0 | 0 |
| A diagnostic log | 0 | 6 | 0 |
| B diagnostic log | 0 | 7 | 0 |

History's `identity` field includes the synthetic character and realm
names. Logs expose absolute lab state/inspection paths. The B log includes
the later v0.3.3 peer startup, but the earlier candidate-only scan already
found three absolute-path occurrences in each log. No private source file
contents are reproduced here.

### G7 — minor — shared-ownership refusal lacks its required explanation

Install CheckSharedOne and CheckSharedTwo with the same SavedVariables
declaration, and different synthetic saved-file bytes on A and B. After
sharing, list and detail show **“Not shared yet”**, not “owned by several
add-ons” or the competing owners. Bytes remain different as above, so this
is a visibility failure, not observed unsafe transfer.
Screenshot: `A-shared-ownership.png`.

### G8 — note — write allow-list needs explicit internal-state treatment

The app writes journals, locks, staging, recovery and settings storage
under each lab variant's `.wowsync`, outside both WTF SavedVariables and
the explicitly passed state directories. These are documented local state,
not evidence of an escape to a real installation. Nevertheless, they cannot
silently be called inside a literal WTF/explicit-state/provider-only
allow-list. Code-compatibility experiments also intentionally require
AddOns writes. This report does not change the gate or move recovery data.

## Memory: three separate complete full-size runs

Each run launched fresh candidate processes A and B, with settings enabled
and the full 118-folder lab. Sampled 35 s startup/settle, 10 s idle, then
55 s capture/publish/apply. The pass changed a byte in A's **33,449,574-byte**
settings fixture and edited B's exception-authority settings. Both
directions converged in every run.

Native GetProcessMemoryInfo extended counters supplied private working set,
private commit and total working set. Private working set was cross-checked
against the Windows PDH “Working Set - Private” counter for the captured PID.
The formal sampler slept 20 ms between paired samples (approximately
31 ms effective interval); a second continuous sampler slept 100 ms.
Values are per logical computer's app process; simultaneous lab peers are
not a claim that two app instances on one real computer fit 100 MiB.
No helper offload was configured. A root process alone exceeding 100 MiB
is sufficient to fail any aggregate gate.

| Run / computer / PID | Idle peak private WS MiB | Pass peak private WS MiB | Startup peak private WS MiB | Highest private commit MiB |
|---|---:|---:|---:|---:|
| 1 / A / 36104 | 77.359 | 81.848 | 80.953 | 110.180 |
| 1 / B / 23932 | 79.836 | 82.363 | 83.320 | 109.340 |
| 2 / A / 10540 | 79.363 | 81.895 | 83.020 | 109.379 |
| 2 / B / 6388 | 76.949 | 83.727 | 81.852 | 109.879 |
| 3 / A / 11260 | 82.250 | 81.953 | 82.668 | 109.078 |
| 3 / B / 27260 | 74.840 | 81.824 | 81.383 | 109.727 |

The formal runs start hidden, as background measurements. Later GUI use and
continued full-lab work remain part of the continuous observation and are
not discarded. **Overall highest candidate private WS: 105.848 MiB; FAIL.**

| Run | A and B settings SHA-256 after apply |
|---|---|
| 1 | `fcfced85b0d1a993b47cc5d0471695f3450add7c75435e420ce5d3e4eff179fe` |
| 2 | `a6616874a646202cfcdf3f39a059bfd8e7ecf65fb1f7abc02e96edf845e5ad71` |
| 3 | `d6bc824889577157eb2dd6a5b1ab516b373cb8d2d0772c0d1a0c51b27387560d` |

The reverse-direction exception settings also matched in every run;
full hashes are in `memory-run-hashes.json`.

## v0.3.3 compatibility: partial, not qualified

Stopped the candidate B process, then launched the downloaded v0.3.3
executable against the **same lab B state/install/provider**, not against
default state. Its GUI opened the existing lab group. A remained the
candidate. Six operations were planned; only two completed before G1 stop.

| Direction / operation | Result | Source and receiver hash evidence |
|---|---|---|
| Candidate → v0.3.3 update | PASS | CheckSettings 1.0 → 2.0; TOC and main.lua matched; 16.1 s observed convergence. |
| Candidate → v0.3.3 addition | PASS | CompatA absent → 1.0; both files matched; 14.0 s observed convergence. |
| Candidate → v0.3.3 removal | BLOCKED | A removal was published; B still held CompatA when stopped. No completion claim. |
| v0.3.3 → candidate update | NOT RUN | Stopped at G1 barrier. |
| v0.3.3 → candidate addition | NOT RUN | Stopped at G1 barrier. |
| v0.3.3 → candidate removal | NOT RUN | Stopped at G1 barrier. |

Update hashes, identical at source and receiver:

- TOC before:
  `090974cc10930979168fce9ba6fb6823dc34603ebb1450632be5ddafbd84215b`;
  after:
  `7acc29c7de93b005132d7ccc8991d4970fbcd1fbe58bbe19ff9c83e35ca6fc4e`.
- main.lua before:
  `d57acff21e402cb5c7e47bcc1021facde340b59e0b6d7aae871274e51dc15f17`;
  after:
  `10ac639107d46f68e002d76eae17531b898e99b08fd1d701268219ebb45b14f5`.

Addition hashes, identical at source and receiver, receiver previously absent:

- TOC:
  `73b13e004d113be09d915dc876b4c7e2b9e9ec553e6050d01b503b64e293f95a`.
- main.lua:
  `8deac3450c0c3988bbd764f56aeb1b183073ec631de11e36bf02448dfbfc9e1b`.

## Write audit and retained evidence

Five FileSystemWatchers monitored A, B, their two explicit state directories,
and the provider. No watcher overflow/error was reported. They started
after fixture copying and initial enrollment had begun, so transient writes
before monitoring are not comprehensively observed. G1 is also outside their
scope: **this is not proof of no protected-path writes**.

The complete de-duplicated list of **62,340 observed changed paths**, including
renames, transient files and directory notifications, is retained as
`changed-paths.txt`; the complete chronological events are in
`write-events.tsv`. Paths are relative to LAB in the de-duplicated list.
Counts are paths, not a claim that every notification names a regular file.

| Class | Unique paths |
|---|---:|
| Provider | 30,567 |
| Explicit state directories | 3,558 |
| Variant internal `.wowsync` state, including transient names | 28,174 |
| SavedVariables shapes | 16 |
| Code fixture / compatibility paths | 13 |
| Verifier's inert fake-client file | 1 |
| Other directory notifications | 11 |

Full SHA-256 manifests of every AddOns file were compared before/after.
All **113 original copied add-on folders** remained byte-identical.
Persistent code differences were only the deliberate CheckSettings update,
CheckNewAuto fixture, and B's still-present CompatA after interrupted removal.
The exact ten changed/new regular-file rows are in `code-diff.csv`.
Unsupported WTF files were neither copied nor observed written by the lab
instances. The verifier-created identity folder and code/fake-client fixtures
are explicitly separate from app settings writes.

Raw evidence digests, SHA-256:

| LAB file | Hash |
|---|---|
| memory-runs.csv | `78d1ff198947795848bb91a718532f8258708ff7a5280018c4326bc430a58032` |
| continuous-memory.csv | `36aa1a107667b82a3b5e39478912e569c8fac39110ebb95fa9cd3eef8a7817a5` |
| write-events.tsv | `daf2e989ccd8b61baded97f4520986e79cf045dadf3f99ed7ad18532cc5fb3c3` |
| changed-paths.txt | `43e23618ec7d11515042f702f4c0f44b7804d8efd0495cb2149a7b2963d65e4c` |
| code-diff.csv | `ad0fbc01fb2a32a595b2988febcee49a6188c7653131bf5ca557fa824d902aff` |
| compat-results.json | `15fd0498be8153fcdee5d7407310d03b16c70d20e94217d714732171ba20551e` |
| privacy-final.csv | `2921e78651b9590f473232b30c6ae04fe54906afce2c5e29eda05c3c613da0bf` |

## Repository verification and cleanup

- `cargo xtask verify --tier offline`: PASS, including fmt, clippy, tests,
  required cases, deny and markdown links.
- `cargo xtask verify --tier native`: PASS.
- Final offline verification was rerun after writing this file: PASS.
- All recorded lab candidate/peer/fake-client processes were stopped; no
  recorded PID with its original creation time remained alive. Both bounded
  monitoring/compatibility shell sessions were stopped.
- `git -C D:\GitHub\wow-sync status --short`: empty. This establishes main
  checkout cleanliness, **not** the missing protected-state assurance in G1.
- Only this evidence file is committed on the worker branch; no push.
- Remaining work: coordinator incident review, fixes/review for G2–G7, then
  a newly authorized clean artifact verification including all six
  compatibility operations and the full safety audit.

## Re-verification (40fad41)

Date: 2026-11-25. **The fixed candidate is NOT release-qualified.**
The table below supersedes the earlier candidate's results, not its retained
observations. All eleven requested gates were exercised. New major findings
G11 and G12 block release; successful retries do not erase them.

Authority: the coordinator's explicitly authorized re-verification and
[OD-40 through OD-46](../DECISIONS-FOR-REVIEW.md). In particular, OD-42 permits
variant-local state, OD-43 accepts developer startup/inspect path lines,
OD-44 permits identity in local history, and OD-45 gates **every hidden-window
private-working-set sample, including after GUI use**. Visible-window peaks
are reported separately. G1's earlier impact was reviewed by the coordinator
and recorded in OD-41; it was not re-investigated against protected data.

### Fixed artifact, environment and isolation

- Same Windows 11 Pro x64 10.0.26300 / i7-13700HX / NTFS environment.
  Checkout `D:\GitHub\wow-sync.worktrees\R11-check`, branch `run011/R11-check`.
  Merged requested main `40fad41` in worker merge `62e7d1d`; no product code
  or product tests were authored by this verification worker.
- Downloaded release-dispatch artifact **45636710359** into LAB `artifact2`.
  Requested and observed build SHA:
  `40fad41a1bc823b1a5b40a27329adb6d9f071226`.
  The GUI diagnostics preview also displayed that full SHA.
- ZIP SHA-256, independently recomputed and equal to its sidecar:
  `7c1a1e2aee214904fcc629847db1c45922ef44d161ca78f23915e0b75c0adb9a`.
  Extracted `candidate2\wow-sync.exe` SHA-256:
  `01c8d84a1adfe2f2c0a404e098d54a9114db4bc919673ee470bb32e934c5d8d0`.
  All candidate observations used this downloaded executable, not a local build.
- Reused the published v0.3.3 ZIP/peer from the first lab. Recomputed ZIP
  SHA-256 still matches its sidecar:
  `ce0445f9e7748de7023daf4f585348f98c9bf93094bdbb747d9a99c14cef9697`.
- Created fresh `round2\A`, `round2\B`, state directories and provider inside
  `D:\WoWSyncLab\R11-check` (LAB). Copied only the retained lab's AddOns and
  scoped WTF fixture trees, plus synthetic install-identification files.
  Did **not** copy previous `.wowsync`, enrollment, state or provider data.
  Each computer started with **119 code folders / 36,154 files /
  3,772,227,171 bytes**, including six synthetic fixtures in the full-size copy.
  B's synthetic missing-character folder was removed before monitoring/launch.
- All **nine** application starts, including the v0.3.3 peer, went through
  `r2-common.ps1`'s guard. Before each start it printed the complete command
  and rejected any executable/install/state/provider path outside LAB or a
  missing `--dev`. `round2\launches.jsonl` retains the commands. No unscoped
  probes or help launches occurred.
- Child profile environment was additionally redirected into each lab state
  before using GUI diagnostics export. The first, outside-lab default export
  destination was previewed but **not saved**; the actual Save was performed
  only after the preview visibly named the lab-only destination.
- The only fake game clients were inert copies of Windows PING named Wow.exe
  inside A's lab variant, with owned PIDs recorded before use. No addon Lua,
  SavedVariables or legacy CMD metadata was executed. No protected default
  state, owner process, real game folder, real provider or demo data was used.

Watchers were ready at **15:07:45.8850072Z**, before the first guarded launch
at **15:07:56Z**. They remained active through final application shutdown.

### Fresh gate table

| Gate | Result | Observed evidence |
|---|---|---|
| 1. Dispatch hash and artifact smoke | PASS | ZIP/sidecar equality above; both downloaded GUIs completed enrollment and actual settings transfers. |
| 2. Guided offers and consent | PASS | Both fresh computers displayed the offer. Account exclusion/re-inclusion, disabled sharing with no account, master choice, Share these now, A's Share settings and B's named-master Use settings were observed. |
| 3. A-11 through A-15 in GUI | PASS | Completed-session capture, B's CheckPause exception authority, routine-peer yield/recovery, explicit peer promotion, distinguishable master/peer choices, headline/Needs attention, global pause/resume and per-addon pause all exercised. The additional restore/choice failure is separately retained as G11. |
| 4. Missing character | PASS | B detail visibly said waiting; folder stayed absent until verifier-created at 16:20:46Z. App then created only the SavedVariables file, matching A's SHA-256. |
| 5. Over-cap/shared ownership shown and excluded | FAIL | Both remained excluded, with different peer hashes and no settings-revision records. Over-cap explanation passed. Shared ownership still reads Not shared yet, not Shared with the other addon, not synced. G9. |
| 6. Activity/history/recovery metadata/restart | FAIL | Settings Recent changes, source/reason/size and restore wording on BOTH computers now pass and survive fresh restarts. Local replaced-copy controls are rich. Shared-checkpoint recovery controls still omit source, kind and size. G10. |
| 7. GUI recovery, including peer replaced copy | FAIL | First-adoption and later B recovery copies successfully restored to both computers through the GUI. A separate accepted master restore/choice left A on different bytes until an additional Use shared settings action. G11. |
| 8. Logs/history/GUI diagnostics privacy | PASS | All 1,205 retained log lines scanned: no tested identity/content canary, no runtime/settings/sync path. Fifteen accepted startup/inspect path lines listed below. GUI-saved diagnostics with settings and character units present has no identity, path or content canary. Local history identities are permitted by OD-44. |
| 9. Pre-launch write audit | PASS | No overflow/error or unexpected install-file path in the bounded audit. Full path list includes approved state/provider/SavedVariables/.wowsync, plus separately enumerated authorized code tests and verifier fixtures. Scope and directory-move handling are detailed below. |
| 10. Three fresh full-lab hidden-after-UI memory runs | FAIL | All three complete; zooms 1.0, 1.0, 1.5. Highest hidden private WS **104.0625 MiB**. Runs 2 and 3 exceed 100 MiB, including user-idle samples. G12. |
| 11. v0.3.3 code compatibility, all six operations | PASS | Update/add/remove succeeded candidate to v0.3.3 and reverse, with per-file source/receiver hashes and absent/absent removals. |
| Exact-SHA CI and release dispatch | PASS | CI **45632760846**: Windows/macOS offline and all four architecture build-only jobs succeeded. Dispatch **45636710359** succeeded. This is not a macOS runtime claim. |
| No open critical/major findings | FAIL | New major G11 and G12 remain unresolved; no product fixes attempted. |

### Functional observations and hashes

A's initial inspection took 307,732 ms; B's took 324,334 ms. B was restarted
once during enrollment to discover the new group; its later cached inspect
took 211 ms. These are actual timings, not an additional approved numeric gate.

Representative screenshots are under `round2\shots`: `A-guided-offer`,
`B-guided-offer`, consent/pause screenshots, `A-A14-needs-attention`,
`A-A14-distinct-controls`, `B-per-addon-correct-resume`,
`B-missing-before/after`, `A/B-restore-after-restart`,
`A-activity-expanded-after-restart` and `B-recovery-persisted-rich` (all PNG).
The following raw JSON/JSONL files retain the corresponding observations.

- **A-11:** B was selected through the GUI as the CheckPause exception
  master while A remained default master; the published policy confirms it.
  Each full hidden run also transferred a B CheckPause edit to A.
  In a separate completed-session check, eight A writes during the inert
  game fixture left CheckSettings publication count **23 → 23** and B's
  hash unchanged. After the fixture exited there was one publication,
  **23 → 24**, and both computers adopted
  `a1cdaafe62fc1825c2ef364d2d18cc52367f8aa6267fd6f792ae706815a0fd2a`.
  See `session-boundary.json` and the action/owned-process ledgers.
- **A-12:** a routine B edit did not publish automatically; B's
  CheckSettings publication count stayed **5 → 5** while a later master
  revision replaced it. In the settled repeat, the peer's 60-byte copy,
  `e8bb6302becc2566c8c49047d65ea2ae5d540a5b1c44938b2366768ea771533b`,
  yielded to A's
  `f81dc98acd09a88e882e9e362b5db2c44d8be0cabc41fe9ac96c81102b665489`.
  The GUI listed the 08:23, 60 B replaced peer copy with source/reason.
- **A-13 / peer recovery:** explicitly restoring that B copy through the
  GUI promoted it to both computers, each matching the full `e8bb…1533b`
  SHA-256 above. Both histories record `RestoredFromRecovery`; A names
  RecheckB, and B says it restored this computer's earlier copy. See
  `stable-routine-yield.json`, `peer-recovery-push.json` and restart screenshots.
  Earlier actual B Share my settings operations also produced CheckSettings
  `SharedFromHere` records during the intentional-conflict scenarios.
- **A-14:** the primary master-versus-intentional-peer choices include kind,
  source, date/time and size. For example, the actual controls were
  “Keep the copy shared from RecheckB (25 November 07:51, 55 B)” and
  “Keep the copy from settings master RecheckA (25 November 07:52, 64 B)”.
  The headline read “Choose which saved settings to keep”; Needs attention
  named the addon and said both copies were kept until a choice. Accepted
  primary choices converged. The distinct restore-related failure below
  is not waived by those successful choices.
- **A-15:** Pause all settings on **both** computers, edit both, wait 25 s:
  hashes and history counts stayed unchanged (A 70 / B 6 records).
  Resume all settings exposed pending share/use choices. Per-addon pause
  likewise preserved B's bytes while A changed; resume offered
  “Use shared, keep mine as a recovery copy” and Stay paused. The pause-list
  index and exception-list index are different; an initial wrong-row
  harness action was reversed and not counted as the CheckSettings test.
- **Missing character:** at 16:20:46.549Z, B's folder was still absent and
  detail visibly said “Waiting for LABMISSING on LABREALM1 to log in on this
  computer”. The verifier created the character/SavedVariables directories
  at 16:20:46.564Z. The app created CheckSettings.lua at 16:20:50.881Z.
  A and B then matched
  `111a9ee7a29c69e372c71220f4294d3cb58d73f31ac4085ae3d607d008679b28`.
  No earlier B character-folder creation appears in the watcher stream.

Other useful before/after hashes:

| Scenario | A SHA-256 | B SHA-256 / observation |
|---|---|---|
| Initial CheckSettings, before consent | `2bf169a7ea11b53003323b4b9cee897df79fb4655b4e8f705f355ffbe5cabd73` | `d3018600f8458a59e49106a63a93e6490dc0e68afe8f0067da78c47e136d01c5` |
| First adoption, then GUI restore of B's original | A copy first applied on B; subsequent restore selected B's original | Both then matched `d3018600f8458a59e49106a63a93e6490dc0e68afe8f0067da78c47e136d01c5` |
| Global pause, unchanged during the wait | `791696fea359d6bd4d9cb0e44a7be5475d9713a79235395afe271ecfd0a8bd15` | `ba7ee5e602c6fd04ae54db0f052c7ee91135d8a2e350ef9646e96749f6f09adb` |
| Per-addon pause, B protected | `8b5a7fdaa9d68b53f33577493b938489ce8c4a0c32b86fc9680d80813a2a2f38` | `f3d779851cede68fb1ac239b2d89834fc065f117cb6c2c3db379aea844a4deef` |
| Over-cap file, both 33,554,433 bytes | `bdf7fbb54387c24608fd757a1b31c32cabcd7b6bee8ff3345c897139fffc072a` | `e017d0ade6150ffd8758d064697f21a276d2e0776fbcc7f480a566c2b3e25f30` |
| Shared-owner file, 38 B / 57 B | `c7682b0077be011c72e8f265b0985f170e1650fcdb5c5a9c0028a8ee6f68af49` | `5399d1d3ab0cf81f7acd5849826a6271e4454fe46dec1a81ade0b26112783aee` |

The last two rows stayed unchanged in `blocked-before.json` versus
`blocked-after.json`. The provider's **212** settings-revision records contain
no CheckLarge, CheckSharedOne or CheckSharedTwo unit. The over-cap detail
said “Too large to share automatically (32.0 MB)”. For newly discovered
memory-fixture addons, initial settings were not claimed as automatically
shared: after discovery, a second synthetic completed-session edit was made,
then actual publication/application and matching hashes were observed.

UIA does not expose all ordinary descriptive text, and its offscreen flags
do not reliably reflect nested clipping. Missing UIA text was checked in
screenshots, not treated as missing product UI. In particular, settings
Recent changes **are present**. At zoom 1.5 the harness needed to scroll both
the outer page and inner addon list; silent/clipped invocations without an
accepted action were not counted. One physical-click attempt was refused
by the owned-window hit-test guard, with no click performed. Subsequent
interaction used UIA/posted messages to owned windows. The run-3 share that
was actually accepted targeted retained R2Mem1 settings; its subsequent
CheckSettings choose and restore were independently accepted and verified.
Likewise, `peer-push.json` alone is **not** proof of the later intended
CheckSettings push: that generic button hit an orphan R2Mem1 row. The valid
peer promotion/recovery evidence is the explicit, hash-checked flow above.

### New findings

**G9 — minor: shared-owner exclusion still lacks its explanation.**
Reproduce with CheckSharedOne and CheckSharedTwo both declaring the same
CheckShared saved-variable name, enable settings, then open either addon.
The installed TOC declarations and distinct A/B fixture bytes were checked.
The detail says **“Not shared yet”**, not “Shared with …, not synced”.
Exclusion itself is safe: neither file transferred and no settings revision
for either owner exists. Observed again after restart in
`G9-shared-owner-after-restart.png`; the over-cap explanation works beside it.

**G10 — minor: shared-checkpoint recovery choices still lack metadata.**
Open CheckSettings Recovery copies or its Settings sync controls after
several revisions. Shared checkpoints are offered only as “Restore the copy
from 25 November 07:35…”, including duplicate minute labels, without source,
kind or size; expanding one still omits those fields. This is distinct from
the **fixed**, richly labelled local replaced-copy controls, e.g.
“Restore this computer's copy from 25 November 08:23, 60 B, replaced by the
settings master from RecheckA…”. The contrast survives restart; see
`memory3-restore-confirm.png` and `B-recovery-persisted-rich.png`.

**G11 — major: an accepted restore/choice can leave the choosing master
unchanged while its peer applies the selection.**

1. In fresh-process memory run 2, complete a peer-versus-master decision by
   choosing the peer's 55-byte copy; actual A/B convergence was observed.
2. On A's CheckSettings detail, restore the original 49-byte shared copy.
   At **15:52:42Z**, RestoreSettings accepted record `2269a9218cbf`.
   It produced a new decision between two “settings master RecheckA” copies
   at 07:52, sizes **55 B** and **49 B**. Neither changed for over 130 s.
3. Explicitly choose the **49 B** option. At **15:56:09Z**, ChooseSettings
   accepted `a2096897b419`; headline returned to up to date.
   B applied it at **15:56:13Z** and had the original
   `2bf169a7ea11b53003323b4b9cee897df79fb4655b4e8f705f355ffbe5cabd73`.
   A retained the 55-byte
   `fdce489af0c31085ff46c282287a3412ec40162dfc2192c0f26dadadcb1ea690`
   for another over-130-second observation.
4. A's unit said “Changed here, not shared”, offering Share my settings and
   Use shared settings despite the up-to-date headline. Only the additional
   **Use shared settings** GUI action at **15:58:52Z** made A adopt the choice.

The raw accepted-action logs, peer apply log, fixture SHA-256, exact choice
labels in `ui-dumps.txt`, and `memory2-after-second-choice.png` corroborate
this. The latter shows the headline/unit disagreement; the separate
`memory2-restore-choice.png` shows the pending headline but is scrolled away
from the choice controls. This was not a failed/clipped automation click:
both restore and choice have accepted record IDs. Later run-3 and B recovery
flows succeeded, so the defect is not claimed to affect every restore.
The cause was not established.

**G12 — major: genuine hidden-after-UI private-WS violations in two runs.**
Follow the full scenario below, hide to tray, then perform settings and code
passes. PID 12032 reached **109,117,440 bytes / 104.0625 MiB**, with all five
top-level windows hidden. PID 23688 independently reached **108,867,584 bytes /
103.82421875 MiB**, also all five hidden. Both also exceeded the limit in
explicit user-idle phases. These are OD-45-gated samples, not visible peaks.
No working-set trimming, memory-limit changes, sample exclusions or medians
were used to turn them into a pass.

### Three complete memory runs

Settings stayed on, with the full lab in each fresh process. Each started
visible, used Settings sync and an addon detail, performed an accepted share,
intentional choice and GUI restore, then hid to the tray. Hidden work included
settle/user-idle, a **33,449,574-byte** CheckSettings capture/apply, code
update/add/remove, a post-discovery settings edit for the new addon, incoming
CheckPause exception settings, and final user-idle. Actual hashes for all
settings and code passes are in `settings-results.jsonl` and
`code-results.jsonl`; all three full sequences completed.
Runs reused the sequentially evolved round2 enrollment, state, provider and
history, not reset data: CheckSettings code started at 2.0, then 4.1 and 4.2
respectively; the preceding run's temporary addon had been removed before
the next process started.

Native EnumWindows/GetWindowThreadProcessId/IsWindowVisible was sampled
alongside GetProcessMemoryInfo private working set and private commit.
Every row records PID, phase, top-level window count, visible count and
ever-visible status. Hidden means **zero visible top-level windows**, not
minimized, obscured or an assumed command-line mode. Sampling requested
25 ms; observed same-PID median was 29.75–29.81 ms, p95 43.21–43.33 ms,
maximum gap 1.020 s. These are sampled peaks, not a claim of continuous
allocation-level measurement.

All values below are MiB. “Idle” means labelled no-intentional-edit intervals;
normal periodic background checking continued.

| Fresh run / PID / zoom | Hidden samples | Hidden peak | Hidden after UI peak | Hidden user-idle peak after UI | Visible peak | Private commit peak |
|---|---:|---:|---:|---:|---:|---:|
| 1 / 26432 / 1.0 | 13,681 | 99.211 | 99.211 | 98.270 | 102.172 | 186.070 |
| 2 / 12032 / 1.0 | 7,838 | **104.0625** | **104.0625** | **100.285** | 102.547 | 167.516 |
| 3 / 23688 / 1.5 | 8,390 | **103.824** | **103.824** | **100.820** | 104.027 | 166.840 |

- Gate maximum: **104.0625 MiB**, not the lowest run or a median.
- **248 hidden samples exceeded 100 MiB:** 45 in run 2, 203 in run 3.
  All had previously shown the UI. Run 2's peak was
  **16:00:36.4619990Z**, harness phase `hidden-code-update`; run 3's was
  **16:18:28.0942246Z**, phase `hidden-code-remove`.
  The former followed the large settings publication and preceded the
  logged 4.1→4.2 publication; the latter followed the new-addon settings
  publication and preceded the logged R2Mem3 removal. These phase/log
  correlations do not isolate an allocator or prove a GC cause.
- Visible peaks in the table are **not gated**. An additional visible
  functional check in run 3's same process is included in its visible peak;
  its formal scenario alone peaked visibly at 103.938 MiB. The visible
  reference-peer candidate peak was 105.566 MiB. The old v0.3.3 peer's
  measurements are separately labelled `compat033`, not candidate results.
- Private commit is recorded, not compared to 100 MiB (OD-38).
  Run 1's window was resized to 1500×1050 during GUI use; runs 2/3 used
  the normal lab window, with native Windows DPI 144. Run 3's zoom stayed
  1.5 throughout its formal visible/hidden scenario.

### All six v0.3.3 compatibility operations

Candidate A and old B reused the enrolled round2 group; B was stopped by
identity-checked PID before the guarded old-peer start (**PID 17128**).
The old peer's startup log and published state both identify v0.3.3.
All six tests finished; no earlier interrupted compatibility result is
being reused. The two hashes in each non-removal row are respectively
the TOC and main.lua SHA-256, **identical on source and receiver**.
`code-results.jsonl` also retains each receiver's before-hash.

| Direction | Operation | Observed after state / hashes on BOTH sides | Wait |
|---|---|---|---:|
| Candidate → v0.3.3 | CheckSettings 4.3 → 5.0 | TOC `8ae0fe122e5f37d62eec35392bc801c74e40e281bbbd3aed262ffbad7345772c`; Lua `cb2988019a7189dd37a38ca7f3a465c582a845274dea0922a299688fad76f5a5` | 16 s |
| Candidate → v0.3.3 | Add R2CompatA 1.0 | TOC `24d392dc625be5d9060ff57f92cdde91c980175f7478d601d0bc76e23fa79e83`; Lua `2325ccea51554002c044fc6012cb91c7094fb3ab94d24b749ce588150b99bb6c` | 16 s |
| Candidate → v0.3.3 | Remove R2CompatA | Source ABSENT / receiver ABSENT; receiver before-hashes are the addition row | 38 s |
| v0.3.3 → candidate | CheckSettings 5.0 → 6.0 | TOC `f172f10f835142b892daf1f0f307079b20af4cff7ae8888ede1e9a2f743d990a`; Lua `dcc95cd7fbd814bb65eb29d9fe57d03da880998de6219fdf40f2038d131769f1` | 28 s |
| v0.3.3 → candidate | Add R2CompatB 1.0 | TOC `52abff67236c55063896713ca61da7d8b77ee4e29e342247feb695218fbac8d5`; Lua `c57cfc8f8000682e2f981799379eba695378110a692e4cd8623893f5118b5b2e` | 16 s |
| v0.3.3 → candidate | Remove R2CompatB | Source ABSENT / receiver ABSENT; receiver before-hashes are the addition row | 52 s |

### Privacy, GUI export and accepted path lines

The actual saved GUI export is
`round2\A-state\profile\Documents\wow-sync-diagnostics-20261125-153108-229.txt`,
69 lines, SHA-256
`9b36df26178eb6a5e625f89aa4d5c131358925b37cc5b01a0a227e2e696ab18b`.
Settings were on, with account and character-scoped units present.
Default export options included addon folder names, not file fingerprints.
The preview's destination was checked before Save
(`A-export-save-destination.png`). The saved output has no tested account,
realm or character canary, content canary, drive-qualified path or UNC path.
Local changes.jsonl does contain scoped identity names, as OD-44 permits.
No claim is made that structured provider protocol identities are absent.

The final scan covered both retained state logs, including their restarts
and the old peer. All **15** absolute-path lines follow. LAB replaces the
approved lab root; none contains an account/realm/character name. Startup
lines are **developer startup**; inspect lines are **setup/inspect**.
There are **zero** absolute-path lines from sync, settings or runtime work.

```text
2026-11-25T15:07:56Z pid=20404 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T15:08:22Z pid=20404 INFO inspect start <LAB>\round2\A\_forever_
2026-11-25T15:13:29Z pid=20404 INFO inspect done <LAB>\round2\A\_forever_ in 307732 ms: 119 add-on(s) eligible, 0 blocked, 36154 file(s) read
2026-11-25T15:30:51Z pid=26432 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T15:50:53Z pid=12032 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T16:03:34Z pid=23688 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T16:26:45Z pid=36240 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T15:08:28Z pid=5100 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T15:08:44Z pid=5100 INFO inspect start <LAB>\round2\B\_forever_
2026-11-25T15:14:09Z pid=5100 INFO inspect done <LAB>\round2\B\_forever_ in 324334 ms: 119 add-on(s) eligible, 0 blocked, 36154 file(s) read
2026-11-25T15:18:06Z pid=27324 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T15:18:50Z pid=27324 INFO inspect start <LAB>\round2\B\_forever_
2026-11-25T15:18:50Z pid=27324 INFO inspect done <LAB>\round2\B\_forever_ in 211 ms: 119 add-on(s) eligible, 0 blocked, 36154 file(s) read
2026-11-25T16:26:45Z pid=17912 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T16:30:10Z pid=17128 INFO WoW Sync 0.3.3 starting, dev=true, state <LAB>\round2\B-state
```

### Write audit, retained evidence and shutdown

The pre-launch watchers covered A, B, both explicit states and the provider,
recursively, with 64 KiB buffers and an explicit Error handler. They recorded
**898,921 notifications**, **zero errors/overflows**, and **194,313 distinct
notified paths**. `write-notified-paths.tsv` retains that exact path set;
`write-events.tsv` retains timestamps, event kinds and both rename names.
Atomic directory installs need not notify every child at its destination,
so the final `write-paths.tsv` additionally includes **ten** destination paths
actually enumerated and hashed by the code-transfer checks: **194,323 paths**
in total. This is a bounded lab filesystem audit, not a machine-wide/PID-
attributed trace or proof about unmonitored operating-system cache writes.

| Path category | Distinct listed paths |
|---|---:|
| Explicit lab state | 11,554 |
| Synthetic provider | 73,229 |
| Variant `.wowsync` local state, including transient paths | 109,481 |
| SavedVariables shapes | 16 |
| Authorized synthetic code test/apply paths | 38 |
| Verifier's inert Wow.exe fixture | 1 |
| Install-directory notifications | 4 |

No unexpected install file was observed. Code paths are confined to
CheckSettings, R2Mem1/2/3, R2CompatA/B and their AddOns parent directories;
none of the 113 original copied addon folders had a write event.
Code test/apply writes are separated from settings-write scope rather than
misrepresented as SavedVariables. The four other notifications are A/B's
variant directories and B's synthetic realm/missing-character directories.
The character directories were explicitly verifier-created, not app-created.
The only blocked-file writes were the verifier's B CheckShared edit
(two notifications at 15:15:07Z) and B CheckLarge mutation (16:29:04Z);
there were no subsequent writes to either blocked fixture.

Raw evidence and the exact harnesses remain inside LAB, not committed or
uploaded. `round2\raw-evidence-sha256.txt` seals **109** evidence/harness files
(paths relative to LAB); its own SHA-256 is
`9bf50647c8c347c8a56591f679fd9aa70b7b561ae586d7cd27ee35101fab70dc`.

| round2 raw evidence | SHA-256 |
|---|---|
| memory.csv | `26f7febb450a003cb2c082b294ce5dd5328b2ee32699e61aa080f0bdc1336e17` |
| memory-summary.json | `8cc496449ed8ee62750dd11c764d19c9396fd2ce6ccf56e0fdd6fda3be5d0fc8` |
| write-events.tsv | `9ddae96d50112e7daf4a95654f97d5526ac00a01b503464b15d7ad3bf195d3ef` |
| write-paths.tsv | `25c16f30aea8987787b430175362a88a8d7aba8bbdf78dc75901d85d6397680a` |
| code-results.jsonl | `5f644ce2ed13e9c9311607725acadbc58683418c34c117a4a9d07d67fd8a84c2` |
| audit-summary.json | `847d5bdbd4d704cbbb2c2a57eff2b17ceb11714419f3bd3aedf8f0513f9fd9ef` |

All recorded app and fake-client processes were stopped by owned PID and
executable/creation identity; none still matched the ledger. The monitor
exited cleanly after final application shutdown; both bounded scenario
drivers completed. PID 8816 was not stopped or otherwise targeted.
Main checkout `git -C D:\GitHub\wow-sync status --short` was empty after
shutdown. Only this evidence file is to be committed; no push.

Local `cargo xtask verify --tier offline`: **PASS**, exit 0, with
`allow-blocked: false`; fmt, clippy, tests, required-cases, deny and
markdown-links all passed on Cargo 1.100.0. Command ran in the worker
checkout, with verification scratch scoped under that checkout's target.
Native verification was not rerun for this fixed-candidate appendix; the
earlier native result above belongs to the earlier checkout/candidate.

## Re-verification 3 (902e720)

### Environment, provenance and scope

Independent verification of dispatch **45692122867**, not a locally built
application. Artifact head:
`902e720179136c235a103617551b97ed1e29ce10`.
The downloaded ZIP's SHA-256 matched its supplied sidecar:
`bab6a6d29864985aafc04411992540e92eff913261d1422a4f99a9100677c3de`.
Extracted `wow-sync.exe`:
`8c61abc140f19836bcc225b9e1740840e104bb5bf85aa521d322cca6e6d019cc`.
The dispatch succeeded; CI **45688966090** succeeded, including macOS
offline verification and the Windows/macOS x86_64/aarch64 build-only jobs.
Those are observed CI results, not claims of local macOS GUI testing.

Host: Microsoft Windows 11 Pro 10.0.26300, build 26300. Rust
`1.100.0 (0ef1bc386 2026-11-09)`; Cargo
`1.100.0 (3acc79c0c 2026-11-05)`. Worker checkout:
`D:\GitHub\wow-sync.worktrees\R11-check`, branch `run011/R11-check`.
The requested main merge fast-forwarded to **8d3070a**, whose change after
902e720 is documentation-only (`run-011: update handoff`). No product,
test, dependency or CI source was changed by this worker.

In this section, **LAB** means `D:\WoWSyncLab\R11-check`. Artifact is in
`artifact3`/`candidate3`; current raw evidence is in `round3`.
The accumulated full A/B installations, provider and local histories in
`round2` were reused, **not reset**: initially each had 119 add-on folders
and 36,154 active add-on files; A/B local history had 125/55 records.
C/D are separate, small synthetic consent/session computers using
`round2\consent-provider`, not members of the full A/B group. Earlier
round-2 raw evidence is preserved in `archive2`.

Every candidate and older-peer launch used `r2-common.ps1`'s guard:
the exact command was printed and recorded before launching; `--dev`,
`--install`, `--state` and `--provider` were mandatory and all path
arguments were checked under LAB. Child profile paths were also scoped.
There were no bare launches or help probes. Synthetic game sessions used
an inert LAB copy of Windows PING.EXE, never WoW or imported Lua/CMD.
Writes used absolute paths. No real group was enabled, no screenshots or
private data were uploaded, and no nested agent was launched.
Policy references: [OD-45–OD-51](../DECISIONS-FOR-REVIEW.md),
[packet](../packets/R11-check.md), [release gates](../PLAN.md).

### Observable settings cases

- **Consent on both computers:** fresh C/D showed the guided offer.
  With no account selected, sharing was disabled. Selected-account
  consent, “Share these now” and master RecheckC were exercised; the
  excluded account stayed excluded. D accepted the matching account and
  received C's bytes. `guided-results.json`: D changed from
  `f136f7ec9c8dc8da0478da55d02fe19f6997c625c408f6e9c0cde8fc418ae232`
  to C/D's
  `0dd65ea6f9cbefa6aa20e7b7076428bbe90806471ad60e05e4edd38f2725e4a6`.
- **A-11, default master and per-add-on exception:** shared master changes
  and the B-master CheckPause exception reached the other computer.
  A newly installed R3NewSettings reached D as code. Its first completed
  game session created an activation capture, **not** a publication:
  D's settings remained absent. The second completed session automatically
  published from C and D received identical bytes, without a Share/apply
  action:
  `bc7eb3d966457dfac9d797f40d1af40dc5042f8bf973b41bc73c83e5daac7c52`.
  Eight writes during each session did not create eight history states.
  This distinguishes activation from later completed-session publication
  in [settings design §4.1](../../../design/settings.md).
- **A-12, routine peer yield:** D's unsent edit remained local, hash
  `04eaddfd23d025491a99ba54e8062696cc03c54105fc48b716fef5b87aee152c`;
  D publications stayed 0→0. After C's next completed game session,
  C/D automatically matched
  `0f14bd8048a4624b965be6a8b07dbb8df9ace720f44f2c8ea7c840bbe7eebc7e`.
  D's replaced 39-byte local copy was offered in GUI recovery, naming
  this computer and RecheckC as the replacing source. UI automation
  observed that control; the screenshot named
  `routine-peer-recovery-visible.png` shows the upper detail, **not**
  the off-screen recovery button.
- **A-13/A-14:** actual CheckSettings “Share my copy” publications from B
  were chosen on master A. The normal competing choices showed kind,
  source, date/time and size, and the headline/Needs attention showed the
  decision. An intentional peer choice was adopted; the rapid subsequent
  restore case remains broken (G13 below).
- **A-15:** “Pause all settings”/“Resume all settings” were exercised.
  Edits on both computers during global pause neither published nor
  applied; resuming offered choices rather than silently discarding them.
  Per-add-on pause also protected local bytes; “Resume…” then
  “Use shared, keep mine as a recovery copy” resumed it safely.
  See `pause-all.json`, `per-addon.json`, actions and UI dumps.
- **Missing character:** the already-shared character folder on B was
  absent before testing, stayed absent while the GUI said waiting, and
  was never created by the application. After the verifier manually
  created the folder, settings applied with matching hashes.
  `missing-existing-before.json`, `missing-existing-after.json`,
  `missing-existing-waiting.png` and watcher events preserve the sequence.
- **Excluded files / former G9:** CheckLarge is 33,554,433 bytes, one byte
  over the 32 MiB cap. It was shown as blocked. CheckSharedOne/Two showed
  **“Shared with …, not synced”**, not “Not shared yet”, both before and
  after restart. A/B blocked bytes retained their original hashes and
  no blocked-unit settings revision was found in the provider.
  Screenshots: `G9-before-restart.png`, `G9-after-restart.png`.
- **History / former G10:** settings appeared in Activity, local history
  and add-on detail Recent changes with source, reason and size. Restores
  read as restores on both computers. The shared-checkpoint recovery
  list and confirmation showed source, kind, date/time (including
  seconds) and size. `G10-shared-entry-confirm.png` is an actual screenshot
  with **Restore and Cancel visibly in frame**, not just UIA text.
  B restart preserved 96 history records and the exact SHA-256
  `9b4a62880c5644b81cdb3a4abe3bab83183d021750b0da92b6d354103bfd398c`.
- **Peer replaced-copy recovery:** selected B's actual local 57-byte
  replaced copy through the GUI. A/B both changed from
  `9c3bddd9cdb9a8d4a6a36331ebfdd32068bc31041127ba8ee300ab57aab4bfca`
  to
  `5f3387d03fcdc124a895496b7d180f79487e0dd00bc9a315e28c5a9dec01ea27`,
  with **no extra choice click**. The exact unspaced before/after hashes
  are in `peer-recovery.json`; confirmation/applied screenshots are
  retained. This positive case does not override the rapid restore failure.

### G13 — major: rapid choose-peer then restore still diverges

The original sequence was rerun, not replaced with the easier variant:

1. With settings enabled and A the master, create an intentional local
   master edit during a synthetic game session and explicitly share B's
   55-byte CheckSettings copy from B.
2. On A's Settings sync page choose **“Keep the copy shared from RecheckB”**.
3. Promptly open CheckSettings detail and restore the retained shared
   **49-byte** copy; confirm Restore.
4. Make no additional settings choice and observe GUI and A/B hashes
   for **150 seconds**.

The accepted restore applied on B but not A. A remained at the 55-byte
peer copy:
`fdce489af0c31085ff46c282287a3412ec40162dfc2192c0f26dadadcb1ea690`.
B reached the requested 49-byte copy:
`2bf169a7ea11b53003323b4b9cee897df79fb4655b4e8f705f355ffbe5cabd73`.
Runs 2 and 3 reproduced this; the denser round reproduced it again.
`restore-failures.jsonl` records the actual repetitions, UTC times,
hashes and the 150-second failures. Corresponding screenshots and
`convergence.jsonl` preserve the GUI observations.

For run 2, restore was accepted around **21:26:13Z**; A automatically
published its still-local 55-byte bytes around **21:26:20Z**, producing
another choice. Run 3 repeated the auto-publication around **21:35:02Z**
after accepting restore around **21:34:58Z**.
The GUI sometimes said **“This computer is up to date” while A's own
bytes were still not the selected restore**, not merely while a remote
computer was catching up. Polling counts are reported with the final
corpus below; samples are not counted as separate user incidents.

Only after recording each failed 150-second observation did the workload
driver make an **extra cleanup-only 49-byte choice** so subsequent memory
passes could proceed. That click is explicitly labelled cleanup in the
raw record and is **not** counted as a successful restore.

A manually delayed original sequence did converge. The separately
requested **restore, then choice made on the master** variant also
converged to the selected 78-byte master copy without an extra click.
Those positive sequences do not invalidate the repeatable prompt failure.
Former G11 is therefore **not closed**; the release's no-open-major gate
is not met.

### Six v0.3.3 compatibility operations

Published v0.3.3 was run through the same scoped guard as B; candidate A
remained the dispatch binary. Each operation checked both TOC and
`main.lua` hashes before and after; removal required active folders to be
absent on both sides. `code-results.jsonl` retains full SHA-256 values.
Below are readable prefixes of the equal final hashes, **not** substitutes
for the full raw hashes.

| Direction | Operation | Result | Final TOC / main.lua SHA-256 prefixes | Observed seconds |
|---|---|---|---|---:|
| candidate → v0.3.3 | CheckSettings update | PASS | `1f8a781a0f4851c2` / `ee2e9037e9797d28` | 14.603 |
| candidate → v0.3.3 | R3CompatA addition | PASS | `f6fd5c56333062b3` / `ee4015f7941313f2` | 16.071 |
| candidate → v0.3.3 | R3CompatA removal | PASS | both active folders ABSENT; preceding hashes as above | 38.154 |
| v0.3.3 → candidate | CheckSettings update | PASS | `817077ce671e02eb` / `667ce218a65bec40` | 28.106 |
| v0.3.3 → candidate | R3CompatB addition | PASS | `da79a0d4ab60be2c` / `c3c1b475283b81f2` | 16.050 |
| v0.3.3 → candidate | R3CompatB removal | PASS | both active folders ABSENT; preceding hashes as above | 38.111 |

Thus valid old/new encoded folder-tree records were observed interoperating
in both directions. This is not a claim that ambiguous duplicate-key input
is accepted; OD-51 requires rejection.

### Warm unchanged-pass speed — BLOCKED

The published v0.3.2 asset was separately downloaded and hash-verified.
Three warm unchanged GUI-triggered passes per edition used the same full
B installation/provider, settings paused, interval 600,000 ms, and other
LAB applications stopped. Current dispatch durations were **683, 479,
533 ms**, median **533 ms** (`speed-results.json`, edition logs).

All three v0.3.2 exact durations were suppressed by its zero-read,
≤2,000 ms logging rule. Consequently there is **no observed v0.3.2
median** and no defensible current ≥3× median ratio. Historical R10
3.185× evidence and locally built benchmarks are not substituted for this
requested artifact comparison. The speed gate is **BLOCKED**, not PASS
and not a demonstrated regression.

### Privacy and audit method

An actual diagnostics export was run through B's GUI, with settings enabled
and character-scoped units present. Destination was inside LAB:
`round2\B-state\profile\Documents\wow-sync-diagnostics-20261125-213635-155.txt`.
SHA-256:
`2da24078038d756b8819eb43f16c025d95db7a077dbde959cd6676993a383ae8`.
It contained none of the tested account/realm/character names, absolute
paths or settings-content canaries. The older A export remains on disk,
but is **not** a new round-3 export.

Current-round A/B log suffixes were scanned from the recorded baseline
byte offsets; whole fresh C/D logs were scanned. Identity tokens covered
the selected/excluded synthetic accounts and synthetic realms/characters;
content canaries included the supplemental new-addon/master/peer session
text. Runtime/sync/settings absolute paths were not permitted. Accepted
developer startup/inspect path lines are quoted below with LAB redacted.

Local `changes.jsonl` does contain scoped identity names, permitted by
OD-44. **Provider settings-revision `unit.identity` also contains the
scoped account identity**, which was observed and is required to place
the shared unit; this is not a claim that identity exists only locally.
No copying of `changes.jsonl` itself into the provider was observed, and
the observed logs/export did not contain those identity tokens.

File watchers were ready **before the first candidate launch**
(`watcher-ready`: 2026-11-25T20:43:01.4951433Z). Original, supplemental C/D,
and native A/B/provider event streams were combined for the final audit.
The audit classifies application/data roots, not authorized artifact
downloads or the verifier's separate control/evidence files. Explicit
state and `<variant>\.wowsync` are allowed by OD-42; code-fixture edits
and code-apply paths belong to the separately required code-sync cases.
Synthetic game fixtures and manual missing-folder creation are identified
as verifier actions, not attributed to the app. Final counts, blocked-file
hashes and unexpected-path/error results are below.

### Memory: all observations retained, sampling qualification

Settings remained enabled on the accumulated full A/B lab. Runs 1–3 and
5–7 were fresh release processes, used Settings sync and add-on detail,
shared, chose the peer copy, confirmed restore, and hid through the tray
workflow before settings/code update/add/remove and idle passes.
Restore failures were retained, with cleanup-only choices explicitly
separated. Runs 3 and 7 used zoom **1.5**. Runs 4 and 8 used
`--start-hidden` and their windows were **never shown**.

All values below are **MiB**, using actual EX2 private working set, not
total working set or private commit. Commit is reported, not gated.
“Startup” is the maximum sampled before the window was ever shown;
for start-hidden runs it is the entire never-shown run. Idle is the
maximum observed in the labelled hidden idle periods after UI use.

| Run | PID | Zoom | A history at launch | Visible peak | Hidden after UI peak | Hidden idle after UI | Startup / never shown peak | Private commit peak |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 35468 | 1.0 | 127 | 84.320 | 81.941 | 81.367 | 34.363 | 150.426 |
| 2 | 16100 | 1.0 | 149 | 86.684 | 84.168 | 80.848 | 41.684 | 152.352 |
| 3 | 36676 | 1.5 | 163 | 84.910 | 87.039 | 85.578 | 43.832 | 152.484 |
| 4, start-hidden | 37596 | 1.0 | 177 | 0 | N/A | N/A | 71.281 | 99.742 |
| 5 | 3232 | 1.0 | 189 | 87.793 | 91.707 | 89.691 | 42.172 | 159.949 |
| 6 | 8460 | 1.0 | 203 | 88.684 | 90.918 | 88.387 | 41.895 | 160.430 |
| 7 | 32936 | 1.5 | 217 | 87.871 | **92.215** | 90.117 | 44.629 | 160.836 |
| 8, start-hidden | 34604 | 1.0 | 231 | 0 | N/A | N/A | 65.012 | 92.617 |

Highest **recorded hidden** sample, including startup and all after-UI
samples: **92.215 MiB**. **Zero** recorded hidden samples exceeded
100 MiB, including recorded auxiliary computers/peers. The sampled
numeric limit therefore passed, with **7.785 MiB** observed margin.
The old v0.3.3 compatibility peer had a 116.348 MiB **visible** peak;
it was not a hidden candidate-gate violation.

A history grew from 125 before the round to **236** records at shutdown.
Within the comparable native cohort, hidden peaks were
91.707→90.918→92.215 MiB (third run zoom 1.5), net +0.508 MiB.
Earlier cohort peaks were lower as history accumulated; the sampler also
changed. This is **not proof of no unbounded long-run growth**, and the
80 MiB author margin target is not substituted for OD-50's 100 MiB gate.

The first monitor targeted 25 ms but achieved roughly 30 ms and had
large gaps. Rather than silently accepting that, three further fresh
full runs (5–7) plus never-shown run 8 used a dedicated native sampling
thread and a high-resolution waitable timer targeting 25 ms. Every
record includes native `EnumWindows`/`IsWindowVisible` results; native
timing uses monotonic `mono_ms`. A peer's negative clock-reset interval
between monitor versions was excluded from interval arithmetic only,
**not from memory observations**. No working-set trimming, helper-process
offloading by the product or global timer-resolution change was used.

| Run | Hidden samples | Hidden interval median ms | Hidden interval maximum ms |
|---|---:|---:|---:|
| 1 | 10,216 | 30.136 | 953.123 |
| 2 | 5,415 | 30.062 | 985.258 |
| 3 | 9,105 | 30.113 | 936.506 |
| 4 | 22,862 | 30.143 | 1,020.202 |
| 5 | 7,288 | 25.005 | 943.154 |
| 6 | 7,201 | 25.005 | 966.073 |
| 7 | 7,041 | 25.002 | 972.252 |
| 8 | 10,978 | 25.003 | 42.737 |

**G15 — note, instrumentation coverage:** the native median meets the
nominal target, but the full after-UI runs still have nearly one-second
gaps while hidden. Reproduce by grouping native CSV rows by run/PID and
subtracting successive `mono_ms` values where both rows have zero visible
windows. No cause is attributed without further evidence. The observed
numeric PASS does **not** bound unsampled peaks or establish uninterrupted
25 ms coverage. Overall gate 10 is conservatively **BLOCKED on that
protocol qualification**, not a measured product memory FAIL.

Run 8's first driver stopped when B's settings file disappeared between
an existence check and `Get-FileHash` during replacement. Subsequent
observations found both files present at 33,449,574 bytes. This
observation failure was preserved; the same never-shown PID was resumed
through the complete hidden workload, with hash-observation retry only.
No app action or UI showing was used to repair the observation, and all
samples before/after it remain included. See `run8-observation-retry.txt`.

### Final observations, audit and unresolved owner-log proof

G13 reproduced in runs **2, 3, 5, 6 and 7**, all with the same divergent
55/49-byte hashes after 150 seconds. Dense failure endpoints were
22:38:14.653Z, 22:46:36.338Z and 22:54:56.213Z.
The final convergence corpus has **1,598 samples**:
**146** locally premature up-to-date headlines (including primary-choice
and cleanup observations) and **153** before both computers matched.
The difference, seven, is remote-only delay, not proof that A itself
was wrong. `corpus-summary.json` provides labels/counts; the actual
failed restore cases are in `restore-failures.jsonl`.

The final write audit contains **866,498 events**, **151,071 unique
paths**, **zero watcher errors** and **zero unexpected paths**:

| Path category | Unique paths |
|---|---:|
| Explicit state | 15,744 |
| Variant-local `.wowsync` state | 133,567 |
| Provider | 1,666 |
| Code fixture / code apply | 62 |
| SavedVariables shape | 18 |
| WTF directory notification | 8 |
| Install directory notification | 5 |
| Verifier inert client | 1 |

Blocked files retained exact before/after SHA-256 values:

| Computer / file | SHA-256, unchanged |
|---|---|
| A CheckLarge | `bdf7fbb54387c24608fd757a1b31c32cabcd7b6bee8ff3345c897139fffc072a` |
| B CheckLarge | `e017d0ade6150ffd8758d064697f21a276d2e0776fbcc7f480a566c2b3e25f30` |
| A CheckShared | `c7682b0077be011c72e8f265b0985f170e1650fcdb5c5a9c0028a8ee6f68af49` |
| B CheckShared | `5399d1d3ab0cf81f7acd5849826a6271e4454fe46dec1a81ade0b26112783aee` |

No blocked-unit settings revision was found. The consent provider had
six settings revisions, all for the selected account, none for the
excluded account. Final scan: **2,163 current-round log lines**, no tested
identity or content canary, **28** accepted startup/inspect path lines
and **zero** unaccepted runtime/sync/settings absolute-path lines.

**G14 — note, metadata-only isolation proof remains unclosed.**
Only the expressly authorized owner's log **metadata** was read; no
contents were read and PID 8816 was never stopped or targeted.
The owner's existing application remained outside this worker's control.
Metadata is **not equal**:

| Observation UTC | Length bytes | Last write UTC |
|---|---:|---|
| Before, 20:41:15.506 | 357,319 | 20:41:14.585 |
| Intermediate after, 22:00:32.105 | 494,316 | 22:00:27.503 |
| Final after, 23:02:41.465 | 602,470 | 23:02:40.438 |

The snapshots cannot attribute those writes to a process. No candidate
write to owner state is asserted, but neither is an unchanged-owner-log
proof. Gate 9's **LAB audit passed**; its additional requested owner-log
proof is conservatively **BLOCKED**, not replaced with an assumption.

### Final round-3 gate table

| Gate | Result | Evidence / qualification |
|---|---|---|
| 1 Artifact SHA-256 and smoke / build prerequisites | PASS | Dispatch ZIP/EXE hashes, guarded GUI smoke, successful dispatch/CI; local tiers below |
| 2 Both guided offers, consent, account and master choice | PASS | Fresh C/D GUI and matching applied hashes; excluded account never shared |
| 3 A-11–A-15 GUI cases | PASS | Completed sessions/new addon, routine yield/recovery, intentional peer choice, chooser fields/attention, global/per-addon pause; rapid-restore defect separately fails gate 7 |
| 4 Missing character | PASS | Waiting, folder absent until manual creation, then matching bytes |
| 5 Over-cap/shared ownership | PASS | Correct blocked/explanation GUI, unchanged hashes, no provider settings revision |
| 6 Activity/history/detail/restart | PASS | Source/reason/size and restore labels on both; visible rich recovery controls; exact 96-record restart hash |
| 7 GUI recovery / original and variant convergence | **FAIL** | Peer replaced-copy recovery and delayed/variant byte convergence passed; prompt original fails repeatedly, G13 major |
| 8 Logs/history/diagnostics privacy, OD-43/44 | PASS | GUI export hash, 2,163-line scan; only accepted developer paths; permitted local/shared placement identity distinguished |
| 9 Pre-launch write audit and owner isolation proof | **BLOCKED** | Scoped audit passed with no unexpected paths/errors; changed owner metadata cannot prove attribution, G14 |
| 10 Full accumulated-lab hidden memory | **BLOCKED** | All sampled hidden values ≤100 MiB, peak 92.215; three native fresh runs + start-hidden complete; 25 ms coverage has disclosed gaps, G15 |
| 11 All six v0.3.3 code operations | PASS | Both directions update/add/remove, full TOC/main.lua hashes and absent active folders |
| Additional R10 warm unchanged ≥3× median speed | **BLOCKED** | Current median 533 ms; v0.3.2 median unobserved because logging suppressed it |
| No open critical/major findings | **FAIL** | G13 major remains open |

### All accepted absolute-path log lines

Code areas: “WoW Sync … starting” is **developer startup**;
“inspect start/done” is **setup/inspect**. There are no accepted lines
from runtime/sync/settings. LAB and any identity token are redacted; all
28 lines, including repeated starts, follow.

```text
2026-11-25T20:43:13Z pid=39852 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T20:57:29Z pid=35468 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T21:23:42Z pid=16100 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T21:32:24Z pid=36676 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T21:42:38Z pid=37596 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T22:33:03Z pid=3232 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T22:41:29Z pid=8460 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T22:49:48Z pid=32936 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T22:58:03Z pid=34604 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-25T20:43:13Z pid=9112 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T21:46:35Z pid=4364 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T21:48:08Z pid=27988 INFO WoW Sync 0.3.3 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T21:55:10Z pid=7144 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T21:56:58Z pid=6824 INFO WoW Sync 0.3.2 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T22:29:28Z pid=5972 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-25T20:43:13Z pid=10996 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C-state
2026-11-25T20:43:57Z pid=10996 INFO inspect start <LAB>\round2\C\_forever_
2026-11-25T20:43:57Z pid=10996 INFO inspect done <LAB>\round2\C\_forever_ in 41 ms: 4 add-on(s) eligible, 0 blocked, 8 file(s) read
2026-11-25T22:12:00Z pid=35616 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C-state
2026-11-25T22:15:41Z pid=26700 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C-state
2026-11-25T20:43:13Z pid=37220 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D-state
2026-11-25T20:44:03Z pid=37220 INFO inspect start <LAB>\round2\D\_forever_
2026-11-25T20:44:03Z pid=37220 INFO inspect done <LAB>\round2\D\_forever_ in 20 ms: 4 add-on(s) eligible, 0 blocked, 8 file(s) read
2026-11-25T20:45:52Z pid=19380 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D-state
2026-11-25T20:47:46Z pid=19380 INFO inspect start <LAB>\round2\D\_forever_
2026-11-25T20:47:46Z pid=19380 INFO inspect done <LAB>\round2\D\_forever_ in 1 ms: 4 add-on(s) eligible, 0 blocked, 8 file(s) read
2026-11-25T22:12:00Z pid=34188 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D-state
2026-11-25T22:15:41Z pid=11500 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D-state
```

### Shutdown and local verification

All matching application/fake-client ledger entries were stopped only by
owned PID plus executable/creation identity. Final live applications
stopped were A **34604** and B **5972**. Native monitor and supplemental
watcher exited cleanly, exit 0. Every one of the **22 application launches**
has an accepted guard record. The dispatch executable hash was rechecked
after shutdown and was unchanged.

`git -C D:\GitHub\wow-sync status --short` was empty after shutdown.
Worker status contained only this evidence edit. No push or public
publication was performed. Raw evidence and screenshots remain inside LAB.

Local offline tier before the appendix: **PASS**, exit 0
(`offline-before-evidence.log`); native tier: **PASS**, exit 0
(`native.log`), on this worker's 8d3070a checkout, whose product code is
902e720. Verification scratch was scoped to the worker target directory.
The post-appendix offline result and final raw seal are recorded below.

Post-appendix `cargo xtask verify --tier offline`: **PASS**, exit 0,
`allow-blocked: false`. Fmt, clippy, tests, required-cases, deny and
markdown-links all passed (`offline-after-evidence.log`). This automation
PASS does not override the functional/protocol findings above.

`round3\raw-evidence-sha256.txt` seals **150** stopped raw
evidence/screenshots and final helper files, relative to LAB. Manifest
SHA-256:
`bdbc23a1740c66ba0028ef19d9061744c2ddac2c119925e5b63c481601b0ee59`.
The subsequently generated post-appendix offline log is explicitly
excluded from that seal; all measured CSV/event streams were closed before
sealing. These files remain local and were not uploaded or committed.

## Re-verification 4 (65fcfd3)

### Candidate, authority and retained state

Focused independent GUI re-verification of dispatch **45730389080**,
head `65fcfd30f6e358777b50e0158a7a174697430c41`, downloaded into LAB
`artifact4` and extracted into `candidate4`. No locally built app was
used. ZIP SHA-256 matched the supplied sidecar:
`b846692bc7f5294332b232536b00f4d09b994b18e8df22202fddfeb5ef850f73`.
Extracted EXE SHA-256:
`4f1e2344b2ab25388c34cfc2dc9c0716dbc72af643a43c985bdbb5224c8b4e3a`.
Dispatch and CI **45727396299** both reported success.
Worker `D:\GitHub\wow-sync.worktrees\R11-check`, branch
`run011/R11-check`, first merged main by fast-forward to 65fcfd3.
Windows host/toolchain are unchanged from round 3.

Read [OD-52–OD-56](../DECISIONS-FOR-REVIEW.md), runtime “Round G13”
and subsequent sections, review “G13 review” and “RV11-17 re-check”.
OD-52 closes round-3 G14 by the orchestrator's PID attribution; OD-53
accepts the sampled-value memory gate despite disclosed gaps; OD-54
carries the speed gate forward. None is represented here as newly
measured evidence. OD-56 supersedes the broad OD-55 stale-file rule.

The full accumulated A/B `round2` installations/state/provider were
retained, not reset: initial A/B history **236/134** records
(`round4\baseline.json` is authoritative for the exact counts).
Fresh small C4/D4 consent computers copied only synthetic C/D add-ons,
SavedVariables and synthetic installation markers, with new independent
state and `consent4-provider`; full A/B history was unaffected.
Raw evidence and screenshots are under LAB `round4`.

All starts, including v0.3.3, use the retained mandatory launch guard,
copied as `r4-common.ps1`: exact command printed/recorded before launch,
`--dev --install --state --provider`, all paths under LAB, scoped child
profile. There were no bare launches/help probes or imported code
execution. The original prelaunch watcher was extended to all existing
lab roots; C4/D4 had their own watcher ready before their first launch.
Only expressly authorized owner-log metadata was read. Contents and
process attribution remain the orchestrator's responsibility.

### Harness qualification, not product failures

Two preliminary attempts are retained but are **not counted** among the
five valid prompt repetitions:

- The first stopped before any choose/restore, because “Sync now” was
  unavailable while B completed an existing pass. An A pass eventually
  logged **462,251 ms**, zero file reads. The apps subsequently completed
  ordinary passes again. No approved speed gate is replaced by this
  outlier or by an invented ratio.
- Run 11 selected **shared-from-B 07:22:30, 49 B**, whose actual payload
  SHA-256 is
  `d3018600f8458a59e49106a63a93e6490dc0e68afe8f0067da78c47e136d01c5`,
  while the harness incorrectly expected the original master's 49-byte
  checkpoint. A/B actually reached the selected D301 bytes. This
  wrong-expectation corpus is retained and explicitly excluded from
  product false-headline counts, not silently deleted.

The **original exact checkpoint** was then selected through the GUI:
**settings master RecheckA, 25 November 07:18:26, 49 B**. Actual restored
file SHA-256 independently verified:
`2bf169a7ea11b53003323b4b9cee897df79fb4655b4e8f705f355ffbe5cabd73`.
Valid prompt repetitions use that exact control and expected hash.
The older D301 payload was separately preserved from the provider,
hash-verified, and used for the RV11-20 byte-identical-old-version probe.

An additional run-22 observer interruption occurred when `Get-FileHash`
hit the transient absence of A's file during replacement. It was not a
150-second application non-convergence result. The observer now records
missing/locked-file observations and retries within the **original**
150-second deadline, without an application action. The interrupted
corpus is retained separately; new complete repetitions replace it.

### G16 — minor: main headline still says up to date before own selection applies

The byte-convergence part of the old G13 case now succeeds in the valid
completed observations below. However, the separately requested truthful
headline assertion still fails:

1. Perform the original competing-settings sequence on master A.
2. Click “Keep the copy shared from RecheckB”.
3. Before A's next settings apply, poll A's actual file SHA-256 and its
   native UIA StatusBar in the same observation.
4. Repeat after confirming the original 49-byte restore.

A's **main headline** says **“This computer is up to date”** for several
500 ms observations while **A's own bytes**, not just B's, still differ
from the just-selected bytes. The local file subsequently converges.
The application log independently records the tray becoming up to date
after a choice was accepted and before the matching settings-applied line.
For example, completed valid run 31 accepted ChooseSettings at
**01:46:37Z**, logged tray up to date at **01:46:38Z**, and applied
the peer bytes at **01:46:42Z**.

This is rated **minor**, rather than reasserting the former long-lived
major divergence: bytes now converge without an extra choice in the
completed valid cases. It nevertheless fails this round's explicit
“headline never says up to date while A's own bytes differ” criterion.
Per-unit waiting wording is not substituted for the observed main
headline. Exact valid-corpus counts and repetition results follow.

### Lighter regression: guided consent and pause

Fresh C4/D4 both displayed the guided settings offer. C4 account controls,
“Share these now: yes” and “RecheckC4 (this computer): master” were
observed and captured. With both accounts not included, the action read
**“Share settings, Choose at least one account.”** (UIA still reported
the explanatory action enabled; this is not described as an observed
native-disabled control.) Only LABCONSENT1 was included; LABEXCLUDED1
remained out. D4 joined the discovered group, saw C4 as master, selected
the matching account and accepted “Use settings”. The matching final
hashes and screenshots are retained in `guided-results.json` and
`C4-guided-offer`, `C4-no-account`, `C4-consent-master`,
`D4-guided-offer`, `D4-consent-applied`.

C4/D4 pause tests used their separate provider, independent of the full
A/B workload. Global pause on **both** computers protected the actual
local hashes for 20 seconds, with publication counts unchanged **4→4**
on C4 and **0→0** on D4:

| Scope | Paused bytes SHA-256, unchanged while paused |
|---|---|
| C4 master | `864bf87726cb6d26d3e988fca6844d7b81d8165f50fa6e4de1c2ebe08e810fda` |
| D4 peer | `6693de8aa6b4767c7d07f051da745d265dbf5d52b274cdd966a854bcaf021644` |

After Resume all settings, C4 offered **“Share my settings”** and
**“Use shared settings”**. The recorded explicit Share my settings action
at **01:44:58Z** brought the pair to C4's bytes. These are the observed
resume choices, not a “Keep…” competing-publication chooser. The latter
is separately exercised by all G13 repetitions and the RV11 probes.

Per-add-on pause on D4 then protected its
`864bf877…` bytes while C4 changed to
`cd129c3bd02e73afd2790e7027b8641307fb38b196261f6d202a6054f1fbce3d`.
The resume chooser offered **“Use shared settings”** / **“Stay paused”**.
Selecting Use shared settings brought both to that exact CD129C3B hash.
An earlier harness expected the longer “Use shared, keep mine as a
recovery copy” wording and stopped before acting; it was corrected to
the actual offered action and the complete case rerun.
`pause-results.json` and `global-pause-checkpoint.json` retain the
before/after hashes, publication counts and actual UI text.

On the small pair, a newly installed settings addon completed **two
synthetic game sessions**. Its first session remained local (D4 file
absent); its second propagated automatically, with no Share click:
`749c2312c5b606db9145e65f0f7386fc0d6109449d739a361713fc79d8a136f3`
on both computers. D4 then made a routine, unshared edit
`9c362f4d6335f6fef065fb88dcd5ab1b4f2836c18188ac625c281287fae8ac16`;
it stayed local for 15 seconds, with D4 publications **0→0**.
A later completed C4 master session automatically replaced it, reaching
`4812728a979b2c5cd7292c849734827f16ad03d6c4019ff2e6653083f8af4eb2`
on both. D4's GUI offered its replaced **59 B** recovery copy, with
source, reason and time (`routine-peer-recovery.png`,
`session-results.json`). The inert C4 game fixture was separately owned
and PID-scoped; no addon Lua/settings was executed.

### G13 repetitions and convergence corpus

Each valid prompt run used the exact original 07:18:26/49 B checkpoint
identified above, **after** the peer choice had reached both files.
No cleanup click was needed in any completed valid case. Times below
run from the recorded restore action through the first observed
matching A/B bytes, within the unchanged **150 s** deadline.

| Run | Sequence | A/B convergence | Automatic A republications after restore | False-local up-to-date samples after restore |
|---|---|---:|---:|---:|
| 21 | Prompt | 7.203 s | 0 | 8 |
| 31 | Prompt | 7.300 s | 0 | 8 |
| 32, zoom 1.5 | Prompt | 7.250 s | 0 | 8 |
| 33 | Prompt | 7.798 s | 0 | 8 |
| 34 | Prompt | 7.263 s | 0 | 8 |
| 35 | Extra 30 s delay before restore | 7.224 s | 0 | 8 |

All six ended at the exact original `2bf169a7…` SHA-256 on both
computers. The five prompt repetitions plus delayed variant contain
**117** choice/share/restore observations, with **98** false-local
up-to-date observations: **50** after primary choices and **48** after
restores. These are not remote-only waits. Run 33 additionally sampled
one non-up-to-date mismatch. The interrupted/wrong-expectation attempts
are excluded from those counts, but retained in the raw corpus.

The intentional ChooseSettings command writes a SettingsMaster history
publication, and RestoreSettings writes RestoredFromRecovery. Those
explicit command publications are **not** being misreported as absent;
no extra automatic stale-byte publication was observed after restore
and its 16-second settling interval. Accepted-command logs correlate the
intentional history rows. `g13-results.jsonl`, `convergence.jsonl`,
`focused-summary.json` and source action logs retain the actual records.
Thus the former **persistent divergence** was not reproduced in these
six complete sequences; the combined G13 gate still **FAILS G16**.

### Gameplay-edit probes, RV11-17 and RV11-20

On A, restore the original shared checkpoint while the synthetic game
process is running, then edit A before ending the session. In both
cases the RestoreSettings command was accepted while the live file
remained unchanged; after the session, the edit was retained and the
GUI displayed **“Choose which saved settings to keep”**, a Needs
attention badge and a corresponding Needs attention item.

| Probe | Bytes retained until decision | Result after explicitly choosing the edited master copy |
|---|---|---|
| RV11-17, genuine 144 B edit | `4b9102c54262effaa29b91bd76174c5c02d4f52f7e1f72eeb16520616914da41` | Same SHA-256 on A and B; PASS |
| RV11-20, exactly the older shared 49 B payload | `d3018600f8458a59e49106a63a93e6490dc0e68afe8f0067da78c47e136d01c5` | Same SHA-256 on A and B; PASS |

No experiment was silently reverted to the pending original 49 B
restore. The master edit was offered, not lost. The first probe also
exercises restore → a later master choice, with the explicit extra
game/edit steps stated rather than hidden. `probe-results.jsonl`,
`probe-observations.jsonl`, accepted-action logs, Needs attention
screenshots and convergence hashes preserve these observations.

The initial second-probe attempt encountered the previous restore
confirmation still expanded, so its original list button was absent.
It stopped without a second restore command. The verifier closed that
old **UI confirmation** with Cancel, then reran the complete RV11-20
case; it did not cancel an operating pending selection or change
product code.

An additional **restore → choose the restored master copy** variant
also completed: repeat the game-held restore/old-byte edit, then select
the second master choice. A/B both reached the original `2bf169a7…`
without a further click. This complements the two edited-master-choice
probes; it is not substituted for the five exact prompt repetitions.

### G17 — minor: equal-size, same-minute master choices are indistinguishable

The RV11-20 negative case creates two **different 49-byte payloads**,
both from master A within one minute: the original restored `2bf169a7…`
and the genuine edited-old-version `d3018600…`. The Settings sync page
renders both buttons identically:

> Keep the copy from settings master RecheckA (25 November 18:26, 49 B)

The visible screenshot
`RV11-20-master-restore-choice-choices-visible.png` shows **both entire
buttons**, the pending headline and Needs attention badge. The earlier
18:12 case has the same duplicated labels in native UIA. There is no
visible payload/kind/time discriminator between these two choices.
The first 18:12 choice was selected by index and verified as D301; the
second 18:26 choice was selected by index and verified as original 2BF.
Both operations converged, but the owner cannot identify which of the
identical choices represents the intended bytes. This fails A-14's
distinguishable-choice obligation, not the underlying byte-retention
checks. No content preview or timestamp policy change is prescribed.

### Restart between selection and apply

With the game fixture running on A, restore original 49 B while A still
has D301. A's on-disk bytes remain D301. Quit through A's **actual native
tray Quit menu command**, then end the game fixture and start a fresh,
guarded A process. PID **32136** exited while fixture PID **37864**
was still alive; fresh PID **26288** adopted original `2bf169a7…`,
identical to B, with **zero automatic CheckSettings republications**.
The one intentional restore history publication is retained separately.

Native tray automation needed harness repairs: the UIA harness did not
locate the popup item; stale/hidden-menu and pointer/DPI attempts were refused
before a click to an unverified process. The successful method
enumerated the PID-owned tray menu, read the actual **Quit WoW Sync**
item/ID, invoked its Win32 menu command, then dismissed native menu
mode on the same owned window (`WM_CANCELMODE`). The log records
**“tray: quit”**; actual process exit, the still-running fixture and
fresh-process convergence were observed. No force-stop substitutes for
this quit. This is native GUI-menu invocation, not a bare executable,
backend command or claimed physical tray click.

The original PowerShell-derived start timestamp accidentally added the
local offset after an implicit DateTime/string conversion. It was
recomputed from the **raw ISO-8601 JSON UTC strings**, and the bounded
history query rerun: one intentional restore, zero automatic
publications. The pre-correction JSON is retained. The corrected
`restart-pending.json`, source actions and `r4-fix-restart-time.py`
make that correction auditable rather than relying on an empty
future-dated query.

### History, recovery labels and diagnostics privacy

Actual Add-ons detail Recent changes shows settings rows, source,
time, changed files/size and reason. On B, restore rows read
**“restored a copy from RecheckA”**, including after restart; A's
viewed Recent changes screenshot reads **“restored this computer's
earlier copy”** (`A-recent-settings-restores.png`). Its accepted restores
and settings-applied history retain RestoredFromRecovery.
Activity also shows settings share/update
entries. Rich shared recovery controls remain visible, including
source, kind, seconds and size (`memory31-after-restore.png`);
D4's replaced-peer-copy control includes time, size and replacement
source.

B's guarded restart preserved **186 history records / 72,423 bytes**
exactly: before and after-prefix SHA-256
`f369d451ea65b334faf0e48b9662f7ca76782ff01dfff051fc6770cab675fdb5`.
Screenshots `B-recent-persisted.png` and `B-shared-after-restart.png`
were viewed: rich restore reasons survived and shared ownership still
reads **“Shared with CheckSharedTwo, not synced”**, not “Not shared yet”.
The oversized unit reads **“Too large to share automatically (32.0 MB)”**.

A fresh diagnostics export was performed on B through **App settings →
Export diagnostics → Preview → Save**, with settings enabled and the
synthetic character-scoped unit present. The destination was read from
the actual screenshot before Save; it was inside B's lab profile:
`round2\B-state\profile\Documents\wow-sync-diagnostics-20261126-020652-957.txt`.
Actual saved file: **4,576 bytes**, SHA-256
`f6a1b50d6dd583e28e0867bccfeb06e0ceef1ed1d4ca6a153e4d52fc11df041e`.
It has no tested account/realm/character canary, drive/UNC path, or
synthetic file-content canary. Default options included addon folder
names, not file fingerprints. `current-export.json` retains the scan;
older exports do not substitute for this current-round GUI action.

### Missing-character negative case

The verifier removed B's synthetic character directory, then changed
the corresponding shared character settings on A. B did **not** create
the missing character folder. In the repeated, properly scrolled GUI
capture, Add-ons → CheckSettings → Settings reads:

> Waiting for LABMISSING on LABREALM1 to log in on this computer

`missing-character-visible-waiting.png` was viewed; that complete
waiting sentence is visible. UIA does not expose this plain body text,
so the harness's `waiting_observed_in_uia: false` is not being silently
changed into a true accessibility observation. After the verifier
manually created the character's SavedVariables directory, B applied
the shared file and A/B both had SHA-256
`fd58224fbe5a59259a01fb72e519e9e9280cc3b66319e05b3d92e3480dc56211`.
The earlier repeat also converged, but its unscrolled settings-page
capture is not used as visual waiting evidence.

### Full-lab memory, including every extra candidate process

The accumulated **119-addon / 36,154-file** A/B lab was not reset.
Three fresh visible processes completed Settings sync, addon detail,
explicit peer sharing, intentional choice and restore, then hid to the
tray. Each completed a **33,449,574-byte** settings capture/apply,
CheckSettings code update, temporary-addon addition/removal, incoming
CheckPause exception settings and initial/final 30-second user-idle
phases. `settings-results.jsonl` has matching A/B hashes for all eight
large-settings/exception results; code hashes are retained as well.
Run 32 stayed at **zoom 1.5**. Run 36 started hidden, performed the full
hidden workload and had **no observed visible top-level window**.

All values below are MiB, conservatively taking the higher value from
the native audit/sampler and independent sampler-only streams. Idle
means the explicitly labelled no-intentional-edit intervals; periodic
background checks continued. “N/P” counts the two independent streams,
not deduplicated observations.

| Fresh run / PID / zoom | Hidden rows N/P | Never-shown peak | Hidden after UI peak | Labelled hidden idle after UI peak | Visible peak | Private commit peak |
|---|---:|---:|---:|---:|---:|---:|
| 21 / 15032 / 1.0 | 6,888 / 6,882 | 41.434 | 91.199 | 87.207 | 90.105 | 160.805 |
| 31 / 25748 / 1.0 | 7,322 / 7,322 | 42.074 | 89.465 | 87.449 | 89.070 | 157.688 |
| 32 / 24564 / 1.5 | 7,043 / 7,043 | 39.664 | 88.961 | 86.031 | 87.895 | 157.512 |
| 36 / 15272 / 1.0, start-hidden | 7,594 / 7,594 | **64.730** | Not shown | Not shown | None observed | 96.848 |
| Extra functional probes / 32136 / 0.85 | 13,465 / 13,464 | 45.082 | **95.164** | No dedicated phase; bounded above by 95.164 | 92.418 | 160.762 |

**Gate maximum over ALL candidate hidden samples is 99,786,752 bytes /
95.1640625 MiB**, not just the three formal rows' 91.199 MiB.
The extra functional run's peak occurred at **02:15:19.534Z**
(independent stream: **02:15:19.556Z**), with all top-level windows
hidden after prior UI use, during the game-held/tray-automation work.
It is included, not discarded as “only a probe”. The highest labelled
formal hidden-idle peak is 87.449 MiB; for unlabelled functional waiting,
95.164 MiB is the conservative all-hidden/idle upper bound rather than
an invented exact idle classification. No allocation/GC cause is claimed.

Across candidate A/B there are **368,273 native rows**, including
42,639 hidden, plus **276,866 independent rows**, including 42,592
hidden. **Zero hidden rows exceed 100 MiB in either stream.**
Every preliminary/interrupted process is retained in those totals.
Candidate visible reference peak was **102.770 MiB on B**, not gated;
private commit is recorded separately, never compared with 100 MiB.
The v0.3.3 peer is separately labelled and excluded from candidate
measurements, not relabelled as a candidate.

Sampling requests **25 ms** using a high-resolution native waitable
timer and monotonic clock, with EnumWindows/IsWindowVisible on every
row and native private working set/private commit. Formal process
medians were **24.997–25.005 ms**. There are still gaps: maximum formal
gap **983.749 ms**, maximum formal hidden gap **978.899 ms**;
all-candidate maximum gap **983.796 ms**, hidden **982.768 ms**.
The cheap isolation experiment put the sampler in its own process
without FileSystemWatcher allocations; it **did not eliminate** the
gaps. Independent clocks are analysed separately, not concatenated.
This is **PASS on sampled values under OD-53**, not a claim that no
unsampled spike is possible. No trimming, limit change or working-set
median was used as the gate value.

A history count at the fresh full-run starts grew **248 → 264 → 275**,
then **304** for start-hidden, and ended at **329**. Partial run 22
between the first two full runs is disclosed above; the full workloads
ran sequentially, not concurrently or on reset state. Full-run hidden
peaks decreased **91.199 → 89.465 → 88.961 MiB**, despite growing
history; that is an observation over these runs, not an unlimited
growth guarantee. The separate small C4/D4 consent/session pair
sometimes ran alongside them, on a separate provider.

### All six v0.3.3 code compatibility operations

The guarded published v0.3.3 peer ran as B, PID **34776**. All six
operations completed again on this candidate. Each listed TOC and Lua
SHA-256 was identical on source and receiver; removals ended ABSENT on
both. Full before/after hashes and actual timings are in
`code-results.jsonl`.

| Direction | Operation | Identical after hashes: TOC; main.lua | Wait |
|---|---|---|---:|
| Candidate → v0.3.3 | CheckSettings 20.36 → 30.1 | `103f2f463e8819f3cfd1340383af896f3da4c2cf337c22ca327b3574b3889233`; `aca4ab4a01a4d173939e71b13fc7bac439f8b86d65e2cbf3e9ebb5f61f81651f` | 18 s |
| Candidate → v0.3.3 | Add R4CompatA 1.0 | `bc09d653179a573edc0a0fcb4f79162ee22c2fda20c346dedd1634edda81ba64`; `312117078926c07274924a7054ee57a7fef6fb52ba3a031cc123c984598982dc` | 20 s |
| Candidate → v0.3.3 | Remove R4CompatA | ABSENT / ABSENT; receiver before-hashes are the addition row | 42 s |
| v0.3.3 → candidate | CheckSettings 30.1 → 31.1 | `c28e1807aabaf6cb655002e1ae138774e815bc5a0f0b8f91b8b6ac59e3a9df99`; `1191e2c4248a52dc9547754cafada99522e4a7a2aec71981f26223b02b253849` | 40 s |
| v0.3.3 → candidate | Add R4CompatB 1.0 | `1261b7467d0430d53ae02e0dfa497f314e1200c79625fe7f620e9f2dc5ba69cc`; `59599ec4ea09e3dced94dba333963a40717082ba6d5caf1364a6591cb60c9826` | 20 s |
| v0.3.3 → candidate | Remove R4CompatB | ABSENT / ABSENT; receiver before-hashes are the addition row | 40 s |

### Privacy scan and prelaunch write audit

The scan covered **1,887 new scoped application-log lines**, including
all candidate restarts and the old peer. It found **zero** tested
account/realm/character canaries or file-content canaries, and **zero**
unaccepted absolute-path lines. Local changes.jsonl identity names
remain allowed by OD-44; this is not a claim that provider protocol
identities are absent.

All **31** accepted absolute-path lines are listed below. LAB replaces
the lab root. Startup lines are developer startup; inspect lines are
setup/inspect. None is a sync/settings/runtime path disclosure.

```text
2026-11-26T01:12:19Z pid=36392 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:13:31Z pid=35976 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:25:44Z pid=12408 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:31:07Z pid=15032 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:36:55Z pid=14940 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:44:32Z pid=25748 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:50:21Z pid=24564 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:56:06Z pid=21476 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:58:57Z pid=12396 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T02:01:43Z pid=37684 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T02:05:03Z pid=15272 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T02:08:15Z pid=32136 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T02:24:41Z pid=26288 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\A-state
2026-11-26T01:12:19Z pid=3700 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-26T02:25:48Z pid=16432 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\B-state
2026-11-26T02:28:10Z pid=34776 INFO WoW Sync 0.3.3 starting, dev=true, state <LAB>\round2\B-state
2026-11-26T01:16:49Z pid=4496 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C4-state
2026-11-26T01:17:28Z pid=4496 INFO inspect start <LAB>\round2\C4\_forever_
2026-11-26T01:17:28Z pid=4496 INFO inspect done <LAB>\round2\C4\_forever_ in 21 ms: 5 add-on(s) eligible, 0 blocked, 10 file(s) read
2026-11-26T01:40:04Z pid=38392 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C4-state
2026-11-26T01:43:44Z pid=36596 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C4-state
2026-11-26T02:01:54Z pid=34528 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\C4-state
2026-11-26T01:16:49Z pid=7088 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D4-state
2026-11-26T01:17:34Z pid=7088 INFO inspect start <LAB>\round2\D4\_forever_
2026-11-26T01:17:34Z pid=7088 INFO inspect done <LAB>\round2\D4\_forever_ in 20 ms: 5 add-on(s) eligible, 0 blocked, 10 file(s) read
2026-11-26T01:24:04Z pid=29800 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D4-state
2026-11-26T01:24:27Z pid=29800 INFO inspect start <LAB>\round2\D4\_forever_
2026-11-26T01:24:27Z pid=29800 INFO inspect done <LAB>\round2\D4\_forever_ in 1 ms: 5 add-on(s) eligible, 0 blocked, 10 file(s) read
2026-11-26T01:40:04Z pid=7160 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D4-state
2026-11-26T01:43:44Z pid=16148 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D4-state
2026-11-26T02:01:54Z pid=9428 INFO WoW Sync 0.4.0 starting, dev=true, state <LAB>\round2\D4-state
```

Watchers were ready **01:12:08Z**, before the first A/B launch;
the additional C4/D4/provider watcher was ready **01:16:45Z** before
their first launch. They closed cleanly with **512,802 notifications**,
**90,509 distinct notified paths**, **zero watcher errors/overflows**
and **zero unexpected paths**. This is a bounded lab-filesystem audit,
not machine-wide/PID-attributed tracing or proof about OS cache writes.

| Allowed category | Distinct notified paths |
|---|---:|
| Explicit lab state | 11,551 |
| Synthetic provider | 1,368 |
| Variant `.wowsync` local state, OD-42 | 77,520 |
| SavedVariables shapes | 15 |
| Authorized synthetic code fixture/apply paths | 48 |
| Install-directory notifications | 3 |
| WTF directory notifications | 3 |
| Verifier's inert game fixture | 1 |

The code writes are confined to synthetic Check* fixtures,
R3NewSettings, R4NewSettings, R4Mem21/31/32/36, R4CompatA/B and their
AddOns parents, not the 113 original copied addon folders.
Code-test writes are explicitly separated from settings-write scope.
Both **33,554,433-byte** blocked files and both shared-owner files
retained their **different per-computer baseline hashes**, with
**zero blocked-file notifications** and **zero blocked settings
revisions** in the provider. `blocked-before.json` and
`audit-summary.json` retain all four full before/after hashes.

| Blocked fixture | Bytes | SHA-256 before = after |
|---|---:|---|
| A CheckLarge | 33,554,433 | `bdf7fbb54387c24608fd757a1b31c32cabcd7b6bee8ff3345c897139fffc072a` |
| B CheckLarge | 33,554,433 | `e017d0ade6150ffd8758d064697f21a276d2e0776fbcc7f480a566c2b3e25f30` |
| A CheckShared | 38 | `c7682b0077be011c72e8f265b0985f170e1650fcdb5c5a9c0028a8ee6f68af49` |
| B CheckShared | 57 | `5399d1d3ab0cf81f7acd5849826a6271e4454fe46dec1a81ade0b26112783aee` |

### Round-4 gate table

This is the authorized **focused/lighter** round, not a claim that
every standalone scenario from round 3 was rerun.

| Gate | Result | Current-round evidence / boundary |
|---|---|---|
| 1 — artifact and smoke | PASS | Dispatch SHA-256 sidecar match; guarded release GUI/runtime used throughout |
| 2 — guided offers/consent/master | PASS | Fresh C4/D4 offers, selected account, Share these now, master and peer consent |
| 3 — A-11–A-15 | **FAIL** | Positive automatic-session, routine-yield, peer-choice, exception and pause checks passed; G16 premature headline and G17 indistinguishable choices fail explicit GUI assertions |
| 4 — missing character | PASS | Visible waiting; folder never app-created; matching hash after verifier creates it |
| 5 — cap/shared ownership | PASS | Rich blocked wording, restart persistence, four unchanged distinct hashes, no blocked revisions |
| 6 — Activity/history/recovery labels | PASS | Both GUIs show restore reasons; rich controls; B's exact history survives restart |
| 7 — full standalone GUI recovery matrix | NOT RUN | Excluded from lighter sweep; shared-checkpoint GUI restores were repeatedly rerun, but the standalone replaced-peer-copy restore was not |
| 8 — logs/history/diagnostics privacy | PASS | New scoped log scan and actual current GUI export; OD-43/44 exceptions explicit |
| 9 — write audit | PASS, scoped | Prelaunch watchers, allowed `.wowsync`, no unexpected path; owner attribution remains orchestrator-owned under OD-52 |
| 10 — hidden private working set | PASS, sampled | All hidden rows ≤100 MiB; maximum **95.1640625 MiB**, gaps disclosed under OD-53 |
| 11 — v0.3.3 code compatibility | PASS | All six fresh update/add/remove operations, both directions, actual hashes |
| Focused G13 prompt/delayed/master-choice cases | **FAIL overall** | Byte convergence and no automatic stale-byte republish passed; truthful-headline assertion fails G16 |
| RV11-17 / RV11-20 gameplay edits | PASS | Retained edits offered as decisions, including exact older shared bytes |
| Restart selection-before-apply | PASS | Native tray quit, game ends after old process exits, fresh process converges without republish |
| R10 warm speed gate | PASS, carried | OD-54; not remeasured or inferred from this round's outlier |

New findings are **G16 minor** and **G17 minor**, described with
reproductions above. No new critical/major byte-loss or long-lived
divergence was observed in the completed scoped cases. That does not
turn either failed GUI obligation into a pass.

### Shutdown, owner metadata and local verification

All matching owned application/game-fixture processes were stopped;
final identity-checked survivors list is **empty**. Samplers/watchers
all exited 0 after final application shutdown. No owner PID was stopped
or selected. The main checkout's status was **empty**; the worker's
only tracked change was this evidence file. No push or product change.

Authorized owner-log **metadata only**:

| Observation | Bytes | Last-write UTC |
|---|---:|---|
| Before, 01:11:22Z | 835,148 | 2026-11-26T01:11:21.6782925Z |
| After, 02:31:33Z | 973,343 | 2026-11-26T02:31:30.4301834Z |

Its size changed **138,195 bytes**. Contents were not read or attributed
by this worker; the orchestrator explicitly owns that attribution
and instructed that metadata growth alone not block gate 9. The guard
ledger contains only lab-scoped starts.

Local `cargo xtask verify --tier offline` on the worker checkout:
**PASS**, exit 0, `allow-blocked: false`; fmt, clippy, workspace tests,
required cases, deny and markdown links all passed. Verification
scratch was scoped under the worker's target directory. No dependency
install was needed. This round did not separately rerun the native
inventory tier; the offline command itself ran the workspace tests.

Raw evidence, final scoped local-state copies, exact harnesses,
downloaded artifact files and both tested binaries are sealed by
`round4\raw-evidence-sha256.txt`: **867 files**, manifest SHA-256
`04617773c61a4efabdf2ab3cba6e2b767247abbdf88eb478504de52b4159b569`.
Its own file is excluded from the seal. The offline log and all closed
measurement streams are included. A separate evidence/raw-data
consistency check passed, explicitly retaining G16/G17 as expected
product failures; it is **not** a substitute product test. All raw
material remains local in LAB, not uploaded or committed.
