use crate::middleware::{
    body_idp::BodyIdpContext, inject_signed_profile,
    profile_signing_state_from, recovery_profile_from_storage_engines,
};

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{
    security_group::{PermissionedRole, SecurityGroup},
    service::Service,
};
use myc_http_tools::{responses::GatewayError, Email, Profile};
use std::{future::Future, pin::Pin};

type ProfileLookup =
    fn(
        HttpRequest,
        Email,
        Option<Vec<PermissionedRole>>,
    ) -> Pin<Box<dyn Future<Output = Result<Profile, GatewayError>>>>;

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
    downstream_request: ClientRequest,
    body_idp: BodyIdpContext,
    service: &Service,
    security_group: &SecurityGroup,
) -> Result<(ClientRequest, Profile), GatewayError> {
    resolve_and_inject_body_profile(
        req,
        downstream_request,
        body_idp,
        service,
        security_group,
        lookup_profile,
    )
    .await
}

async fn resolve_and_inject_body_profile(
    req: HttpRequest,
    downstream_request: ClientRequest,
    body_idp: BodyIdpContext,
    service: &Service,
    security_group: &SecurityGroup,
    lookup: ProfileLookup,
) -> Result<(ClientRequest, Profile), GatewayError> {
    enforce_mandatory_allowlist(service)?;
    let signing = profile_signing_state_from(&req)?;
    let profile =
        resolve_body_profile(req, body_idp, security_group, lookup).await?;
    let request = inject_signed_profile(
        downstream_request,
        profile.clone(),
        &service.name,
        &signing,
    )?;
    Ok((request, profile))
}

async fn resolve_body_profile(
    req: HttpRequest,
    body_idp: BodyIdpContext,
    security_group: &SecurityGroup,
    lookup: ProfileLookup,
) -> Result<Profile, GatewayError> {
    let roles = extract_roles(security_group);
    let email = body_idp
        .resolver
        .resolve_email(&body_idp.user_id, &req)
        .await?;

    let profile = lookup(req, email, roles.clone()).await?;

    if let Some(ref r) = roles {
        enforce_rbac(&profile, r)?;
    }

    Ok(profile)
}

fn lookup_profile(
    req: HttpRequest,
    email: Email,
    roles: Option<Vec<PermissionedRole>>,
) -> Pin<Box<dyn Future<Output = Result<Profile, GatewayError>>>> {
    Box::pin(recovery_profile_from_storage_engines(
        req, email, None, roles,
    ))
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

#[cfg(test)]
#[path = "body_profile_signature_tests.rs"]
mod tests;
