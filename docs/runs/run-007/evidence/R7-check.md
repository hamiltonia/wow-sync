# R7-check — independent verification

Source under test: `aba7d249d8486af126ee40743d4a28595a99ad4b`.
Checkout `D:\GitHub\wow-sync.worktrees\R7-check`, branch `run007/R7-check`.
Authority: [packet](../packets/R7-check.md), [plan](../PLAN.md), D-29,
[R-03/R-05](../../../../SPEC.md), [A-47/A-48](../../../../ACCEPTANCE.md).
Windows 11 Pro x64, build 26300, 2026-11-23; Rust 1.100.0,
cargo-deny 0.20.2. Verification only; no product changes, dependency changes,
new required cases or nested agents.

Requested role/model: independent verification / gpt-6.1-sol. Actual model
selection and metered AIU are harness/coordinator records, not exposed by these
tools. Worker estimate: approximately **400 AIU**, below the 900 stop ceiling;
this is an estimate, not invented metering.

**Verdict: FAIL.** One confirmed major finding, G1. The stable-observation
RV7-01/RV7-10 cases pass, but normal recent-file observations expose a related
lineage loss and silently undo a live revert. This is not release acceptance.

## Results

| Packet check | Status | Executed outcome / limitation |
|---|---|---|
| Build | PASS | Own `cargo build --release` completed at the source above. |
| 1 — A-47 discovery | BLOCKED | G: beyond F, launcher record, outside-probe exclusion, native picker root/variant normalization and consent all PASS. The zero-install GUI cannot be safely exercised while the real D: install remains discoverable; that subcase is code-path-only, not a GUI PASS. |
| 2 — A-48 lab copies | FAIL | Exact initial transfer, identical copies, mtime-only edits, competing-copy GUI decisions, recovery, removal and GUI restore PASS. Ordinary live single-author edits do not auto-converge: G1. |
| Three-computer offline case | PASS | With the author's edit settled before resume, A/B converged while C was stopped; C returned to the new copy, with its exact old copy in recovery and zero decisions. Does not waive G1 for live edits. |
| Revert addition | FAIL | Settled revert retained the old copy and offered exactly one decision. A live recent-file revert was silently undone: G1. |
| 3 — real data | PASS | 102 eligible, zero blocked; QuestieDB selected at 1.1.1 and published. Its complete tree and every published blob hash matched the backup. Full AddOns/WTF restore had exact manifest equality. |
| 4 — log | PASS | Default path, discovery/setup/reasons, local privacy audit, actual two rotations, override path and actual Explorer data-folder target verified. |
| 5 — regression | PASS | Both offline and native tiers passed at the source under test; final precommit rerun also passed. No allow-blocked exception. |
| Dispatch artifact | PASS | Actions 45237943587 succeeded at this exact source; checksum, packaged notes and isolated GUI executable smoke passed. |
| Unreachable mapped network drive | NOT RUN | Optional packet subcase: no approved unreachable share/mapping was provisioned. No credentials or external network mapping created. |

### Commands and evidence location

