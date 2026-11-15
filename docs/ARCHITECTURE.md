# Architecture (PROPOSAL)

> **Status: PROPOSAL, not approved.** Written in run-001 packet P1-map to set
> boundaries for the Phase 2 feasibility spikes (SPEC §7). It prescribes no
> protocol, storage format or library (see §5).
> Spike evidence may change it. Product meaning lives in [SPEC.md](../SPEC.md)
> and [ACCEPTANCE.md](../ACCEPTANCE.md); this file only proposes where that
> meaning is enforced.

## 1. Drivers

- One testable domain policy, independent of UI/provider, and a single authority
  for reconciliation decisions; no duplicate engines (E-08).
- A representative domain change is testable without GUI, network or provider
  setup and without a second policy implementation (A-43; SPEC §5 "Code success").
- GUI, background operation and optional CLI do not become independent
  implementations of policy (SPEC §5; R-13).
- The same observed inputs and accepted policies produce the same result (R-06).

## 2. Shape

```text
          observations (typed facts)             user decisions / consent
 adapters ───────────────────────────►  ┌──────────────────────┐ ◄──── GUI / CLI
          ◄─────────────────────────── │  Domain core (pure)  │ ────► status, explanations,
             plans (intended operations)│  single policy       │       audit records
                                        │  authority           │
 shell: runs plans step by step via     └──────────────────────┘
 adapters, revalidates safety, reports outcomes back as observations
```

### 2.1 Domain core (platform-free)

Proposed home: [crates/wow-sync-core/](../crates/wow-sync-core/) (currently an
empty skeleton).

- **Owns all policy** for R-04 (membership, absence is not deletion), R-05
  (code ordering), R-06 (conflict scope, determinism, late/duplicate arrivals),
  R-07 (settings authority/master/exceptions), R-08 (pushes, pause, routine
  yield-to-master), R-09 (no settings merge), R-10 (retention, rejection,
  paired/code-only recovery). It also owns the policy parts of R-02 (isolation
  keys), R-03 (enrollment, retirement, re-enrollment) and R-12 (deriving
  published/available/applied/confirmed status).
- **Inputs:** typed observations from adapters, decoded group records, user
  decisions. Times arrive as data for display/observation limits (R-12), never
  as release-ordering proof (R-05, R-09).
- **Outputs:** plans (intended operations with their preconditions), decisions
  needing the user (scoped to an addon or inseparable package, R-06), status,
  and audit/explanation records (R-05, R-12).
- **Must not:** perform I/O, read clocks, spawn processes, depend on GPUI or a
  provider, or parse executable content. Deterministic for identical inputs.
