//! Closed set of structured explanations and their plain-English rendering (R-12).

use crate::ids::{ComputerId, Sha256};
use crate::package::FolderEvidence;

/// One competing version of an ambiguous package and who holds it (A-08, G5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousCandidate {
    pub label: String,
    /// Other computers holding exactly this content.
    pub sources: Vec<ComputerId>,
    /// This computer holds it too.
    pub here: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Explanation {
    NewerVersion {
        folder: String,
        from: String,
        to: String,
        source: ComputerId,
    },
    Installed {
        source: ComputerId,
    },
    AmbiguousVersions {
        evidence: Vec<FolderEvidence>,
        candidates: Vec<AmbiguousCandidate>,
    },
    PartialCandidate {
        missing: Vec<String>,
    },
    NoCompatibleToc {
        folder: String,
    },
    DifferentTocMapping {
        here: String,
        there: String,
    },
    UnvalidatedTocMapping {
        folder: String,
    },
    GameRunning,
    ActivityUnknown(String),
    Unstable,
    NotAvailableYet {
        missing: Vec<Sha256>,
    },
    TreeCorrupt {
        tree: Sha256,
    },
    MemberUnusable {
        computer: ComputerId,
        seq: u64,
        reason: String,
    },
    MemberRegressed {
        computer: ComputerId,
    },
    DestinationChanged {
        folder: String,
    },
    InterruptedByGame {
        package: String,
    },
    InterruptedByError {
        package: String,
        detail: String,
    },
    LiveMismatch {
        folder: String,
    },
    RecoveryMismatch {
        folder: String,
    },
    RestoreFailed {
        folder: String,
    },
    UnknownInterruptedState {
        package: String,
    },
    UnexpectedStaging {
        package: String,
    },
    StagedMismatch {
        folder: String,
    },
    StagingFailed {
        package: String,
        detail: String,
    },
    FinalizeFailed {
        package: String,
        detail: String,
    },
    Applied {
        package: String,
    },
    RolledBack {
        package: String,
    },
    AlreadyNewerHere,
    InSync,
    AppliedHere,
    /// Applied and verified here, with where it came from (G3).
    AppliedFrom {
        package: String,
        from: Option<String>,
        to: Option<String>,
        source: ComputerId,
    },
    /// Interrupted while the game was not running: the cause is not claimed.
    Interrupted {
        package: String,
    },
    ConfirmedBy {
        computers: Vec<ComputerId>,
    },
    Unsupported {
        folder: String,
        why: String,
    },
    GroupedBy {
        edges: Vec<String>,
    },
    /// The group removed this package; it is moved to recovery here, never reinstalled.
    RemovedByGroup,
    /// Nothing is installed or removed yet: removal history for it is still being learned.
    WaitingForGroupHistory,
}

fn another_computer(_: &ComputerId) -> String {
    "another computer".to_string()
}

/// Like [`render_named`] with no names known: a computer reads "another computer".
pub fn render(e: &Explanation) -> String {
    render_named(e, &another_computer)
}

