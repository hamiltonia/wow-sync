# Work packet R2-fix: resolve R2-check findings

Role: implementation. Model: claude-sonnet-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

Resolve [R2-check](../evidence/R2-check.md) M1–M5 and m1–m5 within the
approved design ([domain.md](../../../design/domain.md) §1–§2.1,
[crates.md](../../../design/crates.md) §6, [tests.md](../../../design/tests.md)
§5b). R-02, R-03, A-08, A-25, A-43, A-46, E-08.

## Owned writes

`crates/wow-sync-core/**`, `crates/wow-sync-runtime/**`,
`crates/wow-sync-app/**`, `tests/required-cases.toml` (add cases in the
owning crate's block), evidence `R2-fix.md`. Not adapters, design or xtask.

## Work

1. M1: evaluate later folders of an unfamiliar enrolled variant by the
   consented mapping rule; a new folder must not invalidate enrollment.
   Product/layout change still requires reconfirmation, offered via the GUI.
2. M2: publication includes only transfer-eligible folders (not merely
   `FolderStatus::Ok`); blocked folders stay visibly "not synced".
3. M3: move the join-product decision and the "publish needed" comparison
   into core functions with core tests; runtime only calls them.
4. M4: show the disclosures domain.md §1 step 5 / §2.1 require before consent.
5. M5: ambiguity status shows each candidate's label and source computer.
6. m1–m5: the wording/rendering deviations listed in R2-check.
7. A required test for each M item that fails without the fix.

Do not measure memory (owner-review of the V1 incident pending).

## Done when

Offline verify and `cargo test --workspace` pass; evidence maps each finding
to change + test, with anything not resolved stated.
