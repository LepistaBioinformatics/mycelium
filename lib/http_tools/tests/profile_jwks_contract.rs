//! Cross-language contract for the profile JWKS
//!
//! Produces `jwks-vectors.json`: the document the gateway really publishes
//! for the TEST ONLY keys 1 and 2 (`PublishedProfileKeys` built from
//! `VersionedEd25519Signer`), JWKS documents every verifier must accept or
//! refuse as a whole, and the `Cache-Control` lifetimes they must derive.
//! The Rust verifier runs the same corpus below; the SDKs copy the file.
//!
//! Regenerate (then copy to the SDKs, see the fixture README):
//!
//! ```bash
//! MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES=1 \
//!     cargo test -p mycelium-http-tools --test profile_jwks_contract
//! ```

use myc_core::domain::dtos::profile_signature::{
    JwksMaxAgeSeconds, SigningKeyVersion,
};
use myc_http_tools::{
    functions::read_jwks_cache_max_age,
    models::{
        PublishedProfileKeys, TrustedProfileKeys, VersionedEd25519Signer,
    },
    settings::{
        PROFILE_JWKS_DEFAULT_MAX_AGE_SECONDS, PROFILE_JWKS_EMPTY_RETRY_SECONDS,
        PROFILE_JWKS_FETCH_TIMEOUT_SECONDS, PROFILE_JWKS_MAX_BYTES,
        PROFILE_JWKS_MAX_MAX_AGE_SECONDS, PROFILE_JWKS_MIN_MAX_AGE_SECONDS,
        PROFILE_JWKS_PATH, PROFILE_JWKS_REFETCH_COOLDOWN_SECONDS,
    },
};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

const FIXTURE_FILE: &str = "test/mock/profile-signature/jwks-vectors.json";

/// TEST ONLY: RFC 8032 section 7.1 test vectors 1 and 2 (PKCS#8)
const PRIVATE_PEM_V1: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
const PRIVATE_PEM_V2: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIEzNCJso/5banbbDRuwRTg9bijGfNaumJNqM9u1PuKb7
-----END PRIVATE KEY-----
";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JwksVector {
    name: &'static str,
    description: &'static str,
    document: String,
    expected: &'static str,
    expected_kids: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheVector {
    cache_control: Option<&'static str>,
    expected_seconds: u64,
}

// ? ---------------------------------------------------------------------------
// ? Inputs
// ? ---------------------------------------------------------------------------

fn signer(version: u32, pem: &str) -> VersionedEd25519Signer {
    VersionedEd25519Signer::from_pem(
        SigningKeyVersion::try_from(version).unwrap(),
        pem,
    )
    .unwrap()
}

/// The exact body the gateway serves for keys 1 and 2
fn published_document() -> String {
    let keys = [
        signer(2, PRIVATE_PEM_V2).public_jwk().clone(),
        signer(1, PRIVATE_PEM_V1).public_jwk().clone(),
    ];

    match PublishedProfileKeys::new(&keys, JwksMaxAgeSeconds::default()) {
        PublishedProfileKeys::Enabled { body, .. } => body.to_string(),
        PublishedProfileKeys::Disabled => unreachable!(),
    }
}

fn published_keys() -> Vec<Value> {
    let document: Value = serde_json::from_str(&published_document()).unwrap();
    document["keys"].as_array().unwrap().clone()
}

/// Key 1 with one member replaced (`Value::Null` removes it)
fn key1_with(member: &str, value: Value) -> Value {
    let mut jwk = published_keys()[0].clone();
    let members = jwk.as_object_mut().unwrap();
    match value {
        Value::Null => members.remove(member),
        value => members.insert(member.to_string(), value),
    };
    jwk
}

fn without_optional_members() -> Value {
    let mut jwk = published_keys()[0].clone();
    let members = jwk.as_object_mut().unwrap();
    members.remove("use");
    members.remove("alg");
    jwk
}

fn set(keys: Vec<Value>) -> String {
    json!({ "keys": keys }).to_string()
}

// ? ---------------------------------------------------------------------------
// ? Catalog
// ? ---------------------------------------------------------------------------

fn valid(
    name: &'static str,
    description: &'static str,
    document: String,
    expected_kids: Vec<&'static str>,
) -> JwksVector {
    JwksVector {
        name,
        description,
        document,
        expected: "valid",
        expected_kids,
    }
}

