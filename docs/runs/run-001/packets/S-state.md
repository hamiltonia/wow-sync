# Work packet S-state: addon-owned saved state identification

Role: design. Model: claude-opus-5.5 (PLAN §3). Common rules:
[S-common.md](S-common.md). ID `S-state`; spike dir `spikes/state/`.

## Intent

SPEC §7 "Safe identification of addon-only account/character saved state";
R-07. Can saved-variable files owned by an addon be identified safely without
executing Lua, and which cases are unknown/shared?

## Work

1. Document the WTF layout (account-wide `WTF\Account\<ACCOUNT>\SavedVariables\
   <Addon>.lua`, per-character `WTF\Account\<ACCOUNT>\<REALM>\<CHARACTER>\
   SavedVariables\<Addon>.lua`, `.bak` files, `config-cache.wtf`,
   `bindings-cache.wtf`, `macros-cache.txt`, `layout-local.txt`, chat cache,
   `Config.wtf`) from public knowledge and by listing **file names only** in
   the dev PC `_forever_\WTF` (D-14). Redact account/realm/character names.
   Do not read saved-variable contents beyond what step 3 needs, and never
   execute them.
2. Ownership mapping: file name ↔ addon folder ↔ `.toc` `## SavedVariables` /
   `## SavedVariablesPerCharacter` declarations. Identify cases: filename
   matches folder; addon declares variables but the file is named after a
   different folder (multi-folder packages); orphaned files for removed addons;
   Blizzard-owned `Blizzard_*.lua`; files with no matching addon.
3. Without executing Lua: is a tokenizer/parser that only reads top-level
   assignment names (`VarName = {`) enough to confirm which declared variables a
   file holds? Implement a minimal non-executing scanner in `spikes/state/` and
   test it on **synthetic fixtures** (`spikes/state/fixtures/`, including
   tricky cases: long strings, comments, escaped quotes, nested brackets,
   huge files). You may run it against the dev PC WTF files to count outcomes,
   committing only aggregate counts and redacted file names.
4. Classify: safely addon-owned / shared or Blizzard / unknown, and the
   consequences for sync scope, uninstall removal, and recovery (R-07, SPEC
   uninstall rules). Note client behaviors that matter (files written at
   logout/reload, `.bak` rotation) as needing S-proc-style timing evidence.

## Owned writes

`spikes/state/`, evidence files (S-common).

## Done when

Classification rules, scanner with fixture tests passing, aggregate dev-PC
outcome counts, and a list of unknown/shared cases needing owner or design
decisions.
