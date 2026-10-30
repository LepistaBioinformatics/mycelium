use super::invalid_verification_setting;
use crate::settings::{
    DEFAULT_PROFILE_SIGNATURE_TOLERANCE_SECONDS,
    PROFILE_SIGNATURE_TOLERANCE_ENV,
};

use myc_core::domain::dtos::profile_signature::MAX_SAFE_TIMESTAMP_SECONDS;
use mycelium_base::utils::errors::MappedErrors;

/// Longest accepted tolerance text (the safe integer has 16 digits)
const MAX_TOLERANCE_DIGITS: usize = 16;

/// Symmetric, inclusive `iat` tolerance of a downstream verifier, in seconds
///
/// Parsed like the SDKs: a canonical positive integer up to the cross-language
/// safe integer. An invalid value is a configuration error, never the default.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileSignatureTolerance(u64);

impl Default for ProfileSignatureTolerance {
    fn default() -> Self {
        Self(DEFAULT_PROFILE_SIGNATURE_TOLERANCE_SECONDS)
    }
}

impl ProfileSignatureTolerance {
    pub fn seconds(&self) -> u64 {
        self.0
    }

    /// Parse `MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS`, or use the default
    pub fn from_variable(raw: Option<&String>) -> Result<Self, MappedErrors> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        let trimmed = raw.trim();
        let parsed = is_canonical_positive(trimmed)
            .then(|| trimmed.parse::<u64>().ok())
            .flatten()
            .filter(|seconds| *seconds <= MAX_SAFE_TIMESTAMP_SECONDS);

        parsed.map(Self).ok_or_else(|| {
            invalid_verification_setting(
                PROFILE_SIGNATURE_TOLERANCE_ENV,
                "must be a positive integer",
            )
        })
    }
}

fn is_canonical_positive(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= MAX_TOLERANCE_DIGITS
        && raw.bytes().all(|byte| byte.is_ascii_digit())
        && !raw.starts_with('0')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Result<ProfileSignatureTolerance, MappedErrors> {
        ProfileSignatureTolerance::from_variable(Some(&raw.to_string()))
    }

    #[test]
    fn absent_variable_uses_the_default() {
        let tolerance = ProfileSignatureTolerance::from_variable(None).unwrap();

        assert_eq!(tolerance.seconds(), 60);
    }

    #[test]
    fn accepts_canonical_values_up_to_the_safe_integer() {
        assert_eq!(parse(" 120 ").unwrap().seconds(), 120);
        assert_eq!(
            parse("9007199254740991").unwrap().seconds(),
            MAX_SAFE_TIMESTAMP_SECONDS
        );
    }

    #[test]
    fn rejects_non_canonical_or_unsafe_values() {
        for raw in ["", "0", "-1", "060", "1.5", "abc", "9007199254740992"] {
            assert!(parse(raw).is_err(), "{raw:?}");
        }
    }
}
