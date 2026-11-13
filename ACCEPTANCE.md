# Acceptance contract

Status: proposed verification contract for review with [SPEC.md](SPEC.md).

## Evidence rules

Every acceptance case has a named independent verifier, environment, actual
command/task sequence, observable expected result and retained evidence. Tests
must validate user intent, not merely check that source contains particular text.

Required cases cannot disappear through zero-test runs, ignored tests,
platform filters, mocked UI, missing credentials or "could not reproduce"
without the missing gate being visible. Simulation and real-provider evidence
are complementary, not interchangeable.

Each change links to requirement and scenario IDs. Verify complete supported
flows, not a happy-path proxy. A screenshot is not proof that its button works;
a method-level test is not proof that the installed app can recover.

## Product acceptance scenarios

| ID | Requirements | Scenario and observable success |
|---|---|---|
| A-01 | R-01, R-02, R-13 | Install/enroll on every declared OS/architecture combination. The real app opens, remains available in the tray, hides/reopens its window, quits explicitly and exposes startup controls without a terminal. No discrete GPU is required. |
| A-02 | R-02 | Enroll multiple side-by-side production/test variants, including an independently verified fixture for the owner's "Forever" installation. Same-named addons/settings never cross variant boundaries. Unknown metadata is not relabeled as Classic Era. |
| A-03 | R-02, R-07 | Use matching and different account/character identities, including case/Unicode differences. Intended matches follow the user; unrelated identities and native WoW UI-state files remain untouched. |
| A-04 | R-03, R-04 | Join with existing local addons/settings and explicit setup consent. Policy applies automatically; replaced data is recoverable. A locally absent addon is not misinterpreted as an uninstall. |
| A-05 | R-03, R-11 | Use hydrated and unavailable cloud files/directories, offline Dropbox, moved roots, denied permissions and a competing local writer. State is accurate, protected paths remain untouched, and pending data is not read as complete. |
| A-06 | R-04, R-05, R-12 | Update/add an addon on one computer, apply on others, and inspect provenance. Received bytes and effective code match the selected release; origin and decision are reviewable. Confirmations occur after application. |
| A-07 | R-05, R-06 | Independently update addon A and addon B on isolated replicas. After delivery, both safe newer releases converge automatically without a whole-profile manual conflict. |
| A-08 | R-05, R-06 | Compare ordinary newer releases plus ambiguous beta/stable, manually modified, duplicate-label/different-content and unreliable-label cases. Proven comparisons automate; ambiguity stops only the affected scope with preserved alternatives. No timestamp-only winner. |
| A-09 | R-05, R-07, R-11 | Exercise logical addons containing multiple folders/files and unknown/shared saved-state ownership. No mixed package, inconsistent shared-file split, whole-WTF fallback or deletion of unowned state occurs. Unsupported identity or inseparable scope is explained, not silently guessed. |
| A-10 | R-04, R-06, R-10 | Deliberately uninstall: active code and settings disappear on applicable computers, while an older recovery point restores both. Separately collide uninstall with an offline update: require an addon-level choice, without silent resurrection/deletion. |
| A-11 | R-07, R-08 | Choose a default master and per-addon exception, finish play sessions, and discover a new addon. Applicable settings publish automatically from the right authority at stable completed-session boundaries. Repeated file writes do not exhaust history. |
| A-12 | R-07, R-08 | Make routine unsent laptop settings changes, then publish master settings. Master state applies automatically; the replaced laptop state is recoverable. No automatic laptop push occurs. |
| A-13 | R-08 | Explicitly push non-master settings. Every applicable computer, including the master, adopts the accepted copy. Master automatic processing does not immediately republish its stale previous copy. |
| A-14 | R-08, R-09 | Compete an intentional laptop push with unpublished intentional master settings. Neither copy is silently lost or merged; the GUI offers addon-level choice and safe unrelated work proceeds. |
| A-15 | R-08, R-09, R-12 | Pause one addon's settings, then pause all settings locally. Protected experiments are neither uploaded nor replaced. Resume explains pending differences. Intentional pause is not represented as fully synchronized. |
| A-16 | R-10 | Create more than three ordinary history states and inspect code/settings history independently. Current plus two previous remain, with necessary paired restoration data. Pruning never corrupts active state or advertised recoverability. |
| A-17 | R-06, R-10 | Require extra copies for unresolved decisions/recovery. Necessary copies survive with a visible exception, later returning safely to target depth. Low storage yields accurate needs-help behavior rather than destructive pruning. |
| A-18 | R-10 | Reject a bad release and recover an earlier one with associated settings. All applicable computers receive the correction; a late/offline peer cannot reintroduce the rejected release. Different variants are unaffected. |
| A-19 | R-10, R-12 | Remove the associated old settings checkpoint, then perform rollback. Code-only recovery proceeds, current settings are preserved, and a clear post-operation warning distinguishes partial from paired recovery. |
| A-20 | R-03, R-06, R-11, R-12 | Use three independent Dropbox replicas; reorder/delay/duplicate/delete deliveries, split metadata and payload availability, and introduce late competing decisions. No stale fallback, false success, timestamp overwrite or accidental deletion is inferred. |
| A-21 | R-11 | Start WoW during detection/staging/application attempts; finish a session with delayed writes; keep an updater open but idle and then actively writing. Demonstrate safe blocking, stability handling and race/recovery behavior. No process is killed. |
| A-22 | R-10, R-11 | Inject crash, disk-full, permission failure and restart at durable mutation transitions, including restoration failure. Retained prior data is usable/recoverable; the next start reports/reconciles the actual state without false success. Exercise real temporary filesystem effects, not only mocked calls. |
| A-23 | R-11, R-15 | Supply malicious/malformed payloads, unexpected paths, unsafe links, conflicting cross-platform names and executable-looking Lua/CMD content. No execution, escape, identity mixup or unauthorized write occurs. Valid unrelated data remains intact. |
| A-24 | R-12 | Observe one current computer, one pending update, one long-unused computer and a late acknowledgement. The GUI distinguishes last confirmed state from unknown present state; no group-wide "all current" claim precedes required confirmations. |
| A-25 | R-06, R-12 | Inspect automatic choices, conflicts, removals, failures and recovery through the GUI. A fresh verifier identifies what changed, which source was chosen, why, affected scope and the next action without internal revision knowledge. |
| A-26 | R-13 | Run real GUI setup, safe sync, settings exception/push/pause, conflict selection, rollback and error recovery. All supported flows complete without a terminal. Keyboard/focus/scaling/themes work; screenshots are accompanied by interaction evidence. |
| A-27 | R-13 | Generate multiple successful item changes, an actionable conflict and an unchanged persistent condition while gaming. Successful work produces grouped summaries, nonurgent alerts wait, actionable status remains available, and unchanged conditions do not spam notifications. |
| A-28 | R-13, R-16 | A new application release is available. The app honestly notifies and guides manual download/install; it does not perform an unrequested background binary replacement. Unsigned status and upgrade/uninstall effects are accurately explained. |
| A-29 | R-14 | Measure locked release builds on representative supported hardware: background aggregate private resident memory is at most 100 MiB at idle, including after UI use and repeated cycles; report GPU and working peaks separately. No hidden helper offload or long-run growth. |
| A-30 | R-11, R-14 | Make complete safe work ready under controlled conditions and measure actual useful sync start. It begins within ten seconds. Repeat with legitimate blockers to verify accurate waiting and resumption instead of gaming the clock or prematurely declaring completion. |
| A-31 | R-14 | Run the owner-approved post-spike CPU/latency/throughput/working-memory/soak workloads, including normal gaming. Preserve raw measurements and comparisons. A threshold is not changed after a failure without approval. |
| A-32 | R-15 | Capture all default telemetry against a test endpoint with planted private paths/names/settings/secrets and unexpected errors. Only the approved anonymous schema leaves the process. Opt-out stops reporting and does not secretly preserve a send-later queue. |
| A-33 | R-15 | Produce a diagnostic export. It is explicitly requested, previewed/redacted, bounded, useful, and contains no unapproved private source data. Provider/IP handling and consent restrictions are honestly documented. |
| A-34 | R-01, R-03, R-11, R-16 | Execute installed-app end-to-end transfers and recovery through real Dropbox on Windows and macOS, with enrolled cross-platform peers and supported cloud-file states. Retain real provenance; simulated tests cannot satisfy this row. |
| A-35 | R-01, R-13, R-16 | Execute platform-specific installed-app/native filesystem/tray/GUI smoke on every claimed architecture and supported OS branch. Build-only or emulated evidence is labeled and cannot silently replace required runtime proof. Missing environments block the corresponding support claim. |
| A-36 | R-03, R-11, R-16 | Exercise incompatible shared data, a stale application, an old competing sync job and path relocation. The app does not overwrite unknown data or execute/delete legacy scripts; GUI guidance supports a safe next action. |
| A-45 | R-03, R-07, R-12 | Retire a computer through the GUI, including a settings master. History remains; group currentness no longer waits for that device; stale/offline publications do not reactivate it. Explicit re-enrollment is required for new changes. Master absence is explained with reassignment guidance, not a silent authority switch or collection deletion. |
| A-46 | R-02, R-03, R-11 | Enroll an unfamiliar side-by-side client through guided identity and safe-scope confirmation. No bytes cross variants before validation; a friendly label is not sufficient proof. An unvalidated layout remains blocked with a useful explanation, while validated enrollment does not require an app update just to recognize a name. |

