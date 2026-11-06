use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};

/// How long downstream verifiers may cache the published profile JWKS
///
/// Sent as `Cache-Control: max-age` by the profile JWKS route. Defaults to
/// 12 hours: key rotation is rare, and verifiers refetch once on an unknown
/// `kid`, so an emergency rotation never waits for the cache to expire.
///
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(try_from = "u32", into = "u32")]
pub struct JwksMaxAgeSeconds(u32);

impl JwksMaxAgeSeconds {
    /// 12 hours
    pub const DEFAULT: u32 = 43_200;

    /// Shortest accepted value (1 minute)
    pub const MIN: u32 = 60;

    /// Longest accepted value (1 day)
    pub const MAX: u32 = 86_400;

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for JwksMaxAgeSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

impl TryFrom<u32> for JwksMaxAgeSeconds {
    type Error = MappedErrors;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return dto_err(format!(
                "profile_signing_jwks_max_age_out_of_range: {value} (allowed {}..={})",
                Self::MIN,
                Self::MAX
            ))
            .with_code(NativeErrorCodes::MYC00033)
            .as_error();
        }

        Ok(Self(value))
    }
}

impl From<JwksMaxAgeSeconds> for u32 {
    fn from(max_age: JwksMaxAgeSeconds) -> Self {
        max_age.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_twelve_hours() {
        assert_eq!(JwksMaxAgeSeconds::default().value(), 43_200);
    }

    #[test]
    fn accepts_the_bounds_only() {
        for value in [60, 43_200, 86_400] {
            assert!(JwksMaxAgeSeconds::try_from(value).is_ok(), "{value}");
        }

        for value in [0, 59, 86_401, u32::MAX] {
            assert!(JwksMaxAgeSeconds::try_from(value).is_err(), "{value}");
        }
    }

    #[test]
    fn deserializes_with_validation() {
        let parsed: JwksMaxAgeSeconds = serde_json::from_str("600").unwrap();
        assert_eq!(parsed.value(), 600);
        assert!(serde_json::from_str::<JwksMaxAgeSeconds>("1").is_err());
    }
}