fn refused(
    name: &'static str,
    description: &'static str,
    document: String,
) -> JwksVector {
    JwksVector {
        name,
        description,
        document,
        expected: "invalid_jwks",
        expected_kids: vec![],
    }
}

fn catalog() -> Vec<JwksVector> {
    let [key1, key2] =
        [published_keys()[0].clone(), published_keys()[1].clone()];
    let x1 = key1["x"].as_str().unwrap().to_string();
    let rsa = json!({"kty": "RSA", "kid": "3", "n": "AQAB", "e": "AQAB"});

    vec![
        valid(
            "valid_published",
            "Exact document published by the gateway for keys 1 and 2",
            published_document(),
            vec!["1", "2"],
        ),
        valid(
            "valid_single_key",
            "One key",
            set(vec![key1.clone()]),
            vec!["1"],
        ),
        valid(
            "valid_without_optional_members",
            "use and alg are optional",
            set(vec![without_optional_members()]),
            vec!["1"],
        ),
        valid(
            "valid_unknown_members_ignored",
            "Unknown members are ignored (RFC 7517)",
            set(vec![key1_with("extra", json!("ignored")), key2.clone()]),
            vec!["1", "2"],
        ),
        valid(
            "valid_unknown_top_level_members_ignored",
            "Members besides keys are ignored",
            json!({"keys": [key1.clone()], "extra": 1}).to_string(),
            vec!["1"],
        ),
        refused(
            "invalid_not_json",
            "Body is not JSON",
            "not json".to_string(),
        ),
        refused(
            "invalid_not_an_object",
            "Top level is an array",
            json!([key1.clone()]).to_string(),
        ),
        refused(
            "invalid_missing_keys",
            "No keys member",
            json!({}).to_string(),
        ),
        refused(
            "invalid_keys_not_array",
            "keys is not an array",
            json!({"keys": key1.clone()}).to_string(),
        ),
        refused("invalid_empty_keys", "keys is empty", set(vec![])),
        refused(
            "invalid_entry_not_object",
            "An entry is not an object",
            set(vec![key1.clone(), json!("x")]),
        ),
        refused(
            "invalid_private_member_d",
            "An entry carries private key material",
            set(vec![key1_with("d", json!(x1.clone()))]),
        ),
        refused(
            "invalid_kty_rsa",
            "Another key type",
            set(vec![key1_with("kty", json!("RSA"))]),
        ),
        refused(
            "invalid_kty_missing",
            "kty is required",
            set(vec![key1_with("kty", Value::Null)]),
        ),
        refused(
            "invalid_crv_x25519",
            "Another curve",
            set(vec![key1_with("crv", json!("X25519"))]),
        ),
        refused(
            "invalid_x_short",
            "x is not 32 bytes",
            set(vec![key1_with("x", json!("AAAA"))]),
        ),
        refused(
            "invalid_x_padded",
            "x must be Base64URL without padding",
            set(vec![key1_with("x", json!(format!("{x1}=")))]),
        ),
        refused(
            "invalid_x_standard_alphabet",
            "x must use the URL-safe alphabet",
            set(vec![key1_with("x", json!(x1.replace('_', "/")))]),
        ),
        refused(
            "invalid_x_missing",
            "x is required",
            set(vec![key1_with("x", Value::Null)]),
        ),
        refused(
            "invalid_kid_leading_zero",
            "kid must be canonical",
            set(vec![key1_with("kid", json!("01"))]),
        ),
        refused(
            "invalid_kid_zero",
            "kid 0 is reserved",
            set(vec![key1_with("kid", json!("0"))]),
        ),
        refused(
            "invalid_kid_number",
            "kid must be a string",
            set(vec![key1_with("kid", json!(1))]),
        ),
        refused(
            "invalid_kid_overflow",
            "kid must fit in u32",
            set(vec![key1_with("kid", json!("4294967296"))]),
        ),
        refused(
            "invalid_kid_missing",
            "kid is required",
            set(vec![key1_with("kid", Value::Null)]),
        ),
        refused(
            "invalid_duplicate_kid",
            "Two entries share a kid",
            set(vec![key1.clone(), key1_with("x", key2["x"].clone())]),
        ),
        refused(
            "invalid_use_enc",
            "use must be sig",
            set(vec![key1_with("use", json!("enc"))]),
        ),
        refused(
            "invalid_alg_rs256",
            "alg must be EdDSA",
            set(vec![key1_with("alg", json!("RS256"))]),
        ),
        refused(
            "invalid_one_bad_entry_among_good",
            "One foreign entry refuses the whole set",
            set(vec![key1, key2, rsa]),
        ),
    ]
}

