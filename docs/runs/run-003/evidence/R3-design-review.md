# R3-design-review: independent phase-A review

## Verdict

**FAIL: 10 major findings, 2 minor findings, no blockers.**

The required existing tests pass, but do not cover the compatibility
counterexamples below. The designs are not ready to be treated as a
policy-correct, unambiguous implementation brief. No design or production
file was changed by this reviewer.

## Identity, authority and environment

- Reviewer: R3-design-review, independent verification worker. Assigned model:
  `gpt-6.1-sol`; actual model selection is controlled by the orchestrator's
  launcher, not independently introspectable from these repository commands.
- Checkout: `D:\GitHub\wow-sync.worktrees\R3-design-review`.
  Branch: `run003/R3-design-review`.
- Reviewed source: `07b380ba146da6b0a741254cf2336fecc665d48e`, including
  Q-21 merge `944141a46a9544d2327d85489c80d3fcb10d2957`.
- Environment actually executed: Windows x64, Windows NT
  `10.0.26300.0` development PC; Rust `1.100.0`
  (`0ef1bc386 2026-11-09`), Cargo `1.100.0`
  (`3acc79c0c 2026-11-05`), debug test builds.
- Authority read: AGENTS, COMMON and this worker's packet, SPEC,
  ACCEPTANCE, OWNER-DECISIONS (particularly D-18..D-20), README,
  ENGINEERING and run-003 PLAN.
- Reviewed artifacts: all five UI Markdown files and twelve SVGs,
  questions Q-26..Q-48, removal.md, domain.md, apply.md, publication.md,
  core target/toc/inventory/status/plan/testkit, runtime service and e2e,
  R3-compat evidence and R2-pin evidence. Author evidence was background,
  not the oracle for the test results.
- Scope: synthetic/offline tests only; no WoW, launcher, real install,
  Dropbox or private settings were inspected or operated. No subordinate
  workers were launched. Only this evidence document is authored here.

## Findings

### R3D-1 — major — malformed Interface entries disappear before the safety check

**Location:** `crates/wow-sync-core/src/toc.rs`, `parse_toc`, lines 55..58;
`crates/wow-sync-core/src/target.rs`, `interface_supported`, lines 156..164;
`docs/design/domain.md` §6, “Client-build compatibility by interface number.”

**Evidence:** the parser uses `filter_map(...parse::<u32>().ok())`, retaining
valid fragments and silently dropping malformed ones. The existing passing
test `toc::tests::parses_header_keys_with_bom_and_crlf` explicitly demonstrates
this with `11507, 16001,x`. Consequently, with local evidence `{11508}`, a
different-build candidate declaring `## Interface: 11508,garbage` or
`11508,4294967296` becomes `[11508]` and passes `interface_supported`.
An empty fragment or an additional malformed Interface header can likewise
be lost. This contradicts §6's claim that an unparsable number requires a
decision. A wholly missing/all-invalid line fails closed; a partially invalid
one does not. Valid multi-number candidates do use `all`, not `any`.

**Required fix:** retain validity information through parsing, inventory and
the wire summary, and reject malformed compatibility evidence rather than
silently narrowing it. Add parser-to-target and runtime regressions for a
valid token plus invalid/overflow/empty tokens and conflicting repeated
headers. Preserve the supported valid multi-number and missing-number cases.
Links: D-18 Q-21, R-05/R-11, A-08/A-23/A-38.

### R3D-2 — major — two unknown client builds bypass Q-21 entirely

**Location:** `crates/wow-sync-core/src/target.rs`, `eligible_holders`,
lines 199..200; `docs/design/domain.md` §6 eligibility and compatibility text.

**Evidence:** compatibility is checked only when
`client.build_version != input.local_client.build_version`. With both values
`None`, that condition is false, so an otherwise eligible non-local candidate
can be selected with no Interface declaration and no local Interface evidence.
The design instead requires equality with **both builds present**, and says
“or either is missing” must go through `interface_supported`. The existing
six Q-21 core tests and two runtime tests do not exercise two missing builds.
`ClientInfo` permits `None`, and runtime `client_info` copies that optional
value from variant metadata.