Repository commands ran in `D:\GitHub\wow-sync.worktrees\R7-check`,
branch `run007/R7-check`, with the prefix below. TEMP/TMP were set to
`D:\WoWSyncLab\R7-check\scratch`, not a system temporary directory.

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build --release
cargo xtask verify --tier offline
cargo xtask verify --tier native
cargo test --locked -p wow-sync-app --bin wow-sync setup::tests::
cargo test --locked -p wow-sync-runtime discover::tests::
gh run watch 45237943587 --repo hamiltonia/wow-sync --exit-status --interval 60
gh run download 45237943587 --repo hamiltonia/wow-sync -D D:\WoWSyncLab\R7-check\artifact
```

Final raw observations, summaries, screenshots, redacted logs and reproducible
lab procedures remain under **`D:\WoWSyncLab\R7-check\evidence`**.
`artifact-manifest.json` hashes the retained artifacts. Only this Markdown file
is committed, per the packet's write boundary.

The tier stdout files are complete *runner* output, not full hidden child-test
stdout. The native report verified seven required native cases against 841
test results. Setup's six and discovery's eleven additional code-path tests
passed, with their actual named output retained. The zero-test public-api
sub-binary in the discovery-filter command is not counted as coverage.
The first verifier wrapper failed to locate cargo before executing any tier;
its PATH lookup was corrected, then both tiers actually executed.

## Check 1 — discovery and picking

- Created the devkit-layout synthetic install at
  `D:\WoWSyncLab\R7-check\G\Games\World of Warcraft\_forever_`.
  G: was free. Ran
  `subst G: D:\WoWSyncLab\R7-check\G`, launched a normal isolated app at
  `--setup-step 3`, and observed two installation rows. Its log reported
  fixed drives `CDG`, a successful G: probe and two found installations.
  `discovery-G.png` retains step 3. Removed G: with `subst G: /D`;
  final `subst` contains no R7-check mapping.
- Read the actual hidden `%ProgramData%\Battle.net\Agent\product.db`
  read-only: 1,604 bytes, SHA-256
  `23e75b51004821df1e8b5d86dee33d4a3bd59649e9cfa8a7dab12f6f70ce0ea6`.
  The app logged exactly one launcher path, normalized to
  `D:\Games\World of Warcraft`, and its probe succeeded.
- The valid install under `outside-probes\odd-game-root` was absent from
  automatic discovery. Drove the **actual native folder dialog** using
  keyboard/address entry plus UI Automation control coordinates, first
  selecting `_forever_`, then the root. Both selected the same normalized
  Forever candidate. Step 4 still required separate game-identity and
  add-on-scope checkboxes, with Next disabled until consent.
- Selecting `not-a-game` displayed its path and the plain explanation
  “is not a World of Warcraft folder. Choose the folder where it is
  installed…”. Retained `picker-*.png` covers these actual interactions.
- Initial synthetic preparation omitted `.flavor.info`; inspection correctly
  refused it. Corrected the fixture to the repository's devkit layout before
  the successful picker/enrollment checks. This was a fixture error, not a
  product finding. No game was started.
- **Empty-state limit:** discovery always finds the real D: install on this
  PC. No real game folder/drive was renamed, hidden or remapped to force an
  empty result. Read the actual `setup_view` empty-list branch and executed
  `the_empty_state_says_what_to_do_in_plain_words`; its not-found text and
  Choose folder branch are **code-path-only** evidence. The zero-result GUI
  remains BLOCKED, not claimed executed.

## Check 2 — lab sync and recovery

All simulated computers used separate state directories and the release exe:

```text
wow-sync.exe --state D:\WoWSyncLab\R7-check\state-a --dev
  --install "D:\WoWSyncLab\R7-check\a\World of Warcraft"
  --provider "D:\Dropbox\Gaming Sync\WoW Sync Test\R7-check\synthetic"
  --name "R7 a" --interval-ms 1500
