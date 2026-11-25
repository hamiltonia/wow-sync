# R11-app evidence

Branch `run011/R11-app`. Step 1 (inventory) only so far.

## §9.3 inventory: what the GUI already does

| §9.3 row | Existing |
|---|---|
| Settings sync page | `pages.rs::settings_sync`: enable panel with account checkboxes (none ticked by default), master panel and "Change master…", per-add-on exceptions, units list with Share/Use shared/Choose/Restore, per-folder Pause/Resume, banner (`settings_ui::banner_text`). |
| Add-on detail | `pages.rs` ~1251: restore controls per identity. No per-add-on settings status line yet. |
| Decision | `features.rs` maps `DecisionScope::Settings`; decisions UI exists. |
| Needs attention | no settings-specific items found beyond data-at-risk. |
| Activity | `history_ui.rs` groups `SettingsApplied`/`SettingsPublished` ("saved settings shared/updated") and the `SettingsMaster` reason; no identity on the detail row. |
| Computers | master shown; retire consequence exists. |
| Setup/consent | no §5.4 consent sheet; enable panel only. |

Gaps to build: guided offer on headline, §5.4 consent with exact text and
all accounts ticked by default, "use master's settings here" confirmation,
persisted "Not now", per-add-on settings status in the Add-ons list/detail,
identity on history detail, settings phase in progress/tray, release notes.

## Runtime interface needed

1. `AppSettings.settings_offer_dismissed: bool` (persisted; load/save in
   `control.rs`), settable through the existing `Cmd::SaveSettings`.
2. A status reason for a unit whose character folder is missing on this
   computer (e.g. `SettingsUnitStatus::WaitingForCharacter`).
3. Guided-offer state is derivable today from `SettingsSnapshot`
   (`enabled`, `master`, `master_name`, `available_accounts`); if
   R11-runtime adds a different query, tell the app its signature.

## Decisions made

- StartSettings enables consent before setting the policy.
- All accounts ticked by default on both computers (OD-11).
- Old enable panel kept only when there is no offer.
- Add-ons list line is generic; detail line names the identity.
- Not now persisted via settings_offer_dismissed.
- Folder match is case-insensitive.
- Retired computer or retired master gets no offer.

Verify: offline tier PASS (fmt, clippy, test, required-cases, deny, links). Native tier not run; live lab screenshots not done.

