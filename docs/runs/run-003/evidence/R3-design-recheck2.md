# R3-design-recheck2 — final removal retry

## Verdict

**Removal: FAIL.** R3D-14, R3D-15 and R3D-16 are closed for their
reported design defects. R3D-13 remains **major / NOT CLOSED**: the new
resolution procedure still cannot resolve an id whose variants name
different packages.

This review is limited to retry 2 and the four findings in
[recheck 1](R3-design-recheck.md). No settled UI/minor findings were
reopened. No additional finding ID is assigned to the remaining R3D-13
resolution defect.

**Q-21 / R3D-3 remains an owner decision and was not re-reviewed.**
Nothing here approves that proxy, Q-53 or another pending owner proposal.
This is design-source verification, not removal implementation acceptance.

## Checkout, authority and method

- Worktree: `D:\GitHub\wow-sync.worktrees\R3-design-recheck2`.
- Branch: `run003/R3-design-recheck2`.
- Reviewed HEAD/merge:
  `82b7d375d22004f48b5dbb14f18975d20430cf11`.
- Read the retry-2 diff and the affected canonical sections of
  [removal.md](../../../design/removal.md), the revised Q-44/Q-50 and
  new Q-53 in [questions.md](../../../design/questions.md), and the
  author's [Retry 2 claims](R3-removal-design.md).
- Read AGENTS and COMMON in this checkout. `git diff --name-only bff2112
  HEAD` confirmed SPEC, ACCEPTANCE, ENGINEERING, owner decisions and the
  approved run plan are unchanged since the previous review; their
  previously read authority remains applicable. Re-read SPEC R-06/R-10.
- Followed the modified rules through the original counterexamples and
  the new variant-reference, status and payload-root rules. These are
  explicit source-level traces, not executed removal behavior.
- Windows x64, Windows NT `10.0.26300.0`; Rust
  `1.100.0 (0ef1bc386 2026-11-09)`; Cargo
  `1.100.0 (3acc79c0c 2026-11-05)`.
- Assigned verification packet/model policy is unchanged. Actual runtime
  model selection is launcher-controlled, not independently introspected.
  No subagents or other worktrees used.

## Dispositions

| Finding | Disposition | Independent evidence |
|---|---|---|
| R3D-13 | **NOT CLOSED — major** | §2.5 V4 now permits known conflicted variants, and status precedence correctly distinguishes missing information from actionable inconsistency. The same-package case can resolve without making its variants valid. However, the required all-variants decision still fails V7 when one variant names another package; trace below |
| R3D-14 | **CLOSED** | E6 explicitly reads P's own snapshot. §1.6 persists scan-indexed snapshots while any package entry references them; §2.8 represents those references and prohibits discarding a referenced snapshot on overflow. Refreshing Q no longer replaces P's historical evidence. T6 now spans multiple cycles, an independent update and restart |
| R3D-15 | **CLOSED** | §2.7 separates permanent metadata from payload roots. Superseded intents/keep decisions no longer root payload; local earlier history remains subject to current plus two previous. Active removal/keep targets and frozen alternatives have explicit roots and visible lifetime/recovery semantics. §6.3 limits provider-backed restore advertisements to active intents. T23 now inspects provider roots through repeated settled cycles. Q-44 is metadata-only; Q-53 is explicitly an owner proposal, not indefinite historical-payload authority |
| R3D-16 | **CLOSED** | §3.3 gives separate total formulas: keep W is `(inherited ∪ seen) − {W}`, automatic is `inherited − allow_again`, and remove is empty. Resuming adds no newly seen release to declined. The V2/V3/V4 trace below matches Q-50 and T11c. `OnlyDeclinedVersions` explicitly freezes rather than returning an empty target |

### R3D-13 residual — cross-package conflicts remain unresolvable

**Severity: major.** Locations: removal §2.3 equivalence;
§2.5 V1/V4/V7 and “Resolving a conflicted id”
(lines 430–465); §3.1 inconsistency rule; §7 GUI action.
Links: R-06/R-12/R-13, A-20/A-23/A-25/A-26.

