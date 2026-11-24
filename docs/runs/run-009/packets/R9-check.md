# R9-check: independent verification of v0.3.2

Role: verification / gpt-6.1-sol. Rules: [COMMON.md](COMMON.md). Write only
your evidence and, if needed, your own required-cases section. Do not change
product code. Report findings G1, G2, … with a severity and reproduction
steps.

Build under test: the `main` SHA named by the orchestrator, plus the dispatch
artifact (check its sha256, unzip it, and run `wow-sync.exe`).

## Checks

1. **Conflicting choices.** Using two synthetic computers (isolated
   `--state` dirs, a synthetic or `WoW Sync Test` provider), create two
   conflicting keeps from one issuer, as in the R9-decide fixture or by the
   GUI on a `v0.3.1` build. On `v0.3.2`, the decision page offers the copies.
   Choosing one converges both computers and clears the decision.
2. **No stale second choice.** Click two different copies quickly, and also
   double-click one. Exactly one live choice results.
3. **Latency.** During a pass on a large synthetic tree, Decide, Pause and
   Export → Preview each respond (visible feedback) within about 1 s and are
   applied. One Pause click is one toggle.
4. **Headline.** A peer state references a tree that is missing from the
   provider:
   - the headline names the add-on (and computer), not Dropbox downloading;
   - it escalates after the documented timeout (use a test override if
     provided);
   - other add-ons still apply;
   - a pending decision stays visible.
5. **Logs.** Click results appear, with no profile paths or names.
6. **Hash cache.**
   - Second pass time vs first on a synthetic tree (≥5×) and a read-only real
     `_forever_` scan.
   - Edit one file, keeping its size, with mtime changed: it is re-hashed.
   - "Check all files" re-reads everything.
   - The cache survives a restart.
   - Background memory ≤ 100 MiB.
7. **Version skew.** A peer on 0.3.1 is shown as a different version.
8. **Regression.** Both verify tiers pass. Smoke-test setup (discovery,
   picker, cancellable check) and Quit.
9. **Artifact smoke test.**

Write `docs/runs/run-009/evidence/R9-check.md` with PASS / FAIL / BLOCKED /
NOT RUN for each check and the environment. Clean up the lab, commit, and
reply with the SHA, a table and the findings.
