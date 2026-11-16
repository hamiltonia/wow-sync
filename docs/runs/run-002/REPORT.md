# Run 002 report: design and first vertical slice

Status: **closed at budget review** (D-17: continue until the credit budget
is used, then review remaining work). Orchestrator: claude-opus-5.5, session
`961f3baa`. Usage: [USAGE.md](USAGE.md). State: [HANDOFF.md](HANDOFF.md).

## 1. Outcome

A **synthetic-only** Windows vertical slice exists on `main`: enroll a
(synthetic) Forever or unfamiliar install with guided consent, inventory
addons, publish addon-code state to a provider folder, apply a newer addon on
a second install with WoW-running gate, journaled staging, recovery and
truthful peer confirmation, in a GPUI tray app. It is an engineering
checkpoint, **not** usable on real WoW data (Q-24 marker gate) and not a
release.

Integration review verdict: **accept with follow-ups**
([R2-review](evidence/R2-review.md)); independent re-check:
**pass with findings** ([R2-recheck](evidence/R2-recheck.md)).

## 2. What was built

| Area | Result | Evidence |
|---|---|---|
| GPUI pin | Zed `v1.28.1` (`e8bc16a`), `QuitMode::Explicit` (no anchor window), AccessKit UIA exposure; lifecycle 14/14; 12–19 MiB private WS; 4-target CI build green | [R2-pin](evidence/R2-pin.md) |
| Design | `docs/ARCHITECTURE.md`, `docs/design/` (9 files); two review rounds, ready with fixes; 25 owner questions with safe defaults | [R2-design](evidence/R2-design.md), [fix](evidence/R2-design-fix.md), [recheck2](evidence/R2-design-recheck2.md) |
| `wow-sync-core` | Platform-free policy, executor over port traits, crash-idempotent journal recovery; arch test enforces no OS I/O | [R2-core](evidence/R2-core.md) |
| `wow-sync-adapters` | Safe fs roots, scan/hash, install staging/journal, provider folder with cloud-flag refusal, process observation | [R2-adapters](evidence/R2-adapters.md) |
| `wow-sync-runtime` / `wow-sync-app` | `SliceService`, e2e two-install tests, GPUI shell with G1–G9 screens, developer mode | [R2-slice](evidence/R2-slice.md), [R2-fix](evidence/R2-fix.md) |
| Verification | Clean clone, 24 negative controls caught, G1–G9 independently driven; M1–M5 found and resolved | [R2-check](evidence/R2-check.md), [R2-recheck](evidence/R2-recheck.md) |
| Hardening | MJ-1 consent reset, plain-English provider errors, changed variant → needs help, durable apply report across crash, consent/status policy in core, no duplicate report after failed clear (R3-M1); A-38 controls recorded per fix | [R2-fix2](evidence/R2-fix2.md), [R2-fix3](evidence/R2-fix3.md), [R2-recheck3](evidence/R2-recheck3.md), [R2-fix4](evidence/R2-fix4.md), [R2-recheck4](evidence/R2-recheck4.md) (PASS) |

CI (`ci`, `spike-gpui`) green on Windows and macOS after the slice merge
(macOS clippy fix `ea9692d`).

## 3. Not done / blocked

- Memory gate (A-29) for the app: **BLOCKED** pending owner review of V1;
  one corrected-method run measured ≈16 MiB, author run 12.4 MiB.
- `native` and `provider` verify tiers: BLOCKED (xtask native tier not wired,
  Q-17; Dropbox online-only/hydration needs owner-prepared files, Q-23).
- macOS runtime, ARM64 runtime, Narrator speech, real tray mouse input:
  NOT RUN.
- Real WoW data: deliberately out of scope (Q-24).
- Settings sync, removal/uninstall, rollback/retire, needs-help GUI,
  packaging: not started (later runs).

## 4. Incidents (owner review requested)

See [HANDOFF.md](HANDOFF.md) "Incidents": possible termination of up to 8
unrelated processes by R2-check's first memory harness (V1); R2-slice ran
`git gc --prune=now` (held decision) and its history rewrite reverted an
orchestrator commit (restored before push); a user-name leak caught before
merge. Packet rules were tightened after each.

## 5. Owner decisions pending

Consolidated in [R2-review](evidence/R2-review.md) "Owner decisions pending":
PLAN §6 provisional items #2–#4, #6, #7 and held #1, #5, #8; design
questions Q-1…Q-25 ([questions.md](../../design/questions.md)); O-N1–O-N3;
the incidents above.

## 6. Remaining work (estimate)

See [HANDOFF.md](HANDOFF.md) "Remaining work".
