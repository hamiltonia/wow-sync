# S-check: independent Phase 2 verification

Run-001, 2026-11-15. Checkout `D:\GitHub\wow-sync.worktrees\S-check`,
branch `run001/S-check`, input `main` at
`5be6ba051b92bb8e6fd7c0f8d7664acc9c604d9e`.
Authority: [packet](../packets/S-check.md), [common rules](../packets/S-common.md),
SPEC §7 / R-02, R-05, R-07, R-11, R-14; A-29 and E-04/E-06.
Actual runtime **PASS**: `gpt-6.1-sol`, independently located in local usage
records for this worker's execution interval ([runtime record](S-check/runtime.txt)).
No nested agents, product changes, fixes, push, merge or publishing.

## Environment and scope

Windows 11 Pro x64 Insider build 26300, i7-13700HX (24 logical processors),
64 GiB class RAM, PowerShell 7.7.0, pinned Rust 1.100.0/MSVC, locked release
`gpui 0.2.2`. Development evidence only, not official stable-Windows support.
[Environment/artifact hashes](S-check/environment.txt).
All macOS runtime and Windows ARM64 runtime: **BLOCKED** (no test hardware).
No new system software, provider credentials or paid services.

S-mem and the specified safe Dropbox operations were independently executed.
S-forever metadata was independently read; its uncommitted report was read
without editing or committing it. Other spikes received a label/data consistency
audit, **not** an independent runtime pass.

## SPEC §7 answers and verdicts

### S-mem — PARTIAL

Question: can GPUI meet 100 MiB aggregate private resident idle memory,
including after UI use and repeated cycles?

**PASS for the main-PID measurement; PARTIAL for aggregate feasibility**, not
a product A-29 pass. Confidence is high for the counter and measured workload;
product feasibility remains unknown pending complete process aggregation, the
real UI, background engine and chosen GPUI pin. The build was independently
repeated with `--release --locked` (exit 0).
The unchanged committed harness was used without `-Trim`.

Independent main-PID results (MiB; steady = final 60 harness samples):

| Run | Samples / invalid | Steady min / median / max | Whole-run peak | Build-busy samples |
|---|---|---|---|---|
| a, never shown | 121 / 0 | 13.184 / 13.184 / 13.184 | 13.242 | 0 |
| b, 60 s shown then closed | 181 / 0 | 21.344 / 21.359 / 21.422 | 24.074 | 0 |
| c first run, contaminated | 361 / 0 | 27.141 / 27.141 / 27.191 | 29.602 | 26 |
| c retry, build-free | 361 / 0 | 27.121 / 27.121 / 27.121 | 29.992 | 0 |

**CONFIRMED**: the b/c main-PID steady values agree with the author's observed
ranges (20.9–22.3 and 26.8–27.6 MiB) and lie well below 100 MiB without trim.
The author's a range was 13.0–13.1 MiB; this run was 13.184 MiB.
Main-PID final GPU dedicated/shared allocations were a 7.8/0.7,
b 12.6/1.0, c 13.6/0.2 MiB, separately reported. Child GPU was not measured.
[Validated analysis](S-check/analysis.txt), [raw main-PID summary](S-check/memory-summary.txt).

The additional app-plus-console-child sampler captured 121 points during
the c retry (13:16:06–13:21:23, beginning partway through cycling).
Final 60 elapsed seconds: **28.176 / 28.234 / 28.234 MiB aggregate**
min/median/max across 23 samples; child alone 1.070–1.113 MiB.
Observed aggregate peak was 30.395 MiB in that **partial-duration** record,
not a full-run aggregate peak. This supports aggregate steady-state feasibility
for c on this PC, not an aggregate pass for every scenario or the product.
[Raw aggregate samples](S-check/aggregate-c-retry.csv);
[direct-child/grandchild check](S-check/descendants.txt).
Main/child counters are queried sequentially, not atomically.

