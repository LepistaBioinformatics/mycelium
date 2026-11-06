//! Behavior of the JWKS-backed downstream verifier
//!
//! Key rotation without touching the downstream, refetch limits, keeping the
//! last valid set, mandatory settings, and the real HTTP fetch (bounded, no
//! redirects). The signature contract itself is covered by the shared
//! vectors in `profile_signature_verifier_contract.rs`.

use actix_web::{web, App, HttpResponse, HttpServer};
use futures::future::join_all;
use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{
        ProfileSignatureAudience, ProfileSignatureRejection, SigningKeyVersion,
        UnixTimestampSeconds,
    },
};
use myc_http_tools::{
    functions::{
        compress_and_encode_profile_to_base64, fetch_profile_jwks,
        sign_profile_payload,
    },
    models::{
        FetchedProfileJwks, JwksProfileSignatureVerifier, ProfileJwksFetcher,
        ProfileJwksUrl, ProfileKeyStore, ProfileSignatureTolerance,
        ProfileSignatureVerification, TrustedProfileKeys,
        VersionedEd25519Signer,
    },
};
use mycelium_base::utils::errors::execution_err;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

/// TEST ONLY: RFC 8032 section 7.1 test vectors 1 and 2 (PKCS#8)
const PRIVATE_PEM_V1: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
const PRIVATE_PEM_V2: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIEzNCJso/5banbbDRuwRTg9bijGfNaumJNqM9u1PuKb7
-----END PRIVATE KEY-----
";

// ? ---------------------------------------------------------------------------
// ? A fake gateway whose published key set can change
// ? ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct FakeGateway {
    published: Arc<Mutex<Option<Vec<u32>>>>,
    fetches: Arc<AtomicUsize>,
    max_age: Duration,
}

impl FakeGateway {
    fn publishing(versions: &[u32], max_age: Duration) -> Self {
        let gateway = Self {
            max_age,
            ..Self::default()
        };
        gateway.publish(Some(versions));
        gateway
    }

    /// `None` makes every fetch fail (gateway down)
    fn publish(&self, versions: Option<&[u32]>) {
        *self.published.lock().unwrap() = versions.map(<[u32]>::to_vec);
    }

    fn fetches(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }

    fn fetcher(&self) -> ProfileJwksFetcher {
        let gateway = self.clone();

        Arc::new(move |_| {
            gateway.fetches.fetch_add(1, Ordering::SeqCst);
            let published = gateway.published.lock().unwrap().clone();
            let max_age = gateway.max_age;

            Box::pin(async move {
                let versions =
                    published.ok_or_else(|| execution_err("gateway down"))?;
                Ok(FetchedProfileJwks {
                    keys: TrustedProfileKeys::from_jwks_document(
                        document(&versions).as_bytes(),
                    )?,
                    max_age,
                })
            })
        })
    }
}

fn signer(version: u32) -> VersionedEd25519Signer {
    let pem = [PRIVATE_PEM_V1, PRIVATE_PEM_V2][version as usize - 1];

    VersionedEd25519Signer::from_pem(
        SigningKeyVersion::try_from(version).unwrap(),
        pem,
    )
    .unwrap()
}

fn document(versions: &[u32]) -> String {
    let keys: Vec<_> = versions
        .iter()
        .map(|version| signer(*version).public_jwk().to_json())
        .collect();

    json!({ "keys": keys }).to_string()
}

fn url() -> ProfileJwksUrl {
    ProfileJwksUrl::parse("https://gateway.test/jwks.json", false).unwrap()
}

fn verifier(
    gateway: &FakeGateway,
    cooldown: Duration,
) -> JwksProfileSignatureVerifier {
    JwksProfileSignatureVerifier::new(
        ProfileKeyStore::new(url(), gateway.fetcher(), cooldown),
        ProfileSignatureTolerance::default(),
        ProfileSignatureAudience::try_from("svc".to_string()).unwrap(),
    )
}

fn now() -> UnixTimestampSeconds {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    UnixTimestampSeconds::try_from(seconds).unwrap()
}

