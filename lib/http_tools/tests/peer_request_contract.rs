//! Cross-language contract for signed federation peer requests (D-9)
//!
//! Produces shared vectors from the real signer and verifier with fixed
//! TEST ONLY keys, a fixed clock and fixed `jti`s, so a peer written in
//! another language can check its implementation against the gateway's.
//! Ed25519 signatures are deterministic: the committed file must equal what
//! this code produces.
//!
//! Regenerate:
//!
//! ```bash
//! MYC_REGENERATE_PEER_REQUEST_FIXTURES=1 \
//!     cargo test -p mycelium-http-tools --test peer_request_contract
//! ```
//!
//! The keys are the RFC 8032 section 7.1 test vectors 1 and 2: public,
//! well-known material that must never be used operationally.

use chrono::Local;
use myc_core::domain::dtos::{
    federation::PeerKey, profile_signature::SigningKeyVersion,
};
use myc_http_tools::{
    federation::{
        sign_peer_request, verify_peer_request, ExpectedPeerRequest,
        PeerRequest, PeerRequestRejection, PEER_REQUEST_TYP,
    },
    models::VersionedEd25519Signer,
};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

const PRIVATE_PEM_V1: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
const PRIVATE_PEM_V2: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIEzNCJso/5banbbDRuwRTg9bijGfNaumJNqM9u1PuKb7
-----END PRIVATE KEY-----
";

const ISSUED_AT: u64 = 1_800_000_000;
const LIFETIME: u64 = 60;
const TOLERANCE: u64 = 30;
const METHOD: &str = "POST";
const PATH: &str = "/federation/resolve";
const BODY: &str = r#"{"subject":"opaque-subject-ref"}"#;
const FIXTURE_FILE: &str = "test/mock/federation-peer-request/vectors.json";

fn caller() -> Uuid {
    Uuid::from_u128(0x1111_1111_1111_4111_8111_1111_1111_1111)
}

fn callee() -> Uuid {
    Uuid::from_u128(0x2222_2222_2222_4222_8222_2222_2222_2222)
}

fn signer(version: u32, pem: &str) -> VersionedEd25519Signer {
    VersionedEd25519Signer::from_pem(
        SigningKeyVersion::try_from(version).unwrap(),
        pem,
    )
    .unwrap()
}

