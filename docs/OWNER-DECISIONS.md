# Owner decision register

Authoritative record of owner decisions required by ENGINEERING.md E-02 and
SPEC.md §8. A decision is binding only when recorded here with a date and
revision. Unanswered or deferred entries are not consent. Changes to a recorded
decision require a new dated entry; do not edit history.

Recorded by the Opus 5.5 orchestrator from owner (@hamiltonia) answers given
in a Copilot CLI session on 2026-11-14, against spec revision `7a2ccdf`.

## Decided

### D-01: Repository and product name

- `hamiltonia/wow-sync` is the permanent project repository; no separate
  repository will be created.
- Product name: **WoW Sync**.

### D-02: Distribution channel

- First public release: GitHub Releases.
- winget and Homebrew may be added later.
- Publishing still requires separate explicit authorization (E-09).

### D-03: Specification approval

- SPEC.md, ACCEPTANCE.md and ENGINEERING.md are approved as-is at `7a2ccdf`.
- This satisfies the E-09 "owner alignment" gate's spec half. The run-plan half
  is D-05/D-13.

### D-04: Worker role/model policy (version 1)

| Role | Model |
|---|---|
| Orchestrator (coordinates only, E-01) | claude-opus-5.5 |
| Implementation | claude-sonnet-5.5 |
| Independent verification | gpt-6.1-sol |
| Design/usability | claude-opus-5.5 |
| Integration/maintainability review | claude-opus-5.5 |
| Security/privacy review | claude-opus-5.5 |

- No alternates are approved. If a required model is unavailable, stop and ask.
- Actual runtime model per worker must be recorded, not inferred from a label.

### D-05: First run limits (bootstrap and feasibility)

- Scope: repository bootstrap plus SPEC §7 feasibility spikes. No product features.
- Maximum 5 concurrent workers, 20 total, no nested workers.
- Maximum 2 retries per task before escalating to the owner.
- No elapsed-time limit; AI credits are the only budget.
- AI-credit ceiling: **not yet set** (see D-13).

### D-07: Test machines

| Target | Machine | Status |
|---|---|---|
| Windows 11 x64 | Primary dev PC: Windows 11 Pro build 26300 (appears to be Insider), i7-13700HX, 64 GB RAM, NVMe SSDs, NVIDIA RTX 2000 Ada + Intel UHD | Available for development |
| macOS Apple Silicon | MacBook Pro M3, 48 GB RAM | Available later |
| Windows 11 ARM64 | None | Build in CI; untested until hardware exists |
| macOS Intel | None | Build in CI; untested until hardware exists |

- Targets are retained. Untested targets cannot be claimed as supported (A-35).
- The dev PC's Insider build is for development. Official support evidence comes
  from CI runners or stable-release machines.
- Workers may install rustup, the Rust toolchain and, if missing, MSVC build
  tools on the dev PC. Rust was not installed as of 2026-11-14.

### D-08: Dropbox access

- Use the existing Dropbox desktop client on the dev PC (`D:\Dropbox`).
  No Dropbox API credentials are needed or authorized.
- Agent scope is **only** `D:\Dropbox\Gaming Sync\WoW Sync Test` (create it if
  absent). Only synthetic data may be placed there.
- Every other entry in `D:\Dropbox\Gaming Sync` is out of scope: do not read,
  write, move or delete it. This includes `WoW` and `Sync`, which hold the
  owner's current legacy sync tool data (A-36, R-16), and other games' data.
- Multi-computer real-provider tests (A-34) wait for the Mac to join.

### D-09: WoW installation access

- Installation root: `D:\Games\World of Warcraft`. Only `_forever_` is installed
  on the dev PC; no `_retail_` or `_classic_` client is present there.
- Superseded in part by D-14 (2026-11-14): this installation is dev/test data.
- Original rule (still applies to any non-dev machine): read installation
  metadata only; never write; do not read `WTF`, account/character names or
  addon contents without a further decision.

### D-06: Run-state location (orchestrator default within approved scope)

- Run plans, work packets, handoffs and cumulative usage: `docs/runs/<run-id>/`.
- Evidence retention policy will be proposed during bootstrap.

