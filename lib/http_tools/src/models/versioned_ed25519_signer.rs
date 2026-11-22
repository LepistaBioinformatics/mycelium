use super::{Ed25519PublicJwk, ED25519_PUBLIC_KEY_BYTES};

use base64::{engine::general_purpose::STANDARD, Engine};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile_signature::SigningKeyVersion,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use ring::{
    hkdf::{Salt, HKDF_SHA256},
    signature::{Ed25519KeyPair, KeyPair},
};
use serde::Serialize;
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// An Ed25519 private key prepared for signing, tagged with its version
///
/// Built once at boot from a PKCS#8 PEM. Construction performs a throwaway
/// signature so an unusable key (wrong algorithm, public key, corrupted PEM)
/// is detected before the server accepts traffic, never on a request.
///
/// Signing emits a compact JWS with `alg: EdDSA` and the version as `kid` in
/// the protected header. The public half is derived at the same time, so the
/// gateway can publish it in the profile JWKS.
///
pub struct VersionedEd25519Signer {
    version: SigningKeyVersion,
    key: EncodingKey,
    public_jwk: Ed25519PublicJwk,
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

        let public_key = derive_public_key(private_key_pem)
            .ok_or_else(|| invalid_key(version, "public_key_derivation"))?;

        let signer = Self {
            version,
            key,
            public_jwk: Ed25519PublicJwk::new(version, public_key),
        };

        signer
            .sign_claims(&SelfTestClaims { self_test: true })
            .map_err(|_| invalid_key(version, "self_test_signature"))?;

        Ok(signer)
    }

    /// Derive the signing key of one version and purpose from a shared secret
    ///
    /// Deterministic: every gateway replica holding the same secret derives
    /// the same key pair, so any replica signs and publishes the same JWKS.
    /// Each purpose derives an independent key from the same secret. The
    /// private key only ever exists in memory.
    ///
    pub fn derive_from_secret(
        version: SigningKeyVersion,
        secret: &[u8],
        purpose: KeyDerivationPurpose,
    ) -> Result<Self, MappedErrors> {
        let seed = derive_seed(secret, purpose)
            .ok_or_else(|| invalid_key(version, "derivation"))?;
        let pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| invalid_key(version, "public_key_derivation"))?;
        let public_key = pair
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| invalid_key(version, "public_key_derivation"))?;

        let signer = Self {
            version,
            key: EncodingKey::from_ed_der(
                &[&PKCS8_ED25519_PREFIX[..], &seed[..]].concat(),
            ),
            public_jwk: Ed25519PublicJwk::new(version, public_key),
        };

        signer
            .sign_claims(&SelfTestClaims { self_test: true })
            .map_err(|_| invalid_key(version, "self_test_signature"))?;

        Ok(signer)
    }

    pub fn version(&self) -> SigningKeyVersion {
        self.version
    }

    /// The public half of this key, as published in the profile JWKS
    pub fn public_jwk(&self) -> &Ed25519PublicJwk {
        &self.public_jwk
    }

    /// Sign arbitrary claims as a compact EdDSA JWS carrying this `kid`
    pub fn sign_claims<T: Serialize>(
        &self,
        claims: &T,
    ) -> Result<String, MappedErrors> {
        self.sign_typed_claims(None, claims)
    }

    /// Like `sign_claims`, with an explicit `typ` in the protected header
    ///
    /// A distinct `typ` per protocol keeps a token of one protocol from being
    /// accepted by the verifier of another.
    ///
    pub fn sign_typed_claims<T: Serialize>(
        &self,
        typ: Option<&str>,
        claims: &T,
    ) -> Result<String, MappedErrors> {
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.version.to_string());
        if let Some(typ) = typ {
            header.typ = Some(typ.to_string());
        }

        encode(&header, claims, &self.key).map_err(|_| {
            execution_err("profile_signature_encoding_failed")
                .with_code(NativeErrorCodes::MYC00042)
        })
    }
}

/// What a derived Ed25519 key is for
///
/// Each purpose has its own HKDF domain separation label. The label is part
/// of the key: changing it changes every derived key. It keeps each derived
/// key cryptographically independent from the others and from the HMAC use
/// of the same secret.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationPurpose {
    /// Signs the downstream profile (`x-mycelium-profile-signature`)
    ProfileSigning,

    /// Signs instance federation assertions and peer requests
    Federation,
}

impl KeyDerivationPurpose {
    fn label(self) -> &'static [u8] {
        match self {
            Self::ProfileSigning => PROFILE_SIGNING_DERIVATION_LABEL,
            Self::Federation => FEDERATION_DERIVATION_LABEL,
        }
    }
}

const PROFILE_SIGNING_DERIVATION_LABEL: &[u8] =
    b"mycelium/profile-signing/ed25519/v1";

