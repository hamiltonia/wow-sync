//! Settings behaviour (settings.md). Step-1 stub: the neutral hooks, handed to R4-core-settings.

use crate::decision::{CycleCx, SettingsTarget};
use crate::hooks::{Neutral, SettingsHooks};
use crate::ids::SettingsUnit;
use crate::paths::FolderKey;
use crate::plan::PackageApply;
use crate::records::SettingsRoot;

#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsBehaviour;

impl SettingsHooks for SettingsBehaviour {
    fn settings_target(&self, cx: &CycleCx, u: &SettingsUnit) -> SettingsTarget {
        Neutral.settings_target(cx, u)
    }
    fn plan_settings(&self, cx: &CycleCx) -> Vec<PackageApply> {
        Neutral.plan_settings(cx)
    }
    fn roots_for(&self, cx: &CycleCx, folders: &[FolderKey]) -> Vec<SettingsRoot> {
        Neutral.roots_for(cx, folders)
    }
}
