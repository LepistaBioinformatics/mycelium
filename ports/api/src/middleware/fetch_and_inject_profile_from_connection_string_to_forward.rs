use crate::middleware::fetch_profile_from_request_connection_string;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::security_group::PermissionedRoles;
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_PROFILE_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::error;

#[tracing::instrument(
    name = "fetch_and_inject_profile_from_connection_string_to_forward",
    skip_all
)]
pub(crate) async fn fetch_and_inject_profile_from_connection_string_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<ClientRequest, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract the role scoped connection string
    // ? -----------------------------------------------------------------------

    let profile = fetch_profile_from_request_connection_string(
        req,
        None,
        roles,
        permissioned_roles,
    )
    .await?
    .to_profile();

    // ? -----------------------------------------------------------------------
    // ? Inject the serialized connection string into the forwarded request
    // ? -----------------------------------------------------------------------

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
        match HeaderValue::from_str(&serde_json::to_string(&profile).unwrap()) {
            Err(err) => {
                error!("err: {:?}", err.to_string());
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => res,
        },
    );

    Ok(forwarded_req)
}
