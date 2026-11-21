use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use myc_core::domain::dtos::profile_signature::SigningKeyVersion;
use serde_json::Value;
use std::str::FromStr;

/// The canonical key version named by a signature's protected header
///
/// Used only to decide whether an `unknown_key` rejection deserves a JWKS
/// refetch: a missing or non-canonical `kid` never triggers network work.
///
pub fn read_signature_key_version(
    signature_header: &str,
) -> Option<SigningKeyVersion> {
    let protected = signature_header.split('.').next()?;
    let bytes = URL_SAFE_NO_PAD.decode(protected).ok()?;
    let header: Value = serde_json::from_slice(&bytes).ok()?;

    SigningKeyVersion::from_str(header.get("kid")?.as_str()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(header: &str) -> String {
        format!("{}.e30.sig", URL_SAFE_NO_PAD.encode(header))
    }

    #[test]
    fn reads_canonical_kid_only() {
        let read = |header: &str| {
            read_signature_key_version(&token(header)).map(|v| v.value())
        };

        assert_eq!(read(r#"{"alg":"EdDSA","kid":"7"}"#), Some(7));
        assert_eq!(read(r#"{"alg":"EdDSA","kid":"07"}"#), None);
        assert_eq!(read(r#"{"alg":"EdDSA","kid":7}"#), None);
        assert_eq!(read(r#"{"alg":"EdDSA"}"#), None);
        assert_eq!(read_signature_key_version("%%%.x.y"), None);
    }
}
