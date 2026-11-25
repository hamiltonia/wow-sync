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
