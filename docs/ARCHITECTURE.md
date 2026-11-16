# Architecture

> **Status: slice design (run-002), pending independent review.**
> History: written as a PROPOSAL in run-001 packet P1-map to bound the
> feasibility spikes; updated in run-002 packet R2-design from spike evidence
> into the design for the first vertical slice. Implementable detail is in
> [design/](design/README.md); this file is the overview and invariant map.
> Product meaning lives in [SPEC.md](../SPEC.md) and
> [ACCEPTANCE.md](../ACCEPTANCE.md); this file only says where that meaning is
> enforced. Choices outside approved intent are open questions in
> [design/questions.md](design/questions.md).

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
 adapters ───────────────────────────►  ┌──────────────────────┐ ◄──── GUI
          ◄─────────────────────────── │  wow-sync-core        │ ────► status, explanations,
             effects via port traits    │  policy + executor    │       audit records
                                        └──────────────────────┘
 wow-sync-runtime: implements the port traits with wow-sync-adapters,
 runs the sync cycle, exposes a service API to wow-sync-app (GPUI)
```

Crates ([design/crates.md](design/crates.md)): `wow-sync-core` (platform-free
policy, plans and the step executor over port traits, R2-core),
`wow-sync-adapters` (Windows filesystem/process, provider folder, local state;
no dependency on core, R2-adapters), `wow-sync-runtime` (glue and service,
R2-slice), `wow-sync-app` (GPUI shell, R2-slice). One process for the slice
(S-mem option 1, Q-19).

### 2.1 Domain core (platform-free)

Home: [crates/wow-sync-core/](../crates/wow-sync-core/).

- **Owns all policy.** In the slice: variant validation (R-02), enrollment
  rules (R-03), absence-is-not-deletion (R-04), `.toc` selection, version
  ordering and package grouping (R-05), deterministic group target and
  addon-scoped ambiguity (R-06), apply/publish plans and their safety gates
  (R-11), status/confirmation derivation (R-12). Later: settings authority,
  pushes, pauses, rejection, retention, retirement (R-07..R-10).
- **Executor.** `exec` sequences plans through port traits
  (`PublishEffects`, `ApplyEffects`), revalidates activity before every commit
  rename, writes journals first and recovers interrupted applies. It performs
  no I/O itself, so step ordering and crash recovery are tested with
  in-memory fakes and failure injection (A-22, A-43).
- **Inputs:** bytes and observations from adapters, decoded records, user
  decisions. Times arrive as data for display only, never as ordering proof
  (R-05, R-09).
- **Must not:** perform I/O, read clocks or RNG, spawn processes, depend on
  GPUI or a provider, or execute content. Enforced by
  `arch::tests::core_sources_use_no_os_io_or_clock` and `#![forbid(unsafe_code)]`.

### 2.2 Runtime (effect wiring)

`wow-sync-runtime` maps core port traits onto adapter calls one to one
([design/adapters-api.md](design/adapters-api.md) §9) and runs the sync cycle
([design/apply.md](design/apply.md) §1). It makes no policy choices: when an
adapter reports a changed precondition it returns the observation to core.
### 2.3 Adapters (responsibilities, unchanged from the proposal)

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

