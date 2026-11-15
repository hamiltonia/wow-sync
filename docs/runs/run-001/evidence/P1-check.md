# P1-check: independent Phase 1 verification

Date: 2026-11-15 UTC (2026-11-14 on the developer PC).
Packet: [P1-check](../packets/P1-check.md), run-001.
Requirements: A-38 (initial), A-39 (initial), A-40; E-03, E-04, E-08.
**Disposition: defects found; not an acceptance or release approval.**
No implementation, tests, workflow, or policy fixes were made.

## Identity, environment, authority

- Source: fresh remote `main`, commit
  `367c0defee9ee9107e7ca0836e65550f99692f91`; evidence worktree initially
  had the same HEAD.
- Verifier: independent verification worker P1-check, agent
  `ed40bcc0-b03a-4b5e-820b-85dd213318eb`; not an implementation author.
- Actual model: **gpt-6.1-sol**. Checked runtime events, not packet frontmatter:
  `subagent.configured` at `2026-11-15T00:43:54.612Z` selected this model,
  medium reasoning, default context. This agent's `session.usage_record`
  events at `00:49:20Z`, `00:49:50Z`, and `00:50:11Z` recorded both `model`
  and `responseModel` as `gpt-6.1-sol`. Runtime source was the local
  orchestrator session `961f3baa-ee85-4e41-b05f-131f1d4d574f`.
  A local session-store SQL query was denied; no database contents were
  reconstructed. Runtime events supplied the necessary evidence.
- OS measured with `Get-CimInstance Win32_OperatingSystem`:
  Microsoft Windows 11 Pro, `10.0.26300`, build **26300**, 64-bit.
  Insider development environment, **not stable-release support evidence**.
- Native host/toolchain: `x86_64-pc-windows-msvc`, MSVC Build Tools present;
  PowerShell. Developer-machine hardware description is in D-07; this packet
  did not independently measure hardware or performance.
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`;
  `cargo 1.100.0 (3acc79c0c 2026-11-05)`;
  `cargo-deny 0.20.2`.
- Artifact: workspace/xtask development-profile binaries, package version
  `0.0.0`; no installed application, GUI, WoW, or provider execution.
- Reviewed packet first, then AGENTS.md, README.md, SPEC.md, ACCEPTANCE.md,
  ENGINEERING.md, OWNER-DECISIONS.md, run PLAN/HANDOFF/USAGE, and verification
  guide. D-04/D-13/D-15 supplied role, ceiling, and plan approvals.
- Ledger snapshot before this worker: 840.70 / 2,500 AIU spent,
  1,659.30 remaining; pause threshold 2,000. These are the committed
  pre-worker figures, **not a live remaining-budget claim**. Orchestrator
  must add this worker's measured usage before further work.

## Isolation and exact common procedure

Owned committed path only:
`D:\GitHub\wow-sync.worktrees\P1-check\docs\runs\run-001\evidence\P1-check.md`,
branch `run001/P1-check`. Initial worktree status was clean.

A fresh **sibling project checkout**, rather than an OS temporary-directory
path, was used to respect the environment's prohibition on temporary-directory
writes. It was outside the main checkout and all worktrees:

```powershell
Set-Location D:\GitHub
git clone --quiet https://github.com/hamiltonia/wow-sync wow-sync-verification-run001
git -C .\wow-sync-verification-run001 rev-parse HEAD
git -C .\wow-sync-verification-run001 branch --show-current
Set-Location D:\GitHub\wow-sync-verification-run001
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Observed HEAD was the full source SHA above, branch `main`.
No commit/push was made from the verification checkout. All mutations below
were confined to that clone and reverted using:

```powershell
git checkout -- .
git clean -fd
git status --short
```

Status after restorations was empty. Build output under ignored `target`
was retained between controls; tracked files and lockfile were restored.
The clone was removed after checks. No writes to the main repository or other
workers' worktrees occurred. No WoW/Dropbox data, new credentials, paid
services, workflow triggers, or sub-agents were used.

Network use: fresh GitHub clone; read-only existing `gh` login; crates.io
metadata/downloads and ordinary Cargo/cargo-deny registry/advisory activity.
The documented `offline` tier means no runtime provider prerequisite, **not
an air-gapped execution**. License-candidate inspection included
`cargo search`, `cargo info` for savage_core, gpl-math, sudoku, svg2gcode,
librespot-protocol, gchemol-core and synfx-dsp, and a public crates.io search.
Only the selected dependency was added to the disposable clone. No external
service received private project/game data.

## 1. Documented baseline and JSON report

