# WoW Sync: product intent and definition of success

Status: approved by the owner at revision `7a2ccdf` on 2026-11-14
(docs/OWNER-DECISIONS.md D-03). Implementation requires an approved run plan.

## 1. Purpose

The user plays World of Warcraft on several computers and uses CurseForge
primarily to discover, install and update third-party addons. Existing sync is
too opaque: the user cannot confidently explain what changed, who published it,
which computer applied it, or how to recover.

This product should make moving between computers routine and trustworthy.
Users should not perform normal synchronization, diagnosis or recovery with
shell commands. They should understand uncertainty instead of receiving
unjustified "everything is synced" messages.

### Goals

| ID | Intent | Success looks like | Implemented through |
|---|---|---|---|
| G-01 | Follow the user across computers | Installed addon changes propagate automatically; individual computers confirm application | R-03, R-04, R-05, R-06, R-12 |
| G-02 | Make authority and exceptions understandable | Code, settings masters, explicit pushes and local experiments have different, explainable rules | R-02, R-07, R-08, R-09, R-12 |
| G-03 | Preserve recoverability | Bad changes, removal, conflict and interrupted operations do not silently destroy the necessary recovery choices | R-04, R-06, R-10, R-11 |
| G-04 | Stay unobtrusive and usable | Low idle cost, useful notifications, clear status, and complete GUI workflows | R-12, R-13, R-14 |
| G-05 | Be a credible public product | Explicit platform claims, real provider tests, privacy, honest unsigned distribution and maintainable releases | R-01, R-02, R-15, R-16 |
| G-06 | Be reproducibly agent-built | A fresh orchestrator can discover intent, assign bounded work and obtain independent evidence without relying on chat memory | E-01, E-02, E-03, E-04, E-05, E-06, E-07, E-08, E-09 |

These goals are not satisfied by compilation, a rendered window, green unit
tests alone, or a worker reporting that it is done.

## 2. Scope, constraints and vocabulary

The first release is public and the source is MIT-licensed. Application
implementation is Rust with a GPUI interface. No permanent duplicate PowerShell
business-logic engine is intended. Platform tooling may still be used for
bootstrap, packaging or testing.

- **Variant:** a distinct WoW client type/build family or installation context.
- **Sync group/profile:** an explicitly enrolled logical collection across
  computers for a particular variant. A display name is not proof of identity.
- **Addon:** a logical third-party addon package, which may contain multiple
  folders or associated saved-variable files. A folder name alone is not always
  sufficient identity.
- **Settings:** third-party addon saved state, including applicable account-wide
  and character-specific data. WoW's own UI/cloud-sync state is excluded.
- **Master:** the default automatic settings publisher for a group, overridable
  per addon. It is not a permanent authority for addon code.
- **Explicit push:** intentional publication of selected non-master settings.
- **Confirmed:** a computer has reported the applied state, not merely observed
  files in Dropbox.
- **Ready:** required local bytes and metadata are complete, verified and stable;
  WoW activity, an unresolved decision or another safety condition does not
  prohibit the operation.
- **Recovery point:** recoverable addon code and associated settings information;
  this term does not prescribe a physical archive format.

The architecture, container/database format, protocol, libraries, scheduling
mechanism and worker-model matrix are not prescribed by this product spec.
Technical decisions must demonstrate how they satisfy the intent.

## 3. Product requirements

### R-01: Supported product and deployment

Support Windows 11 x64 and ARM64, and macOS Apple Silicon and Intel. Support the
current and previous vendor-supported major/feature releases, retaining the
previous release for at least six months after a new release. Keep a dated,
explicit supported/tested matrix; do not imply support for vendor-unsupported
OS releases or architectures never executed.

Reference computers are reasonably new, have at least 16 GB RAM, typically
32 GB, and at least 1 TB storage. A discrete GPU is not required. Storage capacity
does not imply SSD performance; representative hardware/data must be measured.

Initial public builds may lack publisher signing/notarization, with clear,
accurate installation warnings. Do not advise globally disabling OS protections.
Future signing is not a blocker for the first release, but artifact integrity,
provenance and install/uninstall behavior still need evidence.

### R-02: Variant, installation and identity isolation

Account for all installed WoW variants, with explicit enrollment of test/PTR/beta
clients. Retail, progressing Classic and other side-by-side clients must not
share code/settings merely because addon names or directory labels match.