/// One plain-English line. Peer clocks are never presented as "current". Computer IDs never
/// appear: a computer is named only through `name_of` (UG-22).
pub fn render_named(e: &Explanation, name_of: &dyn Fn(&ComputerId) -> String) -> String {
    use Explanation::*;
    match e {
        NewerVersion {
            folder,
            from,
            to,
            source,
        } => {
            format!(
                "{folder}: version {to} from {} is newer than {from} here",
                name_of(source)
            )
        }
        Installed { source } => format!("added from {}", name_of(source)),
        AmbiguousVersions {
            evidence,
            candidates,
        } => {
            let parts: Vec<String> = evidence
                .iter()
                .map(|x| {
                    format!(
                        "{} ({} vs {}: {})",
                        x.folder,
                        x.a_label.as_deref().unwrap_or("none"),
                        x.b_label.as_deref().unwrap_or("none"),
                        x.outcome
                    )
                })
                .collect();
            let held: Vec<String> = candidates
                .iter()
                .map(|c| {
                    let mut who: Vec<String> = Vec::new();
                    if c.here {
                        who.push("this computer".to_string());
                    }
                    who.extend(c.sources.iter().map(name_of));
                    format!("version {} on {}", c.label, who.join(" and "))
                })
                .collect();
            let mut s = format!("cannot tell which version is newer: {}", parts.join("; "));
            if !held.is_empty() {
                s.push_str(&format!(" [{}]", held.join("; ")));
            }
            s
        }
        PartialCandidate { missing } => format!(
            "the newer copy is incomplete; missing folders: {}",
            missing.join(", ")
        ),
        NoCompatibleToc { folder } => {
            format!("{folder} has no usable .toc file for this client, so it is not synced")
        }
        DifferentTocMapping { here, there } => {
            format!("the other computer reads .toc files differently ({there} vs {here})")
        }
        UnvalidatedTocMapping { folder } => format!(
            "{folder} has client-specific .toc files that are not validated for this client, so it is not synced"
        ),
        GameRunning => "waiting: World of Warcraft is running".to_string(),
        ActivityUnknown(why) => format!("waiting: cannot tell whether the game is running ({why})"),
        Unstable => "waiting: files are still changing".to_string(),
        NotAvailableYet { missing } => format!(
            "waiting for {} file(s) to arrive from the sync folder",
            missing.len()
        ),
        TreeCorrupt { tree } => format!(
            "the shared copy {} is damaged and was not used",
            tree.prefix()
        ),
        MemberUnusable {
            computer,
            seq,
            reason,
        } => {
            format!(
                "waiting for usable data from {} (record {seq}: {reason})",
                name_of(computer)
            )
        }
        MemberRegressed { computer } => format!(
            "data from {} went backwards and was not used",
            name_of(computer)
        ),
        DestinationChanged { folder } => {
            format!("{folder} changed while it was being prepared; nothing was replaced")
        }
        InterruptedByGame { package } => {
            format!("WoW started while {package} was being updated; it will be restored to its previous version after WoW exits")
        }
        InterruptedByError { package, detail } => format!("updating {package} stopped: {detail}"),
        LiveMismatch { folder } => {
            format!("{folder} does not contain the expected files; both copies were kept")
        }
        RecoveryMismatch { folder } => {
            format!("the saved previous copy of {folder} does not match; both copies were kept")
        }
        RestoreFailed { folder } => format!("restoring {folder} failed; it will be retried"),
        UnknownInterruptedState { package } => {
            format!("{package} was left in a state that cannot be classified; nothing was touched")
        }
        UnexpectedStaging { package } => {
            format!("unexpected leftover files while cleaning up {package}; kept for inspection")
        }
        StagedMismatch { folder } => {
            format!("the downloaded copy of {folder} did not verify; nothing was changed")
        }
        StagingFailed { package, detail } => format!("could not prepare {package}: {detail}"),
        FinalizeFailed { package, detail } => {
            format!("{package} was updated but cleanup is pending: {detail}")
        }
        Applied { package } => format!("{package} updated and verified"),
        RolledBack { package } => format!("{package} was restored to its previous version"),
        AlreadyNewerHere => "this computer already has the newest version".to_string(),
        InSync => "up to date with the versions this computer can see".to_string(),
        AppliedHere => "updated and verified on this computer".to_string(),
        AppliedFrom {
            package,
            from,
            to,
            source,
        } => {
            let change = match (from, to) {
                (Some(f), Some(t)) => format!("updated from {f} to {t}"),
                (None, Some(t)) => format!("installed version {t}"),
                _ => "installed".to_string(),
            };
            format!(
                "{package}: {change} from {} and verified on this computer",
                name_of(source)
            )
        }
        Interrupted { package } => format!(
            "updating {package} was interrupted; it will be finished or restored on the next sync"
        ),
        ConfirmedBy { computers } => {
            let names: Vec<String> = computers.iter().map(name_of).collect();
            format!("applied and reported by {}", names.join(", "))
        }
        Unsupported { folder, why } => format!("{folder} is not supported: {why}"),
        GroupedBy { edges } => format!("grouped because: {}", edges.join("; ")),
        RemovedByGroup => "removed by the group; it is kept in recovery here".to_string(),
        WaitingForGroupHistory => "waiting for the group's removal history".to_string(),
    }
}
