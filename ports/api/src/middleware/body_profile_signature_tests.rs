//! Producer tests: the real IdP -> allowlist/RBAC -> encoding/signing pipeline.
use super::*;
use crate::middleware::body_idp::{BodyIdpContext, BodyIdpResolver};
use actix_web::{test::TestRequest, web, HttpRequest};
use async_trait::async_trait;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use myc_core::domain::dtos::profile_signature::{
    ProfileHeaderHash, ProfileSignatureClaims, SigningKeyVersion,
};
use myc_http_tools::{
    models::{ProfileSigningState, VersionedEd25519Signer},
    settings::{DEFAULT_PROFILE_KEY, DEFAULT_PROFILE_SIGNATURE_KEY},
};
use std::{cell::Cell, rc::Rc, sync::Arc};
use uuid::Uuid;

// TEST ONLY: RFC 8032 test vector 1, never use operationally.
const PRIVATE: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g\n-----END PRIVATE KEY-----\n";
const PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=\n-----END PUBLIC KEY-----\n";

struct Idp(Rc<Cell<usize>>);

#[async_trait(?Send)]
impl BodyIdpResolver for Idp {
    fn extract_user_id(&self, _: &web::Bytes) -> Result<String, GatewayError> {
        Ok("42".into())
    }

    async fn resolve_email(
        &self,
        user: &str,
        _: &HttpRequest,
    ) -> Result<Email, GatewayError> {
        assert_eq!(user, "42");
        self.0.set(self.0.get() + 1);
        Ok(Email::from_string("synthetic@example.test".into()).unwrap())
    }
}

