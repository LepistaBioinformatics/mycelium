use actix_web::{web, HttpRequest};
use awc::{
    http::header::{HeaderName, HeaderValue},
    ClientRequest,
};
use myc_core::domain::dtos::profile_signature::{
    ProfileSignatureAudience, UnixTimestampSeconds,
};
use myc_http_tools::{
    functions::{compress_and_encode_profile_to_base64, sign_profile_payload},
    models::ProfileSigningState,
    responses::GatewayError,
    settings::{DEFAULT_PROFILE_KEY, DEFAULT_PROFILE_SIGNATURE_KEY},
    Profile,
};
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_KEY: HeaderName = HeaderName::from_static(DEFAULT_PROFILE_KEY);

const PROFILE_SIGNATURE_KEY: HeaderName =
    HeaderName::from_static(DEFAULT_PROFILE_SIGNATURE_KEY);

/// Source of "now" for signatures, replaceable in tests
pub(crate) type SignatureClock = fn() -> Result<u64, GatewayError>;

/// Read the profile signing state registered at boot
///
/// Must be called before the `HttpRequest` is moved into identity resolution.
/// A missing registration is an internal error: silently forwarding an
/// unsigned profile would turn a wiring bug into a security downgrade.
///
pub(crate) fn profile_signing_state_from(
    req: &HttpRequest,
) -> Result<ProfileSigningState, GatewayError> {
    let Some(state) = req.app_data::<web::Data<ProfileSigningState>>() else {
        tracing::error!(
            event = "profile_signature.sign",
            outcome = "error",
            reason = "state_not_registered",
            "Profile signing state is not registered"
        );

        return Err(internal_error());
    };

    Ok(state.get_ref().clone())
}

/// Encode the profile once and inject it, plus its signature when enabled
///
/// Enabled: either both headers are inserted or the request is not sent
/// (error). Disabled: only the profile header, exactly as before, and any
/// residual signature header is removed.
///
pub(crate) fn inject_signed_profile(
    request: ClientRequest,
    profile: Profile,
    service_name: &str,
    signing: &ProfileSigningState,
) -> Result<ClientRequest, GatewayError> {
    inject_signed_profile_with_clock(
        request,
        profile,
        service_name,
        signing,
        system_clock,
    )
}

pub(crate) fn inject_signed_profile_with_clock(
    mut request: ClientRequest,
    profile: Profile,
    service_name: &str,
    signing: &ProfileSigningState,
    clock: SignatureClock,
) -> Result<ClientRequest, GatewayError> {
    let encoded = compress_and_encode_profile_to_base64(profile)?;
    let profile_value = header_value(&encoded)?;
    let signature = issue_signature(&encoded, service_name, signing, clock)?;

    let headers = request.headers_mut();
    headers.remove(&PROFILE_SIGNATURE_KEY);
    headers.insert(PROFILE_KEY, profile_value);

    if let Some(signature_value) = signature {
        headers.insert(PROFILE_SIGNATURE_KEY, signature_value);
    }

    Ok(request)
}

fn issue_signature(
    encoded_profile: &str,
    service_name: &str,
    signing: &ProfileSigningState,
    clock: SignatureClock,
) -> Result<Option<HeaderValue>, GatewayError> {
    let ProfileSigningState::Enabled(signer) = signing else {
        return Ok(None);
    };

    let issued_at = UnixTimestampSeconds::try_from(clock()?)
        .map_err(|_| signing_failure("clock_out_of_range"))?;
    let audience = ProfileSignatureAudience::try_from(service_name.to_owned())
        .map_err(|_| signing_failure("invalid_audience"))?;

    let token =
        sign_profile_payload(encoded_profile, audience, signer, issued_at)
            .map_err(|_| signing_failure("encoding_failed"))?;

    tracing::debug!(
        event = "profile_signature.sign",
        outcome = "ok",
        kid = %signer.version()
    );

    header_value(&token).map(Some)
}

fn system_clock() -> Result<u64, GatewayError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| signing_failure("clock_before_unix_epoch"))
}

fn header_value(raw: &str) -> Result<HeaderValue, GatewayError> {
    HeaderValue::from_str(raw).map_err(|_| signing_failure("invalid_header"))
}

fn signing_failure(reason: &'static str) -> GatewayError {
    tracing::error!(
        event = "profile_signature.sign",
        outcome = "error",
        reason = reason,
        code = "MYC00034",
        "Unable to issue downstream profile signature"
    );

    internal_error()
}

