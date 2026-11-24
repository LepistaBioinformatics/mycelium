use std::fmt::{Display, Formatter, Result as FmtResult};

/// Why a downstream verifier refused a signed profile
///
/// The stable, log-safe `reason` shared by every verifier (Rust, Python,
/// JavaScript, Go). The contract vectors encode these strings, and the checks
/// run in a fixed precedence so all verifiers report the same reason for the
/// same input. A reason never carries tokens, claims or profile content.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSignatureRejection {
    MissingProfile,
    MissingSignature,
    DuplicateHeader,
    MalformedSignature,
    UnsupportedAlgorithm,
    UnknownKey,
    InvalidSignature,
    InvalidClaims,
    TimestampOutOfTolerance,
    ProfileHashMismatch,
    AudienceMismatch,
    InvalidProfile,
}

impl ProfileSignatureRejection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingProfile => "missing_profile",
            Self::MissingSignature => "missing_signature",
            Self::DuplicateHeader => "duplicate_header",
            Self::MalformedSignature => "malformed_signature",
            Self::UnsupportedAlgorithm => "unsupported_algorithm",
            Self::UnknownKey => "unknown_key",
            Self::InvalidSignature => "invalid_signature",
            Self::InvalidClaims => "invalid_claims",
            Self::TimestampOutOfTolerance => "timestamp_out_of_tolerance",
            Self::ProfileHashMismatch => "profile_hash_mismatch",
            Self::AudienceMismatch => "audience_mismatch",
            Self::InvalidProfile => "invalid_profile",
        }
    }
}

impl Display for ProfileSignatureRejection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_match_the_sdk_wire_strings() {
        assert_eq!(
            ProfileSignatureRejection::TimestampOutOfTolerance.to_string(),
            "timestamp_out_of_tolerance"
        );
        assert_eq!(
            ProfileSignatureRejection::DuplicateHeader.as_str(),
            "duplicate_header"
        );
    }
}
