use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// A peer field as the JSONB the table stores
pub(super) fn to_json<T: serde::Serialize>(
    value: &T,
) -> Result<serde_json::Value, MappedErrors> {
    serde_json::to_value(value)
        .map_err(|e| creation_err(format!("Failed to serialize peer: {e}")))
}
