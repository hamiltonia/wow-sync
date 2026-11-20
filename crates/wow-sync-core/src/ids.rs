//! Identities and the SHA-256 value type.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    BadLength(usize),
    NotLowerHex,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::BadLength(n) => write!(f, "identifier has {n} characters"),
            IdError::NotLowerHex => write!(f, "identifier is not lowercase hexadecimal"),
        }
    }
}

fn check_hex(s: &str, len: usize) -> Result<(), IdError> {
    if s.len() != len {
        return Err(IdError::BadLength(s.len()));
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(IdError::NotLowerHex);
    }
    Ok(())
}

macro_rules! hex_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(s: &str) -> Result<Self, IdError> {
                check_hex(s, 32)?;
                Ok(Self(s.to_string()))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::parse(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

hex_id!(
    ComputerId,
    "Random computer identity: 32 lowercase hex characters."
);
hex_id!(EnrollmentId, "One enrollment of a computer in a group.");
hex_id!(GroupId, "One group = one variant product code.");
hex_id!(OpId, "One apply operation (one package).");

/// SHA-256 value; displayed and parsed as 64 lowercase hex characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256(pub [u8; 32]);

impl Sha256 {
    pub fn parse(s: &str) -> Result<Self, IdError> {
        check_hex(s, 64)?;
        let mut out = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = (chunk[0] as char).to_digit(16).unwrap_or(0) as u8;
            let lo = (chunk[1] as char).to_digit(16).unwrap_or(0) as u8;
            out[i] = (hi << 4) | lo;
        }
        Ok(Self(out))
    }

    /// First two hex characters, used for provider fan-out directories.
    pub fn prefix(&self) -> String {
        format!("{:02x}", self.0[0])
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Sha256 {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// A settings checkpoint reference: `sha256(exact manifest bytes)` (contracts.md §3.4).
pub type CheckpointRef = Sha256;

/// Unicode NFC form used for every identity comparison (contracts.md §3.3).
pub fn nfc(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}

/// Account scope or character scope of a settings file, from the WTF path. Text is stored in
/// NFC and compared exactly (case-sensitive).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "IdentityWire", into = "IdentityWire")]
pub struct Identity {
    account: String,
    scope: Option<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct IdentityWire {
    account: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    realm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    character: Option<String>,
}

impl TryFrom<IdentityWire> for Identity {
    type Error = String;
    fn try_from(w: IdentityWire) -> Result<Self, String> {
        match (w.realm, w.character) {
            (None, None) => Identity::account(&w.account),
            (Some(r), Some(c)) => Identity::character(&w.account, &r, &c),
            _ => Err("a character identity needs both realm and character".into()),
        }
    }
}

impl From<Identity> for IdentityWire {
    fn from(i: Identity) -> Self {
        let (realm, character) = match i.scope {
            Some((r, c)) => (Some(r), Some(c)),
            None => (None, None),
        };
        IdentityWire {
            account: i.account,
            realm,
            character,
        }
    }
}

fn identity_part(s: &str) -> Result<String, String> {
    if s.is_empty() || s.len() > 128 || s.chars().any(char::is_control) {
        return Err("invalid identity text".into());
    }
    if s.contains(['/', '\\']) {
        return Err("identity text contains a path separator".into());
    }
    Ok(nfc(s))
}

impl Identity {
    pub fn account(account: &str) -> Result<Self, String> {
        Ok(Self {
            account: identity_part(account)?,
            scope: None,
        })
    }

    pub fn character(account: &str, realm: &str, character: &str) -> Result<Self, String> {
        Ok(Self {
            account: identity_part(account)?,
            scope: Some((identity_part(realm)?, identity_part(character)?)),
        })
    }

    pub fn account_name(&self) -> &str {
        &self.account
    }

    pub fn realm_character(&self) -> Option<(&str, &str)> {
        self.scope.as_ref().map(|(r, c)| (r.as_str(), c.as_str()))
    }

    fn lowered(&self) -> (String, Option<(String, String)>) {
        (
            self.account.to_lowercase(),
            self.scope
                .as_ref()
                .map(|(r, c)| (r.to_lowercase(), c.to_lowercase())),
        )
    }

    /// Two computers' identities that differ only by letter case are not matched (Q-69).
    pub fn differs_only_by_case(&self, other: &Identity) -> bool {
        self != other && self.lowered() == other.lowered()
    }
}

/// Why an identity is not synced on this computer (contracts.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityDiagnosis {
    /// Two local paths whose NFC forms are equal but whose bytes differ. Both are named.
    NormalizationCollision { first: String, second: String },
    /// Two computers' identities differ only by letter case.
    CaseMismatch { here: Identity, there: Identity },
}

/// Local path texts that collapse to one NFC form while differing in bytes.
pub fn normalization_collisions(local_names: &[String]) -> Vec<IdentityDiagnosis> {
    let mut by_form: std::collections::BTreeMap<String, Vec<&String>> = Default::default();
    for n in local_names {
        by_form.entry(nfc(n)).or_default().push(n);
    }
    let mut out = Vec::new();
    for names in by_form.values() {
        let mut distinct: Vec<&String> = names.clone();
        distinct.sort();
        distinct.dedup();
        for pair in distinct.windows(2) {
            out.push(IdentityDiagnosis::NormalizationCollision {
                first: pair[0].clone(),
                second: pair[1].clone(),
            });
        }
    }
    out
}

/// The scope of one syncable settings file: its addon folder and the WTF identity.
/// Never keyed by package key (contracts.md §3.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SettingsUnit {
    pub folder: crate::paths::FolderKey,
    pub identity: Identity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_collision_and_case_only_difference_are_diagnosed_and_not_matched() {
        // T-C11-identity-collisions (offline fixtures; native NTFS/APFS: NOT RUN, R4D-22)
        let composed = "Caf\u{e9}".to_string();
        let decomposed = "Cafe\u{301}".to_string();
        assert_ne!(composed, decomposed);
        let found =
            normalization_collisions(&[composed.clone(), decomposed.clone(), "Other".into()]);
        assert_eq!(found.len(), 1);
        let IdentityDiagnosis::NormalizationCollision { first, second } = &found[0] else {
            panic!("wrong diagnosis");
        };
        let mut named = [first.clone(), second.clone()];
        named.sort();
        let mut want = [composed.clone(), decomposed.clone()];
        want.sort();
        assert_eq!(named, want);
        // Identity text is stored NFC, so the two spellings are one identity text.
        assert_eq!(
            Identity::account(&composed).unwrap(),
            Identity::account(&decomposed).unwrap()
        );
        // No collision without a differing byte form.
        assert!(normalization_collisions(&[composed.clone(), composed]).is_empty());

        let a = Identity::character("Acc", "Realm", "Thrall").unwrap();
        let b = Identity::character("Acc", "Realm", "THRALL").unwrap();
        assert!(a.differs_only_by_case(&b) && b.differs_only_by_case(&a));
        assert!(!a.differs_only_by_case(&a));
        assert_ne!(a, b, "identities compare exactly");
        let other = Identity::character("Acc", "Realm", "Jaina").unwrap();
        assert!(!a.differs_only_by_case(&other));
    }

    #[test]
    fn ids_require_32_lowercase_hex() {
        assert!(ComputerId::parse(&"a".repeat(32)).is_ok());
        assert_eq!(ComputerId::parse("abc"), Err(IdError::BadLength(3)));
        assert_eq!(GroupId::parse(&"A".repeat(32)), Err(IdError::NotLowerHex));
    }

    #[test]
    fn sha256_round_trips_through_hex() {
        let h = Sha256([0xab; 32]);
        assert_eq!(Sha256::parse(&h.to_string()), Ok(h));
        assert_eq!(h.prefix(), "ab");
    }
}
