# Evidence S-a11y-design: screen-reader options for D-12

Packet: [S-a11y](../packets/S-a11y.md) ("S-a11y-design work" section) plus
[S-common](../packets/S-common.md). Worker S-a11y-design (design role), branch
`run001/S-a11y-design`. Authority: [SPEC.md](../../../../SPEC.md) R-13 and §7,
[ACCEPTANCE.md](../../../../ACCEPTANCE.md) A-26/A-42,
[ENGINEERING.md](../../../../ENGINEERING.md) E-07/E-08, deferred decision D-12 in
[OWNER-DECISIONS.md](../../../OWNER-DECISIONS.md). No code was written. This is a
design document: it proposes options; it does not decide D-12, change scope,
switch frameworks or edit `deny.toml`.

Inputs read: [S-a11y evidence](S-a11y.md) and its raw dumps,
[S-gpui evidence](S-gpui.md), `spikes/a11y/Cargo.lock` and
`spikes/a11y-main/Cargo.lock`, `deny.toml`. **S-mem evidence is not present** on
`main` or on branch `run001/S-mem` at the time of writing (that branch has no
commits beyond `main`); per the orchestrator it used `gpui 0.2.2` like S-gpui.
Statements about S-mem below rest only on that fact.

## Question and answer

SPEC §7 / E-07 question: can the GPUI GUI meet screen-reader needs, and if not,
what release limitation should the owner approve (D-12)?

- **Published GPUI (crates.io 0.2.2): no.** Nothing inside the window reaches
  UI Automation (S-a11y, measured FAIL, Windows 11 x64). No crates.io release
  with accessibility exists (verified below).
- **GPUI from Zed source with AccessKit: feasible on Windows at the UIA level**
  (S-a11y measured PASS for roles, names, focus, Invoke/Toggle/Value), with open
  upstream bugs and no speech evidence yet. Narrator speech: **NOT RUN** (owner
  check). macOS VoiceOver: **BLOCKED** (no Mac).
- **Recommendation (Option B below):** pin GPUI to an immutable Zed *release
  tag* that contains AccessKit, annotate controls from the first prototype, and
  set D-12 to "Windows screen-reader support for core flows is a verified goal;
  anything not verified at release is disclosed; macOS VoiceOver is disclosed
  as untested until a Mac is available". Re-run the S-gpui (and S-mem) checks on
  that pin before relying on their numbers. Confidence: medium. What would
  change it: the owner's Narrator check failing on `spikes/a11y-main`, the pin
  failing tray/memory/ARM64 re-checks, or a crates.io GPUI release with
  AccessKit appearing (would remove the git-pin cost).

## Facts this design relies on

| Fact | Source / status |
|---|---|
| crates.io `gpui` newest = max = default version `0.2.2`, created 2025-10-22; 7 versions total; license Apache-2.0 | <https://crates.io/api/v1/crates/gpui>, fetched 2026-11-15 (PASS, public API) |
| 0.2.2 exposes one empty `Window` to UIA; Tab focus invisible to UIA | [S-a11y](S-a11y.md), [raw dump](S-a11y/uia-gpui.txt) (FAIL, Win 11 x64) |
| Zed `main` `24d3772…` exposes 16 annotated elements with patterns and UIA focus tracking | [S-a11y](S-a11y.md), [raw dump](S-a11y/uia-gpui-main.txt) (PASS, Win 11 x64) |
| AccessKit landed in GPUI by PR 56065, merged 2026-05-27, GPUI only; "first step" toward issue 41138 | <https://github.com/zed-industries/zed/pull/56065> |
| Issue 41138 "Windows: Screen reader accessibility missing completely" still open | <https://github.com/zed-industries/zed/issues/41138> (fetched 2026-11-15) |
| PR 57954 adds `Application::inaccessible()` to disable AccessKit adapters; motivated by panics in nightly because "AccessKit panics when an invalid `TreeUpdate` is provided" | <https://github.com/zed-industries/zed/pull/57954> |
| Open PR 66631: on Windows "Narrator can stay silent in a GPUI window that opened behind another window and was then clicked", UIA reports no focused element | <https://github.com/zed-industries/zed/pull/66631> (open, 2026-11-06) |
| Open issue 66690: views drawn from the cache drop accessibility nodes, focus falls back to the window | <https://github.com/zed-industries/zed/issues/66690> (open) |
| Other open items (66730 clipped nodes, 65623 duplicate node id on list autoscroll, no `disabled`/`read_only` builders) | Cited in [S-a11y](S-a11y.md); not re-fetched here |
| Latest Zed release `v1.28.1` (commit `e8bc16a`, 2026-11-11) is marked "Immutable release" | <https://github.com/zed-industries/zed/releases/tag/v1.28.1> |
| At tag `v1.28.1`, `crates/gpui/Cargo.toml` depends on `accesskit`, `crates/gpui_windows/Cargo.toml` on `accesskit` and `accesskit_windows`; gpui still declares `version = "0.2.2"` | raw files at <https://github.com/zed-industries/zed/tree/v1.28.1/crates>, fetched 2026-11-15 (read-only; not built) |
| Zed contributions require signing a CLA | <https://github.com/zed-industries/zed/blob/main/CONTRIBUTING.md>, <https://zed.dev/cla> |
| Cargo locks git dependencies to a commit in `Cargo.lock` and only moves on `cargo update`; crates.io packages cannot depend on git code | <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html> |
| cargo-deny `[sources]` supports `allow-git` (exact URLs) and `required-git-spec = "rev"` | <https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html> |
| AccessKit: cross-platform accessibility infrastructure for UI toolkits, with platform adapters | <https://github.com/AccessKit/accesskit>; lockfile resolves `accesskit_windows 0.34.0` and `accesskit_macos 0.26.3` |

