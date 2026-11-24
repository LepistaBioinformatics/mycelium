//! Micro-measurement of the per-request signing cost (TASK-025, partial)
//!
//! Not a benchmark of the HTTP pipeline and not a performance guarantee:
//! compares the existing encoding step alone with encoding + signing, on the
//! small synthetic profile and on `test/mock/large-profile.json`. Ignored by
//! default; run in release mode:
//!
//! ```bash
//! cargo test --release -p mycelium-http-tools --test profile_signature_cost \
//!     -- --ignored --nocapture
//! ```

use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{
        ProfileSignatureAudience, SigningKeyVersion, UnixTimestampSeconds,
    },
};
use myc_http_tools::{
    functions::{compress_and_encode_profile_to_base64, sign_profile_payload},
    models::VersionedEd25519Signer,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// TEST ONLY: RFC 8032 test vector 1 (PKCS#8)
const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

const WARMUP: usize = 200;
const ITERATIONS: usize = 2_000;
const ROUNDS: usize = 5;

fn small_profile() -> Profile {
    Profile::new(
        vec![],
        Uuid::nil(),
        false,
        false,
        false,
        true,
        true,
        true,
        false,
        false,
        None,
        None,
        None,
    )
}

fn large_profile() -> Profile {
    serde_json::from_str(include_str!("../test/mock/large-profile.json"))
        .unwrap()
}

fn per_operation(mut operation: impl FnMut()) -> Duration {
    for _ in 0..WARMUP {
        operation();
    }

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        operation();
    }

    started.elapsed() / ITERATIONS as u32
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn measure(label: &str, profile: Profile) {
    let signer = VersionedEd25519Signer::from_pem(
        SigningKeyVersion::try_from(1).unwrap(),
        PRIVATE_PEM,
    )
    .unwrap();
    let audience =
        ProfileSignatureAudience::try_from("svc".to_string()).unwrap();
    let issued_at = UnixTimestampSeconds::try_from(1_800_000_000).unwrap();

    let header =
        compress_and_encode_profile_to_base64(profile.clone()).unwrap();
    let token =
        sign_profile_payload(&header, audience.clone(), &signer, issued_at)
            .unwrap();

    let encode_only = median(
        (0..ROUNDS)
            .map(|_| {
                per_operation(|| {
                    compress_and_encode_profile_to_base64(profile.clone())
                        .unwrap();
                })
            })
            .collect(),
    );

    let encode_and_sign = median(
        (0..ROUNDS)
            .map(|_| {
                per_operation(|| {
                    let header =
                        compress_and_encode_profile_to_base64(profile.clone())
                            .unwrap();
                    sign_profile_payload(
                        &header,
                        audience.clone(),
                        &signer,
                        issued_at,
                    )
                    .unwrap();
                })
            })
            .collect(),
    );

    println!(
        "{label}: profile header {} bytes, signature header {} bytes, \
         encode {:?}/op, encode+sign {:?}/op, signing overhead {:?}/op \
         (median of {ROUNDS} rounds x {ITERATIONS} ops)",
        header.len(),
        token.len(),
        encode_only,
        encode_and_sign,
        encode_and_sign.saturating_sub(encode_only),
    );
}

#[test]
#[ignore = "measurement, run explicitly in release mode"]
fn signing_cost_per_request() {
    measure("small profile", small_profile());
    measure("large profile", large_profile());
}
