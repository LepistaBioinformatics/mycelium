use super::fetch_profile_from_request_token;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::security_group::PermissionedRoles;
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_PROFILE_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::{warn, Instrument};
use uuid::Uuid;

/// Fetch profile from email and inject on client request
///
/// Try to extract profile from email (these extracted from the bearer token)
/// and, then find the profile from the email and inject profile into the
/// forward request.
///
/// These use-case is usual over middleware or routers parts of the application.
#[tracing::instrument(
    name = "fetch_and_inject_profile_from_token_to_forward", 
    skip_all,
fields(
        //
        // User information
        //
        myc.router.profile_id = tracing::field::Empty,
        myc.router.is_staff = tracing::field::Empty,
        myc.router.is_manager = tracing::field::Empty,
        myc.router.is_subscription = tracing::field::Empty,
        myc.router.has_tenant_ownership = tracing::field::Empty,
        myc.router.has_licensed_resources = tracing::field::Empty,
    )
)]
pub async fn fetch_and_inject_profile_from_token_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<ClientRequest, GatewayError> {
    let span = tracing::Span::current();

    tracing::trace!("Injecting profile to forward");

    let profile = fetch_profile_from_request_token(
        req,
        tenant,
        roles.to_owned(),
        permissioned_roles.to_owned(),
    )
    .instrument(span.to_owned())
    .await?;

    span.record("myc.router.profile_id", &Some(profile.acc_id.to_string()))
        .record("myc.router.is_staff", &Some(profile.is_staff))
        .record("myc.router.is_manager", &Some(profile.is_manager))
        .record("myc.router.is_subscription", &Some(profile.is_subscription))
        .record(
            "myc.router.has_tenant_ownership",
            &Some(profile.tenants_ownership.is_some()),
        )
        .record(
            "myc.router.has_licensed_resources",
            &Some(profile.licensed_resources.is_some()),
        );

    //
    // Permissioned roles have priority over the roles. Them, it should be
    // evaluated first.
    //
    if let Some(_) = permissioned_roles {
        if profile.licensed_resources.is_none()
            && (!profile.is_manager && !profile.is_staff)
        {
            return Err(GatewayError::Forbidden(
                "User does not have permission to perform this action"
                    .to_string(),
            ));
        }
    }

    if let Some(_) = roles {
        if profile.licensed_resources.is_none()
            && (!profile.is_manager && !profile.is_staff)
        {
            return Err(GatewayError::Forbidden(
                "User does not have permission to perform this action"
                    .to_string(),
            ));
        }
    }

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
        match HeaderValue::from_str(
            &serde_json::to_string(&profile.to_profile()).unwrap(),
        ) {
            Err(err) => {
                warn!("err: {:?}", err.to_string());
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => res,
        },
    );

    tracing::trace!("Profile injected to forward");

    Ok(forwarded_req)
}