fn sign(signer: &VersionedEd25519Signer, jti: &str) -> String {
    sign_peer_request(
        signer,
        &PeerRequest {
            iss: caller(),
            aud: callee(),
            method: METHOD,
            path: PATH,
            body: BODY.as_bytes(),
            jti,
            issued_at: ISSUED_AT,
            lifetime_secs: LIFETIME,
        },
    )
    .unwrap()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Vector {
    name: &'static str,
    description: &'static str,
    jws: String,
    trusted_kids: Vec<&'static str>,
    audience: Uuid,
    method: &'static str,
    path: &'static str,
    body: &'static str,
    now_seconds: u64,
    expected: String,
}

struct Case {
    name: &'static str,
    description: &'static str,
    jws: String,
    trusted_kids: Vec<&'static str>,
    audience: Uuid,
    method: &'static str,
    path: &'static str,
    body: &'static str,
    now: u64,
}

fn case(name: &'static str, description: &'static str, jws: String) -> Case {
    Case {
        name,
        description,
        jws,
        trusted_kids: vec!["1", "2"],
        audience: callee(),
        method: METHOD,
        path: PATH,
        body: BODY,
        now: ISSUED_AT,
    }
}

fn cases() -> Vec<Case> {
    let v1 = signer(1, PRIVATE_PEM_V1);
    let v2 = signer(2, PRIVATE_PEM_V2);
    let token = sign(&v1, "00000000-0000-4000-8000-000000000001");

    vec![
        case("valid_v1", "Signed with key 1, same instant", token.clone()),
        case(
            "valid_v2_during_rotation",
            "Caller rotated to key 2; callee trusts 1 and 2",
            sign(&v2, "00000000-0000-4000-8000-000000000002"),
        ),
        Case {
            now: ISSUED_AT + LIFETIME + TOLERANCE,
            ..case(
                "valid_at_expiry_tolerance",
                "exp + tolerance",
                token.clone(),
            )
        },
        Case {
            now: ISSUED_AT + LIFETIME + TOLERANCE + 1,
            ..case("expired", "One second past exp + tolerance", token.clone())
        },
        Case {
            now: ISSUED_AT - TOLERANCE - 1,
            ..case("issued_in_future", "iat ahead of the clock", token.clone())
        },
        Case {
            audience: caller(),
            ..case("wrong_audience", "aud is another instance", token.clone())
        },
        Case {
            trusted_kids: vec!["2"],
            ..case("unknown_kid", "Key 1 no longer published", token.clone())
        },
        Case {
            body: r#"{"subject":"other-ref"}"#,
            ..case("body_changed", "Body differs from bh", token.clone())
        },
        Case {
            path: "/federation/tombstone",
            ..case("path_changed", "Path differs from htp", token.clone())
        },
        Case {
            method: "PUT",
            ..case("method_changed", "Method differs from htm", token.clone())
        },
        case(
            "signed_by_another_key",
            "kid 1 header, signature by key 2",
            resign_as_kid_1(&v2),
        ),
        case(
            "untyped_token",
            "EdDSA token without the peer request typ",
            v1.sign_claims(&json!({"iss": caller(), "aud": callee()}))
                .unwrap(),
        ),
    ]
}

/// A token whose header names kid 1 but whose signature is key 2's
fn resign_as_kid_1(v2: &VersionedEd25519Signer) -> String {
    let forged = sign(v2, "00000000-0000-4000-8000-000000000003");
    let honest = sign(&signer(1, PRIVATE_PEM_V1), "unused");
    let header_v1 = honest.split('.').next().unwrap();
    let mut parts = forged.splitn(2, '.');
    parts.next();

    format!("{header_v1}.{}", parts.next().unwrap())
}

fn trusted_keys(kids: &[&str]) -> Vec<PeerKey> {
    [signer(1, PRIVATE_PEM_V1), signer(2, PRIVATE_PEM_V2)]
        .into_iter()
        .filter(|key| kids.contains(&key.version().to_string().as_str()))
        .map(|key| PeerKey {
            peer_id: Uuid::nil(),
            kid: key.version().to_string(),
            alg: "EdDSA".to_string(),
            public_key: key.public_jwk().x(),
            fetched_at: Local::now(),
            expires_at: None,
        })
        .collect()
}

fn outcome(case: &Case) -> String {
    let result = verify_peer_request(
        &case.jws,
        &trusted_keys(&case.trusted_kids),
        &ExpectedPeerRequest {
            audience: case.audience,
            method: case.method,
            path: case.path,
            body: case.body.as_bytes(),
            now: case.now,
            tolerance_secs: TOLERANCE,
        },
    );

    match result {
        Ok(_) => "valid".to_string(),
        Err(PeerRequestRejection::UnknownKid(_)) => "unknown_kid".to_string(),
        Err(rejection) => format!("{rejection:?}"),
    }
}

fn render_fixture() -> String {
    let vectors: Vec<Vector> = cases()
        .into_iter()
        .map(|case| Vector {
            expected: outcome(&case),
            name: case.name,
            description: case.description,
            jws: case.jws,
            trusted_kids: case.trusted_kids,
            audience: case.audience,
            method: case.method,
            path: case.path,
            body: case.body,
            now_seconds: case.now,
        })
        .collect();
    let fixture = json!({
        "description": "Federation signed peer request contract vectors. Generated by the gateway (mycelium-http-tools tests/peer_request_contract.rs) from the real signer and verifier. TEST ONLY keys (RFC 8032 section 7.1 vectors 1 and 2). Do not edit by hand.",
        "contract": {
            "algorithm": "EdDSA",
            "typ": PEER_REQUEST_TYP,
            "claims": ["iss", "aud", "iat", "exp", "jti", "htm", "htp", "bh"],
            "bodyHash": "base64url(sha256(body)), no padding",
            "toleranceSeconds": TOLERANCE,
            "caller": caller(),
            "callee": callee(),
        },
        "publicKeys": {
            "1": signer(1, PRIVATE_PEM_V1).public_jwk().x(),
            "2": signer(2, PRIVATE_PEM_V2).public_jwk().x(),
        },
        "vectors": vectors,
    });

    let mut rendered = serde_json::to_string_pretty(&fixture).unwrap();
    rendered.push('\n');
    rendered
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_FILE)
}

#[test]
fn every_case_has_the_intended_outcome() {
    let outcomes: Vec<(String, String)> = cases()
        .iter()
        .map(|case| (case.name.to_string(), outcome(case)))
        .collect();
    let expected = [
        ("valid_v1", "valid"),
        ("valid_v2_during_rotation", "valid"),
        ("valid_at_expiry_tolerance", "valid"),
        ("expired", "Expired"),
        ("issued_in_future", "IssuedInFuture"),
        ("wrong_audience", "WrongAudience"),
        ("unknown_kid", "unknown_kid"),
        ("body_changed", "RequestMismatch"),
        ("path_changed", "RequestMismatch"),
        ("method_changed", "RequestMismatch"),
        ("signed_by_another_key", "BadSignature"),
        ("untyped_token", "WrongType"),
    ]
    .map(|(name, outcome)| (name.to_string(), outcome.to_string()));

    assert_eq!(outcomes, expected);
}

#[test]
fn committed_fixture_is_what_the_gateway_produces() {
    let rendered = render_fixture();

    if std::env::var("MYC_REGENERATE_PEER_REQUEST_FIXTURES").is_ok() {
        std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
        std::fs::write(fixture_path(), &rendered).unwrap();
    }

    let committed = std::fs::read_to_string(fixture_path())
        .expect("fixture missing: regenerate with MYC_REGENERATE_PEER_REQUEST_FIXTURES=1")
        .replace("\r\n", "\n");

    assert_eq!(committed, rendered, "fixture drifted: regenerate it");
}