The owner identifies "Forever" as a separate newer installation, not Classic
Era. Its actual product/build metadata requires verification. An unfamiliar
client can be enrolled through a guided explicit identity and scope confirmation;
do not require an app update merely because its name is unknown. No transfer may
occur until identity, addon/settings layout and safe scope are confirmed.
User-entered naming alone does not validate an unsupported layout. Never guess
a mapping to another variant or copy across clients as a discovery mechanism.

Follow matching account and character identities across computers. Do not copy
between different identities automatically. Cross-identity migration is not a
first-release goal. Users must be able to see the variant/group/identity context
before a consequential operation.

### R-03: Enrollment and Dropbox prerequisites

Require the Dropbox desktop client. Guide users through selecting their shared
location, establishing a group, identifying computers, choosing the settings
master and keeping required data available offline.

Explain the effects of setup consent, including automatic settings sharing and
replacement of local group-managed data. After consent, joining an existing
group may apply its policy automatically, preserving replaced local data for
recovery. New enrollment must not reinterpret a merely absent local addon as
a deliberate group-wide uninstall.

Support diagnosis of unavailable/online-only data, missing permissions, moved
paths and another local writer. Do not claim that requesting offline availability
means downloading/validation has completed.

Provide a GUI retirement action for a permanently removed/replaced computer.
Stop expecting its confirmations, preserve its historical record and require
explicit re-enrollment before accepting new changes from that identity. Delayed
delivery of pre-retirement history is not proof of renewed membership. Retirement
is not deletion of its addon collection from the remaining computers. If it was
a settings master, surface the absent authority and an explicit reassignment path
rather than silently electing a replacement.

### R-04: Collection membership

Participating computers follow the same addon collection for a group. New addons
and deliberate removals propagate to all members when safe.

An uninstall removes active addon code and its applicable saved settings.
Earlier recovery points remain safe and restore both. A missing Dropbox object,
incomplete join or temporary local/provider failure is not deletion authority.

### R-05: Automatic addon-code reconciliation

Share the newest versions already installed on participating computers of the
same game variant. CurseForge or another updater obtains releases. This app does not
become an addon catalog, downloader or substitute updater.

Automatically combine independently updated addons when their identities and
ordering are demonstrable. Keep a reviewable explanation of the evidence and
chosen result. Identical content should converge without unnecessary replacement.

Do not use file modification time, computer clock or an arbitrary machine
preference as proof of a newer release. Version labels, channels, manual edits,
multi-folder packages and identical labels with different contents can be
ambiguous. If ordering cannot be established, require a decision for that addon
and continue safe work elsewhere.

The app is a sync tool, not addon police (D-21). Whether an addon works with
a game build is the user's responsibility. Never withhold an addon because of
a game update or its `## Interface:` declaration. Variant and matching
account/character boundaries still apply.

Do not claim semantic compatibility that the app cannot establish. Version
recognition must be tested with representative real metadata, not fixtures
invented only to make the comparator pass.

### R-06: Conflict scope and competing membership intent

Keep independent safe addons progressing when another addon needs attention.
Expose the affected addon or genuinely inseparable package/dependency scope,
not an unexplained whole-profile failure.

If one computer deliberately uninstalls an addon while an offline computer
updates it, the removal wins (D-21). The offline computer's updated copy is
preserved in recovery and named in the summary, so the user can restore it.
Preserve the alternatives and provenance.

The same observed inputs and accepted policies must produce the same result.
Out-of-order, duplicate or late arrivals must not silently reverse a resolved
decision. New information can create a new visible decision; uncertainty is not
evidence that nothing happened.

### R-07: Settings scope, authority and checkpoints

Synchronize third-party addon state only, not the entire WTF tree or WoW's own
UI/cloud-sync files.

Unknown or shared saved-file ownership requires diagnosis for the affected
scope. Do not fall back to copying the whole WTF tree, split a shared file
inconsistently, or delete unowned state to satisfy an addon removal.

Each variant/group has one default settings master with per-addon exceptions.
Choosing a master enables sharing for all addon settings, including newly
discovered addons. This scope must be understandable during consent.

