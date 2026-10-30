//! The Rust downstream verifier against the shared contract vectors
//!
//! `vectors.json` is produced by the gateway's real encoder and signer (see
//! `profile_signature_contract.rs`). The Python, JavaScript and Go SDKs run
//! the same corpus; this test proves `verify_and_decode_profile`, used by
//! `GatewayProfileData`, reports the same outcome for every case.

use myc_core::domain::dtos::profile_signature::UnixTimestampSeconds;
use myc_http_tools::{
    functions::verify_and_decode_profile, models::ProfileSignatureVerifier,
};
use serde::Deserialize;
use std::collections::BTreeMap;

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

fn verifier(corpus: &Corpus, vector: &Vector) -> ProfileSignatureVerifier {
    let mut variables: BTreeMap<String, String> = vector
        .trusted_key_versions
        .iter()
        .map(|version| {
            (
                format!("MYC_PROFILE_PUBLIC_KEY_{version}"),
                corpus.public_keys[version].clone(),
            )
        })
        .collect();
    variables.insert(
        "MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS".to_string(),
        vector.tolerance_seconds.to_string(),
    );
    if let Some(audience) = &vector.expected_audience {
        variables.insert(
            "MYC_PROFILE_SIGNATURE_AUDIENCE".to_string(),
            audience.clone(),
        );
    }

    ProfileSignatureVerifier::from_variables(&variables)
        .unwrap()
        .unwrap()
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