## Resolved after initial recording

### D-13: AI-credit ceiling for the first run

- **Approved 2026-11-14: 2,500 AIU cumulative for run-001**, including the
  ~424 AIU spent before approval. Pause and ask at 2,000; stop at 2,500.
- Includes the orchestrator's own usage. Fresh orchestrator session per phase.
- Reporting: per worker and cumulative in `docs/runs/run-001/USAGE.md`.
- Superseded in part by D-16 (ceiling and session rule).

### D-16: Run-001 ceiling raised to 10,000 AIU (2026-11-15)

- Owner: "raise the ceiling to 10000, continue until you are complete or within
  5% of the limit, if you go over by 10% it is acceptable".
- Run-001 ceiling: **10,000 AIU cumulative**. Stop launching work at ≥9,500
  (within 5%); settling in-flight work up to 11,000 (+10%) is acceptable.
- The 2,000 pause no longer applies. The current orchestrator session
  (`961f3baa`) may continue through the remaining phases; the D-13 fresh-session
  rule is waived for this continuation.
- D-04 models, D-05 worker limits, D-08/D-09/D-14 data scope and PLAN §7 stops
  other than the credit pause (including owner review of S-forever) still apply.

### D-17: Continue past run-001 within the D-16 budget (2026-11-15)

- Owner: "I want you to continue until you run out of tokens, then we'll
  discuss how much work is remaining".
- Run-002 ([runs/run-002/PLAN.md](runs/run-002/PLAN.md)) proceeds under the
  remaining D-16 allowance (stop launching at ≥9,500 AIU cumulative across
  run-001 and run-002).
- Owner was unavailable for the prerequisite questions; the orchestrator's
  recommendations in run-002 PLAN §6 apply as **provisional, reversible
  defaults** pending owner review. Held (not done): committing the S-forever
  report, dismissing Dependabot alerts, `git gc --prune`.
- D-04 models, D-05 limits, data scope (D-08/D-09/D-14) still apply.

### D-18: Owner review of run-002 (2026-11-16)

Answers from an interactive review session with the orchestrator.

- **Incidents:**
  - V1: the owner noticed nothing closing unexpectedly. The memory gate may be
    re-measured, using a harness that stops only processes it launched.
  - The `git gc` incident is accepted with no further action.
- **Held items:**
  - Commit the S-forever report to `main`.
  - Run-003 triages and fixes the Dependabot alerts.
- **Confirmed run-002 PLAN §6 items:**
  - GPUI is pinned to a Zed release tag, currently `v1.28.1`. It is bumped
    deliberately, only for a needed fix or a security issue, and each bump
    is re-checked for lifecycle, memory and accessibility.
  - The `deny.toml` policy.
  - The memory measurement method.
  - R-11: minimize and report the launch race.
- **D-12 (screen reader), resolved:**
  - Narrator support is a nice-to-have, not a release blocker.
  - The app keeps exposing accessible controls.
  - Unverified screen-reader items are listed as known limitations. The owner
    explicitly approves this under R-13 and E-07.
- **Design questions (`docs/design/questions.md`), confirmed as written:**
  - Q-1, Q-2, Q-4, Q-5, Q-6, Q-8, Q-9, Q-10, Q-12, Q-16, Q-18, Q-22 and Q-25.
  - Q-22: same label with different content stays a decision.
- **Q-21, changed:**
  - A candidate from a computer on a different client build installs
    automatically when its `## Interface:` number is supported by the
    receiving client.
  - Otherwise it is a decision (`NeedsDecision`).
  - This is the compatibility evidence that R-05 requires. It is not a
    blanket allow.
- **Q-14:** non-ASCII names stay listed but not synced. Supporting them
  (normalization and case-fold design) is wanted later, not in run-003.
- **O-N1:** blocked folders appear in the details only and do not affect the
  headline status.
- **O-N2:** a build-version change does not force reconfirmation.
- **O-N3:** the consent predicate belongs in core.
- **Still open:**
  - Q-3 and Q-23 (real-PC checks).
  - Q-24 (real-data gate).
  - UI direction.
  - Run-003 scope and credit ceiling.