Machine-load observations: first monitor 117 points, total CPU mean/median/max
9.14/7/40%, including five points with other compilers; c-retry monitor
68 points, CPU 9.82/10/28%, **zero** cargo/rustc observations.
Both monitors saw at most one app plus one child.
The clean b/a harness and clean c-retry harness each record zero build-busy
samples. Thus "quiet" is confirmed as **build-free for those measured runs**,
not a claim of zero desktop/background CPU or proof that no other agent existed.
No CPU pass threshold was invented.
Cleanup **PASS**: the retry log records zero remaining app processes after a
2 s wait; the final lookup of the retry app and its known console-child PIDs
found neither process.

Counter validation **PASS**: the installed CIM class maps `WorkingSetPrivate`
to `Working Set - Private`, raw byte counter type 65792.
The installed Windows counter help explicitly says the working set used only
by this process, not shared or shareable with others
([native counter metadata](S-check/counter.txt)).
This measures private resident pages, **not** Private Bytes/commit, total
working set or GPU allocations. The latter are retained separately in the CSV.

Important method difference: `spikes\mem\src\main.rs` closes each visible
window with `window.remove_window()`; reopening constructs a fresh window.
The invisible 1x1 anchor keeps the app alive. This is **destroy/recreate**,
not the retained-window `ShowWindow(SW_HIDE)` lifecycle in S-gpui. Calling all
of these results "hide" can mislead integration. R-14 allows opening/closing,
so this does not invalidate this workload, but retained-window tray behavior
and a future AccessKit/WGPU GPUI pin require new measurements.

Audit findings **FAIL** (report precision, not the 100 MiB comparison):

- **REFUTED:** "the GPUI app spawns no child processes here (aggregate = one
  process)." Independent process-parent enumeration found a `conhost.exe`
  child; its private working set at the first explicit sample was 1,200,128
  bytes (~1.14 MiB). The unchanged harness measures only the main PID.
  The original raw data therefore do not prove an aggregate-memory result.
  This may be a console-spike/harness artifact rather than a shipped GUI
  helper, but it must be counted or explicitly eliminated in the measured
  artifact, not assumed absent. A separate sampler records its overhead
  during the c retry; it is not a harness fix.
- **REFUTED for the first c run:** the machine was not continuously
  build-free, despite a zero-build precheck and no other cargo command from
  this verifier. Its CSV records 26 busy samples at t=56–167 s; the
  independent load monitor caught five samples with 1–3 cargo/rustc
  processes. The last 60 s were clean, but the whole run is retained as
  contaminated. No foreign build was killed or attributed to a worker.
  One c retry was launched only after three zero-build checks 5 s apart.
- The author's b steady medians span 20.9–22.3 MiB: a 1.4 MiB spread, not
  the claimed "< 1 MiB" agreement across all runs. Build overlap was recorded,
  but these data alone do not establish that builds affected only startup.
- The sentence saying b (~21.6 MiB) remains ~8 MiB above the anchor baseline
  (17.6 MiB) has incorrect arithmetic: the difference is ~4 MiB.
- Twenty cycles and five minutes do not establish absence of long-run growth;
  the author appropriately limits the no-leak observation to that short run.
- The committed `summarize.ps1` blindly accepts every CSV in its directory.
  With the extra load/aggregate CSV schemas in this verifier's evidence
  directory it prints spurious zero-memory rows for those files. These rows
  in `memory-summary.txt` are **invalid / NOT USED**, not passing measurements.
  `analysis-command.txt` selects explicit main-run schemas and separately
  analyzes load/aggregate records. No spike code was changed.

What would change the answer: representative UI/assets, real reconciliation
work, retained-window measurements, long soak, additional app-owned processes,
or a dependency/renderer change. No new resource thresholds are proposed here.

### S-forever — CONFIRMED (local installation identity)

Question: can authoritative variant/install identity, including "Forever",
be derived without executing the client or Lua?

**PASS / feasible on this installation**, high confidence in local agreement:
the active `.build.info` row and `_forever_\.flavor.info` both identify `wowf`;
the build row and executable version agree on `1.60.1.70938`.
The binary `.product.db` contains that product, folder and version.
An independent TOC-only count found 112 occurrences of `16001` across 171
TOCs, agreeing with the report. [Sanitized observations](S-check/identity.txt).