fn internal_error() -> GatewayError {
    GatewayError::InternalServerError(
        "Unable to forward the request. Please contact administrators"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    use myc_core::domain::dtos::profile_signature::{
        ProfileHeaderHash, ProfileSignatureClaims, SigningKeyVersion,
    };
    use myc_http_tools::models::VersionedEd25519Signer;
    use std::sync::Arc;
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

    fn fixed_clock() -> Result<u64, GatewayError> {
        Ok(1_800_000_000)
    }

    fn later_clock() -> Result<u64, GatewayError> {
        Ok(1_800_000_030)
    }

    #[actix_web::test]
    async fn cached_profile_gets_a_fresh_signature_each_forward() {
        let signing = enabled();
        let first = inject_signed_profile_with_clock(
            downstream_request(),
            profile(),
            "svc",
            &signing,
            fixed_clock,
        )
        .unwrap();
        let later = inject_signed_profile_with_clock(
            downstream_request(),
            profile(),
            "svc",
            &signing,
            later_clock,
        )
        .unwrap();
        assert_eq!(
            header(&first, DEFAULT_PROFILE_KEY),
            header(&later, DEFAULT_PROFILE_KEY)
        );
        let first_token =
            header(&first, DEFAULT_PROFILE_SIGNATURE_KEY).unwrap();
        let later_token =
            header(&later, DEFAULT_PROFILE_SIGNATURE_KEY).unwrap();
        assert_ne!(first_token, later_token);
        assert_eq!(verify(first_token).ph, verify(later_token).ph);
        assert_eq!(
            verify(later_token).iat.value() - verify(first_token).iat.value(),
            30
        );
    }

    fn broken_clock() -> Result<u64, GatewayError> {
        Ok(u64::MAX)
    }

    fn profile() -> Profile {
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

    fn enabled() -> ProfileSigningState {
        let version = SigningKeyVersion::try_from(4).unwrap();
        let signer =
            VersionedEd25519Signer::from_pem(version, PRIVATE_PEM).unwrap();

        ProfileSigningState::Enabled(Arc::new(signer))
    }

    fn downstream_request() -> ClientRequest {
        awc::Client::default().get("http://downstream.test/whoami")
    }

    fn header<'a>(request: &'a ClientRequest, name: &str) -> Option<&'a str> {
        request
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap())
    }

    fn verify(token: &str) -> ProfileSignatureClaims {
        let key = DecodingKey::from_ed_pem(PUBLIC_PEM.as_bytes()).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        validation.validate_aud = false;

        decode::<ProfileSignatureClaims>(token, &key, &validation)
            .unwrap()
            .claims
    }

    #[actix_web::test]
    async fn disabled_injects_only_the_legacy_profile_header() {
        let request = downstream_request()
            .insert_header((DEFAULT_PROFILE_SIGNATURE_KEY, "residual"));

        let request = inject_signed_profile_with_clock(
            request,
            profile(),
            "svc",
            &ProfileSigningState::Disabled,
            fixed_clock,
        )
        .unwrap();

        let expected =
            compress_and_encode_profile_to_base64(profile()).unwrap();
        assert_eq!(header(&request, DEFAULT_PROFILE_KEY), Some(&*expected));
        assert!(header(&request, DEFAULT_PROFILE_SIGNATURE_KEY).is_none());
    }

    #[actix_web::test]
    async fn enabled_signs_the_exact_injected_header() {
        let request = inject_signed_profile_with_clock(
            downstream_request(),
            profile(),
            "billing-api",
            &enabled(),
            fixed_clock,
        )
        .unwrap();

        let profile_value = header(&request, DEFAULT_PROFILE_KEY).unwrap();
        let token = header(&request, DEFAULT_PROFILE_SIGNATURE_KEY).unwrap();
        let claims = verify(token);

        assert_eq!(
            profile_value,
            compress_and_encode_profile_to_base64(profile()).unwrap()
        );
        assert_eq!(
            claims.ph,
            ProfileHeaderHash::from_profile_header(profile_value)
        );
        assert_eq!(claims.iat.value(), 1_800_000_000);
        assert_eq!(claims.aud.as_str(), "billing-api");

        let protected = token.split('.').next().unwrap();
        let header_json: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(protected).unwrap())
                .unwrap();
        assert_eq!(header_json["kid"], "4");
        assert_eq!(header_json["alg"], "EdDSA");
    }

    #[actix_web::test]
    async fn enabled_never_forwards_without_a_signature() {
        let unsafe_clock = inject_signed_profile_with_clock(
            downstream_request(),
            profile(),
            "svc",
            &enabled(),
            broken_clock,
        );
        let blank_audience = inject_signed_profile_with_clock(
            downstream_request(),
            profile(),
            " ",
            &enabled(),
            fixed_clock,
        );

        assert!(matches!(
            unsafe_clock,
            Err(GatewayError::InternalServerError(_))
        ));
        assert!(matches!(
            blank_audience,
            Err(GatewayError::InternalServerError(_))
        ));
    }

    #[actix_web::test]
    async fn missing_state_registration_is_an_internal_error() {
        let req = TestRequest::default().to_http_request();

        assert!(matches!(
            profile_signing_state_from(&req),
            Err(GatewayError::InternalServerError(_))
        ));
    }

    #[actix_web::test]
    async fn registered_state_is_returned() {
        let req = TestRequest::default()
            .app_data(web::Data::new(ProfileSigningState::Disabled))
            .to_http_request();

        assert!(!profile_signing_state_from(&req).unwrap().is_enabled());
    }
}