### D-28: Real installs without opt-in; run-006 approved (2026-11-22)

- Owner: "I don't think the data is so precious that we can't test against
  live? I can easily create separate backups, and we're not even working from
  the primary gaming system where the real data is." Then: "you can have
  another 10k AIU, I also prefer the addon to work without opt in, but having
  a 'test opt-out' feature is nice to have. People using this aren't going to
  lose anything on first run."
- **Q-24 answered: the synthetic marker gate is removed by default.** Real
  installs enroll through the normal guided identity and scope confirmation,
  with no extra opt-in. All other safety rules stay in force.
- **Test mode (nice-to-have).** The orchestrator reads "test opt-out" as a
  user setting that brings the marker gate back, so only marker-carrying test
  folders can be enrolled. The owner may correct this reading.
- **First run loses nothing.** This is now a verified requirement, not an
  assumption: anything the first sync replaces or removes on a computer must be
  recoverable through the GUI.
- **D-08 amended.** Real add-on and settings data from the dev PC may be
  placed in `D:\Dropbox\Gaming Sync\WoW Sync Test`. The rest of
  `D:\Dropbox\Gaming Sync` stays out of scope. Repository evidence still never
  contains add-on contents, settings, or account, realm or character names.
- **D-14 still applies to the dev PC `_forever_` install.** Workers snapshot
  it before tests and restore it after. Nobody starts, kills or injects into
  WoW.
- **Q-3 working answer.** The orchestrator took this from public
  documentation (warcraft.wiki.gg "TOC format", read 2026-11-22), not from an
  owner client test. Forever's game-type suffixes are `Forever` and `Camelot`
  (aliases), and its family suffix is `Mainline`. Selection order: game-type
  suffixes, then `Mainline`, then the plain `.toc`. Other suffixes
  (`Classic`, `Vanilla`, `TBC`, `Mists`, …) are ignored. The run-006 profile
  rule implements this. The owner's in-client check stays optional
  confirmation.
- **Q-22.** The current fail-safe behaviour is accepted for real data: the
  same version label with different content is a decision, never guessed.
- **Run-006: ceiling 10,000 AIU** from run-006 start, stopping new work at
  ≥9,500. [run-006 PLAN.md](runs/run-006/PLAN.md) is approved as committed
  with this decision. It includes publishing a `v0.2.0` GitHub pre-release
  after its gates pass.

### D-32: Visibility into what changed; faster idle passes; run-010 approved (2026-11-24)

- **Trigger:** live use of `v0.3.2` on RYZENFISH and LENOVOFISH.
  - RYZENFISH gained a new Hide Anything build (43 files) and about 1.8 GB of
    Spoken add-ons. The owner picked RYZENFISH's copy there.
  - LENOVOFISH then spent about 7.5 min (15:28–15:35 local) staging and
    installing those add-ons. All that time the tray still said "Waiting for
    your decision on 1 add-on", the log had no lines, and nothing showed what
    was arriving, from where, or how far along it was. The owner believed it
    was stuck.
  - The Activity page records only "hideanything applied": no source
    computer, no version change, no file or size change, no reason.
  - A pass where every file is a cache hit still takes 19–110 s and reports
    about 108k reused files for roughly 36k installed files, which suggests
    repeated scans per pass.
- **Owner direction:** "there needs to be a lot more visibility into what
  changed."
- **Approval:** the owner approved run-010 ("yes, continue"). There is no AIU
  ceiling (as in D-30 and D-31); usage is tracked.
- **Scope of run-010:**
  1. A durable, readable change history: for every add-on or settings change
     applied, removed, rolled back or published, show what changed, from
     which computer, the old and new version, file and size changes, and why.
  2. Live progress during a pass: phase, current add-on, counts and bytes,
     and an incoming-changes summary. No stale headline while work runs.
  3. Log lines for pass phases and each package applied, without profile
     paths or names.
  4. Idle passes scan each file at most once, and do not re-read files the
     app just installed. Target (approved as part of this plan): an
     unchanged pass on the real `_forever_` at least 3× faster than `v0.3.2`.
  5. Background memory stays within 100 MiB during a large apply.
  6. Publish `v0.3.3`.
