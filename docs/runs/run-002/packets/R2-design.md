# Work packet R2-design: core design for the first vertical slice

Role: design. Model: claude-opus-5.5. Rules: [COMMON.md](COMMON.md).

## Intent

Produce the design that R2-core, R2-adapters and R2-slice implement, using
run-001 evidence. It must preserve SPEC intent; anything it cannot decide
within approved intent becomes an explicit owner question, not an assumption.
SPEC R-02, R-03, R-04, R-05, R-11, R-12; E-08 (single policy authority,
platform-free domain core).

## Inputs

SPEC.md, ACCEPTANCE.md, ENGINEERING.md, docs/ARCHITECTURE.md (P1-map
proposal), docs/REPO-MAP.md, run-001 evidence (S-addon, S-state, S-proc,
S-dropbox, S-gpui, S-mem, S-check, P3-review, P1-map open questions), the
uncommitted S-forever finding as summarized in run-001 REPORT §3 (`wowf`
product code from `.flavor.info`/`.build.info`/`.product.db`; do not read the
report file).

## Owned writes

- `docs/ARCHITECTURE.md` (update from proposal to slice design; keep its
  history note), new `docs/design/` files.
- Evidence file (summary + open questions).

## Work

1. Scope of the slice (PLAN §1) and what is explicitly out.
2. Crate/module layout: platform-free `wow-sync-core` (exists as skeleton)
   holding all policy; adapter crates/modules for Windows filesystem/process
   and Dropbox-folder provider; app shell. Interfaces (Rust trait sketches)
   between them so R2-core and R2-adapters can work in parallel.
3. Domain: computer/enrollment identity; variant identity derivation and
   validation (refuse on disagreement/unknown); install root; addon identity
   (case-folded folder name, package grouping by ids + hard deps, primary
   folder); version ordering (same-scheme strict only; ambiguous → needs
   addon-level decision); one `.toc` selection rule for `wowf` with stated
   uncertainty.
4. Publication format in the Dropbox folder for addon-code state: layout,
   manifest (versioned schema), content addressing/hashes, atomic publish
   (temp + rename), what "published" does and does not prove (S-dropbox),
   no Lua execution, size considerations (3.3 GB dataset).
5. Apply algorithm on a target computer: plan computation (pure, in core),
   safety gates (WoW running blocks game-file writes; idle updater does not),
   staging + atomic rename + re-check before each commit point, recovery
   point before change, residual launch race reported (PLAN §6.7).
6. Status/provenance records for R-12 (minimal for the slice).
7. Test strategy: required behavioral cases (list IDs/names) for R2-core and
   R2-adapters, synthetic fixture builder, end-to-end test with two synthetic
   installs and a temp "provider" folder; which use real Dropbox test folder.
8. Owner/design questions remaining, each with a recommended default.

Keep it implementable by sonnet workers in one pass each: concrete types,
file formats and function signatures, not essays.