/// A gateway-signed pair (profile header, signature header)
fn signed_by(version: u32) -> (String, String) {
    let profile = Profile::new(
        vec![],
        Uuid::nil(),
        false,
        false,
        true,
        true,
        true,
        true,
        false,
        false,
        None,
        None,
        None,
    );
    let header = compress_and_encode_profile_to_base64(profile).unwrap();
    let audience =
        ProfileSignatureAudience::try_from("svc".to_string()).unwrap();
    let signature =
        sign_profile_payload(&header, audience, &signer(version), now())
            .unwrap();

    (header, signature)
}

async fn verify(
    verifier: &JwksProfileSignatureVerifier,
    version: u32,
) -> Result<Profile, ProfileSignatureRejection> {
    let (header, signature) = signed_by(version);

    verifier.verify_and_decode(&header, &signature, now()).await
}

// ? ---------------------------------------------------------------------------
// ? Key cache behavior
// ? ---------------------------------------------------------------------------

#[actix_web::test]
async fn rotation_is_picked_up_without_touching_the_downstream() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    let verifier = verifier(&gateway, Duration::ZERO);
    verifier.prefetch().await.unwrap();
    assert!(verify(&verifier, 1).await.is_ok());

    gateway.publish(Some(&[1, 2]));

    assert!(verify(&verifier, 2).await.is_ok());
    assert_eq!(gateway.fetches(), 2);
}

#[actix_web::test]
async fn unknown_kid_refetches_at_most_once_per_cooldown() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    let verifier = verifier(&gateway, Duration::from_secs(30));
    verifier.prefetch().await.unwrap();

    let first = verify(&verifier, 2).await;
    let second = verify(&verifier, 2).await;

    assert!(matches!(first, Err(ProfileSignatureRejection::UnknownKey)));
    assert!(matches!(second, Err(ProfileSignatureRejection::UnknownKey)));
    assert_eq!(gateway.fetches(), 1, "cooldown since the prefetch");
}

#[actix_web::test]
async fn concurrent_unknown_kids_share_one_fetch() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    let verifier = verifier(&gateway, Duration::ZERO);
    verifier.prefetch().await.unwrap();
    gateway.publish(Some(&[1, 2]));

    let results = join_all((0..8).map(|_| verify(&verifier, 2))).await;

    assert!(results.iter().all(Result::is_ok));
    assert_eq!(gateway.fetches(), 2);
}

#[actix_web::test]
async fn a_failed_refresh_keeps_the_last_valid_set() {
    let gateway = FakeGateway::publishing(&[1], Duration::ZERO);
    let verifier = verifier(&gateway, Duration::ZERO);
    verifier.prefetch().await.unwrap();

    gateway.publish(None);

    assert!(verify(&verifier, 1).await.is_ok());
    assert!(gateway.fetches() >= 2, "the stale set was refreshed");
}

#[actix_web::test]
async fn prefetch_fails_when_the_gateway_is_unreachable() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    gateway.publish(None);

    assert!(verifier(&gateway, Duration::ZERO).prefetch().await.is_err());
}

#[actix_web::test]
async fn lazy_verifier_fetches_on_first_use() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    let verifier = verifier(&gateway, Duration::from_secs(30));

    assert!(verify(&verifier, 1).await.is_ok());
    assert_eq!(gateway.fetches(), 1);
}

/// A service started before the gateway: requests fail closed, then the keys
/// are picked up within the short first-download retry, not the cooldown
#[actix_web::test]
async fn a_service_started_before_the_gateway_recovers_within_seconds() {
    let gateway = FakeGateway::publishing(&[1], Duration::from_secs(600));
    let verifier = verifier(&gateway, Duration::from_secs(30));
    gateway.publish(None);
    assert!(verify(&verifier, 1).await.is_err(), "fails closed");

    gateway.publish(Some(&[1]));
    assert!(verify(&verifier, 1).await.is_err(), "retry is bounded");
    assert_eq!(gateway.fetches(), 1);

    std::thread::sleep(Duration::from_millis(5_100));
    assert!(verify(&verifier, 1).await.is_ok());
    assert_eq!(gateway.fetches(), 2);
}

// ? ---------------------------------------------------------------------------
// ? Settings
// ? ---------------------------------------------------------------------------

