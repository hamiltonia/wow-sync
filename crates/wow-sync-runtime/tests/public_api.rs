//! The GUI crate names every command, outcome and refusal through `wow-sync-runtime` alone.
//! This compiles from outside the crate, so a type that is not exported fails the build.

use wow_sync_runtime::core::decision::{DecisionId, DecisionScope, EvidenceToken, OptionId};
use wow_sync_runtime::core::diagnostics::ExportOptions;
use wow_sync_runtime::core::hash::sha256;
use wow_sync_runtime::core::ids::{ComputerId, EnrollmentId, Identity, SettingsUnit};
use wow_sync_runtime::core::kinds::{ChoiceReason, EnrollmentRef};
use wow_sync_runtime::core::paths::validate_folder_name;
use wow_sync_runtime::core::settings::ResumeOption;
use wow_sync_runtime::{
    Command, CommandError, CommandOutcome, ExportError, RollbackRefusal, SaveError,
    SettingsHistoryEntry, Started,
};

fn unit() -> SettingsUnit {
    SettingsUnit {
        folder: validate_folder_name("Alpha").unwrap(),
        identity: Identity::account("ACC").unwrap(),
    }
}

/// An exhaustive match: a new outcome variant must be named here too.
fn outcome_name(o: &CommandOutcome) -> &'static str {
    match o {
        CommandOutcome::Written(_) => "written",
        CommandOutcome::Kept => "kept",
        CommandOutcome::Local(_) => "local",
        CommandOutcome::ReEnrolled => "re-enrolled",
        CommandOutcome::Recorded => "recorded",
        CommandOutcome::NothingToDo => "nothing",
        CommandOutcome::UpdateCheck(_) => "update-check",
        CommandOutcome::Exported(_) => "exported",
    }
}

/// An exhaustive match over every command: a new variant must be named here too.
fn command_name(c: &Command) -> &'static str {
    match c {
        Command::Decide { .. } => "decide",
        Command::Retire { .. } => "retire",
        Command::AllowAgain { .. } => "allow-again",
        Command::RestoreRemoved { .. } => "restore-removed",
        Command::ReEnroll => "re-enroll",
        Command::Keep { .. } => "keep",
        Command::Resume { .. } => "resume",
        Command::SetSettingsPolicy { .. } => "policy",
        Command::ShareSettings { .. } => "share",
        Command::EnableSettings { .. } => "enable",
        Command::PauseSettings { .. } => "pause-settings",
        Command::ResumeSettings { .. } => "resume-settings",
        Command::UseSharedSettings { .. } => "use-shared",
        Command::ChooseSettings { .. } => "choose",
        Command::RestoreSettings { .. } => "restore-settings",
        Command::RestoreRemovedSettings { .. } => "restore-removed-settings",
        Command::Rollback { .. } => "rollback",
        Command::CheckUpdates => "check-updates",
        Command::ExportDiagnostics { .. } => "export-diagnostics",
        Command::SaveDiagnostics { .. } => "save-diagnostics",
    }
}