**Required fix:** bypass Interface checking only for equal, present build
identities. Add target and runtime cases for both missing, either missing,
and matching known builds; missing builds must need a decision unless valid
Interface evidence independently authorizes the transfer.
Links: D-18 Q-21, R-05, A-08/A-38.

### R3D-3 — major — installed addon declarations are not proof of client support

**Location:** `crates/wow-sync-core/src/target.rs`,
`local_interface_evidence`; `docs/design/domain.md` §6 compatibility rule;
`docs/design/questions.md` Q-21.

**Evidence:** the supported set is the union of **all numbers declared by
all usable installed selected TOCs**, not evidence of which number the
receiving client supports. For example, a local plain TOC declaring
`11508,99999` authorizes a different-build candidate declaring only `99999`.
An outdated, incompatible or manually installed local addon also contributes
its number. Nothing checks that the receiver actually supports those
declarations. The design itself acknowledges stale local addons as evidence;
D-18 contains no approval to equate installed declarations with actual
client support. The existing positive e2e uses synthetic TOCs with equal
numbers, not an independent client-support oracle.

This is distinct from accidentally scanning another variant: the runtime
does scan only the selected variant, and filters `Ok`/`Selected` correctly.
A multi-client declaration inside that variant is still not proof that this
client accepts every number in the declaration.

**Required fix:** establish supported Interface identity from validated
client/build evidence, or keep unverifiable cross-build candidates as
`NeedsDecision`. If this proxy is desired instead, obtain an explicit owner
decision acknowledging that weaker guarantee. Add a multi-client local-TOC
and stale-incompatible-local-addon negative case.
Links: D-18 Q-21, R-02/R-05, A-02/A-08.

### R3D-4 — major — the light dashboard relabels Forever as Classic Era

**Location:** `docs/design/ui/wireframes/dashboard-light.svg`, line 37,
installation text.

**Evidence:** the actual SVG says
`Installation: World of Warcraft Forever (_classic_era_)`.
The authoritative verified installation is `_forever_`/`wowf`; AGENTS and
SPEC R-02 explicitly prohibit conflating Forever with Classic Era.
This is the light dashboard supplied as R3-proto's reference, not merely an
unrendered comment.

**Required fix:** use the canonical verified Forever identity in the
wireframe and audit the fake scenario identity labels. Do not copy this
mapping into prototype data.
Links: R-02, A-02/A-46, D-09/D-18.

### R3D-5 — major — the reconfirm scenario contradicts the approved build rule

**Location:** `docs/design/ui/prototype.md`, scenario S12 and owner
checklist item 15; `docs/design/ui/screens.md`, “Setup and reconfirm.”

**Evidence:** S12 supplies “build 1.15.7 → 1.15.8” as its changed evidence
and stops syncing until confirmation. D-18 O-N2 explicitly says a
build-version change does **not** force reconfirmation. No additional
layout/mapping change is specified, so a worker following this brief would
demonstrate a forbidden product rule to the owner.

**Required fix:** make S12 an actual consent-relevant layout/mapping change
consistent with core; separately demonstrate that an ordinary build-only
change does not force reconfirmation. Align the screen wording and checklist.
Links: D-18 O-N2, R-02/R-03, A-46.

### R3D-6 — major — the needs-help dashboard promises work that core blocks

**Location:** `docs/design/ui/screens.md`, Overview “Needs help” and
Needs attention “Needs help”; `docs/design/ui/prototype.md`, S7;
`docs/design/ui/wireframes/dashboard-dark.svg`, header subline.

**Evidence:** these screens unconditionally say “Other add-ons keep
syncing.” S7 simultaneously includes Deadly Boss Mods with a journal
`NeedsHelp`. The same design's Needs-help detail correctly says **no updates
are applied on this computer** until that journal is resolved. Core
`plan_apply` returns `UnresolvedJournal` for `gate.any_journal` before
considering any package; apply.md §1 and Q-25 deliberately specify this
computer-wide limitation. Publishing continuing is not other addon updates
being applied.