The retry explicitly recognizes differences in package key as an id
conflict, and the resolving decision must list **every** known variant.
But V7 still requires every referenced object's package key to equal the
decision's one package key. Failure is `invalid(wrong_package)`, ignored
and never authority. Permitting conflicted references in V4 does not
exempt them from V7.

Counterexample, with current group-member issuers and otherwise readable,
well-formed objects:

1. Learn intent variant `(T, hP)` with `package_key = P` and another
   variant `(T, hQ)` with `package_key = Q`, where P differs from Q.
   The hashes are the recomputed hashes of the respective different
   contents. V1 makes the id conflicted.
2. The user resolves P's inconsistency through the advertised GUI,
   choosing a valid version of P. Decision D has `package_key = P`,
   lists both variants in `considered.intents` and `knows`, and supplies
   a valid chosen manifest/formula.
3. V4 is satisfied because both variants are known and conflicted.
   V7 fails because the second referenced object belongs to Q.
   D is invalid and cannot resolve the conflict.
4. Leaving out Q's variant fails the all-known-variants resolution rule.
   A decision scoped to Q fails symmetrically on P's variant.
   Replay/restart preserves the conflict in the grow-only ledger.

This is not a request to weaken ordinary cross-package authority checks.
The design needs an explicit distinction between **conflict evidence**
and authority/effects: a narrowly scoped decision must be able to name
foreign-package conflicting evidence without applying any change across
that safety boundary. Define which package(s) the resolution covers.
Extend T31b/T34 through resolution of mismatched-package variants,
restart and replay. T34 currently requires detection of a package-key
mismatch, but T31b does not require resolving this case.

### Replayed closure traces

- **R3D-14:** P's baseline points to scan s0. An unsupported rename of P
  appears at s1 while Q updates. Q now points to s1; P still points to
  retained s0. E6 therefore continues to see the unexplained newcomer,
  including after restart. An orphaned snapshot can be dropped, but s0
  is not orphaned while P references it.
- **R3D-15:** after a re-add/decision supersedes T, T remains metadata but
  its removed manifest is no longer a removal payload root. Retention
  comes only from another current/temporary root or local bounded history,
  not from the fact that T is a valid historical object. Pending Q-53
  concerns the currently removed state, not accumulating old removals.
- **R3D-16:** D1 keeps V2 and declines V3. Resume with no allow-again:
  inherited is `{V3}`, so the new declined set is `{V3}`. With no new
  release, ordinary selection can choose V2; with new V4 it can choose
  V4. Late V3 stays excluded. If V3 alone is eligible,
  `OnlyDeclinedVersions` holds the package for a visible choice.

## Actual verification and limits

Commands ran only in the assigned worktree. Cargo PATH was prefixed with
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

| Check | Result |
|---|---|
| `git status --short --branch`; `git rev-parse HEAD` | **PASS**: expected branch, clean starting tree and exact retry-2 merge above |
| `git show --stat 82b7d37`; bounded `git diff 82b7d37^1 82b7d37` and canonical reads | **PASS**: reviewed only removal/questions/retry evidence changes and their immediate rules |
| `rustc --version`; `cargo --version`; OS version | **PASS**: environment recorded above |
| `cargo xtask verify --tier offline` | **PASS**, exit 0: fmt, clippy, test, required-cases, deny and markdown-links; `overall: PASS (allow-blocked: false)` |

The author's retry-2 command table contains `VERIFY_RESULT`; that
placeholder was not accepted as test evidence. Only this worker's actual
offline result is recorded here.

Removal T6/T11c/T23/T31b/T34, multi-replica delivery, GUI interaction,
native/provider/macOS checks and negative controls are **NOT RUN** as
removal behavior: removal is not implemented. Offline verification checks
the current slice and repository hygiene, not execution of these design
traces. No production/test changes, imported Lua execution, game-file
access, process stopping or push. Only this evidence file is authored.
