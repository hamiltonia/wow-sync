# R11-review: settings sync safety review

Role: review, claude-opus-5.5. Read [COMMON.md](COMMON.md). Read-only
except `docs/runs/run-011/evidence/R11-review.md` and probes you revert.

Review `main` after the R11-runtime and R11-app merges against SPEC R-07 to
R-10, A-11 to A-17, A-21, A-24, A-25, and design/settings.md. Focus:

- **Data safety on a real `WTF`:** every write path (shape, existing
  folders only, temp + rename, recovery copy first, WoW-running gate,
  cap, shared ownership); no write outside the two §1.1 shapes; no Lua
  executed; nothing lost on a crash mid-apply.
- **Authority:** no automatic master election; non-master routine changes
  yield with recovery; pushes and competing choices never merge or drop.
- **Consent:** nothing read for sharing or written before consent on that
  computer; account list truthful; "Not now" honoured.
- **Privacy:** logs, history, diagnostics and evidence hold no account,
  realm or character names, absolute paths or file contents where the
  rules say they must not.
- **Visibility:** statuses and history are truthful (no "in sync" while
  anything is waiting, paused, or unconfirmed).

Report findings as RV11-NN with severity (critical, major, minor, note),
evidence and a suggested fix. Confirm each critical or major with a probe
or test. Commit the evidence; reply with SHA and verdict.
