# R3-proto (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first.

**Authority:**
- [docs/design/ui/prototype.md](../../../design/ui/prototype.md) is the brief.
- The rest of `docs/design/ui/**` is the specification. The written prose
  is authoritative; the wireframes illustrate it.
- Review status: UI design PASS
  ([R3-design-recheck](../evidence/R3-design-recheck.md)).
- D-18, D-19 (roomy Syncthing-style dashboard, follows the system light/dark
  theme).

**Owned paths:**
- `crates/wow-sync-proto/**`, which is new: a workspace member binary crate
  `wow-sync-proto`.
- The root `Cargo.toml` workspace `members` entry only.
- `Cargo.lock`, only for changes caused by the new crate using
  already-locked dependencies.
- `docs/runs/run-003/evidence/R3-proto.md`.
- Screenshots in `docs/runs/run-003/evidence/R3-proto/`, as PNG, total under
  1 MB.
- `tests/required-cases.toml`: add entries only.

## Goal

Build a clickable GPUI prototype that runs on **fake, in-memory scenario
data only**. It has no provider, no install, no file writes and no runtime
service. The owner uses it to judge the design (A-42). Build exactly what
`prototype.md` lists: every screen, every scenario (S1 onwards), a scenario
switcher, light/dark/system theme, keyboard navigation and focus, and
accessible names.

## Rules

- **Dependencies:** use GPUI at the existing Zed `v1.28.1` pin through the
  same dependency declaration as `wow-sync-app`. Add no new third-party
  crates; if one is unavoidable, stop and report.
- **Product crates:** the prototype must not depend on `wow-sync-runtime`
  or `wow-sync-adapters`. It may depend on `wow-sync-core` for types only.
  It must not call product policy to decide anything.
- **Wording and identity:** user-visible text comes from the design
  wording. Forever is "World of Warcraft: Forever (wowf)" and never
  Classic Era.
- **Disagreements:** if the design is ambiguous or contradicts itself,
  follow the prose. Record each instance under "Design disagreements" in
  the evidence; do not invent product policy.
- **Feasibility questions:** record the outcome for each of these:
  - Q-30 (notification preview);
  - Q-31 (text input);
  - Q-32 (list and roving focus, measured with S14);
  - Q-33 (sheets);
  - Q-35 (live theme change: whether GPUI reports it at the pin);
  - Q-39 (tray click; if the tray isn't in scope for the prototype, record
    NOT RUN).
- **Tests:** add at least a few unit tests over the pure scenario and
  view-model code, for example that every scenario has the data its screens
  need and that Forever labels are correct. Add them to the offline tier in
  required-cases. They must run on macOS too, which means no Windows-only
  code in them.
- **Builds:** CI builds every workspace member on Windows and macOS targets,
  so keep the code cross-platform. Any Windows-only code needs a `cfg` gate.

## Evidence

- **Screenshots:** at least the dashboard in light and dark, and one screen
  per major state, as PNGs (downscaled if needed). Capture only the
  prototype window, using a method that does not capture other windows or
  private data.
- **Owner launcher:** `D:\WoWSyncDemo\Start-Prototype.cmd`, created outside
  the repository. It runs a release build of `wow-sync-proto.exe` copied to
  `D:\WoWSyncDemo\bin\`. Document it in the evidence.
- **Owner checklist:** the checklist from `prototype.md`, with your own
  observed result for each item (PASS / FAIL / NOT RUN, with notes). Your
  results are self-checks; the owner decides.
- **Memory:** the private working set of the prototype after idling with
  the window hidden or open, for information only.

## Done

`cargo xtask verify --tier offline` passes. The commit uses the prefix
`run003(R3-proto):` and the trailer.
