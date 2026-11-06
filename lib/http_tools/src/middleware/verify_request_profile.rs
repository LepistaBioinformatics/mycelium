use super::{
    resolve_profile_signature_verification, SIGNATURE_INTERNAL_DETAIL,
};
use crate::{
    models::{JwksProfileSignatureVerifier, ProfileSignatureVerification},
    responses::GatewayError,
    settings::{DEFAULT_PROFILE_KEY, DEFAULT_PROFILE_SIGNATURE_KEY},
};

use actix_web::HttpRequest;
use futures::future::LocalBoxFuture;
use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{ProfileSignatureRejection, UnixTimestampSeconds},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// Generic detail of every rejection; the reason is only logged
pub(crate) const SIGNATURE_REJECTION_DETAIL: &str =
    "Unable to verify Mycelium profile";

/// Whether the "signed profile but verification disabled" warning was logged
static UNVERIFIED_SIGNATURE_WARNED: AtomicBool = AtomicBool::new(false);

/// The signed profile of a request, when verification applies to it
///
/// `None` means the caller keeps its legacy path: verification is disabled,
/// or the request carries neither profile header (for example a direct call
/// authenticated with a bearer token, resolved by the gateway itself). As
/// soon as either header is present while verification is enabled, the
/// request must carry a valid signed profile or it is refused (403).
///
pub(crate) fn signed_profile_from_request(
    req: &HttpRequest,
) -> Option<LocalBoxFuture<'static, Result<Profile, GatewayError>>> {
    match resolve_profile_signature_verification(req) {
        Err(error) => Some(Box::pin(async move { Err(error) })),
        Ok(ProfileSignatureVerification::Disabled) => {
            warn_when_signature_is_ignored(req);
            None
        }
        Ok(ProfileSignatureVerification::Enabled(verifier)) => {
            carries_profile_headers(req).then(|| {
                let req = req.clone();
                Box::pin(async move {
                    verify_request_profile(&req, &verifier, current_time())
                        .await
                }) as LocalBoxFuture<'static, _>
            })
        }
    }
}

/// Authenticate and decode the profile of a request at a given instant
#[tracing::instrument(name = "verify_request_profile", skip_all)]
pub(crate) async fn verify_request_profile(
    req: &HttpRequest,
    verifier: &Arc<JwksProfileSignatureVerifier>,
    now: Option<UnixTimestampSeconds>,
) -> Result<Profile, GatewayError> {
    let Some(now) = now else {
        return Err(GatewayError::InternalServerError(
            SIGNATURE_INTERNAL_DETAIL.to_string(),
        ));
    };

    verified_profile(req, verifier, now)
        .await
        .map_err(|reason| {
            tracing::warn!(
                reason = reason.as_str(),
                "profile_signature.verify outcome=rejected"
            );

            GatewayError::Forbidden(SIGNATURE_REJECTION_DETAIL.to_string())
        })
}

async fn verified_profile(
    req: &HttpRequest,
    verifier: &JwksProfileSignatureVerifier,
    now: UnixTimestampSeconds,
) -> Result<Profile, ProfileSignatureRejection> {
    let profile_header = single_value(
        req,
        DEFAULT_PROFILE_KEY,
        ProfileSignatureRejection::MissingProfile,
        ProfileSignatureRejection::InvalidProfile,
    )?;
    let signature_header = single_value(
        req,
        DEFAULT_PROFILE_SIGNATURE_KEY,
        ProfileSignatureRejection::MissingSignature,
        ProfileSignatureRejection::MalformedSignature,
    )?;

    verifier
        .verify_and_decode(profile_header, signature_header, now)
        .await
}

fn carries_profile_headers(req: &HttpRequest) -> bool {
    req.headers().contains_key(DEFAULT_PROFILE_KEY)
        || req.headers().contains_key(DEFAULT_PROFILE_SIGNATURE_KEY)
}

/// Log once that a signed profile arrived while verification is off
///
/// The usual cause is a service that forgot `MYCELIUM_JWKS_URL`: the profile
/// is still trusted as received, exactly as before.
///
fn warn_when_signature_is_ignored(req: &HttpRequest) {
    if !req.headers().contains_key(DEFAULT_PROFILE_SIGNATURE_KEY)
        || UNVERIFIED_SIGNATURE_WARNED.swap(true, Ordering::Relaxed)
    {
        return;
    }

    tracing::warn!(
        event = "profile_signature.verify",
        outcome = "skipped",
        "received a signed profile but MYCELIUM_JWKS_URL is not set; \
         the signature is not verified"
    );
}

/// The one value of a header, refusing duplicates
///
/// Neither Base64 nor a compact JWS contains ",": a comma can only come from
/// repeated headers folded by a proxy, so it counts as a duplicate too.
///
fn single_value<'a>(
    req: &'a HttpRequest,
    name: &str,
    missing: ProfileSignatureRejection,
    unreadable: ProfileSignatureRejection,
) -> Result<&'a str, ProfileSignatureRejection> {
    let mut values = Vec::new();

    for value in req.headers().get_all(name) {
        let text = value.to_str().map_err(|_| unreadable)?;
        values.extend(text.split(',').map(str::trim));
    }

    match values.as_slice() {
        [] | [""] => Err(missing),
        [single] => Ok(single),
        _ => Err(ProfileSignatureRejection::DuplicateHeader),
    }
}