## Project and engineering acceptance

Engineering requirements are defined in [ENGINEERING.md](ENGINEERING.md).

| ID | Requirements | Observable success |
|---|---|---|
| A-37 | E-01, E-02 | A new run records actual Opus 5.5 orchestration, approved role/model matrix and cumulative budget. No worker starts before approval; unavailable required models or exhausted limits stop/escalate rather than silently substituting. |
| A-38 | E-03, E-04 | An independent verifier demonstrates a behavioral test fails for a relevant broken implementation, then passes for the fix. No-op implementations and weakened assertions cannot satisfy the requirement. |
| A-39 | E-04, E-08 | From a clean checkout, documented verification discovers and executes the required scenario set with environment-aware reporting. Missing/skipped required tests fail the release gate rather than producing a misleading green result. |
| A-40 | E-05, E-08 | A fresh worker, given only its work packet and repository, locates authoritative intent, acceptance IDs, boundaries, dependencies and verification commands without original chat. A clean handoff resumes without lost decisions or reset budget accounting. |
| A-41 | E-03, E-06 | Reviewer rejects a seeded private-data upload, instruction-like payload or unjustified dependency/permission change. No agent uses real user data, paid services or new credentials outside approval. |
| A-42 | E-07, E-09 | The independent worker completes real GUI tasks with retained evidence. Owner approves the early prototype and release candidate. Screen-reader limitations and unresolved measurements are not silently excused. |
| A-43 | E-08 | A representative domain change is testable without GUI/network/provider setup and does not require a second policy implementation. Formatting, lint/static checks, dependency/license review, architecture and documentation checks run reproducibly. |
| A-44 | E-02, E-09 | A repeated failure or missing owner prerequisite pauses the run with reproduction/evidence, remaining budget and a precise request. No endless workers, fake validation or unapproved public publish occurs. |