fn cache_catalog() -> Vec<CacheVector> {
    [
        (None, 43_200),
        (Some("public, max-age=43200"), 43_200),
        (Some("public, max-age=600"), 600),
        (Some("Max-Age=600"), 600),
        (Some("max-age=1"), 60),
        (Some("max-age=999999"), 86_400),
        (Some("no-cache"), 43_200),
        (Some("max-age=abc"), 43_200),
    ]
    .into_iter()
    .map(|(cache_control, expected_seconds)| CacheVector {
        cache_control,
        expected_seconds,
    })
    .collect()
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

fn render_fixture() -> String {
    let fixture = json!({
        "description": "Profile JWKS contract vectors. Generated by the gateway (mycelium-http-tools tests/profile_jwks_contract.rs) from the real publisher. A document is accepted or refused as a whole. TEST ONLY keys (RFC 8032 section 7.1 vectors 1 and 2), the same keys as vectors.json. Do not edit by hand.",
        "contract": {
            "path": PROFILE_JWKS_PATH,
            "urlVariable": "MYCELIUM_JWKS_URL",
            "insecureHttpVariable": "MYCELIUM_JWKS_ALLOW_INSECURE_HTTP",
            "audienceVariable": "MYC_PROFILE_SIGNATURE_AUDIENCE",
            "fetchTimeoutSeconds": PROFILE_JWKS_FETCH_TIMEOUT_SECONDS,
            "maxBytes": PROFILE_JWKS_MAX_BYTES,
            "refetchCooldownSeconds": PROFILE_JWKS_REFETCH_COOLDOWN_SECONDS,
            "emptyRetrySeconds": PROFILE_JWKS_EMPTY_RETRY_SECONDS,
            "defaultMaxAgeSeconds": PROFILE_JWKS_DEFAULT_MAX_AGE_SECONDS,
            "minMaxAgeSeconds": PROFILE_JWKS_MIN_MAX_AGE_SECONDS,
            "maxMaxAgeSeconds": PROFILE_JWKS_MAX_MAX_AGE_SECONDS,
        },
        "published": published_document(),
        "vectors": catalog(),
        "cacheControl": cache_catalog(),
    });

    let mut rendered = serde_json::to_string_pretty(&fixture).unwrap();
    rendered.push('\n');
    rendered
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_FILE)
}

fn outcome(document: &str) -> (&'static str, Vec<String>) {
    match TrustedProfileKeys::from_jwks_document(document.as_bytes()) {
        Err(_) => ("invalid_jwks", vec![]),
        Ok(keys) => {
            let kids = ["1", "2", "3"]
                .into_iter()
                .filter(|kid| keys.key_for(kid).is_some())
                .map(str::to_string)
                .collect();
            ("valid", kids)
        }
    }
}

#[test]
fn rust_verifier_matches_every_jwks_vector() {
    for vector in catalog() {
        let (expected, kids) = outcome(&vector.document);

        assert_eq!(expected, vector.expected, "{}", vector.name);
        assert_eq!(kids, vector.expected_kids, "{}", vector.name);
    }
}

#[test]
fn rust_verifier_matches_every_cache_vector() {
    for vector in cache_catalog() {
        assert_eq!(
            read_jwks_cache_max_age(vector.cache_control).as_secs(),
            vector.expected_seconds,
            "{:?}",
            vector.cache_control
        );
    }
}

#[test]
fn published_document_never_carries_private_or_certificate_members() {
    for key in published_keys() {
        for member in ["d", "x5c", "x5u", "p", "q"] {
            assert!(key.get(member).is_none(), "{member}");
        }
    }
}

#[test]
fn vector_names_are_unique() {
    let mut names: Vec<&str> = catalog().iter().map(|v| v.name).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), total);
}

#[test]
fn committed_fixture_is_what_the_gateway_produces() {
    let rendered = render_fixture();

    if std::env::var("MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES").is_ok() {
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
