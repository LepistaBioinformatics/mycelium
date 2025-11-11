use crate::responses::GatewayError;

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
    let compressed_profile = encode_all(serialized_profile.as_bytes(), 5)
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
