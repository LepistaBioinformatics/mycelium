//! Cross-language contract for the downstream profile signature
//!
//! Produces the shared vectors consumed by the Python and JavaScript SDKs
//! from the *real* gateway code paths (`compress_and_encode_profile_to_base64`
//! + `sign_profile_payload`) with fixed TEST ONLY keys and a fixed clock.
//! Ed25519 signatures are deterministic, so regenerating yields the same
//! bytes: the committed file must equal what this code produces.
//!
//! Regenerate (then copy to the SDKs, see the fixture README):
//!
//! ```bash
//! MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES=1 \
//!     cargo test -p mycelium-http-tools --test profile_signature_contract
//! ```
//!
//! The keys are the RFC 8032 section 7.1 test vectors 1 and 2: public,
//! well-known material that must never be used operationally.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{
    crypto::{sign, verify},
    Algorithm, DecodingKey, EncodingKey,
};
use myc_core::domain::dtos::{
    account::VerboseStatus,
    guest_role::Permission,
    profile::{LicensedResource, LicensedResources, Owner, Profile},
    profile_signature::{
        ProfileHeaderHash, ProfileSignatureAudience, SigningKeyVersion,
        UnixTimestampSeconds, MAX_SAFE_TIMESTAMP_SECONDS,
    },
};
use myc_http_tools::{
    functions::{
        compress_and_encode_profile_to_base64,
        decode_and_decompress_profile_from_base64, sign_profile_payload,
    },
    models::VersionedEd25519Signer,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};
use uuid::Uuid;

const PRIVATE_PEM_V1: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
const PUBLIC_PEM_V1: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";
const PRIVATE_PEM_V2: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIEzNCJso/5banbbDRuwRTg9bijGfNaumJNqM9u1PuKb7
-----END PRIVATE KEY-----
";
const PUBLIC_PEM_V2: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAPUAXw+hDiVqStwqnTRt+vJyYLM8uxJaMwM1V8Sr0Zgw=
-----END PUBLIC KEY-----
";

const ISSUED_AT: u64 = 1_800_000_000;
const TOLERANCE: u64 = 60;
const MAX_SIGNATURE_BYTES: usize = 4096;
const AUDIENCE: &str = "billing-api";
const FIXTURE_FILE: &str = "test/mock/profile-signature/vectors.json";

// ? ---------------------------------------------------------------------------
// ? Fixture model
// ? ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Vector {
    name: &'static str,
    description: &'static str,
    profile_header: String,
    signature: String,
    trusted_key_versions: Vec<&'static str>,
    now_seconds: u64,
    tolerance_seconds: u64,
    expected_audience: Option<&'static str>,
    expected: &'static str,
}

struct Inputs {
    profile_header: String,
    other_profile_header: String,
    undecodable_profile_header: String,
}

fn version(value: u32) -> SigningKeyVersion {
    SigningKeyVersion::try_from(value).unwrap()
}

fn signer(value: u32, pem: &str) -> VersionedEd25519Signer {
    VersionedEd25519Signer::from_pem(version(value), pem).unwrap()
}

fn synthetic_profile(account: u128, is_staff: bool) -> Profile {
    let owner = Owner {
        id: Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0001),
        email: "alice@example.test".to_string(),
        first_name: Some("Alice".to_string()),
        last_name: Some("Synthetic".to_string()),
        username: Some("alice".to_string()),
        is_principal: true,
    };

    Profile::new(
        vec![owner],
        Uuid::from_u128(account),
        false,
        false,
        is_staff,
        true,
        true,
        true,
        false,
        false,
        Some(VerboseStatus::Verified),
        None,
        None,
    )
}

fn inputs() -> Inputs {
    let primary =
        synthetic_profile(0x1111_1111_1111_4111_8111_1111_1111_1111, true);
    let other =
        synthetic_profile(0x2222_2222_2222_4222_8222_2222_2222_2222, false);

    Inputs {
        profile_header: compress_and_encode_profile_to_base64(primary).unwrap(),
        other_profile_header: compress_and_encode_profile_to_base64(other)
            .unwrap(),
        undecodable_profile_header: "bm90LWEtenN0ZC1wYXlsb2Fk".to_string(),
    }
}

