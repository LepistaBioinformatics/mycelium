use crate::dtos::MyceliumRoleScopedConnectionStringData;

use actix_web::{web, HttpRequest};
use myc_core::domain::{
    dtos::token::{MultiTypeMeta, RoleWithPermissionsScope},
    entities::TokenFetching,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_CONNECTION_STRING_KEY,
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use tracing::error;

#[tracing::instrument(
    name = "fetch_role_scoped_connection_string_from_request",
    skip_all
)]
pub async fn fetch_role_scoped_connection_string_from_request(
    req: HttpRequest,
) -> Result<MyceliumRoleScopedConnectionStringData, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Fetch connection string from request header
    //
    // Use the `DEFAULT_CONNECTION_STRING_KEY` to fetch the connection string
    // from the request header.
    //
    // ? -----------------------------------------------------------------------

    let connection_string_header =
        match req.headers().get(DEFAULT_CONNECTION_STRING_KEY) {
            Some(value) => value,
            None => {
                return Err(GatewayError::Unauthorized(
                    "Connection string not found in request".to_string(),
                ))
            }
        };

    let connection_string = match connection_string_header.to_str() {
        Ok(value) => value,
        Err(_) => {
            return Err(GatewayError::Unauthorized(
                "Connection string is not a valid string".to_string(),
            ))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Parse scope from connection string
    // ? -----------------------------------------------------------------------

    let scope =
        match RoleWithPermissionsScope::try_from(connection_string.to_string())
        {
            Ok(value) => value,
            Err(_) => {
                return Err(GatewayError::Unauthorized(
                    "Connection string has invalid scope".to_string(),
                ))
            }
        };

    // ? -----------------------------------------------------------------------
    // ? Build dependencies
    // ? -----------------------------------------------------------------------

    let repo: &dyn TokenFetching = match req.app_data::<web::Data<SqlAppModule>>()
    {
        Some(module) => module.resolve_ref(),
        None => {
            error!("Unable to extract profile fetching module from request");

            return Err(GatewayError::InternalServerError(
                "Unexpected error on get profile".to_string(),
            ));
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Extract the connection string from the repo
    // ? -----------------------------------------------------------------------

    let token = match repo
        .get_connection_string_by_role_with_permissioned_roles_scope(scope)
        .await
    {
        Ok(value) => {
            match value {
                FetchResponseKind::Found(token) => token,
                FetchResponseKind::NotFound(msg) => {
                    if let Some(msg) = msg {
                        error!("Connection string not found in the database: {msg}");
                    } else {
                        error!("Connection string not found in the database");
                    }

                    return Err(GatewayError::Unauthorized(
                        "Invalid connection string".to_string(),
                    ));
                }
            }
        }
        Err(err) => {
            return Err(GatewayError::InternalServerError(format!(
                "Unable to fetch connection string: {err}"
            )))
        }
    };

    let meta = match token.meta {
        MultiTypeMeta::RoleScopedConnectionString(string) => string,
        _ => {
            error!("Connection string is not a RoleScopedConnectionString");

            return Err(GatewayError::InternalServerError(
                "Invalid connection string".to_string(),
            ));
        }
    };

    Ok(MyceliumRoleScopedConnectionStringData::new(meta))
}
