# R10-speed evidence (run 010, goals 4-5)

Branch `run010/R10-speed`. Environment: Windows, this development machine, release builds, OS file
cache warm. Lab harnesses: `crates/wow-sync-runtime/examples/speed_lab.rs` and `mem_lab.rs`.

## Idle pass speed (unchanged pass, real `D:\Games\World of Warcraft\_forever_`)

Method: `speed_lab` enrols a synthetic group (isolated `--state`, synthetic provider directory) on
the real install, then times repeated `sync_once` passes with a warm hash cache. The install's
add-on files are only read. Enrolment creates `.wowsync` work directories inside `_forever_`
(already present from the owner's own client); nothing was published to any real group.
Baseline: the v0.3.2 tag built in a temporary worktree with the same visit counters patched in
(removed afterwards).

| | v0.3.2 | after | ratio |
|---|---|---|---|
| warm pass wall time, median | 945 ms (938, 942, 949, 984) | 355 ms (265-455 over 15 passes, noisy) | **2.7x** |
| files hash-cache-checked per pass | 108,426 (3 scans of 36,142) | 36,142 (1 scan) | 3.0x fewer |
| directory listings per pass | 9,864 | 2,810 | 3.5x fewer |
| directory entries listed | 259,842 | 72,189 | 3.6x fewer |
| stat calls | 4,513 | 1,438 | 3.1x fewer |
| files re-read from disk | 0 | 0 | |

Gate (>= 3x faster): **visit counts meet it (3.0-3.6x); wall time on this warm-cache machine is
2.7x, not 3x.** Remaining per-pass cost (about 330 ms): first scan 95 ms, `build_inventory`
about 45 ms per observation (two observations), recovery-point verification walk in the status
snapshot about 70 ms (`recovery_rows`, a metadata walk of the recovery copies), the rest provider
reads and derivation. Not met strictly; see open issues in the reply.

Changes that produce it:
- A pass scanned the install three times (observe, re-observe, last look). A pass that applied
  nothing now reuses a settled scan younger than 10 s for the re-observe and skips the last look
  when nothing was published (`quiet_cycle`).
- A folder whose files all came from the hash cache and that took under 250 ms takes one directory
  snapshot instead of two.
- Decoded tree records are reused for 10 s (content addressed; only `Ok` results are kept).
- After an install the staged size, mtime and verified hash are carried into the live folder and
  seeded into the hash cache, and the post-install hash passes (`hash_slot` of staging and live)
  no longer re-read the files just written. The post-apply scan therefore reads 0 installed
  files. Size or mtime changes still miss; "Check all files" still forces full reads.

## Memory (apply of about 2 GB / 20,000 files)

Method: `mem_lab gen` made a synthetic install of 20,000 files, 2,094 MiB (100 add-ons, 15-200 KiB
pseudo-random files). Computer A published it to a synthetic provider; computer B (separate
process, empty synthetic install, isolated state) joined the group and applied it in one pass.
B's samples every 400 ms (566 samples): peak private working set **36.4 MiB**, peak private
bytes **39.9 MiB**, peak working set 42.5 MiB (limit 100 MiB). B ended with 20,000 files /
2,094 MiB, and the next pass took 150 ms. A's publish peak private bytes: 33 MiB. The apply pass
took 562 s here (sampling and Defender included); not compared with v0.3.2.

## Tests and verification

- New: `fs_install::tests::an_installed_folder_is_not_read_again_by_the_next_scan_unless_it_changed`
  (cache seeded after install: next scan 3 hits, 0 reads; an edited file is re-read; "Check all
  files" re-reads everything). Registered in `tests/required-cases.toml`
  (`R10-speed:idle-pass-hashing`).
- `cargo xtask verify --tier offline`: PASS. `cargo xtask verify --tier native`: PASS.
