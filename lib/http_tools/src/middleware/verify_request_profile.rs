use super::{
    resolve_profile_signature_verification, SIGNATURE_INTERNAL_DETAIL,
};
use crate::{
    functions::verify_and_decode_profile,
    models::{ProfileSignatureVerification, ProfileSignatureVerifier},
    responses::GatewayError,
    settings::{DEFAULT_PROFILE_KEY, DEFAULT_PROFILE_SIGNATURE_KEY},
};

use actix_web::HttpRequest;
use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{ProfileSignatureRejection, UnixTimestampSeconds},
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generic detail of every rejection; the reason is only logged
pub(crate) const SIGNATURE_REJECTION_DETAIL: &str =
    "Unable to verify Mycelium profile";

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
) -> Option<Result<Profile, GatewayError>> {
    match resolve_profile_signature_verification(req) {
        Err(error) => Some(Err(error)),
        Ok(ProfileSignatureVerification::Disabled) => None,
        Ok(ProfileSignatureVerification::Enabled(verifier)) => {
            carries_profile_headers(req)
                .then(|| verify_request_profile(req, &verifier, current_time()))
        }
    }
}

/// Authenticate and decode the profile of a request at a given instant
pub(crate) fn verify_request_profile(
    req: &HttpRequest,
    verifier: &ProfileSignatureVerifier,
    now: Option<UnixTimestampSeconds>,
) -> Result<Profile, GatewayError> {
    let Some(now) = now else {
        return Err(GatewayError::InternalServerError(
            SIGNATURE_INTERNAL_DETAIL.to_string(),
        ));
    };

    verified_profile(req, verifier, now).map_err(|reason| {
        tracing::warn!(
            reason = reason.as_str(),
            "profile_signature.verify outcome=rejected"
        );

        GatewayError::Forbidden(SIGNATURE_REJECTION_DETAIL.to_string())
    })
}

fn verified_profile(
    req: &HttpRequest,
    verifier: &ProfileSignatureVerifier,
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

    verify_and_decode_profile(profile_header, signature_header, verifier, now)
}

fn carries_profile_headers(req: &HttpRequest) -> bool {
    req.headers().contains_key(DEFAULT_PROFILE_KEY)
        || req.headers().contains_key(DEFAULT_PROFILE_SIGNATURE_KEY)
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
        models::VersionedEd25519Signer,
    };
    use actix_web::{dev::Payload, test::TestRequest, web, FromRequest};
    use myc_core::domain::dtos::profile_signature::{
        ProfileSignatureAudience, SigningKeyVersion,
    };
    use std::{collections::BTreeMap, sync::Arc};
    use uuid::Uuid;

    /// TEST ONLY: RFC 8032 test vector 1 (PKCS#8 / SPKI)
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";
    const PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";

    fn verifier() -> ProfileSignatureVerifier {
        let variables = BTreeMap::from([(
            "MYC_PROFILE_PUBLIC_KEY_1".to_string(),
            PUBLIC_PEM.to_string(),
        )]);

        ProfileSignatureVerifier::from_variables(&variables)
            .unwrap()
            .unwrap()
    }

    fn enabled() -> web::Data<ProfileSignatureVerification> {
        web::Data::new(ProfileSignatureVerification::Enabled(Arc::new(
            verifier(),
        )))
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
        let signer = VersionedEd25519Signer::from_pem(
            SigningKeyVersion::try_from(1).unwrap(),
            PRIVATE_PEM,
        )
        .unwrap();
        let signature = sign_profile_payload(
            &header,
            ProfileSignatureAudience::try_from("svc".to_string()).unwrap(),
            &signer,
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

    #[test]
    fn missing_clock_is_an_internal_error() {
        let (header, signature) = signed_headers();
        let req = TestRequest::default()
            .insert_header((DEFAULT_PROFILE_KEY, header))
            .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, signature))
            .to_http_request();

        let error =
            verify_request_profile(&req, &verifier(), None).unwrap_err();

        assert!(matches!(error, GatewayError::InternalServerError(_)));
    }

    #[test]
    fn folded_duplicate_headers_are_refused() {
        let req = TestRequest::default()
            .insert_header((DEFAULT_PROFILE_KEY, "a, b"))
            .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, "x.y.z"))
            .to_http_request();

        let reason =
            verified_profile(&req, &verifier(), current_time().unwrap())
                .unwrap_err();

        assert_eq!(reason, ProfileSignatureRejection::DuplicateHeader);
    }
}