**Required fix:** make continuation text depend on typed, core-provided
operation scope/block information. Demonstrate ordinary addon decisions
with unrelated work proceeding separately from journal needs-help with all
local applies blocked. Do not promise R-06 behavior the current engine does
not implement, and do not silently change Q-25 in the UI.
Links: D-18 Q-25, R-06/R-12, A-25/A-42.

### R3D-7 — major — normal publication destroys the removal detector's baseline

**Location:** `docs/design/removal.md` §1.2 E1/E4 and §2.6;
`docs/design/apply.md` §1 steps 3..5; `publication.md` §4.

**Evidence:** E1 requires the **newest published and admitted own state**
to contain the package, and E4 requires E1..E3 to hold in two separate scan
cycles. In the unchanged sync cycle, the first complete absent inventory
differs from the prior one and is published immediately, before target
evaluation. On the next scan, the newest own state no longer lists P, so E1
fails and the hold cannot mature. The local observation record stores
`present_seq`, but the design never exempts that persisted baseline from
E1's “newest” requirement or defines suppressing/restructuring publication.
This also risks losing the old present state to the three-own-state-file
retention while the hold is pending.

**Required fix:** define the cycle insertion point and durable prior-presence
baseline explicitly. Preserve its required content/evidence while a hold is
pending, including through ordinary absent-state publications and restarts.
T2/T3 must run through the normal publish loop over multiple cycles, not only
call the proposed observation function in isolation.
Links: R-04/R-11, A-10/A-20/A-22/A-38.

### R3D-8 — major — removal authority can vanish when a newest state is unusable

**Location:** `docs/design/removal.md` §2.1, §2.4, §2.5 and §3.1;
`docs/design/publication.md` §6 admission/no-fallback rule.

**Evidence:** active tombstones and supersessions are recomputed from
**currently admitted** states only. Tombstones are copied by a receiver
when it acknowledges them, not merely when it learns them. Suppose B learns
A's tombstone while WoW blocks applying it. A's next state arrives cloud-only
or malformed; A then contributes no admitted state. Before B has acknowledged
and republished the tombstone, a late peer C's covered old addon arrives.
The specified inputs now contain C's version but no active tombstone, so
ordinary target selection can keep/spread the removed release. The claim that
the sets “only grow” is false for a union over an admission set that can
shrink. No durable learned-authority ledger or stop-on-missing-authority rule
is defined. Similar gaps apply to decision/supersession availability.

**Required fix:** persist and monotonically admit learned removal/decision
authority independently of availability of the originator's latest
inventory, with retirement/authorization validation. Do not restore stale
inventory fallback. Extend T9/T13/T28 to cover learning an intent before ack,
then losing its carrier, across restart and reordered late updates.
Links: R-04/R-06/R-10/R-16, A-18/A-20/A-22/A-45.

### R3D-9 — major — compact tombstones invalidate themselves and lose restore data

**Location:** `docs/design/removal.md` §2.2 hash, §2.5 settled representation,
§6.3 provider restore; `docs/design/questions.md` Q-44.

**Evidence:** §2.2 hashes all fields except the timestamp and declares the
same intent with different content invalid. §2.5 replaces a full tombstone
with a compact form that drops hashed fields. Full and compact copies
therefore have different hashes for the same intent while replicas
legitimately coexist, triggering `InconsistentRemovalRecord`.

The compact representation also drops `removed` (original folder names and
the exact removed tree per folder). `covers` can contain several historical
trees per key and `removed_hash` is only a one-way package hash.
§6.3 nevertheless promises provider-only restore of the tombstone's exact
removed version when no local recovery point exists. That version's manifest
and its retention roots cannot be obtained by the stated restore algorithm
from the compact record.

