# WoW Sync: intent specification seed

Status: first complete draft for product-owner review, 2026-11-13.

This bundle specifies a new project. It is not the application, an approved
implementation plan, or permission to launch a swarm. The product owner intends
to create a separate GitHub repository from it.

## Read in this order

1. [SPEC.md](SPEC.md): product intent, constraints, user scenarios and success.
2. [ACCEPTANCE.md](ACCEPTANCE.md): observable acceptance obligations and evidence.
3. [ENGINEERING.md](ENGINEERING.md): agentic delivery, quality and repository contract.
4. [AGENTS.md](AGENTS.md): concise entry point for agents in the future repository.

The `.github` directory contains a short Copilot instruction entry point that
points back to these documents instead of duplicating requirements.

## Authority and approval

- The owner supplied twelve interview rounds behind this draft.
- Owner-confirmed product decisions are incorporated in SPEC.md.
- Acceptance procedures and engineering controls operationalize those decisions
  and remain subject to review of this complete bundle.
- Technical choices are deliberately not prescribed where intent is sufficient.
- Unresolved feasibility questions are explicit gates, not permission to guess.
- The owner must approve the specification and a run/budget proposal before
  implementation begins.
- Passing tests does not authorize public publishing.

If documents conflict, stop and request a correction. Do not choose whichever
interpretation is easiest to implement.

## Product in one paragraph

A public, MIT-licensed Rust application with a GPUI interface remains available
in the tray on Windows and macOS. It uses the user's Dropbox desktop client to
share installed WoW addons across computers, automatically selecting demonstrably
newer compatible addon releases. Saved addon settings follow a selected master,
with per-addon exceptions and deliberate pushes from other computers. Users see
what actually applied on each computer, can resolve item-level ambiguity, and
can recover code and associated settings without a terminal.

## Important decisions that must not be lost

- Windows x64/ARM64 and macOS Apple Silicon/Intel are first-release targets.
- WoW variants are isolated, including explicitly enrolled test/PTR/beta clients.
- "Forever" is the owner's name for a separate, newer installation, not Classic
  Era. Verify its actual metadata; do not silently relabel it.
- CurseForge or another updater obtains releases. This app synchronizes installed
  copies rather than becoming an addon marketplace/updater.
- Code and settings have different reconciliation rules.
- Settings are not permanently manual-only: selecting a master enables automatic
  sharing for all addon settings, including newly discovered addons.
- Deliberate competing settings choices are never merged.
- Additions and deliberate removals propagate, including removal of active saved
  settings; older recovery points preserve the removed code and settings.
- Three history states means current plus two previous, per addon, independently
  for code and settings, with explicit temporary safety exceptions.
- A rejected bad addon release must not return from an offline computer.
- Retiring a computer is explicit, preserves history, and requires re-enrollment
  before new changes from that identity are accepted.
- An unfamiliar client can be enrolled after guided identity/safe-scope validation;
  its name alone is not permission to transfer or cross variant boundaries.
- Idle background private resident memory must be at most 100 MiB in aggregate;
  50 MiB is a stretch goal. GPU allocations are reported separately.
- Safe synchronization starts within ten seconds of complete local readiness.
- Other numeric performance gates require measurement and an approved proposal.
- Anonymous crash/performance reporting is default-on with a clear opt-out,
  constrained by the privacy contract.
- Real Dropbox validation on Windows and macOS is required before public release.
- Initial public builds may be unsigned with honest installation warnings.
- App upgrades are initially notification plus guided manual installation.
- The owner approves an early UI prototype and the release candidate.
- The Opus 5.5 orchestrator coordinates; workers implement. Propose a budget first.

## How to use this seed

Copy the contents into the new repository root. Read and approve the product
specification first. An orchestrator should then propose a bounded initial run
covering repository bootstrap and feasibility, with an explicit worker/model
policy and budget. Do not start by implementing every screen or choosing a
snapshot format.

The first technical work should establish whether the framework, tray lifecycle,
four architectures, memory target, addon identity/version evidence, saved-state
scope and provider behavior can satisfy the intent. Problems discovered there
must become owner decisions before a large implementation investment.

## Not included or authorized

- No application source or Cargo scaffold.
- No launched swarm or approved worker/model matrix.
- No numerical AI-credit, concurrency or elapsed-time authorization.
- No access to real WoW installations, Dropbox data, accounts or credentials.
- No cloud service, telemetry vendor, paid subscription or public publishing.
- No automatic migration or deletion of old CMD/PowerShell data or jobs.

These are intentional boundaries of a specification handoff, not missing
features to silently fill in.
