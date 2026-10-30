use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Length of a SHA-256 digest encoded as unpadded Base64URL
const ENCODED_DIGEST_LENGTH: usize = 43;

/// Hash binding a signature to one exact `x-mycelium-profile` value
///
/// The `ph` claim: unpadded Base64URL of the SHA-256 of the header value's
/// UTF-8 bytes, exactly as transmitted (no decoding, decompression or
/// normalization before hashing).
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProfileHeaderHash(String);

impl ProfileHeaderHash {
    pub fn from_profile_header(profile_header: &str) -> Self {
        let digest = Sha256::digest(profile_header.as_bytes());

        Self(URL_SAFE_NO_PAD.encode(digest))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProfileHeaderHash {
    type Error = MappedErrors;

    /// Accept only the canonical encoding of a 32-byte digest
    fn try_from(encoded: String) -> Result<Self, Self::Error> {
        let decoded = URL_SAFE_NO_PAD.decode(encoded.as_bytes()).ok();

        let is_canonical = encoded.len() == ENCODED_DIGEST_LENGTH
            && decoded.as_ref().is_some_and(|bytes| {
                bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == encoded
            });

        if !is_canonical {
            return dto_err("profile_header_hash_not_canonical")
                .with_code(NativeErrorCodes::MYC00034)
                .as_error();
        }

        Ok(Self(encoded))
    }
}

impl From<ProfileHeaderHash> for String {
    fn from(hash: ProfileHeaderHash) -> Self {
        hash.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_exact_header_bytes() {
        let hash = ProfileHeaderHash::from_profile_header("abc");

        // RFC 6234 SHA-256("abc"), Base64URL without padding
        assert_eq!(
            hash.as_str(),
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
        );
    }

    #[test]
    fn different_headers_produce_different_hashes() {
        let first = ProfileHeaderHash::from_profile_header("profile");
        let second = ProfileHeaderHash::from_profile_header("profile ");

        assert_ne!(first, second);
    }

    #[test]
    fn accepts_canonical_encoding() {
        let hash = ProfileHeaderHash::from_profile_header("abc");
        let parsed = ProfileHeaderHash::try_from(hash.as_str().to_string());

        assert_eq!(parsed.unwrap(), hash);
    }

    #[test]
    fn rejects_non_canonical_encodings() {
        let valid = ProfileHeaderHash::from_profile_header("abc");
        let padded = format!("{}=", valid.as_str());
        let short = valid.as_str()[..42].to_string();
        let standard_alphabet = valid.as_str().replace('-', "+");

        for candidate in [padded, short, standard_alphabet, String::new()] {
            assert!(ProfileHeaderHash::try_from(candidate).is_err());
        }
    }

    #[test]
    fn rejects_non_canonical_trailing_bits() {
        let valid = ProfileHeaderHash::from_profile_header("abc");
        let mut tampered = valid.as_str().to_string();
        tampered.pop();
        tampered.push('1');

        assert!(ProfileHeaderHash::try_from(tampered).is_err());
    }
}
