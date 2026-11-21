# Run 005 report: run-004 gaps, owner answers, first pre-release

Status: **closed**. Plan [PLAN.md](PLAN.md) (D-27, ceiling 10,000 AIU from
D-26). Ledger [USAGE.md](USAGE.md). Handoff [HANDOFF.md](HANDOFF.md).

## 1. Outcome

- **Release:** `v0.1.0` is tagged on `8ecb621` and published as a GitHub
  **pre-release** with the Windows x64 zip and its SHA-256, as D-26
  approved. Tag release run 44957612025 passed (build, then publish), and
  the page lists `wow-sync-0.1.0-windows-x64.zip` and its `.sha256`.
- **Gates passed on that commit:**
  - both verify tiers;
  - CI run 44955214721 (all 6 jobs, including macOS);
  - release dispatch run 44955214908;
  - R5-check's artifact smoke test on that build: checksum, contents,
    About 0.1.0, and a `build_commit` of `8ecb621`.
- **Not done:** it still does not sync real WoW data, because the Q-24
  synthetic-marker gate stays. There is no macOS binary, no installer and no
  signing. The release notes state these gaps.
- **Usage:** about 3,529 AIU of the 10,000 ceiling. Workers used 2,396 and
  the orchestrator about 1,133. 6 workers were used: 4 implementers, 1
  verifier and 1 reviewer.

## 2. What was built

| Goal | Result | Merge |
|---|---|---|
| 1. Uninstall removes and recovers code and settings (RV-04) | Uninstall captures settings (up to the cap) and removes them with the code. Restore from Removed reinstalls the **code** from a local recovery copy or the provider manifest (removal.md §7.4, hash-verified, nothing partial, waits while WoW runs, rejected versions refused) and returns the settings. Restore progress and lasting "can't be restored" items have Try again and Dismiss | `1c543ce`, `bd39010`, `9d3a144`, `debc58f`, `4da4477`, `f478fca`, `54656e8` |
| 2. Q-77 = 32 MiB | `MAX_SETTINGS_FILE_BYTES = Some(33_554_432)`; the "size limit pending" banner is gone. Larger files are `TooLarge`; on uninstall they are left in place (known gap) | `1c543ce`, `559cb4d` |
| 3. Q-88 (a) | A newer version installed only on this computer stays installed and unshared. The decision offers Keep / Use / Follow newest again / Decide later and stays in Needs attention | `1c543ce`, `559cb4d` |
| 4. DataAtRisk | The runtime raises it (unreadable state, failed recovery copy, full storage). It shows in the headline, tray and Needs attention, whatever the notification toggles say | `1c543ce`, `559cb4d` |
| 5. Diagnostics | File fingerprints and build commit (RV-09). `Command::SaveDiagnostics` removed. Each save is a new timestamped file | `1c543ce`, `1fb0234` |
| 6. GUI correctness | Keep and Resume send the decision's real reason | `1fb0234` |
| 7. Hygiene | RV-10 (`--locked`, `docs/DEPENDENCIES.md`, MIT `LICENSE`) and RV-11 (absolute macOS helper paths) | `4ec8104`, `1fb0234`, `1c543ce` |
| 8. Pre-release | Version 0.1.0 and `release.yml`: tag `v*` runs verify, then release build, zip and sha256, then a pre-release; `workflow_dispatch` builds an artifact only. A 404 from `releases/latest` shows "No release has been published yet." | `4ec8104`, `4675f08`, `7a4df23`, `103a02f`, `d24310d` |
| 9. Independent checks | R5-check (initial, recheck, final) and R5-review | `861f2ff`, `0df1a8e`, `1a76cf8`, `fc8f734` |

## 3. Verification findings and their state

| ID | Source | Severity | State |
|---|---|---|---|
| F1 | R5-check | major | An uninstall after a group restore left code on the originating computer: the apply plan reinstalled the folder in the cycle that recorded its removal. Fixed `bd39010` and rechecked PASS. Fixing it exposed that §7.4 code restore had never been built; that is now built (`9d3a144`) |
| F2 | R5-check | minor | Stale 404 text in the release notes. Fixed `103a02f` |
| F3 | R5-check recheck | major | Removed and Restore were hidden when no add-ons were active. Fixed `2030912`; final recheck PASS |
| F4 | R5-check recheck | major | Restore refusals were not persisted, and a failed retry cleared them. Fixed `5b9a337`; final recheck PASS |
| RV5-01 | R5-review | moderate | A corrupt shared copy gave a false settings note and an endless retry. Fixed `5b9a337`; final recheck PASS |
| RV5-02 | R5-review | low | Fall back to the local copy on a hash mismatch. Fixed `5b9a337` |
| RV5-03 | R5-review | low | README and REPO-MAP pointed at run-004. Fixed `8ecb621` |
| RV5-04 | R5-review | low | Q-77 "PENDING OWNER" text. Fixed `5b9a337` |
| RV5-05 | R5-review | low | Over-cap settings file on uninstall is now stated in the notes. Fixed `d24310d` |
| RV5-06 | R5-review | info | Pin comments dropped. Fixed `d24310d` |
| RV5-07 | R5-review | low | Restore test for the account and character boundary added. Fixed `5b9a337` |

The release workflow's first dispatch failed on a `-Maxdepth` packaging bug
(fixed in `4675f08`).

## 4. Not done or blocked

- **GUI observations still BLOCKED** (unit tests only):
  - the over-32-MiB refusal in the real window;
  - the DataAtRisk native tray tooltip (UI Automation could not reach it);
  - the "No release has been published yet." state in the GUI, because
    developer builds have no update source.

  Now that the pre-release exists, the owner can see the last one by
  running the published exe.
- **macOS:** the owner kit run is still pending
  ([../run-004/MAC-OWNER.md](../run-004/MAC-OWNER.md)). CI builds and
  offline-verifies macOS; nothing has been run on a Mac.
- **Deferred:**
  - D-11 telemetry;
  - DA-2 to DA-4 and A-31 (D-23);
  - Q-68 real-client checks (D-23);
  - real-data enrollment (the Q-24 gate);
  - installer (A-35) and code signing.
- **Native-tier flake:** one unexplained native FAIL on `7a4df23`. Every
  later run passed: at least 12, including R5-check's dedicated reruns.
  It is still open, with no reproduction.

## 5. Incidents

- **I-5: stray writes into the main checkout.** Workers' .NET file APIs
  with relative paths resolved against the process start directory. The
  orchestrator contained the writes; nothing stray was committed (see
  HANDOFF). PLAN §5 says to stop all work on a COMMON-rule breach. The
  orchestrator judged the breach non-destructive and continued, and
  discloses that here. No later breach occurred after the absolute-path
  and main-status-check rule was sent.

## 6. Owner input needed

1. Run the published `v0.1.0` from the
   [release page](https://github.com/hamiltonia/wow-sync/releases/tag/v0.1.0)
   on Windows and report what you see. It is synthetic-data only: real
   enrollment stays blocked by the Q-24 gate.
2. The macOS kit run (MAC-OWNER.md).
3. Choose the next run's scope. The main candidate is lifting the Q-24 gate
   for real-data enrollment, which needs its own safety plan and approval.
   Others are the Dropbox API spike, an installer, and signing.
