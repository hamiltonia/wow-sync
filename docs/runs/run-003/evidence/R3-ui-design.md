# Evidence: R3-ui-design (run-003)

Worker: R3-ui-design. Packet:
[R3-ui-design.md](../packets/R3-ui-design.md). Checkout:
`D:\GitHub\wow-sync.worktrees\R3-ui-design`, branch `run003/R3-ui-design`,
based on `b420717`. Environment: Windows 11 Pro x64 dev PC, Python 3.15.2,
Rust toolchain from `$env:USERPROFILE\.cargo\bin`.

Scope: documentation only. No code, test, build or CI file was changed. Nothing
was rendered in GPUI; every GPUI capability named in the design is either taken
from R2-pin evidence or listed as an open question for R3-proto.

## Files written

| File | Content |
|---|---|
| [docs/design/ui/README.md](../../../design/ui/README.md) | Principles, window layout, navigation map, tray, close/pause/quit, startup, first-run wizard |
| [docs/design/ui/screens.md](../../../design/ui/screens.md) | Shared vocabulary and mappings from core types; every screen with data, actions and a 9-state table; data gaps UG-1..UG-24 |
| [docs/design/ui/style.md](../../../design/ui/style.md) | Light/dark tokens with measured contrast, buttons/chips, status vocabulary and icons, focus ring, spacing, type, high-DPI |
| [docs/design/ui/interaction.md](../../../design/ui/interaction.md) | Keyboard map, focus order per screen, AccessKit roles/names with fallbacks, notifications |
| [docs/design/ui/prototype.md](../../../design/ui/prototype.md) | R3-proto brief: screens P1..P14, fake scenarios S1..S14, evidence to produce, owner checklist (25 items) |
| `docs/design/ui/wireframes/*.svg` (12) | [dashboard light](../../../design/ui/wireframes/dashboard-light.svg), [dashboard dark](../../../design/ui/wireframes/dashboard-dark.svg), [tray and quit](../../../design/ui/wireframes/tray-and-quit.svg), [setup](../../../design/ui/wireframes/setup.svg), [computers](../../../design/ui/wireframes/computers.svg), [add-ons](../../../design/ui/wireframes/addons.svg), [add-on detail](../../../design/ui/wireframes/addon-detail.svg), [decision](../../../design/ui/wireframes/decision.svg), [activity](../../../design/ui/wireframes/activity.svg), [settings sync](../../../design/ui/wireframes/settings-sync.svg), [app settings](../../../design/ui/wireframes/app-settings.svg), [about](../../../design/ui/wireframes/about.svg) |
| [docs/design/questions.md](../../../design/questions.md) | Appended Q-26..Q-39 |
| this file | Evidence |

Inputs read: AGENTS.md, COMMON.md, the packet, SPEC.md, ACCEPTANCE.md,
OWNER-DECISIONS D-18/D-19, run-003 PLAN.md, R2-pin evidence,
`docs/design/README.md`, `questions.md`, `apply.md` §6,
`crates/wow-sync-core/src/status.rs`, `explain.rs`,
`crates/wow-sync-runtime/src/service.rs` (`StatusSnapshot`, `EnrollPreview`),
`crates/wow-sync-app/src/{ui,backend,main}.rs`. None of the sources were
modified.

## Commands and results

