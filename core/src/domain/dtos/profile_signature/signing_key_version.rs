use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

/// Version of a downstream profile signing key
///
/// Carried as the JWS `kid` header in its canonical decimal form (no sign,
/// no leading zeros, no whitespace). Version `0` is reserved and rejected so
/// that a missing or defaulted value can never select a key.
///
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[serde(try_from = "u32", into = "u32")]
pub struct SigningKeyVersion(u32);

impl SigningKeyVersion {
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for SigningKeyVersion {
    type Error = MappedErrors;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 0 {
            return dto_err("profile_signing_key_version_must_be_positive")
                .with_code(NativeErrorCodes::MYC00033)
                .as_error();
        }

        Ok(Self(value))
    }
}

impl From<SigningKeyVersion> for u32 {
    fn from(version: SigningKeyVersion) -> Self {
        version.0
    }
}

impl FromStr for SigningKeyVersion {
    type Err = MappedErrors;

    /// Parse the canonical decimal form used by the JWS `kid` header
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let is_canonical = !raw.is_empty()
            && raw.bytes().all(|byte| byte.is_ascii_digit())
            && !raw.starts_with('0');

        if !is_canonical {
            return dto_err("profile_signing_key_version_not_canonical")
                .with_code(NativeErrorCodes::MYC00033)
                .as_error();
        }

        let parsed = raw.parse::<u32>().map_err(|_| {
            dto_err("profile_signing_key_version_out_of_range")
                .with_code(NativeErrorCodes::MYC00033)
        })?;

        Self::try_from(parsed)
    }
}

impl Display for SigningKeyVersion {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_extreme_valid_versions() {
        assert_eq!(SigningKeyVersion::try_from(1).unwrap().value(), 1);
        assert_eq!(
            SigningKeyVersion::try_from(u32::MAX).unwrap().value(),
            u32::MAX
        );
    }

    #[test]
    fn rejects_zero() {
        assert!(SigningKeyVersion::try_from(0).is_err());
        assert!("0".parse::<SigningKeyVersion>().is_err());
    }

    #[test]
    fn parses_only_canonical_decimal() {
        assert_eq!("42".parse::<SigningKeyVersion>().unwrap().value(), 42);

        for raw in ["", "01", "+1", "-1", " 1", "1 ", "1.0", "0x1", "１"] {
            assert!(raw.parse::<SigningKeyVersion>().is_err(), "{raw:?}");
        }
    }

    #[test]
    fn rejects_overflow() {
        assert!("4294967296".parse::<SigningKeyVersion>().is_err());
    }

    #[test]
    fn displays_canonical_form() {
        let version = SigningKeyVersion::try_from(7).unwrap();
        assert_eq!(version.to_string(), "7");
    }

    #[test]
    fn deserializes_with_validation() {
        let parsed: SigningKeyVersion = serde_json::from_str("3").unwrap();
        assert_eq!(parsed.value(), 3);
        assert!(serde_json::from_str::<SigningKeyVersion>("0").is_err());
    }
}