const FEDERATION_DERIVATION_LABEL: &[u8] = b"mycelium/federation/ed25519/v1";

/// PKCS#8 v1 prefix of an Ed25519 private key (RFC 8410); the seed follows
const PKCS8_ED25519_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70,
    0x04, 0x22, 0x04, 0x20,
];

/// HKDF-SHA256 of the secret, expanded to one Ed25519 seed per purpose
fn derive_seed(
    secret: &[u8],
    purpose: KeyDerivationPurpose,
) -> Option<[u8; ED25519_PUBLIC_KEY_BYTES]> {
    let mut seed = [0u8; ED25519_PUBLIC_KEY_BYTES];

    Salt::new(HKDF_SHA256, &[])
        .extract(secret)
        .expand(&[purpose.label()], HKDF_SHA256)
        .ok()?
        .fill(&mut seed)
        .ok()?;

    Some(seed)
}

/// The raw Ed25519 public key of a PKCS#8 PEM private key
///
/// The PEM was already accepted by `jsonwebtoken`; this only strips the
/// armor and lets `ring` compute the public point from the seed.
///
fn derive_public_key(
    private_key_pem: &str,
) -> Option<[u8; ED25519_PUBLIC_KEY_BYTES]> {
    let body: String = private_key_pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("-----"))
        .collect();
    let der = STANDARD.decode(body).ok()?;
    let pair = Ed25519KeyPair::from_pkcs8_maybe_unchecked(&der).ok()?;

    pair.public_key().as_ref().try_into().ok()
}

/// Sanitized error: names the version and failing step, never the key
fn invalid_key(version: SigningKeyVersion, step: &str) -> MappedErrors {
    execution_err(format!(
        "profile_signing_key_invalid: version {version}, step {step}"
    ))
    .with_code(NativeErrorCodes::MYC00041)
}

impl Debug for VersionedEd25519Signer {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("VersionedEd25519Signer")
            .field("version", &self.version)
            .field("key", &"<redacted>")
            .field("public_jwk", &self.public_jwk.x())
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
    fn derives_the_rfc_8032_public_key() {
        let signer =
            VersionedEd25519Signer::from_pem(version(1), TEST_PRIVATE_PEM)
                .unwrap();

        assert_eq!(
            signer.public_jwk().x(),
            "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"
        );
        assert_eq!(signer.public_jwk().version(), version(1));
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
    #[test]
    fn derivation_is_deterministic_per_secret() {
        let derive = |secret: &[u8]| {
            VersionedEd25519Signer::derive_from_secret(
                version(1),
                secret,
                KeyDerivationPurpose::ProfileSigning,
            )
            .unwrap()
            .public_jwk()
            .x()
        };

        assert_eq!(derive(b"shared-secret"), derive(b"shared-secret"));
        assert_ne!(derive(b"shared-secret"), derive(b"other-secret"));
    }

    #[test]
    fn derived_key_signs_what_its_public_half_verifies() {
        use jsonwebtoken::{crypto::verify, Algorithm, DecodingKey};

        let signer = VersionedEd25519Signer::derive_from_secret(
            version(3),
            b"shared-secret",
            KeyDerivationPurpose::ProfileSigning,
        )
        .unwrap();
        let token = signer
            .sign_claims(&SelfTestClaims { self_test: true })
            .unwrap();
        let (message, signature) = token.rsplit_once('.').unwrap();
        let key =
            DecodingKey::from_ed_components(&signer.public_jwk().x()).unwrap();

        assert!(
            verify(signature, message.as_bytes(), &key, Algorithm::EdDSA)
                .unwrap()
        );
    }

    #[test]
    fn each_purpose_derives_an_independent_key() {
        use jsonwebtoken::{crypto::verify, Algorithm, DecodingKey};

        let derive = |purpose| {
            VersionedEd25519Signer::derive_from_secret(
                version(1),
                b"shared-secret",
                purpose,
            )
            .unwrap()
        };
        let profile = derive(KeyDerivationPurpose::ProfileSigning);
        let federation = derive(KeyDerivationPurpose::Federation);

        assert_ne!(profile.public_jwk().x(), federation.public_jwk().x());

        let token = federation
            .sign_claims(&SelfTestClaims { self_test: true })
            .unwrap();
        let (message, signature) = token.rsplit_once('.').unwrap();
        let profile_key =
            DecodingKey::from_ed_components(&profile.public_jwk().x()).unwrap();
        let federation_key =
            DecodingKey::from_ed_components(&federation.public_jwk().x())
                .unwrap();

        assert!(!verify(
            signature,
            message.as_bytes(),
            &profile_key,
            Algorithm::EdDSA
        )
        .unwrap());
        assert!(verify(
            signature,
            message.as_bytes(),
            &federation_key,
            Algorithm::EdDSA
        )
        .unwrap());
    }
}