| # | Command | Result |
|---|---|---|
| 1 | Contrast script below (`python -`, stdin; no file written) | PASS: 0 failures; every text and status token ≥ 4.5:1 on `bg`, `surface`, `sidebar`, `selected` and its chip fill, both themes; values match the tables in style.md |
| 2 | `[xml](Get-Content <svg> -Raw)` for each of the 12 wireframes | PASS: all 12 parse as XML |
| 3 | `cargo xtask verify --tier offline` (PATH prefixed with `$env:USERPROFILE\.cargo\bin`) | PASS, see [Verification](#verification) |
| — | Rendering the SVGs in a browser, rendering any screen in GPUI, Narrator/UIA checks, macOS | NOT RUN (out of scope; R3-proto) |

Contrast script (WCAG 2.x relative luminance):

```python
def lum(h):
    h = h.lstrip("#")
    c = [int(h[i:i+2], 16) / 255 for i in (0, 2, 4)]
    c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]

def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)

themes = {
    "light": dict(bg="#F5F6F8", surface="#FFFFFF", sidebar="#EBEEF2", selected="#DCE6F7",
        tokens=dict(text="#1A1F29", muted="#4A5363", accent="#1D5BBF", ok="#17703A",
            waiting="#8A5300", help="#A64205", error="#B42318", paused="#6941C6"),
        chips=dict(ok="#E3F3E8", working="#E3ECFA", waiting="#FBF0D9", help="#FDEBDD",
            error="#FCE4E2", unknown="#ECEEF1", paused="#EEE8FB"),
        pairs=[("#FFFFFF", "#1D5BBF"), ("#FFFFFF", "#A64205"), ("#FFFFFF", "#B42318"),
            ("#8A94A6", "#FFFFFF"), ("#7D8799", "#FFFFFF"), ("#7D8799", "#F5F6F8"),
            ("#7D8799", "#EBEEF2"), ("#1D5BBF", "#FFFFFF")]),
    "dark": dict(bg="#14171C", surface="#1D2128", sidebar="#191C22", selected="#26344D",
        tokens=dict(text="#E8EAEE", muted="#A7AFBB", accent="#8AB8FF", ok="#5CCB7F",
            waiting="#F2C14E", help="#FFA463", error="#FF8A80", paused="#C4A8FF"),
        chips=dict(ok="#1C3326", working="#1E2D47", waiting="#3A2F14", help="#3F2716",
            error="#432020", unknown="#2A2F37", paused="#2E2645"),
        pairs=[("#0E1626", "#8AB8FF"), ("#14171C", "#FFA463"), ("#14171C", "#FF8A80"),
            ("#6B7585", "#1D2128"), ("#6B7585", "#14171C"), ("#6B7585", "#191C22"),
            ("#FFD166", "#1D2128"), ("#FFD166", "#14171C")]),
}
fail = 0
for name, t in themes.items():
    print(f"== {name}: text tokens on bg/surface/sidebar/selected")
    for tok, hx in t["tokens"].items():
        r = [ratio(hx, t[s]) for s in ("bg", "surface", "sidebar", "selected")]
        ok = min(r) >= 4.5
        fail += not ok
        print(f"{tok:8} {hx} " + " ".join(f"{x:5.2f}" for x in r) + f"  min {min(r):5.2f} {'PASS' if ok else 'FAIL'}")
    print(f"== {name}: status text on chip fill; text on chip fill")
    status = dict(t["tokens"]); status["working"] = status["accent"]; status["unknown"] = status["muted"]
    for st, fill in t["chips"].items():
        r, rt = ratio(status[st], fill), ratio(t["tokens"]["text"], fill)
        ok = r >= 4.5
        fail += not ok
        print(f"{st:8} {fill} status {r:5.2f} text {rt:5.2f} {'PASS' if ok else 'FAIL'}")
    print(f"== {name}: other pairs (fg on bg)")
    for fg, bgc in t["pairs"]:
        print(f"{fg} on {bgc}: {ratio(fg, bgc):5.2f}")
print("FAILURES:", fail)
```

Output (abridged to the minimum per row; full output reproduced every value
in style.md):

```text
light min: text 13.13, muted 6.17, accent 5.06, ok 4.89, waiting 5.03,
           help 4.91, error 5.23, paused 5.27
light chips (status on fill): 5.34 5.35 5.60 5.32 5.43 6.67 5.54; text on fill >= 13.63
dark min:  text 10.37, muted 5.65, accent 6.18, ok 6.13, waiting 7.45,
           help 6.39, error 5.47, paused 6.18
dark chips (status on fill): 6.64 6.82 7.84 7.10 6.27 6.08 7.02; text on fill >= 10.92
control-border: light 3.62/3.35/3.11, dark 3.47/3.86/3.66 (>= 3:1)
focus ring: light #1D5BBF on surface 6.36; dark #FFD166 on surface 11.20, on bg 12.46
FAILURES: 0
```

## Coverage: acceptance IDs against screens

| A-ID | What the UI must make possible | Where it is designed | Prototype check |
|---|---|---|---|
| A-01 | App remains available in the tray; hide/reopen; start at sign-in | README [Tray](../../../design/ui/README.md#tray), [Close](../../../design/ui/README.md#close), [Startup](../../../design/ui/README.md#startup); screens App settings | checklist 16, 18; P12, P13 |
| A-04 | Join with explicit consent; replaced data recoverable | README [onboarding](../../../design/ui/README.md#first-run-onboarding) step 5; screens Setup | checklist 12, 14; S1 |
| A-08 | Ambiguous/partial/different-build choices are per add-on | screens Decision, Needs attention | checklist 9, 10; S7 |
| A-15 | Settings pause (future) | screens Settings sync (placeholder only) | checklist 24 |
| A-24 | Last confirmed vs unknown now; long-unused computer; late acknowledgement | screens Overview, Computers, Add-on detail "On each computer" | checklist 5, 6, 7; S9 |
| A-25 | What changed, which copy, why, affected computers, next action; IDs only in Details | screens Add-on detail, Decision, Activity | checklist 8, 9; S3, S4, S7 |
| A-26 | All supported flows through the GUI, keyboard, readable themes | all screens; interaction.md keyboard/focus/AccessKit; style.md contrast | checklist 1–4, 20–22 |
| A-27 | Batched success notifications, separate actions, deferral while gaming, no repeats | interaction.md [Notifications](../../../design/ui/interaction.md#notifications); Activity | checklist 23; S5 |
| A-28 | Honest update notice, manual install, no self-replacement | screens About and updates | checklist 25; S13 |
| A-33 | Diagnostic export | App settings (disabled, "Coming later") | — (future) |
| A-42 | Owner approves the early prototype | prototype.md checklist | the whole checklist |
| A-45 | Retire a computer; absent settings master is not silently replaced | Computer detail (disabled "Retire this computer…"); Settings sync absent-master text | — (future; UG-20) |
| A-46 | Guided identity and safe-scope confirmation for an unfamiliar client | README onboarding step 4; screens Setup and reconfirm; setup wireframe | checklist 13, 15; S2, S12 |

No negative-control test (A-38) applies: no behaviour code was written.

## Gaps and questions

Data gaps (core/runtime does not expose what a screen needs) are UG-1..UG-24
in [screens.md, Data gaps](../../../design/ui/screens.md#data-gaps), each with a
suggested field. The most important for build-out:

- UG-1 runtime status is strings only; UG-2 no paused state; UG-3 package
  display name and version labels; UG-4 per-computer confirmation list;
  UG-7/UG-19 no audit-log reader or batch events (Activity, notifications);
  UG-8/UG-9 no decision or needs-help resolution API (Decision is read-only,
  Q-25); UG-12 no next action per item; UG-17 no safe-point stop signal
  (pause, quit); UG-22 explanations render hex computer IDs and the Ok
  headline reads as group-wide; UG-23 no start-at-sign-in API.

New questions with recommended defaults (Q-26..Q-39 in
[questions.md](../../../design/questions.md)):

- Owner: Q-26 close hides to tray, Q-27 72 h "not heard from" threshold, Q-28
  pause persists until resumed, Q-29 quit waits for a safe point, Q-34 theme
  override, Q-36 automatic update check on with disclosure and opt-out.
- Design/orchestrator: Q-30 notification dependency, Q-31 no text input,
  Q-32 lists and roving focus, Q-33 no stock menus/dialogs, Q-35 live theme
  detection, Q-37 local clock, Q-38 multiple groups, Q-39 tray click.

## Limits

- Docs only. No screen was rendered; SVG wireframes were checked to parse,
  not visually reviewed in a renderer by this worker.
- GPUI capabilities assumed but unverified (prototype checks): AccessKit roles
  Navigation/Banner/Status/Alert/Switch/Tab/Dialog/ProgressIndicator (fallbacks
  in interaction.md), live region announcements, `set_window_title`, SVG icon
  tinting, live appearance change on Windows (Q-35), text-scale honouring.
- macOS behaviour (tray click, template icon) is designed from platform
  convention only.
- Possible merge overlap: R3-removal-design appends Q-40+ to
  `docs/design/questions.md` directly after this worker's rows.

## Verification

`cargo xtask verify --tier offline` from the worktree root, with the new files
staged (so the markdown-link check covers them), on the Windows 11 dev PC:

```text
[PASS   ] offline/fmt
[PASS   ] offline/clippy
[PASS   ] offline/test
[PASS   ] offline/required-cases
[PASS   ] offline/deny
[PASS   ] offline/markdown-links
overall: PASS (allow-blocked: false) -> exit 0
```

The commit SHA is reported in this worker's final reply (not written here, to
avoid amending the commit).

## Retry 1 (R3-ui-fix)

Worktree `D:\GitHub\wow-sync.worktrees\R3-ui-fix`, branch `run003/R3-ui-fix`
from main `917bc05`. Input: the FAIL review
[R3-design-review.md](R3-design-review.md). Scope: UI findings R3D-4, R3D-5,
R3D-6, R3D-11 and R3D-12 only. R3D-1/2/3 (Q-21 code) and R3D-7..10 (removal)
belong to other workers and were not touched.

Sources re-read for the fixes (read only): `wow-sync-core` `variant.rs`
(`confirm_enrolled`, `display_label`), `status.rs` (headline derivation, test
`needs_decision_shows_evidence`), `plan.rs` (`PlanBlock::UnresolvedJournal`),
`wow-sync-runtime` `service.rs` (journal path skips planning and pushes no
block), OWNER-DECISIONS D-09 and D-18 O-N2, and GPUI at the pin
(`e8bc16a`, `crates/gpui/src/elements/div.rs`, `focus_visible`).

### Dispositions

| Finding | Disposition | What changed |
|---|---|---|
| R3D-4 Forever shown as Classic Era | Fixed | New screens.md [Identity labels](../../../design/ui/screens.md#identity-labels): canonical "World of Warcraft: Forever (wowf)", folder `_forever_` (core `display_label`, D-09), never Classic Era; fake builds `fake-build-A/B`; fake unfamiliar client `_example_`/`wowx`. Every header context line, README layout and onboarding step 3, the interaction.md accessible-name example, prototype ground rule 5 and S1/S2 now use it. `dashboard-light.svg` (`_classic_era_`), `computers.svg` and `setup.svg` (`_anniversary_`, `1.15.8`) corrected. Audit grep of `docs/design/ui` for `Classic Era|_classic|1.15|Warcraft Forever|_anniversary_` leaves only the two "never Classic Era" rules. |
| R3D-5 S12 reconfirm used a build-only change | Fixed | screens.md Reconfirm gains a trigger table from `confirm_enrolled`: `mapping_id` change and recognition-class change reconfirm; build-only change does not (D-18 O-N2); new folders under the same rule do not; product/folder change is `VariantChanged`, an Error with no Confirm. S12 is now a mapping change `validated-plain/1` → `profile:wowf/1` ("5 more folders would sync"). New S15 demonstrates a build-only patch (`fake-build-A` → `fake-build-B`) with no reconfirm, Ok headline and the new build shown. Checklist 16 (S12) and 17 (S15). UG-16 now names a typed `ReconfirmCause`. |
| R3D-6 continuation text contradicted Q-25 | Fixed | New screens.md [Continuation scope](../../../design/ui/screens.md#continuation-scope) with two cases that are never merged: a decision ("Only WeakAuras is affected. Other add-ons keep syncing.") and a journal needing help ("No updates are applied on this computer until this is resolved. Sharing this computer's add-ons continues."); the journal sentence wins when both exist. Overview, Needs attention, Add-on detail and Decision use it. S7 is decisions only (DBM removed; Details! applies during the scenario); new S16 is the journal case with 3 held updates. Typed scope named as UI gap **UG-25** (`StatusView.apply_scope` or exported `PlanBlock::UnresolvedJournal`); the prototype fakes it (`FakeApplyScope`), and the build-out shows neither sentence until UG-25 exists. Checklist 9 (S7), 10 (S16). |
| R3D-11 coverage mixed prototype coverage and deferred obligations | Fixed | Coverage split into two tables below. prototype.md gains "Deferred: not demonstrated and not claimed" (A-45, recovery restore/rollback, removal, settings sync A-15/A-26 settings parts, A-33). |
| R3D-12 SVGs disagreed with prose; headline mismatch; focus note | Fixed | README and screens.md prose declared the single authoritative layout; wireframes follow it, and a disagreement is a design bug. Header is identical on every page (icon, headline, context line, Sync now, Pause); page buttons never replace them; all 10 page wireframes now draw it. `computers.svg` is the full-width list only (detail replaces it). `addons.svg`: no sort control, columns Name · Version here · Status · Confirmed by, chips All (44) / Not up to date (3) / Needs you (3) / Not synced here (2). `dashboard-light.svg`: This computer key/value rows + Recent changes left, Other computers right, no Needs attention panel in Success. `dashboard-dark.svg` (S7): Sync now restored, context line, Needs attention / This computer / Recent changes left, Other computers right. The NeedsDecision → `Headline::Waiting` mismatch is recorded as core gap **UG-26**; the UI renders core's value ("Waiting for your decision on 3 add-ons", hourglass) in the header, tray, dashboard-dark, addons and decision wireframes. style.md focus note corrected: GPUI `focus_visible` exists at the pin (keyboard-only, needs `track_focus`). |

### Coverage now (Retry 1; replaces the table above)

**Covered by the prototype now** (an owner checklist item and scenario
demonstrate it on fake data; still subject to the owner's A-42 decision):

| A-ID | Part demonstrated | Checklist / scenario |
|---|---|---|
| A-01 | Tray presence, hide/reopen, explicit quit (start at sign-in faked, UG-23) | 18, 20; P12, P13 |
| A-04 | Join with explicit consent; replaced add-ons kept | 13, 15; S1 |
| A-08 | Ambiguous/partial/different-build choices per add-on | 9, 11; S7 |
| A-24 | Last confirmed vs unknown now; silent computer; late acknowledgement | 5, 6, 7; S9 |
| A-25 | What changed, which copy, why, affected computers, next action; IDs only in Details (apply and decision only) | 8, 9, 10; S3, S4, S7, S16 |
| A-26 | Add-on sync flows through the GUI and keyboard; readable themes | 1–4, 22–24 |
| A-27 | Batched notifications, separate actions, deferral while gaming, no repeats (preview only) | 25; S5, S7 |
| A-28 | Honest update notice, manual install, no self-replacement | 27; S13 |
| A-42 | Owner reviews the early prototype | whole checklist |
| A-46 | Guided identity and scope confirmation; reconfirm only for consent-relevant changes | 14, 16, 17; S2, S12, S15 |

**Deferred obligations (not covered; no acceptance claim):**

| A-ID | Deferred part | Where it stands |
|---|---|---|
| A-15 | Settings pause and all settings sync | Placeholder page only (P9, checklist 26 checks the placeholder, not A-15) |
| A-25 | Recovery restore and removal explanations | Recovery count shown only; restore, rollback and removal UI not designed for the prototype (removal design owns removal) |
| A-26 | GUI-only recovery restore, rollback, retirement and settings flows | Not designed or demonstrated |
| A-33 | Diagnostic export | Disabled "Coming later" in App settings |
| A-45 | Retire a computer; absent settings master not silently replaced | Disabled "Retire this computer…" only; UG-20 |

### New or changed gaps

- UG-16 changed: typed `ReconfirmCause { MappingChanged, RecognitionChanged }`.
- UG-25 new: typed continuation/apply scope (Q-25).
- UG-26 new: core decision-headline policy (`Waiting` vs `NeedsHelp`).
- No new Q rows; Q-26..Q-39 unchanged.

### Limits (Retry 1)

- Docs only; SVGs were checked to parse as XML, not visually reviewed in a
  renderer. Text widths in wireframes are approximate.
- GPUI `focus_visible` behaviour on Windows is read from source, not run.

### Verification (Retry 1)

`cargo xtask verify --tier offline` from the R3-ui-fix worktree root with all
changes staged, Windows 11 dev PC:

```text
[PASS   ] offline/fmt
[PASS   ] offline/clippy
[PASS   ] offline/test
[PASS   ] offline/required-cases
[PASS   ] offline/deny
[PASS   ] offline/markdown-links
overall: PASS (allow-blocked: false) -> exit 0
```

All 12 SVGs parsed as XML with PowerShell `[xml]`. The commit SHA is reported
in the final reply.