**Required fix:** separate immutable hashed intent content from an explicitly
versioned, validated compact representation, retaining/referencing an exact
restorable package manifest and associated settings roots. Define full/compact
equivalence. Test mixed full/compact arrivals and restore on a never-present
computer after every local point has been pruned.
Links: R-06/R-10/R-11, A-10/A-16/A-20/A-23.

### R3D-10 — major — choosing a version does not make that version the result

**Location:** `docs/design/removal.md` §3.3 “Keep version V” and subsequent
“New information” rule; domain.md §6 ordinary target ordering.

**Evidence:** keeping V merely supersedes the tombstone and makes V an
ordinary candidate. If the alternatives already considered include V2 and
V3 and the user deliberately chooses V2, V3 is still present in an admitted
inventory and dominates V2 under ordinary comparison. The next target can
therefore install V3, silently overriding the choice. Conversely, a later
unconsidered higher version is promised a new decision in §3.3, but the
ordinary candidate rule would select it. No enforced decision constraint or
candidate rejection/supersession rule is specified. The record example only
shows `chose: {remove: true}`, not the keep-version representation.

**Required fix:** specify and enforce a durable selected-version decision
including exact content, considered alternatives and later-new-information
handling. Explain its interaction with rejection and supersession; do not
rely on ordinary newest-version ordering to honor a deliberate choice.
Extend T11/T13 to choose a lower considered version and deliver a new higher
version after the decision.
Links: R-06/R-10/R-12, A-10/A-18/A-20/A-25.

### R3D-11 — minor — full GUI design coverage is not demonstrated by placeholders

**Location:** `docs/design/ui/screens.md`, Computer detail, Recovery copies,
Settings sync; `prototype.md`, P3/P5/P9 and scenarios S1..S14.

**Evidence:** retirement has only a disabled action and a consequence
paragraph; recovery has a disabled Restore button; settings choices are
static placeholders. There is no prototype scenario for retirement,
master-absence reassignment, rollback's paired/code-only result or a settings
choice. The explicit future labels are honest and appropriate for the
run's non-goals, but they do not establish this packet's stronger “every
screen and state” coverage of A-26/A-45. The owner checklist cannot observe
those missing flows.

**Required fix:** distinguish current prototype coverage from later
acceptance obligations in the design coverage table. Either provide
documentation/fake-state journeys for the deferred screens or mark their
screen/state design obligations explicitly deferred, without claiming their
acceptance cases covered. No live settings/retirement/rollback feature is
requested for this run.
Links: R-10/R-13, A-26/A-42/A-45.

### R3D-12 — minor — the brief leaves conflicting layout and headline rules

**Location:** `docs/design/ui/README.md`, Window layout;
`style.md`, Focus ring; `screens.md`, Shared vocabulary/Headline;
`wireframes/addons.svg`, `wireframes/computers.svg`,
`wireframes/dashboard-dark.svg`; `prototype.md`, ground rules.

**Evidence:** the prose specifies a one-column Computers list followed by a
replacement detail page, but its SVG shows simultaneous list and detail.
The Add-ons prose specifies a “Confirmed by” column and fixed status/name
ordering, while the SVG specifies “Changed” and a sort menu. The dark
dashboard replaces the global Sync now button with Review and omits the
promised persistent group/variant/computer context. The prototype explicitly
instructs a worker to report disagreements, not guess.

Also, `NeedsDecision` currently derives `Headline::Waiting` in core
`status.rs` (asserted by `needs_decision_shows_evidence`), whereas the
proposed mapping presents decisions under `NeedsHelp` without naming that
core-policy change as a gap. `style.md` says GPUI has no focus-visible
distinction, but the pinned `InteractiveElement::focus_visible` implements
exactly that distinction.

**Required fix:** choose one authoritative layout/interaction specification
and align its SVGs; explicitly name the headline-policy gap rather than
implementing it as UI policy; correct the focus-visible capability note.
Links: R-02/R-12/R-13, A-25/A-26/A-42/A-43.

## Independent checks

### Actual commands and results

All commands below ran from this worktree root on the Windows environment
above. Every Cargo invocation prefixes PATH with:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

