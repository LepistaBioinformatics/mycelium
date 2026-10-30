use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile_signature::SigningKeyVersion,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::Serialize;
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// An Ed25519 private key prepared for signing, tagged with its version
///
/// Built once at boot from a PKCS#8 PEM. Construction performs a throwaway
/// signature so an unusable key (wrong algorithm, public key, corrupted PEM)
/// is detected before the server accepts traffic, never on a request.
///
/// Signing emits a compact JWS with `alg: EdDSA` and the version as `kid` in
/// the protected header.
///
pub struct VersionedEd25519Signer {
    version: SigningKeyVersion,
    key: EncodingKey,
}

#[derive(Serialize)]
struct SelfTestClaims {
    self_test: bool,
}

impl VersionedEd25519Signer {
    pub fn from_pem(
        version: SigningKeyVersion,
        private_key_pem: &str,
    ) -> Result<Self, MappedErrors> {
        let key = EncodingKey::from_ed_pem(private_key_pem.as_bytes())
            .map_err(|_| invalid_key(version, "import"))?;

        let signer = Self { version, key };

        signer
            .sign_claims(&SelfTestClaims { self_test: true })
            .map_err(|_| invalid_key(version, "self_test_signature"))?;

        Ok(signer)
    }

    pub fn version(&self) -> SigningKeyVersion {
        self.version
    }

    /// Sign arbitrary claims as a compact EdDSA JWS carrying this `kid`
    pub fn sign_claims<T: Serialize>(
        &self,
        claims: &T,
    ) -> Result<String, MappedErrors> {
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.version.to_string());

        encode(&header, claims, &self.key).map_err(|_| {
            execution_err("profile_signature_encoding_failed")
                .with_code(NativeErrorCodes::MYC00034)
        })
    }
}

/// Sanitized error: names the version and failing step, never the key
fn invalid_key(version: SigningKeyVersion, step: &str) -> MappedErrors {
    execution_err(format!(
        "profile_signing_key_invalid: version {version}, step {step}"
    ))
    .with_code(NativeErrorCodes::MYC00033)
}

impl Debug for VersionedEd25519Signer {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("VersionedEd25519Signer")
            .field("version", &self.version)
            .field("key", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    /// TEST ONLY Ed25519 key (RFC 8032 test vector 1 secret, PKCS#8)
    const TEST_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

    /// TEST ONLY RFC 8032 test vector 1 public key (SPKI)
    const TEST_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";

    fn version(value: u32) -> SigningKeyVersion {
        SigningKeyVersion::try_from(value).unwrap()
    }

    #[test]
    fn signs_with_eddsa_and_kid() {
        let signer =
            VersionedEd25519Signer::from_pem(version(7), TEST_PRIVATE_PEM)
                .unwrap();
        let token = signer.sign_claims(&SelfTestClaims { self_test: true });
        let token = token.unwrap();

        let protected = token.split('.').next().unwrap();
        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(protected).unwrap())
                .unwrap();

        assert_eq!(header["alg"], "EdDSA");
        assert_eq!(header["kid"], "7");
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn rejects_public_key_and_garbage() {
        for pem in [TEST_PUBLIC_PEM, "not a pem", ""] {
            assert!(VersionedEd25519Signer::from_pem(version(1), pem).is_err());
        }
    }

    #[test]
    fn errors_and_debug_never_leak_key_material() {
        let signer =
            VersionedEd25519Signer::from_pem(version(1), TEST_PRIVATE_PEM)
                .unwrap();
        let rendered = format!("{signer:?}");
        assert!(!rendered.contains("MC4CAQAw"));

        let error =
            VersionedEd25519Signer::from_pem(version(3), "secret-material")
                .unwrap_err()
                .to_string();
        assert!(!error.contains("secret-material"));
        assert!(error.contains("version 3"));
    }
}