fn current_time() -> Option<UnixTimestampSeconds> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;

    UnixTimestampSeconds::try_from(elapsed.as_secs()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dtos::gateway_profile_data::GatewayProfileData,
        functions::{
            compress_and_encode_profile_to_base64, sign_profile_payload,
        },
        models::{
            FetchedProfileJwks, ProfileJwksFetcher, TrustedProfileKeys,
            VersionedEd25519Signer,
        },
    };
    use actix_web::{dev::Payload, test::TestRequest, web, FromRequest};
    use myc_core::domain::dtos::profile_signature::{
        ProfileSignatureAudience, SigningKeyVersion,
    };
    use serde_json::json;
    use std::{collections::BTreeMap, time::Duration};
    use uuid::Uuid;

    /// TEST ONLY: RFC 8032 test vector 1 (PKCS#8)
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

    fn signer() -> VersionedEd25519Signer {
        VersionedEd25519Signer::from_pem(
            SigningKeyVersion::try_from(1).unwrap(),
            PRIVATE_PEM,
        )
        .unwrap()
    }

    fn fetcher() -> ProfileJwksFetcher {
        let document = json!({"keys": [signer().public_jwk().to_json()]});
        let keys = TrustedProfileKeys::from_jwks_document(
            document.to_string().as_bytes(),
        )
        .unwrap();

        Arc::new(move |_| {
            let keys = keys.clone();
            Box::pin(async move {
                Ok(FetchedProfileJwks {
                    keys,
                    max_age: Duration::from_secs(600),
                })
            })
        })
    }

    fn verifier() -> Arc<JwksProfileSignatureVerifier> {
        let variables = BTreeMap::from([
            (
                "MYCELIUM_JWKS_URL".to_string(),
                "https://gateway.test/jwks.json".to_string(),
            ),
            (
                "MYC_PROFILE_SIGNATURE_AUDIENCE".to_string(),
                "svc".to_string(),
            ),
        ]);

        Arc::new(
            JwksProfileSignatureVerifier::from_variables(&variables, fetcher())
                .unwrap()
                .unwrap(),
        )
    }

    fn enabled() -> web::Data<ProfileSignatureVerification> {
        web::Data::new(ProfileSignatureVerification::Enabled(verifier()))
    }

    fn signed_headers() -> (String, String) {
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
        let signature = sign_profile_payload(
            &header,
            ProfileSignatureAudience::try_from("svc".to_string()).unwrap(),
            &signer(),
            current_time().unwrap(),
        )
        .unwrap();

        (header, signature)
    }

    async fn extract(
        request: TestRequest,
    ) -> Result<GatewayProfileData, GatewayError> {
        let req = request.to_http_request();

        GatewayProfileData::from_request(&req, &mut Payload::None).await
    }

    #[actix_web::test]
    async fn extractor_accepts_a_fresh_gateway_signature() {
        let (header, signature) = signed_headers();

        let profile = extract(
            TestRequest::default()
                .app_data(enabled())
                .insert_header((DEFAULT_PROFILE_KEY, header))
                .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, signature)),
        )
        .await
        .unwrap();

        assert!(profile.is_staff);
    }

    #[actix_web::test]
    async fn extractor_refuses_unsigned_and_tampered_profiles() {
        let (header, signature) = signed_headers();
        let (other_header, _) = signed_headers();
        let tampered = format!("{other_header}A");

        let requests = [
            TestRequest::default()
                .insert_header((DEFAULT_PROFILE_KEY, header.clone())),
            TestRequest::default()
                .insert_header((DEFAULT_PROFILE_KEY, tampered))
                .insert_header((
                    DEFAULT_PROFILE_SIGNATURE_KEY,
                    signature.clone(),
                )),
            TestRequest::default().insert_header((
                DEFAULT_PROFILE_SIGNATURE_KEY,
                signature.clone(),
            )),
            TestRequest::default()
                .append_header((DEFAULT_PROFILE_KEY, header.clone()))
                .append_header((DEFAULT_PROFILE_KEY, header.clone()))
                .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, signature)),
        ];

        for request in requests {
            let error = extract(request.app_data(enabled())).await.unwrap_err();

            assert!(matches!(error, GatewayError::Forbidden(_)), "{error:?}");
        }
    }

    #[actix_web::test]
    async fn disabled_verification_keeps_the_legacy_extractor() {
        let (header, _) = signed_headers();

        let profile = extract(
            TestRequest::default()
                .app_data(web::Data::new(
                    ProfileSignatureVerification::Disabled,
                ))
                .insert_header((DEFAULT_PROFILE_KEY, header)),
        )
        .await
        .unwrap();

        assert!(profile.is_staff);
    }

    #[actix_web::test]
    async fn missing_clock_is_an_internal_error() {
        let (header, signature) = signed_headers();
        let req = TestRequest::default()
            .insert_header((DEFAULT_PROFILE_KEY, header))
            .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, signature))
            .to_http_request();

        let error = verify_request_profile(&req, &verifier(), None)
            .await
            .unwrap_err();

        assert!(matches!(error, GatewayError::InternalServerError(_)));
    }

    #[actix_web::test]
    async fn folded_duplicate_headers_are_refused() {
        let req = TestRequest::default()
            .insert_header((DEFAULT_PROFILE_KEY, "a, b"))
            .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, "x.y.z"))
            .to_http_request();

        let reason =
            verified_profile(&req, &verifier(), current_time().unwrap())
                .await
                .unwrap_err();

        assert_eq!(reason, ProfileSignatureRejection::DuplicateHeader);
    }
}
