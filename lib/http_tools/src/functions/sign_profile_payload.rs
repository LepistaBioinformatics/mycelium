use crate::models::VersionedEd25519Signer;

use myc_core::domain::dtos::profile_signature::{
    ProfileHeaderHash, ProfileSignatureAudience, ProfileSignatureClaims,
    UnixTimestampSeconds,
};
use mycelium_base::utils::errors::MappedErrors;

/// Issue the downstream profile signature for one exact header value
///
/// `profile_header` must be the value placed in `x-mycelium-profile` byte for
/// byte (the output of `compress_and_encode_profile_to_base64`). The JWS
/// carries only `{iat, ph, aud}`: the profile is bound by its SHA-256 hash,
/// never duplicated. The clock is an input so callers and tests control it.
///
#[tracing::instrument(name = "sign_profile_payload", skip_all)]
pub fn sign_profile_payload(
    profile_header: &str,
    audience: ProfileSignatureAudience,
    signer: &VersionedEd25519Signer,
    issued_at: UnixTimestampSeconds,
) -> Result<String, MappedErrors> {
    let claims = ProfileSignatureClaims {
        iat: issued_at,
        ph: ProfileHeaderHash::from_profile_header(profile_header),
        aud: audience,
    };

    signer.sign_claims(&claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;

    /// TEST ONLY: RFC 8032 test vector 1 (PKCS#8 / SPKI)
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
    const PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";
    const NOW: u64 = 1_800_000_000;

    fn signer() -> VersionedEd25519Signer {
        let version = SigningKeyVersion::try_from(1).unwrap();
        VersionedEd25519Signer::from_pem(version, PRIVATE_PEM).unwrap()
    }

    fn issue(profile_header: &str) -> String {
        sign_profile_payload(
            profile_header,
            ProfileSignatureAudience::try_from("svc".to_string()).unwrap(),
            &signer(),
            UnixTimestampSeconds::try_from(NOW).unwrap(),
        )
        .unwrap()
    }

    fn verify(token: &str) -> ProfileSignatureClaims {
        let key = DecodingKey::from_ed_pem(PUBLIC_PEM.as_bytes()).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        validation.validate_aud = false;

        decode::<ProfileSignatureClaims>(token, &key, &validation)
            .unwrap()
            .claims
    }

    #[test]
    fn claims_bind_the_exact_header() {
        let claims = verify(&issue("cHJvZmlsZQ=="));

        assert_eq!(claims.iat.value(), NOW);
        assert_eq!(claims.aud.as_str(), "svc");
        assert_eq!(
            claims.ph,
            ProfileHeaderHash::from_profile_header("cHJvZmlsZQ==")
        );
        assert_ne!(
            claims.ph,
            ProfileHeaderHash::from_profile_header("cHJvZmlsZQ")
        );
    }

    #[test]
    fn token_does_not_embed_the_profile() {
        let profile_header = "A".repeat(10_000);
        let token = issue(&profile_header);

        assert!(token.len() < 512, "token grew with the profile");
        assert!(!token.contains(&profile_header));
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let token = issue("cHJvZmlsZQ==");
        let other = issue("b3RoZXI=");
        let parts: Vec<&str> = token.split('.').collect();
        let other_parts: Vec<&str> = other.split('.').collect();
        let tampered = format!("{}.{}.{}", parts[0], other_parts[1], parts[2]);

        let key = DecodingKey::from_ed_pem(PUBLIC_PEM.as_bytes()).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        validation.validate_aud = false;

        assert!(
            decode::<ProfileSignatureClaims>(&tampered, &key, &validation)
                .is_err()
        );
    }
}
