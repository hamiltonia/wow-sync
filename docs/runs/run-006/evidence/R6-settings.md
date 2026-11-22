# R6-settings evidence: H2 uninstall leaves declared settings

Intent: R-04, R-07, R-10; A-10, A-26. Worktree `D:\GitHub\wow-sync.worktrees\R6-settings`,
branch `run006/R6-settings` from main a33e0f3. Synthetic fixtures only.

## Cause

`SettingsPass::build` in `crates/wow-sync-runtime/src/removal_cycle.rs` returned
no scope unless this computer had settings-sync consent and a non-empty included
accounts list. Settings sync stays off until an owner picks a master (Q-70), so
on a group that never enabled it the removal pass saw no owned files. It
planned a code-only removal and left the add-on's declared saved file in place,
on both the originating and receiving computers. Ownership lookup, the
declaration book and the real-WTF scan were not at fault. The design does not
exclude this case: removal.md §6.2 lists only unowned/shared, paused and
over-cap exceptions, and says removal "does not need the settings master";
settings.md §8 says the app must move the owned files itself because WoW
keeps them (Q-45).

## Fix

When consent is absent or no accounts were chosen, the removal scope
searches every account folder on this computer (`available_accounts`), by
declared ownership only. Explicit consent with included accounts still bounds
the scope. Unowned, shared, paused and over-cap files, recovery checkpoints
and Lua non-execution are unchanged. This is an interpretation of R-04
("applicable saved settings") that the owner may wish to confirm.

## Tests

- New (failed before the fix, passes after):
  `e2e::tests::uninstall_removes_and_restores_settings_without_settings_sync`.
  Two peers, no settings policy, add-on-created files with differing bytes on
  both, plus an unowned file. It checks that the uninstall removes the owned file
  on the originating and the receiving computer and keeps the unowned file.
  A restore from Removed returns code and each peer's own settings
  byte-identical.
- Existing `e2e::tests::an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both` stays green.
- Required case `R6-settings:uninstall-removes-and-restores-settings`.

## Negative control

Mutation: the fallback account list was made empty. The new test failed at
"the originating computer" (the file was still present). Mutation reverted; test
passes.

## Commands

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --locked -p wow-sync-runtime uninstall_removes_and_restores
cargo xtask verify --tier offline
cargo xtask verify --tier native
```

| Check | Result |
|---|---|
| New test on unfixed main | FAIL (expected) |
| New test after fix | PASS |
| Control mutation | FAIL (expected), reverted |
| `verify --tier offline` | PASS |
| `verify --tier native` | PASS |
| Live-lab recheck (real WoW/Dropbox) | NOT RUN |
