# R9-app: click latency, honest headline, logging, hash cache, version skew

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goals 3–8 in [../PLAN.md](../PLAN.md).

**You own:** `crates/wow-sync-app`, `crates/wow-sync-adapters` (including
`fs_scan.rs`), the state-format additions for the cache and member version,
the version bump to 0.3.2, `docs/release/v0.3.2.md`, and user docs whose
behaviour text changes. **You do not own** `wow-sync-core`
decide/choice/target (R9-decide has them). Small read-only uses of core types
are fine. If you need a core change, describe it and ask the orchestrator.

Evidence to read first:
- `D:\Dropbox\Gaming Sync\wow-sync\logs\wowsync-RYZENFISH-20261123-173413.log`;
- `backend.rs` `run()` (~551–790);
- `model.rs` `waiting_sentence` (~216–245);
- `target.rs` Unavailable (~440, ~985);
- `fs_safe.rs` online-only detection (14–62).

## Goals

1. **Hash cache (goal 6) — do this first; it removes most of the lag.**
   - Cache `(relative path, size, mtime) → sha256` per install root in the
     state dir. Re-hash only new or changed files. Drop entries for deleted
     files, and bound the cache size.
   - Use it in sync passes and the setup check.
   - Add an optional "Check all files" action (for example on the add-ons
     page or in Settings) that re-hashes everything once.
   - Modification time is never ordering evidence.
   - Measure first and second pass times on the real `_forever_` (read-only,
     isolated state) and on a synthetic tree. Check the 100 MiB background
     memory limit.
2. **Clicks respond at once (goal 3).**
   - User commands are handled within about 1 s while a pass runs: poll the
     command channel between packages or steps, or make the pass
     interruptible and resumable. Quit-when-safe semantics are unchanged.
   - Show immediate pending feedback for Decide, Pause/Resume and Export.
   - One Pause click is one toggle: disable it or show it pending until
     applied, and ignore repeats meanwhile.
   - Test with a slow fake scan.
3. **Honest headline (goal 4).**
   - Unavailable names the add-on, and the computer when known.
   - Only true cloud-only or partly downloaded blobs say Dropbox is
     downloading.
   - Escalate after a timeout (pick and document one, for example 30 min)
     to "<computer> hasn't shared <add-on>", with a next step such as "open
     WoW Sync on <computer> and update it".
   - A pending decision must not be hidden.
   - Add a test that other packages still apply while one is Unavailable
     (runtime e2e if needed; coordinate if it touches core).
4. **Log results (goal 5).** Log the result of every `Cmd::Group`: Decide
   (accepted or refused, the reason, the record hash), Pause/Resume, Export
   preview and save (ok or error), and others. No profile paths, names or
   contents; use `~`.
5. **Version skew (goal 7).**
   - Publish the app version in `member.json` or state, additively and
     backward compatibly.
   - Show a peer on a different version on its computer page and in the
     diagnostics.
   - Note: "LENOVOFISH runs 0.3.1; update it to match."
6. **Goal 2 hook.** R9-decide may report an app-side call needed so a second
   Decide sees this computer's own fresh records. The orchestrator will
   relay it; wire it when asked.
7. **Release.** Bump the version to 0.3.2. Write `docs/release/v0.3.2.md`
   covering all run-009 fixes, plus "update every computer".

## Done

- Both verify tiers pass; no Windows-only paths in tests. Add a
  required-cases section.
- Evidence: timings before and after, latency measurements, headline
  screenshots or UIA text, log samples, and the A-38 controls.
- Commit on `run009/R9-app`, then reply with the SHA and a summary.