// ? ---------------------------------------------------------------------------
// ? Token builders
// ? ---------------------------------------------------------------------------

fn issue(
    profile_header: &str,
    signer: &VersionedEd25519Signer,
    issued_at: u64,
) -> String {
    sign_profile_payload(
        profile_header,
        ProfileSignatureAudience::try_from(AUDIENCE.to_string()).unwrap(),
        signer,
        UnixTimestampSeconds::try_from(issued_at).unwrap(),
    )
    .unwrap()
}

fn encode_segment(value: &Value) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap())
}

/// Sign an arbitrary header/claims pair, for validly signed hostile tokens
fn sign_raw(header: Value, claims: Value, pem: &str, alg: Algorithm) -> String {
    let message =
        format!("{}.{}", encode_segment(&header), encode_segment(&claims));
    let key = match alg {
        Algorithm::HS256 => EncodingKey::from_secret(PUBLIC_PEM_V1.as_bytes()),
        _ => EncodingKey::from_ed_pem(pem.as_bytes()).unwrap(),
    };

    format!("{message}.{}", sign(message.as_bytes(), &key, alg).unwrap())
}

fn valid_claims(profile_header: &str) -> Value {
    json!({
        "iat": ISSUED_AT,
        "ph": ProfileHeaderHash::from_profile_header(profile_header).as_str(),
        "aud": AUDIENCE,
    })
}

fn eddsa_header(kid: &str) -> Value {
    json!({ "typ": "JWT", "alg": "EdDSA", "kid": kid })
}

fn with_member(mut base: Value, key: &str, value: Value) -> Value {
    base[key] = value;
    base
}

fn without_member(mut base: Value, key: &str) -> Value {
    base.as_object_mut().unwrap().remove(key);
    base
}

fn swap_payload(token: &str, donor: &str) -> String {
    let parts: Vec<&str> = token.split('.').collect();
    let donor_parts: Vec<&str> = donor.split('.').collect();

    format!("{}.{}.{}", parts[0], donor_parts[1], parts[2])
}

// ? ---------------------------------------------------------------------------
// ? Vector catalog
// ? ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn vector(
    name: &'static str,
    description: &'static str,
    profile_header: &str,
    signature: String,
    trusted_key_versions: Vec<&'static str>,
    now_seconds: u64,
    expected_audience: Option<&'static str>,
    expected: &'static str,
) -> Vector {
    Vector {
        name,
        description,
        profile_header: profile_header.to_string(),
        signature,
        trusted_key_versions,
        now_seconds,
        tolerance_seconds: TOLERANCE,
        expected_audience,
        expected,
    }
}

fn catalog() -> Vec<Vector> {
    let inputs = inputs();
    let header = inputs.profile_header.as_str();
    let v1 = signer(1, PRIVATE_PEM_V1);
    let v2 = signer(2, PRIVATE_PEM_V2);
    let token_v1 = issue(header, &v1, ISSUED_AT);
    let token_v2 = issue(header, &v2, ISSUED_AT);

    let mut vectors = acceptance_vectors(&inputs, &token_v1, &token_v2);
    vectors.extend(rejection_vectors(&inputs, &token_v1, &token_v2));
    vectors.extend(hostile_header_vectors(header));
    vectors.extend(hostile_claims_vectors(header));
    vectors.push(licensed_wire_vector(&v1));
    vectors
}

fn licensed_wire_vector(signer: &VersionedEd25519Signer) -> Vector {
    let mut profile =
        synthetic_profile(0x1111_1111_1111_4111_8111_1111_1111_1111, true);
    profile.verbose_status = None;
    profile.licensed_resources = Some(LicensedResources::Records(
        [Permission::Read, Permission::Write]
            .into_iter()
            .map(|perm| LicensedResource {
                acc_id: profile.acc_id,
                sys_acc: false,
                tenant_id: Uuid::nil(),
                acc_name: "Synthetic license".to_string(),
                role: "synthetic-role".to_string(),
                role_id: Uuid::from_u128(
                    0x3333_3333_3333_4333_8333_3333_3333_3333,
                ),
                perm,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            })
            .collect(),
    ));
    let header = compress_and_encode_profile_to_base64(profile).unwrap();
    vector(
        "valid_rust_permissions_and_null_status",
        "Real Rust wire strings read/write, nil tenant and null verboseStatus",
        &header,
        issue(&header, signer, ISSUED_AT),
        vec!["1"],
        ISSUED_AT,
        Some(AUDIENCE),
        "valid",
    )
}