fn settings(
    entries: &[(&str, &str)],
) -> Result<ProfileSignatureVerification, String> {
    let variables: BTreeMap<String, String> = entries
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();

    ProfileSignatureVerification::from_variables(
        &variables,
        FakeGateway::default().fetcher(),
    )
    .map_err(|error| {
        assert!(error.has_str_code("MYC00035"), "{error}");
        error.msg()
    })
}

const HTTPS: (&str, &str) = ("MYCELIUM_JWKS_URL", "https://gw.test/jwks.json");
const HTTP: (&str, &str) = ("MYCELIUM_JWKS_URL", "http://gw.test/jwks.json");
const AUDIENCE: (&str, &str) = ("MYC_PROFILE_SIGNATURE_AUDIENCE", "svc");
const INSECURE: &str = "MYCELIUM_JWKS_ALLOW_INSECURE_HTTP";

#[test]
fn verification_is_enabled_by_the_jwks_url_only() {
    assert!(!settings(&[]).unwrap().is_enabled());
    assert!(!settings(&[AUDIENCE]).unwrap().is_enabled());
    assert!(settings(&[HTTPS, AUDIENCE]).unwrap().is_enabled());
}

#[test]
fn audience_is_mandatory_when_enabled() {
    let error = settings(&[HTTPS]).unwrap_err();

    assert!(error.contains("MYC_PROFILE_SIGNATURE_AUDIENCE"));
}

#[test]
fn plain_http_needs_the_explicit_development_switch() {
    assert!(settings(&[HTTP, AUDIENCE]).is_err());
    assert!(settings(&[HTTP, AUDIENCE, (INSECURE, "false")]).is_err());
    assert!(settings(&[HTTP, AUDIENCE, (INSECURE, "true")]).is_ok());
    assert!(settings(&[HTTP, AUDIENCE, (INSECURE, "yes")]).is_err());
}

#[test]
fn invalid_settings_fail_even_when_disabled() {
    for entry in [
        ("MYC_PROFILE_SIGNATURE_AUDIENCE", "  "),
        ("MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS", "0"),
        (INSECURE, "maybe"),
    ] {
        assert!(settings(&[entry]).is_err(), "{entry:?}");
    }
}

// ? ---------------------------------------------------------------------------
// ? Real HTTP fetch
// ? ---------------------------------------------------------------------------

/// A local gateway stand-in; returns its base URL
fn serve_jwks() -> String {
    let body = document(&[1, 2]);
    let server = HttpServer::new(move || {
        let body = body.clone();
        App::new()
            .route(
                "/jwks.json",
                web::get().to(move || {
                    let body = body.clone();
                    async move {
                        HttpResponse::Ok()
                            .insert_header((
                                "cache-control",
                                "public, max-age=600",
                            ))
                            .content_type("application/json")
                            .body(body)
                    }
                }),
            )
            .route(
                "/huge.json",
                web::get().to(|| async {
                    HttpResponse::Ok().body(" ".repeat(70 * 1024))
                }),
            )
            .route(
                "/redirect.json",
                web::get().to(|| async {
                    HttpResponse::Found()
                        .insert_header(("location", "/jwks.json"))
                        .finish()
                }),
            )
            .route("/missing.json", web::get().to(HttpResponse::NotFound))
    })
    .workers(1)
    .bind(("127.0.0.1", 0))
    .unwrap();

    let address = server.addrs()[0];
    actix_web::rt::spawn(server.run());

    format!("http://{address}")
}

fn local(base: &str, path: &str) -> ProfileJwksUrl {
    ProfileJwksUrl::parse(&format!("{base}{path}"), true).unwrap()
}

#[actix_web::test]
async fn fetches_the_set_and_its_cache_lifetime_over_http() {
    let base = serve_jwks();

    let fetched = fetch_profile_jwks(local(&base, "/jwks.json"))
        .await
        .unwrap();

    assert_eq!(fetched.keys.len(), 2);
    assert_eq!(fetched.max_age, Duration::from_secs(600));
}

#[actix_web::test]
async fn refuses_errors_oversized_bodies_and_redirects() {
    let base = serve_jwks();

    for path in ["/missing.json", "/huge.json", "/redirect.json"] {
        let result = fetch_profile_jwks(local(&base, path)).await;
        assert!(result.is_err(), "{path}");
    }
}
