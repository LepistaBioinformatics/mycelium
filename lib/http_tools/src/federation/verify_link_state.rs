use super::{
    read_jws_segments::decode_claims, verify_typed_jws::verify_typed_jws,
    ExpectedLinkState, LinkStateClaims, PeerRequestRejection,
    VerifiedLinkState, LINK_STATE_TYP,
};

use myc_core::domain::dtos::federation::PeerKey;

/// Verify a link redirect state against the consumer's stored keys
///
/// Algorithm pinned to EdDSA, `typ` to `LINK_STATE_TYP`. The `jti` is
/// returned, not checked: single use is the issuer's consent store.
///
pub fn verify_link_state(
    jws: &str,
    keys: &[PeerKey],
    expected: &ExpectedLinkState,
) -> Result<VerifiedLinkState, PeerRequestRejection> {
    let (kid, payload) = verify_typed_jws(jws, keys, LINK_STATE_TYP)?;
    let claims: LinkStateClaims = decode_claims(payload)?;

    check_claims(&claims, expected)?;

    Ok(VerifiedLinkState { kid, claims })
}

fn check_claims(
    claims: &LinkStateClaims,
    expected: &ExpectedLinkState,
) -> Result<(), PeerRequestRejection> {
    if claims.aud != expected.audience {
        return Err(PeerRequestRejection::WrongAudience);
    }
    if claims.exp <= claims.iat {
        return Err(PeerRequestRejection::Malformed);
    }
    if expected.now > claims.exp.saturating_add(expected.tolerance_secs) {
        return Err(PeerRequestRejection::Expired);
    }
    if claims.iat > expected.now.saturating_add(expected.tolerance_secs) {
        return Err(PeerRequestRejection::IssuedInFuture);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::{
        peek_peer_request_issuer, sign_link_state, sign_peer_request,
        PeerRequest,
    };
    use crate::models::{KeyDerivationPurpose, VersionedEd25519Signer};

    use chrono::Local;
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;
    use uuid::Uuid;

    const NOW: u64 = 1_800_000_000;

    fn signer() -> VersionedEd25519Signer {
        VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            b"test-only-secret",
            KeyDerivationPurpose::Federation,
        )
        .unwrap()
    }

    fn consumer() -> Uuid {
        Uuid::from_u128(0x2222_2222_2222_4222_8222_2222_2222_2222)
    }

    fn issuer() -> Uuid {
        Uuid::from_u128(0x1111_1111_1111_4111_8111_1111_1111_1111)
    }

    fn keys_of(signer: &VersionedEd25519Signer) -> Vec<PeerKey> {
        vec![PeerKey {
            peer_id: Uuid::nil(),
            kid: signer.version().to_string(),
            alg: "EdDSA".to_string(),
            public_key: signer.public_jwk().x(),
            fetched_at: Local::now(),
            expires_at: None,
        }]
    }

    fn claims() -> LinkStateClaims {
        LinkStateClaims {
            iss: consumer(),
            aud: issuer(),
            iat: NOW,
            exp: NOW + 600,
            jti: Uuid::from_u128(7),
            email: "user@example.com".to_string(),
            return_url: "https://myc-02.example.com/federation/link/done"
                .to_string(),
        }
    }

    fn expected(now: u64) -> ExpectedLinkState {
        ExpectedLinkState {
            audience: issuer(),
            now,
            tolerance_secs: 30,
        }
    }

    #[test]
    fn a_signed_state_round_trips_and_names_its_issuer() {
        let signer = signer();
        let jws = sign_link_state(&signer, &claims()).unwrap();

        let verified =
            verify_link_state(&jws, &keys_of(&signer), &expected(NOW)).unwrap();

        assert_eq!(verified.claims, claims());
        assert_eq!(verified.kid, signer.version().to_string());
        assert_eq!(
            peek_peer_request_issuer(&jws).unwrap(),
            (consumer(), signer.version().to_string())
        );
    }

    #[test]
    fn another_audience_or_an_expired_state_is_refused() {
        let signer = signer();
        let jws = sign_link_state(&signer, &claims()).unwrap();
        let keys = keys_of(&signer);
        let mut elsewhere = expected(NOW);
        elsewhere.audience = consumer();

        assert_eq!(
            verify_link_state(&jws, &keys, &elsewhere),
            Err(PeerRequestRejection::WrongAudience)
        );
        assert_eq!(
            verify_link_state(&jws, &keys, &expected(NOW + 600 + 31)),
            Err(PeerRequestRejection::Expired)
        );
    }

    #[test]
    fn a_peer_request_token_is_not_a_link_state() {
        let signer = signer();
        let request = PeerRequest {
            iss: consumer(),
            aud: issuer(),
            method: "POST",
            path: "/federation/link/confirm",
            body: b"{}",
            jti: "00000000-0000-4000-8000-000000000001",
            issued_at: NOW,
            lifetime_secs: 60,
        };
        let jws = sign_peer_request(&signer, &request).unwrap();

        assert_eq!(
            verify_link_state(&jws, &keys_of(&signer), &expected(NOW)),
            Err(PeerRequestRejection::WrongType)
        );
    }

    #[test]
    fn a_tampered_state_or_a_foreign_key_is_refused() {
        let signer = signer();
        let jws = sign_link_state(&signer, &claims()).unwrap();
        let mut forged = claims();
        forged.email = "attacker@example.com".to_string();
        let forged_jws = sign_link_state(&signer, &forged).unwrap();
        let spliced = format!(
            "{}.{}.{}",
            jws.split('.').next().unwrap(),
            forged_jws.split('.').nth(1).unwrap(),
            jws.split('.').nth(2).unwrap()
        );

        assert_eq!(
            verify_link_state(&spliced, &keys_of(&signer), &expected(NOW)),
            Err(PeerRequestRejection::BadSignature)
        );

        let stranger = VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            b"another-secret",
            KeyDerivationPurpose::Federation,
        )
        .unwrap();
        assert_eq!(
            verify_link_state(&jws, &keys_of(&stranger), &expected(NOW)),
            Err(PeerRequestRejection::BadSignature)
        );
    }
}