- SPEC and ACCEPTANCE are unchanged. Plan:
  [run-010 PLAN.md](runs/run-010/PLAN.md).
### D-31: Live sync problems on two PCs; lighter re-checks; run-009 approved (2026-11-24)

- **Trigger:** live use of `v0.3.1` on RYZENFISH and LENOVOFISH.
  - For about 15 h, RYZENFISH said "Waiting for Dropbox to finish downloading".
    The cause was LENOVOFISH still running `v0.2.0`. It had published a
    QuestieDB tree reference whose tree file was never uploaded. The headline
    mislabelled this, and the owner had no way to see which add-on or
    computer was at fault.
  - Clicks (Decide, Pause, Export diagnostics → Preview) seemed to do nothing.
    The backend reads commands only between sync passes, and each pass
    re-hashed about 3.2 GB (~45 s).
  - Hide Anything (both copies labelled 1.10.0, different content) needed a
    decision. The owner's two clicks at 08:42 and 08:44 were both recorded,
    from the same computer, as two keeps that conflict and that do not know
    each other. The app then raises "choices differ", builds no options for
    it, and the page shows only "Decide later".
  - Neither the results of clicks nor what they did is logged.
- **Owner direction on file checks:** "it's a very low probability that
  addon's will be touched directly, I would make it optional or just remove
  it."
  - The orchestrator's reading: routine passes stop re-reading unchanged
    files. A file whose size and modification time are unchanged since it
    was last hashed keeps its recorded hash; a full re-read becomes an
    optional, owner-started check.
  - Content hashes are still computed for new or changed files, because
    blobs, trees and copy decisions depend on them.
  - Modification time is used only as a "did this file change" hint. It is
    never evidence that one copy is newer (SPEC is unchanged in that respect).
- **Approval:** the owner approved run-009 as proposed. There is no AIU
  ceiling (as in D-30), and usage is tracked.
- **Scope of run-009:**
  1. Choices that conflict offer a pick between the copies, and the pick
     covers both records.
  2. One computer does not record a second choice that conflicts with its own
     earlier one from stale evidence.
  3. Clicks respond at once and are not queued behind a full pass.
  4. A blocked add-on is named in the headline with an honest reason, and
     other add-ons keep syncing.
  5. Click results are logged.
  6. The hash cache, with an optional full check.
  7. Computers in a group running different app versions are shown.
  8. Publish `v0.3.2`.
- SPEC gains one sentence on change detection. ACCEPTANCE is unchanged. Plan:
  [run-009 PLAN.md](runs/run-009/PLAN.md).

### D-30: Setup hang on a second PC; run-008 approved, no credit ceiling (2026-11-23)

- **Trigger:** on RYZENFISH, `v0.3.0` found `G:\Games\World of Warcraft`
  through the launcher record. When the owner chose `_forever_`, setup stayed
  on "checking" indefinitely, and tray Quit did not exit; the owner had to end
  the process in Task Manager.
  - **Quit:** the log and code show that Quit waits for the backend thread,
    which was still inside the read-only inspection.
  - **Not the disk:** a read-only probe showed a fast NVMe drive (G:) with
    31k files / 3.2 GB and no cloud or reparse entries. Disk speed does not
    explain it.
- **Approval:** the orchestrator proposed run-008 (fix and publish `v0.3.1`).
  The owner replied "stop worrying about AIU, you have unlimited use now" and
  ran the probe. The orchestrator records this as approval of run-008 as
  proposed, with **no AIU ceiling**. Usage is still tracked and reported. The
  owner may correct this reading.
- **Scope of run-008:**
  - find and fix the root cause of the inspection hang;
  - Quit always exits promptly, whatever the backend is doing;
  - a long read-only check shows progress and can be cancelled;
  - a usable install is selected by default over a refused one;
  - the log records when each inspection starts, how long it took, and which
    tray item was clicked;
  - publish `v0.3.1` after its gates pass.