Capture useful automatic settings history at completed play-session boundaries,
not every filesystem write. Do not treat an interrupted/incomplete write as a
successfully captured session. Define safe initialization and missed-session
handling during the technical design.

### R-08: Routine changes, explicit pushes and experiments

Routine unsent settings changes on a non-master do not become automatic
publications. When a new master version arrives, it wins automatically over
routine local changes, preserving the replaced copy for recovery.

A non-master can deliberately push settings. An accepted push becomes the new
shared settings state, updating all applicable computers including the master.
The master must not immediately undo it with an unobserved/stale automatic push.

Provide settings pause per addon on a computer and pause-all-settings there.
Local experiments must not be published or overwritten while protected.
Resuming must explain any pending competing state rather than accidentally
broadcasting an experiment. Other safe addon work continues.

### R-09: Competing saved-settings choices

Never merge competing addon saved settings automatically. If intentional choices
compete, including a non-master explicit push and unpublished master changes,
ask which copy to keep for that addon. Preserve both until resolution and
continue other safe addons.

Do not confuse settings conflicts with addon release ordering. A timestamp
does not determine which preferences the user intended. The GUI must explain
the source, identity scope, affected addon, consequence and recovery choices
without requiring knowledge of Lua or internal revision IDs.

### R-10: Recovery, retention and rejected releases

Retain three history states total per addon, independently for code and settings:
current plus two previous. Preserve necessary unresolved/recovery copies
temporarily beyond that target, explain the exception and return to three once
safe. Do not prune an associated settings copy still needed for a promised
recovery point. Explain storage pressure rather than silently destroying safety.

Rollback rejects the bad addon release for the sync group and distributes the
recovered version. A late/offline computer must not reinstall the rejected
release simply because its release number is higher.

Automatically restore an associated earlier settings checkpoint when available.
If it is missing, proceed with code-only rollback, preserve current settings
and warn afterwards. Report that fallback distinctly: do not claim a complete
paired restore or proven settings compatibility.

Uninstall and rollback history must allow the user to recover the removed addon
and its settings together. Rejection and restore scope must respect variant,
addon and account/character boundaries.

### R-11: Safe operation and recoverable failure

Never apply game-file changes while WoW is running. An updater may remain open
when its work has finished and the files are stable. Do not kill game/updater
processes, inject into the game or bypass its protections.

Design and test launch-during-transfer races, unstable sources, interrupted
sessions, application/OS crashes, storage exhaustion and failed restoration.
Quiescence must be revalidated at consequential points, not checked only before
a long staging operation. Unsupported guarantees must be escalated honestly.

Validate scope, integrity and complete intended content before replacement.
Prevent overlapping local writers. A failure must leave intact/recoverable
prior data and an accurate recovery state; it must not report success for a
partially applied collection.

Do not execute addon code, saved-variable Lua, legacy CMD metadata or remote
payloads to discover or validate them. Prevent traversal, unsafe links, identity
confusion and unintended writes outside explicitly authorized locations.

### R-12: Truthful status, provenance and audit

Prioritize the current computer's state: success, working/waiting, needs help
or error. Show other enrolled computers' last confirmed states and confirmation
times. Support drilling into what differs, what is waiting, and why.

Never count offline/stale computers as confirmed current. Distinguish:
local publication, locally available shared data, local application, and actual
per-computer confirmation. Explain the observation time/limits rather than
pretending to know a powered-off computer's present state.

Users must be able to review automatic addon-version choices, settings source,
explicit pushes, removals, rejections, recovery, failed/pending operations and
the next meaningful corrective action. Audit records must not leak private
settings into telemetry.

### R-13: Tray, GUI, notifications and application updates

Provide a polished, consistent cross-platform utility respecting normal OS
window and interaction conventions. It remains available in the tray, with
clear pause/quit behavior and understandable startup control. Closing a window
must not misleadingly appear to stop an agent that continues working.

Setup, ordinary use, conflict handling and all supported recovery work through
the GUI. An optional advanced CLI must not be a hidden requirement or duplicate
the domain engine.

Require keyboard operation, visible focus, scaling/high-DPI and themes. Test
screen-reader feasibility early; any release limitation requires explicit owner
approval rather than silent omission.

Send one grouped summary per completed sync batch; actionable problems are
separate. Defer nonurgent notifications while gaming and avoid repeated alerts
about an unchanged condition.