| Command/check | Result |
|---|---|
| `git --no-pager status --short --branch`; `git rev-parse HEAD` | PASS: assigned branch, initially clean; reviewed SHA above |
| `rustc --version`; `cargo --version`; `[System.Environment]::OSVersion.VersionString`; `(Get-CimInstance Win32_OperatingSystem).OSArchitecture` | PASS: environment recorded above |
| `cargo test -q -p wow-sync-core` | PASS, exit 0: **178 passed**, 0 failed, 0 ignored; doc-tests: 0 |
| `cargo test -q -p wow-sync-runtime e2e` | PASS, exit 0: **28 passed**, 0 failed, 0 ignored, 0 filtered out; includes both Q-21 runtime cases |
| Python 3, stdin-only contrast calculation below | PASS: 10 tokens per theme, both surface/selected pairs recomputed (20 pairs per theme); rounded ratios match within 0.011 and all are at least 4.5:1 |
| Python `xml.etree.ElementTree.fromstring` on every `docs\design\ui\wireframes\*.svg`, plus extraction/read of all text nodes | PASS for XML parsing: 12/12; semantic failures are findings above, not XML errors |
| Pinned GPUI source/API spot checks and R2-pin evidence review | PASS for basic feasibility, with the correction in R3D-12 and runtime limits below |
| `cargo xtask verify --tier offline` | PASS, exit 0: all six offline checks passed; result recorded below |

The first exploratory contrast command accidentally included the dark rows
in the light-section printout. It did not assert a failed design; the corrected
section-bounded command below was rerun and is the reported result.

### Reproducible contrast calculation and observations

Run this script through a PowerShell single-quoted here-string piped to
`python -` (no script or output file is created):