Lockfile comparison (this worktree, read-only analysis of committed files):

| | `spikes/a11y` (crates.io 0.2.2) | `spikes/a11y-main` (git rev) |
|---|---|---|
| Packages | 709 | 585 |
| Git-sourced packages | 0 | 37, from 6 repositories: `zed-industries/zed` (19 crates), `gfx-rs/wgpu` (13), `zed-industries/oo7`, `zed-industries/wasm_thread`, `zed-industries/font-kit`, `zed-industries/scap` |
| Accessibility crates | none | `accesskit 0.24.1`, `accesskit_windows 0.34.0`, `accesskit_macos 0.26.3` |
| Notable differences | `ring 0.17.14`, `zed-reqwest`, `blade-graphics 0.7.1` | no `ring`, no `reqwest`; `wgpu 30.0.0` from git |
| Git cache on this PC | n/a | `~\.cargo\git\db\zed-*` = 413 MB (full Zed history clone) |

The absence of `ring` matters: S-gpui's Windows ARM64 build was BLOCKED by
`ring`'s build script needing clang. A newer pin *may* remove that blocker; this
is a lockfile observation only, **NOT RUN** as an ARM64 build.

## What "git pin vs crates.io release" means

| Property | crates.io `gpui = "=0.2.2"` (S-gpui, S-mem, `spikes/a11y`) | git `rev`/`tag` pin (`spikes/a11y-main`) |
|---|---|---|
| Immutability | Published versions cannot be changed (only yanked); `Cargo.lock` stores a SHA-256 checksum | Exact commit SHA recorded in `Cargo.lock`; content is fixed by the git hash, but there is no registry checksum and the commit must stay reachable upstream (force-push/repo removal would break fresh builds) |
| Clean-checkout build | Registry download, small | Clones the whole Zed repository (413 MB here) plus five other repos; needs GitHub reachable or a vendored copy (`cargo vendor` would mitigate; not tried) |
| Version identity | `0.2.2` identifies the code | Every Zed commit still says `gpui 0.2.2`; only the SHA/tag identifies it. Reports and evidence must record the SHA |
| Supply-chain policy | Passes current `deny.toml` (`allow-git = []`, `unknown-git = "deny"`) | Fails current `deny.toml`; needs six exact `allow-git` entries and, recommended, `required-git-spec = "rev"` (orchestrator-owned file; approval needed) |
| Upstream API stability | Frozen but stale (over a year without a release) | Moves daily; Zed treats GPUI as its internal framework. Upgrades are manual and may break the app (e.g. the `gpui_platform` crate split needed by `spikes/a11y-main`) |
| Accessibility | none, no path forward | AccessKit, with open bugs; fixes arrive only by moving the pin |
| Choosing the commit | n/a | A Zed *release tag* (e.g. `v1.28.1`) has had a stable-channel release on top of it and is marked immutable on GitHub; an arbitrary `main` commit (S-a11y used `24d3772…`) has no such signal. Prefer `rev = "<SHA of tag>"` so cargo-deny `required-git-spec = "rev"` holds |
| Our own publishing | Unaffected | We cannot publish our crates to crates.io with a git dependency (not planned; the product ships binaries) |

