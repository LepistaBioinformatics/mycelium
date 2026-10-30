use crate::models::ProfileSignatureVerifier;

use myc_core::domain::dtos::profile_signature::{
    ProfileHeaderHash, ProfileSignatureAudience, ProfileSignatureRejection,
    UnixTimestampSeconds,
};
use serde_json::Value;
use subtle::ConstantTimeEq;

/// Claims of an authenticated signature, validated member by member
///
/// Unknown members are ignored, like the SDK verifiers; each known member
/// must have its exact wire type (`iat` a non-negative safe integer, `ph` a
/// canonical digest, `aud` a non-blank string) or the claims are invalid.
///
pub struct SignedProfileClaims {
    iat: UnixTimestampSeconds,
    ph: ProfileHeaderHash,
    aud: ProfileSignatureAudience,
}

/// Parse the verified JWS payload (`invalid_claims` on any deviation)
pub fn read_profile_signature_claims(
    payload: &[u8],
) -> Result<SignedProfileClaims, ProfileSignatureRejection> {
    let Ok(Value::Object(claims)) = serde_json::from_slice::<Value>(payload)
    else {
        return Err(ProfileSignatureRejection::InvalidClaims);
    };

    let iat = claims
        .get("iat")
        .and_then(Value::as_u64)
        .and_then(|value| UnixTimestampSeconds::try_from(value).ok());
    let ph = claims
        .get("ph")
        .and_then(Value::as_str)
        .and_then(|value| ProfileHeaderHash::try_from(value.to_string()).ok());
    let aud = claims.get("aud").and_then(Value::as_str).and_then(|value| {
        ProfileSignatureAudience::try_from(value.to_string()).ok()
    });

    let (Some(iat), Some(ph), Some(aud)) = (iat, ph, aud) else {
        return Err(ProfileSignatureRejection::InvalidClaims);
    };

    Ok(SignedProfileClaims { iat, ph, aud })
}

/// Apply the time window, the profile hash and the optional audience
pub fn check_profile_signature_claims(
    claims: &SignedProfileClaims,
    profile_header: &str,
    verifier: &ProfileSignatureVerifier,
    now: UnixTimestampSeconds,
) -> Result<(), ProfileSignatureRejection> {
    if now.value().abs_diff(claims.iat.value()) > verifier.tolerance().seconds()
    {
        return Err(ProfileSignatureRejection::TimestampOutOfTolerance);
    }

    let received = ProfileHeaderHash::from_profile_header(profile_header);
    let hash_matches = received
        .as_str()
        .as_bytes()
        .ct_eq(claims.ph.as_str().as_bytes());
    if !bool::from(hash_matches) {
        return Err(ProfileSignatureRejection::ProfileHashMismatch);
    }

    if verifier
        .expected_audience()
        .is_some_and(|expected| *expected != claims.aud)
    {
        return Err(ProfileSignatureRejection::AudienceMismatch);
    }

    Ok(())
}
