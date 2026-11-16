//! Folder-name and relative-path validation.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    Empty,
    NonAscii,
    ForbiddenChar(char),
    Reserved,
    TrailingDotOrSpace,
    DotSegment,
    TooLong,
    Absolute,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::Empty => write!(f, "empty name"),
            PathError::NonAscii => write!(f, "name has non-ASCII characters"),
            PathError::ForbiddenChar(c) => write!(f, "name has forbidden character {c:?}"),
            PathError::Reserved => write!(f, "name is a reserved device name"),
            PathError::TrailingDotOrSpace => {
                write!(f, "name starts/ends with a space or ends with a dot")
            }
            PathError::DotSegment => write!(f, "path has a dot segment"),
            PathError::TooLong => write!(f, "name or path is too long"),
            PathError::Absolute => write!(f, "path is absolute"),
        }
    }
}

const MAX_COMPONENT: usize = 255;
const MAX_REL_PATH: usize = 400;

/// Lowercased validated folder name (or, for unsupported folders, the lowercased raw name).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FolderKey(String);

impl FolderKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Key for a name that failed validation; never used to address files.
    pub(crate) fn lossy(name: &str) -> Self {
        Self(name.to_lowercase())
    }
}

impl fmt::Display for FolderKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for FolderKey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for FolderKey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s.is_empty()
            || s.len() > MAX_COMPONENT
            || s.chars().any(char::is_control)
            || s != s.to_lowercase()
        {
            return Err(serde::de::Error::custom("invalid folder key"));
        }
        Ok(Self(s))
    }
}

/// Validated `/`-separated path relative to a folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelPath(String);

impl RelPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for RelPath {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RelPath {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        validate_rel_path(&s).map_err(serde::de::Error::custom)
    }
}

fn is_reserved(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn validate_component(name: &str) -> Result<(), PathError> {
    if name.is_empty() {
        return Err(PathError::Empty);
    }
    if name == "." || name == ".." {
        return Err(PathError::DotSegment);
    }
    for c in name.chars() {
        if !c.is_ascii() {
            return Err(PathError::NonAscii);
        }
        if (c as u32) < 0x20 || c as u32 == 0x7f || "<>:\"/\\|?*".contains(c) {
            return Err(PathError::ForbiddenChar(c));
        }
    }
    if name.len() > MAX_COMPONENT {
        return Err(PathError::TooLong);
    }
    if name.starts_with(' ') || name.ends_with(' ') || name.ends_with('.') {
        return Err(PathError::TrailingDotOrSpace);
    }
    if is_reserved(name) {
        return Err(PathError::Reserved);
    }
    Ok(())
}

pub fn validate_folder_name(name: &str) -> Result<FolderKey, PathError> {
    validate_component(name)?;
    Ok(FolderKey(name.to_lowercase()))
}

pub fn validate_rel_path(path: &str) -> Result<RelPath, PathError> {
    if path.is_empty() {
        return Err(PathError::Empty);
    }
    let b = path.as_bytes();
    if path.starts_with('/')
        || path.starts_with('\\')
        || (b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic())
    {
        return Err(PathError::Absolute);
    }
    if path.len() > MAX_REL_PATH {
        return Err(PathError::TooLong);
    }
    for component in path.split('/') {
        validate_component(component)?;
    }
    Ok(RelPath(path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_and_absolute_paths_rejected() {
        assert_eq!(validate_rel_path("../x"), Err(PathError::DotSegment));
        assert_eq!(validate_rel_path("a/../b"), Err(PathError::DotSegment));
        assert_eq!(validate_rel_path("./a"), Err(PathError::DotSegment));
        assert_eq!(validate_rel_path("/etc/x"), Err(PathError::Absolute));
        assert_eq!(validate_rel_path("\\x"), Err(PathError::Absolute));
        assert_eq!(validate_rel_path("C:/x"), Err(PathError::Absolute));
        assert_eq!(
            validate_rel_path("a\\b"),
            Err(PathError::ForbiddenChar('\\'))
        );
        assert_eq!(validate_rel_path("a//b"), Err(PathError::Empty));
        assert_eq!(validate_folder_name(".."), Err(PathError::DotSegment));
        assert!(validate_rel_path("Sub/file.lua").is_ok());
    }

    #[test]
    fn reserved_device_names_rejected() {
        for n in ["CON", "nul", "Aux.txt", "COM1", "lpt9.lua", "PRN"] {
            assert_eq!(validate_folder_name(n), Err(PathError::Reserved), "{n}");
        }
        assert!(validate_folder_name("COM10").is_ok());
        assert!(validate_folder_name("CONFIG").is_ok());
        assert_eq!(validate_rel_path("a/NUL.lua"), Err(PathError::Reserved));
    }

    #[test]
    fn trailing_dot_space_and_control_rejected() {
        assert_eq!(
            validate_folder_name("Addon."),
            Err(PathError::TrailingDotOrSpace)
        );
        assert_eq!(
            validate_folder_name("Addon "),
            Err(PathError::TrailingDotOrSpace)
        );
        assert_eq!(
            validate_folder_name(" Addon"),
            Err(PathError::TrailingDotOrSpace)
        );
        assert_eq!(
            validate_folder_name("A\tB"),
            Err(PathError::ForbiddenChar('\t'))
        );
        assert_eq!(
            validate_folder_name("A\nB"),
            Err(PathError::ForbiddenChar('\n'))
        );
        assert_eq!(
            validate_folder_name("A:B"),
            Err(PathError::ForbiddenChar(':'))
        );
        assert_eq!(validate_folder_name(""), Err(PathError::Empty));
        assert_eq!(
            validate_folder_name(&"a".repeat(256)),
            Err(PathError::TooLong)
        );
        assert_eq!(
            validate_rel_path(&format!("{}/{}", "a".repeat(200), "b".repeat(201))),
            Err(PathError::TooLong)
        );
    }

    #[test]
    fn non_ascii_is_unsupported() {
        assert_eq!(validate_folder_name("Addön"), Err(PathError::NonAscii));
        assert_eq!(validate_rel_path("dir/файл.lua"), Err(PathError::NonAscii));
    }

    #[test]
    fn folder_key_is_lowercased() {
        assert_eq!(
            validate_folder_name("DBM-Core").unwrap().as_str(),
            "dbm-core"
        );
    }
}
