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

#[cfg(test)]
mod tests {
    use super::*;

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
