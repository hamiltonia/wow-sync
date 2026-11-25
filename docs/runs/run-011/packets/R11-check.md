# R11-check: independent verification of v0.4.0

Role: verification, gpt-6.1-sol. Read [COMMON.md](COMMON.md). Read-only
except `docs/runs/run-011/evidence/R11-check.md` and your lab.

Use the release dispatch artifact for the candidate SHA the orchestrator
gives you, not a local build. Build your own two-computer lab from the
dev data per COMMON (synthetic account and character names), separate
`--state` dirs, one synthetic provider root.

Check every R11-check gate in [../PLAN.md](../PLAN.md) through the GUI
where the user would, and report each as PASS, FAIL, BLOCKED or NOT RUN
with the evidence (steps, observed text, screenshots without real names,
file hashes before and after). Include:

- the artifact's SHA-256 matches the `.sha256`;
- a write audit: list every file changed under the lab roots during the
  run and confirm all are inside the lab `WTF` SavedVariables shapes, the
  state dirs or the provider root;
- memory (RV11-16, OD-40): peak private working set during capture and
  apply, with settings on. Use the full lab (the 113-add-on, ~36k-file
  dev-data copy) and do at least three separate full runs, each starting
  from a fresh process. Report every run's idle and pass peak; the gate
  value is the highest peak seen, not a median. Any sample above 100 MiB
  is a FAIL;
- add-on code sync with a `v0.3.3` peer (the code publish path changed in
  `ea798cd`): the candidate publishes an update, a new add-on and a
  removal, and a `v0.3.3` computer in the same lab group applies each one;
  then the reverse direction. Record the file hashes on both sides.

Do not fix code. Commit evidence; reply with SHA and the gate table.