fn acceptance_vectors(
    inputs: &Inputs,
    token_v1: &str,
    token_v2: &str,
) -> Vec<Vector> {
    let header = inputs.profile_header.as_str();
    let both = || vec!["1", "2"];

    vec![
        vector(
            "valid_v1",
            "Gateway signature with key 1, same instant",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT,
            None,
            "valid",
        ),
        vector(
            "valid_v2_during_rotation",
            "Primary rotated to key 2; consumer trusts 1 and 2",
            header,
            token_v2.into(),
            both(),
            ISSUED_AT,
            None,
            "valid",
        ),
        vector(
            "valid_v1_during_rotation",
            "Old gateway replica still on key 1; consumer trusts 1 and 2",
            header,
            token_v1.into(),
            both(),
            ISSUED_AT,
            None,
            "valid",
        ),
        vector(
            "tolerance_past_boundary",
            "Verified exactly 60 seconds after issuance",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT + TOLERANCE,
            None,
            "valid",
        ),
        vector(
            "tolerance_future_boundary",
            "Verifier clock exactly 60 seconds behind the gateway",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT - TOLERANCE,
            None,
            "valid",
        ),
        vector(
            "replay_inside_window",
            "Same pair verified again 30 seconds later (documented limitation)",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT + 30,
            None,
            "valid",
        ),
        vector(
            "audience_matches",
            "Consumer configured with the audience it was signed for",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT,
            Some(AUDIENCE),
            "valid",
        ),
    ]
}