```python
import re
from pathlib import Path

s = Path(r"docs\design\ui\style.md").read_text(encoding="utf-8")

def lum(h):
    c = [int(h[i:i+2], 16) / 255 for i in (1, 3, 5)]
    c = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055)**2.4
         for v in c]
    return sum(a*b for a, b in zip(c, (.2126, .7152, .0722)))

def ratio(a, b):
    low, high = sorted((lum(a), lum(b)))
    return (high + .05) / (low + .05)

for theme, heading, surface, selected in [
    ("light", "Light theme:", "#FFFFFF", "#DCE6F7"),
    ("dark", "Dark theme:", "#1D2128", "#26344D"),
]:
    section = s.split(heading, 1)[1].split("### Buttons", 1)[0]
    section = section.split("Dark theme:", 1)[0]
    rows = re.findall(
        r"\| `([^`]+)`[^|]*\| `(#\w{6})` \| ([\d.]+) \| ([\d.]+) "
        r"\| ([\d.]+) \| ([\d.]+) \| ([\d.]+) \|", section)
    assert len(rows) == 10
    for token, color, bg, surf, side, sel, minimum in rows:
        a, b = ratio(color, surface), ratio(color, selected)
        assert min(a, b) >= 4.5
        assert abs(a-float(surf)) < .011 and abs(b-float(sel)) < .011
        print(theme, token, round(a, 2), round(b, 2))
```

Actual selected-background ratios, sufficient to reproduce at least six
pairs per theme:

| Token | Light | Dark |
|---|---:|---:|
| text | 13.13 | 10.37 |
| text-muted | 6.17 | 5.65 |
| accent | 5.06 | 6.18 |
| ok | 4.89 | 6.13 |
| working | 5.06 | 6.18 |
| waiting | 5.03 | 7.45 |
| help | 4.91 | 6.39 |
| error | 5.23 | 5.47 |
| unknown | 6.17 | 5.65 |
| paused | 5.27 | 6.18 |

These are mathematical token checks, not rendered usability, OS contrast
mode or owner acceptance evidence.

### GPUI feasibility, independently checked against the pin

Read-only inspection of the Cargo Git checkout at revision
`e8bc16acee5ae1937600d571d441c4b5d7b0d288` (Zed `v1.28.1`), specifically:

- `crates\gpui\src\window.rs`: `observe_window_appearance` (2537),
  `appearance` (3154), `set_window_title` (3278),
  `on_window_should_close` (7160).
- `crates\gpui\src\app.rs`: `with_quit_mode` (291),
  `prompt_for_paths` (2032), supporting the native folder picker.
- `crates\gpui\src\elements\div.rs`: `role`, `aria_label`,
  `aria_description`, `aria_selected`, `aria_expanded`, `aria_toggled`,
  and `focus_visible`.
- `crates\gpui\src\elements\uniform_list.rs`: a lazy uniform-height
  list exists; choosing plain lists for wrapping/variable-height rows is
  feasible and does not require inventing virtualization.

R2-pin supplies prior, explicitly limited Windows lifecycle/UIA evidence.
I did not rerun its process-driving harness. The proposed custom sheets,
roving focus and fake scenario transitions remain implementation/interaction
work, not proven behavior merely because the lower-level APIs exist.
Live system-theme response, SVG tinting, DPI/text scaling, UIA/Narrator and
all macOS runtime behavior were **NOT RUN** by this reviewer.

### Data-gap check

The principal UG gaps are real: runtime `StatusSnapshot` exposes strings,
not typed packages/member states; there are no pause/resume, user-decision,
needs-help-resolution, audit-reader, retirement, settings-sync or startup
APIs in the exported slice service. Titles are parsed in `TocInfo` but are
not retained by `TocSummary`; package versions/source detail and per-peer
confirmation summaries are not exported. Recovery records exist internally,
but the snapshot exports only text/counts, not an actionable history list.
The typed activity and update/build/progress fields proposed by the UI are
also not exposed.

Two qualifications matter: runtime already name-resolves some package
sentences via `name_peers` (UG-22 remains real for core renderers), and
merely exporting `StatusView` will not correct the existing
decision-headline policy mismatch identified in R3D-12. Proposed fields are
not authorization to invent missing product policy in the UI.

### Coverage and limits

- **PASS, source review:** same-variant runtime scan, foreign-product/group
  candidate exclusion, `Ok`/`Selected` local-evidence filtering, valid
  multi-number `all` check, unchanged mapping gate, three-state removal
  model on paper, and WoW gates before the described remove/rollback
  renames. Removal tests T1..T33 map to relevant acceptance IDs.
- **FAIL, source/design review:** compatibility safety and removal
  authority/decision/compaction guarantees for the concrete counterexamples
  above; truthful UI identity/reconfirm/continuation rules.
- **NOT RUN:** executing newly added regression cases for R3D-1..R3D-3;
  no tests were added because only this evidence path is owned.
  Findings are independently traced source counterexamples, not fabricated
  failing-test executions.
- **NOT RUN:** independent A-38 production mutations. They would violate
  this reviewer's write boundary; author mutation claims were not counted as
  independently executed evidence. Phase E has the separate mutation remit.
- **NOT RUN:** removal implementation/crash-injection tests; this artifact is
  a future design, not implemented behavior.
- **NOT RUN/BLOCKED for acceptance:** real GUI/owner prototype tasks,
  platform/provider tiers, real-client supported-interface verification and
  all unexecuted architectures. No public-product readiness claim.
- Follow-up belongs to the design/implementation owners and orchestrator.
  No scope approval, default-policy approval, owner usability acceptance or
  additional budget authority is implied by this report.

## Final pre-commit verification

Executed with this evidence document staged, from the assigned worktree root:

```text
[PASS   ] offline/fmt
[PASS   ] offline/clippy
[PASS   ] offline/test
[PASS   ] offline/required-cases
[PASS   ] offline/deny
[PASS   ] offline/markdown-links
overall: PASS (allow-blocked: false) -> exit 0
```

`git --no-pager diff --cached --check` also passed. Passing the offline
tooling gate does not override the FAIL review verdict. The commit SHA is
returned in the worker's final handoff, rather than introducing a
self-referential commit identifier here.