## Coverage and environments

### Deterministic offline layer

Use temporary installations and at least three independent replicas, with
independently controlled delivery/failures. Cover state-machine/property tests,
real temporary filesystem integration, protocol/identity adversarial inputs and
installed-app behavior. Tests must not require private user data or accounts.

### Native platform layer

Maintain a dated matrix of supported OS branches and architectures, exact
hardware/runtime/build identity, native cloud-file/process/filesystem behavior
and real tray/GUI interaction. Cross-compilation is useful but is not execution.

### Real-provider layer

Run real Dropbox scenarios on both Windows and macOS in explicitly provisioned
test accounts/folders. Owner-supplied access is a prerequisite, not something an
agent may invent, scrape or reuse from unrelated contexts. Preserve the
distinction between provider behavior observed and failures induced in simulation.

### Human acceptance layer

The owner sees an early rendered prototype and the release candidate. Supply
actual interaction recordings/screenshots, task outcomes, error paths and known
limits. A mocked prototype must be labeled as such. Agent approval of aesthetics
is not owner usability acceptance.

## Required evidence record

For each applicable case record:

- Case and requirement IDs, source commit, installed artifact/version and build mode.
- Verifier identity/role and actual model where applicable.
- OS/architecture, hardware/data profile, provider mode and prerequisites.
- Exact execution/interaction procedure and actual pass/fail/blocked/skipped status.
- Expected and observed state, including private-data-safe hashes or comparisons.
- Logs, GUI evidence, measurements and reproduction information where relevant.
- Any limitations, approvals, unresolved defects and links to the deciding record.

CI artifacts may expire only under an explicit retention policy that preserves
release decisions and reproducibility. Never commit secrets, raw private settings
or unrestricted user dumps as test evidence.

## Release decision

All mandatory cases and their prerequisites must be accounted for. A failing
or unexecuted mandatory gate is not green because the rest passed. Approved
limitations must be explicit, bounded and consistent with the advertised scope.
Scope changes require owner approval and updated traceability, not a renamed test.

The owner must accept the candidate and separately authorize publication.
