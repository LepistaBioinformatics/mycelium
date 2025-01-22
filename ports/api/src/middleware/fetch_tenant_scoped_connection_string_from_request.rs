use crate::dtos::MyceliumTenantScopedConnectionStringData;

use actix_web::{web, HttpRequest};
use myc_core::domain::{
    dtos::token::{MultiTypeMeta, TenantWithPermissionsScope},
    entities::TokenFetching,
};
use myc_diesel::repositories::AppModule;
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_CONNECTION_STRING_KEY,
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use tracing::error;

#[tracing::instrument(
    name = "fetch_tenant_scoped_connection_string_from_request",
    skip_all
)]
pub async fn fetch_tenant_scoped_connection_string_from_request(
    req: HttpRequest,
) -> Result<MyceliumTenantScopedConnectionStringData, GatewayError> {
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

    let scope = match TenantWithPermissionsScope::try_from(
        connection_string.to_string(),
    ) {
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

    let repo: &dyn TokenFetching = match req.app_data::<web::Data<AppModule>>()
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
        .get_connection_string_by_tenant_with_permissioned_roles_scope(scope)
        .await
    {
        Ok(value) => match value {
            FetchResponseKind::Found(token) => token,
            FetchResponseKind::NotFound(_) => {
                return Err(GatewayError::Unauthorized(
                    "Invalid connection string".to_string(),
                ))
            }
        },
        Err(err) => {
            return Err(GatewayError::InternalServerError(format!(
                "Unable to fetch connection string: {err}"
            )))
        }
    };

    let meta = match token.meta {
        MultiTypeMeta::TenantScopedConnectionString(string) => string,
        _ => {
            return Err(GatewayError::InternalServerError(
                "Connection string is not a RoleScopedConnectionString"
                    .to_string(),
            ))
        }
    };

    Ok(MyceliumTenantScopedConnectionStringData::new(meta))
}
