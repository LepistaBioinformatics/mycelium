use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};

/// Largest integer every SDK language represents exactly
///
/// JavaScript numbers are IEEE-754 doubles, so the `iat` claim is bounded to
/// `Number.MAX_SAFE_INTEGER` to keep the contract identical in Rust, Python
/// and JavaScript.
///
pub const MAX_SAFE_TIMESTAMP_SECONDS: u64 = 9_007_199_254_740_991;

/// Unix timestamp in whole seconds, bounded to the cross-language safe range
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(try_from = "u64", into = "u64")]
pub struct UnixTimestampSeconds(u64);

impl UnixTimestampSeconds {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for UnixTimestampSeconds {
    type Error = MappedErrors;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > MAX_SAFE_TIMESTAMP_SECONDS {
            return dto_err("unix_timestamp_seconds_out_of_safe_range")
                .with_code(NativeErrorCodes::MYC00034)
                .as_error();
        }

        Ok(Self(value))
    }
}

impl From<UnixTimestampSeconds> for u64 {
    fn from(timestamp: UnixTimestampSeconds) -> Self {
        timestamp.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_range_boundaries() {
        assert_eq!(UnixTimestampSeconds::try_from(0).unwrap().value(), 0);
        assert_eq!(
            UnixTimestampSeconds::try_from(MAX_SAFE_TIMESTAMP_SECONDS)
                .unwrap()
                .value(),
            MAX_SAFE_TIMESTAMP_SECONDS
        );
    }

    #[test]
    fn rejects_unsafe_values() {
        assert!(
            UnixTimestampSeconds::try_from(MAX_SAFE_TIMESTAMP_SECONDS + 1)
                .is_err()
        );
        assert!(UnixTimestampSeconds::try_from(u64::MAX).is_err());
    }

    #[test]
    fn serializes_as_plain_integer() {
        let timestamp = UnixTimestampSeconds::try_from(1_800_000_000).unwrap();
        assert_eq!(serde_json::to_string(&timestamp).unwrap(), "1800000000");
    }
}