**PASS**: use the product/flavor identity cross-checked against the matching
installation record; do not identify the client by directory, executable name,
version, or Interface number alone. Unknown/disagreeing metadata must stop
automatic enrollment pending guided identity/safe-scope validation (R-02).
This refutes relabeling this local product as Classic Era merely from its age
or UI name; it is not an independently verified universal Blizzard product map.

Limits **NOT RUN**: public-source corroboration, client TOC selection behavior,
other installations/regions, and future product-code stability. `.product.db`
was searched for known byte substrings, not structurally decoded; strings alone
do not prove which protobuf field or installation row owns a value.
Interface/version agreement is correlation, not proof of client compatibility.
Those limits qualify the report's broad "authoritative" recommendation, not
its observed local product/flavor/version values.

Privacy **PASS** for this report under the bounded scan below; no personal
identifier found. The original owner-review-before-commit gate is unchanged:
this verifier did not commit the S-forever author's report.

### S-dropbox — PARTIAL

Question: what real Dropbox cloud-file states and filesystem behavior can
be established from this client?

**PASS / feasible** for local placeholder conversion and temp-then-rename
on this Windows client; confidence medium, two synthetic repetitions.
Dropbox 279.4.5723 was running. Only a fresh GUID-named child of the exact
authorized `D:\Dropbox\Gaming Sync\WoW Sync Test` root was touched.
[Commands](S-check/dropbox-command.txt), [timestamped raw log](S-check/dropbox.txt).

- Both 4 MiB files changed from attributes `0x80020` to `0x80420` by the
  first ~1 s poll. Rep 2 showed cloud tag `0x9000201a`, then `0x9000601a`
  by ~3 s. Rep 1 retained `0x9000201a` through the final ~8 s poll.
  Thus placeholder conversion is confirmed; a universal 8 s final-tag or
  upload-complete guarantee is not.
- Two temp-then-rename replacements changed `synthetic-v1` to `synthetic-v2`,
  removed the temp name, and returned in 7.55 / 1.02 ms.
  This confirms before/after local visibility and successful replacement,
  **not** atomicity to concurrent readers or peer delivery/apply.
- Cleanup **PASS**: first removal failed, the second succeeded after 5 s;
  the verifier's own directory no longer exists. No other Dropbox entry
  was enumerated. The transient failure is observed; its cause is unknown.
- Attribute and reparse-tag queries are separate calls. In rep 1 the initial
  attribute read preceded a successful reparse query: they are not a coherent
  atomic snapshot and should not be treated as one.

**BLOCKED**: genuine online-only hydration, multi-computer conflicts and
macOS provider behavior. **NOT RUN**: attrib pin toggles (the author already
recorded two unsuccessful approaches), large uploads or new status APIs.
No data were claimed online-only.

Overclaim findings **FAIL** (scope of inference):

- The committed raw script says "read online-only file" while its recorded
  attributes remain pinned; the prose correctly disclaims that label.
- In the author's upload test, polling starts **after** the synchronous
  writer closes the file. Seeing full size from the first stat does not
  test partial visibility **during** the write.
- "Completion detection NOT possible" is stronger than the experiment:
  no reliable signal was demonstrated from the tested attributes/tags/locks.
  Shell properties and Cloud Files APIs were explicitly untested.
- Successful temp-then-rename is not a proof of atomic replacement; nor do
  transient deletion failures establish Dropbox, rather than another holder,
  as their cause.

What would change it: owner-created online-only synthetic data, an instrumented
concurrent reader, supported status APIs, and a second real client. Publication
must remain distinct from remote application/confirmation (R-12).

## Every Phase 2 headline: label/data audit

These are **PARTIAL** evidence-consistency findings unless an execution above
is explicitly marked PASS. Reading an author's raw log is not rerunning it.

