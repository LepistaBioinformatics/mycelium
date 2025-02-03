use actix_web::{web, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use awc::http::header::Header;
use jwt::{Header as JwtHeader, RegisteredClaims, Token};
use myc_config::optional_config::OptionalConfig;
use myc_http_tools::{
    models::{
        auth_config::AuthConfig,
        external_providers_config::ExternalProviderConfig,
    },
    responses::GatewayError,
};

/// Parse issuer from request
///
/// This function is used to parse issuer from request.
#[tracing::instrument(name = "parse_issuer_from_request", skip_all)]
pub(crate) async fn parse_issuer_from_request(
    req: HttpRequest,
) -> Result<String, GatewayError> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            let msg =
                format!("Unexpected error on get bearer from request: {err}");

            tracing::error!("{msg}");

            return Err(GatewayError::Unauthorized(msg));
        }
        Ok(res) => res,
    }
    .to_string()
    .replace("Bearer ", "")
    .replace("bearer ", "");

    let unverified: Token<JwtHeader, RegisteredClaims, _> =
        match Token::parse_unverified(&auth) {
            Err(err) => {
                let msg = format!(
                    "Unexpected error on parse unverified token: {err}"
                );

                tracing::error!("{msg}");

                return Err(GatewayError::Unauthorized(msg));
            }
            Ok(res) => res,
        };

    let issuer = unverified.claims().issuer.as_ref().ok_or(
        GatewayError::Unauthorized("Could not check issuer.".to_string()),
    )?;

    Ok(issuer.to_owned().to_lowercase())
}

#[tracing::instrument(name = "parse_issuer_from_request_v2", skip_all)]
pub(crate) async fn parse_issuer_from_request_v2(
    req: HttpRequest,
) -> Result<(ExternalProviderConfig, String), GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract auth config from request
    // ? -----------------------------------------------------------------------

    let req_auth_config = if let Some(config) =
        req.app_data::<web::Data<AuthConfig>>()
    {
        config
    } else {
        tracing::error!(
            "Unable to extract AuthConfig from request. Authentication will not be completed"
        );

        return Err(GatewayError::InternalServerError(
            "Unable to initialize auth config".to_string(),
        ));
    };

    let external_providers = if let OptionalConfig::Enabled(config) =
        &req_auth_config.external
    {
        config
    } else {
        tracing::error!(
            "Unable to extract external providers from request. Authentication will not be completed"
        );

        return Err(GatewayError::Unauthorized(
            "Authentication with external providers disabled".to_string(),
        ));
    };

    // ? -----------------------------------------------------------------------
    // ? Extract issuer from request
    // ? -----------------------------------------------------------------------

    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            let msg =
                format!("Unexpected error on get bearer from request: {err}");

            tracing::error!("{msg}");

            return Err(GatewayError::Unauthorized(msg));
        }
        Ok(res) => res,
    }
    .to_string()
    .replace("Bearer ", "")
    .replace("bearer ", "");

    let unverified: Token<JwtHeader, RegisteredClaims, _> =
        match Token::parse_unverified(&auth) {
            Err(err) => {
                let msg = format!(
                    "Unexpected error on parse unverified token: {err}"
                );

                tracing::error!("{msg}");

                return Err(GatewayError::Unauthorized(msg));
            }
            Ok(res) => res,
        };

    let issuer = unverified.claims().issuer.as_ref().ok_or(
        GatewayError::Unauthorized("Could not check issuer.".to_string()),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Check if issuer is valid
    // ? -----------------------------------------------------------------------

    for provider in external_providers {
        let local_issuer =
            provider.issuer.async_get_or_error().await.map_err(|_| {
                GatewayError::Unauthorized(
                    "Could not check issuer.".to_string(),
                )
            })?;

        if local_issuer.to_lowercase() == issuer.to_lowercase() {
            return Ok((provider.to_owned(), auth));
        }
    }

    tracing::error!("Invalid issuer: {}", issuer);

    Err(GatewayError::Unauthorized("Invalid issuer".to_string()))
}