Started from only `docs/VERIFY.md` for execution instructions:

```powershell
cargo xtask verify --tier all
$LASTEXITCODE
Get-Content target\verify\report.json
cargo xtask verify --tier all --allow-blocked
$LASTEXITCODE
Get-Content target\verify\report.json
```

Actual first-run printout:

```text
[PASS   ] offline/fmt
[PASS   ] offline/clippy
[PASS   ] offline/test
[PASS   ] offline/required-cases
[PASS   ] offline/deny
[PASS   ] offline/markdown-links
[BLOCKED] native/tier
          no native cases defined yet
[BLOCKED] provider/tier
          requires D-08 Dropbox test folder and enrolled peers
overall: BLOCKED (allow-blocked: false) -> exit 1
```

| Check | Observed status | Reported subprocess exit / execution |
|---|---|---|
| offline/fmt | PASS | `cargo fmt --all --check`, 0 |
| offline/clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`, 0 |
| offline/test | PASS | `cargo test --workspace`, 0 |
| offline/required-cases | PASS | Internal check; 1 listed offline case against 19 results |
| offline/deny | PASS | `cargo deny check`, 0 |
| offline/markdown-links | PASS | Internal check; 22 Markdown files, no broken relative links |
| native/tier | BLOCKED | Not executed; no native cases defined |
| provider/tier | BLOCKED | Not executed; required test folder/enrolled peers unavailable to this packet |

JSON matched all eight printed statuses and the corresponding details.
For the first invocation its fields were:

```json
{
  "tiers": ["offline", "native", "provider"],
  "allow_blocked": false,
  "overall": "BLOCKED",
  "exit_ok": false
}
```

The second invocation retained all eight statuses and details, with
`allow_blocked: true`, `overall: "BLOCKED"`, `exit_ok: true`; observed process
exit **0**. It did not relabel unavailable runtime tiers PASS.
These excerpts summarize report fields; the actual report also contained
the eight `checks` objects described in the table.

After all controls/restorations:
`cargo xtask verify --tier offline` returned **0**, all six checks PASS,
same 1-case/19-result/22-Markdown details. Independent
`cargo test --workspace` returned **0**: xtask 19 passed, 0 failed/ignored;
empty core library and its doctests each ran 0 tests.

## 2. Negative controls and behavioral counterexample

Status below describes whether the **control was correctly caught**.
An expected verifier FAIL is a passing negative control, not a product pass.
The exit column is the directly observed command/gate exit; internal check
functions do not have separate process exit codes.
Unless noted, all other offline checks remained PASS.

| Control / IDs | Status | Command | Observed failing check / exit |
|---|---|---|---|
| NC-a nonexistent required name; A-39/E-04 | PASS, caught | `cargo xtask verify --tier offline` | required-cases FAIL; 1 |
| NC-b ignored listed test; A-39/E-04 | PASS, caught | `cargo xtask verify --tier offline` | required-cases FAIL; 1 |
| NC-c formatting + clippy diagnostic; E-08 | PASS, caught | `cargo xtask verify --tier offline` | fmt and clippy FAIL; 1 |
| NC-c with override; A-39/E-04 | PASS, caught | `cargo xtask verify --tier all --allow-blocked` | fmt/clippy FAIL, native/provider BLOCKED; 1 |
| NC-c2 warning-only escalation; E-08 | PASS, caught | `cargo xtask verify --tier offline` | clippy FAIL, fmt PASS; 1 |
| NC-d GPL dependency, attempt 1; E-08 | BLOCKED before verifier | `cargo xtask verify --tier offline` | dependency compilation failed; 101 |
| NC-d GPL dependency, attempt 2; E-08 | PASS, caught | `cargo xtask verify --tier offline` | deny FAIL; 1 |
| NC-e relative Markdown link; E-08 | PASS, caught | `cargo xtask verify --tier offline` | markdown-links FAIL; 1 |
| NC-zero listed inventory preserved; A-39/E-04 | PASS, caught | `cargo xtask verify --tier offline` | required-cases FAIL; 1 |
| NC-zero empty inventory; A-39/E-04 | **FAIL, missed** | `cargo xtask verify --tier offline` | none: all PASS; **0** |
| NC-noop behavioral oracle; A-38/E-03/E-04 | PASS, caught | `cargo xtask verify --tier offline --allow-blocked` | test FAIL, required-cases SKIPPED; 1 |
| NC-noop direct test | PASS, caught | `cargo test --workspace inventory::tests` | 5 passed, 2 failed; 101 |
| NC-noop restored implementation | PASS | `cargo test --workspace inventory::tests` | 7 passed, 0 failed; 0 |
| Missing deny detection; A-39/E-08 | PASS, visibly BLOCKED | `cargo xtask verify --tier offline` with environment below | deny BLOCKED, unrelated link FAIL; 1 |
| Clean missing-deny overall/override isolation | BLOCKED | Two environment attempts below | Contaminated link check; isolated overall behavior not established |

### NC-a

Appended to `tests\required-cases.toml`:

```toml
[[case]]
id = "P1-check-negative-a"
tier = "offline"
tests = ["inventory::tests::p1_check_nonexistent_test"]
```

Observed:
`P1-check-negative-a: listed test 'inventory::tests::p1_check_nonexistent_test' did not run`.
Restored before NC-b.

### NC-b

Added `#[ignore]` directly below `#[test]` on
`inventory::tests::zero_tests_with_listed_cases_is_a_problem`.
Cargo's test command itself passed, but inventory enforcement reported:
`A-39: listed test 'inventory::tests::zero_tests_with_listed_cases_is_a_problem' was ignored`.
Restored before NC-c.

