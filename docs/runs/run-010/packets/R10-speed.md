# R10-speed: one scan per pass, no re-read after install, memory

Role: implementation / claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).
Goals 4–5 in [../PLAN.md](../PLAN.md).

**You own:** `crates/wow-sync-adapters` (`fs_scan.rs`, `hash_cache.rs`,
staging/install helpers), and the scan call sites in
`crates/wow-sync-runtime/src/service.rs`. Keep those `service.rs` edits small
and list them in your reply. R10-history owns the rest of `service.rs`.

Evidence:
- `v0.3.2` LENOVOFISH log lines (in PLAN):
  - `sync pass took 453437 ms: 77649 reused, 28958 read` (the apply pass);
  - `sync pass took 19420 ms: 108365 reused, 43 read`;
- RYZENFISH: `D:\Dropbox\Gaming Sync\wow-sync\logs\wowsync-RYZENFISH-*.log`,
  where passes take 21–110 s with 0 read.

## Work

1. **Baseline.** Build `v0.3.2` (tag) and measure an unchanged pass on the
   real `_forever_`, read-only, with an isolated `--state` and a synthetic
   provider: a warm cache, 3 runs, the median. Count how many times each file
   is stat'ed or visited per pass.
2. **Profile and fix:**
   - scan each install root at most once per pass and reuse the result across
     phases (target, removal, settings, publish);
   - drop redundant directory walks and stat calls;
   - keep the cache lookup O(1) per file.
3. **No re-read after install.** When files are installed from blobs whose
   hashes are known, write those hashes into the cache with the installed
   file's size and mtime. The next pass must read 0 of them.
4. **Correctness.** A file changed after install (size or mtime) is still
   re-hashed. "Check all files" still re-reads all. mtime is never ordering
   evidence.
5. **Memory.** Apply about 2 GB / 20k files of synthetic add-ons in a lab
   (the size distribution of Spoken audio: 15–200 KB .ogg files). Sample the
   private working set and private bytes. The peak private working set must be
   ≤ 100 MiB. Stream copies and hashes, and bound buffers and queues.
6. **Gate:** the unchanged pass on the real `_forever_` is ≥ 3× faster than
   the baseline. Report the before and after medians and the file visit
   counts.

Tests: one scan per pass (a counting fake fs), cache seeded after install (0
reads on the next pass), a changed file after install re-hashed, and the
bounded memory or buffer behaviour where testable.

Evidence: `docs/runs/run-010/evidence/R10-speed.md`.