#[test]
fn every_command_can_be_built_and_every_refusal_named_from_outside_the_crate() {
    let folder = validate_folder_name("Alpha").unwrap();
    let hash = sha256(b"release");
    let me = EnrollmentRef {
        computer_id: ComputerId::parse(&"a".repeat(32)).unwrap(),
        enrollment_id: EnrollmentId::parse(&"b".repeat(32)).unwrap(),
    };
    let commands = vec![
        Command::Decide {
            item: DecisionId(DecisionScope::Folders(vec![folder.clone()])),
            option: OptionId(0),
            shown: EvidenceToken(hash),
        },
        Command::Retire {
            subject: me.clone(),
        },
        Command::AllowAgain {
            rejections: vec![hash],
        },
        Command::RestoreRemoved { removal: hash },
        Command::ReEnroll,
        Command::Keep {
            scope: vec![folder.clone()],
            copy: hash,
            why: ChoiceReason::Ambiguous,
        },
        Command::Resume {
            scope: vec![folder.clone()],
            allow_again: vec![],
            why: ChoiceReason::Ambiguous,
        },
        Command::SetSettingsPolicy {
            master: Some(me),
            exceptions: vec![],
        },
        Command::ShareSettings {
            unit: unit(),
            checkpoint: hash,
        },
        Command::EnableSettings {
            included_accounts: vec!["ACC".into()],
        },
        Command::PauseSettings {
            folder: folder.clone(),
        },
        Command::ResumeSettings {
            folder: folder.clone(),
            choice: ResumeOption::Resume,
        },
        Command::UseSharedSettings { unit: unit() },
        Command::ChooseSettings {
            unit: unit(),
            checkpoint: hash,
            shown_heads: vec![hash],
        },
        Command::RestoreSettings {
            unit: unit(),
            checkpoint: hash,
        },
        Command::RestoreRemovedSettings {
            removal: hash,
            unit: unit(),
        },
        Command::Rollback {
            package: folder,
            recovered: hash,
        },
        Command::CheckUpdates,
        Command::ExportDiagnostics {
            options: ExportOptions::default(),
        },
        Command::SaveDiagnostics {
            options: ExportOptions::default(),
            dest: std::path::PathBuf::from("diagnostics.txt"),
        },
    ];
    let mut names: Vec<_> = commands.iter().map(command_name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 20, "one of each command");

    assert_eq!(outcome_name(&CommandOutcome::Written(hash)), "written");
    assert_eq!(outcome_name(&CommandOutcome::Kept), "kept");
    assert_eq!(
        outcome_name(&CommandOutcome::UpdateCheck(Started::Debounced)),
        "update-check"
    );
    let refused_export = CommandError::Export(ExportError::Save(SaveError::ForbiddenLocation));
    assert_ne!(refused_export, CommandError::UpdatesUnavailable);

    let refused = CommandError::Rollback(RollbackRefusal::UnknownRelease(hash));
    assert_eq!(
        refused,
        CommandError::Rollback(RollbackRefusal::UnknownRelease(hash))
    );
    assert_ne!(
        refused,
        CommandError::Rollback(RollbackRefusal::NotAvailable)
    );
    let history: Vec<SettingsHistoryEntry> = Vec::new();
    assert!(history.is_empty());
}

#[test]
fn every_snapshot_field_can_be_named_and_read_from_outside_the_crate() {
    use std::collections::BTreeMap;
    use wow_sync_runtime::core::status::{AllHoldCause, HoldScope, RetiredInfo};
    use wow_sync_runtime::core::update::UpdateResult;
    use wow_sync_runtime::{
        HeldRelease, PeerRow, PeerStanding, RemovedRow, RetirementRow, StatusSnapshot,
    };
    use wow_sync_runtime::{PausedFolder, SettingsSnapshot};

    let st = StatusSnapshot::default();
    let peers: &Vec<PeerRow> = &st.peers;
    let rejected: &Vec<wow_sync_runtime::RejectedRelease> = &st.rejected_releases;
    let hashes: &Vec<wow_sync_runtime::core::ids::Sha256> = &st.removal_hashes;
    let older: &Option<String> = &st.older_peer;
    assert!(rejected.is_empty() && hashes.is_empty() && older.is_none());
    let settings: &SettingsSnapshot = &st.settings;
    let paused: &Vec<PausedFolder> = &settings.paused;
    assert!(!settings.enabled && paused.is_empty() && !settings.policy_conflict);
    assert!(peers.is_empty() && PeerStanding::Active != PeerStanding::Retired);
    let hold: &HoldScope = &st.hold;
    assert_eq!(hold, &HoldScope::None);
    let retired: &Option<RetiredInfo> = &st.this_retirement;
    assert!(retired.is_none());
    let rows: &Vec<RetirementRow> = &st.retirements;
    let held: &Vec<HeldRelease> = &st.held_releases;
    let removed: &Vec<RemovedRow> = &st.removed;
    let names: &BTreeMap<EnrollmentId, String> = &st.enrollment_names;
    let update: &Option<UpdateResult> = &st.update;
    assert!(rows.is_empty() && held.is_empty() && removed.is_empty() && names.is_empty());
    assert!(!st.review_needed && update.is_none());
    assert_ne!(
        HoldScope::AllApplies(AllHoldCause::Retired),
        HoldScope::AllApplies(AllHoldCause::Paused)
    );
}