### NC-c and warning-only follow-up

Appended to `crates\wow-sync-core\src\lib.rs`:

```rust
pub fn p1_check_warning()->bool{ true == true }
```

Formatting reported the unformatted function; clippy reported
`equal expressions as operands to ==`, `clippy::eq_op`.
That lint is deny-by-default, so a separate, restored-baseline warning-only
control confirmed the requested warning escalation:

```rust
pub fn p1_check_warning_only() -> bool {
    return true;
}
```

Observed fmt PASS and clippy FAIL:
`unneeded return statement`;
`-D clippy::needless-return implied by -D warnings`.
Both mutations were removed.

### NC-d: actual GPL-3.0 license

`cargo info synfx-dsp` confirmed **0.5.6, GPL-3.0-or-later**.
First added `synfx-dsp = "=0.5.6"` under xtask's normal dependencies.
The pinned stable toolchain could not compile it:

```text
error[E0554]: #![feature] may not be used on the stable release channel
#![feature(portable_simd)]
error: could not compile synfx-dsp (lib) due to 1 previous error
```

Exit 101 occurred **before the verifier ran**. The old `report.json` remained
and still contained a previous PASS for deny; this was explicitly not counted
as license-test evidence.

Reverted that attempt, then used this host-safe, declared-target dependency
in `xtask\Cargo.toml`:

```toml
[target.'cfg(target_arch = "aarch64")'.dependencies]
synfx-dsp = "=0.5.6"
```

The configured cargo-deny graph includes ARM targets. Deleted the previous
report before the second invocation. All host offline checks except deny
passed; new report and printout contained:

```text
[FAIL   ] offline/deny
license = "GPL-3.0-or-later"
rejected: license is not explicitly allowed
GPL-3.0-or-later - GNU General Public License v3.0 or later
synfx-dsp v0.5.6
  └── xtask v0.0.0
overall: FAIL (allow-blocked: false) -> exit 1
```

Manifest and Cargo.lock restored. No third attempt or toolchain change.
This establishes detection of a GPL dependency on a declared non-host target;
it does not establish that this nightly-only crate builds on stable.

### NC-e

Inserted in README.md an inline link to
`docs/P1-check-missing-relative-link.md`, which did not exist.
Observed:

```text
[FAIL   ] offline/markdown-links
          broken relative links:
          README.md: docs/P1-check-missing-relative-link.md
overall: FAIL (allow-blocked: false) -> exit 1
```

Restored before additional controls.

### Zero-test controls and confirmed false green

Added `#[cfg(any())]` after `#[cfg(test)]` on all four xtask test modules:
`main.rs`, `inventory.rs`, `links.rs`, `report.rs`.
With the required inventory unchanged:

```text
[PASS   ] offline/test
[FAIL   ] offline/required-cases
          tier 'offline' ran zero tests but has 1 listed case(s)
overall: FAIL (allow-blocked: false) -> exit 1
```

Direct `cargo test --workspace` returned **0** with 0 tests in all suites.
Then, while keeping those module filters, removed the `[[case]]` block from
`tests\required-cases.toml` (left only comments).
The verifier returned **0**, all six offline checks PASS. JSON contained:

```json
{
  "tier": "offline",
  "name": "required-cases",
  "status": "PASS",
  "detail": "0 listed offline case(s) verified against 0 test result(s)"
}
```

