use super::fetch_role_scoped_connection_string_from_request;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, security_group::PermissionedRoles,
    token::RoleScopedConnectionString,
};
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_SCOPE_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::error;

#[tracing::instrument(
    name = "fetch_and_inject_role_scoped_connection_string_to_forward",
    skip_all
)]
pub async fn fetch_and_inject_role_scoped_connection_string_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<ClientRequest, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract the role scoped connection string
    // ? -----------------------------------------------------------------------

    let connection_string: RoleScopedConnectionString =
        fetch_role_scoped_connection_string_from_request(req)
            .await?
            .connection_string()
            .to_owned();

    // ? -----------------------------------------------------------------------
    // ? Check if the connection string has the needed roles
    // ? -----------------------------------------------------------------------

    if let Some(roles) = roles {
        if let Err(err) = connection_string.contain_enough_roles(roles) {
            if err.is_in(vec![NativeErrorCodes::MYC00013]) {
                return Err(GatewayError::Forbidden(
                    "Insufficient permissions to access resource".to_string(),
                ));
            }

            error!("Unexpected error while checking permissions: {err}");

            return Err(GatewayError::InternalServerError(
                "Unexpected error while checking permissions".to_string(),
            ));
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Check if the connection string has the needed roles
    // ? -----------------------------------------------------------------------

    if let Some(permissioned_roles) = permissioned_roles {
        if let Err(err) = connection_string
            .contain_enough_permissioned_roles(permissioned_roles)
        {
            if err.is_in(vec![NativeErrorCodes::MYC00013]) {
                return Err(GatewayError::Forbidden(
                    "Insufficient permissions to access resource".to_string(),
                ));
            }

            error!("Unexpected error while checking permissions: {err}");

            return Err(GatewayError::InternalServerError(
                "Unexpected error while checking permissions".to_string(),
            ));
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Inject the serialized connection string into the forwarded request
    // ? -----------------------------------------------------------------------

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_SCOPE_KEY).unwrap(),
        match HeaderValue::from_str(
            &serde_json::to_string(&connection_string.scope).unwrap(),
        ) {
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