| Evidence | Audit of headline and labels against supplied data |
|---|---|
| [S-gpui](S-gpui.md) | PASS for consistency of scoped synthetic Windows lifecycle claims: raw external trace shows hidden/alive, two synthetic tray callbacks and one instance; self-test logs quit. Real shell clicks are honestly NOT RUN; ARM64 build attempt BLOCKED; macOS builds NOT RUN locally. Overall four-target feasibility remains PARTIAL. The memory acceptance references to A-37 are wrong: the memory case is A-29. No cross-target CI result was independently checked here. |
| [S-mem](S-mem.md) | PARTIAL: independently measured above; main-PID numbers support a wide margin below 100 MiB, but the claimed absence of child processes is REFUTED. The original aggregation, retained-window/product lifecycle and a different pin are not proved. Quantitative prose errors and first-run build contamination are listed above. |
| S-forever, uncommitted | CONFIRMED for local metadata agreement only; the original report was not added to this branch. Other-install behavior, public claims, Interface semantics and binary-field association remain unverified. |
| [S-addon](S-addon.md) | PARTIAL: raw coverage shows 95/102 recognizable version schemes (93.1%), not 95 proven correct real-release orderings. Real pairs are explicitly unavailable and ordering tests synthetic. "Confident" groups covering 77/102 folders overstates package identity: the same report identifies AtlasBIStooltips over-grouped with AtlasLootClassic merely by dependency. Derived coverage counts are useful; automatic package/uninstall boundaries are not proved. |
| [S-state](S-state.md) | PARTIAL: supplied redacted survey supports 31 account + 16 character owned files; perturbation/empty inventories are clearly labeled rather than real-world failures. Real-client serialization and active flavor selection were not verified; union-of-TOCs may over-accept. The scanner explicitly does not fully validate literals. "Feasible" is conditional on those conventions, not universal safe ownership. The individual packet expressly permits the dev-PC survey, despite PLAN's older "synthetic fixtures only" summary. No Lua executed by this verifier. |
| [S-proc](S-proc.md) | PARTIAL: raw output supports synthetic results and honestly labels real-client work NOT RUN/BLOCKED. Recheck still has 1 violating commit in 20 trials; probes caused 38,473/74,865 failed writer opens at 100 ms. Thus headline "feasible" cannot be read as satisfying strict R-11. Race-safe apply/recovery and non-disturbing updater stability remain design/owner blockers, not solved guarantees. |
| [S-dropbox](S-dropbox.md) | PARTIAL: real local-client tests are distinguished from unavailable hydration/conflicts/macOS. Attribute conversion and local rename independently rerun above. Upload-during-write, atomicity and categorical impossibility/causation overclaims are listed above. |
| [S-a11y](S-a11y.md) | PARTIAL: raw UIA dumps show 0 descendants on registry 0.2.2 versus 16 on the git pin. Action log shows Invoke and Toggle work, but SetValue returns success without changing the value; prose discloses that limitation. Broad "UIA PASS" must mean exposure, not every action. Narrator speech NOT RUN and macOS BLOCKED are honest. No independent UIA or speech execution here. |
| [S-a11y-design](S-a11y-design.md) | PARTIAL: source/lockfile analysis and author-supplied runtime evidence are distinguished; release-tag build is NOT RUN, speech NOT RUN and VoiceOver BLOCKED. Recommendations and 10–20% effort estimates are proposals, not measured outcomes or an approved D-12. Its warning that changing pins invalidates memory/lifecycle evidence is important. Wrong memory reference A-37 should be A-29. |

No spot-checked build-only claim was presented as runtime evidence for another
architecture. **BLOCKED** items remain blockers; this audit does not waive
SPEC §7, E-09, D-12 or the original S-forever review.

## Data hygiene and permissions