**Effect on other spikes.** S-gpui (tray, hide/show via raw HWND, single
instance, iGPU, build size/time, ARM64 blocker) and S-mem (memory) measured
`0.2.2`. A move to a git pin changes the dependency tree (585 vs 709 packages;
`wgpu` replaces `blade`; AccessKit adds a per-window adapter) and the public API
(`gpui_platform::application()`). Their results are **evidence for 0.2.2 only**
and must be re-run on the chosen pin before the memory gate (A-37 100 MiB) or
tray lifecycle are claimed for the product. Code-level changes are expected to be
small (S-a11y ported the same UI) but are unmeasured for the tray spike.
Mixing pins (GUI on git, something else on crates.io GPUI) is not possible in one
binary: both resolve to a `gpui` crate and Cargo would compile two copies.

## Options

Costs are relative to an implementation that must already deliver keyboard
operation, visible focus, scaling and themes (R-13, A-26), which apply in every
option and are **not** what D-12 waives. "Worker packets" are rough planning
units, not measured.

### Option A: stay on crates.io 0.2.2, approve a full limitation

D-12 text: "The first release does not support screen readers on any OS;
disclosed in README and release notes."

- Cost: none beyond disclosure. Keeps the most reproducible dependency setup and
  the S-gpui/S-mem results valid.
- Risk: the app is completely unusable with Narrator/VoiceOver (window title
  only). No upgrade path without a later pin move and retrofitting every
  control. Tab/Shift+Tab and focus drawing must still be built (S-a11y).
- Release gate: disclosure text present; keyboard/focus A-26 still required.

### Option B (recommended): pin a Zed release tag with AccessKit, annotate from day one

D-12 text (proposed): "Windows: screen-reader use of the core flows (setup and
enrollment, status, pause/resume, settings exception/push, conflict selection,
rollback/recovery, quit) is a release goal verified with Narrator (and NVDA if
available). Any flow or state not verified at release is listed as a known
limitation. macOS: VoiceOver untested until a Mac test environment exists;
disclosed as a limitation."

- Work: pin `gpui`/`gpui_platform` at the SHA of a Zed release tag; update
  `deny.toml` (approval); re-run S-gpui lifecycle and S-mem memory on the pin;
  give every control role/name/state and every changing status message an
  accessible representation; accessible text input (S-a11y's field was a bare
  `div`); bind Tab/Shift+Tab; set-value/selection actions where needed; owner
  Narrator checks at prototype and release candidate (E-07). Rough size: one
  pin/re-check packet, ~10–20% extra effort on each GUI packet, one owner
  speech check per milestone.
- Risks and mitigations:
  - Upstream bugs directly relevant to this app: a tray app re-showing its
    window without taking the foreground is the PR 66631 case (Narrator
    silent); cached views (66690) and list autoscroll (65623) fit a
    status-heavy UI. Mitigate: add these scenarios to the A-26 interaction
    checklist; move the pin when fixes merge; keep local workarounds in our code
    (e.g. explicit activation on reopen) rather than a fork.
  - AccessKit panics on invalid tree updates (PR 57954). A GUI panic must not
    stop sync: keep the domain engine independent of the GUI (already required
    by E-08) and keep `Application::inaccessible()` as an owner-visible
    fallback switch if a pin proves unstable.
  - Pin maintenance: each pin move re-runs offline/native tiers and the
    accessibility checklist; record SHA in evidence.
  - Unreleased API churn: annotation builders may be renamed; contained to the
    GUI crate.
- Release gate: Narrator script per core flow PASS on Windows x64 with recorded
  Narrator/Windows versions; failures become listed limitations only with owner
  approval (A-42), never silently.

### Option C: Option B plus upstream contributions

Fix 66690/66631-class bugs or missing builders (`disabled`, `read_only`) in Zed.

