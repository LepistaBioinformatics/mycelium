use crate::responses::GatewayError;

use base64::{engine::general_purpose, Engine};
use myc_core::domain::dtos::profile::Profile;
use tracing::{error, warn};
use zstd::decode_all;

/// Decode and decompress profile from Base64
///
/// This function is used to decode and decompress the profile from Base64. The
/// decompression is done using ZSTD.
///
#[tracing::instrument(
    name = "decode_and_decompress_profile_from_base64",
    skip_all
)]
pub fn decode_and_decompress_profile_from_base64(
    profile: String,
) -> Result<Profile, GatewayError> {
    //
    // Decode from Base64
    //
    let decoded_profile =
        general_purpose::STANDARD.decode(profile).map_err(|e| {
            warn!("Failed to decode base64 profile: {e}");

            GatewayError::InternalServerError(format!(
                "Failed to decode base64 profile: {e}"
            ))
        })?;

    //
    // Decompress profile from ZSTD
    //
    let decompressed_profile =
        decode_all(decoded_profile.as_slice()).map_err(|e| {
            error!("Failed to decompress profile: {e}");

            GatewayError::InternalServerError(format!(
                "Failed to decompress profile: {e}"
            ))
        })?;

    //
    // Deserialize from JSON
    //
    let profile_string =
        String::from_utf8(decompressed_profile).map_err(|e| {
            warn!("Failed to convert decompressed profile to string: {e}");

            GatewayError::InternalServerError(format!(
                "Failed to convert decompressed profile to string: {e}"
            ))
        })?;

    let profile = serde_json::from_str(&profile_string).map_err(|e| {
        warn!("Failed to deserialize profile: {e}");

        GatewayError::InternalServerError(format!(
            "Failed to deserialize profile: {e}"
        ))
    })?;

    Ok(profile)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::compress_and_encode_profile_to_base64;
    use myc_core::domain::dtos::profile::Profile;

    /// Load large profile from JSON file
    fn load_large_profile() -> Profile {
        let json_content = include_str!("../../test/mock/large-profile.json");
        serde_json::from_str(json_content)
            .expect("Failed to deserialize large profile from JSON file")
    }

    #[test]
    fn test_decode_and_decompress_profile_roundtrip() {
        let original_profile = load_large_profile();

        // Compress and encode the profile
        let encoded =
            compress_and_encode_profile_to_base64(original_profile.clone())
                .expect("Failed to compress and encode profile");

        assert!(!encoded.is_empty(), "Encoded profile should not be empty");

        // Decode and decompress the profile
        let decoded_profile =
            decode_and_decompress_profile_from_base64(encoded)
                .expect("Failed to decode and decompress profile");

        // Verify that the decoded profile matches the original
        assert_eq!(
            original_profile.acc_id, decoded_profile.acc_id,
            "Account ID should match"
        );
        assert_eq!(
            original_profile.owners.len(),
            decoded_profile.owners.len(),
            "Number of owners should match"
        );
        assert_eq!(
            original_profile.is_subscription, decoded_profile.is_subscription,
            "Subscription status should match"
        );
        assert_eq!(
            original_profile.is_manager, decoded_profile.is_manager,
            "Manager status should match"
        );
        assert_eq!(
            original_profile.is_staff, decoded_profile.is_staff,
            "Staff status should match"
        );

        // Compare the full profiles by serializing to JSON
        let original_json = serde_json::to_string(&original_profile)
            .expect("Failed to serialize original profile");
        let decoded_json = serde_json::to_string(&decoded_profile)
            .expect("Failed to serialize decoded profile");

        println!("Original JSON: {}", original_json);
        println!("Decoded JSON: {}", decoded_json);

        assert_eq!(
            original_json, decoded_json,
            "Full profile JSON should match exactly"
        );
    }
}
