# Run 007: find WoW anywhere, sync everything, diagnostic log, `v0.3.0`

Status: **approved** (D-29). Ceiling 4,000 AIU from the run-007 baseline in
[USAGE.md](USAGE.md); stop launching new work at ≥3,700.

- Drafted 2026-11-23 by the Opus 5.5 orchestrator. Triggers from the owner's
  use of `v0.2.0`:
  - on a second PC, WoW at `G:\Games\World of Warcraft` was not found, and
    setup step 3 showed an empty list with no way to choose a folder;
  - on the dev PC, QuestieDB never synced: its two `.toc` files are 17.6 MB
    and the inspection cap is 256 KiB;
  - the only log needs `WOWSYNC_LOG` set before launch, and it records nothing
    about discovery.
- Inputs: D-29, SPEC R-03 and R-05 as amended, A-47, A-48,
  [../run-006/REPORT.md](../run-006/REPORT.md).

## 1. Goals

1. **Find WoW anywhere (A-47).** Discovery covers:
   - the install path recorded by Battle.net
     (`%ProgramData%\Battle.net\Agent\product.db`, read as untrusted bytes);
   - every local fixed drive A–Z (network, removable and optical drives are
     skipped so a dead network drive cannot stall setup), each with the
     `World of Warcraft`, `Games\World of Warcraft`,
     `Program Files (x86)\World of Warcraft` and
     `Program Files\World of Warcraft` locations;
   - the existing `%ProgramFiles%` probes.
   When nothing is found, step 3 says so and offers "Choose folder…". It also
   offers it when installs were found, for a different one. The chosen folder
   may be the install root or a variant folder (`_forever_`). It then goes
   through the normal identity and scope confirmation. A folder that is not a
   WoW install gets a plain explanation.
2. **Oversized `.toc` files are read from their header (A-48).** Read at most
   the cap, parse the complete lines in it, and treat the result as the
   description. QuestieDB becomes a normal recognised add-on.
3. **Sync add-ons that cannot be recognised (A-48, SPEC R-05).** A folder that
   is readable and safe, but whose `.toc` is missing, ambiguous, unvalidated
   or broken, becomes transfer-eligible as an "unrecognised" add-on with no
   version.
   - Ordering comes only from content lineage. Identical copies converge. A
     copy that changed from the last shared copy, while the others still have
     that shared copy, wins automatically.
   - Differing copies without lineage raise one ChooseCopy decision for that
     add-on, with the alternatives preserved.
   - Unreadable, unsafe (links) and unstable folders stay blocked, with their
     reason.
   - Removal, recovery, rollback and the three-state history work as for any
     other add-on.
   - Settings for an unrecognised add-on follow the `<Folder>.lua` rule (Q-67)
     only if that is already how ownership works. Otherwise they stay unsynced,
     and the release notes say so.
4. **Diagnostic log on by default.**
   - The default location is `logs\wow-sync.log` in the app data folder
     (`%LOCALAPPDATA%\WoWSync`), rotated at 1 MiB with one previous file.
     `WOWSYNC_LOG` still overrides the path.
   - Log the start, the version, every discovery probe and its result, setup
     steps, blocked and unrecognised add-ons with their reason, and errors.
   - Never log add-on or settings contents, or account, realm or character
     names.
   - "Open data folder" in App settings becomes active and opens that folder.
5. **Pre-release `v0.3.0`**, with notes covering goals 1–4 and the remaining
   gaps.

## 2. Non-goals

- No Dropbox API, installer, signing or macOS binary.
- No full-drive recursive search.
- No change to the variant boundary rules.

## 3. Phases and packets

| Phase | ID | Role / model | Work |
|---|---|---|---|
| A | R7-sync | Implementation / claude-sonnet-5.5 | Goals 2 and 3: core, adapters, runtime sync and design docs |
| A | R7-setup | Implementation / claude-sonnet-5.5 | Goals 1, 4 and 5: runtime `discover.rs`, the app, version, release notes, README |
| B | R7-check | Verification / gpt-6.1-sol | A-47 (with a `subst` drive letter beyond F and a picker-only folder), A-48 on lab copies and real dev-PC data per D-14/D-28, the log, artifact smoke |
| C | R7-review | Integration / claude-opus-5.5 | Safety review of goal 3 against SPEC R-04/R-05/R-06 and the AGENTS rules, before the tag |

Gates for the tag:
- both verify tiers pass;
- CI is green;
- a dispatch build succeeds;
- R7-check passes;
- R7-review has no open critical or major finding.

Planned workers: 4. Up to 4 more for fixes and rechecks (D-05 limits).

## 4. Budget estimate

| Item | AIU |
|---|---|
| R7-sync | 1,000 |
| R7-setup | 700 |
| R7-check | 700 |
| R7-review | 300 |
| Orchestrator | 600 |
| Fixes and rechecks | 600 |
| **Total** | **3,900** |

## 5. Stop conditions

- Stop at ≥3,700 AIU, at an owner barrier, or on any unapproved scope,
  credential or user-data need.
- **Stop all work** if a COMMON rule is broken, or if real dev-PC data is
  changed without a verified backup.
- No tag without the §3 gates.

## 6. Owner questions

None blocking. The owner may correct the D-29 defaults.

## 7. Packet rules

[packets/COMMON.md](packets/COMMON.md).
