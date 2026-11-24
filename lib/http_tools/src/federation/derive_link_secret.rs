use crate::models::VersionedEd25519Signer;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use mycelium_base::utils::errors::MappedErrors;
use serde::Serialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// What a secret derived for a link consent is used for
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkSecretPurpose {
    /// The PKCE verifier of the issuer's own hosted login exchange
    Pkce,
    /// The token binding the confirmation form to its consent
    Csrf,
}

#[derive(Serialize)]
struct LinkSecretInput {
    purpose: LinkSecretPurpose,
    consent: Uuid,
}

/// A secret bound to one consent, derived from the federation key
///
/// SHA-256 of the key's EdDSA signature over the purpose and the consent id,
/// Base64URL without padding (43 characters, a valid PKCE verifier). Ed25519
/// signatures are deterministic, so every pod derives the same value and
/// nothing is stored; without the private key it cannot be computed from the
/// consent id.
///
pub fn derive_link_secret(
    signer: &VersionedEd25519Signer,
    purpose: LinkSecretPurpose,
    consent_id: Uuid,
) -> Result<String, MappedErrors> {
    let signature = signer.sign_claims(&LinkSecretInput {
        purpose,
        consent: consent_id,
    })?;

    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(signature.as_bytes())))
}

/// Constant-time check of a submitted secret
pub fn link_secret_matches(
    signer: &VersionedEd25519Signer,
    purpose: LinkSecretPurpose,
    consent_id: Uuid,
    candidate: &str,
) -> bool {
    derive_link_secret(signer, purpose, consent_id)
        .map(|secret| secret.as_bytes().ct_eq(candidate.as_bytes()).into())
        .unwrap_or(false)
}

/// The S256 PKCE challenge of a verifier (RFC 7636 §4.2)
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KeyDerivationPurpose;
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;

    fn signer(secret: &[u8]) -> VersionedEd25519Signer {
        VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            secret,
            KeyDerivationPurpose::Federation,
        )
        .unwrap()
    }

    #[test]
    fn the_secret_is_stable_and_bound_to_key_purpose_and_consent() {
        let consent = Uuid::from_u128(1);
        let key = signer(b"test-only-secret");
        let pkce =
            derive_link_secret(&key, LinkSecretPurpose::Pkce, consent).unwrap();

        assert_eq!(pkce.len(), 43);
        assert_eq!(
            derive_link_secret(
                &signer(b"test-only-secret"),
                LinkSecretPurpose::Pkce,
                consent
            )
            .unwrap(),
            pkce
        );
        for other in [
            derive_link_secret(&key, LinkSecretPurpose::Csrf, consent),
            derive_link_secret(
                &key,
                LinkSecretPurpose::Pkce,
                Uuid::from_u128(2),
            ),
            derive_link_secret(
                &signer(b"another"),
                LinkSecretPurpose::Pkce,
                consent,
            ),
        ] {
            assert_ne!(other.unwrap(), pkce);
        }
    }

    #[test]
    fn only_the_derived_secret_matches() {
        let consent = Uuid::from_u128(1);
        let key = signer(b"test-only-secret");
        let csrf =
            derive_link_secret(&key, LinkSecretPurpose::Csrf, consent).unwrap();

        assert!(link_secret_matches(
            &key,
            LinkSecretPurpose::Csrf,
            consent,
            &csrf
        ));
        assert!(!link_secret_matches(
            &key,
            LinkSecretPurpose::Csrf,
            consent,
            "x"
        ));
    }

    #[test]
    fn the_challenge_follows_rfc_7636() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