Initially notify about new application releases and guide manual download/install.
Automatic application replacement is not a first-release requirement.

### R-14: Performance and responsiveness evidence

Approved initial gates:

| Measure | Requirement |
|---|---|
| Idle background memory | At most 100 MiB aggregate private resident memory for all app-owned background processes |
| Stretch goal | 50 MiB idle background memory; failure of this stretch alone is not a release failure |
| GPU allocations | Report separately; do not hide integrated-GPU or renderer costs |
| Ready-change reaction | Start necessary safe synchronization within ten seconds of readiness |

Memory measurements include steady background operation after startup and
after opening/closing the interface, not only an artificially pristine process.
Do not offload work to uncounted helpers to pass. Use locked release builds,
repeatable workloads and disclosed platform-specific measurement definitions.

The reaction gate excludes external Dropbox delivery time, powered-off clients
and legitimate safety blockers. It measures beginning useful safe reconciliation,
not falsely declaring transfer complete before validation.

CPU, working-mode memory, startup, interaction, throughput and long-run budgets
must be proposed after early measurements and approved before becoming release
gates. Do not quietly use earlier suggested numbers as approved targets. Measure
background impact during play and bound work/queues/storage; no unbounded growth
or unexplained resource consumption is acceptable.

### R-15: Privacy, security and consent

Anonymous crash/performance reporting is on by default with a clear opt-out.
Make the policy discoverable before first transmission and verify that opting
out actually stops reporting. Anonymous is a property to demonstrate, not a label.

Default reports must not contain account/character/computer names, private paths,
saved settings, addon contents, credentials or stable identifiers that defeat the
anonymity claim. Do not upload unrestricted memory dumps or arbitrary error text.
Use a documented minimal schema, validate/redact locally, and fail closed.

Document provider handling of unavoidable transport metadata and applicable
consent/legal constraints. If the chosen service cannot meet the contract,
request an owner decision rather than weakening privacy silently. New paid
services or credential access need separate approval.

Diagnostics remain useful locally. Any support export must be explicitly
requested, reviewed/redacted and bounded. Dropbox group members can access
shared data; disclose that trust boundary. Integrity checking is not a claim
that third-party addon code is malware-free.

### R-16: Distribution, compatibility and supportability

Publish supported installers/packages, version and provenance information,
integrity checks and honest unsigned-installation guidance. Demonstrate install,
upgrade notification, normal shutdown and uninstall without silently deleting
recoverable user data.

Keep a clear protocol/application compatibility policy. Unknown or incompatible
shared data must not be silently overwritten or treated as an empty collection.
Explain when a manual app update or other intervention is required.

Guide migration from older sync tools and prevent known competing local writers.
Do not automatically execute/delete old scripts, archives or tasks. Existing
PowerShell behavior is a reference, not the new protocol or acceptance authority.

## 4. Representative user scenarios

1. The desktop updates an addon through CurseForge. After the play/updater safe
   point, the agent publishes. A laptop applies it when its own conditions permit.
   The desktop shows that laptop confirmed only after the actual acknowledgement.
2. Different computers independently update different addons. Safe newer copies
   combine automatically, with an understandable decision history.
3. Two copies of one addon cannot be ordered. That addon needs attention; other
   safe addons continue. The user resolves it in the GUI.
4. Routine laptop saved state is replaced by a master settings update, while the
   prior laptop copy remains recoverable. A protected experiment is not replaced.
5. A laptop explicitly shares settings. If no intentional choice competes,
   everybody including the master adopts it. Competing intentional settings
   require an addon-level selection, never an automatic merge.
6. A bad release is rolled back for the group with associated earlier settings
   when available. An offline peer later returns with that bad release; it stays
   rejected. A missing settings checkpoint produces the chosen code-only warning.
7. An addon is deliberately removed. Its active settings are removed too.
   Restoring an older recovery point restores both. A concurrent offline
   update loses to the removal and is kept in recovery.
8. A computer is unused for weeks. Its last confirmation remains visible but is
   never described as current. Returning cannot silently undo accepted decisions.
9. A computer is permanently retired. History remains visible, but confirmation
   expectations stop and delayed data cannot silently reactivate its authority.
10. A newly installed client is unfamiliar. Guided enrollment confirms its
    identity and safe scope before it participates, without conflating it with
    another side-by-side installation.

