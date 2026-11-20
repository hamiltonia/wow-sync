//! Decision, retirement and cycle types exchanged between core, runtime and app
//! (contracts.md §9.1, ops.md §1.1). Types only; behaviour lives behind the §9.4 hooks.

use crate::explain::Explanation;
use crate::ids::{EnrollmentId, SettingsUnit, Sha256};
use crate::kinds::{EnrollmentRef, Retirement};
use crate::ledger::{Derived, Ledger, Membership};
use crate::package::PackageVersion;
use crate::paths::FolderKey;
use crate::records::SettingsRoot;
use crate::target::DecisionReason;
use crate::{authority::AuthorityRecord, ids::ComputerId, ids::GroupId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use crate::kinds::AckItem;

/// Why an addon's metadata cannot be used (the renamed `NoCompatibleToc`, contracts.md §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnusableKind {
    NoCompatibleToc,
    Unreadable,
    Unstable,
    Unsafe,
}

/// Every decision reason of contracts.md §5.1. The slice `DecisionReason` stays as it is and
/// converts into this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionKind {
    Ambiguous,
    UnusableMetadata(UnusableKind),
    DifferentTocMapping,
    UnvalidatedTocMapping,
    PartialCandidate,
    NewVersionAfterDecision,
    ChoicesDiffer,
    RollbackTargetsDiffer,
    RecoveredUnavailable,
    LookalikeOfRejected,
    NewerAfterRollback,
}

impl From<DecisionReason> for DecisionKind {
    fn from(r: DecisionReason) -> Self {
        match r {
            DecisionReason::Ambiguous => Self::Ambiguous,
            DecisionReason::NoCompatibleToc => {
                Self::UnusableMetadata(UnusableKind::NoCompatibleToc)
            }
            DecisionReason::DifferentTocMapping => Self::DifferentTocMapping,
            DecisionReason::UnvalidatedTocMapping => Self::UnvalidatedTocMapping,
            DecisionReason::PartialCandidate => Self::PartialCandidate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DecisionScope {
    Folders(Vec<FolderKey>),
    Settings(SettingsUnit),
}

/// The identity of a decision item is its scope; a package name is display only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DecisionId(pub DecisionScope);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DecisionReach {
    Group,
    ThisComputer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OptionId(pub u32);

/// Typed option labels; the app renders the words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionLabel {
    Release { version_label: String },
    KeepCurrent,
    AllowAgain,
    ChangeDecision,
    RemoveEverywhere,
    KeepOnThisComputer,
    UseSettingsFrom(EnrollmentId),
    KeepMySettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Consequence {
    pub installs: Vec<FolderKey>,
    pub removals: Vec<FolderKey>,
    pub settings_moves: Vec<SettingsUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionWarning {
    OlderThanInstalled,
    DropsFolders(Vec<FolderKey>),
    ReplacesSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionOption {
    pub id: OptionId,
    pub label: OptionLabel,
    pub holders: Vec<EnrollmentRef>,
    pub consequence: Consequence,
    pub warning: Option<OptionWarning>,
}

/// Hash of everything a decision item was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceToken(pub Sha256);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionItem {
    pub scope: DecisionScope,
    pub reason: DecisionKind,
    pub reach: DecisionReach,
    pub options: Vec<DecisionOption>,
    pub evidence: EvidenceToken,
}

/// Current evidence at write time: the items recomputed from the ledger, admitted states and
/// local inventory (contracts.md §5.5).
#[derive(Debug, Clone, Default)]
pub struct FreshEvidence {
    pub items: Vec<DecisionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalResolution {
    Dismiss(Sha256),
    KeepPaused(FolderKey),
    KeepOnThisComputer(DecisionScope),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionEffect {
    /// One authority record to write.
    Write(Box<AuthorityRecord>),
    /// A choice that only changes this computer.
    Local(LocalResolution),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecideError {
    UnknownItem,
    UnknownOption,
    /// Evidence changed between showing and choosing; nothing was written.
    Changed,
    NotAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecideOutcome {
    Effect(DecisionEffect),
    Refused(DecideError),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetirePlan {
    pub retirement: Option<Retirement>,
    pub reassign_master: Option<EnrollmentRef>,
    pub pause_own_publish: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    Keep,
    Rejection,
}

/// A live code choice or rejection for one folder (contracts.md §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub record: Sha256,
    pub kind: ConstraintKind,
    /// `None`: the constraint names the folder without a tree.
    pub tree: Option<Sha256>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemovalView {
    /// Valid removals not superseded for the folder.
    pub live: Vec<Sha256>,
    /// A pending removal names the folder: installs wait.
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupClass {
    Acknowledge,
    AbsentAcknowledge,
    WaitForCleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsTarget {
    NoChange,
    Adopt(Sha256),
    KeepLocal,
    Waiting,
    NeedsDecision(DecisionKind),
}

/// A code candidate: one complete package version and who holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub version: PackageVersion,
    pub holders: Vec<ComputerId>,
}

/// Everything a hook may read in one cycle. Hooks never perform effects.
pub struct CycleCx<'a> {
    pub group_id: &'a GroupId,
    pub own: &'a EnrollmentRef,
    pub ledger: &'a Ledger,
    pub derived: &'a Derived,
    pub members: &'a Membership,
    pub now_unix_ms: u64,
    pub held: &'a BTreeSet<FolderKey>,
    pub roots: &'a [SettingsRoot],
}

/// Display-only wrapper that carries the explanation next to a kind (used by `view`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explained {
    pub kind: DecisionKind,
    pub why: Explanation,
}