- Cost: medium-high and open-ended; review timing is outside our control.
- Requires the owner (or an approved identity) to sign Zed's CLA (a legal
  action agents must not take) and public contribution activity: both need
  explicit owner approval (AGENTS.md "public publishing", "new credential
  access"). Carrying patches in our own fork via `[patch]` adds a seventh git
  source and a maintained fork.
- Recommendation: not now. Revisit only if a blocking bug has no app-side
  workaround and no upstream fix by the prototype milestone.

### Option D: parallel native accessible shell for key flows

A second UI (Win32/WinUI on Windows, AppKit on macOS) for setup, status,
conflicts and recovery, sharing the domain engine.

- Benefit: mature platform accessibility independent of GPUI.
- Cost: high. Two extra UIs covering every supported flow, each needing A-26
  evidence, themes and scaling; permanent divergence risk between UIs.
- Scope: SPEC §2 states a GPUI interface; a second permanent GUI is a
  framework/scope change requiring explicit owner approval (SPEC §7 last
  paragraph). Not recommended for the first release.
- A narrower, cheaper relative already in the design: OS-rendered surfaces
  (native tray menu from `tray-icon`, OS notifications) are not GPUI and are
  likely reachable by screen readers. **NOT RUN**: not tested by S-a11y or here;
  must not be claimed until checked.

### Not proposed

Switching GUI framework (other Rust toolkits with AccessKit) changes approved
product meaning and is listed only so its absence is visible; it would need an
owner decision and new feasibility spikes for tray, memory and four targets.

## Comparison

| | A: limitation on 0.2.2 | B: release-tag pin + annotate | C: B + upstream | D: native shell |
|---|---|---|---|---|
| Screen-reader outcome (Windows) | none | core flows, verified, gaps disclosed | as B, fewer gaps over time | strongest for covered flows |
| macOS VoiceOver | none | unknown (BLOCKED) | unknown | needs AppKit UI and a Mac |
| Engineering cost | lowest | medium | medium-high | high |
| Reproducibility | best (registry, checksums) | good if SHA-pinned and allow-listed; 413 MB git fetch | as B plus fork upkeep | adds platform UI deps |
| Invalidates S-gpui/S-mem results | no | yes, re-run needed | yes | no (but new UIs untested) |
| Owner approvals beyond D-12 | none | `deny.toml` git sources | CLA, public contributions | scope/framework change |
| Retrofit risk if changed later | high (every control) | low | low | n/a |

## Recommendation for the owner

1. Before deciding, run the 10-step Narrator check in [S-a11y](S-a11y.md) on
   `spikes/a11y-main` (about 10 minutes, owner only; agents cannot hear). If
   Narrator speaks roles/names/states there, Option B is supported by speech
   evidence; if it is silent, prefer Option A for this release.
2. Approve Option B with the proposed D-12 text, pinning `rev` = SHA of an
   immutable Zed release tag containing AccessKit (currently `v1.28.1`,
   `e8bc16a…`; verified to contain AccessKit by source, not built).
3. Make the pin a Phase 3 prerequisite: one packet re-runs S-gpui lifecycle,
   S-mem memory and a Windows ARM64 build attempt on the pinned SHA and
   proposes the `deny.toml` allow-list; the GUI work starts only after that.
4. Keep Option D and C as escalation paths at the prototype review (E-07), not
   as planned work.

Owner decisions requested: D-12 policy text (A or B wording, or other), and
whether a git-pinned GPUI is acceptable under E-08 dependency pinning.

## Claims and status (all in this document)

| Claim | Status | Environment |
|---|---|---|
| 0.2.2 exposes nothing to UIA | FAIL (measured by S-a11y) | Win 11 x64 Insider 26300 |
| git `main` exposes annotated controls to UIA | PASS by S-a11y | same |
| Narrator speech | NOT RUN | owner check pending |
| macOS VoiceOver | BLOCKED | no Mac |
| Zed `v1.28.1` contains AccessKit | PASS by source inspection only; build NOT RUN | public GitHub, 2026-11-15 |
| git pin removes `ring` (ARM64 blocker) | lockfile analysis only; ARM64 build NOT RUN | this worktree |
| Tray menu / OS notifications accessible | NOT RUN | — |
| S-mem results | not available to this worker | — |

## Method limits

Single Windows machine and Insider build; no speech, NVDA, Accessibility
Insights or macOS testing; upstream status is a 2026-11-15 snapshot of a fast-
moving repository; cost sizes are judgement, not measurement. Public URLs above
were fetched on 2026-11-15; S-a11y's other citations were not re-fetched.

## Rerun

Documentation only. Lockfile figures: count `[[package]]` and
`source = "git+` entries in `spikes\a11y\Cargo.lock` and
`spikes\a11y-main\Cargo.lock`. Tag check: fetch
`crates/gpui/Cargo.toml` and `crates/gpui_windows/Cargo.toml` from
`raw.githubusercontent.com/zed-industries/zed/v1.28.1/` and search for
`accesskit`. Verification: `cargo xtask verify --tier offline` from the worktree
root.
