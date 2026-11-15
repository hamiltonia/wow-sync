# Work packet S-addon: addon/package identity and version ordering

Role: implementation. Model: claude-sonnet-5.5 (D-04). Common rules:
[S-common.md](S-common.md). ID `S-addon`; spike dir `spikes/addon/`.

## Intent

SPEC §7 "Reliable addon/package identity and version-ordering coverage with
real metadata"; R-05 (addon code chooses a demonstrably newer compatible
release; ambiguous ordering needs an addon-level decision, never timestamp
guessing). Dataset: dev PC `_forever_` AddOns (D-14, read-only here).

## Work

1. Small Rust tool in `spikes/addon/` that parses `.toc` files (including
   flavor-suffixed variants like `Name_Vanilla.toc`, `Name-Classic.toc`,
   `Name_Mainline.toc` and the client's selection rules if documented) without
   executing anything. Extract: `## Interface` (may be comma lists),
   `## Version`, `## Title`, `## X-*` fields (e.g. `X-Curse-Project-ID`,
   `X-WoWI-ID`, `X-Wago-ID`), `## Dependencies`/`RequiredDeps`/`OptionalDeps`,
   `## LoadOnDemand`, `## SavedVariables*`, `## Author`, packager tokens like
   `@project-version@`.
2. Package grouping: identify multi-folder packages (e.g. DBM-*, ElvUI_*)
   via shared ids, `## X-*` fields, dependencies or naming; report how many
   folders cannot be grouped confidently.
3. Version ordering: classify each version string (semver-like, date, git
   describe `v1.2.3-4-gabc`, alpha/beta suffixes, unsubstituted tokens,
   missing). For each scheme state whether two versions of the same addon can be
   ordered demonstrably, and list ambiguous cases. Include synthetic fixtures
   under `spikes/addon/fixtures/` (synthetic `.toc` only) for every scheme and
   unit tests for the ordering rules.
4. Coverage report: counts per scheme, % of folders with a reliable identity,
   % orderable; an ambiguity catalog. Committed output may contain only folder
   names, ids, versions and interface numbers (D-14 derived metadata) — no
   file contents beyond those fields.
5. Note which `## Interface` values appear and whether they reveal the client
   flavor (feeds S-forever, but do not do S-forever's work).

## Owned writes

`spikes/addon/`, evidence files (S-common).

## Done when

Coverage report and ambiguity catalog committed, tests for the ordering rules
pass, recommendation on identity key and ordering policy.
