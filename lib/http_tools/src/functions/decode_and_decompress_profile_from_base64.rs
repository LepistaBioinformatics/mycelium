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