```

B/C used their respective paths/names. All three explicitly confirmed
identity/scope and joined the same group through the GUI. These are three
local app instances over one Dropbox test folder, **not** three physical
computers or cross-platform Dropbox-delivery evidence.

- `R7Large.toc`: **1,200,069 bytes**, complete header first, selected version
  **7.1.2** in the GUI/provider state. Every transferred file matched A.
- `R7Bare`: no toc. `R7Ambiguous`: competing Forever/Camelot tocs with
  different versions. Both were eligible, labelled unrecognised and
  initially propagated from A exactly. Identical no-toc copies needed no
  decision. Touching only mtimes left content hashes and decisions unchanged.
- A-only live edits on both unrecognised kinds failed automation (G1).
  GUI “Keep this” resolved them; exact displaced B trees were in recovery.
- Fresh `R7TwinBare` and `R7TwinAmbiguous` shared a baseline, then A/B
  independently changed while paused. After resume, there was **exactly one
  affected add-on decision**, with both active alternatives unchanged.
  “Needs attention → [add-on] → Keep this (This computer)” converged the
  selected A tree; each displaced B alternative matched its recovery tree.
  The old C copy did not resurrect either loser on resume.
- Removed `R7Bare`, then `R7Ambiguous`, from A. Both removals reached all
  three computers. Used **Add-ons → Removed → Restore → Restore** in the
  GUI; all three reinstalls matched their saved pre-removal manifests.
  No CLI recovery action was used.
- Offline case: all three shared a no-toc copy; stopped only C's captured
  PID, changed A while paused, waited for the recent-file window to pass,
  resumed A, and verified A/B equality. Returning C converged exactly with
  **zero decisions**; C's differing prior bytes matched a recovery tree.
  An author-alone variant also passed with a settled edit and B/C stopped.
- Settled revert: paused A, restored its earlier no-toc bytes, waited, and
  resumed. A kept the reverted bytes and offered **exactly one** decision.
  GUI resolution converged all three and preserved the displaced newer
  copy. The separate live-revert counterexample below prevents an overall
  revert PASS.

Tree oracles compare relative paths, directories, file sizes and streaming
SHA-256, deliberately excluding timestamps. Representative tree digests and
per-step results are in the retained `*-convergence.json`, `*-recovery-*.json`,
decision UI observations and synthetic code-state snapshots.
Unrecognised settings are not claimed synced; that is the documented
ownership limitation, not a fallback to whole-WTF transfer.

## Check 3 — real-data backup, publication and exact restore

Checked the running-process inventory before real work and again before
restoration: no WoW client was running. Never started, killed or automated
WoW or Battle.net. The owner's default-state app was not stopped or
reconfigured; every launched app had an explicit lab state directory.

Before enrollment, independently hashed AddOns/WTF, copied both to the lab,
then verified **both backup and a fresh source scan** equal the original:

- **33,312 files, 1,321 directories, 3,455,374,298 bytes**.
- Canonical SHA-256:
  `f96144a9fba4618bb22b31b1bedcbb9e32789aa7689f221d624f486c6ad9ee25`.

Enrolled the real Forever install with isolated `state-real` and the scoped
`R7-check\real` test provider, using `--dev --install` as instructed.
The actual GUI inspection reported **102 eligible / zero blocked**.
The completed `wowsync.code-state/1` publication reported a complete
102-folder inventory with no blocked folder. QuestieDB's selected toc was
`QuestieDB_Forever.toc`, version **1.1.1**, interface **16001**.
Its 64-file published tree and **all 64 provider blob hashes** matched the
backup's original QuestieDB manifest exactly. No addon/settings content was
printed or executed.

Stopped only the real-test app's captured PID. Revalidated the backup,
restored AddOns/WTF, restored the pre-existing metadata state (or removed
verifier-created metadata), and independently rehashed everything.
The restored manifest equalled the original exactly, with the same canonical
digest and file count. `real-backup-proof.json`, `real-publication-proof.json`
and `real-restore-proof.json` retain the non-private results.
Private full manifests and backup bytes were deleted only after this proof.

## Check 4 — default log, privacy and controls

- Actual default files existed at each isolated `<state>\logs\wow-sync.log`.
  Discovery logged each probe/result, the real product.db result, setup
  transitions and inspection counts. Unrecognised reasons appeared.
  A synthetic junction wholly inside the lab produced a blocked unsafe-folder
  reason; it was removed without touching its target.
- Local log audit: **zero profile-path occurrences, zero synthetic payload
  content occurrences, zero private account/realm/character identifier hits**.
  Only safe public addon metadata and synthetic folder identifiers were
  retained. Profile formatting's redaction path is covered by the tier tests;
  no private profile-folder choice was made just to manufacture a log entry.
- Exercised actual log growth through secondary launches against the same
  isolated state; each secondary PID was captured and waited to completion.
  **6,120** launches caused two rotations in about **156 seconds**.
  Previous-file sizes were **1,048,612** and **1,048,626 bytes**: the
  1 MiB threshold plus one log line. Only `wow-sync.log` and
  `wow-sync.log.1` remained, including after the second rotation.
  `rotation-proof.json` records actual measurements.
- The packaged app with `WOWSYNC_LOG=D:\WoWSyncLab\R7-check\override.log`
  wrote there, including its 0.3.0 startup; no default log was created for
  that fresh state.
- Clicked **App settings → Open data folder**. Queried the actual Explorer
  window via Shell automation: it opened
  `D:\WoWSyncLab\R7-check\state-b`, not the owner's default data folder.
  Closed only that newly opened Explorer window, not the Explorer process.

## Dispatch artifact

[Actions run 45237943587](https://github.com/hamiltonia/wow-sync/actions/runs/45237943587)
completed **success** with head
`aba7d249d8486af126ee40743d4a28595a99ad4b`.
ZIP SHA-256, equal to its shipped `.sha256`:

`9b5e0dac6046040edf003621a3b833f9481d9836e7d3e7a3793f315fa4537788`

Safely extracted exactly `wow-sync.exe`, `README.md`, `LICENSE` and
`RELEASE-NOTES.md`; notes matched the checkout byte-for-byte.
Launched that **packaged** exe with isolated state/install/provider flags;
it stayed alive, rendered the Welcome GUI and logged version 0.3.0.
Retained `artifact-proof.json`, `override-proof.json` and
`artifact-smoke.png`. Stopped only its captured PID afterwards.

## Finding

### G1 — MAJOR: an unstable publication forgets lineage

1. Enroll A and B against an isolated test provider. Let a no-`.toc` folder
   converge identically, with a complete published state on both computers.
2. With both apps running, change only A's payload file. Do not change B.
3. A publishes an intermediate `status: unstable`, `tree: null` observation,
   with no lineage entry for that package. Its next complete publication has
   the new hash but **empty ancestors**.
4. Both computers offer one “Choose a copy” decision. B retains the old
   content instead of automatically receiving A's lineage-newer change.
   The condition remained across repeated syncs. A second reproduction on an
   ambiguous-`.toc` folder behaved identically, without the earlier mtime test.

Actual first reproduction (synthetic R7Bare):

| A sequence | Folder state | Published package lineage |
|---|---|---|
| 1 | old copy, `ok` | old hash, empty ancestors |
| 2 | `unstable`, no tree | entry omitted |
| 3 | new copy, `ok` | new hash, empty ancestors |

`next_lineage` in `crates/wow-sync-core/src/lineage.rs` derives the replaced
hash only from the immediately previous publication. If that publication has
no usable package version, the last stable ancestor is forgotten. The new
`local_prev` field fixes RV7-10 for stable observations, but not this
normal live-edit path. No code change was made.

**Live-revert reproduction of the same root cause:**

1. A/B share a fresh no-toc folder. Stop only A, change it to a new copy, wait
   for the recent-file window to pass, and restart A. B automatically receives
   the newer copy, whose published lineage correctly names the old hash.
2. With both apps running, write the old bytes back on A.
3. A publishes an unstable observation, dropping its prior package lineage.
   The next stable scan silently installs B's newer copy over A's deliberate
   revert. The add-on is shown **up to date**, not needing a decision.
4. The reverted bytes are retained in recovery, so this is not unrecoverable
   loss. It nevertheless fails the explicit “must not be silently undone;
   exactly one decision” requirement.

The final fresh probes used 500 ms scans to capture the recent-file interval.
The initial failures used 1,500 ms scans; no source mutation, forged provider
record or special timestamp was needed. A settled-before-scan edit passes,
which explains why the earlier stable-observation regression tests pass.

This is one root-cause finding with two observed effects, not two inflated
defect counts. Suggested worker direction: preserve/recover the last usable
published package lineage across blocked/unstable observations, without
treating absent data as proof of replacement or removal. Add a service-level
regression that actually traverses an unstable scan, including a live revert.

Raw evidence: `G1-live-captured-publications.json` records A's stable baseline
at sequence 37, unstable sequence 38 with no lineage, stable updated sequence
39 with empty ancestors, and B still at the shared baseline. The live-revert
counterexample is `G1-live-revert.json`/`.png` (stable new sequence 42,
unstable 43, replacement of the reverted bytes by sequence 44).
The earlier UI failures and GUI resolutions are retained separately.

## Cleanup and limits

- All launched app instances stopped **only by their recorded launch PIDs**.
  Secondary smoke/rotation launches exited and were waited, not killed by name.
- G: removed; no other drive letter changed.
- Test provider `D:\Dropbox\Gaming Sync\WoW Sync Test\R7-check` deleted.
- Real AddOns/WTF restored exactly before backup removal. Verifier metadata
  was restored to its prior existence/content state.
- Lab contains only the retained `evidence` directory, including safe raw
  observations and procedures. Private manifests, backups, app states,
  extracted binaries, fixtures and scratch files were removed.
- Never touched `D:\WoWSyncDemo`, any other Dropbox Gaming Sync entry, the
  owner's default-state app, or Battle.net/WoW processes.
- No independent physical-peer Dropbox, macOS, dead-share GUI, zero-install
  GUI or timing/performance acceptance is claimed.

`cleanup-proof.json` records actual cleanup. Product source and
`tests/required-cases.toml` remained unchanged; only this evidence file was
committed. Main-checkout cleanliness was checked before commit.