**PASS, bounded static/visual review; not a guarantee of absence.**
[Scan commands](S-check/hygiene-command.txt), [results and triage](S-check/hygiene.txt).
Compared tracked text against account/realm/character folder names obtained
privately from the approved dev WTF tree (1 account, 1 realm, 8 characters).
No identifier values or saved-variable contents were printed or persisted.
The initial scan covered 182 text files in the tracked-tree/report scan
(including the uncommitted report), skipping 19 binary PNGs. Its 17 identity matches were
numeric measurement coincidences: elapsed-time fields and a build-sample count,
not copied account identifiers. The two email matches were the mandated public
Copilot commit trailer. No current-user name, absolute user profile, SID or
high-confidence credential-pattern match was found. A final staged-file scan
covered 215 text files plus 19 separately viewed binary PNGs: 25 numeric-only
identity coincidences (the eight new matches were `t_s` / `elapsed_s` fields),
two public trailer emails, and zero user/profile/SID/BattleTag/credential
matches. Detailed updated counts and numeric triage remain in the log.

All 19 committed PNGs were separately viewed: synthetic UI content, no obvious
personal identifier or secret found. Their rounded window edges expose thin
strips of unrelated background text: **hygiene finding**, use tightly masked
window-only captures for future evidence. This review is not OCR or a guarantee
that every background fragment is innocuous.

Only derived TOC metadata, synthetic fixtures and redacted survey paths were
found in the reviewed spike evidence; this is not a source-provenance proof
for every byte. Token-pattern scanning cannot detect arbitrary secrets.
No outside Dropbox folder was listed/read; only exact approved paths were used.
The WoW installation was read-only; no Lua, launcher, game or legacy script
was executed, started, stopped, injected into or automated.

## Exact execution and rerun

All repository commands below start in `D:\GitHub\wow-sync.worktrees\S-check`
on `run001/S-check`. Command transcripts above retain the actual operations
and narrow data paths. Files named `*-command.txt` are PowerShell command
records, not new product tooling.

```powershell
Set-Location 'D:\GitHub\wow-sync.worktrees\S-check'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
rustc -Vv
cargo build --release --locked --manifest-path 'spikes\mem\Cargo.toml'
# Use two independent shells concurrently for these two commands, after building:
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\memory-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\load-command.txt' -Raw)))
# Do not overlap another cargo build with the measurements.
# Recorded c retry: run in separate shells with the parameterized load monitor:
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\memory-c-retry-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\load-command.txt' -Raw))) -Seconds 390 -Filename 'load-c-retry.csv'
# Additional read-only app/child sampler, started during that retry:
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\aggregate-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\analysis-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\identity-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\dropbox-command.txt' -Raw)))
& ([scriptblock]::Create((Get-Content 'docs\runs\run-001\evidence\S-check\hygiene-command.txt' -Raw)))
cargo xtask verify --tier offline
```

The installed counter definition was also queried directly:

```powershell
(Get-CimClass Win32_PerfFormattedData_PerfProc_Process).CimClassProperties['WorkingSetPrivate'].Qualifiers
$counter = [Diagnostics.PerformanceCounter]::new('Process','Working Set - Private','gpui_mem')
$counter.CounterHelp
$counter.Dispose()
```

In the recorded execution Dropbox ran during the rebuild, **not** during the
memory scenarios; identity reads and this evidence audit used the otherwise
idle measurement interval. The memory command uses committed
`spikes\mem\measure.ps1` and `summarize.ps1`; b is 180 s total (shown 60 s,
then closed idle), c is 360 s (20 cycles, then ~5 min idle), a is 120 s.
The total harness duration includes startup: for example b's first shown
marker occurs about 5 s after launch, leaving about 115 s, not exactly 120 s,
of closed-window idle.
Logs/CSVs, [first load samples](S-check/load.csv) and
[retry load samples](S-check/load-c-retry.csv) are retained without trim.

Repository offline verification: **PASS**, exit 0, after all memory samplers
finished; formatting, clippy, tests, required-case inventory, dependency check
and Markdown links passed on this Windows worktree.
[Gate output](S-check/offline-output.txt), [structured result](S-check/offline-report.json).
The root gate covers the product workspace, **not** standalone spike tests
or spike dependency/license audits. It is not a product/platform/provider pass.
Native/provider product acceptance: **NOT RUN**.
Remaining run budget is tracked by the orchestrator; the point-in-time model
record is not a replacement cumulative ledger.