Thus the verifier detects zero execution only **while obligations remain
listed**. It does not protect against inventory disappearance accompanied by
zero execution. No acceptance-ID minimum or independent inventory preservation
gate prevented this observed false green. All filters/inventory edits restored.

### A-38 no-op implementation counterexample

Replaced only the body of `inventory::check_required` with:

```rust
let _ = (inv, tier, results);
Vec::new()
```

Formatting and clippy passed. Real behavioral tests failed:

```text
inventory::tests::zero_tests_with_listed_cases_is_a_problem
assertion left == right failed: left: 0, right: 1
inventory::tests::missing_ignored_and_failed_listed_tests_are_reported
assertion left == right failed: left: 0, right: 2
test result: FAILED. 17 passed; 2 failed; 0 ignored
[SKIPPED] offline/required-cases
overall: FAIL (allow-blocked: true) -> exit 1
```

The direct seven-test inventory selection failed with those two names,
exit 101, then passed all seven after restoring the implementation, exit 0.
No assertions were weakened and no fix was authored. A-38 initial evidence
therefore concerns the verification engine, not unimplemented product behavior.

### Missing cargo-deny: detected, isolation limitation

Attempt 1 prepended the real pinned toolchain bin and removed
`%USERPROFILE%\.cargo\bin` from PATH:

```powershell
$toolchain=Split-Path (rustup which cargo)
$env:PATH="$toolchain;"+(($env:PATH -split ';' |
    Where-Object { $_ -notlike '*\.cargo\bin*' }) -join ';')
cargo deny --version
```

It still returned `cargo-deny 0.20.2`, exit 0: Cargo also searches CARGO_HOME.
No missing-tool claim was made for this attempt.

Attempt 2 created clone-local `.cargo-isolated`, with a directory junction
`registry` referencing the existing registry cache, set CARGO_HOME to it,
and used the PATH above. No executable was removed/uninstalled.
`cargo deny --version` then genuinely returned:
`error: no such command: deny`, exit **101**.
Both documented verifier variants were run in this environment.

Observed deny check was **BLOCKED**, never PASS, with:
`tool unavailable ... install with: cargo install --locked cargo-deny`.
Fmt/clippy/test/required-cases remained PASS.
However, the Markdown walker traversed the cache junction inside the clone
and reported dependency-document relative links missing, for example
`allocator-api2-0.2.21\README.md: COPYING`. Therefore both ordinary and
`--allow-blocked` invocations returned **1 / overall FAIL**.
This was test-setup contamination, not evidence that allow-blocked incorrectly
handles an otherwise isolated missing-deny case.

After two environment attempts, stopped this sub-step rather than trying again.
The clean missing-deny overall/override result is **BLOCKED**, not claimed
passed. Removed the junction itself using its directory object's `Delete()`,
then removed the isolated home; did not recursively delete through the junction.
Confirmed original registry still exists, cargo-deny still reports 0.20.2,
clone status clean, and restored normal offline verification PASS/0.

## 3. Fresh-worker onboarding: A-40

Started at AGENTS.md. Its numbered links locate SPEC intent, ACCEPTANCE IDs,
ENGINEERING responsibilities, README boundaries and OWNER-DECISIONS approvals.
D-06 and AGENTS' `docs/runs/` instruction locate run-001. PLAN's packet table
locates P1-check, which specifies branch, worktree, inputs and owned evidence
path. REPO-MAP links PLAN/HANDOFF/USAGE/packets/evidence and tells the worker
that `docs/VERIFY.md`, once present, is the command authority.

| Onboarding item | Status | Finding |
|---|---|---|
| Intent/acceptance/engineering/decisions | PASS | Authoritative documents present and reachable |
| Approved scope/model/ceiling | PASS | D-04/D-13/D-15 resolve approval and role policy |
| Plan/packet/dependencies/write boundary | PASS | Discoverable without original chat |
| Verification command/report location | PASS | VERIFY exists; documented commands executed successfully |
| Evidence and cumulative ledger location | PASS | Run evidence directory and USAGE discoverable |
| Current-state accuracy / safe resume | **FAIL** | Contradictory stale status described below |

Concrete stale/contradictory records at source SHA:

1. **README.md** says “No implementation exists yet,” excludes a Cargo
   scaffold, an approved worker/model matrix, and numeric credit authorization.
   Workspace/xtask/CI exist and D-04/D-13/D-15 have approved policy/ceiling/plan.
2. **PLAN.md** status, budget table and prerequisites still say Phase 1 waits
   for D-13, ceiling TBD/open, while D-13 and USAGE record approval and actual
   Phase 1 work. This is stale, not a newly missing approval.