fn profile(staff: bool) -> Profile {
    Profile::new(
        vec![],
        Uuid::nil(),
        false,
        false,
        staff,
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

fn staff_lookup(
    _: HttpRequest,
    email: Email,
    roles: Option<Vec<PermissionedRole>>,
) -> Pin<Box<dyn Future<Output = Result<Profile, GatewayError>>>> {
    assert_eq!(email.email(), "synthetic@example.test");
    assert_eq!(roles.unwrap()[0].name, "admin");
    Box::pin(async { Ok(profile(true)) })
}

fn personal_lookup(
    _: HttpRequest,
    _: Email,
    _: Option<Vec<PermissionedRole>>,
) -> Pin<Box<dyn Future<Output = Result<Profile, GatewayError>>>> {
    Box::pin(async { Ok(profile(false)) })
}

fn setup(
    allowlist: bool,
) -> (HttpRequest, Service, BodyIdpContext, Rc<Cell<usize>>) {
    let signer = VersionedEd25519Signer::from_pem(
        SigningKeyVersion::try_from(1).unwrap(),
        PRIVATE,
    )
    .unwrap();
    let req = TestRequest::default()
        .app_data(web::Data::new(ProfileSigningState::Enabled(Arc::new(
            signer,
        ))))
        .to_http_request();
    let mut service: Service = serde_json::from_value(serde_json::json!({"name":"body-svc","host":"localhost:1","routes":[],"healthCheckPath":"/health"})).unwrap();
    service.allowed_sources = allowlist.then(|| vec!["127.0.0.1".into()]);
    let calls = Rc::new(Cell::new(0));
    let context = BodyIdpContext {
        resolver: Box::new(Idp(calls.clone())),
        user_id: "42".into(),
    };
    (req, service, context, calls)
}

fn roles() -> SecurityGroup {
    SecurityGroup::ProtectedByRoles(vec![PermissionedRole {
        name: "admin".into(),
        permission: None,
    }])
}

#[actix_web::test]
async fn body_identity_produces_signed_profiles_in_all_non_role_groups() {
    // The router selects body IdP before its traditional security-group match.
    for group in [
        SecurityGroup::Public,
        SecurityGroup::Authenticated,
        SecurityGroup::Protected,
    ] {
        let (req, service, context, calls) = setup(true);
        let (request, _) = resolve_and_inject_body_profile(
            req,
            awc::Client::default().get("http://localhost:1/whoami"),
            context,
            &service,
            &group,
            personal_lookup,
        )
        .await
        .unwrap();
        assert!(request.headers().contains_key(DEFAULT_PROFILE_KEY));
        assert!(request
            .headers()
            .contains_key(DEFAULT_PROFILE_SIGNATURE_KEY));
        assert_eq!(calls.get(), 1);
    }
}

#[actix_web::test]
async fn body_identity_disabled_keeps_legacy_profile_without_signature() {
    let (_, service, context, calls) = setup(true);
    let req = TestRequest::default()
        .app_data(web::Data::new(ProfileSigningState::Disabled))
        .to_http_request();
    let (request, _) = resolve_and_inject_body_profile(
        req,
        awc::Client::default().get("http://localhost:1/whoami"),
        context,
        &service,
        &SecurityGroup::Protected,
        personal_lookup,
    )
    .await
    .unwrap();
    assert!(request.headers().contains_key(DEFAULT_PROFILE_KEY));
    assert!(!request
        .headers()
        .contains_key(DEFAULT_PROFILE_SIGNATURE_KEY));
    assert_eq!(calls.get(), 1);
}

#[actix_web::test]
async fn body_identity_missing_signing_state_fails_before_idp_or_forwarding() {
    let (_, service, context, calls) = setup(true);
    let result = resolve_and_inject_body_profile(
        TestRequest::default().to_http_request(),
        awc::Client::default().get("http://localhost:1/whoami"),
        context,
        &service,
        &SecurityGroup::Protected,
        personal_lookup,
    )
    .await;
    assert!(matches!(result, Err(GatewayError::InternalServerError(_))));
    assert_eq!(calls.get(), 0);
}

#[actix_web::test]
async fn body_idp_profile_is_signed_after_resolution_and_rbac() {
    let (req, service, context, calls) = setup(true);
    let outgoing = awc::Client::default().get("http://localhost:1/whoami");
    let (request, returned) = resolve_and_inject_body_profile(
        req,
        outgoing,
        context,
        &service,
        &roles(),
        staff_lookup,
    )
    .await
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert!(returned.is_staff);
    let header = request
        .headers()
        .get(DEFAULT_PROFILE_KEY)
        .unwrap()
        .to_str()
        .unwrap();
    let token = request
        .headers()
        .get(DEFAULT_PROFILE_SIGNATURE_KEY)
        .unwrap()
        .to_str()
        .unwrap();
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.required_spec_claims.clear();
    validation.validate_exp = false;
    validation.validate_aud = false;
    let claims = decode::<ProfileSignatureClaims>(
        token,
        &DecodingKey::from_ed_pem(PUBLIC.as_bytes()).unwrap(),
        &validation,
    )
    .unwrap()
    .claims;
    assert_eq!(claims.ph, ProfileHeaderHash::from_profile_header(header));
    assert_eq!(claims.aud.as_str(), "body-svc");
}

#[actix_web::test]
async fn missing_allowlist_rejects_before_identity_resolution() {
    let (req, service, context, calls) = setup(false);
    let outgoing = awc::Client::default().get("http://localhost:1/whoami");
    let result = resolve_and_inject_body_profile(
        req,
        outgoing,
        context,
        &service,
        &roles(),
        staff_lookup,
    )
    .await;
    assert!(matches!(result, Err(GatewayError::Forbidden(_))));
    assert_eq!(calls.get(), 0);
}

#[actix_web::test]
async fn missing_roles_rejects_before_emission() {
    let (req, service, context, calls) = setup(true);
    let outgoing = awc::Client::default().get("http://localhost:1/whoami");
    let result = resolve_and_inject_body_profile(
        req,
        outgoing,
        context,
        &service,
        &roles(),
        personal_lookup,
    )
    .await;
    assert!(matches!(result, Err(GatewayError::Forbidden(_))));
    assert_eq!(calls.get(), 1);
}
