# Work packet S-forever: identity of the `_forever_` installation

Role: implementation. Model: claude-sonnet-5.5 (D-04); checked later by
gpt-6.1-sol (S-check). Common rules: [S-common.md](S-common.md). ID
`S-forever`; spike dir `spikes/forever/` (optional).

## Intent

SPEC §7 "Authoritative variant/install identity, including verification of
'Forever'". AGENTS.md: "Forever" is not a synonym for Classic Era; verify its
identity from installation metadata only (D-09; D-14 allows `_forever_`
AddOns/WTF reads, not needed here).

## Work

1. Read-only metadata under `D:\Games\World of Warcraft`: `.build.info`,
   `.flavor.info` (in variant dirs), `.product.db` / `.patch.result` if
   present, `Launcher.db`, executable names and file version resources
   (product name, version) of client executables in `_forever_`. Read file
   **metadata and these small config files only**; do not read Data/ archives.
2. Determine: product code (e.g. `wow`, `wow_classic`, `wow_classic_era`, or
   something new), branch/region, build number, client version, flavor, the
   exe name, and how a WoW Sync client would derive a stable variant identity.
   Compare with public sources (cite URLs) on Blizzard product codes. State
   plainly whether "Forever" equals any known variant or is distinct, with
   confidence.
3. Also record what `## Interface` numbers the installed addons target, only as
   corroboration (count by value).
4. Sanitize: no account names, emails, Battle.net ids, tokens, region-specific
   personal data, or user paths beyond the install root.

## Owner review before commit (PLAN §7)

**Do not commit the evidence report.** Write it to
`docs/runs/run-001/evidence/S-forever.md` in your worktree and leave it
**uncommitted**. You may commit spike code (if any) only. Your final message
must say the report is ready for owner review and list exactly which raw
values it contains.

## Owned writes

`spikes/forever/` (optional), the uncommitted evidence file.
