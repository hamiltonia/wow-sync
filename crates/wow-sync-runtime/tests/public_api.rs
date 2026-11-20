//! The GUI crate names every command, outcome and refusal through `wow-sync-runtime` alone.
//! This compiles from outside the crate, so a type that is not exported fails the build.

use wow_sync_runtime::core::decision::{DecisionId, DecisionScope, EvidenceToken, OptionId};
use wow_sync_runtime::core::hash::sha256;
use wow_sync_runtime::core::ids::{ComputerId, EnrollmentId, Identity, SettingsUnit};
use wow_sync_runtime::core::kinds::{ChoiceReason, EnrollmentRef};
use wow_sync_runtime::core::paths::validate_folder_name;
use wow_sync_runtime::core::settings::ResumeOption;
use wow_sync_runtime::{
    Command, CommandError, CommandOutcome, RollbackRefusal, SettingsHistoryEntry,
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
    ];
    let mut names: Vec<_> = commands.iter().map(command_name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 17, "one of each command");

    assert_eq!(outcome_name(&CommandOutcome::Written(hash)), "written");
    assert_eq!(outcome_name(&CommandOutcome::Kept), "kept");

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