| Invariant | Source | Core (slice module) | Adapter / runtime | Evidence |
|---|---|---|---|---|
| Variant isolation; no cross-variant guessing; unfamiliar clients blocked until guided confirmation | R-02, R-03 | `variant` (`ConfirmedVariant` only from `confirm_variant` after `assess_layout`; unfamiliar clients need digest-bound identity + scope confirmation, suffixed-toc folders blocked for every client in the slice, Q-3/Q-20; slice enrollment only on installs with the synthetic marker, Q-24); records rejected when product/group differ; blobs/trees per group | `fs_scan::read_variant_files` reads metadata only | A-02, A-46 |
| Account/character isolation | R-02, R-07 | Out of slice (no settings) | Never reads/writes `WTF` | A-03 (later) |
| No execution of imported content | R-11, E-06 | Parses `.toc` headers and JSON as data only | Adapters copy bytes; no interpreter | A-23 |
| Path safety | R-11, R-15 | `paths` validation of every folder and tree path | `fs_safe::Root` containment; links/junctions/unknown reparse refused; cloud-file tags accepted only under the provider root; files and directories whose enumeration attributes carry a recall or offline flag are never opened or enumerated (Q-23) | A-23 |
| Published ≠ available ≠ applied ≠ confirmed; peer clocks never mean "current" | R-12 | `status`; confirmation only from a peer's own later apply report | Provider reports presence/cloud state only | A-20, A-24 |
| WoW running blocks game-file changes; idle launcher does not | R-11 | `plan` gate; `exec` activity gate before **every** live rename incl. rollback; on launch mid-commit no further game-file change until WoW exits (Q-4); `Unknown` blocks | `winproc` observes image paths under the variant folder; uninspectable possible clients → `Unknown` (Q-16) | A-21, A-30 |
| Absence is not deletion authority | R-03, R-04 | `target` and `plan` have no removal output in the slice | Replaced folders are renamed to recovery, never deleted | A-04, A-20 |
| A member's newest state is used or the member is unusable; no fallback to an older state; regressions detected; provider objects verified by hash, never by existence | R-03, R-12 | `admit` (watermarks), `exec::check_object`, tree hashes recomputed on read | `provider::check_object`/`put_object` (temp + hash + rename) | A-20 |
| Only compatible, complete candidates install: selected toc, same toc mapping and client build (Q-21), no partial packages (complete against group-wide membership, also on computers without the package, Q-2) | R-05, R-11 | `target` eligibility → `NeedsDecision` | — | A-07, A-09 |
| No timestamp/clock/machine-preference ordering | R-05, R-09 | `version`, `package`, `target` take no time inputs | — | A-08 |
| Ambiguity is addon-scoped | R-05, R-06 | `target` → `NeedsDecision` for that package only | — | A-07, A-08 |
| Failure leaves recoverable prior data and accurate state | R-11 | `exec` destination revalidation after staging (fingerprint + content), one operation (journal) per package, journal-first commit, recovery from observed slot configurations (re-entrant, [apply.md](design/apply.md) §4.1), per-op cleanup with the journal removed last, finalize only after verification, durable `needs_help`, any journal blocks new applies (Q-25), `recover_interrupted` | `fs_scan` complete-tree snapshots before/after hashing; `fs_install` same-volume renames, atomic journal writes | A-22 |
| Current + two previous per addon (code) | R-10 | `exec` prunes to 2 unpinned recovery points per package; pinned/journal-referenced points kept and shown as extra copies (A-17) | `fs_install` deletes only inside `.wowsync\recovery` | A-16 (partial) |
| Rejected release stays rejected | R-10 | Out of slice | — | A-18 (later) |
| Competing intentional settings never merged | R-09 | Out of slice | — | A-14 (later) |
| Retired computer cannot publish without re-enrollment | R-03 | Out of slice; `retired` field reserved | — | A-45 (later) |
## 4. Detailed design

See [design/README.md](design/README.md) for the document index, slice scope, out-of-scope list and traceability.

## 5. SPEC §7 questions and run-001 spikes (history)

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

## 6. Chosen for the slice, still unchosen

Chosen by this design for the slice (each may change after review; see
[design/questions.md](design/questions.md)):

- Shared-data layout and record schemas `wowsync.*/1`
  ([design/publication.md](design/publication.md)); compatibility rule: ignore
  unknown fields within a major, refuse unknown majors (Q-13).
- Recovery points and journals local to each install under
  `<variant>\.wowsync\` (Q-9, Q-18).
- Libraries: `serde`/`serde_json` (records), `sha2` (hashing), `getrandom`
  (ids), `windows-sys` (process/file attributes); GPUI pin per R2-pin.
- Single process (Q-19); polling sync cycle every ~3 s.

Still unchosen: settings data model and protocol (R-07..R-09), rejection and
group-wide rollback, retention/GC of shared blobs (Q-7), retirement and
re-enrollment flows, macOS adapters, notifications, telemetry (D-11),
packaging and startup integration, filesystem watching.