Each scenario needs observable evidence in [ACCEPTANCE.md](ACCEPTANCE.md).

## 5. Definitions of success

### User-scenario success

The owner can complete the representative week across computers without routine
commands, guesswork or hidden settings propagation. The owner understands current
state at a glance and can drill into causes/actions. The owner approves both the
early rendered prototype and the release candidate after real GUI task evidence.

### Code success

The installed application satisfies the requirements under positive, negative,
interrupted and reordered scenarios. Safety is checked independently of the
implementation's own assertions. No release-critical case is silently skipped.
Core behavior is testable without rendering a UI; GUI, background operation and
optional CLI do not become independent implementations of policy.

### Project success

A clean checkout can bootstrap and verify using documented, reproducible steps.
A fresh agent can locate authoritative intent, current work, permissions and
evidence without the original chat. The Opus 5.5 orchestrator stays within
approved role/model/resource boundaries and only claims what independent
evidence establishes. See [ENGINEERING.md](ENGINEERING.md).

### Public-release success

The supported OS/architecture claims are backed by runtime evidence and real
Dropbox end-to-end testing on Windows and macOS. Required GUI, privacy,
performance, recovery, packaging and documentation gates pass. Approved
screen-reader limitations are disclosed. The owner accepts the release candidate
and separately authorizes public publishing.

## 6. Non-goals

- Addon discovery, provider downloads or automatic CurseForge replacement.
- Synchronizing WoW's own cloud/UI state or arbitrary installation directories.
- Automatic merging of competing saved settings.
- Automatic cross-variant or cross-account/character migration.
- Instantaneous global consistency, proof of delivery to powered-off computers,
  or a Dropbox-local lock that excludes other computers.
- Protection against a compromised trusted Dropbox account, malicious upstream
  addon behavior or every action of other software.
- Automatic application self-updating in the first release.
- Linux in the first release.
- Reproducing the prototype's archive format, whole-WTF replacement, manual-only
  settings policy, confusing legacy switches or recovery limitations.

## 7. Mandatory feasibility decisions

Before large implementation investment, produce evidence and a recommendation for:

- GPUI/tray lifecycle, reopening, accessibility, idle cost and all four architectures.
- Authoritative variant/install identity, including verification of "Forever."
- Reliable addon/package identity and version-ordering coverage with real metadata.
- Safe identification of addon-only account/character saved state.
- Session completion, updater stability and game-launch-during-operation races.
- Recovery-point associations, rejection persistence and safe retention.
- Real Dropbox cloud-file states and filesystem differences on both OSes.
- Settings-master reassignment/retirement, concurrent policy decisions and
  distinguishing observed saved-state changes from intentional user pushes.
- Measured remaining performance budgets and representative datasets.
- Anonymous reporting schema/service and compliant data handling.
- Reproducible packages and truthful unsigned installation paths.

An infeasible approved requirement is a blocker for an owner decision, not a
reason for an agent to silently change frameworks, drop a platform, omit a test
or call the requirement complete.

## 8. Review status and unresolved implementation decisions

The interview defines product intent; it is not acceptance of an application
that has not been built. The complete specification bundle remains for owner
review. Approval is recorded against a particular revision, separately from
run budgets, prototype acceptance and public-release authorization.

The following are deliberately not silently fixed by this draft:

- Product/repository name and public distribution channel.
- Approved worker-model table, actual available runtime models and each run's
  numerical resource limits.
- Native test machines/accounts and current dated OS/architecture support matrix.
- Actual addon collection sizes, reference datasets and remaining measured
  performance budgets.
- Official identity/layout evidence for "Forever" and unknown-client enrollment.
- Addon identity/version coverage and safe treatment of unknown/shared saved
  files; no claim that arbitrary addon state is safely understood.
- Protocol, durable decision semantics, snapshot association/storage format and
  compatibility handling sufficient to implement the confirmed behavior.
- Screen-reader feasibility and any explicitly approved release limitation.
- Telemetry schema/provider, transport-data policy and legal/consent review.

These are assigned feasibility/design obligations with evidence and escalation,
not blanket authority to ask the owner to invent technical implementation details.
Choose ordinary implementation details within approved intent; seek approval
when the evidence requires a product, scope, permission or budget change.
