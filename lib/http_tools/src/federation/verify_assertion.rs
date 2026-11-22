use super::{
    read_jws_segments::decode_claims, verify_typed_jws::verify_typed_jws,
    AssertionClaims, ExpectedAssertion, PeerRequestRejection, ASSERTION_TYP,
};

use myc_core::domain::dtos::federation::PeerKey;

/// Verify a peer's assertion against its stored keys (FED-14)
///
/// Algorithm pinned to EdDSA, `typ` to `ASSERTION_TYP`; the issuer, the
/// audience, the subject and the lifetime must be the expected ones.
///
pub fn verify_assertion(
    jws: &str,
    keys: &[PeerKey],
    expected: &ExpectedAssertion,
) -> Result<AssertionClaims, PeerRequestRejection> {
    let (_, payload) = verify_typed_jws(jws, keys, ASSERTION_TYP)?;
    let claims: AssertionClaims = decode_claims(payload)?;

    check_binding(&claims, expected)?;
    check_lifetime(&claims, expected)?;

    Ok(claims)
}

fn check_binding(
    claims: &AssertionClaims,
    expected: &ExpectedAssertion,
) -> Result<(), PeerRequestRejection> {
    if claims.aud != expected.audience {
        return Err(PeerRequestRejection::WrongAudience);
    }
    if claims.iss != expected.issuer || claims.sub != expected.subject {
        return Err(PeerRequestRejection::WrongSubject);
    }

    Ok(())
}

fn check_lifetime(
    claims: &AssertionClaims,
    expected: &ExpectedAssertion,
) -> Result<(), PeerRequestRejection> {
    if claims.exp <= claims.iat {
        return Err(PeerRequestRejection::Malformed);
    }
    if expected.now >= claims.exp {
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
    use crate::federation::{sign_assertion, sign_link_state, LinkStateClaims};
    use crate::models::{KeyDerivationPurpose, VersionedEd25519Signer};

    use chrono::Local;
    use myc_core::domain::dtos::{
        federation::ProfileAssertion, profile_signature::SigningKeyVersion,
    };
    use uuid::Uuid;

    const NOW: u64 = 1_800_000_000;

    fn signer(secret: &[u8]) -> VersionedEd25519Signer {
        VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            secret,
            KeyDerivationPurpose::Federation,
        )
        .unwrap()
    }

    fn issuer() -> Uuid {
        Uuid::from_u128(0x1111_1111_1111_4111_8111_1111_1111_1111)
    }

    fn consumer() -> Uuid {
        Uuid::from_u128(0x2222_2222_2222_4222_8222_2222_2222_2222)
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

    fn claims() -> AssertionClaims {
        AssertionClaims {
            iss: issuer(),
            aud: consumer(),
            sub: "subject-ref-01".to_string(),
            iat: NOW,
            exp: NOW + 60,
            jti: Uuid::from_u128(7),
            kind: "profile.v1".to_string(),
            payload: ProfileAssertion::default(),
        }
    }

    fn expected(now: u64) -> ExpectedAssertion {
        ExpectedAssertion {
            issuer: issuer(),
            audience: consumer(),
            subject: "subject-ref-01".to_string(),
            now,
            tolerance_secs: 30,
        }
    }

    fn refusal(claims: &AssertionClaims, now: u64) -> PeerRequestRejection {
        let signer = signer(b"test-only-secret");
        let jws = sign_assertion(&signer, claims).unwrap();
        verify_assertion(&jws, &keys_of(&signer), &expected(now)).unwrap_err()
    }

    #[test]
    fn a_signed_assertion_round_trips() {
        let signer = signer(b"test-only-secret");
        let jws = sign_assertion(&signer, &claims()).unwrap();

        let verified =
            verify_assertion(&jws, &keys_of(&signer), &expected(NOW)).unwrap();

        assert_eq!(verified, claims());
    }

    #[test]
    fn a_foreign_audience_issuer_or_subject_is_refused() {
        let mut elsewhere = claims();
        elsewhere.aud = issuer();
        let mut other_issuer = claims();
        other_issuer.iss = consumer();
        let mut other_subject = claims();
        other_subject.sub = "subject-ref-02".to_string();

        assert_eq!(
            refusal(&elsewhere, NOW),
            PeerRequestRejection::WrongAudience
        );
        assert_eq!(
            refusal(&other_issuer, NOW),
            PeerRequestRejection::WrongSubject
        );
        assert_eq!(
            refusal(&other_subject, NOW),
            PeerRequestRejection::WrongSubject
        );
    }

    #[test]
    fn an_expired_or_future_assertion_is_refused() {
        assert_eq!(refusal(&claims(), NOW + 60), PeerRequestRejection::Expired);
        assert_eq!(
            refusal(&claims(), NOW - 31),
            PeerRequestRejection::IssuedInFuture
        );
    }

    #[test]
    fn a_link_state_or_a_foreign_key_is_refused() {
        let signer = signer(b"test-only-secret");
        let state = LinkStateClaims {
            iss: issuer(),
            aud: consumer(),
            iat: NOW,
            exp: NOW + 60,
            jti: Uuid::from_u128(7),
            email: "user-01@example.com".to_string(),
            return_url: "https://myc-02.example.com/done".to_string(),
        };
        let state_jws = sign_link_state(&signer, &state).unwrap();
        let jws = sign_assertion(&signer, &claims()).unwrap();
        let stranger = keys_of(&self::signer(b"another-secret"));

        assert_eq!(
            verify_assertion(&state_jws, &keys_of(&signer), &expected(NOW)),
            Err(PeerRequestRejection::WrongType)
        );
        assert_eq!(
            verify_assertion(&jws, &stranger, &expected(NOW)),
            Err(PeerRequestRejection::BadSignature)
        );
    }
}
