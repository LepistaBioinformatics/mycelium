use crate::{responses::GatewayError, settings::DEFAULT_COMPRESSION_LEVEL};

use base64::{engine::general_purpose, Engine};
use myc_core::domain::dtos::profile::Profile;
use tracing::{error, warn};
use zstd::encode_all;

/// Encode profile to Base64
///
/// This function is used to compress and encode the profile to Base64. The
/// compression is done using ZSTD.
///
#[tracing::instrument(
    name = "compress_and_encode_profile_to_base64",
    skip_all,
    fields(
        profile_id = profile.acc_id.to_string(),
    )
)]
pub fn compress_and_encode_profile_to_base64(
    profile: Profile,
) -> Result<String, GatewayError> {
    //
    // Encode profile to JSON
    //
    let serialized_profile = serde_json::to_string(&profile).map_err(|e| {
        warn!("Failed to serialize profile: {e}");

        GatewayError::InternalServerError(format!(
            "Failed to serialize profile: {e}"
        ))
    })?;

    //
    // Compress profile to ZSTD
    //
    let compressed_profile =
        encode_all(serialized_profile.as_bytes(), DEFAULT_COMPRESSION_LEVEL)
            .map_err(|e| {
                error!("Failed to compress profile: {e}");

                GatewayError::InternalServerError(format!(
                    "Failed to compress profile: {e}"
                ))
            })?;

    //
    // Encode to Base64
    //
    let encoded_profile =
        general_purpose::STANDARD.encode(compressed_profile.as_slice());

    Ok(encoded_profile)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::profile::Profile;

    /// Load large profile from JSON file
    fn load_large_profile() -> Profile {
        let json_content = include_str!("../../test/mock/large-profile.json");
        serde_json::from_str(json_content)
            .expect("Failed to deserialize large profile from JSON file")
    }

    #[test]
    fn test_compress_and_encode_large_profile() {
        let profile = load_large_profile();

        // Calculate size before compression (serialized JSON)
        let json_before = serde_json::to_string(&profile)
            .expect("Failed to serialize profile to JSON");
        let size_before = json_before.len();

        let result = compress_and_encode_profile_to_base64(profile);

        assert!(
            result.is_ok(),
            "Compression and encoding should be successful"
        );

        let encoded = result.unwrap();
        assert!(!encoded.is_empty(), "Encoded result should not be empty");

        println!("Encoded profile: {}", encoded);

        // Calculate size after compression (Base64 encoded)
        let size_after = encoded.len();

        // Calculate compression ratio
        let compression_ratio =
            (1.0 - (size_after as f64 / size_before as f64)) * 100.0;

        println!("Size before compression: {} bytes", size_before);
        println!("Size after compression (Base64): {} bytes", size_after);
        println!("Compression ratio: {:.2}%", compression_ratio);
    }
}