fn rejection_vectors(
    inputs: &Inputs,
    token_v1: &str,
    token_v2: &str,
) -> Vec<Vector> {
    let header = inputs.profile_header.as_str();
    let v1 = signer(1, PRIVATE_PEM_V1);
    let wrong_key_same_kid = signer(1, PRIVATE_PEM_V2);
    let other_token = issue(&inputs.other_profile_header, &v1, ISSUED_AT);
    let undecodable = &inputs.undecodable_profile_header;

    vec![
        vector(
            "timestamp_too_old",
            "Verified 61 seconds after issuance",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT + TOLERANCE + 1,
            None,
            "timestamp_out_of_tolerance",
        ),
        vector(
            "timestamp_in_future",
            "Verifier clock 61 seconds behind the gateway",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT - TOLERANCE - 1,
            None,
            "timestamp_out_of_tolerance",
        ),
        vector(
            "profile_swapped",
            "Valid signature of another profile sent with this profile",
            header,
            other_token.clone(),
            vec!["1"],
            ISSUED_AT,
            None,
            "profile_hash_mismatch",
        ),
        vector(
            "profile_appended_byte",
            "Profile header altered by one trailing byte",
            &format!("{header}A"),
            token_v1.into(),
            vec!["1"],
            ISSUED_AT,
            None,
            "profile_hash_mismatch",
        ),
        vector(
            "unknown_key_version",
            "Signed with key 2; consumer only trusts key 1",
            header,
            token_v2.into(),
            vec!["1"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
        vector(
            "retired_key_version",
            "Signed with key 1 after the consumer removed it",
            header,
            token_v1.into(),
            vec!["2"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
        vector(
            "payload_swapped",
            "Claims of another token under this signature",
            header,
            swap_payload(token_v1, &other_token),
            vec!["1"],
            ISSUED_AT,
            None,
            "invalid_signature",
        ),
        vector(
            "wrong_key_same_kid",
            "Key 2 material presented as kid 1",
            header,
            issue(header, &wrong_key_same_kid, ISSUED_AT),
            vec!["1"],
            ISSUED_AT,
            None,
            "invalid_signature",
        ),
        vector(
            "audience_mismatch",
            "Pair signed for billing-api replayed to another service",
            header,
            token_v1.into(),
            vec!["1"],
            ISSUED_AT,
            Some("reports-api"),
            "audience_mismatch",
        ),
        vector(
            "profile_not_decodable",
            "Valid signature over a header that is not a Mycelium profile",
            undecodable,
            issue(undecodable, &v1, ISSUED_AT),
            vec!["1"],
            ISSUED_AT,
            None,
            "invalid_profile",
        ),
        vector(
            "empty_signature",
            "Signature header present but empty",
            header,
            String::new(),
            vec!["1"],
            ISSUED_AT,
            None,
            "missing_signature",
        ),
        vector(
            "two_segments",
            "Not a compact JWS",
            header,
            token_v1.rsplitn(2, '.').nth(1).unwrap().into(),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "oversized_signature",
            "Signature header above 4096 bytes",
            header,
            format!("{}.{}.{}", "a".repeat(2048), "b".repeat(2048), "c"),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "header_not_json",
            "Protected header is not Base64URL JSON",
            header,
            format!("bm90LWpzb24.{}", token_v1.split_once('.').unwrap().1),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
    ]
}

fn hostile_header_vectors(header: &str) -> Vec<Vector> {
    let claims = || valid_claims(header);
    let ed = |h: Value| sign_raw(h, claims(), PRIVATE_PEM_V1, Algorithm::EdDSA);
    let none_token = format!(
        "{}.{}.",
        encode_segment(&json!({"alg": "none", "kid": "1"})),
        encode_segment(&claims())
    );

    vec![
        vector(
            "alg_none",
            "Unsigned token with alg none",
            header,
            none_token,
            vec!["1"],
            ISSUED_AT,
            None,
            "unsupported_algorithm",
        ),
        vector(
            "alg_hs256_confusion",
            "HS256 keyed with the public key PEM",
            header,
            sign_raw(
                json!({"alg": "HS256", "kid": "1"}),
                claims(),
                PRIVATE_PEM_V1,
                Algorithm::HS256,
            ),
            vec!["1"],
            ISSUED_AT,
            None,
            "unsupported_algorithm",
        ),
        vector(
            "alg_ed25519_identifier",
            "Fully-specified Ed25519 alg is not this contract",
            header,
            ed(with_member(eddsa_header("1"), "alg", json!("Ed25519"))),
            vec!["1"],
            ISSUED_AT,
            None,
            "unsupported_algorithm",
        ),
        vector(
            "crit_header",
            "Protected header declares crit",
            header,
            ed(with_member(eddsa_header("1"), "crit", json!(["exp"]))),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "b64_header",
            "Protected header declares b64",
            header,
            ed(with_member(eddsa_header("1"), "b64", json!(false))),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "jku_header",
            "Protected header points to a remote key set",
            header,
            ed(with_member(
                eddsa_header("1"),
                "jku",
                json!("https://attacker.example/jwks"),
            )),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "jwk_header",
            "Protected header embeds a key",
            header,
            ed(with_member(
                eddsa_header("1"),
                "jwk",
                json!({"kty": "OKP", "crv": "Ed25519", "x": "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"}),
            )),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "x5u_header",
            "Protected header points to a remote certificate",
            header,
            ed(with_member(
                eddsa_header("1"),
                "x5u",
                json!("https://attacker.example/cert"),
            )),
            vec!["1"],
            ISSUED_AT,
            None,
            "malformed_signature",
        ),
        vector(
            "kid_missing",
            "No kid in the protected header",
            header,
            ed(without_member(eddsa_header("1"), "kid")),
            vec!["1"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
        vector(
            "kid_not_canonical",
            "kid with a leading zero",
            header,
            ed(eddsa_header("01")),
            vec!["1"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
        vector(
            "kid_numeric",
            "kid as a JSON number instead of a string",
            header,
            ed(with_member(eddsa_header("1"), "kid", json!(1))),
            vec!["1"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
        vector(
            "kid_path_traversal",
            "kid crafted as a path",
            header,
            ed(eddsa_header("../1")),
            vec!["1"],
            ISSUED_AT,
            None,
            "unknown_key",
        ),
    ]
}

fn hostile_claims_vectors(header: &str) -> Vec<Vector> {
    let signed = |claims: Value| {
        sign_raw(eddsa_header("1"), claims, PRIVATE_PEM_V1, Algorithm::EdDSA)
    };
    let base = || valid_claims(header);
    let case = |name, description, claims| {
        vector(
            name,
            description,
            header,
            signed(claims),
            vec!["1"],
            ISSUED_AT,
            None,
            "invalid_claims",
        )
    };

    vec![
        case(
            "iat_missing",
            "Signed claims without iat",
            without_member(base(), "iat"),
        ),
        case(
            "iat_string",
            "iat as a string",
            with_member(base(), "iat", json!(ISSUED_AT.to_string())),
        ),
        case(
            "iat_fractional",
            "iat with a fraction",
            with_member(base(), "iat", json!(ISSUED_AT as f64 + 0.5)),
        ),
        case(
            "iat_negative",
            "Negative iat",
            with_member(base(), "iat", json!(-1)),
        ),
        case(
            "iat_boolean",
            "iat as a boolean",
            with_member(base(), "iat", json!(true)),
        ),
        case(
            "iat_unsafe_integer",
            "iat above 2^53 - 1",
            with_member(base(), "iat", json!(MAX_SAFE_TIMESTAMP_SECONDS + 1)),
        ),
        case(
            "ph_missing",
            "Signed claims without ph",
            without_member(base(), "ph"),
        ),
        case(
            "ph_padded",
            "ph with Base64 padding",
            with_member(
                base(),
                "ph",
                json!(format!(
                    "{}=",
                    ProfileHeaderHash::from_profile_header(header).as_str()
                )),
            ),
        ),
        case(
            "ph_wrong_length",
            "ph that is not a SHA-256",
            with_member(base(), "ph", json!("c2hvcnQ")),
        ),
        case(
            "aud_missing",
            "Signed claims without aud",
            without_member(base(), "aud"),
        ),
        case(
            "aud_empty",
            "Empty aud",
            with_member(base(), "aud", json!("")),
        ),
        case(
            "aud_array",
            "aud as an array",
            with_member(base(), "aud", json!([AUDIENCE])),
        ),
    ]
}

// ? ---------------------------------------------------------------------------
// ? Rust reference verifier (same precedence as the SDKs)
// ? ---------------------------------------------------------------------------

const FORBIDDEN_HEADER_MEMBERS: [&str; 6] =
    ["crit", "b64", "jku", "jwk", "x5u", "x5c"];

fn public_keys() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([("1", PUBLIC_PEM_V1), ("2", PUBLIC_PEM_V2)])
}

fn decode_json(segment: &str) -> Option<Value> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()
        .filter(Value::is_object)
}

fn reference_verify(vector: &Vector) -> &'static str {
    let token = vector.signature.as_str();
    if token.is_empty() {
        return "missing_signature";
    }

    let parts: Vec<&str> = token.split('.').collect();
    if token.len() > MAX_SIGNATURE_BYTES || parts.len() != 3 {
        return "malformed_signature";
    }

    let Some(header) = decode_json(parts[0]) else {
        return "malformed_signature";
    };

    if header["alg"] != "EdDSA" {
        return "unsupported_algorithm";
    }

    if FORBIDDEN_HEADER_MEMBERS
        .iter()
        .any(|member| header.get(member).is_some())
    {
        return "malformed_signature";
    }

    let kid = header["kid"].as_str().unwrap_or_default();
    let is_trusted = SigningKeyVersion::from_str(kid).is_ok()
        && vector
            .trusted_key_versions
            .iter()
            .any(|trusted| *trusted == kid);
    if !is_trusted {
        return "unknown_key";
    }

    reference_verify_signed(vector, &parts, public_keys()[kid])
}

fn reference_verify_signed(
    vector: &Vector,
    parts: &[&str],
    public_pem: &str,
) -> &'static str {
    let key = DecodingKey::from_ed_pem(public_pem.as_bytes()).unwrap();
    let message = format!("{}.{}", parts[0], parts[1]);
    let signature_ok =
        verify(parts[2], message.as_bytes(), &key, Algorithm::EdDSA)
            .unwrap_or(false);
    if !signature_ok {
        return "invalid_signature";
    }

    let Some(claims) = decode_json(parts[1]) else {
        return "invalid_claims";
    };

    let iat = claims["iat"]
        .as_u64()
        .filter(|iat| *iat <= MAX_SAFE_TIMESTAMP_SECONDS);
    let ph = claims["ph"]
        .as_str()
        .map(|ph| ProfileHeaderHash::try_from(ph.to_string()));
    let aud = claims["aud"].as_str().filter(|aud| !aud.is_empty());
    let (Some(iat), Some(Ok(ph)), Some(aud)) = (iat, ph, aud) else {
        return "invalid_claims";
    };

    if vector.now_seconds.abs_diff(iat) > vector.tolerance_seconds {
        return "timestamp_out_of_tolerance";
    }

    if ph != ProfileHeaderHash::from_profile_header(&vector.profile_header) {
        return "profile_hash_mismatch";
    }

    if vector
        .expected_audience
        .is_some_and(|expected| expected != aud)
    {
        return "audience_mismatch";
    }

    match decode_and_decompress_profile_from_base64(
        vector.profile_header.clone(),
    ) {
        Ok(_) => "valid",
        Err(_) => "invalid_profile",
    }
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

fn render_fixture() -> String {
    let fixture = json!({
        "description": "Downstream profile signature contract vectors. Generated by the gateway (mycelium-http-tools tests/profile_signature_contract.rs) from the real encoder and signer. TEST ONLY keys (RFC 8032 section 7.1 vectors 1 and 2). Do not edit by hand.",
        "contract": {
            "algorithm": "EdDSA",
            "profileHeader": "x-mycelium-profile",
            "signatureHeader": "x-mycelium-profile-signature",
            "maxSignatureBytes": MAX_SIGNATURE_BYTES,
            "defaultToleranceSeconds": TOLERANCE,
            "reasonPrecedence": [
                "missing_signature", "malformed_signature", "unsupported_algorithm",
                "malformed_signature (crit/b64/jku/jwk/x5u/x5c)", "unknown_key",
                "invalid_signature", "invalid_claims", "timestamp_out_of_tolerance",
                "profile_hash_mismatch", "audience_mismatch", "invalid_profile"
            ],
        },
        "publicKeys": public_keys(),
        "expectedValidProfile": {
            "accId": "11111111-1111-4111-8111-111111111111",
            "isStaff": true,
        },
        "vectors": catalog(),
    });

    let mut rendered = serde_json::to_string_pretty(&fixture).unwrap();
    rendered.push('\n');
    rendered
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_FILE)
}

#[test]
fn reference_verifier_matches_every_expected_reason() {
    for vector in catalog() {
        assert_eq!(
            reference_verify(&vector),
            vector.expected,
            "vector {}",
            vector.name
        );
    }
}

#[test]
fn vector_names_are_unique() {
    let vectors = catalog();
    let mut names: Vec<&str> =
        vectors.iter().map(|vector| vector.name).collect();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), vectors.len());
}

#[test]
fn signatures_stay_far_below_the_size_limit() {
    let issued: Vec<Vector> = catalog()
        .into_iter()
        .filter(|vector| vector.expected == "valid")
        .collect();

    assert!(issued.iter().all(|vector| vector.signature.len() < 512));
}

#[test]
fn valid_profile_decodes_to_the_synthetic_account() {
    let profile =
        decode_and_decompress_profile_from_base64(inputs().profile_header)
            .unwrap();

    assert_eq!(
        profile.acc_id.to_string(),
        "11111111-1111-4111-8111-111111111111"
    );
    assert!(profile.is_staff);
}

#[test]
fn committed_fixture_is_what_the_gateway_produces() {
    let rendered = render_fixture();

    if std::env::var("MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES").is_ok() {
        std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
        std::fs::write(fixture_path(), &rendered).unwrap();
    }

    let committed = std::fs::read_to_string(fixture_path())
        .expect("fixture missing: regenerate with MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES=1")
        .replace("\r\n", "\n");

    assert_eq!(
        committed, rendered,
        "fixture drifted: regenerate and copy to the SDKs"
    );
}