- No product-intent change; SPEC and ACCEPTANCE are unchanged. Plan:
  [run-008 PLAN.md](runs/run-008/PLAN.md).

### D-29: Find WoW on every drive; sync everything; run-007 approved (2026-11-23)

- Trigger: on a second PC, WoW at `G:\Games\World of Warcraft` was not found
  (v0.2.0 probes only drives C–F) and setup had no way to choose a folder. On
  the dev PC QuestieDB was blocked because its `.toc` files (17.6 MB each)
  exceed the 256 KiB inspection cap.
- Owner: "in general we need a way to sync everything, even if wrong". Then:
  "I use all drives (a-z), so you should assume that. It really makes more
  sense to ask the user for the folder if you can't find one, if scanning a-z
  is too slow".
- **Install discovery** must cover every local drive A–Z (and may also read
  Battle.net's own install record). When nothing is found, setup says so and
  lets the user choose the folder. A chosen folder still goes through guided
  identity and scope confirmation.
- **"Sync everything"** is accepted as direction.
- **Run-007 approved (2026-11-23).** The owner was offered a run-007 proposal
  with defaults and replied "approved, continue". The orchestrator records the
  offered defaults as approved:
  - scope: install discovery with a folder picker; header-only reading of
    oversized `.toc` files; syncing add-ons that cannot be recognised; a
    diagnostic log that is on by default and size-limited;
  - unrecognised add-ons: sync automatically when there is no disagreement or
    when lineage shows which copy is newer; otherwise ask once for that
    add-on (SPEC R-05 as amended);
  - ceiling **4,000 AIU** from the run-007 baseline in
    [run-007 USAGE.md](runs/run-007/USAGE.md), stopping new work at ≥3,700;
  - publish a `v0.3.0` pre-release after its gates pass.
  The owner may correct any of these.
- SPEC R-03 and R-05 and ACCEPTANCE A-47 and A-48 are amended with this
  decision. [run-007 PLAN.md](runs/run-007/PLAN.md) is approved as committed.

### D-27: Run-005 plan approved (2026-11-21)

- The owner said "draft and execute the plan, unless there's a real
  conflict that needs my attention, keep going. Use a swarm."
- The orchestrator drafted [run-005 PLAN.md](runs/run-005/PLAN.md) within
  the D-26 ceiling (10,000 AIU, stop at 9,500) and the D-26 answers. That
  instruction is recorded as approval of the plan as committed.
- Publishing the `v0.1.0` pre-release (D-26) happens only after the plan's
  artifact smoke test passes.

### D-26: Run-004 open decisions answered; run-005 ceiling (2026-11-21)

- **Q-88: option (a).** After a live keep, a demonstrably newer version
  that exists only on this computer stays installed and is not shared.
  The decision stays open, and the user keeps being asked until they
  answer. The kept tree is not reinstalled over it.
- **Q-77: 32 MiB.** `MAX_SETTINGS_FILE_BYTES` = 32 MiB (33,554,432 bytes).
  A larger file is a `TooLarge` diagnosis for that unit and is not synced.
  This lifts the D-25 block on settings sync with real data, but the Q-24
  synthetic-data marker gate and the D-14 data scope still apply.
- **macOS:** the owner will run [MAC-OWNER.md](runs/run-004/MAC-OWNER.md)
  on a Mac. Until those results are recorded, macOS stays NOT RUN.
- **First release: approved.** A `v0.1.0` GitHub pre-release with the
  Windows build, marked as a pre-release. No macOS binary until the macOS
  runs pass. Publishing is a public action. It needs a run-005 plan step
  that the owner has seen, and release notes that state the known gaps.
- **Run-005 ceiling: 10,000 AIU** from run-005 start, stopping at 9,500.
  The run-005 plan still needs owner approval before any worker launches.

### D-25: Run-004 design defaults accepted (2026-11-20)

- The owner replied "accept defaults" to the run-004 design questions.
  The conservative defaults recorded in
  [questions.md](design/questions.md) for Q-55, Q-57, Q-58, Q-60, Q-61,
  Q-63, Q-64, Q-77, Q-78, Q-81, Q-82, Q-84, Q-85, Q-86 and Q-87 are now
  owner decisions.
- **Q-77:** no settings-file size value is approved. Settings sync on real
  data stays disabled until the owner approves a value; tests use a
  test-only cap.
- **Q-63:** native macOS notifications are required; an in-app-only
  fallback is not accepted.
- **R4D-28:** the owner said "continue" at the design barrier after the
  design-fix retries were exhausted. The orchestrator took that as approval
  of one bounded correction plus one independent recheck, which then passed
  (`99e4877`).

### D-24: Run-004 plan approved (2026-11-17)

- The owner approved [run-004 PLAN.md](runs/run-004/PLAN.md) as written,
  with a ceiling of 20,000 AIU and a stop at 19,000.
- **P4-1: yes.** The update notice may make an anonymous HTTPS GET to
  GitHub Releases for this public repository. It sends no credentials and
  no identifiers beyond the HTTP request itself.
- **P4-2: yes.** The notification work may add a small, free, widely used
  OS notification crate. It must be justified under E-06 and pass
  `cargo deny`.

### D-23: Run-004 direction and pending answers (2026-11-17)

Answers from an interactive session with the orchestrator after run-003
closed.

- **Run-004 scope:**
  - removal (after the R3D-13 re-review);
  - settings sync (master, exceptions, peer push);
  - rollback;
  - resolving decisions in the GUI;
  - retiring a computer;
  - notifications and the update notice;
  - diagnostics export.

  The Windows installer is not in scope.
- **Ceiling:** 20,000 AIU for run-004. The owner first set 10,000, then
  added another 10,000 instead of dropping scope; no items are dropped. The
  run plan itself still needs owner approval before the swarm launches.
- **R3D-13:** simplify. Under D-21, any removal wins and conflicting
  removal records merge into one removal. The simplified design must be
  independently re-reviewed before it is implemented.
- **Real-PC checks (Q-3, Q-23):** later. Run-004 must not depend on them.
- **DA-1:** yes. All of the owner's computers use the same Dropbox account.
- **DA-2 to DA-4:** deferred. Version 1 uses the desktop folder only, with
  no Dropbox app registration and no API calls. Revisit after the Q-23
  check.
- **UQ-1:** yes. When a guided reconfirmation is pending, promote the
  dashboard headline to "Review needed".
- **UQ-2:** yes. Core exposes a typed scope for a pending decision.
- **Tray badge:** keep the status badge on the brand tray icon.
- **A-31:** numeric budgets deferred until closer to release.
- **Mac:** the owner has a Mac available for real runtime testing (macOS
  runtime and A-29 memory on macOS).

### D-22: App icon art (2026-11-17)

- The owner provided app art: a PNG and an ICO with sizes from 16 to 256 px.
  It contains Blizzard's WoW "W" crest and the Dropbox logo.
- The orchestrator raised the concern that this repository is public and the
  art includes third-party trademarks. The owner answered: "pretty sure the
  imagery is fair use".
- **Decision:** commit the art as is under `assets/brand/` and use it as the
  app icon for the window, tray and `.exe`. The owner accepts the trademark
  risk.
- The orchestrator's recorded caution:
  - fair use is a copyright defence;
  - brand rules on using logos as an app icon are a separate trademark
    question;
  - revisit this before any public release or store listing.
- This supersedes the OQ-ICON question in the run-003 HANDOFF.

### D-21: Prototype review and design answers (2026-11-17)

Answers from an interactive session with the orchestrator at the run-003
phase D barrier.

- **Prototype (A-42): approved** as the basis for the real UI.
  - Phase E and phase F may proceed within the D-19 budget.
- **Product principle:** WoW Sync is a sync tool, not addon police.
  - Users are responsible for their own addon setup.
  - The app does not protect users from their own choices beyond keeping
    recovery points (R-10).
- **Q-21: replaces the D-18 Q-21 rule and the interim rule A.**
  - Never withhold an addon because of a game update or its `## Interface:`
    declaration.
  - Only the variant boundary and the matching account/character boundary
    limit sync.
  - SPEC R-05 is amended to match.
- **Removal:** a deliberate removal syncs everywhere.
  - **Q-41 kept:** a short settle wait (about 30 s, two stable scans) before a
    missing folder counts as removed. This is correct observation, so a
    half-extracted updater install is not published as a removal.
  - **Q-42 rejected:** no confirmation when many addons vanish at once.
  - **R-06 amended:** if a removal collides with a concurrent offline update,
    the removal wins. The updated copy is kept in recovery and named in the
    summary. ACCEPTANCE A-10 is amended to match.
  - The other removal defaults (Q-43, Q-44, Q-46 to Q-53) are accepted as far
    as they are consistent with "removed means removed everywhere".
    - Q-48: a joining computer removes the addon; there is no
      removal-versus-update decision.
    - Q-52 is unchanged, because post-retirement actions are not authority.
- **Q-45:** the owner reports that WoW keeps an uninstalled addon's saved
  variables.
- **UI defaults:**
  - **Q-26:** close hides to the tray by default. App settings has an option
    to make close quit.
  - **Q-27:** 72 hours.
  - **Q-28:** pause lasts until resumed.
  - **Q-29:** "Quit when safe" sheet.
  - **Q-34:** Follow system / Light / Dark.
  - **Q-36:** update check on by default, with the disclosure and an opt-out.
- **Dependabot:**
  - Delete the obsolete `spikes/gpui`, `spikes/mem` and `spikes/a11y` (their
    results remain in the docs).
  - Dismiss the Linux-only glib alerts, since Linux is not a supported
    platform.
- **New spike:** evaluate the Dropbox public API as an alternative or
  complement to the desktop-client folder.
  - It is research only. D-08 still applies: no Dropbox API credentials.
  - A live API trial needs a separate approval.

### D-20: Run-003 plan approved (2026-11-16)

- The owner approved `docs/runs/run-003/PLAN.md` as written.
- Ceiling per D-19. Baseline at approval: 9,513.4 AIU cumulative (sessions
  `961f3baa`, `fea20f53`).
  - Stop launching new work at ≥19,013.
  - Settling up to ≈20,513 is acceptable.
- Phase F requires a recorded owner approval of the prototype.

### D-19: Run-003 direction and ceiling (2026-11-16)

- **Credit ceiling:** 10,000 AIU for run-003, counted from run-003 start and
  separate from the D-16 allowance.
  - Stop launching new work at 95% of the ceiling.
  - Settling up to 110% is acceptable, as under D-16.
- **Scope:**
  - UI design plus a clickable prototype that needs owner approval (A-42).
  - Memory gate re-measured with a safe harness.
  - Windows `native` test tier enforced, and the flaky CI test fixed.
  - Dependabot triage and fix.
  - Q-21 interface-number compatibility rule.
  - Addon uninstall/removal design.
- **UI direction:**
  - A roomier dashboard rather than a compact tray popup.
  - Follows the system light/dark theme.
  - Owner reference: Syncthing.
- **Owner help:** the owner will do the real-PC checks for Q-3 (`.toc`
  selection) and Q-23 (an online-only Dropbox file) once given instructions.
- **Still required:** the run-003 plan itself needs owner approval before any
  worker launches.

### D-10: Representative addon dataset

- Resolved by D-14: the dev PC's `_forever_` addons (102 folders, ~3.3 GB as of
  2026-11-14) are the representative dataset for this phase. Testing with the
  owner's real gaming data comes later and needs a new decision.

