//! The Rust downstream verifier against the shared contract vectors
//!
//! `vectors.json` is produced by the gateway's real encoder and signer (see
//! `profile_signature_contract.rs`). The Python, JavaScript and Go SDKs run
//! the same corpus; this test proves `verify_and_decode_profile`, used by
//! `GatewayProfileData`, reports the same outcome for every case.

use base64::{engine::general_purpose::STANDARD, Engine};
use myc_core::domain::dtos::profile_signature::{
    ProfileSignatureAudience, SigningKeyVersion, UnixTimestampSeconds,
};
use myc_http_tools::{
    functions::verify_and_decode_profile,
    models::{
        Ed25519PublicJwk, ProfileSignatureTolerance, ProfileSignatureVerifier,
        TrustedProfileKeys,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};

const VECTORS: &str =
    include_str!("../test/mock/profile-signature/vectors.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Corpus {
    public_keys: BTreeMap<String, String>,
    vectors: Vec<Vector>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vector {
    name: String,
    profile_header: String,
    signature: String,
    trusted_key_versions: Vec<String>,
    now_seconds: u64,
    tolerance_seconds: u64,
    expected_audience: Option<String>,
    expected: String,
}

/// The JWK of a corpus SPKI PEM (an Ed25519 SPKI ends with the raw key)
fn jwk(version: &str, pem: &str) -> Value {
    let body: String =
        pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = STANDARD.decode(body).unwrap();
    let raw: [u8; 32] = der[der.len() - 32..].try_into().unwrap();

    Ed25519PublicJwk::new(version.parse::<SigningKeyVersion>().unwrap(), raw)
        .to_json()
}

fn verifier(corpus: &Corpus, vector: &Vector) -> ProfileSignatureVerifier {
    let keys: Vec<Value> = vector
        .trusted_key_versions
        .iter()
        .map(|version| jwk(version, &corpus.public_keys[version]))
        .collect();
    let document = json!({ "keys": keys }).to_string();
    let tolerance = vector.tolerance_seconds.to_string();

    ProfileSignatureVerifier::new(
        Arc::new(
            TrustedProfileKeys::from_jwks_document(document.as_bytes())
                .unwrap(),
        ),
        ProfileSignatureTolerance::from_variable(Some(&tolerance)).unwrap(),
        vector.expected_audience.clone().map(|audience| {
            ProfileSignatureAudience::try_from(audience).unwrap()
        }),
    )
}

fn outcome(corpus: &Corpus, vector: &Vector) -> String {
    let result = verify_and_decode_profile(
        &vector.profile_header,
        &vector.signature,
        &verifier(corpus, vector),
        UnixTimestampSeconds::try_from(vector.now_seconds).unwrap(),
    );

    match result {
        Ok(_) => "valid".to_string(),
        Err(reason) => reason.as_str().to_string(),
    }
}

#[test]
fn every_contract_vector_has_the_expected_outcome() {
    let corpus: Corpus = serde_json::from_str(VECTORS).unwrap();
    assert_eq!(corpus.vectors.len(), 46);

    let mismatches: Vec<String> = corpus
        .vectors
        .iter()
        .filter_map(|vector| {
            let actual = outcome(&corpus, vector);
            (actual != vector.expected).then(|| {
                format!(
                    "{}: expected {}, got {actual}",
                    vector.name, vector.expected
                )
            })
        })
        .collect();

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn valid_profile_decodes_to_the_signed_account() {
    let corpus: Corpus = serde_json::from_str(VECTORS).unwrap();
    let vector = &corpus.vectors[0];

    let profile = verify_and_decode_profile(
        &vector.profile_header,
        &vector.signature,
        &verifier(&corpus, vector),
        UnixTimestampSeconds::try_from(vector.now_seconds).unwrap(),
    )
    .unwrap();

    assert_eq!(
        profile.acc_id.to_string(),
        "11111111-1111-4111-8111-111111111111"
    );
    assert!(profile.is_staff);
}
