# R4-mac-kit: phase D (Mac build, run and memory kit plus owner steps)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-mac-kit`, on branch
  `run004/R4-mac-kit`, created from `main` after the R4-integrate snapshot
  merge (`fba5b63`).
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** R4-app owns `crates\wow-sync-app` and is running. You
  own no crate file.

## Why

D-23 records that the owner has a MacBook Pro (Apple Silicon) for real
runtime testing: the macOS runtime, native notifications (Q-63, T-27-11),
the T-28-13 live update check on macOS, and A-29 memory on macOS. This
Windows machine has no C toolchain for macOS targets, so nothing macOS can
run here. You produce a kit the owner runs, and the exact instructions.

## Authority

- [ops.md](../../../design/ops.md) §3 (notifications), §4 (update check), the
  §7 rows T-27-11 and T-28-13 (macOS halves).
- A-29 in [ACCEPTANCE.md](../../../../ACCEPTANCE.md) and the AGENTS.md rule:
  background memory is at most 100 MiB aggregate private resident memory;
  sync starts within ten seconds of readiness.
- The Windows memory harness [tools\mem\run.ps1](../../../../tools/mem/run.ps1)
  and `MemLib.ps1`: mirror its scenarios and its synthetic-data-only rule.
- [docs\VERIFY.md](../../../VERIFY.md) (toolchain needs, including the macOS
  Command Line Tools).
- D-07 / D-23 (Mac hardware), D-24 P4-1 (one anonymous live GET).

## Files you own

- New `tools\mac\` directory: shell scripts (POSIX `sh` or `zsh`, as macOS
  ships them) for:
  1. **Build:** check prerequisites (Xcode Command Line Tools, rustup with
     the pinned toolchain), then `cargo xtask verify --tier offline` and a
     release build of the app and devkit.
  2. **Run:** start the app against a synthetic lab under a neutral lab
     root (never real WoW, Battle.net or Dropbox data), using devkit as the
     Windows harness does.
  3. **Memory:** the A-29 scenarios of `tools\mem\run.ps1` on macOS. Measure
     private resident memory with a documented macOS method (for example
     `footprint` or `vmmap --summary` physical footprint), aggregated across
     every WoW Sync process, and the sync-start time. Write a results file
     with the environment.
  4. **Notification check (T-27-11, macOS):** trigger one native
     notification through the existing runtime path and ask the owner to
     confirm the app name shown.
- `docs\runs\run-004\MAC-OWNER.md`: step-by-step owner instructions (what to
  install, the exact commands with the checkout path, what to look at, what
  to send back, how to clean up), written for a non-developer.
- Your section of `tests\required-cases.toml` only if you add a test that
  runs on this machine (you probably add none).
- Evidence: `docs\runs\run-004\evidence\R4-mac-kit.md`.

Don't touch any crate, `Cargo.lock`, CI workflow or Windows tool. If the app
or devkit cannot run on macOS as written (for example a Windows-only code
path with no macOS counterpart), don't fix it: list the exact file, line
and gap under **Handover** in your evidence and design the kit around it
where possible.

## Work

1. Read the app and devkit entry points and the runtime's platform code to
   find what actually runs on macOS. Record it.
2. Write the scripts. Validate what you can here: syntax-check each script
   (for example `bash -n` through Git Bash if available) and review it
   against the Windows harness. Record exactly what was and was not
   executed. Every macOS row is NOT RUN until the owner runs it.
3. Write `MAC-OWNER.md`.

## Gate and evidence

- Offline verify passes on the final tree (your files must not break
  markdown-links).
- `git diff --name-only main..HEAD` lists only your files and evidence.
- **Evidence:** what runs on macOS (with file references), each script and
  how it was checked, the rows the owner will produce (T-27-11 macOS, T-28-13
  macOS, A-29 macOS) marked NOT RUN, and the handover.
- **Commits:** prefix `run004(R4-mac-kit):`, with the Copilot trailer. Commit
  per work item. Don't push. Edit and create files only with the
  edit/create tools. Never use `git checkout --`/`restore`/`reset --hard`/
  `stash` on uncommitted work.
