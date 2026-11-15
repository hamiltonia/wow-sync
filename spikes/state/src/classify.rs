//! Ownership classification for saved-variable files (pure; no I/O).

use crate::scanner::{Scan, ScanError};
use crate::toc::AddonDecl;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// `WTF\SavedVariables` (client-wide, not account-specific).
    Machine,
    /// `WTF\Account\<ACCOUNT>\SavedVariables`.
    Account,
    /// `WTF\Account\<ACCOUNT>\<REALM>\<CHARACTER>\SavedVariables`.
    Character,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    Parse(ScanError),
    NonLiteral,
    DuplicateNames,
    /// File names a folder, but holds names that folder does not declare at all.
    UndeclaredNames(Vec<String>),
    /// Names declared by the folder, but for the other scope.
    WrongScope(Vec<String>),
    /// Matching folder declares no saved variables for any scope.
    NoDeclaration,
    /// No folder named after the file; its names are declared by these folders.
    DeclaredByOtherFolder(Vec<String>),
    /// Third-party name in the client-wide `WTF\SavedVariables`.
    ThirdPartyMachineScope,
    /// A `Blizzard_*` file whose name matches an installed (third-party) folder.
    BlizzardNameInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    /// File named after an installed folder; every top-level name is declared
    /// by that folder for this scope and by no other installed folder.
    Owned {
        folder: String,
    },
    /// As `Owned`, but the file holds no assignments.
    OwnedEmpty {
        folder: String,
    },
    /// Client-owned `Blizzard_*` state (excluded from sync by R-07).
    Blizzard,
    /// Owned by name, but a held global is also declared by other folders.
    Shared {
        folder: String,
        others: Vec<String>,
    },
    /// No installed folder matches and no installed folder declares its names.
    Orphan,
    Unknown(Reason),
}

impl Class {
    pub fn label(&self) -> &'static str {
        match self {
            Class::Owned { .. } => "owned",
            Class::OwnedEmpty { .. } => "owned-empty",
            Class::Blizzard => "blizzard",
            Class::Shared { .. } => "shared",
            Class::Orphan => "orphan",
            Class::Unknown(r) => match r {
                Reason::Parse(_) => "unknown:parse-error",
                Reason::NonLiteral => "unknown:non-literal",
                Reason::DuplicateNames => "unknown:duplicate-names",
                Reason::UndeclaredNames(_) => "unknown:undeclared-names",
                Reason::WrongScope(_) => "unknown:wrong-scope",
                Reason::NoDeclaration => "unknown:no-declaration",
                Reason::DeclaredByOtherFolder(_) => "unknown:declared-by-other-folder",
                Reason::ThirdPartyMachineScope => "unknown:third-party-machine-scope",
                Reason::BlizzardNameInstalled => "unknown:blizzard-name-installed",
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub class: Class,
    /// Some held names are declared only by some of the folder's flavor `.toc` files.
    pub flavor_dependent: bool,
    /// File stem differs in case from the matching folder name.
    pub case_mismatch: bool,
}

/// Installed addon folders keyed by lower-case folder name (Windows and the
/// client match folder names case-insensitively).
#[derive(Debug, Default, Clone)]
pub struct Inventory {
    pub folders: BTreeMap<String, (String, AddonDecl)>,
}

impl Inventory {
    pub fn insert(&mut self, folder: &str, decl: AddonDecl) {
        self.folders
            .insert(folder.to_ascii_lowercase(), (folder.to_string(), decl));
    }

    fn declarers(&self, name: &str) -> Vec<String> {
        self.folders
            .values()
            .filter(|(_, d)| d.union.account.contains(name) || d.union.character.contains(name))
            .map(|(f, _)| f.clone())
            .collect()
    }
}

/// `stem` is the file name without `.lua` / `.lua.bak`.
pub fn classify(
    inv: &Inventory,
    scope: Scope,
    stem: &str,
    scan: &Result<Scan, ScanError>,
) -> Outcome {
    let key = stem.to_ascii_lowercase();
    let entry = inv.folders.get(&key);
    let mut out = Outcome {
        class: Class::Orphan,
        flavor_dependent: false,
        case_mismatch: entry.is_some_and(|(f, _)| f != stem),
    };
    let unknown = |out: Outcome, r| Outcome {
        class: Class::Unknown(r),
        ..out
    };

    if key.starts_with("blizzard_") {
        let r = if entry.is_some() {
            Class::Unknown(Reason::BlizzardNameInstalled)
        } else {
            Class::Blizzard
        };
        return Outcome { class: r, ..out };
    }
    if scope == Scope::Machine {
        return unknown(out, Reason::ThirdPartyMachineScope);
    }
    let scan = match scan {
        Ok(s) => s,
        Err(e) => return unknown(out, Reason::Parse(e.clone())),
    };
    if !scan.non_literal.is_empty() {
        return unknown(out, Reason::NonLiteral);
    }
    if !scan.duplicates.is_empty() {
        return unknown(out, Reason::DuplicateNames);
    }

    let Some((folder, decl)) = entry else {
        let mut by: Vec<String> = scan.names.iter().flat_map(|n| inv.declarers(n)).collect();
        by.sort();
        by.dedup();
        if by.is_empty() {
            return out;
        }
        return unknown(out, Reason::DeclaredByOtherFolder(by));
    };
    let (mine, other, mine_all) = match scope {
        Scope::Character => (
            &decl.union.character,
            &decl.union.account,
            &decl.intersection.character,
        ),
        _ => (
            &decl.union.account,
            &decl.union.character,
            &decl.intersection.account,
        ),
    };
    if mine.is_empty() && other.is_empty() {
        return unknown(out, Reason::NoDeclaration);
    }
    let undeclared: Vec<String> = scan
        .names
        .iter()
        .filter(|n| !mine.contains(*n))
        .cloned()
        .collect();
    if !undeclared.is_empty() {
        let (wrong, none): (Vec<String>, Vec<String>) =
            undeclared.into_iter().partition(|n| other.contains(n));
        return unknown(
            out,
            if none.is_empty() {
                Reason::WrongScope(wrong)
            } else {
                Reason::UndeclaredNames(none)
            },
        );
    }
    out.flavor_dependent = scan.names.iter().any(|n| !mine_all.contains(n));
    let mut others: Vec<String> = scan
        .names
        .iter()
        .flat_map(|n| inv.declarers(n))
        .filter(|f| f.to_ascii_lowercase() != key)
        .collect();
    others.sort();
    others.dedup();
    out.class = if !others.is_empty() {
        Class::Shared {
            folder: folder.clone(),
            others,
        }
    } else if scan.names.is_empty() {
        Class::OwnedEmpty {
            folder: folder.clone(),
        }
    } else {
        Class::Owned {
            folder: folder.clone(),
        }
    };
    out
}