### D-14: Dev PC WoW data is test data (2026-11-14)

- Owner: the dev PC is not a primary gaming system; its addons are there for
  agents to work with, and `WTF` is in a basic state ready for testing.
- Workers may read and modify `D:\Games\World of Warcraft\_forever_\Interface\AddOns`
  and `...\_forever_\WTF` for development and tests.
- Still required: never execute addon Lua, saved-variable files or legacy
  scripts (AGENTS.md); do not run, kill or inject into WoW or launchers;
  snapshot before destructive tests so the dev install can be restored.
- Do not commit addon contents or saved settings to the repository. Third-party
  addon code is not ours to redistribute (E-06). Committed fixtures must be
  synthetic or minimal derived metadata (names, versions, interface numbers).
- This does not extend to the owner's other machines or real gaming data.

### D-15: Run-001 plan approved (2026-11-14)

- `docs/runs/run-001/PLAN.md` approved as committed in `baa81e3`.
- Phase 0 may start; Phase 1 still waits for the D-13 credit ceiling.
- S-forever and S-addon may use the dev PC data under D-14.

## Deferred

- D-11: Telemetry provider and schema (R-15). Propose after feasibility.
- D-12: resolved by D-18 (Narrator is a nice-to-have; limitations are listed).

## Decision history

| Date | IDs | Summary | Revision |
|---|---|---|---|
| 2026-11-14 | D-01–D-09 | Initial owner answers recorded; D-10, D-13 open | `7a2ccdf` |
| 2026-11-14 | D-10, D-14, D-15 | Dev PC WoW data is test data; run-001 plan approved | `baa81e3` |
| 2026-11-14 | D-13 | Run-001 ceiling 2,500 AIU cumulative approved | `a4d471b` |
| 2026-11-15 | D-16 | Run-001 ceiling raised to 10,000 AIU; continue in current session | `f2a1323` |
| 2026-11-15 | D-17 | Continue into run-002 within D-16 budget; provisional defaults | `961a133` |
| 2026-11-16 | D-18 | Run-002 owner review: incidents, PLAN §6, D-12, design questions, Q-21 interface rule | `d83fda7` |
| 2026-11-16 | D-19 | Run-003 scope, UI direction, 10,000 AIU ceiling; plan approval pending | `3d332fd` |
| 2026-11-16 | D-20 | Run-003 plan approved as written | `b420717` |
| 2026-11-17 | D-21 | Prototype approved; Q-21 no interface gating; removal wins; UI defaults; Dropbox API spike | `f92a61a` |
| 2026-11-17 | D-20 | Restored the D-20 heading that `f92a61a` accidentally deleted (R3-review RV-10) | `ee244ef` |
| 2026-11-17 | D-22 | App icon art committed and used as is; owner accepts trademark risk | `8601907` |
| 2026-11-17 | D-23 | Run-004 scope and 20,000 AIU ceiling; R3D-13 simplify; DA-1 yes, DA-2 to DA-4 deferred; UQ-1/UQ-2 yes; A-31 deferred; Mac available | `1dbd81e` |
| 2026-11-17 | D-24 | Run-004 plan approved; P4-1 GitHub Releases check yes; P4-2 notification crate yes | `573615b` |
| 2026-11-20 | D-25 | Run-004 design defaults accepted for Q-55..Q-87 pending set; Q-77 no value (real settings sync disabled); R4D-28 bounded fix | `a951c2f` |
| 2026-11-21 | D-26 | Q-88 (a); Q-77 32 MiB; owner runs macOS kit; v0.1.0 Windows pre-release approved; run-005 ceiling 10,000 AIU, plan pending | `9ca8f10` |
| 2026-11-21 | D-27 | Run-005 plan drafted and approved by owner instruction ("draft and execute") | `fe13e3d` |
| 2026-11-22 | D-28 | Q-24 marker gate removed by default (test mode nice-to-have); D-08 amended; Q-3 working answer; run-006 10,000 AIU, plan approved | `c77d050` |
| 2026-11-23 | D-29 | Discovery on drives A–Z with a folder picker; sync unrecognised add-ons; SPEC R-03/R-05, A-47/A-48; run-007 4,000 AIU, plan approved | `8b0dee0` |
| 2026-11-23 | D-30 | Setup inspection hang and stuck Quit on a second PC; run-008 approved with no AIU ceiling; `v0.3.1` | `ff938da` |
| 2026-11-24 | D-31 | Live sync fixes (conflicting choices, click lag, headline, logging), hash cache with optional full check; run-009 approved with no AIU ceiling; `v0.3.2` | this commit |
