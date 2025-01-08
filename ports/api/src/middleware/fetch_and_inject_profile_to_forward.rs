use super::fetch_profile_from_request;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::route_type::PermissionedRoles;
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_PROFILE_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::warn;
use uuid::Uuid;

/// Fetch profile from email and inject on client request
///
/// Try to extract profile from email (these extracted from the bearer token)
/// and, then find the profile from the email and inject profile into the
/// forward request.
///
/// These use-case is usual over middleware or routers parts of the application.
#[tracing::instrument(name = "fetch_and_inject_profile_to_forward", skip_all)]
pub async fn fetch_and_inject_profile_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<ClientRequest, GatewayError> {
    let profile = fetch_profile_from_request(
        req,
        tenant,
        roles.to_owned(),
        permissioned_roles.to_owned(),
    )
    .await?;

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

    Ok(forwarded_req)
}
