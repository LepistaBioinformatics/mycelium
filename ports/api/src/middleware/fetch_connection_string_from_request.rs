use crate::dtos::MyceliumConnectionStringData;

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{
        dtos::token::{MultiTypeMeta, UserAccountScope},
        entities::TokenFetching,
    },
    models::AccountLifeCycle,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_CONNECTION_STRING_KEY,
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use tracing::{error, warn};

#[tracing::instrument(name = "fetch_connection_string_from_request", skip_all)]
pub async fn fetch_connection_string_from_request(
    req: HttpRequest,
) -> Result<MyceliumConnectionStringData, GatewayError> {
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

    let scope = match UserAccountScope::try_from(connection_string.to_string())
    {
        Ok(value) => value,
        Err(_) => {
            return Err(GatewayError::Unauthorized(
                "Connection string has invalid scope".to_string(),
            ))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Verify HMAC before touching the database
    //
    // Reject forged or downgraded tokens up-front. Native error codes
    // MYC00030 / MYC00031 / MYC00032 are carried through so logs can be
    // searched by reason. See `22-hmac-key-rotation.md` for the contract.
    //
    // ? -----------------------------------------------------------------------

    let life_cycle = match req.app_data::<web::Data<AccountLifeCycle>>() {
        Some(value) => value,
        None => {
            error!(
                "AccountLifeCycle is not registered as app data; cannot \
                 verify connection-string signature",
            );
            return Err(GatewayError::InternalServerError(
                "Server misconfiguration: HMAC key set unavailable".to_string(),
            ));
        }
    };

    if let Err(err) = scope.verify_signature(life_cycle.get_ref()).await {
        warn!(
            connection_string_verification_failed = true,
            code = %err.code(),
            "Rejecting connection string: {err}",
        );
        return Err(GatewayError::Unauthorized(format!(
            "Invalid connection string: {}",
            err.code(),
        )));
    }

    // ? -----------------------------------------------------------------------
    // ? Build dependencies
    // ? -----------------------------------------------------------------------

    let repo: &dyn TokenFetching = match req
        .app_data::<web::Data<SqlAppModule>>()
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

    let token = match repo.get_connection_string(scope).await {
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
        MultiTypeMeta::UserAccountConnectionString(string) => string,
        _ => {
            error!("Connection string is not a UserAccountConnectionString");

            return Err(GatewayError::InternalServerError(
                "Invalid connection string".to_string(),
            ));
        }
    };

    Ok(MyceliumConnectionStringData::new(meta))
}
