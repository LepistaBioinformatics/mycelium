use super::{
    body_hash, read_jws_segments::decode_claims,
    verify_typed_jws::verify_typed_jws, ExpectedPeerRequest, PeerRequestClaims,
    PeerRequestRejection, VerifiedPeerRequest, PEER_REQUEST_TYP,
};

use myc_core::domain::dtos::federation::PeerKey;

/// Verify a signed peer request against the caller's stored keys
///
/// The algorithm is pinned to EdDSA and the `typ` to `PEER_REQUEST_TYP`. The
/// `jti` is returned, not checked: single use is the caller's store (D-9).
///
pub fn verify_peer_request(
    jws: &str,
    keys: &[PeerKey],
    expected: &ExpectedPeerRequest,
) -> Result<VerifiedPeerRequest, PeerRequestRejection> {
    let (kid, payload) = verify_typed_jws(jws, keys, PEER_REQUEST_TYP)?;
    let claims: PeerRequestClaims = decode_claims(payload)?;

    check_claims(&claims, expected)?;

    Ok(VerifiedPeerRequest { kid, claims })
}

fn check_claims(
    claims: &PeerRequestClaims,
    expected: &ExpectedPeerRequest,
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

    check_request(claims, expected)
}

fn check_request(
    claims: &PeerRequestClaims,
    expected: &ExpectedPeerRequest,
) -> Result<(), PeerRequestRejection> {
    let same = claims.htm.eq_ignore_ascii_case(expected.method)
        && claims.htp == expected.path
        && claims.bh == body_hash(expected.body);

    match same {
        true => Ok(()),
        false => Err(PeerRequestRejection::RequestMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::{
        peek_peer_request_issuer, sign_peer_request, PeerRequest,
    };
    use crate::models::{KeyDerivationPurpose, VersionedEd25519Signer};
    use uuid::Uuid;

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use chrono::Local;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;

    const NOW: u64 = 1_800_000_000;
    const TOLERANCE: u64 = 30;
    const BODY: &[u8] = br#"{"subject":"opaque-ref"}"#;
    const PATH: &str = "/federation/resolve";

    fn signer(purpose: KeyDerivationPurpose) -> VersionedEd25519Signer {
        VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            b"test-only-secret",
            purpose,
        )
        .unwrap()
    }

    fn caller() -> Uuid {
        Uuid::from_u128(0x1111_1111_1111_4111_8111_1111_1111_1111)
    }

    fn callee() -> Uuid {
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

    fn request() -> PeerRequest<'static> {
        PeerRequest {
            iss: caller(),
            aud: callee(),
            method: "POST",
            path: PATH,
            body: BODY,
            jti: "00000000-0000-4000-8000-000000000001",
            issued_at: NOW,
            lifetime_secs: 60,
        }
    }

    fn expected() -> ExpectedPeerRequest<'static> {
        ExpectedPeerRequest {
            audience: callee(),
            method: "POST",
            path: PATH,
            body: BODY,
            now: NOW,
            tolerance_secs: TOLERANCE,
        }
    }

    fn signed() -> (String, Vec<PeerKey>) {
        let signer = signer(KeyDerivationPurpose::Federation);
        let jws = sign_peer_request(&signer, &request()).unwrap();

        (jws, keys_of(&signer))
    }

    fn verify(
        jws: &str,
        keys: &[PeerKey],
        expected: ExpectedPeerRequest,
    ) -> Result<VerifiedPeerRequest, PeerRequestRejection> {
        verify_peer_request(jws, keys, &expected)
    }

    #[test]
    fn a_signed_request_verifies_and_exposes_its_claims() {
        let (jws, keys) = signed();

        let verified = verify(&jws, &keys, expected()).unwrap();

        assert_eq!(verified.kid, "1");
        assert_eq!(verified.claims.iss, caller());
        assert_eq!(verified.claims.jti, request().jti);
        assert_eq!(verified.claims.exp, NOW + 60);
        assert_eq!(peek_peer_request_issuer(&jws), Ok((caller(), "1".into())));
    }

    #[test]
    fn the_method_is_compared_case_insensitively() {
        let (jws, keys) = signed();

        assert!(verify(
            &jws,
            &keys,
            ExpectedPeerRequest {
                method: "post",
                ..expected()
            }
        )
        .is_ok());
    }

    #[test]
    fn rejects_the_wrong_audience() {
        let (jws, keys) = signed();

        assert_eq!(
            verify(
                &jws,
                &keys,
                ExpectedPeerRequest {
                    audience: caller(),
                    ..expected()
                }
            ),
            Err(PeerRequestRejection::WrongAudience)
        );
    }

    #[test]
    fn rejects_expired_past_the_tolerance_only() {
        let (jws, keys) = signed();
        let at = |now| {
            verify(&jws, &keys, ExpectedPeerRequest { now, ..expected() })
        };

        assert!(at(NOW + 60 + TOLERANCE).is_ok());
        assert_eq!(
            at(NOW + 60 + TOLERANCE + 1),
            Err(PeerRequestRejection::Expired)
        );
    }

    #[test]
    fn rejects_iat_in_the_future_past_the_tolerance_only() {
        let (jws, keys) = signed();
        let at = |now| {
            verify(&jws, &keys, ExpectedPeerRequest { now, ..expected() })
        };

        assert!(at(NOW - TOLERANCE).is_ok());
        assert_eq!(
            at(NOW - TOLERANCE - 1),
            Err(PeerRequestRejection::IssuedInFuture)
        );
    }

    #[test]
    fn rejects_an_unknown_kid() {
        let (jws, mut keys) = signed();
        keys[0].kid = "2".to_string();

        assert_eq!(
            verify(&jws, &keys, expected()),
            Err(PeerRequestRejection::UnknownKid("1".to_string()))
        );
    }

    #[test]
    fn rejects_a_changed_body_path_or_method() {
        let (jws, keys) = signed();

        for changed in [
            ExpectedPeerRequest {
                body: br#"{"subject":"other-ref"}"#,
                ..expected()
            },
            ExpectedPeerRequest {
                path: "/federation/tombstone",
                ..expected()
            },
            ExpectedPeerRequest {
                method: "PUT",
                ..expected()
            },
        ] {
            assert_eq!(
                verify(&jws, &keys, changed),
                Err(PeerRequestRejection::RequestMismatch)
            );
        }
    }

    #[test]
    fn rejects_an_algorithm_other_than_eddsa() {
        let (_, keys) = signed();
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("1".to_string());
        header.typ = Some(PEER_REQUEST_TYP.to_string());
        let claims = serde_json::to_value(PeerRequestClaims {
            iss: caller(),
            aud: callee(),
            iat: NOW,
            exp: NOW + 60,
            jti: "x".to_string(),
            htm: "POST".to_string(),
            htp: PATH.to_string(),
            bh: String::new(),
        })
        .unwrap();
        let forged =
            encode(&header, &claims, &EncodingKey::from_secret(b"k")).unwrap();

        assert_eq!(
            verify(&forged, &keys, expected()),
            Err(PeerRequestRejection::UnsupportedAlgorithm)
        );
    }

    #[test]
    fn rejects_a_signature_from_the_profile_signing_key() {
        let federation = signer(KeyDerivationPurpose::Federation);
        let profile = signer(KeyDerivationPurpose::ProfileSigning);
        let jws = sign_peer_request(&profile, &request()).unwrap();

        assert_eq!(
            verify(&jws, &keys_of(&federation), expected()),
            Err(PeerRequestRejection::BadSignature)
        );
    }

    #[test]
    fn rejects_a_token_of_another_type() {
        let signer = signer(KeyDerivationPurpose::Federation);
        let claims = PeerRequestClaims {
            iss: caller(),
            aud: callee(),
            iat: NOW,
            exp: NOW + 60,
            jti: "x".to_string(),
            htm: "POST".to_string(),
            htp: PATH.to_string(),
            bh: URL_SAFE_NO_PAD.encode([0u8; 32]),
        };
        let untyped = signer.sign_claims(&claims).unwrap();

        assert_eq!(
            verify(&untyped, &keys_of(&signer), expected()),
            Err(PeerRequestRejection::WrongType)
        );
    }

    #[test]
    fn rejects_garbage() {
        let (_, keys) = signed();

        for jws in ["", "a.b", "a.b.c", "....."] {
            assert!(verify(jws, &keys, expected()).is_err(), "{jws}");
        }
        assert_eq!(
            peek_peer_request_issuer("not-a-jws"),
            Err(PeerRequestRejection::Malformed)
        );
    }
}
