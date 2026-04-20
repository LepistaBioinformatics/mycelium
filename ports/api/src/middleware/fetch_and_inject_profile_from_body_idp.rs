use crate::middleware::{
    body_idp::BodyIdpContext, recovery_profile_from_storage_engines,
};

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{
    security_group::{PermissionedRole, SecurityGroup},
    service::Service,
};
use myc_http_tools::{
    functions::compress_and_encode_profile_to_base64, responses::GatewayError,
    settings::DEFAULT_PROFILE_KEY, Profile,
};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;

/// Resolve the caller's Mycelium profile from a body-based identity provider
/// and inject it as the `DEFAULT_PROFILE_KEY` header into the downstream request.
///
/// Used for routes with `identity_source` set. The caller's host must already
/// have passed the source reliability check (IP allowlist). This function
/// additionally enforces that the allowlist is mandatory — body-based IdP
/// routes without `allowed_sources` are rejected.
#[tracing::instrument(
    name = "fetch_and_inject_profile_from_body_idp",
    skip_all,
    fields(user_id = body_idp.user_id.as_str())
)]
pub(crate) async fn fetch_and_inject_profile_from_body_idp(
    req: HttpRequest,
    mut downstream_request: ClientRequest,
    body_idp: BodyIdpContext,
    service: &Service,
    security_group: &SecurityGroup,
) -> Result<(ClientRequest, Profile), GatewayError> {
    enforce_mandatory_allowlist(service)?;

    let roles = extract_roles(security_group);
    let email = body_idp
        .resolver
        .resolve_email(&body_idp.user_id, &req)
        .await?;

    let profile =
        recovery_profile_from_storage_engines(req, email, None, roles.clone())
            .await?;

    if let Some(ref r) = roles {
        enforce_rbac(&profile, r)?;
    }

    let encoded = compress_and_encode_profile_to_base64(profile.clone())?;

    downstream_request.headers_mut().insert(
        HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
        HeaderValue::from_str(&encoded).unwrap(),
    );

    Ok((downstream_request, profile))
}

fn extract_roles(
    security_group: &SecurityGroup,
) -> Option<Vec<PermissionedRole>> {
    match security_group {
        SecurityGroup::ProtectedByRoles(roles) => Some(roles.clone()),
        _ => None,
    }
}

fn enforce_rbac(
    profile: &Profile,
    _roles: &[PermissionedRole],
) -> Result<(), GatewayError> {
    if profile.licensed_resources.is_none()
        && (!profile.is_manager && !profile.is_staff)
    {
        return Err(GatewayError::Forbidden(
            "User does not have permission to perform this action".to_string(),
        ));
    }

    Ok(())
}

/// Body-based IdP routes require `allowed_sources` to be non-empty. The
/// standard `check_source_reliability` call already verified the host when a
/// list is present — this guard ensures the list was mandatory, not absent.
fn enforce_mandatory_allowlist(service: &Service) -> Result<(), GatewayError> {
    if service.allowed_sources.is_none() {
        tracing::warn!(
            service = service.name,
            "Body IdP route rejected: allowed_sources must be set"
        );
        return Err(GatewayError::Forbidden(
            "Routes with a body-based identity source require allowed_sources"
                .to_string(),
        ));
    }

    Ok(())
}
