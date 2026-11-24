use super::invalid_verification_setting;
use crate::settings::{
    DEFAULT_PROFILE_SIGNATURE_TOLERANCE_SECONDS,
    MAX_PROFILE_SIGNATURE_TOLERANCE_SECONDS, PROFILE_SIGNATURE_TOLERANCE_ENV,
};

use mycelium_base::utils::errors::MappedErrors;

/// Symmetric, inclusive `iat` tolerance of a downstream verifier, in seconds
///
/// Parsed like the SDKs: a canonical positive integer of at most 300 (the
/// tolerance is the replay window of a captured signature). An invalid value is a configuration error, never the default.
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
            .filter(|seconds| {
                *seconds <= MAX_PROFILE_SIGNATURE_TOLERANCE_SECONDS
            });

        parsed.map(Self).ok_or_else(|| {
            invalid_verification_setting(
                PROFILE_SIGNATURE_TOLERANCE_ENV,
                "must be an integer between 1 and 300",
            )
        })
    }
}

fn is_canonical_positive(raw: &str) -> bool {
    !raw.is_empty()
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
    fn accepts_canonical_values_up_to_five_minutes() {
        assert_eq!(parse(" 120 ").unwrap().seconds(), 120);
        assert_eq!(parse("300").unwrap().seconds(), 300);
    }

    #[test]
    fn rejects_non_canonical_or_too_wide_values() {
        for raw in ["", "0", "-1", "060", "1.5", "abc", "301", "86400"] {
            assert!(parse(raw).is_err(), "{raw:?}");
        }
    }
}