- **Testability (A-43):** exercised with in-memory inputs, including multiple
  replicas with controlled delivery order (ACCEPTANCE "Deterministic offline
  layer", A-20). Real temporary filesystem tests (A-22) apply to the shell and
  filesystem adapter, not as a substitute for them.

### 2.2 Shell (effect runner)

Sequences the core's plans through adapters, feeds outcomes back as
observations, and owns crash-safe step ordering (R-11, A-22). It makes no
policy choices: when a precondition no longer holds it reports back to the core
rather than choosing an alternative. It revalidates quiescence and scope
immediately before each consequential mutation, not only before staging (R-11).
Whether the shell is a separate crate or process is unchosen.

### 2.3 Adapters

Every adapter treats file, provider and log contents as untrusted data (E-06)
and never executes Lua, saved-variable files, legacy CMD metadata or remote
payloads (R-11).

| Adapter | May | May not |
|---|---|---|
| **Filesystem** (WoW installs, AddOns, WTF, local app state) | Read installation metadata and addon/settings bytes as data; stage, validate and replace within explicitly authorized roots; report integrity/stability observations | Write outside authorized roots, follow unsafe links or traversal (R-11, A-23); copy the whole WTF tree or WoW's own UI/cloud-sync files (R-07); decide what is newer or deletable |
| **Dropbox local folder** | Read/write group data in the user-selected shared location; report presence, completeness, availability (online-only/partial/conflicted) as observations (R-03, A-05) | Use a Dropbox API or credentials (D-08); treat presence as application or confirmation (R-12); treat a missing object as deletion (R-04); request offline availability and report it as downloaded (R-03); act as a lock across computers (SPEC §6) |
| **Process / game activity** | Observe WoW and updater activity and file stability; report session start/end observations (R-07, R-11) | Kill, inject into or bypass protections of game/updater processes (R-11); decide on its own that an operation is safe |
| **Tray / GUI** (GPUI) | Render core status/explanations; collect consent, decisions, pushes, pauses, rollback and retirement requests; full supported recovery (R-13) | Hold policy or compute reconciliation results (SPEC §5); hide that background work continues when a window closes (R-13) |
| **Optional CLI** | Same role as GUI for advanced users (R-13) | Be required for any supported flow, or duplicate the engine (R-13) |
| **Notifications** | Deliver grouped batch summaries and separate actionable problems; defer nonurgent ones while gaming; suppress repeats of unchanged conditions (R-13, A-27) | Originate status not derived from the core; claim confirmation the core has not derived |
| **Telemetry** | Send only an approved anonymous schema after local validation/redaction; stop on opt-out (R-15, A-32) | Send names, private paths, settings, addon contents, credentials, stable identifiers, memory dumps or arbitrary error text; fail open; keep a send-later queue after opt-out (R-15). Provider and schema are deferred (D-11) |

## 3. Invariants and where enforced

| Invariant | Source | Core | Adapter / shell | Evidence |
|---|---|---|---|---|
| Variant and account/character isolation; no cross-variant guessing; unfamiliar clients blocked until validated | R-02, R-03 | Every item is scoped by group/variant/identity; plans crossing scopes are not producible; unvalidated enrollment yields no transfer plans | Filesystem adapter enforces per-scope authorized roots | A-02, A-03, A-46 |
| No execution of imported content | R-11, E-06 | Receives parsed data only | Adapters parse as data; no interpreter path | A-23 |
| Published ≠ available ≠ applied ≠ confirmed; offline/stale never current | R-12 | Distinct status states; confirmation only from a peer's record issued after its local application | Dropbox adapter reports presence only | A-06, A-24 |
| WoW running blocks game-file changes; idle updater does not by itself | R-11 | No game-file mutation plan while activity is present or cannot be established (SPEC §2 "Ready") | Shell revalidates just before each mutation; process adapter observes only | A-21, A-30 |
| Rejected release stays rejected for the group | R-10 | Rejection records are durable group state that version ordering cannot override | Shared-data format must carry them (unchosen) | A-18 |
| Absence is not deletion authority | R-03, R-04 | Only an intentional removal record removes | Adapters report absence as absence | A-04, A-10, A-20 |
| No timestamp/clock/machine-preference ordering | R-05, R-09 | Ordering requires demonstrable evidence; otherwise an addon-scoped decision | — | A-08 |
| Competing intentional settings never merged | R-09 | Produces an addon-level choice, preserving both | — | A-14 |
| Retired computer cannot publish without re-enrollment; no silent master election | R-03 | Retirement state filters its new changes; absent master surfaces a decision | — | A-45 |
| Current + two previous per addon, code and settings, with visible temporary exceptions | R-10 | Retention plans never prune data needed for a promised recovery or unresolved decision | Storage pressure reported, not resolved by destruction | A-16, A-17 |
| Failure leaves recoverable prior data and accurate state | R-11 | Never derives success for a partial collection | Shell step ordering and validation before replacement | A-22 |

Whether some invariants can also be checked statically (for example, the core
crate having no I/O dependencies) is a candidate architecture check under E-08
and A-43; no such check is defined yet.

## 4. SPEC §7 questions and run-001 spikes

Spike IDs from [PLAN.md](runs/run-001/PLAN.md) §3, Phase 2.

| SPEC §7 question | Spike(s) | Boundary affected | Notes |
|---|---|---|---|
| GPUI/tray lifecycle, reopening, accessibility, idle cost, four architectures | S-gpui, S-a11y, S-mem | Tray/GUI; process model | macOS runtime blocked until the Mac is available; ARM64/Intel build-only (D-07) |
| Variant/install identity, including "Forever" | S-forever | Filesystem (metadata), core scoping | Owner reviews output before commit (PLAN §7) |
| Addon/package identity and version-ordering coverage | S-addon | Core ordering (R-05) | Dev PC data under D-14 |
| Addon-only account/character saved state | S-state | Filesystem, core settings scope (R-07) | Synthetic fixtures only |
| Session completion, updater stability, launch races | S-proc | Process adapter, shell revalidation | Windows only in run-001 |
| Real Dropbox cloud-file states and filesystem differences on both OSes | S-dropbox | Dropbox adapter | Windows, `WoW Sync Test` only (D-08); macOS blocked |
| Measured remaining performance budgets and datasets | S-mem (memory only) | Process model | Other budgets not covered in run-001 |
| Recovery-point associations, rejection persistence, safe retention | **None in run-001** | Core, shared-data format | Gap; see evidence questions |
| Master reassignment/retirement, concurrent policy decisions, observed vs intentional pushes | **None in run-001** | Core | Gap |
| Anonymous reporting schema/service | **None** | Telemetry | Deferred (D-11) |
| Reproducible packages, unsigned install paths | **None** (P1-ci builds only) | Packaging | Gap |

## 5. Explicitly unchosen

Per SPEC §2 and §8, none of the following is chosen by this proposal:

- Shared-data protocol, durable decision semantics and compatibility policy (R-16).
- Recovery-point and local-state storage format, database or archive layout.
- Where recovery history physically lives (shared folder, local, or both).
- Process model: single process vs separate background/GUI processes, and IPC (R-14 counts all app-owned background processes).
- Crate split beyond the existing `wow-sync-core`, and crate names.
- Libraries other than the SPEC-mandated Rust and GPUI, including filesystem
  watching, serialization, hashing, tray integration, notifications and telemetry.
- Scheduling/trigger mechanism and startup integration.
- Telemetry provider and schema (D-11).