3. **HANDOFF.md** says Phase 1 is only merged locally at `0e0b876`, push is
   blocked by workflow scope, and P1-check is not launched. Remote source is
   `367c0de`, the first CI run exists/completed, and this worker is executing.
   Its “next actions” would send a fresh orchestrator toward already-completed
   credential/push/launch steps.
4. **REPO-MAP.md** marks verifier, inventory, deny config, reports and workflow
   as Planned and says only P0 commands have been reproduced. Those files are
   present and this packet independently reproduced the verifier.
   It also assigns deny policy to P1-verify while PLAN assigns it to P1-ci.
5. **USAGE.md** still has four workers, no P1-check row. This is a pre-worker
   snapshot, not a trustworthy live total; must be updated after completion.

A fresh worker can find the authoritative files, but cannot obtain a fully
accurate current handoff from them. **A-40 is not fully satisfied.**
No status/document repairs were made.

## 4. First real GitHub Actions run

Read-only commands:

```powershell
gh run view 43659578231 --repo hamiltonia/wow-sync --json status,conclusion,headSha,jobs,url
```

Repeated bounded status observations while doing local checks.
No watch, rerun, cancellation or workflow mutation was needed: it completed.
Run URL: https://github.com/hamiltonia/wow-sync/actions/runs/43659578231
Source SHA matched the tested source. Created `2026-11-15T00:43:18Z`;
last updated `00:47:30Z`; status **completed**, conclusion **success**.

| Job | Status | Conclusion | Completed UTC | Claim |
|---|---|---|---|---|
| verify-offline (windows-latest) | completed | PASS / success | 00:47:29 | Offline verification only |
| verify-offline (macos-latest) | completed | PASS / success | 00:46:17 | Offline verification only |
| build-only aarch64-pc-windows-msvc | completed | PASS / success | 00:44:13 | Compile only |
| build-only aarch64-apple-darwin | completed | PASS / success | 00:44:03 | Compile only |
| build-only x86_64-apple-darwin | completed | PASS / success | 00:43:53 | Compile only |
| build-only x86_64-pc-windows-msvc | completed | PASS / success | 00:44:10 | Compile only |

All six jobs had no failed steps in their returned job records.
`gh run view --log-failed`: **NOT RUN**, no failed job/step to inspect.
Job-name honesty: **PASS**. Names explicitly distinguish offline verification
and build-only targets; build step says nothing is run or uploaded.
They do not imply GUI/native/provider/product runtime support.
All public-target product runtime acceptance remains **NOT RUN** here.

## 5. Findings, limitations and handoff

| Acceptance / outcome | Status | Evidence |
|---|---|---|
| A-38 initial verifier-engine behavioral counterexample | PASS | No-op fails real tests; restored implementation passes |
| A-39 initial documented discovery/reporting | **FAIL overall** | Ordinary inventory controls caught; empty inventory plus zero tests falsely passes |
| A-40 fresh-worker onboarding | **FAIL overall** | Locations discoverable; current-state documents contradictory |
| E-08 formatting/lint/license/link controls | PASS for exercised controls | NC-c/c2/d/e caught |
| Native execution | BLOCKED | No native cases defined; no support claim |
| Real-provider execution | BLOCKED | Unavailable scoped prerequisite; no WoW/Dropbox access |
| Installed-app/GUI/performance/product acceptance | NOT RUN | Outside bootstrap packet |

Defects/limitations for orchestrator and assigned authors:

- **Confirmed false green (A-39/E-04):** zero executed tests plus emptied
  required-case inventory returns offline PASS/0.
- **Confirmed documentation drift (A-40/E-08):** README, PLAN, HANDOFF and
  REPO-MAP cannot provide an accurate clean resume despite decision register
  and packet being discoverable.
- **Report freshness limitation:** compilation failure before xtask starts
  leaves a prior report in place; JSON has no source SHA/run timestamp.
  Do not treat an existing report as evidence for a failed current invocation.
  This was directly observed in NC-d attempt 1.
- **Missing-tool isolation limitation:** deny itself visibly BLOCKED; a clean
  overall missing-deny override test was not established after two setup
  attempts. Cache-junction Markdown scanning also demonstrates that the link
  walker scans repository-contained directory junction targets.
- This evidence cannot establish product behavior, stable OS support,
  native tray/GUI behavior, real-provider behavior, or release readiness.

Persistent output is this evidence file only. The orchestrator owns evidence
review, current-state/usage updates and defect assignment. No push or merge
authorized or performed.
