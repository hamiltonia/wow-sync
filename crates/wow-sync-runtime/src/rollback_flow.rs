//! The settings side of a rollback (rollback.md §9, R4D-27): the rejection's `settings` field
//! describes what the rollback actually restored, not what the recovery point expected.

#![allow(dead_code)] // called by the rollback command in a later step; unit tests exercise it

use std::collections::BTreeSet;

use wow_sync_core::ids::SettingsUnit;
use wow_sync_core::kinds::RejectionSettings;
use wow_sync_core::records::SettingsRoot;

/// `roots` are the recovery point's settings roots; `restored` the units whose files were really
/// restored by this rollback. A root that was captured but not restored counts as not restored.
pub fn actual_settings_result(
    roots: &[SettingsRoot],
    restored: &BTreeSet<SettingsUnit>,
) -> RejectionSettings {
    if roots.is_empty() {
        return RejectionSettings::NotApplicable;
    }
    let done = roots
        .iter()
        .filter(|r| r.captured() && restored.contains(&r.unit))
        .count();
    if done == roots.len() {
        RejectionSettings::Paired
    } else if done == 0 {
        RejectionSettings::CodeOnly
    } else {
        RejectionSettings::Partial
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::Identity;
    use wow_sync_core::paths::validate_folder_name;
    use wow_sync_core::records::{Location, MissingReason, RootState};

    fn unit(f: &str) -> SettingsUnit {
        SettingsUnit {
            folder: validate_folder_name(f).unwrap(),
            identity: Identity::account("ERIC").unwrap(),
        }
    }

    fn captured(f: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(f),
            state: RootState::Captured {
                checkpoint: sha256(f.as_bytes()),
                location: Location::Local,
            },
        }
    }

    fn missing(f: &str) -> SettingsRoot {
        SettingsRoot {
            unit: unit(f),
            state: RootState::Missing {
                reason: MissingReason::Paused,
            },
        }
    }

    #[test]
    fn the_result_reports_what_was_restored_not_what_was_expected() {
        let all: BTreeSet<SettingsUnit> = [unit("a"), unit("b")].into();
        let roots = vec![captured("a"), captured("b")];
        assert_eq!(
            actual_settings_result(&roots, &all),
            RejectionSettings::Paired
        );
        // both were expected, only one was restored
        let one: BTreeSet<SettingsUnit> = [unit("a")].into();
        assert_eq!(
            actual_settings_result(&roots, &one),
            RejectionSettings::Partial
        );
        assert_eq!(
            actual_settings_result(&roots, &BTreeSet::new()),
            RejectionSettings::CodeOnly
        );
        // a missing root is never counted, even if its unit is in the restored set
        let mixed = vec![captured("a"), missing("b")];
        assert_eq!(
            actual_settings_result(&mixed, &all),
            RejectionSettings::Partial
        );
        assert_eq!(
            actual_settings_result(&[], &all),
            RejectionSettings::NotApplicable
        );
    }
}
